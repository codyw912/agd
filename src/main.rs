mod adoption;
mod cleanup;
mod cli;
mod doctor;
mod git;
mod guardrails;
mod json_output;
mod operation_lock;
mod output;
mod paths;
mod project;
mod review;
mod sync;
mod workspace;

use anyhow::{bail, Result};
use clap::Parser;
use cli::Command;

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
        Some(Command::Path) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            let project = context.project();
            let workspace = json_output::default_workspace(project)?;
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
