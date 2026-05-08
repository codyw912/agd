use crate::git;
use crate::project::{Project, ProjectContext};
use anyhow::Result;
use std::path::Path;

pub fn status(context: &ProjectContext, cwd: &Path) -> Result<()> {
    let project = context.project();
    match context {
        ProjectContext::HumanCheckout(_) => println!("Mode: human checkout"),
        ProjectContext::AgentWorkspace(_) => println!("Mode: agent workspace"),
    }
    println!("Project: {}", project.name);
    println!("Human checkout:");
    println!("  {}", project.human_checkout.display());
    if let Some(workspace) = default_workspace(project) {
        println!("Agent workspace:");
        println!("  {}", workspace.path.display());
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

fn default_workspace(project: &Project) -> Option<&crate::project::Workspace> {
    project
        .workspaces
        .iter()
        .find(|workspace| workspace.id == project.default_workspace)
}
