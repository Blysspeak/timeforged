use chrono::Utc;
use colored::Colorize;

use timeforged_core::models::Summary;

use crate::client::TfClient;

pub async fn run(client: &TfClient) {
    let today = Utc::now().date_naive();
    let from = format!("{}T00:00:00Z", today);
    let to = format!("{}T23:59:59Z", today);

    match client
        .get_with_query::<Summary>("/api/v1/reports/summary", &[("from", &from), ("to", &to)])
        .await
    {
        Ok(summary) => {
            let hours = summary.total_seconds / 3600.0;
            let mins = (summary.total_seconds % 3600.0) / 60.0;

            println!("{}", format!("Today — {today}").bold());
            println!(
                "  Total: {}",
                format!("{:.0}h {:.0}m", hours, mins).green()
            );

            if !summary.projects.is_empty() {
                println!("\n  {}", "Projects:".bold());
                for p in &summary.projects {
                    let ph = p.total_seconds / 3600.0;
                    let pm = (p.total_seconds % 3600.0) / 60.0;
                    println!(
                        "    {:<20} {:>5.0}h {:>2.0}m  ({:.0}%)",
                        p.name, ph, pm, p.percent
                    );
                }
            }

            if !summary.languages.is_empty() {
                println!("\n  {}", "Languages:".bold());
                for l in &summary.languages {
                    let lh = l.total_seconds / 3600.0;
                    let lm = (l.total_seconds % 3600.0) / 60.0;
                    println!(
                        "    {:<20} {:>5.0}h {:>2.0}m  ({:.0}%)",
                        l.name, lh, lm, l.percent
                    );
                }
            }
        }
        Err(e) => {
            eprintln!("{}: {e}", "error".red());
            std::process::exit(1);
        }
    }
}
