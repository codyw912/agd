use crate::git;
use crate::paths::AgdPaths;
use anyhow::{Context, Result};
use serde_json::json;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::Path;

pub(crate) const PROTECTED_BRANCH_HOOK_MESSAGE: &str = "AGD: commits on protected branch";
pub(crate) const PROTECTED_BRANCH_HOOK_HINT: &str =
    "AGD: create an agent branch first: git switch -c agent/<name>";

pub fn install(paths: &AgdPaths, workspace_path: &Path) -> Result<()> {
    let deny_signer = install_deny_signer(paths)?;
    configure_identity(workspace_path)?;
    configure_signing_denial(workspace_path, &deny_signer)?;
    configure_push_denial(workspace_path)?;
    install_pre_commit_hook(workspace_path)?;
    install_pre_push_hook(workspace_path)?;
    Ok(())
}

pub fn deny_signing(paths: &AgdPaths) -> Result<()> {
    eprintln!("AGD denied explicit signing: agent workspaces cannot use signing keys");

    let log_path = paths.home.join("logs/signing-denials.jsonl");
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let event = json!({
        "time": time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string()),
        "cwd": env::current_dir()
            .ok()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        "message": "explicit signing attempt denied",
    });
    let mut line = serde_json::to_string(&event).context("serialize signing denial")?;
    line.push('\n');
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .with_context(|| format!("open {}", log_path.display()))?
        .write_all(line.as_bytes())
        .with_context(|| format!("write {}", log_path.display()))?;

    anyhow::bail!("explicit signing denied")
}

fn configure_identity(workspace_path: &Path) -> Result<()> {
    git::run(workspace_path, ["config", "user.name", "Local Agent"])?;
    git::run(
        workspace_path,
        ["config", "user.email", "agent@agd.invalid"],
    )?;
    git::run(workspace_path, ["config", "commit.gpgsign", "false"])?;
    git::run(workspace_path, ["config", "tag.gpgSign", "false"])?;
    Ok(())
}

fn configure_signing_denial(workspace_path: &Path, deny_signer: &Path) -> Result<()> {
    let _ = git::run(workspace_path, ["config", "--unset-all", "user.signingkey"]);
    git::run(workspace_path, ["config", "gpg.format", "openpgp"])?;
    git_config_path(workspace_path, "gpg.program", deny_signer)?;
    git_config_path(workspace_path, "gpg.ssh.program", deny_signer)?;
    Ok(())
}

fn configure_push_denial(workspace_path: &Path) -> Result<()> {
    git::run(
        workspace_path,
        [
            "remote",
            "set-url",
            "--push",
            "origin",
            "agd-deny://push-disabled",
        ],
    )
}

fn install_pre_commit_hook(workspace_path: &Path) -> Result<()> {
    let hook_path = workspace_path.join(".git/hooks/pre-commit");
    fs::write(
        &hook_path,
        format!(
            "#!/bin/sh\nbranch=$(git symbolic-ref --quiet --short HEAD 2>/dev/null || true)\ncase \"$branch\" in\n  main|master|trunk|develop|stable|production|prod|release/*|stable/*|production/*|prod/*)\n    echo \"{PROTECTED_BRANCH_HOOK_MESSAGE} '$branch' are disabled.\" >&2\n    echo '{PROTECTED_BRANCH_HOOK_HINT}' >&2\n    exit 1\n    ;;\nesac\nexit 0\n"
        ),
    )
    .with_context(|| format!("write {}", hook_path.display()))?;
    make_executable(&hook_path)
}

fn install_pre_push_hook(workspace_path: &Path) -> Result<()> {
    let hook_path = workspace_path.join(".git/hooks/pre-push");
    fs::write(
        &hook_path,
        "#!/bin/sh\necho 'AGD: push is disabled for this agent workspace.' >&2\nexit 1\n",
    )
    .with_context(|| format!("write {}", hook_path.display()))?;
    make_executable(&hook_path)
}

fn install_deny_signer(paths: &AgdPaths) -> Result<std::path::PathBuf> {
    let bin_dir = paths.home.join("bin");
    fs::create_dir_all(&bin_dir).with_context(|| format!("create {}", bin_dir.display()))?;
    let path = bin_dir.join("agd-deny-signer");
    let exe = env::current_exe().context("locate current executable")?;
    fs::write(
        &path,
        format!(
            "#!/bin/sh\nAGD_HOME={} exec {} deny-signer \"$@\"\n",
            shell_quote(&paths.home.display().to_string()),
            shell_quote(&exe.display().to_string())
        ),
    )
    .with_context(|| format!("write {}", path.display()))?;
    make_executable(&path)?;
    Ok(path)
}

fn git_config_path(workspace_path: &Path, key: &str, value: &Path) -> Result<()> {
    git::run(
        workspace_path,
        [
            OsString::from("config"),
            OsString::from(key),
            value.as_os_str().to_owned(),
        ],
    )
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)
        .with_context(|| format!("stat {}", path.display()))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).with_context(|| format!("chmod {}", path.display()))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}
