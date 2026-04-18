use super::super::*;

#[test]
fn datasource_root_help_includes_examples() {
    let mut command = DatasourceCliArgs::command();
    let mut output = Vec::new();
    command.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("Examples:"));
    assert!(help.contains("grafana-util datasource browse"));
    assert!(help.contains("grafana-util datasource types"));
    assert!(help.contains("grafana-util datasource list"));
    assert!(help.contains("grafana-util datasource list --input-dir ./datasources"));
    assert!(help.contains("--all-orgs"));
    assert!(help.contains("grafana-util datasource add"));
    assert!(help.contains("grafana-util datasource import"));
}

#[test]
fn types_help_includes_examples() {
    let mut command = DatasourceCliArgs::command();
    let subcommand = command
        .find_subcommand_mut("types")
        .unwrap_or_else(|| panic!("missing datasource types help"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("--output-format"));
    assert!(help.contains("yaml"));
    assert!(help.contains("grafana-util datasource types"));
}

#[test]
fn list_help_explains_org_scope_flags() {
    let mut command = DatasourceCliArgs::command();
    let subcommand = command
        .find_subcommand_mut("list")
        .unwrap_or_else(|| panic!("missing datasource list help"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("--org-id"));
    assert!(help.contains("--all-orgs"));
    assert!(help.contains("Requires Basic auth"));
    assert!(help.contains("Examples:"));
    assert!(help.contains("--input-dir"));
    assert!(help.contains("--input-format"));
    assert!(help.contains("--text"));
    assert!(help.contains("--table"));
    assert!(help.contains("--csv"));
    assert!(help.contains("--json"));
    assert!(help.contains("--yaml"));
    assert!(help.contains("--output-columns"));
    assert!(help.contains("--list-columns"));
}

#[test]
fn browse_help_mentions_edit_delete_and_examples() {
    let mut command = DatasourceCliArgs::command();
    let subcommand = command
        .find_subcommand_mut("browse")
        .unwrap_or_else(|| panic!("missing datasource browse help"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("Live-only browse against Grafana"));
    assert!(help.contains("--org-id"));
    assert!(help.contains("--all-orgs"));
    assert!(help.contains("grafana-util datasource browse"));
    assert!(help.contains("edit"));
    assert!(help.contains("delete"));
}

#[test]
fn import_help_explains_common_operator_flags() {
    let mut command = DatasourceCliArgs::command();
    let subcommand = command
        .find_subcommand_mut("import")
        .unwrap_or_else(|| panic!("missing datasource import help"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("--input-dir"));
    assert!(help.contains("--org-id"));
    assert!(help.contains("--use-export-org"));
    assert!(help.contains("--only-org-id"));
    assert!(help.contains("--create-missing-orgs"));
    assert!(help.contains("--require-matching-export-org"));
    assert!(help.contains("--replace-existing"));
    assert!(help.contains("--update-existing-only"));
    assert!(help.contains("--secret-values"));
    assert!(help.contains("--dry-run"));
    assert!(help.contains("--table"));
    assert!(help.contains("--json"));
    assert!(help.contains("--output-format"));
    assert!(help.contains("--output-columns"));
    assert!(help.contains("--progress"));
    assert!(help.contains("--verbose"));
    assert!(help.contains("secureJsonDataPlaceholders"));
    assert!(help.contains("Secrets"));
    assert!(help.contains("Examples:"));
    assert!(help.contains("Input Options"));
}

#[test]
fn parse_datasource_browse_supports_org_scope_flag() {
    let args: DatasourceCliArgs =
        DatasourceCliArgs::parse_from(["grafana-util datasource", "browse", "--org-id", "7"]);

    match args.command {
        DatasourceGroupCommand::Browse(inner) => {
            assert_eq!(inner.org_id, Some(7));
            assert!(!inner.all_orgs);
        }
        _ => panic!("expected datasource browse"),
    }
}

#[test]
fn parse_datasource_browse_supports_all_orgs_flag() {
    let args: DatasourceCliArgs =
        DatasourceCliArgs::parse_from(["grafana-util datasource", "browse", "--all-orgs"]);

    match args.command {
        DatasourceGroupCommand::Browse(inner) => {
            assert!(inner.all_orgs);
            assert_eq!(inner.org_id, None);
        }
        _ => panic!("expected datasource browse"),
    }
}

#[test]
fn parse_datasource_browse_rejects_conflicting_org_scope_flags() {
    let result = DatasourceCliArgs::try_parse_from([
        "grafana-util datasource",
        "browse",
        "--org-id",
        "7",
        "--all-orgs",
    ]);

    assert!(result.is_err());
}

#[test]
fn parse_datasource_list_supports_input_dir_and_json() {
    let args: DatasourceCliArgs = DatasourceCliArgs::parse_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--json",
    ]);

    match args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(inner.json);
            assert!(!inner.table);
            assert!(!inner.csv);
            assert!(!inner.text);
            assert!(!inner.yaml);
        }
        _ => panic!("expected datasource list"),
    }
}

#[test]
fn parse_datasource_list_supports_all_output_flags_and_aliases() {
    let table_args: DatasourceCliArgs = DatasourceCliArgs::parse_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--table",
    ]);
    let csv_args: DatasourceCliArgs = DatasourceCliArgs::parse_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--csv",
    ]);
    let text_args: DatasourceCliArgs = DatasourceCliArgs::parse_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--text",
    ]);
    let json_args: DatasourceCliArgs = DatasourceCliArgs::parse_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--json",
    ]);
    let yaml_args: DatasourceCliArgs = DatasourceCliArgs::parse_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--yaml",
    ]);
    let table_alias_args: DatasourceCliArgs = DatasourceCliArgs::parse_normalized_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--output-format",
        "table",
    ]);
    let csv_alias_args: DatasourceCliArgs = DatasourceCliArgs::parse_normalized_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--output-format",
        "csv",
    ]);
    let text_alias_args: DatasourceCliArgs = DatasourceCliArgs::parse_normalized_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--output-format",
        "text",
    ]);
    let json_alias_args: DatasourceCliArgs = DatasourceCliArgs::parse_normalized_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--output-format",
        "json",
    ]);
    let yaml_alias_args: DatasourceCliArgs = DatasourceCliArgs::parse_normalized_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--output-format",
        "yaml",
    ]);

    match table_args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(inner.table);
            assert!(!inner.csv);
            assert!(!inner.text);
            assert!(!inner.json);
            assert!(!inner.yaml);
        }
        _ => panic!("expected datasource list"),
    }

    match csv_args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(!inner.table);
            assert!(inner.csv);
            assert!(!inner.text);
            assert!(!inner.json);
            assert!(!inner.yaml);
        }
        _ => panic!("expected datasource list"),
    }

    match text_args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(!inner.table);
            assert!(!inner.csv);
            assert!(inner.text);
            assert!(!inner.json);
            assert!(!inner.yaml);
        }
        _ => panic!("expected datasource list"),
    }

    match json_args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(!inner.table);
            assert!(!inner.csv);
            assert!(!inner.text);
            assert!(inner.json);
            assert!(!inner.yaml);
        }
        _ => panic!("expected datasource list"),
    }

    match yaml_args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(!inner.table);
            assert!(!inner.csv);
            assert!(!inner.text);
            assert!(!inner.json);
            assert!(inner.yaml);
        }
        _ => panic!("expected datasource list"),
    }

    match table_alias_args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(inner.table);
            assert_eq!(
                inner.output_format,
                Some(crate::datasource::ListOutputFormat::Table)
            );
        }
        _ => panic!("expected datasource list"),
    }

    match csv_alias_args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(inner.csv);
            assert_eq!(
                inner.output_format,
                Some(crate::datasource::ListOutputFormat::Csv)
            );
        }
        _ => panic!("expected datasource list"),
    }

    match text_alias_args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(inner.text);
            assert_eq!(
                inner.output_format,
                Some(crate::datasource::ListOutputFormat::Text)
            );
        }
        _ => panic!("expected datasource list"),
    }

    match json_alias_args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(inner.json);
            assert_eq!(
                inner.output_format,
                Some(crate::datasource::ListOutputFormat::Json)
            );
        }
        _ => panic!("expected datasource list"),
    }

    match yaml_alias_args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(inner.yaml);
            assert_eq!(
                inner.output_format,
                Some(crate::datasource::ListOutputFormat::Yaml)
            );
        }
        _ => panic!("expected datasource list"),
    }
}

#[test]
fn parse_datasource_list_supports_output_format_aliases() {
    let args: DatasourceCliArgs = DatasourceCliArgs::parse_normalized_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--output-format",
        "yaml",
    ]);

    match args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(inner.yaml);
            assert!(!inner.table);
            assert!(!inner.csv);
            assert!(!inner.text);
            assert!(!inner.json);
            assert_eq!(
                inner.output_format,
                Some(crate::datasource::ListOutputFormat::Yaml)
            );
        }
        _ => panic!("expected datasource list"),
    }
}

#[test]
fn parse_datasource_list_rejects_conflicting_output_flags() {
    let result = DatasourceCliArgs::try_parse_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--interactive",
        "--json",
    ]);

    assert!(result.is_err());
}

#[test]
fn export_help_explains_org_scope_flags() {
    let mut command = DatasourceCliArgs::command();
    let subcommand = command
        .find_subcommand_mut("export")
        .unwrap_or_else(|| panic!("missing datasource export help"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("--org-id"));
    assert!(help.contains("--all-orgs"));
    assert!(help.contains("--overwrite"));
    assert!(help.contains("--dry-run"));
    assert!(help.contains("Examples:"));
}
