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
fn pr_command_pushes_agent_branch_and_invokes_gh() {
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

    fixture.git_in(&workspace, ["switch", "-c", "agent/pr-test"]);
    fixture.write_file(&workspace, "pr.txt", "agent PR work\n");
    fixture.git_in(&workspace, ["add", "pr.txt"]);
    fixture.git_in(&workspace, ["commit", "-m", "agent PR work"]);

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
    assert!(gh_args.contains("agent PR work"));
    assert!(gh_args.contains("pr.txt"));
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
        .stdout(predicate::str::contains("ok  pre-push hook"));

    let signing = fixture.git_stdout(&workspace, ["config", "commit.gpgsign"]);
    assert_eq!(signing.trim(), "false");
    let deny_signer = fixture.git_stdout(&workspace, ["config", "gpg.program"]);
    assert!(std::path::Path::new(deny_signer.trim()).exists());
    let pushurl = fixture.git_stdout(&workspace, ["remote", "get-url", "--push", "origin"]);
    assert_eq!(pushurl.trim(), "agd-deny://push-disabled");
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
