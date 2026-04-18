//! Datasource reconcile plan model, builder, and renderers.
//!
//! The builder returns a pure plan document so CLI renderers and future TUI
//! views can consume the same stable action model.

mod builder;
mod model;
mod render;

#[cfg(test)]
mod tests;

pub(crate) use builder::build_datasource_plan;
pub(crate) use model::{DatasourcePlanInput, DatasourcePlanOrgInput};
pub(crate) use render::{datasource_plan_column_ids, print_datasource_plan_report};
