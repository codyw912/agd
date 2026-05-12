use crate::git;
use crate::operation_lock::OperationLock;
use crate::paths::AgdPaths;
use crate::project::{self, Project};
use crate::workspace;
use anyhow::{Context, Result};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
pub struct DiscardResult {
    pub branch: String,
    pub workspace_id: String,
}

#[derive(Debug, Serialize)]
pub struct ResetWorkspaceResult {
    pub workspace_id: String,
    pub path: PathBuf,
}

pub fn discard(
    paths: &AgdPaths,
    project: &Project,
    branch: &str,
    force: bool,
) -> Result<DiscardResult> {
    let workspace = default_workspace(project)?;
    if is_protected_branch(project, branch) {
        anyhow::bail!("refusing to discard protected branch");
    }
    if !force {
        require_clean(&workspace.path, "agent workspace")?;
    }
    let _lock = OperationLock::acquire(paths, &project.project_id, &workspace.id, "discard")?;
    if force {
        discard_workspace_changes(&workspace.path)?;
        if current_branch(&workspace.path)?.as_deref() == Some(branch) {
            git::run(&workspace.path, ["switch", project.default_target.as_str()])?;
        }
    }
    git::run(&workspace.path, ["branch", "-D", branch])?;
    Ok(DiscardResult {
        branch: branch.to_string(),
        workspace_id: workspace.id.clone(),
    })
}

fn is_protected_branch(project: &Project, branch: &str) -> bool {
    branch.is_empty() || branch == project.default_target || branch == "main"
}

pub fn reset_workspace(
    paths: &AgdPaths,
    project: &Project,
    force: bool,
) -> Result<ResetWorkspaceResult> {
    let mut project = project.clone();
    let workspace = default_workspace(&project)?;
    let workspace_id = workspace.id.clone();
    if !force {
        require_clean(&workspace.path, "agent workspace")?;
    }
    let workspace_path = workspace.path.clone();
    let _lock =
        OperationLock::acquire(paths, &project.project_id, &workspace.id, "reset-workspace")?;

    fs::remove_dir_all(&workspace_path)
        .with_context(|| format!("remove {}", workspace_path.display()))?;
    let recreated = workspace::ensure_default_workspace(paths, &mut project)?;
    project::save_project(paths, &project)?;

    Ok(ResetWorkspaceResult {
        workspace_id,
        path: recreated,
    })
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

fn discard_workspace_changes(repo: &Path) -> Result<()> {
    git::run(repo, ["reset", "--hard"])?;
    git::run(repo, ["clean", "-fd"])?;
    Ok(())
}

fn current_branch(repo: &Path) -> Result<Option<String>> {
    let branch = git::stdout(repo, ["branch", "--show-current"])?;
    let branch = branch.trim();
    Ok((!branch.is_empty()).then(|| branch.to_string()))
}
