# Grafana Utilities

`grafana-utils.py` exports Grafana dashboards to JSON and can also import JSON back through the Grafana HTTP API.
Use explicit subcommands to avoid mixing the two workflows:

- `python3 grafana-utils.py export ...`
- `python3 grafana-utils.py import ...`

Compatibility:

- supported on RHEL 8 and later
- both Python entrypoints are kept compatible with Python 3.6 syntax so they remain parseable on RHEL 8 environments

The default export root is `dashboards/`. One export run now writes two variants automatically:

- `dashboards/raw/`
- `dashboards/prompt/`

You can suppress one side explicitly:

- `--without-raw`
- `--without-prompt`

## Modes

### `export` parameters

| Parameter | Purpose |
| --- | --- |
| `--url` | Grafana base URL. Default is `http://127.0.0.1:3000`. |
| `--api-token` | Use a Grafana API token. Falls back to `GRAFANA_API_TOKEN`. |
| `--username` | Grafana username. Falls back to `GRAFANA_USERNAME`. |
| `--password` | Grafana password. Falls back to `GRAFANA_PASSWORD`. |
| `--timeout` | HTTP timeout in seconds. Default is `30`. |
| `--verify-ssl` | Enable TLS certificate verification. Disabled by default. |
| `--export-dir` | Root directory for exported dashboards. Default is `dashboards/`. |
| `--page-size` | Grafana dashboard search page size. Default is `500`. |
| `--flat` | Write files directly under the export root instead of folder-based subdirectories. |
| `--overwrite` | Overwrite existing exported files. |
| `--without-raw` | Skip the `dashboards/raw/` export variant. |
| `--without-prompt` | Skip the `dashboards/prompt/` export variant. |

### `raw/` export

- preserves dashboard `uid`
- preserves dashboard `title`
- sets dashboard `id` to `null`
- keeps datasource references unchanged

Example:

```bash
python3 grafana-utils.py export \
  --url http://127.0.0.1:3000 \
  --export-dir ./dashboards \
  --overwrite
```

Use `dashboards/raw/` when you want minimal changes and want to re-import the dashboard with the same identity.

If you only want the prompt variant:

```bash
python3 grafana-utils.py export --export-dir ./dashboards --without-raw
```

### `prompt/` export

`dashboards/prompt/` is generated in the same export run. It is for Grafana web import when you want Grafana to ask which datasource to map during import.

Example:

```bash
python3 grafana-utils.py export \
  --url http://127.0.0.1:3000 \
  --export-dir ./dashboards \
  --overwrite
```

This prompt variant follows the expected Grafana web-import prompt shape:

- creates non-empty `__inputs`
- keeps `__elements`
- adds or normalizes a dashboard datasource variable when applicable
- rewrites dependent template-query datasource references to `${DS_...}`
- normalizes panel datasource references to `{"uid":"$datasource"}` for single-datasource-type dashboards

Notes:

- Mixed-datasource dashboards keep explicit `DS_...` placeholders because one `$datasource` variable cannot safely represent multiple datasource mappings.
- Untyped datasource variables such as `{"uid":"$datasource"}` without a datasource `type` cannot be converted into a Grafana import prompt safely, so they are preserved as-is.

If you only want the raw variant:

```bash
python3 grafana-utils.py export --export-dir ./dashboards --without-prompt
```

### API import

`import` imports dashboard JSON files through the Grafana API.

Example:

```bash
python3 grafana-utils.py import \
  --url http://127.0.0.1:3000 \
  --import-dir ./dashboards/raw \
  --replace-existing
```

This path is for normal dashboard JSON, not prompt JSON. Files containing `__inputs` should be imported through the Grafana web UI instead.

Point `--import-dir` at `dashboards/raw/` explicitly. Do not point it at the combined `dashboards/` root.

## Authentication

Use either API token or username/password.

API token:

```bash
export GRAFANA_API_TOKEN='your-token'
python3 grafana-utils.py export --export-dir ./dashboards
```

Username/password:

```bash
export GRAFANA_USERNAME='your-user'
export GRAFANA_PASSWORD='your-pass'
python3 grafana-utils.py export --export-dir ./dashboards
```

## SSL

SSL verification is disabled by default.

If you want strict verification:

```bash
python3 grafana-utils.py export --verify-ssl
```

## Import behavior summary

- `dashboards/raw/`: best for preserving the same dashboard `uid` with minimal changes.
- `dashboards/prompt/`: best for Grafana web import when you want datasource mapping prompts.
- `python3 grafana-utils.py import --import-dir ./dashboards/raw`: best for API import of normal dashboard JSON.

## Alerting Utility

`grafana-alert-utils.py` is a separate CLI for Grafana alerting resources. It exists to keep alerting logic out of `grafana-utils.py`.

Current scope:

- alert rules
- contact points
- mute timings
- notification policies
- export to a tool-owned JSON format under `alerts/raw/`
- import that same tool-owned format back through the Grafana alerting provisioning HTTP API
- export linked-dashboard metadata for alert rules that carry `__dashboardUid__` / `__panelId__`
- repair linked alert-rule dashboard UIDs on import when the original dashboard UID is missing on the target Grafana

Not in scope:

- message templates
- direct reuse of Grafana provisioning `/export` files for API import

### Alerting export

Example:

```bash
python3 grafana-alert-utils.py \
  --url http://127.0.0.1:3000 \
  --output-dir ./alerts \
  --overwrite
```

This writes:

- `alerts/raw/rules/`
- `alerts/raw/contact-points/`
- `alerts/raw/mute-timings/`
- `alerts/raw/policies/`
- `alerts/index.json`

Default paths:

- rules: `alerts/raw/rules/<folderUID>/<ruleGroup>/<title>__<uid>.json`
- contact points: `alerts/raw/contact-points/<name>/<name>__<uid>.json`
- mute timings: `alerts/raw/mute-timings/<name>/<name>.json`
- notification policies: `alerts/raw/policies/notification-policies.json`

If you want a flat layout:

```bash
python3 grafana-alert-utils.py --output-dir ./alerts --flat
```

### Alerting import

Example:

```bash
python3 grafana-alert-utils.py \
  --url http://127.0.0.1:3000 \
  --import-dir ./alerts/raw \
  --replace-existing
```

Behavior:

- `--replace-existing` updates existing rules by `uid`, contact points by `uid`, and mute timings by `name`
- notification policies are always applied with `PUT`, because Grafana exposes them as one policy tree
- without `--replace-existing`, rule/contact-point/mute-timing import uses create and Grafana will reject conflicting identities
- import expects files exported by `grafana-alert-utils.py`
- do not point `--import-dir` at the combined `alerts/` root
- for rules linked to dashboards, import first tries the original `__dashboardUid__`; if that UID does not exist on the target Grafana, the tool falls back to exported dashboard metadata and looks for a unique dashboard match by title, folder title, and slug before rewriting `__dashboardUid__`

Important limitation:

- Grafana alert provisioning `/export` output is not accepted by this import path
- Grafana documents that provisioning export format is for file/Terraform provisioning, not direct HTTP API round-trip updates
- dashboard linkage repair currently rewrites `__dashboardUid__` only; `__panelId__` is preserved as-is

Validation approach:

- unit tests via `python3 -m unittest -v`
- container-based end-to-end validation during development
- verified export/import of rules, contact points, mute timings, notification policies, and dashboard-linked alert rules

## Validation

Run tests with:

```bash
python3 -m unittest -v
```
