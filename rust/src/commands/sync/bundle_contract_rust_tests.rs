//! Sync bundle document/render/preflight contract test facade.
//! Keeps the source-bundle document contract and bundle-preflight contract checks split.
#![allow(unused_imports)]

#[cfg(test)]
#[path = "bundle_contract_source_bundle_rust_tests.rs"]
mod sync_bundle_contract_source_bundle_rust_tests;

#[cfg(test)]
#[path = "bundle_contract_preflight_rust_tests.rs"]
mod sync_bundle_contract_preflight_rust_tests;

#[cfg(test)]
#[path = "promotion_preflight_rust_tests.rs"]
mod sync_promotion_preflight_rust_tests;
