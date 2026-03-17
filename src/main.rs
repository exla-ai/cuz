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
    /// Initialize .cuz/ in the current git repo
    Init,
    /// PostToolUse hook — verify intent trailers on commits
    Verify,
    /// Show why a line of code exists
    Why {
        /// Target in file:line format (line defaults to 1)
        target: String,
    },
    /// Show a specific intent by ID
    Show {
        /// Intent ID (e.g. cuz_8f3a1b or cuz_parent_f1a2b3)
        id: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Search intents by keyword
    Search {
        /// Search query
        query: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
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
    /// Show token cost across intents
    Cost {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show intents for changed files
    Diff {
        /// Show staged changes only
        #[arg(long)]
        cached: bool,
    },
    /// Manage multi-session parent intents
    Parent {
        #[command(subcommand)]
        action: ParentAction,
    },
    /// Remove cuz from ~/.claude/ (keeps .cuz/ data)
    Teardown,
}

#[derive(Subcommand)]
enum ParentAction {
    /// Start a new parent intent
    Start {
        /// Goal for this multi-session work
        goal: String,
    },
    /// End the active parent intent
    End,
    /// Show the active parent intent
    Show,
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Setup => commands::setup::run(),
        Commands::Init => commands::init::run(),
        Commands::Verify => commands::verify::run(),
        Commands::Why { target } => commands::why::run(&target),
        Commands::Show { id, json } => commands::show::run(&id, json),
        Commands::Search { query, json } => commands::search::run(&query, json),
        Commands::Log { limit, all, json } => commands::log::run(limit, all, json),
        Commands::Status => commands::status::run(),
        Commands::Cost { json } => commands::cost::run(json),
        Commands::Diff { cached } => commands::diff::run(cached),
        Commands::Parent { action } => match action {
            ParentAction::Start { goal } => commands::parent::run_start(&goal),
            ParentAction::End => commands::parent::run_end(),
            ParentAction::Show => commands::parent::run_show(),
        },
        Commands::Teardown => commands::teardown::run(),
    };

    if let Err(e) = result {
        eprintln!("error: {:#}", e);
        std::process::exit(1);
    }
}
