# 身分與存取管理 (Identity & Access)

這一章整理 Grafana 的身分與存取資產：org、使用者、team 與 service account。重點是先把人、組織與自動化憑證的生命週期講清楚，再談匯出、同步與回放。
盤點可以直接來自 live Grafana，也可以來自本機匯出的套件；流程本身是一樣的。

## 適用對象

- 負責 org、使用者、team 或 service account 管理的人
- 需要做權限盤點、同步、匯出或回放的人
- 需要把身分資產接進 Git 或 CI 流程的人

## 主要目標

- 先理解 org / user / team / service account 的關係
- 再把盤點、匯出、匯入與 diff 做成可重複流程，而且盤點可以來自 live 或本機
- 需要時才對 token 做輪替或刪除

## 採用前後對照

- 以前：身分與存取的工作分散在不同 command 群組，命名風格也不一致。
- 現在：org、user、team 與 service account 放在同一份導引，入口與術語都更清楚。

## 成功判準

- 你一眼就能分辨任務是 org 管理、user 管理、team 管理，還是 service account 處理。
- 你在動手前就知道自己要進哪一組 command。
- 你知道什麼時候需要先看審查步驟，再做變更。

## 失敗時先檢查

- 如果這次身分變更會影響到比預期更多的 org，先停下來確認範圍。
- 如果不確定 token 或 service account 能不能做這件事，先看指令頁，不要直接 mutate。
- 如果你還覺得 team / org / user 的說法不一致，先回 glossary 和 command reference。

## 🔗 指令頁面

如果你現在要查的是指令細節，而不是工作流程章節，可以直接看下面這些指令頁：

- [access 指令總覽](../../commands/zh-TW/access.md)
- [access user](../../commands/zh-TW/access-user.md)
- [access org](../../commands/zh-TW/access-org.md)
- [access team](../../commands/zh-TW/access-team.md)
- [access service-account](../../commands/zh-TW/access-service-account.md)
- [access service-account token](../../commands/zh-TW/access-service-account-token.md)
- [指令詳細說明總索引](../../commands/zh-TW/index.md)

---

## 🏢 org 管理

需要用 Basic auth 盤點、匯出或回放 org 時，請使用 `access org`。它的 `list` 也可以直接讀本機 bundle。

### 1. 列出、匯出與回放 org
```bash
# 用途：1. 列出、匯出與回放 org。
grafana-util access org list --table
```

```bash
# 用途：1. 列出、匯出與回放 org。
grafana-util access org list --input-dir ./access-orgs --table
```

```bash
# 用途：1. 列出、匯出與回放 org。
grafana-util access org export --output-dir ./access-orgs
```

```bash
# 用途：1. 列出、匯出與回放 org。
grafana-util access org import --input-dir ./access-orgs --dry-run
```
**預期輸出：**
```text
ID   NAME        IS_MAIN   QUOTA
1    Main Org    true      -
5    SRE Team    false     10

Exported org inventory -> access-orgs/orgs.json
Exported org metadata   -> access-orgs/export-metadata.json

PREFLIGHT IMPORT:
  - would create 0 org(s)
  - would update 1 org(s)
```
先用 list 確認主 org，再用 export/import 建立可重播的 org 快照。

---

## 👤 使用者與 team 管理

需要調整成員、管理快照或檢查漂移時，請使用 `access user` 與 `access team`。它們的 `list` 與 `browse` 都可以讀本機 bundle。

### 1. 新增、修改與比對使用者
```bash
# 新增一個具備全域 Admin 角色的使用者
grafana-util access user add --login dev-user --role Admin --prompt-password

# 修改現有使用者在特定 org 中的角色
grafana-util access user modify --login dev-user --org-id 5 --role Editor

# 將儲存的使用者快照與即時 Grafana 比對
grafana-util access user diff --diff-dir ./access-users --scope global
```

如果要看同一份本機套件，可以改用：

```bash
# 從本機套件檢視同一份使用者 inventory
grafana-util access user list --input-dir ./access-users
```
**預期輸出：**
```text
Created user dev-user -> id=12 orgRole=Editor grafanaAdmin=true

No user differences across 12 user(s).
```
如果不想把密碼留在 shell history 裡，請改用 `--prompt-password`。`--scope global` 需要 Basic auth。

### 2. team 盤點與同步
```bash
# 用途：2. team 盤點與同步。
grafana-util access team list --org-id 1 --table
```

```bash
# 用途：2. team 盤點與同步。
grafana-util access team list --input-dir ./access-teams --table
```

```bash
# 用途：2. team 盤點與同步。
grafana-util access team export --output-dir ./access-teams --with-members
```

```bash
# 用途：2. team 盤點與同步。
grafana-util access team import --input-dir ./access-teams --replace-existing --dry-run --table
```
**預期輸出：**
```text
ID   NAME           MEMBERS   EMAIL
10   Platform SRE   5         sre@company.com

Exported team inventory -> access-teams/teams.json
Exported team metadata   -> access-teams/export-metadata.json

LOGIN       ROLE    ACTION   STATUS
dev-admin   Admin   update   existing
ops-user    Viewer  create   missing
```
匯出時加上 `--with-members` 才會保留成員狀態；要做可能覆寫的匯入前，先用 `--dry-run --table` 看一次。

---

## 🤖 service account 管理

service account 是自動化流程常見的基礎元件。它的 inventory 也可以先從本機套件看，不必先碰 live Grafana。

### 1. 列出與匯出 service account
```bash
# 用途：1. 列出與匯出 service account。
grafana-util access service-account list --json
```

```bash
# 用途：1. 列出與匯出 service account。
grafana-util access service-account list --input-dir ./access-sa --output-format text
```

```bash
# 用途：1. 列出與匯出 service account。
grafana-util access service-account export --output-dir ./access-sa
```
**預期輸出：**
```text
[
  {
    "id": "15",
    "name": "deploy-bot",
    "role": "Editor",
    "disabled": false,
    "tokens": "1",
    "orgId": "1"
  }
]

Listed 1 service account(s) at http://127.0.0.1:3000

Exported service account inventory -> access-sa/service-accounts.json
Exported service account tokens    -> access-sa/tokens.json
```
`access service-account export` 會寫出盤點結果與 token bundle。`tokens.json` 包含敏感資訊，請妥善保管。

### 2. 建立與刪除 token
```bash
# 以名稱新增一個 token
grafana-util access service-account token add --name deploy-bot --token-name nightly --seconds-to-live 3600

# 以數字 ID 新增 token，並保留一次性的 secret
grafana-util access service-account token add --service-account-id 15 --token-name ci-deployment-token --json

# 驗證後刪除舊 token
grafana-util access service-account token delete --service-account-id 15 --token-name nightly --yes --json
```
**預期輸出：**
```text
Created service-account token nightly -> serviceAccountId=15

{
  "serviceAccountId": "15",
  "name": "ci-deployment-token",
  "secondsToLive": "3600",
  "key": "eyJ..."
}

{
  "serviceAccountId": "15",
  "tokenId": "42",
  "name": "nightly",
  "message": "Service-account token deleted."
}
```
如果需要一次性的 `key`，請加上 `--json`。純文字輸出適合寫入日誌，不適合拿來擷取憑證。

---

## 🔍 漂移檢查 (Diff)

比較本機快照與 live Grafana 之間的差異。

```bash
# 用途：比較本機快照與 live Grafana 之間的差異。
grafana-util access user diff --input-dir ./access-users
```

```bash
# 用途：比較本機快照與 live Grafana 之間的差異。
grafana-util access team diff --diff-dir ./access-teams
```

```bash
# 用途：比較本機快照與 live Grafana 之間的差異。
grafana-util access service-account diff --diff-dir ./access-sa
```
**預期輸出：**
```text
--- Live Users
+++ Snapshot Users
-  "login": "old-user"
+  "login": "new-user"

No team differences across 4 team(s).
No service account differences across 2 service account(s).
```
可以用 diff 輸出判斷快照是否適合匯入，也能先看出 live 環境是否已經發生漂移。

---
[⬅️ 上一章：告警治理](alert.md) | [🏠 回首頁](index.md) | [➡️ 下一章：Workspace 審查與狀態](status-workspace.md)
