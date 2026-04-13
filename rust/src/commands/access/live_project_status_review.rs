use serde_json::{Map, Value};

use super::{
    scalar_text, value_bool, LiveReviewSignalGroup, LiveScopeReviewSignal,
    ACCESS_FINDING_KIND_SERVICE_ACCOUNTS_DISABLED_COUNT,
    ACCESS_FINDING_KIND_SERVICE_ACCOUNTS_ROLE_GAP,
    ACCESS_FINDING_KIND_SERVICE_ACCOUNTS_TOKENLESS_COUNT, ACCESS_FINDING_KIND_TEAMS_EMAIL_GAP,
    ACCESS_FINDING_KIND_TEAMS_EMPTY_COUNT, ACCESS_FINDING_KIND_USERS_ADMIN_COUNT,
    ACCESS_FINDING_KIND_USERS_IDENTITY_GAP,
};

fn is_admin_user(user: &Map<String, Value>) -> bool {
    super::normalize_org_role(user.get("role").or_else(|| user.get("orgRole"))) == "Admin"
        || value_bool(user.get("isGrafanaAdmin"))
            .or_else(|| value_bool(user.get("isAdmin")))
            .unwrap_or(false)
}

pub(super) fn build_user_review_signals(
    users: &[Map<String, Value>],
) -> Vec<LiveScopeReviewSignal> {
    let identity_gap_count = users
        .iter()
        .filter(|user| {
            scalar_text(user.get("login")).trim().is_empty()
                || scalar_text(user.get("email")).trim().is_empty()
        })
        .count();
    let admin_count = users.iter().filter(|user| is_admin_user(user)).count();
    let mut review_signals = Vec::new();
    if identity_gap_count > 0 {
        review_signals.push(LiveScopeReviewSignal::new(
            LiveReviewSignalGroup::ImportReview,
            "users missing login or email",
            "live.users.identityGapCount",
            ACCESS_FINDING_KIND_USERS_IDENTITY_GAP,
            identity_gap_count,
        ));
    }
    if admin_count > 0 {
        review_signals.push(LiveScopeReviewSignal::new(
            LiveReviewSignalGroup::DriftSeverity,
            "admin users",
            "live.users.adminCount",
            ACCESS_FINDING_KIND_USERS_ADMIN_COUNT,
            admin_count,
        ));
    }
    review_signals
}

pub(super) fn build_team_review_signals(
    teams: &[Map<String, Value>],
) -> Vec<LiveScopeReviewSignal> {
    let email_gap_count = teams
        .iter()
        .filter(|team| scalar_text(team.get("email")).trim().is_empty())
        .count();
    let empty_count = teams
        .iter()
        .filter(|team| {
            scalar_text(team.get("memberCount"))
                .parse::<usize>()
                .unwrap_or(0)
                == 0
        })
        .count();
    let mut review_signals = Vec::new();
    if email_gap_count > 0 {
        review_signals.push(LiveScopeReviewSignal::new(
            LiveReviewSignalGroup::ImportReview,
            "teams missing email",
            "live.teams.emailGapCount",
            ACCESS_FINDING_KIND_TEAMS_EMAIL_GAP,
            email_gap_count,
        ));
    }
    if empty_count > 0 {
        review_signals.push(LiveScopeReviewSignal::new(
            LiveReviewSignalGroup::DriftSeverity,
            "empty teams",
            "live.teams.emptyCount",
            ACCESS_FINDING_KIND_TEAMS_EMPTY_COUNT,
            empty_count,
        ));
    }
    review_signals
}

pub(super) fn build_service_account_review_signals(
    service_accounts: &[Map<String, Value>],
) -> Vec<LiveScopeReviewSignal> {
    let role_gap_count = service_accounts
        .iter()
        .filter(|service_account| scalar_text(service_account.get("role")).trim().is_empty())
        .count();
    let disabled_count = service_accounts
        .iter()
        .filter(|service_account| {
            value_bool(service_account.get("disabled"))
                .or_else(|| value_bool(service_account.get("isDisabled")))
                .unwrap_or(false)
        })
        .count();
    let tokenless_count = service_accounts
        .iter()
        .filter(|service_account| scalar_text(service_account.get("tokens")) == "0")
        .count();
    let mut review_signals = Vec::new();
    if role_gap_count > 0 {
        review_signals.push(LiveScopeReviewSignal::new(
            LiveReviewSignalGroup::ImportReview,
            "service accounts missing role",
            "live.serviceAccounts.roleGapCount",
            ACCESS_FINDING_KIND_SERVICE_ACCOUNTS_ROLE_GAP,
            role_gap_count,
        ));
    }
    if disabled_count > 0 {
        review_signals.push(LiveScopeReviewSignal::new(
            LiveReviewSignalGroup::DriftSeverity,
            "disabled service accounts",
            "live.serviceAccounts.disabledCount",
            ACCESS_FINDING_KIND_SERVICE_ACCOUNTS_DISABLED_COUNT,
            disabled_count,
        ));
    }
    if tokenless_count > 0 {
        review_signals.push(LiveScopeReviewSignal::new(
            LiveReviewSignalGroup::DriftSeverity,
            "tokenless service accounts",
            "live.serviceAccounts.tokenlessCount",
            ACCESS_FINDING_KIND_SERVICE_ACCOUNTS_TOKENLESS_COUNT,
            tokenless_count,
        ));
    }
    review_signals
}
