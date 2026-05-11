use crate::git;
use crate::project::Project;
use anyhow::{Context, Result};
use serde::Serialize;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Serialize)]
pub struct PullRequestResult {
    pub branch: String,
    pub url: String,
}

pub fn open(project: &Project, branch: Option<&str>, cwd: &Path) -> Result<PullRequestResult> {
    let workspace = default_workspace(project)?;
    let branch = resolve_branch(project, branch, cwd)?;
    let pr_ref = format!("refs/agd/pr/{branch}");
    let fetch_spec = format!("refs/heads/{branch}:{pr_ref}");
    git::run(
        &project.human_checkout,
        [
            OsString::from("fetch"),
            workspace.path.as_os_str().to_owned(),
            OsString::from(fetch_spec),
        ],
    )?;

    let push_spec = format!("{pr_ref}:refs/heads/{branch}");
    git::run(&project.human_checkout, ["push", "origin", &push_spec])?;

    let body = pr_body(project, workspace, &branch)?;
    let output = Command::new("gh")
        .args([
            "pr",
            "create",
            "--base",
            &project.default_target,
            "--head",
            &branch,
            "--title",
            &branch,
            "--body",
            &body,
        ])
        .current_dir(&project.human_checkout)
        .output()
        .context("run gh")?;
    if !output.status.success() {
        anyhow::bail!(
            "gh pr create failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let url = String::from_utf8(output.stdout).context("gh output was not utf-8")?;
    Ok(PullRequestResult {
        branch,
        url: url.trim().to_string(),
    })
}

fn pr_body(
    project: &Project,
    workspace: &crate::project::Workspace,
    branch: &str,
) -> Result<String> {
    let base_commit = git::stdout(&workspace.path, ["rev-parse", &project.default_target])?;
    let commits = git::stdout(
        &workspace.path,
        [
            "log",
            "--format=- %s",
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
    let files = files
        .lines()
        .map(|file| format!("- {file}"))
        .collect::<Vec<_>>()
        .join("\n");
    let commits = if commits.trim().is_empty() {
        "- No unique commits".to_string()
    } else {
        commits.trim_end().to_string()
    };
    let files = if files.trim().is_empty() {
        "- No changed files".to_string()
    } else {
        files
    };

    Ok(format!(
        "## Summary\n- Agent branch: {branch}\n- Base branch: {}\n- Base commit: {}\n\n## Commits\n{commits}\n\n## Changed Files\n{files}\n\n## Adoption Recommendation\nReview this PR, then adopt with `agd bless {branch}` if it should become signed human history.",
        project.default_target,
        base_commit.trim()
    ))
}

fn resolve_branch(project: &Project, branch: Option<&str>, cwd: &Path) -> Result<String> {
    let branch = match branch {
        Some(branch) => branch.to_string(),
        None => git::stdout(cwd, ["branch", "--show-current"])?
            .trim()
            .to_string(),
    };
    if branch.is_empty() || branch == project.default_target || branch == "main" {
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
