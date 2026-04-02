# Alert And Access Contract Policy

Requirements and maintainer policy for the current `alert` and `access`
contract boundaries.

This document exists to prevent accidental convergence into the
`dashboard` / `datasource` export-root pattern before either domain defines
its own stable root contract.

## Summary

- `dashboard` and `datasource` are the repo's current export-root /
  output-layering domains.
- `alert` and `access` are not in that category today.
- Both domains may evolve later, but only after their contract is defined
  explicitly and intentionally.

## Current Contract Types

### Alert

- `alert` currently uses a resource-tree export contract with a root index.
- The root index is currently organization/navigation metadata for the export
  tree.
- The root index is not yet a repo-level export-root contract equivalent to
  `dashboard` or `datasource`.
- Import/diff/review behavior should continue to treat the resource tree and
  its per-kind documents as the primary contract boundary unless a future
  promotion changes that explicitly.

### Access

- `access` currently uses resource-specific export bundle contracts.
- The contract boundary is the individual bundle family, such as users,
  teams, orgs, or service accounts.
- Bundle `kind`, version, and records remain the primary compatibility
  boundary.
- A directory containing multiple access bundles is not, by itself, an
  export-root contract.

## Current Maintainer Rules

- Do not add `scopeKind`, `org-root`, `all-orgs-root`, `workspace-root`, or
  shared root-manifest vocabulary to `alert` or `access` unless the domain is
  being explicitly promoted.
- Do not treat a convenience `index.json` or a directory of related bundles as
  sufficient proof that a domain has become an export-root domain.
- Keep domain-owned loaders, validators, and help text aligned with the
  current contract type:
  - `alert`: resource tree with root index
  - `access`: resource-specific bundles

## Promotion Requirements

A future promotion of `alert` or `access` into an explicit export-root domain
must satisfy all of the following before implementation lands:

- Stable root identity:
  the domain must define what the root represents and how operators identify
  it.
- Stable scope/layout semantics:
  the domain must define any aggregate, org-scoped, or workspace-scoped
  meaning instead of inferring it from layout alone.
- Cross-command root consumers:
  more than one command path must need the root-level contract directly,
  rather than only reading per-resource documents or bundles.
- Written contract first:
  the root contract must be documented in `docs/DEVELOPER.md` before code adds
  root-contract vocabulary or behavior.

## Non-Goals

- This document does not promote `alert` or `access` today.
- This document does not introduce a shared helper or generic manifest model.
- This document does not change runtime behavior.

## Documentation Guidance

- Current maintainer documentation is fragmented enough that contract language
  can drift across short notes, architecture summaries, and change traces.
- Keep this file as the detailed requirements document for `alert` / `access`
  contract boundaries.
- Keep `docs/DEVELOPER.md` as the short policy summary and link back here for
  the detailed requirements.
- Keep `docs/internal/ai-status.md` and `docs/internal/ai-changes.md` limited
  to concise trace entries; they should point to the stable policy document
  instead of restating the full contract.
- If future cleanup reduces duplication, prefer one current requirements doc
  plus one short maintainer summary rather than multiple partially overlapping
  policy notes.

## Decision Default

- Until a promotion is explicitly documented, `alert` remains a resource-tree
  domain and `access` remains a bundle-contract domain.
- When in doubt, keep new behavior inside the current domain-specific
  contract instead of borrowing `dashboard` / `datasource` export-root terms.
