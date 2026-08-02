//! Minimal Event shape — subset of attribution-forge's wire format.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;
use std::io::BufRead;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Event {
    pub ts: DateTime<Utc>,
    pub cost_usd: f64,
    #[serde(default)]
    pub dimensions: HashMap<String, String>,
}

impl Event {
    pub fn dim(&self, key: &str) -> &str {
        self.dimensions
            .get(key)
            .map(String::as_str)
            .filter(|s| !s.is_empty())
            .unwrap_or("(missing)")
    }
}

pub fn load_jsonl(path: &Path) -> Result<Vec<Event>> {
    let f = std::fs::File::open(path)
        .with_context(|| format!("opening events file {}", path.display()))?;
    let r = std::io::BufReader::new(f);
    let mut out = vec![];
    for (idx, line) in r.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let ev: Event = serde_json::from_str(&line)
            .with_context(|| format!("parsing event on line {}", idx + 1))?;
        out.push(ev);
    }
    Ok(out)
}
