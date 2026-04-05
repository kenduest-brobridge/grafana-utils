# dashboard publish

## 用途
透過既有的儀表板匯入流程發佈一個本地儀表板 JSON 檔。

## 何時使用
當本地草稿已經準備好要上線，且您希望這個指令沿用 CLI 內相同的匯入路徑時，使用這個指令。

## 重點旗標
- `--input`：要發佈的儀表板 JSON 檔。
- `--replace-existing`：當 UID 已存在時更新既有儀表板。
- `--folder-uid`：覆寫目的資料夾 UID。
- `--message`：儲存在 Grafana 的修訂訊息。
- `--dry-run`：預覽發佈內容，但不變更 Grafana。
- `--table`、`--json`：dry-run 的輸出模式。

## 範例
```bash
# 用途：透過既有的儀表板匯入流程發佈一個本地儀表板 JSON 檔。
grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --folder-uid infra --message 'Promote CPU dashboard'
```

```bash
# 用途：透過既有的儀表板匯入流程發佈一個本地儀表板 JSON 檔。
grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --dry-run --table
```

## 相關指令
- [dashboard import](./dashboard-import.md)
- [dashboard review](./dashboard-review.md)
- [dashboard patch-file](./dashboard-patch-file.md)
