use crate::git;
use crate::paths::AgdPaths;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

const DEFAULT_WORKSPACE_ID: &str = "default";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub project_id: String,
    pub name: String,
    pub human_checkout: PathBuf,
    pub default_target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_remote: Option<UpstreamRemote>,
    pub default_workspace: String,
    pub created_at: String,
    pub agent_identity: Identity,
    pub workspaces: Vec<Workspace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamRemote {
    pub name: String,
    pub fetch_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub push_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub name: String,
    pub email: String,
    pub signing: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub path: PathBuf,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanMarker {
    pub kind: String,
    pub project_id: String,
}

#[derive(Debug, Clone)]
pub enum ProjectContext {
    HumanCheckout(Project),
    AgentWorkspace(Project),
}

impl ProjectContext {
    pub fn project(&self) -> &Project {
        match self {
            ProjectContext::HumanCheckout(project) | ProjectContext::AgentWorkspace(project) => {
                project
            }
        }
    }
}

pub fn init_project(paths: &AgdPaths, cwd: &Path) -> Result<Project> {
    let human_checkout = git::stdout(cwd, ["rev-parse", "--show-toplevel"])?;
    let human_checkout = PathBuf::from(human_checkout.trim());
    let git_dir = git::stdout(&human_checkout, ["rev-parse", "--git-dir"])?;
    let git_dir = resolve_git_dir(&human_checkout, git_dir.trim());
    let marker_path = git_dir.join("agd").join("project.json");

    if marker_path.exists() {
        let marker = read_json::<HumanMarker>(&marker_path)?;
        let project = load_project(paths, &marker.project_id)?;
        return Ok(project);
    }

    require_clean_human_checkout(&human_checkout)?;

    let project_id = format!("project_{}", Uuid::new_v4().simple());
    let now = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .context("format timestamp")?;
    let default_target = git::stdout(&human_checkout, ["branch", "--show-current"])?;
    let name = human_checkout
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("project")
        .to_string();
    let upstream_remote = upstream_remote(&human_checkout);

    let project = Project {
        project_id: project_id.clone(),
        name,
        human_checkout,
        default_target: default_target.trim().to_string(),
        upstream_remote,
        default_workspace: DEFAULT_WORKSPACE_ID.to_string(),
        created_at: now,
        agent_identity: Identity {
            name: "Local Agent".to_string(),
            email: "agent@agd.invalid".to_string(),
            signing: "unsigned".to_string(),
        },
        workspaces: Vec::new(),
    };

    save_project(paths, &project)?;
    write_json(
        &marker_path,
        &HumanMarker {
            kind: "agd-project".to_string(),
            project_id,
        },
    )?;

    Ok(project)
}

fn require_clean_human_checkout(human_checkout: &Path) -> Result<()> {
    let status = git::stdout(human_checkout, ["status", "--porcelain"])?;
    if !status.trim().is_empty() {
        anyhow::bail!("human checkout has uncommitted changes");
    }
    Ok(())
}

fn upstream_remote(human_checkout: &Path) -> Option<UpstreamRemote> {
    let fetch_url = git::stdout(human_checkout, ["config", "--get", "remote.origin.url"])
        .ok()
        .map(|url| url.trim().to_string())
        .filter(|url| !url.is_empty())?;
    let push_url = git::stdout(human_checkout, ["config", "--get", "remote.origin.pushurl"])
        .ok()
        .map(|url| url.trim().to_string())
        .filter(|url| !url.is_empty());

    Some(UpstreamRemote {
        name: "origin".to_string(),
        fetch_url,
        push_url,
    })
}

pub fn save_project(paths: &AgdPaths, project: &Project) -> Result<()> {
    write_json(&paths.project_file(&project.project_id), project)
}

pub fn load_project(paths: &AgdPaths, project_id: &str) -> Result<Project> {
    read_json::<Project>(&paths.project_file(project_id))
}

pub fn discover(paths: &AgdPaths, cwd: &Path) -> Result<ProjectContext> {
    if let Ok(human_checkout) = git::stdout(cwd, ["rev-parse", "--show-toplevel"]) {
        let human_checkout = PathBuf::from(human_checkout.trim());
        let git_dir = git::stdout(&human_checkout, ["rev-parse", "--git-dir"])?;
        let marker_path = resolve_git_dir(&human_checkout, git_dir.trim())
            .join("agd")
            .join("project.json");
        if marker_path.exists() {
            let marker = read_json::<HumanMarker>(&marker_path)?;
            return Ok(ProjectContext::HumanCheckout(load_project(
                paths,
                &marker.project_id,
            )?));
        }
    }

    let workspace_marker = find_workspace_marker(cwd).context("not an AGD project or workspace")?;
    let marker = read_json::<crate::workspace::WorkspaceMarker>(&workspace_marker)?;
    Ok(ProjectContext::AgentWorkspace(load_project(
        paths,
        &marker.project_id,
    )?))
}

fn resolve_git_dir(repo: &Path, git_dir: &str) -> PathBuf {
    let path = PathBuf::from(git_dir);
    if path.is_absolute() {
        path
    } else {
        repo.join(path)
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("parse {}", path.display()))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let bytes = serde_json::to_vec_pretty(value).context("serialize json")?;
    fs::write(path, bytes).with_context(|| format!("write {}", path.display()))
}

fn find_workspace_marker(cwd: &Path) -> Option<PathBuf> {
    for ancestor in cwd.ancestors() {
        let marker = ancestor.join(".agd/workspace.json");
        if marker.exists() {
            return Some(marker);
        }
    }
    None
}
