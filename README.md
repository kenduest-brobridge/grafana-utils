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

### 5. Dashboard Snapshots & Screenshots
- **High-Fidelity Captures**: Capture full dashboards or individual panels as PNG, JPEG, or PDF using headless Chromium.
- **State Replay**: Support replaying template variables and query states via URL or CLI parameters to ensure screenshots reflect the desired data state.
- **Reporting Ready**: Add customizable dark headers with titles, URLs, and timestamps directly to captured images.

### Support Matrix

| Domain | List / Inspect / Capture | Add / Modify / Delete | Export / Import / Diff | Notes |
| --- | --- | --- | --- | --- |
| Dashboard | Yes | No | Yes | Import-driven changes, folder-aware migration, dry-run support, and screenshot/PDF capture |
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

**GitHub Releases:**
Published release assets are available at:
`https://github.com/kenduest-brobridge/grafana-utils/releases`

Examples:
```bash
# Install a published Python wheel
python3 -m pip install \
  https://github.com/kenduest-brobridge/grafana-utils/releases/download/vX.Y.Z/grafana_util-X.Y.Z-py3-none-any.whl

# Or install the published source distribution
python3 -m pip install \
  https://github.com/kenduest-brobridge/grafana-utils/releases/download/vX.Y.Z/grafana_util-X.Y.Z.tar.gz
```

Download the prebuilt Rust binaries for your target platform from the same Releases page when a tagged release is published.

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

`dashboard export` writes `raw/` API-import JSON, `prompt/` UI-import JSON, and raw inventory metadata including `folders.json`, `datasources.json`, and `permissions.json` for dashboard/folder ACL backup.
With `--all-orgs`, the export root `export-metadata.json` now summarizes every exported org instead of only one scoped org.
`dashboard import` currently ignores `raw/permissions.json`, so the permission bundle remains export-only backup metadata for review and future restore flows.

**Preview Changes Before Importing:**
```bash
grafana-util dashboard import \
  --url http://localhost:3000 \
  --import-dir ./dashboards/raw \
  --replace-existing \
  --dry-run --table
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
