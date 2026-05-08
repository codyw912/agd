use crate::git;
use crate::project::Project;
use anyhow::{Context, Result};
use std::ffi::OsString;
use std::path::Path;

pub fn sync(project: &Project) -> Result<()> {
    let workspace = project
        .workspaces
        .iter()
        .find(|workspace| workspace.id == project.default_workspace)
        .context("default workspace not found")?;
    let target = &project.default_target;

    require_clean(&project.human_checkout, "human checkout")?;
    require_clean(&workspace.path, "agent workspace")?;

    let fetch_spec = format!("refs/heads/{target}:refs/remotes/origin/{target}");
    git::run(
        &workspace.path,
        [
            OsString::from("fetch"),
            OsString::from("origin"),
            OsString::from(fetch_spec),
        ],
    )?;

    let fetched_ref = format!("refs/remotes/origin/{target}");
    let human_tip = git::stdout(&workspace.path, ["rev-parse", &fetched_ref])?;
    let human_tip = human_tip.trim();
    let workspace_tip = git::stdout(&workspace.path, ["rev-parse", target])?;
    let workspace_tip = workspace_tip.trim();
    ensure_fast_forward(&workspace.path, workspace_tip, human_tip)?;

    let target_ref = format!("refs/heads/{target}");
    git::run(&workspace.path, ["update-ref", &target_ref, human_tip])?;

    println!("Synced {target}");
    Ok(())
}

fn require_clean(repo: &Path, name: &str) -> Result<()> {
    let status = git::stdout(repo, ["status", "--porcelain"])?;
    if !status.trim().is_empty() {
        anyhow::bail!("{name} has uncommitted changes");
    }
    Ok(())
}

fn ensure_fast_forward(repo: &Path, old: &str, new: &str) -> Result<()> {
    let merge_base = git::stdout(repo, ["merge-base", old, new])?;
    if merge_base.trim() != old {
        anyhow::bail!("default target cannot be fast-forwarded");
    }
    Ok(())
}
