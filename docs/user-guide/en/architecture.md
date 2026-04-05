# 🏛️ Architecture & Design Principles

Use this chapter when you want to understand why the handbook and command surfaces are split the way they are, and how that split should change your day-to-day operator choices.

Understanding the architectural philosophy of `grafana-util` is key to managing large-scale Grafana estates effectively. This chapter explains the "Why" behind the design decisions, but it also tells you how those decisions should change the way you operate.

## Who It Is For

- Maintainers or operators who want to understand why the CLI is shaped this way.
- Teams deciding how far to trust export, replay, review, and staged workflows.
- Reviewers who need design context before changing docs, scripts, or operator process.

## Primary Goals

- Explain the design tradeoffs behind the command families and workflow lanes.
- Show how those tradeoffs affect day-to-day operator behavior.
- Give enough context that you can explain the tool, not just run it.

## Before / After

- Before: it was easy to think the repo was just a collection of commands with the same shape.
- After: the architecture chapter explains why the tool separates surfaces, lanes, and command families, so a reader can place each workflow in the right layer.

## What success looks like

- You can explain why the runtime and docs are split the way they are.
- You can tell which workflows belong in a handbook, which belong in command docs, and which belong in internal maintainer notes.
- You can choose the right surface for a new feature before you implement it.

## Failure checks

- If a workflow does not fit any existing surface, pause and decide whether it needs a new chapter or a new command family.
- If the runtime shape and the docs shape drift apart, treat that as an architecture bug, not just a docs bug.
- If you are not sure why the split exists, reread the surface and lane sections before adding new work.

For the command surface behind these ideas, see [status](../../commands/en/status.md), [overview](../../commands/en/overview.md), [change](../../commands/en/change.md), and [dashboard](../../commands/en/dashboard.md).

---

## 🏗️ The Three Surfaces Pattern

`grafana-util` separates operational concerns into three distinct "Surfaces." This prevents mixing human-facing data with machine-readable contracts.

| Surface | Purpose | Primary Target | Output Formats |
| :--- | :--- | :--- | :--- |
| **Status** | **Readiness & Contracts** | CI/CD Pipelines, Scripts | JSON, Table |
| **Overview** | **Global Observability** | Human SREs, Managers | Interactive TUI, Summary |
| **Change** | **Intent & Lifecycle** | PR Reviews, Audit Logs | JSON Plan, Diff |

### How to choose the right surface

- Use `status` when you need a gate, a machine-readable result, or a clean pass/fail readout.
- Use `overview` when you need to look across the estate as a human and decide where to drill in next.
- Use `change` when you already know there is intended work and need to summarize, preflight, plan, review, or apply it.

Typical operator decisions:

- "Can I trust the current state enough to proceed?" -> `status live`
- "What does this Grafana estate look like right now?" -> `overview live`
- "Is my staged package structurally and operationally sane?" -> `status staged` plus `change preflight`
- "What exactly will change?" -> `change summary`, `change plan`, then `change review`

### Why the split matters

If the three surfaces collapse into one mental bucket, teams usually make one of these mistakes:

- they use a human-facing summary where a machine gate was required
- they treat a live read as if it proved staged inputs were ready
- they skip review/plan flow because the current live state "looks fine"

The design is intentionally opinionated so operators do not have to guess which output is safe to automate against and which output is meant for human interpretation.

---

## 🛣️ Lane Isolation Policy

To prevent configuration drift and "Frankenstein" assets, we enforce strict isolation between data lanes.

1. **The Raw Lane (`raw/`)**: A 1:1 API fidelity snapshot. Used for backup and full-estate DR. No manual editing allowed.
2. **The Prompt Lane (`prompt/`)**: Optimized for UI-based imports. Metadata is stripped to ensure clean adoption by new organizations.
3. **The Provisioning Lane (`provisioning/`)**: Static files for disk-based Grafana provisioning. This is a one-way transformation from the canonical API model.

### How lane isolation changes real operator behavior

- Treat `raw/` as the canonical replay and audit lane for dashboards.
- Treat `prompt/` as the clean handoff lane when you want to move dashboards between environments without dragging source-environment metadata along.
- Treat `provisioning/` as a derived deployment lane, not as the place where you invent the source of truth.

If you ignore lane isolation, the failure is often subtle rather than spectacular:

- imported dashboards keep the wrong environment assumptions
- provisioning files no longer match the canonical export tree
- teams patch the wrong artifact family and later cannot explain why live state drifted

### What to do when you are unsure which lane to use

- Use `raw/` for backup, replay, diff, and audit workflows.
- Use `prompt/` for environment promotion and UI-first adoption.
- Use `provisioning/` only when the target workflow is Grafana disk provisioning.

---

## 🔐 Secret Governance (Masked Recovery)

`grafana-util` follows a **"Safe-by-Default"** architecture for secrets such as datasource passwords and secure connection fields.

- **Export**: Sensitive fields are masked. The export file is safe to commit to Git.
- **Recovery**: During `import`, the CLI identifies missing secrets and provides a secure protocol for re-injection via environment variables or interactive prompts.

### Why this matters in practice

This design is meant to avoid two bad outcomes:

- leaking a working datasource secret into Git
- treating a masked export as if it still contained everything needed for live replay

Success looks like this:

- you can commit the datasource inventory safely
- replay/import tells you which secrets still need to be re-injected
- operators know that secret recovery is a deliberate step, not an invisible side effect

When this goes wrong, the symptoms are usually:

- imports fail because masked fields were never re-supplied
- teams assume the export is "complete" and only discover the gap during restore
- people try to stuff plaintext secrets back into the export file instead of using the supported recovery path

---

## 🔄 State Transition Model

`grafana-util` operates as a **State Reconciler** for Alerting, and a **Snapshot Replayer** for Dashboards.

- **Dashboards (Snapshot/Replay)**: Imperative. "Make the target look exactly like this file right now."
- **Alerting (Desired State)**: Declarative. "Compute the difference (Plan) between my files and the server, then apply only the delta."

### Why dashboards and alerts are intentionally different

They solve different operational problems:

- dashboards are usually handled as exported artifacts that you inspect, patch, and replay
- alerts behave more like managed desired state where review and delta visibility matter before execution

Operational consequence:

- for dashboards, think in terms of artifact quality, lane choice, and replay target
- for alerts, think in terms of staged intent, route correctness, plan review, and controlled apply

### Decision guide

- if your first concern is "is this file the right artifact to replay?" you are in dashboard thinking
- if your first concern is "what delta will this create?" you are in alert/change thinking

---

## ✅ What Good Looks Like

The architecture is working for you when:

- your team can explain the difference between `status`, `overview`, and `change`
- live checks and staged checks are not treated as interchangeable
- dashboard lanes are not mixed casually
- masked secret exports are treated as safe artifacts, not as complete replay payloads
- operators know when to stop at read-only validation and when to move into plan/apply workflows

---
[⬅️ Previous: Getting Started](getting-started.md) | [🏠 Home](index.md) | [➡️ Next: Dashboard Management](dashboard.md)
