//! Rust regression coverage for Dashboard behavior at this module boundary.

use super::test_support::{
    build_dashboard_capture_url, infer_screenshot_output_format, parse_cli_from,
    resolve_manifest_title, validate_screenshot_args, DashboardCliArgs, DashboardCommand,
    ScreenshotFullPageOutput, ScreenshotOutputFormat, ScreenshotTheme,
};
use crate::common::GrafanaCliError;
use clap::{CommandFactory, Parser};
use std::path::{Path, PathBuf};

fn render_dashboard_help() -> String {
    let mut command = DashboardCliArgs::command();
    let mut output = Vec::new();
    command.write_long_help(&mut output).unwrap();
    String::from_utf8(output).unwrap()
}

fn render_dashboard_subcommand_help(name: &str) -> String {
    let mut command = DashboardCliArgs::command();
    let subcommand = command
        .find_subcommand_mut(name)
        .unwrap_or_else(|| panic!("missing subcommand {name}"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    String::from_utf8(output).unwrap()
}

#[test]
fn parse_cli_supports_screenshot_mode() {
    let args = parse_cli_from([
        "grafana-util",
        "screenshot",
        "--url",
        "https://grafana.example.com",
        "--dashboard-uid",
        "cpu-main",
        "--slug",
        "cpu-overview",
        "--output",
        "./cpu-main.pdf",
        "--panel-id",
        "7",
        "--org-id",
        "3",
        "--from",
        "now-6h",
        "--to",
        "now",
        "--var",
        "env=prod",
        "--var",
        "region=us-east-1",
        "--theme",
        "light",
        "--output-format",
        "pdf",
        "--width",
        "1600",
        "--height",
        "900",
        "--device-scale-factor",
        "2",
        "--full-page",
        "--full-page-output",
        "manifest",
        "--wait-ms",
        "9000",
        "--browser-path",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "--header-title",
        "--header-url",
        "https://grafana.example.com/rendered/cpu-main",
        "--header-captured-at",
        "--header-text",
        "Nightly capture",
        "--prompt-token",
    ]);

    match args.command {
        DashboardCommand::Screenshot(screenshot_args) => {
            assert_eq!(screenshot_args.common.url, "https://grafana.example.com");
            assert!(screenshot_args.common.prompt_token);
            assert_eq!(screenshot_args.dashboard_uid.as_deref(), Some("cpu-main"));
            assert_eq!(screenshot_args.slug.as_deref(), Some("cpu-overview"));
            assert_eq!(screenshot_args.output, PathBuf::from("./cpu-main.pdf"));
            assert_eq!(screenshot_args.panel_id, Some(7));
            assert_eq!(screenshot_args.org_id, Some(3));
            assert_eq!(screenshot_args.from.as_deref(), Some("now-6h"));
            assert_eq!(screenshot_args.to.as_deref(), Some("now"));
            assert_eq!(screenshot_args.vars_query, None);
            assert!(!screenshot_args.print_capture_url);
            assert_eq!(screenshot_args.header_title.as_deref(), Some("__auto__"));
            assert_eq!(
                screenshot_args.header_url.as_deref(),
                Some("https://grafana.example.com/rendered/cpu-main")
            );
            assert!(screenshot_args.header_captured_at);
            assert_eq!(
                screenshot_args.header_text.as_deref(),
                Some("Nightly capture")
            );
            assert_eq!(
                screenshot_args.vars,
                vec!["env=prod".to_string(), "region=us-east-1".to_string()]
            );
            assert_eq!(screenshot_args.theme, ScreenshotTheme::Light);
            assert_eq!(
                screenshot_args.output_format,
                Some(ScreenshotOutputFormat::Pdf)
            );
            assert_eq!(screenshot_args.width, 1600);
            assert_eq!(screenshot_args.height, 900);
            assert_eq!(screenshot_args.device_scale_factor, 2.0);
            assert!(screenshot_args.full_page);
            assert_eq!(
                screenshot_args.full_page_output,
                ScreenshotFullPageOutput::Manifest
            );
            assert_eq!(screenshot_args.wait_ms, 9000);
            assert_eq!(
                screenshot_args.browser_path,
                Some(PathBuf::from(
                    "/Applications/Chromium.app/Contents/MacOS/Chromium"
                ))
            );
        }
        other => panic!("expected screenshot args, got {other:?}"),
    }
}

#[test]
fn parse_cli_screenshot_defaults_match_browser_capture_defaults() {
    let args = parse_cli_from([
        "grafana-util",
        "screenshot",
        "--dashboard-uid",
        "cpu-main",
        "--output",
        "./cpu-main.png",
        "--token",
        "secret",
    ]);

    match args.command {
        DashboardCommand::Screenshot(screenshot_args) => {
            assert_eq!(screenshot_args.slug, None);
            assert_eq!(screenshot_args.panel_id, None);
            assert_eq!(screenshot_args.org_id, None);
            assert_eq!(screenshot_args.from, None);
            assert_eq!(screenshot_args.to, None);
            assert!(screenshot_args.vars.is_empty());
            assert_eq!(screenshot_args.theme, ScreenshotTheme::Dark);
            assert_eq!(screenshot_args.output_format, None);
            assert_eq!(screenshot_args.width, 1440);
            assert_eq!(screenshot_args.height, 1024);
            assert_eq!(screenshot_args.device_scale_factor, 1.0);
            assert!(!screenshot_args.full_page);
            assert_eq!(
                screenshot_args.full_page_output,
                ScreenshotFullPageOutput::Single
            );
            assert_eq!(screenshot_args.wait_ms, 5000);
            assert_eq!(screenshot_args.browser_path, None);
            assert_eq!(screenshot_args.header_title, None);
            assert_eq!(screenshot_args.header_url, None);
            assert!(!screenshot_args.header_captured_at);
            assert_eq!(screenshot_args.header_text, None);
        }
        other => panic!("expected screenshot args, got {other:?}"),
    }
}

#[test]
fn screenshot_help_mentions_capture_options() {
    let root_help = render_dashboard_help();
    assert!(root_help.contains("screenshot"));
    assert!(root_help.contains("inspect-vars"));

    let help = render_dashboard_subcommand_help("screenshot");
    assert!(help.contains("--dashboard-uid"));
    assert!(help.contains("--dashboard-url"));
    assert!(help.contains("--output"));
    assert!(help.contains("--panel-id"));
    assert!(help.contains("--vars-query"));
    assert!(help.contains("--print-capture-url"));
    assert!(help.contains("--header-title"));
    assert!(help.contains("--header-url"));
    assert!(help.contains("--header-captured-at"));
    assert!(help.contains("--header-text"));
    assert!(help.contains("--var"));
    assert!(help.contains("--browser-path"));
    assert!(help.contains("--device-scale-factor"));
    assert!(help.contains("--full-page-output"));
    assert!(help.contains("Target Options"));
    assert!(help.contains("State Options"));
    assert!(help.contains("Rendering Options"));
    assert!(help.contains("Header Options"));
    assert!(help.contains("Capture a full dashboard from a browser URL"));
    assert!(help.contains("Capture a solo panel with a vars-query fragment"));
}

#[test]
fn screenshot_parser_requires_dashboard_uid_or_dashboard_url() {
    let error = DashboardCliArgs::try_parse_from([
        "grafana-util",
        "screenshot",
        "--output",
        "./cpu-main.png",
        "--token",
        "secret",
    ])
    .unwrap_err()
    .to_string();

    assert!(error.contains("--dashboard-uid"));
    assert!(error.contains("--dashboard-url"));
}

#[test]
fn build_dashboard_capture_url_includes_panel_time_theme_and_vars() {
    let args = match parse_cli_from([
        "grafana-util",
        "screenshot",
        "--url",
        "https://grafana.example.com/root",
        "--dashboard-uid",
        "cpu-main",
        "--slug",
        "cpu-overview",
        "--output",
        "./cpu-main.png",
        "--panel-id",
        "4",
        "--org-id",
        "7",
        "--from",
        "now-6h",
        "--to",
        "now",
        "--var",
        "env=prod",
        "--var",
        "region=us-east-1",
        "--theme",
        "dark",
        "--token",
        "secret",
    ])
    .command
    {
        DashboardCommand::Screenshot(args) => args,
        other => panic!("expected screenshot args, got {other:?}"),
    };

    let url = build_dashboard_capture_url(&args).unwrap();
    assert!(url.starts_with("https://grafana.example.com/d-solo/cpu-main/cpu-overview?"));
    assert!(url.contains("panelId=4"));
    assert!(url.contains("viewPanel=4"));
    assert!(url.contains("orgId=7"));
    assert!(url.contains("from=now-6h"));
    assert!(url.contains("to=now"));
    assert!(url.contains("theme=dark"));
    assert!(url.contains("kiosk=tv"));
    assert!(url.contains("var-env=prod"));
    assert!(url.contains("var-region=us-east-1"));
}

#[test]
fn build_dashboard_capture_url_supports_datasource_style_template_variables() {
    let args = match parse_cli_from([
        "grafana-util",
        "screenshot",
        "--url",
        "https://grafana.example.com",
        "--dashboard-uid",
        "infra-main",
        "--output",
        "./infra-main.png",
        "--var",
        "datasource=prom-main",
        "--var",
        "cluster=prod-a",
        "--token",
        "secret",
    ])
    .command
    {
        DashboardCommand::Screenshot(args) => args,
        other => panic!("expected screenshot args, got {other:?}"),
    };

    let url = build_dashboard_capture_url(&args).unwrap();
    assert!(url.contains("theme=dark"));
    assert!(url.contains("var-datasource=prom-main"));
    assert!(url.contains("var-cluster=prod-a"));
}

#[test]
fn build_dashboard_capture_url_reuses_full_dashboard_url_state() {
    let args = match parse_cli_from([
        "grafana-util",
        "screenshot",
        "--dashboard-url",
        "https://grafana.example.com/d/infra-main/infra-overview?orgId=9&from=now-12h&to=now&var-datasource=prom-main&var-cluster=prod-a",
        "--output",
        "./infra-main.png",
        "--var",
        "cluster=prod-b",
        "--token",
        "secret",
    ])
    .command
    {
        DashboardCommand::Screenshot(args) => args,
        other => panic!("expected screenshot args, got {other:?}"),
    };

    let url = build_dashboard_capture_url(&args).unwrap();
    assert!(url.starts_with("https://grafana.example.com/d/infra-main/infra-overview?"));
    assert!(url.contains("orgId=9"));
    assert!(url.contains("from=now-12h"));
    assert!(url.contains("to=now"));
    assert!(url.contains("var-datasource=prom-main"));
    assert!(url.contains("var-cluster=prod-b"));
}

#[test]
fn build_dashboard_capture_url_rejects_invalid_dashboard_url_as_url_error() {
    let args = match parse_cli_from([
        "grafana-util",
        "screenshot",
        "--url",
        "https://grafana.example.com",
        "--dashboard-uid",
        "cpu-main",
        "--dashboard-url",
        "not a url",
        "--output",
        "./cpu-main.png",
        "--token",
        "secret",
    ])
    .command
    {
        DashboardCommand::Screenshot(args) => args,
        other => panic!("expected screenshot args, got {other:?}"),
    };

    let error = build_dashboard_capture_url(&args).unwrap_err();
    assert!(matches!(error, GrafanaCliError::Url { .. }));
    assert_eq!(error.kind(), "url");
    assert!(error
        .to_string()
        .contains("Invalid URL for --dashboard-url"));
}

#[test]
fn build_dashboard_capture_url_merges_vars_query_between_url_and_explicit_vars() {
    let args = match parse_cli_from([
        "grafana-util",
        "screenshot",
        "--dashboard-url",
        "https://grafana.example.com/d/infra-main/infra-overview?var-env=prod&var-cluster=old",
        "--vars-query",
        "var-cluster=mid&var-host=web01",
        "--var",
        "host=web02",
        "--output",
        "./infra-main.png",
        "--token",
        "secret",
    ])
    .command
    {
        DashboardCommand::Screenshot(args) => args,
        other => panic!("expected screenshot args, got {other:?}"),
    };

    let url = build_dashboard_capture_url(&args).unwrap();
    assert!(url.contains("var-env=prod"));
    assert!(url.contains("var-cluster=mid"));
    assert!(url.contains("var-host=web02"));
}

#[test]
fn build_dashboard_capture_url_preserves_non_var_query_from_vars_query() {
    let args = match parse_cli_from([
        "grafana-util",
        "screenshot",
        "--url",
        "https://grafana.example.com",
        "--dashboard-uid",
        "infra-main",
        "--vars-query",
        "var-job=node-exporter&refresh=1m&showCategory=Panel%20links&timezone=browser",
        "--output",
        "./infra-main.png",
        "--token",
        "secret",
    ])
    .command
    {
        DashboardCommand::Screenshot(args) => args,
        other => panic!("expected screenshot args, got {other:?}"),
    };

    let url = build_dashboard_capture_url(&args).unwrap();
    assert!(url.contains("var-job=node-exporter"));
    assert!(url.contains("refresh=1m"));
    assert!(url.contains("showCategory=Panel+links") || url.contains("showCategory=Panel%20links"));
    assert!(url.contains("timezone=browser"));
}

#[test]
fn parse_screenshot_args_accepts_print_capture_url() {
    let args = match parse_cli_from([
        "grafana-util",
        "screenshot",
        "--url",
        "https://grafana.example.com",
        "--dashboard-uid",
        "infra-main",
        "--print-capture-url",
        "--output",
        "./infra-main.png",
        "--token",
        "secret",
    ])
    .command
    {
        DashboardCommand::Screenshot(args) => args,
        other => panic!("expected screenshot args, got {other:?}"),
    };

    assert!(args.print_capture_url);
}

#[test]
fn parse_screenshot_args_supports_auto_header_url_and_title_flags() {
    let args = match parse_cli_from([
        "grafana-util",
        "screenshot",
        "--dashboard-uid",
        "infra-main",
        "--header-title",
        "--header-url",
        "--output",
        "./infra-main.png",
        "--token",
        "secret",
    ])
    .command
    {
        DashboardCommand::Screenshot(args) => args,
        other => panic!("expected screenshot args, got {other:?}"),
    };

    assert_eq!(args.header_title.as_deref(), Some("__auto__"));
    assert_eq!(args.header_url.as_deref(), Some("__auto__"));
}

#[test]
fn infer_screenshot_output_format_uses_extension_and_explicit_override() {
    assert_eq!(
        infer_screenshot_output_format(Path::new("/tmp/cpu-main.jpeg"), None).unwrap(),
        ScreenshotOutputFormat::Jpeg
    );
    assert_eq!(
        infer_screenshot_output_format(
            Path::new("/tmp/cpu-main.anything"),
            Some(ScreenshotOutputFormat::Pdf)
        )
        .unwrap(),
        ScreenshotOutputFormat::Pdf
    );
}

#[test]
fn validate_screenshot_args_rejects_invalid_var_assignment() {
    let args = match parse_cli_from([
        "grafana-util",
        "screenshot",
        "--dashboard-uid",
        "cpu-main",
        "--output",
        "./cpu-main.png",
        "--var",
        "env",
        "--token",
        "secret",
    ])
    .command
    {
        DashboardCommand::Screenshot(args) => args,
        other => panic!("expected screenshot args, got {other:?}"),
    };

    let error = validate_screenshot_args(&args).unwrap_err().to_string();
    assert!(error.contains("Invalid --var value 'env'"));
}

#[test]
fn validate_screenshot_args_rejects_split_output_without_full_page() {
    let args = match parse_cli_from([
        "grafana-util",
        "screenshot",
        "--dashboard-uid",
        "cpu-main",
        "--output",
        "./cpu-main.png",
        "--full-page-output",
        "tiles",
        "--token",
        "secret",
    ])
    .command
    {
        DashboardCommand::Screenshot(args) => args,
        other => panic!("expected screenshot args, got {other:?}"),
    };

    let error = validate_screenshot_args(&args).unwrap_err().to_string();
    assert!(error.contains("--full-page-output tiles or manifest requires --full-page"));
}

#[test]
fn validate_screenshot_args_rejects_invalid_device_scale_factor() {
    let args = match parse_cli_from([
        "grafana-util",
        "screenshot",
        "--dashboard-uid",
        "cpu-main",
        "--output",
        "./cpu-main.png",
        "--device-scale-factor",
        "0",
        "--token",
        "secret",
    ])
    .command
    {
        DashboardCommand::Screenshot(args) => args,
        other => panic!("expected screenshot args, got {other:?}"),
    };

    let error = validate_screenshot_args(&args).unwrap_err().to_string();
    assert!(error.contains("--device-scale-factor must be greater than 0"));
}

#[test]
fn validate_screenshot_args_rejects_pdf_split_output() {
    let args = match parse_cli_from([
        "grafana-util",
        "screenshot",
        "--dashboard-uid",
        "cpu-main",
        "--output",
        "./cpu-main.pdf",
        "--full-page",
        "--full-page-output",
        "manifest",
        "--token",
        "secret",
    ])
    .command
    {
        DashboardCommand::Screenshot(args) => args,
        other => panic!("expected screenshot args, got {other:?}"),
    };

    let error = validate_screenshot_args(&args).unwrap_err().to_string();
    assert!(error.contains("PDF output does not support --full-page-output tiles or manifest"));
}

#[test]
fn resolve_manifest_title_prefers_panel_then_dashboard_then_uid_then_output_stem() {
    let args = match parse_cli_from([
        "grafana-util",
        "screenshot",
        "--dashboard-uid",
        "cpu-main",
        "--output",
        "./capture-name.png",
        "--token",
        "secret",
    ])
    .command
    {
        DashboardCommand::Screenshot(args) => args,
        other => panic!("expected screenshot args, got {other:?}"),
    };

    assert_eq!(
        resolve_manifest_title(
            Some("cpu-main"),
            Some("CPU Overview"),
            Some("CPU Busy"),
            &args
        ),
        Some("CPU Busy".to_string())
    );

    assert_eq!(
        resolve_manifest_title(Some("cpu-main"), Some("CPU Overview"), None, &args),
        Some("CPU Overview".to_string())
    );

    assert_eq!(
        resolve_manifest_title(Some("cpu-main"), None, None, &args),
        Some("cpu-main".to_string())
    );

    assert_eq!(
        resolve_manifest_title(None, None, None, &args),
        Some("capture-name".to_string())
    );
}

#[cfg(not(feature = "browser"))]
#[test]
fn capture_dashboard_screenshot_reports_missing_browser_support() {
    let args = match parse_cli_from([
        "grafana-util",
        "screenshot",
        "--dashboard-uid",
        "cpu-main",
        "--output",
        "./cpu-main.png",
        "--token",
        "secret",
    ])
    .command
    {
        DashboardCommand::Screenshot(args) => args,
        other => panic!("expected screenshot args, got {other:?}"),
    };

    let error = crate::dashboard::capture_dashboard_screenshot(&args)
        .unwrap_err()
        .to_string();
    assert!(error.contains("Dashboard screenshot support was not built in"));
    assert!(error.contains("browser"));
}
