# Competitor Analysis

Date: 2026-03-15
Scope: OSS Grafana backup / restore / sync tools and the current positioning of this repo.

## Purpose

This document captures the current OSS landscape around Grafana backup and restore tooling.

It exists to answer three practical questions:

- which projects are still meaningfully active
- what problem shape each project actually solves
- where `grafana-utils` is stronger, weaker, or simply different

## Summary

The OSS market has many Grafana backup-style tools, but most of them fall into one of these buckets:

- dashboard-centric export/import helpers
- raw snapshot backup tools
- environment-specific automation samples
- declarative GitOps or IaC surfaces rather than true backup/restore products

`grafana-utils` already sits outside the narrow backup category.

Its current strength is not merely exporting and restoring dashboards. Its actual differentiators are:

- multi-resource coverage across dashboard, datasource, alerting, and access workflows
- dry-run and diff-first operator safety
- multi-org export/import routing and missing-org recreation
- inventory, inspection, and governance reporting beyond simple backup

That makes the project closer to an operator migration and governance CLI than to a generic save/restore utility.

## External Landscape

### 1. `ysde/grafana-backup-tool`

Repository:

- <https://github.com/ysde/grafana-backup-tool>

Maintenance read:

- appears maintained
- GitHub showed latest release `1.8.0` dated 2024-12-22 when reviewed on 2026-03-15

Primary value:

- one of the best-known traditional Grafana backup tools
- focused on snapshot-style backup and restore workflows
- broad resource coverage compared with many smaller tools

Observed feature shape:

- backup and restore of dashboards, datasources, folders, teams, orgs, users, snapshots, annotations, and some other Grafana resources
- practical for disaster recovery or recurring exports

Implication for this repo:

- this is the clearest reference implementation for backup productization
- it is still less differentiated on review safety, inspection depth, and controlled migration workflows than `grafana-utils`

### 2. `grafana-tools/grafana-backup`

Repository:

- <https://github.com/grafana-tools/sdk>

Maintenance read:

- weak maintenance signal
- repository branding includes `[ON HOLD]`
- GitHub showed latest release `1.0.4` dated 2023-07-20 when reviewed on 2026-03-15

Primary value:

- older backup-oriented utility with a broader monitoring-stack framing

Observed feature shape:

- backup and restore oriented
- no strong sign of ongoing evolution around newer Grafana operator needs

Implication for this repo:

- useful as evidence that "simple backup" alone is no longer a strong differentiator
- not a strong current benchmark for roadmap direction

### 3. `beam-cloud/beam-dashboard-manager`

Repository:

- <https://github.com/beam-cloud/beam-dashboard-manager>

Maintenance read:

- active maintenance signal
- GitHub showed latest release `v1.13.7` dated 2025-09-24 when reviewed on 2026-03-15

Primary value:

- content and resource packaging for Grafana dashboards and related assets
- supports local and remote sources such as GitHub and URLs

Observed feature shape:

- import/export of dashboards, folders, datasources, alerts, and plugins
- stronger content-bundle ergonomics than a minimal backup script

Implication for this repo:

- closer competitor than many raw backup tools
- still does not obviously match `grafana-utils` on access workflows, inspection/governance depth, or multi-org replay guardrails

### 4. `aws-samples/grafana-automated-backup-tool`

Repository:

- <https://github.com/aws-samples/grafana-automated-backup-tool>

Maintenance read:

- sample-quality project rather than a broad OSS product
- maintenance signal looked limited when reviewed on 2026-03-15

Primary value:

- automated backup example for Amazon Managed Grafana

Observed feature shape:

- environment-specific automation
- useful as a reference for scheduled backup workflows in managed AWS environments

Implication for this repo:

- not a direct benchmark for general Grafana operator tooling
- more relevant as a reminder that scheduled backup packaging may matter if the repo later wants recurring DR use cases

### 5. Grafana Git Sync

Documentation:

- <https://grafana.com/docs/grafana/latest/observability-as-code/provision-resources/provisioned-dashboards/>

Maintenance read:

- official Grafana feature under active development
- documentation still indicated non-GA maturity when reviewed on 2026-03-15
- different Grafana docs surfaces described it as `experimental` or `public preview` depending on product context

Primary value:

- Git-backed synchronization of Grafana resources, especially dashboards

Observed feature shape:

- GitOps-oriented provisioning rather than backup/restore
- not a full-instance reviewable backup system
- does not replace multi-resource migration, access replay, or explicit dry-run/diff contracts

Implication for this repo:

- important strategic reference for declarative sync direction
- should not cause the repo to collapse its current migration/inspection value into a dashboard-only Git feature

### 6. `grafana/grizzly`

Repository:

- <https://github.com/grafana/grizzly>

Maintenance read:

- archived on 2025-08-05

Primary value:

- Grafana-as-code workflow built around declarative configuration

Observed feature shape:

- IaC and configuration management, not classic backup and restore

Implication for this repo:

- useful historical context for declarative workflows
- no longer a good active benchmark

## Comparison With `grafana-utils`

### Resource coverage

Typical OSS backup tools center on dashboards and sometimes datasources, folders, orgs, or teams.

`grafana-utils` already covers:

- dashboards
- datasources
- alerting
- access users
- access orgs
- access teams
- service accounts

That gives the repo a broader operator scope than most backup-first tools.

### Safety model

Many OSS tools optimize for "can I export and replay state."

`grafana-utils` puts more emphasis on:

- explicit dry-run output
- live-vs-local diff workflows
- import guardrails such as org matching and folder-path matching
- conservative datasource import contracts

This is a major differentiator for production use.

### Multi-org workflows

A recurring weak point in Grafana tooling is multi-org handling.

`grafana-utils` already treats this as a first-class concern through:

- `--all-orgs` export flows
- `--use-export-org` import routing
- `--only-org-id` filtering
- `--create-missing-orgs` replay behavior

That is materially stronger than a simple current-org snapshot workflow.

### Inspection and governance

Most backup tools stop after export/import.

`grafana-utils` goes further with:

- `inspect-live`
- `inspect-export`
- query report outputs
- graph and governance renderers
- datasource usage, orphan detection, and blast-radius style analysis

This is one of the repo's clearest strategic advantages.

### Access and administrative workflows

The access surface is another meaningful gap versus backup-only tools.

`grafana-utils` supports workflow families for:

- users
- teams
- orgs
- service accounts

That makes the project useful for environment administration, not just content backup.

## Where Other Tools Are Simpler Or Stronger

The repo should still recognize where simpler competitors may win.

### Easier backup-only story

Traditional backup tools can be easier to explain:

- run one command
- store one archive
- restore later

`grafana-utils` is richer, but also more complex to position.

### Disaster recovery framing

Projects like `grafana-backup-tool` are easier to adopt when the user wants:

- recurring backups
- simple restore documentation
- a narrow DR story instead of a broader operator toolkit

The current repo messaging should not assume every user wants inspection or governance depth on day one.

### Remote storage and scheduled job ergonomics

Some backup-oriented tools or examples are easier to combine with:

- cron
- Dockerized scheduled backup jobs
- object storage retention patterns

`grafana-utils` can support these indirectly, but this is not yet one of its most legible strengths.

## Current Positioning Read

The repo should continue to position itself primarily as:

- a Grafana migration, diff, inspection, and governance CLI
- a safety-first operator toolkit for reviewable changes

It should not reposition itself as only:

- a generic backup utility
- a Terraform replacement
- a dashboard-only Git sync layer

That would undersell the current differentiators and push the project into more crowded or less defensible categories.

## Strategic Recommendations

### Keep

- keep emphasizing dry-run, diff, and preflight trustworthiness
- keep treating multi-org routing as a core capability
- keep investing in inspection, dependency analysis, and governance reporting
- keep the project identity centered on operator workflows, not just data export

### Consider

- consider adding a more explicit "backup bundle" story only if it reuses current export contracts and safety checks
- consider documenting one recommended scheduled backup pattern for operators who want recurring snapshots
- consider bundle metadata and validation improvements that make exported state easier to archive and verify

### Avoid

- avoid building a second parallel backup-only command model that bypasses existing import/export/diff contracts
- avoid reducing the roadmap to dashboard-only Git synchronization in response to official Grafana Git Sync
- avoid chasing broad platform scope that obscures the CLI's reviewable operator value

## Notes For Roadmap

This review supports the current roadmap direction in `docs/internal/project-roadmap.md`.

The strongest competitor-informed conclusions are:

- inspection and governance remain strong differentiators and should continue to deepen
- promotion and preflight work are more defensible than a race toward generic backup parity alone
- declarative sync is strategically relevant, but should remain constrained and safety-first
- a backup or restore command family is optional, not mandatory, and only valuable if it simplifies packaging without splitting the contract model

## Sources Reviewed

- <https://github.com/ysde/grafana-backup-tool>
- <https://github.com/grafana-tools/sdk>
- <https://github.com/beam-cloud/beam-dashboard-manager>
- <https://github.com/aws-samples/grafana-automated-backup-tool>
- <https://grafana.com/docs/grafana/latest/observability-as-code/provision-resources/provisioned-dashboards/>
- <https://github.com/grafana/grizzly>

## Local Repo References

- `README.md`
- `docs/DEVELOPER.md`
- `docs/internal/project-roadmap.md`
