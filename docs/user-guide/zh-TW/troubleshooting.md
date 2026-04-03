# 🔍 疑難排解與名詞解釋

本章的目的不是只列錯誤訊息，而是幫您分辨問題到底屬於哪一類：

- auth 還是 scope
- live 還是 staged
- 指令形狀錯，還是輸出形狀不對
- 本機 profile wiring 問題，還是遠端 Grafana 行為

如果您正在追查驗證或連線設定，請把 [profile](../../commands/zh-TW/profile.md)、[status](../../commands/zh-TW/status.md)、[overview](../../commands/zh-TW/overview.md) 與 [access](../../commands/zh-TW/access.md) 一起開著。

---

## 🛠️ CLI 診斷與調修

### 1. 啟用 verbose 日誌

`grafana-util` 使用標準 Rust logging。您可以提高日誌等級來看實際 API request / response。

```bash
RUST_LOG=debug grafana-util overview live --profile prod
grafana-util dashboard list -v
```

適合拿來回答這些問題：

- CLI 打到的是不是您以為的那台主機
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
| 同一個 token 在某個 job 可用，換一個 job 就失敗 | 第二個 job 用到更廣的 surface | 檢查該 workflow 是否其實該用 profile-backed Basic auth |

原則：

- 驗證成功不代表 scope 一定足夠
- 如果輸出「看起來怪怪但不是完全報錯」，先懷疑 scope，不要先懷疑 parser

### 4. staged 與 live 搞混

這是最常見的維運誤判之一。

| 症狀 | 真正發生的事 | 建議修補方式 |
| :--- | :--- | :--- |
| `status staged` 看起來健康，但 live apply 仍失敗 | staged 檔案結構正確，不代表 live state 或權限也正確 | 先跑 `status live`，再跑 `change preflight` 或 command-specific dry-run |
| `overview live` 看起來正常，就直接略過 preflight / plan | live 可讀性不等於 staged package 正確 | apply 前仍要跑 staged gate 與 review path |
| import 或 apply 比預期改得更多 | staged package 從來沒有先做 summary 或 plan | 執行前先用 `change summary`、`change plan` 與 `--dry-run` |

### 5. profile 與 secret 問題

| 症狀 | 常見原因 | 建議修補方式 |
| :--- | :--- | :--- |
| `profile show --show-secrets` 解不出來 | env var 不存在、OS store entry 不見、或 encrypted secret file / key 不見 | 回頭檢查 profile 指到的 secret source |
| 本機可跑、CI 跑不起來 | env 注入或 config path 不同 | 檢查 `GRAFANA_UTIL_CONFIG`、env vars 與必要的 secret files |
| `--store-secret os` 在 macOS 可用，但 Linux 不行 | Linux runner 沒有可用的 Secret Service session | 改用 `password_env`、`token_env` 或 `encrypted-file` |

### 6. 輸出格式用錯

| 症狀 | 常見原因 | 建議修補方式 |
| :--- | :--- | :--- |
| CI parser 突然壞掉 | 用到了給人看的輸出模式 | 改用 `json`、`yaml` 或其他明確機器可讀輸出 |
| 指令不接受 `--output-format` | 這個 surface 實際上使用 `--output` | 直接查該 command help 或逐指令頁 |
| 第一次檢查時互動式輸出太複雜 | 畫面好看，但原始訊號不夠直觀 | 先切成 `yaml` 或 `json` |

---

## 📖 名詞解釋

| 術語 | 定義 |
| :--- | :--- |
| **Surface** | 高階介面分類，例如 `Status`、`Overview`、`Change` |
| **Lane** | 資產的隔離資料流路徑，例如 `raw/`、`prompt/`、`provisioning/` |
| **Contract** | 用來定義 readiness 或 compatibility 的機器可讀 JSON 文件 |
| **Masked Recovery** | 匯出時把 secrets 遮蔽，匯入或 replay 時再補回 |
| **Desired State** | 儲存在 Git 中、CLI 拿來對照 live Grafana 的目標設定 |
| **Drift** | live Grafana 與本地 staged / desired artifacts 之間的差距 |

---

## 🆘 取得更多協助

- **先確認版本**：回報問題時先執行 `grafana-util --version`
- **專案儲存庫**：請在 [GitHub Issues](https://github.com/kendlee/grafana-utils/issues) 回報 Bug 或提需求

回報問題時，盡量附上：

- 完整指令
- 這次是 live 還是 staged
- 使用的是 `--profile`、direct Basic auth，還是 token auth
- 問題比較像語法、連線、scope，還是 staged input shape

---
[⬅️ 上一章：實戰錦囊與最佳實踐](recipes.md) | [🏠 回首頁](index.md)
