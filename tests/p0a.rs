use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::fs;
use std::process::Command as StdCommand;
use tempfile::TempDir;

struct Fixture {
    _tmp: TempDir,
    human: std::path::PathBuf,
    agd_home: std::path::PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let tmp = tempfile::tempdir().expect("tempdir");
        let human = tmp.path().join("human");
        let agd_home = tmp.path().join("agd-home");
        fs::create_dir_all(&human).expect("human dir");
        fs::create_dir_all(&agd_home).expect("agd home");
        Self {
            _tmp: tmp,
            human,
            agd_home,
        }
    }

    fn agd(&self) -> Command {
        let mut cmd = Command::cargo_bin("agd").expect("binary exists");
        cmd.env("AGD_HOME", &self.agd_home);
        cmd
    }

    fn init_human_repo(&self) {
        self.git(["init", "-b", "main"]);
        self.git(["config", "user.name", "Human Developer"]);
        self.git(["config", "user.email", "human@example.test"]);
        self.git(["config", "commit.gpgsign", "false"]);
        fs::write(self.human.join("README.md"), "# test\n").expect("write readme");
        self.git(["add", "README.md"]);
        self.git(["commit", "-m", "initial"]);
    }

    fn agd_path(&self) -> std::path::PathBuf {
        let output = self
            .agd()
            .arg("path")
            .current_dir(&self.human)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        std::path::PathBuf::from(String::from_utf8(output).unwrap().trim())
    }

    fn agd_json<const N: usize>(&self, args: [&str; N], cwd: &std::path::Path) -> Value {
        let output = self
            .agd()
            .args(args)
            .current_dir(cwd)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        serde_json::from_slice(&output).expect("valid json")
    }

    fn project_id(&self) -> String {
        let marker = fs::read(self.human.join(".git/agd/project.json")).expect("read marker");
        let marker: Value = serde_json::from_slice(&marker).expect("parse marker");
        marker["project_id"]
            .as_str()
            .expect("project id")
            .to_string()
    }

    fn lock_dir(&self) -> std::path::PathBuf {
        self.agd_home
            .join("projects")
            .join(self.project_id())
            .join("locks")
    }

    fn write_operation_lock(&self, operation: &str) -> std::path::PathBuf {
        let lock_dir = self.lock_dir();
        fs::create_dir_all(&lock_dir).expect("create lock dir");
        let lock_path = lock_dir.join(format!("{operation}.lock"));
        fs::write(
            &lock_path,
            format!(
                r#"{{
  "operation_id": "op_existing",
  "operation": "{operation}",
  "project_id": "{}",
  "workspace_id": "default",
  "pid": {},
  "started_at": "2026-05-09T00:00:00Z"
}}
"#,
                self.project_id(),
                std::process::id()
            ),
        )
        .expect("write operation lock");
        lock_path
    }

    fn write_file(&self, repo: &std::path::Path, path: &str, contents: &str) {
        let path = repo.join(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(path, contents).expect("write file");
    }

    fn configure_fake_human_signer(&self) {
        let signer = self._tmp.path().join("fake-gpg");
        fs::write(
            &signer,
            "#!/bin/sh\ncat >/dev/null\necho '[GNUPG:] SIG_CREATED D 1 10 00 0 0 0 0' >&2\nprintf '%s\n' '-----BEGIN PGP SIGNATURE-----' '' 'fake' '-----END PGP SIGNATURE-----'\n",
        )
        .expect("write fake signer");
        make_executable(&signer);
        let signer = signer.to_str().expect("utf-8 signer");
        self.git(["config", "gpg.format", "openpgp"]);
        self.git(["config", "commit.gpgsign", "true"]);
        self.git(["config", "user.signingkey", "human@example.test"]);
        self.git(["config", "gpg.program", signer]);
    }

    fn configure_lock_capturing_human_signer(&self) {
        let signer = self._tmp.path().join("capture-lock-gpg");
        fs::write(
            &signer,
            "#!/bin/sh\ncat >/dev/null\ncp \"$AGD_TEST_LOCK_PATH\" \"$AGD_TEST_CAPTURE_PATH\"\necho '[GNUPG:] SIG_CREATED D 1 10 00 0 0 0 0' >&2\nprintf '%s\n' '-----BEGIN PGP SIGNATURE-----' '' 'fake' '-----END PGP SIGNATURE-----'\n",
        )
        .expect("write lock capturing signer");
        make_executable(&signer);
        let signer = signer.to_str().expect("utf-8 signer");
        self.git(["config", "gpg.format", "openpgp"]);
        self.git(["config", "commit.gpgsign", "true"]);
        self.git(["config", "user.signingkey", "human@example.test"]);
        self.git(["config", "gpg.program", signer]);
    }

    fn fake_gh_path(&self, capture_path: &std::path::Path) -> std::ffi::OsString {
        let bin_dir = self._tmp.path().join("fake-bin");
        fs::create_dir_all(&bin_dir).expect("create fake bin");
        let gh = bin_dir.join("gh");
        fs::write(
            &gh,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\nprintf '%s\\n' 'https://example.test/pr/1'\n",
                capture_path.display()
            ),
        )
        .expect("write fake gh");
        make_executable(&gh);
        let current_path = std::env::var_os("PATH").expect("PATH set");
        let mut paths = std::env::split_paths(&current_path).collect::<Vec<_>>();
        paths.insert(0, bin_dir);
        std::env::join_paths(paths).expect("join PATH")
    }

    fn fake_glab_path(&self, capture_path: &std::path::Path) -> std::ffi::OsString {
        let bin_dir = self._tmp.path().join("fake-glab-bin");
        fs::create_dir_all(&bin_dir).expect("create fake glab bin");
        let git = find_executable("git");
        #[cfg(unix)]
        std::os::unix::fs::symlink(git, bin_dir.join("git")).expect("symlink git");
        let glab = bin_dir.join("glab");
        fs::write(
            &glab,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\nprintf '%s\\n' 'https://gitlab.example.test/mr/1'\n",
                capture_path.display()
            ),
        )
        .expect("write fake glab");
        make_executable(&glab);
        std::env::join_paths([bin_dir]).expect("join PATH")
    }

    fn fake_gh_and_glab_path(
        &self,
        gh_capture_path: &std::path::Path,
        glab_capture_path: &std::path::Path,
    ) -> std::ffi::OsString {
        let bin_dir = self._tmp.path().join("fake-gh-glab-bin");
        fs::create_dir_all(&bin_dir).expect("create fake gh glab bin");
        let git = find_executable("git");
        #[cfg(unix)]
        std::os::unix::fs::symlink(git, bin_dir.join("git")).expect("symlink git");
        let gh = bin_dir.join("gh");
        fs::write(
            &gh,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\nprintf '%s\\n' 'https://github.example.test/pr/1'\n",
                gh_capture_path.display()
            ),
        )
        .expect("write fake gh");
        make_executable(&gh);
        let glab = bin_dir.join("glab");
        fs::write(
            &glab,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\nprintf '%s\\n' 'https://gitlab.example.test/mr/1'\n",
                glab_capture_path.display()
            ),
        )
        .expect("write fake glab");
        make_executable(&glab);
        std::env::join_paths([bin_dir]).expect("join PATH")
    }

    fn git_only_path(&self) -> std::ffi::OsString {
        let bin_dir = self._tmp.path().join("git-only-bin");
        fs::create_dir_all(&bin_dir).expect("create git-only bin");
        let git = find_executable("git");
        #[cfg(unix)]
        std::os::unix::fs::symlink(git, bin_dir.join("git")).expect("symlink git");
        std::env::join_paths([bin_dir]).expect("join PATH")
    }

    fn git<const N: usize>(&self, args: [&str; N]) {
        self.git_in(&self.human, args);
    }

    fn git_in<const N: usize>(&self, repo: &std::path::Path, args: [&str; N]) {
        let output = StdCommand::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "git failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn git_stdout<const N: usize>(&self, repo: &std::path::Path, args: [&str; N]) -> String {
        let output = StdCommand::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "git failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("utf-8 stdout")
    }

    fn git_stdout_bytes<const N: usize>(&self, repo: &std::path::Path, args: [&str; N]) -> Vec<u8> {
        let output = StdCommand::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "git failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    }

    fn git_fails<const N: usize>(&self, repo: &std::path::Path, args: [&str; N]) -> String {
        let output = StdCommand::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .expect("git runs");
        assert!(
            !output.status.success(),
            "git unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }
}

#[cfg(unix)]
fn make_executable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path).expect("stat").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("chmod");
}

#[cfg(not(unix))]
fn make_executable(_path: &std::path::Path) {}

fn find_executable(name: &str) -> std::path::PathBuf {
    let path = std::env::var_os("PATH").expect("PATH set");
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| panic!("{name} not found in PATH"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut hex, "{byte:02x}").expect("write to string");
    }
    hex
}

#[test]
fn agd_reports_help() {
    let mut cmd = Command::cargo_bin("agd").expect("binary exists");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Agent Git Delegation"));
}

#[test]
fn init_writes_project_metadata_and_human_marker() {
    let fixture = Fixture::new();
    fixture.init_human_repo();

    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized AGD."));

    assert!(fixture.agd_home.join("projects").exists());
    assert!(fixture.human.join(".git/agd/project.json").exists());
}

#[test]
fn init_refuses_dirty_human_checkout() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.write_file(&fixture.human, "dirty.txt", "not committed\n");

    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "human checkout has uncommitted changes",
        ));

    assert!(!fixture.human.join(".git/agd/project.json").exists());
    assert!(!fixture.agd_home.join("projects").exists());
}

#[test]
fn init_warns_when_submodules_are_declared() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.write_file(
        &fixture.human,
        ".gitmodules",
        "[submodule \"vendor/lib\"]\n\tpath = vendor/lib\n\turl = https://example.invalid/lib.git\n",
    );
    fixture.git(["add", ".gitmodules"]);
    fixture.git(["commit", "-m", "declare submodule"]);

    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized AGD."))
        .stderr(predicate::str::contains("warn submodules"))
        .stderr(predicate::str::contains(".gitmodules"));
}

#[test]
fn json_init_outputs_project_and_workspace_metadata() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    let human_checkout = fixture.human.canonicalize().expect("canonical human");
    let human_checkout = human_checkout.to_str().expect("human checkout utf-8");

    let response = fixture.agd_json(["--json", "init"], &fixture.human);
    assert!(response["project_id"]
        .as_str()
        .is_some_and(|project_id| project_id.starts_with("project_")));
    assert_eq!(response["human_checkout"], human_checkout);
    assert_eq!(response["workspace"]["id"], "default");
    assert_eq!(response["workspace"]["status"], "active");
    let workspace = response["workspace"]["path"]
        .as_str()
        .expect("workspace path");
    assert!(std::path::Path::new(workspace).join(".git").exists());
}

#[test]
fn init_records_upstream_remote_metadata() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    let remote = fixture._tmp.path().join("origin.git");
    let remote = remote.to_str().expect("remote path utf-8");
    fixture.git(["init", "--bare", remote]);
    fixture.git([
        "remote",
        "add",
        "origin",
        "git@github.com:example/project.git",
    ]);
    fixture.git(["remote", "set-url", "--push", "origin", remote]);

    let response = fixture.agd_json(["--json", "init"], &fixture.human);
    assert_eq!(response["upstream_remote"]["name"], "origin");
    assert_eq!(
        response["upstream_remote"]["fetch_url"],
        "git@github.com:example/project.git"
    );
    assert_eq!(response["upstream_remote"]["push_url"], remote);

    let project_path = fixture
        .agd_home
        .join("projects")
        .join(response["project_id"].as_str().expect("project id"))
        .join("project.json");
    let project = fs::read(project_path).expect("read project metadata");
    let project: Value = serde_json::from_slice(&project).expect("parse project metadata");
    assert_eq!(
        project["upstream_remote"]["fetch_url"],
        "git@github.com:example/project.git"
    );
    assert_eq!(project["upstream_remote"]["push_url"], remote);
}

#[test]
fn json_init_warns_on_lfs_without_polluting_stdout() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.write_file(
        &fixture.human,
        ".gitattributes",
        "*.bin filter=lfs diff=lfs merge=lfs -text\n",
    );
    fixture.git(["add", ".gitattributes"]);
    fixture.git(["commit", "-m", "declare lfs filters"]);

    let output = fixture
        .agd()
        .args(["--json", "init"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stderr(predicate::str::contains("warn Git LFS"))
        .stderr(predicate::str::contains("filter=lfs"))
        .get_output()
        .stdout
        .clone();
    let response: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(response["workspace"]["id"], "default");
}

#[test]
fn init_creates_managed_clone_and_path_returns_it() {
    let fixture = Fixture::new();
    fixture.init_human_repo();

    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();

    let path_output = fixture
        .agd()
        .arg("path")
        .current_dir(&fixture.human)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let workspace = std::path::PathBuf::from(String::from_utf8(path_output).unwrap().trim());

    assert!(workspace.join(".git").exists());
    assert!(workspace.join(".agd/workspace.json").exists());
}

#[test]
fn agent_commit_uses_local_agent_identity_and_unsigned_commit() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/identity"]);
    fixture.write_file(&workspace, "agent.txt", "agent work\n");
    fixture.git_in(&workspace, ["add", "agent.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent work"]);

    let author = fixture.git_stdout(&workspace, ["log", "-1", "--format=%an <%ae>"]);
    assert_eq!(author.trim(), "Local Agent <agent@agd.invalid>");
}

#[test]
fn explicit_agent_signing_is_denied_and_logged() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/signing-denied"]);
    fixture.write_file(&workspace, "signed.txt", "signed work\n");
    fixture.git_in(&workspace, ["add", "signed.txt"]);
    let output = fixture.git_fails(&workspace, ["commit", "-S", "-m", "signed work"]);

    assert!(output.contains("AGD denied explicit signing"));
    assert!(fixture.agd_home.join("logs/signing-denials.jsonl").exists());
}

#[test]
fn push_to_origin_is_disabled_by_default() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    let pushurl = fixture.git_stdout(&workspace, ["remote", "get-url", "--push", "origin"]);
    assert_eq!(pushurl.trim(), "agd-deny://push-disabled");
}

#[test]
fn status_identifies_human_checkout_and_agent_workspace() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture
        .agd()
        .arg("status")
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Mode: human checkout"));

    fixture
        .agd()
        .arg("status")
        .current_dir(&workspace)
        .assert()
        .success()
        .stdout(predicate::str::contains("Mode: agent workspace"));
}

#[test]
fn status_shows_pending_agent_branch_summaries() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/status-one"]);
    fixture.write_file(&workspace, "one.txt", "one\n");
    fixture.git_in(&workspace, ["add", "one.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "status one"]);
    fixture.write_file(&workspace, "two.txt", "two\n");
    fixture.git_in(&workspace, ["add", "two.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "status two"]);

    fixture.git_in(&workspace, ["switch", "main"]);
    fixture.git_in(&workspace, ["switch", "-c", "agent/status-two"]);
    fixture.write_file(&workspace, "three.txt", "three\n");
    fixture.git_in(&workspace, ["add", "three.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "status three"]);

    fixture
        .agd()
        .arg("status")
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Pending agent branches:"))
        .stdout(predicate::str::contains("agent/status-one"))
        .stdout(predicate::str::contains("2 commits"))
        .stdout(predicate::str::contains("2 files changed"))
        .stdout(predicate::str::contains("agent/status-two"))
        .stdout(predicate::str::contains("1 commit"))
        .stdout(predicate::str::contains("1 file changed"));
}

#[test]
fn identity_shows_project_agent_identity() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture
        .agd()
        .arg("identity")
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Local Agent"))
        .stdout(predicate::str::contains("agent@agd.invalid"))
        .stdout(predicate::str::contains("unsigned"));

    fixture
        .agd()
        .arg("identity")
        .current_dir(&workspace)
        .assert()
        .success()
        .stdout(predicate::str::contains("Local Agent"))
        .stdout(predicate::str::contains("agent@agd.invalid"))
        .stdout(predicate::str::contains("unsigned"));
}

#[test]
fn json_identity_outputs_project_agent_identity() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();

    let identity = fixture.agd_json(["--json", "identity"], &fixture.human);
    assert_eq!(identity["name"], "Local Agent");
    assert_eq!(identity["email"], "agent@agd.invalid");
    assert_eq!(identity["signing"], "unsigned");
}

#[test]
fn shell_enters_default_workspace_with_agd_environment() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();
    let capture = fixture._tmp.path().join("shell-capture.txt");
    let fake_shell = fixture._tmp.path().join("fake-shell");
    fs::write(
        &fake_shell,
        "#!/bin/sh\n{\nprintf 'pwd=%s\\n' \"$(pwd -P)\"\nprintf 'AGD_WORKSPACE=%s\\n' \"$AGD_WORKSPACE\"\nprintf 'AGD_PROJECT_ID=%s\\n' \"$AGD_PROJECT_ID\"\nprintf 'AGD_WORKSPACE_ID=%s\\n' \"$AGD_WORKSPACE_ID\"\n} > \"$AGD_TEST_SHELL_CAPTURE\"\n",
    )
    .expect("write fake shell");
    make_executable(&fake_shell);

    fixture
        .agd()
        .arg("shell")
        .env("SHELL", &fake_shell)
        .env("AGD_TEST_SHELL_CAPTURE", &capture)
        .current_dir(&fixture.human)
        .assert()
        .success();

    let output = fs::read_to_string(capture).expect("read shell capture");
    let workspace = workspace.canonicalize().expect("canonical workspace");
    assert!(output.contains(&format!("pwd={}\n", workspace.display())));
    assert!(output.contains("AGD_WORKSPACE=1\n"));
    assert!(output.contains(&format!("AGD_PROJECT_ID={}\n", fixture.project_id())));
    assert!(output.contains("AGD_WORKSPACE_ID=default\n"));
}

#[test]
fn agent_workspace_blocks_commits_on_protected_branches() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.write_file(&workspace, "main.txt", "do not commit on main\n");
    fixture.git_in(&workspace, ["add", "main.txt"]);
    let output = fixture.git_fails(&workspace, ["commit", "-m", "main work"]);
    assert!(output.contains("AGD: commits on protected branch 'main' are disabled"));
    assert!(output.contains("git switch -c agent/"));
}

#[test]
fn agent_workspace_allows_commits_on_agent_branches() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/protected-ok"]);
    fixture.write_file(&workspace, "agent.txt", "commit on agent branch\n");
    fixture.git_in(&workspace, ["add", "agent.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent work"]);
}

#[test]
fn workspace_list_shows_recorded_workspaces() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();
    let workspace_text = workspace.to_str().expect("workspace path utf-8");

    fixture
        .agd()
        .args(["workspace", "list"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("default"))
        .stdout(predicate::str::contains("active"))
        .stdout(predicate::str::contains(workspace_text));

    fixture
        .agd()
        .args(["workspace", "list"])
        .current_dir(&workspace)
        .assert()
        .success()
        .stdout(predicate::str::contains("default"))
        .stdout(predicate::str::contains("active"))
        .stdout(predicate::str::contains(workspace_text));
}

#[test]
fn json_workspace_list_outputs_recorded_workspaces() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();
    let workspace_text = workspace.to_str().expect("workspace path utf-8");

    let response = fixture.agd_json(["--json", "workspace", "list"], &fixture.human);
    assert_eq!(response["workspaces"][0]["id"], "default");
    assert_eq!(response["workspaces"][0]["status"], "active");
    assert_eq!(response["workspaces"][0]["path"], workspace_text);
}

#[test]
fn workspace_create_adds_named_managed_workspace() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();

    fixture
        .agd()
        .args(["workspace", "create", "review"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Created workspace review"));

    let output = fixture
        .agd()
        .args(["path", "--workspace", "review"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let review = std::path::PathBuf::from(String::from_utf8(output).unwrap().trim());
    assert!(review.join(".git").exists());
    assert!(review.join(".agd/workspace.json").exists());

    fixture
        .agd()
        .args(["workspace", "list"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("review"))
        .stdout(predicate::str::contains(
            review.to_str().expect("review path utf-8"),
        ));

    let marker = fs::read(review.join(".agd/workspace.json")).expect("read workspace marker");
    let marker: Value = serde_json::from_slice(&marker).expect("parse workspace marker");
    assert_eq!(marker["workspace_id"], "review");
    let author = fixture.git_stdout(&review, ["config", "user.email"]);
    assert_eq!(author.trim(), "agent@agd.invalid");
    let pushurl = fixture.git_stdout(&review, ["remote", "get-url", "--push", "origin"]);
    assert_eq!(pushurl.trim(), "agd-deny://push-disabled");
}

#[test]
fn json_workspace_create_outputs_created_workspace() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();

    let created = fixture.agd_json(["--json", "workspace", "create", "review"], &fixture.human);
    assert_eq!(created["workspace"]["id"], "review");
    assert_eq!(created["workspace"]["status"], "active");
    let review_path = created["workspace"]["path"]
        .as_str()
        .expect("workspace path");
    assert!(std::path::Path::new(review_path).join(".git").exists());

    let path = fixture.agd_json(["--json", "path", "--workspace", "review"], &fixture.human);
    assert_eq!(path["path"], review_path);
}

#[test]
fn json_path_and_status_outputs_are_machine_readable() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();
    let workspace = workspace.to_str().expect("workspace utf-8");
    let human_checkout = fixture.human.canonicalize().expect("canonical human");
    let human_checkout = human_checkout.to_str().expect("human checkout utf-8");

    let path = fixture.agd_json(["--json", "path"], &fixture.human);
    assert_eq!(path["path"], workspace);

    let status = fixture.agd_json(["--json", "status"], &fixture.human);
    assert_eq!(status["mode"], "human_checkout");
    assert_eq!(status["project"], "human");
    assert_eq!(status["human_checkout"], human_checkout);
    assert_eq!(status["workspace"]["path"], workspace);
    assert_eq!(status["agent_identity"]["email"], "agent@agd.invalid");
    assert_eq!(status["signing"], "disabled");
    assert_eq!(status["push"], "denied");
    assert_eq!(status["default_target"], "main");
    assert_eq!(status["current_branch"], "main");
}

#[test]
fn json_status_outputs_agent_branch_summaries() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/status-json-one"]);
    fixture.write_file(&workspace, "json-one.txt", "one\n");
    fixture.git_in(&workspace, ["add", "json-one.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "status json one"]);
    fixture.write_file(&workspace, "json-two.txt", "two\n");
    fixture.git_in(&workspace, ["add", "json-two.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "status json two"]);

    fixture.git_in(&workspace, ["switch", "main"]);
    fixture.git_in(&workspace, ["switch", "-c", "agent/status-json-two"]);
    fixture.write_file(&workspace, "json-three.txt", "three\n");
    fixture.git_in(&workspace, ["add", "json-three.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "status json three"]);

    let status = fixture.agd_json(["--json", "status"], &fixture.human);
    assert_eq!(
        status["agent_branches"],
        serde_json::json!([
            {
                "branch": "agent/status-json-one",
                "commits": 2,
                "files_changed": 2
            },
            {
                "branch": "agent/status-json-two",
                "commits": 1,
                "files_changed": 1
            }
        ])
    );
}

#[test]
fn bless_defaults_to_human_adoption_branch() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.configure_fake_human_signer();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();
    let main_before = fixture.git_stdout(&fixture.human, ["rev-parse", "main"]);

    fixture.git_in(&workspace, ["switch", "-c", "agent/refactor-auth"]);
    fixture.write_file(&workspace, "agent.txt", "one\n");
    fixture.git_in(&workspace, ["add", "agent.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent one"]);

    fixture
        .agd()
        .args(["bless", "agent/refactor-auth"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Blessed agent/refactor-auth onto refactor-auth",
        ));

    let current_branch = fixture.git_stdout(&fixture.human, ["branch", "--show-current"]);
    assert_eq!(current_branch.trim(), "refactor-auth");
    let main_after = fixture.git_stdout(&fixture.human, ["rev-parse", "main"]);
    assert_eq!(main_after, main_before);
    let subject = fixture.git_stdout(&fixture.human, ["log", "-1", "--format=%s"]);
    assert!(subject.contains("Adopt agent/refactor-auth"));
}

#[test]
fn bless_agent_main_defaults_to_adopt_main_branch() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.configure_fake_human_signer();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();
    let main_before = fixture.git_stdout(&fixture.human, ["rev-parse", "main"]);

    fixture.git_in(&workspace, ["switch", "-c", "agent/main"]);
    fixture.write_file(&workspace, "session.txt", "integrated work\n");
    fixture.git_in(&workspace, ["add", "session.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "session integration"]);

    fixture
        .agd()
        .args(["bless", "agent/main"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Blessed agent/main onto adopt/main",
        ));

    let current_branch = fixture.git_stdout(&fixture.human, ["branch", "--show-current"]);
    assert_eq!(current_branch.trim(), "adopt/main");
    let main_after = fixture.git_stdout(&fixture.human, ["rev-parse", "main"]);
    assert_eq!(main_after, main_before);
    let body = fixture.git_stdout(&fixture.human, ["log", "-1", "--format=%B"]);
    assert!(body.contains("AGD-Agent-Branch: agent/main"));
}

#[test]
fn bless_uses_explicit_human_adoption_branch() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.configure_fake_human_signer();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/refactor-auth"]);
    fixture.write_file(&workspace, "agent.txt", "one\n");
    fixture.git_in(&workspace, ["add", "agent.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent one"]);

    fixture
        .agd()
        .args([
            "bless",
            "agent/refactor-auth",
            "--branch",
            "cody/refactor-auth",
        ])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Blessed agent/refactor-auth onto cody/refactor-auth",
        ));

    let current_branch = fixture.git_stdout(&fixture.human, ["branch", "--show-current"]);
    assert_eq!(current_branch.trim(), "cody/refactor-auth");
}

#[test]
fn bless_uses_explicit_target_branch_for_adoption_branch() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.configure_fake_human_signer();
    fixture.git(["switch", "-c", "release"]);
    fixture.write_file(&fixture.human, "release.txt", "release-only\n");
    fixture.git(["add", "release.txt"]);
    fixture.git(["commit", "-m", "prepare release"]);
    fixture.git(["switch", "main"]);
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/refactor-auth"]);
    fixture.write_file(&workspace, "agent.txt", "one\n");
    fixture.git_in(&workspace, ["add", "agent.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent one"]);

    let main_before = fixture.git_stdout(&fixture.human, ["rev-parse", "main"]);
    fixture
        .agd()
        .args(["bless", "agent/refactor-auth", "--target", "release"])
        .current_dir(&fixture.human)
        .assert()
        .success();

    let current_branch = fixture.git_stdout(&fixture.human, ["branch", "--show-current"]);
    assert_eq!(current_branch.trim(), "refactor-auth");
    let release_file = fs::read_to_string(fixture.human.join("release.txt")).expect("release file");
    assert_eq!(release_file, "release-only\n");
    let main_after = fixture.git_stdout(&fixture.human, ["rev-parse", "main"]);
    assert_eq!(main_after, main_before);
}

#[test]
fn bless_direct_adopts_onto_current_branch() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.configure_fake_human_signer();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/refactor-auth"]);
    fixture.write_file(&workspace, "agent.txt", "one\n");
    fixture.git_in(&workspace, ["add", "agent.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent one"]);

    fixture
        .agd()
        .args(["bless", "agent/refactor-auth", "--direct"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Blessed agent/refactor-auth\n"));

    let current_branch = fixture.git_stdout(&fixture.human, ["branch", "--show-current"]);
    assert_eq!(current_branch.trim(), "main");
    let main_count = fixture.git_stdout(&fixture.human, ["rev-list", "--count", "main"]);
    assert_eq!(main_count.trim(), "2");
}

#[test]
fn bless_refuses_existing_human_adoption_branch() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.configure_fake_human_signer();
    fixture.git(["branch", "refactor-auth"]);
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/refactor-auth"]);
    fixture.write_file(&workspace, "agent.txt", "one\n");
    fixture.git_in(&workspace, ["add", "agent.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent one"]);

    fixture
        .agd()
        .args(["bless", "agent/refactor-auth"])
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "adoption branch already exists: refactor-auth",
        ));

    let current_branch = fixture.git_stdout(&fixture.human, ["branch", "--show-current"]);
    assert_eq!(current_branch.trim(), "main");
}

#[test]
fn bless_squashes_agent_branch_into_one_human_commit() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.configure_fake_human_signer();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/refactor-auth"]);
    fixture.write_file(&workspace, "agent.txt", "one\n");
    fixture.git_in(&workspace, ["add", "agent.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent one"]);
    fixture.write_file(&workspace, "agent.txt", "two\n");
    fixture.git_in(&workspace, ["add", "agent.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent two"]);

    fixture
        .agd()
        .args(["bless", "agent/refactor-auth"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Blessed agent/refactor-auth"));

    let subject = fixture.git_stdout(&fixture.human, ["log", "-1", "--format=%s"]);
    assert!(subject.contains("Adopt agent/refactor-auth"));
    let body = fixture.git_stdout(&fixture.human, ["log", "-1", "--format=%B"]);
    assert!(body.contains("AGD-Agent-Branch: agent/refactor-auth"));
    assert_eq!(
        fixture
            .git_stdout(&fixture.human, ["rev-list", "--count", "HEAD"])
            .trim(),
        "2"
    );
}

#[test]
fn json_bless_outputs_blessed_status() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.configure_fake_human_signer();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/refactor-auth"]);
    fixture.write_file(&workspace, "agent.txt", "one\n");
    fixture.git_in(&workspace, ["add", "agent.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent one"]);

    let response = fixture.agd_json(["--json", "bless", "agent/refactor-auth"], &fixture.human);
    assert_eq!(response["status"], "blessed");
    assert_eq!(response["branch"], "agent/refactor-auth");
    assert_eq!(response["adoption"], "squash");
    assert_eq!(response["target_branch"], "main");
    assert_eq!(response["adoption_branch"], "refactor-auth");

    let status = fixture.git_stdout(&fixture.human, ["status", "--porcelain"]);
    assert!(status.trim().is_empty());
    let subject = fixture.git_stdout(&fixture.human, ["log", "-1", "--format=%s"]);
    assert!(subject.contains("Adopt agent/refactor-auth"));
}

#[test]
fn verify_reports_missing_agd_metadata_for_plain_commit() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();

    fixture
        .agd()
        .args(["verify", "HEAD"])
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stdout(predicate::str::contains("missing AGD metadata"));
}

#[test]
fn json_verify_reports_missing_agd_metadata() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();

    let output = fixture
        .agd()
        .args(["--json", "verify", "HEAD"])
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let report: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(report["status"], "missing_metadata");
    assert_eq!(report["commit"], "HEAD");
}

#[test]
fn verify_accepts_current_squash_adoption_commit() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.configure_fake_human_signer();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/verify"]);
    fixture.write_file(&workspace, "verify.txt", "verify me\n");
    fixture.git_in(&workspace, ["add", "verify.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "verify me"]);

    fixture
        .agd()
        .args(["bless", "agent/verify"])
        .current_dir(&fixture.human)
        .assert()
        .success();

    fixture
        .agd()
        .args(["verify", "HEAD"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("verified AGD patch"));

    let report = fixture.agd_json(["--json", "verify", "HEAD"], &fixture.human);
    assert_eq!(report["status"], "verified");
    assert_eq!(
        report["trailers"]["AGD-Patch-SHA256"]
            .as_str()
            .expect("patch hash")
            .len(),
        64
    );
}

#[test]
fn verify_rejects_commit_when_adopted_patch_differs_from_agent_patch() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/verify"]);
    fixture.write_file(&workspace, "verify.txt", "agent work\n");
    fixture.git_in(&workspace, ["add", "verify.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent work"]);

    let base = fixture.git_stdout(&workspace, ["rev-parse", "main"]);
    let tip = fixture.git_stdout(&workspace, ["rev-parse", "agent/verify"]);
    let patch = fixture.git_stdout(
        &workspace,
        [
            "diff",
            "--binary",
            "--full-index",
            "--no-ext-diff",
            base.trim(),
            tip.trim(),
        ],
    );
    let patch_hash = sha256_hex(patch.as_bytes());
    let workspace_path = workspace.to_str().expect("workspace path");
    fixture.git_in(
        &fixture.human,
        [
            "fetch",
            workspace_path,
            "refs/heads/agent/verify:refs/agd/agent/agent/verify",
        ],
    );

    fixture.write_file(&fixture.human, "verify.txt", "different work\n");
    fixture.git(["add", "verify.txt"]);
    fixture.git_in(
        &fixture.human,
        [
            "commit",
            "-m",
            "Adopt agent/verify",
            "-m",
            &format!(
                "AGD-Project: {}\nAGD-Workspace: default\nAGD-Agent-Branch: agent/verify\nAGD-Agent-Base: {}\nAGD-Agent-Tip: {}\nAGD-Adoption: squash\nAGD-Patch-SHA256: {}",
                fixture.project_id(),
                base.trim(),
                tip.trim(),
                patch_hash
            ),
        ],
    );

    fixture
        .agd()
        .args(["verify", "HEAD"])
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stdout(predicate::str::contains("AGD patch hash mismatch"));

    let output = fixture
        .agd()
        .args(["--json", "verify", "HEAD"])
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let report: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(report["status"], "mismatch");
    assert_eq!(report["expected_patch_sha256"], patch_hash);
    assert_ne!(report["actual_patch_sha256"], patch_hash);
}

#[test]
fn bless_abort_restores_human_checkout_after_squash_conflict() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.configure_fake_human_signer();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/conflict"]);
    fixture.write_file(&workspace, "README.md", "agent change\n");
    fixture.git_in(&workspace, ["add", "README.md"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent conflict"]);

    fixture.write_file(&fixture.human, "README.md", "human change\n");
    fixture.git(["add", "README.md"]);
    fixture.git(["commit", "-m", "human conflict"]);
    let human_head = fixture.git_stdout(&fixture.human, ["rev-parse", "HEAD"]);

    fixture
        .agd()
        .args(["bless", "agent/conflict"])
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains("agd bless --abort"));

    let conflicted_status = fixture.git_stdout(&fixture.human, ["status", "--porcelain"]);
    assert!(conflicted_status.contains("UU README.md"));
    let bless_state = fixture.human.join(".git/agd/bless.json");
    assert!(bless_state.exists());

    fixture
        .agd()
        .args(["bless", "--abort"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Aborted bless operation"));

    let status = fixture.git_stdout(&fixture.human, ["status", "--porcelain"]);
    assert!(status.trim().is_empty());
    let head_after_abort = fixture.git_stdout(&fixture.human, ["rev-parse", "HEAD"]);
    assert_eq!(head_after_abort.trim(), human_head.trim());
    let readme = fs::read_to_string(fixture.human.join("README.md")).expect("read README");
    assert_eq!(readme, "human change\n");
    assert!(!bless_state.exists());
}

#[test]
fn json_bless_abort_outputs_aborted_status() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.configure_fake_human_signer();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/conflict"]);
    fixture.write_file(&workspace, "README.md", "agent change\n");
    fixture.git_in(&workspace, ["add", "README.md"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent conflict"]);

    fixture.write_file(&fixture.human, "README.md", "human change\n");
    fixture.git(["add", "README.md"]);
    fixture.git(["commit", "-m", "human conflict"]);
    let human_head = fixture.git_stdout(&fixture.human, ["rev-parse", "HEAD"]);

    fixture
        .agd()
        .args(["bless", "agent/conflict"])
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains("agd bless --abort"));

    let bless_state = fixture.human.join(".git/agd/bless.json");
    assert!(bless_state.exists());

    let response = fixture.agd_json(["--json", "bless", "--abort"], &fixture.human);
    assert_eq!(response["status"], "aborted");

    let status = fixture.git_stdout(&fixture.human, ["status", "--porcelain"]);
    assert!(status.trim().is_empty());
    let head_after_abort = fixture.git_stdout(&fixture.human, ["rev-parse", "HEAD"]);
    assert_eq!(head_after_abort.trim(), human_head.trim());
    assert!(!bless_state.exists());
}

#[test]
fn bless_continue_commits_resolved_squash_conflict() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.configure_fake_human_signer();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/conflict"]);
    fixture.write_file(&workspace, "README.md", "agent change\n");
    fixture.git_in(&workspace, ["add", "README.md"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent conflict"]);

    fixture.write_file(&fixture.human, "README.md", "human change\n");
    fixture.git(["add", "README.md"]);
    fixture.git(["commit", "-m", "human conflict"]);

    fixture
        .agd()
        .args(["bless", "agent/conflict"])
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains("agd bless --continue"));
    let bless_state = fixture.human.join(".git/agd/bless.json");
    assert!(bless_state.exists());

    fixture.write_file(&fixture.human, "README.md", "resolved change\n");
    fixture.git(["add", "README.md"]);

    fixture
        .agd()
        .args(["bless", "--continue"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Continued bless operation"));

    let status = fixture.git_stdout(&fixture.human, ["status", "--porcelain"]);
    assert!(status.trim().is_empty());
    let subject = fixture.git_stdout(&fixture.human, ["log", "-1", "--format=%s"]);
    assert!(subject.contains("Adopt agent/conflict"));
    let body = fixture.git_stdout(&fixture.human, ["log", "-1", "--format=%B"]);
    assert!(body.contains("AGD-Agent-Branch: agent/conflict"));
    assert!(body.contains("AGD-Adoption: squash"));
    let readme = fs::read_to_string(fixture.human.join("README.md")).expect("read README");
    assert_eq!(readme, "resolved change\n");
    assert!(!bless_state.exists());
}

#[test]
fn json_bless_continue_outputs_continued_status() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.configure_fake_human_signer();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/conflict"]);
    fixture.write_file(&workspace, "README.md", "agent change\n");
    fixture.git_in(&workspace, ["add", "README.md"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent conflict"]);

    fixture.write_file(&fixture.human, "README.md", "human change\n");
    fixture.git(["add", "README.md"]);
    fixture.git(["commit", "-m", "human conflict"]);

    fixture
        .agd()
        .args(["bless", "agent/conflict"])
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains("agd bless --continue"));

    let bless_state = fixture.human.join(".git/agd/bless.json");
    assert!(bless_state.exists());

    fixture.write_file(&fixture.human, "README.md", "resolved change\n");
    fixture.git(["add", "README.md"]);

    let response = fixture.agd_json(["--json", "bless", "--continue"], &fixture.human);
    assert_eq!(response["status"], "continued");
    assert_eq!(response["branch"], "agent/conflict");
    assert_eq!(response["adoption"], "squash");

    let status = fixture.git_stdout(&fixture.human, ["status", "--porcelain"]);
    assert!(status.trim().is_empty());
    let subject = fixture.git_stdout(&fixture.human, ["log", "-1", "--format=%s"]);
    assert!(subject.contains("Adopt agent/conflict"));
    let readme = fs::read_to_string(fixture.human.join("README.md")).expect("read README");
    assert_eq!(readme, "resolved change\n");
    assert!(!bless_state.exists());
}

#[test]
fn bless_preserve_replays_agent_commits_with_human_signatures() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.configure_fake_human_signer();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/refactor-auth"]);
    fixture.write_file(&workspace, "one.txt", "one\n");
    fixture.git_in(&workspace, ["add", "one.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent one"]);
    fixture.write_file(&workspace, "two.txt", "two\n");
    fixture.git_in(&workspace, ["add", "two.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent two"]);

    fixture
        .agd()
        .args(["bless", "agent/refactor-auth", "--preserve"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Preserved agent/refactor-auth"));

    assert_eq!(
        fixture
            .git_stdout(&fixture.human, ["rev-list", "--count", "HEAD"])
            .trim(),
        "3"
    );
    let history = fixture.git_stdout(
        &fixture.human,
        [
            "log",
            "-2",
            "--reverse",
            "--format=%an <%ae>|%cn <%ce>|%s%n%B%x1e",
        ],
    );
    assert!(history.contains(
        "Local Agent <agent@agd.invalid>|Human Developer <human@example.test>|agent one"
    ));
    assert!(history.contains(
        "Local Agent <agent@agd.invalid>|Human Developer <human@example.test>|agent two"
    ));
    assert_eq!(history.matches("AGD-Adoption: preserve").count(), 2);
    assert_eq!(
        history
            .matches("AGD-Agent-Branch: agent/refactor-auth")
            .count(),
        2
    );

    let commits = fixture.git_stdout(&fixture.human, ["rev-list", "-2", "HEAD"]);
    for commit in commits.lines() {
        let raw = fixture.git_stdout(&fixture.human, ["cat-file", "-p", commit]);
        assert!(raw.contains("gpgsig "));
    }
}

#[test]
fn verify_accepts_preserve_adoption_commits() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.configure_fake_human_signer();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/refactor-auth"]);
    fixture.write_file(&workspace, "one.txt", "one\n");
    fixture.git_in(&workspace, ["add", "one.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent one"]);
    fixture.write_file(&workspace, "two.txt", "two\n");
    fixture.git_in(&workspace, ["add", "two.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent two"]);

    fixture
        .agd()
        .args(["bless", "agent/refactor-auth", "--preserve"])
        .current_dir(&fixture.human)
        .assert()
        .success();

    for commit in ["HEAD", "HEAD~1"] {
        fixture
            .agd()
            .args(["verify", commit])
            .current_dir(&fixture.human)
            .assert()
            .success()
            .stdout(predicate::str::contains("verified AGD patch"));
    }
}

#[test]
fn bless_merge_creates_signed_merge_commit_and_preserves_agent_commits() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.configure_fake_human_signer();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/refactor-auth"]);
    fixture.write_file(&workspace, "one.txt", "one\n");
    fixture.git_in(&workspace, ["add", "one.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent one"]);
    fixture.write_file(&workspace, "two.txt", "two\n");
    fixture.git_in(&workspace, ["add", "two.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent two"]);

    fixture
        .agd()
        .args(["bless", "agent/refactor-auth", "--merge"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Merged agent/refactor-auth"));

    assert_eq!(
        fixture
            .git_stdout(&fixture.human, ["rev-list", "--count", "HEAD"])
            .trim(),
        "4"
    );
    let merge_parents = fixture.git_stdout(&fixture.human, ["rev-list", "--parents", "-1", "HEAD"]);
    assert_eq!(merge_parents.split_whitespace().count(), 3);

    let merge_raw = fixture.git_stdout(&fixture.human, ["cat-file", "-p", "HEAD"]);
    assert!(merge_raw.contains("gpgsig "));
    assert!(merge_raw.contains("AGD-Adoption: merge"));
    assert!(merge_raw.contains("AGD-Agent-Branch: agent/refactor-auth"));

    let agent_history = fixture.git_stdout(
        &fixture.human,
        [
            "log",
            "--first-parent",
            "--invert-grep",
            "--grep",
            "Merge agent/refactor-auth",
            "--format=%an <%ae>|%cn <%ce>|%s",
            "HEAD^2",
        ],
    );
    assert!(agent_history
        .contains("Local Agent <agent@agd.invalid>|Local Agent <agent@agd.invalid>|agent one"));
    assert!(agent_history
        .contains("Local Agent <agent@agd.invalid>|Local Agent <agent@agd.invalid>|agent two"));
}

#[test]
fn bless_refuses_existing_operation_lock() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.configure_fake_human_signer();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/refactor-auth"]);
    fixture.write_file(&workspace, "agent.txt", "agent work\n");
    fixture.git_in(&workspace, ["add", "agent.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent work"]);

    let lock_dir = fixture
        .agd_home
        .join("projects")
        .join(fixture.project_id())
        .join("locks");
    fs::create_dir_all(&lock_dir).expect("create lock dir");
    fs::write(lock_dir.join("bless.lock"), "{}\n").expect("write lock");

    fixture
        .agd()
        .args(["bless", "agent/refactor-auth"])
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "bless operation already in progress",
        ))
        .stderr(predicate::str::contains("bless.lock"));

    assert_eq!(
        fixture
            .git_stdout(&fixture.human, ["rev-list", "--count", "HEAD"])
            .trim(),
        "1"
    );
}

#[test]
fn bless_lock_records_recovery_metadata_while_operation_runs() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.configure_lock_capturing_human_signer();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/refactor-auth"]);
    fixture.write_file(&workspace, "agent.txt", "agent work\n");
    fixture.git_in(&workspace, ["add", "agent.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent work"]);

    let project_id = fixture.project_id();
    let lock_path = fixture
        .agd_home
        .join("projects")
        .join(&project_id)
        .join("locks")
        .join("bless.lock");
    let captured_lock_path = fixture._tmp.path().join("captured-bless.lock.json");

    fixture
        .agd()
        .args(["bless", "agent/refactor-auth"])
        .env("AGD_TEST_LOCK_PATH", &lock_path)
        .env("AGD_TEST_CAPTURE_PATH", &captured_lock_path)
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Blessed agent/refactor-auth"));

    assert!(!lock_path.exists());
    let lock = fs::read(&captured_lock_path).expect("captured bless lock");
    let lock: Value = serde_json::from_slice(&lock).expect("parse captured bless lock");
    assert!(lock["operation_id"]
        .as_str()
        .expect("operation id")
        .starts_with("op_"));
    assert_eq!(lock["operation"], "bless");
    assert_eq!(lock["project_id"], project_id);
    assert_eq!(lock["workspace_id"], "default");
    assert!(lock["pid"].as_u64().expect("pid") > 0);
    let started_at = lock["started_at"].as_str().expect("started_at");
    assert!(started_at.contains('T'));
    assert!(started_at.ends_with('Z'));
}

#[test]
fn branches_lists_agent_branches() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/refactor-auth"]);
    fixture.write_file(&workspace, "agent.txt", "agent work\n");
    fixture.git_in(&workspace, ["add", "agent.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent work"]);

    fixture
        .agd()
        .arg("branches")
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("agent/refactor-auth"));
}

#[test]
fn branches_omits_main_mirror_when_default_target_is_not_main() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.git(["switch", "-c", "develop"]);
    fixture.write_file(&fixture.human, "develop.txt", "develop base\n");
    fixture.git(["add", "develop.txt"]);
    fixture.git(["commit", "-m", "develop base"]);
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git(["switch", "main"]);
    fixture.write_file(&fixture.human, "main.txt", "main update\n");
    fixture.git(["add", "main.txt"]);
    fixture.git(["commit", "-m", "main update"]);
    fixture.git(["switch", "develop"]);
    fixture
        .agd()
        .arg("sync")
        .current_dir(&fixture.human)
        .assert()
        .success();

    fixture.git_in(&workspace, ["switch", "-c", "agent/review"]);
    fixture.write_file(&workspace, "agent.txt", "agent work\n");
    fixture.git_in(&workspace, ["add", "agent.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent work"]);

    let branches = fixture.agd_json(["--json", "branches"], &fixture.human);
    assert_eq!(branches["branches"], serde_json::json!(["agent/review"]));
}

#[test]
fn branches_omits_default_protected_branch_patterns() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    for branch in ["master", "trunk", "develop", "release/1.0", "stable/fix"] {
        fixture.git_in(&workspace, ["switch", "main"]);
        fixture.git_in(&workspace, ["switch", "-c", branch]);
    }
    fixture.git_in(&workspace, ["switch", "main"]);
    fixture.git_in(&workspace, ["switch", "-c", "scratch/experiment"]);
    fixture.write_file(&workspace, "scratch.txt", "scratch work\n");
    fixture.git_in(&workspace, ["add", "scratch.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "scratch work"]);

    let branches = fixture.agd_json(["--json", "branches"], &fixture.human);
    assert_eq!(
        branches["branches"],
        serde_json::json!(["scratch/experiment"])
    );
}

#[test]
fn review_commands_reject_main_mirror_when_default_target_is_not_main() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.git(["switch", "-c", "develop"]);
    fixture.write_file(&fixture.human, "develop.txt", "develop base\n");
    fixture.git(["add", "develop.txt"]);
    fixture.git(["commit", "-m", "develop base"]);
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();

    fixture.git(["switch", "main"]);
    fixture.write_file(&fixture.human, "main.txt", "main update\n");
    fixture.git(["add", "main.txt"]);
    fixture.git(["commit", "-m", "main update"]);
    fixture.git(["switch", "develop"]);
    fixture
        .agd()
        .arg("sync")
        .current_dir(&fixture.human)
        .assert()
        .success();

    for command in ["log", "diff", "files"] {
        fixture
            .agd()
            .args([command, "main"])
            .current_dir(&fixture.human)
            .assert()
            .failure()
            .stderr(predicate::str::contains("agent branch is required"));
    }
}

#[test]
fn review_commands_reject_default_protected_branch_patterns() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "release/1.0"]);

    for command in ["log", "diff", "files"] {
        fixture
            .agd()
            .args([command, "release/1.0"])
            .current_dir(&fixture.human)
            .assert()
            .failure()
            .stderr(predicate::str::contains("agent branch is required"));
    }
}

#[test]
fn review_commands_show_branch_changes() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/refactor-auth"]);
    fixture.write_file(&workspace, "agent.txt", "one\n");
    fixture.git_in(&workspace, ["add", "agent.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent one"]);
    fixture.write_file(&workspace, "agent.txt", "two\n");
    fixture.git_in(&workspace, ["add", "agent.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent two"]);

    fixture
        .agd()
        .args(["log", "agent/refactor-auth"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("agent one"))
        .stdout(predicate::str::contains("agent two"));

    fixture
        .agd()
        .args(["diff", "agent/refactor-auth"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("diff --git"))
        .stdout(predicate::str::contains("agent.txt"));

    fixture
        .agd()
        .args(["files", "agent/refactor-auth"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("agent.txt"));
}

#[test]
fn json_branches_and_files_outputs_are_machine_readable() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/refactor-auth"]);
    fixture.write_file(&workspace, "agent.txt", "agent work\n");
    fixture.git_in(&workspace, ["add", "agent.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent work"]);

    let branches = fixture.agd_json(["--json", "branches"], &fixture.human);
    assert_eq!(
        branches["branches"],
        serde_json::json!(["agent/refactor-auth"])
    );

    let files = fixture.agd_json(["--json", "files", "agent/refactor-auth"], &fixture.human);
    assert_eq!(files["branch"], "agent/refactor-auth");
    assert_eq!(files["files"], serde_json::json!(["agent.txt"]));
}

#[test]
fn json_log_outputs_branch_commits() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/refactor-auth"]);
    fixture.write_file(&workspace, "agent.txt", "one\n");
    fixture.git_in(&workspace, ["add", "agent.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent one"]);
    fixture.write_file(&workspace, "agent.txt", "two\n");
    fixture.git_in(&workspace, ["add", "agent.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent two"]);

    let log = fixture.agd_json(["--json", "log", "agent/refactor-auth"], &fixture.human);
    assert_eq!(log["branch"], "agent/refactor-auth");
    assert_eq!(log["commits"].as_array().expect("commits").len(), 2);
    assert_eq!(log["commits"][0]["subject"], "agent two");
    assert_eq!(log["commits"][1]["subject"], "agent one");
    assert!(log["commits"][0]["hash"].as_str().expect("hash").len() >= 40);
    assert_eq!(
        log["commits"][0]["short_hash"]
            .as_str()
            .expect("short hash")
            .len(),
        7
    );
}

#[test]
fn json_diff_outputs_branch_patch() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/refactor-auth"]);
    fixture.write_file(&workspace, "agent.txt", "agent work\n");
    fixture.git_in(&workspace, ["add", "agent.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent work"]);

    let diff = fixture.agd_json(["--json", "diff", "agent/refactor-auth"], &fixture.human);
    assert_eq!(diff["branch"], "agent/refactor-auth");
    let patch = diff["patch"].as_str().expect("patch");
    assert!(patch.contains("diff --git"));
    assert!(patch.contains("agent.txt"));
    assert!(patch.contains("+agent work"));
}

#[test]
fn review_commands_default_to_current_agent_branch() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/current-branch"]);
    fixture.write_file(&workspace, "current.txt", "current work\n");
    fixture.git_in(&workspace, ["add", "current.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "current branch work"]);

    fixture
        .agd()
        .arg("log")
        .current_dir(&workspace)
        .assert()
        .success()
        .stdout(predicate::str::contains("current branch work"));

    fixture
        .agd()
        .arg("diff")
        .current_dir(&workspace)
        .assert()
        .success()
        .stdout(predicate::str::contains("current.txt"));

    fixture
        .agd()
        .arg("files")
        .current_dir(&workspace)
        .assert()
        .success()
        .stdout(predicate::str::contains("current.txt"));
}

#[test]
fn pr_command_pushes_agent_branch_and_invokes_gh() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    let remote = fixture._tmp.path().join("origin.git");
    let remote = remote.to_str().expect("remote path utf-8");
    fixture.git(["init", "--bare", remote]);
    fixture.git(["remote", "add", "origin", remote]);
    fixture.git(["push", "-u", "origin", "main"]);
    let base_commit = fixture.git_stdout(&fixture.human, ["rev-parse", "main"]);
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/pr-test"]);
    fixture.write_file(&workspace, "pr.txt", "agent PR work\n");
    fixture.git_in(&workspace, ["add", "pr.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent PR work"]);
    let agent_tip = fixture.git_stdout(&workspace, ["rev-parse", "agent/pr-test"]);
    let patch = fixture.git_stdout_bytes(
        &workspace,
        [
            "diff",
            "--binary",
            "--full-index",
            "--no-ext-diff",
            base_commit.trim(),
            agent_tip.trim(),
        ],
    );
    let patch_sha256 = sha256_hex(&patch);

    let gh_capture = fixture._tmp.path().join("gh-args.txt");
    let fake_path = fixture.fake_gh_path(&gh_capture);

    fixture
        .agd()
        .args(["pr", "agent/pr-test"])
        .env("PATH", fake_path)
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("https://example.test/pr/1"));

    fixture.git_stdout(
        std::path::Path::new(remote),
        ["rev-parse", "--verify", "refs/heads/agent/pr-test"],
    );
    let gh_args = fs::read_to_string(gh_capture).expect("read gh args");
    assert!(gh_args.contains("pr\n"));
    assert!(gh_args.contains("create\n"));
    assert!(gh_args.contains("--base\nmain\n"));
    assert!(gh_args.contains("--head\nagent/pr-test\n"));
    assert!(gh_args.contains("--title\nagent/pr-test\n"));
    assert!(gh_args.contains("Agent branch: agent/pr-test"));
    assert!(gh_args.contains(&format!("Base commit: {}", base_commit.trim())));
    assert!(gh_args.contains(&format!("Agent tip: {}", agent_tip.trim())));
    assert!(gh_args.contains("## Provenance"));
    assert!(gh_args.contains("AGD-Agent-Branch: agent/pr-test"));
    assert!(gh_args.contains(&format!("AGD-Agent-Base: {}", base_commit.trim())));
    assert!(gh_args.contains(&format!("AGD-Agent-Tip: {}", agent_tip.trim())));
    assert!(gh_args.contains(&format!("AGD-Patch-SHA256: {patch_sha256}")));
    assert!(gh_args.contains(
        "This PR was prepared with AGD; provenance details are below. Learn more: https://github.com/codyw912/agd"
    ));
    assert!(gh_args.contains("agent PR work"));
    assert!(gh_args.contains("pr.txt"));
}

#[test]
fn pr_bless_creates_human_adoption_branch_and_invokes_gh() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.configure_fake_human_signer();
    let remote = fixture._tmp.path().join("origin.git");
    let remote = remote.to_str().expect("remote path utf-8");
    fixture.git(["init", "--bare", remote]);
    fixture.git(["remote", "add", "origin", remote]);
    fixture.git(["push", "-u", "origin", "main"]);
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/pr-bless"]);
    fixture.write_file(&workspace, "pr-bless.txt", "agent PR bless work\n");
    fixture.git_in(&workspace, ["add", "pr-bless.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent PR bless work"]);

    let gh_capture = fixture._tmp.path().join("gh-bless-args.txt");
    let fake_path = fixture.fake_gh_path(&gh_capture);
    fixture
        .agd()
        .args(["pr", "--bless", "agent/pr-bless"])
        .env("PATH", fake_path)
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("https://example.test/pr/1"));

    fixture.git_stdout(
        std::path::Path::new(remote),
        ["rev-parse", "--verify", "refs/heads/pr-bless"],
    );
    let agent_branch_exists = StdCommand::new("git")
        .args(["rev-parse", "--verify", "refs/heads/agent/pr-bless"])
        .current_dir(remote)
        .output()
        .expect("git rev-parse")
        .status
        .success();
    assert!(!agent_branch_exists);
    let current_branch = fixture.git_stdout(&fixture.human, ["branch", "--show-current"]);
    assert_eq!(current_branch.trim(), "pr-bless");
    let gh_args = fs::read_to_string(gh_capture).expect("read gh args");
    assert!(gh_args.contains("--base\nmain\n"));
    assert!(gh_args.contains("--head\npr-bless\n"));
    assert!(gh_args.contains("--title\npr-bless\n"));
    assert!(gh_args.contains("Human adoption branch: pr-bless"));
    assert!(gh_args.contains("AGD-Agent-Branch: agent/pr-bless"));
    assert!(gh_args.contains("AGD-Adoption: squash"));
    assert!(gh_args.contains("AGD-Patch-SHA256:"));
    assert!(gh_args.contains(
        "This PR was prepared with AGD; provenance details are below. Learn more: https://github.com/codyw912/agd"
    ));
}

#[test]
fn json_pr_outputs_created_pull_request() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    let remote = fixture._tmp.path().join("origin.git");
    let remote = remote.to_str().expect("remote path utf-8");
    fixture.git(["init", "--bare", remote]);
    fixture.git(["remote", "add", "origin", remote]);
    fixture.git(["push", "-u", "origin", "main"]);
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/pr-json"]);
    fixture.write_file(&workspace, "pr-json.txt", "agent PR json work\n");
    fixture.git_in(&workspace, ["add", "pr-json.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent PR json work"]);

    let gh_capture = fixture._tmp.path().join("gh-json-args.txt");
    let fake_path = fixture.fake_gh_path(&gh_capture);
    let response = fixture
        .agd()
        .args(["--json", "pr", "agent/pr-json"])
        .env("PATH", fake_path)
        .current_dir(&fixture.human)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let response: Value = serde_json::from_slice(&response).expect("valid json");
    assert_eq!(response["branch"], "agent/pr-json");
    assert_eq!(response["url"], "https://example.test/pr/1");

    fixture.git_stdout(
        std::path::Path::new(remote),
        ["rev-parse", "--verify", "refs/heads/agent/pr-json"],
    );
}

#[test]
fn pr_falls_back_to_next_step_when_gh_is_missing() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    let remote = fixture._tmp.path().join("origin.git");
    let remote = remote.to_str().expect("remote path utf-8");
    fixture.git(["init", "--bare", remote]);
    fixture.git(["remote", "add", "origin", remote]);
    fixture.git(["push", "-u", "origin", "main"]);
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/pr-no-gh"]);
    fixture.write_file(&workspace, "pr-no-gh.txt", "agent PR fallback work\n");
    fixture.git_in(&workspace, ["add", "pr-no-gh.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent PR fallback work"]);

    fixture
        .agd()
        .args(["pr", "agent/pr-no-gh"])
        .env("PATH", fixture.git_only_path())
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Pushed agent/pr-no-gh to origin"))
        .stdout(predicate::str::contains(
            "Open a pull request from agent/pr-no-gh into main",
        ));

    fixture.git_stdout(
        std::path::Path::new(remote),
        ["rev-parse", "--verify", "refs/heads/agent/pr-no-gh"],
    );
}

#[test]
fn pr_fallback_includes_github_compare_url() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    let remote = fixture._tmp.path().join("origin.git");
    let remote = remote.to_str().expect("remote path utf-8");
    fixture.git(["init", "--bare", remote]);
    fixture.git([
        "remote",
        "add",
        "origin",
        "git@github.com:example/project.git",
    ]);
    fixture.git(["remote", "set-url", "--push", "origin", remote]);
    fixture.git(["push", "-u", "origin", "main"]);
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/pr-url"]);
    fixture.write_file(&workspace, "pr-url.txt", "agent PR url work\n");
    fixture.git_in(&workspace, ["add", "pr-url.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent PR url work"]);

    fixture
        .agd()
        .args(["pr", "agent/pr-url"])
        .env("PATH", fixture.git_only_path())
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Pushed agent/pr-url to origin"))
        .stdout(predicate::str::contains(
            "https://github.com/example/project/compare/main...agent%2Fpr-url?expand=1",
        ));

    fixture.git_stdout(
        std::path::Path::new(remote),
        ["rev-parse", "--verify", "refs/heads/agent/pr-url"],
    );
}

#[test]
fn pr_fallback_includes_gitlab_mr_url() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    let remote = fixture._tmp.path().join("origin.git");
    let remote = remote.to_str().expect("remote path utf-8");
    fixture.git(["init", "--bare", remote]);
    fixture.git([
        "remote",
        "add",
        "origin",
        "git@gitlab.com:example/project.git",
    ]);
    fixture.git(["remote", "set-url", "--push", "origin", remote]);
    fixture.git(["push", "-u", "origin", "main"]);
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/pr-url"]);
    fixture.write_file(&workspace, "pr-url.txt", "agent PR url work\n");
    fixture.git_in(&workspace, ["add", "pr-url.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent PR url work"]);

    fixture
        .agd()
        .args(["pr", "agent/pr-url"])
        .env("PATH", fixture.git_only_path())
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Pushed agent/pr-url to origin"))
        .stdout(predicate::str::contains(
            "https://gitlab.com/example/project/-/merge_requests/new",
        ))
        .stdout(predicate::str::contains(
            "merge_request[source_branch]=agent%2Fpr-url",
        ))
        .stdout(predicate::str::contains(
            "merge_request[target_branch]=main",
        ));

    fixture.git_stdout(
        std::path::Path::new(remote),
        ["rev-parse", "--verify", "refs/heads/agent/pr-url"],
    );
}

#[test]
fn pr_uses_glab_when_gh_is_missing() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    let remote = fixture._tmp.path().join("origin.git");
    let remote = remote.to_str().expect("remote path utf-8");
    fixture.git(["init", "--bare", remote]);
    fixture.git(["remote", "add", "origin", remote]);
    fixture.git(["push", "-u", "origin", "main"]);
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/pr-glab"]);
    fixture.write_file(&workspace, "pr-glab.txt", "agent PR glab work\n");
    fixture.git_in(&workspace, ["add", "pr-glab.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent PR glab work"]);

    let glab_capture = fixture._tmp.path().join("glab-args.txt");
    fixture
        .agd()
        .args(["pr", "agent/pr-glab"])
        .env("PATH", fixture.fake_glab_path(&glab_capture))
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("https://gitlab.example.test/mr/1"));

    fixture.git_stdout(
        std::path::Path::new(remote),
        ["rev-parse", "--verify", "refs/heads/agent/pr-glab"],
    );
    let glab_args = fs::read_to_string(glab_capture).expect("read glab args");
    assert!(glab_args.contains("mr\n"));
    assert!(glab_args.contains("create\n"));
    assert!(glab_args.contains("--target-branch\nmain\n"));
    assert!(glab_args.contains("--source-branch\nagent/pr-glab\n"));
    assert!(glab_args.contains("--title\nagent/pr-glab\n"));
    assert!(glab_args.contains("--description\n"));
    assert!(glab_args.contains("agent PR glab work"));
}

#[test]
fn pr_uses_glab_for_gitlab_origin_even_when_gh_exists() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    let remote = fixture._tmp.path().join("origin.git");
    let remote = remote.to_str().expect("remote path utf-8");
    fixture.git(["init", "--bare", remote]);
    fixture.git([
        "remote",
        "add",
        "origin",
        "git@gitlab.com:example/project.git",
    ]);
    fixture.git(["remote", "set-url", "--push", "origin", remote]);
    fixture.git(["push", "-u", "origin", "main"]);
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/pr-gitlab"]);
    fixture.write_file(&workspace, "pr-gitlab.txt", "agent PR gitlab work\n");
    fixture.git_in(&workspace, ["add", "pr-gitlab.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent PR gitlab work"]);

    let gh_capture = fixture._tmp.path().join("gh-gitlab-args.txt");
    let glab_capture = fixture._tmp.path().join("glab-gitlab-args.txt");
    fixture
        .agd()
        .args(["pr", "agent/pr-gitlab"])
        .env(
            "PATH",
            fixture.fake_gh_and_glab_path(&gh_capture, &glab_capture),
        )
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("https://gitlab.example.test/mr/1"));

    fixture.git_stdout(
        std::path::Path::new(remote),
        ["rev-parse", "--verify", "refs/heads/agent/pr-gitlab"],
    );
    assert!(!gh_capture.exists());
    let glab_args = fs::read_to_string(glab_capture).expect("read glab args");
    assert!(glab_args.contains("mr\n"));
    assert!(glab_args.contains("--source-branch\nagent/pr-gitlab\n"));
    assert!(glab_args.contains("agent PR gitlab work"));
}

#[test]
fn pr_rejects_main_mirror_when_default_target_is_not_main() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    let remote = fixture._tmp.path().join("origin.git");
    let remote = remote.to_str().expect("remote path utf-8");
    fixture.git(["init", "--bare", remote]);
    fixture.git(["remote", "add", "origin", remote]);
    fixture.git(["push", "-u", "origin", "main"]);
    fixture.git(["switch", "-c", "develop"]);
    fixture.write_file(&fixture.human, "develop.txt", "develop base\n");
    fixture.git(["add", "develop.txt"]);
    fixture.git(["commit", "-m", "develop base"]);
    fixture.git(["push", "-u", "origin", "develop"]);
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();

    fixture.git(["switch", "main"]);
    fixture.write_file(&fixture.human, "main.txt", "main update\n");
    fixture.git(["add", "main.txt"]);
    fixture.git(["commit", "-m", "main update"]);
    fixture.git(["switch", "develop"]);
    fixture
        .agd()
        .arg("sync")
        .current_dir(&fixture.human)
        .assert()
        .success();

    let gh_capture = fixture._tmp.path().join("gh-main-args.txt");
    let fake_path = fixture.fake_gh_path(&gh_capture);
    fixture
        .agd()
        .args(["pr", "main"])
        .env("PATH", fake_path)
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains("agent branch is required"));
    assert!(!gh_capture.exists());
}

#[test]
fn pr_rejects_default_protected_branch_patterns() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    let remote = fixture._tmp.path().join("origin.git");
    let remote = remote.to_str().expect("remote path utf-8");
    fixture.git(["init", "--bare", remote]);
    fixture.git(["remote", "add", "origin", remote]);
    fixture.git(["push", "-u", "origin", "main"]);
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();
    fixture.git_in(&workspace, ["switch", "-c", "release/1.0"]);

    let gh_capture = fixture._tmp.path().join("gh-release-args.txt");
    let fake_path = fixture.fake_gh_path(&gh_capture);
    fixture
        .agd()
        .args(["pr", "release/1.0"])
        .env("PATH", fake_path)
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains("agent branch is required"));
    assert!(!gh_capture.exists());
    fixture.git_fails(
        std::path::Path::new(remote),
        ["rev-parse", "--verify", "refs/heads/release/1.0"],
    );
}

#[test]
fn sync_fast_forwards_default_target_from_human_checkout() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.write_file(&fixture.human, "human.txt", "new human work\n");
    fixture.git(["add", "human.txt"]);
    fixture.git(["commit", "-m", "human update"]);
    let human_head = fixture.git_stdout(&fixture.human, ["rev-parse", "main"]);

    fixture
        .agd()
        .arg("sync")
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Synced main"));

    let workspace_main = fixture.git_stdout(&workspace, ["rev-parse", "main"]);
    assert_eq!(workspace_main.trim(), human_head.trim());
}

#[test]
fn json_sync_outputs_updated_target() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();
    let before = fixture.git_stdout(&workspace, ["rev-parse", "main"]);

    fixture.write_file(&fixture.human, "human.txt", "new human work\n");
    fixture.git(["add", "human.txt"]);
    fixture.git(["commit", "-m", "human update"]);
    let after = fixture.git_stdout(&fixture.human, ["rev-parse", "main"]);

    let response = fixture.agd_json(["--json", "sync"], &fixture.human);
    assert_eq!(response["target"], "main");
    assert_eq!(response["workspace_id"], "default");
    assert_eq!(response["before"], before.trim());
    assert_eq!(response["after"], after.trim());

    let workspace_main = fixture.git_stdout(&workspace, ["rev-parse", "main"]);
    assert_eq!(workspace_main.trim(), after.trim());
}

#[test]
fn sync_does_not_touch_agent_branches() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/work"]);
    fixture.write_file(&workspace, "agent.txt", "agent work\n");
    fixture.git_in(&workspace, ["add", "agent.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent work"]);
    let agent_tip = fixture.git_stdout(&workspace, ["rev-parse", "agent/work"]);
    fixture.git_in(&workspace, ["switch", "main"]);

    fixture.write_file(&fixture.human, "human.txt", "new human work\n");
    fixture.git(["add", "human.txt"]);
    fixture.git(["commit", "-m", "human update"]);

    fixture
        .agd()
        .arg("sync")
        .current_dir(&fixture.human)
        .assert()
        .success();

    let agent_tip_after_sync = fixture.git_stdout(&workspace, ["rev-parse", "agent/work"]);
    assert_eq!(agent_tip_after_sync.trim(), agent_tip.trim());
}

#[test]
fn sync_updates_main_and_non_main_default_target() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.git(["switch", "-c", "develop"]);
    fixture.write_file(&fixture.human, "develop.txt", "develop base\n");
    fixture.git(["add", "develop.txt"]);
    fixture.git(["commit", "-m", "develop base"]);
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git(["switch", "main"]);
    fixture.write_file(&fixture.human, "main.txt", "main update\n");
    fixture.git(["add", "main.txt"]);
    fixture.git(["commit", "-m", "main update"]);
    let human_main = fixture.git_stdout(&fixture.human, ["rev-parse", "main"]);

    fixture.git(["switch", "develop"]);
    fixture.write_file(&fixture.human, "develop.txt", "develop update\n");
    fixture.git(["add", "develop.txt"]);
    fixture.git(["commit", "-m", "develop update"]);
    let human_develop = fixture.git_stdout(&fixture.human, ["rev-parse", "develop"]);

    let response = fixture.agd_json(["--json", "sync"], &fixture.human);
    assert_eq!(response["target"], "develop");
    assert_eq!(response["updates"].as_array().expect("updates").len(), 2);
    assert!(response["updates"]
        .as_array()
        .expect("updates")
        .iter()
        .any(|update| update["target"] == "main"));
    assert!(response["updates"]
        .as_array()
        .expect("updates")
        .iter()
        .any(|update| update["target"] == "develop"));

    let workspace_main = fixture.git_stdout(&workspace, ["rev-parse", "main"]);
    assert_eq!(workspace_main.trim(), human_main.trim());
    let workspace_develop = fixture.git_stdout(&workspace, ["rev-parse", "develop"]);
    assert_eq!(workspace_develop.trim(), human_develop.trim());
}

#[test]
fn sync_rebases_selected_agent_branch_after_mirror_update() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/rebase-me"]);
    fixture.write_file(&workspace, "agent.txt", "agent work\n");
    fixture.git_in(&workspace, ["add", "agent.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent work"]);
    let agent_before = fixture.git_stdout(&workspace, ["rev-parse", "agent/rebase-me"]);
    fixture.git_in(&workspace, ["switch", "main"]);

    fixture.write_file(&fixture.human, "human.txt", "new human work\n");
    fixture.git(["add", "human.txt"]);
    fixture.git(["commit", "-m", "human update"]);
    let human_head = fixture.git_stdout(&fixture.human, ["rev-parse", "main"]);

    fixture
        .agd()
        .args(["sync", "--rebase", "agent/rebase-me"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Synced main"))
        .stdout(predicate::str::contains("Rebased agent/rebase-me"));

    let workspace_main = fixture.git_stdout(&workspace, ["rev-parse", "main"]);
    assert_eq!(workspace_main.trim(), human_head.trim());
    let agent_after = fixture.git_stdout(&workspace, ["rev-parse", "agent/rebase-me"]);
    assert_ne!(agent_after.trim(), agent_before.trim());
    let merge_base = fixture.git_stdout(
        &workspace,
        ["merge-base", human_head.trim(), agent_after.trim()],
    );
    assert_eq!(merge_base.trim(), human_head.trim());
    assert_eq!(
        fs::read_to_string(workspace.join("agent.txt")).expect("read agent work"),
        "agent work\n"
    );
}

#[test]
fn json_sync_rebase_reports_rebased_branch() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/rebase-json"]);
    fixture.write_file(&workspace, "agent.txt", "agent json work\n");
    fixture.git_in(&workspace, ["add", "agent.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent json work"]);
    let agent_before = fixture.git_stdout(&workspace, ["rev-parse", "agent/rebase-json"]);
    fixture.git_in(&workspace, ["switch", "main"]);

    fixture.write_file(&fixture.human, "human.txt", "new human work\n");
    fixture.git(["add", "human.txt"]);
    fixture.git(["commit", "-m", "human update"]);

    let response = fixture.agd_json(
        ["--json", "sync", "--rebase", "agent/rebase-json"],
        &fixture.human,
    );
    assert_eq!(response["target"], "main");
    assert_eq!(response["rebase"]["branch"], "agent/rebase-json");
    assert_eq!(response["rebase"]["before"], agent_before.trim());
    let agent_after = fixture.git_stdout(&workspace, ["rev-parse", "agent/rebase-json"]);
    assert_eq!(response["rebase"]["after"], agent_after.trim());
    assert_ne!(agent_after.trim(), agent_before.trim());
}

#[test]
fn sync_rebase_rejects_protected_branch_before_syncing() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();
    let workspace_main_before = fixture.git_stdout(&workspace, ["rev-parse", "main"]);

    fixture.write_file(&fixture.human, "human.txt", "new human work\n");
    fixture.git(["add", "human.txt"]);
    fixture.git(["commit", "-m", "human update"]);

    fixture
        .agd()
        .args(["sync", "--rebase", "main"])
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains("agent branch is required"));

    let workspace_main_after = fixture.git_stdout(&workspace, ["rev-parse", "main"]);
    assert_eq!(workspace_main_after.trim(), workspace_main_before.trim());
}

#[test]
fn sync_rebase_rejects_default_protected_branch_patterns() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();
    fixture.git_in(&workspace, ["switch", "-c", "release/1.0"]);
    fixture.git_in(&workspace, ["switch", "main"]);
    let workspace_main_before = fixture.git_stdout(&workspace, ["rev-parse", "main"]);

    fixture.write_file(&fixture.human, "human.txt", "new human work\n");
    fixture.git(["add", "human.txt"]);
    fixture.git(["commit", "-m", "human update"]);

    fixture
        .agd()
        .args(["sync", "--rebase", "release/1.0"])
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains("agent branch is required"));

    let workspace_main_after = fixture.git_stdout(&workspace, ["rev-parse", "main"]);
    assert_eq!(workspace_main_after.trim(), workspace_main_before.trim());
}

#[test]
fn sync_refuses_dirty_or_diverged_state() {
    let dirty_human = Fixture::new();
    dirty_human.init_human_repo();
    dirty_human
        .agd()
        .arg("init")
        .current_dir(&dirty_human.human)
        .assert()
        .success();
    dirty_human.write_file(&dirty_human.human, "dirty.txt", "dirty human\n");
    dirty_human
        .agd()
        .arg("sync")
        .current_dir(&dirty_human.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "human checkout has uncommitted changes",
        ));

    let dirty_agent = Fixture::new();
    dirty_agent.init_human_repo();
    dirty_agent
        .agd()
        .arg("init")
        .current_dir(&dirty_agent.human)
        .assert()
        .success();
    let workspace = dirty_agent.agd_path();
    dirty_agent.write_file(&workspace, "dirty.txt", "dirty agent\n");
    dirty_agent
        .agd()
        .arg("sync")
        .current_dir(&dirty_agent.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "agent workspace has uncommitted changes",
        ));

    let diverged = Fixture::new();
    diverged.init_human_repo();
    diverged
        .agd()
        .arg("init")
        .current_dir(&diverged.human)
        .assert()
        .success();
    let workspace = diverged.agd_path();
    diverged.write_file(&workspace, "workspace-main.txt", "workspace main\n");
    diverged.git_in(&workspace, ["add", "workspace-main.txt"]);
    diverged.git_in(
        &workspace,
        ["commit", "--no-verify", "-m", "workspace main update"],
    );
    diverged.write_file(&diverged.human, "human-main.txt", "human main\n");
    diverged.git(["add", "human-main.txt"]);
    diverged.git(["commit", "-m", "human main update"]);
    diverged
        .agd()
        .arg("sync")
        .current_dir(&diverged.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "default target cannot be fast-forwarded",
        ))
        .stderr(predicate::str::contains("agd reset-workspace"));
}

#[test]
fn handoff_applies_human_tracked_changes_to_clean_agent_workspace() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();
    let workspace_head = fixture.git_stdout(&workspace, ["rev-parse", "HEAD"]);

    fixture.write_file(&fixture.human, "README.md", "# test\nhuman sketch\n");

    fixture
        .agd()
        .arg("handoff")
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Handed off human changes to default",
        ));

    let workspace_readme = fs::read_to_string(workspace.join("README.md")).expect("read readme");
    assert_eq!(workspace_readme, "# test\nhuman sketch\n");
    let workspace_status = fixture.git_stdout(&workspace, ["status", "--porcelain"]);
    assert_eq!(workspace_status.trim(), "M README.md");
    let workspace_head_after = fixture.git_stdout(&workspace, ["rev-parse", "HEAD"]);
    assert_eq!(workspace_head_after.trim(), workspace_head.trim());
    let metadata =
        fs::read(workspace.join(".git/agd/handoff.json")).expect("read handoff metadata");
    let metadata: Value = serde_json::from_slice(&metadata).expect("parse handoff metadata");
    assert_eq!(metadata["status"], "applied");
    assert_eq!(metadata["workspace_id"], "default");
    assert_eq!(metadata["human_head"], workspace_head.trim());
    assert_eq!(metadata["tracked_files"], serde_json::json!(["README.md"]));
    assert_eq!(metadata["untracked_files"], serde_json::json!([]));
}

#[test]
fn handoff_refuses_dirty_agent_workspace() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.write_file(&fixture.human, "README.md", "# test\nhuman sketch\n");
    fixture.write_file(&workspace, "agent.txt", "dirty agent\n");

    fixture
        .agd()
        .arg("handoff")
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "agent workspace has uncommitted changes",
        ));
}

#[test]
fn handoff_refuses_untracked_human_files() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();

    fixture.write_file(&fixture.human, "sketch.txt", "untracked sketch\n");

    fixture
        .agd()
        .arg("handoff")
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "human checkout has untracked files",
        ))
        .stderr(predicate::str::contains("sketch.txt"));
}

#[test]
fn handoff_copies_selected_untracked_human_files() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.write_file(&fixture.human, "notes/sketch.txt", "selected sketch\n");
    fixture.write_file(&fixture.human, "private.txt", "unselected sketch\n");

    fixture
        .agd()
        .args(["handoff", "--include-untracked", "notes/sketch.txt"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Handed off human changes to default",
        ));

    let copied =
        fs::read_to_string(workspace.join("notes/sketch.txt")).expect("read selected sketch");
    assert_eq!(copied, "selected sketch\n");
    assert!(!workspace.join("private.txt").exists());
    let selected_status = fixture.git_stdout(
        &workspace,
        ["status", "--porcelain", "--", "notes/sketch.txt"],
    );
    assert_eq!(selected_status.trim(), "?? notes/sketch.txt");
    let metadata =
        fs::read(workspace.join(".git/agd/handoff.json")).expect("read handoff metadata");
    let metadata: Value = serde_json::from_slice(&metadata).expect("parse handoff metadata");
    assert_eq!(metadata["status"], "applied");
    assert_eq!(metadata["tracked_files"], serde_json::json!([]));
    assert_eq!(metadata["untracked_files"][0], "notes/sketch.txt");
}

#[test]
fn handoff_rejects_unsafe_selected_untracked_paths() {
    let parent = Fixture::new();
    parent.init_human_repo();
    parent
        .agd()
        .arg("init")
        .current_dir(&parent.human)
        .assert()
        .success();
    parent
        .agd()
        .args(["handoff", "--include-untracked", "../outside.txt"])
        .current_dir(&parent.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "untracked handoff path must be relative",
        ));

    let absolute = Fixture::new();
    absolute.init_human_repo();
    absolute
        .agd()
        .arg("init")
        .current_dir(&absolute.human)
        .assert()
        .success();
    let absolute_path = absolute._tmp.path().join("outside.txt");
    fs::write(&absolute_path, "outside\n").expect("write outside");
    absolute
        .agd()
        .arg("handoff")
        .arg("--include-untracked")
        .arg(&absolute_path)
        .current_dir(&absolute.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "untracked handoff path must be relative",
        ));
}

#[test]
fn json_handoff_outputs_applied_changes() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();
    let human_head = fixture.git_stdout(&fixture.human, ["rev-parse", "HEAD"]);

    fixture.write_file(&fixture.human, "README.md", "# test\nhuman sketch\n");
    fixture.write_file(&fixture.human, "notes/sketch.txt", "selected sketch\n");

    let response = fixture.agd_json(
        [
            "--json",
            "handoff",
            "--include-untracked",
            "notes/sketch.txt",
        ],
        &fixture.human,
    );
    assert_eq!(response["status"], "applied");
    assert_eq!(response["workspace_id"], "default");
    assert_eq!(response["human_head"], human_head.trim());
    assert_eq!(response["tracked_files"], serde_json::json!(["README.md"]));
    assert_eq!(
        response["untracked_files"],
        serde_json::json!(["notes/sketch.txt"])
    );
    assert_eq!(
        fs::read_to_string(workspace.join("README.md")).expect("read readme"),
        "# test\nhuman sketch\n"
    );
    assert!(workspace.join("notes/sketch.txt").exists());
}

#[test]
fn operation_locks_block_mutating_commands() {
    let sync_locked = Fixture::new();
    sync_locked.init_human_repo();
    sync_locked
        .agd()
        .arg("init")
        .current_dir(&sync_locked.human)
        .assert()
        .success();
    let sync_lock = sync_locked.write_operation_lock("sync");
    sync_locked
        .agd()
        .arg("sync")
        .current_dir(&sync_locked.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains("operation already in progress"))
        .stderr(predicate::str::contains("sync.lock"));
    assert!(sync_lock.exists());

    let discard_locked = Fixture::new();
    discard_locked.init_human_repo();
    discard_locked
        .agd()
        .arg("init")
        .current_dir(&discard_locked.human)
        .assert()
        .success();
    let discard_workspace = discard_locked.agd_path();
    discard_locked.git_in(&discard_workspace, ["switch", "-c", "agent/discard-locked"]);
    discard_locked.write_file(&discard_workspace, "discard.txt", "discard locked\n");
    discard_locked.git_in(&discard_workspace, ["add", "discard.txt"]);
    discard_locked.git_in(&discard_workspace, ["commit", "-m", "discard locked"]);
    discard_locked.git_in(&discard_workspace, ["switch", "main"]);
    discard_locked.write_operation_lock("discard");
    discard_locked
        .agd()
        .args(["discard", "agent/discard-locked"])
        .current_dir(&discard_locked.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains("operation already in progress"))
        .stderr(predicate::str::contains("discard.lock"));
    discard_locked.git_stdout(
        &discard_workspace,
        ["rev-parse", "--verify", "agent/discard-locked"],
    );

    let reset_locked = Fixture::new();
    reset_locked.init_human_repo();
    reset_locked
        .agd()
        .arg("init")
        .current_dir(&reset_locked.human)
        .assert()
        .success();
    let reset_workspace = reset_locked.agd_path();
    reset_locked.write_operation_lock("reset-workspace");
    reset_locked
        .agd()
        .arg("reset-workspace")
        .current_dir(&reset_locked.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains("operation already in progress"))
        .stderr(predicate::str::contains("reset-workspace.lock"));
    assert!(reset_workspace.join(".git").exists());

    let handoff_locked = Fixture::new();
    handoff_locked.init_human_repo();
    handoff_locked
        .agd()
        .arg("init")
        .current_dir(&handoff_locked.human)
        .assert()
        .success();
    handoff_locked.write_file(&handoff_locked.human, "README.md", "# test\nlocked\n");
    handoff_locked.write_operation_lock("handoff");
    handoff_locked
        .agd()
        .arg("handoff")
        .current_dir(&handoff_locked.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains("operation already in progress"))
        .stderr(predicate::str::contains("handoff.lock"));

    let doctor_locked = Fixture::new();
    doctor_locked.init_human_repo();
    doctor_locked
        .agd()
        .arg("init")
        .current_dir(&doctor_locked.human)
        .assert()
        .success();
    doctor_locked.write_operation_lock("sync");
    doctor_locked
        .agd()
        .arg("doctor")
        .current_dir(&doctor_locked.human)
        .assert()
        .failure()
        .stdout(predicate::str::contains("fail AGD operation lock"))
        .stdout(predicate::str::contains("sync.lock"))
        .stderr(predicate::str::contains("doctor found failed checks"));
}

#[test]
fn discard_deletes_clean_agent_branch() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();
    let human_head = fixture.git_stdout(&fixture.human, ["rev-parse", "HEAD"]);

    fixture.git_in(&workspace, ["switch", "-c", "agent/discard-me"]);
    fixture.write_file(&workspace, "discard.txt", "discard me\n");
    fixture.git_in(&workspace, ["add", "discard.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "discard me"]);
    fixture.git_in(&workspace, ["switch", "main"]);

    fixture
        .agd()
        .args(["discard", "agent/discard-me"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Discarded agent/discard-me"));

    fixture.git_fails(&workspace, ["rev-parse", "--verify", "agent/discard-me"]);
    let human_head_after_discard = fixture.git_stdout(&fixture.human, ["rev-parse", "HEAD"]);
    assert_eq!(human_head_after_discard.trim(), human_head.trim());
}

#[test]
fn json_discard_outputs_deleted_branch() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/discard-json"]);
    fixture.write_file(&workspace, "discard-json.txt", "discard me\n");
    fixture.git_in(&workspace, ["add", "discard-json.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "discard json"]);
    fixture.git_in(&workspace, ["switch", "main"]);

    let response = fixture.agd_json(["--json", "discard", "agent/discard-json"], &fixture.human);
    assert_eq!(response["branch"], "agent/discard-json");
    assert_eq!(response["workspace_id"], "default");
    fixture.git_fails(&workspace, ["rev-parse", "--verify", "agent/discard-json"]);
}

#[test]
fn discard_targets_named_workspace() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let default_workspace = fixture.agd_path();
    fixture
        .agd()
        .args(["workspace", "create", "review"])
        .current_dir(&fixture.human)
        .assert()
        .success();
    let review_output = fixture
        .agd()
        .args(["path", "--workspace", "review"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let review = std::path::PathBuf::from(String::from_utf8(review_output).unwrap().trim());

    fixture.git_in(&default_workspace, ["switch", "-c", "agent/keep-default"]);
    fixture.write_file(&default_workspace, "keep.txt", "keep default\n");
    fixture.git_in(&default_workspace, ["add", "keep.txt"]);
    fixture.git_in(&default_workspace, ["commit", "-m", "keep default"]);

    fixture.git_in(&review, ["switch", "-c", "agent/review-discard"]);
    fixture.write_file(&review, "discard-review.txt", "discard review\n");
    fixture.git_in(&review, ["add", "discard-review.txt"]);
    fixture.git_in(&review, ["commit", "-m", "discard review"]);
    fixture.git_in(&review, ["switch", "main"]);

    fixture
        .agd()
        .args(["discard", "--workspace", "review", "agent/review-discard"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Discarded agent/review-discard"));

    fixture.git_fails(&review, ["rev-parse", "--verify", "agent/review-discard"]);
    fixture.git_stdout(
        &default_workspace,
        ["rev-parse", "--verify", "agent/keep-default"],
    );
}

#[test]
fn discard_rejects_main_mirror_when_default_target_is_not_main() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.git(["switch", "-c", "develop"]);
    fixture.write_file(&fixture.human, "develop.txt", "develop base\n");
    fixture.git(["add", "develop.txt"]);
    fixture.git(["commit", "-m", "develop base"]);
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git(["switch", "main"]);
    fixture.write_file(&fixture.human, "main.txt", "main update\n");
    fixture.git(["add", "main.txt"]);
    fixture.git(["commit", "-m", "main update"]);
    fixture.git(["switch", "develop"]);
    fixture
        .agd()
        .arg("sync")
        .current_dir(&fixture.human)
        .assert()
        .success();

    fixture
        .agd()
        .args(["discard", "main"])
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "refusing to discard protected branch",
        ));

    fixture.git_stdout(&workspace, ["rev-parse", "--verify", "main"]);
}

#[test]
fn discard_refuses_dirty_agent_workspace() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/dirty"]);
    fixture.write_file(&workspace, "dirty.txt", "dirty work\n");

    fixture
        .agd()
        .args(["discard", "agent/dirty"])
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "agent workspace has uncommitted changes",
        ));

    fixture.git_stdout(&workspace, ["rev-parse", "--verify", "agent/dirty"]);
}

#[test]
fn discard_force_deletes_dirty_agent_branch() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/dirty-force"]);
    fixture.write_file(&workspace, "dirty.txt", "dirty work\n");

    fixture
        .agd()
        .args(["discard", "--force", "agent/dirty-force"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Discarded agent/dirty-force"));

    fixture.git_fails(&workspace, ["rev-parse", "--verify", "agent/dirty-force"]);
    let current = fixture.git_stdout(&workspace, ["branch", "--show-current"]);
    assert_eq!(current.trim(), "main");
    let status = fixture.git_stdout(&workspace, ["status", "--porcelain"]);
    assert!(status.trim().is_empty());
    assert!(!workspace.join("dirty.txt").exists());
}

#[test]
fn discard_force_still_rejects_protected_branch() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();
    fixture.write_file(&workspace, "dirty-main.txt", "dirty main\n");

    fixture
        .agd()
        .args(["discard", "--force", "main"])
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "refusing to discard protected branch",
        ));

    fixture.git_stdout(&workspace, ["rev-parse", "--verify", "main"]);
    assert!(workspace.join("dirty-main.txt").exists());
}

#[test]
fn discard_force_rejects_default_protected_branch_patterns() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();
    fixture.git_in(&workspace, ["switch", "-c", "release/1.0"]);
    fixture.write_file(&workspace, "dirty-release.txt", "dirty release\n");

    fixture
        .agd()
        .args(["discard", "--force", "release/1.0"])
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "refusing to discard protected branch",
        ));

    fixture.git_stdout(&workspace, ["rev-parse", "--verify", "release/1.0"]);
    assert!(workspace.join("dirty-release.txt").exists());
}

#[test]
fn reset_workspace_recreates_clean_managed_clone() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/reset-me"]);
    fixture.write_file(&workspace, "reset.txt", "reset me\n");
    fixture.git_in(&workspace, ["add", "reset.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "reset me"]);
    fixture.git_in(&workspace, ["switch", "main"]);

    fixture
        .agd()
        .arg("reset-workspace")
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Reset workspace"));

    assert!(workspace.join(".git").exists());
    assert!(workspace.join(".agd/workspace.json").exists());
    fixture.git_fails(&workspace, ["rev-parse", "--verify", "agent/reset-me"]);
    let author = fixture.git_stdout(&workspace, ["config", "user.email"]);
    assert_eq!(author.trim(), "agent@agd.invalid");
    let pushurl = fixture.git_stdout(&workspace, ["remote", "get-url", "--push", "origin"]);
    assert_eq!(pushurl.trim(), "agd-deny://push-disabled");
}

#[test]
fn reset_workspace_force_recreates_dirty_workspace() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/reset-dirty"]);
    fixture.write_file(&workspace, "dirty-reset.txt", "dirty reset\n");

    fixture
        .agd()
        .args(["reset-workspace", "--force"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Reset workspace"));

    assert!(workspace.join(".git").exists());
    assert!(workspace.join(".agd/workspace.json").exists());
    assert!(!workspace.join("dirty-reset.txt").exists());
    fixture.git_fails(&workspace, ["rev-parse", "--verify", "agent/reset-dirty"]);
    let status = fixture.git_stdout(&workspace, ["status", "--porcelain"]);
    assert!(status.trim().is_empty());
}

#[test]
fn reset_workspace_targets_named_workspace() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let default_workspace = fixture.agd_path();
    fixture
        .agd()
        .args(["workspace", "create", "review"])
        .current_dir(&fixture.human)
        .assert()
        .success();
    let review_output = fixture
        .agd()
        .args(["path", "--workspace", "review"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let review = std::path::PathBuf::from(String::from_utf8(review_output).unwrap().trim());

    fixture.git_in(&default_workspace, ["switch", "-c", "agent/keep-default"]);
    fixture.write_file(&default_workspace, "keep.txt", "keep default\n");
    fixture.git_in(&default_workspace, ["add", "keep.txt"]);
    fixture.git_in(&default_workspace, ["commit", "-m", "keep default"]);

    fixture.git_in(&review, ["switch", "-c", "agent/review-reset"]);
    fixture.write_file(&review, "dirty-review.txt", "dirty review\n");

    fixture
        .agd()
        .args(["reset-workspace", "--workspace", "review", "--force"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Reset workspace"));

    assert!(review.join(".git").exists());
    assert!(review.join(".agd/workspace.json").exists());
    assert!(!review.join("dirty-review.txt").exists());
    fixture.git_fails(&review, ["rev-parse", "--verify", "agent/review-reset"]);
    fixture.git_stdout(
        &default_workspace,
        ["rev-parse", "--verify", "agent/keep-default"],
    );
}

#[test]
fn json_reset_workspace_outputs_recreated_workspace() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/reset-json"]);
    fixture.write_file(&workspace, "reset-json.txt", "reset me\n");
    fixture.git_in(&workspace, ["add", "reset-json.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "reset json"]);
    fixture.git_in(&workspace, ["switch", "main"]);

    let response = fixture.agd_json(["--json", "reset-workspace"], &fixture.human);
    assert_eq!(response["workspace_id"], "default");
    assert_eq!(
        response["path"].as_str().expect("workspace path"),
        workspace.to_str().expect("workspace path utf-8")
    );
    assert!(workspace.join(".git").exists());
    fixture.git_fails(&workspace, ["rev-parse", "--verify", "agent/reset-json"]);
}

#[test]
fn doctor_reports_healthy_workspace() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();

    fixture
        .agd()
        .arg("doctor")
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("ok  project metadata"))
        .stdout(predicate::str::contains("ok  workspace exists"))
        .stdout(predicate::str::contains("ok  workspace marker"))
        .stdout(predicate::str::contains("ok  agent identity"))
        .stdout(predicate::str::contains("ok  signing disabled"))
        .stdout(predicate::str::contains("ok  deny signer"))
        .stdout(predicate::str::contains("ok  push disabled"));
}

#[test]
fn doctor_reports_broken_guardrails() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["config", "commit.gpgsign", "true"]);
    fixture.git_in(&workspace, ["config", "--unset-all", "gpg.program"]);
    fixture.git_in(
        &workspace,
        [
            "remote",
            "set-url",
            "--push",
            "origin",
            fixture.human.to_str().unwrap(),
        ],
    );

    fixture
        .agd()
        .arg("doctor")
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stdout(predicate::str::contains("fail signing disabled"))
        .stdout(predicate::str::contains("fail deny signer"))
        .stdout(predicate::str::contains("fail push disabled"))
        .stderr(predicate::str::contains("doctor found failed checks"));
}

#[test]
fn doctor_detects_human_checkout_using_agd_deny_signer() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();
    let deny_signer = fixture.git_stdout(&workspace, ["config", "gpg.program"]);
    fixture.git(["config", "gpg.program", deny_signer.trim()]);

    fixture
        .agd()
        .arg("doctor")
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stdout(predicate::str::contains("fail human signing"))
        .stdout(predicate::str::contains("AGD deny signer"))
        .stderr(predicate::str::contains("doctor found failed checks"));
}

#[test]
fn doctor_detects_missing_pre_push_hook() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fs::remove_file(workspace.join(".git/hooks/pre-push")).expect("remove pre-push hook");

    fixture
        .agd()
        .arg("doctor")
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stdout(predicate::str::contains("fail pre-push hook"))
        .stdout(predicate::str::contains("missing"))
        .stderr(predicate::str::contains("doctor found failed checks"));
}

#[test]
fn doctor_detects_missing_protected_branch_hook() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fs::remove_file(workspace.join(".git/hooks/pre-commit")).expect("remove pre-commit hook");

    fixture
        .agd()
        .arg("doctor")
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stdout(predicate::str::contains("fail protected branch hook"))
        .stdout(predicate::str::contains("missing"))
        .stderr(predicate::str::contains("doctor found failed checks"));
}

#[test]
fn doctor_detects_workspace_object_alternates() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    let alternates = workspace.join(".git/objects/info/alternates");
    fs::create_dir_all(alternates.parent().expect("alternates parent"))
        .expect("create alternates parent");
    fs::write(&alternates, "/tmp/shared-objects\n").expect("write alternates");

    fixture
        .agd()
        .arg("doctor")
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stdout(predicate::str::contains("fail workspace independence"))
        .stdout(predicate::str::contains("alternates"))
        .stderr(predicate::str::contains("doctor found failed checks"));
}

#[test]
fn doctor_repair_recreates_missing_workspace_marker() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();
    let marker_path = workspace.join(".agd/workspace.json");
    fs::remove_file(&marker_path).expect("remove workspace marker");

    fixture
        .agd()
        .arg("doctor")
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stdout(predicate::str::contains("fail workspace marker"))
        .stderr(predicate::str::contains("doctor found failed checks"));

    fixture
        .agd()
        .args(["doctor", "--repair"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Repaired workspace marker"));

    fixture
        .agd()
        .arg("doctor")
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("ok  workspace marker"));

    let marker = fs::read(&marker_path).expect("read workspace marker");
    let marker: Value = serde_json::from_slice(&marker).expect("parse workspace marker");
    assert_eq!(marker["kind"], "agd-workspace");
    assert_eq!(marker["project_id"], fixture.project_id());
    assert_eq!(marker["workspace_id"], "default");
    let human_checkout = fixture
        .human
        .canonicalize()
        .expect("canonical human checkout");
    assert_eq!(
        marker["human_checkout"],
        human_checkout.to_str().expect("human checkout utf-8")
    );
}

#[test]
fn doctor_detects_and_repairs_workspace_marker_drift() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();
    let marker_path = workspace.join(".agd/workspace.json");
    fs::write(
        &marker_path,
        r#"{
  "kind": "agd-workspace",
  "project_id": "project_wrong",
  "workspace_id": "wrong",
  "human_checkout": "/tmp/wrong",
  "created_at": "2026-05-09T00:00:00Z"
}
"#,
    )
    .expect("write drifted workspace marker");

    fixture
        .agd()
        .arg("doctor")
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stdout(predicate::str::contains("fail workspace marker"))
        .stdout(predicate::str::contains("project_id"))
        .stderr(predicate::str::contains("doctor found failed checks"));

    fixture
        .agd()
        .args(["doctor", "--repair"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Repaired workspace marker"));

    fixture
        .agd()
        .arg("doctor")
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("ok  workspace marker"));

    let marker = fs::read(&marker_path).expect("read workspace marker");
    let marker: Value = serde_json::from_slice(&marker).expect("parse workspace marker");
    assert_eq!(marker["project_id"], fixture.project_id());
    assert_eq!(marker["workspace_id"], "default");
}

#[test]
fn doctor_detects_incomplete_git_state() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();

    fs::write(fixture.human.join(".git/MERGE_HEAD"), "deadbeef\n").expect("write MERGE_HEAD");

    fixture
        .agd()
        .arg("doctor")
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stdout(predicate::str::contains("fail human git state"))
        .stdout(predicate::str::contains("MERGE_HEAD"))
        .stderr(predicate::str::contains("doctor found failed checks"));
}

#[test]
fn doctor_detects_incomplete_agd_bless_state() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.configure_fake_human_signer();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["switch", "-c", "agent/conflict"]);
    fixture.write_file(&workspace, "README.md", "agent change\n");
    fixture.git_in(&workspace, ["add", "README.md"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent conflict"]);

    fixture.write_file(&fixture.human, "README.md", "human change\n");
    fixture.git(["add", "README.md"]);
    fixture.git(["commit", "-m", "human conflict"]);

    fixture
        .agd()
        .args(["bless", "agent/conflict"])
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains("agd bless --continue"));

    fixture
        .agd()
        .arg("doctor")
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stdout(predicate::str::contains("fail AGD bless state"))
        .stdout(predicate::str::contains("agd bless --continue"))
        .stdout(predicate::str::contains("agd bless --abort"))
        .stderr(predicate::str::contains("doctor found failed checks"));
}

#[test]
fn doctor_detects_existing_agd_operation_lock() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();

    let lock_dir = fixture
        .agd_home
        .join("projects")
        .join(fixture.project_id())
        .join("locks");
    fs::create_dir_all(&lock_dir).expect("create lock dir");
    fs::write(lock_dir.join("bless.lock"), "{}\n").expect("write lock");

    fixture
        .agd()
        .arg("doctor")
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stdout(predicate::str::contains("fail AGD operation lock"))
        .stdout(predicate::str::contains("bless.lock"))
        .stderr(predicate::str::contains("doctor found failed checks"));
}

#[test]
fn doctor_repair_removes_stale_agd_operation_lock() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();

    let lock_dir = fixture
        .agd_home
        .join("projects")
        .join(fixture.project_id())
        .join("locks");
    fs::create_dir_all(&lock_dir).expect("create lock dir");
    let lock_path = lock_dir.join("bless.lock");
    fs::write(
        &lock_path,
        r#"{
  "operation_id": "op_stale",
  "operation": "bless",
  "project_id": "project_test",
  "workspace_id": "default",
  "pid": 999999999,
  "started_at": "2026-05-09T00:00:00Z"
}
"#,
    )
    .expect("write stale lock");

    fixture
        .agd()
        .arg("doctor")
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stdout(predicate::str::contains("fail AGD operation lock"))
        .stderr(predicate::str::contains("doctor found failed checks"));

    fixture
        .agd()
        .args(["doctor", "--repair"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed stale AGD operation lock"));

    assert!(!lock_path.exists());
    fixture
        .agd()
        .arg("doctor")
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("ok  AGD operation lock"));
}

#[test]
fn doctor_detects_moved_human_checkout_path() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();

    let moved = fixture._tmp.path().join("moved-human");
    fs::rename(&fixture.human, &moved).expect("move human checkout");

    fixture
        .agd()
        .arg("doctor")
        .current_dir(&moved)
        .assert()
        .failure()
        .stdout(predicate::str::contains("fail human checkout exists"))
        .stdout(predicate::str::contains("fail human checkout path"))
        .stdout(predicate::str::contains("current checkout"))
        .stderr(predicate::str::contains("doctor found failed checks"));
}

#[test]
fn doctor_repair_fixes_moved_human_checkout_path() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();
    let project_id = fixture.project_id();

    let moved = fixture._tmp.path().join("moved-human");
    fs::rename(&fixture.human, &moved).expect("move human checkout");
    let moved = moved.canonicalize().expect("canonical moved checkout");
    let moved_str = moved.to_str().expect("moved checkout utf-8");

    fixture
        .agd()
        .args(["doctor", "--repair"])
        .current_dir(&moved)
        .assert()
        .success()
        .stdout(predicate::str::contains("Repaired human checkout path"));

    fixture
        .agd()
        .arg("doctor")
        .current_dir(&moved)
        .assert()
        .success()
        .stdout(predicate::str::contains("ok  human checkout path"));

    let project = fs::read(
        fixture
            .agd_home
            .join("projects")
            .join(project_id)
            .join("project.json"),
    )
    .expect("read project metadata");
    let project: Value = serde_json::from_slice(&project).expect("parse project metadata");
    assert_eq!(project["human_checkout"], moved_str);

    let origin = fixture.git_stdout(&workspace, ["remote", "get-url", "origin"]);
    assert_eq!(origin.trim(), moved_str);
    let pushurl = fixture.git_stdout(&workspace, ["remote", "get-url", "--push", "origin"]);
    assert_eq!(pushurl.trim(), "agd-deny://push-disabled");
}

#[test]
fn doctor_repair_restores_workspace_guardrails() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["config", "commit.gpgsign", "true"]);
    fixture.git_in(&workspace, ["config", "--unset-all", "gpg.program"]);
    fixture.git_in(
        &workspace,
        [
            "remote",
            "set-url",
            "--push",
            "origin",
            fixture.human.to_str().unwrap(),
        ],
    );
    fs::remove_file(workspace.join(".git/hooks/pre-commit")).expect("remove pre-commit hook");
    fs::remove_file(workspace.join(".git/hooks/pre-push")).expect("remove pre-push hook");

    fixture
        .agd()
        .args(["doctor", "--repair"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Repaired workspace guardrails"));

    fixture
        .agd()
        .arg("doctor")
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("ok  signing disabled"))
        .stdout(predicate::str::contains("ok  deny signer"))
        .stdout(predicate::str::contains("ok  push disabled"))
        .stdout(predicate::str::contains("ok  protected branch hook"))
        .stdout(predicate::str::contains("ok  pre-push hook"));

    let signing = fixture.git_stdout(&workspace, ["config", "commit.gpgsign"]);
    assert_eq!(signing.trim(), "false");
    let deny_signer = fixture.git_stdout(&workspace, ["config", "gpg.program"]);
    assert!(std::path::Path::new(deny_signer.trim()).exists());
    let pushurl = fixture.git_stdout(&workspace, ["remote", "get-url", "--push", "origin"]);
    assert_eq!(pushurl.trim(), "agd-deny://push-disabled");
    assert!(workspace.join(".git/hooks/pre-commit").exists());
    assert!(workspace.join(".git/hooks/pre-push").exists());
}

#[test]
fn doctor_detects_and_repairs_broken_workspace_origin() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    let stale_origin = fixture._tmp.path().join("stale-human");
    fixture.git_in(
        &workspace,
        [
            "remote",
            "set-url",
            "origin",
            stale_origin.to_str().expect("stale origin utf-8"),
        ],
    );

    fixture
        .agd()
        .arg("doctor")
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stdout(predicate::str::contains("fail workspace origin"))
        .stdout(predicate::str::contains("stale-human"))
        .stderr(predicate::str::contains("doctor found failed checks"));

    fixture
        .agd()
        .args(["doctor", "--repair"])
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("Repaired workspace origin"));

    fixture
        .agd()
        .arg("doctor")
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("ok  workspace origin"));

    let current_checkout = fixture.git_stdout(&fixture.human, ["rev-parse", "--show-toplevel"]);
    let origin = fixture.git_stdout(&workspace, ["remote", "get-url", "origin"]);
    assert_eq!(origin.trim(), current_checkout.trim());
}

#[test]
fn doctor_warns_when_submodules_are_declared() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.write_file(
        &fixture.human,
        ".gitmodules",
        "[submodule \"vendor/lib\"]\n\tpath = vendor/lib\n\turl = https://example.invalid/lib.git\n",
    );
    fixture.git(["add", ".gitmodules"]);
    fixture.git(["commit", "-m", "declare submodule"]);

    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();

    fixture
        .agd()
        .arg("doctor")
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("warn submodules"))
        .stdout(predicate::str::contains(".gitmodules"));
}

#[test]
fn doctor_warns_when_lfs_filters_are_declared() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.write_file(
        &fixture.human,
        ".gitattributes",
        "*.bin filter=lfs diff=lfs merge=lfs -text\n",
    );
    fixture.git(["add", ".gitattributes"]);
    fixture.git(["commit", "-m", "declare lfs filters"]);

    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();

    fixture
        .agd()
        .arg("doctor")
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("warn Git LFS"))
        .stdout(predicate::str::contains("filter=lfs"));
}

#[test]
fn doctor_warns_when_lfs_pointer_files_are_tracked() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.write_file(
        &fixture.human,
        "pointer.bin",
        "version https://git-lfs.github.com/spec/v1\noid sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\nsize 123\n",
    );
    fixture.git(["add", "pointer.bin"]);
    fixture.git(["commit", "-m", "track lfs pointer"]);

    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();

    fixture
        .agd()
        .arg("doctor")
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("warn Git LFS"))
        .stdout(predicate::str::contains("pointer.bin"))
        .stdout(predicate::str::contains("pointer"));
}

#[test]
fn init_and_doctor_ignore_files_that_only_mention_lfs_pointer_text() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture.write_file(
        &fixture.human,
        "docs/lfs-notes.md",
        "This note documents the fixture format.\n\
version https://git-lfs.github.com/spec/v1\n\
oid sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
size 123\n\
This file is not a pointer.\n",
    );
    fixture.git(["add", "docs/lfs-notes.md"]);
    fixture.git(["commit", "-m", "document lfs pointer fixture"]);

    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stderr(predicate::str::contains("warn Git LFS").not());

    fixture
        .agd()
        .arg("doctor")
        .current_dir(&fixture.human)
        .assert()
        .success()
        .stdout(predicate::str::contains("warn Git LFS").not())
        .stdout(predicate::str::contains("looks like Git LFS pointer").not());
}

#[test]
fn json_doctor_outputs_checks() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    fixture.git_in(&workspace, ["config", "commit.gpgsign", "true"]);

    let output = fixture
        .agd()
        .args(["--json", "doctor"])
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stderr(predicate::str::contains("doctor found failed checks"))
        .get_output()
        .stdout
        .clone();
    let report: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(report["failed"], true);

    let checks = report["checks"].as_array().expect("checks array");
    assert!(checks.iter().any(|check| {
        check["name"] == "signing disabled"
            && check["status"] == "fail"
            && check["detail"]
                .as_str()
                .is_some_and(|detail| detail.contains("expected false") && detail.contains("true"))
    }));
    assert!(checks
        .iter()
        .any(|check| check["name"] == "project metadata" && check["status"] == "ok"));
}

#[test]
fn doctor_reports_missing_project_metadata() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();

    let metadata = fixture
        .agd_home
        .join("projects")
        .join(fixture.project_id())
        .join("project.json");
    fs::remove_file(metadata).expect("remove project metadata");

    fixture
        .agd()
        .arg("doctor")
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stdout(predicate::str::contains("fail project metadata"))
        .stdout(predicate::str::contains("missing"))
        .stderr(predicate::str::contains("doctor found failed checks"));
}

#[test]
fn doctor_reports_missing_project_metadata_from_agent_workspace() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();

    let metadata = fixture
        .agd_home
        .join("projects")
        .join(fixture.project_id())
        .join("project.json");
    fs::remove_file(metadata).expect("remove project metadata");

    fixture
        .agd()
        .arg("doctor")
        .current_dir(&workspace)
        .assert()
        .failure()
        .stdout(predicate::str::contains("fail project metadata"))
        .stdout(predicate::str::contains("missing"))
        .stderr(predicate::str::contains("doctor found failed checks"));
}

#[test]
fn doctor_reports_missing_workspace_marker() {
    let fixture = Fixture::new();
    fixture.init_human_repo();
    fixture
        .agd()
        .arg("init")
        .current_dir(&fixture.human)
        .assert()
        .success();
    let workspace = fixture.agd_path();
    fs::remove_file(workspace.join(".agd/workspace.json")).expect("remove workspace marker");

    fixture
        .agd()
        .arg("doctor")
        .current_dir(&fixture.human)
        .assert()
        .failure()
        .stdout(predicate::str::contains("fail workspace marker"))
        .stderr(predicate::str::contains("doctor found failed checks"));
}
