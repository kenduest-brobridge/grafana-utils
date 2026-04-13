//! Inspect-live export parity regression tests.
//! Keeps live/export contract checks for core and all-orgs cases separate from the
//! large dashboard test file.

#[cfg(test)]
#[path = "inspect_live_export_parity_core_family_rust_tests.rs"]
mod inspect_live_export_parity_core_family_rust_tests;

#[cfg(test)]
#[path = "inspect_live_export_parity_all_orgs_rust_tests.rs"]
mod inspect_live_export_parity_all_orgs_rust_tests;
