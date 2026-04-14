# ai-changes-archive-2026-04-15

## 2026-04-13 - Add shell completion command
- Summary: added `grafana-util completion bash|zsh`, implemented completion rendering through `clap_complete` from the unified Clap command tree, and routed the command through the existing CLI dispatch spine without entering Grafana runtime/auth paths.
- Tests: added parser coverage for Bash/Zsh and unsupported shell rejection, plus render coverage that completion scripts include common root commands from the unified CLI tree.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all`; `cargo test --manifest-path rust/Cargo.toml --quiet completion -- --test-threads=1`; `make man`; `make html`.
- Impact: `rust/src/cli/mod.rs`, `rust/src/cli/dispatch.rs`, new `rust/src/cli/completion.rs`, Rust CLI tests, `rust/Cargo.toml`, `rust/Cargo.lock`, README files, command docs/contracts, generated `docs/man/`, generated `docs/html/`, and AI trace docs.
- Rollback/Risk: low. Completion generation is read-only and Clap-backed; rollback removes the root `completion` command, dependency, docs, and generated completion man/html pages.
- Follow-up: none.
