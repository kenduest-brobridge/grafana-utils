# Grafana Utilities User Guide

`grafana-util` is a professional command-line interface (CLI) written in Rust, designed for Grafana administrators and DevOps engineers. It provides a comprehensive suite of tools for managing Grafana resources, including dashboards, data sources, alerts, and access control. This guide focuses on the Rust implementation of the CLI.

---

## 1. Getting Started

### 1.1 Installation

To build the `grafana-util` binary from the source repository, ensure you have the Rust toolchain installed and run:

```bash
cargo build --release --manifest-path rust/Cargo.toml --bin grafana-util
```

When you build through `make build-rust`, the compiled binary is written to `build/rust/release/grafana-util`. Verify the installation by displaying the help menu:

```bash
grafana-util --help
```

### 1.2 Authentication & Shared Options

Most commands support a common set of flags for authentication and connectivity.

| Option | Description |
| --- | --- |
| `--url` | Base URL of the Grafana instance (default: `http://localhost:3000`). |
| `--token` | Grafana API Token or Service Account Token. |
| `--basic-user` | Username for Basic Authentication. |
| `--basic-password` | Password for Basic Authentication. |
| `--prompt-password` | Interactively prompt for the Basic Authentication password. |
| `--org-id` | Target a specific organization ID for the request. |
| `--timeout` | HTTP request timeout in seconds. |
| `--verify-ssl` | Enable or disable TLS certificate verification. |

**Authentication Rules:**
- Use either Token-based authentication or Basic Authentication, but not both.
- Basic Authentication is typically required for administrative tasks across multiple organizations.

---

## 2. Dashboard Management (`dashboard`)

The `dashboard` command group handles the lifecycle of Grafana dashboards, including inventory, backup, and restoration.

### 2.1 Inventory and Listing
List dashboards with detailed metadata, including folder paths and associated data sources.

```bash
grafana-util dashboard list --url <URL> --basic-user <USER> --table --with-sources
```

### 2.2 Exporting Dashboards
Backup dashboards to a local directory. The export includes both a "raw" format suitable for restoration and a "prompt" format for UI-oriented reviews.

```bash
grafana-util dashboard export --export-dir ./backups/dashboards --overwrite
```

### 2.3 Importing and Restoring
Import dashboards from a local directory. Use `--dry-run` to preview changes before applying them.

```bash
grafana-util dashboard import \
  --import-dir ./backups/dashboards/raw \
  --replace-existing \
  --dry-run \
  --output-format table
```

### 2.4 Inspection and Diffing
Compare local dashboard definitions with live versions or inspect exported files for consistency.

```bash
grafana-util dashboard diff --dashboard-file ./dashboard.json --url <URL>
```

---

## 3. Data Source Management (`datasource`)

Manage Grafana data sources, including provisioning and secret handling.

### 3.1 Listing Data Sources
View all configured data sources and their types.

```bash
grafana-util datasource list --table
```

### 3.2 Adding and Modifying
Programmatically add new data sources.

```bash
grafana-util datasource add \
  --name "Prometheus-Main" \
  --type "prometheus" \
  --datasource-url "http://prometheus:9090" \
  --access "proxy"
```

### 3.3 Backup and Migration
Export and import data source configurations, with support for secret placeholders.

```bash
# Export
grafana-util datasource export --export-dir ./backups/datasources

# Import (Dry-Run)
grafana-util datasource import \
  --import-dir ./backups/datasources \
  --dry-run \
  --output-format table
```

---

## 4. Alerting Management (`alert`)

Manage Grafana Alerting resources, including alert rules, contact points, and notification policies.

### 4.1 Resource Listing
List various alerting components for auditing and review.

```bash
grafana-util alert list-rules
grafana-util alert list-contact-points
grafana-util alert list-mute-timings
grafana-util alert list-templates
```

### 4.2 Synchronization
Export and import alert configurations, facilitating migration between environments.

```bash
grafana-util alert export --export-dir ./backups/alerts
grafana-util alert import --import-dir ./backups/alerts --dry-run
```

---

## 5. Access & Identity Management (`access`)

Manage organizations, users, teams, and service accounts.

### 5.1 Organizations
List and create Grafana organizations.

```bash
grafana-util access org list --with-users --table
grafana-util access org add --name "Engineering"
```

### 5.2 Users and Teams
Manage user accounts and team memberships across the Grafana instance.

```bash
# List users globally
grafana-util access user list --scope global --table

# List teams and their members
grafana-util access team list --with-members --table
```

### 5.3 Service Accounts
Lifecycle management for Service Accounts, including backup and restore.

```bash
# List service accounts
grafana-util access service-account list --table

# Backup service accounts
grafana-util access service-account export --export-dir ./backups/service-accounts
```

---

## 6. Staged Sync (GitOps Workflow) (`sync`)

The `sync` command group enables a GitOps-style workflow by comparing "desired" state files against "live" Grafana environments.

### 6.1 Workflow Steps
1.  **Summary**: Get a high-level overview of resources in a desired state file.
2.  **Plan**: Generate a detailed execution plan comparing desired vs. live state.
3.  **Preflight**: Validate that all dependencies (e.g., data sources, plugins) are available.
4.  **Apply**: Execute the plan to synchronize Grafana with the desired state.

### 6.2 Example: Generating a Sync Plan
```bash
grafana-util sync plan \
  --desired-file ./desired-state.json \
  --live-file ./current-live-state.json
```

---

## 7. Advanced Usage

### 7.1 Output Formats
Most commands support `--output-format`. Use `table` for human readability and `json` for programmatic integration (e.g., piping to `jq`).

### 7.2 Multi-Organization Support
For large-scale Grafana instances, use flags like `--all-orgs`, `--use-export-org`, and `--create-missing-orgs` to manage resources across multiple tenants effectively.

### 7.3 Dry-Run Mode
Always use `--dry-run` with `import`, `add`, or `sync` commands to verify the intended changes without modifying the live Grafana environment.
