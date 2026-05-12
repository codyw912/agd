use crate::git;
use crate::project::Project;
use crate::provenance;
use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::ffi::OsString;
use std::io::ErrorKind;
use std::path::Path;
use std::process::{Command, Output};

#[derive(Debug, Serialize)]
pub struct PullRequestResult {
    pub branch: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_step: Option<String>,
}

pub fn open(project: &Project, branch: Option<&str>, cwd: &Path) -> Result<PullRequestResult> {
    let workspace = default_workspace(project)?;
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

    let body = pr_body(project, workspace, &branch)?;
    let Some(output) = create_pull_request(project, &branch, &body)? else {
        return Ok(PullRequestResult {
            branch: branch.clone(),
            url: None,
            next_step: Some(next_step(project, &branch)),
        });
    };
    let url = String::from_utf8(output.stdout).context("PR tool output was not utf-8")?;
    Ok(PullRequestResult {
        branch,
        url: Some(url.trim().to_string()),
        next_step: None,
    })
}

enum PrCreateError {
    MissingCli,
    Failure(anyhow::Error),
}

fn create_pull_request(project: &Project, branch: &str, body: &str) -> Result<Option<Output>> {
    if is_gitlab_remote(project) {
        return match create_gitlab_mr(project, branch, body) {
            Ok(output) => Ok(Some(output)),
            Err(PrCreateError::MissingCli) => Ok(None),
            Err(PrCreateError::Failure(error)) => Err(error),
        };
    }

    match create_github_pr(project, branch, body) {
        Ok(output) => Ok(Some(output)),
        Err(PrCreateError::MissingCli) => match create_gitlab_mr(project, branch, body) {
            Ok(output) => Ok(Some(output)),
            Err(PrCreateError::MissingCli) => Ok(None),
            Err(PrCreateError::Failure(error)) => Err(error),
        },
        Err(PrCreateError::Failure(error)) => Err(error),
    }
}

fn is_gitlab_remote(project: &Project) -> bool {
    match git::stdout(
        &project.human_checkout,
        ["config", "--get", "remote.origin.url"],
    ) {
        Ok(url) => url.to_ascii_lowercase().contains("gitlab"),
        Err(_) => false,
    }
}

fn create_github_pr(project: &Project, branch: &str, body: &str) -> Result<Output, PrCreateError> {
    let output = match Command::new("gh")
        .args([
            "pr",
            "create",
            "--base",
            &project.default_target,
            "--head",
            branch,
            "--title",
            branch,
            "--body",
            body,
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

fn create_gitlab_mr(project: &Project, branch: &str, body: &str) -> Result<Output, PrCreateError> {
    let output = match Command::new("glab")
        .args([
            "mr",
            "create",
            "--target-branch",
            &project.default_target,
            "--source-branch",
            branch,
            "--title",
            branch,
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

fn next_step(project: &Project, branch: &str) -> String {
    format!(
        "Pushed {branch} to origin. Open a pull request from {branch} into {}.",
        project.default_target
    )
}

fn pr_body(
    project: &Project,
    workspace: &crate::project::Workspace,
    branch: &str,
) -> Result<String> {
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
        "## Summary\n- Agent branch: {branch}\n- Base branch: {}\n- Base commit: {}\n- Agent tip: {}\n\n## Commits\n{commits}\n\n## Changed Files\n{files}\n\n## Provenance\nAGD-Agent-Branch: {branch}\nAGD-Agent-Base: {}\nAGD-Agent-Tip: {}\nAGD-Patch-SHA256: {patch_sha256}\n\n## Adoption Recommendation\nReview this PR, then adopt with `agd bless {branch}` if it should become signed human history.",
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
    if branch.is_empty() || branch == project.default_target || branch == "main" {
        anyhow::bail!("agent branch is required");
    }
    Ok(branch)
}

fn default_workspace(project: &Project) -> Result<&crate::project::Workspace> {
    project
        .workspaces
        .iter()
        .find(|workspace| workspace.id == project.default_workspace)
        .context("default workspace not found")
}
