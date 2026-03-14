"""Dashboard import workflow orchestration helpers."""

from pathlib import Path


def _normalize_org_id(org):
    if not isinstance(org, dict):
        return None
    value = org.get("id")
    if value is None:
        return None
    text = str(value).strip()
    return text or None


def _validate_export_org_match(args, deps, client, import_dir, metadata):
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
            "Dashboard org switching requires Basic auth. Use --basic-user and --basic-password."
        )
    if org_id:
        client = client.with_org_id(str(org_id))
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
