//! Shared overview artifact-kind metadata.

use super::{
    OverviewSummary, OVERVIEW_ARTIFACT_ACCESS_ORG_EXPORT_KIND,
    OVERVIEW_ARTIFACT_ACCESS_SERVICE_ACCOUNT_EXPORT_KIND,
    OVERVIEW_ARTIFACT_ACCESS_TEAM_EXPORT_KIND, OVERVIEW_ARTIFACT_ACCESS_USER_EXPORT_KIND,
    OVERVIEW_ARTIFACT_ALERT_EXPORT_KIND, OVERVIEW_ARTIFACT_BUNDLE_PREFLIGHT_KIND,
    OVERVIEW_ARTIFACT_DASHBOARD_EXPORT_KIND, OVERVIEW_ARTIFACT_DATASOURCE_EXPORT_KIND,
    OVERVIEW_ARTIFACT_PROMOTION_PREFLIGHT_KIND, OVERVIEW_ARTIFACT_SYNC_SUMMARY_KIND,
};
use crate::common::{message, Result};
#[cfg(feature = "tui")]
use ratatui::style::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OverviewArtifactKind {
    DashboardExport,
    DatasourceExport,
    AlertExport,
    AccessUserExport,
    AccessTeamExport,
    AccessOrgExport,
    AccessServiceAccountExport,
    SyncSummary,
    BundlePreflight,
    PromotionPreflight,
}

impl OverviewArtifactKind {
    pub(crate) fn parse(kind: &str) -> Option<Self> {
        match kind {
            OVERVIEW_ARTIFACT_DASHBOARD_EXPORT_KIND => Some(Self::DashboardExport),
            OVERVIEW_ARTIFACT_DATASOURCE_EXPORT_KIND => Some(Self::DatasourceExport),
            OVERVIEW_ARTIFACT_ALERT_EXPORT_KIND => Some(Self::AlertExport),
            OVERVIEW_ARTIFACT_ACCESS_USER_EXPORT_KIND => Some(Self::AccessUserExport),
            OVERVIEW_ARTIFACT_ACCESS_TEAM_EXPORT_KIND => Some(Self::AccessTeamExport),
            OVERVIEW_ARTIFACT_ACCESS_ORG_EXPORT_KIND => Some(Self::AccessOrgExport),
            OVERVIEW_ARTIFACT_ACCESS_SERVICE_ACCOUNT_EXPORT_KIND => {
                Some(Self::AccessServiceAccountExport)
            }
            OVERVIEW_ARTIFACT_SYNC_SUMMARY_KIND => Some(Self::SyncSummary),
            OVERVIEW_ARTIFACT_BUNDLE_PREFLIGHT_KIND => Some(Self::BundlePreflight),
            OVERVIEW_ARTIFACT_PROMOTION_PREFLIGHT_KIND => Some(Self::PromotionPreflight),
            _ => None,
        }
    }

    pub(crate) fn item_kind(self) -> &'static str {
        match self {
            Self::DashboardExport => "dashboard",
            Self::DatasourceExport => "datasource",
            Self::AlertExport => "alert",
            Self::AccessUserExport => "user",
            Self::AccessTeamExport => "team",
            Self::AccessOrgExport => "org",
            Self::AccessServiceAccountExport => "service-account",
            Self::SyncSummary => "sync",
            Self::BundlePreflight => "bundle-preflight",
            Self::PromotionPreflight => "promotion-preflight",
        }
    }

    pub(crate) fn fact_breakdown_label(self) -> &'static str {
        match self {
            Self::DashboardExport => "Coverage",
            Self::DatasourceExport => "Inventory Facts",
            Self::AlertExport => "Alert Assets",
            Self::AccessUserExport
            | Self::AccessTeamExport
            | Self::AccessOrgExport
            | Self::AccessServiceAccountExport => "Export Facts",
            Self::SyncSummary => "Resource Mix",
            Self::BundlePreflight => "Blocking Signals",
            Self::PromotionPreflight => "Mapping Signals",
        }
    }

    #[cfg(feature = "tui")]
    pub(crate) fn section_color(self) -> Color {
        match self {
            Self::DashboardExport => Color::Yellow,
            Self::DatasourceExport => Color::Cyan,
            Self::AlertExport => Color::Red,
            Self::AccessUserExport
            | Self::AccessTeamExport
            | Self::AccessOrgExport
            | Self::AccessServiceAccountExport => Color::Green,
            Self::SyncSummary => Color::Magenta,
            Self::BundlePreflight | Self::PromotionPreflight => Color::LightBlue,
        }
    }

    pub(crate) fn count_summary(self, summary: &mut OverviewSummary) {
        match self {
            Self::DashboardExport => summary.dashboard_export_count += 1,
            Self::DatasourceExport => summary.datasource_export_count += 1,
            Self::AlertExport => summary.alert_export_count += 1,
            Self::AccessUserExport => summary.access_user_export_count += 1,
            Self::AccessTeamExport => summary.access_team_export_count += 1,
            Self::AccessOrgExport => summary.access_org_export_count += 1,
            Self::AccessServiceAccountExport => summary.access_service_account_export_count += 1,
            Self::SyncSummary => summary.sync_summary_count += 1,
            Self::BundlePreflight => summary.bundle_preflight_count += 1,
            Self::PromotionPreflight => summary.promotion_preflight_count += 1,
        }
    }
}

pub(crate) fn parse_overview_artifact_kind(kind: &str) -> Result<OverviewArtifactKind> {
    OverviewArtifactKind::parse(kind)
        .ok_or_else(|| message(format!("Overview artifact kind is not supported: {kind}")))
}
