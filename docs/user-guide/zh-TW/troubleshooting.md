# 🔍 疑難排解與名詞解釋

本章不是只列錯誤訊息，而是幫你分辨問題到底屬於哪一類：

- auth 還是 scope
- live 還是 staged
- 指令形狀錯，還是輸出形狀不對
- 本機 profile 設定問題，還是遠端 Grafana 行為

如果你正在追查驗證或連線設定，請把 [config profile](../../commands/zh-TW/config.md)、[指令詳細說明總索引](../../commands/zh-TW/index.md) 與 [access](../../commands/zh-TW/access.md) 一起開著。

## 適用對象

- 遇到錯誤訊息，但不確定是語法、權限還是 scope 的人
- 想先分辨 live / staged / profile 問題的人
- 需要把常見錯誤整理成可重複排查流程的人

## 主要目標

- 先分辨問題類型
- 再決定要看哪一頁、跑哪個檢查
- 最後才進到修正或回報

## 採用前後對照

- 以前：看到錯誤時，很難分辨是語法、scope，還是工作流選錯。
- 現在：把連線、範圍、staged input 與輸出格式問題拆開來看，排障會快很多。

## 成功判準

- 你可以在看完整份 command 頁之前，就先判斷問題大概屬於哪一類。
- 你知道該先修驗證、範圍、staged input，還是輸出格式。
- 你能判斷這個問題是不是應該回到工作流章節，而不是只留在這裡。

## 失敗時先檢查

- 如果看完第一步還是不清楚症狀，先不要繼續動 live。
- 如果語法錯誤蓋掉真正問題，先確認 lane 與 auth source。
- 如果輸出形狀怪，但命令其實正常結束，先回文件確認 contract，不要直接懷疑 renderer。

---

## 🛠️ CLI 診斷與調修

### 1. 啟用 verbose 日誌

`grafana-util` 使用標準 Rust logging。你可以提高日誌等級來看實際 API request / response。

```bash
# 用途：grafana-util 使用標準 Rust logging。你可以提高日誌等級來看實際 API request / response。
RUST_LOG=debug grafana-util status live --profile prod
grafana-util dashboard list -v
```

適合拿來回答這些問題：

- CLI 打到的是不是你以為的那台主機
- 是 auth、scope，還是 network 在拒絕請求
- 指令形狀是不是和文件理解的不一樣

### 2. 常見錯誤與快速修補

| 症狀 | 常見原因 | 建議修補方式 |
| :--- | :--- | :--- |
| `401 Unauthorized` | token 或帳密無效 | 檢查 profile、環境變數、或實際輸入的憑證 |
| `403 Forbidden` | credential 有效，但權限不足 | 確認角色/權限，或改用更廣的管理員憑證 |
| `Connection Refused` | URL 錯誤或網路阻擋 | 驗證 `--url` 與 Grafana 網路可達性 |
| `Timeout` | estate 太大或後端太慢 | 增加 `--timeout`，必要時先縮小 scope |

### 3. 權限範圍與驗證方式不匹配

這類問題最麻煩的地方是，看起來像「指令有跑成功」，但其實回傳結果不完整。

| 症狀 | 常見原因 | 下一步先檢查什麼 |
| :--- | :--- | :--- |
| `--all-orgs` 回來的 org 比預期少 | token scope 比要求的讀取範圍窄 | 改用 admin-backed profile 或 direct Basic auth 重試 |
| read-only status 可跑，但 access/admin 類指令失敗 | credential 有效，但範圍不夠 | 對照目前 credential 與要執行的 command family |
| 同一個 token 在某個 job 可用，換一個 job 就失敗 | 第二個 job 用到更廣的操作面 | 檢查該流程是否其實該用 profile-backed Basic auth |

原則：

- 驗證成功不代表 scope 一定足夠
- 如果輸出「看起來怪怪但不是完全報錯」，先懷疑 scope，不要先懷疑 parser

### 4. staged 與 live 搞混

這是最常見的維運誤判之一。

| 症狀 | 真正發生的事 | 建議修補方式 |
| :--- | :--- | :--- |
| `status staged` 看起來健康，但 live apply 仍失敗 | staged 檔案結構正確，不代表 live state 或權限也正確 | 先跑 `status live`，再跑 `workspace test`、`workspace preview` 或 command-specific dry-run |
| `status overview live` 看起來正常，就直接略過 workspace review | live 可讀性不等於 staged 套件正確 | apply 前仍要跑 staged gate 與 preview path |
| import 或 apply 比預期改得更多 | staged 套件從來沒有先做 inspect 或 preview | 執行前先用 `workspace scan`、`workspace preview` 與 `--dry-run` |

### 5. profile 與 secret 問題

| 症狀 | 常見原因 | 建議修補方式 |
| :--- | :--- | :--- |
| `config profile show --show-secrets` 解不出來 | env var 不存在、OS store entry 不見，或 encrypted secret file / key 不見 | 回頭檢查 config profile 指到的 secret source |
| 本機可跑、CI 跑不起來 | env 注入或 config path 不同 | 檢查 `GRAFANA_UTIL_CONFIG`、env vars 與必要的 secret files |
| `--store-secret os` 在 macOS 可用，但 Linux 不行 | Linux runner 沒有可用的 Secret Service session | 改用 `password_env`、`token_env` 或 `encrypted-file` |

### 6. 輸出格式用錯

| 症狀 | 常見原因 | 建議修補方式 |
| :--- | :--- | :--- |
| CI parser 突然壞掉 | 用到了給人看的輸出模式 | 改用 `json`、`yaml` 或其他適合腳本處理的結構化輸出 |
| 指令不接受某個 `--output-format` 值 | 這個指令只支援較少的輸出格式 | 直接查該 command help 或指令頁 |
| 第一次檢查時互動式輸出太複雜 | 畫面雖然好看，但資訊不夠直接 | 先切成 `yaml` 或 `json` |

---

## 📖 名詞解釋

| 術語 | 定義 |
| :--- | :--- |
| **Live** | 直接讀目前正在執行的 Grafana；第一次確認連線時先從這裡開始。 |
| **Staged** | 本地已準備好、但還沒套用到 Grafana 的檔案或變更包。 |
| **Overview** | 給人快速判斷下一步的總覽；讀 live Grafana 時用 `status overview live`。 |
| **Workspace** | 一包本地輸入，應該先 scan、test、preview，再決定是否 apply。 |
| **Governance** | 用政策或規則先抓風險；不是第一次使用時的起手式。 |
| **Promotion** | 把已審查的資產從一個環境或交付包推往另一個目標環境。 |
| **Surface** | 高階操作面分類，例如 `status`、`dashboard`、`workspace`、`access`。 |
| **Lane** | 資料路徑，例如 `raw/`、`prompt/`、`provisioning/`。 |
| **Contract** | 用來定義 readiness 或 compatibility 的結構化 JSON 文件 |
| **Masked Recovery** | 匯出時先把 secrets 遮蔽，匯入或 replay 時再補回 |
| **Desired State** | 儲存在 Git 中、CLI 拿來對照 live Grafana 的目標設定 |
| **Drift** | live Grafana 與本地 staged / desired 輸出物之間的差距 |

---

## 🆘 取得更多協助

- **先確認版本**：回報問題時先執行 `grafana-util --version`
- **專案儲存庫**：請在 [GitHub Issues](https://github.com/kenduest-brobridge/grafana-util/issues) 回報 Bug 或提需求

回報問題時，盡量附上：

- 完整指令
- 這次是 live 還是 staged
- 使用的是 `--profile`、direct Basic auth，還是 token auth
- 問題比較像語法、連線、scope，還是 staged input shape

---
[⬅️ 上一章：實戰錦囊與最佳實踐](recipes.md) | [🏠 回首頁](index.md)
