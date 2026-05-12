use crate::git;
use crate::project::Project;
use crate::review;
use anyhow::{Context, Result};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AgentBranchSummary {
    pub branch: String,
    pub commits: usize,
    pub files_changed: usize,
}

pub fn agent_branches(project: &Project) -> Result<Vec<AgentBranchSummary>> {
    let workspace = project
        .workspaces
        .iter()
        .find(|workspace| workspace.id == project.default_workspace)
        .context("default workspace not found")?;

    review::branch_names(project)?
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
