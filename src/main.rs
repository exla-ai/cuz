mod commands;
mod git;
mod intent;
mod prompt;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cuz", version, about = "Give every piece of code a traceable reason for existing")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Patch ~/.claude/CLAUDE.md and install PostToolUse hook
    Setup,
    /// PostToolUse hook — verify intent trailers on commits
    Verify,
    /// Show why a line of code exists
    Why {
        /// Target in file:line format (line defaults to 1)
        target: String,
    },
    /// Show intent history
    Log {
        /// Maximum number of intents to show
        #[arg(short = 'n', long, default_value = "20")]
        limit: u32,
        /// Show all intents (ignore limit)
        #[arg(long)]
        all: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show intent tracking status for this repo
    Status,
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Setup => commands::setup::run(),
        Commands::Verify => commands::verify::run(),
        Commands::Why { target } => commands::why::run(&target),
        Commands::Log { limit, all, json } => commands::log::run(limit, all, json),
        Commands::Status => commands::status::run(),
    };

    if let Err(e) = result {
        eprintln!("error: {:#}", e);
        std::process::exit(1);
    }
}
