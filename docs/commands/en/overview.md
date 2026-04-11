# Removed root path: `grafana-util overview`

## Root

Purpose: migration note for the removed project-wide overview root.

When to use: when you are translating older docs or scripts to the current `status` surface.

Description: the public overview surface now lives under `grafana-util status`. The top-level `overview` root is no longer runnable. Use `status overview` or `status live` instead.

Canonical replacement:

- `grafana-util overview ...` -> `grafana-util status overview ...`
- `grafana-util overview live ...` -> `grafana-util status overview live ...`

Useful next page: [status](./status.md)
