//! View builders. Three canonical shapes:
//! 1. `render` — wide table: rows = dim values, columns = periods.
//! 2. `trend` — same data + period-over-period delta + sparkline of recent
//!    periods.
//! 3. `top` — top N dim values by cost in the most recent period.

use chrono::{DateTime, TimeZone, Utc};
use forge_core::duration::parse_seconds;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

use crate::event::Event;

#[derive(Debug, Clone, Serialize)]
pub struct WideTable {
    pub dimension: String,
    pub period: String,
    pub periods: Vec<DateTime<Utc>>,
    pub rows: Vec<WideRow>,
    pub totals: Vec<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WideRow {
    pub key: String,
    pub period_costs: Vec<f64>,
    pub row_total: f64,
}

impl WideTable {
    pub fn render_human(&self) -> String {
        let mut lines = vec![format!(
            "by {} ({} periods of {})",
            self.dimension,
            self.periods.len(),
            self.period
        )];
        // Header.
        let mut header = format!("  {:<24}", self.dimension);
        for p in &self.periods {
            header.push_str(&format!(" {:>12}", p.format("%Y-%m-%d")));
        }
        header.push_str(&format!(" {:>12}", "total"));
        lines.push(header);
        lines.push("  ".to_string() + &"-".repeat(24 + self.periods.len() * 13 + 13));
        for r in &self.rows {
            let mut line = format!("  {:<24}", truncate(&r.key, 24));
            for c in &r.period_costs {
                line.push_str(&format!(" {:>12.2}", c));
            }
            line.push_str(&format!(" {:>12.2}", r.row_total));
            lines.push(line);
        }
        lines.push("  ".to_string() + &"-".repeat(24 + self.periods.len() * 13 + 13));
        let mut totals_line = format!("  {:<24}", "(period totals)");
        for t in &self.totals {
            totals_line.push_str(&format!(" {:>12.2}", t));
        }
        let grand: f64 = self.totals.iter().sum();
        totals_line.push_str(&format!(" {:>12.2}", grand));
        lines.push(totals_line);
        lines.join("\n")
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TrendReport {
    pub dimension: String,
    pub period: String,
    pub periods: Vec<DateTime<Utc>>,
    pub rows: Vec<TrendRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrendRow {
    pub key: String,
    pub period_costs: Vec<f64>,
    pub current_cost: f64,
    pub previous_cost: f64,
    pub delta_usd: f64,
    pub delta_pct: Option<f64>,
}

impl TrendReport {
    pub fn render_human(&self) -> String {
        let mut lines = vec![format!(
            "{}-by-period trend over {} periods of {}",
            self.dimension,
            self.periods.len(),
            self.period
        )];
        lines.push(format!(
            "  {:<24}  {:>10}  {:>10}  {:>10}  {:>8}  {}",
            self.dimension, "current", "previous", "delta_usd", "delta_%", "sparkline"
        ));
        lines.push("  ".to_string() + &"-".repeat(82));
        for r in &self.rows {
            let pct = match r.delta_pct {
                Some(p) if p.is_finite() => format!("{p:+7.1}%"),
                Some(_) => "    n/a".to_string(),
                None => "    n/a".to_string(),
            };
            let spark = sparkline(&r.period_costs);
            lines.push(format!(
                "  {:<24}  {:>10.2}  {:>10.2}  {:>+10.2}  {}  {}",
                truncate(&r.key, 24),
                r.current_cost,
                r.previous_cost,
                r.delta_usd,
                pct,
                spark
            ));
        }
        lines.join("\n")
    }
}

fn sparkline(values: &[f64]) -> String {
    if values.is_empty() {
        return String::new();
    }
    let blocks = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let max = values.iter().copied().fold(0.0f64, f64::max);
    if max <= 0.0 {
        return values.iter().map(|_| blocks[0]).collect();
    }
    values
        .iter()
        .map(|v| {
            let normalized = (v / max).clamp(0.0, 1.0);
            let idx = ((normalized * (blocks.len() as f64 - 1.0)).round() as usize)
                .min(blocks.len() - 1);
            blocks[idx]
        })
        .collect()
}

#[derive(Debug, Clone, Serialize)]
pub struct TopReport {
    pub dimension: String,
    pub period: String,
    pub period_start: DateTime<Utc>,
    pub limit: usize,
    pub rows: Vec<TopRow>,
    pub total: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopRow {
    pub key: String,
    pub cost_usd: f64,
    pub pct_of_total: f64,
}

impl TopReport {
    pub fn render_human(&self) -> String {
        let mut lines = vec![format!(
            "top {} by {} for period starting {} ({}). Total: ${:.2}",
            self.limit,
            self.dimension,
            self.period_start.format("%Y-%m-%d"),
            self.period,
            self.total
        )];
        lines.push(format!(
            "  {:<24}  {:>12}  {:>8}",
            self.dimension, "cost_usd", "% total"
        ));
        lines.push("  ".to_string() + &"-".repeat(48));
        for r in &self.rows {
            lines.push(format!(
                "  {:<24}  {:>12.2}  {:>7.1}%",
                truncate(&r.key, 24),
                r.cost_usd,
                r.pct_of_total
            ));
        }
        lines.join("\n")
    }
}

fn align_bucket(ts: DateTime<Utc>, bucket_s: u64) -> DateTime<Utc> {
    let secs = ts.timestamp().max(0) as u64;
    let aligned = (secs / bucket_s) * bucket_s;
    Utc.timestamp_opt(aligned as i64, 0).single().unwrap_or(ts)
}

/// Bucket events by `(dim_value, period_start)` and sum the costs.
fn bucketize(
    events: &[Event],
    dimension: &str,
    period_s: u64,
) -> HashMap<(String, DateTime<Utc>), f64> {
    let mut out: HashMap<(String, DateTime<Utc>), f64> = HashMap::new();
    for ev in events {
        let v = ev.dim(dimension).to_string();
        let b = align_bucket(ev.ts, period_s);
        *out.entry((v, b)).or_insert(0.0) += ev.cost_usd;
    }
    out
}

/// Return the most recent `n` periods, descending. The caller reverses if it
/// wants ascending.
fn recent_periods(
    buckets: &HashMap<(String, DateTime<Utc>), f64>,
    n: usize,
) -> Vec<DateTime<Utc>> {
    let mut ps: std::collections::BTreeSet<DateTime<Utc>> = std::collections::BTreeSet::new();
    for (_, ts) in buckets.keys() {
        ps.insert(*ts);
    }
    let mut v: Vec<DateTime<Utc>> = ps.into_iter().collect();
    v.reverse();
    v.into_iter().take(n).collect()
}

pub fn render(events: &[Event], dimension: &str, period: &str, n_periods: usize) -> anyhow::Result<WideTable> {
    let period_s = parse_seconds(period)?;
    let buckets = bucketize(events, dimension, period_s);
    let mut periods = recent_periods(&buckets, n_periods);
    periods.reverse(); // ascending for human reading

    let mut by_key: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for ((dim_val, ts), cost) in &buckets {
        let row = by_key
            .entry(dim_val.clone())
            .or_insert_with(|| vec![0.0; periods.len()]);
        if let Some(idx) = periods.iter().position(|p| p == ts) {
            row[idx] += cost;
        }
    }

    let mut rows: Vec<WideRow> = by_key
        .into_iter()
        .map(|(key, costs)| {
            let total: f64 = costs.iter().sum();
            WideRow {
                key,
                period_costs: costs,
                row_total: total,
            }
        })
        .collect();
    rows.sort_by(|a, b| {
        b.row_total
            .partial_cmp(&a.row_total)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut totals = vec![0.0; periods.len()];
    for r in &rows {
        for (i, c) in r.period_costs.iter().enumerate() {
            totals[i] += c;
        }
    }

    Ok(WideTable {
        dimension: dimension.to_string(),
        period: period.to_string(),
        periods,
        rows,
        totals,
    })
}

pub fn trend(events: &[Event], dimension: &str, period: &str, n_periods: usize) -> anyhow::Result<TrendReport> {
    let table = render(events, dimension, period, n_periods)?;
    let mut rows: Vec<TrendRow> = table
        .rows
        .iter()
        .map(|r| {
            let n = r.period_costs.len();
            let current = if n > 0 { r.period_costs[n - 1] } else { 0.0 };
            let previous = if n >= 2 { r.period_costs[n - 2] } else { 0.0 };
            let delta = current - previous;
            let pct = if previous.abs() > f64::EPSILON {
                Some(delta / previous * 100.0)
            } else if current.abs() > f64::EPSILON {
                Some(f64::INFINITY)
            } else {
                None
            };
            TrendRow {
                key: r.key.clone(),
                period_costs: r.period_costs.clone(),
                current_cost: current,
                previous_cost: previous,
                delta_usd: delta,
                delta_pct: pct,
            }
        })
        .collect();
    // Sort by absolute current cost desc (biggest first)
    rows.sort_by(|a, b| {
        b.current_cost
            .partial_cmp(&a.current_cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(TrendReport {
        dimension: dimension.to_string(),
        period: period.to_string(),
        periods: table.periods,
        rows,
    })
}

pub fn top(events: &[Event], dimension: &str, period: &str, limit: usize) -> anyhow::Result<TopReport> {
    let period_s = parse_seconds(period)?;
    let buckets = bucketize(events, dimension, period_s);
    let latest = recent_periods(&buckets, 1).into_iter().next();
    let period_start = match latest {
        Some(p) => p,
        None => {
            return Ok(TopReport {
                dimension: dimension.to_string(),
                period: period.to_string(),
                period_start: chrono::Utc::now(),
                limit,
                rows: vec![],
                total: 0.0,
            });
        }
    };
    let mut latest_buckets: Vec<(String, f64)> = buckets
        .into_iter()
        .filter(|((_, p), _)| *p == period_start)
        .map(|((k, _), c)| (k, c))
        .collect();
    latest_buckets.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let total: f64 = latest_buckets.iter().map(|(_, c)| c).sum();
    let rows: Vec<TopRow> = latest_buckets
        .into_iter()
        .take(limit)
        .map(|(key, cost)| {
            let pct = if total > 0.0 { cost / total * 100.0 } else { 0.0 };
            TopRow {
                key,
                cost_usd: cost,
                pct_of_total: pct,
            }
        })
        .collect();
    Ok(TopReport {
        dimension: dimension.to_string(),
        period: period.to_string(),
        period_start,
        limit,
        rows,
        total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn ev(day: u32, dim_value: &str, cost: f64) -> Event {
        let mut d = HashMap::new();
        d.insert("cost_center".into(), dim_value.into());
        Event {
            ts: Utc.with_ymd_and_hms(2026, 1, day, 12, 0, 0).unwrap(),
            cost_usd: cost,
            dimensions: d,
        }
    }

    #[test]
    fn render_basic_shape() {
        let events = vec![
            ev(1, "cc-a", 10.0),
            ev(1, "cc-b", 5.0),
            ev(2, "cc-a", 12.0),
            ev(2, "cc-b", 6.0),
            ev(3, "cc-a", 11.0),
            ev(3, "cc-b", 7.0),
        ];
        let t = render(&events, "cost_center", "1d", 3).unwrap();
        assert_eq!(t.periods.len(), 3);
        assert_eq!(t.rows.len(), 2);
        // cc-a is bigger, sorts first
        assert_eq!(t.rows[0].key, "cc-a");
        assert_eq!(t.rows[0].row_total, 33.0);
        assert_eq!(t.totals, vec![15.0, 18.0, 18.0]);
    }

    #[test]
    fn trend_computes_delta() {
        let events = vec![
            ev(1, "cc-a", 10.0),
            ev(2, "cc-a", 12.0),
            ev(3, "cc-a", 15.0), // current
        ];
        let t = trend(&events, "cost_center", "1d", 3).unwrap();
        let row = &t.rows[0];
        assert_eq!(row.current_cost, 15.0);
        assert_eq!(row.previous_cost, 12.0);
        assert_eq!(row.delta_usd, 3.0);
        assert!((row.delta_pct.unwrap() - 25.0).abs() < 1e-9);
    }

    #[test]
    fn top_returns_latest_period_only() {
        let events = vec![
            ev(1, "cc-a", 100.0),
            ev(1, "cc-b", 50.0),
            ev(2, "cc-a", 30.0),
            ev(2, "cc-b", 70.0),
        ];
        let t = top(&events, "cost_center", "1d", 5).unwrap();
        // Latest period is day 2 — cc-b > cc-a
        assert_eq!(t.rows[0].key, "cc-b");
        assert_eq!(t.rows[0].cost_usd, 70.0);
        assert_eq!(t.total, 100.0); // 30 + 70 for the day
        assert!((t.rows[0].pct_of_total - 70.0).abs() < 1e-9);
    }

    #[test]
    fn sparkline_renders_chars() {
        let s = sparkline(&[1.0, 4.0, 2.0, 8.0, 5.0]);
        assert_eq!(s.chars().count(), 5);
    }

    #[test]
    fn missing_dim_becomes_explicit_marker() {
        let mut e = ev(1, "cc-a", 10.0);
        e.dimensions.remove("cost_center");
        let t = render(&[e], "cost_center", "1d", 1).unwrap();
        assert_eq!(t.rows[0].key, "(missing)");
    }
}
