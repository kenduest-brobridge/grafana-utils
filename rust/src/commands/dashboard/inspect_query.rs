//! Query analysis and dispatch helpers for dashboard inspection.
//!
//! This module stays as the stable public facade while the family-specific registry
//! and dispatcher live in a dedicated sibling module.
#[path = "inspect_analyzer_registry.rs"]
mod inspect_analyzer_registry;

#[allow(unused_imports)]
pub(crate) use inspect_analyzer_registry::{
    dispatch_query_analysis, ordered_unique_push,
    resolve_query_analyzer_family_from_datasource_type,
    resolve_query_analyzer_family_from_query_signature, QueryAnalysis, QueryExtractionContext,
    DATASOURCE_FAMILY_FLUX, DATASOURCE_FAMILY_LOKI, DATASOURCE_FAMILY_PROMETHEUS,
    DATASOURCE_FAMILY_SEARCH, DATASOURCE_FAMILY_SQL, DATASOURCE_FAMILY_TRACING,
    DATASOURCE_FAMILY_UNKNOWN,
};
