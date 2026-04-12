use super::*;

fn assert_contains_all(rendered: &str, expected: &[&str]) {
    for needle in expected {
        assert!(
            rendered.contains(needle),
            "expected help to contain {needle:?}\n{rendered}"
        );
    }
}

#[test]
fn list_help_mentions_cross_org_basic_auth_examples() {
    let help = render_dashboard_subcommand_help("list");
    assert_contains_all(
        &help,
        &[
            "--all-orgs",
            "Prefer Basic auth when you need cross-org listing",
            "List dashboards across all visible orgs with Basic auth",
            "List dashboards from one explicit org ID",
            "List dashboards from the current org with an API token",
        ],
    );
}

#[test]
fn parse_cli_supports_list_csv_mode() {
    let args = parse_cli_from([
        "grafana-util",
        "list",
        "--url",
        "https://grafana.example.com",
        "--csv",
    ]);

    match args.command {
        DashboardCommand::List(list_args) => {
            assert_eq!(list_args.org_id, None);
            assert!(!list_args.all_orgs);
            assert!(!list_args.table);
            assert!(list_args.csv);
            assert!(!list_args.json);
        }
        _ => panic!("expected list command"),
    }
}

#[test]
fn parse_cli_supports_export_progress_and_verbose_flags() {
    let args = parse_cli_from(["grafana-util", "export", "--progress", "--verbose"]);

    match args.command {
        DashboardCommand::Export(export_args) => {
            assert!(export_args.progress);
            assert!(export_args.verbose);
        }
        _ => panic!("expected export command"),
    }
}

#[test]
fn parse_cli_supports_import_progress_and_verbose_flags() {
    let args = parse_cli_from([
        "grafana-util",
        "import",
        "--input-dir",
        "./dashboards/raw",
        "--progress",
        "-v",
    ]);

    match args.command {
        DashboardCommand::Import(import_args) => {
            assert!(import_args.progress);
            assert!(import_args.verbose);
        }
        _ => panic!("expected import command"),
    }
}

#[test]
fn parse_cli_supports_import_interactive_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "import",
        "--input-dir",
        "./dashboards/raw",
        "--interactive",
    ]);

    match args.command {
        DashboardCommand::Import(import_args) => {
            assert!(import_args.interactive);
        }
        _ => panic!("expected import command"),
    }
}

#[test]
fn parse_cli_supports_provisioning_import_format() {
    let args = parse_cli_from([
        "grafana-util",
        "import",
        "--input-dir",
        "./dashboards/provisioning",
        "--input-format",
        "provisioning",
    ]);

    match args.command {
        DashboardCommand::Import(import_args) => {
            assert_eq!(
                import_args.input_format,
                DashboardImportInputFormat::Provisioning
            );
        }
        _ => panic!("expected import command"),
    }
}

#[test]
fn import_help_mentions_interactive_review_picker() {
    let mut help = Vec::new();
    let mut command = DashboardCliArgs::command();
    command
        .find_subcommand_mut("import")
        .unwrap()
        .write_long_help(&mut help)
        .unwrap();
    let rendered = String::from_utf8(help).unwrap();

    assert_contains_all(
        &rendered,
        &[
            "interactive review picker",
            "create/update/skip action",
            "With --dry-run",
            "--input-format",
            "raw export files or Grafana file-provisioning artifacts",
        ],
    );
}

#[test]
fn parse_cli_supports_provisioning_diff_input_format() {
    let args = parse_cli_from([
        "grafana-util",
        "diff",
        "--input-dir",
        "./dashboards/provisioning",
        "--input-format",
        "provisioning",
    ]);

    match args.command {
        DashboardCommand::Diff(diff_args) => {
            assert_eq!(
                diff_args.input_format,
                DashboardImportInputFormat::Provisioning
            );
            assert_eq!(
                diff_args.input_dir,
                PathBuf::from("./dashboards/provisioning")
            );
        }
        _ => panic!("expected diff command"),
    }
}

#[test]
fn parse_cli_supports_patch_file_command() {
    let args = parse_cli_from([
        "grafana-util",
        "patch",
        "--input",
        "./drafts/cpu-main.json",
        "--output",
        "./drafts/cpu-main-patched.json",
        "--name",
        "CPU Overview",
        "--uid",
        "cpu-main",
        "--folder-uid",
        "infra",
        "--message",
        "Promote CPU dashboard",
        "--tag",
        "prod",
        "--tag",
        "sre",
    ]);

    match args.command {
        DashboardCommand::PatchFile(patch_args) => {
            assert_eq!(patch_args.input, PathBuf::from("./drafts/cpu-main.json"));
            assert_eq!(
                patch_args.output,
                Some(PathBuf::from("./drafts/cpu-main-patched.json"))
            );
            assert_eq!(patch_args.name.as_deref(), Some("CPU Overview"));
            assert_eq!(patch_args.uid.as_deref(), Some("cpu-main"));
            assert_eq!(patch_args.folder_uid.as_deref(), Some("infra"));
            assert_eq!(patch_args.message.as_deref(), Some("Promote CPU dashboard"));
            assert_eq!(patch_args.tags, vec!["prod", "sre"]);
        }
        _ => panic!("expected patch command"),
    }
}

#[test]
fn parse_cli_supports_patch_file_stdin_input() {
    let args = parse_cli_from([
        "grafana-util",
        "patch",
        "--input",
        "-",
        "--output",
        "./drafts/cpu-main.json",
    ]);

    match args.command {
        DashboardCommand::PatchFile(patch_args) => {
            assert_eq!(patch_args.input, PathBuf::from("-"));
            assert_eq!(
                patch_args.output,
                Some(PathBuf::from("./drafts/cpu-main.json"))
            );
        }
        _ => panic!("expected patch command"),
    }
}

#[test]
fn parse_cli_supports_publish_command() {
    let args = parse_cli_from([
        "grafana-util",
        "publish",
        "--url",
        "https://grafana.example.com",
        "--basic-user",
        "admin",
        "--basic-password",
        "admin",
        "--input",
        "./drafts/cpu-main.json",
        "--folder-uid",
        "infra",
        "--message",
        "Promote CPU dashboard",
        "--dry-run",
        "--table",
    ]);

    match args.command {
        DashboardCommand::Publish(publish_args) => {
            assert_eq!(publish_args.common.url, "https://grafana.example.com");
            assert_eq!(publish_args.common.username.as_deref(), Some("admin"));
            assert_eq!(publish_args.common.password.as_deref(), Some("admin"));
            assert_eq!(publish_args.input, PathBuf::from("./drafts/cpu-main.json"));
            assert_eq!(publish_args.folder_uid.as_deref(), Some("infra"));
            assert_eq!(publish_args.message, "Promote CPU dashboard");
            assert!(publish_args.dry_run);
            assert!(!publish_args.watch);
            assert!(publish_args.table);
            assert!(!publish_args.json);
        }
        _ => panic!("expected publish command"),
    }
}

#[test]
fn parse_cli_supports_publish_watch_and_stdin_input() {
    let watch_args = parse_cli_from([
        "grafana-util",
        "publish",
        "--url",
        "https://grafana.example.com",
        "--input",
        "./drafts/cpu-main.json",
        "--watch",
    ]);
    let stdin_args = parse_cli_from([
        "grafana-util",
        "publish",
        "--url",
        "https://grafana.example.com",
        "--input",
        "-",
    ]);

    match watch_args.command {
        DashboardCommand::Publish(publish_args) => {
            assert_eq!(publish_args.input, PathBuf::from("./drafts/cpu-main.json"));
            assert!(publish_args.watch);
        }
        _ => panic!("expected publish command"),
    }

    match stdin_args.command {
        DashboardCommand::Publish(publish_args) => {
            assert_eq!(publish_args.input, PathBuf::from("-"));
            assert!(!publish_args.watch);
        }
        _ => panic!("expected publish command"),
    }
}

#[test]
fn diff_help_mentions_provisioning_input_format() {
    let help = render_dashboard_subcommand_help("diff");
    assert_contains_all(
        &help,
        &[
            "--input-format",
            "Grafana file-provisioning artifacts",
            "provisioning/ root or its dashboards/ subdirectory",
            "Compare a provisioning export root against the current org",
        ],
    );
}

#[test]
fn patch_file_help_mentions_in_place_and_output_paths() {
    let help = render_dashboard_subcommand_help("patch");
    assert_contains_all(
        &help,
        &[
            "--input",
            "--output",
            "--name",
            "--uid",
            "--folder-uid",
            "--message",
            "--tag",
            "Patch a raw export file in place",
            "Patch one draft file into a new output path",
            "Patch one dashboard from standard input into an explicit output file",
        ],
    );
}

#[test]
fn parse_cli_supports_review_command() {
    let args = parse_cli_from([
        "grafana-util",
        "review",
        "--input",
        "./drafts/cpu-main.json",
        "--output-format",
        "yaml",
    ]);

    match args.command {
        DashboardCommand::Review(review_args) => {
            assert_eq!(review_args.input, PathBuf::from("./drafts/cpu-main.json"));
            assert_eq!(review_args.output_format, Some(SimpleOutputFormat::Yaml));
        }
        _ => panic!("expected review command"),
    }
}

#[test]
fn parse_cli_supports_review_output_format_variants() {
    for (flag, expected) in [
        ("text", SimpleOutputFormat::Text),
        ("table", SimpleOutputFormat::Table),
        ("csv", SimpleOutputFormat::Csv),
        ("json", SimpleOutputFormat::Json),
        ("yaml", SimpleOutputFormat::Yaml),
    ] {
        let args = parse_cli_from([
            "grafana-util",
            "review",
            "--input",
            "./drafts/cpu-main.json",
            "--output-format",
            flag,
        ]);

        match args.command {
            DashboardCommand::Review(review_args) => {
                assert_eq!(review_args.output_format, Some(expected));
            }
            _ => panic!("expected review command"),
        }
    }
}

#[test]
fn review_help_mentions_local_file_only_output_modes() {
    let help = render_dashboard_subcommand_help("review");
    assert_contains_all(
        &help,
        &[
            "--input",
            "--output-format",
            "text",
            "table",
            "csv",
            "json",
            "yaml",
            "Review one local dashboard JSON file without touching Grafana.",
            "grafana-util dashboard review",
            "standard input",
            "Review one generated dashboard from standard input",
        ],
    );
}

#[test]
fn publish_help_mentions_dry_run_preview() {
    let help = render_dashboard_subcommand_help("publish");
    assert_contains_all(
        &help,
        &[
            "--input",
            "--folder-uid",
            "--message",
            "--dry-run",
            "--watch",
            "--table",
            "Publish one draft file to the current Grafana org",
            "Preview the same publish without writing to Grafana",
            "Publish one generated dashboard from standard input",
            "Watch one local draft file and rerun dry-run after each save",
        ],
    );
}

#[test]
fn parse_cli_supports_dashboard_serve_command() {
    let args = parse_cli_from([
        "grafana-util",
        "serve",
        "--script",
        "jsonnet dashboards/cpu.jsonnet",
        "--watch",
        "./dashboards",
        "--port",
        "18080",
        "--open-browser",
    ]);

    match args.command {
        DashboardCommand::Serve(serve_args) => {
            assert_eq!(
                serve_args.script.as_deref(),
                Some("jsonnet dashboards/cpu.jsonnet")
            );
            assert_eq!(serve_args.port, 18080);
            assert_eq!(serve_args.watch, vec![PathBuf::from("./dashboards")]);
            assert!(serve_args.input.is_none());
            assert!(serve_args.open_browser);
        }
        _ => panic!("expected serve command"),
    }
}

#[test]
fn serve_help_mentions_local_preview_server() {
    let help = render_dashboard_subcommand_help("serve");
    assert_contains_all(
        &help,
        &[
            "--input",
            "--script",
            "--watch",
            "--open-browser",
            "--port",
            "local preview server",
        ],
    );
}

#[test]
fn parse_cli_supports_dashboard_edit_live_command() {
    let args = parse_cli_from([
        "grafana-util",
        "edit-live",
        "--url",
        "https://grafana.example.com",
        "--dashboard-uid",
        "cpu-main",
        "--output",
        "./drafts/cpu-main.edited.json",
    ]);

    match args.command {
        DashboardCommand::EditLive(edit_args) => {
            assert_eq!(edit_args.common.url, "https://grafana.example.com");
            assert_eq!(edit_args.dashboard_uid, "cpu-main");
            assert_eq!(
                edit_args.output,
                Some(PathBuf::from("./drafts/cpu-main.edited.json"))
            );
            assert!(!edit_args.apply_live);
            assert!(!edit_args.publish_dry_run);
        }
        _ => panic!("expected edit-live command"),
    }
}

#[test]
fn parse_cli_supports_dashboard_edit_live_publish_dry_run() {
    let args = parse_cli_from([
        "grafana-util",
        "edit-live",
        "--url",
        "https://grafana.example.com",
        "--dashboard-uid",
        "cpu-main",
        "--output",
        "./drafts/cpu-main.edited.json",
        "--publish-dry-run",
    ]);

    match args.command {
        DashboardCommand::EditLive(edit_args) => {
            assert_eq!(edit_args.dashboard_uid, "cpu-main");
            assert_eq!(
                edit_args.output,
                Some(PathBuf::from("./drafts/cpu-main.edited.json"))
            );
            assert!(edit_args.publish_dry_run);
            assert!(!edit_args.apply_live);
        }
        _ => panic!("expected edit-live command"),
    }
}

#[test]
fn edit_live_help_mentions_safe_local_draft_default() {
    let help = render_dashboard_subcommand_help("edit-live");
    assert_contains_all(
        &help,
        &[
            "--dashboard-uid",
            "--output",
            "--apply-live",
            "--publish-dry-run",
            "preview",
            "review output",
        ],
    );
}

#[test]
fn parse_cli_supports_import_dry_run_json_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "import",
        "--input-dir",
        "./dashboards/raw",
        "--dry-run",
        "--json",
    ]);

    match args.command {
        DashboardCommand::Import(import_args) => {
            assert!(import_args.dry_run);
            assert!(import_args.json);
        }
        _ => panic!("expected import command"),
    }
}

#[test]
fn parse_cli_supports_import_dry_run_output_format_table() {
    let args = parse_cli_from([
        "grafana-util",
        "import",
        "--input-dir",
        "./dashboards/raw",
        "--dry-run",
        "--output-format",
        "table",
    ]);

    match args.command {
        DashboardCommand::Import(import_args) => {
            assert!(import_args.dry_run);
            assert!(import_args.table);
            assert!(!import_args.json);
        }
        _ => panic!("expected import command"),
    }
}

#[test]
fn parse_cli_supports_browse_with_path() {
    let args = parse_cli_from([
        "grafana-util",
        "browse",
        "--url",
        "https://grafana.example.com",
        "--org-id",
        "2",
        "--path",
        "Platform / Infra",
    ]);

    match args.command {
        DashboardCommand::Browse(browse_args) => {
            assert_eq!(browse_args.common.url, "https://grafana.example.com");
            assert_eq!(browse_args.org_id, Some(2));
            assert!(!browse_args.all_orgs);
            assert_eq!(browse_args.path.as_deref(), Some("Platform / Infra"));
            assert_eq!(browse_args.page_size, 500);
        }
        _ => panic!("expected browse command"),
    }
}

#[test]
fn parse_cli_supports_browse_local_export_tree() {
    let args = parse_cli_from([
        "grafana-util",
        "browse",
        "--input-dir",
        "./dashboards/raw",
        "--input-format",
        "raw",
        "--path",
        "Platform / Infra",
    ]);

    match args.command {
        DashboardCommand::Browse(browse_args) => {
            assert_eq!(
                browse_args.input_dir,
                Some(PathBuf::from("./dashboards/raw"))
            );
            assert_eq!(browse_args.input_format, DashboardImportInputFormat::Raw);
            assert_eq!(browse_args.path.as_deref(), Some("Platform / Infra"));
        }
        _ => panic!("expected browse command"),
    }
}

#[test]
fn parse_cli_supports_browse_workspace_root() {
    let args = parse_cli_from([
        "grafana-util",
        "browse",
        "--workspace",
        "./grafana-oac-repo",
        "--input-format",
        "provisioning",
        "--path",
        "Platform / Infra",
    ]);

    match args.command {
        DashboardCommand::Browse(browse_args) => {
            assert_eq!(
                browse_args.workspace,
                Some(PathBuf::from("./grafana-oac-repo"))
            );
            assert_eq!(browse_args.input_dir, None);
            assert_eq!(
                browse_args.input_format,
                DashboardImportInputFormat::Provisioning
            );
            assert_eq!(browse_args.path.as_deref(), Some("Platform / Infra"));
        }
        _ => panic!("expected browse command"),
    }
}
