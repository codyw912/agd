use crate::project::Project;
use crate::review;
use anyhow::Result;

pub use crate::review::AgentBranchSummary;

pub fn agent_branches(
    project: &Project,
    workspace_id: Option<&str>,
) -> Result<Vec<AgentBranchSummary>> {
    review::branch_summaries(project, workspace_id)
}
