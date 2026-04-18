# Mutation Review Envelope Inventory

This note records the current review/dry-run/apply shapes before introducing a
shared mutation review envelope.

## Current Shared Vocabulary

`rust/src/commands/review_contract.rs` already centralizes action, status,
reason, and hint strings used across plan, preview, apply, and TUI-adjacent
flows.

Current stable vocabulary:

- actions: `would-create`, `would-update`, `would-delete`, `same`,
  `extra-remote`, `unmanaged`, and `blocked-*`
- statuses: `ready`, `same`, `warning`, `blocked`
- reasons: ambiguous live name match, missing target org, provisioned/managed
  target, read-only target, UID/name mismatch
- hints: missing remote, remote-only, requires secret values

## Current Domain Shapes

- Workspace preview/review already has the closest internal adapter:
  `WorkspaceReviewView`, `WorkspaceReviewAction`, `WorkspaceReviewDomain`, and
  `WorkspaceReviewSummary`.
- Datasource, access, dashboard, and alert plan/apply paths use the shared
  vocabulary but still keep domain-specific row payloads and summaries.
- Dashboard import dry-run and history restore preview expose review evidence,
  but they are not yet normalized as generic mutation actions.
- Snapshot review is inventory-oriented and should not be forced into a mutation
  envelope.

## Constraints

- Do not change public JSON contracts just to introduce the shared envelope.
- Keep domain payloads behind an adapter until at least two concrete domains use
  the same fields.
- TUI consumers should read a normalized view model, not drive plan-builder
  shape decisions.
- Shared fields should be derived from existing domain outputs, not invented
  ahead of real consumers.

## Candidate Internal Envelope

Start with an internal adapter only:

- `action_id`
- `domain`
- `resource_kind`
- `identity`
- `action`
- `status`
- `reason`
- `blocked_reason`
- `review_hints`
- `risk`
- `raw`

Do not promote this to a public contract until the adapter has covered at least
workspace plus one domain plan surface without requiring lossy mapping.
