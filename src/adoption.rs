use crate::git;
use crate::operation_lock::OperationLock;
use crate::paths::AgdPaths;
use crate::project::Project;
use anyhow::{Context, Result};
use std::ffi::OsString;
use std::path::Path;

pub enum AdoptionMode {
    Squash,
    Preserve,
    Merge,
}

pub fn bless(paths: &AgdPaths, project: &Project, branch: &str, mode: AdoptionMode) -> Result<()> {
    let prepared = prepare(paths, project, branch)?;
    match mode {
        AdoptionMode::Squash => bless_squash(project, &prepared),
        AdoptionMode::Preserve => bless_preserve(project, &prepared),
        AdoptionMode::Merge => bless_merge(project, &prepared),
    }
}

pub fn abort(project: &Project) -> Result<()> {
    git::run(&project.human_checkout, ["reset", "--merge"])?;
    println!("Aborted bless operation");
    Ok(())
}

struct PreparedAdoption<'a> {
    workspace: &'a crate::project::Workspace,
    branch: &'a str,
    fetched_ref: String,
    base: String,
    tip: String,
    _lock: OperationLock,
}

fn prepare<'a>(
    paths: &AgdPaths,
    project: &'a Project,
    branch: &'a str,
) -> Result<PreparedAdoption<'a>> {
    let workspace = project
        .workspaces
        .iter()
        .find(|workspace| workspace.id == project.default_workspace)
        .context("default workspace not found")?;

    require_clean(&project.human_checkout, "human checkout")?;
    require_clean(&workspace.path, "agent workspace")?;

    let lock = OperationLock::acquire(paths, &project.project_id, &workspace.id, "bless")?;
    let safety_ref = format!("refs/agd/safety/{}", lock.operation_id());
    git::run(&project.human_checkout, ["update-ref", &safety_ref, "HEAD"])?;

    let fetched_ref = format!("refs/agd/agent/{branch}");
    let fetch_spec = format!("refs/heads/{branch}:{fetched_ref}");
    git::run(
        &project.human_checkout,
        [
            OsString::from("fetch"),
            workspace.path.as_os_str().to_owned(),
            OsString::from(fetch_spec),
        ],
    )?;

    let base = git::stdout(
        &project.human_checkout,
        ["merge-base", "HEAD", &fetched_ref],
    )?;
    let tip = git::stdout(&project.human_checkout, ["rev-parse", &fetched_ref])?;

    Ok(PreparedAdoption {
        workspace,
        branch,
        fetched_ref,
        base: base.trim().to_string(),
        tip: tip.trim().to_string(),
        _lock: lock,
    })
}

fn bless_squash(project: &Project, prepared: &PreparedAdoption<'_>) -> Result<()> {
    if let Err(error) = git::run(
        &project.human_checkout,
        ["merge", "--squash", &prepared.fetched_ref],
    ) {
        anyhow::bail!(
            "bless squash failed: {error}\nrun `agd bless --abort` to restore the human checkout"
        );
    }

    let trailers = trailers(project, prepared, "squash");

    git::run(
        &project.human_checkout,
        [
            OsString::from("commit"),
            OsString::from("-S"),
            OsString::from("-m"),
            OsString::from(format!("Adopt {}", prepared.branch)),
            OsString::from("-m"),
            OsString::from(trailers),
        ],
    )?;

    println!("Blessed {}", prepared.branch);
    Ok(())
}

fn bless_merge(project: &Project, prepared: &PreparedAdoption<'_>) -> Result<()> {
    let trailers = trailers(project, prepared, "merge");
    git::run(
        &project.human_checkout,
        [
            OsString::from("merge"),
            OsString::from("--no-ff"),
            OsString::from("-S"),
            OsString::from(&prepared.fetched_ref),
            OsString::from("-m"),
            OsString::from(format!("Merge {}", prepared.branch)),
            OsString::from("-m"),
            OsString::from(trailers),
        ],
    )?;

    println!("Merged {}", prepared.branch);
    Ok(())
}

fn bless_preserve(project: &Project, prepared: &PreparedAdoption<'_>) -> Result<()> {
    let range = format!("{}..{}", project.default_target, prepared.fetched_ref);
    let commits = git::stdout(&project.human_checkout, ["rev-list", "--reverse", &range])?;
    let commits: Vec<_> = commits.lines().map(str::to_string).collect();
    if commits.is_empty() {
        anyhow::bail!("selected branch has no commits to preserve");
    }

    for commit in commits {
        git::run(
            &project.human_checkout,
            ["cherry-pick", "--no-commit", &commit],
        )?;
        git::run(
            &project.human_checkout,
            [
                OsString::from("commit"),
                OsString::from("-S"),
                OsString::from("-C"),
                OsString::from(&commit),
                OsString::from("--trailer"),
                OsString::from(format!("AGD-Project={}", project.project_id)),
                OsString::from("--trailer"),
                OsString::from(format!("AGD-Workspace={}", prepared.workspace.id)),
                OsString::from("--trailer"),
                OsString::from(format!("AGD-Agent-Branch={}", prepared.branch)),
                OsString::from("--trailer"),
                OsString::from(format!("AGD-Agent-Base={}", prepared.base)),
                OsString::from("--trailer"),
                OsString::from(format!("AGD-Agent-Tip={}", prepared.tip)),
                OsString::from("--trailer"),
                OsString::from(format!("AGD-Agent-Commit={commit}")),
                OsString::from("--trailer"),
                OsString::from("AGD-Adoption=preserve"),
            ],
        )?;
    }

    println!("Preserved {}", prepared.branch);
    Ok(())
}

fn trailers(project: &Project, prepared: &PreparedAdoption<'_>, adoption: &str) -> String {
    format!(
        "AGD-Project: {}\nAGD-Workspace: {}\nAGD-Agent-Branch: {}\nAGD-Agent-Base: {}\nAGD-Agent-Tip: {}\nAGD-Adoption: {}",
        project.project_id,
        prepared.workspace.id,
        prepared.branch,
        prepared.base,
        prepared.tip,
        adoption
    )
}

fn require_clean(repo: &Path, name: &str) -> Result<()> {
    let status = git::stdout(repo, ["status", "--porcelain"])?;
    if !status.trim().is_empty() {
        anyhow::bail!("{name} has uncommitted changes");
    }
    Ok(())
}
