//! showback-forge — per-team / per-product cost rendering.

mod event;
mod view;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "showback-forge",
    version,
    about = "Render per-team / per-product cost views (showback) from attribution events",
    long_about = None,
)]
struct Cli {
    #[arg(short, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Render a wide table: rows by dimension, columns by period.
    Render {
        events: PathBuf,
        #[arg(long, default_value = "cost_center")]
        dimension: String,
        #[arg(long, default_value = "1mo")]
        period: String,
        #[arg(long, default_value_t = 6)]
        periods: usize,
        #[arg(long, default_value = "human")]
        format: String,
    },

    /// Render a period-over-period trend with delta + sparkline.
    Trend {
        events: PathBuf,
        #[arg(long, default_value = "cost_center")]
        dimension: String,
        #[arg(long, default_value = "1mo")]
        period: String,
        #[arg(long, default_value_t = 6)]
        periods: usize,
        #[arg(long, default_value = "human")]
        format: String,
    },

    /// Top N dimension values by cost in the most recent period.
    Top {
        events: PathBuf,
        #[arg(long, default_value = "cost_center")]
        dimension: String,
        #[arg(long, default_value = "1mo")]
        period: String,
        #[arg(long, default_value_t = 10)]
        limit: usize,
        #[arg(long, default_value = "human")]
        format: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    match cli.command {
        Command::Render {
            events,
            dimension,
            period,
            periods,
            format,
        } => {
            let evs = event::load_jsonl(&events)?;
            let table = view::render(&evs, &dimension, &period, periods)?;
            match format.as_str() {
                "json" => println!("{}", serde_json::to_string_pretty(&table)?),
                _ => println!("{}", table.render_human()),
            }
        }
        Command::Trend {
            events,
            dimension,
            period,
            periods,
            format,
        } => {
            let evs = event::load_jsonl(&events)?;
            let report = view::trend(&evs, &dimension, &period, periods)?;
            match format.as_str() {
                "json" => println!("{}", serde_json::to_string_pretty(&report)?),
                _ => println!("{}", report.render_human()),
            }
        }
        Command::Top {
            events,
            dimension,
            period,
            limit,
            format,
        } => {
            let evs = event::load_jsonl(&events)?;
            let report = view::top(&evs, &dimension, &period, limit)?;
            match format.as_str() {
                "json" => println!("{}", serde_json::to_string_pretty(&report)?),
                _ => println!("{}", report.render_human()),
            }
        }
    }
    Ok(())
}

fn init_tracing(verbosity: u8) {
    let level = match verbosity {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| format!("showback_forge={level}"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}
