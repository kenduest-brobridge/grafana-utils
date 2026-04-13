//! Promotion-preflight checks for staged sync handoff.
//!
//! This module is now a facade over focused helper modules so the public
//! entrypoints stay stable while the summary, mapping, and rendering logic
//! remain easier to maintain.

#[path = "promotion_preflight_checks.rs"]
mod promotion_preflight_checks;
#[path = "promotion_preflight_mapping.rs"]
mod promotion_preflight_mapping;
#[path = "promotion_preflight_render.rs"]
mod promotion_preflight_render;

pub const SYNC_PROMOTION_PREFLIGHT_KIND: &str =
    promotion_preflight_checks::SYNC_PROMOTION_PREFLIGHT_KIND;
pub const SYNC_PROMOTION_PREFLIGHT_SCHEMA_VERSION: i64 =
    promotion_preflight_checks::SYNC_PROMOTION_PREFLIGHT_SCHEMA_VERSION;
pub const SYNC_PROMOTION_MAPPING_KIND: &str =
    promotion_preflight_checks::SYNC_PROMOTION_MAPPING_KIND;
pub const SYNC_PROMOTION_MAPPING_SCHEMA_VERSION: i64 =
    promotion_preflight_checks::SYNC_PROMOTION_MAPPING_SCHEMA_VERSION;

#[allow(unused_imports)]
pub(crate) use promotion_preflight_checks::SyncPromotionPreflightSummary;

use crate::common::Result;
use serde_json::Value;

pub fn build_sync_promotion_preflight_document(
    source_bundle: &Value,
    target_inventory: &Value,
    availability: Option<&Value>,
    mapping: Option<&Value>,
) -> Result<Value> {
    promotion_preflight_checks::build_sync_promotion_preflight_document(
        source_bundle,
        target_inventory,
        availability,
        mapping,
    )
}

pub fn render_sync_promotion_preflight_text(document: &Value) -> Result<Vec<String>> {
    promotion_preflight_render::render_sync_promotion_preflight_text(document)
}
