# 身分與存取管理 (Identity & Access)

管理 Grafana 的身分層資產：組織、使用者、團隊與服務帳號。

## 🔗 逐指令頁面

如果您現在想看的是逐指令說明，而不是工作流章節，請直接使用逐指令頁面：

- [access 指令總覽](../../commands/zh-TW/access.md)
- [access user](../../commands/zh-TW/access-user.md)
- [access org](../../commands/zh-TW/access-org.md)
- [access team](../../commands/zh-TW/access-team.md)
- [access service-account](../../commands/zh-TW/access-service-account.md)
- [access service-account token](../../commands/zh-TW/access-service-account-token.md)
- [逐指令總索引](../../commands/zh-TW/index.md)

---

## 🏢 組織管理 (Organization)

在需要以 Basic auth 做組織盤點、匯出或回放時，請使用 `access org`。

### 1. 列出、匯出與回放組織
```bash
grafana-util access org list --table
grafana-util access org export --export-dir ./access-orgs
grafana-util access org import --import-dir ./access-orgs --dry-run
```
**預期輸出：**
```text
ID   NAME        IS_MAIN   QUOTA
1    Main Org    true      -
5    SRE Team    false     10

Exported organization inventory -> access-orgs/orgs.json
Exported organization metadata   -> access-orgs/export-metadata.json

PREFLIGHT IMPORT:
  - would create 0 org(s)
  - would update 1 org(s)
```
先用 list 確認主組織，再用 export/import 建立可重播的組織快照。

---

## 👤 使用者與團隊管理 (User & Team)

需要調整成員、管理快照或檢查漂移時，請使用 `access user` 與 `access team`。

### 1. 新增、修改與比對使用者
```bash
# 新增一個具備全域 Admin 角色的使用者
grafana-util access user add --login dev-user --role Admin --prompt-password

# 修改現有使用者在特定組織中的角色
grafana-util access user modify --login dev-user --org-id 5 --role Editor

# 將儲存的使用者快照與即時 Grafana 比對
grafana-util access user diff --diff-dir ./access-users --scope global
```
**預期輸出：**
```text
Created user dev-user -> id=12 orgRole=Editor grafanaAdmin=true

No user differences across 12 user(s).
```
如果不想把密碼留在 shell history 裡，請改用 `--prompt-password`。`--scope global` 需要 Basic auth。

### 2. 團隊發現與同步
```bash
grafana-util access team list --org-id 1 --table
grafana-util access team export --export-dir ./access-teams --with-members
grafana-util access team import --import-dir ./access-teams --replace-existing --dry-run --table
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
匯出時加上 `--with-members` 才會保留成員狀態；做可能覆寫的匯入前，先用 `--dry-run --table` 檢查。

---

## 🤖 服務帳號管理 (Service Account)

服務帳號是自動化流程的基礎。

### 1. 列出與匯出服務帳號
```bash
grafana-util access service-account list --json
grafana-util access service-account export --export-dir ./access-sa
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
`access service-account export` 會寫出盤點結果與 token bundle。`tokens.json` 具敏感性，請妥善保管。

### 2. 建立與刪除權杖 (Token)
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

## 🔍 漂移偵測 (Diff)

比較本地身份快照與即時 Grafana 伺服器之間的差異。

```bash
grafana-util access user diff --import-dir ./access-users
grafana-util access team diff --diff-dir ./access-teams
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
[⬅️ 上一章：告警治理](alert.md) | [🏠 回首頁](index.md) | [➡️ 下一章：變更與狀態](change-overview-status.md)
