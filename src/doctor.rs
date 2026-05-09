use crate::git;
use crate::guardrails;
use crate::paths::AgdPaths;
use crate::project::{self, Project, ProjectContext};
use crate::workspace;
use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

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

pub fn doctor(paths: &AgdPaths, context: &ProjectContext, cwd: &Path) -> Result<()> {
    let report = report(paths, context, cwd);
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

pub fn report(paths: &AgdPaths, context: &ProjectContext, cwd: &Path) -> DoctorReport {
    let checks: Vec<_> = checks(paths, context, cwd)
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

pub fn repair(paths: &AgdPaths, context: &ProjectContext, cwd: &Path) -> Result<()> {
    let ProjectContext::HumanCheckout(project) = context else {
        anyhow::bail!("doctor --repair must be run from the human checkout");
    };

    let current_checkout = git::stdout(cwd, ["rev-parse", "--show-toplevel"])?;
    let current_checkout = PathBuf::from(current_checkout.trim())
        .canonicalize()
        .context("canonicalize current checkout")?;
    let mut project = project.clone();
    let mut repaired = false;

    if !equivalent_path(&project.human_checkout, &current_checkout) {
        project.human_checkout = current_checkout.clone();
        println!(
            "Repaired human checkout path: {}",
            current_checkout.display()
        );
        repaired = true;
    }

    for workspace in &project.workspaces {
        if !workspace.path.exists() {
            continue;
        }
        git::run(
            &workspace.path,
            [
                OsString::from("remote"),
                OsString::from("set-url"),
                OsString::from("origin"),
                current_checkout.as_os_str().to_owned(),
            ],
        )?;
        println!(
            "Repaired workspace origin for {}: {}",
            workspace.id,
            current_checkout.display()
        );
        guardrails::install(paths, &workspace.path)?;
        println!("Repaired workspace guardrails for {}", workspace.id);
        workspace::repair_workspace_marker(&project, workspace)?;
        println!("Repaired workspace marker for {}", workspace.id);
        repaired = true;
    }

    if repair_stale_agd_lock(paths, &project)? {
        repaired = true;
    }

    if repaired {
        project::save_project(paths, &project)?;
    } else {
        println!("No repairs needed");
    }

    Ok(())
}

fn repair_stale_agd_lock(paths: &AgdPaths, project: &Project) -> Result<bool> {
    let lock = paths
        .project_dir(&project.project_id)
        .join("locks/bless.lock");
    if !lock.exists() {
        return Ok(false);
    }

    let contents = fs::read(&lock).with_context(|| format!("read {}", lock.display()))?;
    let metadata: Value =
        serde_json::from_slice(&contents).with_context(|| format!("parse {}", lock.display()))?;
    let Some(pid) = metadata["pid"].as_u64() else {
        return Ok(false);
    };
    if pid_is_alive(pid) {
        return Ok(false);
    }

    fs::remove_file(&lock).with_context(|| format!("remove {}", lock.display()))?;
    println!("Removed stale AGD operation lock: {}", lock.display());
    Ok(true)
}

fn pid_is_alive(pid: u64) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .is_ok_and(|status| status.success())
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

fn checks(paths: &AgdPaths, context: &ProjectContext, cwd: &Path) -> Vec<Check> {
    let project = context.project();
    let mut checks = Vec::new();
    checks.push(path_check(
        "project metadata",
        &paths.project_file(&project.project_id),
    ));
    checks.push(path_check("human checkout exists", &project.human_checkout));
    checks.push(human_checkout_path_check(context, cwd));

    let Some(workspace) = project
        .workspaces
        .iter()
        .find(|workspace| workspace.id == project.default_workspace)
    else {
        checks.push(fail("workspace exists", "default workspace missing"));
        return checks;
    };

    checks.push(path_check("workspace exists", &workspace.path));
    checks.push(workspace_marker_check(project, workspace));
    checks.push(workspace_independence_check(&workspace.path));
    checks.push(workspace_origin_check(project, &workspace.path));
    checks.push(submodule_check(project, &workspace.path));
    checks.push(lfs_check(project, &workspace.path));
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
    checks.push(human_signing_check(
        &project.human_checkout,
        &workspace.path,
    ));
    checks.push(pre_push_hook_check(&workspace.path));
    checks.push(git_state_check("human git state", &project.human_checkout));
    checks.push(git_state_check("workspace git state", &workspace.path));
    checks.push(agd_lock_check(paths, project));
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

fn human_checkout_path_check(context: &ProjectContext, cwd: &Path) -> Check {
    let ProjectContext::HumanCheckout(project) = context else {
        return ok("human checkout path", "");
    };

    match git::stdout(cwd, ["rev-parse", "--show-toplevel"]) {
        Ok(path) => {
            let current_checkout = PathBuf::from(path.trim());
            if equivalent_path(&project.human_checkout, &current_checkout) {
                ok("human checkout path", "")
            } else {
                fail(
                    "human checkout path",
                    format!(
                        "metadata {}, current checkout {}",
                        project.human_checkout.display(),
                        current_checkout.display()
                    ),
                )
            }
        }
        Err(error) => fail("human checkout path", error.to_string()),
    }
}

fn workspace_marker_check(project: &Project, workspace: &project::Workspace) -> Check {
    let marker_path = workspace.path.join(".agd/workspace.json");
    let marker = match fs::read(&marker_path) {
        Ok(marker) => marker,
        Err(error) => return fail("workspace marker", error.to_string()),
    };
    let marker: workspace::WorkspaceMarker = match serde_json::from_slice(&marker) {
        Ok(marker) => marker,
        Err(error) => return fail("workspace marker", error.to_string()),
    };

    if marker.kind != "agd-workspace" {
        return fail("workspace marker", format!("kind {}", marker.kind));
    }
    if marker.project_id != project.project_id {
        return fail(
            "workspace marker",
            format!(
                "project_id expected {}, got {}",
                project.project_id, marker.project_id
            ),
        );
    }
    if marker.workspace_id != workspace.id {
        return fail(
            "workspace marker",
            format!(
                "workspace_id expected {}, got {}",
                workspace.id, marker.workspace_id
            ),
        );
    }
    if !equivalent_path(&project.human_checkout, &marker.human_checkout) {
        return fail(
            "workspace marker",
            format!(
                "human_checkout expected {}, got {}",
                project.human_checkout.display(),
                marker.human_checkout.display()
            ),
        );
    }

    ok("workspace marker", "")
}

fn workspace_independence_check(workspace: &Path) -> Check {
    let alternates = match git::stdout(
        workspace,
        ["rev-parse", "--git-path", "objects/info/alternates"],
    ) {
        Ok(path) => workspace.join(path.trim()),
        Err(error) => return fail("workspace independence", error.to_string()),
    };
    match fs::read_to_string(&alternates) {
        Ok(contents) if contents.trim().is_empty() => ok("workspace independence", ""),
        Ok(_) => fail(
            "workspace independence",
            format!("alternates {}", alternates.display()),
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            ok("workspace independence", "")
        }
        Err(error) => fail("workspace independence", error.to_string()),
    }
}

fn workspace_origin_check(project: &Project, workspace: &Path) -> Check {
    match git::stdout(workspace, ["config", "--get", "remote.origin.url"]) {
        Ok(origin) => {
            let origin = origin.trim();
            let origin_path = PathBuf::from(origin);
            if equivalent_path(&project.human_checkout, &origin_path) {
                ok("workspace origin", "")
            } else {
                fail(
                    "workspace origin",
                    format!(
                        "expected {}, got {origin}",
                        project.human_checkout.display()
                    ),
                )
            }
        }
        Err(error) => fail("workspace origin", error.to_string()),
    }
}

fn human_signing_check(human_checkout: &Path, workspace: &Path) -> Check {
    let human_signer = match git::stdout(human_checkout, ["config", "--get", "gpg.program"]) {
        Ok(signer) => signer,
        Err(_) => return ok("human signing", ""),
    };
    let human_signer = human_signer.trim();
    if human_signer.is_empty() {
        return ok("human signing", "");
    }

    let workspace_signer = match git::stdout(workspace, ["config", "--get", "gpg.program"]) {
        Ok(signer) => signer,
        Err(_) => return ok("human signing", ""),
    };
    if human_signer == workspace_signer.trim() {
        fail(
            "human signing",
            format!("human checkout uses AGD deny signer {human_signer}"),
        )
    } else {
        ok("human signing", "")
    }
}

fn submodule_check(project: &Project, workspace: &Path) -> Check {
    let locations = checkout_files(project, workspace, ".gitmodules");
    let present: Vec<_> = locations
        .into_iter()
        .filter(|(_, path)| path.exists())
        .map(|(label, path)| format!("{label} {}", path.display()))
        .collect();

    if present.is_empty() {
        ok("submodules", "")
    } else {
        warn("submodules", present.join(", "))
    }
}

fn lfs_check(project: &Project, workspace: &Path) -> Check {
    let locations = checkout_files(project, workspace, ".gitattributes");
    let present: Vec<_> = locations
        .into_iter()
        .filter_map(|(label, path)| {
            let contents = fs::read_to_string(&path).ok()?;
            contents
                .contains("filter=lfs")
                .then(|| format!("{label} {} declares filter=lfs", path.display()))
        })
        .collect();

    if present.is_empty() {
        ok("Git LFS", "")
    } else {
        warn(
            "Git LFS",
            format!("{}, {}", present.join(", "), git_lfs_status()),
        )
    }
}

fn checkout_files(project: &Project, workspace: &Path, file: &str) -> Vec<(&'static str, PathBuf)> {
    vec![
        ("human", project.human_checkout.join(file)),
        ("workspace", workspace.join(file)),
    ]
}

fn git_lfs_status() -> &'static str {
    match std::process::Command::new("git")
        .args(["lfs", "version"])
        .output()
    {
        Ok(output) if output.status.success() => "git-lfs available",
        _ => "git-lfs unavailable",
    }
}

fn pre_push_hook_check(workspace: &Path) -> Check {
    let hook = workspace.join(".git/hooks/pre-push");
    if !hook.exists() {
        return fail("pre-push hook", format!("missing {}", hook.display()));
    }

    match fs::read_to_string(&hook) {
        Ok(contents) if contents.contains("AGD: push is disabled for this agent workspace.") => {
            ok("pre-push hook", "")
        }
        Ok(_) => fail("pre-push hook", format!("stale {}", hook.display())),
        Err(error) => fail("pre-push hook", error.to_string()),
    }
}

fn equivalent_path(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
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

fn git_state_check(name: &'static str, repo: &Path) -> Check {
    let states = [
        "MERGE_HEAD",
        "CHERRY_PICK_HEAD",
        "REBASE_HEAD",
        "rebase-merge",
        "rebase-apply",
        "index.lock",
    ];
    let present: Vec<_> = states
        .iter()
        .filter_map(|state| {
            git::stdout(repo, ["rev-parse", "--git-path", state])
                .ok()
                .map(|path| repo.join(path.trim()))
                .filter(|path| path.exists())
                .map(|_| *state)
        })
        .collect();

    if present.is_empty() {
        ok(name, "")
    } else {
        fail(name, present.join(", "))
    }
}

fn agd_lock_check(paths: &AgdPaths, project: &Project) -> Check {
    let lock = paths
        .project_dir(&project.project_id)
        .join("locks/bless.lock");
    if lock.exists() {
        fail("AGD operation lock", lock.display().to_string())
    } else {
        ok("AGD operation lock", "")
    }
}

fn ok(name: &'static str, detail: impl Into<String>) -> Check {
    Check {
        status: CheckStatus::Ok,
        name,
        detail: detail.into(),
    }
}

fn warn(name: &'static str, detail: impl Into<String>) -> Check {
    Check {
        status: CheckStatus::Warn,
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
