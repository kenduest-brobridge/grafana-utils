#![cfg(feature = "tui")]
use reqwest::Method;
use serde_json::Value;

use crate::common::Result;

use super::browse_input_shared::live_view_cache_key;
use crate::dashboard::browse_state::BrowserState;
use crate::dashboard::BrowseArgs;

pub(super) fn refresh_selected_dashboard_view<F>(
    request_json: &mut F,
    args: &BrowseArgs,
    state: &mut BrowserState,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if let Some(node) = state.selected_node().cloned() {
        if let Some(cache_key) = live_view_cache_key(&node) {
            state.live_view_cache.remove(&cache_key);
        }
    }
    super::ensure_selected_dashboard_view(request_json, args, state, true)
}
