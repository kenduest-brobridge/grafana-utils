# datasource export

## 用途
將線上 Grafana datasource inventory 匯出成標準化 JSON 與 provisioning 檔案。

## 何時使用
當您需要一個本地 dataworkspace package，供後續檢查、比對或匯入時，使用這個指令。

## 重點旗標
- `--output-dir`：匯出樹的目標目錄。
- `--org-id`：匯出指定的 Grafana org。
- `--all-orgs`：把每個可見 org 匯出到各自的子目錄。需要 Basic auth。
- `--overwrite`：取代既有檔案。
- `--without-datasource-provisioning`：略過 provisioning 變體。
- `--dry-run`：預覽會寫出哪些內容。

## 範例
```bash
# 用途：將線上 Grafana datasource inventory 匯出成標準化 JSON 與 provisioning 檔案。
grafana-util datasource export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./datasources --overwrite
```

```bash
# 用途：將線上 Grafana datasource inventory 匯出成標準化 JSON 與 provisioning 檔案。
grafana-util datasource export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --output-dir ./datasources --overwrite
```

## 採用前後對照

- **採用前**：live datasource 狀態很容易散掉，因為匯出後的結構不夠標準，也不容易再利用。
- **採用後**：一個匯出就能得到本地 bundle，後續檢視、比對或匯入都能直接沿用。

## 成功判準

- 匯出樹完整到可以日後不連 Grafana 也能檢查
- 標準化 JSON 與 provisioning 檔案都能和來源 inventory 對得上
- 這個 bundle 可以直接拿去做 diff 或 import，不需要再手動清理

## 失敗時先檢查

- 如果匯出樹少了 org 資料，先確認 org 範圍與驗證資訊是否真的看得到它
- 如果 `--all-orgs` 失敗，先改用 Basic auth，並確認帳號是否能看見每個目標 org
- 如果 bundle 看起來像舊資料，先確認匯出目錄與 `--overwrite` 是否有刻意使用
- 如果你原本期待匯出裡直接有 secret 明文，先記得 Grafana live datasource API 不會回這些值；export 只會保留 config 與 `secureJsonDataPlaceholders`

## 相關指令
- [datasource list](./datasource-list.md)
- [datasource import](./datasource-import.md)
- [datasource diff](./datasource-diff.md)
