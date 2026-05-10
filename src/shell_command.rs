use crate::json_output;
use crate::project::Project;
use anyhow::{bail, Context, Result};
use std::ffi::OsString;
use std::process::Command;

pub fn run(project: &Project) -> Result<()> {
    let workspace = json_output::default_workspace(project)?;
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
