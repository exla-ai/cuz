use anyhow::Result;
use colored::Colorize;

use crate::intent;

pub fn run(query: &str, json: bool) -> Result<()> {
    let cuz_dir = intent::find_cuz_dir()?;
    let ids = intent::list_intents(&cuz_dir)?;

    let query_lower = query.to_lowercase();
    let mut matches = Vec::new();

    for id in &ids {
        if let Ok(record) = intent::read_intent(id) {
            let searchable = format!(
                "{} {} {} {}",
                record.goal,
                record.approach,
                record.alternatives.iter().map(|a| format!("{} {}", a.option, a.rejected_because)).collect::<Vec<_>>().join(" "),
                record.files_modified.join(" "),
            )
            .to_lowercase();

            if searchable.contains(&query_lower) {
                matches.push(record);
            }
        }
    }

    if matches.is_empty() {
        println!("{}", "No matching intents found.".dimmed());
        return Ok(());
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&matches)?);
        return Ok(());
    }

    println!("{} result{}\n", matches.len(), if matches.len() == 1 { "" } else { "s" });

    for record in &matches {
        println!("{} — {}", record.id.cyan(), record.goal.bold());
        // Show matching context
        if record.approach.to_lowercase().contains(&query_lower) {
            println!("  {}", truncate(&record.approach, 80).dimmed());
        }
        // Highlight which alternative matched
        for alt in &record.alternatives {
            let alt_text = format!("{} {}", alt.option, alt.rejected_because).to_lowercase();
            if alt_text.contains(&query_lower) {
                intent::print_alternative(alt, "  ");
            }
        }
        if !record.files_modified.is_empty() {
            println!("  {}", record.files_modified.join(", ").dimmed());
        }
        println!();
    }

    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}
