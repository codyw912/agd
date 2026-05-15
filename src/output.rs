use crate::git;
use crate::project::{Project, ProjectContext, Workspace};
use crate::status_report;
use anyhow::Result;
use std::path::Path;

pub fn status(context: &ProjectContext, workspace_id: Option<&str>, cwd: &Path) -> Result<()> {
    let project = context.project();
    match context {
        ProjectContext::HumanCheckout(_) => println!("Mode: human checkout"),
        ProjectContext::AgentWorkspace { .. } => println!("Mode: agent workspace"),
    }
    println!("Project: {}", project.name);
    println!("Human checkout:");
    println!("  {}", project.human_checkout.display());
    let workspace = find_workspace(project, workspace_id);
    if let Some(workspace) = workspace {
        println!("Agent workspace:");
        println!("  {}", workspace.path.display());
    }
    let agent_branches = status_report::agent_branches(project, workspace_id)?;
    if !agent_branches.is_empty() {
        println!("Pending agent branches:");
        for branch in agent_branches {
            println!(
                "  {}  {}  {}",
                branch.branch,
                plural(branch.commits, "commit", "commits"),
                plural(branch.files_changed, "file changed", "files changed")
            );
        }
    }
    println!("Agent identity:");
    println!(
        "  {} <{}>",
        project.agent_identity.name, project.agent_identity.email
    );
    println!("Signing:");
    println!("  disabled");
    println!("Push:");
    println!("  denied");
    println!("Default target:");
    println!("  {}", project.default_target);
    if let Ok(branch) = git::stdout(cwd, ["branch", "--show-current"]) {
        let branch = branch.trim();
        if !branch.is_empty() {
            println!("Current branch:");
            println!("  {branch}");
        }
    }
    Ok(())
}

fn plural(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("{count} {singular}")
    } else {
        format!("{count} {plural}")
    }
}

fn find_workspace<'a>(project: &'a Project, workspace_id: Option<&str>) -> Option<&'a Workspace> {
    let workspace_id = workspace_id.unwrap_or(&project.default_workspace);
    project
        .workspaces
        .iter()
        .find(|workspace| workspace.id == workspace_id)
}
