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
  `requiredPaths` / `pathTypes` for small nested shape guarantees in golden fixtures.
