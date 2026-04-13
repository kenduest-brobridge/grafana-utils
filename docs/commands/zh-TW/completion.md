# completion

## Root

用途：從目前 `grafana-util` 指令樹產生 shell completion script。

何時使用：
- 你希望 Bash 或 Zsh 能補完 command、subcommand 與 flags
- 你剛安裝新的 `grafana-util` binary，希望 completion 跟目前 binary 一致
- command 名稱或 flags 改過，需要重新產生 shell completion

重要輸入：
- shell 是必要 positional value：`bash` 或 `zsh`
- completion script 會輸出到 stdout；請 redirect 到你的 shell 會讀取的位置

範例：

```bash
# 產生 Bash completion。
grafana-util completion bash
```

```bash
# 產生 Zsh completion。
grafana-util completion zsh
```

## 這個指令做什麼

`grafana-util completion` 會從 Rust Clap command tree 產生 shell completion script。它不會連線到 Grafana，不會讀 profile，也不會解析認證。它只描述目前這個 binary 暴露出的 CLI surface。

因為 script 是從 command tree 產生的，升級 `grafana-util` 或切到不同 command 定義的 checkout 後，應該重新產生一次。

## 從 GitHub 安裝時一起安裝 completion

GitHub install script 可以安裝 binary 後，立刻用同一個 binary 產生 completion：

```bash
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | INSTALL_COMPLETION=auto sh
```

`auto` 會從 `SHELL` 偵測 `bash` 或 `zsh`。若想明確指定 shell，請用 `INSTALL_COMPLETION=bash` 或 `INSTALL_COMPLETION=zsh`。如果你的 shell 會從其他目錄讀取 completion file，可以設定 `COMPLETION_DIR=/path/to/dir`。

若要互動安裝，請把 `--interactive` 傳給 pipe 後面的 `sh`：

```bash
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | sh -s -- --interactive
```

互動模式會詢問 binary 安裝目錄、是否安裝 shell completion，以及 completion output directory。它會從終端機讀取回答，所以即使 install script 本身來自 `curl` pipe，也不會把 script 內容誤當成回答。

## 安裝到 Bash

選擇你的 Bash 設定已經會載入的 completion 目錄。常見的 per-user 位置是：

```bash
mkdir -p ~/.local/share/bash-completion/completions
grafana-util completion bash > ~/.local/share/bash-completion/completions/grafana-util
```

接著開一個新的 shell，或重新載入 Bash completion 設定。

## 安裝到 Zsh

選擇一個會出現在 `fpath` 裡的目錄。常見的 per-user 設定是：

```bash
mkdir -p ~/.zfunc
grafana-util completion zsh > ~/.zfunc/_grafana-util
```

然後確認 Zsh 在 `compinit` 前載入該目錄：

```zsh
fpath=(~/.zfunc $fpath)
autoload -Uz compinit
compinit
```

如果你的 Zsh 啟動檔還沒有這些設定，請把它們放進去。

## 成功判準

- 在 `grafana-util ` 後按 tab，會看到 `dashboard`、`datasource`、`alert`、`access`、`status`、`workspace`、`config`、`version`、`completion` 這類 root commands
- 在 subcommand 後按 tab，會看到目前 binary 知道的 flags 與下一層 subcommands
- 升級後重新產生 script，completion 內容會跟著更新

## 失敗時先檢查

- 如果沒有任何補完，先確認 shell 有載入你寫出的檔案
- 如果補完內容過舊，請用目前安裝的 binary 重新產生 script
- 如果 Bash 或 Zsh 無法載入檔案，確認你使用的 shell value 跟實際 shell 一致

## 相關指令

- [version](./version.md)
- [config](./config.md)
- [status](./status.md)
