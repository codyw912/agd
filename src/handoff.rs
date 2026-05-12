use crate::git;
use crate::json_output;
use crate::operation_lock::OperationLock;
use crate::paths::AgdPaths;
use crate::project::{Project, Workspace};
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Debug, Serialize)]
struct HandoffMetadata {
    project_id: String,
    workspace_id: String,
    status: &'static str,
    human_head: String,
    tracked_files: Vec<String>,
    untracked_files: Vec<String>,
    created_at: String,
}

#[derive(Debug, Serialize)]
pub struct HandoffResult {
    pub status: &'static str,
    pub workspace_id: String,
    pub human_head: String,
    pub tracked_files: Vec<String>,
    pub untracked_files: Vec<String>,
}

pub fn handoff(
    paths: &AgdPaths,
    project: &Project,
    include_untracked: &[PathBuf],
) -> Result<HandoffResult> {
    let workspace = json_output::default_workspace(project)?;
    require_clean(&workspace.path, "agent workspace")?;
    require_no_untracked_human_files(&project.human_checkout, include_untracked)?;
    let _lock = OperationLock::acquire(paths, &project.project_id, &workspace.id, "handoff")?;

    let diff = git::stdout(&project.human_checkout, ["diff", "--binary", "HEAD"])?;
    let human_head = git::stdout(&project.human_checkout, ["rev-parse", "HEAD"])?;
    let tracked_files = tracked_files(&project.human_checkout)?;
    let untracked_files = selected_untracked_files(include_untracked);
    if diff.trim().is_empty() && include_untracked.is_empty() {
        return Ok(HandoffResult {
            status: "noop",
            workspace_id: workspace.id.clone(),
            human_head: human_head.trim().to_string(),
            tracked_files,
            untracked_files,
        });
    }

    if !diff.trim().is_empty() {
        apply_patch(&workspace.path, &diff)?;
    }
    copy_untracked_files(&project.human_checkout, &workspace.path, include_untracked)?;
    write_metadata(
        project,
        workspace,
        "applied",
        &human_head,
        &tracked_files,
        &untracked_files,
    )?;

    Ok(HandoffResult {
        status: "applied",
        workspace_id: workspace.id.clone(),
        human_head: human_head.trim().to_string(),
        tracked_files,
        untracked_files,
    })
}

fn require_clean(repo: &Path, name: &str) -> Result<()> {
    let status = git::stdout(repo, ["status", "--porcelain"])?;
    if !status.trim().is_empty() {
        bail!("{name} has uncommitted changes");
    }
    Ok(())
}

fn require_no_untracked_human_files(repo: &Path, include_untracked: &[PathBuf]) -> Result<()> {
    let status = git::stdout(repo, ["status", "--porcelain"])?;
    if !include_untracked.is_empty() {
        return Ok(());
    }
    if let Some(line) = status.lines().find(|line| line.starts_with("?? ")) {
        bail!(
            "human checkout has untracked files; stage or remove them before handoff: {}",
            &line[3..]
        );
    }
    Ok(())
}

fn tracked_files(repo: &Path) -> Result<Vec<String>> {
    let files = git::stdout(repo, ["diff", "--name-only", "HEAD"])?;
    Ok(files.lines().map(str::to_string).collect())
}

fn selected_untracked_files(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect()
}

fn copy_untracked_files(human_checkout: &Path, workspace: &Path, paths: &[PathBuf]) -> Result<()> {
    for path in paths {
        let relative_path = safe_relative_path(path)?;
        require_untracked_file(human_checkout, &relative_path)?;
        let source = human_checkout.join(&relative_path);
        let destination = workspace.join(&relative_path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        fs::copy(&source, &destination)
            .with_context(|| format!("copy {} to {}", source.display(), destination.display()))?;
    }
    Ok(())
}

fn safe_relative_path(path: &Path) -> Result<PathBuf> {
    if path.components().any(|component| {
        matches!(
            component,
            Component::Prefix(_) | Component::RootDir | Component::ParentDir
        )
    }) {
        bail!("untracked handoff path must be relative and stay within the checkout");
    }
    Ok(path.to_path_buf())
}

fn require_untracked_file(repo: &Path, path: &Path) -> Result<()> {
    let status = Command::new("git")
        .args(["status", "--porcelain", "--"])
        .arg(path)
        .current_dir(repo)
        .output()
        .context("run git status")?;
    if !status.status.success() {
        bail!(
            "git failed: {}",
            String::from_utf8_lossy(&status.stderr).trim()
        );
    }
    let status = String::from_utf8(status.stdout).context("git status output was not utf-8")?;
    if !status.lines().any(|line| line.starts_with("?? ")) {
        bail!(
            "untracked handoff path is not an untracked file: {}",
            path.display()
        );
    }
    if repo.join(path).is_dir() {
        bail!("untracked handoff path must be a file: {}", path.display());
    }
    Ok(())
}

fn apply_patch(repo: &Path, diff: &str) -> Result<()> {
    let mut child = Command::new("git")
        .args(["apply", "--binary", "-"])
        .current_dir(repo)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("start git apply")?;

    child
        .stdin
        .as_mut()
        .context("open git apply stdin")?
        .write_all(diff.as_bytes())
        .context("write patch to git apply")?;

    let output = child.wait_with_output().context("wait for git apply")?;
    if !output.status.success() {
        bail!(
            "git apply failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(())
}

fn write_metadata(
    project: &Project,
    workspace: &Workspace,
    status: &'static str,
    human_head: &str,
    tracked_files: &[String],
    untracked_files: &[String],
) -> Result<()> {
    let created_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .context("format timestamp")?;
    let metadata = HandoffMetadata {
        project_id: project.project_id.clone(),
        workspace_id: workspace.id.clone(),
        status,
        human_head: human_head.trim().to_string(),
        tracked_files: tracked_files.to_vec(),
        untracked_files: untracked_files.to_vec(),
        created_at,
    };
    let path = git_dir(&workspace.path)?.join("agd").join("handoff.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let bytes = serde_json::to_vec_pretty(&metadata).context("serialize handoff metadata")?;
    fs::write(&path, bytes).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn git_dir(repo: &Path) -> Result<PathBuf> {
    let git_dir = git::stdout(repo, ["rev-parse", "--git-dir"])?;
    let path = PathBuf::from(git_dir.trim());
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(repo.join(path))
    }
}
