# Export-Root And Output-Layering Policy

Detailed requirements for the repo-level export-root/output-layering pattern.

## Summary

- `dashboard` and `datasource` are the current repo-level export-root /
  output-layering domains.
- Their root export directory is a typed contract boundary.
- Projection files are derived outputs, not alternate primary restore shapes.
- `alert` and `access` remain outside this pattern today.

## Current Domain Rule

- Extend the export-root/output-layering pattern first when adding new
  dashboard or datasource export/output variants.
- Do not assume every staged tree or root index should become a repo-level
  export-root contract.
- Keep domain-owned loaders and validators aligned with the domain's written
  contract rather than inferring new root semantics from layout alone.

## Contract Promotion Rule

- A domain should not be treated as an export-root domain until it has:
  - a stable root identity
  - stable scope/layout semantics
  - real cross-command root-level consumers
  - written contract documentation before the code lands

## Documentation Guidance

- Keep the short repo-level summary in `docs/DEVELOPER.md`.
- Put domain-specific details in the dedicated spec docs.
- Keep `ai-status.md` and `ai-changes.md` trace-oriented and point back to the
  spec docs instead of repeating the same policy text.
