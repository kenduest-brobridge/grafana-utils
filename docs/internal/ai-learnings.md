# ai-learnings.md

## 2026-04-06 - keyring needs explicit backend features on macOS
- Mistake/Symptom: a macOS Keychain compatibility smoke failed because `SystemOsSecretStore` could not read an item that had just been written with the `security` CLI.
- Root Cause: `keyring = "3"` was added without `apple-native`, and the crate falls back to its `mock` backend on macOS when that feature is absent.
- Fix: enable `keyring` with `features = ["apple-native"]` on the macOS target and keep a manual ignored smoke that checks `security` CLI interoperability.
- Prevention: when adopting `keyring`, always verify the target-specific backend feature set instead of assuming the real platform store is enabled by default.
- Keywords: keyring apple-native macos keychain mock backend security-framework profile_secret_store
- Refs: `rust/Cargo.toml`, `rust/src/profile_secret_store.rs`, `/Users/kendlee/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/keyring-3.6.3/src/lib.rs`
