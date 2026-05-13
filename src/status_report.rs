use crate::project::Project;
use crate::review;
use anyhow::Result;

pub use crate::review::AgentBranchSummary;

pub fn agent_branches(project: &Project) -> Result<Vec<AgentBranchSummary>> {
    review::branch_summaries(project)
}
