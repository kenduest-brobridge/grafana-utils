//! Governance policy source resolution and file loading helpers.

use crate::common::{parse_error, validation, Result};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::Path;

use super::{GovernanceGateArgs, GovernancePolicySource};

const DEFAULT_BUILTIN_POLICY_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/dashboard/assets/builtin_governance_policy.json"
));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GovernancePolicyFormat {
    Json,
    Yaml,
}

#[derive(Clone, Debug)]
struct YamlLine {
    line_number: usize,
    indent: usize,
    text: String,
}

pub(crate) fn built_in_governance_policy() -> Value {
    serde_json::from_str(DEFAULT_BUILTIN_POLICY_JSON)
        .expect("checked-in built-in governance policy must be valid JSON")
}

fn merge_policy_overlay(base: &mut Value, overlay: &Value) {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (key, overlay_value) in overlay_map {
                match base_map.get_mut(key) {
                    Some(base_value) => merge_policy_overlay(base_value, overlay_value),
                    None => {
                        base_map.insert(key.clone(), overlay_value.clone());
                    }
                }
            }
        }
        (base_value, overlay_value) => *base_value = overlay_value.clone(),
    }
}

fn built_in_governance_policy_with_overlay(overlay: Value) -> Value {
    let mut policy = built_in_governance_policy();
    merge_policy_overlay(&mut policy, &overlay);
    policy
}

fn supported_builtin_policy_names() -> &'static str {
    "default, strict, balanced, lenient"
}

pub(crate) fn load_governance_policy(args: &GovernanceGateArgs) -> Result<Value> {
    load_governance_policy_source(
        args.policy_source,
        args.policy.as_deref(),
        args.builtin_policy.as_deref(),
    )
}

pub(crate) fn load_governance_policy_source(
    source: GovernancePolicySource,
    policy_path: Option<&Path>,
    builtin_policy: Option<&str>,
) -> Result<Value> {
    match source {
        GovernancePolicySource::File => {
            let policy_path = policy_path.ok_or_else(|| {
                validation(
                    "Governance gate requires --policy when --policy-source file is selected.",
                )
            })?;
            load_governance_policy_file(policy_path)
        }
        GovernancePolicySource::Builtin => {
            let policy_name = builtin_policy.unwrap_or("default");
            load_builtin_governance_policy(policy_name)
        }
    }
}

pub(crate) fn load_governance_policy_file(path: &Path) -> Result<Value> {
    let raw = fs::read_to_string(path)?;
    let preferred_format = preferred_policy_format(path, &raw);
    let value = parse_governance_policy_document(&raw, preferred_format).map_err(|error| {
        parse_error(
            format!("governance policy file {}", path.display()),
            format!(
                "could not be parsed as {}.\n{} parse error: {}",
                policy_format_label(preferred_format),
                policy_format_label(preferred_format),
                error
            ),
        )
    })?;
    if !value.is_object() {
        return Err(validation(format!(
            "Governance policy file {} must contain a JSON object or YAML mapping.",
            path.display()
        )));
    }
    Ok(value)
}

pub(crate) fn load_builtin_governance_policy(name: &str) -> Result<Value> {
    match name.trim().to_ascii_lowercase().as_str() {
        "default" | "example" => Ok(built_in_governance_policy()),
        "strict" => Ok(built_in_governance_policy_with_overlay(json!({
            "queries": {
                "maxQueriesPerDashboard": 40,
                "maxQueriesPerPanel": 4,
                "maxQueryComplexityScore": 4,
                "maxDashboardComplexityScore": 20
            },
            "enforcement": {
                "failOnWarnings": true
            }
        }))),
        "balanced" => Ok(built_in_governance_policy_with_overlay(json!({
            "queries": {
                "maxQueriesPerDashboard": 60,
                "maxQueriesPerPanel": 6,
                "maxQueryComplexityScore": 5,
                "maxDashboardComplexityScore": 30
            }
        }))),
        "lenient" => Ok(built_in_governance_policy_with_overlay(json!({
            "queries": {
                "maxQueriesPerDashboard": 120,
                "maxQueriesPerPanel": 12,
                "maxQueryComplexityScore": 8,
                "maxDashboardComplexityScore": 60,
                "forbidSelectStar": false,
                "requireSqlTimeFilter": false,
                "forbidBroadLokiRegex": false
            }
        }))),
        other => Err(validation(format!(
            "Unknown built-in governance policy {other:?}. Supported values: {}. Alias: example -> default.",
            supported_builtin_policy_names()
        ))),
    }
}

fn preferred_policy_format(path: &Path, raw: &str) -> GovernancePolicyFormat {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        _ if looks_like_json(raw) => GovernancePolicyFormat::Json,
        Some("json") => GovernancePolicyFormat::Json,
        Some("yaml") | Some("yml") => GovernancePolicyFormat::Yaml,
        _ => GovernancePolicyFormat::Yaml,
    }
}

fn looks_like_json(raw: &str) -> bool {
    matches!(raw.trim_start().chars().next(), Some('{') | Some('['))
}

fn policy_format_label(format: GovernancePolicyFormat) -> &'static str {
    match format {
        GovernancePolicyFormat::Json => "JSON",
        GovernancePolicyFormat::Yaml => "YAML",
    }
}

fn parse_governance_policy_document(
    raw: &str,
    format: GovernancePolicyFormat,
) -> std::result::Result<Value, String> {
    match format {
        GovernancePolicyFormat::Json => {
            serde_json::from_str(raw).map_err(|error| error.to_string())
        }
        GovernancePolicyFormat::Yaml => parse_governance_policy_yaml(raw),
    }
}

fn collect_yaml_lines(raw: &str) -> std::result::Result<Vec<YamlLine>, String> {
    let mut lines = Vec::new();
    for (index, raw_line) in raw.lines().enumerate() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed == "---" || trimmed == "..." || trimmed.starts_with('#') {
            continue;
        }
        if raw_line.contains('\t') {
            return Err(format!(
                "YAML governance policy cannot use tab indentation on line {}.",
                index + 1
            ));
        }
        let indent = raw_line.chars().take_while(|ch| *ch == ' ').count();
        lines.push(YamlLine {
            line_number: index + 1,
            indent,
            text: raw_line[indent..].trim_end().to_string(),
        });
    }
    Ok(lines)
}

fn split_yaml_mapping_pair(text: &str) -> Option<(&str, &str)> {
    let colon = text.find(':')?;
    let key = text[..colon].trim();
    let raw_value = &text[colon + 1..];
    if key.is_empty() {
        return None;
    }
    if raw_value.is_empty() || raw_value.starts_with(' ') {
        Some((key, raw_value.trim_start()))
    } else {
        None
    }
}

fn parse_yaml_scalar(text: &str) -> Value {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Value::Null;
    }
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return value;
    }
    if trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2 {
        return Value::String(trimmed[1..trimmed.len() - 1].replace("''", "'"));
    }
    match trimmed.to_ascii_lowercase().as_str() {
        "null" | "~" => Value::Null,
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        _ => trimmed
            .parse::<i64>()
            .map(|value| Value::Number(value.into()))
            .unwrap_or_else(|_| Value::String(trimmed.to_string())),
    }
}

fn parse_yaml_block(
    lines: &[YamlLine],
    start_index: usize,
) -> std::result::Result<(Value, usize), String> {
    let Some(first_line) = lines.get(start_index) else {
        return Err("YAML governance policy document is empty.".to_string());
    };
    if first_line.text.trim_start().starts_with('-') {
        parse_yaml_sequence(lines, start_index, first_line.indent)
    } else {
        parse_yaml_mapping(lines, start_index, first_line.indent)
    }
}

fn parse_yaml_mapping(
    lines: &[YamlLine],
    mut index: usize,
    indent: usize,
) -> std::result::Result<(Value, usize), String> {
    let mut map = Map::new();
    while let Some(line) = lines.get(index) {
        if line.indent < indent {
            break;
        }
        if line.indent > indent {
            return Err(format!(
                "Unexpected indentation in YAML governance policy on line {}.",
                line.line_number
            ));
        }
        let Some((key, value_text)) = split_yaml_mapping_pair(&line.text) else {
            return Err(format!(
                "Expected a YAML mapping entry on line {}.",
                line.line_number
            ));
        };
        index += 1;
        let value = if value_text.is_empty() {
            if let Some(next_line) = lines.get(index) {
                if next_line.indent > indent {
                    let (child_value, next_index) = parse_yaml_block(lines, index)?;
                    index = next_index;
                    child_value
                } else {
                    Value::Null
                }
            } else {
                Value::Null
            }
        } else {
            if let Some(next_line) = lines.get(index) {
                if next_line.indent > indent {
                    return Err(format!(
                        "YAML mapping entry on line {} cannot also own an indented block.",
                        line.line_number
                    ));
                }
            }
            parse_yaml_scalar(value_text)
        };
        map.insert(key.to_string(), value);
    }
    Ok((Value::Object(map), index))
}

fn parse_yaml_sequence_item(
    after_dash: &str,
    child_value: Option<Value>,
    line_number: usize,
) -> std::result::Result<Value, String> {
    if after_dash.is_empty() {
        return Ok(child_value.unwrap_or(Value::Null));
    }

    if let Some((key, value_text)) = split_yaml_mapping_pair(after_dash) {
        let mut map = Map::new();
        if value_text.is_empty() {
            map.insert(key.to_string(), child_value.unwrap_or(Value::Null));
            return Ok(Value::Object(map));
        }
        map.insert(key.to_string(), parse_yaml_scalar(value_text));
        if let Some(Value::Object(child_map)) = child_value {
            for (child_key, child_value) in child_map {
                map.insert(child_key, child_value);
            }
            return Ok(Value::Object(map));
        }
        if child_value.is_some() {
            return Err(format!(
                "YAML sequence item on line {} cannot own a nested block after a scalar value.",
                line_number
            ));
        }
        return Ok(Value::Object(map));
    }

    if child_value.is_some() {
        return Err(format!(
            "YAML sequence item on line {} cannot own an indented block after a scalar value.",
            line_number
        ));
    }
    Ok(parse_yaml_scalar(after_dash))
}

fn parse_yaml_sequence(
    lines: &[YamlLine],
    mut index: usize,
    indent: usize,
) -> std::result::Result<(Value, usize), String> {
    let mut items = Vec::new();
    while let Some(line) = lines.get(index) {
        if line.indent < indent {
            break;
        }
        if line.indent > indent {
            return Err(format!(
                "Unexpected indentation in YAML governance policy on line {}.",
                line.line_number
            ));
        }
        let trimmed = line.text.trim_start();
        if !trimmed.starts_with('-') {
            break;
        }
        let after_dash = trimmed[1..].trim_start();
        index += 1;
        let child_value = if let Some(next_line) = lines.get(index) {
            if next_line.indent > indent {
                let (child_value, next_index) = parse_yaml_block(lines, index)?;
                index = next_index;
                Some(child_value)
            } else {
                None
            }
        } else {
            None
        };
        items.push(parse_yaml_sequence_item(
            after_dash,
            child_value,
            line.line_number,
        )?);
    }
    Ok((Value::Array(items), index))
}

fn parse_governance_policy_yaml(raw: &str) -> std::result::Result<Value, String> {
    let lines = collect_yaml_lines(raw)?;
    let (value, next_index) = parse_yaml_block(&lines, 0)?;
    if next_index != lines.len() {
        let line_number = lines
            .get(next_index)
            .map(|line| line.line_number)
            .unwrap_or_else(|| lines.last().map(|line| line.line_number).unwrap_or(1));
        return Err(format!(
            "Unexpected trailing content in YAML governance policy on line {}.",
            line_number
        ));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn load_governance_policy_source_supports_builtin_policy() {
        let policy =
            load_governance_policy_source(GovernancePolicySource::Builtin, None, None).unwrap();

        assert_eq!(policy["version"], json!(1));
    }

    #[test]
    fn load_governance_policy_source_supports_named_builtin_policy() {
        let policy =
            load_governance_policy_source(GovernancePolicySource::Builtin, None, Some("default"))
                .unwrap();

        assert_eq!(policy["version"], json!(1));
    }

    #[test]
    fn load_governance_policy_source_supports_strict_builtin_policy() {
        let policy =
            load_governance_policy_source(GovernancePolicySource::Builtin, None, Some("strict"))
                .unwrap();

        assert_eq!(policy["version"], json!(1));
        assert_eq!(policy["queries"]["maxQueriesPerDashboard"], json!(40));
        assert_eq!(policy["queries"]["maxQueriesPerPanel"], json!(4));
        assert_eq!(policy["queries"]["maxQueryComplexityScore"], json!(4));
        assert_eq!(policy["queries"]["maxDashboardComplexityScore"], json!(20));
        assert_eq!(policy["enforcement"]["failOnWarnings"], json!(true));
    }

    #[test]
    fn load_governance_policy_source_supports_balanced_builtin_policy() {
        let policy =
            load_governance_policy_source(GovernancePolicySource::Builtin, None, Some("balanced"))
                .unwrap();

        assert_eq!(policy["version"], json!(1));
        assert_eq!(policy["queries"]["maxQueriesPerDashboard"], json!(60));
        assert_eq!(policy["queries"]["maxQueriesPerPanel"], json!(6));
        assert_eq!(policy["queries"]["maxQueryComplexityScore"], json!(5));
        assert_eq!(policy["queries"]["maxDashboardComplexityScore"], json!(30));
        assert_eq!(policy["queries"]["forbidSelectStar"], json!(true));
        assert_eq!(policy["enforcement"]["failOnWarnings"], json!(false));
    }

    #[test]
    fn load_governance_policy_source_supports_lenient_builtin_policy() {
        let policy =
            load_governance_policy_source(GovernancePolicySource::Builtin, None, Some("lenient"))
                .unwrap();

        assert_eq!(policy["version"], json!(1));
        assert_eq!(policy["queries"]["maxQueriesPerDashboard"], json!(120));
        assert_eq!(policy["queries"]["maxQueriesPerPanel"], json!(12));
        assert_eq!(policy["queries"]["maxQueryComplexityScore"], json!(8));
        assert_eq!(policy["queries"]["maxDashboardComplexityScore"], json!(60));
        assert_eq!(policy["queries"]["forbidSelectStar"], json!(false));
        assert_eq!(policy["queries"]["requireSqlTimeFilter"], json!(false));
        assert_eq!(policy["queries"]["forbidBroadLokiRegex"], json!(false));
        assert_eq!(policy["enforcement"]["failOnWarnings"], json!(false));
    }

    #[test]
    fn load_governance_policy_source_reports_supported_builtin_policy_names() {
        let error =
            load_governance_policy_source(GovernancePolicySource::Builtin, None, Some("unknown"))
                .unwrap_err()
                .to_string();

        assert!(error.contains("Supported values: default, strict, balanced, lenient."));
        assert!(error.contains("Alias: example -> default."));
    }

    #[test]
    fn load_governance_policy_file_accepts_json_policy_documents() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("policy.json");
        fs::write(
            &path,
            serde_json::to_string_pretty(&json!({
                "version": 1,
                "queries": {
                    "maxQueriesPerDashboard": 4
                }
            }))
            .unwrap(),
        )
        .unwrap();

        let policy = load_governance_policy_file(&path).unwrap();

        assert_eq!(policy["version"], json!(1));
        assert_eq!(policy["queries"]["maxQueriesPerDashboard"], json!(4));
    }

    #[test]
    fn load_governance_policy_file_accepts_yaml_policy_documents() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("policy.yaml");
        fs::write(
            &path,
            r#"
version: 1
queries:
  maxQueriesPerDashboard: 2
  maxQueriesPerPanel: 1
enforcement:
  failOnWarnings: true
"#,
        )
        .unwrap();

        let policy = load_governance_policy_file(&path).unwrap();

        assert_eq!(policy["version"], json!(1));
        assert_eq!(policy["queries"]["maxQueriesPerDashboard"], json!(2));
        assert_eq!(policy["enforcement"]["failOnWarnings"], json!(true));
    }

    #[test]
    fn load_governance_policy_file_accepts_json_content_without_extension() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("policy");
        fs::write(
            &path,
            serde_json::to_string_pretty(&json!({
                "version": 1,
                "dashboards": {
                    "minRefreshIntervalSeconds": 30
                }
            }))
            .unwrap(),
        )
        .unwrap();

        let policy = load_governance_policy_file(&path).unwrap();

        assert_eq!(policy["dashboards"]["minRefreshIntervalSeconds"], json!(30));
    }

    #[test]
    fn load_governance_policy_file_reports_yaml_parse_errors_clearly() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("policy.yaml");
        fs::write(
            &path,
            r#"
version: 1
queries:
  maxQueriesPerDashboard: 2
  maxQueriesPerPanel: 1
  invalid
"#,
        )
        .unwrap();

        let error = load_governance_policy_file(&path).unwrap_err().to_string();

        assert!(error.contains("governance policy file"));
        assert!(error.contains("YAML"));
        assert!(error.contains("line"));
    }
}
