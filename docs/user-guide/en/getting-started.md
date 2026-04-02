# Getting Started

This chapter provides a step-by-step path to verifying your `grafana-util` installation and establishing a secure connection to your Grafana estate. 

> **Goal**: Ensure the binary is correctly installed, verify the connection to Grafana, and run your first read-only commands safely.

---

## 🛠️ Step 1: Verification

Confirm the installed version and explore the available command surface. This ensures you are not running an outdated binary.

```bash
# Verify the binary version
grafana-util --version

# Explore global help and specific domain surfaces
grafana-util -h
grafana-util dashboard -h
grafana-util alert -h
grafana-util datasource -h
grafana-util access -h
grafana-util profile -h
grafana-util change -h
grafana-util overview -h
grafana-util status -h
```

---

## 🔐 Step 2: Connection Models

`grafana-util` supports two primary methods for interacting with Grafana:

### 1. Direct CLI Flags
Best for one-off tasks or testing. You pass all credentials directly to each command.

```bash
grafana-util dashboard list \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin
```

### 2. Repo-Local Profiles (Recommended)
Best for repeated operator work. Profiles store URL, auth, and timeout settings in a `grafana-profiles.yaml` file.

**Initialize a profile:**
```bash
grafana-util profile init
```

**View and manage profiles:**
```bash
# List all profiles
grafana-util profile list

# Show detailed configuration for a specific profile
grafana-util profile show --profile prod --output-format yaml
```

---

## 📋 Step 3: Safe First Commands

Before performing any mutations (import/apply), verify read access with these safe, non-destructive commands.

| Task | Full Command Example | Proves |
| :--- | :--- | :--- |
| **Inventory** | `grafana-util dashboard list --all-orgs --with-sources --table` | API connectivity and folder visibility. |
| **Datasources** | `grafana-util datasource list --table` | Ability to read datasource plugins and types. |
| **Readiness** | `grafana-util status live --output-format table` | Full project-wide health and readiness report. |
| **Overview** | `grafana-util overview live` | Human-facing summary of the entire estate. |

---

## 🖥️ Interactive Mode (TUI)

Some commands support a **Terminal User Interface (TUI)** for guided human review. Note that these are separate from automation-friendly outputs like JSON.

| Command | Usage | Best for |
| :--- | :--- | :--- |
| `dashboard browse` | `grafana-util dashboard browse` | Guided discovery of live dashboards. |
| `inspect` | `grafana-util dashboard inspect-export --interactive` | Offline review of exported dashboard trees. |
| `overview` | `grafana-util overview --output interactive` | Visual dashboard of project status in the terminal. |

---

## ⏭️ Next Steps

- Consult the [**Reference**](./reference.md) for detailed flag documentation and authentication rules.
- Follow the [**Scenarios**](./scenarios.md) for step-by-step workflows (e.g., migration, backup).
