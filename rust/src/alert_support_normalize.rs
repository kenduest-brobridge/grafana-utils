use serde_json::{Map, Value};

use super::alert_support_io::value_to_string;

pub fn strip_server_managed_fields(kind: &str, payload: &Map<String, Value>) -> Map<String, Value> {
    let managed_fields = match kind {
        super::super::RULE_KIND => ["id", "updated", "provenance"].as_slice(),
        super::super::CONTACT_POINT_KIND => ["provenance"].as_slice(),
        super::super::MUTE_TIMING_KIND => ["version", "provenance"].as_slice(),
        super::super::POLICIES_KIND => ["provenance"].as_slice(),
        super::super::TEMPLATE_KIND => ["version", "provenance"].as_slice(),
        _ => [].as_slice(),
    };

    payload
        .iter()
        .filter(|(key, _)| !managed_fields.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn remove_null_field(object: &mut Map<String, Value>, key: &str) {
    if matches!(object.get(key), Some(Value::Null)) {
        object.remove(key);
    }
}

fn remove_empty_object_field(object: &mut Map<String, Value>, key: &str) {
    if object
        .get(key)
        .and_then(Value::as_object)
        .map(|value| value.is_empty())
        .unwrap_or(false)
    {
        object.remove(key);
    }
}

fn remove_string_field_when(object: &mut Map<String, Value>, key: &str, expected: &str) {
    if object.get(key).and_then(Value::as_str) == Some(expected) {
        object.remove(key);
    }
}

fn remove_bool_field_when(object: &mut Map<String, Value>, key: &str, expected: bool) {
    if object.get(key).and_then(Value::as_bool) == Some(expected) {
        object.remove(key);
    }
}

fn sort_string_array_field(object: &mut Map<String, Value>, key: &str) {
    let Some(values) = object.get_mut(key).and_then(Value::as_array_mut) else {
        return;
    };
    values.sort_by_key(value_to_string);
    values.dedup_by(|left, right| value_to_string(left) == value_to_string(right));
}

fn sort_matcher_values(matchers: &mut Vec<Value>) {
    matchers.sort_by_key(value_to_string);
    matchers.dedup_by(|left, right| value_to_string(left) == value_to_string(right));
}

fn normalize_compare_value(value: Value) -> Value {
    match value {
        Value::Array(items) => {
            Value::Array(items.into_iter().map(normalize_compare_value).collect())
        }
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .map(|(key, item)| (key, normalize_compare_value(item)))
                .collect(),
        ),
        Value::Number(number) => {
            if let Some(float_value) = number.as_f64() {
                if float_value.fract() == 0.0
                    && float_value >= i64::MIN as f64
                    && float_value <= i64::MAX as f64
                {
                    return Value::Number(serde_json::Number::from(float_value as i64));
                }
            }
            Value::Number(number)
        }
        other => other,
    }
}

fn normalize_rule_compare_payload(payload: &mut Map<String, Value>) {
    payload.remove("orgID");
    remove_bool_field_when(payload, "isPaused", false);
    normalize_rule_duration_field(payload, "for");
    normalize_rule_duration_field(payload, "keep_firing_for");
    remove_null_field(payload, "notification_settings");
    remove_null_field(payload, "record");
    remove_empty_object_field(payload, "annotations");

    if let Some(data) = payload.get_mut("data").and_then(Value::as_array_mut) {
        for item in data {
            let Some(item_object) = item.as_object_mut() else {
                continue;
            };
            remove_string_field_when(item_object, "queryType", "");
        }
    }
}

fn normalize_rule_duration_field(payload: &mut Map<String, Value>, field: &str) {
    let Some(raw_value) = payload.get(field).and_then(Value::as_str) else {
        return;
    };
    let Some(duration_seconds) = parse_duration_seconds(raw_value) else {
        return;
    };
    if duration_seconds == 0 {
        payload.remove(field);
        return;
    }
    payload.insert(
        field.to_string(),
        Value::String(format!("{duration_seconds}s")),
    );
}

fn normalize_contact_point_compare_payload(payload: &mut Map<String, Value>) {
    remove_bool_field_when(payload, "disableResolveMessage", false);
}

fn normalize_policy_route_for_compare(route: &mut Map<String, Value>) {
    remove_bool_field_when(route, "continue", false);
    sort_string_array_field(route, "group_by");
    if let Some(matchers) = route
        .get_mut("object_matchers")
        .and_then(Value::as_array_mut)
    {
        sort_matcher_values(matchers);
    }
    if let Some(routes) = route.get_mut("routes").and_then(Value::as_array_mut) {
        for nested_route in routes {
            let Some(route_object) = nested_route.as_object_mut() else {
                continue;
            };
            normalize_policy_route_for_compare(route_object);
        }
    }
}

fn normalize_policy_compare_payload(payload: &mut Map<String, Value>) {
    sort_string_array_field(payload, "group_by");
    if let Some(routes) = payload.get_mut("routes").and_then(Value::as_array_mut) {
        for route in routes {
            let Some(route_object) = route.as_object_mut() else {
                continue;
            };
            normalize_policy_route_for_compare(route_object);
        }
    }
}

fn normalize_template_compare_payload(payload: &mut Map<String, Value>) {
    let Some(template) = payload
        .get("template")
        .and_then(Value::as_str)
        .map(ToString::to_string)
    else {
        return;
    };
    let mut normalized = template.replace("\r\n", "\n");
    while normalized.ends_with('\n') {
        normalized.pop();
    }
    if !normalized.is_empty() {
        normalized.push('\n');
    }
    payload.insert("template".to_string(), Value::String(normalized));
}

fn parse_duration_seconds(value: &str) -> Option<u64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let split = trimmed
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(trimmed.len());
    let (number_part, unit_part) = trimmed.split_at(split);
    let quantity = number_part.parse::<u64>().ok()?;
    let unit = unit_part.trim();
    let multiplier = match unit {
        "s" | "" => 1,
        "m" => 60,
        "h" => 60 * 60,
        "d" => 60 * 60 * 24,
        "w" => 60 * 60 * 24 * 7,
        _ => return None,
    };
    quantity.checked_mul(multiplier)
}

pub fn normalize_compare_payload(kind: &str, payload: &Map<String, Value>) -> Map<String, Value> {
    let mut normalized = strip_server_managed_fields(kind, payload);
    match kind {
        super::super::RULE_KIND => normalize_rule_compare_payload(&mut normalized),
        super::super::CONTACT_POINT_KIND => {
            normalize_contact_point_compare_payload(&mut normalized)
        }
        super::super::POLICIES_KIND => normalize_policy_compare_payload(&mut normalized),
        super::super::TEMPLATE_KIND => normalize_template_compare_payload(&mut normalized),
        _ => {}
    }
    normalize_compare_value(Value::Object(normalized))
        .as_object()
        .cloned()
        .expect("normalized compare payload must remain an object")
}
