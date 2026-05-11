use crate::git;
use crate::project::Project;
use crate::provenance;
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
            expected_patch_sha256: None,
            actual_patch_sha256: None,
        });
    }

    if !trailers.contains_key("AGD-Patch-SHA256") {
        return Ok(VerifyReport {
            commit: commit.to_string(),
            status: "missing_patch_hash",
            missing: vec!["AGD-Patch-SHA256".to_string()],
            trailers,
            expected_patch_sha256: None,
            actual_patch_sha256: None,
        });
    }

    let expected_patch_sha256 = trailers
        .get("AGD-Patch-SHA256")
        .expect("patch hash trailer checked")
        .to_string();
    let agent_patch_sha256 = provenance::patch_sha256(
        &project.human_checkout,
        trailers
            .get("AGD-Agent-Base")
            .expect("base trailer checked"),
        trailers.get("AGD-Agent-Tip").expect("tip trailer checked"),
    )?;
    let actual_patch_sha256 = provenance::adopted_patch_sha256(&project.human_checkout, commit)?;
    if expected_patch_sha256 != agent_patch_sha256 || expected_patch_sha256 != actual_patch_sha256 {
        return Ok(VerifyReport {
            commit: commit.to_string(),
            status: "mismatch",
            missing: Vec::new(),
            trailers,
            expected_patch_sha256: Some(expected_patch_sha256),
            actual_patch_sha256: Some(actual_patch_sha256),
        });
    }

    Ok(VerifyReport {
        commit: commit.to_string(),
        status: "verified",
        missing: Vec::new(),
        trailers,
        expected_patch_sha256: Some(expected_patch_sha256),
        actual_patch_sha256: Some(actual_patch_sha256),
    })
}

pub fn ensure_verified(report: &VerifyReport) -> Result<()> {
    match report.status {
        "verified" => Ok(()),
        "missing_metadata" => anyhow::bail!("commit is missing AGD metadata"),
        "missing_patch_hash" => anyhow::bail!("commit is missing AGD patch hash metadata"),
        "mismatch" => anyhow::bail!("AGD patch hash mismatch"),
        _ => anyhow::bail!("unsupported AGD verification status"),
    }
}

fn print_text(report: &VerifyReport) {
    match report.status {
        "verified" => {
            println!("verified AGD patch");
            if let Some(hash) = &report.actual_patch_sha256 {
                println!("AGD-Patch-SHA256: {hash}");
            }
        }
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
        "mismatch" => {
            println!("AGD patch hash mismatch");
            if let Some(expected) = &report.expected_patch_sha256 {
                println!("Expected: {expected}");
            }
            if let Some(actual) = &report.actual_patch_sha256 {
                println!("Actual: {actual}");
            }
        }
        _ => {
            println!("unsupported AGD verification status");
        }
    }
}

#[derive(Debug, Serialize)]
pub struct VerifyReport {
    commit: String,
    status: &'static str,
    missing: Vec<String>,
    trailers: BTreeMap<String, String>,
    expected_patch_sha256: Option<String>,
    actual_patch_sha256: Option<String>,
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
