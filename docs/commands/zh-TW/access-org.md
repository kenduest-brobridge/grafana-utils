# `grafana-util access org`

## 目的

列出、建立、修改、匯出、匯入、比對或刪除 Grafana 組織。

## 使用時機

- 檢視組織清單與 org 使用者。
- 建立新組織或重新命名既有組織。
- 在環境之間匯出或匯入 org 套件。
- 以 id 或精確名稱刪除組織。

## 主要旗標

- `list`: `--org-id`, `--name`, `--query`, `--with-users`, `--table`, `--csv`, `--json`, `--yaml`, `--output-format`
- `add`: `--name`, `--json`
- `modify`: `--org-id`, `--name`, `--set-name`, `--json`
- `export` 與 `diff`: `--org-id`, `--name`, `--export-dir` 或 `--diff-dir`, `--overwrite`, `--dry-run`, `--with-users`
- `import`: `--import-dir`, `--replace-existing`, `--dry-run`, `--yes`
- `delete`: `--org-id`, `--name`, `--yes`, `--json`

## 說明

- 只要 profile 具備必要管理員權限，就可優先用 `--profile` 做可重複的 org inventory。
- org 管理面通常比窄權限 API token 更廣。建立、重新命名、匯出、匯入與刪除流程，較建議使用 Basic auth 或管理員憑證支援的 profile。

## 範例

```bash
grafana-util access org list --profile prod --output-format text
grafana-util access org modify --url http://localhost:3000 --basic-user admin --basic-password admin --name platform --set-name platform-core --json
grafana-util access org list --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --json
grafana-util access org delete --url http://localhost:3000 --basic-user admin --basic-password admin --name platform --yes
```

## 相關命令

- [access](./access.md)
- [access user](./access-user.md)
- [access team](./access-team.md)
