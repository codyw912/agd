mod adoption;
mod cli;
mod git;
mod guardrails;
mod output;
mod paths;
mod project;
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
