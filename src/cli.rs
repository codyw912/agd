use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "agd", about = "Agent Git Delegation")]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Init,
    Path {
        #[arg(long)]
        workspace: Option<String>,
    },
    Status,
    Identity,
    Doctor {
        #[arg(long)]
        repair: bool,
    },
    Sync,
    Handoff {
        #[arg(long = "include-untracked")]
        include_untracked: Vec<PathBuf>,
    },
    Branches,
    Log {
        branch: Option<String>,
    },
    Diff {
        branch: Option<String>,
    },
    Pr {
        branch: Option<String>,
    },
    Shell,
    Files {
        branch: Option<String>,
    },
    Discard {
        branch: String,
    },
    ResetWorkspace,
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommand,
    },
    Bless {
        branch: Option<String>,
        #[arg(long)]
        preserve: bool,
        #[arg(long)]
        merge: bool,
        #[arg(long = "continue")]
        r#continue: bool,
        #[arg(long)]
        abort: bool,
    },
    #[command(hide = true)]
    DenySigner {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        _args: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum WorkspaceCommand {
    List,
    Create { name: String },
}
