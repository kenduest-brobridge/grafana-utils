# Developer Notes

This document is for maintainers. Keep `README.md` GitHub-facing and task-oriented; put implementation detail, internal tradeoffs, and maintenance notes here.

## Repository Scope

- `cmd/grafana-utils.py`: dashboard export/import utility
- `cmd/grafana-alert-utils.py`: alerting resource export/import utility
- `tests/test_dump_grafana_dashboards.py`: dashboard utility unit tests
- `tests/test_grafana_alert_utils.py`: alerting utility unit tests

## Python Baseline

- Both Python entrypoints are kept parseable on Python 3.6 syntax for RHEL 8 compatibility.
- Avoid Python 3.9+ built-in generics such as `list[str]`.
- Avoid Python 3.10 union syntax such as `str | None`.
- Prefer `typing.List`, `typing.Dict`, `typing.Optional`, `typing.Set`, and `typing.Tuple`.

## Dashboard Utility

### CLI shape

- Mode selection is explicit.
- Use `python3 cmd/grafana-utils.py export ...` for export.
- Use `python3 cmd/grafana-utils.py import ...` for import.
- The export subcommand intentionally uses `--export-dir` instead of `--output-dir` to avoid mixing export terminology with import behavior.

### Export variants

Dashboard export writes two variants by default:

- `raw/`: API-safe dashboard JSON intended for later `import`
- `prompt/`: Grafana web-import JSON with datasource `__inputs`

The two variants serve different consumers and should not be treated as interchangeable.

### Raw export intent

- Keep dashboard JSON close to Grafana's API payload.
- Preserve `uid`.
- Clear numeric `id`.
- Keep datasource references unchanged.
- Best input for `python3 cmd/grafana-utils.py import`.

### Prompt export intent

- Transform datasource references into Grafana web-import placeholders.
- Populate `__inputs`, `__requires`, and `__elements` in the shape Grafana expects.
- Intended for Grafana UI import, not for API re-import.

### Prompt export datasource pipeline

The prompt export rewrite flow is intentionally multi-stage:

1. Fetch datasource catalog from Grafana.
2. Index datasources by both `uid` and `name`.
3. Walk the dashboard tree and collect every `datasource` reference.
4. Normalize each datasource reference into a stable key.
5. Build one generated input mapping per unique datasource reference.
6. Rewrite matching dashboard refs to `${DS_*}` placeholders.
7. If every datasource resolves to the same plugin type, add Grafana's shared `$datasource` variable and collapse panel-level refs to it.

This is why prompt export needs live datasource metadata while raw export does not.

### Dashboard import constraints

- Import expects raw dashboard JSON, not prompt JSON.
- Files containing `__inputs` should be imported through Grafana web UI.
- Import can override folder destination with `--import-folder-uid`.
- Import can set the dashboard version-history message with `--import-message`.

## Alerting Utility

### Supported resource kinds

`cmd/grafana-alert-utils.py` currently supports:

- alert rules
- contact points
- mute timings
- notification policies
- notification message templates

The alerting export root is `alerts/raw/`, with one subdirectory per resource kind.

Default layout:

- `alerts/raw/rules/<folderUID>/<ruleGroup>/<title>__<uid>.json`
- `alerts/raw/contact-points/<name>/<name>__<uid>.json`
- `alerts/raw/mute-timings/<name>/<name>.json`
- `alerts/raw/policies/notification-policies.json`
- `alerts/raw/templates/<name>/<name>.json`

### Import behavior by resource kind

- rules: create by default, update by `uid` when `--replace-existing` is set
- contact points: create by default, update by `uid` when `--replace-existing` is set
- mute timings: create by default, update by `name` when `--replace-existing` is set
- notification policies: always applied as one policy tree with `PUT`
- notification templates: applied with `PUT`; when `--replace-existing` is set, fetch the current template version first and send it back with the update payload

Template handling notes:

- Grafana template identity is the template `name`
- template list may return JSON `null`; treat that as an empty list
- template updates should strip `name` from the request body because the API path already carries the name
- without `--replace-existing`, importing an existing template should fail fast instead of silently updating it

### Alerting import shape and rejection rules

- Import accepts the tool-owned document format emitted by `cmd/grafana-alert-utils.py`
- `detect_document_kind(...)` also accepts plain resource-shaped JSON for rules/contact points/mute timings/policies/templates
- Grafana provisioning `/export` payloads are intentionally rejected for API import
- Reject the combined `alerts/` export root on import; require callers to point at `alerts/raw/`

### Dashboard-linked alert rules

Alert rules may contain `__dashboardUid__` and `__panelId__` in annotations.

Export behavior:

- preserve the original linkage fields
- export extra linked-dashboard metadata used for import-time repair
- when the source dashboard still exists during export, enrich metadata with:
  - `dashboardTitle`
  - `folderTitle`
  - `folderUid`
  - `dashboardSlug`
  - `panelTitle`
  - `panelType`

Import behavior:

1. try the original `__dashboardUid__`
2. if `--dashboard-uid-map` is present, apply that mapping first
3. if `--panel-id-map` is present, rewrite `__panelId__` using the mapped source dashboard UID plus source panel ID
4. if the target Grafana has the mapped or original dashboard UID, stop there
5. otherwise fall back to exported dashboard metadata
6. search target dashboards by exported title, then narrow by folder title and slug
7. rewrite `__dashboardUid__` only when that fallback search resolves to exactly one dashboard

Current limitation:

- automatic fallback only rewrites `__dashboardUid__`
- `__panelId__` is preserved unless `--panel-id-map` is supplied
- panel matching is intentionally explicit; there is no heuristic panel-title-based rewrite

### Mapping file formats

Dashboard UID map:

```json
{
  "old-dashboard-uid": "new-dashboard-uid"
}
```

Panel ID map:

```json
{
  "old-dashboard-uid": {
    "7": "19"
  }
}
```

Notes:

- both mapping loaders coerce keys and values to strings
- panel maps are keyed by source dashboard UID, then source panel ID
- explicit maps take precedence over fallback dashboard metadata matching

### Live validation notes

- Primary automated coverage lives in `tests/test_grafana_alert_utils.py`
- Container-based validation was done against Grafana `12.4.1`
- Verified round-trip coverage includes:
  - rules
  - contact points
  - mute timings
  - notification policies
  - notification templates
  - dashboard-linked rules with repaired `__dashboardUid__`

## Validation

Common checks:

```bash
python3 -m unittest tests.test_dump_grafana_dashboards
python3 -m unittest tests.test_grafana_alert_utils
python3 -m unittest -v
```

Useful CLI help checks:

```bash
python3 cmd/grafana-utils.py -h
python3 cmd/grafana-utils.py export -h
python3 cmd/grafana-utils.py import -h
python3 cmd/grafana-alert-utils.py -h
```

## Documentation split

- `README.md`: public usage and high-level behavior
- `DEVELOPER.md`: maintenance notes, internal architecture, compatibility rules, and implementation tradeoffs
- `docs/internal/ai-status.md` / `docs/internal/ai-changes.md`: internal working notes only; do not treat them as public GitHub-facing documentation

Documentation policy:

- keep `README.md` suitable for GitHub readers
- keep environment-specific validation logs, migration notes, and maintainer-only tradeoffs in `DEVELOPER.md`
- avoid relying on `docs/internal/ai-status.md` and `docs/internal/ai-changes.md` for public project documentation
- if user-facing release history is needed, prefer a curated `CHANGELOG.md`
