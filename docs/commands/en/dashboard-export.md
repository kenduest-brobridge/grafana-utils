# dashboard export

## Purpose
Export dashboards to `raw/`, `prompt/`, and `provisioning/` files, with optional `history/` artifacts.

## When to use
Use this when you need a local export tree for later import, review, diff, or file-provisioning workflows. Add `--include-history` when you also need dashboard revision-history artifacts under each exported org scope. The `prompt/` lane is for Grafana UI import, not dashboard API import; use `dashboard raw-to-prompt` when you need to convert ordinary or raw dashboard JSON into prompt JSON.

## Before / After
- **Before**: export is a one-off action, and you only discover later whether the tree is reviewable or reusable.
- **After**: export becomes the first artifact in a repeatable workflow that can feed inspect, diff, dry-run import, and Git review.

## Key flags
- `--export-dir`: target directory for the export tree.
- `--org-id`: export one explicit Grafana org.
- `--all-orgs`: export each visible org into per-org subdirectories. Prefer Basic auth.
- `--flat`: write files directly into each export variant directory.
- `--overwrite`: replace existing export files.
- `--without-dashboard-raw`, `--without-dashboard-prompt`, `--without-dashboard-provisioning`: skip a variant.
- `--include-history`: write dashboard revision-history artifacts under a `history/` subdirectory for each exported org scope.
- `--provisioning-provider-name`, `--provisioning-provider-org-id`, `--provisioning-provider-path`: customize the generated provisioning provider file.
- `--provisioning-provider-disable-deletion`, `--provisioning-provider-allow-ui-updates`, `--provisioning-provider-update-interval-seconds`: tune provisioning behavior.
- `--dry-run`: preview what would be written.

## Notes
- Use `--profile` for normal single-org export flows.
- For `--all-orgs`, prefer admin-backed `--profile` or direct Basic auth because token visibility may not cover every org you expect.
- When you combine `--all-orgs` with `--include-history`, each exported org scope gets its own `org_<id>_<name>/history/` subtree.

## What success looks like
- a `raw/` tree exists for API replay and deeper inspection
- a `prompt/` tree exists when you need a cleaner handoff for UI-style import
- a `history/` tree exists under each exported org scope when you pass `--include-history`
- the export tree is stable enough to commit, diff, or inspect before mutation

## Failure checks
- if dashboards are missing, check org scope before suspecting the exporter
- if multi-org output looks partial, check whether the credential can really see every org
- if expected history artifacts are missing, confirm that you passed `--include-history` and are checking the right org scope
- if the next step is import, confirm whether you should continue from `raw/` or `prompt/`

## Examples
```bash
# Purpose: Export dashboards to `raw/`, `prompt/`, and `provisioning/` files.
grafana-util dashboard export --profile prod --export-dir ./dashboards --overwrite
```

```bash
# Purpose: Export dashboards to `raw/`, `prompt/`, and `provisioning/` files.
grafana-util dashboard export --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --export-dir ./dashboards --overwrite
```

```bash
# Purpose: Export dashboards to `raw/`, `prompt/`, and `provisioning/` files.
grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --export-dir ./dashboards --overwrite
```

```bash
# Purpose: Export dashboards plus per-org revision-history artifacts into a reusable tree.
grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --include-history --export-dir ./dashboards --overwrite
```

## Related commands
- [dashboard analyze (local)](./dashboard-analyze-export.md)
- [dashboard import](./dashboard-import.md)
- [dashboard diff](./dashboard-diff.md)
- [dashboard raw-to-prompt](./dashboard-raw-to-prompt.md)
- [dashboard history](./dashboard-history.md)
