//! Single-pass summary accumulation.
//!
//! The summary used to run four window-function queries (LAG ... PARTITION BY ... ORDER BY
//! timestamp), each re-sorting the whole year of events -- 6.5s on 182k rows. get_summary now
//! streams one timestamp-ordered SELECT and folds it here: global ts order gives every
//! partition the same relative order LAG consumed, and memory stays O(distinct projects +
//! languages + days), never O(rows) -- the daemon runs under a 50MB cap.

use std::collections::HashMap;
use std::hash::Hash;

use chrono::{DateTime, NaiveDate, Utc};

use timeforged_core::models::{CategorySummary, DaySummary, Summary};

/// Running state of one partition: last seen event and the gap total so far.
#[derive(Default)]
struct Partition {
    last: Option<DateTime<Utc>>,
    total: f64,
}

pub(super) struct SummaryAccumulator {
    idle_timeout: f64,
    project: HashMap<String, Partition>,
    language: HashMap<String, Partition>,
    day: HashMap<NaiveDate, Partition>,
    // The NULL project is its own partition: it counts toward total_seconds
    // but must never show up in the projects list.
    unassigned: Partition,
}

impl SummaryAccumulator {
    pub(super) fn new(idle_timeout: u64) -> Self {
        Self {
            idle_timeout: idle_timeout as f64,
            project: HashMap::new(),
            language: HashMap::new(),
            day: HashMap::new(),
            unassigned: Partition::default(),
        }
    }

    /// Feed the next event of the timestamp-ordered stream.
    pub(super) fn push(&mut self, ts: DateTime<Utc>, project: Option<&str>, language: Option<&str>) {
        match project {
            Some(p) => Self::bump(&mut self.project, p.to_string(), ts, self.idle_timeout),
            None => Self::bump_partition(&mut self.unassigned, ts, self.idle_timeout),
        }
        if let Some(l) = language {
            Self::bump(&mut self.language, l.to_string(), ts, self.idle_timeout);
        }
        // Same key as SQL date(timestamp): the UTC calendar date of the stored RFC3339 string.
        Self::bump(&mut self.day, ts.date_naive(), ts, self.idle_timeout);
    }

    pub(super) fn finish(self, from: DateTime<Utc>, to: DateTime<Utc>) -> Summary {
        // total_seconds is the union of ALL project partitions, the unassigned one included.
        let total_seconds =
            self.unassigned.total + self.project.values().map(|p| p.total).sum::<f64>();

        let projects = category_summaries(self.project);
        let languages = category_summaries(self.language);

        let mut days: Vec<DaySummary> = self
            .day
            .into_iter()
            .map(|(date, p)| DaySummary {
                date,
                total_seconds: p.total,
            })
            .collect();
        days.sort_by_key(|d| d.date);

        Summary {
            total_seconds,
            from,
            to,
            projects,
            languages,
            days,
        }
    }

    fn bump<K: Eq + Hash>(
        map: &mut HashMap<K, Partition>,
        key: K,
        ts: DateTime<Utc>,
        idle_timeout: f64,
    ) {
        Self::bump_partition(map.entry(key).or_default(), ts, idle_timeout);
    }

    fn bump_partition(part: &mut Partition, ts: DateTime<Utc>, idle_timeout: f64) {
        // Same rule as the old window function: only gaps strictly shorter than
        // idle_timeout count, and a partition's first event contributes 0 (LAG's NULL prev_ts).
        if let Some(last) = part.last {
            let gap = (ts - last).num_milliseconds() as f64 / 1000.0;
            if gap < idle_timeout {
                part.total += gap;
            }
        }
        part.last = Some(ts);
    }
}

fn category_summaries(map: HashMap<String, Partition>) -> Vec<CategorySummary> {
    // Percent denominator covers only the listed partitions -- the old query summed
    // exactly the non-NULL rows it returned.
    let grand_total: f64 = map.values().map(|p| p.total).sum();
    let mut rows: Vec<CategorySummary> = map
        .into_iter()
        .map(|(name, p)| CategorySummary {
            name,
            total_seconds: p.total,
            percent: if grand_total > 0.0 {
                p.total / grand_total * 100.0
            } else {
                0.0
            },
        })
        .collect();
    // total_cmp: total order on f64, no NaN panic (and same ORDER BY total DESC shape).
    rows.sort_by(|a, b| b.total_seconds.total_cmp(&a.total_seconds));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn ts(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, d, h, mi, s).unwrap()
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn gap_capped_at_idle_timeout() {
        let mut acc = SummaryAccumulator::new(60);
        let t0 = ts(2026, 9, 28, 10, 0, 0);
        acc.push(t0, Some("p"), None); // first: 0
        acc.push(t0 + chrono::Duration::seconds(30), Some("p"), None); // 30 < 60: +30
        acc.push(t0 + chrono::Duration::seconds(300), Some("p"), None); // 300 >= 60: +0
        acc.push(t0 + chrono::Duration::seconds(310), Some("p"), None); // 10 < 60: +10

        let s = acc.finish(t0, t0);
        assert_close(s.total_seconds, 40.0);
        assert_eq!(s.projects.len(), 1);
        assert_eq!(s.projects[0].name, "p");
        assert_close(s.projects[0].total_seconds, 40.0);
        assert_close(s.projects[0].percent, 100.0);
    }

    #[test]
    fn first_event_contributes_zero() {
        let mut acc = SummaryAccumulator::new(60);
        acc.push(ts(2026, 9, 28, 10, 0, 0), Some("p"), Some("rust"));

        let s = acc.finish(ts(2026, 9, 28, 0, 0, 0), ts(2026, 9, 28, 23, 59, 59));
        assert_close(s.total_seconds, 0.0);
        assert_close(s.projects[0].total_seconds, 0.0);
        assert_close(s.languages[0].total_seconds, 0.0);
        // grand_total == 0 must yield percent 0, not NaN
        assert_close(s.projects[0].percent, 0.0);
    }

    #[test]
    fn null_project_counts_in_total_but_excluded_from_projects() {
        let mut acc = SummaryAccumulator::new(60);
        let t0 = ts(2026, 9, 28, 10, 0, 0);
        acc.push(t0, None, None);
        acc.push(t0 + chrono::Duration::seconds(10), Some("a"), None);
        acc.push(t0 + chrono::Duration::seconds(20), None, None); // NULL gap 20 < 60

        let s = acc.finish(t0, t0);
        assert_close(s.total_seconds, 20.0);
        assert_eq!(s.projects.len(), 1);
        assert_eq!(s.projects[0].name, "a");
        assert_close(s.projects[0].total_seconds, 0.0);
        assert_close(s.projects[0].percent, 0.0); // grand total of listed projects is 0
    }

    #[test]
    fn day_partition_boundary_at_utc_midnight() {
        let mut acc = SummaryAccumulator::new(60);
        let late = ts(2026, 9, 28, 23, 59, 0);
        // 2-minute gap across midnight: different day partitions, so nothing is bridged.
        acc.push(late, Some("p"), None);
        acc.push(late + chrono::Duration::seconds(120), Some("p"), None);
        acc.push(late + chrono::Duration::seconds(150), Some("p"), None); // 30 < 60

        let s = acc.finish(late, late);
        assert_eq!(s.days.len(), 2);
        assert_eq!(s.days[0].date, chrono::NaiveDate::from_ymd_opt(2026, 9, 28).unwrap());
        assert_close(s.days[0].total_seconds, 0.0);
        assert_eq!(s.days[1].date, chrono::NaiveDate::from_ymd_opt(2026, 9, 29).unwrap());
        assert_close(s.days[1].total_seconds, 30.0);
        assert_close(s.total_seconds, 30.0);
    }

    #[test]
    fn project_partitions_do_not_bridge_each_other() {
        // Interleaved projects must not turn each other's idle gaps into work
        // (regression behind timeforged-inflated-hours-2026-09-13).
        let mut acc = SummaryAccumulator::new(60);
        let t0 = ts(2026, 9, 28, 10, 0, 0);
        acc.push(t0, Some("a"), None);
        acc.push(t0 + chrono::Duration::seconds(10), Some("b"), None);
        acc.push(t0 + chrono::Duration::seconds(20), Some("a"), None);

        let s = acc.finish(t0, t0);
        assert_close(s.total_seconds, 20.0); // only a's 20s gap counts; b is its own partition
        assert_close(s.projects[0].total_seconds, 20.0);
        assert_close(s.projects[1].total_seconds, 0.0);
    }

    #[test]
    fn languages_partition_separately_from_projects() {
        let mut acc = SummaryAccumulator::new(60);
        let t0 = ts(2026, 9, 28, 10, 0, 0);
        acc.push(t0, Some("p"), Some("rust"));
        acc.push(t0 + chrono::Duration::seconds(5), Some("p"), Some("go"));
        acc.push(t0 + chrono::Duration::seconds(35), Some("p"), Some("rust"));

        let s = acc.finish(t0, t0);
        assert_eq!(s.languages.len(), 2);
        // Sorted by total DESC: rust 35s first, then go with 0s.
        assert_eq!(s.languages[0].name, "rust");
        assert_close(s.languages[0].total_seconds, 35.0);
        assert_close(s.languages[0].percent, 100.0);
        assert_eq!(s.languages[1].name, "go");
        assert_close(s.languages[1].total_seconds, 0.0);
        assert_close(s.languages[1].percent, 0.0);
        // Days and languages are independent partitions of the same stream.
        assert_eq!(s.days.len(), 1);
        assert_close(s.days[0].total_seconds, 35.0);
    }

    #[test]
    fn push_is_pure_no_state_leaks_between_partitions() {
        // Purity check: same event fed to two accumulators yields identical output,
        // so any project filtering stays SQL-side.
        let feed = |acc: &mut SummaryAccumulator| {
            let t0 = ts(2026, 9, 28, 10, 0, 0);
            acc.push(t0, Some("a"), Some("rust"));
            acc.push(t0 + chrono::Duration::seconds(42), Some("a"), Some("rust"));
        };

        let mut a = SummaryAccumulator::new(60);
        let mut b = SummaryAccumulator::new(60);
        feed(&mut a);
        feed(&mut b);

        let sa = a.finish(ts(2026, 9, 28, 0, 0, 0), ts(2026, 9, 28, 0, 0, 0));
        let sb = b.finish(ts(2026, 9, 28, 0, 0, 0), ts(2026, 9, 28, 0, 0, 0));
        assert_close(sa.total_seconds, sb.total_seconds);
        assert_eq!(sa.projects.len(), sb.projects.len());
        assert_eq!(sa.days.len(), sb.days.len());
    }
}
