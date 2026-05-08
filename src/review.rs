use crate::git;
use crate::project::Project;
use anyhow::{Context, Result};
use std::path::Path;

pub fn branches(project: &Project) -> Result<()> {
    for branch in branch_names(project)? {
        println!("{branch}");
    }
    Ok(())
}

pub fn branch_names(project: &Project) -> Result<Vec<String>> {
    let workspace = default_workspace(project)?;
    let output = git::stdout(
        &workspace.path,
        ["for-each-ref", "--format=%(refname:short)", "refs/heads"],
    )?;
    Ok(output
        .lines()
        .filter(|branch| is_review_branch(project, branch))
        .map(str::to_string)
        .collect())
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
    let files = changed_files(project, branch, cwd)?;
    for file in files.files {
        println!("{file}");
    }
    Ok(())
}

#[derive(Debug)]
pub struct ChangedFiles {
    pub branch: String,
    pub files: Vec<String>,
}

pub fn changed_files(project: &Project, branch: Option<&str>, cwd: &Path) -> Result<ChangedFiles> {
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
    Ok(ChangedFiles {
        branch,
        files: output.lines().map(str::to_string).collect(),
    })
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
