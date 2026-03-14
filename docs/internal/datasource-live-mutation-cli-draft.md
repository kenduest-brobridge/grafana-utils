# Datasource Live Mutation CLI Draft (Unwired)

## Goal

Define a CLI-facing shape for future datasource live add/delete wiring without
touching the current parser or dispatcher yet.

## Proposed Commands

### Add

```text
grafana-util datasource add --uid <UID> --name <NAME> --type <TYPE> [options]
```

Options draft:

- `--uid`
- `--name`
- `--type`
- `--access`
- `--url`
- `--default`
- `--json-data-file <FILE>`
- `--secure-json-data-file <FILE>`
- `--dry-run`
- `--table`
- `--json`

### Delete

```text
grafana-util datasource delete --uid <UID> [--dry-run]
grafana-util datasource delete --name <NAME> [--dry-run]
```

Options draft:

- `--uid`
- `--name`
- `--dry-run`
- `--table`
- `--json`

## Output Draft

Dry-run rows should be built from the unwired render helper in:

- `grafana_utils/datasource/live_mutation_render.py`

Suggested columns:

- `OPERATION`
- `UID`
- `NAME`
- `TYPE`
- `MATCH`
- `ACTION`
- `TARGET_ID`

## Wire Targets Later

- `grafana_utils/datasource/parser.py`
- `grafana_utils/datasource_cli.py`
- `grafana_utils/datasource/workflows.py`
