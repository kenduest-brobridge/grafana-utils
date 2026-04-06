# dashboard patch-file

## 用途
原地修補一個本地儀表板 JSON 檔，或將其寫到新路徑。

## 何時使用
當您要在檢視或發佈之前先在本地重寫儀表板中繼資料，而且不需要連到 Grafana 時，使用這個指令。

## 重點旗標
- `--input`：要修補的儀表板 JSON 檔。可用 `-` 從標準輸入讀入一份 wrapped 或 bare 的儀表板 JSON。
- `--output`：寫到不同路徑，而不是覆蓋輸入檔。當使用 `--input -` 時必須指定。
- `--name`：替換儀表板標題。
- `--uid`：替換儀表板 UID。
- `--folder-uid`：設定保留的資料夾 UID。
- `--message`：在修補後的檔案中存放備註。
- `--tag`：替換儀表板標籤；可重複使用多次。

## 範例
```bash
# 用途：原地修補一個本地儀表板 JSON 檔，或將其寫到新路徑。
grafana-util dashboard patch-file --input ./dashboards/raw/cpu-main.json --name 'CPU Overview' --folder-uid infra --tag prod --tag sre
```

```bash
# 用途：原地修補一個本地儀表板 JSON 檔，或將其寫到新路徑。
grafana-util dashboard patch-file --input ./drafts/cpu-main.json --output ./drafts/cpu-main-patched.json --uid cpu-main --message 'Add folder metadata before publish'
```

```bash
# 用途：把標準輸入中的生成儀表板修補後寫到明確輸出路徑。
jsonnet dashboards/cpu.jsonnet | grafana-util dashboard patch-file --input - --output ./drafts/cpu-main.json --folder-uid infra
```

## 相關指令
- [dashboard review](./dashboard-review.md)
- [dashboard publish](./dashboard-publish.md)
- [dashboard fetch-live](./dashboard-fetch-live.md)
