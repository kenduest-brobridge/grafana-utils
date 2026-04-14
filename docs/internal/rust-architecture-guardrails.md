# Rust Architecture Guardrails

Use this page when you are changing Rust code and need a short rule set that
keeps module boundaries, refactors, and review expectations consistent.

## Layers

Keep each layer focused on one job:

- `CLI topology`
  - owns clap shape, namespace wiring, and help entrypoints
  - does not own workflow logic, transport details, or file discovery
- `dispatch spine`
  - routes parsed commands into the owning domain runtime
  - does not interpret contracts, render output, or perform business work
- `domain runtime`
  - wires auth, client construction, and top-level execution flow
  - does not absorb render code or typed contract ownership
- `workflow/service`
  - owns the real task: fetch, mutate, compare, export, import, or validate
  - does not become a second CLI parser or a second help system
- `contract/model`
  - owns typed envelopes, stable fields, compatibility rules, and schema shape
  - does not perform ad hoc IO or UI rendering
- `render`
  - turns model data into human-readable output
  - does not re-discover data or mutate contract state
- `fixtures`
  - holds validation data, golden cases, and sample inputs
  - does not become a hidden production scratch area

## Supported Rust Feature Matrix

The maintained Rust release surfaces are intentionally narrow:

- default features: TUI-enabled CLI, without browser screenshot dependencies
- `--features browser`: default surface plus browser screenshot support

`--no-default-features` is not a supported release surface. It currently
removes the TUI dependencies while several TUI modules and tests remain part of
the crate-level test graph, so it must not be advertised as a lean supported
artifact until the TUI code and tests are split behind a complete feature
boundary.

Use `make quality-rust-feature-matrix` when touching Cargo features, release
artifact policy, browser support, or TUI gating. Use `make quality-architecture`
for the faster policy drift check that prevents accidental no-default claims.

## Anti-Patterns

Treat these as refactor signals:

- one file owns topology, runtime, and rendering together
- `mod.rs` becomes a junk drawer instead of a thin facade
- command code re-creates raw `/api/...` ownership outside the workflow helper
- help text is built deep inside runtime paths instead of through shared help wiring
- tracked artifacts or scratch exports land in source roots
- tests only assert giant full-string snapshots when a smaller semantic check is enough

## When To Split A Large File

Split by responsibility, not by line count alone. A file is ready to split when
it starts to:

- change in two or more unrelated directions
- own both data shape and IO
- mix parser/dispatch concerns with domain behavior
- mix render formatting with live API or file discovery
- accumulate command-specific branches that are hard to test independently

Prefer a small set of new modules that match the responsibility boundary.
Keep the parent file as a routing facade, re-export surface, or assembly point.

## Hotspot Order

When you need to reduce risk or prepare a larger refactor, work in this order:

1. remove misplaced artifacts from source roots
2. thin out facade modules such as `mod.rs`
3. split command/runtime helpers that mix topology and workflow logic
4. separate render code from business logic
5. split large test files once production boundaries are clearer

Current high-value refactor candidates usually follow that order in this repo:

- `alert_support.rs`
- `alert_cli_defs.rs`
- `dashboard/history.rs`
- `dashboard/browse_input.rs`
- datasource import/export helpers
- large CLI/help tests

Already handled and no longer current candidates:

- `sync/mod.rs`
- `sync/bundle_preflight.rs`
- `sync/promotion_preflight.rs`
- `sync/workspace_discovery.rs`
- `sync/bundle_inputs.rs`

Later candidates from the current architecture report may also include:

- `alert.rs`
- `access/render.rs`
- `dashboard/import_lookup.rs`
- `datasource_import_export.rs`
- `datasource_import_export_support.rs`
- `datasource_export_support.rs`

## Architecture Lint

Use the architecture lint as an early review gate, not as a cleanup step.

- run it in report mode first when you are still shaping a change
- use directory summaries when a domain like `sync/` needs a totals view, for example `python3 scripts/rust_maintainability_report.py --root rust/src --summary-root rust/src/sync --summary-hotspots 3`
- treat new root artifacts, layer breaks, and facade bloat as high-signal findings
- keep size-only warnings as triage input unless the module also violates a layer rule
- read directory summaries as well as single-file hotspots; a domain like `sync/` can be risky even when `mod.rs` is no longer the largest file
- keep handled modules out of the current candidate list; once a hotspot is split, remove it from the report and guardrails instead of keeping a permanent allowlist
- rerun it after a refactor that changes ownership boundaries or module layout

If a warning points to a file that mixes responsibilities, fix the boundary first
and let the module size fall as a consequence.
