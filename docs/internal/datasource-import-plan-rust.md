# Rust Datasource Import Plan

## Goal

Add a first-class Rust `grafana-utils datasource import` workflow that can round-trip the existing datasource export format back into Grafana with an operator experience close to `dashboard import`, while keeping datasource-specific semantics explicit where dashboards and datasources differ.

## Current Baseline

- Rust now has a standalone datasource namespace with datasource import support.
- Python now has `grafana_utils/datasource_cli.py` support for datasource `list`, `export`, and `import`.
- Existing datasource export writes:
  - `datasources.json`
  - `index.json`
  - `export-metadata.json`
- Exported datasource records already carry `uid`, `name`, `type`, `access`, `url`, `isDefault`, `org`, and `orgId`.

This note is retained as historical design context for the Rust implementation and for any later datasource import follow-up work.

## Implemented Rust CLI Shape

Rust now ships a dedicated datasource utility rather than a dashboard subcommand overload.

Preferred user-facing commands:

- `grafana-utils datasource list ...`
- `grafana-utils datasource export ...`
- `grafana-utils datasource import ...`

Rust already has a dedicated datasource namespace, so follow-on work should continue there rather than reusing `dashboard list-data-sources`.

### Proposed `datasource import` flags

Common auth/transport flags:

- `--url`
- `--token`
- `--basic-user`
- `--basic-password`
- `--prompt-password`
- `--timeout`
- `--verify-ssl`

Import input and execution flags:

- `--import-dir <DIR>`
  - Points at the datasource export root that contains `datasources.json` and `export-metadata.json`.
- `--org-id <ID>`
  - Explicit target org override for the whole run.
  - Keep the same Basic-auth-only rule already used by dashboard `--org-id`.
- `--replace-existing`
  - Update existing datasources that match the identity key.
- `--update-existing-only`
  - Only update existing datasources; skip missing ones.
- `--require-matching-export-org`
  - Reuse the new dashboard safety pattern.
  - Fail if the exported datasource inventory `orgId` does not match the resolved target org.
- `--name-map <FILE>`
  - Optional JSON map from source datasource name to target datasource name.
  - Needed because datasource identity often drifts by name across environments even when dashboards do not.
- `--uid-map <FILE>`
  - Optional JSON map from source datasource UID to target datasource UID.
  - More deterministic than name-only matching when operators already know the target UIDs.
- `--type-map <FILE>`
  - Optional narrow mapping for plugin-type rename scenarios.
  - Example: legacy plugin type id to replacement plugin type id.
- `--allow-plugin-type-change`
  - Explicit opt-in when an import would rewrite `type`.
- `--import-message`
  - Not recommended for first version unless Grafana datasource API has an equivalent change note field. Likely omit initially.

Dry-run and output flags:

- `--dry-run`
- `--table`
- `--json`
- `--no-header`
- `--progress`
- `--verbose`

## Import Input Format

First version should accept only the existing datasource export root, not arbitrary hand-written JSON.

Expected files:

- `datasources.json`
- `index.json`
- `export-metadata.json`

Validation:

- `export-metadata.json::kind` must match the datasource export kind.
- `schemaVersion` must match the currently supported datasource export schema.
- `datasources.json` must be a JSON array of datasource records.
- Reject directories that look like dashboard export roots.

## Identity And Match Semantics

Datasource import cannot copy dashboard import identity rules directly because Grafana datasource lifecycle is more sensitive to name/UID collisions and secret fields.

Recommended matching precedence:

1. `--uid-map` hit
2. Exported `uid` match in target
3. `--name-map` hit
4. Exported `name` match in target
5. If `--update-existing-only`, skip
6. Otherwise create

Rationale:

- UID is the most stable deterministic match when preserved.
- Name is often what actually survives between environments.
- Mappings should override raw export identity so migration intent is explicit.

## Mode Semantics

Align with dashboard import where it still makes sense.

### Default mode: create-only

- No update of existing target datasources.
- Existing match becomes `blocked-existing`.

### `--replace-existing`

- Create missing datasources.
- Update matched datasources.

### `--update-existing-only`

- Update only matched datasources.
- Missing datasources become `skip-missing`.
- Implies overwrite behavior for matched entries.

### Proposed dry-run actions

- `would-create`
- `would-update`
- `would-fail-existing`
- `would-skip-missing`
- `would-fail-org-mismatch`
- `would-fail-plugin-type-change`
- `would-fail-secret-loss`

Datasource import should prefer fail/skip wording over silent fallthrough because datasource drift is higher risk than dashboard drift.

## Dry-Run Behavior

`--dry-run` should do all validation and target matching without calling write APIs.

Plain dry-run line output:

- `Dry-run import datasource uid=prom-main name="Prometheus Main" dest=exists action=update file=...`

`--dry-run --table` columns:

- `UID`
- `NAME`
- `TYPE`
- `DESTINATION`
- `ACTION`
- `TARGET_UID`
- `TARGET_NAME`
- `ORG_ID`
- `FILE`

`--dry-run --json` document:

- `mode`
- `sourceOrgId`
- `targetOrgId`
- `datasources`
- `summary`

Summary counts:

- `datasourceCount`
- `wouldCreate`
- `wouldUpdate`
- `wouldFailExisting`
- `wouldSkipMissing`
- `orgMismatchCount`
- `pluginTypeChangeCount`

## Org And Auth Rules

Keep datasource import org/auth behavior consistent with dashboard import to reduce operator surprise.

- Plain token auth:
  - Valid for current-org import.
- `--org-id`:
  - Basic-auth-only.
- `--require-matching-export-org`:
  - Compare exported datasource `orgId` against the resolved target org.
  - Resolved target org is:
    - explicit `--org-id`, else
    - `GET /api/org`

Do not auto-route import by exported `orgId` in the first version.

That should remain a later explicit mode such as:

- `--use-export-org`
- or `--import-by-export-org`

## Datasource-Specific Rewrite And Mapping Concerns

Datasource import needs explicit handling for fields that do not round-trip safely.

### Secrets

Do not attempt to round-trip secure secret values from export unless the export format explicitly carries them, which it currently does not.

First-version rule:

- Treat secure fields as non-exportable and non-round-trippable.
- If an update would overwrite a datasource that likely depends on secrets, require explicit operator acknowledgement or refuse unsafe destructive replacement.

Recommended first-version policy:

- Allow create/update only for datasource fields that exist in exported inventory.
- Do not send empty secret payloads that could clear existing credentials.
- If the API contract would clear secrets implicitly, fail with `secret-loss-risk`.

### Plugin type drift

Changing datasource `type` is materially risky.

First-version rule:

- If matched target datasource exists and `type` differs:
  - fail unless `--allow-plugin-type-change` is set

### Name and UID drift

Support explicit maps:

- `--uid-map`
- `--name-map`

JSON shape:

```json
{
  "prom-main": "prom-prod"
}
```

### Access/url drift

These should be normal update fields and visible in dry-run diff/action reasoning, but not require special flags.

## Proposed Rust Internal Structure

If a standalone Rust datasource CLI is added, prefer the same module split style already used elsewhere.

Suggested files:

- `rust/src/datasource.rs`
  - top-level datasource dispatch
- `rust/src/datasource_cli_defs.rs`
  - clap args and auth/client helpers
- `rust/src/datasource_export.rs`
  - export helpers if Rust gains first-class datasource export parity
- `rust/src/datasource_import.rs`
  - import workflow, dry-run rendering, matching, validation
- `rust/src/datasource_files.rs`
  - export file load/validate helpers
- `rust/src/datasource_rust_tests.rs`
  - focused unit tests

If the repo intentionally keeps datasource support Python-only for now, still isolate Rust planning this way rather than mixing datasource import into dashboard modules.

## Grafana API Considerations

Before implementation, confirm the exact datasource write APIs and their replacement semantics:

- create endpoint
- update endpoint
- whether UID can be preserved on create
- whether secure fields are omitted vs cleared
- whether plugin `type` can be changed safely

This API review is the main blocker for a safe first implementation.

## First-Version Recommendation

Keep V1 narrow and safe.

Ship:

- `datasource import`
- `--import-dir`
- `--replace-existing`
- `--update-existing-only`
- `--org-id`
- `--require-matching-export-org`
- `--dry-run`
- `--table`
- `--json`
- `--no-header`
- `--progress`
- `--verbose`
- `--uid-map`
- `--name-map`

Defer:

- `--type-map`
- `--allow-plugin-type-change`
- multi-org replay by exported org
- secret re-entry workflows
- datasource diff command if import is not yet safe enough

## Suggested Rust Test Matrix

Parser/help:

- help shows `datasource import`
- parser accepts `--replace-existing`
- parser accepts `--update-existing-only`
- parser accepts `--org-id`
- parser accepts `--require-matching-export-org`
- parser accepts `--uid-map` and `--name-map`

Validation:

- reject missing `datasources.json`
- reject bad export metadata kind/schema
- reject token auth with `--org-id`
- reject export-org mismatch when guard is enabled
- reject multi-org or ambiguous export org metadata when guard is enabled

Matching:

- create missing datasource in default mode
- block existing datasource in default mode
- update existing datasource with `--replace-existing`
- skip missing datasource with `--update-existing-only`
- prefer UID match over name match
- apply `--uid-map` before raw UID match
- apply `--name-map` before raw name match

Dry-run rendering:

- plain dry-run line output
- `--table` columns
- `--json` shape
- `--no-header` behavior

Risk handling:

- reject plugin type change without explicit override
- preserve existing secret-bearing datasources safely or fail with explicit message

Live request behavior:

- dry-run must not call write API
- live import calls create endpoint for create path
- live import calls update endpoint for update path
- `--require-matching-export-org` fails before write requests

## Open Questions

1. Should Rust datasource import exist only after Rust gains a first-class `grafana-utils datasource ...` namespace, or is it acceptable to land datasource import in Python first and defer Rust namespace parity?
2. What exact Grafana datasource API fields are safe to round-trip from the current export format?
3. Do we want V1 to reject updates for datasource types known to require secrets unless the operator explicitly opts in?
4. Should `datasource diff` be designed together with import so dry-run reasoning and diff reasoning share one normalization path?
