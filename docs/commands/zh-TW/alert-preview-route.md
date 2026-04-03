# `grafana-util alert preview-route`

## 目的

在不改變執行行為的前提下，預覽受管理的路由輸入。

## 使用時機

- 檢查你打算提供給 `set-route` 的 matcher 組合。
- 在寫入受管理路由文件前驗證路由輸入。

## 主要旗標

- `--desired-dir` 指向暫存的 alert 樹。
- `--label` 以 `key=value` 形式加入預覽 matcher。
- `--severity` 加入方便使用的 severity matcher 值。

## 範例

```bash
grafana-util alert preview-route --desired-dir ./alerts/desired --label team=platform --severity critical
```

## 相關命令

- [alert](./alert.md)
- [alert set-route](./alert-set-route.md)
