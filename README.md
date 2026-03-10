# Grafana Utilities

`grafana-utils.py` exports Grafana dashboards to JSON and can also import JSON back through the Grafana HTTP API.

The default export root is `dashboards/`. One export run now writes two variants automatically:

- `dashboards/raw/`
- `dashboards/prompt/`

You can suppress one side explicitly:

- `--without-raw`
- `--without-prompt`

## Modes

### `raw/` export

- preserves dashboard `uid`
- preserves dashboard `title`
- sets dashboard `id` to `null`
- keeps datasource references unchanged

Example:

```bash
python3 grafana-utils.py \
  --url http://127.0.0.1:3000 \
  --output-dir ./dashboards \
  --overwrite
```

Use `dashboards/raw/` when you want minimal changes and want to re-import the dashboard with the same identity.

If you only want the prompt variant:

```bash
python3 grafana-utils.py --output-dir ./dashboards --without-raw
```

### `prompt/` export

`dashboards/prompt/` is generated in the same export run. It is for Grafana web import when you want Grafana to ask which datasource to map during import.

Example:

```bash
python3 grafana-utils.py \
  --url http://127.0.0.1:3000 \
  --output-dir ./dashboards \
  --overwrite
```

This prompt variant now follows the working pattern from [`1-prompt.json`](/Users/kendlee/work/scsb/tmp/1-prompt.json):

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
python3 grafana-utils.py --output-dir ./dashboards --without-prompt
```

### API import

`--import-dir` imports dashboard JSON files through the Grafana API.

Example:

```bash
python3 grafana-utils.py \
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
python3 grafana-utils.py --output-dir ./dashboards
```

Username/password:

```bash
export GRAFANA_USERNAME='your-user'
export GRAFANA_PASSWORD='your-pass'
python3 grafana-utils.py --output-dir ./dashboards
```

## SSL

SSL verification is disabled by default.

If you want strict verification:

```bash
python3 grafana-utils.py --verify-ssl
```

## Import behavior summary

- `dashboards/raw/`: best for preserving the same dashboard `uid` with minimal changes.
- `dashboards/prompt/`: best for Grafana web import when you want datasource mapping prompts.
- `--import-dir ./dashboards/raw`: best for API import of normal dashboard JSON.

## Alerting Utility

`grafana-alert-utils.py` is a separate CLI for Grafana alerting resources. It exists to keep alerting logic out of `grafana-utils.py`.

Current scope:

- alert rules
- contact points
- mute timings
- notification policies
- export to a tool-owned JSON format under `alerts/raw/`
- import that same tool-owned format back through the Grafana alerting provisioning HTTP API

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

Important limitation:

- Grafana alert provisioning `/export` output is not accepted by this import path
- Grafana documents that provisioning export format is for file/Terraform provisioning, not direct HTTP API round-trip updates

Validation done in this workspace:

- unit tests via `python3 -m unittest -v`
- live Docker round-trip against Grafana `12.4.1`
- verified export/import of one alert rule, one contact point, one mute timing, and one notification policy tree

## Validation

Run tests with:

```bash
python3 -m unittest -v
```
