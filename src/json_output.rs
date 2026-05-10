use crate::git;
use crate::project::{Identity, Project, ProjectContext, Workspace};
use crate::review;
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
struct PathResponse {
    path: String,
}

#[derive(Debug, Serialize)]
struct StatusResponse {
    mode: &'static str,
    project_id: String,
    project: String,
    human_checkout: String,
    workspace: WorkspaceResponse,
    agent_identity: IdentityResponse,
    signing: &'static str,
    push: &'static str,
    default_target: String,
    current_branch: Option<String>,
}

#[derive(Debug, Serialize)]
struct WorkspaceResponse {
    id: String,
    name: String,
    kind: String,
    path: String,
    status: String,
}

#[derive(Debug, Serialize)]
struct IdentityResponse {
    name: String,
    email: String,
    signing: String,
}

#[derive(Debug, Serialize)]
struct BranchesResponse {
    branches: Vec<String>,
}

#[derive(Debug, Serialize)]
struct FilesResponse {
    branch: String,
    files: Vec<String>,
}

#[derive(Debug, Serialize)]
struct WorkspacesResponse {
    workspaces: Vec<WorkspaceResponse>,
}

#[derive(Debug, Serialize)]
struct WorkspaceCreatedResponse {
    workspace: WorkspaceResponse,
}

pub fn path(workspace: &Workspace) -> Result<()> {
    print(&PathResponse {
        path: workspace.path.display().to_string(),
    })
}

pub fn status(context: &ProjectContext, cwd: &Path) -> Result<()> {
    let project = context.project();
    let workspace = default_workspace(project)?;
    let current_branch = git::stdout(cwd, ["branch", "--show-current"])
        .ok()
        .map(|branch| branch.trim().to_string())
        .filter(|branch| !branch.is_empty());

    print(&StatusResponse {
        mode: match context {
            ProjectContext::HumanCheckout(_) => "human_checkout",
            ProjectContext::AgentWorkspace(_) => "agent_workspace",
        },
        project_id: project.project_id.clone(),
        project: project.name.clone(),
        human_checkout: project.human_checkout.display().to_string(),
        workspace: workspace_response(workspace),
        agent_identity: identity_response(&project.agent_identity),
        signing: "disabled",
        push: "denied",
        default_target: project.default_target.clone(),
        current_branch,
    })
}

pub fn branches(branches: Vec<String>) -> Result<()> {
    print(&BranchesResponse { branches })
}

pub fn files(files: review::ChangedFiles) -> Result<()> {
    print(&FilesResponse {
        branch: files.branch,
        files: files.files,
    })
}

pub fn identity(identity: &Identity) -> Result<()> {
    print(&identity_response(identity))
}

pub fn workspaces(project: &Project) -> Result<()> {
    print(&WorkspacesResponse {
        workspaces: project.workspaces.iter().map(workspace_response).collect(),
    })
}

pub fn workspace_created(workspace: &Workspace) -> Result<()> {
    print(&WorkspaceCreatedResponse {
        workspace: workspace_response(workspace),
    })
}

pub fn print<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

pub fn default_workspace(project: &Project) -> Result<&Workspace> {
    project
        .workspaces
        .iter()
        .find(|workspace| workspace.id == project.default_workspace)
        .context("default workspace not found")
}

fn workspace_response(workspace: &Workspace) -> WorkspaceResponse {
    WorkspaceResponse {
        id: workspace.id.clone(),
        name: workspace.name.clone(),
        kind: workspace.kind.clone(),
        path: workspace.path.display().to_string(),
        status: workspace.status.clone(),
    }
}

fn identity_response(identity: &Identity) -> IdentityResponse {
    IdentityResponse {
        name: identity.name.clone(),
        email: identity.email.clone(),
        signing: identity.signing.clone(),
    }
}
