# ai-status-archive-2026-04-14

## 2026-04-13 - Reduce sync maintainability hotspots
- State: Done
- Scope: sync bundle preflight, promotion preflight, workspace discovery rules, source-bundle input loading, Rust maintainability reporting, and architecture guardrail notes.
- Baseline: `sync/bundle_preflight.rs`, `sync/promotion_preflight.rs`, `sync/workspace_discovery.rs`, and `sync/bundle_inputs.rs` mixed document assembly, mapping/discovery rules, rendering, file loading, and normalization helpers in large files; the maintainability reporter listed only file-level findings, so domain-level sync growth was harder to see.
- Current Update: split bundle preflight assessments, promotion preflight checks/mapping/rendering, workspace discovery path rules, and source-bundle input loading into focused modules; converted alert artifact, promotion remap, alert export section, and alert sync-kind differences into small rule/spec structures instead of scattered per-case branches; added a shared source-bundle input pipeline and directory summaries in the maintainability reporter.
- Result: public CLI and JSON/text contracts are unchanged; focused sync tests, reporter tests, formatting, and static checks pass locally. The remaining sync hotspots are now other production/test domains rather than the preflight/discovery/bundle-input facades.
