# Reference Manual

This chapter provides a comprehensive technical reference for `grafana-util`. It is designed for operators who need precise details on command syntax, authentication protocols, and output contracts.

---

## 🏗️ Command Families

`grafana-util` is organized by functional domains. Each domain supports a specific set of operations for Grafana estate management.

| Domain | Purpose | Key Commands (with Arguments) |
| :--- | :--- | :--- |
| **Dashboard** | Inventory & Analysis | `list --all-orgs`, `export --export-dir <dir>`, `import --import-dir <dir> --dry-run`, `inspect-vars --uid <uid>`, `patch-file --input <file>`, `clone-live --uid <uid>`, `browse` |
| **Datasource** | Lifecycle & Recovery | `list --table`, `export --output-dir <dir>`, `import --replace-existing`, `diff`, `add --type <type>`, `modify --id <id>`, `delete --id <id>`, `browse` |
| **Alert** | Review-First Mgmt | `plan --desired-dir <dir>`, `apply --plan-file <file>`, `add-rule --name <name>`, `set-route`, `preview-route`, `new-rule`, `export --overwrite` |
| **Access** | Identity & Orgs | `org list`, `user add --login <user>`, `team list --org-id <id>`, `service-account token add --id <id>`, `export`, `import`, `diff` |
| **Change** | Staged Workflows | `summary`, `bundle --output-file <file>`, `preflight --staged-dir <dir>`, `assess-alerts`, `plan`, `review`, `apply` |
| **Status** | Readiness Reports | `status live --output-format json`, `status staged --output-format interactive`, `overview`, `overview live` |

---

## 🔐 Global Authentication & Connection

These flags are common across almost all live Grafana commands.

### Connection Flags
| Flag | Description | Default |
| :--- | :--- | :--- |
| `--url` | The base URL of the Grafana instance. | `http://localhost:3000` |
| `--timeout` | HTTP request timeout in seconds. | `30` |
| `--verify-ssl` | Enable/Disable TLS certificate verification. | `true` |
| `--profile` | Load settings from a specific profile in `grafana-profiles.yaml`. | (None) |

### Authentication Modes
| Mode | Flags |
| :--- | :--- |
| **Token** | `--token`, `--api-token`, `--prompt-token` |
| **Basic Auth** | `--basic-user`, `--basic-password`, `--prompt-password` |

> **Security Note**: Prefer `--prompt-token` or `--prompt-password` in interactive sessions to avoid leaking secrets in shell history.

---

## 📊 Output Surfaces

The CLI provides multiple ways to consume data depending on whether the consumer is a human or a machine.

| Mode | Flag | Typical Use Case |
| :--- | :--- | :--- |
| **Text** | (Default) | Quick summaries, dry-run previews, and logs. |
| **JSON** | `--output-format json` | Automation, piping to `jq`, or saving as artifacts. |
| **Table** | `--table` or `--output-format table` | Audits and human-readable inventory listings. |
| **Interactive** | `--output-format interactive` | Guided browsing and complex state review (TUI). |

---

## 🛠️ Profiles (`grafana-profiles.yaml`)

Profiles eliminate the need to repeat connection flags. A typical configuration looks like this:

```yaml
default_profile: dev
profiles:
  dev:
    url: http://localhost:3000
    token_env: GRAFANA_DEV_TOKEN
    verify_ssl: false
  prod:
    url: https://grafana.example.com
    username: admin
    password_env: GRAFANA_PROD_PASSWORD
```

### Profile Selection Logic
1.  **Explicit**: `--profile prod` always takes precedence.
2.  **Implicit**: If no flag is provided, the `default_profile` is used.
3.  **Automatic**: If only one profile exists, it is selected automatically.
4.  **Override**: Any CLI flag (e.g., `--url`) will override the value stored in the profile.

---

## ⚖️ Resource Capability Matrix

| Resource | List | Export | Import | Diff | Mutation (Add/Mod/Del) |
| :--- | :---: | :---: | :---: | :---: | :---: |
| Dashboards | ✅ | ✅ | ✅ | ✅ | ✅ |
| Datasources | ✅ | ✅ | ✅ | ✅ | ✅ |
| Alerts | ✅ | ✅ | ✅ | ✅ | ✅ (via Plan/Apply) |
| Orgs/Users | ✅ | ✅ | ✅ | ✅ | ✅ |
| Service Accts | ✅ | ✅ | ✅ | ✅ | ✅ |

---

## ⏭️ Next Step
For practical, task-oriented guides, proceed to the [**Scenarios**](./scenarios.md) chapter.
