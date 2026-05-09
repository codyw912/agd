use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
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
    diverged.git_in(&workspace, ["commit", "-m", "workspace main update"]);
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
        ));
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
