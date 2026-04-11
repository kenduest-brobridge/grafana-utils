# `grafana-util config`

## Root

用途：打開 repo-local 設定工作流，讓 Grafana 連線方式可以重複使用。

適用時機：當你不想在每個 live 指令上重複輸入連線旗標，而是想把預設值整理成 repo-local 設定。

說明：目前公開的 `config` surface 很小，主要就是承載 `config profile`。這條路徑負責 repo-local 連線預設、secret 處理，以及本機與 CI 可重複執行的認證設定。

主要入口：

- [`config profile`](./profile.md)：新增、驗證、檢視、初始化 repo-local profiles

Examples:

```bash
# 用途：在目前 checkout 中初始化一份 starter config。
grafana-util config profile init --overwrite
```

```bash
# 用途：建立一個可重複使用的 production profile，secret 以 prompt 方式輸入。
grafana-util config profile add prod --url https://grafana.example.com --basic-user admin --prompt-password --store-secret encrypted-file
```

相關指令：`grafana-util status live`、`grafana-util status overview`、`grafana-util workspace preview`。
