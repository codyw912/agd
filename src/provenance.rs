use crate::git;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub fn patch_sha256(repo: &Path, base: &str, tip: &str) -> Result<String> {
    let patch = git::stdout_bytes(
        repo,
        [
            "diff",
            "--binary",
            "--full-index",
            "--no-ext-diff",
            base,
            tip,
        ],
    )?;
    Ok(hex_sha256(&patch))
}

pub fn adopted_patch_sha256(repo: &Path, commit: &str) -> Result<String> {
    let parent_ref = format!("{commit}^");
    let parent = git::stdout(repo, ["rev-parse", parent_ref.as_str()])?;
    patch_sha256(repo, parent.trim(), commit)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalProvenanceRecord {
    version: u8,
    project_id: String,
    workspace_id: String,
    agent_branch: String,
    agent_base: String,
    agent_tip: String,
    agent_commit: Option<String>,
    adoption: String,
    human_branch: String,
    human_commit: String,
    patch_sha256: String,
    created_at: String,
}

pub struct LocalProvenanceInput<'a> {
    pub project_id: &'a str,
    pub workspace_id: &'a str,
    pub agent_branch: &'a str,
    pub agent_base: &'a str,
    pub agent_tip: &'a str,
    pub agent_commit: Option<&'a str>,
    pub adoption: &'a str,
    pub human_branch: &'a str,
    pub human_commit: &'a str,
    pub patch_sha256: &'a str,
}

impl LocalProvenanceRecord {
    pub fn new(input: LocalProvenanceInput<'_>) -> Result<Self> {
        let created_at = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .context("format provenance timestamp")?;
        Ok(Self {
            version: 1,
            project_id: input.project_id.to_string(),
            workspace_id: input.workspace_id.to_string(),
            agent_branch: input.agent_branch.to_string(),
            agent_base: input.agent_base.to_string(),
            agent_tip: input.agent_tip.to_string(),
            agent_commit: input.agent_commit.map(str::to_string),
            adoption: input.adoption.to_string(),
            human_branch: input.human_branch.to_string(),
            human_commit: input.human_commit.to_string(),
            patch_sha256: input.patch_sha256.to_string(),
            created_at,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct AgentProvenanceIndex {
    version: u8,
    project_id: String,
    workspace_id: String,
    agent_branch: String,
    agent_base: String,
    agent_tip: String,
    records: Vec<AgentProvenanceIndexRecord>,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentProvenanceIndexRecord {
    human_commit: String,
    agent_commit: Option<String>,
    adoption: String,
    patch_sha256: String,
}

pub fn write_local_record(repo: &Path, record: &LocalProvenanceRecord) -> Result<()> {
    let records_dir = provenance_dir(repo)?.join("records");
    fs::create_dir_all(&records_dir)
        .with_context(|| format!("create {}", records_dir.display()))?;
    let record_path = records_dir.join(format!("{}.json", record.human_commit));
    write_json_atomically(&record_path, record)?;
    update_agent_index(repo, record)
}

fn update_agent_index(repo: &Path, record: &LocalProvenanceRecord) -> Result<()> {
    let index_dir = provenance_dir(repo)?.join("by-agent");
    fs::create_dir_all(&index_dir).with_context(|| format!("create {}", index_dir.display()))?;
    let index_path = index_dir.join(format!("{}.json", record.agent_tip));
    let mut index = match fs::read(&index_path) {
        Ok(contents) => serde_json::from_slice::<AgentProvenanceIndex>(&contents)
            .with_context(|| format!("parse {}", index_path.display()))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => AgentProvenanceIndex {
            version: 1,
            project_id: record.project_id.clone(),
            workspace_id: record.workspace_id.clone(),
            agent_branch: record.agent_branch.clone(),
            agent_base: record.agent_base.clone(),
            agent_tip: record.agent_tip.clone(),
            records: Vec::new(),
            updated_at: record.created_at.clone(),
        },
        Err(error) => return Err(error).with_context(|| format!("read {}", index_path.display())),
    };

    index
        .records
        .retain(|entry| entry.human_commit != record.human_commit);
    index.records.push(AgentProvenanceIndexRecord {
        human_commit: record.human_commit.clone(),
        agent_commit: record.agent_commit.clone(),
        adoption: record.adoption.clone(),
        patch_sha256: record.patch_sha256.clone(),
    });
    index.updated_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .context("format provenance index timestamp")?;
    write_json_atomically(&index_path, &index)
}

fn provenance_dir(repo: &Path) -> Result<PathBuf> {
    Ok(git_dir(repo)?.join("agd/provenance"))
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

fn write_json_atomically<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let tmp_path = path.with_file_name(format!(
        ".{}.tmp.{}",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("provenance"),
        std::process::id()
    ));
    let contents = serde_json::to_vec_pretty(value).context("serialize provenance json")?;
    fs::write(&tmp_path, contents).with_context(|| format!("write {}", tmp_path.display()))?;
    fs::rename(&tmp_path, path)
        .with_context(|| format!("rename {} to {}", tmp_path.display(), path.display()))
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut hex, "{byte:02x}").expect("write to string");
    }
    hex
}
