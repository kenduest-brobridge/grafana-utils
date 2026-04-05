# 維運情境手冊 (Operator Scenarios)

本章把分散的指令整理成幾個常見的實際操作流程。

## 適用對象

- 已經知道自己要做哪一類維運流程的人
- 想看端到端操作順序，而不是單一 command 的人
- 需要把 dashboard、access、alert、change 串在一起的人

## 主要目標

- 把零散指令串成完整流程
- 讓你先知道哪一步該先看、哪一步該先檢查
- 在真正套用前先做可重播的準備

## 採用前後對照

- 以前：讀者得從很多獨立 command 頁自己拼出完整的維運故事。
- 現在：每個情境都先交代問題、lane 和預期證據，再進入實際命令。

## 成功判準

- 你能直接選到跟眼前問題最接近的情境。
- 你知道下一步是盤點、回放、審查還是排障。
- 你在執行前就能說出預期輸出長什麼樣。

## 失敗時先檢查

- 如果情境一開始就走錯 lane，先停下來切到真正符合工作流的章節。
- 如果預期輸出跟目前階段對不上，先修正差異再繼續。
- 如果你其實更需要精確旗標，請改看指令詳細說明，不要硬靠情境頁猜。

如果要對照每個工作流對應的精確旗標，請搭配 [dashboard](../../commands/zh-TW/dashboard.md)、[access](../../commands/zh-TW/access.md)、[alert](../../commands/zh-TW/alert.md)、[change](../../commands/zh-TW/change.md)、[status](../../commands/zh-TW/status.md) 與 [overview](../../commands/zh-TW/overview.md) 一起看。

---

## 1. 變更前環境驗證

在執行任何變更前，先確認連線正確，而且版本沒搞錯。

**採用前**：你還不知道 CLI 連的是哪一套 Grafana、目前憑證到底吃到哪組、live 狀態是否真的適合往下做。

**採用後**：你會先得到一個可驗證的 live 目標、一個可讀的 readiness 結果，以及一個可瀏覽的 overview，再進入後續流程。

```bash
# 用途：在執行任何變更前，先確認連線正確，而且版本沒搞錯。
grafana-util profile list
```

```bash
# 用途：在執行任何變更前，先確認連線正確，而且版本沒搞錯。
grafana-util status live --profile prod --output-format table
```

```bash
# 用途：在執行任何變更前，先確認連線正確，而且版本沒搞錯。
grafana-util overview live --profile prod --output-format interactive
```
**預期輸出：**
```text
PROFILES:
  * prod (預設) -> http://grafana.internal

OVERALL: status=ready

Project overview
Live status: ready
```
先用 `profile list` 確認專案本地的預設值，再用 `status live` 做驗證，最後用 `overview live --output-format interactive` 看同一個 live 操作面的可瀏覽版本。

---

## 2. 全資產稽核 (Estate-Wide Audit)

盤點跨 org 的資產。

**採用前**：你知道環境裡有 dashboard、org、使用者，但很難快速回答「現在到底有哪些東西」。

**採用後**：你可以用 dashboard inventory 搭配 access inventory，把目前 estate 狀態先講清楚，再決定要不要匯出、回放或整理。

```bash
# 用途：盤點跨 org 的資產。
grafana-util dashboard list --profile prod --all-orgs --with-sources --table
```

```bash
# 用途：盤點跨 org 的資產。
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

把 live dashboard 匯出成可長期保存的目錄結構。

**採用前**：所謂備份常常只是 UI 匯出或一批零散檔案，之後很難 review、比對或 replay。

**採用後**：你會得到一份可審查、可回放、也能接後續 inspect 與 migration 的匯出樹。

```bash
# 用途：把 live dashboard 匯出成可長期保存的目錄結構。
grafana-util dashboard export --export-dir ./backups --overwrite --progress
```

```bash
# 用途：把 live dashboard 匯出成可長期保存的目錄結構。
grafana-util access org export --export-dir ./access-orgs
```

```bash
# 用途：把 live dashboard 匯出成可長期保存的目錄結構。
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
如果目標是建立可重播的環境快照，dashboard、org 與 service account 的匯出最好一起保留。

---

## 4. 受控的恢復 (Dashboard Import)

把備份回放到 live Grafana 實例。

**採用前**：回放常常是先執行再看結果，直到覆寫到不該動的東西才發現問題。

**採用後**：你會先看 dry-run 的表格化結果，再決定這次回放是否安全、是否會覆蓋既有資產。

```bash
# 用途：把備份回放到 live Grafana 實例。
grafana-util dashboard import --import-dir ./backups/raw --replace-existing --dry-run --table
```

```bash
# 用途：把備份回放到 live Grafana 實例。
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
# 用途：告警變更應遵循受審查的生命週期。
grafana-util change summary --desired-file ./desired.json
```

```bash
# 用途：告警變更應遵循受審查的生命週期。
grafana-util change preflight --desired-file ./desired.json --output-format json
```

```bash
# 用途：告警變更應遵循受審查的生命週期。
grafana-util alert plan --profile prod --desired-dir ./alerts/desired --output-format json
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

透過快照管理使用者、team 與 service account。

```bash
# 用途：透過快照管理使用者、team 與 service account。
grafana-util access user import --import-dir ./access-users --dry-run --table
```

```bash
# 用途：透過快照管理使用者、team 與 service account。
grafana-util access service-account token add --service-account-id 15 --token-name nightly --seconds-to-live 3600 --json
```

```bash
# 用途：透過快照管理使用者、team 與 service account。
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
這個工作流是用來安全回放身分狀態的：先看 import dry-run，自動化憑證則透過 service account token 指令做輪替，不必再靠人工猜測目標帳號。

---
[⬅️ 上一章：變更與狀態](change-overview-status.md) | [🏠 回首頁](index.md) | [➡️ 下一章：技術參考手冊](reference.md)
