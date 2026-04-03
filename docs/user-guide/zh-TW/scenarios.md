# 維運情境手冊 (Operator Scenarios)

本章把分散的指令家族整理成端對端的工作流程。

如果要對照每個工作流對應的精確旗標，請搭配 [dashboard](../../commands/zh-TW/dashboard.md)、[access](../../commands/zh-TW/access.md)、[alert](../../commands/zh-TW/alert.md)、[change](../../commands/zh-TW/change.md)、[status](../../commands/zh-TW/status.md) 與 [overview](../../commands/zh-TW/overview.md) 一起看。

---

## 1. 變更前環境驗證

在執行任何變更前，先確認連線正確且版本一致。

```bash
grafana-util profile list
grafana-util status live --profile prod --output table
grafana-util overview live --profile prod --output interactive
```
**預期輸出：**
```text
PROFILES:
  * prod (預設) -> http://grafana.internal

OVERALL: status=ready

Project overview
Live status: ready
```
先用 `profile list` 確認 repo-local 預設值，再用 `status live` 做驗證，最後用 `overview live --output interactive` 看同一個 live surface 的可瀏覽版本。

---

## 2. 全資產稽核 (Estate-Wide Audit)

盤點跨組織的所有資產。

```bash
grafana-util dashboard list --profile prod --all-orgs --with-sources --table
grafana-util access org list --basic-user admin --basic-password admin --with-users --output-format yaml
```
**預期輸出：**
```text
UID        TITLE             FOLDER    ORG   SOURCES
cpu-view   CPU Metrics       Metrics   1     prometheus-main
mem-view   Memory Usage      Metrics   5     loki-prod

id: 1
name: Main Org
users:
  - alice@example.com
```
Dashboard 與 access inventory 一起看，才比較能在動手前回答「目前到底有哪些東西」。

---

## 3. 可靠的備份 (Dashboard Export)

將即時 Dashboard 匯出為可持久化的目錄結構。

```bash
grafana-util dashboard export --export-dir ./backups --overwrite --progress
grafana-util access org export --export-dir ./access-orgs
grafana-util access service-account export --export-dir ./access-service-accounts
```
**預期輸出：**
```text
Exporting dashboard 1/32: cpu-metrics
Exporting dashboard 2/32: memory-leak-check
...
匯出完成：32 個儀表板已儲存至 ./backups/raw

Exported organization inventory -> access-orgs/orgs.json
Exported service account inventory -> access-service-accounts/service-accounts.json
```
如果目標是建立可重播的 estate snapshot，dashboard、org 與 service-account 的匯出最好一起保留。

---

## 4. 受控的恢復 (Dashboard Import)

將備份回放到即時 Grafana 實例。

```bash
grafana-util dashboard import --import-dir ./backups/raw --replace-existing --dry-run --table
grafana-util access team import --import-dir ./access-teams --replace-existing --dry-run --table
```
**預期輸出：**
```text
UID        TITLE          ACTION   DESTINATION
cpu-view   CPU Metrics    update   exists
net-view   Network IO     create   missing

LOGIN       ROLE    ACTION   STATUS
dev-admin   Admin   update   existing
ops-user    Viewer  create   missing
```
先看 dry-run 表格，再決定這次回放是以新增為主，還是會覆寫既有資產。

---

## 5. 告警治理流程 (Plan/Apply)

告警變更應遵循受審查的生命週期。

```bash
grafana-util change summary --desired-file ./desired.json
grafana-util change preflight --desired-file ./desired.json --output json
grafana-util alert plan --profile prod --desired-dir ./alerts/desired --output json
```
**預期輸出 (摘要片段)：**
```text
CHANGE PACKAGE SUMMARY:
- dashboards: 5 modified, 2 added
- alerts: 3 modified

PREFLIGHT CHECK:
- dashboards: valid (7 files)
- result: 0 errors, 0 blockers

{
  "summary": { "modified": 2, "added": 1, "deleted": 0 },
  "plan_id": "plan-2026-04-02-abc"
}
```
想先了解變更包規模時，先跑 `change summary`；要確認 staged 輸入結構正確時，再跑 `change preflight`；最後才進入 alert-specific planning。

---

## 6. 身份快照回放 (Access Management)

透過快照管理使用者、團隊與服務帳號。

```bash
grafana-util access user import --import-dir ./access-users --dry-run --table
grafana-util access service-account token add --service-account-id 15 --token-name nightly --seconds-to-live 3600 --json
grafana-util access service-account token delete --service-account-id 15 --token-name nightly --yes --json
```
**預期輸出：**
```text
LOGIN       ROLE    ACTION   STATUS
dev-admin   Admin   update   existing
ops-user    Viewer  create   missing

{
  "serviceAccountId": "15",
  "name": "nightly",
  "secondsToLive": "3600",
  "key": "eyJ..."
}
```
這個工作流是用來安全回放身分狀態的：使用者先看 import dry-run，自動化憑證則透過 service-account token 指令做輪替，不必靠猜目標帳號。

---
[⬅️ 上一章：變更與狀態](change-overview-status.md) | [🏠 回首頁](index.md) | [➡️ 下一章：技術參考手冊](reference.md)
