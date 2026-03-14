# Grafana Utilities

Language: English | [Traditional Chinese README.zh-TW.md](README.zh-TW.md)

Grafana Utilities is an admin-focused toolkit for a common Grafana problem: the official UI is fine for one-off clicks, but large environments need inventory, repeatable change control, safer dry-runs, and migration workflows that can survive review.

One simple way to think about it:

> Official Grafana tools are for using Grafana. Grafana Utilities is for operating Grafana.

The project combines Python for CLI flexibility and workflow logic with Rust for performance-oriented paths and standalone binaries. The goal is not to replace Grafana itself. The goal is to make Grafana administration more observable, reviewable, and scriptable.

## Why Admins Need This

When you are managing dozens of datasources, hundreds of dashboards, and multiple Grafana environments, the hard part is rarely "how do I click this screen". The hard part is operational control.

- Import/export friction:
  UI and ad hoc API calls make it hard to preserve structure, UIDs, and predictable replay behavior across environments.
- Inventory blind spots:
  It is hard to answer basic questions such as "what exists now?", "which datasources are actually in use?", and "what changed since the last snapshot?"
- Risky live mutations:
  Datasource and access changes can break dashboards, alerts, or automation if they are applied without preview.
- Fragmented governance:
  Dashboards, datasources, alerting, users, teams, and service accounts often end up managed through different manual habits instead of one repeatable workflow.

Grafana Utilities turns those problems into explicit CLI operations with stable output, dry-run support, diffable artifacts, and environment-to-environment replay flows.

## What It Does Well

- Environment inventory:
  List live dashboards, datasources, alerting resources, users, teams, and service accounts without browsing the UI page by page.
- Backup and replay:
  Export Grafana state into versionable JSON and replay it back into the same environment or another one.
- Change review:
  Compare local export bundles with live Grafana before import or cleanup work.
- Safer live operations:
  Use dry-run output before mutating dashboards, datasources, access state, and other operator-managed resources.
- Governance-oriented inspection:
  Analyze dashboard structure, datasource usage, and query inventory in more depth than a normal UI browse flow.

## Core Capabilities

1. Dashboard administration
- Export, import, diff, and inspect dashboards with folder-aware workflows and machine-readable output.
- Analyze dashboard datasource usage and query inventory for migration review and governance work.

2. Datasource administration
- Inventory, export, import, diff, and live add/delete datasources.
- Preview datasource mutations with dry-run output before writing to Grafana.

3. Access administration
- Manage users, teams, service accounts, and service-account tokens.
- Export/import/diff access snapshots for repeatable reconciliation instead of one-off manual changes.

4. One operator CLI
- Use the same top-level tool across `dashboard`, `datasource`, `alert`, and `access`.
- Prefer table/csv/json output modes depending on whether the caller is a human or automation.

## Supported Grafana Resources

| Resource | List | Export | Import | Diff | Inspect | Add | Modify | Delete | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Dashboards | ✓ | ✓ | ✓ | ✓ | ✓ | - | - | - | Inventory, backup, restore, and cross-environment migration |
| Datasources | ✓ | ✓ | ✓ | ✓ | - | ✓ | - | ✓ | Inventory, replay, drift review, and live datasource administration |
| Alert rules and alerting resources | ✓ | ✓ | ✓ | ✓ | - | - | - | - | Covers alert rules, contact points, mute timings, and templates |
| Users | ✓ | ✓ | ✓ | ✓ | - | ✓ | ✓ | ✓ | Access workflows support snapshot export/import and drift review, including optional `--with-teams` membership state |
| Teams (alias: group) | ✓ | ✓ | ✓ | ✓ | - | ✓ | ✓ | ✓ | Team membership and team administration with export/import and drift comparison |
| Service accounts | ✓ | ✓ | ✓ | ✓ | - | ✓ | ✓ | ✓ | Service account lifecycle management with snapshot export/import and drift review |
| Service account tokens | ✓ | - | - | - | - | ✓ | - | ✓ | Token creation, review, and revocation |

### Access command support design

Access workflows follow one operator model across the project:

- `access user export|import|diff` handles user snapshots and optional team membership state.
- `access team export|import|diff` handles team snapshots and membership/admin drift review.
- `access service-account export|import|diff` handles automation-identity snapshots and drift review for mutable account state.
- `team import` performs deterministic membership sync and requires `--yes` when existing memberships would be removed.
- Export/import snapshot files are intended for controlled migration, cleanup review, and repeatable reconciliation.

## How To Think About It

Use this tool when you want Grafana operations to behave more like infrastructure operations:

- observable
- reviewable
- repeatable
- scriptable

If your current process depends on "open Grafana, click around, hope nothing drifted", this tool is meant to replace that with explicit inventory, export, diff, and import flows.

## Entry Points

Installed CLI:

```text
grafana-util <domain> <command> [options]
```

Source-tree entrypoints:

```text
python3 -m grafana_utils <domain> <command> [options]
cargo run --bin grafana-util -- <domain> <command> [options]
```

Main domains:

- `dashboard`: export, import, list, diff, inspect
- `datasource`: list, add, delete, export, import, diff
- `alert`: export, import, diff, list
- `access`: users, teams, service accounts, tokens

## Quick Start

Export dashboards:

```bash
grafana-util dashboard export \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --export-dir ./dashboards \
  --overwrite
```

List dashboards:

```bash
grafana-util dashboard list \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --table
```

Dry-run a dashboard restore:

```bash
grafana-util dashboard import \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --import-dir ./dashboards/raw \
  --replace-existing \
  --dry-run \
  --table
```

Compare exported dashboards with live Grafana:

```bash
grafana-util dashboard diff \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --import-dir ./dashboards/raw
```

## Install

Python package:

```bash
python3 -m pip install .
```

Rust binary from source:

```bash
cd rust
cargo build --release
```

## Documentation

- Operator guide: [docs/user-guide.md](docs/user-guide.md)
- Traditional Chinese operator guide: [docs/user-guide-TW.md](docs/user-guide-TW.md)
- Python implementation overview: [docs/overview-python.md](docs/overview-python.md)
- Rust implementation overview: [docs/overview-rust.md](docs/overview-rust.md)
- Maintainer notes: [docs/DEVELOPER.md](docs/DEVELOPER.md)

## Compatibility

- RHEL 8 and later
- Python runtime target: 3.9+
