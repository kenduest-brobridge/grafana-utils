# Developer Notes

This page is a maintainer map, not a full spec. Keep it short, orientation-first, and point detailed contract or workflow rules to the dedicated internal docs.

## Documentation Layers

- Summary: `docs/DEVELOPER.md`
- Spec: `docs/internal/contract-doc-map.md` and the linked contract docs it lists
- Trace: `docs/internal/ai-status.md` and `docs/internal/ai-changes.md`

## Current User-Facing Surface

- `grafana-util dashboard`
- `grafana-util alert`
- `grafana-util access`
- `grafana-util change`
- `grafana-util status`
- `grafana-util overview`

Keep `README.md`, `README.zh-TW.md`, `docs/user-guide.md`, and `docs/user-guide-TW.md` focused on the maintained operator surface. When command behavior changes, update both user guides together.

## Maintainer Pointers

- Use the Rust runtime as the supported implementation surface.
- Keep the Python package as legacy maintainer reference material only.
- Treat `rust/src/cli.rs` as the command-topology entrypoint and the domain facade modules as runtime dispatch layers.
- Put typed contract details, compatibility rules, and stable field lists in the dedicated internal spec docs referenced by `docs/internal/contract-doc-map.md`.
- Keep version bumps and validation in the standard maintainer flow documented in the repo’s scripts, Makefile, and release notes rather than expanding this page.

## Key File Map

- `rust/src/cli.rs`: namespaced CLI dispatch and help routing.
- `rust/src/dashboard/`: dashboard export, import, diff, inspect, prompt-export, and screenshot workflows.
- `rust/src/datasource.rs`: datasource list, export, import, diff, add, modify, and delete workflows.
- `rust/src/alert.rs`: alerting export, import, diff, and shared alert helpers.
- `rust/src/access/`: access org, user, team, and service-account workflows plus shared helpers.
- `rust/src/sync/`: internal runtime namespace behind the public `change` workflow.
- `python/grafana_utils/`: legacy Python reference implementation.
- `python/tests/`: legacy Python regression coverage.
- `docs/overview-rust.md` and `docs/overview-python.md`: architecture walkthroughs.

## Contract Pointers

- `datasources.json` is the canonical masked-recovery replay artifact; `provisioning/datasources.yaml` is a derived projection.
- `raw/` is the canonical dashboard export variant for staged consumers; `provisioning/` is a derived projection.
- `dashboard` and `datasource` follow the export-root/output-layering contract described in `docs/internal/export-root-output-layering-policy.md`.
- `alert` and `access` follow the boundary rules described in `docs/internal/alert-access-contract-policy.md`.
- `docs/internal/dashboard-export-root-contract.md` and `docs/internal/datasource-masked-recovery-contract.md` carry the detailed current requirements for those domains.

For the current summary/spec/trace map, start with [`docs/internal/contract-doc-map.md`](/Users/kendlee/work/grafana-utils/docs/internal/contract-doc-map.md).
