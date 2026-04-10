# 這個工具是做什麼的

`grafana-util` 不是單純把 Grafana HTTP API 包成一堆指令，也不是只有備份匯出工具。它的重點是把日常維運會碰到的幾條工作流接起來，讓你在「盤點、檢查、審查、搬移、回放」之間有一套一致的做法。

## 適用對象

- 想先知道這工具到底在解什麼問題的人
- 已經在用 Grafana UI，但想把部分流程變成可重複 CLI 的人
- 想判斷這工具適不適合自己工作流的人

如果你曾經遇過下面這些痛點，這個工具就是為了解這些事：

- 想先看整個 Grafana 環境目前長什麼樣，但 UI 很難快速盤點多 org 或大量資產
- 想搬 dashboard、alert 或 data source，卻不想只靠手動點 UI
- 想先知道變更會影響什麼，再決定要不要套用
- 想把匯出結果放進 Git、CI/CD 或 review 流程，但又不想把秘密資料直接寫進檔案
- 想做可重複的維運流程，而不是每次都重新拼湊參數與操作步驟

---

## 採用前後對照

| 原本常見情況 | 改用 `grafana-util` 後 |
| :--- | :--- |
| 想知道現在 Grafana 環境長什麼樣，只能一直切 UI 或查 API。 | 先用 `observe live`、`observe overview` 或 `change inspect` 建立第一個審查面。 |
| 匯出/匯入像一次性的動作，缺少中間檢查點。 | 先匯出、再 inspect、再 dry-run，最後才決定要不要回放。 |
| 告警或權限變更很難在套用前說清楚。 | 先看 summary、plan 與結構化輸出，再進入 apply。 |
| 認證與 secret 容易散落在命令列與腳本裡。 | 用 profile 與 secret 模式把重複設定收起來。 |

這個工具真正改變的是維運順序，不只是把某一條命令縮短。

## 成功判準

- 你可以直接指出這個工具要解掉哪一類維運痛點。
- 你知道自己第一步應該走哪條工作流：盤點、審查、回放，還是變更控管。
- 你能判斷這個 repo 是否比單次 shell script 或純 UI 點選更適合你的情境。

## 失敗時先檢查

- 如果只是一次性的 UI 小改，這個工具不一定是第一選擇。
- 如果你還說不出自己需要哪條工作流，先回到角色路線頁，不要直接往指令頁鑽。
- 如果你要的是精確語法，直接切到指令詳細說明，不要把這頁當成命令手冊。

---

## 它的定位

`grafana-util` 比較接近一套 Grafana 維運工作流工具，而不是單一功能的 CLI。

它是我長期維護的個人工具，不是完整平台，也不是要把 Grafana 的所有 API 面都包起來。重點比較放在那些高摩擦、容易出錯的維運路徑，讓它們更容易審查、重複執行，或接進自動化流程。

它把常見需求拆成幾個面向：

- **盤點與觀察**：用 `observe` 先看目前狀態
- **資產操作**：用 `dashboard`、`datasource`、`alert`、`access` 管理不同類型的 Grafana 資產
- **變更審查**：用 `change` 走 `inspect`、`check`、`preview`、`apply` 這條 task-first 路徑
- **連線與憑證**：用 `config profile` 把 URL、驗證方式與 secret 來源整理起來

重點不是記住每個 command，而是先知道自己在做哪一種工作。

## 放到工具脈絡裡看

如果你也知道 `grafanactl` 或 `grizzly`，比較適合把差異理解成設計取向，而不是輸贏比較：

- `grafanactl` 比較接近通用的 Grafana 資源/API 操作 CLI。
- `grizzly` 比較接近宣告式的 Grafana-as-code 管理方式。
- `grafana-util` 比較偏向可審查操作、inspection/governance 流程，以及較安全的搬移或回放路徑。

它們會有重疊，但真正有用的判斷方式，是你現在需要哪一種工作形狀。

## 哪些放首頁，哪些放目錄

README 和手冊首頁要短，先露出大家最常用的工作流；更細的 command 樹放在 docs index 和各自的指令頁。

- 放在 README / 首頁：`observe live`、`observe overview`、`export dashboard|alert|datasource`、`change inspect/check/preview/apply`、`config profile`、`advanced dashboard`、`advanced alert`、`advanced datasource`、`access`。
- 放在 docs index 和逐指令頁：`dashboard browse/fetch-live/clone-live/serve/edit-live/delete/history/list-vars/governance-gate/topology/impact/screenshot`、`datasource browse/types/list/add/modify/delete`、`snapshot`、`resource`、以及相容別名頁面。

## 主要目標

- 先讓你一眼看懂這工具在解什麼問題
- 幫你判斷它適不適合你的 Grafana 維運工作
- 讓你知道應該從哪個 chapter 或 command 面向開始

---

## 功能總覽表

| 功能面向 | 主要 command | 你會用它來做什麼 |
| :--- | :--- | :--- |
| 環境狀態檢查 | `observe live` / `observe staged` | 看 live 或 staged 狀態是否健康、是否適合往下做 |
| 全域總覽 | `observe overview` | 快速盤點整體 Grafana 環境、先決定下一步要往哪裡鑽 |
| Dashboard 維運 | `dashboard` | 匯出、匯入、diff、inspect、截圖、拓樸分析 |
| Data source 維運 | `datasource` | data source 盤點、匯出、匯入、diff、修改與恢復 |
| 告警治理 | `alert` | 告警規則、通知路由、contact point、plan / apply |
| 身分與存取 | `access` | org、user、team、service account 與 token 管理 |
| 變更審查 | `change` | 先 inspect、check、preview，再決定要不要 apply |
| 連線與憑證設定 | `config profile` | 把 URL、驗證方式與 secret 來源整理成可重複使用的設定 |

如果你只想知道「現在該從哪裡開始」，可以先用這個表判斷自己遇到的是哪一類問題，再往對應章節走。

---

## 這個工具特別適合哪些情境

### 1. 日常維運與巡檢

你想先回答：

- 目前有哪些 dashboard、alert、data source？
- live 狀態是否正常？
- 哪些地方看起來已經漂移或快要出問題？

這時通常會先從 `observe live` 或 `observe overview` 開始。

### 2. 匯出、搬移與回放

你想把 dashboard 或 data source 從一個環境搬到另一個環境，或保留一份可重播的匯出樹。這時你需要的不只是「匯出」本身，而是：

- 匯出成適合的資料路徑
- 先做 diff / inspect / dry-run
- 再決定要不要匯入或回放

### 3. 變更前先做審查

你不想直接套用變更，而是先回答：

- 這次到底會改到哪些東西？
- staged 輸入是不是完整？
- 權限、secret、路由、依賴是否合理？

這時 `change inspect`、`change check`、`change preview`、`alert plan` 這些流程就很重要。

### 4. 自動化與 CI/CD

你想把 Grafana 維運流程接進腳本、pipeline 或例行工作，而不是只靠人手動操作。

這時重點通常是：

- 用 `config profile` 或 env 把連線整理好
- 讓輸出格式穩定可讀
- 讓變更流程有 review 與 gate

---

## 第一條成功路徑通常長什麼樣

如果這個工具真的適合你，第一次順利上手通常會長這樣：

1. 先確認 binary 與第一個唯讀 live 檢查正常
2. 匯出一份可審查的資產樹
3. 在 replay 前先 inspect 這份匯出樹
4. 在 apply 前先 preview 這次變更

這條路徑比先看完所有 command，更能快速感受到這個工具到底在幫什麼忙。

---

## 它不特別想解的事

有些情況其實不一定要先用 `grafana-util`：

- 你只是臨時在 Grafana UI 改一個小設定
- 你只想查單一畫面上的某個值
- 你不需要匯出、審查、搬移、回放或自動化

如果工作本身不需要留下可重複、可審查的操作脈絡，直接用 Grafana UI 可能更快。

---

## 建議怎麼開始

第一次接觸時，不用先把所有 command 看完。比較自然的順序是：

1. 先看這個工具支援哪些連線與驗證方式
2. 先跑一次安全的唯讀檢查
3. 再決定要走新手、安全、SRE 或自動化路線
4. 需要精確語法時，再去看逐指令說明

如果你現在就是第一次使用，下一步建議接著看：

- [開始使用](getting-started.md)
- [新手快速入門](role-new-user.md)
- [指令詳細說明](../../commands/zh-TW/index.md)

---
[⬅️ 回手冊首頁](index.md) | [➡️ 下一章：開始使用](getting-started.md)
