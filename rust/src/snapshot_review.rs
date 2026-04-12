//! Snapshot review helpers split into shared validation, text rendering,
//! tabular output, and interactive browser shaping.

#[path = "snapshot_review_browser.rs"]
mod browser;
#[path = "snapshot_review_common.rs"]
mod common;
#[path = "snapshot_review_output.rs"]
mod output;
#[path = "snapshot_review_render.rs"]
mod render;

#[cfg(test)]
pub(crate) use self::browser::build_snapshot_review_browser_items;
pub(crate) use self::output::emit_snapshot_review_output;
#[cfg(test)]
pub(crate) use self::render::build_snapshot_review_summary_lines;
pub use self::render::render_snapshot_review_text;
