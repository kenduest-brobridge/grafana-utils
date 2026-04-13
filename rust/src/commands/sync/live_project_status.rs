//! First-pass live sync domain-status producers.
//!
//! Maintainer note:
//! - These builders are intentionally conservative and do not infer readiness
//!   from live transport plumbing alone.
//! - Until live sync and live promotion gain staged desired/bundle inputs, the
//!   returned status stays `unknown` with explicit source attribution and next
//!   steps.

#![allow(dead_code)]

use crate::project_status::ProjectDomainStatus;

const LIVE_STATUS_UNKNOWN: &str = "unknown";
const LIVE_REASON_LIVE_SUPPORT_UNAVAILABLE: &str = "live-support-unavailable";
const LIVE_SCOPE: &str = "live";
const LIVE_MODE: &str = "transport-only";

const LIVE_SYNC_DOMAIN_ID: &str = "sync";
const LIVE_SYNC_SOURCE_KIND: &str = "live-sync-status";
const LIVE_SYNC_SIGNAL_KEYS: &[&str] = &["staged.desired", "staged.package-test"];
const LIVE_SYNC_NEXT_ACTIONS: &[&str] =
    &["provide staged desired and package-test inputs before interpreting live sync readiness"];

const LIVE_PROMOTION_DOMAIN_ID: &str = "promotion";
const LIVE_PROMOTION_SOURCE_KIND: &str = "live-promotion-status";
const LIVE_PROMOTION_SIGNAL_KEYS: &[&str] = &[
    "staged.source-bundle",
    "staged.target-inventory",
    "staged.mapping",
];
const LIVE_PROMOTION_NEXT_ACTIONS: &[&str] = &[
    "provide staged workspace package, target inventory, and mapping inputs before interpreting live promotion readiness",
];

fn build_unknown_live_domain_status(
    id: &str,
    source_kind: &str,
    signal_keys: &[&str],
    next_actions: &[&str],
) -> ProjectDomainStatus {
    ProjectDomainStatus {
        id: id.to_string(),
        scope: LIVE_SCOPE.to_string(),
        mode: LIVE_MODE.to_string(),
        status: LIVE_STATUS_UNKNOWN.to_string(),
        reason_code: LIVE_REASON_LIVE_SUPPORT_UNAVAILABLE.to_string(),
        primary_count: 0,
        blocker_count: 0,
        warning_count: 0,
        source_kinds: vec![source_kind.to_string()],
        signal_keys: signal_keys.iter().map(|item| (*item).to_string()).collect(),
        blockers: Vec::new(),
        warnings: Vec::new(),
        next_actions: next_actions
            .iter()
            .map(|item| (*item).to_string())
            .collect(),
        freshness: Default::default(),
    }
}

pub(crate) fn build_live_sync_domain_status() -> ProjectDomainStatus {
    build_unknown_live_domain_status(
        LIVE_SYNC_DOMAIN_ID,
        LIVE_SYNC_SOURCE_KIND,
        LIVE_SYNC_SIGNAL_KEYS,
        LIVE_SYNC_NEXT_ACTIONS,
    )
}

pub(crate) fn build_live_promotion_domain_status() -> ProjectDomainStatus {
    build_unknown_live_domain_status(
        LIVE_PROMOTION_DOMAIN_ID,
        LIVE_PROMOTION_SOURCE_KIND,
        LIVE_PROMOTION_SIGNAL_KEYS,
        LIVE_PROMOTION_NEXT_ACTIONS,
    )
}

#[cfg(test)]
mod tests {
    use super::{build_live_promotion_domain_status, build_live_sync_domain_status};

    #[test]
    fn build_live_sync_domain_status_is_conservative_and_explicit() {
        let status = build_live_sync_domain_status();

        assert_eq!(status.id, "sync");
        assert_eq!(status.scope, "live");
        assert_eq!(status.mode, "transport-only");
        assert_eq!(status.status, "unknown");
        assert_eq!(status.reason_code, "live-support-unavailable");
        assert_eq!(status.primary_count, 0);
        assert_eq!(status.blocker_count, 0);
        assert_eq!(status.warning_count, 0);
        assert_eq!(status.source_kinds, vec!["live-sync-status".to_string()]);
        assert_eq!(
            status.signal_keys,
            vec![
                "staged.desired".to_string(),
                "staged.package-test".to_string(),
            ]
        );
        assert!(status.blockers.is_empty());
        assert!(status.warnings.is_empty());
        assert_eq!(
            status.next_actions,
            vec![
                "provide staged desired and package-test inputs before interpreting live sync readiness".to_string(),
            ]
        );
    }

    #[test]
    fn build_live_promotion_domain_status_is_conservative_and_explicit() {
        let status = build_live_promotion_domain_status();

        assert_eq!(status.id, "promotion");
        assert_eq!(status.scope, "live");
        assert_eq!(status.mode, "transport-only");
        assert_eq!(status.status, "unknown");
        assert_eq!(status.reason_code, "live-support-unavailable");
        assert_eq!(status.primary_count, 0);
        assert_eq!(status.blocker_count, 0);
        assert_eq!(status.warning_count, 0);
        assert_eq!(
            status.source_kinds,
            vec!["live-promotion-status".to_string()]
        );
        assert_eq!(
            status.signal_keys,
            vec![
                "staged.source-bundle".to_string(),
                "staged.target-inventory".to_string(),
                "staged.mapping".to_string(),
            ]
        );
        assert!(status.blockers.is_empty());
        assert!(status.warnings.is_empty());
        assert_eq!(
            status.next_actions,
            vec![
                "provide staged workspace package, target inventory, and mapping inputs before interpreting live promotion readiness".to_string(),
            ]
        );
    }
}
