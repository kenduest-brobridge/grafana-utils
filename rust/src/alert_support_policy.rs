use crate::common::{message, sanitize_path_component, Result};
use serde_json::{json, Map, Value};

use super::alert_support_io::value_to_string;

pub const MANAGED_ROUTE_LABEL_KEY: &str = "grafana_utils_route";

pub fn stable_route_label_key() -> &'static str {
    MANAGED_ROUTE_LABEL_KEY
}

pub fn build_stable_route_label_value(name: &str) -> String {
    let value = sanitize_path_component(name);
    if value.is_empty() {
        "managed-route".to_string()
    } else {
        value
    }
}

#[allow(dead_code)]
pub fn build_stable_route_matcher(route_name: &str) -> Value {
    json!([
        stable_route_label_key(),
        "=",
        build_stable_route_label_value(route_name)
    ])
}

fn value_list(value: Option<&Value>) -> Vec<Value> {
    value.and_then(Value::as_array).cloned().unwrap_or_default()
}

fn route_matcher_entries(route: &Map<String, Value>) -> Vec<Vec<String>> {
    route
        .get("object_matchers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|matcher| {
            let items = matcher.as_array()?;
            if items.len() != 3 {
                return None;
            }
            Some(items.iter().map(value_to_string).collect::<Vec<String>>())
        })
        .collect()
}

fn normalize_route_matchers(route: &mut Map<String, Value>, route_name: &str) {
    let managed_value = build_stable_route_label_value(route_name);
    let mut matchers = route_matcher_entries(route)
        .into_iter()
        .filter(|matcher| {
            !(matcher.first().map(String::as_str) == Some(stable_route_label_key())
                && matcher.get(1).map(String::as_str) == Some("="))
        })
        .map(|matcher| Value::Array(matcher.into_iter().map(Value::String).collect()))
        .collect::<Vec<Value>>();
    matchers.push(json!([stable_route_label_key(), "=", managed_value]));
    matchers.sort_by_key(value_to_string);
    matchers.dedup_by(|left, right| value_to_string(left) == value_to_string(right));
    route.insert("object_matchers".to_string(), Value::Array(matchers));
}

pub fn route_matches_stable_label(route: &Map<String, Value>, route_name: &str) -> bool {
    let expected_value = build_stable_route_label_value(route_name);
    route_matcher_entries(route).into_iter().any(|matcher| {
        matcher.first().map(String::as_str) == Some(stable_route_label_key())
            && matcher.get(1).map(String::as_str) == Some("=")
            && matcher.get(2).map(String::as_str) == Some(expected_value.as_str())
    })
}

pub fn build_route_preview(route: &Map<String, Value>) -> Value {
    let mut group_by = value_list(route.get("group_by"));
    group_by.sort_by_key(value_to_string);
    group_by.dedup_by(|left, right| value_to_string(left) == value_to_string(right));
    let mut matchers = value_list(route.get("object_matchers"));
    matchers.sort_by_key(value_to_string);
    matchers.dedup_by(|left, right| value_to_string(left) == value_to_string(right));
    json!({
        "receiver": crate::common::string_field(route, "receiver", ""),
        "continue": route.get("continue").and_then(Value::as_bool).unwrap_or(false),
        "groupBy": group_by,
        "matchers": matchers,
        "childRouteCount": route.get("routes").and_then(Value::as_array).map(Vec::len).unwrap_or(0),
    })
}

#[allow(dead_code)]
fn normalize_managed_policy_route(
    route_name: &str,
    route: &Map<String, Value>,
) -> Map<String, Value> {
    let mut normalized = route.clone();
    normalize_route_matchers(&mut normalized, route_name);
    normalized
}

#[allow(dead_code)]
fn route_list_with_indexes(policy: &Map<String, Value>) -> Vec<(usize, Map<String, Value>)> {
    policy
        .get("routes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(index, route)| route.as_object().cloned().map(|item| (index, item)))
        .collect()
}

#[allow(dead_code)]
pub fn upsert_managed_policy_subtree(
    policy: &Map<String, Value>,
    route_name: &str,
    route: &Map<String, Value>,
) -> Result<(Map<String, Value>, &'static str)> {
    let normalized_route = normalize_managed_policy_route(route_name, route);
    let mut next_policy = policy.clone();
    let routes = route_list_with_indexes(policy);
    let matching = routes
        .iter()
        .filter(|(_, item)| route_matches_stable_label(item, route_name))
        .map(|(index, _)| *index)
        .collect::<Vec<usize>>();
    if matching.len() > 1 {
        return Err(message(format!(
            "Managed route label {:?} is not unique in notification policies.",
            build_stable_route_label_value(route_name)
        )));
    }

    let mut next_routes = policy
        .get("routes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let action = if let Some(index) = matching.first().copied() {
        if next_routes
            .get(index)
            .and_then(Value::as_object)
            .map(|item| item == &normalized_route)
            .unwrap_or(false)
        {
            "noop"
        } else {
            next_routes[index] = Value::Object(normalized_route);
            "updated"
        }
    } else {
        next_routes.push(Value::Object(normalized_route));
        "created"
    };
    next_policy.insert("routes".to_string(), Value::Array(next_routes));
    Ok((next_policy, action))
}

#[allow(dead_code)]
pub fn remove_managed_policy_subtree(
    policy: &Map<String, Value>,
    route_name: &str,
) -> Result<(Map<String, Value>, &'static str)> {
    let routes = route_list_with_indexes(policy);
    let matching = routes
        .iter()
        .filter(|(_, item)| route_matches_stable_label(item, route_name))
        .map(|(index, _)| *index)
        .collect::<Vec<usize>>();
    if matching.len() > 1 {
        return Err(message(format!(
            "Managed route label {:?} is not unique in notification policies.",
            build_stable_route_label_value(route_name)
        )));
    }
    let Some(index) = matching.first().copied() else {
        return Ok((policy.clone(), "noop"));
    };

    let next_routes = policy
        .get("routes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .filter_map(|(position, value)| (position != index).then_some(value))
        .collect::<Vec<Value>>();
    let mut next_policy = policy.clone();
    next_policy.insert("routes".to_string(), Value::Array(next_routes));
    Ok((next_policy, "deleted"))
}

#[allow(dead_code)]
pub fn build_managed_policy_route_preview(
    current_policy: &Map<String, Value>,
    route_name: &str,
    desired_route: Option<&Map<String, Value>>,
) -> Result<Value> {
    let current_route = route_list_with_indexes(current_policy)
        .into_iter()
        .find(|(_, route)| route_matches_stable_label(route, route_name))
        .map(|(_, route)| route);
    let (next_policy, action) = match desired_route {
        Some(route) => upsert_managed_policy_subtree(current_policy, route_name, route)?,
        None => remove_managed_policy_subtree(current_policy, route_name)?,
    };
    let next_route = route_list_with_indexes(&next_policy)
        .into_iter()
        .find(|(_, route)| route_matches_stable_label(route, route_name))
        .map(|(_, route)| route);
    Ok(json!({
        "action": action,
        "managedRouteKey": stable_route_label_key(),
        "managedRouteValue": build_stable_route_label_value(route_name),
        "currentRoute": current_route.map(|route| build_route_preview(&route)).unwrap_or(Value::Null),
        "nextRoute": next_route.map(|route| build_route_preview(&route)).unwrap_or(Value::Null),
        "nextPolicyRouteCount": next_policy.get("routes").and_then(Value::as_array).map(Vec::len).unwrap_or(0),
    }))
}
