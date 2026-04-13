//! Alert bundle input registries for export sections and sync resource kinds.

use crate::alert::{
    build_contact_point_import_payload, build_mute_timing_import_payload,
    build_policies_import_payload, build_template_import_payload, CONTACT_POINT_KIND,
    MUTE_TIMING_KIND, POLICIES_KIND, RULE_KIND, TEMPLATE_KIND,
};
use crate::common::Result;
use serde_json::{Map, Value};

pub(crate) type AlertPayloadBuilder = fn(&Map<String, Value>) -> Result<Map<String, Value>>;

#[derive(Debug, Clone, Copy)]
pub(crate) struct AlertExportSectionSpec {
    pub(crate) path_prefix: &'static str,
    pub(crate) section_key: &'static str,
    pub(crate) summary_key: &'static str,
}

pub(crate) const ALERT_EXPORT_SECTION_SPECS: &[AlertExportSectionSpec] = &[
    AlertExportSectionSpec {
        path_prefix: "rules",
        section_key: "rules",
        summary_key: "ruleCount",
    },
    AlertExportSectionSpec {
        path_prefix: "contact-points",
        section_key: "contactPoints",
        summary_key: "contactPointCount",
    },
    AlertExportSectionSpec {
        path_prefix: "mute-timings",
        section_key: "muteTimings",
        summary_key: "muteTimingCount",
    },
    AlertExportSectionSpec {
        path_prefix: "policies",
        section_key: "policies",
        summary_key: "policyCount",
    },
    AlertExportSectionSpec {
        path_prefix: "templates",
        section_key: "templates",
        summary_key: "templateCount",
    },
];

#[derive(Clone, Copy)]
pub(crate) struct AlertSyncKindSpec {
    pub(crate) document_kind: &'static str,
    pub(crate) sync_kind: &'static str,
    pub(crate) identity_fields: &'static [&'static str],
    pub(crate) title_fields: &'static [&'static str],
    pub(crate) uid_from_identity: bool,
    pub(crate) name_from_identity: bool,
    pub(crate) default_identity: Option<&'static str>,
    pub(crate) payload_builder: Option<AlertPayloadBuilder>,
}

pub(crate) const ALERT_SYNC_KIND_SPECS: &[AlertSyncKindSpec] = &[
    AlertSyncKindSpec {
        document_kind: RULE_KIND,
        sync_kind: "alert",
        identity_fields: &["uid", "name"],
        title_fields: &["name", "title", "receiver"],
        uid_from_identity: true,
        name_from_identity: false,
        default_identity: None,
        payload_builder: None,
    },
    AlertSyncKindSpec {
        document_kind: CONTACT_POINT_KIND,
        sync_kind: "alert-contact-point",
        identity_fields: &["uid", "name"],
        title_fields: &["name", "title", "receiver"],
        uid_from_identity: true,
        name_from_identity: false,
        default_identity: None,
        payload_builder: Some(build_contact_point_import_payload),
    },
    AlertSyncKindSpec {
        document_kind: MUTE_TIMING_KIND,
        sync_kind: "alert-mute-timing",
        identity_fields: &["name"],
        title_fields: &["name", "title", "receiver"],
        uid_from_identity: false,
        name_from_identity: true,
        default_identity: None,
        payload_builder: Some(build_mute_timing_import_payload),
    },
    AlertSyncKindSpec {
        document_kind: POLICIES_KIND,
        sync_kind: "alert-policy",
        identity_fields: &["receiver"],
        title_fields: &["name", "title", "receiver"],
        uid_from_identity: false,
        name_from_identity: false,
        default_identity: Some("root"),
        payload_builder: Some(build_policies_import_payload),
    },
    AlertSyncKindSpec {
        document_kind: TEMPLATE_KIND,
        sync_kind: "alert-template",
        identity_fields: &["name"],
        title_fields: &["name", "title", "receiver"],
        uid_from_identity: false,
        name_from_identity: true,
        default_identity: None,
        payload_builder: Some(build_template_import_payload),
    },
];

pub(crate) fn alert_export_section_for_path(relative_path: &str) -> Option<AlertExportSectionSpec> {
    let first = relative_path.split('/').next().unwrap_or("");
    ALERT_EXPORT_SECTION_SPECS
        .iter()
        .copied()
        .find(|spec| spec.path_prefix == first)
}

pub(crate) fn alert_sync_kind_spec_for_document_kind(
    document_kind: &str,
) -> Option<AlertSyncKindSpec> {
    ALERT_SYNC_KIND_SPECS
        .iter()
        .copied()
        .find(|spec| spec.document_kind == document_kind)
}

pub(crate) fn alert_sync_kind_spec(sync_kind: &str) -> Option<AlertSyncKindSpec> {
    ALERT_SYNC_KIND_SPECS
        .iter()
        .copied()
        .find(|spec| spec.sync_kind == sync_kind)
}
