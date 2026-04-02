# Dashboard Export-Root Contract

Detailed requirements for the current dashboard staged export contract.

## Summary

- The dashboard export root is a typed contract boundary.
- `raw/` is the canonical staged export variant for dashboard consumers.
- `provisioning/` is a derived provisioning-oriented variant with its own
  explicit input contract.
- Combined roots are valid only for commands that explicitly consume the
  dashboard export-root contract.

## Stable Root Contract

- Stable root-manifest fields:
  `schemaVersion`, `toolVersion`, `kind`, `variant`, `scopeKind`,
  `dashboardCount`, `indexFile`, `format`, `foldersFile`, `datasourcesFile`,
  `permissionsFile`, `org`, `orgId`, `orgCount`, `orgs`
- Stable root scope kinds:
  `org-root`, `all-orgs-root`, `workspace-root`
- Scope semantics:
  - `org-root`: one dashboard export scope with one export identity
  - `all-orgs-root`: aggregate dashboard export root that still owns dashboard
    export metadata directly
  - `workspace-root`: higher-level staged workspace that contains the dashboard
    export tree and sibling staged domains such as `datasources/`

## Output-Layering Rule

- `dashboard inspect-export` summary output keeps `text`, `table`, and `csv`
  as operator-summary views.
- Summary `json` and `yaml` remain the machine-readable full summary contract.
- Specialized report outputs such as governance and dependency reports stay
  report-specific and are not part of the baseline summary-layer contract.

## Compatibility Rule

- Additive evolution is allowed when older readers can ignore new fields
  without changing the meaning of existing fields.
- `schemaVersion` should bump only for real breaking changes to the dashboard
  root-manifest or staged input semantics.

## Documentation Guidance

- Keep the short summary in `docs/DEVELOPER.md`.
- Keep this file as the current detailed dashboard contract doc.
- Trace files should record changes to this contract, not restate the whole
  contract.
