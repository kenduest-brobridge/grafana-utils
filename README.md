# Grafana Utilities

Language: **English** | [繁體中文版](README.zh-TW.md)

`grafana-util` is a Rust-first operator CLI for Grafana inventory, migration, review-first import/export, dashboard analysis, project-wide overview/status reads, and staged change workflows.

## Why This Exists

Grafana's UI is good at object-by-object administration. It is much weaker when operators need estate-level work such as bulk export, cross-org inventory, dependency review, migration rehearsal, or reviewable change evidence.

That gap becomes obvious in larger environments:

- dashboard export in the Grafana UI is still awkward for bulk migration because it is largely one dashboard at a time
- cross-environment migration often depends on exporting the "share externally" style payload so the target side can remap datasources during import
- teams need to inventory which datasources exist, which dashboards depend on them, and what query or metric expressions are actually embedded in panels
- permissions, users, orgs, teams, and service-account state are hard to review cleanly from scattered UI pages
- alerting resources are painful to move because Grafana does not give the same kind of complete UI import path for alert bundles
- risky changes should support `diff`, `dry-run`, staged review, and explicit apply steps instead of direct blind mutation

`grafana-util` exists to turn Grafana state into something operators can inventory, export, inspect, diff, dry-run, review, and then apply deliberately. The tool is aimed at migration, audit, governance, and operator handoff workflows rather than dashboard authoring itself.

## Output And Interaction Modes

Many commands are intentionally available through more than one surface so the same workflow can fit human review, TUI exploration, or automation.

| Mode | What it is for | Examples |
| --- | --- | --- |
| Interactive TUI | Guided browsing, review, and in-terminal workflows | `dashboard browse`, `dashboard inspect-export --interactive`, `dashboard inspect-live --interactive`, `datasource browse`, `overview --output interactive`, `status ... --output interactive` |
| Plain text | Default operator-facing summaries and dry-run previews | `change`, `overview`, `status`, dry-run summaries |
| JSON | CI, scripting, structured review artifacts, handoff between commands | import dry-runs, change documents, staged/live status contracts |
| Table / CSV / report outputs | Inventory listings and dashboard analysis reports | list commands, `dashboard inspect-*`, review tables |

## Support Levels By Area

Use this as the quick "how far does this project actually go?" view.

| Area | Support level | What you can do today | Output and interaction surfaces | Notes |
| --- | --- | --- | --- | --- |
| `dashboard` | Deepest surface | list, export, import, diff, delete, inspect live/exported dashboards, query inventory, datasource dependency review, permission export | text, table/csv/json, report modes, interactive TUI, screenshot/PDF | most complete and analysis-heavy area |
| `datasource` | Deep and mature | list, export, import, diff, add, modify, delete, browse live datasources, replay across orgs | text, table/csv/json, interactive browse | supports both live mutation and file-based replay |
| `alert` | Mature management and migration surface | list rules plus contact points, mute timings, templates; build reviewable alert plans; apply reviewed changes; preview explicit deletes; scaffold managed desired-state files; export, import, diff, dry-run bundles | text/json, table/csv/json | supports both operator-first management and older migration/replay flows |
| `access` | Mature inventory and replay surface | manage orgs, users, teams, service accounts; export, import, diff, dry-run; service-account token add/delete | table/csv/json | good for access-state inventory and controlled rebuilds |
| `change` | Advanced staged workflow | build summaries, bundles, preflight checks, plans, review records, apply intents, audits, promotion-preflight documents | text/json | review-first project change lane rather than blind direct mutation |
| `overview` | Human project entrypoint | summarize staged exports or live Grafana into one operator-facing project snapshot | text/json/interactive | best first stop for handoff, triage, and human review |
| `status` | Canonical status contract | render the project-wide staged/live readiness contract for people or automation | text/json/interactive | use when you need one stable cross-domain status surface |

## Quick Capability Matrix

This is the README-sized support matrix. Use the user guide for the full per-command breakdown.

Core resource workflows:

| Area | List | Export | Import | Diff | Inspect / Analyze | Live mutation | TUI |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `dashboard` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| `datasource` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| `alert` | ✓ | ✓ | ✓ | ✓ | - | ✓ | - |
| `access` | ✓ | ✓ | ✓ | ✓ | - | ✓ | - |

Project-level workflows:

| Surface | Staged review | Live read | Interactive view |
| --- | --- | --- | --- |
| `change` | ✓ | ✓ | - |
| `overview` | ✓ | ✓ | ✓ |
| `status` | ✓ | ✓ | ✓ |

## Quick Start

Inspect the maintained command surface:

```bash
grafana-util -h
grafana-util dashboard -h
grafana-util datasource -h
grafana-util alert -h
grafana-util access -h
grafana-util change -h
grafana-util overview -h
grafana-util status -h
```

Live-tested examples for this README were validated against local Docker Grafana `12.4.1` with seeded sample orgs, dashboards, datasources, alerting resources, users, teams, and service accounts.

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

Inspect an exported dashboard set to see datasource usage, query structure, and governance-oriented reports:

```bash
grafana-util dashboard inspect-export \
  --import-dir ./dashboards/raw \
  --output-format report-table
```

`dashboard export` now writes three distinct lanes by default: `raw/` for grafana-util API replay, `prompt/` for Grafana UI import, and `provisioning/` for Grafana file-provisioning artifacts.

Preview a dashboard import before changing Grafana:

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

Export alerting resources for migration or review:

```bash
grafana-util alert export \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --output-dir ./alerts \
  --overwrite
```

Build a reviewable alert management plan from desired YAML or JSON files:

```bash
grafana-util alert plan \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --desired-dir ./alerts/desired \
  --prune \
  --output json
```

Alert now has three operator-facing layers:

1. Authoring layer: `alert add-rule`, `clone-rule`, `add-contact-point`, `set-route`, `preview-route`, plus the lower-level `init` and `new-*` scaffolds. These commands only write or preview desired-state files and never mutate live Grafana.
2. Review/apply layer: `alert plan` compares desired files against live Grafana, and `alert apply` executes only the reviewed create, update, and delete rows.
3. Migration layer: `alert export`, `import`, and `diff` stay focused on raw inventory, replay, and bundle-oriented migration workflows.

Authoring boundaries:
- `add-rule` is for simple threshold or classic-condition style authoring. For richer rules, start from a real rule with `clone-rule`, then hand-edit the desired file.
- `set-route` owns one managed route. Re-running it overwrites that same route instead of merging fields.
- `preview-route` is only a desired-state preview helper. It is not a full Grafana routing simulator.
- `--folder` sets the authored desired metadata only. It is not a live folder resolve/create workflow.
- `--dry-run` is available on the authoring commands so operators can inspect the emitted desired document before writing files.

Short alert workflow:

1. Use the authoring layer to emit or edit desired files under `./alerts/desired`.
2. Run `alert plan` to review what would create, update, block, or delete in live Grafana.
3. Run `alert apply` only with the reviewed plan file.
4. For deletes, remove the desired file, rerun `alert plan --prune`, then apply the reviewed delete plan.

Minimal authoring-to-apply example:

```bash
grafana-util alert init --desired-dir ./alerts/desired
grafana-util alert add-contact-point --desired-dir ./alerts/desired --name pagerduty-primary
grafana-util alert add-rule --desired-dir ./alerts/desired --name cpu-high --folder platform-alerts --rule-group cpu --receiver pagerduty-primary --label team=platform --severity critical --expr A --threshold 80 --above --for 5m
grafana-util alert preview-route --desired-dir ./alerts/desired --label team=platform --severity critical
grafana-util alert plan --url http://localhost:3000 --basic-user admin --basic-password admin --desired-dir ./alerts/desired --output json
grafana-util alert apply --url http://localhost:3000 --basic-user admin --basic-password admin --plan-file ./alert-plan-reviewed.json --approve --output json
```

Complex rule path:

```bash
grafana-util alert clone-rule --desired-dir ./alerts/desired --source cpu-high --name cpu-high-staging --folder staging-alerts --rule-group cpu --receiver slack-platform
# edit ./alerts/desired/rules/cpu-high-staging.yaml or .json by hand
grafana-util alert plan --url http://localhost:3000 --basic-user admin --basic-password admin --desired-dir ./alerts/desired --output json
grafana-util alert apply --url http://localhost:3000 --basic-user admin --basic-password admin --plan-file ./alert-plan-reviewed.json --approve --output json
```

Delete path:

```bash
rm ./alerts/desired/rules/cpu-high.yaml
grafana-util alert plan --url http://localhost:3000 --basic-user admin --basic-password admin --desired-dir ./alerts/desired --prune --output json
grafana-util alert apply --url http://localhost:3000 --basic-user admin --basic-password admin --plan-file ./alert-plan-reviewed.json --approve --output json
```

The full alert authoring, review/apply, migration, and prune-delete guide is in [docs/user-guide.md](./docs/user-guide.md).

## Install Or Build

Install from the repo-owned script:

```bash
curl -fsSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-utils/main/scripts/install.sh | sh
```

Override the install directory or pinned version when needed:

```bash
curl -fsSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-utils/main/scripts/install.sh | BIN_DIR=/usr/local/bin VERSION=v0.5.0 sh
```

If you already have a local checkout, run the script directly from the repo:

```bash
sh ./scripts/install.sh
```

Build locally when you need the current checkout or want to compile from source:

```bash
cd rust && cargo build --release
```

- [Latest release](https://github.com/kenduest-brobridge/grafana-utils/releases/latest)
- [All releases](https://github.com/kenduest-brobridge/grafana-utils/releases)

## Docs

- [English user guide](docs/user-guide.md)
- [Traditional Chinese user guide](docs/user-guide-TW.md)
- [Rust technical overview](docs/overview-rust.md)
- [Developer guide](docs/DEVELOPER.md)

## Compatibility

- OS: Linux, macOS
- Runtime: Rust release binary
- Grafana: validated on `12.4.1`; intended for `8.x` through current `12.x`
