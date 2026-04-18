# Contract Documentation Map

Current guide for where contract information belongs.

Use three layers:

- Summary:
  short maintainer-facing policy in `docs/DEVELOPER.md`
- Spec:
  detailed current requirements in dedicated `docs/internal/*` contract docs
- Trace:
  concise status/change history in `docs/internal/ai-status.md` and
  `docs/internal/ai-changes.md`

## Current Contract Specs

- Repo-level export-root policy:
  [`export-root-output-layering-policy.md`](docs/internal/export-root-output-layering-policy.md)
- Dashboard export-root contract:
  [`dashboard-export-root-contract.md`](docs/internal/dashboard-export-root-contract.md)
- Datasource masked-recovery contract:
  [`datasource-masked-recovery-contract.md`](docs/internal/datasource-masked-recovery-contract.md)
- Alert/access boundary policy:
  [`alert-access-contract-policy.md`](docs/internal/alert-access-contract-policy.md)
- CLI/docs surface contract:
  [`scripts/contracts/command-surface.json`](scripts/contracts/command-surface.json)
- Docs-entrypoint contract:
  [`scripts/contracts/docs-entrypoints.json`](scripts/contracts/docs-entrypoints.json)
- JSON output contract registry:
  [`scripts/contracts/output-contracts.json`](scripts/contracts/output-contracts.json)
- Schema/help manifest source:
  [`schemas/manifests/`](schemas/manifests/)
  - family-owned `contracts.json` and `routes.json`
  - generated schema artifacts under `schemas/jsonschema/`
  - generated schema-help artifacts under `schemas/help/`
- Generated docs navigation projections:
  [`scripts/contracts/command-reference-index.json`](scripts/contracts/command-reference-index.json)
  and [`scripts/contracts/handbook-nav.json`](scripts/contracts/handbook-nav.json)

## Ownership Rules

- Command-surface contracts own public CLI routing.
  - Treat `scripts/contracts/command-surface.json` as the source of truth for
    public command paths, legacy replacements, removed public paths, docs
    routing, and `--help-full` / `--help-flat` support.
  - Keep `scripts/contracts/command-surface.json` current when public paths or
    docs-routing behavior change.

- Docs-entrypoint contracts own navigation shortcuts.
  - Treat `scripts/contracts/docs-entrypoints.json` as the source of truth for
    landing quick commands, jump-select entries, and handbook sidebar command
    shortcuts.
  - Treat `scripts/contracts/command-reference-index.json` and
    `scripts/contracts/handbook-nav.json` as generated navigation projections,
    not as primary authoring surfaces.

- Output contracts own runtime golden regression gates.
  - Treat `scripts/contracts/output-contracts.json` as the registry for
    machine-readable runtime JSON output contracts.
  - Use it to define which fields, nested paths, array shapes, enum values, and
    forbidden fields must stay stable in golden regression fixtures.
  - When a runtime output shape changes, update the contract registry and the
    matching runtime golden fixtures together so the checker is verifying live
    behavior, not a stale expectation.

- Schema manifests own published schema/help contracts.
  - Treat `schemas/manifests/**/contracts.json` and `schemas/manifests/**/routes.json`
    as the source of truth for published schema/help surfaces.
  - The generated `schemas/jsonschema/` and `schemas/help/` trees are published
    artifacts derived from those manifests, not the place to author policy.
  - Any new `--help-schema` or schema-oriented command surface should be
    represented in the manifest layer first, then projected into generated
    schema/help output.

- Stable public artifacts need a promotion gate.
  - Promote an artifact only when its command surface, docs-entrypoint
    navigation, schema/help manifest, and runtime output contract all agree on
    the same shape.
  - A stable public artifact should have golden coverage for its runtime output,
    manifest coverage for its published schema/help contract, and docs routing
    coverage through `command-surface.json` and `docs-entrypoints.json` when the
    command path is public.
  - If the artifact is still under active shape churn, keep it in the runtime
    golden / manifest layer and do not describe it as a stable public contract
    yet.

## Current Contract Lane Overlap

The output-contract registry and schema manifests intentionally do not have a
one-to-one contract-id mapping yet.

- Runtime output contracts currently cover deep golden regression checks for
  selected machine outputs, including dashboard summary/governance, dashboard
  topology/impact/policy, datasource export index, and sync plan/preflight/source
  bundle artifacts.
- Schema manifests currently publish schema/help contracts for status, change
  workflow, dashboard history, and diff families.
- Overlap should be promoted by artifact family, not by renaming IDs. For
  example, `grafana-utils-sync-plan` in the runtime output registry and
  `sync-plan` in the change schema manifest both describe the workspace preview
  family, but each layer owns a different validation purpose.

Promotion rule:

- Add or update runtime output-contract coverage when a JSON command output has
  behavior-sensitive fields that need golden regression checks.
- Add or update schema manifest coverage when the same output is documented as a
  published `--help-schema` or schema/help surface.
- Treat an output as stable public surface only after both lanes have coverage
  or there is an explicit reason that one lane does not apply.

## Maintainer Rules

- Keep `docs/DEVELOPER.md` short enough to orient maintainers quickly.
- Put stable field lists, promotion gates, compatibility rules, and detailed
  contract requirements in the dedicated spec docs.
- Keep `ai-status.md` and `ai-changes.md` current and trace-oriented; do not
  restate full specs there.
- Archive older trace entries once they stop helping with current navigation.
- Keep `scripts/contracts/command-surface.json` current when public command paths, legacy
  replacements, docs routing, removed public path guards, or `--help-full` /
  `--help-flat` support change.
- Keep `scripts/contracts/docs-entrypoints.json` current when landing quick
  commands, jump-select entries, or handbook sidebar shortcuts change.
- Treat `scripts/contracts/command-reference-index.json` and
  `scripts/contracts/handbook-nav.json` as generated projections of the docs
  entrypoint and handbook routing contracts.
- Keep `scripts/contracts/output-contracts.json` current when adding or changing
  machine-readable JSON outputs; use root `requiredFields` for envelope checks and
  `requiredPaths` / `pathTypes` / `arrayItemTypes` / `minimumItems` / `enumValues`
  for small nested and collection shape guarantees in golden fixtures.
