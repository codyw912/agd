use crate::branch_policy;
use crate::git;
use crate::operation_lock::OperationLock;
use crate::paths::AgdPaths;
use crate::project::Project;
use crate::provenance;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

pub enum AdoptionMode {
    Squash,
    Preserve,
    Merge,
}

pub enum AdoptionTarget {
    Direct,
    Branch {
        target_branch: String,
        adoption_branch: String,
    },
}

#[derive(Debug, Serialize)]
pub struct BlessAbortResult {
    pub status: &'static str,
}

#[derive(Debug, Serialize)]
pub struct BlessContinueResult {
    pub status: &'static str,
    pub branch: String,
    pub adoption: String,
    pub target_branch: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adoption_branch: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BlessResult {
    pub status: &'static str,
    pub branch: String,
    pub adoption: &'static str,
    pub target_branch: String,
    pub adoption_branch: Option<String>,
}

pub fn bless(
    paths: &AgdPaths,
    project: &Project,
    branch: &str,
    mode: AdoptionMode,
    target: AdoptionTarget,
) -> Result<BlessResult> {
    if !branch_policy::is_agent_branch(project, branch) {
        anyhow::bail!("agent branch is required");
    }
    let prepared = prepare(paths, project, branch, target)?;
    match mode {
        AdoptionMode::Squash => bless_squash(project, &prepared),
        AdoptionMode::Preserve => bless_preserve(project, &prepared),
        AdoptionMode::Merge => bless_merge(project, &prepared),
    }
}

pub fn derive_adoption_branch(branch: &str) -> String {
    if branch == "agent/main" {
        return "adopt/main".to_string();
    }
    branch.strip_prefix("agent/").unwrap_or(branch).to_string()
}

pub fn abort(project: &Project) -> Result<BlessAbortResult> {
    git::run(&project.human_checkout, ["reset", "--merge"])?;
    remove_bless_state(project)?;
    Ok(BlessAbortResult { status: "aborted" })
}

pub fn continue_bless(paths: &AgdPaths, project: &Project) -> Result<BlessContinueResult> {
    let state = read_bless_state(project)?;
    if state.adoption != "squash" {
        anyhow::bail!("only squash bless operations can be continued");
    }
    let target_branch = state
        .target_branch
        .clone()
        .unwrap_or_else(|| project.default_target.clone());
    let adoption_branch = match state.adoption_branch.clone() {
        Some(branch) => Some(branch),
        None => {
            let current = git::stdout(&project.human_checkout, ["branch", "--show-current"])?;
            let current = current.trim();
            if current.is_empty() || current == target_branch {
                None
            } else {
                Some(current.to_string())
            }
        }
    };
    let _lock = OperationLock::acquire(paths, &project.project_id, &state.workspace_id, "bless")?;
    let trailers = trailers_from_parts(
        &project.human_checkout,
        &project.project_id,
        &state.workspace_id,
        &state.branch,
        &state.base,
        &state.tip,
        &state.adoption,
    )?;
    commit_squash(project, &state.branch, trailers)?;
    remove_bless_state(project)?;
    Ok(BlessContinueResult {
        status: "continued",
        branch: state.branch,
        adoption: state.adoption,
        target_branch,
        adoption_branch,
    })
}

struct PreparedAdoption<'a> {
    workspace: &'a crate::project::Workspace,
    branch: &'a str,
    fetched_ref: String,
    base: String,
    tip: String,
    target_branch: String,
    adoption_branch: Option<String>,
    _lock: OperationLock,
}

fn prepare<'a>(
    paths: &AgdPaths,
    project: &'a Project,
    branch: &'a str,
    target: AdoptionTarget,
) -> Result<PreparedAdoption<'a>> {
    let workspace = project
        .workspaces
        .iter()
        .find(|workspace| workspace.id == project.default_workspace)
        .context("default workspace not found")?;

    require_clean(&project.human_checkout, "human checkout")?;
    require_clean(&workspace.path, "agent workspace")?;

    let lock = OperationLock::acquire(paths, &project.project_id, &workspace.id, "bless")?;
    let safety_ref = format!("refs/agd/safety/{}", lock.operation_id());
    git::run(&project.human_checkout, ["update-ref", &safety_ref, "HEAD"])?;

    let (target_branch, adoption_branch) = match target {
        AdoptionTarget::Direct => (
            git::stdout(&project.human_checkout, ["branch", "--show-current"])?
                .trim()
                .to_string(),
            None,
        ),
        AdoptionTarget::Branch {
            target_branch,
            adoption_branch,
        } => {
            ensure_adoption_branch_available(project, &adoption_branch)?;
            git::run(
                &project.human_checkout,
                ["switch", "-c", &adoption_branch, &target_branch],
            )?;
            (target_branch, Some(adoption_branch))
        }
    };

    let fetched_ref = format!("refs/agd/agent/{branch}");
    let fetch_spec = format!("refs/heads/{branch}:{fetched_ref}");
    git::run(
        &project.human_checkout,
        [
            OsString::from("fetch"),
            workspace.path.as_os_str().to_owned(),
            OsString::from(fetch_spec),
        ],
    )?;

    let base = git::stdout(
        &project.human_checkout,
        ["merge-base", "HEAD", &fetched_ref],
    )?;
    let tip = git::stdout(&project.human_checkout, ["rev-parse", &fetched_ref])?;

    Ok(PreparedAdoption {
        workspace,
        branch,
        fetched_ref,
        base: base.trim().to_string(),
        tip: tip.trim().to_string(),
        target_branch,
        adoption_branch,
        _lock: lock,
    })
}

fn ensure_adoption_branch_available(project: &Project, branch: &str) -> Result<()> {
    if git::run(
        &project.human_checkout,
        ["check-ref-format", "--branch", branch],
    )
    .is_err()
    {
        anyhow::bail!("invalid adoption branch: {branch}");
    }
    let refname = format!("refs/heads/{branch}");
    if git::run(
        &project.human_checkout,
        ["show-ref", "--verify", "--quiet", &refname],
    )
    .is_ok()
    {
        anyhow::bail!("adoption branch already exists: {branch}");
    }
    Ok(())
}

fn bless_squash(project: &Project, prepared: &PreparedAdoption<'_>) -> Result<BlessResult> {
    write_bless_state(project, &BlessState::from_prepared(prepared, "squash"))?;
    if let Err(error) = git::run(
        &project.human_checkout,
        ["merge", "--squash", &prepared.fetched_ref],
    ) {
        anyhow::bail!(
            "bless squash failed: {error}\nrun `agd bless --continue` after resolving conflicts, or `agd bless --abort` to restore the human checkout"
        );
    }

    let trailers = trailers(project, prepared, "squash")?;
    if let Err(error) = commit_squash(project, prepared.branch, trailers) {
        anyhow::bail!(
            "bless commit failed: {error}\nrun `agd bless --continue` after fixing the commit problem, or `agd bless --abort` to restore the human checkout"
        );
    }
    remove_bless_state(project)?;

    Ok(bless_result(prepared, "squash"))
}

fn commit_squash(project: &Project, branch: &str, trailers: String) -> Result<()> {
    git::run(
        &project.human_checkout,
        [
            OsString::from("commit"),
            OsString::from("-S"),
            OsString::from("-m"),
            OsString::from(format!("Adopt {branch}")),
            OsString::from("-m"),
            OsString::from(trailers),
        ],
    )?;
    Ok(())
}

fn bless_merge(project: &Project, prepared: &PreparedAdoption<'_>) -> Result<BlessResult> {
    let trailers = trailers(project, prepared, "merge")?;
    git::run(
        &project.human_checkout,
        [
            OsString::from("merge"),
            OsString::from("--no-ff"),
            OsString::from("-S"),
            OsString::from(&prepared.fetched_ref),
            OsString::from("-m"),
            OsString::from(format!("Merge {}", prepared.branch)),
            OsString::from("-m"),
            OsString::from(trailers),
        ],
    )?;

    Ok(bless_result(prepared, "merge"))
}

fn bless_preserve(project: &Project, prepared: &PreparedAdoption<'_>) -> Result<BlessResult> {
    let range = format!("{}..{}", prepared.base, prepared.fetched_ref);
    let commits = git::stdout(&project.human_checkout, ["rev-list", "--reverse", &range])?;
    let commits: Vec<_> = commits.lines().map(str::to_string).collect();
    if commits.is_empty() {
        anyhow::bail!("selected branch has no commits to preserve");
    }

    for commit in commits {
        let patch_hash = provenance::adopted_patch_sha256(&project.human_checkout, &commit)?;
        git::run(
            &project.human_checkout,
            ["cherry-pick", "--no-commit", &commit],
        )?;
        git::run(
            &project.human_checkout,
            [
                OsString::from("commit"),
                OsString::from("-S"),
                OsString::from("-C"),
                OsString::from(&commit),
                OsString::from("--trailer"),
                OsString::from(format!("AGD-Project={}", project.project_id)),
                OsString::from("--trailer"),
                OsString::from(format!("AGD-Workspace={}", prepared.workspace.id)),
                OsString::from("--trailer"),
                OsString::from(format!("AGD-Agent-Branch={}", prepared.branch)),
                OsString::from("--trailer"),
                OsString::from(format!("AGD-Agent-Base={}", prepared.base)),
                OsString::from("--trailer"),
                OsString::from(format!("AGD-Agent-Tip={}", prepared.tip)),
                OsString::from("--trailer"),
                OsString::from(format!("AGD-Agent-Commit={commit}")),
                OsString::from("--trailer"),
                OsString::from("AGD-Adoption=preserve"),
                OsString::from("--trailer"),
                OsString::from(format!("AGD-Patch-SHA256={patch_hash}")),
            ],
        )?;
    }

    Ok(bless_result(prepared, "preserve"))
}

fn bless_result(prepared: &PreparedAdoption<'_>, adoption: &'static str) -> BlessResult {
    BlessResult {
        status: "blessed",
        branch: prepared.branch.to_string(),
        adoption,
        target_branch: prepared.target_branch.clone(),
        adoption_branch: prepared.adoption_branch.clone(),
    }
}

fn trailers(project: &Project, prepared: &PreparedAdoption<'_>, adoption: &str) -> Result<String> {
    trailers_from_parts(
        &project.human_checkout,
        &project.project_id,
        &prepared.workspace.id,
        prepared.branch,
        &prepared.base,
        &prepared.tip,
        adoption,
    )
}

fn trailers_from_parts(
    repo: &Path,
    project_id: &str,
    workspace_id: &str,
    branch: &str,
    base: &str,
    tip: &str,
    adoption: &str,
) -> Result<String> {
    let patch_hash = provenance::patch_sha256(repo, base, tip)?;
    Ok(format!(
        "AGD-Project: {project_id}\nAGD-Workspace: {workspace_id}\nAGD-Agent-Branch: {branch}\nAGD-Agent-Base: {base}\nAGD-Agent-Tip: {tip}\nAGD-Adoption: {adoption}\nAGD-Patch-SHA256: {patch_hash}"
    ))
}

fn require_clean(repo: &Path, name: &str) -> Result<()> {
    let status = git::stdout(repo, ["status", "--porcelain"])?;
    if !status.trim().is_empty() {
        anyhow::bail!("{name} has uncommitted changes");
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct BlessState {
    branch: String,
    workspace_id: String,
    base: String,
    tip: String,
    adoption: String,
    target_branch: Option<String>,
    adoption_branch: Option<String>,
}

impl BlessState {
    fn from_prepared(prepared: &PreparedAdoption<'_>, adoption: &str) -> Self {
        Self {
            branch: prepared.branch.to_string(),
            workspace_id: prepared.workspace.id.clone(),
            base: prepared.base.clone(),
            tip: prepared.tip.clone(),
            adoption: adoption.to_string(),
            target_branch: Some(prepared.target_branch.clone()),
            adoption_branch: prepared.adoption_branch.clone(),
        }
    }
}

fn write_bless_state(project: &Project, state: &BlessState) -> Result<()> {
    let path = bless_state_path(project)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let contents = serde_json::to_vec_pretty(state)?;
    fs::write(&path, contents).with_context(|| format!("write {}", path.display()))
}

fn read_bless_state(project: &Project) -> Result<BlessState> {
    let path = bless_state_path(project)?;
    let contents = match fs::read(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            anyhow::bail!("no pending bless operation");
        }
        Err(error) => return Err(error).with_context(|| format!("read {}", path.display())),
    };
    serde_json::from_slice(&contents).with_context(|| format!("parse {}", path.display()))
}

fn remove_bless_state(project: &Project) -> Result<()> {
    let path = bless_state_path(project)?;
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("remove {}", path.display())),
    }
}

fn bless_state_path(project: &Project) -> Result<PathBuf> {
    Ok(git_dir(&project.human_checkout)?.join("agd/bless.json"))
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
