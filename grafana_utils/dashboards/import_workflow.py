"""Dashboard import workflow orchestration helpers."""

from pathlib import Path


def run_import_dashboards(args, deps):
    """Import previously exported raw dashboard JSON files through Grafana's API."""
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
    client = deps["build_client"](args)
    import_dir = Path(args.import_dir)
    metadata = deps["load_export_metadata"](
        import_dir, expected_variant=deps["RAW_EXPORT_SUBDIR"]
    )
    dashboard_files = deps["discover_dashboard_files"](import_dir)
    folder_inventory = deps["resolve_folder_inventory_requirements"](
        args, import_dir, metadata
    )
    folder_inventory_lookup = deps["build_folder_inventory_lookup"](folder_inventory)

    dry_run_records = []
    imported_count = 0
    skipped_missing_count = 0
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
        if args.dry_run:
            action = deps["determine_dashboard_import_action"](
                client,
                payload,
                effective_replace_existing,
                update_existing_only=bool(getattr(args, "update_existing_only", False)),
            )
            if getattr(args, "table", False) or json_output:
                dry_run_records.append(
                    deps["build_dashboard_import_dry_run_record"](
                        dashboard_file,
                        str(uid),
                        action,
                        folder_path=folder_path,
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

        if bool(getattr(args, "update_existing_only", False)):
            action = deps["determine_dashboard_import_action"](
                client,
                payload,
                effective_replace_existing,
                update_existing_only=True,
            )
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
        if json_output:
            print(
                deps["render_dashboard_import_dry_run_json"](
                    mode,
                    folder_dry_run_records,
                    dry_run_records,
                    import_dir,
                    skipped_missing_count,
                )
            )
        elif getattr(args, "table", False):
            for line in deps["render_dashboard_import_dry_run_table"](
                dry_run_records,
                include_header=not bool(getattr(args, "no_header", False)),
            ):
                print(line)
        if json_output:
            pass
        elif getattr(args, "update_existing_only", False) and skipped_missing_count:
            print(
                "Dry-run checked %s dashboard files from %s; would skip %s missing dashboards"
                % (len(dashboard_files), import_dir, skipped_missing_count)
            )
        else:
            print(f"Dry-run checked {len(dashboard_files)} dashboard files from {import_dir}")
    else:
        if getattr(args, "update_existing_only", False) and skipped_missing_count:
            print(
                "Imported %s dashboard files from %s; skipped %s missing dashboards"
                % (imported_count, import_dir, skipped_missing_count)
            )
        else:
            print(f"Imported {imported_count} dashboard files from {import_dir}")
    return 0
