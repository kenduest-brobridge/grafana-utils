# Generated Docs Maintainer Playbook

Use this playbook when you need to change the generated docs system without re-learning the whole design from scratch.

For the architecture and rationale, read [`generated-docs-architecture.md`](/Users/kendlee/work/grafana-utils/docs/internal/generated-docs-architecture.md) first. This file is the practical companion: what to edit, in what order, and how to validate it.

## Fast Map

If you need to:

- add or edit handbook content: work under `docs/user-guide/`
- add or edit command reference content: work under `docs/commands/`
- change handbook ordering or prev/next navigation: edit `scripts/docgen_handbook.py`
- change command-to-handbook back-links in HTML: edit `scripts/generate_command_html.py`
- add or remove a namespace manpage: edit `scripts/generate_manpages.py`
- change supported Markdown behavior: edit `scripts/docgen_command_docs.py`
- change generated output write/check behavior: edit `scripts/docgen_common.py`

## Standard Validation Loop

After any docs-generator change:

```bash
make man
make html
make man-check
make html-check
python3 -m unittest -v \
  python.tests.test_python_generate_manpages \
  python.tests.test_python_generate_command_html
```

Useful local review commands:

```bash
man ./docs/man/grafana-util.1
open ./docs/html/index.html
```

On Linux, replace `open` with `xdg-open`.

## Task: Add A New Command Reference Page

Example shape:

- new `grafana-util dashboard something`
- new `grafana-util alert something`

Steps:

1. add the English source page under `docs/commands/en/`
2. add the Traditional Chinese source page under `docs/commands/zh-TW/`
3. keep the filename mirrored across locales
4. follow the existing command-doc schema:
   - `#` page title
   - `## Root` or supported section structure
   - `Purpose`
   - `When to use`
   - `Key flags`
   - `Examples`
5. add links from the locale command indexes if needed:
   - `docs/commands/en/index.md`
   - `docs/commands/zh-TW/index.md`
6. regenerate HTML
7. decide whether the new command should appear in a namespace manpage

When it also needs manpage coverage:

- update the matching `NamespaceSpec` in [generate_manpages.py](/Users/kendlee/work/grafana-utils/scripts/generate_manpages.py)
- add the new Markdown filename to `sub_docs`

If the new command should link back to a handbook chapter in HTML:

- update `HANDBOOK_CONTEXT_BY_COMMAND` in [generate_command_html.py](/Users/kendlee/work/grafana-utils/scripts/generate_command_html.py)

## Task: Add A New Handbook Chapter

Steps:

1. add the English chapter under `docs/user-guide/en/`
2. add the Traditional Chinese chapter under `docs/user-guide/zh-TW/`
3. use the same filename in both locales
4. add the filename to `HANDBOOK_ORDER` in [docgen_handbook.py](/Users/kendlee/work/grafana-utils/scripts/docgen_handbook.py)
5. update handbook index pages if the new chapter should be surfaced explicitly:
   - `docs/user-guide/en/index.md`
   - `docs/user-guide/zh-TW/index.md`
6. regenerate HTML
7. check previous/next navigation in both locales

If command pages should link back to this chapter:

- update `HANDBOOK_CONTEXT_BY_COMMAND` in [generate_command_html.py](/Users/kendlee/work/grafana-utils/scripts/generate_command_html.py)

## Task: Add A New Namespace Manpage

This is for a new top-level family such as `grafana-util foo`, not every single subcommand.

Steps:

1. confirm the namespace has an English root command doc under `docs/commands/en/`
2. add a `NamespaceSpec` entry to `NAMESPACE_SPECS` in [generate_manpages.py](/Users/kendlee/work/grafana-utils/scripts/generate_manpages.py)
3. set:
   - `stem`
   - `cli_path`
   - `title`
   - `root_doc`
   - optional `aliases`
   - optional `sub_docs`
   - `related_manpages`
   - optional `workflow_notes`
4. regenerate manpages
5. inspect the new `SEE ALSO` section
6. if useful, add README or command-index links to the new manpage family

Notes:

- manpages currently generate from English command docs only
- namespace manpages are curated summaries, not one page per subcommand

## Task: Remove Or Rename A Command Page

This is the change most likely to break generated links.

Checklist:

1. rename or remove the Markdown file in both locales
2. update locale command indexes
3. update any `sub_docs` references in `NAMESPACE_SPECS`
4. update `HANDBOOK_CONTEXT_BY_COMMAND` if the stem changed
5. update handbook pages that deep-link to the old filename
6. regenerate HTML and manpages
7. review for broken navigation or missing generated pages

## Task: Change HTML Layout Or Theme

Most layout and visual changes belong in [generate_command_html.py](/Users/kendlee/work/grafana-utils/scripts/generate_command_html.py).

Common places:

- page shell and shared layout
- landing page
- theme toggle
- sidebar structure
- breadcrumbs
- handbook and command navigation blocks

When doing layout work:

1. regenerate HTML
2. open `docs/html/index.html`
3. inspect at least:
   - landing page
   - one handbook page in English
   - one handbook page in `zh-TW`
   - one command page in English
   - one command page in `zh-TW`
4. verify `Auto`, `Light`, and `Dark` theme modes

Do not add a second HTML renderer for Pages only. Pages must publish the same `docs/html/` tree used locally.

## Task: Add A New Markdown Feature

Examples:

- new table style
- new inline syntax
- new heading behavior
- different code-block handling

Where to change:

- [docgen_command_docs.py](/Users/kendlee/work/grafana-utils/scripts/docgen_command_docs.py)

Process:

1. decide whether the feature should be supported repo-wide or not supported at all
2. implement it once in the shared Markdown subset renderer
3. keep the behavior simple and deterministic
4. regenerate HTML
5. rerun both determinism tests

Do not patch one generator around the parser unless the behavior is truly output-specific.

## Task: Add A New Command Locale

Current supported HTML command locales are explicit in [generate_command_html.py](/Users/kendlee/work/grafana-utils/scripts/generate_command_html.py) via `COMMAND_DOC_LOCALES`.

Steps:

1. add the locale tree under `docs/commands/<locale>/`
2. add the locale to `COMMAND_DOC_LOCALES`
3. update locale switch and label handling as needed
4. regenerate HTML
5. verify the locale command index and page switching

Handbook locales are separate and controlled by `HANDBOOK_LOCALES` and `LOCALE_LABELS` in [docgen_handbook.py](/Users/kendlee/work/grafana-utils/scripts/docgen_handbook.py).

Do not assume adding one locale automatically wires the other layer.

## Task: Change Generated Output Inventory

Examples:

- add a new special HTML file
- add a new top-level generated entrypoint
- change what `html-check` or `man-check` should validate

Likely files:

- [docgen_common.py](/Users/kendlee/work/grafana-utils/scripts/docgen_common.py)
- [generate_command_html.py](/Users/kendlee/work/grafana-utils/scripts/generate_command_html.py)
- [generate_manpages.py](/Users/kendlee/work/grafana-utils/scripts/generate_manpages.py)
- `python/tests/test_python_generate_*.py`

Checklist:

1. update the generator output map
2. update the short console summary if the output contract changed
3. update the determinism test to include the new file set
4. regenerate outputs
5. rerun checks and tests

Example:

- `.nojekyll` is part of the generated HTML output set even though it is not an `.html` file

## Task: Update GitHub Pages Behavior

Pages deployment is owned by [.github/workflows/docs-pages.yml](/Users/kendlee/work/grafana-utils/.github/workflows/docs-pages.yml).

Safe changes:

- trigger rules
- deployment permissions
- publishing `docs/html/`

Unsafe changes unless intentional:

- changing Pages to build a different HTML tree than `make html`
- adding Pages-only transforms that are not reflected in checked-in outputs

Rule:

- `make html` is the only supported HTML build path
- Pages must publish that exact tree

## Review Checklist Before Merging

- source Markdown changed instead of generated files only
- English and `zh-TW` filenames stay mirrored where expected
- handbook order is still explicit and correct
- namespace manpage specs still reference real English source files
- `HANDBOOK_CONTEXT_BY_COMMAND` still points command pages at the right handbook chapter
- `make man-check` passes
- `make html-check` passes
- deterministic unittest coverage passes
- HTML landing page opens
- representative manpage renders

## Common Scenarios

### New command docs exist in HTML but not in manpages

Expected if:

- the command was added in source docs but not attached to a namespace `sub_docs` list

Fix:

- update `NAMESPACE_SPECS` if the command should be surfaced in a namespace manpage

### HTML locale switch is missing or broken

Most likely causes:

- filenames differ between locales
- the locale was not added to the explicit locale tuple

Fix:

- restore mirrored filenames
- update locale metadata in the generator

### `make html-check` fails right after changing the generator

Most likely cause:

- checked-in generated files were not rewritten yet

Fix:

```bash
make html
make html-check
```

### A command page parses incorrectly

Most likely cause:

- the Markdown no longer matches the supported command-doc schema

Fix:

- align the page back to the schema
- or deliberately extend the parser in `docgen_command_docs.py`

## Keep These In Sync

When the generated-docs system changes materially, update:

- [`docs/internal/generated-docs-architecture.md`](/Users/kendlee/work/grafana-utils/docs/internal/generated-docs-architecture.md)
- this playbook
- `docs/DEVELOPER.md` if the maintainer entrypoint changed
- the determinism tests if the generated file set changed
