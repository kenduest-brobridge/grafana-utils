//! Generic read-only resource queries for Grafana objects.
//!
//! This surface intentionally stays narrower than the higher-level workflow
//! namespaces. It exists so operators can inspect a few live Grafana resource
//! kinds before richer domain-specific flows exist.

#[path = "help.rs"]
mod resource_help;

use clap::{Args, Parser, Subcommand, ValueEnum};
use reqwest::Method;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::common::{
    message, render_json_value, set_json_color_choice, string_field, CliColorChoice, Result,
};
use crate::dashboard::{CommonCliArgs, DEFAULT_TIMEOUT, DEFAULT_URL};
use crate::grafana_api::{
    expect_object, expect_object_list, AuthInputs, GrafanaApiClient, GrafanaConnection,
};
use crate::profile_config::ConnectionMergeInput;
use crate::tabular_output::{print_lines, render_summary_table, render_table, render_yaml};
use resource_help::{
    RESOURCE_DESCRIBE_AFTER_HELP, RESOURCE_GET_AFTER_HELP, RESOURCE_KINDS_AFTER_HELP,
    RESOURCE_LIST_AFTER_HELP,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ResourceOutputFormat {
    Text,
    Table,
    Json,
    Yaml,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize)]
pub enum ResourceKind {
    Dashboards,
    Folders,
    Datasources,
    #[value(name = "alert-rules")]
    AlertRules,
    Orgs,
}

impl ResourceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Dashboards => "dashboards",
            Self::Folders => "folders",
            Self::Datasources => "datasources",
            Self::AlertRules => "alert-rules",
            Self::Orgs => "orgs",
        }
    }

    fn singular_label(self) -> &'static str {
        match self {
            Self::Dashboards => "dashboard",
            Self::Folders => "folder",
            Self::Datasources => "datasource",
            Self::AlertRules => "alert-rule",
            Self::Orgs => "org",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Dashboards => "Grafana dashboards from /api/search and /api/dashboards/uid/{uid}.",
            Self::Folders => "Grafana folders from /api/folders and /api/folders/{uid}.",
            Self::Datasources => "Grafana datasources from /api/datasources and /api/datasources/uid/{uid}.",
            Self::AlertRules => {
                "Grafana alert rules from /api/v1/provisioning/alert-rules and /api/v1/provisioning/alert-rules/{uid}."
            }
            Self::Orgs => "Grafana org inventory from /api/orgs and /api/orgs/{id}.",
        }
    }

    fn selector_pattern(self) -> &'static str {
        match self {
            Self::Dashboards => "dashboards/<uid>",
            Self::Folders => "folders/<uid>",
            Self::Datasources => "datasources/<uid>",
            Self::AlertRules => "alert-rules/<uid>",
            Self::Orgs => "orgs/<id>",
        }
    }

    fn list_endpoint(self) -> &'static str {
        match self {
            Self::Dashboards => "GET /api/search",
            Self::Folders => "GET /api/folders",
            Self::Datasources => "GET /api/datasources",
            Self::AlertRules => "GET /api/v1/provisioning/alert-rules",
            Self::Orgs => "GET /api/orgs",
        }
    }

    fn get_endpoint(self) -> &'static str {
        match self {
            Self::Dashboards => "GET /api/dashboards/uid/{uid}",
            Self::Folders => "GET /api/folders/{uid}",
            Self::Datasources => "GET /api/datasources/uid/{uid}",
            Self::AlertRules => "GET /api/v1/provisioning/alert-rules/{uid}",
            Self::Orgs => "GET /api/orgs/{id}",
        }
    }
}

#[derive(Debug, Clone, Args)]
pub struct ResourceKindsArgs {
    #[arg(
        long,
        value_enum,
        default_value_t = ResourceOutputFormat::Table,
        help = "Render supported resource kinds as text, table, json, or yaml."
    )]
    pub output_format: ResourceOutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct ResourceDescribeArgs {
    #[arg(
        value_enum,
        help = "Optional resource kind to describe. Omit this to describe every supported kind."
    )]
    pub kind: Option<ResourceKind>,
    #[arg(
        long,
        value_enum,
        default_value_t = ResourceOutputFormat::Table,
        help = "Render resource descriptions as text, table, json, or yaml."
    )]
    pub output_format: ResourceOutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct ResourceListArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        value_enum,
        help = "Grafana resource kind to list. Use grafana-util resource describe to see the current selector patterns and endpoints."
    )]
    pub kind: ResourceKind,
    #[arg(
        long,
        value_enum,
        default_value_t = ResourceOutputFormat::Table,
        help = "Render live resource inventory as text, table, json, or yaml."
    )]
    pub output_format: ResourceOutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct ResourceGetArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        value_name = "SELECTOR",
        help = "Fetch one live resource by selector. Use <kind>/<identity>, for example dashboards/cpu-main or folders/infra. Run grafana-util resource describe first if you need the supported selector patterns."
    )]
    pub selector: String,
    #[arg(
        long,
        value_enum,
        default_value_t = ResourceOutputFormat::Json,
        help = "Render the fetched live resource as text, table, json, or yaml."
    )]
    pub output_format: ResourceOutputFormat,
}

#[derive(Debug, Clone, Subcommand)]
pub enum ResourceCommand {
    #[command(
        name = "kinds",
        about = "List the resource kinds supported by the generic read-only resource query surface.",
        after_help = RESOURCE_KINDS_AFTER_HELP
    )]
    Kinds(ResourceKindsArgs),
    #[command(
        name = "describe",
        about = "Describe the supported live Grafana resource kinds and selector patterns.",
        after_help = RESOURCE_DESCRIBE_AFTER_HELP
    )]
    Describe(ResourceDescribeArgs),
    #[command(
        name = "list",
        about = "List one supported live Grafana resource kind.",
        after_help = RESOURCE_LIST_AFTER_HELP
    )]
    List(ResourceListArgs),
    #[command(
        name = "get",
        about = "Fetch one supported live Grafana resource by selector.",
        after_help = RESOURCE_GET_AFTER_HELP
    )]
    Get(ResourceGetArgs),
}

#[derive(Debug, Clone, Parser)]
pub struct ResourceCliArgs {
    #[arg(
        long,
        value_enum,
        default_value_t = CliColorChoice::Auto,
        help = "Colorize JSON and YAML output. Use auto, always, or never."
    )]
    pub color: CliColorChoice,
    #[command(subcommand)]
    pub command: ResourceCommand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResourceSelector {
    kind: ResourceKind,
    identity: String,
}

#[derive(Debug, Clone, Serialize)]
struct ResourceKindRecord {
    kind: &'static str,
    singular: &'static str,
    description: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct ResourceDescribeRecord {
    kind: &'static str,
    singular: &'static str,
    selector: &'static str,
    list_endpoint: &'static str,
    get_endpoint: &'static str,
    description: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct ResourceListDocument {
    kind: &'static str,
    count: usize,
    items: Vec<Map<String, Value>>,
}

#[derive(Debug, Clone, Serialize)]
struct ResourceDescribeDocument {
    kind: Option<&'static str>,
    count: usize,
    items: Vec<ResourceDescribeRecord>,
}

fn supported_kinds() -> [ResourceKind; 5] {
    [
        ResourceKind::Dashboards,
        ResourceKind::Folders,
        ResourceKind::Datasources,
        ResourceKind::AlertRules,
        ResourceKind::Orgs,
    ]
}

fn supported_kind_names() -> String {
    supported_kinds()
        .into_iter()
        .map(|kind| kind.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn supported_kind_records() -> Vec<ResourceKindRecord> {
    supported_kinds()
        .into_iter()
        .map(|kind| ResourceKindRecord {
            kind: kind.as_str(),
            singular: kind.singular_label(),
            description: kind.description(),
        })
        .collect()
}

fn describe_records(kind: Option<ResourceKind>) -> Vec<ResourceDescribeRecord> {
    let kinds = kind
        .map(|item| vec![item])
        .unwrap_or_else(|| supported_kinds().to_vec());
    kinds
        .into_iter()
        .map(|kind| ResourceDescribeRecord {
            kind: kind.as_str(),
            singular: kind.singular_label(),
            selector: kind.selector_pattern(),
            list_endpoint: kind.list_endpoint(),
            get_endpoint: kind.get_endpoint(),
            description: kind.description(),
        })
        .collect()
}

fn parse_selector(input: &str) -> Result<ResourceSelector> {
    let (kind, identity) = input
        .split_once('/')
        .ok_or_else(|| {
            message(
                "Resource selector must use <kind>/<identity>. Use grafana-util resource describe to see selector patterns and grafana-util resource kinds to list supported kinds.",
            )
        })?;
    let kind = kind.trim();
    let identity = identity.trim();
    if kind.is_empty() || identity.is_empty() {
        return Err(message(
            "Resource selector kind and identity cannot be empty.",
        ));
    }
    let kind = match kind {
        "dashboards" => ResourceKind::Dashboards,
        "folders" => ResourceKind::Folders,
        "datasources" => ResourceKind::Datasources,
        "alert-rules" => ResourceKind::AlertRules,
        "orgs" => ResourceKind::Orgs,
        _ => {
            return Err(message(format!(
                "Unsupported resource selector kind '{kind}'. Use grafana-util resource describe to see selector patterns. Supported kinds: {}.",
                supported_kind_names()
            )))
        }
    };
    Ok(ResourceSelector {
        kind,
        identity: identity.to_string(),
    })
}

fn build_client(common: &CommonCliArgs) -> Result<GrafanaApiClient> {
    let connection = GrafanaConnection::resolve(
        common.profile.as_deref(),
        ConnectionMergeInput {
            url: &common.url,
            url_default: DEFAULT_URL,
            api_token: common.api_token.as_deref(),
            username: common.username.as_deref(),
            password: common.password.as_deref(),
            org_id: None,
            timeout: common.timeout,
            timeout_default: DEFAULT_TIMEOUT,
            verify_ssl: common.verify_ssl,
            insecure: false,
            ca_cert: None,
        },
        AuthInputs {
            api_token: common.api_token.as_deref(),
            username: common.username.as_deref(),
            password: common.password.as_deref(),
            prompt_password: common.prompt_password,
            prompt_token: common.prompt_token,
        },
        false,
    )?;
    GrafanaApiClient::from_connection(connection)
}

fn list_resource_items(
    client: &GrafanaApiClient,
    kind: ResourceKind,
) -> Result<Vec<Map<String, Value>>> {
    match kind {
        ResourceKind::Dashboards => client.dashboard().list_dashboard_summaries(500),
        ResourceKind::Folders => expect_object_list(
            client
                .dashboard()
                .request_json(Method::GET, "/api/folders", &[], None)?,
            "Unexpected folder list response from Grafana.",
        ),
        ResourceKind::Datasources => client.datasource().list_datasources(),
        ResourceKind::AlertRules => client.alerting().list_alert_rules(),
        ResourceKind::Orgs => client.access().list_orgs(),
    }
}

fn get_resource_item(client: &GrafanaApiClient, selector: &ResourceSelector) -> Result<Value> {
    match selector.kind {
        ResourceKind::Dashboards => client.dashboard().fetch_dashboard(&selector.identity),
        ResourceKind::Folders => Ok(Value::Object(expect_object(
            client.dashboard().request_json(
                Method::GET,
                &format!("/api/folders/{}", selector.identity),
                &[],
                None,
            )?,
            "Unexpected folder payload from Grafana.",
        )?)),
        ResourceKind::Datasources => Ok(Value::Object(expect_object(
            client.datasource().request_json(
                Method::GET,
                &format!("/api/datasources/uid/{}", selector.identity),
                &[],
                None,
            )?,
            "Unexpected datasource payload from Grafana.",
        )?)),
        ResourceKind::AlertRules => Ok(Value::Object(
            client.alerting().get_alert_rule(&selector.identity)?,
        )),
        ResourceKind::Orgs => Ok(Value::Object(expect_object(
            client.access().request_json(
                Method::GET,
                &format!("/api/orgs/{}", selector.identity),
                &[],
                None,
            )?,
            "Unexpected org payload from Grafana.",
        )?)),
    }
}

fn list_row(kind: ResourceKind, item: &Map<String, Value>) -> Vec<String> {
    match kind {
        ResourceKind::Dashboards => vec![
            string_field(item, "uid", ""),
            string_field(item, "title", &string_field(item, "name", "")),
            string_field(item, "folderTitle", ""),
        ],
        ResourceKind::Folders => vec![
            string_field(item, "uid", ""),
            string_field(item, "title", ""),
            string_field(item, "parentUid", ""),
        ],
        ResourceKind::Datasources => vec![
            string_field(item, "uid", ""),
            string_field(item, "name", ""),
            string_field(item, "type", ""),
        ],
        ResourceKind::AlertRules => vec![
            string_field(item, "uid", ""),
            string_field(item, "title", ""),
            string_field(item, "folderUID", ""),
        ],
        ResourceKind::Orgs => vec![
            string_field(item, "id", ""),
            string_field(item, "name", ""),
            string_field(item, "address", ""),
        ],
    }
}

fn list_headers(kind: ResourceKind) -> [&'static str; 3] {
    match kind {
        ResourceKind::Dashboards => ["uid", "title", "folder"],
        ResourceKind::Folders => ["uid", "title", "parent_uid"],
        ResourceKind::Datasources => ["uid", "name", "type"],
        ResourceKind::AlertRules => ["uid", "title", "folder_uid"],
        ResourceKind::Orgs => ["id", "name", "address"],
    }
}

fn render_kind_catalog(args: &ResourceKindsArgs) -> Result<()> {
    let records = supported_kind_records();
    match args.output_format {
        ResourceOutputFormat::Text => {
            print_lines(
                &records
                    .iter()
                    .map(|record| {
                        format!(
                            "{} ({}) - {}",
                            record.kind, record.singular, record.description
                        )
                    })
                    .collect::<Vec<_>>(),
            );
        }
        ResourceOutputFormat::Table => {
            let rows = records
                .iter()
                .map(|record| {
                    vec![
                        record.kind.to_string(),
                        record.singular.to_string(),
                        record.description.to_string(),
                    ]
                })
                .collect::<Vec<_>>();
            print_lines(&render_table(&["kind", "singular", "description"], &rows));
        }
        ResourceOutputFormat::Json => {
            print!("{}", render_json_value(&records)?);
        }
        ResourceOutputFormat::Yaml => {
            print!("{}", render_yaml(&records)?);
        }
    }
    Ok(())
}

fn render_describe(args: &ResourceDescribeArgs) -> Result<()> {
    let items = describe_records(args.kind);
    let document = ResourceDescribeDocument {
        kind: args.kind.map(|kind| kind.as_str()),
        count: items.len(),
        items,
    };
    match args.output_format {
        ResourceOutputFormat::Text => {
            let mut lines = Vec::new();
            if let Some(kind) = document.kind {
                lines.push(format!("Resource kind: {kind}"));
            } else {
                lines.push("Resource kinds:".to_string());
            }
            for (index, record) in document.items.iter().enumerate() {
                if index > 0 {
                    lines.push(String::new());
                }
                lines.push(format!("Kind: {}", record.kind));
                lines.push(format!("Singular: {}", record.singular));
                lines.push(format!("Selector: {}", record.selector));
                lines.push(format!("List endpoint: {}", record.list_endpoint));
                lines.push(format!("Get endpoint: {}", record.get_endpoint));
                lines.push(format!("Description: {}", record.description));
            }
            print_lines(&lines);
        }
        ResourceOutputFormat::Table => {
            let rows = document
                .items
                .iter()
                .map(|record| {
                    vec![
                        record.kind.to_string(),
                        record.singular.to_string(),
                        record.selector.to_string(),
                        record.list_endpoint.to_string(),
                        record.get_endpoint.to_string(),
                        record.description.to_string(),
                    ]
                })
                .collect::<Vec<_>>();
            print_lines(&render_table(
                &[
                    "kind",
                    "singular",
                    "selector",
                    "list_endpoint",
                    "get_endpoint",
                    "description",
                ],
                &rows,
            ));
        }
        ResourceOutputFormat::Json => {
            print!("{}", render_json_value(&document)?);
        }
        ResourceOutputFormat::Yaml => {
            print!("{}", render_yaml(&document)?);
        }
    }
    Ok(())
}

fn render_list(args: &ResourceListArgs) -> Result<()> {
    let client = build_client(&args.common)?;
    let items = list_resource_items(&client, args.kind)?;
    let document = ResourceListDocument {
        kind: args.kind.as_str(),
        count: items.len(),
        items,
    };
    match args.output_format {
        ResourceOutputFormat::Text => {
            let lines = vec![
                format!("Resource list: {}", document.kind),
                format!("Count: {}", document.count),
            ];
            print_lines(&lines);
        }
        ResourceOutputFormat::Table => {
            let headers = list_headers(args.kind);
            let rows = document
                .items
                .iter()
                .map(|item| list_row(args.kind, item))
                .collect::<Vec<_>>();
            print_lines(&render_table(&headers, &rows));
        }
        ResourceOutputFormat::Json => {
            print!("{}", render_json_value(&document)?);
        }
        ResourceOutputFormat::Yaml => {
            print!("{}", render_yaml(&document)?);
        }
    }
    Ok(())
}

fn render_get(args: &ResourceGetArgs) -> Result<()> {
    let client = build_client(&args.common)?;
    let selector = parse_selector(&args.selector)?;
    let value = get_resource_item(&client, &selector)?;
    match args.output_format {
        ResourceOutputFormat::Text => {
            let object = value
                .as_object()
                .ok_or_else(|| message("Resource get text output requires a JSON object."))?;
            let summary = [
                ("kind", selector.kind.as_str().to_string()),
                ("identity", selector.identity.clone()),
                (
                    "title",
                    string_field(
                        object,
                        "title",
                        &string_field(object, "name", &string_field(object, "uid", "")),
                    ),
                ),
            ];
            print_lines(&render_summary_table(&summary));
        }
        ResourceOutputFormat::Table => {
            let object = value
                .as_object()
                .ok_or_else(|| message("Resource get table output requires a JSON object."))?;
            let rows = object
                .iter()
                .map(|(field, value)| {
                    (
                        field.as_str(),
                        match value {
                            Value::String(text) => text.clone(),
                            Value::Null => "null".to_string(),
                            _ => value.to_string(),
                        },
                    )
                })
                .collect::<Vec<_>>();
            print_lines(&render_summary_table(&rows));
        }
        ResourceOutputFormat::Json => {
            print!("{}", render_json_value(&value)?);
        }
        ResourceOutputFormat::Yaml => {
            print!("{}", render_yaml(&value)?);
        }
    }
    Ok(())
}

pub fn run_resource_cli(args: ResourceCliArgs) -> Result<()> {
    // Resource is a pure read-only surface; dispatch only routes to renderers,
    // with output shape determined by each command variant.
    set_json_color_choice(args.color);
    match args.command {
        ResourceCommand::Kinds(inner) => render_kind_catalog(&inner),
        ResourceCommand::Describe(inner) => render_describe(&inner),
        ResourceCommand::List(inner) => render_list(&inner),
        ResourceCommand::Get(inner) => render_get(&inner),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_selector_requires_kind_and_identity() {
        let error = parse_selector("dashboards").unwrap_err().to_string();
        assert!(error.contains("<kind>/<identity>"));
        assert!(error.contains("resource describe"));
        assert!(error.contains("resource kinds"));
    }

    #[test]
    fn parse_selector_trims_whitespace() {
        let selector = parse_selector(" dashboards / cpu-main ").unwrap();
        assert_eq!(selector.kind, ResourceKind::Dashboards);
        assert_eq!(selector.identity, "cpu-main");
    }

    #[test]
    fn parse_selector_accepts_supported_kinds() {
        let selector = parse_selector("datasources/prom-main").unwrap();
        assert_eq!(selector.kind, ResourceKind::Datasources);
        assert_eq!(selector.identity, "prom-main");
    }

    #[test]
    fn parse_selector_rejects_unsupported_kind_with_help() {
        let error = parse_selector("widgets/demo").unwrap_err().to_string();
        assert!(error.contains("Unsupported resource selector kind 'widgets'."));
        assert!(error.contains("resource describe"));
        assert!(error.contains("dashboards, folders, datasources, alert-rules, orgs"));
    }

    #[test]
    fn describe_records_include_selector_and_endpoints() {
        let records = describe_records(Some(ResourceKind::Dashboards));
        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert_eq!(record.kind, "dashboards");
        assert_eq!(record.selector, "dashboards/<uid>");
        assert_eq!(record.list_endpoint, "GET /api/search");
        assert_eq!(record.get_endpoint, "GET /api/dashboards/uid/{uid}");
    }
}
