//! Shared review/action contract vocabulary.
//!
//! Keep machine-readable action and status strings centralized so plan, preview,
//! apply, and TUI layers do not drift when comparing the same review contract.

pub(crate) const REVIEW_ACTION_BLOCKED: &str = "blocked";
pub(crate) const REVIEW_ACTION_BLOCKED_AMBIGUOUS: &str = "blocked-ambiguous";
pub(crate) const REVIEW_ACTION_BLOCKED_MISSING_ORG: &str = "blocked-missing-org";
pub(crate) const REVIEW_ACTION_BLOCKED_READ_ONLY: &str = "blocked-read-only";
pub(crate) const REVIEW_ACTION_BLOCKED_TARGET: &str = "blocked-target";
pub(crate) const REVIEW_ACTION_BLOCKED_UID_MISMATCH: &str = "blocked-uid-mismatch";
pub(crate) const REVIEW_ACTION_EXTRA_REMOTE: &str = "extra-remote";
pub(crate) const REVIEW_ACTION_SAME: &str = "same";
pub(crate) const REVIEW_ACTION_UNMANAGED: &str = "unmanaged";
pub(crate) const REVIEW_ACTION_WOULD_CREATE: &str = "would-create";
pub(crate) const REVIEW_ACTION_WOULD_DELETE: &str = "would-delete";
pub(crate) const REVIEW_ACTION_WOULD_UPDATE: &str = "would-update";

pub(crate) const REVIEW_STATUS_BLOCKED: &str = "blocked";
pub(crate) const REVIEW_STATUS_READY: &str = "ready";
pub(crate) const REVIEW_STATUS_SAME: &str = "same";
pub(crate) const REVIEW_STATUS_WARNING: &str = "warning";

pub(crate) const REVIEW_REASON_AMBIGUOUS_LIVE_NAME_MATCH: &str = "ambiguous-live-name-match";
pub(crate) const REVIEW_REASON_TARGET_ORG_MISSING: &str = "target-org-missing";
pub(crate) const REVIEW_REASON_TARGET_PROVISIONED_OR_MANAGED: &str =
    "target-provisioned-or-managed";
pub(crate) const REVIEW_REASON_TARGET_READ_ONLY: &str = "target-read-only";
pub(crate) const REVIEW_REASON_UID_NAME_MISMATCH: &str = "uid-name-mismatch";

pub(crate) const REVIEW_HINT_MISSING_REMOTE: &str = "missing-remote";
pub(crate) const REVIEW_HINT_REMOTE_ONLY: &str = "remote-only";
pub(crate) const REVIEW_HINT_REQUIRES_SECRET_VALUES: &str = "requires-secret-values";

pub(crate) fn is_review_apply_action(action: &str) -> bool {
    matches!(
        action,
        REVIEW_ACTION_WOULD_CREATE | REVIEW_ACTION_WOULD_UPDATE | REVIEW_ACTION_WOULD_DELETE
    )
}

pub(crate) fn is_review_blocked_action(action: &str) -> bool {
    action.starts_with("blocked-")
        || action == REVIEW_ACTION_BLOCKED
        || action == REVIEW_ACTION_UNMANAGED
}
