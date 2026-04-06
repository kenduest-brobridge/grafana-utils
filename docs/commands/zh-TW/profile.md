# `grafana-util profile`

## Root

用途：列出、檢視、新增與初始化 repo-local 的 `grafana-util` profile。

適用時機：當你想把 Grafana 連線預設放在目前 checkout，之後再用 `--profile` 重複使用。

說明：如果你想先理解整個 profile 工作流，再決定要進哪個子命令，先看這一頁最合適。`profile` 指令群組是 repo-local 連線預設、secret 處理，以及本機與 CI 重複執行方式的入口。

## 採用前後對照

- **採用前**：連線設定散在各種旗標或 shell 歷史裡，想重跑同一個 live 指令時很容易漏掉參數。
- **採用後**：一個具名 profile 就能把 URL、驗證與 secret 處理收在一起，live 指令會短很多，也比較好重複使用。

## 成功判準

- 你想重複使用的連線設定可以被一個 profile 名稱完整代表
- secret 保存模式符合目前環境，不需要每條命令都重複寫驗證資訊
- 下游 live 指令因為 profile 接手重複參數，所以還維持得住可讀性

## 失敗時先檢查

- 如果切換 profile 後指令失敗，先看 `show` 的解析結果，再確認是不是命令本身有問題
- 如果秘密值不見了，先確認目前 profile 使用的是 `file`、`os` 還是 `encrypted-file` 模式，以及這個模式是否適合目前機器
- 如果 live 指令還是要帶一長串旗標，可能代表 profile 還沒把預設 URL 或 auth 值收進去

主要旗標：root 指令本身只是指令群組；實際操作旗標都在子指令上。共用 root 旗標是 `--color`。

範例：

```bash
# 用途：列出目前 checkout 可用的 profile。
grafana-util profile list
```

```bash
# 用途：在執行 live 指令前，先查看解析後的 profile。
grafana-util profile show --profile prod --output-format yaml
```

```bash
# 用途：建立可重複使用的 production profile，並用互動式密碼保存 secret。
grafana-util profile add prod --url https://grafana.example.com --basic-user admin --prompt-password --store-secret encrypted-file
```

```bash
# 用途：建立會從環境變數讀取 token 的 CI profile。
grafana-util profile add ci --url https://grafana.example.com --token-env GRAFANA_CI_TOKEN --store-secret os
```

```bash
# 用途：輸出一份註解完整的 profile 範本。
grafana-util profile example --mode full
```

```bash
# 用途：在目前 checkout 初始化一份新的 grafana-util.yaml。
grafana-util profile init --overwrite
```

相關指令：`grafana-util status live`、`grafana-util overview live`、`grafana-util change preview`。

## `list`

用途：從解析後的 `grafana-util` 設定檔列出 profile 名稱。

適用時機：當你要確認目前 checkout 裡有哪些 profile 可用。

主要旗標：除了共用的 root `--color` 之外，沒有其他旗標。

範例：

```bash
# 用途：list。
grafana-util profile list
```

相關指令：`profile show`、`profile add`、`profile init`。

## `show`

用途：以 text、table、csv、json 或 yaml 顯示目前選定的 profile。

適用時機：當你想在執行 live 指令前，先確認最後解析到的連線設定。

主要旗標：
- `--profile`
- `--output-format`
- `--show-secrets`

範例：

```bash
# 用途：show。
grafana-util profile show --profile prod --output-format yaml
```

```bash
# 用途：show。
grafana-util profile show --profile prod --output-format json
```

```bash
# 用途：show。
grafana-util profile show --profile prod --show-secrets --output-format yaml
```

說明：
- 預設會遮蔽秘密值。
- 加上 `--show-secrets` 才會顯示明文，或解出 secret-store 參照。

相關指令：`profile list`、`profile add`、`status live`、`overview live`。

## `add`

用途：不用手改 `grafana-util.yaml`，直接建立或覆蓋一個命名 profile。

適用時機：當你想更快建立可重用的連線設定，尤其是需要把驗證資訊一起記住時。

主要旗標：
- `--url`
- 驗證輸入：`--token`、`--token-env`、`--prompt-token`、`--basic-user`、`--basic-password`、`--password-env`、`--prompt-password`
- 秘密保存模式：`--store-secret file|os|encrypted-file`
- `encrypted-file` 相關：`--secret-file`、`--prompt-secret-passphrase`、`--secret-passphrase-env`
- 行為控制：`--replace-existing`、`--set-default`

範例：

```bash
# 用途：add。
grafana-util profile add dev --url http://127.0.0.1:3000 --basic-user admin --password-env GRAFANA_DEV_PASSWORD
```

```bash
# 用途：add。
grafana-util profile add prod --url https://grafana.example.com --basic-user admin --prompt-password --store-secret os --set-default
```

```bash
# 用途：add。
grafana-util profile add stage --url https://grafana-stage.example.com --token-env GRAFANA_STAGE_TOKEN --store-secret encrypted-file --prompt-secret-passphrase
```

說明：
- 預設 config path：`grafana-util.yaml`
- 預設加密秘密檔：`.grafana-util.secrets.yaml`
- `encrypted-file` 且未設 passphrase 時，預設本地 key file：`.grafana-util.secrets.key`
- `profile add --store-secret encrypted-file` 若 helper 檔位於 config 目錄樹內，會順手更新該目錄的 `.gitignore` 以忽略這些檔案。
- 這些預設 secret path 都是以 config file 所在目錄為基準，不是用臨時的 process cwd 去算。
- `file` 是預設模式。
- `os` 與 `encrypted-file` 都是明確 opt-in。
- `os` 模式會把 secret 放進 macOS Keychain 或 Linux Secret Service，而不是寫進 `grafana-util.yaml`。
- `os` 目前只支援 macOS 與 Linux；如果是 headless Linux shell，通常要改用 `password_env`、`token_env` 或 `encrypted-file`。
- 對重複執行的自動化工作，優先把秘密放進 profile 的 `password_env` 或 `token_env`，不要把秘密直接貼進每次 live 指令。

相關指令：`profile show`、`profile example`、`profile init`。

## `example`

用途：輸出一份帶完整註解的參考設定，方便直接拿來改。

適用時機：當你想看一份完整、可讀、可參考的 profile 設定範本，而不是只看零碎欄位說明。

主要旗標：
- `--mode basic|full`

範例：

```bash
# 用途：example。
grafana-util profile example
```

```bash
# 用途：example。
grafana-util profile example --mode basic
```

```bash
# 用途：example。
grafana-util profile example --mode full
```

說明：
- `basic` 是較短的起手範本。
- `full` 會包含 `file`、`os`、`encrypted-file` 三種模式的註解示例。
- `os` 類型範例的前提是本機 macOS Keychain 或 Linux Secret Service 可用。

相關指令：`profile add`、`profile init`、`profile show`。

## `init`

用途：在目前工作目錄初始化 `grafana-util.yaml`。

適用時機：當某個 checkout 還沒有 repo-local profile 檔案，而你想先產生內建起手範本時。

主要旗標：
- `--overwrite`

範例：

```bash
# 用途：init。
grafana-util profile init
```

```bash
# 用途：init。
grafana-util profile init --overwrite
```

說明：
- `init` 會寫入內建起手範本。
- 如果你是想直接建立一個真正可用的 profile，通常 `add` 會比較順手。

相關指令：`profile add`、`profile example`、`status live`。
