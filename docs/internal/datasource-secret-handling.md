# Datasource Secret Handling

`grafana-util` now treats datasource secrets with one simple rule:

- `datasource export` captures config structure only
- `datasource import` is the point where external secret values are injected

## Upstream Grafana API limit

Grafana does not generally return datasource secret plaintext from live read APIs.

What live reads can usually give us:

- normal datasource fields such as `url`, `database`, `jsonData`, `basicAuthUser`
- sometimes `secureJsonFields`, which only tells us which secure field names exist

What live reads do not give us:

- `secureJsonData.token`
- `basicAuthPassword`
- TLS cert/key plaintext
- header secret plaintext

So `grafana-util` cannot export a full secret backup from Grafana alone, because
Grafana never returns the real secret values.

## Design consequence

Because Grafana is not a recoverable datasource secret source:

- `datasources.json` remains the canonical datasource artifact
- plaintext datasource secrets are never exported from live Grafana
- export writes placeholder metadata only
- import resolves those placeholders from an external secret source

This keeps the product model honest:

- export is config capture
- import is secret injection

## Canonical export contract

Datasource export writes:

- `datasources.json`
- `index.json`
- `export-metadata.json`
- `provisioning/datasources.yaml` unless disabled

`datasources.json` may contain `secureJsonDataPlaceholders`, for example:

```json
{
  "uid": "influx-main",
  "name": "Influx Main",
  "type": "influxdb",
  "url": "http://influxdb:8086",
  "jsonData": {
    "organization": "acme",
    "defaultBucket": "metrics"
  },
  "secureJsonDataPlaceholders": {
    "token": "${secret:influx-main-token}"
  }
}
```

Meaning:

- left side `token` = the Grafana secure field name to restore into `secureJsonData.token`
- `${secret:influx-main-token}` = the external secret lookup key that import must resolve

## Current command behavior

### `datasource list`

Purpose:

- inspect live datasource inventory
- inspect local exported datasource inventory

Secret behavior:

- shows non-secret datasource fields
- may show `secureJsonFields` when Grafana returned them
- never shows plaintext secret values

Important note:

- Grafana `GET /api/datasources` does not reliably include `secureJsonFields`
- `GET /api/datasources/uid/:uid` can include secure field names
- neither endpoint gives plaintext secret values

### `datasource export`

Purpose:

- export datasource config and placeholder contract

Secret behavior:

- exports placeholders only
- does not read or write datasource secret companion files
- does not claim to back up live secret plaintext

This means:

- you can fully export replayable config structure
- you cannot recover secret plaintext from Grafana export alone

### `datasource import`

Purpose:

- replay datasource config into Grafana

Secret behavior:

- accepts external secret injection through:
  - `--secret-values`
  - `--secret-values-file`
- resolves `secureJsonDataPlaceholders` before any write request
- fails closed when required placeholders are missing

Example:

```bash
grafana-util datasource import \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --input-dir ./datasources \
  --secret-values-file ./datasource-secret-values.json
```

Example secret file:

```json
{
  "influx-main-token": "real-token"
}
```

The import layer will map:

- placeholder key `influx-main-token`
- back into Grafana secure field `token`

and send:

```json
{
  "secureJsonData": {
    "token": "real-token"
  }
}
```

### `snapshot export`

Purpose:

- build a combined offline snapshot root for dashboards, datasources, and access

Datasource secret behavior:

- snapshot datasource lane follows normal datasource export behavior
- snapshot contains datasource config and placeholders
- snapshot does not export datasource plaintext secrets

### `snapshot review`

Purpose:

- inspect exported snapshot inventory without talking to Grafana

Secret behavior:

- review can show datasource structure already present in the snapshot
- review does not decrypt or recover datasource secret values

## Supported secret sources

For datasource import, the supported secret sources are now:

- inline JSON via `--secret-values`
- JSON file via `--secret-values-file`

Datasource secret source of truth is external to Grafana export.

Good examples:

- CI secret injection
- environment-specific generated JSON file
- another secret manager that renders a JSON map before calling import

## What is intentionally not supported

Not supported:

- exporting live datasource plaintext secrets from Grafana
- restoring datasource secrets automatically from export metadata
- export-side encrypted datasource secret companion files
- snapshot datasource secret companion metadata

Those behaviors were removed because they suggested Grafana export could act as
a secret backup source when the upstream API does not provide the needed data.

## Operator guidance

Use this model:

1. Export datasource config from Grafana.
2. Manage secret values outside the export artifact.
3. Import config back with `--secret-values` or `--secret-values-file`.

Do not use this model:

1. Expect Grafana datasource export to recover secret plaintext.
2. Treat `datasources.json` as a secret backup.

## Related docs

- `docs/commands/en/datasource-export.md`
- `docs/commands/en/datasource-import.md`
- `docs/commands/en/snapshot.md`
- `docs/internal/datasource-masked-recovery-contract.md`
