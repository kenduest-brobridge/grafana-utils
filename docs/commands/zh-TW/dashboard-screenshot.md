# dashboard screenshot

## 用途
在無頭瀏覽器中開啟一個儀表板，並擷取圖片或 PDF 輸出。

## 何時使用
當您需要可重現的儀表板或面板截圖，特別是用於文件、事件處理紀錄或視覺除錯時，使用這個指令。

## 說明
這一頁對應的是 `dashboard` 指令群組裡的視覺擷取工作流。當純文字匯出不夠時，你可以用它產生可重現的圖片或 PDF 輸出，把當下 Grafana 的畫面、變數狀態與 panel 佈局保留下來。

這個指令特別適合要補 runbook、事件時間線、視覺驗證，或在除錯與變更審查時留下前後對照畫面的維運人員。

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
# 用途：在無頭瀏覽器中開啟一個儀表板，並擷取圖片或 PDF 輸出。
grafana-util dashboard screenshot --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --profile prod --output ./cpu-main.png --full-page --header-title --header-url --header-captured-at
```

```bash
# 用途：在無頭瀏覽器中開啟一個儀表板，並擷取圖片或 PDF 輸出。
grafana-util dashboard screenshot --url https://grafana.example.com --dashboard-uid rYdddlPWk --panel-id 20 --vars-query 'var-datasource=prom-main&var-job=node-exporter&var-node=host01:9100' --basic-user admin --prompt-password --output ./panel.png --header-title 'CPU Busy' --header-text 'Solo panel debug capture'
```

```bash
# 用途：在無頭瀏覽器中開啟一個儀表板，並擷取圖片或 PDF 輸出。
grafana-util dashboard screenshot --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --token "$GRAFANA_API_TOKEN" --output ./cpu-main.png --full-page --header-title --header-url --header-captured-at
```

## 相關指令
- [dashboard inspect-vars](./dashboard-inspect-vars.md)
- [dashboard inspect-live](./dashboard-inspect-live.md)
- [dashboard topology](./dashboard-topology.md)
