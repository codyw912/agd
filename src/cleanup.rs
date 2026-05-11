use crate::git;
use crate::operation_lock::OperationLock;
use crate::paths::AgdPaths;
use crate::project::{self, Project};
use crate::workspace;
use anyhow::{Context, Result};
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct DiscardResult {
    pub branch: String,
    pub workspace_id: String,
}

pub fn discard(paths: &AgdPaths, project: &Project, branch: &str) -> Result<DiscardResult> {
    let workspace = default_workspace(project)?;
    require_clean(&workspace.path, "agent workspace")?;
    if branch == project.default_target {
        anyhow::bail!("refusing to discard default target");
    }
    let _lock = OperationLock::acquire(paths, &project.project_id, &workspace.id, "discard")?;
    git::run(&workspace.path, ["branch", "-D", branch])?;
    Ok(DiscardResult {
        branch: branch.to_string(),
        workspace_id: workspace.id.clone(),
    })
}

pub fn reset_workspace(paths: &AgdPaths, project: &Project) -> Result<()> {
    let mut project = project.clone();
    let workspace = default_workspace(&project)?;
    require_clean(&workspace.path, "agent workspace")?;
    let workspace_path = workspace.path.clone();
    let _lock =
        OperationLock::acquire(paths, &project.project_id, &workspace.id, "reset-workspace")?;

    fs::remove_dir_all(&workspace_path)
        .with_context(|| format!("remove {}", workspace_path.display()))?;
    let recreated = workspace::ensure_default_workspace(paths, &mut project)?;
    project::save_project(paths, &project)?;

    println!("Reset workspace");
    println!("  {}", recreated.display());
    Ok(())
}

fn default_workspace(project: &Project) -> Result<&crate::project::Workspace> {
    project
        .workspaces
        .iter()
        .find(|workspace| workspace.id == project.default_workspace)
        .context("default workspace not found")
}

fn require_clean(repo: &Path, name: &str) -> Result<()> {
    let status = git::stdout(repo, ["status", "--porcelain"])?;
    if !status.trim().is_empty() {
        anyhow::bail!("{name} has uncommitted changes");
    }
    Ok(())
}
