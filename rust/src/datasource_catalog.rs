//! Datasource type catalog shared by datasource CLI surfaces.
//!
//! Purpose:
//! - Keep supported datasource categories and type ids centralized.
//! - Provide one stable scaffold for future datasource-specific validation and presets.

#[path = "datasource_catalog_data.rs"]
mod datasource_catalog_data;
#[path = "datasource_catalog_defaults.rs"]
mod datasource_catalog_defaults;
#[path = "datasource_catalog_lookup.rs"]
mod datasource_catalog_lookup;
#[path = "datasource_catalog_render.rs"]
mod datasource_catalog_render;

pub use datasource_catalog_data::{
    DatasourceCatalogEntry, DatasourceCatalogJsonDefaultValue, DatasourcePresetProfile,
};
pub use datasource_catalog_defaults::build_add_defaults_for_supported_type;
pub use datasource_catalog_lookup::{
    find_supported_datasource_entry, normalize_supported_datasource_type,
    supported_datasource_catalog,
};
pub use datasource_catalog_render::{
    render_supported_datasource_catalog_json, render_supported_datasource_catalog_text,
};
