# showback-forge — Claude / agent guidance

## What this repo is

`showback-forge` is Wave 4's first tool — the rendering layer that turns
attribution-forge's JSONL stream into standardized views for team
consumption. The views are designed to be composed by `cadence-forge`
into review packets.

**Open source under MIT.** Generic framework; team / cost-center / product
names live in private overlays.

## Architecture

Pure functions over `&[Event]` returning structured reports
(`WideTable`, `TrendReport`, `TopReport`). Three subcommands (`render`,
`trend`, `top`) map 1:1 to the three view types. All views support
human + JSON output formats — the JSON shape is the API contract for
downstream embedding.

## Module map

- `src/main.rs` — clap dispatch.
- `src/event.rs` — minimal `Event` (subset of attribution-forge's wire
  format) + JSONL loader.
- `src/view.rs` — the three view builders + their report types + human
  renderers + a small `sparkline()` helper. Tests live here.

Consumes `forge-core::duration::parse_seconds` for the period flag.

## Invariants

1. **Views are pure functions.** No I/O inside `render`/`trend`/`top`;
   only in `event::load_jsonl` and `main.rs`.
2. **Period alignment is UTC, floor-to-bucket.** Same approach as the
   other forge tools.
3. **Missing dim values become `(missing)`.** Explicit marker beats
   silent drop.
4. **Sort order is biggest-first.** Both `render` (by row total) and
   `trend` (by current cost) and `top` (by current cost) sort so the
   most-cost items surface first.
5. **JSON shape is the wire format for downstream consumers.**
   `cadence-forge` embeds these; semver-relevant.

## When extending

- **New view type** — add a function in `view.rs` + a subcommand in
  `main.rs`. Each view returns a Serializable report with both human
  and JSON output.
- **Bundle/report subcommand** (v0.2) — load `configs/*.yaml`, iterate
  through view definitions, compose into a single report. The
  configs/default.yaml already documents the intended shape.
- **Confluence publish target** — separate output mode that renders to
  Confluence markdown and uploads via mcp__atlassian. Don't bake into
  this tool; let cadence-forge orchestrate.
- **Chargeback mode** — add a `mode: showback|chargeback` field to
  view configs; chargeback rows include budget-debit columns.

## Don't

- Don't commit organization-specific values to this repo.
- Don't widen the Event shape — it's a subset, kept lean.
- Don't add cloud SDK dependencies. JSONL input is the wire format.
- Don't make `trend` recompute the bucket grid — it builds on top of
  `render`'s table to keep the math in one place.
- Don't sort alphabetically by default. Cost-descending is the right
  order for a glance-then-skim consumer.
