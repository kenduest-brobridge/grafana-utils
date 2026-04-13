use serde_json::Value;
use tempfile::tempdir;

use crate::export_metadata::{
    build_export_metadata_common, export_metadata_common_map, EXPORT_BUNDLE_KIND_ROOT,
    EXPORT_METADATA_VERSION,
};

#[test]
fn export_metadata_fields_include_common_contract() {
    let temp = tempdir().unwrap();
    let common = build_export_metadata_common(
        "dashboard",
        "dashboards",
        EXPORT_BUNDLE_KIND_ROOT,
        "live-export",
        Some("http://grafana.example.com"),
        Some(temp.path()),
        Some("prod"),
        Some("all-orgs"),
        None,
        None,
        &temp.path().join("index.json"),
        &temp.path().join("export-metadata.json"),
        3,
    );
    let fields = export_metadata_common_map(&common);

    assert_eq!(
        fields["metadataVersion"],
        Value::from(EXPORT_METADATA_VERSION)
    );
    assert_eq!(fields["domain"], Value::from("dashboard"));
    assert_eq!(fields["resourceKind"], Value::from("dashboards"));
    assert_eq!(fields["bundleKind"], Value::from("export-root"));
    assert_eq!(fields["source"]["kind"], Value::from("live-export"));
    assert_eq!(
        fields["source"]["url"],
        Value::from("http://grafana.example.com")
    );
    assert_eq!(fields["source"]["profile"], Value::from("prod"));
    assert_eq!(fields["source"]["orgScope"], Value::from("all-orgs"));
    assert_eq!(fields["capture"]["recordCount"], Value::from(3));
    assert_eq!(
        fields["paths"]["metadata"],
        Value::from(
            temp.path()
                .join("export-metadata.json")
                .display()
                .to_string()
        )
    );
}

#[test]
fn export_metadata_common_keeps_org_scope_context() {
    let temp = tempdir().unwrap();
    let fields = export_metadata_common_map(&build_export_metadata_common(
        "access",
        "orgs",
        EXPORT_BUNDLE_KIND_ROOT,
        "live-export",
        None,
        Some(temp.path()),
        None,
        Some("single-org"),
        Some("7"),
        Some("platform"),
        &temp.path().join("orgs.json"),
        &temp.path().join("export-metadata.json"),
        3,
    ));

    assert_eq!(fields["domain"], Value::from("access"));
    assert_eq!(fields["resourceKind"], Value::from("orgs"));
    assert_eq!(fields["source"]["orgId"], Value::from("7"));
    assert_eq!(fields["source"]["orgName"], Value::from("platform"));
    assert_eq!(
        fields["paths"]["artifact"],
        Value::from(temp.path().join("orgs.json").display().to_string())
    );
}
