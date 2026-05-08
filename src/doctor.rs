use crate::git;
use crate::paths::AgdPaths;
use crate::project::Project;
use anyhow::Result;
use serde::Serialize;
use std::path::Path;

#[derive(Debug)]
struct Check {
    status: CheckStatus,
    name: &'static str,
    detail: String,
}

#[derive(Debug, PartialEq, Eq)]
enum CheckStatus {
    Ok,
    Warn,
    Fail,
}

#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub checks: Vec<DoctorCheck>,
    pub failed: bool,
}

#[derive(Debug, Serialize)]
pub struct DoctorCheck {
    pub status: &'static str,
    pub name: &'static str,
    pub detail: String,
}

pub fn doctor(paths: &AgdPaths, project: &Project) -> Result<()> {
    let report = report(paths, project);
    for check in &report.checks {
        let status = match check.status {
            "ok" => "ok ",
            "warn" => "warn",
            "fail" => "fail",
            _ => unreachable!("unknown doctor check status"),
        };
        if check.detail.is_empty() {
            println!("{status} {}", check.name);
        } else {
            println!("{status} {}: {}", check.name, check.detail);
        }
    }

    ensure_passed(&report)
}

pub fn report(paths: &AgdPaths, project: &Project) -> DoctorReport {
    let checks: Vec<_> = checks(paths, project)
        .into_iter()
        .map(|check| DoctorCheck {
            status: check.status.as_str(),
            name: check.name,
            detail: check.detail,
        })
        .collect();
    let failed = checks.iter().any(|check| check.status == "fail");
    DoctorReport { checks, failed }
}

pub fn ensure_passed(report: &DoctorReport) -> Result<()> {
    if report.failed {
        anyhow::bail!("doctor found failed checks");
    }
    Ok(())
}

impl CheckStatus {
    fn as_str(&self) -> &'static str {
        match self {
            CheckStatus::Ok => "ok",
            CheckStatus::Warn => "warn",
            CheckStatus::Fail => "fail",
        }
    }
}

fn checks(paths: &AgdPaths, project: &Project) -> Vec<Check> {
    let mut checks = Vec::new();
    checks.push(path_check(
        "project metadata",
        &paths.project_file(&project.project_id),
    ));

    let Some(workspace) = project
        .workspaces
        .iter()
        .find(|workspace| workspace.id == project.default_workspace)
    else {
        checks.push(fail("workspace exists", "default workspace missing"));
        return checks;
    };

    checks.push(path_check("workspace exists", &workspace.path));
    checks.push(path_check(
        "workspace marker",
        &workspace.path.join(".agd/workspace.json"),
    ));
    checks.push(config_check(
        "agent identity",
        &workspace.path,
        "user.email",
        "agent@agd.invalid",
    ));
    checks.push(config_check(
        "signing disabled",
        &workspace.path,
        "commit.gpgsign",
        "false",
    ));
    checks.push(non_empty_config_check(
        "deny signer",
        &workspace.path,
        "gpg.program",
    ));
    checks.push(config_check(
        "push disabled",
        &workspace.path,
        "remote.origin.pushurl",
        "agd-deny://push-disabled",
    ));
    checks.push(dirty_check("human dirty", &project.human_checkout));
    checks.push(dirty_check("workspace dirty", &workspace.path));

    checks
}

fn path_check(name: &'static str, path: &Path) -> Check {
    if path.exists() {
        ok(name, "")
    } else {
        fail(name, format!("missing {}", path.display()))
    }
}

fn config_check(name: &'static str, repo: &Path, key: &str, expected: &str) -> Check {
    match git::stdout(repo, ["config", "--get", key]) {
        Ok(value) if value.trim() == expected => ok(name, ""),
        Ok(value) => fail(name, format!("expected {expected}, got {}", value.trim())),
        Err(error) => fail(name, error.to_string()),
    }
}

fn non_empty_config_check(name: &'static str, repo: &Path, key: &str) -> Check {
    match git::stdout(repo, ["config", "--get", key]) {
        Ok(value) if !value.trim().is_empty() => ok(name, ""),
        Ok(_) => fail(name, "empty"),
        Err(error) => fail(name, error.to_string()),
    }
}

fn dirty_check(name: &'static str, repo: &Path) -> Check {
    match git::stdout(repo, ["status", "--porcelain"]) {
        Ok(value) if value.trim().is_empty() => ok(name, ""),
        Ok(_) => Check {
            status: CheckStatus::Warn,
            name,
            detail: "uncommitted changes".to_string(),
        },
        Err(error) => fail(name, error.to_string()),
    }
}

fn ok(name: &'static str, detail: impl Into<String>) -> Check {
    Check {
        status: CheckStatus::Ok,
        name,
        detail: detail.into(),
    }
}

fn fail(name: &'static str, detail: impl Into<String>) -> Check {
    Check {
        status: CheckStatus::Fail,
        name,
        detail: detail.into(),
    }
}
