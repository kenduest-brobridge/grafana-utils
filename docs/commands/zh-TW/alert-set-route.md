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

## 採用前後對照

- 之前：手工改 route tree，很容易把 matcher 結構改散。
- 之後：直接寫出一份暫存中的受管理路由文件，接收器與 matcher 都很清楚。

## 成功判準

- 暫存樹裡出現你要的接收器與 matcher 值。
- dry-run 看到的路由文件跟你預期的一樣。
- 這份輸出可以直接拿去跟 `preview-route` 的結果比對。

## 失敗時先檢查

- `--desired-dir` 先確認是不是指到正確的暫存樹。
- 先看 receiver 與 labels 是否真的符合你要的路由。
- 如果 dry-run 的內容跟預期不同，先修 matcher 再寫檔。

## 範例

```bash
# 用途：撰寫或替換由工具擁有的暫存通知路由。
grafana-util alert set-route --desired-dir ./alerts/desired --receiver pagerduty-primary --label team=platform --severity critical
```

```bash
# 用途：撰寫或替換由工具擁有的暫存通知路由。
grafana-util alert set-route --desired-dir ./alerts/desired --receiver pagerduty-primary --label team=platform --severity critical --dry-run
```

## 相關命令

- [alert](./alert.md)
- [alert preview-route](./alert-preview-route.md)
- [alert add-contact-point](./alert-add-contact-point.md)
