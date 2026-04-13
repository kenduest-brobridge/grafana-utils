# 指令參考

## 語言

- 英文指令說明：[English command reference](../en/index.md)
- 繁體中文指令說明：[目前頁面](./index.md)
- 英文手冊：[英文手冊](../../user-guide/en/index.md)
- 繁體中文手冊：[繁體中文手冊](../../user-guide/zh-TW/index.md)

---

這些頁面對應 `grafana-util` 目前 Rust CLI 的指令樹。

如果你要的是每個指令或子指令一頁的穩定說明，請看這裡；如果你想先理解操作流程與場景，請先看手冊。

指令參考故意比手冊短。它的任務是讓你在已經知道工作流後，快速確認這條命令何時用、會讀寫什麼、常見下一步是什麼，以及應該使用哪些 flags。若你還在判斷「我到底該走 dashboard、workspace、status 還是 access」，先回手冊；若你已經知道要執行哪條命令，這裡才是正確入口。

閱讀單一指令頁時，先看「何時使用」與「成功判準」，再看範例。不要一開始就從 flags 往回推工作流；那通常會讓人選到能跑但不適合的路徑。

如果你只想在終端機快速看完整公開指令清單與用途，請跑：

```bash
grafana-util --help-flat
```

## 先怎麼讀這份指令參考

1. 先在這頁判斷你是要走 `status`、`workspace`、`dashboard`、`alert`、`datasource` 還是 `access`。
2. 進到 domain 頁後，先讀「你現在要做哪一種事」與「先選哪條資料路徑」。
3. 再進到單一子命令頁，先看「何時使用」與「最短成功路徑」。
4. 最後才看完整 flags 與範例，避免一開始就被所有命令與選項淹沒。

## 先從這裡開始

這一頁不是把全部命令平鋪列出來，而是先把你導到正確的 command family。

如果你只是第一次進來，先走這三步：

1. [version](./version.md)：確認 binary 與版本
2. [completion](./completion.md)：產生 Bash 或 Zsh shell completion script
3. [status](./status.md)：確認 CLI 跟 Grafana 能不能溝通
4. [config](./config.md)：把 profile 與 secret 存起來，後面少打一堆參數

## 先選一條操作路徑

| 你現在最像哪一種情境 | 先開哪一頁 | 下一步通常接什麼 |
| :--- | :--- | :--- |
| 想先確認 CLI 與 Grafana 連得通 | [version](./version.md) / [status](./status.md) | `status live`、`status overview live` |
| 想把連線設定存起來，之後少打一堆參數 | [config](./config.md) | `config profile add/show/list` |
| 想先備份或抓本地 inventory | [export](./export.md) | `export dashboard` / `alert` / `datasource` |
| 想審查一包本地變更 | [workspace](./workspace.md) | `scan` -> `preview` -> `test` / `apply` |
| 想處理 dashboard 的瀏覽、分析、草稿或發佈 | [dashboard](./dashboard.md) | `browse` / `summary` / `diff` / `publish` |
| 想處理 alert inventory 或 authoring | [alert](./alert.md) | `list-*` / `new-*` / `apply` |
| 想處理 datasource inventory 或 lifecycle | [datasource](./datasource.md) | `list` / `export` / `diff` / `modify` |
| 想管理 user、team、org 或 service account | [access](./access.md) | `user` / `team` / `org` / `service-account` |

## 指令地圖

| 類型 | 入口頁 | 這組命令主要是做什麼 |
| :--- | :--- | :--- |
| 起步與連線 | [version](./version.md)、[completion](./completion.md)、[status](./status.md)、[config](./config.md) | 驗證 binary、安裝 shell completion、檢查 live 狀態、保存 profile |
| 匯出與離線成品 | [export](./export.md)、[snapshot bundles](./snapshot.md)、[resource queries](./resource.md) | 抓本地 inventory、做 snapshot、查單一資源 |
| 本地審查工作區 | [workspace](./workspace.md) | scan、test、preview、package、apply |
| Dashboard | [dashboard](./dashboard.md) | 瀏覽、分析、草稿、匯出匯入、發佈 |
| Alert | [alert](./alert.md) | inventory、authoring、route、plan / apply |
| Datasource | [datasource](./datasource.md) | inventory、類型查找、匯出匯入、live mutation |
| Access | [access](./access.md) | user、org、team、service account、token |

## 我該用哪個指令？

| 需求 | 先從這裡開始 |
| :--- | :--- |
| 確認目前安裝的 binary 或 scriptable version | `grafana-util version` |
| 安裝 shell completion | `grafana-util completion bash` 或 `grafana-util completion zsh` |
| 確認 Grafana 連得到 | `grafana-util status live` |
| 用人的角度看 live 總覽 | `grafana-util status overview live` |
| 儲存連線預設值 | `grafana-util config profile` |
| 匯出備份 | `grafana-util export dashboard` / `export alert` / `export datasource` |
| 審查本地變更包 | `grafana-util workspace scan`，再跑 `workspace preview` |
| 深入檢查 dashboard | `grafana-util dashboard summary` / `dashboard diff` |
| 通用查詢單一 live resource | `grafana-util status resource describe`、`list` 或 `get` |
| 匯出或檢視 snapshot bundle | `grafana-util status snapshot export` 或 `review` |
| 管理 user、team、org 或 service account | `grafana-util access ...` |

## 各領域入口與延伸頁

| 領域 | 先進哪頁 | 這頁之後通常會再去哪裡 |
| :--- | :--- | :--- |
| Dashboard | [dashboard](./dashboard.md) | [dashboard export](./dashboard-export.md)、[dashboard import](./dashboard-import.md)、[dashboard summary](./dashboard-summary.md)、[dashboard impact](./dashboard-impact.md) |
| Datasource | [datasource](./datasource.md) | [datasource export](./datasource-export.md)、[datasource import](./datasource-import.md)、[datasource diff](./datasource-diff.md) |
| Alert | [alert](./alert.md) | [alert export](./alert-export.md)、[alert import](./alert-import.md)、[alert plan](./alert-plan.md)、[alert apply](./alert-apply.md) |
| Access | [access](./access.md) | [access user](./access-user.md)、[access org](./access-org.md)、[access team](./access-team.md)、[access service-account](./access-service-account.md) |

## 常見工作流範例

| 你想完成的事 | 建議閱讀順序 |
| :--- | :--- |
| 第一次確認環境並存好連線設定 | [version](./version.md) -> [status](./status.md) -> [config](./config.md) |
| 先抓一份 dashboard 備份，再離線 review | [export](./export.md) -> [dashboard export](./dashboard-export.md) -> [dashboard review](./dashboard-review.md) |
| 先看 live dashboard 結構與依賴 | [dashboard](./dashboard.md) -> [dashboard summary](./dashboard-summary.md) -> [dashboard dependencies](./dashboard-dependencies.md) |
| 規劃 alert desired-state 變更 | [alert](./alert.md) -> [alert plan](./alert-plan.md) -> [alert apply](./alert-apply.md) |
| 檢查 datasource 現況再決定是否修改 | [datasource](./datasource.md) -> [datasource list](./datasource-list.md) -> [datasource modify](./datasource-modify.md) |
| 盤點 org / team / service account | [access](./access.md) -> [access org](./access-org.md) / [access team](./access-team.md) / [access service-account](./access-service-account.md) |

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
- `dashboard dependencies` 是特例：它支援 `text`、`json`、`mermaid`、`dot`，但沒有 `--table`
- 像 `--output-file`、`--output` 這種輸出檔路徑旗標不是 render format selector

如果不確定，請以該指令自己的頁面為準。

如果你偏好 `man` 形式，可查看 [grafana-util(1)](../../man/grafana-util.1)。

- macOS：`man ./docs/man/grafana-util.1`
- GNU/Linux：`man -l docs/man/grafana-util.1`

如果你偏好網頁閱讀，直接使用這份指令參考 HTML 即可。
