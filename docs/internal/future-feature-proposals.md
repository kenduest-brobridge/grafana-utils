# Future Feature Proposals & Enhancements

Reviewed: 2026-03-24

This file should only keep ideas that are still future-facing. Several earlier
items are now partially covered by existing inspection, governance, sync, or
workbench paths, so each section below calls out current status before listing
the remaining gap.

## 1. Resource Scavenging & Cleanup Automation

Current status:

- Partially covered.
- Dashboard inspection already reports datasource usage, datasource inventory,
  orphaned datasources, mixed-datasource dashboards, and governance risks.
- What is still missing is cleanup automation, stale-resource age tracking,
  and safe review-first remediation flows.

Remaining proposals:

- **Zombie Dashboard Detection**
  - Use Grafana API access timestamps or equivalent signals to identify
    dashboards not accessed for a configurable period.
- **Unused Datasource Detection**
  - Identify datasources that are not referenced by dashboards and also show no
    recent usage signal inside a configurable time window such as 30 days.
- **Dead Link Detection**
  - Flag dashboards or panels that reference deleted datasources, broken
    template variables, or otherwise invalid upstream references.
- **Dashboard Version Pruning**
  - Provide a safe review-first path to clean up excessive dashboard version
    history beyond a configured threshold.
- **Bulk Deletion/Archiving**
  - Safe mechanisms to remove or move identified stale resources after review.

## 2. Performance & Quality Audit

Current status:

- Partially covered.
- Query-family analyzers and dashboard inspection contracts exist, but they do
  not yet provide stronger operator-facing quality scoring.

Remaining proposals:

- **Dashboard Performance Linter**
  - Add a stronger rule engine on top of the current query-feature extraction
    so expensive Prometheus, Flux/Influx, and SQL query patterns can be linted
    consistently instead of surfaced as raw traits only.
  - Detect dangerous wide-scope queries such as missing label filters,
    unbounded or overly large time windows, and obviously unsafe wildcard
    usage.
- **Panel Load Auditing**
  - Warn if a single dashboard exceeds a threshold of panels that degrades UI
    performance.
- **Layout Repair**
  - Validate and optionally normalize `gridPos` blocks to resolve panel
    overlap, out-of-bounds placement, and large accidental gaps after manual
    JSON edits.
- **Variable Dependency Mapping**
  - Detect circular dependencies or broken filter chains in template variables.
  - Extend into chained-variable latency and fan-out analysis so operators can
    see when one slow variable can stall a whole dashboard.

## 3. Alert Simulation & Confidence Validation

Current status:

- Mostly not started.
- Alert sync and related contracts exist, but they do not yet provide
  historical replay, rule confidence scoring, or notification-path simulation.

Remaining proposals:

- **Alert Backtesting**
  - Given an alert rule and a historical lookback window, estimate how many
    times the rule would have fired and resolved before enabling it in
    production.
- **Parallel Rule Replay**
  - Use Rust concurrency to fetch historical evaluation inputs efficiently for
    higher-volume backtest scenarios.
- **Routing Path Verification**
  - Simulate alert labels against notification policies and contact points to
    verify whether Slack, PagerDuty, or other targets will match as intended.

## 4. Migration & Provisioning Support

Current status:

- Partially started.
- Export/import/diff contracts are strong, and the staged `sync` workflow
  already provides desired-state planning, review, preflight, audit, and gated
  apply paths.
- What is still missing is richer migration mapping, stronger rewrite support,
  and broader provisioning/IaC export surfaces.

Remaining proposals:

- **Datasource Migration Mapper**
  - Add cross-type migration helpers for large architecture changes such as
    InfluxDB to Prometheus, including variable syntax rewrites and query
    keyword translation where deterministic mappings are possible.
- **Credential Rotation & Proxy Rollout**
  - Bulk update datasource credentials or proxy settings and verify the result
    with explicit health checks before marking the batch successful.
- **Provisioning YAML Generator**
  - Convert existing live dashboards or datasources into Grafana
    `provisioning/*.yaml` layouts for GitOps migration.
- **UID Conflict Resolver**
  - Automate re-mapping of UIDs and datasource IDs when migrating resources
    between different Grafana instances.
- **State-Based Provisioning Engine**
  - Deepen the existing desired-state sync model into a more operator-friendly
    `grafana.yaml` style workflow for folders, teams, permissions,
    datasources, dashboards, and selected alert resources when the current
    plan/dry-run contract is mature enough.
- **Tenant Isolation Validation**
  - Check that dashboards inside a folder only reference datasources and
    permissions intended for that tenant or team boundary.

## 5. Advanced Security & Secret Management

Current status:

- Partially staged only.
- Datasource secret-provider workbench and bundle-preflight coverage exist, but
  provider IO is still intentionally unwired.

Remaining proposals:

- **External Secret Provider Integration**
  - Formalize support for Vault, AWS Secrets Manager, Azure Key Vault, or
    equivalent backends on top of the current staged contract.
- **Service Account Audit**
  - Generate reports on service-account or API-key expiration, usage, and
    rotation posture for compliance.

## 6. Infrastructure as Code Bridging

Current status:

- Mostly not started.
- Today the project supports export/import/diff and stronger replay contracts,
  but not direct Terraform/Pulumi generation or two-live-instance structural
  diffing as a first-class workflow.

Remaining proposals:

- **Terraform/Pulumi Exporter**
  - Export existing manual Grafana configurations into HCL or Pulumi code for
    formal IaC management.
- **Cross-Environment Diffing**
  - Deep diff entire folder structures between two Grafana instances to
    identify drift without relying on manual export staging.

## 7. Multi-Instance Governance & Drift Audit

Current status:

- Mostly not started.
- The repo can already compare and inspect one environment at a time, but it
  does not yet provide a first-class global fleet audit view.

Remaining proposals:

- **Global Instance Audit**
  - Compare Grafana versions, plugin versions, and other baseline inventory
    details across multiple instances or regions.
- **Permission Drift Monitoring**
  - Detect manual permission changes in the UI that no longer match the
    reviewed Git-managed or exported state.

## 8. Rust Structure Candidates

Current status:

- Architectural suggestion only.
- Existing large modules such as `common.rs` and `cli.rs` should not absorb all
  future behavior by default.

Remaining proposals:

- **`src/linter/`**
  - Hold dashboard/static analysis rules, scoring, and lint-report contracts.
- **`src/simulator/`**
  - Hold alert historical replay, policy-routing simulation, and related test
    fixtures.
- **`src/optimizer/`**
  - Hold JSON normalization, cleanup, and size/layout-oriented rewrite helpers.
