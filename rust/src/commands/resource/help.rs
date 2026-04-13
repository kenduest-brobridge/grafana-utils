//! Long-form resource CLI help text constants.

pub(crate) const RESOURCE_KINDS_AFTER_HELP: &str = r#"Examples:

  Show supported resource kinds as a table:
    grafana-util resource kinds

  Render the same kind catalog as JSON:
    grafana-util resource kinds --output-format json"#;

pub(crate) const RESOURCE_DESCRIBE_AFTER_HELP: &str = r#"Examples:

  Describe every supported kind as a table:
    grafana-util resource describe

  Describe one supported kind as JSON:
    grafana-util resource describe dashboards --output-format json"#;

pub(crate) const RESOURCE_LIST_AFTER_HELP: &str = r#"Examples:

  List dashboards as a table:
    grafana-util resource list dashboards --url http://localhost:3000 --basic-user admin --basic-password admin

  List folders as YAML:
    grafana-util resource list folders --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format yaml

  List alert rules as JSON:
    grafana-util resource list alert-rules --profile prod --output-format json"#;

pub(crate) const RESOURCE_GET_AFTER_HELP: &str = r#"Examples:

  Fetch one dashboard by UID:
    grafana-util resource get dashboards/cpu-main --url http://localhost:3000 --basic-user admin --basic-password admin

  Fetch one datasource by UID as YAML:
    grafana-util resource get datasources/prom-main --profile prod --output-format yaml

  Fetch one org by numeric ID:
    grafana-util resource get orgs/1 --profile prod --output-format json"#;
