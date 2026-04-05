# `grafana-util alert delete`

## 目的

刪除一個明確指定的 alert 資源識別。

## 使用時機

- 依識別刪除單一規則、聯絡點、靜音時段、政策樹或範本。
- 只有在確定要這麼做時，才重設由工具管理的通知政策樹。

## 主要旗標

- `--kind` 選擇要刪除的資源種類。
- `--identity` 提供明確的資源識別。
- `--allow-policy-reset` 允許重設政策樹。
- `--output-format` 可將刪除預覽或執行結果呈現為 `text` 或 `json`。

## 採用前後對照

- 之前：常常得進 Grafana UI 裡慢慢找要刪哪個 alert 資源。
- 之後：用 kind + identity 直接刪掉你點名的那一筆資源。

## 成功判準

- 只有你指定的 kind / identity 會受到影響。
- 預覽或執行結果會清楚顯示你要移除的資源。
- 如果要重設政策樹，只有在加上 `--allow-policy-reset` 時才會發生。

## 失敗時先檢查

- 先確認 kind 和 identity 是否真的對應到你要刪的資源。
- 如果不在預期的 org 或 profile，先切對上下文再刪。
- 涉及 policy tree 時，確認你真的要允許 reset，再加 `--allow-policy-reset`。

## 範例

```bash
# 用途：刪除一個明確指定的 alert 資源識別。
grafana-util alert delete --profile prod --kind rule --identity cpu-main
```

```bash
# 用途：刪除一個明確指定的 alert 資源識別。
grafana-util alert delete --url http://localhost:3000 --basic-user admin --basic-password admin --kind policy-tree --identity default --allow-policy-reset
```

```bash
# 用途：刪除一個明確指定的 alert 資源識別。
grafana-util alert delete --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --kind rule --identity cpu-main
```

## 相關命令

- [alert](./alert.md)
- [alert plan](./alert-plan.md)
- [alert apply](./alert-apply.md)
