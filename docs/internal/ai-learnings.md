# ai-learnings.md

## 2026-04-18 - Fix Rust 1.95 sync review clippy failure
- Mistake/Symptom: local `cargo clippy --all-targets -- -D warnings` passed, but GitHub Actions `rust-quality` failed on the same branch with many `clippy::collapsible_match` errors in `sync/review_tui.rs`.
- Root Cause: local stable was Rust 1.94.1 while CI installed Rust 1.95.0, which promoted or added stricter `collapsible_match` diagnostics for `match key.code` arms containing only nested `if diff_mode` checks.
- Fix: rewrite the key handling as guarded match arms such as `KeyCode::Up if diff_mode` and fallback non-diff arms, preserving behavior while satisfying Rust 1.95 clippy.
- Prevention: when CI fails only on clippy after tests pass, compare the local and CI Rust versions before assuming the code path is untested locally.
- Keywords: Rust 1.95 clippy collapsible_match GitHub Actions rust-quality review_tui diff_mode match guard
- Refs: `rust/src/commands/sync/review_tui.rs`, `cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings`

## 2026-04-16 - Dashboard export layout extra-file detection
- Mistake/Symptom: real export-layout dry-run initially reported four `extraFiles` for dashboards whose index paths used `ARCHIVED` while the filesystem entry was `Archived`.
- Root Cause: the planner compared index-relative paths as strings. On a case-insensitive macOS filesystem, `ARCHIVED/...` and `Archived/...` can refer to the same file, so string comparison produced false unindexed-file findings.
- Fix: treat a discovered file as indexed when the exact relative path matches or when the indexed path and discovered path canonicalize to the same filesystem path.
- Prevention: when validating local export artifact membership, compare canonical paths in addition to serialized index path strings before reporting an operator-facing extra-file warning.
- Keywords: dashboard export-layout extraFiles case-insensitive macOS canonicalize index path Archived ARCHIVED
- Refs: `rust/src/commands/dashboard/export_layout.rs`, `/Users/kendlee/work/scsb/grafana-dashboard/scsb-dev/dashboards`

## 2026-04-12 - Infer unique long option prefixes
- Mistake/Symptom: repeated focused Rust test attempts used invalid multiple `cargo test` filters, then Cargo printed `error: test failed, to rerun pass --lib` after normal lib-test failures, creating noisy and misleading iteration.
- Root Cause: `cargo test` accepts at most one positional test filter before `--`; Cargo's `--lib` line is a rerun hint for the failed target, not a diagnostic or a better next command.
- Fix: use one broad, intentional filter such as `long_option` or a full suite target; treat `to rerun pass --lib` as informational unless narrowing to the lib target is specifically useful.
- Prevention: before running focused Rust tests, choose one filter string or run the owning suite; do not concatenate multiple test names in one `cargo test` command.
- Keywords: cargo test multiple filters --lib rerun hint focused tests Rust test filter
- Refs: `cargo test --manifest-path rust/Cargo.toml --quiet long_option -- --test-threads=1`

## 2026-04-06 - keyring needs explicit backend features on macOS
- Mistake/Symptom: a macOS Keychain compatibility smoke failed because `SystemOsSecretStore` could not read an item that had just been written with the `security` CLI.
- Root Cause: `keyring = "3"` was added without `apple-native`, and the crate falls back to its `mock` backend on macOS when that feature is absent.
- Fix: enable `keyring` with `features = ["apple-native"]` on the macOS target and keep a manual ignored smoke that checks `security` CLI interoperability.
- Prevention: when adopting `keyring`, always verify the target-specific backend feature set instead of assuming the real platform store is enabled by default.
- Keywords: keyring apple-native macos keychain mock backend security-framework profile_secret_store
- Refs: `rust/Cargo.toml`, `rust/src/commands/config/profile/secret_store.rs`, `<cargo-home>/registry/src/index.crates.io-1949cf8c6b5b557f/keyring-3.6.3/src/lib.rs`
