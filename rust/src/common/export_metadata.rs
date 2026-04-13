//! Shared additive export-metadata contract helpers.
//!
//! This module owns the common machine-readable metadata block that can be
//! attached to export roots and other export artifacts without changing the
//! legacy top-level reader contract.

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::path::Path;

use crate::common::tool_version;

pub(crate) const EXPORT_METADATA_VERSION: i64 = 2;
pub(crate) const EXPORT_BUNDLE_KIND_ROOT: &str = "export-root";
#[allow(dead_code)]
pub(crate) const EXPORT_BUNDLE_KIND_ARTIFACT: &str = "export-artifact";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ExportMetadataSource {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(rename = "orgScope", skip_serializing_if = "Option::is_none")]
    pub org_scope: Option<String>,
    #[serde(rename = "orgId", skip_serializing_if = "Option::is_none")]
    pub org_id: Option<String>,
    #[serde(rename = "orgName", skip_serializing_if = "Option::is_none")]
    pub org_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ExportMetadataCapture {
    #[serde(rename = "toolVersion")]
    pub tool_version: String,
    #[serde(rename = "capturedAt")]
    pub captured_at: String,
    #[serde(rename = "recordCount")]
    pub record_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ExportMetadataPaths {
    pub artifact: String,
    pub metadata: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ExportMetadataCommon {
    #[serde(rename = "metadataVersion")]
    pub metadata_version: i64,
    pub domain: String,
    #[serde(rename = "resourceKind")]
    pub resource_kind: String,
    #[serde(rename = "bundleKind")]
    pub bundle_kind: String,
    pub source: ExportMetadataSource,
    pub capture: ExportMetadataCapture,
    pub paths: ExportMetadataPaths,
}

pub(crate) fn build_export_metadata_source(
    kind: &str,
    url: Option<&str>,
    path: Option<&Path>,
    profile: Option<&str>,
    org_scope: Option<&str>,
    org_id: Option<&str>,
    org_name: Option<&str>,
) -> ExportMetadataSource {
    ExportMetadataSource {
        kind: kind.to_string(),
        url: url.map(str::to_owned),
        path: path.map(|path| path.display().to_string()),
        profile: profile.map(str::to_owned),
        org_scope: org_scope.map(str::to_owned),
        org_id: org_id.map(str::to_owned),
        org_name: org_name.map(str::to_owned),
    }
}

pub(crate) fn build_export_metadata_capture(record_count: usize) -> ExportMetadataCapture {
    ExportMetadataCapture {
        tool_version: tool_version().to_string(),
        captured_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        record_count: record_count as u64,
    }
}

pub(crate) fn build_export_metadata_paths(artifact: &Path, metadata: &Path) -> ExportMetadataPaths {
    ExportMetadataPaths {
        artifact: artifact.display().to_string(),
        metadata: metadata.display().to_string(),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_export_metadata_common(
    domain: &str,
    resource_kind: &str,
    bundle_kind: &str,
    source_kind: &str,
    source_url: Option<&str>,
    source_path: Option<&Path>,
    source_profile: Option<&str>,
    org_scope: Option<&str>,
    org_id: Option<&str>,
    org_name: Option<&str>,
    artifact: &Path,
    metadata: &Path,
    record_count: usize,
) -> ExportMetadataCommon {
    ExportMetadataCommon {
        metadata_version: EXPORT_METADATA_VERSION,
        domain: domain.to_string(),
        resource_kind: resource_kind.to_string(),
        bundle_kind: bundle_kind.to_string(),
        source: build_export_metadata_source(
            source_kind,
            source_url,
            source_path,
            source_profile,
            org_scope,
            org_id,
            org_name,
        ),
        capture: build_export_metadata_capture(record_count),
        paths: build_export_metadata_paths(artifact, metadata),
    }
}

pub(crate) fn export_metadata_common_map(common: &ExportMetadataCommon) -> Map<String, Value> {
    match serde_json::to_value(common) {
        Ok(Value::Object(object)) => object,
        Ok(other) => panic!("export metadata common serialized to non-object: {other:?}"),
        Err(error) => panic!("failed to serialize export metadata common: {error}"),
    }
}
