# 🏛️ 系統架構與設計原則

理解 `grafana-util` 的設計原則，會幫助你在管理較大規模的 Grafana 環境時做出比較穩定的判斷。這一章不只解釋為什麼這樣設計，也會說明這些設計會怎麼影響實際操作。

## 適用對象

- 想理解這套工具為什麼這樣分層的人
- 要決定團隊怎麼用 status / overview / change 的人
- 想知道 lane、masked recovery 與 staged/live 差異的人

## 主要目標

- 先把操作面分清楚
- 再把資產路徑與 secret 管理規則看懂
- 最後理解 dashboard 與 alert 為什麼故意走不同模型

## 採用前後對照

- 以前：很容易以為這個 repo 只是一堆長得差不多的指令集合。
- 現在：架構章節會說明為什麼這個工具要拆成不同操作面、lane 與 command 群組，讓你知道每條流程應該放在哪一層。

## 成功判準

- 你能說明為什麼 runtime 和文件要分成現在這種形狀。
- 你能判斷某個工作流應該放在手冊、指令詳細說明，還是內部維護文件。
- 你在實作新功能前，就能先選對對應的操作面。

## 失敗時先檢查

- 如果一條工作流找不到對應的操作面，先停下來判斷它是要開新章節還是開新 command 群。
- 如果 runtime 的形狀和文件的形狀開始分岔，要把它當成架構問題，而不只是文件問題。
- 如果你還不確定這個切分為什麼存在，先回去重讀 surface 與 lane 的段落，再往下加工作。

如果想對照這些概念實際對應到哪些指令，請搭配 [status](../../commands/zh-TW/status.md)、[overview](../../commands/zh-TW/overview.md)、[change](../../commands/zh-TW/change.md) 與 [dashboard](../../commands/zh-TW/dashboard.md) 一起看。

---

## 🏗️ 三層操作面模式

`grafana-util` 把維運工作拆成三種獨立的操作面，避免把「給人看的資訊」和「給程式判斷的資料」混在一起。

| 介面類型 | 核心用途 | 主要受眾 | 輸出格式 |
| :--- | :--- | :--- | :--- |
| **Status** | **整備度與技術合約** | CI/CD 管線、自動化腳本 | JSON, Table |
| **Overview** | **全域觀測性** | SRE 工程師、維運主管 | 互動式 TUI, 摘要報告 |
| **Change** | **變更意向與生命週期** | PR 審查、稽核紀錄 | JSON 計畫書, Diff |

### 什麼時候該選哪一個操作面

- 需要 gate、結構化輸出，或明確 pass/fail 判斷時，用 `status`
- 需要從人的角度先看整個 estate、決定接下來往哪裡鑽時，用 `overview`
- 已經知道有變更意圖，要做 summary、preflight、plan、review、apply 時，用 `change`

常見判斷：

- 「我現在能不能放心往下做？」 -> `status live`
- 「整個 Grafana 環境現在長什麼樣？」 -> `overview live`
- 「我的 staged 套件結構和檢查結果是否合理？」 -> `status staged` + `change preflight`
- 「到底會改到什麼？」 -> `change summary`、`change plan`、`change review`

### 為什麼這個切分很重要

如果這三個操作面在團隊心中混成一團，最常發生的是：

- 拿給人看的摘要去當成 CI gate
- 把 live 讀取結果當成 staged 套件也正確的依據
- 因為目前 live 狀態看起來沒問題，就直接跳過 preflight / review

這個設計是刻意分清楚的，目的就是讓你不用猜哪個輸出適合拿去自動化，哪個輸出比較適合給人判斷方向。

---

## 🛣️ 路徑隔離政策

為了避免設定漂移與混雜資產，我們把資料路徑明確分開。

1. **Raw Lane (`raw/`)**：與 API 100% 同步的原始快照。用於備份與災難恢復 (DR)。禁止人工手動編輯。
2. **Prompt Lane (`prompt/`)**：針對 UI 匯入最佳化的資產。剝離特定 metadata，確保新組織能乾淨接收。
3. **Provisioning Lane (`provisioning/`)**：磁碟型佈署專用檔案。這是從 API 模型轉換而來的單向投影。

### 路徑隔離對實際維運判斷的影響

- 用哪一條 lane，就代表你選了對應的工作流程；不要因為檔案看起來相似就混用
- `raw/` 是 dashboard 的標準回放與稽核路徑
- `prompt/` 適合跨環境搬移與 Grafana UI 匯入
- `provisioning/` 是部署投影，不是可以隨意當成 source of truth 的地方

如果忽略路徑隔離，最常見的問題不是語法錯，而是比較隱晦的漂移：

- dashboard 帶著不該帶的環境資訊被匯入
- provisioning 檔與標準匯出樹逐漸脫節
- 團隊後來無法解釋 live state 為什麼和手上的檔案不一致

### 不確定時怎麼選路徑

- 備份、回放、diff、audit：用 `raw/`
- 跨環境遷移、乾淨匯入：用 `prompt/`
- 目標是 Grafana disk provisioning：用 `provisioning/`

---

## 🔐 秘密治理 (Masked Recovery)

對於敏感資訊，例如 data source 密碼與 secure connection 欄位，`grafana-util` 採用 **「預設安全 (Safe-by-Default)」** 的做法。

- **匯出 (Export)**：敏感欄位會被遮蔽 (masked)，匯出檔可以安全地進 Git。
- **恢復 (Recovery)**：執行 `import` 時，CLI 會辨識哪些 secret 缺失，並透過環境變數或互動式提示提供安全的補回流程。

### 這件事在實務上代表什麼

這個設計是為了避免兩種最糟的結果：

- 可用的 data source secret 被直接洩漏到 Git
- 團隊誤以為 masked export 已經包含完整 replay 所需的全部資料

理想狀態是：

- 你可以安全地把 data source inventory commit 進 Git
- replay / import 流程會清楚指出哪些 secret 還需要補回
- 團隊知道 secret recovery 是明確步驟，不是暗中完成的副作用

---

## 🔄 狀態流轉模型 (State Transition)

`grafana-util` 對 Alerting 是「**狀態調解器 (State Reconciler)**」，對 Dashboard 則是「**快照回放器 (Snapshot Replayer)**」。

- **Dashboard (Snapshot/Replay)**：命令式 (Imperative)。「讓目標 Grafana 此刻看起來與此檔案完全一致」。
- **Alerting (Desired State)**：宣告式 (Declarative)。「先計算我的檔案與伺服器之間的差異 (Plan)，再只套用該差異」。

### 為什麼 dashboard 和 alert 故意走不同模型

它們解決的是不同的維運問題：

- dashboard 更像一組可匯出、可檢查、可 patch、可 replay 的 artifact
- alert 更像一組需要先看 delta、再 review、最後才 apply 的 desired state

實務影響：

- dashboard 請優先思考輸出物品質、路徑選擇與 replay 目標
- alert 請優先思考 staged intent、route 正確性、plan review 與受控 apply

### 快速判斷

- 你第一個問題是「這是不是正確的 replay artifact？」時，通常是 dashboard 思維
- 你第一個問題是「它到底會造成什麼 delta？」時，通常是 alert / change 思維

---

## ✅ 什麼叫做架構真的有幫到你

當下面幾點成立時，代表這套架構不是只有名詞，而是真的落地：

- 團隊能清楚分辨 `status`、`overview`、`change`
- live check 與 staged check 不再被當成可互換
- dashboard lanes 不會被隨意混用
- 被遮蔽的 secret 匯出被當成安全輸出物，而不是完整回放內容
- 維運者知道什麼時候應該停在唯讀驗證，什麼時候才進入 plan/apply 流程

---
[⬅️ 上一章：開始使用](getting-started.md) | [🏠 回首頁](index.md) | [➡️ 下一章：Dashboard 管理](dashboard.md)
