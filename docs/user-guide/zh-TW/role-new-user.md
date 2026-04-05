# 👤 新手快速入門

這一頁是給第一次接觸 `grafana-util` 的人。目標不是一開始就背完所有指令，而是先把連線、驗證方式、profile 與唯讀檢查流程搞清楚。

先抓住一個重點：`grafana-util` 不是只能靠 profile 連線。這個工具本來就支援多種方式：

- 每次執行時直接帶 `--url` 與驗證參數
- 用 `--prompt-password` 或 `--prompt-token` 互動輸入，不把敏感值直接打在命令列
- 讓環境變數提供帳號、密碼或 token
- 把常用預設寫進專案本地的 profile，再用 `--profile` 重複使用

profile 的價值是把重複的連線資訊收斂起來，不是代表前面那些方式不能用。比較自然的學習順序是：

1. 先用一個安全的唯讀指令確認真的連得上 Grafana
2. 再搞清楚自己目前用的是哪一種驗證方式
3. 確認連線沒問題後，再把常用設定整理進 profile

## 適用對象

- 第一次接觸這個工具的工程師或維運人員。
- 需要先確認連線、版本與 profile 是否正常的人。
- 還不需要執行匯入、套用或跨 org 作業的使用者。

## 主要目標

- 先確認執行檔、Grafana 連線與唯讀檢查都正常。
- 了解這個工具支援哪些連線與驗證方式。
- 釐清何時直接帶旗標就夠，何時應該改用 profile 簡化。
- 熟悉 `status live` 與 `overview live` 的差異。
- 了解 token 比較適合權限範圍明確的單一 org 自動化作業。

## 採用前後對照

- 以前：新手一開始很容易卡在 profile、token、Basic auth 要先選哪個。
- 現在：先用一次直接唯讀命令證明連得上，再把重複設定整理進 profile。

## 成功判準

- 你能跑出一條安全的 live 唯讀命令，而且看得懂輸出。
- 你知道目前使用的是哪一種驗證方式，也知道為什麼。
- 你能判斷什麼時候適合用 profile，什麼時候直接帶旗標就夠。

## 失敗時先檢查

- 如果第一條 live read 失敗，先停下來修連線或驗證，不要直接進入變更流程。
- 如果你不確定目前是從哪個 auth source 取值，先檢查命令列和 profile 檔。
- 如果你正準備把密碼直接打進 shell 歷史，先改用 env-backed 或 prompt 方式。

## 典型新手任務

- 確認執行檔已加入 `PATH` 環境變數。
- 先對本地實驗環境或開發用 Grafana 跑一次直接的唯讀檢查。
- 確認沒問題後，再為同一個 Grafana 建立 profile。
- 執行一次安全的即時讀取 (Live Read)，並區分 `status live` 與 `overview live` 的差異。
- 瞭解後續進行儀表板、告警或存取權限管理時應參考的說明文件。

## 先搞懂連線與驗證是怎麼運作的

`grafana-util` 可以從幾個地方拿連線資訊：

- `--url`：指定 Grafana 位址
- `--basic-user` 搭配 `--basic-password`，或改用 `--prompt-password`
- `--token`，或改用 `--prompt-token`
- `GRAFANA_USERNAME`、`GRAFANA_PASSWORD`、`GRAFANA_API_TOKEN` 這類環境變數
- `--profile`：從 `grafana-util.yaml` 讀取已整理好的預設值

也就是說，你完全可以先用一次性的直接旗標確認連線，再決定要不要把同一組設定整理成 profile。

## 身分驗證與秘密資料處理建議

1. **第一次連線先用直接旗標驗證**：例如 `--url` 加上 Basic auth，先確定這台 Grafana 真的連得到。
2. **日常重複工作改用 `--profile`**：連線確認後，把 URL、帳號與 secret 來源整理進 profile，之後就不用每次重打。
3. **本機引導或臨時檢查可用 Basic auth**：若還沒建 profile，可先用 `--basic-user` 搭配 `--prompt-password`。
4. **特定自動化場景才直接用 token**：只有在你很清楚 token 權限範圍時，才建議直接拿來跑命令。
5. **secret 優先放在環境變數或 secret store**：像 `password_env`、`token_env`、`os` 或 `encrypted-file` 都比把敏感值直接打在命令列上安全。

## 建議先執行的 5 個指令

```bash
# 用途：建議先執行的 5 個指令。
grafana-util --version
```

```bash
# 用途：建議先執行的 5 個指令。
grafana-util status live --url http://localhost:3000 --basic-user admin --prompt-password --output-format yaml
```

```bash
# 用途：建議先執行的 5 個指令。
grafana-util profile init --overwrite
```

```bash
# 用途：建議先執行的 5 個指令。
grafana-util profile add dev --url http://127.0.0.1:3000 --basic-user admin --prompt-password
```

```bash
# 用途：建議先執行的 5 個指令。
grafana-util status live --profile dev --output-format yaml
```

這個順序不是隨便排的：

- 先看 binary 有沒有裝好
- 再用一次直接旗標確認 Grafana 真的連得通
- 確認沒問題後再初始化設定檔
- 把同一組連線整理成 profile
- 最後再用 `--profile` 跑同一種唯讀檢查

如果你暫時還沒有 profile，這就是最短的安全起手式：

```bash
# 用途：如果你暫時還沒有 profile，這就是最短的安全起手式。
grafana-util status live --url http://localhost:3000 --basic-user admin --prompt-password --output-format yaml
```

如果你手邊已有範圍明確的 token，也可以直接做同一類唯讀檢查：

```bash
# 用途：如果你手邊已有範圍明確的 token，也可以直接做同一類唯讀檢查。
grafana-util overview live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format json
```

如果你的 shell 已經有環境變數，也可以不先建 profile，直接這樣跑：

```bash
# 用途：如果你的 shell 已經有環境變數，也可以不先建 profile，直接這樣跑。
export GRAFANA_USERNAME=admin
export GRAFANA_PASSWORD=admin
grafana-util status live --url http://localhost:3000 --output-format yaml
```

## 學習進度檢核

當你符合以下幾點時，就可以進到後續章節：

- 可在常用的終端機環境中正常執行 `grafana-util --version`。
- 至少有一個直接的唯讀指令能穩定連到目標 Grafana。
- `profile show --profile dev` 解析出的欄位符合預期。
- `status live --profile dev` 能穩定回傳可讀的結果。
- 已清楚後續要進行 dashboard、alert 或 access 管理的操作流程。

## 後續閱讀建議

- [開始使用](getting-started.md)
- [技術參考手冊](reference.md)
- [疑難排解與名詞解釋](troubleshooting.md)

## 推薦搭配參考的指令頁面

- [profile](../../commands/zh-TW/profile.md)
- [status](../../commands/zh-TW/status.md)
- [overview](../../commands/zh-TW/overview.md)
- [指令詳細總索引](../../commands/zh-TW/index.md)

## 常見錯誤與限制

- **不要以為一開始就一定要先建 profile**：第一次確認連線時，先跑一個直接的唯讀命令很正常。
- **旗標誤用**：請勿混用 `--output-format` 與 `--output`，這兩個旗標位於不同的輸出控制層級。
- **設定檔安全性**：請勿在 `grafana-util.yaml` 中寫入明文密碼，除非僅用於一次性的實驗或展示。
- **Token 權限限制**：窄權限 token 無法執行所有操作，特別是跨 org 盤點或管理類任務。
- **循序漸進**：在熟悉 Profile、Status 與 Overview 的讀取流程前，建議先不要執行匯入或套用變更的作業。

## 何時切換至深度文件

- **流程理解**：需要理解完整操作脈絡時，請參閱維運指南 (Handbook) 章節。
- **精確查閱**：已知操作流程僅需確認特定旗標時，請參閱指令詳細說明頁。
- **故障排除**：語法正確但結果不符預期時，請參閱疑難排解章節。

## 下一步

- [回到手冊首頁](index.md)
- [開始使用](getting-started.md)
- [技術參考手冊](reference.md)
- [查看指令詳細總索引](../../commands/zh-TW/index.md)
