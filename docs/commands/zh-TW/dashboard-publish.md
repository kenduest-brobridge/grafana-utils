# dashboard publish

## 用途
透過既有的儀表板匯入流程發佈一個本地儀表板 JSON 檔。

## 何時使用
當本地草稿已經準備好要上線，且您希望這個指令沿用 CLI 內相同的匯入路徑時，使用這個指令。

## 重點旗標
- `--input`：要發佈的儀表板 JSON 檔。可用 `-` 從標準輸入讀入一份 wrapped 或 bare 的儀表板 JSON。
- `--replace-existing`：當 UID 已存在時更新既有儀表板。
- `--folder-uid`：覆寫目的資料夾 UID。內建的 General folder 會被正規化回預設 root publish 路徑，不會硬送出字面上的 `general` folder UID。
- `--message`：儲存在 Grafana 的修訂訊息。
- `--dry-run`：預覽發佈內容，但不變更 Grafana。
- `--watch`：當本地輸入檔變更時重新執行發佈或 dry-run。只適合本地檔案路徑，不支援 `--input -`。watcher 會回報檔案變更、暫時性失敗與重跑狀態，並持續監看直到你手動停止。
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

```bash
# 用途：從標準輸入發佈一份生成儀表板。
jsonnet dashboards/cpu.jsonnet | grafana-util dashboard publish --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --input - --replace-existing
```

```bash
# 用途：監看一份本地草稿，並在每次儲存後重新執行 dry-run。
grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --dry-run --watch
```

## 相關指令
- [dashboard import](./dashboard-import.md)
- [dashboard review](./dashboard-review.md)
- [dashboard patch-file](./dashboard-patch-file.md)
