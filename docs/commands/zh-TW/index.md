# 指令文件

## 語言

- 英文指令說明：[目前頁面](./index.md)
- 繁體中文指令說明：[目前頁面](./index.md)
- 英文手冊：[Operator Handbook](../../user-guide/en/index.md)
- 繁體中文手冊：[繁體中文手冊](../../user-guide/zh-TW/index.md)

---

這些頁面對應 `grafana-util` 目前 Rust CLI 的指令樹。

如果你要的是每個指令或子指令一頁的穩定說明，請看這裡；如果你想先理解操作流程與場景，請先看手冊。

## 先從這裡開始

現在建議的新手入口是比較小的 task-first surface：

- `observe`：唯讀狀態、overview、snapshot、resource 查詢
- `export`：常見備份與本地 inventory 擷取
- `change`：以 review 為先的 staged change workflow
- `config profile`：repo-local 連線與 secret 管理
- `advanced`：你已經知道是某個 domain 時，再打開進階樹

像 `dashboard`、`alert`、`datasource`、`access` 這些舊根命令仍可使用，但不再是第一層 onboarding 入口。
較舊的 `status`、`overview`、`profile` 頁面只保留作為相容參考；對外請以 `observe` 與 `config profile` 為主入口。

## 常用工作

- [change](./change.md)
- [change inspect](./change-inspect.md)
- [change check](./change-check.md)
- [change preview](./change-preview.md)
- [change apply](./change-apply.md)
- [export](./export.md)
- `export dashboard`
- `export alert`
- `export datasource`
- `export access user|org|team|service-account`
- `observe live`
- `observe staged`
- `observe overview`
- `observe snapshot`
- `observe resource describe|kinds|list|get`
- `config profile`

## 進階工作

- [advanced](./advanced.md)
- `advanced dashboard live|draft|sync|analyze|capture`
- `advanced alert live|migrate|author|scaffold|change`
- `advanced datasource types|list|browse|add|modify|delete|export|import|diff`
- `advanced access user|org|team|service-account`
- [migrate](./migrate.md)

## Domain 參考頁

- [dashboard](./dashboard.md)
- [dashboard export](./dashboard-export.md)
- [dashboard import](./dashboard-import.md)
- [datasource](./datasource.md)
- [datasource export](./datasource-export.md)
- [datasource import](./datasource-import.md)
- [alert](./alert.md)
- [alert export](./alert-export.md)
- [alert import](./alert-import.md)
- [access](./access.md)
- [access user](./access-user.md)
- [access org](./access-org.md)
- [access team](./access-team.md)
- [access service-account](./access-service-account.md)
- [access service-account token](./access-service-account-token.md)

## 輸出格式旗標慣例

許多 list、review、dry-run 指令同時支援長格式輸出選擇器與較短的快捷旗標。

常見對應如下：

- `--output-format table` 通常等同 `--table`
- `--output-format json` 通常等同 `--json`
- `--output-format csv` 通常等同 `--csv`
- `--output-format yaml` 通常等同 `--yaml`
- `--output-format text` 通常等同 `--text`

如果你要讓 script 或 template 更清楚，優先用長格式；如果你只是互動式操作，短旗標通常更快。

注意例外：

- 有些指令只支援其中一部分快捷旗標
- `dashboard topology` 是特例：它支援 `text`、`json`、`mermaid`、`dot`，但沒有 `--table`
- 像 `--output-file`、`--output` 這種輸出檔路徑旗標不是 render format selector

如果不確定，請以該指令自己的頁面為準。

如果你偏好 `man` 形式，可以在本地用 `man ./docs/man/grafana-util.1`（macOS）或 `man -l docs/man/grafana-util.1`（GNU/Linux）查看 [grafana-util(1)](../../man/grafana-util.1)。
版本庫中的 `docs/man/*.1` 是由這些英文 command source page 經 `python3 scripts/generate_manpages.py` 產生。
版本庫中的 `docs/html/commands/zh-TW/*.html` 則來自同一份 source，再經 `python3 scripts/generate_command_html.py` 產生。
