# 📊 Grafana Utilities (Operator Toolkit)

Language: **English** | [繁體中文版](README.zh-TW.md)

`grafana-utils` is an operator-focused toolkit designed for Grafana administrators and SREs.

## Project Status

This project is still in active development.

- Expect ongoing CLI, workflow, and documentation refinement.
- Bug reports, edge cases, and operator feedback are welcome.
- Please use GitHub issues or pull requests for reporting and discussion.
- Maintainer: `Kenduest`

### 💡 The Philosophy: Why This Tool?

**"Official tools are for users. Grafana Utilities is for admins."**

While the official Grafana UI and CLI are excellent for day-to-day interactions, they often fall short when managing **environments at scale**—dozens of datasources, hundreds of dashboards, and multiple clusters. Administrators frequently face these operational challenges:

- **Inventory Blind Spots**: Difficult to quickly answer "What assets exist?", "Which datasources are unused or broken?", or "What changed since the last snapshot?"
- **Migration Friction**: Manual export/import struggles to preserve folder structures and UID consistency without repeatable, automated workflows.
- **Risky Live Mutations**: Applying changes directly to production is dangerous. The lack of a preview (dry-run) mechanism can lead to broken dashboards or silent alert failures.
- **Fragmented Governance**: Dashboards, datasources, and access rules often drift into inconsistent manual habits instead of a standardized workflow.

`grafana-utils` turns these problems into **standardized CLI operations** with stable outputs, diffing capabilities, dry-run support, and environment-to-environment state synchronization.

---

## 🚀 Key Capabilities & Advantages

### 1. Deep Environment Inventory
- Full-spectrum scanning of Dashboards, Datasources, Alerting rules, Organizations, Users, Teams, and Service Accounts.
- Multiple output modes (Table, CSV, JSON) for human review or CI/CD integration.

### 2. Safe Change Management
- **Diffing**: Compare local snapshots with live environments before committing any changes.
- **Dry-run Support**: Preview expected actions (Create/Update/Skip) in detail to ensure operational safety.

### 3. Smart Backup & Migration
- **Folder-aware Workflows**: Automatically reconstruct folder hierarchies and handle path-matching across environments.
- **State Replay**: Transform Grafana state into Git-ops-friendly JSON for rapid restoration or environment mirroring.

### 4. Governance-Oriented Inspection
- Analyze dashboard structures and query inventory to identify redundant or inefficient resources.
- Optimized for large-scale instances using high-performance pagination and processing (powered by Rust).

### Support Matrix

| Domain | List / Inspect | Add / Modify / Delete | Export / Import / Diff | Notes |
| --- | --- | --- | --- | --- |
| Dashboard | Yes | No | Yes | Import-driven changes, folder-aware migration, dry-run support, and routed multi-org export/import with missing-org creation |
| Alerting | Yes | No | Yes | Import-driven rule and contact-point workflows |
| Datasource | Yes | Yes | Yes | Dry-run and diff supported, plus all-org export and routed multi-org import with missing-org creation |
| Access User | Yes | Yes | Yes | Supports `--password-file` / `--prompt-user-password` and `--set-password-file` / `--prompt-set-password` |
| Access Org | Yes | Yes | Yes | Includes org membership replay during import |
| Access Team | Yes | Yes | Yes | Membership-aware export/import/diff |
| Access Service Account | Yes | Yes | Yes | Snapshot export/import/diff plus token add/delete workflows |

---

## 🏗️ Technical Architecture

This project leverages a hybrid approach for efficiency:
- **Python (Workflow Logic)**: Handles CLI definitions, complex business logic, and flexible integration workflows.
- **Rust (Performance Engine)**: Powers high-performance data parsing, query validation, and provides standalone binaries.

---

## 🛠️ Quick Start

### Installation

**Python Package:**
```bash
python3 -m pip install .
```

**Rust Binary:**
```bash
cd rust && cargo build --release
```

### Common Usage Example

**Batch Export Dashboards (Preserving Structure):**
```bash
grafana-util dashboard export \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --export-dir ./dashboards \
  --overwrite
```

**Preview Changes Before Importing:**
```bash
grafana-util dashboard import \
  --url http://localhost:3000 \
  --import-dir ./dashboards/raw \
  --replace-existing \
  --dry-run --table
```

### Rust `sync` Workflow

The Rust binary also exposes a staged `sync` workflow for reviewable local JSON contracts before any live mutation:

Canonical demo fixtures live in `tests/fixtures/`:
- `tests/fixtures/rust_sync_demo_desired.json`
- `tests/fixtures/rust_sync_demo_live.json`
- `tests/fixtures/rust_sync_demo_availability.json`
- `tests/fixtures/rust_sync_demo_bundle.json`
- `tests/fixtures/rust_sync_demo_target_inventory.json`

```bash
grafana-util sync summary --desired-file ./tests/fixtures/rust_sync_demo_desired.json
grafana-util sync plan --desired-file ./tests/fixtures/rust_sync_demo_desired.json --live-file ./tests/fixtures/rust_sync_demo_live.json --output json
grafana-util sync review --plan-file ./plan.json --review-token reviewed-sync-plan --output json
grafana-util sync preflight --desired-file ./tests/fixtures/rust_sync_demo_desired.json --availability-file ./tests/fixtures/rust_sync_demo_availability.json
grafana-util sync bundle-preflight --source-bundle ./tests/fixtures/rust_sync_demo_bundle.json --target-inventory ./tests/fixtures/rust_sync_demo_target_inventory.json --availability-file ./tests/fixtures/rust_sync_demo_availability.json
grafana-util sync apply --plan-file ./reviewed-plan.json --approve --output json
```

Operator model:
- `summary` normalizes the desired managed slice.
- `plan` computes create/update/delete/noop/unmanaged operations plus alert assessment.
- `review` marks the plan reviewed and carries trace/lineage metadata forward.
- `preflight` and `bundle-preflight` surface blocking dependencies before apply.
- `apply` emits a gated apply-intent document first; add `--execute-live` only after review and preflight checks are already in place.

Short output examples:

```text
Sync summary
Resources: 3 total, 1 dashboards, 1 datasources, 1 folders, 0 alerts
```

```text
Sync plan
Summary: create=1 update=1 delete=0 noop=1 unmanaged=0
Alerts: candidate=0 plan-only=0 blocked=0
Review: required=true reviewed=false
```

```text
Sync preflight summary
Resources: 3 total
Checks: 4 total, 3 ok, 0 create-planned, 1 blocking
Blocking split: dependency=1 policy=0
```

```text
Sync apply intent
Summary: create=1 update=1 delete=0 executable=2
Review: required=true reviewed=true approved=true
```

---

## 📄 Documentation

- **[Traditional Chinese Guide](docs/user-guide-TW.md)**: Detailed commands and authentication rules.
- **[English User Guide](docs/user-guide.md)**: Standard operator instructions.
- **[Technical Overview (Python)](docs/overview-python.md)** | **[Technical Overview (Rust)](docs/overview-rust.md)**
- **[Developer Guide](docs/DEVELOPER.md)**: Maintenance and contribution notes.

---

## 📈 Compatibility
- **OS**: RHEL 8+, macOS (ARM/Intel), Linux.
- **Runtime**: Python 3.9+.
- **Grafana**: Supports v8.x, v9.x, v10.x+.
