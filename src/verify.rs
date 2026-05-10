use crate::git;
use crate::project::Project;
use anyhow::Result;
use serde::Serialize;
use std::collections::BTreeMap;

const REQUIRED_TRAILERS: [&str; 6] = [
    "AGD-Project",
    "AGD-Workspace",
    "AGD-Agent-Branch",
    "AGD-Agent-Base",
    "AGD-Agent-Tip",
    "AGD-Adoption",
];

pub fn verify(project: &Project, commit: &str) -> Result<()> {
    let report = report(project, commit)?;
    print_text(&report);
    ensure_verified(&report)
}

pub fn report(project: &Project, commit: &str) -> Result<VerifyReport> {
    let message = git::stdout(
        &project.human_checkout,
        ["log", "-1", "--format=%B", commit],
    )?;
    let trailers = parse_trailers(&message);
    let missing: Vec<_> = REQUIRED_TRAILERS
        .iter()
        .copied()
        .filter(|key| !trailers.contains_key(*key))
        .map(str::to_string)
        .collect();

    if !missing.is_empty() {
        return Ok(VerifyReport {
            commit: commit.to_string(),
            status: "missing_metadata",
            missing,
            trailers,
        });
    }

    if !trailers.contains_key("AGD-Patch-SHA256") {
        return Ok(VerifyReport {
            commit: commit.to_string(),
            status: "missing_patch_hash",
            missing: vec!["AGD-Patch-SHA256".to_string()],
            trailers,
        });
    }

    Ok(VerifyReport {
        commit: commit.to_string(),
        status: "unsupported",
        missing: Vec::new(),
        trailers,
    })
}

pub fn ensure_verified(report: &VerifyReport) -> Result<()> {
    match report.status {
        "verified" => Ok(()),
        "missing_metadata" => anyhow::bail!("commit is missing AGD metadata"),
        "missing_patch_hash" => anyhow::bail!("commit is missing AGD patch hash metadata"),
        _ => anyhow::bail!("AGD patch verification is not implemented yet"),
    }
}

fn print_text(report: &VerifyReport) {
    match report.status {
        "missing_metadata" => {
            println!("missing AGD metadata: {}", report.missing.join(", "));
        }
        "missing_patch_hash" => {
            println!("AGD metadata found");
            if let Some(project) = report.trailers.get("AGD-Project") {
                println!("Project: {project}");
            }
            if let Some(workspace) = report.trailers.get("AGD-Workspace") {
                println!("Workspace: {workspace}");
            }
            if let Some(branch) = report.trailers.get("AGD-Agent-Branch") {
                println!("Agent branch: {branch}");
            }
            println!("missing AGD-Patch-SHA256");
        }
        _ => {
            println!("patch hash metadata present; stable verification is not implemented yet");
        }
    }
}

#[derive(Debug, Serialize)]
pub struct VerifyReport {
    commit: String,
    status: &'static str,
    missing: Vec<String>,
    trailers: BTreeMap<String, String>,
}

fn parse_trailers(message: &str) -> BTreeMap<String, String> {
    message
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            let key = key.trim();
            if !key.starts_with("AGD-") {
                return None;
            }
            Some((key.to_string(), value.trim().to_string()))
        })
        .collect()
}
