//! Shared governance risk metadata, scoring, and duration parsing helpers.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GovernanceRiskSpec {
    pub(crate) category: &'static str,
    pub(crate) severity: &'static str,
    pub(crate) recommendation: &'static str,
}

pub(crate) const GOVERNANCE_RISK_KIND_MIXED_DASHBOARD: &str = "mixed-datasource-dashboard";
pub(crate) const GOVERNANCE_RISK_KIND_ORPHANED_DATASOURCE: &str = "orphaned-datasource";
pub(crate) const GOVERNANCE_RISK_KIND_UNKNOWN_DATASOURCE_FAMILY: &str = "unknown-datasource-family";
pub(crate) const GOVERNANCE_RISK_KIND_EMPTY_QUERY_ANALYSIS: &str = "empty-query-analysis";
pub(crate) const GOVERNANCE_RISK_KIND_BROAD_LOKI_SELECTOR: &str = "broad-loki-selector";
pub(crate) const GOVERNANCE_RISK_KIND_BROAD_PROMETHEUS_SELECTOR: &str = "broad-prometheus-selector";
pub(crate) const GOVERNANCE_RISK_KIND_PROMETHEUS_REGEX_HEAVY: &str = "prometheus-regex-heavy";
pub(crate) const GOVERNANCE_RISK_KIND_PROMETHEUS_HIGH_CARDINALITY_REGEX: &str =
    "prometheus-high-cardinality-regex";
pub(crate) const GOVERNANCE_RISK_KIND_PROMETHEUS_DEEP_AGGREGATION: &str =
    "prometheus-deep-aggregation";
pub(crate) const GOVERNANCE_RISK_KIND_LARGE_PROMETHEUS_RANGE: &str = "large-prometheus-range";
pub(crate) const GOVERNANCE_RISK_KIND_UNSCOPED_LOKI_SEARCH: &str = "unscoped-loki-search";
pub(crate) const GOVERNANCE_RISK_KIND_DASHBOARD_PANEL_PRESSURE: &str = "dashboard-panel-pressure";
pub(crate) const GOVERNANCE_RISK_KIND_DASHBOARD_REFRESH_PRESSURE: &str =
    "dashboard-refresh-pressure";

const GOVERNANCE_RISK_DEFAULT_SPEC: GovernanceRiskSpec = GovernanceRiskSpec {
    category: "other",
    severity: "low",
    recommendation:
        "Review this governance finding and assign a follow-up owner if action is needed.",
};

const GOVERNANCE_RISK_SPECS: [(&str, GovernanceRiskSpec); 13] = [
    (
        GOVERNANCE_RISK_KIND_MIXED_DASHBOARD,
        GovernanceRiskSpec {
            category: "topology",
            severity: "medium",
            recommendation:
                "Split panel queries by datasource or document why mixed datasource composition is required.",
        },
    ),
    (
        GOVERNANCE_RISK_KIND_ORPHANED_DATASOURCE,
        GovernanceRiskSpec {
            category: "inventory",
            severity: "low",
            recommendation:
                "Remove the unused datasource or reattach it to retained dashboards before the next cleanup cycle.",
        },
    ),
    (
        GOVERNANCE_RISK_KIND_UNKNOWN_DATASOURCE_FAMILY,
        GovernanceRiskSpec {
            category: "coverage",
            severity: "medium",
            recommendation:
                "Map this datasource plugin type to a known governance family or extend analyzer support for it.",
        },
    ),
    (
        GOVERNANCE_RISK_KIND_EMPTY_QUERY_ANALYSIS,
        GovernanceRiskSpec {
            category: "coverage",
            severity: "low",
            recommendation:
                "Review the query text and extend analyzer coverage if this datasource family should emit governance signals.",
        },
    ),
    (
        GOVERNANCE_RISK_KIND_BROAD_LOKI_SELECTOR,
        GovernanceRiskSpec {
            category: "cost",
            severity: "medium",
            recommendation:
                "Narrow the Loki stream selector before running expensive line filters or aggregations.",
        },
    ),
    (
        GOVERNANCE_RISK_KIND_BROAD_PROMETHEUS_SELECTOR,
        GovernanceRiskSpec {
            category: "cost",
            severity: "medium",
            recommendation:
                "Add label filters to the Prometheus selector before promoting this dashboard to shared or high-refresh use.",
        },
    ),
    (
        GOVERNANCE_RISK_KIND_PROMETHEUS_REGEX_HEAVY,
        GovernanceRiskSpec {
            category: "cost",
            severity: "medium",
            recommendation:
                "Reduce Prometheus regex matcher scope or replace it with exact labels where possible.",
        },
    ),
    (
        GOVERNANCE_RISK_KIND_PROMETHEUS_HIGH_CARDINALITY_REGEX,
        GovernanceRiskSpec {
            category: "cost",
            severity: "high",
            recommendation:
                "Avoid regex matchers on high-cardinality Prometheus labels such as instance, pod, or container unless the scope is already tightly bounded.",
        },
    ),
    (
        GOVERNANCE_RISK_KIND_PROMETHEUS_DEEP_AGGREGATION,
        GovernanceRiskSpec {
            category: "cost",
            severity: "medium",
            recommendation:
                "Reduce nested Prometheus aggregation layers or pre-aggregate upstream before adding more dashboard fanout.",
        },
    ),
    (
        GOVERNANCE_RISK_KIND_LARGE_PROMETHEUS_RANGE,
        GovernanceRiskSpec {
            category: "cost",
            severity: "medium",
            recommendation:
                "Shorten the Prometheus range window or pre-aggregate the series before using long lookback queries in dashboards.",
        },
    ),
    (
        GOVERNANCE_RISK_KIND_UNSCOPED_LOKI_SEARCH,
        GovernanceRiskSpec {
            category: "cost",
            severity: "high",
            recommendation:
                "Add at least one concrete Loki label matcher before running full-text or regex log search.",
        },
    ),
    (
        GOVERNANCE_RISK_KIND_DASHBOARD_PANEL_PRESSURE,
        GovernanceRiskSpec {
            category: "dashboard-load",
            severity: "medium",
            recommendation:
                "Split the dashboard into smaller views or collapse low-value panels before broad rollout.",
        },
    ),
    (
        GOVERNANCE_RISK_KIND_DASHBOARD_REFRESH_PRESSURE,
        GovernanceRiskSpec {
            category: "dashboard-load",
            severity: "medium",
            recommendation:
                "Increase the dashboard refresh interval to reduce repeated load on Grafana and backing datasources.",
        },
    ),
];

pub(crate) fn severity_for_score(score: usize) -> String {
    match score {
        0..=1 => "low".to_string(),
        2..=3 => "medium".to_string(),
        _ => "high".to_string(),
    }
}

pub(crate) fn ordered_push(values: &mut Vec<String>, candidate: &str) {
    let candidate = candidate.trim();
    if candidate.is_empty() {
        return;
    }
    if !values.iter().any(|value| value == candidate) {
        values.push(candidate.to_string());
    }
}

pub(crate) fn parse_duration_seconds(value: &str) -> Option<u64> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("off") {
        return None;
    }
    let mut digits = String::new();
    let mut suffix = String::new();
    for character in trimmed.chars() {
        if character.is_ascii_digit() && suffix.is_empty() {
            digits.push(character);
        } else if !character.is_ascii_whitespace() {
            suffix.push(character);
        }
    }
    let number = digits.parse::<u64>().ok()?;
    let multiplier = match suffix.to_ascii_lowercase().as_str() {
        "ms" => 0,
        "s" | "" => 1,
        "m" => 60,
        "h" => 60 * 60,
        "d" => 60 * 60 * 24,
        "w" => 60 * 60 * 24 * 7,
        _ => return None,
    };
    Some(number.saturating_mul(multiplier))
}

pub(crate) fn lookup_governance_risk_spec(kind: &str) -> GovernanceRiskSpec {
    GOVERNANCE_RISK_SPECS
        .iter()
        .copied()
        .find(|(registered_kind, _)| *registered_kind == kind)
        .map(|(_, spec)| spec)
        .unwrap_or(GOVERNANCE_RISK_DEFAULT_SPEC)
}

#[cfg(test)]
pub(crate) fn governance_risk_spec(kind: &str) -> (&'static str, &'static str, &'static str) {
    let spec = lookup_governance_risk_spec(kind);
    (spec.category, spec.severity, spec.recommendation)
}
