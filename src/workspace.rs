use crate::git;
use crate::guardrails;
use crate::paths::AgdPaths;
use crate::project::{Project, Workspace};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

const DEFAULT_WORKSPACE_ID: &str = "default";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMarker {
    pub kind: String,
    pub project_id: String,
    pub workspace_id: String,
    pub human_checkout: PathBuf,
    pub created_at: String,
}

pub fn ensure_default_workspace(paths: &AgdPaths, project: &mut Project) -> Result<PathBuf> {
    ensure_workspace(paths, project, DEFAULT_WORKSPACE_ID)
}

pub fn ensure_workspace(
    paths: &AgdPaths,
    project: &mut Project,
    workspace_id: &str,
) -> Result<PathBuf> {
    validate_workspace_id(workspace_id)?;
    let workspace_path = workspace_path(paths, project, workspace_id);
    if !workspace_path.exists() {
        fs::create_dir_all(
            workspace_path
                .parent()
                .context("workspace path had no parent")?,
        )
        .with_context(|| format!("create {}", workspace_path.display()))?;
        git::run(
            &project.human_checkout,
            [
                "clone".into(),
                project.human_checkout.as_os_str().to_owned(),
                workspace_path.as_os_str().to_owned(),
            ],
        )?;
    }

    let now = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .context("format timestamp")?;
    write_workspace_marker(project, &workspace_path, workspace_id, &now)?;
    ignore_workspace_metadata(&workspace_path)?;
    guardrails::install(paths, &workspace_path)?;

    if project
        .workspaces
        .iter()
        .all(|workspace| workspace.id != workspace_id)
    {
        project.workspaces.push(Workspace {
            id: workspace_id.to_string(),
            name: workspace_id.to_string(),
            kind: "managed_clone".to_string(),
            path: workspace_path.clone(),
            status: "active".to_string(),
            created_at: now,
        });
    }

    Ok(workspace_path)
}

pub fn repair_workspace_marker(project: &Project, workspace: &Workspace) -> Result<()> {
    write_workspace_marker(
        project,
        &workspace.path,
        &workspace.id,
        &workspace.created_at,
    )
}

fn workspace_path(paths: &AgdPaths, project: &Project, workspace_id: &str) -> PathBuf {
    paths
        .home
        .join("workspaces")
        .join(format!(
            "{}-{}",
            slug(&project.name),
            project_short_id(&project.project_id)
        ))
        .join(workspace_id)
}

fn project_short_id(project_id: &str) -> String {
    project_id
        .strip_prefix("project_")
        .unwrap_or(project_id)
        .chars()
        .take(8)
        .collect()
}

fn slug(name: &str) -> String {
    let mut slug = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_matches('-').to_string()
}

fn write_workspace_marker(
    project: &Project,
    workspace_path: &Path,
    workspace_id: &str,
    created_at: &str,
) -> Result<()> {
    let marker = WorkspaceMarker {
        kind: "agd-workspace".to_string(),
        project_id: project.project_id.clone(),
        workspace_id: workspace_id.to_string(),
        human_checkout: project.human_checkout.clone(),
        created_at: created_at.to_string(),
    };
    let marker_path = workspace_path.join(".agd/workspace.json");
    if let Some(parent) = marker_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    fs::write(
        &marker_path,
        serde_json::to_vec_pretty(&marker).context("serialize workspace marker")?,
    )
    .with_context(|| format!("write {}", marker_path.display()))
}

fn ignore_workspace_metadata(workspace_path: &Path) -> Result<()> {
    let exclude = workspace_path.join(".git/info/exclude");
    let current = fs::read_to_string(&exclude).unwrap_or_default();
    if !current.lines().any(|line| line.trim() == ".agd/") {
        fs::write(&exclude, format!("{current}\n.agd/\n"))
            .with_context(|| format!("write {}", exclude.display()))?;
    }
    Ok(())
}

fn validate_workspace_id(workspace_id: &str) -> Result<()> {
    if workspace_id.is_empty()
        || workspace_id == "."
        || workspace_id == ".."
        || workspace_id.contains('/')
        || workspace_id.contains('\\')
    {
        anyhow::bail!("invalid workspace name: {workspace_id}");
    }
    Ok(())
}
