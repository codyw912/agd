use assert_cmd::Command;
use predicates::prelude::*;
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
