use clap::Args;
use std::path::PathBuf;

use super::super::super::DEFAULT_PAGE_SIZE;
use super::super::{
    CommonCliArgs, DashboardImportInputFormat, GovernanceGateOutputFormat, GovernancePolicySource,
    ImpactOutputFormat, InspectExportInputType, TopologyOutputFormat, ValidationOutputFormat,
};
use super::parse_dashboard_summary_input_format;
use super::parse_dashboard_validate_input_format;

/// Struct definition for GovernanceGateArgs.
#[derive(Debug, Clone, Args)]
pub struct GovernanceGateArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        default_value_t = DEFAULT_PAGE_SIZE,
        help = "Dashboard search page size when policy stages live review artifacts."
    )]
    pub page_size: usize,
    #[arg(
        long,
        conflicts_with = "all_orgs",
        help = "Stage dashboard findings from one explicit Grafana org ID instead of the current org when reading live Grafana."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "org_id",
        help = "Stage dashboard findings across all visible Grafana orgs when reading live Grafana. Prefer Basic auth when you need cross-org review because API tokens are often scoped to one org."
    )]
    pub all_orgs: bool,
    #[arg(
        long = "input-dir",
        conflicts_with_all = ["governance", "queries"],
        help = "Stage dashboard findings from this local export tree directly. Prefer --url for live Grafana or saved artifacts only for advanced reuse. Use --input-format git-sync for a repo-backed Git Sync dashboard tree."
    )]
    pub input_dir: Option<PathBuf>,
    #[arg(
        long = "input-format",
        default_value = "raw",
        value_parser = parse_dashboard_summary_input_format,
        value_name = "raw|provisioning|git-sync",
        help = "Interpret --input-dir as raw export files, Grafana file-provisioning artifacts, or a repo-backed Git Sync dashboard tree from a local review source."
    )]
    pub input_format: DashboardImportInputFormat,
    #[arg(
        long = "input-type",
        value_enum,
        help = "Disambiguate a mixed export root when --input-dir can resolve to both raw and source-style dashboard inputs."
    )]
    pub input_type: Option<InspectExportInputType>,
    #[arg(
        long,
        value_enum,
        default_value_t = GovernancePolicySource::File,
        help = "Select the governance policy source. Use file with --policy, or builtin without --policy."
    )]
    pub policy_source: GovernancePolicySource,
    #[arg(
        long,
        help = "Path to the dashboard governance policy file (JSON or YAML)."
    )]
    pub policy: Option<PathBuf>,
    #[arg(
        long = "builtin-policy",
        conflicts_with = "policy",
        help = "Built-in governance policy name. Use with --policy-source builtin."
    )]
    pub builtin_policy: Option<String>,
    #[arg(
        long,
        help = "Reuse a saved governance-json artifact. Prefer --url or --input-dir for the common path; keep this for advanced reuse."
    )]
    pub governance: Option<PathBuf>,
    #[arg(
        long,
        help = "Reuse a saved queries-json artifact. Prefer --url or --input-dir for the common path; keep this for advanced reuse."
    )]
    pub queries: Option<PathBuf>,
    #[arg(
        long,
        value_enum,
        default_value_t = GovernanceGateOutputFormat::Text,
        help = "Render the governance gate result as text or JSON."
    )]
    pub output_format: GovernanceGateOutputFormat,
    #[arg(
        long,
        help = "Optional path to also write the normalized governance gate result JSON."
    )]
    pub json_output: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Open an interactive terminal browser over governance findings."
    )]
    pub interactive: bool,
}

/// Struct definition for TopologyArgs.
#[derive(Debug, Clone, Args)]
pub struct TopologyArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        default_value_t = DEFAULT_PAGE_SIZE,
        help = "Dashboard search page size when dependencies stages live review artifacts."
    )]
    pub page_size: usize,
    #[arg(
        long,
        conflicts_with = "all_orgs",
        help = "Stage dashboard findings from one explicit Grafana org ID instead of the current org when reading live Grafana."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "org_id",
        help = "Stage dashboard findings across all visible Grafana orgs when reading live Grafana. Prefer Basic auth when you need cross-org review because API tokens are often scoped to one org."
    )]
    pub all_orgs: bool,
    #[arg(
        long = "input-dir",
        conflicts_with_all = ["governance", "queries"],
        help = "Stage dashboard findings from this local export tree directly. Prefer --url for live Grafana or saved artifacts only for advanced reuse. Use --input-format git-sync for a repo-backed Git Sync dashboard tree."
    )]
    pub input_dir: Option<PathBuf>,
    #[arg(
        long = "input-format",
        default_value = "raw",
        value_parser = parse_dashboard_summary_input_format,
        value_name = "raw|provisioning|git-sync",
        help = "Interpret --input-dir as raw export files, Grafana file-provisioning artifacts, or a repo-backed Git Sync dashboard tree from a local review source."
    )]
    pub input_format: DashboardImportInputFormat,
    #[arg(
        long,
        value_enum,
        help = "Disambiguate a mixed export root when --input-dir can resolve to both raw and source-style dashboard inputs."
    )]
    pub input_type: Option<InspectExportInputType>,
    #[arg(
        long,
        help = "Reuse a saved governance-json artifact. Prefer --url or --input-dir for the common path; keep this for advanced reuse."
    )]
    pub governance: Option<PathBuf>,
    #[arg(
        long,
        help = "Reuse a saved queries-json artifact when you already split review into artifact files. Prefer --url or --input-dir for the common path; keep this for advanced reuse."
    )]
    pub queries: Option<PathBuf>,
    #[arg(
        long = "alert-contract",
        help = "Optional path to a sync alert contract JSON document."
    )]
    pub alert_contract: Option<PathBuf>,
    #[arg(
        long,
        value_enum,
        default_value_t = TopologyOutputFormat::Text,
        help = "Choose how to render the dependency view: text for terminal reading, json for CI/scripts, mermaid for Markdown/docs, or dot for Graphviz."
    )]
    pub output_format: TopologyOutputFormat,
    #[arg(
        long,
        help = "Optional path to also write the rendered dependency output."
    )]
    pub output_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        requires = "output_file",
        help = "When --output-file is set, also print the rendered dependency output to stdout."
    )]
    pub also_stdout: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Open an interactive terminal browser over dependency nodes and edges."
    )]
    pub interactive: bool,
}

/// Struct definition for ImpactArgs.
#[derive(Debug, Clone, Args)]
pub struct ImpactArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        default_value_t = DEFAULT_PAGE_SIZE,
        help = "Dashboard search page size when impact stages live review artifacts."
    )]
    pub page_size: usize,
    #[arg(
        long,
        conflicts_with = "all_orgs",
        help = "Trace dashboard impact from one explicit Grafana org ID instead of the current org when reading live Grafana."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "org_id",
        help = "Trace dashboard impact across all visible Grafana orgs when reading live Grafana. Prefer Basic auth when you need cross-org review because API tokens are often scoped to one org."
    )]
    pub all_orgs: bool,
    #[arg(
        long = "input-dir",
        conflicts_with_all = ["governance", "queries"],
        help = "Trace dashboard impact from this export directory instead of live Grafana or prebuilt artifact files. Use --input-format git-sync for a repo-backed Git Sync dashboard tree."
    )]
    pub input_dir: Option<PathBuf>,
    #[arg(
        long = "input-format",
        default_value = "raw",
        value_parser = parse_dashboard_summary_input_format,
        value_name = "raw|provisioning|git-sync",
        help = "Interpret --input-dir as raw export files, Grafana file-provisioning artifacts, or a repo-backed Git Sync dashboard tree."
    )]
    pub input_format: DashboardImportInputFormat,
    #[arg(
        long = "input-type",
        value_enum,
        help = "Disambiguate a mixed export root when --input-dir can resolve to both raw and source-style dashboard inputs."
    )]
    pub input_type: Option<InspectExportInputType>,
    #[arg(
        long,
        help = "Reuse this dashboard governance JSON artifact instead of running dashboard summary or dependencies first."
    )]
    pub governance: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional query report artifact path when reusing prebuilt review files. Impact itself derives blast radius from governance data plus optional alert-contract data."
    )]
    pub queries: Option<PathBuf>,
    #[arg(
        long = "datasource-uid",
        help = "Datasource UID to review for downstream dashboard and alert impact."
    )]
    pub datasource_uid: String,
    #[arg(
        long = "alert-contract",
        help = "Optional path to a sync alert contract JSON document."
    )]
    pub alert_contract: Option<PathBuf>,
    #[arg(
        long,
        value_enum,
        default_value_t = ImpactOutputFormat::Text,
        help = "Render the blast radius summary as text or json."
    )]
    pub output_format: ImpactOutputFormat,
    #[arg(
        long,
        default_value_t = false,
        help = "Open an interactive terminal browser over the blast radius document."
    )]
    pub interactive: bool,
}

/// Struct definition for ValidateExportArgs.
#[derive(Debug, Clone, Args)]
pub struct ValidateExportArgs {
    #[arg(
        long = "input-dir",
        help = "Validate dashboards from this export directory. Use raw/ by default, use provisioning/ or its dashboards/ subdirectory with --input-format provisioning, or use --input-format git-sync for a Grafana OaC repo root."
    )]
    pub input_dir: PathBuf,
    #[arg(
        long,
        default_value = "raw",
        value_parser = parse_dashboard_validate_input_format,
        value_name = "raw|provisioning|git-sync",
        help = "Interpret --input-dir as raw export files, Grafana file-provisioning artifacts, or a repo-backed Git Sync dashboard tree. Use git-sync for a Grafana OaC repo root; use provisioning for a provisioning/ root or its dashboards/ subdirectory."
    )]
    pub input_format: DashboardImportInputFormat,
    #[arg(
        long,
        default_value_t = false,
        help = "Reject unsupported custom panel and datasource plugin types."
    )]
    pub reject_custom_plugins: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Reject legacy dashboard properties such as row layouts or web-import scaffolding."
    )]
    pub reject_legacy_properties: bool,
    #[arg(
        long,
        help = "Optional target dashboard schemaVersion required for this export set."
    )]
    pub target_schema_version: Option<i64>,
    #[arg(
        long,
        value_enum,
        default_value_t = ValidationOutputFormat::Text,
        help = "Render the validation result as text or JSON."
    )]
    pub output_format: ValidationOutputFormat,
    #[arg(long, help = "Optional path to also write the validation JSON result.")]
    pub output_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        requires = "output_file",
        help = "When --output-file is set, also print the validation result to stdout."
    )]
    pub also_stdout: bool,
}
