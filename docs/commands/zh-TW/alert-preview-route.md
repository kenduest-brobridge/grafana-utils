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

## 採用前後對照

- 之前：只能猜想 matcher 送進 `set-route` 後會變成什麼樣子。
- 之後：先預覽輸入，確認 receiver 與 matcher 長什麼樣，再決定要不要寫檔。

## 成功判準

- 預覽輸出和你準備交給 `set-route` 的 matcher 集合一致。
- 不用先改 staged tree，也能看出路由的最終樣子。

## 失敗時先檢查

- 如果預覽跟預期不同，先檢查 labels 與 severity 值。
- 確認 `--desired-dir` 指到的是你要預覽的暫存樹。

## 範例

```bash
# 用途：在不改變執行行為的前提下，預覽受管理的路由輸入。
grafana-util alert preview-route --desired-dir ./alerts/desired --label team=platform --severity critical
```

## 相關命令

- [alert](./alert.md)
- [alert set-route](./alert-set-route.md)
