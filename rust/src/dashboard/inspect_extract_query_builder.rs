//! Inspection path for Dashboard resources: analysis, extraction, and report shaping.

use serde_json::{Map, Value};

use crate::common::string_field;

pub(crate) fn extract_query_field_and_text(target: &Map<String, Value>) -> (String, String) {
    for key in [
        "expr",
        "expression",
        "query",
        "logql",
        "rawSql",
        "sql",
        "rawQuery",
    ] {
        if let Some(value) = target.get(key).and_then(Value::as_str) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return (key.to_string(), trimmed.to_string());
            }
        }
    }
    let synthesized = synthesize_influx_builder_query(target);
    if !synthesized.is_empty() {
        return ("builder".to_string(), synthesized);
    }
    (String::new(), String::new())
}

fn first_step_param(step: &Map<String, Value>) -> String {
    step.get("params")
        .and_then(Value::as_array)
        .and_then(|params| params.first())
        .map(|value| match value {
            Value::String(text) => text.trim().to_string(),
            other => other.to_string(),
        })
        .unwrap_or_default()
}

fn render_influx_select_chain(chain: &Value) -> String {
    let Some(steps) = chain.as_array() else {
        return String::new();
    };
    let mut expression = String::new();
    for step in steps {
        let Some(step_object) = step.as_object() else {
            continue;
        };
        let step_type = string_field(step_object, "type", "");
        let param = first_step_param(step_object);
        match step_type.as_str() {
            "field" => {
                if !param.is_empty() {
                    expression = format!("\"{param}\"");
                }
            }
            "math" => {
                if !param.is_empty() {
                    if expression.is_empty() {
                        expression = param;
                    } else {
                        expression.push_str(&param);
                    }
                }
            }
            "alias" => {}
            "" => {}
            _ => {
                if !expression.is_empty() {
                    expression = format!("{step_type}({expression})");
                } else if !param.is_empty() {
                    expression = format!("{step_type}({param})");
                } else {
                    expression = format!("{step_type}()");
                }
            }
        }
    }
    expression.trim().to_string()
}

fn render_influx_group_by_clause(group_by: Option<&Value>) -> String {
    let Some(items) = group_by.and_then(Value::as_array) else {
        return String::new();
    };
    let mut parts = Vec::new();
    for item in items {
        let Some(group_object) = item.as_object() else {
            continue;
        };
        let group_type = string_field(group_object, "type", "");
        let param = first_step_param(group_object);
        let rendered = match group_type.as_str() {
            "time" if !param.is_empty() => format!("time({param})"),
            "fill" if !param.is_empty() => format!("fill({param})"),
            "tag" if !param.is_empty() => format!("\"{param}\""),
            _ if !group_type.is_empty() && !param.is_empty() => format!("{group_type}({param})"),
            _ if !group_type.is_empty() => group_type,
            _ => String::new(),
        };
        if !rendered.is_empty() {
            parts.push(rendered);
        }
    }
    parts.join(", ")
}

fn render_influx_where_clause(tags: Option<&Value>) -> String {
    let Some(items) = tags.and_then(Value::as_array) else {
        return String::new();
    };
    let mut parts = Vec::new();
    for item in items {
        let Some(tag_object) = item.as_object() else {
            continue;
        };
        let key = string_field(tag_object, "key", "");
        let operator = string_field(tag_object, "operator", "=");
        let value = string_field(tag_object, "value", "");
        if key.is_empty() || value.is_empty() {
            continue;
        }
        let condition = string_field(tag_object, "condition", "").to_ascii_uppercase();
        if !parts.is_empty() && (condition == "AND" || condition == "OR") {
            parts.push(condition);
        }
        parts.push(format!("\"{key}\" {operator} {value}"));
    }
    parts.join(" ")
}

fn synthesize_influx_builder_query(target: &Map<String, Value>) -> String {
    let measurement = string_field(target, "measurement", "");
    let select_parts = target
        .get("select")
        .and_then(Value::as_array)
        .map(|chains| {
            chains
                .iter()
                .map(render_influx_select_chain)
                .filter(|value| !value.is_empty())
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
    if measurement.is_empty() && select_parts.is_empty() {
        return String::new();
    }
    let mut query = format!(
        "SELECT {}",
        if select_parts.is_empty() {
            "*".to_string()
        } else {
            select_parts.join(", ")
        }
    );
    if !measurement.is_empty() {
        query.push_str(&format!(" FROM \"{measurement}\""));
    }
    let where_clause = render_influx_where_clause(target.get("tags"));
    if !where_clause.is_empty() {
        query.push_str(&format!(" WHERE {where_clause}"));
    }
    let group_by_clause = render_influx_group_by_clause(target.get("groupBy"));
    if !group_by_clause.is_empty() {
        query.push_str(&format!(" GROUP BY {group_by_clause}"));
    }
    query
}

#[cfg(test)]
mod tests {
    use super::extract_query_field_and_text;
    use serde_json::json;

    #[test]
    fn extract_query_field_and_text_synthesizes_influx_builder_query() {
        let target = json!({
            "measurement": "cpu",
            "select": [[
                {"type": "field", "params": ["usage_idle"]},
                {"type": "mean", "params": []}
            ]],
            "tags": [
                {"key": "host", "operator": "=", "value": "\"server1\""}
            ],
            "groupBy": [
                {"type": "time", "params": ["5m"]}
            ]
        })
        .as_object()
        .expect("target must be an object")
        .clone();

        assert_eq!(
            extract_query_field_and_text(&target),
            (
                "builder".to_string(),
                "SELECT mean(\"usage_idle\") FROM \"cpu\" WHERE \"host\" = \"server1\" GROUP BY time(5m)"
                    .to_string()
            )
        );
    }
}
