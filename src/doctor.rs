use crate::git;
use crate::guardrails;
use crate::operation_lock::{self, OperationLock};
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
    print_report(&report)
}

pub fn print_report(report: &DoctorReport) -> Result<()> {
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

    ensure_passed(report)
}

pub fn print_warnings(report: &DoctorReport) {
    for check in &report.checks {
        if check.status != "warn" {
            continue;
        }
        if check.detail.is_empty() {
            eprintln!("warn {}", check.name);
        } else {
            eprintln!("warn {}: {}", check.name, check.detail);
        }
    }
}

pub fn metadata_failure_report(paths: &AgdPaths, cwd: &Path) -> Option<DoctorReport> {
    let detail = human_marker_metadata_failure(paths, cwd)
        .or_else(|| workspace_marker_metadata_failure(paths, cwd))?;
    Some(DoctorReport {
        checks: vec![DoctorCheck {
            status: "fail",
            name: "project metadata",
            detail,
        }],
        failed: true,
    })
}

fn human_marker_metadata_failure(paths: &AgdPaths, cwd: &Path) -> Option<String> {
    let human_checkout = git::stdout(cwd, ["rev-parse", "--show-toplevel"]).ok()?;
    let human_checkout = PathBuf::from(human_checkout.trim());
    let git_dir = git::stdout(&human_checkout, ["rev-parse", "--git-dir"]).ok()?;
    let marker_path = resolve_git_dir(&human_checkout, git_dir.trim())
        .join("agd")
        .join("project.json");
    if !marker_path.exists() {
        return None;
    }

    project_metadata_failure_detail(paths, &marker_path)
}

fn workspace_marker_metadata_failure(paths: &AgdPaths, cwd: &Path) -> Option<String> {
    let marker_path = find_workspace_marker(cwd)?;
    project_metadata_failure_detail(paths, &marker_path)
}

fn project_metadata_failure_detail(paths: &AgdPaths, marker_path: &Path) -> Option<String> {
    let marker = match fs::read(marker_path) {
        Ok(bytes) => bytes,
        Err(error) => return Some(format!("read {}: {error}", marker_path.display())),
    };
    let marker: Value = match serde_json::from_slice(&marker) {
        Ok(marker) => marker,
        Err(error) => return Some(format!("parse {}: {error}", marker_path.display())),
    };
    let Some(project_id) = marker["project_id"].as_str() else {
        return Some(format!("missing project_id in {}", marker_path.display()));
    };

    let project_file = paths.project_file(project_id);
    if !project_file.exists() {
        return Some(format!("missing {}", project_file.display()));
    }
    let project = match fs::read(&project_file) {
        Ok(bytes) => bytes,
        Err(error) => return Some(format!("read {}: {error}", project_file.display())),
    };
    serde_json::from_slice::<Project>(&project)
        .err()
        .map(|error| format!("parse {}: {error}", project_file.display()))
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
    let mut repaired = repair_stale_agd_locks(paths, &project)?;
    let _lock = OperationLock::acquire(
        paths,
        &project.project_id,
        &project.default_workspace,
        "repair",
    )?;

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

    if repaired {
        project::save_project(paths, &project)?;
    } else {
        println!("No repairs needed");
    }

    Ok(())
}

fn repair_stale_agd_locks(paths: &AgdPaths, project: &Project) -> Result<bool> {
    let mut repaired = false;
    for lock in operation_lock::lock_paths(paths, &project.project_id)? {
        let contents = fs::read(&lock).with_context(|| format!("read {}", lock.display()))?;
        let metadata: Value = serde_json::from_slice(&contents)
            .with_context(|| format!("parse {}", lock.display()))?;
        let Some(pid) = metadata["pid"].as_u64() else {
            continue;
        };
        if pid_is_alive(pid) {
            continue;
        }

        fs::remove_file(&lock).with_context(|| format!("remove {}", lock.display()))?;
        println!("Removed stale AGD operation lock: {}", lock.display());
        repaired = true;
    }
    Ok(repaired)
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
    checks.push(pre_commit_hook_check(&workspace.path));
    checks.push(pre_push_hook_check(&workspace.path));
    checks.push(git_state_check("human git state", &project.human_checkout));
    checks.push(git_state_check("workspace git state", &workspace.path));
    checks.push(agd_bless_state_check(&project.human_checkout));
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

fn resolve_git_dir(repo: &Path, git_dir: &str) -> PathBuf {
    let path = PathBuf::from(git_dir);
    if path.is_absolute() {
        path
    } else {
        repo.join(path)
    }
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
    let mut present: Vec<_> = locations
        .into_iter()
        .filter_map(|(label, path)| {
            let contents = fs::read_to_string(&path).ok()?;
            contents
                .contains("filter=lfs")
                .then(|| format!("{label} {} declares filter=lfs", path.display()))
        })
        .collect();
    present.extend(lfs_pointer_files("human", &project.human_checkout));
    present.extend(lfs_pointer_files("workspace", workspace));

    if present.is_empty() {
        ok("Git LFS", "")
    } else {
        warn(
            "Git LFS",
            format!("{}, {}", present.join(", "), git_lfs_status()),
        )
    }
}

fn lfs_pointer_files(label: &'static str, repo: &Path) -> Vec<String> {
    let Ok(files) = git::stdout(repo, ["ls-files"]) else {
        return Vec::new();
    };
    files
        .lines()
        .filter(|file| {
            fs::read_to_string(repo.join(file)).is_ok_and(|contents| is_lfs_pointer(&contents))
        })
        .map(|file| format!("{label} {file} looks like Git LFS pointer"))
        .collect()
}

fn is_lfs_pointer(contents: &str) -> bool {
    let mut lines = contents.lines();
    if lines.next() != Some("version https://git-lfs.github.com/spec/v1") {
        return false;
    }

    let mut has_oid = false;
    let mut has_size = false;
    for line in lines {
        if let Some(oid) = line.strip_prefix("oid sha256:") {
            if has_oid || oid.len() != 64 || !oid.chars().all(|c| c.is_ascii_hexdigit()) {
                return false;
            }
            has_oid = true;
        } else if let Some(size) = line.strip_prefix("size ") {
            if has_size || size.is_empty() || !size.chars().all(|c| c.is_ascii_digit()) {
                return false;
            }
            has_size = true;
        } else {
            return false;
        }
    }

    has_oid && has_size
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

fn pre_commit_hook_check(workspace: &Path) -> Check {
    let hook = workspace.join(".git/hooks/pre-commit");
    if !hook.exists() {
        return fail(
            "protected branch hook",
            format!("missing {}", hook.display()),
        );
    }

    match fs::read_to_string(&hook) {
        Ok(contents) if protected_branch_hook_is_current(&contents) => {
            ok("protected branch hook", "")
        }
        Ok(_) => fail("protected branch hook", format!("stale {}", hook.display())),
        Err(error) => fail("protected branch hook", error.to_string()),
    }
}

fn protected_branch_hook_is_current(contents: &str) -> bool {
    contents.contains(guardrails::PROTECTED_BRANCH_HOOK_MESSAGE)
        && contents.contains(guardrails::PROTECTED_BRANCH_HOOK_HINT)
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

fn agd_bless_state_check(repo: &Path) -> Check {
    let state_path = match git::stdout(repo, ["rev-parse", "--git-path", "agd/bless.json"]) {
        Ok(path) => repo.join(path.trim()),
        Err(error) => return fail("AGD bless state", error.to_string()),
    };
    if state_path.exists() {
        fail(
            "AGD bless state",
            "run `agd bless --continue` after resolving conflicts, or `agd bless --abort`",
        )
    } else {
        ok("AGD bless state", "")
    }
}

fn agd_lock_check(paths: &AgdPaths, project: &Project) -> Check {
    match operation_lock::lock_paths(paths, &project.project_id) {
        Ok(locks) if locks.is_empty() => ok("AGD operation lock", ""),
        Ok(locks) => fail(
            "AGD operation lock",
            locks
                .iter()
                .map(|lock| lock.display().to_string())
                .collect::<Vec<_>>()
                .join(", "),
        ),
        Err(error) => fail("AGD operation lock", error.to_string()),
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
