use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "agd", about = "Agent Git Delegation")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Init,
    Path,
    Status,
    Doctor,
    Sync,
    Branches,
    Log {
        branch: Option<String>,
    },
    Diff {
        branch: Option<String>,
    },
    Files {
        branch: Option<String>,
    },
    Discard {
        branch: String,
    },
    ResetWorkspace,
    Bless {
        branch: String,
    },
    #[command(hide = true)]
    DenySigner {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        _args: Vec<String>,
    },
}
