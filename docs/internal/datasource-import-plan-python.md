# Python Datasource Import Design Note

## Goal

Add a first-class Python `grafana-utils datasource import` workflow that can round-trip the existing datasource export format back into Grafana with a dry-run safety layer and operator-facing semantics that feel familiar to `dashboard import`, while still respecting datasource-specific constraints.

This note is Python-only. It does not define Rust implementation details beyond parity expectations.

## Current Baseline

- `grafana_utils/datasource_cli.py` now supports:
  - `datasource list`
  - `datasource export`
  - `datasource import`
- Export writes:
  - `datasources.json`
  - `index.json`
  - `export-metadata.json`
- Export records already include:
  - `uid`
  - `name`
  - `type`
  - `access`
  - `url`
  - `isDefault`
  - `org`
  - `orgId`

Datasource import is implemented. This note is retained as historical design context for the Python implementation and any later datasource diff follow-up.

## Design Principles

- Reuse dashboard import operator patterns where that genuinely helps:
  - `--import-dir`
  - `--replace-existing`
  - `--update-existing-only`
  - `--dry-run`
  - `--table`
  - `--json`
  - `--no-header`
  - `--progress`
  - `--verbose`
- Do not force dashboard-specific concepts onto datasources:
  - no folder options
  - no folder-path guard
  - no dashboard-style payload wrapper
- Fail early for unsafe or ambiguous cases.
- Keep the exported datasource inventory format as the canonical import input.

## Implemented CLI Shape

The implemented Python subcommand is:

```bash
grafana-utils datasource import \
  --url http://localhost:3000 \
  --import-dir ./datasources \
  --replace-existing
```

Proposed flags:

- `--import-dir`
  - required
  - points at the datasource export root that contains `datasources.json` and `export-metadata.json`
- `--org-id`
  - optional
  - mirrors dashboard import org override behavior
  - requires Basic auth
- `--require-matching-export-org`
  - optional safety guard
  - compares exported datasource `orgId` against resolved target import org
- `--replace-existing`
  - update existing datasources when the incoming datasource already matches an existing destination record
- `--update-existing-only`
  - skip creates; only update existing destination datasources
- `--dry-run`
  - predict create/update/skip/block behavior without calling write APIs
- `--table`
  - only valid with `--dry-run`
- `--json`
  - only valid with `--dry-run`
- `--no-header`
  - only valid with `--dry-run --table`
- `--progress`
  - concise per-datasource progress output
- `--verbose`
  - detailed per-datasource output

Not proposed for first version:

- `--import-message`
  - datasource APIs do not have dashboard revision history semantics
- `--ensure-*`
  - no datasource equivalent to folder inventory recreation
- `--all-orgs`
  - not appropriate for import v1

## Import Input Contract

Import should accept the existing datasource export root, not arbitrary raw files.

Expected files:

- `datasources.json`
- `index.json`
- `export-metadata.json`

Validation rules:

- import directory must exist and be a directory
- `export-metadata.json` must match:
  - `kind = grafana-utils-datasource-export-index`
  - expected schema version
  - expected resource / variant contract
- `datasources.json` must be a JSON array of datasource inventory records
- reject import when the export metadata does not describe datasource inventory

## Identity And Matching Strategy

Datasource import cannot simply reuse dashboard `uid` logic without adjustment.

Recommended matching order for destination lookup:

1. exported `uid`, when non-empty
2. fallback to exact `name` if `uid` is absent

Rationale:

- modern Grafana datasources commonly have stable `uid`
- exported inventory already preserves `uid`
- name fallback is useful for older or imperfect source data
- matching by type or URL alone is too weak and too error-prone

For dry-run and live import, define destination state as:

- `missing`
- `exists-uid`
- `exists-name`
- `ambiguous`

If multiple datasources match the same fallback identity:

- dry-run should mark the datasource as blocked/ambiguous
- live import should fail or skip with a clear error

## Import Mode Semantics

Align with dashboard import where that makes sense.

### Default mode: create-only

- missing destination datasource -> create
- existing destination datasource -> blocked-existing

### `--replace-existing`

- missing destination datasource -> create
- existing destination datasource -> update

### `--update-existing-only`

- missing destination datasource -> skip-missing
- existing destination datasource -> update

`--update-existing-only` should imply overwrite-on-existing behavior, just as dashboard import does.

## Dry-Run Behavior

Dry-run should not write anything to Grafana.

For each datasource, determine:

- resolved identity:
  - `uid`
  - `name`
- destination state
- predicted action
- target org id

Suggested actions:

- `would-create`
- `would-update`
- `would-skip-missing`
- `would-fail-existing`
- `would-fail-ambiguous`

### Dry-run default text output

One line per datasource, similar to dashboard import:

```text
Dry-run datasource uid=prom-main name=Prometheus Main dest=exists action=update file=...
```

### Dry-run table output

Suggested columns:

- `UID`
- `NAME`
- `TYPE`
- `DESTINATION`
- `ACTION`
- `ORG_ID`
- `FILE`

### Dry-run JSON output

Suggested top-level shape:

```json
{
  "mode": "create-or-update",
  "datasources": [
    {
      "uid": "prom-main",
      "name": "Prometheus Main",
      "type": "prometheus",
      "destination": "exists-uid",
      "action": "would-update",
      "orgId": "2",
      "file": "./datasources/datasources.json#0"
    }
  ],
  "summary": {
    "datasourceCount": 1,
    "createCount": 0,
    "updateCount": 1,
    "skipCount": 0,
    "blockedCount": 0
  }
}
```

`file` can be synthetic because import reads one array file rather than one file per datasource. A stable reference such as `datasources.json#<index>` is enough.

## Live Import Behavior

Live import should use Grafana datasource create/update APIs and print a final summary.

Suggested flow:

1. build effective client
2. apply optional `--org-id` client override
3. validate optional `--require-matching-export-org`
4. load export metadata and datasource records
5. resolve destination match for each datasource
6. either:
   - create
   - update
   - skip
   - fail on ambiguous/unsupported records

Summary examples:

- `Imported 5 datasource(s) from ./datasources`
- `Imported 5 datasource(s) from ./datasources; skipped 2 missing destination datasources`

## Org And Auth Rules

Use the same policy family already established for dashboard import.

### Current-org token behavior

- token auth is valid for datasource import into the token's current org
- without `--org-id`, import targets the current org context

### Explicit org switching

- `--org-id` should require Basic auth
- it should scope the entire import run to one destination org

### Export-org safety guard

- `--require-matching-export-org` should compare exported `orgId` values from `datasources.json` and/or `index.json` against the resolved target org
- if the export does not provide one stable source org id, fail closed
- if source org id and target org id differ, fail with a clear error

This mirrors the dashboard import safety guard already added for cross-org replay mistakes.

## Datasource-Specific Mapping / Rewrite Concerns

Datasource import has different mapping needs than dashboard import.

### First version: no automatic rewrite flags

Do not start with:

- `--uid-map`
- `--name-map`
- `--type-map`
- `--url-map`

Reason:

- datasource identity is the resource being imported, not just a reference target
- automatic rewrite semantics become dangerous quickly
- v1 should prioritize deterministic round-trip and predictable conflict handling

### Future extension candidates

Possible later flags:

- `--rewrite-url`
  - controlled environment migration use case
- `--rewrite-basic-auth`
  - if secure-json handling is later supported
- `--name-map` or `--uid-map`
  - only if there is a strong operator case

These should be deferred until the base import contract is stable.

## Secure Fields And Credential Handling

This is the largest datasource-specific design issue.

Exported datasource inventory currently contains non-secret fields and metadata. It does not appear to preserve secure credential material such as:

- secure JSON data
- tokens
- passwords
- TLS keys / cert bodies

That means datasource import v1 should not promise full secret round-trip.

Recommended v1 behavior:

- import only the exported visible fields
- if the datasource type normally depends on secret fields, allow create/update with missing secrets only when Grafana API accepts it
- warn or fail for datasource types where missing secure configuration would create a broken datasource

The safer policy is:

- start with best-effort visible-field import
- document clearly that secure fields are not restored
- add explicit validation/warning hooks for datasource types known to rely heavily on secrets

Future options:

- separate secure input document
- prompt/env-driven secret injection
- type-specific secret merge support

## Validation Rules

Suggested v1 validation:

- `--table` requires `--dry-run`
- `--json` requires `--dry-run`
- `--table` and `--json` are mutually exclusive
- `--no-header` requires `--dry-run --table`
- `--org-id` requires Basic auth
- `--require-matching-export-org` fails if:
  - no source export org id is available
  - multiple source org ids are present
  - source org id != target org id
- each datasource record must be a JSON object
- reject records with neither usable `uid` nor usable `name`
- reject or block ambiguous destination matches

## Proposed Python Module Shape

Keep `grafana_utils/datasource_cli.py` as the stable CLI facade, similar to the dashboard split direction.

Suggested helper split:

- `grafana_utils/datasources/import_workflow.py`
  - orchestration for dry-run and live import
- `grafana_utils/datasources/import_support.py`
  - export-metadata validation
  - datasource matching logic
  - dry-run record rendering
  - payload builders
- optionally `grafana_utils/datasources/common.py`
  - shared constants if the datasource surface grows

If the feature stays small, v1 can remain in `datasource_cli.py`, but the dashboard codebase suggests import logic will become easier to maintain if extracted early.

## Suggested API Expectations

Likely Grafana API families to use:

- list existing datasources
- create datasource
- update datasource by id or uid, depending on available API path

The design should normalize around whatever Grafana API is stable across supported versions, but the CLI contract should remain based on exported `uid` and `name`, not API-internal numeric ids.

## Suggested Python Test Matrix

Parser / help:

- parse supports `datasource import`
- parse supports `--replace-existing`
- parse supports `--update-existing-only`
- parse supports `--org-id`
- parse supports `--require-matching-export-org`
- import help includes dry-run/table/json restrictions

Input validation:

- reject missing import dir
- reject invalid export metadata kind/schema
- reject `--table` without `--dry-run`
- reject `--json` without `--dry-run`
- reject `--no-header` without `--dry-run --table`
- reject `--org-id` with token auth
- reject `--require-matching-export-org` when source org metadata missing
- reject `--require-matching-export-org` when source org metadata is mixed

Dry-run behavior:

- dry-run create for missing datasource
- dry-run blocked-existing in default mode
- dry-run update with `--replace-existing`
- dry-run skip-missing with `--update-existing-only`
- dry-run fail-ambiguous when fallback matching finds multiple candidates
- dry-run JSON output shape
- dry-run table output columns and `--no-header`

Org behavior:

- token-scoped dry-run uses current org
- explicit `--org-id` uses scoped client
- `--require-matching-export-org` blocks mismatch for token current org
- `--require-matching-export-org` blocks mismatch for explicit `--org-id`
- matching export org allows run to continue

Live import behavior:

- create path posts expected payload
- update path calls expected update API
- update-existing-only skips missing datasources
- replace-existing preserves visible exported identity fields
- ambiguous live match is rejected before write

Credential / secure-field contract:

- visible exported fields survive payload build
- secure fields are not silently invented
- warning or failure behavior for secret-dependent datasource types is covered once the exact policy is chosen

## Recommended Phasing

### Phase 1

- add `datasource import`
- support create-only, replace-existing, update-existing-only
- support dry-run text/table/json
- support `--org-id`
- support `--require-matching-export-org`
- no rewrite/mapping flags
- no secure field restoration

### Phase 2

- add datasource diff
- improve type-aware validation
- add explicit warnings around secret-dependent datasource types

### Phase 3

- evaluate controlled mapping/rewrite options
- evaluate secret injection / merge model

## Recommendation

Implement datasource import v1 as a conservative current-org or explicit-org importer for the existing exported inventory format, with dry-run and explicit export-org safety checks, but without automatic rewrite logic or secret restoration promises.

That gives operators a predictable backup-and-replay path with guardrails, while leaving the more dangerous datasource-specific migration features for later, explicit follow-up work.
