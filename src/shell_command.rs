use crate::project::{Project, Workspace};
use anyhow::{bail, Context, Result};
use std::ffi::OsString;
use std::process::Command;

pub fn run(project: &Project, workspace_id: Option<&str>) -> Result<()> {
    let workspace = find_workspace(project, workspace_id)?;
    let shell = std::env::var_os("SHELL").unwrap_or_else(|| OsString::from("/bin/sh"));
    let status = Command::new(&shell)
        .current_dir(&workspace.path)
        .env("AGD_WORKSPACE", "1")
        .env("AGD_PROJECT_ID", &project.project_id)
        .env("AGD_WORKSPACE_ID", &workspace.id)
        .status()
        .context("run shell")?;

    if !status.success() {
        bail!("shell exited with {status}");
    }

    Ok(())
}

fn find_workspace<'a>(project: &'a Project, workspace_id: Option<&str>) -> Result<&'a Workspace> {
    let workspace_id = workspace_id.unwrap_or(&project.default_workspace);
    project
        .workspaces
        .iter()
        .find(|workspace| workspace.id == workspace_id)
        .with_context(|| format!("workspace not found: {workspace_id}"))
}
