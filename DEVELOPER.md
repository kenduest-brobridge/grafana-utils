# Developer Notes

This document is for maintainers. Keep `README.md` GitHub-facing and task-oriented; put implementation detail, internal tradeoffs, and maintenance notes here.

## Repository Scope

- `grafana-utils.py`: dashboard export/import utility
- `grafana-alert-utils.py`: alerting resource export/import utility
- `test_dump_grafana_dashboards.py`: dashboard utility unit tests
- `test_grafana_alert_utils.py`: alerting utility unit tests

## Python Baseline

- Both Python entrypoints are kept parseable on Python 3.6 syntax for RHEL 8 compatibility.
- Avoid Python 3.9+ built-in generics such as `list[str]`.
- Avoid Python 3.10 union syntax such as `str | None`.
- Prefer `typing.List`, `typing.Dict`, `typing.Optional`, `typing.Set`, and `typing.Tuple`.

## Dashboard Utility

### CLI shape

- Mode selection is explicit.
- Use `python3 grafana-utils.py export ...` for export.
- Use `python3 grafana-utils.py import ...` for import.
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
- Best input for `python3 grafana-utils.py import`.

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

`grafana-alert-utils.py` currently supports:

- alert rules
- contact points
- mute timings
- notification policies

The alerting export root is `alerts/raw/`, with one subdirectory per resource kind.

### Import behavior by resource kind

- rules: create by default, update by `uid` when `--replace-existing` is set
- contact points: create by default, update by `uid` when `--replace-existing` is set
- mute timings: create by default, update by `name` when `--replace-existing` is set
- notification policies: always applied as one policy tree with `PUT`

### Dashboard-linked alert rules

Alert rules may contain `__dashboardUid__` and `__panelId__` in annotations.

Export behavior:

- preserve the original linkage fields
- export extra linked-dashboard metadata used for import-time repair

Import behavior:

1. try the original `__dashboardUid__`
2. if the target Grafana does not have that UID, fall back to exported dashboard metadata
3. try to find one unique target dashboard match by title, folder title, and slug
4. rewrite `__dashboardUid__` when a unique match is found

Current limitation:

- only `__dashboardUid__` is rewritten
- `__panelId__` is preserved as-is

## Validation

Common checks:

```bash
python3 -m unittest test_dump_grafana_dashboards.py
python3 -m unittest test_grafana_alert_utils.py
python3 -m unittest -v
```

Useful CLI help checks:

```bash
python3 grafana-utils.py -h
python3 grafana-utils.py export -h
python3 grafana-utils.py import -h
python3 grafana-alert-utils.py -h
```

## Documentation split

- `README.md`: public usage and high-level behavior
- `DEVELOPER.md`: maintenance notes, internal architecture, compatibility rules, and implementation tradeoffs
