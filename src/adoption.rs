use crate::git;
use crate::paths::AgdPaths;
use crate::project::Project;
use anyhow::{Context, Result};
use std::ffi::OsString;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

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

struct PreparedAdoption<'a> {
    workspace: &'a crate::project::Workspace,
    branch: &'a str,
    fetched_ref: String,
    base: String,
    tip: String,
    _lock: AdoptionLock,
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

    let operation_id = format!("op_{}", Uuid::new_v4().simple());
    let _lock = AdoptionLock::acquire(paths, &project.project_id, &operation_id)?;
    let safety_ref = format!("refs/agd/safety/{operation_id}");
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
        _lock,
    })
}

fn bless_squash(project: &Project, prepared: &PreparedAdoption<'_>) -> Result<()> {
    git::run(
        &project.human_checkout,
        ["merge", "--squash", &prepared.fetched_ref],
    )?;

    let trailers = trailers(project, prepared, "squash");

    git::run(
        &project.human_checkout,
        [
            OsString::from("commit"),
            OsString::from("-S"),
            OsString::from("-m"),
            OsString::from(format!("Adopt {}", prepared.branch)),
            OsString::from("-m"),
            OsString::from(trailers),
        ],
    )?;

    println!("Blessed {}", prepared.branch);
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
    format!(
        "AGD-Project: {}\nAGD-Workspace: {}\nAGD-Agent-Branch: {}\nAGD-Agent-Base: {}\nAGD-Agent-Tip: {}\nAGD-Adoption: {}",
        project.project_id,
        prepared.workspace.id,
        prepared.branch,
        prepared.base,
        prepared.tip,
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

struct AdoptionLock {
    path: PathBuf,
}

impl AdoptionLock {
    fn acquire(paths: &AgdPaths, project_id: &str, operation_id: &str) -> Result<Self> {
        let lock_dir = paths.project_dir(project_id).join("locks");
        fs::create_dir_all(&lock_dir).with_context(|| format!("create {}", lock_dir.display()))?;
        let path = lock_dir.join("bless.lock");
        let lock = serde_json::json!({
            "operation_id": operation_id,
            "operation": "bless",
            "project_id": project_id,
        });
        let mut lock_file = match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(lock_file) => lock_file,
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                anyhow::bail!("bless operation already in progress: {}", path.display());
            }
            Err(error) => {
                return Err(error).with_context(|| format!("create lock {}", path.display()));
            }
        };
        lock_file
            .write_all(serde_json::to_string_pretty(&lock)?.as_bytes())
            .with_context(|| format!("write lock {}", path.display()))?;
        Ok(Self { path })
    }
}

impl Drop for AdoptionLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
