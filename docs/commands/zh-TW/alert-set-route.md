# `grafana-util alert set-route`

## 目的

撰寫或替換由工具擁有的暫存通知路由。

## 使用時機

- 以新的接收器與 matcher 集合取代受管理的路由。
- 重新執行此命令可完整替換受管理路由，而不是合併欄位。

## 主要旗標

- `--desired-dir` 指向暫存的 alert 樹。
- `--receiver` 設定路由接收器。
- `--label` 以 `key=value` 形式加入路由 matcher。
- `--severity` 加入方便使用的 severity matcher。
- `--dry-run` 只會渲染受管理路由文件，不會寫入檔案。

## 範例

```bash
grafana-util alert set-route --desired-dir ./alerts/desired --receiver pagerduty-primary --label team=platform --severity critical
grafana-util alert set-route --desired-dir ./alerts/desired --receiver pagerduty-primary --label team=platform --severity critical --dry-run
```

## 相關命令

- [alert](./alert.md)
- [alert preview-route](./alert-preview-route.md)
- [alert add-contact-point](./alert-add-contact-point.md)
