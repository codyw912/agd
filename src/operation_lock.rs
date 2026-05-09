use crate::paths::AgdPaths;
use anyhow::{Context, Result};
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct OperationLock {
    path: PathBuf,
    operation_id: String,
}

impl OperationLock {
    pub fn acquire(
        paths: &AgdPaths,
        project_id: &str,
        workspace_id: &str,
        operation: &str,
    ) -> Result<Self> {
        let lock_dir = paths.project_dir(project_id).join("locks");
        fs::create_dir_all(&lock_dir).with_context(|| format!("create {}", lock_dir.display()))?;

        if let Some(lock) = existing_lock(&lock_dir)? {
            return Err(lock_error(operation, lock));
        }

        let operation_id = format!("op_{}", Uuid::new_v4().simple());
        let path = lock_dir.join(format!("{operation}.lock"));
        let started_at = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .context("format lock timestamp")?;
        let lock = serde_json::json!({
            "operation_id": operation_id,
            "operation": operation,
            "project_id": project_id,
            "workspace_id": workspace_id,
            "pid": std::process::id(),
            "started_at": started_at,
        });
        let mut lock_file = match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(lock_file) => lock_file,
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                return Err(lock_error(operation, path));
            }
            Err(error) => {
                return Err(error).with_context(|| format!("create lock {}", path.display()));
            }
        };
        lock_file
            .write_all(serde_json::to_string_pretty(&lock)?.as_bytes())
            .with_context(|| format!("write lock {}", path.display()))?;
        Ok(Self { path, operation_id })
    }

    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }
}

impl Drop for OperationLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub fn lock_paths(paths: &AgdPaths, project_id: &str) -> Result<Vec<PathBuf>> {
    let lock_dir = paths.project_dir(project_id).join("locks");
    lock_paths_in(&lock_dir)
}

fn existing_lock(lock_dir: &Path) -> Result<Option<PathBuf>> {
    Ok(lock_paths_in(lock_dir)?.into_iter().next())
}

fn lock_paths_in(lock_dir: &Path) -> Result<Vec<PathBuf>> {
    if !lock_dir.exists() {
        return Ok(Vec::new());
    }

    let mut locks = Vec::new();
    for entry in fs::read_dir(lock_dir).with_context(|| format!("read {}", lock_dir.display()))? {
        let path = entry
            .with_context(|| format!("read {}", lock_dir.display()))?
            .path();
        if path
            .extension()
            .is_some_and(|extension| extension == "lock")
        {
            locks.push(path);
        }
    }
    locks.sort();
    Ok(locks)
}

fn lock_error(operation: &str, path: PathBuf) -> anyhow::Error {
    anyhow::anyhow!(
        "{operation} operation already in progress: {}",
        path.display()
    )
}
