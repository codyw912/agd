use crate::git;
use crate::json_output;
use crate::operation_lock::OperationLock;
use crate::paths::AgdPaths;
use crate::project::{Project, Workspace};
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Debug, Serialize)]
struct HandoffMetadata {
    project_id: String,
    workspace_id: String,
    human_head: String,
    created_at: String,
}

pub fn handoff(paths: &AgdPaths, project: &Project) -> Result<()> {
    let workspace = json_output::default_workspace(project)?;
    require_clean(&workspace.path, "agent workspace")?;
    require_no_untracked_human_files(&project.human_checkout)?;
    let _lock = OperationLock::acquire(paths, &project.project_id, &workspace.id, "handoff")?;

    let diff = git::stdout(&project.human_checkout, ["diff", "--binary", "HEAD"])?;
    if diff.trim().is_empty() {
        println!("No human changes to hand off");
        return Ok(());
    }

    apply_patch(&workspace.path, &diff)?;
    write_metadata(project, workspace)?;

    println!("Handed off human changes to {}", workspace.id);
    Ok(())
}

fn require_clean(repo: &Path, name: &str) -> Result<()> {
    let status = git::stdout(repo, ["status", "--porcelain"])?;
    if !status.trim().is_empty() {
        bail!("{name} has uncommitted changes");
    }
    Ok(())
}

fn require_no_untracked_human_files(repo: &Path) -> Result<()> {
    let status = git::stdout(repo, ["status", "--porcelain"])?;
    if let Some(line) = status.lines().find(|line| line.starts_with("?? ")) {
        bail!(
            "human checkout has untracked files; stage or remove them before handoff: {}",
            &line[3..]
        );
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

fn write_metadata(project: &Project, workspace: &Workspace) -> Result<()> {
    let human_head = git::stdout(&project.human_checkout, ["rev-parse", "HEAD"])?;
    let created_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .context("format timestamp")?;
    let metadata = HandoffMetadata {
        project_id: project.project_id.clone(),
        workspace_id: workspace.id.clone(),
        human_head: human_head.trim().to_string(),
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
