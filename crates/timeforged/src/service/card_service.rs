use chrono::{Datelike, Duration, NaiveDate};
use std::collections::HashMap;
use timeforged_core::models::Summary;

const LANG_COLORS: &[(&str, &str)] = &[
    ("Rust", "#dea584"),
    ("TypeScript", "#3178c6"),
    ("JavaScript", "#f1e05a"),
    ("Python", "#3572A5"),
    ("Go", "#00ADD8"),
    ("Java", "#b07219"),
    ("C", "#555555"),
    ("C++", "#f34b7d"),
    ("C#", "#178600"),
    ("Ruby", "#701516"),
    ("PHP", "#4F5D95"),
    ("Swift", "#F05138"),
    ("Kotlin", "#A97BFF"),
    ("Dart", "#00B4AB"),
    ("Lua", "#000080"),
    ("Shell", "#89e051"),
    ("HTML", "#e34c26"),
    ("CSS", "#563d7c"),
    ("SCSS", "#c6538c"),
    ("Vue", "#41b883"),
    ("Svelte", "#ff3e00"),
    ("JSON", "#292929"),
    ("YAML", "#cb171e"),
    ("TOML", "#9c4221"),
    ("Markdown", "#083fa1"),
    ("SQL", "#e38c00"),
    ("Dockerfile", "#384d54"),
    ("Zig", "#ec915c"),
    ("Elixir", "#6e4a7e"),
    ("Haskell", "#5e5086"),
    ("OCaml", "#3be133"),
    ("Scala", "#c22d40"),
    ("R", "#198CE7"),
    ("Nix", "#7e7eff"),
];

fn lang_color(name: &str) -> &str {
    LANG_COLORS
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, c)| *c)
        .unwrap_or("#8b949e")
}

/// Data formats and configs — not real programming languages
const EXCLUDED_LANGS: &[&str] = &[
    "JSON", "YAML", "TOML", "Markdown", "XML", "CSV", "INI",
    "Env", "Git Config", "EditorConfig", "Properties", "Plain Text",
    "SVG", "Lock",
];

fn is_programming_lang(name: &str) -> bool {
    !EXCLUDED_LANGS.iter().any(|e| e.eq_ignore_ascii_case(name))
}

#[derive(Debug, Clone, Copy)]
pub enum Theme {
    Dark,
    Light,
}

impl Theme {
    pub fn from_str(s: &str) -> Self {
        match s {
            "light" => Self::Light,
            _ => Self::Dark,
        }
    }

    fn bg(&self) -> &str {
        match self {
            Self::Dark => "#0d1117",
            Self::Light => "#ffffff",
        }
    }

    fn border(&self) -> &str {
        match self {
            Self::Dark => "#30363d",
            Self::Light => "#d0d7de",
        }
    }

    fn text(&self) -> &str {
        match self {
            Self::Dark => "#c9d1d9",
            Self::Light => "#1f2328",
        }
    }

    fn muted(&self) -> &str {
        match self {
            Self::Dark => "#6e7681",
            Self::Light => "#656d76",
        }
    }

    fn bar_bg(&self) -> &str {
        match self {
            Self::Dark => "#161b22",
            Self::Light => "#eaeef2",
        }
    }

    fn separator(&self) -> &str {
        match self {
            Self::Dark => "#21262d",
            Self::Light => "#d8dee4",
        }
    }

    fn heat_empty(&self) -> &str {
        match self {
            Self::Dark => "#161b22",
            Self::Light => "#ebedf0",
        }
    }

    fn heat_levels(&self) -> [&str; 4] {
        match self {
            Self::Dark => ["#4a2a0a", "#7c4a15", "#c46d1a", "#ef973e"],
            Self::Light => ["#fde5c8", "#f5c78e", "#e8a04e", "#d97b16"],
        }
    }

    fn accent(&self) -> &str {
        match self {
            Self::Dark => "#ef973e",
            Self::Light => "#c46d1a",
        }
    }
}

fn format_duration(seconds: f64) -> String {
    let total_min = (seconds / 60.0).round() as u64;
    let h = total_min / 60;
    let m = total_min % 60;
    if h > 0 {
        format!("{}h {:02}m", h, m)
    } else if m > 0 {
        format!("{}m", m)
    } else {
        "0m".to_string()
    }
}

fn month_abbr(month: u32) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "",
    }
}

/// Embedded TimeForged logo (favicon-48.png, base64-encoded)
const LOGO_PNG_B64: &str = "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";

/// Inline SVG logo using the real TimeForged PNG
fn logo_svg(x: i32, y: i32, size: i32) -> String {
    format!(
        r#"  <image x="{x}" y="{y}" width="{size}" height="{size}" xlink:href="data:image/png;base64,{LOGO_PNG_B64}" href="data:image/png;base64,{LOGO_PNG_B64}"/>
"#
    )
}

pub fn render_svg(summary: &Summary, theme: Theme) -> String {
    let pad_x = 25;

    // Build day→seconds map
    let day_map: HashMap<NaiveDate, f64> = summary
        .days
        .iter()
        .map(|d| (d.date, d.total_seconds))
        .collect();

    // Generate full date range
    let end_date = summary.to.date_naive();
    let start_date = summary.from.date_naive();
    let total_days = (end_date - start_date).num_days().max(1) as usize;

    // Collect all days with seconds
    let mut all_days: Vec<(NaiveDate, f64)> = Vec::with_capacity(total_days);
    let mut d = start_date;
    while d <= end_date {
        let secs = day_map.get(&d).copied().unwrap_or(0.0);
        all_days.push((d, secs));
        d += Duration::days(1);
    }

    // Find max for intensity scaling
    let max_secs = all_days.iter().map(|(_, s)| *s).fold(0.0_f64, f64::max);

    // Align to start of week (Monday = 0)
    let start_weekday = start_date.weekday().num_days_from_monday() as i32;

    // Number of columns (weeks)
    let total_cells = start_weekday as usize + all_days.len();
    let num_cols = (total_cells + 6) / 7;

    // Cell sizing — adaptive based on column count
    let cell_size: i32;
    let cell_gap: i32;
    if num_cols <= 10 {
        cell_size = 13;
        cell_gap = 3;
    } else if num_cols <= 20 {
        cell_size = 11;
        cell_gap = 2;
    } else {
        // Year view: ~53 columns, need compact cells
        cell_size = 10;
        cell_gap = 3;
    }
    let cell_stride = cell_size + cell_gap;

    let day_label_width: i32 = 30;
    let grid_x = pad_x + day_label_width;
    let grid_width = num_cols as i32 * cell_stride - cell_gap;

    // Dynamic card width based on grid
    let width = grid_x + grid_width + pad_x;

    // Sections vertical layout
    let logo_size = 28;
    let header_y = 18;
    let month_labels_y = 48;
    let grid_y = month_labels_y + 14;
    let grid_height = 7 * cell_stride - cell_gap;
    let legend_y = grid_y + grid_height + 14;
    let stats_y = legend_y + 18;
    let separator_y = stats_y + 10;

    // Languages
    // Filter out non-programming languages (JSON, Markdown, YAML, etc.)
    let prog_langs: Vec<_> = summary
        .languages
        .iter()
        .filter(|l| is_programming_lang(&l.name))
        .collect::<Vec<_>>();
    let prog_total: f64 = prog_langs.iter().map(|l| l.total_seconds).sum();
    let top_langs: Vec<timeforged_core::models::CategorySummary> = prog_langs
        .iter()
        .take(5)
        .map(|l| timeforged_core::models::CategorySummary {
            name: l.name.clone(),
            total_seconds: l.total_seconds,
            percent: if prog_total > 0.0 {
                (l.total_seconds / prog_total) * 100.0
            } else {
                0.0
            },
        })
        .collect();
    let lang_bar_y = separator_y + 18;
    let lang_bar_height = 8;
    let lang_label_start = lang_bar_y + lang_bar_height + 16;
    let lang_row_height = 20;
    let lang_rows = if top_langs.is_empty() {
        0
    } else {
        ((top_langs.len() + 2) / 3) as i32
    };
    let langs_section_height = if top_langs.is_empty() {
        0
    } else {
        lang_bar_height + 16 + lang_rows * lang_row_height
    };

    let height = if top_langs.is_empty() {
        stats_y + 20
    } else {
        separator_y + 18 + langs_section_height + 14
    };

    let total_str = format_duration(summary.total_seconds);
    let project_count = summary.projects.len();
    let heat = theme.heat_levels();

    // Badge text based on time range
    let badge_text = if total_days <= 7 {
        format!("{total_str} this week")
    } else if total_days <= 31 {
        format!("{total_str} this month")
    } else {
        format!("{total_str} this year")
    };

    let mut svg = String::with_capacity(32768);

    // === Card background ===
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{width}" height="{height}" viewBox="0 0 {width} {height}" fill="none">
  <rect x="0.5" y="0.5" width="{}" height="{}" rx="6" fill="{}" stroke="{}"/>
"#,
        width - 1,
        height - 1,
        theme.bg(),
        theme.border(),
    ));

    // === Header: logo + title + total badge ===
    svg.push_str(&logo_svg(pad_x, header_y - 8, logo_size));

    svg.push_str(&format!(
        r#"  <text x="{}" y="{}" fill="{}" font-family="'Segoe UI',Ubuntu,Roboto,sans-serif" font-size="14" font-weight="700">TimeForged</text>
"#,
        pad_x + logo_size + 6,
        header_y + 9,
        theme.accent(),
    ));

    // Total time badge on right
    let badge_x = width - pad_x - 8;
    svg.push_str(&format!(
        r#"  <text x="{badge_x}" y="{}" text-anchor="end" fill="{}" font-family="'Segoe UI',Ubuntu,Roboto,sans-serif" font-size="11" font-weight="600">{badge_text}</text>
"#,
        header_y + 8,
        theme.accent(),
    ));

    // === Month labels above grid ===
    {
        let mut last_month = 0u32;
        for col in 0..num_cols {
            let day_idx = col * 7_usize;
            if day_idx >= start_weekday as usize {
                let actual_idx = day_idx - start_weekday as usize;
                if actual_idx < all_days.len() {
                    let date = all_days[actual_idx].0;
                    let m = date.month();
                    if m != last_month {
                        last_month = m;
                        let x = grid_x + col as i32 * cell_stride;
                        svg.push_str(&format!(
                            r#"  <text x="{x}" y="{month_labels_y}" fill="{}" font-family="'Segoe UI',Ubuntu,Roboto,sans-serif" font-size="10">{}</text>
"#,
                            theme.muted(),
                            month_abbr(m),
                        ));
                    }
                }
            }
        }
    }

    // === Day labels (Mon, Wed, Fri) ===
    for (row, label) in [(1, "Mon"), (3, "Wed"), (5, "Fri")] {
        let y = grid_y + row * cell_stride + cell_size - 2;
        svg.push_str(&format!(
            r#"  <text x="{pad_x}" y="{y}" fill="{}" font-family="'Segoe UI',Ubuntu,Roboto,sans-serif" font-size="10">{label}</text>
"#,
            theme.muted(),
        ));
    }

    // === Heatmap grid ===
    for (i, &(_date, secs)) in all_days.iter().enumerate() {
        let cell_index = i + start_weekday as usize;
        let col = (cell_index / 7) as i32;
        let row = (cell_index % 7) as i32;
        let x = grid_x + col * cell_stride;
        let y = grid_y + row * cell_stride;

        let color = if secs < 60.0 {
            theme.heat_empty()
        } else if max_secs > 0.0 {
            let ratio = secs / max_secs;
            if ratio < 0.15 {
                heat[0]
            } else if ratio < 0.35 {
                heat[1]
            } else if ratio < 0.65 {
                heat[2]
            } else {
                heat[3]
            }
        } else {
            theme.heat_empty()
        };

        svg.push_str(&format!(
            r#"  <rect x="{x}" y="{y}" width="{cell_size}" height="{cell_size}" rx="2" fill="{color}"/>
"#,
        ));
    }

    // === Legend: Less □□□□□ More ===
    {
        let legend_right = width - pad_x;
        let box_size = 10;
        let box_gap = 3;
        let total_legend_w = 5 * (box_size + box_gap) - box_gap;
        let legend_start_x = legend_right - total_legend_w - 35;

        svg.push_str(&format!(
            r#"  <text x="{}" y="{legend_y}" fill="{}" font-family="'Segoe UI',Ubuntu,Roboto,sans-serif" font-size="10">Less</text>
"#,
            legend_start_x - 28,
            theme.muted(),
        ));

        let colors = [theme.heat_empty(), heat[0], heat[1], heat[2], heat[3]];
        for (j, c) in colors.iter().enumerate() {
            let bx = legend_start_x + j as i32 * (box_size + box_gap);
            svg.push_str(&format!(
                r#"  <rect x="{bx}" y="{}" width="{box_size}" height="{box_size}" rx="2" fill="{c}"/>
"#,
                legend_y - 9,
            ));
        }

        svg.push_str(&format!(
            r#"  <text x="{}" y="{legend_y}" fill="{}" font-family="'Segoe UI',Ubuntu,Roboto,sans-serif" font-size="10">More</text>
"#,
            legend_start_x + 5 * (box_size + box_gap) + 3,
            theme.muted(),
        ));
    }

    // === Stats line ===
    let projects_label = match project_count {
        0 => "no projects".to_string(),
        1 => "1 project".to_string(),
        n => format!("{n} projects"),
    };

    svg.push_str(&format!(
        r#"  <text x="{pad_x}" y="{stats_y}" fill="{}" font-family="'Segoe UI',Ubuntu,Roboto,sans-serif" font-size="11">Total: <tspan font-weight="600" fill="{}">{total_str}</tspan>  ·  <tspan fill="{}">{projects_label}</tspan></text>
"#,
        theme.muted(),
        theme.accent(),
        theme.muted(),
    ));

    // === Languages section ===
    if !top_langs.is_empty() {
        // Separator
        svg.push_str(&format!(
            r#"  <line x1="{pad_x}" y1="{separator_y}" x2="{}" y2="{separator_y}" stroke="{}" stroke-width="1"/>
"#,
            width - pad_x,
            theme.separator(),
        ));

        // Stacked language bar
        let total_lang_secs: f64 = top_langs.iter().map(|l| l.total_seconds).sum();
        let bar_width = (width - 2 * pad_x) as f64;

        svg.push_str(&format!(
            r#"  <clipPath id="lc"><rect x="{pad_x}" y="{lang_bar_y}" width="{}" height="{lang_bar_height}" rx="4"/></clipPath>
  <rect x="{pad_x}" y="{lang_bar_y}" width="{}" height="{lang_bar_height}" rx="4" fill="{}"/>
  <g clip-path="url(#lc)">
"#,
            width - 2 * pad_x,
            width - 2 * pad_x,
            theme.bar_bg(),
        ));

        let mut offset = 0.0_f64;
        for lang in &top_langs {
            let seg_w = if total_lang_secs > 0.0 {
                (lang.total_seconds / total_lang_secs) * bar_width
            } else {
                0.0
            };
            if seg_w > 0.5 {
                let x = pad_x as f64 + offset;
                svg.push_str(&format!(
                    r#"    <rect x="{x:.1}" y="{lang_bar_y}" width="{seg_w:.1}" height="{lang_bar_height}" fill="{}"/>
"#,
                    lang_color(&lang.name),
                ));
            }
            offset += seg_w;
        }

        svg.push_str("  </g>\n");

        // Language labels (3 per row)
        let col_width = (width - 2 * pad_x) / 3;
        for (i, lang) in top_langs.iter().enumerate() {
            let col = i % 3;
            let row = i / 3;
            let x = pad_x + col as i32 * col_width;
            let y = lang_label_start + row as i32 * lang_row_height;
            let pct = lang.percent.round() as u32;

            svg.push_str(&format!(
                r#"  <circle cx="{}" cy="{}" r="4" fill="{}"/>
  <text x="{}" y="{y}" fill="{}" font-family="'Segoe UI',Ubuntu,Roboto,sans-serif" font-size="11"><tspan font-weight="500">{}</tspan> <tspan fill="{}">{pct}%</tspan></text>
"#,
                x + 4,
                y - 4,
                lang_color(&lang.name),
                x + 14,
                theme.text(),
                lang.name,
                theme.muted(),
            ));
        }
    }

    svg.push_str("</svg>\n");
    svg
}
