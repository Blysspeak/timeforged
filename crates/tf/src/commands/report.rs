use chrono::Utc;
use colored::Colorize;
use comfy_table::{Cell, Table};

use timeforged_core::models::Summary;

use crate::client::TfClient;

#[derive(Debug, Clone)]
pub enum Range {
    Today,
    Yesterday,
    Week,
    Month,
    Custom { from: String, to: String },
}

pub async fn run(client: &TfClient, range: Range, project: Option<&str>) {
    let (from, to) = resolve_range(&range);

    let mut query: Vec<(&str, &str)> = vec![("from", &from), ("to", &to)];
    if let Some(p) = project {
        query.push(("project", p));
    }

    match client
        .get_with_query::<Summary>("/api/v1/reports/summary", &query)
        .await
    {
        Ok(summary) => print_summary(&summary, &range),
        Err(e) => {
            eprintln!("{}: {e}", "error".red());
            std::process::exit(1);
        }
    }
}

fn resolve_range(range: &Range) -> (String, String) {
    let now = Utc::now();
    match range {
        Range::Today => {
            let today = now.date_naive();
            (
                format!("{today}T00:00:00Z"),
                format!("{today}T23:59:59Z"),
            )
        }
        Range::Yesterday => {
            let yesterday = now.date_naive() - chrono::Duration::days(1);
            (
                format!("{yesterday}T00:00:00Z"),
                format!("{yesterday}T23:59:59Z"),
            )
        }
        Range::Week => {
            let from = now - chrono::Duration::days(7);
            (from.to_rfc3339(), now.to_rfc3339())
        }
        Range::Month => {
            let from = now - chrono::Duration::days(30);
            (from.to_rfc3339(), now.to_rfc3339())
        }
        Range::Custom { from, to } => (from.clone(), to.clone()),
    }
}

fn print_summary(summary: &Summary, range: &Range) {
    let hours = summary.total_seconds / 3600.0;
    let mins = (summary.total_seconds % 3600.0) / 60.0;

    let label = match range {
        Range::Today => "Today",
        Range::Yesterday => "Yesterday",
        Range::Week => "Last 7 days",
        Range::Month => "Last 30 days",
        Range::Custom { .. } => "Custom range",
    };

    println!("{}", format!("Report — {label}").bold());
    println!(
        "  Total: {}",
        format!("{:.0}h {:.0}m", hours, mins).green()
    );

    if !summary.projects.is_empty() {
        println!("\n{}", "Projects".bold());
        let mut table = Table::new();
        table.set_header(vec!["Project", "Time", "%"]);
        for p in &summary.projects {
            let ph = p.total_seconds / 3600.0;
            let pm = (p.total_seconds % 3600.0) / 60.0;
            table.add_row(vec![
                Cell::new(&p.name),
                Cell::new(format!("{:.0}h {:.0}m", ph, pm)),
                Cell::new(format!("{:.0}%", p.percent)),
            ]);
        }
        println!("{table}");
    }

    if !summary.languages.is_empty() {
        println!("\n{}", "Languages".bold());
        let mut table = Table::new();
        table.set_header(vec!["Language", "Time", "%"]);
        for l in &summary.languages {
            let lh = l.total_seconds / 3600.0;
            let lm = (l.total_seconds % 3600.0) / 60.0;
            table.add_row(vec![
                Cell::new(&l.name),
                Cell::new(format!("{:.0}h {:.0}m", lh, lm)),
                Cell::new(format!("{:.0}%", l.percent)),
            ]);
        }
        println!("{table}");
    }

    if !summary.days.is_empty() {
        println!("\n{}", "Daily Breakdown".bold());
        let mut table = Table::new();
        table.set_header(vec!["Date", "Time"]);
        for d in &summary.days {
            let dh = d.total_seconds / 3600.0;
            let dm = (d.total_seconds % 3600.0) / 60.0;
            table.add_row(vec![
                Cell::new(d.date.to_string()),
                Cell::new(format!("{:.0}h {:.0}m", dh, dm)),
            ]);
        }
        println!("{table}");
    }
}
