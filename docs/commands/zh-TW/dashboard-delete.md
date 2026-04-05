# dashboard delete

## 用途
依 UID 或資料夾路徑刪除線上儀表板。

## 何時使用
當您需要移除單一儀表板、某個資料夾子樹，或連同相符資料夾一起刪除子樹時，使用這個指令。

## 重點旗標
- `--uid`：依 UID 刪除單一儀表板。
- `--path`：刪除某個資料夾子樹底下的儀表板。
- `--delete-folders`：搭配 `--path` 時，也一併移除相符的資料夾。
- `--yes`：確認這次線上刪除。
- `--interactive`：以互動方式預覽並確認。
- `--dry-run`、`--table`、`--json`、`--output-format`、`--no-header`：預覽輸出控制。

## 範例
```bash
# 用途：依 UID 或資料夾路徑刪除線上儀表板。
grafana-util dashboard delete --profile prod --uid cpu-main --dry-run --json
```

```bash
# 用途：依 UID 或資料夾路徑刪除線上儀表板。
grafana-util dashboard delete --url http://localhost:3000 --basic-user admin --basic-password admin --path 'Platform / Infra' --yes
```

```bash
# 用途：依 UID 或資料夾路徑刪除線上儀表板。
grafana-util dashboard delete --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --uid cpu-main --dry-run --json
```

## 相關指令
- [dashboard browse](./dashboard-browse.md)
- [dashboard list](./dashboard-list.md)
- [dashboard import](./dashboard-import.md)
