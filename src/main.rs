mod adoption;
mod branch_policy;
mod cleanup;
mod cli;
mod doctor;
mod git;
mod guardrails;
mod handoff;
mod identity;
mod json_output;
mod operation_lock;
mod output;
mod paths;
mod project;
mod provenance;
mod pull_request;
mod review;
mod shell_command;
mod status_report;
mod sync;
mod verify;
mod workspace;
mod workspace_commands;

use anyhow::{bail, Result};
use clap::Parser;
use cli::{Command, WorkspaceCommand};

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    let json = cli.json;
    let paths = paths::AgdPaths::from_env()?;

    match cli.command {
        Some(Command::Init) => {
            let cwd = std::env::current_dir()?;
            let mut project = project::init_project(&paths, &cwd)?;
            let workspace_path = workspace::ensure_default_workspace(&paths, &mut project)?;
            project::save_project(&paths, &project)?;
            let context = project::ProjectContext::HumanCheckout(project.clone());
            let report = doctor::report(&paths, &context, &cwd);
            doctor::print_warnings(&report);
            if json {
                let workspace = json_output::default_workspace(&project)?;
                json_output::init(&project, workspace)?;
            } else {
                println!("Initialized AGD.");
                println!("Human checkout:");
                println!("  {}", project.human_checkout.display());
                println!("Agent workspace:");
                println!("  {}", workspace_path.display());
            }
        }
        Some(Command::Path { workspace }) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            let project = context.project();
            let workspace = match workspace.as_deref() {
                Some(workspace) => project
                    .workspaces
                    .iter()
                    .find(|candidate| candidate.id == workspace)
                    .ok_or_else(|| anyhow::anyhow!("workspace not found: {workspace}"))?,
                None => json_output::default_workspace(project)?,
            };
            if json {
                json_output::path(workspace)?;
            } else {
                println!("{}", workspace.path.display());
            }
        }
        Some(Command::Status) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            if json {
                json_output::status(&context, &cwd)?;
            } else {
                output::status(&context, &cwd)?;
            }
        }
        Some(Command::Identity) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            if json {
                json_output::identity(&context.project().agent_identity)?;
            } else {
                identity::show(context.project());
            }
        }
        Some(Command::Doctor { repair }) => {
            let cwd = std::env::current_dir()?;
            let context = match project::discover(&paths, &cwd) {
                Ok(context) => context,
                Err(error) if !repair => {
                    if let Some(report) = doctor::metadata_failure_report(&paths, &cwd) {
                        if json {
                            json_output::print(&report)?;
                            doctor::ensure_passed(&report)?;
                        } else {
                            doctor::print_report(&report)?;
                        }
                        return Ok(());
                    }
                    return Err(error);
                }
                Err(error) => return Err(error),
            };
            if repair {
                if json {
                    bail!("doctor --repair does not support --json");
                }
                doctor::repair(&paths, &context, &cwd)?;
            } else if json {
                let report = doctor::report(&paths, &context, &cwd);
                json_output::print(&report)?;
                doctor::ensure_passed(&report)?;
            } else {
                doctor::doctor(&paths, &context, &cwd)?;
            }
        }
        Some(Command::Sync { workspace, rebase }) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            let workspace = selected_workspace(&context, workspace.as_deref());
            let rebase = resolve_sync_rebase(&context, &cwd, rebase)?;
            let result = sync::sync(&paths, context.project(), workspace, rebase.as_deref())?;
            if json {
                json_output::print(&result)?;
            } else {
                println!("Synced {}", result.target);
                if let Some(rebase) = &result.rebase {
                    println!("Rebased {}", rebase.branch);
                }
            }
        }
        Some(Command::Handoff { include_untracked }) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            let result = handoff::handoff(&paths, context.project(), &include_untracked)?;
            if json {
                json_output::print(&result)?;
            } else if result.status == "noop" {
                println!("No human changes to hand off");
            } else {
                println!("Handed off human changes to {}", result.workspace_id);
            }
        }
        Some(Command::Branches { workspace }) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            let workspace = selected_workspace(&context, workspace.as_deref());
            if json {
                json_output::branches(review::branch_names(context.project(), workspace)?)?;
            } else {
                review::branches(context.project(), workspace)?;
            }
        }
        Some(Command::Log { workspace, branch }) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            let workspace = selected_workspace(&context, workspace.as_deref());
            if json {
                let log =
                    review::commit_log(context.project(), workspace, branch.as_deref(), &cwd)?;
                json_output::print(&log)?;
            } else {
                review::log(context.project(), workspace, branch.as_deref(), &cwd)?;
            }
        }
        Some(Command::Diff { workspace, branch }) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            let workspace = selected_workspace(&context, workspace.as_deref());
            if json {
                let diff =
                    review::branch_diff(context.project(), workspace, branch.as_deref(), &cwd)?;
                json_output::print(&diff)?;
            } else {
                review::diff(context.project(), workspace, branch.as_deref(), &cwd)?;
            }
        }
        Some(Command::Pr {
            bless,
            r#continue,
            target,
            adoption_branch,
            branch,
        }) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            let result = if r#continue {
                if bless || target.is_some() || adoption_branch.is_some() || branch.is_some() {
                    bail!("pr --continue cannot be combined with other pr options");
                }
                pull_request::continue_blessed(&paths, context.project())?
            } else if bless {
                pull_request::open_blessed(
                    &paths,
                    context.project(),
                    branch.as_deref(),
                    target,
                    adoption_branch,
                    &cwd,
                )?
            } else {
                if target.is_some() || adoption_branch.is_some() {
                    bail!("pr --target and --branch require --bless");
                }
                pull_request::open(context.project(), branch.as_deref(), &cwd)?
            };
            if json {
                json_output::print(&result)?;
            } else if let Some(url) = result.url {
                println!("{url}");
            } else if let Some(next_step) = result.next_step {
                println!("{next_step}");
            } else {
                bail!("pull request result did not include a URL or next step");
            }
        }
        Some(Command::Verify { commit }) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            if json {
                let report = verify::report(context.project(), &commit)?;
                json_output::print(&report)?;
                verify::ensure_verified(&report)?;
            } else {
                verify::verify(context.project(), &commit)?;
            }
        }
        Some(Command::Shell) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            shell_command::run(context.project())?;
        }
        Some(Command::Files { workspace, branch }) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            let workspace = selected_workspace(&context, workspace.as_deref());
            if json {
                json_output::files(review::changed_files(
                    context.project(),
                    workspace,
                    branch.as_deref(),
                    &cwd,
                )?)?;
            } else {
                review::files(context.project(), workspace, branch.as_deref(), &cwd)?;
            }
        }
        Some(Command::Discard {
            branch,
            force,
            workspace,
        }) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            let result = cleanup::discard(
                &paths,
                context.project(),
                &branch,
                force,
                workspace.as_deref(),
            )?;
            if json {
                json_output::print(&result)?;
            } else {
                println!("Discarded {}", result.branch);
            }
        }
        Some(Command::ResetWorkspace { force, workspace }) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            let result =
                cleanup::reset_workspace(&paths, context.project(), force, workspace.as_deref())?;
            if json {
                json_output::print(&result)?;
            } else {
                println!("Reset workspace");
                println!("  {}", result.path.display());
            }
        }
        Some(Command::Workspace { command }) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            match command {
                WorkspaceCommand::List => {
                    if json {
                        json_output::workspaces(context.project())?;
                    } else {
                        workspace_commands::list(context.project());
                    }
                }
                WorkspaceCommand::Create { name } => {
                    let mut project = context.project().clone();
                    if json {
                        workspace::ensure_workspace(&paths, &mut project, &name)?;
                        project::save_project(&paths, &project)?;
                        let workspace = project
                            .workspaces
                            .iter()
                            .find(|workspace| workspace.id == name)
                            .ok_or_else(|| anyhow::anyhow!("workspace not found: {name}"))?;
                        json_output::workspace_created(workspace)?;
                    } else {
                        workspace_commands::create(&paths, &mut project, &name)?;
                        project::save_project(&paths, &project)?;
                    }
                }
            }
        }
        Some(Command::Bless {
            branch,
            preserve,
            merge,
            target,
            adoption_branch,
            direct,
            r#continue,
            abort,
        }) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            if r#continue {
                if abort
                    || branch.is_some()
                    || preserve
                    || merge
                    || target.is_some()
                    || adoption_branch.is_some()
                    || direct
                {
                    bail!("bless --continue cannot be combined with other bless options");
                }
                let result = adoption::continue_bless(&paths, context.project())?;
                if json {
                    json_output::print(&result)?;
                } else {
                    println!("Continued bless operation");
                }
                return Ok(());
            }
            if abort {
                if branch.is_some()
                    || preserve
                    || merge
                    || target.is_some()
                    || adoption_branch.is_some()
                    || direct
                {
                    bail!("bless --abort cannot be combined with branch adoption options");
                }
                let result = adoption::abort(context.project())?;
                if json {
                    json_output::print(&result)?;
                } else {
                    println!("Aborted bless operation");
                }
                return Ok(());
            }
            let branch = resolve_bless_branch(&context, &cwd, branch)?;
            if preserve && merge {
                bail!("choose only one bless adoption mode");
            }
            if direct && (target.is_some() || adoption_branch.is_some()) {
                bail!("bless --direct cannot be combined with --target or --branch");
            }
            let mode = if merge {
                adoption::AdoptionMode::Merge
            } else if preserve {
                adoption::AdoptionMode::Preserve
            } else {
                adoption::AdoptionMode::Squash
            };
            let target = if direct {
                adoption::AdoptionTarget::Direct
            } else {
                adoption::AdoptionTarget::Branch {
                    target_branch: target
                        .unwrap_or_else(|| context.project().default_target.clone()),
                    adoption_branch: adoption_branch
                        .unwrap_or_else(|| adoption::derive_adoption_branch(&branch)),
                }
            };
            let result = adoption::bless(&paths, context.project(), &branch, mode, target)?;
            if json {
                json_output::print(&result)?;
            } else {
                let location = result
                    .adoption_branch
                    .as_ref()
                    .map(|branch| format!(" onto {branch}"))
                    .unwrap_or_default();
                match result.adoption {
                    "merge" => println!("Merged {}{}", result.branch, location),
                    "preserve" => println!("Preserved {}{}", result.branch, location),
                    _ => println!("Blessed {}{}", result.branch, location),
                }
            }
        }
        Some(Command::DenySigner { _args: _ }) => {
            guardrails::deny_signing(&paths)?;
        }
        None => {}
    }

    Ok(())
}

fn selected_workspace<'a>(
    context: &'a project::ProjectContext,
    explicit: Option<&'a str>,
) -> Option<&'a str> {
    explicit.or_else(|| context.workspace_id())
}

fn resolve_bless_branch(
    context: &project::ProjectContext,
    cwd: &std::path::Path,
    branch: Option<String>,
) -> Result<String> {
    match branch {
        Some(branch) => Ok(branch),
        None => {
            match context {
                project::ProjectContext::HumanCheckout(_) => {
                    bail!("bless requires a branch, or run it from an agent workspace on an agent branch");
                }
                project::ProjectContext::AgentWorkspace { project, .. } => current_agent_branch(
                    project,
                    cwd,
                    "current agent branch is required for bless without a branch",
                ),
            }
        }
    }
}

fn current_agent_branch(
    project: &project::Project,
    cwd: &std::path::Path,
    error: &str,
) -> Result<String> {
    let branch = git::stdout(cwd, ["branch", "--show-current"])?;
    let branch = branch.trim().to_string();
    if !branch_policy::is_agent_branch(project, &branch) {
        bail!("{error}");
    }
    Ok(branch)
}

fn resolve_sync_rebase(
    context: &project::ProjectContext,
    cwd: &std::path::Path,
    rebase: Option<Option<String>>,
) -> Result<Option<String>> {
    match rebase {
        None => Ok(None),
        Some(Some(branch)) => Ok(Some(branch)),
        Some(None) => match context {
            project::ProjectContext::HumanCheckout(_) => {
                bail!("sync --rebase without a branch must be run from an agent workspace");
            }
            project::ProjectContext::AgentWorkspace { project, .. } => {
                Ok(Some(current_agent_branch(
                    project,
                    cwd,
                    "current agent branch is required for sync --rebase without a branch",
                )?))
            }
        },
    }
}
