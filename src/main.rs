mod adoption;
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
        Some(Command::Sync { rebase }) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            let result = sync::sync(&paths, context.project(), rebase.as_deref())?;
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
        Some(Command::Branches) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            if json {
                json_output::branches(review::branch_names(context.project())?)?;
            } else {
                review::branches(context.project())?;
            }
        }
        Some(Command::Log { branch }) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            if json {
                let log = review::commit_log(context.project(), branch.as_deref(), &cwd)?;
                json_output::print(&log)?;
            } else {
                review::log(context.project(), branch.as_deref(), &cwd)?;
            }
        }
        Some(Command::Diff { branch }) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            if json {
                let diff = review::branch_diff(context.project(), branch.as_deref(), &cwd)?;
                json_output::print(&diff)?;
            } else {
                review::diff(context.project(), branch.as_deref(), &cwd)?;
            }
        }
        Some(Command::Pr { branch }) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            let result = pull_request::open(context.project(), branch.as_deref(), &cwd)?;
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
        Some(Command::Files { branch }) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            if json {
                json_output::files(review::changed_files(
                    context.project(),
                    branch.as_deref(),
                    &cwd,
                )?)?;
            } else {
                review::files(context.project(), branch.as_deref(), &cwd)?;
            }
        }
        Some(Command::Discard { branch }) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            let result = cleanup::discard(&paths, context.project(), &branch)?;
            if json {
                json_output::print(&result)?;
            } else {
                println!("Discarded {}", result.branch);
            }
        }
        Some(Command::ResetWorkspace) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            let result = cleanup::reset_workspace(&paths, context.project())?;
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
            r#continue,
            abort,
        }) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            if r#continue {
                if abort || branch.is_some() || preserve || merge {
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
                if branch.is_some() || preserve || merge {
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
            let Some(branch) = branch else {
                bail!("bless requires a branch, or use bless --abort");
            };
            if preserve && merge {
                bail!("choose only one bless adoption mode");
            }
            let mode = if merge {
                adoption::AdoptionMode::Merge
            } else if preserve {
                adoption::AdoptionMode::Preserve
            } else {
                adoption::AdoptionMode::Squash
            };
            let result = adoption::bless(&paths, context.project(), branch.as_str(), mode)?;
            if json {
                json_output::print(&result)?;
            } else {
                match result.adoption {
                    "merge" => println!("Merged {}", result.branch),
                    "preserve" => println!("Preserved {}", result.branch),
                    _ => println!("Blessed {}", result.branch),
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
