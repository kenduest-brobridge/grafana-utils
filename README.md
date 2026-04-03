# Grafana Utilities

Language: **English** | [繁體中文版](README.zh-TW.md)

`grafana-util` is a Rust-first operator CLI for Grafana inventory, export/import, drift review, staged change workflows, and project-wide status reads.

It is built for estate-level operations, not one-object-at-a-time UI work. The tool is strongest when you need to inspect what exists, export it into reviewable files, compare staged state against live Grafana, and then apply changes deliberately.

## What This Tool Is Good At

Use `grafana-util` when you need to:

- export dashboards, datasources, alerting resources, or access state into reviewable local files
- compare staged files with live Grafana before changing anything
- run `dry-run`, review, and apply workflows instead of mutating Grafana blindly
- inspect dashboard queries, datasource dependencies, and staged inventory at project scale
- summarize a whole project through one `overview` or `status` surface

This project is not trying to replace Grafana's authoring UI. It is focused on migration, audit, governance, handoff, and operator-safe change workflows.

## Main Command Areas

| Area | Primary use | Typical commands |
| --- | --- | --- |
| `dashboard` | dashboard inventory, export/import, diff, analysis, screenshot | `list`, `export`, `import`, `diff`, `inspect-export`, `inspect-live`, `browse`, `screenshot` |
| `datasource` | datasource inventory, masked recovery export, replay, live mutation | `list`, `export`, `import`, `diff`, `browse`, `add`, `modify`, `delete` |
| `alert` | alert management, review/apply workflows, migration bundles | `plan`, `apply`, `delete`, `export`, `import`, `diff`, `list-*`, `add-rule`, `clone-rule` |
| `access` | org, user, team, and service-account inventory and replay | `org ...`, `user ...`, `team ...`, `service-account ...` |
| `change` | staged review-first change workflow | `summary`, `plan`, `review`, `apply`, `preflight`, `bundle-preflight` |
| `overview` | operator-facing whole-project summary | `overview`, `overview live` |
| `status` | canonical staged/live status contract | `status staged`, `status live` |
| `profile` | repo-local connection defaults | `init`, `list`, `show` |

## Output Modes

Many workflows are available through more than one output surface:

| Mode | Best for |
| --- | --- |
| text | default operator summaries and dry-run previews |
| json | CI, scripting, stable machine-readable handoff |
| table / csv | inventory review and spreadsheet-style output |
| interactive TUI | guided browsing and in-terminal review on selected commands |

## Install

Install the latest release:

```bash
curl -fsSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-utils/main/scripts/install.sh | sh
```

Pin a version or install location when needed:

```bash
curl -fsSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-utils/main/scripts/install.sh | BIN_DIR=/usr/local/bin VERSION=v0.6.1 sh
```

If you already have a checkout:

```bash
sh ./scripts/install.sh
```

Build from source:

```bash
cd rust && cargo build --release
```

## Quick Start

Check the installed version and available command surface:

```bash
grafana-util --version
grafana-util version
grafana-util -h
grafana-util dashboard -h
grafana-util datasource -h
grafana-util alert -h
grafana-util access -h
grafana-util change -h
grafana-util overview -h
grafana-util status -h
```

Live-tested examples for this README were validated against local Docker Grafana `12.4.1`.

List dashboards across orgs with datasource context:

```bash
grafana-util dashboard list \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --all-orgs \
  --with-sources \
  --table
```

Inspect an exported dashboard tree:

```bash
grafana-util dashboard inspect-export \
  --import-dir ./dashboards/raw \
  --output-format report-table
```

Preview a dashboard import before applying it:

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

Export alerting resources for review or migration:

```bash
grafana-util alert export \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --output-dir ./alerts \
  --overwrite
```

Build a reviewable alert plan from desired files:

```bash
grafana-util alert plan \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --desired-dir ./alerts/desired \
  --prune \
  --output json
```

## Important Workflow Rules

### Dashboard exports are split into distinct lanes

`dashboard export` writes three different outputs on purpose:

- `raw/`: the canonical grafana-util replay/import lane
- `prompt/`: Grafana UI import lane
- `provisioning/`: Grafana file provisioning lane

These lanes are not interchangeable. Use the lane that matches the workflow you are performing.

### Datasource export uses a masked recovery contract

`datasource export` writes:

- `datasources.json`: the canonical masked recovery and replay contract
- `provisioning/datasources.yaml`: a derived provisioning projection

Use `datasources.json` for restore and replay. Treat `provisioning/` as a projection for Grafana file provisioning, not as the primary restore source.

### Alert management has separate authoring and apply phases

The alert surface is intentionally split:

- author desired-state files with commands such as `add-rule`, `clone-rule`, and `add-contact-point`
- review the delta with `alert plan`
- execute only reviewed operations with `alert apply`

That separation is deliberate. It keeps alert changes reviewable and reduces accidental live mutation.

## Coverage By Area

Use this as the quick maturity map.

| Area | Current depth | Notes |
| --- | --- | --- |
| `dashboard` | deepest surface | most complete analysis, export/import, and review tooling |
| `datasource` | deep and mature | supports live mutation plus file-based recovery/replay |
| `alert` | mature | supports both review/apply management and migration-style export/import |
| `access` | mature | strongest for inventory, replay, and controlled rebuilds |
| `change` | advanced | review-first staged workflow rather than direct mutation |
| `overview` | stable human entrypoint | best first stop for operator handoff and triage |
| `status` | stable contract surface | use when you need one cross-domain staged/live status view |

## Documentation

- [English user guide](docs/user-guide.md)
- [Traditional Chinese user guide](docs/user-guide-TW.md)
- [Developer guide](docs/DEVELOPER.md)
- [Rust technical overview](docs/overview-rust.md)
- [Changelog](CHANGELOG.md)

## Releases

- [Latest release](https://github.com/kenduest-brobridge/grafana-utils/releases/latest)
- [All releases](https://github.com/kenduest-brobridge/grafana-utils/releases)
