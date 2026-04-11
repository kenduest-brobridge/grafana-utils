# 已移除的 root path：`grafana-util overview`

## Root

用途：已移除 project-wide overview root 的遷移說明。

適用時機：當你在對照舊文件或舊腳本，想對應到目前的 `status` surface 時。

說明：目前公開的 overview surface 已移到 `grafana-util status`。top-level `overview` root 已不可直接執行。請改用 `status overview` 或 `status live`。

Canonical replacement：

- `grafana-util overview ...` -> `grafana-util status overview ...`
- `grafana-util overview live ...` -> `grafana-util status overview live ...`

下一步請看：[status](./status.md)
