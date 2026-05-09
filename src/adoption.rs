use crate::git;
use crate::operation_lock::OperationLock;
use crate::paths::AgdPaths;
use crate::project::Project;
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

pub fn bless(paths: &AgdPaths, project: &Project, branch: &str, mode: AdoptionMode) -> Result<()> {
    let prepared = prepare(paths, project, branch)?;
    match mode {
        AdoptionMode::Squash => bless_squash(project, &prepared),
        AdoptionMode::Preserve => bless_preserve(project, &prepared),
        AdoptionMode::Merge => bless_merge(project, &prepared),
    }
}

pub fn abort(project: &Project) -> Result<()> {
    git::run(&project.human_checkout, ["reset", "--merge"])?;
    remove_bless_state(project)?;
    println!("Aborted bless operation");
    Ok(())
}

pub fn continue_bless(paths: &AgdPaths, project: &Project) -> Result<()> {
    let state = read_bless_state(project)?;
    if state.adoption != "squash" {
        anyhow::bail!("only squash bless operations can be continued");
    }
    let _lock = OperationLock::acquire(paths, &project.project_id, &state.workspace_id, "bless")?;
    let trailers = trailers_from_parts(
        &project.project_id,
        &state.workspace_id,
        &state.branch,
        &state.base,
        &state.tip,
        &state.adoption,
    );
    commit_squash(project, &state.branch, trailers)?;
    remove_bless_state(project)?;
    println!("Continued bless operation");
    Ok(())
}

struct PreparedAdoption<'a> {
    workspace: &'a crate::project::Workspace,
    branch: &'a str,
    fetched_ref: String,
    base: String,
    tip: String,
    _lock: OperationLock,
}

fn prepare<'a>(
    paths: &AgdPaths,
    project: &'a Project,
    branch: &'a str,
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
        _lock: lock,
    })
}

fn bless_squash(project: &Project, prepared: &PreparedAdoption<'_>) -> Result<()> {
    write_bless_state(project, &BlessState::from_prepared(prepared, "squash"))?;
    if let Err(error) = git::run(
        &project.human_checkout,
        ["merge", "--squash", &prepared.fetched_ref],
    ) {
        anyhow::bail!(
            "bless squash failed: {error}\nrun `agd bless --continue` after resolving conflicts, or `agd bless --abort` to restore the human checkout"
        );
    }

    let trailers = trailers(project, prepared, "squash");
    commit_squash(project, prepared.branch, trailers)?;
    remove_bless_state(project)?;

    println!("Blessed {}", prepared.branch);
    Ok(())
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

fn bless_merge(project: &Project, prepared: &PreparedAdoption<'_>) -> Result<()> {
    let trailers = trailers(project, prepared, "merge");
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

    println!("Merged {}", prepared.branch);
    Ok(())
}

fn bless_preserve(project: &Project, prepared: &PreparedAdoption<'_>) -> Result<()> {
    let range = format!("{}..{}", project.default_target, prepared.fetched_ref);
    let commits = git::stdout(&project.human_checkout, ["rev-list", "--reverse", &range])?;
    let commits: Vec<_> = commits.lines().map(str::to_string).collect();
    if commits.is_empty() {
        anyhow::bail!("selected branch has no commits to preserve");
    }

    for commit in commits {
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
            ],
        )?;
    }

    println!("Preserved {}", prepared.branch);
    Ok(())
}

fn trailers(project: &Project, prepared: &PreparedAdoption<'_>, adoption: &str) -> String {
    trailers_from_parts(
        &project.project_id,
        &prepared.workspace.id,
        prepared.branch,
        &prepared.base,
        &prepared.tip,
        adoption,
    )
}

fn trailers_from_parts(
    project_id: &str,
    workspace_id: &str,
    branch: &str,
    base: &str,
    tip: &str,
    adoption: &str,
) -> String {
    format!(
        "AGD-Project: {}\nAGD-Workspace: {}\nAGD-Agent-Branch: {}\nAGD-Agent-Base: {}\nAGD-Agent-Tip: {}\nAGD-Adoption: {}",
        project_id,
        workspace_id,
        branch,
        base,
        tip,
        adoption
    )
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
}

impl BlessState {
    fn from_prepared(prepared: &PreparedAdoption<'_>, adoption: &str) -> Self {
        Self {
            branch: prepared.branch.to_string(),
            workspace_id: prepared.workspace.id.clone(),
            base: prepared.base.clone(),
            tip: prepared.tip.clone(),
            adoption: adoption.to_string(),
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
    let contents = fs::read(&path).with_context(|| format!("read {}", path.display()))?;
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
