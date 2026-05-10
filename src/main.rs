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
            println!("Initialized AGD.");
            println!("Human checkout:");
            println!("  {}", project.human_checkout.display());
            println!("Agent workspace:");
            println!("  {}", workspace_path.display());
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
            let context = project::discover(&paths, &cwd)?;
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
        Some(Command::Sync) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            sync::sync(&paths, context.project())?;
        }
        Some(Command::Handoff { include_untracked }) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            handoff::handoff(&paths, context.project(), &include_untracked)?;
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
            review::log(context.project(), branch.as_deref(), &cwd)?;
        }
        Some(Command::Diff { branch }) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            review::diff(context.project(), branch.as_deref(), &cwd)?;
        }
        Some(Command::Pr { branch }) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            pull_request::open(context.project(), branch.as_deref(), &cwd)?;
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
            cleanup::discard(&paths, context.project(), &branch)?;
        }
        Some(Command::ResetWorkspace) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            cleanup::reset_workspace(&paths, context.project())?;
        }
        Some(Command::Workspace { command }) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            match command {
                WorkspaceCommand::List => workspace_commands::list(context.project()),
                WorkspaceCommand::Create { name } => {
                    let mut project = context.project().clone();
                    workspace_commands::create(&paths, &mut project, &name)?;
                    project::save_project(&paths, &project)?;
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
                adoption::continue_bless(&paths, context.project())?;
                return Ok(());
            }
            if abort {
                if branch.is_some() || preserve || merge {
                    bail!("bless --abort cannot be combined with branch adoption options");
                }
                adoption::abort(context.project())?;
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
            adoption::bless(&paths, context.project(), branch.as_str(), mode)?;
        }
        Some(Command::DenySigner { _args: _ }) => {
            guardrails::deny_signing(&paths)?;
        }
        None => {}
    }

    Ok(())
}
