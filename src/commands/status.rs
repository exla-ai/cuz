use anyhow::Result;
use colored::Colorize;

use crate::git;
use crate::intent;

pub fn run() -> Result<()> {
    let cuz_dir = intent::find_cuz_dir()?;

    // Count intent files
    let intents = intent::list_intents(&cuz_dir)?;
    let intent_count = intents.len();

    // Active parent
    let active_parent = intent::read_active_parent()?;

    // Coverage: commits in last 30 days
    let total_commits = git::commit_count_since("30 days ago")?;
    let intent_commits = git::intent_commit_count_since("30 days ago")?;
    let coverage = if total_commits > 0 {
        (intent_commits as f64 / total_commits as f64) * 100.0
    } else {
        0.0
    };

    // Print status
    println!("{}", "cuz status".bold());
    println!();
    println!("  {} {}", "Intent records:".dimmed(), intent_count);

    if let Some(ref parent_id) = active_parent {
        let goal = intent::read_parent_intent(parent_id)
            .map(|p| p.goal)
            .unwrap_or_else(|_| "unknown".to_string());
        println!(
            "  {} {} — {}",
            "Active parent:".dimmed(),
            parent_id.cyan(),
            goal
        );
    } else {
        println!("  {} {}", "Active parent:".dimmed(), "none".dimmed());
    }

    println!();
    println!("  {} (last 30 days)", "Coverage".dimmed());
    println!(
        "  {} / {} commits tracked",
        intent_commits, total_commits
    );

    // ASCII progress bar
    let bar_width = 30;
    let filled = ((coverage / 100.0) * bar_width as f64).round() as usize;
    let empty = bar_width - filled;
    let bar = format!(
        "[{}{}] {:.0}%",
        "█".repeat(filled).green(),
        "░".repeat(empty),
        coverage
    );
    println!("  {}", bar);

    Ok(())
}
