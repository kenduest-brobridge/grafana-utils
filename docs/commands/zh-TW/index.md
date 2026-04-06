# 指令詳細說明

## 語言切換

- 繁體中文指令詳細說明：[目前頁面](./index.md)
- English command reference: [英文指令總索引](../en/index.md)
- 繁體中文手冊：[維運手冊](../../user-guide/zh-TW/index.md)
- English handbook: [Operator Handbook](../../user-guide/en/index.md)

---

這個目錄收錄 `grafana-util` 各個 command 與 subcommand 的獨立頁面。  
如果手冊章節是幫你理解整體工作流程，這裡就是用來查實際語法、常用旗標，以及相近命令差在哪裡的地方。

## 輸出旗標慣例

很多 list、review、dry-run 類指令，同時支援完整寫法與較短的快捷旗標。

常見對應關係：

- `--output-format table` 通常等於 `--table`
- `--output-format json` 通常等於 `--json`
- `--output-format csv` 通常等於 `--csv`
- `--output-format yaml` 通常等於 `--yaml`
- `--output-format text` 通常等於 `--text`

如果你是在腳本、CI 或範本裡統一帶參數，建議優先用完整寫法。  
如果你是在終端直接操作，快捷旗標會比較省事。

但也有幾個重要例外：

- 有些指令只支援部分快捷旗標，不一定每種格式都有短寫
- `dashboard topology` 是特例：它支援 `text`、`json`、`mermaid`、`dot`，但沒有 `--table` 這類快捷旗標
- `--output-file`，或某些匯出 / draft 指令裡的 `--output`，代表的是輸出檔案路徑，不是輸出格式

所以如果你要確認某一條指令到底支援哪些格式，還是以該指令頁面為準。

如果你是從繁體中文手冊進來，建議這樣使用：

| 你要查什麼 | 建議閱讀順序 |
| :--- | :--- |
| 先理解功能目的和操作流程 | 先讀 `docs/user-guide/zh-TW/` 對應章節 |
| 需要查某個 command 或 subcommand 怎麼用 | 直接進這裡的繁中指令頁 |
| 想核對目前 Rust CLI help 的實際語法 | 以這裡的指令頁為主，必要時再對照英文頁 |

如果你習慣用 `man` 格式閱讀頂層命令，macOS 可執行 `man ./docs/man/grafana-util.1`，GNU/Linux 可執行 `man -l docs/man/grafana-util.1`。

## Dashboard

- [dashboard](./dashboard.md)
- [dashboard browse](./dashboard-browse.md)
- [dashboard fetch-live](./dashboard-fetch-live.md)
- [dashboard clone-live](./dashboard-clone-live.md)
- [dashboard list](./dashboard-list.md)
- [dashboard export](./dashboard-export.md)
- [dashboard raw-to-prompt](./dashboard-raw-to-prompt.md)
- [dashboard import](./dashboard-import.md)
- [dashboard patch-file](./dashboard-patch-file.md)
- [dashboard serve](./dashboard-serve.md)
- [dashboard edit-live](./dashboard-edit-live.md)
- [dashboard review](./dashboard-review.md)
- [dashboard publish](./dashboard-publish.md)
- [dashboard delete](./dashboard-delete.md)
- [dashboard diff](./dashboard-diff.md)
- [dashboard list-vars](./dashboard-list-vars.md)
- [dashboard governance-gate](./dashboard-governance-gate.md)
- [dashboard topology](./dashboard-topology.md)
- [dashboard impact](./dashboard-impact.md)
- [dashboard history](./dashboard-history.md)
- [dashboard screenshot](./dashboard-screenshot.md)

### 相容別名頁面

- [dashboard analyze（本地別名）](./dashboard-analyze-export.md)
- [dashboard analyze（即時別名）](./dashboard-analyze-live.md)

## Datasource

- [datasource](./datasource.md)
- [datasource types](./datasource-types.md)
- [datasource list](./datasource-list.md)
- [datasource browse](./datasource-browse.md)
- [datasource export](./datasource-export.md)
- [datasource import](./datasource-import.md)
- [datasource diff](./datasource-diff.md)
- [datasource add](./datasource-add.md)
- [datasource modify](./datasource-modify.md)
- [datasource delete](./datasource-delete.md)

## Alert

- [alert](./alert.md)
- [alert export](./alert-export.md)
- [alert import](./alert-import.md)
- [alert diff](./alert-diff.md)
- [alert plan](./alert-plan.md)
- [alert apply](./alert-apply.md)
- [alert delete](./alert-delete.md)
- [alert add-rule](./alert-add-rule.md)
- [alert clone-rule](./alert-clone-rule.md)
- [alert add-contact-point](./alert-add-contact-point.md)
- [alert set-route](./alert-set-route.md)
- [alert preview-route](./alert-preview-route.md)
- [alert new-rule](./alert-new-rule.md)
- [alert new-contact-point](./alert-new-contact-point.md)
- [alert new-template](./alert-new-template.md)
- [alert list-rules](./alert-list-rules.md)
- [alert list-contact-points](./alert-list-contact-points.md)
- [alert list-mute-timings](./alert-list-mute-timings.md)
- [alert list-templates](./alert-list-templates.md)

## Access

- [access](./access.md)
- [access user](./access-user.md)
- [access org](./access-org.md)
- [access team](./access-team.md)
- [access service-account](./access-service-account.md)
- [access service-account token](./access-service-account-token.md)

## 共用介面

- [change](./change.md)
- [change inspect](./change-inspect.md)
- [change check](./change-check.md)
- [change preview](./change-preview.md)
- [change apply](./change-apply.md)
- [change advanced](./change.md#advanced)
- [change advanced 子指令（summary、plan、review、audit、bundle、promotion handoff）](./change.md#advanced)
- [overview](./overview.md)
- [overview live](./overview.md#live)
- [status](./status.md)
- [status staged](./status.md#staged)
- [status live](./status.md#live)
- [profile](./profile.md)
- [profile list](./profile.md#list)
- [profile show](./profile.md#show)
- [profile add](./profile.md#add)
- [profile example](./profile.md#example)
- [profile init](./profile.md#init)
- [snapshot](./snapshot.md)
- [snapshot export](./snapshot.md#export)
- [snapshot review](./snapshot.md#review)

## 通用資源查詢

- [resource](./resource.md)
- [resource describe](./resource-describe.md)
- [resource kinds](./resource-kinds.md)
- [resource list](./resource-list.md)
- [resource get](./resource-get.md)
