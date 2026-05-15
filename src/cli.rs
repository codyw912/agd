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
    Sync {
        #[arg(long)]
        workspace: Option<String>,
        #[arg(long, num_args = 0..=1)]
        rebase: Option<Option<String>>,
    },
    Handoff {
        #[arg(long)]
        workspace: Option<String>,
        #[arg(long = "include-untracked")]
        include_untracked: Vec<PathBuf>,
    },
    Branches {
        #[arg(long)]
        workspace: Option<String>,
    },
    Log {
        #[arg(long)]
        workspace: Option<String>,
        branch: Option<String>,
    },
    Diff {
        #[arg(long)]
        workspace: Option<String>,
        branch: Option<String>,
    },
    Pr {
        #[arg(long)]
        bless: bool,
        #[arg(long = "continue")]
        r#continue: bool,
        #[arg(long)]
        workspace: Option<String>,
        #[arg(long)]
        target: Option<String>,
        #[arg(long = "branch")]
        adoption_branch: Option<String>,
        branch: Option<String>,
    },
    Verify {
        commit: String,
    },
    Shell,
    Files {
        #[arg(long)]
        workspace: Option<String>,
        branch: Option<String>,
    },
    Discard {
        #[arg(long)]
        force: bool,
        #[arg(long)]
        workspace: Option<String>,
        branch: String,
    },
    ResetWorkspace {
        #[arg(long)]
        force: bool,
        #[arg(long)]
        workspace: Option<String>,
    },
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommand,
    },
    Bless {
        branch: Option<String>,
        #[arg(long)]
        workspace: Option<String>,
        #[arg(long)]
        preserve: bool,
        #[arg(long)]
        merge: bool,
        #[arg(long)]
        target: Option<String>,
        #[arg(long = "branch")]
        adoption_branch: Option<String>,
        #[arg(long)]
        direct: bool,
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
