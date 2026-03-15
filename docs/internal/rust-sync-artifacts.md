# Rust Sync Artifacts

This note is the maintainer reference for the canonical Rust `sync` staged
artifact shapes and the fixture files that demos/tests should reuse.

## Artifact kinds

- `grafana-utils-sync-summary`
  - Purpose: normalized desired-state inventory for the constrained managed
    slice.
  - Producer: `crate::sync_contracts::build_sync_summary_document(...)`
- `grafana-utils-sync-plan`
  - Purpose: review-required desired-vs-live diff with `create/update/delete/noop/unmanaged`
    operations, alert assessment, and explicit `scope`.
  - Producer: `crate::sync_contracts::build_sync_plan_document(...)`
- `grafana-utils-sync-preflight`
  - Purpose: staged dependency/policy checks for datasource, dashboard, folder,
    and alert sync before apply.
  - Producer: `crate::sync_preflight::build_sync_preflight_document(...)`
- `grafana-utils-sync-bundle-preflight`
  - Purpose: aggregate sync preflight plus provider-related blocking checks for
    one multi-resource bundle.
  - Producer: `crate::sync_bundle_preflight::build_sync_bundle_preflight_document(...)`
- `grafana-utils-sync-apply-intent`
  - Purpose: reviewed/approved apply surface that filters to executable
    operations while preserving staged metadata such as `scope`, lineage, and
    optional preflight summaries.
  - Producer: `crate::sync_contracts::build_sync_apply_intent_document(...)`

## Canonical demo fixtures

These fixture files are the preferred demo inputs for Rust `sync` docs and
future fixture-driven tests:

- `tests/fixtures/rust_sync_demo_desired.json`
- `tests/fixtures/rust_sync_demo_live.json`
- `tests/fixtures/rust_sync_demo_availability.json`
- `tests/fixtures/rust_sync_demo_bundle.json`
- `tests/fixtures/rust_sync_demo_target_inventory.json`

If a user-facing README or guide needs a Rust `sync` example, point to these
fixture paths instead of embedding long inline JSON blobs again.

## Canonical test fixtures

Fixture-driven Rust tests should prefer small shared JSON fixtures under
`tests/fixtures/` rather than rebuilding large staged documents inline.

Current canonical contract fixture targets:

- `tests/fixtures/rust_sync_contract_cases.json`
  - Intended for `sync summary`, `sync plan`, and `sync apply-intent` contract
    assertions.
- `tests/fixtures/rust_sync_preflight_cases.json`
  - Intended for `sync preflight` and `sync bundle-preflight` summary/rendering
    assertions.

When adding new staged fields, update the canonical fixture file first, then
update the Rust tests that load it with `include_str!(...)`.

## Fixture usage rules

- Keep fixture JSON focused on contract-significant fields. Avoid stuffing
  unrelated Grafana payload noise into the canonical files.
- Prefer asserting artifact `kind`, `schemaVersion`, summary keys, `scope`,
  lineage, and blocking semantics over exact full-document equality.
- Keep `sync` docs, fixture files, and Rust tests aligned. If a demo command or
  maintainer note names a fixture file, at least one Rust test should consume
  that contract family from `tests/fixtures/`.
- Treat `scope`, prune behavior, and alert managed-field ownership as part of
  the stable staged contract. Do not hide those semantics only in text
  renderers.
