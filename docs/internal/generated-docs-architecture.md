# Generated Docs Architecture

This document explains how the repo's generated documentation works so a future maintainer can safely change it without reverse-engineering the generators first.

For task-by-task maintenance steps, read [`generated-docs-playbook.md`](generated-docs-playbook.md).

## Why This Exists

The repo now publishes the same user-facing command and handbook content in multiple shapes:

- Markdown source for GitHub and repo editing
- `man` pages for terminal lookup
- static HTML for handbook-style browsing
- GitHub Pages output for remote browsing

The design goal is not to build a generic docs engine. The goal is to keep the repo's actual documentation sources close to normal Markdown, then project them into a small set of repo-owned outputs with code that maintainers can read quickly.

## Source Of Truth

There are three human-maintained source layers:

- `docs/landing/{en,zh-TW}.md`
  - homepage entrypoint content
  - task grouping and landing-page copy
  - curated top-level links
- `docs/user-guide/{en,zh-TW}/`
  - handbook layer
  - ordered reading path
  - workflow guidance
  - conceptual context
- `docs/commands/{en,zh-TW}/`
  - command-reference layer
  - one page per command or subcommand
  - exact usage, key flags, examples, related commands

Generated artifacts are not the long-term truth source:

- `docs/man/*.1`
- `docs/html/`

The only exception is that the current `man` family is generated from `docs/commands/en/*.md` only. There is no separate Traditional Chinese manpage lane.

### Command Surface Contract

- `scripts/contracts/command-surface.json`
  - machine-readable public command-path and docs-routing contract
  - records legacy replacements and `--help-full` / `--help-flat` support
  - is the sync point for docs examples, maintainer routing, and command-page
    follow-up when public names change
- `scripts/contracts/docs-entrypoints.json`
  - machine-readable docs-entry taxonomy for landing quick commands, jump-select
    command entries, and handbook sidebar command shortcuts
  - lets maintainers change navigation content without editing Python logic

## Output Model

### Manpages

Source:

- `docs/commands/en/*.md`

Output:

- `docs/man/grafana-util.1`
- `docs/man/grafana-util-dashboard.1`
- `docs/man/grafana-util-alert.1`
- `docs/man/grafana-util-datasource.1`
- `docs/man/grafana-util-access.1`
- `docs/man/grafana-util-config.1`
- `docs/man/grafana-util-status.1`
- `docs/man/grafana-util-snapshot.1`
- `docs/man/grafana-util-*.1` for every generated subcommand manpage

Design intent:

- top-level quick reference
- namespace-level lookup pages
- per-subcommand lookup pages for exact command syntax and examples
- not a full handbook dump

### HTML Site

Source:

- `docs/landing/{en,zh-TW}.md`
- `docs/user-guide/{en,zh-TW}/*.md`
- `docs/commands/{en,zh-TW}/*.md`

Output:

- `docs/html/index.html`
- `docs/html/handbook/en/*.html`
- `docs/html/handbook/zh-TW/*.html`
- `docs/html/commands/en/*.html`
- `docs/html/commands/zh-TW/*.html`
- `docs/html/.nojekyll`

Design intent:

- handbook and command reference stay separate but cross-linked
- repo-local browsing works directly from checked-in HTML
- GitHub Pages can publish the same output tree without Jekyll rewriting

## Module Layout

The generator code is intentionally split by responsibility instead of hiding everything behind one large renderer.

### `scripts/docgen_common.py`

Shared boring plumbing:

- repo root resolution
- shared version lookup
- repo-relative link helper with `relative_href()`
- generated file write/check/report helpers

If a change is about "how generated files are written or checked", it belongs here.

### `scripts/docgen_command_docs.py`

Shared parser and Markdown subset renderer:

- splits command docs by `##` sections

### `scripts/docgen_entrypoints.py`

Shared docs-entry loader and validator:

- loads `scripts/contracts/docs-entrypoints.json`
- validates quick-command, jump-select, and handbook sidebar shortcut structure
- keeps docs-entry content in data files while generators stay focused on rendering
- extracts `Purpose`, `When to use`, `Key flags`, and `Examples`
- handles inline-subcommand pages such as `profile.md`
- renders the limited HTML subset used by both handbook and command pages

If a change is about "what Markdown constructs are supported" or "how command pages are structurally parsed", it belongs here.

### `scripts/docgen_handbook.py`

Handbook metadata only:

- supported locales
- explicit chapter order
- page titles
- previous/next navigation
- language-switch target resolution

The chapter order is intentionally explicit. Do not infer it from filesystem ordering.

### `scripts/docgen_landing.py`

Landing metadata only:

- supported landing locales
- landing Markdown source paths
- fixed landing-page Markdown contract
- task and link extraction from landing source files

The landing page intentionally has its own source layer so homepage copy and
section ordering do not have to be hardcoded into the HTML renderer.

### `scripts/generate_manpages.py`

Manpage projection only:

- defines the namespace manpage set with `NamespaceSpec`
- maps each namespace to the English command docs that feed it
- turns parsed command-doc fields into roff sections
- owns top-level, namespace-level, and per-subcommand `SEE ALSO` relationships

If a change is about "what manpages exist" or "what roff sections appear", it belongs here.

### `scripts/generate_command_html.py`

HTML site projection only:

- renders the docs landing page
- renders handbook pages
- renders command-reference pages
- applies handbook-to-command and command-to-handbook navigation
- fills file-backed HTML shell templates under `scripts/templates/`
- loads shared CSS and runtime JS from `scripts/templates/`
- emits `.nojekyll`

If a change is about layout, theme, navigation, GitHub Pages output shape, or
landing-page rendering, it belongs here. Shared shell markup now lives in
`scripts/templates/`, while `generate_command_html.py` prepares the view data
and fills those templates. Shared CSS and client-side behavior also live there
as file-backed assets. If a change is about homepage copy, task ordering,
or curated landing links, edit `docs/landing/` first.

## Why The Parsing Logic Is Small On Purpose

The generators do not use a full Markdown library. This is deliberate.

Reasons:

- maintainers can read the parsing rules in one file
- output is deterministic and repo-shaped
- the supported source schema stays narrow enough that manpage and HTML projections remain predictable
- there is less hidden behavior than a general Markdown pipeline would introduce

Tradeoff:

- not every Markdown feature is supported
- when adding a new content pattern, maintainers must decide whether it belongs in the supported subset first

## Supported Markdown Subset

The HTML generator supports the repo's current manual/reference writing style, not arbitrary GitHub-flavored Markdown.

Supported well:

- `#`, `##`, `###` headings
- paragraphs
- flat bullet lists
- fenced code blocks
- simple tables
- inline code
- emphasis and strong emphasis
- standard Markdown links

Important behavior:

- fenced code blocks stay intact as a single rendered code block
- generated manpages keep each example as one coherent command block
- source-relative `.md` links are rewritten to generated `.html` paths when needed

Unsupported or intentionally narrow:

- nested list semantics
- arbitrary HTML passthrough
- advanced Markdown extensions
- generic automatic heading tree inference outside the supported heading levels

When adding a new source pattern, update both:

- `scripts/docgen_command_docs.py`
- the determinism tests in `python/tests/`

## Command Doc Schema

Command docs are expected to be structured enough for generators to extract stable fields.

Current command-reference expectations:

- page title at `#`
- second-level sections such as `## Root`
- labeled fields under the root section or direct `## Purpose`-style sections, depending on the file
- stable labels for:
  - `Purpose`
  - `When to use`
  - `Key flags`
  - `Examples`

Why this matters:

- `man` output depends on extracting these sections consistently
- HTML command pages depend on the same fields for summaries and navigation sidebars

If a command page format changes, update the parser first instead of working around it in one output generator only.

## Handbook Model

Handbook pages are narrative, so they are rendered from Markdown more directly.

The key handbook-specific rules are:

- page ordering is defined by `HANDBOOK_ORDER` in `scripts/docgen_handbook.py`
- sidebar grouping is defined separately by `HANDBOOK_NAV_GROUPS` in `scripts/docgen_handbook.py`
- each locale must provide the same chapter set and filenames
- sidebar labels are title-driven, but group membership is metadata-driven
- the HTML handbook experience depends on the ordered sequence for previous/next controls

When adding a new handbook chapter:

1. add the Markdown file in both `en` and `zh-TW`
2. add its filename to `HANDBOOK_ORDER`
3. place it in the right `HANDBOOK_NAV_GROUPS` section
4. regenerate HTML
5. verify the chapter appears in both locale flows

## Locale Policy

Current locale policy is intentionally asymmetric:

- HTML handbook: English and Traditional Chinese
- HTML command reference: English and Traditional Chinese
- manpages: English only

This is a maintenance choice, not a renderer limitation. Manpages are currently optimized for the conventional English `man` surface.

Any future change to localized manpages should be treated as a documentation policy decision first, not just a renderer toggle.

## HTML Site Design Rules

The HTML site is a manual, not a bare Markdown dump.

Current design rules:

- one landing page at `docs/html/index.html`
- landing-page content is sourced from `docs/landing/{en,zh-TW}.md`
- the homepage renders one locale view at a time and lets browser language or a manual toggle choose between `en` and `zh-TW`
- two entry families:
  - handbook
  - command reference
- theme selector supports:
  - auto
  - light
  - dark
- command pages link back to relevant handbook context
- handbook pages link into command-reference pages
- generated relative links must work from checked-in files and from GitHub Pages hosting

Important HTML-specific files:

- `.nojekyll`
  - disables Jekyll processing on GitHub Pages
  - required because the generated site should be served as-is

## Manpage Design Rules

The manpage lane is intentionally curated rather than one page per every subcommand.

Current design rules:

- one top-level page for `grafana-util`
- one page per major namespace
- `SEE ALSO` sections should help operators move between related namespaces
- the content is command-reference oriented, not handbook-longform
- examples should stay compact and terminal-friendly

When adding a new major namespace:

1. add a `NamespaceSpec` entry in `scripts/generate_manpages.py`
2. point it at the root command doc and any subcommand docs
3. add related manpages
4. regenerate and review the new `SEE ALSO` paths

## Cross-Linking Rules

Cross-linking is explicit, not inferred from title similarity.

Important current rules:

- command-to-handbook links are controlled by `HANDBOOK_CONTEXT_BY_COMMAND` in `scripts/generate_command_html.py`
- handbook locale switching is controlled by the mirrored filename convention in `scripts/docgen_handbook.py`
- command reference locale switching assumes mirrored command source filenames under `docs/commands/{locale}/`

If you rename files, you must update these mappings or mirrored filenames at the same time.

## Maintainer Workflow

The expected maintenance loop is:

1. edit source Markdown under `docs/user-guide/` or `docs/commands/`
2. regenerate outputs
3. inspect the local HTML or `man` result
4. run determinism checks

Primary commands:

```bash
# Purpose: Primary commands.
make man
make man-check
make html
make html-check
```

Useful local entrypoints:

```bash
# Purpose: Useful local entrypoints.
man ./docs/man/grafana-util.1
open ./docs/html/index.html
```

On Linux, replace `open` with `xdg-open`.

## Tests And Validation

The repo uses deterministic output tests instead of fragile golden snippets.

Current tests:

- `python/tests/test_python_generate_manpages.py`
- `python/tests/test_python_generate_command_html.py`

What they check:

- generate outputs in memory
- compare them to checked-in generated files
- fail if the generated set or contents drift

This means generator work normally needs both steps:

```bash
# Purpose: This means generator work normally needs both steps.
python3 ./scripts/generate_manpages.py --write
python3 ./scripts/generate_command_html.py --write
python3 -m unittest -v \
  python.tests.test_python_generate_manpages \
  python.tests.test_python_generate_command_html
```

## GitHub Pages Deployment

Published HTML docs are deployed by:

- `.github/workflows/docs-pages.yml`

Current behavior:

- runs on pushes to `main`
- runs `scripts/build_pages_site.py`
- assembles `.nojekyll`, release lanes, the `latest/` lane, and the root `index.html` version portal
- includes the optional `dev/` lane when the workflow requests a preview build
- deploys the assembled Pages tree to GitHub Pages

The release, latest, and dev lane pages still come from
`generate_command_html.generate_outputs()` and the same Markdown/contracts used
by local `make html`. The root Pages portal is generated separately by
`scripts/docsite_version_portal.py` from
`scripts/contracts/versioned-docs-portal.json`; it is a version selector and
direct-output shortcut page, not the local landing page.

Do not introduce a custom renderer for Pages lane pages. Pages-only behavior
should stay limited to the root version portal and the lane assembly script.

## Common Maintenance Rules

- Prefer editing Markdown source, not generated HTML or roff output.
- Keep generator modules narrow; do not collapse them back into one large script.
- If you add a new supported Markdown feature, update the subset renderer once and reuse it from both handbook and command paths.
- Keep handbook order explicit.
- Keep locale symmetry explicit.
- Keep `make html` and `make man` output short and maintainer-readable.
- Keep examples profile-first where possible, then direct Basic auth, then token with caveat where token behavior is intentionally documented.
- Keep command examples and local links passing `make quality-docs-surface`.
- Update `scripts/contracts/command-surface.json` when public command paths,
  legacy replacements, command-doc routing, or `--help-full` / `--help-flat` support change.
- Update `scripts/contracts/versioned-docs-portal.json` and
  `scripts/docsite_version_portal.py` when changing the root GitHub Pages
  version portal copy, lane selection, or direct output links.

## Common Failure Modes

### `make html-check` or `make man-check` fails

Most common cause:

- source Markdown changed but generated outputs were not rewritten

Action:

- rerun `make html` or `make man`
- rerun the matching unittest

### A link works in Markdown but not in generated HTML

Most common causes:

- the source link points to a file that is not mirrored in the generated tree
- a handbook or command filename changed without updating the explicit mapping

Action:

- inspect link rewriting in `scripts/generate_command_html.py`
- inspect locale or chapter metadata in `scripts/docgen_handbook.py`

### A section shows up in HTML but not in manpages

Most common cause:

- the content is outside the structured fields extracted from command docs

Action:

- verify the command doc uses the expected labeled schema
- extend `parse_command_page()` or `parse_inline_subcommands()` only if the new field is meant to be stable across outputs

## What Not To Do

- Do not treat `docs/man/*.1` as hand-edited truth.
- Do not add a second Markdown parser just for one output.
- Do not make chapter order depend on filesystem sorting.
- Do not let English and `zh-TW` handbook filenames drift apart unless you also change the language-switch model.
- Do not add Pages-only HTML transforms inside generated handbook, command,
  or manpage lane pages.

## Short Decision Guide

If you are changing:

- output write/check behavior: edit `scripts/docgen_common.py`
- supported Markdown subset: edit `scripts/docgen_command_docs.py`
- handbook order or locale wiring: edit `scripts/docgen_handbook.py`
- manpage inventory or roff layout: edit `scripts/generate_manpages.py`
- HTML layout, landing page, or cross-linking: edit `scripts/generate_command_html.py`

Start there first instead of patching generated artifacts.
