use crate::git;
use crate::project::Project;
use anyhow::{Context, Result};
use serde::Serialize;
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
    let log = commit_log(project, branch, cwd)?;
    for commit in log.commits {
        println!("{} {}", commit.short_hash, commit.subject);
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct CommitLog {
    pub branch: String,
    pub commits: Vec<CommitLogEntry>,
}

#[derive(Debug, Serialize)]
pub struct CommitLogEntry {
    pub hash: String,
    pub short_hash: String,
    pub subject: String,
}

pub fn commit_log(project: &Project, branch: Option<&str>, cwd: &Path) -> Result<CommitLog> {
    let workspace = default_workspace(project)?;
    let branch = resolve_branch(project, branch, cwd)?;
    let hashes = git::stdout(
        &workspace.path,
        ["rev-list", &format!("{}..{branch}", project.default_target)],
    )?;
    let commits = hashes
        .lines()
        .map(|hash| commit_log_entry(&workspace.path, hash))
        .collect::<Result<Vec<_>>>()?;
    Ok(CommitLog { branch, commits })
}

fn commit_log_entry(repo: &Path, hash: &str) -> Result<CommitLogEntry> {
    let full_hash = git::stdout(repo, ["show", "-s", "--format=%H", hash])?;
    let short_hash = git::stdout(repo, ["show", "-s", "--format=%h", hash])?;
    let subject = git::stdout(repo, ["show", "-s", "--format=%s", hash])?;
    Ok(CommitLogEntry {
        hash: full_hash.trim().to_string(),
        short_hash: short_hash.trim().to_string(),
        subject: subject.trim().to_string(),
    })
}

pub fn diff(project: &Project, branch: Option<&str>, cwd: &Path) -> Result<()> {
    let diff = branch_diff(project, branch, cwd)?;
    print!("{}", diff.patch);
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct BranchDiff {
    pub branch: String,
    pub patch: String,
}

pub fn branch_diff(project: &Project, branch: Option<&str>, cwd: &Path) -> Result<BranchDiff> {
    let workspace = default_workspace(project)?;
    let branch = resolve_branch(project, branch, cwd)?;
    let patch = git::stdout(
        &workspace.path,
        ["diff", &format!("{}...{branch}", project.default_target)],
    )?;
    Ok(BranchDiff { branch, patch })
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
    if branch.is_empty() || !is_review_branch(project, &branch) {
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
    branch != project.default_target && branch != "main"
}
