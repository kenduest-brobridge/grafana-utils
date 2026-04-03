# Profile Secret Storage Architecture

This document explains how `grafana-util` stores and resolves repo-local profile secrets, why multiple secret backends exist, and what maintainers should preserve when changing this area.

## Scope

This note covers profile-backed connection secrets used by:

- `grafana-util profile`
- live commands that resolve `--profile`
- profile fields such as:
  - `password`
  - `token`
  - `password_env`
  - `token_env`
  - `password_store`
  - `token_store`

It does not describe datasource secure field replay or the masked-recovery contract for exported datasource secrets. That is a separate surface.

## Why This Exists

Operators need more than one secret-storage mode because their environments differ:

- a desktop maintainer may want OS-native secret storage
- CI may already inject secrets as environment variables
- a repo-local checkout may need encrypted storage without relying on desktop keyrings
- a quick local lab may accept plaintext for speed

The profile system therefore separates:

- connection settings stored in `grafana-util.yaml`
- secret value sources resolved at runtime

## Secret Modes

### Plaintext In `grafana-util.yaml`

Shape:

```yaml
profiles:
  prod:
    url: https://grafana.example.com
    username: admin
    password: change-me
```

Benefits:

- simplest possible setup
- easy to edit by hand

Limits:

- the secret is present in plaintext in the config file
- not appropriate for shared repos or routine production handling

Use for:

- throwaway labs
- demos
- initial bootstrap only

### Environment-Backed Secrets

Shape:

```yaml
profiles:
  prod:
    url: https://grafana.example.com
    username: admin
    password_env: GRAFANA_PROD_PASSWORD
```

or:

```yaml
profiles:
  ci:
    url: https://grafana.example.com
    token_env: GRAFANA_CI_TOKEN
```

Benefits:

- keeps plaintext secrets out of `grafana-util.yaml`
- good fit for CI and wrappers
- easy to rotate without rewriting the config file

Limits:

- the runtime environment still needs to inject and protect the env vars
- shell history and process inspection risks depend on how the env is managed outside the tool

Use for:

- CI jobs
- automation wrappers
- local shells that already manage env injection

### OS Secret Store

Shape:

```yaml
profiles:
  prod:
    url: https://grafana.example.com
    username: admin
    password_store:
      provider: os
      key: grafana-util/profile/prod/password
```

Benefits:

- keeps secrets out of the YAML file
- good fit for repeated operator use on desktop systems
- pairs naturally with `profile add --store-secret os`

Current platform behavior:

- macOS:
  - stores secrets in the Keychain
  - implemented through the `security` command-line tool
- Linux:
  - stores secrets through the system keyring integration
  - intended backend is Linux Secret Service

Limits:

- only supported on macOS and Linux
- Linux headless environments, containers, and CI runners may not have a working secret-service session
- portability is lower than `encrypted-file` because the backend is host-local

Use for:

- repeated desktop operator workflows
- local admin work where repo-local plaintext is undesirable

### Encrypted Secret File

Shape:

```yaml
profiles:
  prod:
    url: https://grafana.example.com
    username: admin
    password_store:
      provider: encrypted-file
      key: grafana-util/profile/prod/password
      path: .grafana-util.secrets.yaml
```

Benefits:

- keeps plaintext secrets out of `grafana-util.yaml`
- works without an OS secret store
- travels with the checkout more easily than host-native keyrings

Current variants:

- passphrase-backed encrypted file
- local-key-file-backed encrypted file

Limits:

- security still depends on how the passphrase or local key is managed
- local-key-file mode is better than plaintext but is not strong protection against local account compromise

Use for:

- repo-local workflows that need encrypted storage
- Linux or container environments without a usable OS secret service
- teams that want one portable secret-file pattern

## Current File Layout

Default paths:

- `grafana-util.yaml`
- `.grafana-util.secrets.yaml`
- `.grafana-util.secrets.key`

Path rules:

- the config file defaults to the current working directory
- `GRAFANA_UTIL_CONFIG` can move the config file
- default secret file and key file paths follow the config file directory
- secret reference paths are resolved relative to the config file, not an arbitrary process cwd

This relative-to-config rule is important. Do not silently switch it to relative-to-launch-directory behavior.

## Runtime Resolution Model

The runtime resolves connection settings from profile data into concrete credentials before live commands execute.

High-level order:

1. select the config file
2. select the profile
3. read non-secret connection settings from `grafana-util.yaml`
4. resolve the credential source:
   - plaintext field
   - environment variable
   - OS secret store reference
   - encrypted-file reference
5. pass the resolved credential into the live connection layer

Important user-visible behavior:

- `profile show` masks secrets by default
- `profile show --show-secrets` reveals plaintext or resolved secret-store values

Maintain this masking-by-default behavior.

## Why `os` And `encrypted-file` Both Exist

They solve different operator problems:

- `os`
  - best for local desktop convenience
  - relies on host-native storage
  - not ideal for portable or headless environments
- `encrypted-file`
  - more portable across checkouts
  - does not depend on a desktop keyring
  - pushes more responsibility onto passphrase or local-key handling

Do not collapse them into one abstraction that hides these tradeoffs from operators.

## Current Implementation Surface

Primary implementation:

- [profile_secret_store.rs](/Users/kendlee/work/grafana-utils/rust/src/profile_secret_store.rs)
- [profile_config.rs](/Users/kendlee/work/grafana-utils/rust/src/profile_config.rs)
- [profile_cli_runtime.rs](/Users/kendlee/work/grafana-utils/rust/src/profile_cli_runtime.rs)
- [profile_cli_render.rs](/Users/kendlee/work/grafana-utils/rust/src/profile_cli_render.rs)

Current documented user surfaces:

- [profile.md](/Users/kendlee/work/grafana-utils/docs/commands/en/profile.md)
- [getting-started.md](/Users/kendlee/work/grafana-utils/docs/user-guide/en/getting-started.md)
- [reference.md](/Users/kendlee/work/grafana-utils/docs/user-guide/en/reference.md)

## Maintainer Rules

- Keep `--profile` the recommended steady-state for repeatable operator work.
- Keep env-backed credentials as a first-class path for CI and automation.
- Keep OS secret storage limited to clearly supported platforms.
- Keep encrypted-file behavior explicit rather than magical.
- Keep masking as the default display behavior.
- Keep path resolution relative to the config file directory.
- Keep docs examples aligned with real supported storage modes.

## Recommended User Guidance

The current intended guidance order is:

1. `password_env` / `token_env` for CI and repeatable automation
2. `os` for desktop operator workflows on macOS or Linux
3. `encrypted-file` when OS secret storage is unavailable or undesirable
4. plaintext `file` only for bootstrap, demos, or throwaway local labs

## Limits And Troubleshooting To Preserve In Docs

When editing user-facing docs, keep these constraints explicit:

- `os` storage is only supported on macOS and Linux
- Linux may fail when no Secret Service session is available
- `encrypted-file` portability depends on the secret file plus passphrase or local key
- `--show-secrets` is diagnostic and prints plaintext
- token-backed profiles may authenticate successfully but still be too narrow for multi-org or admin-scoped workflows

## Documentation Pointers

If this architecture changes, update:

- [docs/DEVELOPER.md](/Users/kendlee/work/grafana-utils/docs/DEVELOPER.md)
- [docs/internal/README.md](/Users/kendlee/work/grafana-utils/docs/internal/README.md)
- [generated-docs-architecture.md](/Users/kendlee/work/grafana-utils/docs/internal/generated-docs-architecture.md) if generator behavior changes
- user-facing profile and reference docs if operator guidance changes
