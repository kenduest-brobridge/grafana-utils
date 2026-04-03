# `grafana-util profile`

## Root

用途：列出、檢視、新增與初始化 repo-local 的 `grafana-util` profile。

適用時機：當你想把 Grafana 連線預設放在目前 checkout，之後再用 `--profile` 重複使用。

主要旗標：root 指令本身只是命名空間；實際操作旗標都在子指令上。共用 root 旗標是 `--color`。

範例：

```bash
grafana-util profile list
grafana-util profile show --profile prod --output-format yaml
grafana-util profile add prod --url https://grafana.example.com --basic-user admin --prompt-password --store-secret encrypted-file
grafana-util profile add ci --url https://grafana.example.com --token-env GRAFANA_CI_TOKEN --store-secret os
grafana-util profile example --mode full
grafana-util profile init --overwrite
```

相關指令：`grafana-util status live`、`grafana-util overview live`、`grafana-util change plan`。

## `list`

用途：從解析後的 `grafana-util` 設定檔列出 profile 名稱。

適用時機：當你要確認目前 checkout 裡有哪些 profile 可用。

主要旗標：除了共用的 root `--color` 之外，沒有其他旗標。

範例：

```bash
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
grafana-util profile show --profile prod --output-format yaml
grafana-util profile show --profile prod --output-format json
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
grafana-util profile add dev --url http://127.0.0.1:3000 --basic-user admin --password-env GRAFANA_DEV_PASSWORD
grafana-util profile add prod --url https://grafana.example.com --basic-user admin --prompt-password --store-secret os --set-default
grafana-util profile add stage --url https://grafana-stage.example.com --token-env GRAFANA_STAGE_TOKEN --store-secret encrypted-file --prompt-secret-passphrase
```

說明：
- 預設 config path：`grafana-util.yaml`
- 預設加密秘密檔：`.grafana-util.secrets.yaml`
- `encrypted-file` 且未設 passphrase 時，預設本地 key file：`.grafana-util.secrets.key`
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
grafana-util profile example
grafana-util profile example --mode basic
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
grafana-util profile init
grafana-util profile init --overwrite
```

說明：
- `init` 會寫入內建起手範本。
- 如果你是想直接建立一個真正可用的 profile，通常 `add` 會比較順手。

相關指令：`profile add`、`profile example`、`status live`。
