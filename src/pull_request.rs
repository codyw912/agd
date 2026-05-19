use crate::adoption::{self, AdoptionMode, AdoptionTarget};
use crate::branch_policy;
use crate::git;
use crate::paths::AgdPaths;
use crate::project::{Project, Workspace};
use crate::provenance;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const AGD_PROVENANCE_HELP: &str =
    "This PR was prepared with AGD; provenance details are below. Learn more: https://github.com/codyw912/agd";

#[derive(Debug, Serialize)]
pub struct PullRequestResult {
    pub branch: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_step: Option<String>,
}

pub struct BlessedPrOptions<'a> {
    pub workspace_id: Option<&'a str>,
    pub branch: Option<&'a str>,
    pub mode: AdoptionMode,
    pub target_branch: Option<String>,
    pub adoption_branch: Option<String>,
}

pub fn open(
    project: &Project,
    workspace_id: Option<&str>,
    branch: Option<&str>,
    cwd: &Path,
) -> Result<PullRequestResult> {
    let workspace = find_workspace(project, workspace_id)?;
    let branch = resolve_branch(project, branch, cwd)?;
    let pr_ref = format!("refs/agd/pr/{branch}");
    let fetch_spec = format!("refs/heads/{branch}:{pr_ref}");
    git::run(
        &project.human_checkout,
        [
            OsString::from("fetch"),
            workspace.path.as_os_str().to_owned(),
            OsString::from(fetch_spec),
        ],
    )?;

    let push_spec = format!("{pr_ref}:refs/heads/{branch}");
    git::run(&project.human_checkout, ["push", "origin", &push_spec])?;

    let title = title_from_range(project, &project.default_target, &pr_ref, &branch)?;
    let body = pr_body(project, workspace, &branch)?;
    let Some(output) =
        create_pull_request(project, &branch, &project.default_target, &title, &body)?
    else {
        return Ok(PullRequestResult {
            branch: branch.clone(),
            url: None,
            next_step: Some(next_step(project, &branch, &project.default_target)),
        });
    };
    let url = String::from_utf8(output.stdout).context("PR tool output was not utf-8")?;
    Ok(PullRequestResult {
        branch,
        url: Some(url.trim().to_string()),
        next_step: None,
    })
}

pub fn open_blessed(
    paths: &AgdPaths,
    project: &Project,
    options: BlessedPrOptions<'_>,
    cwd: &Path,
) -> Result<PullRequestResult> {
    let branch = resolve_branch(project, options.branch, cwd)?;
    let target_branch = options
        .target_branch
        .unwrap_or_else(|| project.default_target.clone());
    let adoption_branch = options
        .adoption_branch
        .unwrap_or_else(|| adoption::derive_adoption_branch(&branch));
    let result = match adoption::bless(
        paths,
        project,
        options.workspace_id,
        &branch,
        options.mode,
        AdoptionTarget::Branch {
            target_branch: target_branch.clone(),
            adoption_branch,
        },
    ) {
        Ok(result) => result,
        Err(error) => return Err(pr_bless_error(project, error)),
    };
    let adoption_branch = result
        .adoption_branch
        .as_ref()
        .context("bless did not create an adoption branch")?;
    let state = PrState::from_bless_result(&target_branch, adoption_branch, &result);
    write_pr_state(project, &state)?;
    finish_blessed_pr(project, &state)
}

fn pr_bless_error(project: &Project, error: anyhow::Error) -> anyhow::Error {
    match adoption::has_pending_bless(project) {
        Ok(true) => anyhow!(
            "{error}\nrun `agd pr --continue` after fixing the adoption problem, or `agd bless --abort` to restore the human checkout"
        ),
        Ok(false) => error,
        Err(state_error) => anyhow!("{error}\nfailed to inspect bless recovery state: {state_error}"),
    }
}

pub fn continue_blessed(paths: &AgdPaths, project: &Project) -> Result<PullRequestResult> {
    if let Some(state) = read_pr_state(project)? {
        return finish_blessed_pr(project, &state);
    }

    let result = adoption::continue_bless(paths, project)?;
    let adoption_branch = result
        .adoption_branch
        .as_ref()
        .context("continued bless did not create an adoption branch")?;
    let state = PrState::from_bless_continue_result(adoption_branch, &result);
    write_pr_state(project, &state)?;
    finish_blessed_pr(project, &state)
}

#[derive(Debug, Serialize, Deserialize)]
struct PrState {
    agent_branch: String,
    adoption_branch: String,
    target_branch: String,
    adoption: String,
}

impl PrState {
    fn from_bless_result(
        target_branch: &str,
        adoption_branch: &str,
        result: &adoption::BlessResult,
    ) -> Self {
        Self {
            agent_branch: result.branch.clone(),
            adoption_branch: adoption_branch.to_string(),
            target_branch: target_branch.to_string(),
            adoption: result.adoption.to_string(),
        }
    }

    fn from_bless_continue_result(
        adoption_branch: &str,
        result: &adoption::BlessContinueResult,
    ) -> Self {
        Self {
            agent_branch: result.branch.clone(),
            adoption_branch: adoption_branch.to_string(),
            target_branch: result.target_branch.clone(),
            adoption: result.adoption.clone(),
        }
    }
}

fn finish_blessed_pr(project: &Project, state: &PrState) -> Result<PullRequestResult> {
    let adoption_branch = &state.adoption_branch;
    let push_spec = format!("refs/heads/{adoption_branch}:refs/heads/{adoption_branch}");
    git::run(&project.human_checkout, ["push", "origin", &push_spec])?;

    let body = adoption_pr_body(
        project,
        &state.target_branch,
        adoption_branch,
        &state.agent_branch,
        &state.adoption,
    )?;
    let title = adoption_pr_title(project, adoption_branch)?;
    let Some(output) = create_pull_request(
        project,
        adoption_branch,
        &state.target_branch,
        &title,
        &body,
    )?
    else {
        remove_pr_state(project)?;
        return Ok(PullRequestResult {
            branch: adoption_branch.clone(),
            url: None,
            next_step: Some(next_step(project, adoption_branch, &state.target_branch)),
        });
    };
    let url = String::from_utf8(output.stdout).context("PR tool output was not utf-8")?;
    remove_pr_state(project)?;
    Ok(PullRequestResult {
        branch: adoption_branch.clone(),
        url: Some(url.trim().to_string()),
        next_step: None,
    })
}

fn write_pr_state(project: &Project, state: &PrState) -> Result<()> {
    let path = pr_state_path(project)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let contents = serde_json::to_vec_pretty(state)?;
    fs::write(&path, contents).with_context(|| format!("write {}", path.display()))
}

fn read_pr_state(project: &Project) -> Result<Option<PrState>> {
    let path = pr_state_path(project)?;
    let contents = match fs::read(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("read {}", path.display())),
    };
    let state =
        serde_json::from_slice(&contents).with_context(|| format!("parse {}", path.display()))?;
    Ok(Some(state))
}

fn remove_pr_state(project: &Project) -> Result<()> {
    let path = pr_state_path(project)?;
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("remove {}", path.display())),
    }
}

fn pr_state_path(project: &Project) -> Result<PathBuf> {
    Ok(git_dir(&project.human_checkout)?.join("agd/pr.json"))
}

fn git_dir(repo: &Path) -> Result<PathBuf> {
    let git_dir = git::stdout(repo, ["rev-parse", "--git-dir"])?;
    let path = PathBuf::from(git_dir.trim());
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(repo.join(path))
    }
}

enum PrCreateError {
    MissingCli,
    Failure(anyhow::Error),
}

fn create_pull_request(
    project: &Project,
    branch: &str,
    base: &str,
    title: &str,
    body: &str,
) -> Result<Option<Output>> {
    if is_gitlab_remote(project) {
        return match create_gitlab_mr(project, branch, base, title, body) {
            Ok(output) => Ok(Some(output)),
            Err(PrCreateError::MissingCli) => Ok(None),
            Err(PrCreateError::Failure(error)) => Err(error),
        };
    }

    match create_github_pr(project, branch, base, title, body) {
        Ok(output) => Ok(Some(output)),
        Err(PrCreateError::MissingCli) => {
            match create_gitlab_mr(project, branch, base, title, body) {
                Ok(output) => Ok(Some(output)),
                Err(PrCreateError::MissingCli) => Ok(None),
                Err(PrCreateError::Failure(error)) => Err(error),
            }
        }
        Err(PrCreateError::Failure(error)) => Err(error),
    }
}

fn is_gitlab_remote(project: &Project) -> bool {
    upstream_remote_url(project).is_some_and(|url| url.to_ascii_lowercase().contains("gitlab"))
}

fn create_github_pr(
    project: &Project,
    branch: &str,
    base: &str,
    title: &str,
    body: &str,
) -> Result<Output, PrCreateError> {
    let output = match Command::new("gh")
        .args([
            "pr", "create", "--base", base, "--head", branch, "--title", title, "--body", body,
        ])
        .current_dir(&project.human_checkout)
        .output()
    {
        Ok(output) => output,
        Err(error) if error.kind() == ErrorKind::NotFound => return Err(PrCreateError::MissingCli),
        Err(error) => return Err(PrCreateError::Failure(anyhow!(error).context("run gh"))),
    };
    if !output.status.success() {
        return Err(PrCreateError::Failure(anyhow!(
            "gh pr create failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(output)
}

fn create_gitlab_mr(
    project: &Project,
    branch: &str,
    base: &str,
    title: &str,
    body: &str,
) -> Result<Output, PrCreateError> {
    let output = match Command::new("glab")
        .args([
            "mr",
            "create",
            "--target-branch",
            base,
            "--source-branch",
            branch,
            "--title",
            title,
            "--description",
            body,
        ])
        .current_dir(&project.human_checkout)
        .output()
    {
        Ok(output) => output,
        Err(error) if error.kind() == ErrorKind::NotFound => return Err(PrCreateError::MissingCli),
        Err(error) => return Err(PrCreateError::Failure(anyhow!(error).context("run glab"))),
    };
    if !output.status.success() {
        return Err(PrCreateError::Failure(anyhow!(
            "glab mr create failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(output)
}

fn next_step(project: &Project, branch: &str, base: &str) -> String {
    let generic =
        format!("Pushed {branch} to origin. Open a pull request from {branch} into {base}.");
    match hosted_pr_url(project, branch, base) {
        Some(url) => format!("{generic}\nOpen: {url}"),
        None => generic,
    }
}

fn hosted_pr_url(project: &Project, branch: &str, base: &str) -> Option<String> {
    let remote_url = upstream_remote_url(project)?;
    let (host, path) = parse_remote_host_path(&remote_url)?;
    let host_lower = host.to_ascii_lowercase();
    let base = percent_encode_component(base);
    let branch = percent_encode_component(branch);

    if host_lower.contains("github") {
        return Some(format!(
            "https://{host}/{path}/compare/{base}...{branch}?expand=1"
        ));
    }
    if host_lower.contains("gitlab") {
        return Some(format!(
            "https://{host}/{path}/-/merge_requests/new?merge_request[source_branch]={branch}&merge_request[target_branch]={base}"
        ));
    }

    None
}

fn upstream_remote_url(project: &Project) -> Option<String> {
    project
        .upstream_remote
        .as_ref()
        .map(|remote| remote.fetch_url.clone())
        .or_else(|| {
            git::stdout(
                &project.human_checkout,
                ["config", "--get", "remote.origin.url"],
            )
            .ok()
            .map(|url| url.trim().to_string())
            .filter(|url| !url.is_empty())
        })
}

fn parse_remote_host_path(remote_url: &str) -> Option<(String, String)> {
    let remote_url = remote_url.trim();
    let (host, path) = if let Some((_, rest)) = remote_url.split_once("://") {
        let rest = rest.rsplit_once('@').map_or(rest, |(_, rest)| rest);
        rest.split_once('/')?
    } else if let Some((host, path)) = remote_url.split_once(':') {
        let host = host.rsplit_once('@').map_or(host, |(_, host)| host);
        (host, path)
    } else {
        return None;
    };
    let path = path.trim_matches('/').strip_suffix(".git").unwrap_or(path);
    if host.is_empty() || path.is_empty() {
        return None;
    }
    Some((host.to_string(), path.to_string()))
}

fn percent_encode_component(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

fn adoption_pr_title(project: &Project, fallback: &str) -> Result<String> {
    let commit_body = git::stdout(&project.human_checkout, ["log", "-1", "--format=%B"])?;
    let trailers = parse_agd_trailers(&commit_body);
    title_from_trailers(project, &trailers, fallback)
}

fn title_from_trailers(
    project: &Project,
    trailers: &BTreeMap<String, String>,
    fallback: &str,
) -> Result<String> {
    let Some(base) = trailers.get("AGD-Agent-Base") else {
        return Ok(fallback.to_string());
    };
    let Some(tip) = trailers.get("AGD-Agent-Tip") else {
        return Ok(fallback.to_string());
    };
    title_from_range(project, base, tip, fallback)
}

fn title_from_range(project: &Project, base: &str, tip: &str, fallback: &str) -> Result<String> {
    let range = format!("{base}..{tip}");
    let subjects = git::stdout(&project.human_checkout, ["log", "--format=%s", &range])?;
    let subjects = subjects
        .lines()
        .filter(|subject| !subject.trim().is_empty())
        .collect::<Vec<_>>();
    if subjects.len() == 1 {
        return Ok(subjects[0].to_string());
    }
    Ok(fallback.to_string())
}

fn adoption_pr_body(
    project: &Project,
    base: &str,
    adoption_branch: &str,
    agent_branch: &str,
    adoption: &str,
) -> Result<String> {
    let adoption_commit = git::stdout(&project.human_checkout, ["rev-parse", "HEAD"])?;
    let commit_body = git::stdout(&project.human_checkout, ["log", "-1", "--format=%B"])?;
    let trailers = parse_agd_trailers(&commit_body);
    let commits = agent_commits(project, &trailers)?;
    let files = agent_changed_files(project, &trailers)?;

    Ok(format!(
        "## Summary\n- Adopts `{agent_branch}` into human-owned branch `{adoption_branch}` for review against `{base}`.\n- Uses `{adoption}` adoption in commit `{}`.\n- Keeps AGD provenance below for traceability.\n\n## Agent Commits\n{commits}\n\n## Changed Files\n{files}\n\n## Adoption\n- Human adoption branch: {adoption_branch}\n- Base branch: {base}\n- Agent branch: {agent_branch}\n- Adoption: {adoption}\n- Adoption commit: {}\n\n## Provenance\n{AGD_PROVENANCE_HELP}\n{}",
        adoption_commit.trim(),
        adoption_commit.trim(),
        commit_body.trim_end()
    ))
}

fn agent_commits(project: &Project, trailers: &BTreeMap<String, String>) -> Result<String> {
    let Some(base) = trailers.get("AGD-Agent-Base") else {
        return Ok("- See the adoption commit for agent commit details.".to_string());
    };
    let Some(tip) = trailers.get("AGD-Agent-Tip") else {
        return Ok("- See the adoption commit for agent commit details.".to_string());
    };
    let range = format!("{base}..{tip}");
    let commits = git::stdout(&project.human_checkout, ["log", "--format=- %s", &range])?;
    if commits.trim().is_empty() {
        return Ok("- No unique agent commits.".to_string());
    }
    Ok(commits.trim_end().to_string())
}

fn agent_changed_files(project: &Project, trailers: &BTreeMap<String, String>) -> Result<String> {
    let Some(base) = trailers.get("AGD-Agent-Base") else {
        return Ok("- See the adoption commit for changed file details.".to_string());
    };
    let Some(tip) = trailers.get("AGD-Agent-Tip") else {
        return Ok("- See the adoption commit for changed file details.".to_string());
    };
    let range = format!("{base}...{tip}");
    let files = git::stdout(&project.human_checkout, ["diff", "--name-only", &range])?;
    let files = files
        .lines()
        .map(|file| format!("- {file}"))
        .collect::<Vec<_>>()
        .join("\n");
    if files.trim().is_empty() {
        return Ok("- No changed files.".to_string());
    }
    Ok(files)
}

fn parse_agd_trailers(message: &str) -> BTreeMap<String, String> {
    message
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            let key = key.trim();
            if !key.starts_with("AGD-") {
                return None;
            }
            Some((key.to_string(), value.trim().to_string()))
        })
        .collect()
}

fn pr_body(project: &Project, workspace: &Workspace, branch: &str) -> Result<String> {
    let base_commit = git::stdout(&workspace.path, ["rev-parse", &project.default_target])?;
    let agent_tip = git::stdout(&workspace.path, ["rev-parse", branch])?;
    let patch_sha256 =
        provenance::patch_sha256(&workspace.path, base_commit.trim(), agent_tip.trim())?;
    let commits = git::stdout(
        &workspace.path,
        [
            "log",
            "--format=- %s",
            &format!("{}..{branch}", project.default_target),
        ],
    )?;
    let files = git::stdout(
        &workspace.path,
        [
            "diff",
            "--name-only",
            &format!("{}...{branch}", project.default_target),
        ],
    )?;
    let files = files
        .lines()
        .map(|file| format!("- {file}"))
        .collect::<Vec<_>>()
        .join("\n");
    let commits = if commits.trim().is_empty() {
        "- No unique commits".to_string()
    } else {
        commits.trim_end().to_string()
    };
    let files = if files.trim().is_empty() {
        "- No changed files".to_string()
    } else {
        files
    };

    Ok(format!(
        "## Summary\n- Publishes `{branch}` for review against `{}` without adopting it into human-owned history yet.\n- Shows the agent commit list and changed files so reviewers can evaluate the work normally.\n- Keeps AGD provenance below for traceability.\n\n## Agent Commits\n{commits}\n\n## Changed Files\n{files}\n\n## Review Details\n- Agent branch: {branch}\n- Base branch: {}\n- Base commit: {}\n- Agent tip: {}\n\n## Adoption Recommendation\nReview this PR, then adopt with `agd bless {branch}` if it should become signed human history.\n\n## Provenance\n{AGD_PROVENANCE_HELP}\nAGD-Agent-Branch: {branch}\nAGD-Agent-Base: {}\nAGD-Agent-Tip: {}\nAGD-Patch-SHA256: {patch_sha256}",
        project.default_target,
        project.default_target,
        base_commit.trim(),
        agent_tip.trim(),
        base_commit.trim(),
        agent_tip.trim()
    ))
}

fn resolve_branch(project: &Project, branch: Option<&str>, cwd: &Path) -> Result<String> {
    let branch = match branch {
        Some(branch) => branch.to_string(),
        None => git::stdout(cwd, ["branch", "--show-current"])?
            .trim()
            .to_string(),
    };
    if !branch_policy::is_agent_branch(project, &branch) {
        anyhow::bail!("agent branch is required");
    }
    Ok(branch)
}

fn find_workspace<'a>(project: &'a Project, workspace_id: Option<&str>) -> Result<&'a Workspace> {
    let workspace_id = workspace_id.unwrap_or(&project.default_workspace);
    project
        .workspaces
        .iter()
        .find(|workspace| workspace.id == workspace_id)
        .with_context(|| format!("workspace not found: {workspace_id}"))
}
