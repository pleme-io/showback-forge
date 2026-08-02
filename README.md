# showback-forge

Per-team / per-product / per-tenant cost rendering from attribution events — **open source under MIT**.

`showback-forge` is Wave 4's first tool. It turns the raw JSONL stream
from `attribution-forge` into standardized recurring **views** for team
consumption: multi-period tables, period-over-period trends with deltas
and sparklines, and top-N rankings. The views are the building blocks
`cadence-forge` will embed into review packets.

## Subcommands

| Verb | Purpose |
| --- | --- |
| `render` | Wide table — rows by dimension value, columns by period |
| `trend` | Period-over-period delta + sparkline; sorted by current cost |
| `top` | Top N by cost in the most recent period |

## Public framework / private config boundary

| What | Where | License |
| --- | --- | --- |
| Binary, views, schemas | pleme-io — public | MIT |
| Real cost-center / team / product names, view-bundle configs | Org's private repo | Proprietary |

## Install

```bash
cargo build --release
./target/release/showback-forge --help

# via Nix (after publish)
nix run github:pleme-io/showback-forge -- --help
```

## Usage

```bash
# Wide table: 6 months × cost_center
showback-forge render events.jsonl --dimension cost_center --period 1mo --periods 6

# Trend with sparkline
showback-forge trend events.jsonl --dimension product --period 1w --periods 8

# Top 10 tenants this month
showback-forge top events.jsonl --dimension tenant --period 1mo --limit 10

# JSON for embedding in dashboards / review packets
showback-forge trend events.jsonl --dimension cost_center --period 1mo --periods 6 --format json
```

## Event format

JSON Lines, same wire format as `attribution-forge` output:

```json
{
  "ts": "2026-01-01T00:00:00Z",
  "cost_usd": 12.50,
  "dimensions": { "cost_center": "cc-platform", "product": "product-alpha" }
}
```

## Period units

The `--period` flag accepts: `s`, `m`, `h`, `d`, `w`, `mo`, `y`.

## Showback vs chargeback

v0.1 is **showback** — visibility, no consequence. Teams see their
costs but their budgets aren't touched.

Mature programs eventually want **chargeback** — internal accounting
mechanism where teams' budgets are debited. That's a v0.2 mode toggle.
Trying to chargeback before showback has built trust in the data is a
classic anti-pattern. See the FinOps Strategy doc.

## Roadmap (post-v0.1)

- `report` subcommand consuming a view-bundle config (see
  `configs/default.yaml`) to produce a single composed packet.
- Confluence-publish target — render the report packet directly to a
  Confluence space (used by `cadence-forge` once it's ready).
- Chargeback mode — same views, with budget debit lines added.
- Comparison view — side-by-side budget vs actual.

## License

MIT — see `LICENSE`.
