# Datasource Import Plan

## Status

Implemented. `grafana-util datasource import` now exists in both the Python and Rust CLIs.

This note now serves as an implementation-status summary plus a pointer to the original Python and Rust design notes:

- `docs/internal/datasource-import-plan-python.md`
- `docs/internal/datasource-import-plan-rust.md`

## Current State

Implemented behavior includes:

- `grafana-util datasource import`
- import modes:
  - `create-only`
  - `create-or-update`
  - `update-or-skip-missing`
- org targeting with `--org-id`
- export-org safety guard with `--require-matching-export-org`
- dry-run output in text, table, and JSON forms
- dedicated Python and Rust datasource namespaces

The remaining sections are retained as historical design context and for any follow-on datasource import work. Older `grafana-utils ...` references below should be read as historical wording rather than current primary command guidance.

## Goal

Add a first-class `grafana-utils datasource import` workflow with a dry-run safety layer and operator semantics that feel close to `dashboard import`, while staying conservative around datasource-specific risks such as identity drift, plugin type changes, and secret fields.

## Recommended V1 Scope

Implement the same core operator shape in Python and Rust:

- `grafana-utils datasource import`
- `--import-dir`
- `--org-id`
- `--require-matching-export-org`
- `--replace-existing`
- `--update-existing-only`
- `--dry-run`
- `--table`
- `--json`
- `--no-header`
- `--progress`
- `--verbose`

Do not add dashboard-specific flags that do not fit datasource semantics:

- no folder flags
- no `--import-message`
- no folder-path guard

## Input Contract

V1 should import only the existing datasource export root, not arbitrary JSON.

Expected files:

- `datasources.json`
- `index.json`
- `export-metadata.json`

Validation should reject:

- missing export root files
- wrong manifest kind or schema version
- non-array `datasources.json`
- directories that are clearly dashboard export roots

## Matching Rules

Datasource identity should be stricter than dashboard identity.

Recommended destination match order:

1. exact exported `uid`
2. exact exported `name` when `uid` is absent or unusable

When fallback matching is ambiguous:

- dry-run should report a blocked action
- live import should fail or skip with a clear error

Recommended destination states:

- `missing`
- `exists-uid`
- `exists-name`
- `ambiguous`

## Import Modes

Keep mode names aligned with dashboard import where possible.

Default mode:

- missing -> create
- existing -> blocked-existing

`--replace-existing`:

- missing -> create
- existing -> update

`--update-existing-only`:

- missing -> skip-missing
- existing -> update

Recommended dry-run actions:

- `would-create`
- `would-update`
- `would-fail-existing`
- `would-skip-missing`
- `would-fail-ambiguous`
- `would-fail-org-mismatch`
- `would-fail-plugin-type-change`
- `would-fail-secret-loss`

## Org And Auth Rules

Keep datasource import org/auth semantics aligned with dashboard import:

- plain token auth is valid for current-org import
- `--org-id` is Basic-auth-only
- `--require-matching-export-org` compares exported `orgId` with the resolved target org
- target org resolution is:
  - explicit `--org-id`, else
  - current org from `GET /api/org`

Do not auto-route import by exported `orgId` in V1.

## Datasource-Specific Safety Rules

V1 should stay conservative:

- if matched target datasource has a different plugin `type`, fail unless a later version introduces explicit opt-in
- do not clear or overwrite secure secret fields implicitly
- if export format does not carry enough information to update secrets safely, fail closed rather than sending empty values

This is the main reason datasource import cannot be a direct copy of dashboard import.

## Output Contract

Plain dry-run output should stay concise and line-oriented.

Suggested `--dry-run --table` columns:

- `UID`
- `NAME`
- `TYPE`
- `DESTINATION`
- `ACTION`
- `ORG_ID`
- `FILE`

Suggested `--dry-run --json` top-level shape:

- `mode`
- `sourceOrgId`
- `targetOrgId`
- `datasources`
- `summary`

## V2 Candidates

Do not include these in V1 unless a concrete migration need forces them in:

- `--uid-map`
- `--name-map`
- `--type-map`
- `--allow-plugin-type-change`
- automatic per-org routing such as `--use-export-org`
- datasource diff workflow

These are useful, but they widen the policy surface substantially.

## Implementation Order

Recommended order:

1. add Python datasource import
2. add Rust datasource namespace parity if needed
3. keep the CLI contract as close as practical across both runtimes
4. add mapping flags only after the baseline import/dry-run path is stable

## Open Question

Rust now has a first-class standalone datasource CLI alongside Python's `grafana_utils/datasource_cli.py`.

Follow-on Rust datasource work, if any, should build on the existing namespace:

- `grafana-utils datasource list`
- `grafana-utils datasource export`
- `grafana-utils datasource import`

as a dedicated namespace, rather than continuing to rely on dashboard-owned datasource helpers.
