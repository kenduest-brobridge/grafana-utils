# dashboard edit-live

## 用途
把一份 live dashboard 拉進外部 editor 編修，且預設仍先落成安全的本地草稿。

## 何時使用
當 Grafana 裡已經有最接近的來源 dashboard，你想直接從那份 live payload 開始改，但不希望預設就直接回寫 Grafana 時，使用這個指令。

## 重點旗標
- `--dashboard-uid`：要編修的 live Grafana dashboard UID。
- `--output`：編修後要寫出的本地草稿路徑。未指定時，預設是 `./<uid>.edited.json`。
- `--apply-live`：把編修後的 payload 直接寫回 Grafana，而不是寫成本地草稿。
- `--yes`：搭配 `--apply-live` 必填，因為它會變更 live Grafana。
- `--message`：`--apply-live` 寫回 Grafana 時使用的 revision message。
- `--profile`、`--url`、`--token`、`--basic-user`、`--basic-password`：live Grafana 連線設定。

## 補充說明
- 指令會依序使用 `$VISUAL`、`$EDITOR`，最後回退到 `vi`。
- 沒有帶 `--apply-live` 時，這個指令一定會把結果寫成本地草稿。
- 編修完成後，指令會印出 review 摘要，包含阻擋性驗證問題與建議的下一步。
- `--apply-live` 只有在編修後的草稿維持 `dashboard.id` 為 null、保留原始 dashboard UID、且沒有阻擋性 review 問題時才會繼續。
- 編修後的 payload 仍必須保留 `dashboard.uid`。

## 範例
```bash
# 用途：編修一份 live dashboard，並把結果保留成本地草稿。
grafana-util dashboard edit-live --profile prod --dashboard-uid cpu-main --output ./drafts/cpu-main.edited.json
```

```bash
# 用途：編修一份 live dashboard，並使用預設輸出路徑 ./cpu-main.edited.json。
grafana-util dashboard edit-live --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main
```

```bash
# 用途：編修一份 live dashboard，並明確回寫到 Grafana。
grafana-util dashboard edit-live --profile prod --dashboard-uid cpu-main --apply-live --yes --message 'Hotfix CPU dashboard'
```

## 相關指令
- [dashboard fetch-live](./dashboard-fetch-live.md)
- [dashboard clone-live](./dashboard-clone-live.md)
- [dashboard publish](./dashboard-publish.md)
