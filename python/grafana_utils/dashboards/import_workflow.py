"""Dashboard import workflow orchestration helpers."""

import argparse
import json
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path


class _CachedDashboardImportClient:
    """Per-import cache for repeated dashboard/folder GET lookups."""

    def __init__(self, client):
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        # Call graph: see callers/callees.
        #   Upstream callers: 無
        #   Downstream callees: 無

        self._client = client
        self._dashboard_cache = {}
        self._folder_cache = {}

    def __getattr__(self, name):
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        # Call graph: see callers/callees.
        #   Upstream callers: 無
        #   Downstream callees: 無

        return getattr(self._client, name)

    def fetch_dashboard_if_exists(self, uid):
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        cache_key = str(uid or "")
        if cache_key not in self._dashboard_cache:
            self._dashboard_cache[cache_key] = self._client.fetch_dashboard_if_exists(uid)
        return self._dashboard_cache[cache_key]

    def fetch_dashboard(self, uid):
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        # Call graph: see callers/callees.
        #   Upstream callers: 無
        #   Downstream callees: 29

        payload = self.fetch_dashboard_if_exists(uid)
        if payload is None:
            return self._client.fetch_dashboard(uid)
        return payload

    def fetch_folder_if_exists(self, uid):
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        # Call graph: see callers/callees.
        #   Upstream callers: 無
        #   Downstream callees: 無

        cache_key = str(uid or "")
        if cache_key not in self._folder_cache:
            self._folder_cache[cache_key] = self._client.fetch_folder_if_exists(uid)
        return self._folder_cache[cache_key]

    def create_folder(self, uid, title, parent_uid=None):
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        # Call graph: see callers/callees.
        #   Upstream callers: 無
        #   Downstream callees: 無

        result = self._client.create_folder(uid, title, parent_uid=parent_uid)
        record = {"uid": uid, "title": title}
        if parent_uid:
            record["parentUid"] = parent_uid
        self._folder_cache[str(uid or "")] = record
        return result


def _normalize_org_id(org):
    """Internal helper for normalize org id."""
    if not isinstance(org, dict):
        return None
    value = org.get("id")
    if value is None:
        return None
    text = str(value).strip()
    return text or None


def _validate_export_org_match(args, deps, client, import_dir, metadata):
    """Internal helper for validate export org match."""
    if not bool(getattr(args, "require_matching_export_org", False)):
        return
    source_org_id = deps["resolve_export_org_id"](import_dir, metadata)
    if not source_org_id:
        raise deps["GrafanaError"](
            "Could not determine one source export orgId from %s while "
            "--require-matching-export-org is active."
            % import_dir
        )
    target_org = client.fetch_current_org()
    target_org_id = _normalize_org_id(target_org)
    if not target_org_id:
        raise deps["GrafanaError"](
            "Grafana did not return a usable target org id while "
            "--require-matching-export-org is active."
        )
    if target_org_id != source_org_id:
        raise deps["GrafanaError"](
            "Raw export orgId %s does not match target Grafana org id %s. "
            "Remove --require-matching-export-org to allow cross-org import."
            % (source_org_id, target_org_id)
        )


def _clone_import_args(args, **overrides):
    """Internal helper for clone import args."""
    values = dict(vars(args))
    values.update(overrides)
    return argparse.Namespace(**values)


def _resolve_existing_orgs_by_id(client):
    """Internal helper for resolve existing orgs by id."""
    orgs_by_id = {}
    for item in client.list_orgs():
        org_id = _normalize_org_id(item)
        if org_id:
            orgs_by_id[org_id] = dict(item)
    return orgs_by_id


def _resolve_created_org_id(created_payload):
    """Internal helper for resolve created org id."""
    if not isinstance(created_payload, dict):
        return None
    org_id = created_payload.get("orgId")
    if org_id is None:
        org_id = created_payload.get("id")
    if org_id is None:
        return None
    text = str(org_id).strip()
    return text or None


def _resolve_multi_org_targets(args, deps, client):
    """Internal helper for resolve multi org targets."""
    import_dir = Path(args.import_dir)
    selected_org_ids = set(
        str(item).strip()
        for item in (getattr(args, "only_org_id", None) or [])
        if str(item).strip()
    )
    create_missing_orgs = bool(getattr(args, "create_missing_orgs", False))
    dry_run = bool(getattr(args, "dry_run", False))
    orgs_by_id = _resolve_existing_orgs_by_id(client)
    targets = []
    matched_source_org_ids = set()
    for raw_dir in deps["discover_org_raw_export_dirs"](import_dir):
        metadata = deps["load_export_metadata"](
            raw_dir, expected_variant=deps["RAW_EXPORT_SUBDIR"]
        )
        source_org_id = deps["resolve_export_org_id"](raw_dir, metadata)
        if not source_org_id:
            raise deps["GrafanaError"](
                "Could not determine one source export orgId from %s while "
                "--use-export-org is active."
                % raw_dir
            )
        if selected_org_ids and source_org_id not in selected_org_ids:
            continue
        matched_source_org_ids.add(source_org_id)
        source_org_name = deps["resolve_export_org_name"](raw_dir, metadata)
        dashboard_count = int(metadata.get("dashboardCount") or 0) if metadata else 0
        target_org_id = source_org_id
        created_org = False
        org_action = "exists"
        preview_only = False
        if source_org_id not in orgs_by_id:
            if dry_run:
                preview_only = True
                if create_missing_orgs:
                    org_action = "would-create-org"
                    target_org_id = "<new>"
                else:
                    org_action = "missing-org"
                    target_org_id = ""
            elif not create_missing_orgs:
                raise deps["GrafanaError"](
                    "Export orgId %s was not found in the destination Grafana org list. "
                    "Use --create-missing-orgs to create it from the export metadata."
                    % source_org_id
                )
            elif not source_org_name:
                raise deps["GrafanaError"](
                    "Cannot create missing destination org for export orgId %s because "
                    "the raw export does not contain one stable org name."
                    % source_org_id
                )
            else:
                created_payload = deps["create_organization"](client, source_org_name)
                target_org_id = _resolve_created_org_id(created_payload)
                if not target_org_id:
                    raise deps["GrafanaError"](
                        "Created organization for export orgId %s did not return a usable id."
                        % source_org_id
                    )
                orgs_by_id[target_org_id] = {
                    "id": target_org_id,
                    "name": source_org_name,
                }
                created_org = True
                org_action = "created-org"
        targets.append(
            {
                "raw_dir": raw_dir,
                "source_org_id": source_org_id,
                "source_org_name": source_org_name or "",
                "target_org_id": target_org_id,
                "created_org": created_org,
                "org_action": org_action,
                "preview_only": preview_only,
                "dashboard_count": dashboard_count,
            }
        )
    if selected_org_ids:
        missing_org_ids = sorted(selected_org_ids - matched_source_org_ids)
        if missing_org_ids:
            raise deps["GrafanaError"](
                "Selected export orgIds were not found in %s: %s"
                % (import_dir, ", ".join(missing_org_ids))
            )
    if not targets:
        raise deps["GrafanaError"](
            "No org-scoped raw exports matched %s under %s."
            % (
                "--only-org-id selection"
                if selected_org_ids
                else "the combined multi-org export root",
                import_dir,
            )
        )
    return targets


def _run_import_dashboards_by_export_org(args, deps, client):
    """Internal helper for run import dashboards by export org."""
    # Call graph: see callers/callees.
    #   Upstream callers: 771
    #   Downstream callees: 109, 139, 338, 407

    auth_header = client.headers.get("Authorization", "")
    if not auth_header.startswith("Basic "):
        raise deps["GrafanaError"](
            "Dashboard import with --use-export-org does not support API token auth. "
            "Use Grafana username/password login with --basic-user and --basic-password."
        )
    targets = _resolve_multi_org_targets(args, deps, client)
    if bool(getattr(args, "dry_run", False)) and bool(getattr(args, "json", False)):
        org_entries = []
        import_entries = []
        for target in targets:
            raw_dir = target["raw_dir"]
            target_org_id = target["target_org_id"]
            org_entry = {
                "sourceOrgId": target["source_org_id"],
                "sourceOrgName": target["source_org_name"],
                "orgAction": target["org_action"],
                "targetOrgId": target_org_id or "",
                "dashboardCount": target["dashboard_count"],
                "importDir": str(raw_dir),
            }
            org_entries.append(org_entry)
            import_entry = dict(org_entry)
            import_entry.update(
                {
                    "mode": None,
                    "folders": [],
                    "dashboards": [],
                    "summary": {
                        "importDir": str(raw_dir),
                        "dashboardCount": target["dashboard_count"],
                    },
                }
            )
            if not target["preview_only"]:
                scoped_args = _clone_import_args(
                    args,
                    import_dir=str(raw_dir),
                    org_id=target_org_id,
                    use_export_org=False,
                    only_org_id=None,
                    create_missing_orgs=False,
                    require_matching_export_org=False,
                )
                stream = StringIO()
                with redirect_stdout(stream):
                    _run_import_dashboards_for_single_org(scoped_args, deps)
                import_entry.update(json.loads(stream.getvalue()))
            import_entries.append(import_entry)
        summary = {
            "orgCount": len(org_entries),
            "existingOrgCount": len(
                [item for item in org_entries if item["orgAction"] == "exists"]
            ),
            "missingOrgCount": len(
                [item for item in org_entries if item["orgAction"] == "missing-org"]
            ),
            "wouldCreateOrgCount": len(
                [item for item in org_entries if item["orgAction"] == "would-create-org"]
            ),
            "dashboardCount": sum([item["dashboardCount"] for item in org_entries]),
        }
        print(
            json.dumps(
                {
                    "mode": "routed-import-preview",
                    "orgs": org_entries,
                    "imports": import_entries,
                    "summary": summary,
                },
                indent=2,
                sort_keys=True,
            )
        )
        return 0
    if bool(getattr(args, "dry_run", False)) and bool(getattr(args, "table", False)):
        headers = [
            "SOURCE_ORG_ID",
            "SOURCE_ORG_NAME",
            "ORG_ACTION",
            "TARGET_ORG_ID",
            "DASHBOARD_COUNT",
        ]
        rows = [
            [
                str(item["source_org_id"]),
                item["source_org_name"] or "-",
                item["org_action"],
                item["target_org_id"] or "-",
                str(item["dashboard_count"]),
            ]
            for item in targets
        ]
        widths = [len(item) for item in headers]
        for row in rows:
            for index, value in enumerate(row):
                widths[index] = max(widths[index], len(value))
        def format_row(values):
            # Purpose: implementation note.
            # Args: see function signature.
            # Returns: see implementation.

            return "  ".join(
                [
                    "%-*s" % (widths[index], value)
                    for index, value in enumerate(values)
                ]
            )
        if not bool(getattr(args, "no_header", False)):
            print(format_row(headers))
            print(format_row(["-" * width for width in widths]))
        for row in rows:
            print(format_row(row))
        return 0
    for target in targets:
        raw_dir = target["raw_dir"]
        source_org_id = target["source_org_id"]
        source_org_name = target["source_org_name"]
        target_org_id = target["target_org_id"]
        if bool(getattr(args, "dry_run", False)):
            print(
                "Dry-run export orgId=%s name=%s orgAction=%s targetOrgId=%s dashboards=%s from %s"
                % (
                    source_org_id,
                    source_org_name or "-",
                    target["org_action"],
                    target_org_id or "-",
                    target["dashboard_count"],
                    raw_dir,
                )
            )
            if target["preview_only"]:
                continue
        elif not bool(getattr(args, "table", False)):
            if target["created_org"]:
                print(
                    "Created destination org from export orgId=%s name=%s -> targetOrgId=%s"
                    % (
                        source_org_id,
                        source_org_name or "-",
                        target_org_id,
                    )
                )
            else:
                print(
                    "Importing export orgId=%s name=%s -> targetOrgId=%s from %s"
                    % (
                        source_org_id,
                        source_org_name or "-",
                        target_org_id,
                        raw_dir,
                    )
                )
        scoped_args = _clone_import_args(
            args,
            import_dir=str(raw_dir),
            org_id=target_org_id,
            use_export_org=False,
            only_org_id=None,
            create_missing_orgs=False,
            require_matching_export_org=False,
        )
        _run_import_dashboards_for_single_org(scoped_args, deps)
    return 0


def _run_import_dashboards_for_single_org(args, deps):
    """Import previously exported raw dashboard JSON files through Grafana's API."""
    # Call graph: see callers/callees.
    #   Upstream callers: 239, 771
    #   Downstream callees: 83

    grafana_error = deps["GrafanaError"]
    if getattr(args, "table", False) and not args.dry_run:
        raise grafana_error("--table is only supported with --dry-run for import-dashboard.")
    if getattr(args, "json", False) and not args.dry_run:
        raise grafana_error("--json is only supported with --dry-run for import-dashboard.")
    if getattr(args, "table", False) and getattr(args, "json", False):
        raise grafana_error(
            "--table and --json are mutually exclusive for import-dashboard."
        )
    if getattr(args, "no_header", False) and not getattr(args, "table", False):
        raise grafana_error(
            "--no-header is only supported with --dry-run --table for import-dashboard."
        )
    if (
        getattr(args, "require_matching_folder_path", False)
        and getattr(args, "import_folder_uid", None) is not None
    ):
        raise grafana_error(
            "--require-matching-folder-path cannot be combined with --import-folder-uid."
        )
    client = deps["build_client"](args)
    org_id = getattr(args, "org_id", None)
    auth_header = client.headers.get("Authorization", "")
    if org_id and not auth_header.startswith("Basic "):
        raise grafana_error(
            "Dashboard org switching does not support API token auth. Use Grafana "
            "username/password login with --basic-user and --basic-password."
        )
    if org_id:
        client = client.with_org_id(str(org_id))
    client = _CachedDashboardImportClient(client)
    import_dir = Path(args.import_dir)
    metadata = deps["load_export_metadata"](
        import_dir, expected_variant=deps["RAW_EXPORT_SUBDIR"]
    )
    _validate_export_org_match(args, deps, client, import_dir, metadata)
    dashboard_files = deps["discover_dashboard_files"](import_dir)
    folder_inventory = deps["resolve_folder_inventory_requirements"](
        args, import_dir, metadata
    )
    folder_inventory_lookup = deps["build_folder_inventory_lookup"](folder_inventory)

    dry_run_records = []
    imported_count = 0
    skipped_missing_count = 0
    skipped_folder_mismatch_count = 0
    effective_replace_existing = bool(
        getattr(args, "replace_existing", False)
        or getattr(args, "update_existing_only", False)
    )
    mode = deps["describe_dashboard_import_mode"](
        bool(getattr(args, "replace_existing", False)),
        bool(getattr(args, "update_existing_only", False)),
    )
    json_output = bool(getattr(args, "json", False))
    if not json_output:
        print("Import mode: %s" % mode)
    folder_dry_run_records = []
    if getattr(args, "dry_run", False) and getattr(args, "ensure_folders", False):
        folder_dry_run_records = deps["inspect_folder_inventory"](client, folder_inventory)
        if json_output:
            pass
        elif getattr(args, "table", False):
            for line in deps["render_folder_inventory_dry_run_table"](
                folder_dry_run_records,
                include_header=not bool(getattr(args, "no_header", False)),
            ):
                print(line)
        else:
            for record in folder_dry_run_records:
                print(
                    "Dry-run folder uid=%s dest=%s status=%s reason=%s expected=%s actual=%s"
                    % (
                        record["uid"],
                        record["destination"],
                        record["status"],
                        record["reason"] or "-",
                        record["expected_path"] or "-",
                        record["actual_path"] or "-",
                    )
                )
        if folder_dry_run_records and not json_output:
            missing_folder_count = len(
                [
                    record
                    for record in folder_dry_run_records
                    if record.get("status") == "missing"
                ]
            )
            mismatched_folder_count = len(
                [
                    record
                    for record in folder_dry_run_records
                    if record.get("status") == "mismatch"
                ]
            )
            print(
                "Dry-run checked %s folder(s) from %s; %s missing, %s mismatched"
                % (
                    len(folder_dry_run_records),
                    import_dir
                    / str(
                        (metadata or {}).get("foldersFile")
                        or deps["FOLDER_INVENTORY_FILENAME"]
                    ),
                    missing_folder_count,
                    mismatched_folder_count,
                )
            )
    if (
        getattr(args, "ensure_folders", False)
        and folder_inventory
        and args.import_folder_uid is None
        and not getattr(args, "dry_run", False)
    ):
        created_folders = deps["ensure_folder_inventory"](client, folder_inventory)
        print(
            "Ensured %s folder(s) from %s"
            % (
                created_folders,
                import_dir
                / str(
                    (metadata or {}).get("foldersFile")
                    or deps["FOLDER_INVENTORY_FILENAME"]
                ),
            )
        )
    total_dashboards = len(dashboard_files)
    for index, dashboard_file in enumerate(dashboard_files, 1):
        document = deps["load_json_file"](dashboard_file)
        dashboard = deps["extract_dashboard_object"](
            document, "Dashboard payload must be a JSON object."
        )
        dashboard_uid = str(dashboard.get("uid") or "")
        source_folder_path = deps["resolve_source_dashboard_folder_path"](
            document,
            dashboard_file,
            import_dir,
            folder_inventory_lookup,
        )
        folder_uid_override = deps["determine_import_folder_uid_override"](
            client,
            dashboard_uid,
            args.import_folder_uid,
            preserve_existing_folder=effective_replace_existing,
        )
        payload = deps["build_import_payload"](
            document=document,
            folder_uid_override=folder_uid_override,
            replace_existing=effective_replace_existing,
            message=args.import_message,
        )
        folder_path = deps["resolve_dashboard_import_folder_path"](
            client,
            payload,
            document,
            dashboard_file,
            import_dir,
            folder_inventory_lookup,
        )
        uid = payload["dashboard"].get("uid") or deps["DEFAULT_UNKNOWN_UID"]
        destination_folder_path = deps["resolve_existing_dashboard_folder_path"](
            client,
            str(uid),
        )
        if args.dry_run:
            action = deps["determine_dashboard_import_action"](
                client,
                payload,
                effective_replace_existing,
                update_existing_only=bool(getattr(args, "update_existing_only", False)),
            )
            match_result = deps["build_folder_path_match_result"](
                source_folder_path=source_folder_path,
                destination_folder_path=destination_folder_path,
                destination_exists=bool(destination_folder_path is not None),
                require_matching_folder_path=bool(
                    getattr(args, "require_matching_folder_path", False)
                ),
            )
            action = deps["apply_folder_path_guard_to_action"](action, match_result)
            if getattr(args, "table", False) or json_output:
                dry_run_records.append(
                    deps["build_dashboard_import_dry_run_record"](
                        dashboard_file,
                        str(uid),
                        action,
                        folder_path=folder_path,
                        source_folder_path=match_result.get("source_folder_path"),
                        destination_folder_path=match_result.get("destination_folder_path"),
                        reason=match_result.get("reason"),
                    )
                )
                continue
            deps["print_dashboard_import_progress"](
                args,
                index,
                total_dashboards,
                dashboard_file,
                str(uid),
                action=action,
                folder_path=folder_path,
                dry_run=True,
            )
            continue

        if bool(getattr(args, "update_existing_only", False)) or bool(
            getattr(args, "require_matching_folder_path", False)
        ):
            action = deps["determine_dashboard_import_action"](
                client,
                payload,
                effective_replace_existing,
                update_existing_only=bool(getattr(args, "update_existing_only", False)),
            )
            match_result = deps["build_folder_path_match_result"](
                source_folder_path=source_folder_path,
                destination_folder_path=destination_folder_path,
                destination_exists=bool(destination_folder_path is not None),
                require_matching_folder_path=bool(
                    getattr(args, "require_matching_folder_path", False)
                ),
            )
            action = deps["apply_folder_path_guard_to_action"](action, match_result)
            if action == "would-skip-missing":
                skipped_missing_count += 1
                if getattr(args, "verbose", False):
                    print(
                        "Skipped import uid=%s dest=missing action=skip-missing file=%s"
                        % (uid, dashboard_file)
                    )
                elif getattr(args, "progress", False):
                    print(
                        "Skipping dashboard %s/%s: %s dest=missing action=skip-missing"
                        % (index, total_dashboards, uid)
                    )
                continue
            if action == "would-skip-folder-mismatch":
                skipped_folder_mismatch_count += 1
                if getattr(args, "verbose", False):
                    print(
                        "Skipped import uid=%s dest=exists action=skip-folder-mismatch sourceFolderPath=%s destinationFolderPath=%s file=%s"
                        % (
                            uid,
                            match_result.get("source_folder_path") or "-",
                            match_result.get("destination_folder_path") or "-",
                            dashboard_file,
                        )
                    )
                elif getattr(args, "progress", False):
                    print(
                        "Skipping dashboard %s/%s: %s dest=exists action=skip-folder-mismatch"
                        % (index, total_dashboards, uid)
                    )
                continue

        result = client.import_dashboard(payload)
        status = result.get("status", "unknown")
        uid = result.get("uid") or uid
        imported_count += 1
        deps["print_dashboard_import_progress"](
            args,
            index,
            total_dashboards,
            dashboard_file,
            str(uid),
            status=str(status),
            dry_run=False,
        )

    if args.dry_run:
        if getattr(args, "update_existing_only", False):
            skipped_missing_count = len(
                [
                    record
                    for record in dry_run_records
                    if record.get("action") == "skip-missing"
                ]
            )
        skipped_folder_mismatch_count = len(
            [
                record
                for record in dry_run_records
                if record.get("action") == "skip-folder-mismatch"
            ]
        )
        if json_output:
            print(
                deps["render_dashboard_import_dry_run_json"](
                    mode,
                    folder_dry_run_records,
                    dry_run_records,
                    import_dir,
                    skipped_missing_count,
                    skipped_folder_mismatch_count,
                )
            )
        elif getattr(args, "table", False):
            for line in deps["render_dashboard_import_dry_run_table"](
                dry_run_records,
                include_header=not bool(getattr(args, "no_header", False)),
                selected_columns=getattr(args, "output_columns", None),
            ):
                print(line)
        if json_output:
            pass
        elif (
            getattr(args, "update_existing_only", False)
            and skipped_missing_count
            and skipped_folder_mismatch_count
        ):
            print(
                "Dry-run checked %s dashboard files from %s; would skip %s missing dashboards and %s folder-mismatched dashboards"
                % (
                    len(dashboard_files),
                    import_dir,
                    skipped_missing_count,
                    skipped_folder_mismatch_count,
                )
            )
        elif getattr(args, "update_existing_only", False) and skipped_missing_count:
            print(
                "Dry-run checked %s dashboard files from %s; would skip %s missing dashboards"
                % (len(dashboard_files), import_dir, skipped_missing_count)
            )
        elif skipped_folder_mismatch_count:
            print(
                "Dry-run checked %s dashboard files from %s; would skip %s folder-mismatched dashboards"
                % (len(dashboard_files), import_dir, skipped_folder_mismatch_count)
            )
        else:
            print(f"Dry-run checked {len(dashboard_files)} dashboard files from {import_dir}")
    else:
        if (
            getattr(args, "update_existing_only", False)
            and skipped_missing_count
            and skipped_folder_mismatch_count
        ):
            print(
                "Imported %s dashboard files from %s; skipped %s missing dashboards and %s folder-mismatched dashboards"
                % (
                    imported_count,
                    import_dir,
                    skipped_missing_count,
                    skipped_folder_mismatch_count,
                )
            )
        elif getattr(args, "update_existing_only", False) and skipped_missing_count:
            print(
                "Imported %s dashboard files from %s; skipped %s missing dashboards"
                % (imported_count, import_dir, skipped_missing_count)
            )
        elif skipped_folder_mismatch_count:
            print(
                "Imported %s dashboard files from %s; skipped %s folder-mismatched dashboards"
                % (imported_count, import_dir, skipped_folder_mismatch_count)
            )
        else:
            print(f"Imported {imported_count} dashboard files from {import_dir}")
    return 0


def run_import_dashboards(args, deps):
    """Import previously exported raw dashboard JSON files through Grafana's API."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 239, 407

    client = deps["build_client"](args)
    if bool(getattr(args, "use_export_org", False)):
        return _run_import_dashboards_by_export_org(args, deps, client)
    return _run_import_dashboards_for_single_org(args, deps)
