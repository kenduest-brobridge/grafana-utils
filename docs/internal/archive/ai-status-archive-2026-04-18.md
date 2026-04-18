# ai-status-archive-2026-04-18

## 2026-04-15 - Make help-flat terminal-safe
- State: Done
- Scope: Rust `--help-flat` rendering and focused help regression tests. Runtime command behavior, public docs, README files, and Python implementation are out of scope.
- Baseline: `grafana-util --help-flat` renders a padded table with long purpose text, so narrow terminals wrap rows and make the flat command inventory hard to scan.
- Current Update: Changed the flat inventory to one public command path per line and removed KIND/PURPOSE columns so terminal output stays readable; detailed purpose and flags remain available through `<COMMAND> --help`.
- Result: Focused help tests and formatting pass; manual `--help-flat` smoke output now shows one command path per line without columns or ellipses.

## 2026-04-16 - Mirror dashboard export folder paths
- State: Done
- Scope: Rust dashboard export path assembly, focused dashboard export tests, dashboard export command docs/help, and AI trace docs. README files, Python implementation, and provisioning layout are out of scope.
- Baseline: non-flat `dashboard export` writes raw and prompt files under the dashboard leaf `folderTitle`, even when Grafana reports a nested folder path through folder inventory. This can flatten distinct `Platform / Infra` and `Apps / Infra` folders into the same `Infra/` export directory.
- Current Update: Raw and prompt export paths now use the collected folder inventory full path when available; `--flat` and provisioning layout continue to use their previous behavior.
- Result: Focused dashboard export tests, full Rust tests, clippy, generated docs, docs-surface checks, formatting, AI workflow, and whitespace checks pass.

## 2026-04-16 - Add dashboard export layout repair
- State: Done
- Scope: Rust dashboard convert CLI surface, local export-layout repair planner/executor, dashboard export index metadata, focused parser/runtime/export tests, command docs/generated docs, command-surface contracts, and AI trace docs. README files, Python implementation, and provisioning layout repair are out of scope.
- Baseline: older dashboard exports can have correct `raw/folders.json` metadata while `raw/` and `prompt/` files are flattened under the leaf `folderTitle`; raw dashboard JSON may also omit `meta.folderUid`, so repair needs stable artifact metadata instead of relying on the dashboard payload alone.
- Current Update: Added `dashboard convert export-layout` with dry-run planning, copy-mode repair, in-place repair with backups, raw/prompt variant selection, folder inventory lookup, updated index paths, report-style summary text/table/csv output with `--show-operations` per-dashboard details, `extraFiles` reporting for unindexed files, case-insensitive/canonical path handling, and index-level `folderUid`/`folderPath` metadata for new exports.
- Result: Focused export-layout/dashboard export index tests, full Rust tests, clippy, docs surface, generated-doc checks, AI workflow, formatting, and whitespace checks pass. Real export smoke reports move=220, same=92, blocked=0, extra=0.

## 2026-04-17 - Align dashboard prompt external export semantics
- State: Done
- Scope: Rust prompt/raw-to-prompt datasource handling and pre-resolution, focused regression tests, internal prompt-lane semantics note, raw-to-prompt command docs in English and zh-TW, and AI trace docs. README files and Python implementation are out of scope.
- Current Update: Aligned prompt conversion with Grafana UI external-export semantics: concrete datasource refs become `__inputs`, built-in Grafana datasource selectors stay excluded, datasource variables keep their own variable shape, and the converter no longer synthesizes a new datasource variable when the source dashboard did not already have one. Used datasource variables with a concrete current datasource now point `current.value` at the generated `${DS_*}` input while panel and target references stay variable-based.
- Result: The prompt builder no longer over-normalizes single datasource families or turns datasource variable plugin filters into extra import inputs, and raw-to-prompt pre-resolution now preserves `$datasource` placeholders instead of failing inference. Focused raw-to-prompt coverage and full Rust tests pass; the docs record the contract, the known bug pattern, and the later `VAR_*` parity follow-up without naming local machine paths.

## 2026-04-17 - Harden datasource masked-recovery import
- State: Done
- Scope: Rust datasource masked-recovery export/import, dry-run evidence, focused datasource tests, datasource command docs, internal contract docs, TODO backlog, and AI trace docs. README files, Python implementation, and Grafana K8s datasource API support are out of scope.
- Current Update: Datasource export now preserves additive `readOnly`, `version`, and `apiVersion` evidence. Datasource import update planning now fetches target live datasource evidence, updates through `/api/datasources/uid/{uid}`, carries target `version`, and blocks read-only/provisioned targets before live writes. Dry-run JSON/table/text now exposes target UID/version/read-only and blocked reason evidence.
- Result: Focused datasource tests, full Rust tests, clippy, formatting, generated docs checks, docs-surface checks, AI workflow, and whitespace checks pass.

## 2026-04-17 - Finish classic prompt export guardrails
- State: Done
- Scope: Rust dashboard prompt/raw-to-prompt conversion, dashboard export validation, focused prompt conversion tests, maintainer prompt semantics docs, and AI trace docs. README files, Python implementation, and dashboard v2 export lane support are out of scope.
- Baseline: classic prompt export already preserves datasource variables and maps used datasource-variable current values, but constant variables are not exported as `VAR_*` inputs, dashboard v2-shaped files are not explicitly rejected by raw-to-prompt, and library panel references are not surfaced as portability warnings.
- Current Update: Added classic prompt parity for constant variables, kept expression datasource refs outside user-mapped inputs, rejected dashboard v2 resource/spec shapes in raw-to-prompt, and surfaced library panel portability warnings without adding a v2 export lane.
- Result: Focused raw-to-prompt and validation tests, full Rust tests, formatting, AI workflow, and whitespace checks pass.

## 2026-04-18 - Harden access user/team import preflight
- State: Done
- Scope: Rust access user/team import planning and dry-run output, focused access tests, access command docs, and AI trace docs. README files, Python implementation, and Grafana IAM/K8s API support are out of scope.
- Baseline: team import can pass login-style identities into Grafana's bulk team membership endpoint even though the official legacy endpoint resolves bulk members by email, and user/team dry-run does not consistently block externally synced users or provisioned teams before live apply.
- Current Update: Team import now resolves member/admin identities to live org-user emails before bulk apply, blocks missing-email identities, and surfaces provisioned-team blockers with target evidence. User import now blocks external/synced profile, org role, and Grafana-admin changes before apply while dry-run rows carry target evidence and blocked status.
- Result: Focused team/user import tests, access test suite, full Rust tests, clippy, formatting, generated docs, docs-surface, man/html checks, AI workflow, and whitespace checks pass.

## 2026-04-18 - Extend access preflight from Grafana source
- State: Done
- Scope: Rust access service-account import/token workflows, user modify guardrails, org import user-role preflight, focused access tests, access command docs, generated docs, and AI trace docs. README files and Python implementation are out of scope.
- Baseline: user/team import preflight is hardened, but service-account import still has an older dry-run shape, user modify can partially write before later Grafana-source blockers, org import dry-run does not inspect live users for role plans, and token add surfaces Grafana TTL/config failures as raw API errors.
- Current Update: Service-account import now validates roles and emits status/blocked/target evidence; user modify preflights external/provisioned/synced blockers before writes; org import dry-run checks live org users and blocks externally synced role changes; token add preserves Grafana errors while adding targeted TTL/duplicate-name hints.
- Result: Focused access tests, full Rust tests, clippy, formatting, generated docs, docs-surface, man/html checks, AI workflow, and whitespace checks pass.

## 2026-04-18 - Align dashboard export/import with Grafana source
- State: Done
- Scope: Rust dashboard prompt export, raw-to-prompt fixture parity tests, import/publish target preflight evidence, dashboard command docs, generated docs, maintainer contract notes, and AI trace docs. README files, Python implementation, and dashboard v2 support are out of scope.
- Baseline: prompt export always emitted empty `__elements`, raw-to-prompt parity coverage did not include Grafana-source datasource/library/v2 fixture shapes, and dashboard import/publish did not surface or block provisioned target dashboard ownership before live overwrite.
- Current Update: Live dashboard export now fetches referenced library panel models into prompt `__elements` while offline raw-to-prompt remains warning-only. Raw-to-prompt tests now cover selected datasource variables, default datasource variables, special datasource refs, string datasource mappings, library panel warnings, and v2/k8s rejection. Dashboard import/publish dry-run surfaces target evidence and live apply blocks provisioned overwrite before POST.
- Result: Focused dashboard tests, full Rust tests, clippy, formatting, docs-surface, generated man/html checks, and whitespace checks pass.

## 2026-04-18 - Repair legacy dashboard all-orgs root aggregates
- State: Done
- Scope: Rust dashboard export-layout repair, all-orgs export regression coverage, command docs, generated docs, and AI trace docs. README files, Python implementation, and live export behavior beyond regression coverage are out of scope.
- Baseline: older all-orgs export artifacts can contain valid `org_*` child exports while the root `index.json` and `export-metadata.json` only describe one org. `export-layout` repairs file layout and existing root index paths, but it does not rebuild missing all-orgs aggregate root data from child org indexes.
- Current Update: `dashboard convert export-layout` now rebuilds legacy all-orgs root `index.json` and root metadata from child `org_*/raw` and `org_*/prompt` indexes, even when no layout moves are possible because legacy folder identity is missing. The current exporter regression now asserts all-orgs root metadata uses all-orgs scope and does not carry a single root `org/orgId`.
- Result: Focused export-layout and all-orgs export tests pass; a copied legacy sample rebuilds root aggregate metadata to 138 dashboards across 2 orgs.
