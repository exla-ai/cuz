use anyhow::Result;
use colored::Colorize;

use crate::intent;

pub fn run(file: &str) -> Result<()> {
    let intents = intent::intents_for_file(file)?;

    let mut found_any = false;
    for record in &intents {
        for alt in &record.alternatives {
            if !found_any {
                println!(
                    "{}\n",
                    format!("Rejected alternatives for {}:", file).bold()
                );
                found_any = true;
            }
            intent::print_alternative(alt, "  ");
            println!(
                "    {} {} — {}",
                "from".dimmed(),
                record.id.cyan(),
                record.goal.dimmed()
            );
        }
    }

    if !found_any {
        println!("{}", "No rejected alternatives found.".dimmed());
    }

    Ok(())
}
