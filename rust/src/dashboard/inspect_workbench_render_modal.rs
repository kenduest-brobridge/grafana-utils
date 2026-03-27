#![cfg(feature = "tui")]

#[path = "inspect_workbench_render_modal_sections.rs"]
mod inspect_workbench_render_modal_sections;

pub(crate) use inspect_workbench_render_modal_sections::{
    render_detail_panel, render_full_detail_viewer, render_search_prompt,
};
