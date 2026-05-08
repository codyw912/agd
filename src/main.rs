mod adoption;
mod cleanup;
mod cli;
mod doctor;
mod git;
mod guardrails;
mod output;
mod paths;
mod project;
mod review;
mod sync;
mod workspace;

use anyhow::{Context, Result};
use clap::Parser;
use cli::Command;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
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
            let workspace = project
                .workspaces
                .iter()
                .find(|workspace| workspace.id == project.default_workspace)
                .context("default workspace not found")?;
            println!("{}", workspace.path.display());
        }
        Some(Command::Status) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            output::status(&context, &cwd)?;
        }
        Some(Command::Doctor) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            doctor::doctor(&paths, context.project())?;
        }
        Some(Command::Sync) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            sync::sync(context.project())?;
        }
        Some(Command::Branches) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            review::branches(context.project())?;
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
            review::files(context.project(), branch.as_deref(), &cwd)?;
        }
        Some(Command::Discard { branch }) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            cleanup::discard(context.project(), &branch)?;
        }
        Some(Command::ResetWorkspace) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            cleanup::reset_workspace(&paths, context.project())?;
        }
        Some(Command::Bless { branch }) => {
            let cwd = std::env::current_dir()?;
            let context = project::discover(&paths, &cwd)?;
            adoption::bless_squash(&paths, context.project(), &branch)?;
        }
        Some(Command::DenySigner { _args: _ }) => {
            guardrails::deny_signing(&paths)?;
        }
        None => {}
    }

    Ok(())
}
