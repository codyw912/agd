use crate::git;
use crate::paths::AgdPaths;
use crate::project::Project;
use anyhow::{Context, Result};
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub fn bless_squash(paths: &AgdPaths, project: &Project, branch: &str) -> Result<()> {
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

    git::run(&project.human_checkout, ["merge", "--squash", &fetched_ref])?;

    let trailers = format!(
        "AGD-Project: {}\nAGD-Workspace: {}\nAGD-Agent-Branch: {}\nAGD-Agent-Base: {}\nAGD-Agent-Tip: {}\nAGD-Adoption: squash",
        project.project_id,
        workspace.id,
        branch,
        base.trim(),
        tip.trim()
    );

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

    println!("Blessed {branch}");
    Ok(())
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
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .with_context(|| format!("create lock {}", path.display()))?
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
