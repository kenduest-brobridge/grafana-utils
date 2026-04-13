//! Snapshot review helpers split into shared validation, text rendering,
//! tabular output, and interactive browser shaping.

#[path = "browser.rs"]
mod browser;
#[path = "common.rs"]
mod common;
#[path = "output.rs"]
mod output;
#[path = "render.rs"]
mod render;

#[cfg(test)]
pub(crate) use self::browser::build_snapshot_review_browser_items;
pub(crate) use self::output::emit_snapshot_review_output;
#[cfg(test)]
pub(crate) use self::render::build_snapshot_review_summary_lines;
pub use self::render::render_snapshot_review_text;
