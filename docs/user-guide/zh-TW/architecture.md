# 🏛️ 系統架構與設計原則

理解 `grafana-util` 的架構哲學，是有效管理大規模 Grafana Estate 的關鍵。本章不只解釋設計上的 Why，也直接說明這些設計應該怎麼影響您的操作判斷。

若要對照這些概念背後的指令面，請參考 [status](../../commands/zh-TW/status.md)、[overview](../../commands/zh-TW/overview.md)、[change](../../commands/zh-TW/change.md) 與 [dashboard](../../commands/zh-TW/dashboard.md)。

---

## 🏗️ 三層介面模式 (The Three Surfaces)

`grafana-util` 將維運關注點拆分為三種獨立的「介面 (Surface)」，避免把「人類可讀的資訊」與「機器可讀的合約」混在一起。

| 介面類型 | 核心用途 | 主要受眾 | 輸出格式 |
| :--- | :--- | :--- | :--- |
| **Status** | **整備度與技術合約** | CI/CD 管線、自動化腳本 | JSON, Table |
| **Overview** | **全域觀測性** | SRE 工程師、維運主管 | 互動式 TUI, 摘要報告 |
| **Change** | **變更意向與生命週期** | PR 審查、稽核紀錄 | JSON 計畫書, Diff |

### 什麼時候該選哪一個 Surface

- 需要 gate、機器可讀結果，或明確 pass/fail 判斷時，用 `status`
- 需要從人的角度先看整個 estate、決定接下來往哪裡鑽時，用 `overview`
- 已經知道有變更意圖，要做 summary、preflight、plan、review、apply 時，用 `change`

常見判斷：

- 「我現在能不能放心往下做？」 -> `status live`
- 「整個 Grafana estate 現況長什麼樣？」 -> `overview live`
- 「我的 staged package 結構和門禁是否合理？」 -> `status staged` + `change preflight`
- 「到底會改到什麼？」 -> `change summary`、`change plan`、`change review`

### 為什麼這個切分很重要

如果三個 surface 在團隊心中混成一團，最常發生的是：

- 拿給人看的摘要去當成 CI gate
- 把 live read 當成 staged package 也正確的證明
- 因為目前 live 狀態看起來沒問題，就直接跳過 preflight / review

這個設計是故意有立場的，目的就是讓您不用猜哪個輸出能放心拿去自動化，哪個輸出是給人判斷方向用的。

---

## 🛣️ 隔離政策 (Lane Isolation Policy)

為了防止設定漂移 (Configuration Drift) 與「Frankenstein」式資產，我們對資料流 (Lanes) 實施嚴格隔離。

1. **Raw Lane (`raw/`)**：與 API 100% 同步的原始快照。用於備份與災難恢復 (DR)。禁止人工手動編輯。
2. **Prompt Lane (`prompt/`)**：針對 UI 匯入最佳化的資產。剝離特定 metadata，確保新組織能乾淨接收。
3. **Provisioning Lane (`provisioning/`)**：磁碟型佈署專用檔案。這是從 API 模型轉換而來的單向投影。

### Lane isolation 對實際維運判斷的影響

- 用哪一條 lane，就代表您選了哪一種工作流；不要因為檔案看起來相似就混用
- `raw/` 是 dashboard 的 canonical replay 與 audit lane
- `prompt/` 適合跨環境搬移與 UI-first adoption
- `provisioning/` 是部署投影，不是您隨意發明 source of truth 的地方

如果忽略 lane isolation，最常見的問題不是語法錯，而是比較隱晦的 drift：

- dashboard 帶著不該帶的環境資訊被匯入
- provisioning 檔與 canonical export tree 逐漸脫節
- 團隊後來無法解釋 live state 為什麼和手上的檔案不一致

### 不確定時怎麼選 lane

- 備份、回放、diff、audit：用 `raw/`
- 跨環境遷移、乾淨匯入：用 `prompt/`
- 目標是 Grafana disk provisioning：用 `provisioning/`

---

## 🔐 秘密治理 (Masked Recovery)

對於敏感資訊，例如 datasource 密碼與 secure connection 欄位，`grafana-util` 遵循 **「預設安全 (Safe-by-Default)」** 的架構。

- **匯出 (Export)**：敏感欄位會被遮蔽 (masked)，匯出檔可以安全地進 Git。
- **恢復 (Recovery)**：執行 `import` 時，CLI 會辨識哪些 secret 缺失，並透過環境變數或互動式提示提供安全的補回流程。

### 這件事在實務上代表什麼

這個設計是為了避免兩種最糟的結果：

- 可用的 datasource secret 被直接洩漏到 Git
- 團隊誤以為 masked export 已經包含完整 replay 所需的全部資料

理想狀態是：

- 您可以安全地把 datasource inventory commit 進 Git
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

- dashboard 請優先思考 artifact 品質、lane 選擇與 replay 目標
- alert 請優先思考 staged intent、route 正確性、plan review 與受控 apply

### 快速判斷

- 您第一個問題是「這是不是正確的 replay artifact？」時，通常是 dashboard 思維
- 您第一個問題是「它到底會造成什麼 delta？」時，通常是 alert / change 思維

---

## ✅ 什麼叫做架構真的有幫到你

當下面幾點成立時，代表這套架構不是只有名詞，而是真的落地：

- 團隊能清楚分辨 `status`、`overview`、`change`
- live check 與 staged check 不再被當成可互換
- dashboard lanes 不會被隨意混用
- 被遮蔽的 secret export 被當成安全 artifact，而不是完整 replay payload
- 維運者知道什麼時候應該停在唯讀驗證，什麼時候才進入 plan/apply 流程

---
[⬅️ 上一章：開始使用](getting-started.md) | [🏠 回首頁](index.md) | [➡️ 下一章：Dashboard 管理](dashboard.md)
