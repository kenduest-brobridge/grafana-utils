# Grafana Utilities

Traditional Chinese guide: [README.zh-TW.md](README.zh-TW.md)

This repository is for exporting, backing up, migrating, and re-importing Grafana configuration as JSON.

It provides two command-line tools:

- `grafana-utils`: export and import dashboards
- `grafana-alert-utils`: export and import alerting resources such as alert rules, contact points, mute timings, notification policies, and templates

Use this repo when you need to:

- back up dashboards or alerting resources from a Grafana instance
- move dashboards or alerting resources from one Grafana instance to another
- keep Grafana JSON under version control
- prepare dashboards either for API re-import or for Grafana web UI import with datasource prompts

Dashboard workflow is handled by `grafana-utils`. Use explicit subcommands to avoid mixing export and import:

- `grafana-utils export ...`
- `grafana-utils import ...`

Alerting workflow is handled separately by `grafana-alert-utils` because Grafana alerting uses different APIs and file shapes than dashboards.

The examples below use `python3 cmd/...` so they also work directly from the git checkout. If you installed the package, use the same arguments with the installed command names instead.

Compatibility:

- supported on RHEL 8 and later
- both Python entrypoints are kept compatible with Python 3.6 syntax so they remain parseable on RHEL 8 environments

## Installation

Install into the current Python environment:

```bash
python3 -m pip install .
```

Install into a user-local environment without touching the system Python site-packages:

```bash
python3 -m pip install --user .
```

Optional HTTP/2 support is available on Python 3.8+ environments with:

```bash
python3 -m pip install '.[http2]'
```

After installation, use the installed commands:

- `grafana-utils export ...`
- `grafana-utils import ...`
- `grafana-alert-utils ...`

If you are running directly from the git checkout instead of installing the package, keep using the thin wrappers under `cmd/`:

- `python3 cmd/grafana-utils.py export ...`
- `python3 cmd/grafana-utils.py import ...`
- `python3 cmd/grafana-alert-utils.py ...`

The default export root is `dashboards/`. One export run now writes two variants automatically:

- `dashboards/raw/`
- `dashboards/prompt/`

You can suppress one side explicitly:

- `--without-dashboard-raw`
- `--without-dashboard-prompt`

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
| `--without-dashboard-raw` | Skip the `dashboards/raw/` export variant. |
| `--without-dashboard-prompt` | Skip the `dashboards/prompt/` export variant. |

### `raw/` export

- preserves dashboard `uid`
- preserves dashboard `title`
- sets dashboard `id` to `null`
- keeps datasource references unchanged

Example:

```bash
python3 cmd/grafana-utils.py export \
  --url http://127.0.0.1:3000 \
  --export-dir ./dashboards \
  --overwrite
```

Use `dashboards/raw/` when you want minimal changes and want to re-import the dashboard with the same identity.

If you only want the prompt variant:

```bash
python3 cmd/grafana-utils.py export --export-dir ./dashboards --without-dashboard-raw
```

### `prompt/` export

`dashboards/prompt/` is generated in the same export run. It is for Grafana web import when you want Grafana to ask which datasource to map during import.

Example:

```bash
python3 cmd/grafana-utils.py export \
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
python3 cmd/grafana-utils.py export --export-dir ./dashboards --without-dashboard-prompt
```

### API import

`import` imports dashboard JSON files through the Grafana API.

Example:

```bash
python3 cmd/grafana-utils.py import \
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
python3 cmd/grafana-utils.py export --export-dir ./dashboards
```

Username/password:

```bash
export GRAFANA_USERNAME='your-user'
export GRAFANA_PASSWORD='your-pass'
python3 cmd/grafana-utils.py export --export-dir ./dashboards
```

## SSL

SSL verification is disabled by default.

If you want strict verification:

```bash
python3 cmd/grafana-utils.py export --verify-ssl
```

## Import behavior summary

- `dashboards/raw/`: best for preserving the same dashboard `uid` with minimal changes.
- `dashboards/prompt/`: best for Grafana web import when you want datasource mapping prompts.
- `python3 cmd/grafana-utils.py import --import-dir ./dashboards/raw`: best for API import of normal dashboard JSON.

## Alerting Utility

`cmd/grafana-alert-utils.py` is a separate CLI for Grafana alerting resources. It exists to keep alerting logic out of `cmd/grafana-utils.py`.

Current scope:

- alert rules
- contact points
- mute timings
- notification policies
- notification message templates
- export to a tool-owned JSON format under `alerts/raw/`
- import that same tool-owned format back through the Grafana alerting provisioning HTTP API

Not in scope:

- direct reuse of Grafana provisioning `/export` files for API import

### Alerting export

Example:

```bash
python3 cmd/grafana-alert-utils.py \
  --url http://127.0.0.1:3000 \
  --output-dir ./alerts \
  --overwrite
```

This writes:

- `alerts/raw/rules/`
- `alerts/raw/contact-points/`
- `alerts/raw/mute-timings/`
- `alerts/raw/policies/`
- `alerts/raw/templates/`
- `alerts/index.json`

If you want a flat layout:

```bash
python3 cmd/grafana-alert-utils.py --output-dir ./alerts --flat
```

Common usage examples:

API token:

```bash
export GRAFANA_API_TOKEN='your-token'

python3 cmd/grafana-alert-utils.py \
  --url https://grafana.example.com \
  --output-dir ./alerts \
  --overwrite
```

Username/password:

```bash
export GRAFANA_USERNAME='admin'
export GRAFANA_PASSWORD='secret'

python3 cmd/grafana-alert-utils.py \
  --url https://grafana.example.com \
  --output-dir ./alerts \
  --overwrite
```

### Alerting import

Example:

```bash
python3 cmd/grafana-alert-utils.py \
  --url http://127.0.0.1:3000 \
  --import-dir ./alerts/raw \
  --replace-existing
```

Import with linked dashboard or panel remapping:

```bash
python3 cmd/grafana-alert-utils.py \
  --url https://grafana.example.com \
  --import-dir ./alerts/raw \
  --replace-existing \
  --dashboard-uid-map ./dashboard-map.json \
  --panel-id-map ./panel-map.json
```

Example `dashboard-map.json`:

```json
{
  "old-dashboard-uid": "new-dashboard-uid"
}
```

Example `panel-map.json`:

```json
{
  "old-dashboard-uid": {
    "7": "19"
  }
}
```

Behavior:

- `--replace-existing` updates existing rules by `uid`, contact points by `uid`, and mute timings by `name`
- notification policies are always applied with `PUT`, because Grafana exposes them as one policy tree
- notification templates are applied with `PUT`; with `--replace-existing` the tool first reads the current template version and sends it back on update
- without `--replace-existing`, rule/contact-point/mute-timing import uses create and Grafana will reject conflicting identities
- without `--replace-existing`, template import fails if the template name already exists
- import expects files exported by `cmd/grafana-alert-utils.py`
- do not point `--import-dir` at the combined `alerts/` root
- use `--dashboard-uid-map` and `--panel-id-map` when linked alert rules must be remapped during import
- internal matching and mapping details are documented in `DEVELOPER.md`

Important limitation:

- Grafana alert provisioning `/export` output is not accepted by this import path
- Grafana documents that provisioning export format is for file/Terraform provisioning, not direct HTTP API round-trip updates

Validation approach:

- unit tests via `python3 -m unittest -v`
- container-based end-to-end validation during development
- verified export/import of rules, contact points, mute timings, notification policies, notification templates, and dashboard-linked alert rules

## Validation

Run tests with:

```bash
python3 -m unittest -v
```
