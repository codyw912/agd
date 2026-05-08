use crate::git;
use crate::project::Project;
use anyhow::{Context, Result};
use std::path::Path;

pub fn branches(project: &Project) -> Result<()> {
    let workspace = default_workspace(project)?;
    let output = git::stdout(
        &workspace.path,
        ["for-each-ref", "--format=%(refname:short)", "refs/heads"],
    )?;
    for branch in output
        .lines()
        .filter(|branch| is_review_branch(project, branch))
    {
        println!("{branch}");
    }
    Ok(())
}

pub fn log(project: &Project, branch: Option<&str>, cwd: &Path) -> Result<()> {
    let workspace = default_workspace(project)?;
    let branch = resolve_branch(project, branch, cwd)?;
    let output = git::stdout(
        &workspace.path,
        [
            "log",
            "--oneline",
            &format!("{}..{branch}", project.default_target),
        ],
    )?;
    print!("{output}");
    Ok(())
}

pub fn diff(project: &Project, branch: Option<&str>, cwd: &Path) -> Result<()> {
    let workspace = default_workspace(project)?;
    let branch = resolve_branch(project, branch, cwd)?;
    let output = git::stdout(
        &workspace.path,
        ["diff", &format!("{}...{branch}", project.default_target)],
    )?;
    print!("{output}");
    Ok(())
}

pub fn files(project: &Project, branch: Option<&str>, cwd: &Path) -> Result<()> {
    let workspace = default_workspace(project)?;
    let branch = resolve_branch(project, branch, cwd)?;
    let output = git::stdout(
        &workspace.path,
        [
            "diff",
            "--name-only",
            &format!("{}...{branch}", project.default_target),
        ],
    )?;
    print!("{output}");
    Ok(())
}

fn resolve_branch(project: &Project, branch: Option<&str>, cwd: &Path) -> Result<String> {
    let branch = match branch {
        Some(branch) => branch.to_string(),
        None => git::stdout(cwd, ["branch", "--show-current"])?
            .trim()
            .to_string(),
    };
    if branch.is_empty() || branch == project.default_target {
        anyhow::bail!("agent branch is required");
    }
    Ok(branch)
}

fn default_workspace(project: &Project) -> Result<&crate::project::Workspace> {
    project
        .workspaces
        .iter()
        .find(|workspace| workspace.id == project.default_workspace)
        .context("default workspace not found")
}

fn is_review_branch(project: &Project, branch: &str) -> bool {
    branch != project.default_target
}
