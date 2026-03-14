# Datasource Live Mutation Safe Draft

## Purpose

Capture a stricter unwired draft that addresses review feedback without
modifying the first-pass scaffolding files.

## New Draft Modules

- `grafana_utils/datasource/live_mutation_safe.py`
- `grafana_utils/datasource/live_mutation_render_safe.py`

## Differences From First Draft

- `jsonData=None` and `secureJsonData=None` are omitted instead of becoming
  empty JSON objects.
- add dry-run / blocked actions distinguish:
  - `would-fail-existing-uid`
  - `would-fail-existing-name`
  - `would-fail-ambiguous-uid`
  - `would-fail-ambiguous-name`
  - `would-fail-uid-name-mismatch`
- delete dry-run / blocked actions also keep the more specific mismatch state.
- live execution prefers datasource-specific client helpers when available.
- dry-run render columns are validated before rendering.

## Status

This is still not wired into parser, facade, workflow dispatch, or user-facing
documentation.
