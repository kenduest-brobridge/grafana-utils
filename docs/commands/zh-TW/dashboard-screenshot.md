# dashboard screenshot

## 用途
在無頭瀏覽器中開啟一個儀表板，並擷取圖片或 PDF 輸出。

## 何時使用
當您需要可重現的儀表板或面板截圖，特別是用於文件、事故紀錄或視覺除錯時，使用這個指令。

## 重點旗標
- `--dashboard-uid` 或 `--dashboard-url`：選擇要擷取的儀表板。
- `--output`：截圖輸出目的檔。
- `--panel-id`：透過 solo 路由只擷取單一面板。
- `--vars-query` 與 `--var`：把變數狀態帶入擷取。
- `--full-page` 與 `--full-page-output`：擷取整個可捲動頁面或平鋪輸出。
- `--header-title`、`--header-url`、`--header-captured-at`、`--header-text`：為 PNG 或 JPEG 加上標頭。
- `--theme`：選擇瀏覽器主題。
- `--output-format`：強制輸出 PNG、JPEG 或 PDF。
- `--width`、`--height`、`--device-scale-factor`、`--wait-ms`、`--browser-path`：渲染控制。

## 範例
```bash
grafana-util dashboard screenshot --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --profile prod --output ./cpu-main.png --full-page --header-title --header-url --header-captured-at
grafana-util dashboard screenshot --url https://grafana.example.com --dashboard-uid rYdddlPWk --panel-id 20 --vars-query 'var-datasource=prom-main&var-job=node-exporter&var-node=host01:9100' --basic-user admin --prompt-password --output ./panel.png --header-title 'CPU Busy' --header-text 'Solo panel debug capture'
grafana-util dashboard screenshot --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --token "$GRAFANA_API_TOKEN" --output ./cpu-main.png --full-page --header-title --header-url --header-captured-at
```

## 相關指令
- [dashboard inspect-vars](./dashboard-inspect-vars.md)
- [dashboard inspect-live](./dashboard-inspect-live.md)
- [dashboard topology](./dashboard-topology.md)
