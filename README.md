# Grafana Utilities

Language: **English** | [繁體中文版](README.zh-TW.md)

`grafana-util` is a Rust-first operator CLI for Grafana inventory, backup, drift review, and staged sync work.

## Scope

- `dashboard`: list, export, import, diff, inspect-export, inspect-live
- `datasource`: list, add, modify, delete, export, import, diff
- `alert`: export, import, diff, list-rules, list-contact-points, list-mute-timings, list-templates
- `access`: user, team, org, service-account lifecycle and snapshot workflows
- `sync`: summary, plan, review, preflight, assess-alerts, bundle-preflight, apply

## Build

```bash
cargo build --release --manifest-path rust/Cargo.toml --bin grafana-util
grafana-util -h
```

## Quick Start

List dashboards from a local Grafana:

```bash
grafana-util dashboard list \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --table
```

Preview a dashboard restore before changing Grafana:

```bash
grafana-util dashboard import \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --import-dir ./dashboards/raw \
  --replace-existing \
  --dry-run \
  --output-format table
```

Preview a datasource import:

```bash
grafana-util datasource import \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --import-dir ./datasources \
  --replace-existing \
  --dry-run \
  --output-format table
```

Build a staged sync plan from local JSON:

```bash
grafana-util sync plan \
  --desired-file ./tests/fixtures/rust_sync_demo_desired.json \
  --live-file ./tests/fixtures/rust_sync_demo_live.json
```

## Documentation

- [User Guide](docs/user-guide.md)
- [Traditional Chinese User Guide](docs/user-guide-TW.md)
- [Developer Guide](docs/DEVELOPER.md)
- [Rust Technical Overview](docs/overview-rust.md)

## Compatibility

- Grafana version used for current local live examples: `12.4.1`
- Tested local smoke paths in this repo: Rust live smoke plus local Docker-backed sample-data workflows
