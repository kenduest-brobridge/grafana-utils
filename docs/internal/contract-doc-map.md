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
  [`export-root-output-layering-policy.md`](/Users/kendlee/work/grafana-utils/docs/internal/export-root-output-layering-policy.md)
- Dashboard export-root contract:
  [`dashboard-export-root-contract.md`](/Users/kendlee/work/grafana-utils/docs/internal/dashboard-export-root-contract.md)
- Datasource masked-recovery contract:
  [`datasource-masked-recovery-contract.md`](/Users/kendlee/work/grafana-utils/docs/internal/datasource-masked-recovery-contract.md)
- Alert/access boundary policy:
  [`alert-access-contract-policy.md`](/Users/kendlee/work/grafana-utils/docs/internal/alert-access-contract-policy.md)

## Maintainer Rules

- Keep `docs/DEVELOPER.md` short enough to orient maintainers quickly.
- Put stable field lists, promotion gates, compatibility rules, and detailed
  contract requirements in the dedicated spec docs.
- Keep `ai-status.md` and `ai-changes.md` current and trace-oriented; do not
  restate full specs there.
- Archive older trace entries once they stop helping with current navigation.
