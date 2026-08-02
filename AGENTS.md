# AGENTS.md

See `CLAUDE.md` for full design.

## Quick rules

- JSON report shapes (`WideTable`, `TrendReport`, `TopReport`) are the
  wire-format contract for downstream consumers (cadence-forge,
  dashboards). Semver-relevant.
- Views are pure functions over `&[Event]`. No I/O inside view.rs.
- Period parsed via `forge-core::duration::parse_seconds`.
- Sort order is biggest-first throughout. Cost-descending.
- Edition 2024, Rust 1.89+. Consumes forge-core.
- No real organization-specific values in this public repo, ever.
