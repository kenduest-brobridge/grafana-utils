# Grafana Dashboard Export/Import Tool

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
  --url https://10.21.104.120 \
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
  --url https://10.21.104.120 \
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
  --url https://10.21.104.120 \
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

## Validation

Run tests with:

```bash
python3 -m unittest -v
```
