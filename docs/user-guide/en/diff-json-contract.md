# Shared Diff JSON Contract

Use this guide when you need to parse or automate around `dashboard diff`, `alert diff`, or `datasource diff` JSON output.

## Contract Shape

All three diff commands emit the same top-level envelope in JSON mode:

- `kind`
- `schemaVersion`
- `toolVersion`
- `summary`
- `rows`

Treat the JSON the same way you treat other machine-readable Grafana-util contracts:

1. check `kind`
2. confirm `schemaVersion`
3. only then inspect the rows

## Versioning Rules

This shared contract uses a family-wide major version in `schemaVersion`.

- The current version is `1`.
- Bump `schemaVersion` only for breaking changes that affect all diff consumers, such as removing or renaming a top-level key, changing a required field, or changing the meaning of an existing field.
- Additive, backward-compatible fields should keep the same `schemaVersion`, but they still need documentation and contract tests.
- Update `dashboard diff`, `alert diff`, and `datasource diff` together when the shared envelope changes.

CLI schema lookups:

- `grafana-util dashboard diff --help-schema`
- `grafana-util alert diff --help-schema`
- `grafana-util datasource diff --help-schema`

## Summary Fields

The shared summary uses these counters:

- `checked`
- `same`
- `different`
- `missingRemote`
- `extraRemote`
- `ambiguous`

These fields are stable across dashboard, alert, and datasource diff outputs.

## Row Fields By Command

### `dashboard diff`

Dashboard diff rows focus on a file path and a compact diff preview:

- `domain`
- `resourceKind`
- `identity`
- `status`
- `path`
- `changedFields`
- `diffText`
- `contextLines`

### `alert diff`

Alert diff rows keep the review surface simple and resource-oriented:

- `domain`
- `resourceKind`
- `identity`
- `status`
- `path`
- `changedFields`

`alert diff` still accepts `--json` as a compatibility flag, but the canonical form is `--output-format json`.

### `datasource diff`

Datasource diff rows add field-level workspace details for drift review:

- `domain`
- `resourceKind`
- `identity`
- `status`
- `path`
- `changedFields`
- `changes[]`

Each `changes[]` item records:

- `field`
- `before`
- `after`

### `dashboard history diff`

Dashboard history diff keeps the same shared envelope but adds source labels so you can compare live history, one exported artifact, or two different export roots:

- `domain`
- `resourceKind`
- `identity`
- `status`
- `path`
- `baseSource`
- `newSource`
- `baseVersion`
- `newVersion`
- `changedFields`
- `diffText`
- `contextLines`

## Practical Reading Order

For automation, read the payload in this order:

1. verify `kind`
2. verify `schemaVersion`
3. use `summary` for gating
4. inspect `rows` for review details

## Related Pages

- [Dashboard diff command](../../commands/en/dashboard-diff.md)
- [Alert diff command](../../commands/en/alert-diff.md)
- [Datasource diff command](../../commands/en/datasource-diff.md)
- [Dashboard history command](../../commands/en/dashboard-history.md)
