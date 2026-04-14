//! Datasource parser and inventory shape behavior tests.

use super::*;

#[test]
fn parse_datasource_import_preserves_requested_path() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "import",
        "--input-dir",
        "./datasources",
        "--org-id",
        "7",
        "--dry-run",
        "--table",
    ]);

    match args.command {
        crate::datasource::DatasourceGroupCommand::Import(inner) => {
            assert_eq!(inner.input_dir, Path::new("./datasources"));
            assert_eq!(inner.input_format, DatasourceImportInputFormat::Inventory);
            assert_eq!(inner.org_id, Some(7));
            assert!(inner.dry_run);
            assert!(inner.table);
        }
        _ => panic!("expected datasource import"),
    }
}

#[test]
fn parse_datasource_list_supports_output_columns_all_and_list_columns() {
    let args = DatasourceCliArgs::parse_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--output-columns",
        "all",
        "--list-columns",
    ]);

    match args.command {
        crate::datasource::DatasourceGroupCommand::List(inner) => {
            assert_eq!(inner.output_columns, vec!["all"]);
            assert!(inner.list_columns);
        }
        _ => panic!("expected datasource list"),
    }
}

#[test]
fn parse_datasource_list_supports_nested_output_columns() {
    let args = DatasourceCliArgs::parse_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--output-columns",
        "uid,jsonData.organization,orgId",
    ]);

    match args.command {
        crate::datasource::DatasourceGroupCommand::List(inner) => {
            assert_eq!(
                inner.output_columns,
                vec!["uid", "jsonData.organization", "org_id"]
            );
        }
        _ => panic!("expected datasource list"),
    }
}

#[test]
fn parse_datasource_import_supports_provisioning_input_format() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "import",
        "--input-dir",
        "./datasources/provisioning",
        "--input-format",
        "provisioning",
        "--dry-run",
    ]);

    match args.command {
        crate::datasource::DatasourceGroupCommand::Import(inner) => {
            assert_eq!(
                inner.input_format,
                DatasourceImportInputFormat::Provisioning
            );
            assert_eq!(inner.input_dir, Path::new("./datasources/provisioning"));
            assert!(inner.dry_run);
        }
        _ => panic!("expected datasource import"),
    }
}

#[test]
fn parse_datasource_import_supports_output_format_table() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "import",
        "--input-dir",
        "./datasources",
        "--dry-run",
        "--output-format",
        "table",
    ]);

    match args.command {
        crate::datasource::DatasourceGroupCommand::Import(inner) => {
            assert!(inner.dry_run);
            assert!(inner.table);
            assert!(!inner.json);
        }
        _ => panic!("expected datasource import"),
    }
}

#[test]
fn parse_datasource_import_supports_output_columns() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "import",
        "--input-dir",
        "./datasources",
        "--dry-run",
        "--output-format",
        "table",
        "--output-columns",
        "uid,matchBasis,action,orgId,file",
    ]);

    match args.command {
        crate::datasource::DatasourceGroupCommand::Import(inner) => {
            assert!(inner.table);
            assert_eq!(
                inner.output_columns,
                vec!["uid", "match_basis", "action", "org_id", "file"]
            );
        }
        _ => panic!("expected datasource import"),
    }
}

#[test]
fn parse_datasource_import_supports_output_columns_all_and_list_columns() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "import",
        "--input-dir",
        "./datasources",
        "--dry-run",
        "--output-format",
        "table",
        "--output-columns",
        "all",
        "--list-columns",
    ]);

    match args.command {
        crate::datasource::DatasourceGroupCommand::Import(inner) => {
            assert!(inner.table);
            assert_eq!(inner.output_columns, vec!["all"]);
            assert!(inner.list_columns);
        }
        _ => panic!("expected datasource import"),
    }
}

#[test]
fn parse_datasource_export_supports_org_scope_flags() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "export",
        "--output-dir",
        "./datasources",
        "--org-id",
        "7",
    ]);

    match args.command {
        crate::datasource::DatasourceGroupCommand::Export(inner) => {
            assert_eq!(inner.output_dir, Path::new("./datasources"));
            assert_eq!(inner.org_id, Some(7));
            assert!(!inner.all_orgs);
        }
        _ => panic!("expected datasource export"),
    }
}

#[test]
fn parse_datasource_export_supports_all_orgs_flag() {
    let args = DatasourceCliArgs::parse_normalized_from(["grafana-util", "export", "--all-orgs"]);

    match args.command {
        crate::datasource::DatasourceGroupCommand::Export(inner) => {
            assert!(inner.all_orgs);
            assert_eq!(inner.org_id, None);
        }
        _ => panic!("expected datasource export"),
    }
}

#[test]
fn parse_datasource_export_supports_without_provisioning_flag() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "export",
        "--without-datasource-provisioning",
    ]);

    match args.command {
        crate::datasource::DatasourceGroupCommand::Export(inner) => {
            assert!(inner.without_datasource_provisioning);
        }
        _ => panic!("expected datasource export"),
    }
}

#[test]
fn parse_datasource_import_supports_use_export_org_flags() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "import",
        "--input-dir",
        "./datasources",
        "--use-export-org",
        "--only-org-id",
        "2",
        "--only-org-id",
        "5",
        "--create-missing-orgs",
    ]);

    match args.command {
        crate::datasource::DatasourceGroupCommand::Import(inner) => {
            assert!(inner.use_export_org);
            assert_eq!(inner.only_org_id, vec![2, 5]);
            assert!(inner.create_missing_orgs);
            assert_eq!(inner.org_id, None);
        }
        _ => panic!("expected datasource import"),
    }
}

#[test]
fn parse_datasource_import_supports_secret_values_argument() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "import",
        "--input-dir",
        "./datasources",
        "--secret-values",
        r#"{"loki-basic-auth":"secret-value"}"#,
    ]);

    match args.command {
        crate::datasource::DatasourceGroupCommand::Import(inner) => {
            assert_eq!(
                inner.secret_values.as_deref(),
                Some(r#"{"loki-basic-auth":"secret-value"}"#)
            );
        }
        _ => panic!("expected datasource import"),
    }
}

#[test]
fn parse_datasource_import_rejects_org_id_with_use_export_org() {
    let error = DatasourceCliArgs::try_parse_from([
        "grafana-util",
        "import",
        "--input-dir",
        "./datasources",
        "--org-id",
        "7",
        "--use-export-org",
    ])
    .unwrap_err();

    assert!(error.to_string().contains("--org-id"));
    assert!(error.to_string().contains("--use-export-org"));
}
