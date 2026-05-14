use crate::branch_policy;
use crate::git;
use crate::project::{Project, Workspace};
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::Path;

pub fn branches(project: &Project, workspace_id: Option<&str>) -> Result<()> {
    for branch in branch_summaries(project, workspace_id)? {
        println!(
            "{}  {}  {}",
            branch.branch,
            plural(branch.commits, "commit", "commits"),
            plural(branch.files_changed, "file changed", "files changed")
        );
    }
    Ok(())
}

pub fn branch_names(project: &Project, workspace_id: Option<&str>) -> Result<Vec<String>> {
    let workspace = selected_workspace(project, workspace_id)?;
    branch_names_for_workspace(project, workspace)
}

fn branch_names_for_workspace(project: &Project, workspace: &Workspace) -> Result<Vec<String>> {
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

#[derive(Debug, Serialize)]
pub struct AgentBranchSummary {
    pub branch: String,
    pub commits: usize,
    pub files_changed: usize,
}

pub fn branch_summaries(
    project: &Project,
    workspace_id: Option<&str>,
) -> Result<Vec<AgentBranchSummary>> {
    let workspace = selected_workspace(project, workspace_id)?;
    branch_names_for_workspace(project, workspace)?
        .into_iter()
        .map(|branch| {
            let commits = git::stdout(
                &workspace.path,
                [
                    "rev-list",
                    "--count",
                    &format!("{}..{branch}", project.default_target),
                ],
            )?;
            let files = git::stdout(
                &workspace.path,
                [
                    "diff",
                    "--name-only",
                    &format!("{}...{branch}", project.default_target),
                ],
            )?;
            Ok(AgentBranchSummary {
                branch,
                commits: commits.trim().parse().context("parse commit count")?,
                files_changed: files.lines().count(),
            })
        })
        .collect()
}

pub fn log(
    project: &Project,
    workspace_id: Option<&str>,
    branch: Option<&str>,
    cwd: &Path,
) -> Result<()> {
    let log = commit_log(project, workspace_id, branch, cwd)?;
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

pub fn commit_log(
    project: &Project,
    workspace_id: Option<&str>,
    branch: Option<&str>,
    cwd: &Path,
) -> Result<CommitLog> {
    let workspace = selected_workspace(project, workspace_id)?;
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

pub fn diff(
    project: &Project,
    workspace_id: Option<&str>,
    branch: Option<&str>,
    cwd: &Path,
) -> Result<()> {
    let diff = branch_diff(project, workspace_id, branch, cwd)?;
    print!("{}", diff.patch);
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct BranchDiff {
    pub branch: String,
    pub patch: String,
}

pub fn branch_diff(
    project: &Project,
    workspace_id: Option<&str>,
    branch: Option<&str>,
    cwd: &Path,
) -> Result<BranchDiff> {
    let workspace = selected_workspace(project, workspace_id)?;
    let branch = resolve_branch(project, branch, cwd)?;
    let patch = git::stdout(
        &workspace.path,
        ["diff", &format!("{}...{branch}", project.default_target)],
    )?;
    Ok(BranchDiff { branch, patch })
}

pub fn files(
    project: &Project,
    workspace_id: Option<&str>,
    branch: Option<&str>,
    cwd: &Path,
) -> Result<()> {
    let files = changed_files(project, workspace_id, branch, cwd)?;
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

pub fn changed_files(
    project: &Project,
    workspace_id: Option<&str>,
    branch: Option<&str>,
    cwd: &Path,
) -> Result<ChangedFiles> {
    let workspace = selected_workspace(project, workspace_id)?;
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
    if !branch_policy::is_agent_branch(project, &branch) {
        anyhow::bail!("agent branch is required");
    }
    Ok(branch)
}

fn selected_workspace<'a>(
    project: &'a Project,
    workspace_id: Option<&str>,
) -> Result<&'a crate::project::Workspace> {
    let workspace_id = workspace_id.unwrap_or(&project.default_workspace);
    project
        .workspaces
        .iter()
        .find(|workspace| workspace.id == workspace_id)
        .with_context(|| format!("workspace not found: {workspace_id}"))
}

fn is_review_branch(project: &Project, branch: &str) -> bool {
    branch_policy::is_agent_branch(project, branch)
}

fn plural(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("{count} {singular}")
    } else {
        format!("{count} {plural}")
    }
}
