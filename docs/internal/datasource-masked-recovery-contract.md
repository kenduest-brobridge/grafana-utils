# Datasource Masked-Recovery Contract

Detailed requirements for the current datasource masked-recovery export/import
contract.

## Summary

- `datasources.json` is the canonical replay, import, and diff artifact for
  the masked-recovery datasource contract.
- `provisioning/datasources.yaml` is a derived provisioning projection only.
- The contract is recovery-capable but does not export plaintext secret
  material from Grafana live export.

## Stable Root Contract

- Stable root-manifest fields:
  `schemaVersion`, `kind`, `variant`, `scopeKind`, `resource`,
  `datasourceCount`, `datasourcesFile`, `indexFile`, `format`, `exportMode`,
  `masked`, `recoveryCapable`, `secretMaterial`,
  `secretPlaceholderProvider`, `provisioningProjection`,
  `provisioningFile`, `toolVersion`
- Stable datasource-record fields:
  `uid`, `name`, `type`, `access`, `url`, `isDefault`, `org`, `orgId`,
  `secureJsonDataPlaceholders`

## Compatibility Data

- Recovery-only enrichment fields such as `basicAuth`, `basicAuthUser`,
  `database`, `jsonData`, `user`, and `withCredentials` are allowed on input.
- Those fields are compatibility data, not the core replay contract.

## Secret Resolution Rule

- Export writes `secureJsonDataPlaceholders`, not plaintext datasource secrets.
- Import resolves those placeholders only from external secret input such as
  `--secret-values` or `--secret-values-file`.
- Export metadata does not carry datasource secret companion discovery fields.

## Compatibility Rule

- Additive evolution is allowed when older readers can ignore new fields and
  stable field names, meanings, and value types stay unchanged.
- `schemaVersion` bumps only for a breaking contract change, such as removing
  or renaming a stable field, changing a stable field type, changing replay
  semantics, or changing the canonical file layout in a way older readers
  cannot safely interpret.
- Import and diff paths must reject unsupported future versions rather than
  guessing at newer layouts.
- Writers should prefer additive metadata until a real compatibility break
  requires a version bump.

## Documentation Guidance

- Keep the short summary in `docs/DEVELOPER.md`.
- Keep this file as the current detailed datasource contract doc.
- Trace files should record contract changes and validation state, not restate
  the full field list.
