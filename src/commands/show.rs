use anyhow::Result;
use colored::Colorize;

use crate::intent;

pub fn run(id: &str, json: bool) -> Result<()> {
    // Try as regular intent first, then parent
    if let Ok(record) = intent::read_intent(id) {
        if json {
            println!("{}", serde_json::to_string_pretty(&record)?);
        } else {
            print_intent(&record);
        }
        return Ok(());
    }

    if let Ok(parent) = intent::read_parent_intent(id) {
        if json {
            println!("{}", serde_json::to_string_pretty(&parent)?);
        } else {
            print_parent(&parent);
        }
        return Ok(());
    }

    anyhow::bail!("Intent {} not found", id);
}

fn print_intent(r: &intent::IntentRecord) {
    println!("{} {}", "Intent:".dimmed(), r.id.cyan());
    println!("{} {}", "Goal:".dimmed(), r.goal.bold());
    println!("{} {}", "Approach:".dimmed(), r.approach);

    if !r.alternatives.is_empty() {
        println!("{}", "Alternatives considered:".dimmed());
        for alt in &r.alternatives {
            println!(
                "  {} {} — {}",
                "•".dimmed(),
                alt.option.yellow(),
                alt.rejected_because.dimmed()
            );
        }
    }

    if !r.files_modified.is_empty() {
        println!("{} {}", "Files modified:".dimmed(), r.files_modified.join(", "));
    }
    if !r.context_files.is_empty() {
        println!("{} {}", "Context files:".dimmed(), r.context_files.join(", "));
    }

    if let Some(conf) = r.confidence {
        let conf_str = format!("{:.0}%", conf * 100.0);
        let colored = if conf >= 0.8 {
            conf_str.green()
        } else if conf >= 0.5 {
            conf_str.yellow()
        } else {
            conf_str.red()
        };
        println!("{} {}", "Confidence:".dimmed(), colored);
    }

    if let Some(cost) = r.token_cost {
        println!("{} {} tokens", "Token cost:".dimmed(), cost);
    }
    if let Some(ref agent) = r.agent {
        println!("{} {}", "Agent:".dimmed(), agent);
    }
    if let Some(ref model) = r.model {
        println!("{} {}", "Model:".dimmed(), model);
    }
    if let Some(ref parent) = r.parent_intent {
        println!("{} {}", "Parent:".dimmed(), parent.cyan());
    }
    println!("{} {}", "Date:".dimmed(), r.timestamp.dimmed());
}

fn print_parent(p: &intent::ParentIntent) {
    println!("{} {}", "Parent:".dimmed(), p.id.cyan());
    println!("{} {}", "Goal:".dimmed(), p.goal.bold());
    if p.child_intents.is_empty() {
        println!("{} {}", "Children:".dimmed(), "none".dimmed());
    } else {
        println!("{}", "Children:".dimmed());
        for child in &p.child_intents {
            let goal = intent::read_intent(child)
                .map(|r| r.goal)
                .unwrap_or_else(|_| "unknown".to_string());
            println!("  {} {} — {}", "•".dimmed(), child.cyan(), goal);
        }
    }
    println!("{} {}", "Date:".dimmed(), p.timestamp.dimmed());
}
