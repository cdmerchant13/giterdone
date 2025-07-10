use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initializes the configuration wizard
    Init,
    /// Runs a backup immediately
    RunNow,
    /// Simulates a backup without committing or pushing
    DryRun,
    /// Shows the current configuration
    Status,
}
