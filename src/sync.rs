use crate::git;
use crate::operation_lock::OperationLock;
use crate::paths::AgdPaths;
use crate::project::Project;
use anyhow::{Context, Result};
use serde::Serialize;
use std::ffi::OsString;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct SyncResult {
    pub target: String,
    pub workspace_id: String,
    pub before: String,
    pub after: String,
    pub updates: Vec<SyncUpdate>,
}

#[derive(Debug, Serialize)]
pub struct SyncUpdate {
    pub target: String,
    pub before: String,
    pub after: String,
}

pub fn sync(paths: &AgdPaths, project: &Project) -> Result<SyncResult> {
    let workspace = project
        .workspaces
        .iter()
        .find(|workspace| workspace.id == project.default_workspace)
        .context("default workspace not found")?;
    require_clean(&project.human_checkout, "human checkout")?;
    require_clean(&workspace.path, "agent workspace")?;
    let _lock = OperationLock::acquire(paths, &project.project_id, &workspace.id, "sync")?;

    let targets = mirror_targets(project)?;
    let mut updates = Vec::new();
    for target in &targets {
        updates.push(sync_target(
            &workspace.path,
            target,
            target == &project.default_target,
        )?);
    }

    let primary = updates
        .iter()
        .find(|update| update.target == project.default_target)
        .context("default target was not synced")?;

    Ok(SyncResult {
        target: primary.target.clone(),
        workspace_id: workspace.id.clone(),
        before: primary.before.clone(),
        after: primary.after.clone(),
        updates,
    })
}

fn mirror_targets(project: &Project) -> Result<Vec<String>> {
    let mut targets = Vec::new();
    if human_branch_exists(&project.human_checkout, "main")? {
        targets.push("main".to_string());
    }
    if !targets
        .iter()
        .any(|target| target == &project.default_target)
    {
        targets.push(project.default_target.clone());
    }
    Ok(targets)
}

fn human_branch_exists(repo: &Path, branch: &str) -> Result<bool> {
    Ok(git::run(
        repo,
        ["rev-parse", "--verify", &format!("refs/heads/{branch}")],
    )
    .is_ok())
}

fn sync_target(workspace: &Path, target: &str, is_default_target: bool) -> Result<SyncUpdate> {
    let fetch_spec = format!("refs/heads/{target}:refs/remotes/origin/{target}");
    git::run(
        workspace,
        [
            OsString::from("fetch"),
            OsString::from("origin"),
            OsString::from(fetch_spec),
        ],
    )?;

    let fetched_ref = format!("refs/remotes/origin/{target}");
    let human_tip = git::stdout(workspace, ["rev-parse", &fetched_ref])?;
    let human_tip = human_tip.trim();
    let workspace_tip = git::stdout(workspace, ["rev-parse", target])
        .ok()
        .map(|tip| tip.trim().to_string());
    if let Some(workspace_tip) = &workspace_tip {
        ensure_fast_forward(
            workspace,
            target,
            is_default_target,
            workspace_tip,
            human_tip,
        )?;
    }

    let target_ref = format!("refs/heads/{target}");
    git::run(workspace, ["update-ref", &target_ref, human_tip])?;

    Ok(SyncUpdate {
        target: target.to_string(),
        before: workspace_tip.unwrap_or_default(),
        after: human_tip.to_string(),
    })
}

fn require_clean(repo: &Path, name: &str) -> Result<()> {
    let status = git::stdout(repo, ["status", "--porcelain"])?;
    if !status.trim().is_empty() {
        anyhow::bail!("{name} has uncommitted changes");
    }
    Ok(())
}

fn ensure_fast_forward(
    repo: &Path,
    target: &str,
    is_default_target: bool,
    old: &str,
    new: &str,
) -> Result<()> {
    let merge_base = git::stdout(repo, ["merge-base", old, new])?;
    if merge_base.trim() != old {
        let guidance =
            "Run `agd reset-workspace` to recreate the managed workspace from the human checkout.";
        if is_default_target {
            anyhow::bail!("default target cannot be fast-forwarded. {guidance}");
        }
        anyhow::bail!("{target} cannot be fast-forwarded. {guidance}");
    }
    Ok(())
}
