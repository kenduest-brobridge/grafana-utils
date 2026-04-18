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
- JSON output contract registry:
  [`scripts/contracts/output-contracts.json`](scripts/contracts/output-contracts.json)
- Schema/help manifest source:
  [`schemas/manifests/`](schemas/manifests/)
  - family-owned `contracts.json` and `routes.json`
  - generated schema artifacts under `schemas/jsonschema/`
  - generated schema-help artifacts under `schemas/help/`

## Ownership Rules

- Output contracts own runtime golden regression gates.
  - Treat `scripts/contracts/output-contracts.json` as the contract registry for
    machine-readable JSON output.
  - Use it to define which fields, nested paths, array shapes, and enum values
    must stay stable in golden regression fixtures.
  - When an output shape changes, update the contract registry and the matching
    runtime golden fixtures together so the checker is verifying the live
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
  - Promote an artifact only when its command surface, schema/help manifest, and
    runtime output contract all agree on the same shape.
  - A stable public artifact should have golden coverage for its runtime output,
    manifest coverage for its published schema/help contract, and docs routing
    coverage through `command-surface.json` when the command path is public.
  - If the artifact is still under active shape churn, keep it in the runtime
    golden / manifest layer and do not describe it as a stable public contract
    yet.

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
- Keep `scripts/contracts/output-contracts.json` current when adding or changing
  machine-readable JSON outputs; use root `requiredFields` for envelope checks and
  `requiredPaths` / `pathTypes` / `arrayItemTypes` / `minimumItems` / `enumValues`
  for small nested and collection shape guarantees in golden fixtures.
