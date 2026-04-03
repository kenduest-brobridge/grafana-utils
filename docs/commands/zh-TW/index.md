# 逐指令說明

這個目錄收錄 `grafana-util` 的逐指令與逐子命令頁面。  
如果手冊章節是用來理解工作流，這裡就是拿來查實際指令面、常用旗標與相鄰命令差異的地方。

如果您是從繁體中文手冊進來，建議這樣使用：

| 您要查什麼 | 建議閱讀順序 |
| :--- | :--- |
| 先理解功能目的與操作流程 | 先讀 `docs/user-guide/zh-TW/` 對應章節 |
| 需要查某個 command 或 subcommand 怎麼用 | 直接進這裡的繁中逐指令頁 |
| 想核對目前 Rust CLI help 的精確形狀 | 以這裡的指令頁為主，必要時再對照英文頁 |

如果您偏好 `man` 格式閱讀頂層命令，macOS 可執行 `man ./docs/man/grafana-util.1`，GNU/Linux 可執行 `man -l docs/man/grafana-util.1`。

## 核心領域

| 領域 | 入口 |
| :--- | :--- |
| Dashboard | [dashboard](./dashboard.md) |
| Datasource | [datasource](./datasource.md) |
| Alert | [alert](./alert.md) |
| Access | [access](./access.md) |

常用的 dashboard 子命令：
- [dashboard export](./dashboard-export.md)
- [dashboard raw-to-prompt](./dashboard-raw-to-prompt.md)
- [dashboard import](./dashboard-import.md)
- [dashboard inspect-export](./dashboard-inspect-export.md)

如果您要把一般 raw dashboard JSON 轉成 Grafana UI prompt JSON，請直接看 [dashboard raw-to-prompt](./dashboard-raw-to-prompt.md)。

## 共用介面

| 頁面 | 入口 |
| :--- | :--- |
| Change | [change](./change.md) |
| Overview | [overview](./overview.md) |
| Status | [status](./status.md) |
| Profile | [profile](./profile.md) |
| Snapshot | [snapshot](./snapshot.md) |

## 英文對照

若您需要和英文版逐頁對照，可從這裡進入：

- [英文逐指令總索引](../en/index.md)
