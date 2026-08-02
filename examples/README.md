# showback-forge examples

Run from the repo root:

```bash
cargo build --release
BIN=./target/release/showback-forge

# 1. Wide-table render: 4 months × cost_center
$BIN render examples/events.jsonl --dimension cost_center --period 1mo --periods 4

# 2. Trend view: same data + period-over-period delta + sparkline
$BIN trend examples/events.jsonl --dimension cost_center --period 1mo --periods 4

# 3. Top-N: top cost centers in the most recent month
$BIN top examples/events.jsonl --dimension cost_center --period 1mo --limit 5

# 4. JSON output for embedding into review packets
$BIN trend examples/events.jsonl --dimension cost_center --period 1mo --periods 4 --format json
```

The synthetic dataset is 4 months of monthly cost, 5 cost_centers,
with deliberate patterns: steady growth (`cc-platform`), spike volatility
(`cc-data`), a successful declining trend (`cc-observability`), and a
flat baseline (`cc-cicd`). All names fictional.
