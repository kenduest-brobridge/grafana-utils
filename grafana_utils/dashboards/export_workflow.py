"""Dashboard export workflow orchestration helpers."""

from pathlib import Path
import sys

from ..batch_error_policy import append_item_failure, should_continue_on_item_error


def run_export_dashboards(args, deps):
    """Export dashboards into raw JSON, prompt JSON, or both variants."""
    grafana_error = deps["GrafanaError"]
    if args.without_dashboard_raw and args.without_dashboard_prompt:
        raise grafana_error(
            "Nothing to export. Remove one of --without-dashboard-raw or --without-dashboard-prompt."
        )

    output_dir = Path(args.export_dir)
    export_raw = not args.without_dashboard_raw
    export_prompt = not args.without_dashboard_prompt
    client = deps["build_client"](args)
    all_orgs = bool(getattr(args, "all_orgs", False))
    org_id = getattr(args, "org_id", None)
    if all_orgs and org_id:
        raise grafana_error("Choose either --org-id or --all-orgs, not both.")
    auth_header = client.headers.get("Authorization", "")
    if (all_orgs or org_id) and not auth_header.startswith("Basic "):
        raise grafana_error(
            "Dashboard org switching does not support API token auth. Use Grafana "
            "username/password login with --basic-user and --basic-password."
        )

    clients = [client]
    if all_orgs:
        clients = []
        for org in client.list_orgs():
            scoped_org_id = str(org.get("id") or "").strip()
            if scoped_org_id:
                clients.append((org, client.with_org_id(scoped_org_id)))
    elif org_id:
        scoped_client = client.with_org_id(str(org_id))
        clients = [(scoped_client.fetch_current_org(), scoped_client)]
    else:
        clients = [(client.fetch_current_org(), client)]

    org_exports = []
    total_dashboards = 0
    for org, scoped_client in clients:
        scoped_output_dir = output_dir
        if all_orgs:
            scoped_output_dir = deps["build_all_orgs_output_dir"](output_dir, org)
        raw_dir, prompt_dir = deps["build_export_variant_dirs"](scoped_output_dir)
        datasource_inventory = [
            deps["build_datasource_inventory_record"](item, org)
            for item in scoped_client.list_datasources()
        ]
        datasource_catalog = None
        if export_prompt:
            datasource_catalog = deps["build_datasource_catalog"](datasource_inventory)

        summaries = deps["attach_dashboard_org"](
            scoped_client,
            scoped_client.iter_dashboard_summaries(args.page_size),
        )
        if not summaries:
            continue
        total_dashboards += len(summaries)
        folder_inventory = deps["collect_folder_inventory"](
            scoped_client, org, summaries
        )
        org_exports.append(
            (
                org,
                scoped_client,
                scoped_output_dir,
                raw_dir,
                prompt_dir,
                datasource_catalog,
                datasource_inventory,
                summaries,
                folder_inventory,
            )
        )

    index_items = []
    processed_dashboards = 0
    exported_dashboards = 0
    folder_inventory = []
    datasource_inventory = []
    failures = []
    for (
        _,
        scoped_client,
        _,
        raw_dir,
        prompt_dir,
        datasource_catalog,
        scoped_datasource_inventory,
        summaries,
        scoped_folder_inventory,
    ) in org_exports:
        folder_inventory.extend(scoped_folder_inventory)
        datasource_inventory.extend(scoped_datasource_inventory)
        for summary in summaries:
            processed_dashboards += 1
            uid = str(summary["uid"])
            try:
                deps["print_dashboard_export_progress_summary"](
                    args,
                    processed_dashboards,
                    total_dashboards,
                    uid,
                    dry_run=bool(args.dry_run),
                )
                payload = scoped_client.fetch_dashboard(uid)
                item = deps["build_dashboard_index_item"](summary, uid)
                if export_raw:
                    raw_document = deps["build_preserved_web_import_document"](payload)
                    raw_path = deps["build_output_path"](raw_dir, summary, args.flat)
                    if args.dry_run:
                        deps["ensure_dashboard_write_target"](
                            raw_path,
                            args.overwrite,
                            create_parents=False,
                        )
                        deps["print_dashboard_export_progress"](
                            args,
                            processed_dashboards,
                            total_dashboards,
                            uid,
                            "raw",
                            raw_path,
                            dry_run=True,
                        )
                    else:
                        deps["write_dashboard"](raw_document, raw_path, args.overwrite)
                        deps["print_dashboard_export_progress"](
                            args,
                            processed_dashboards,
                            total_dashboards,
                            uid,
                            "raw",
                            raw_path,
                            dry_run=False,
                        )
                    item["raw_path"] = str(raw_path)
                if export_prompt:
                    assert datasource_catalog is not None
                    prompt_document = deps["build_external_export_document"](
                        payload, datasource_catalog
                    )
                    prompt_path = deps["build_output_path"](
                        prompt_dir, summary, args.flat
                    )
                    if args.dry_run:
                        deps["ensure_dashboard_write_target"](
                            prompt_path,
                            args.overwrite,
                            create_parents=False,
                        )
                        deps["print_dashboard_export_progress"](
                            args,
                            processed_dashboards,
                            total_dashboards,
                            uid,
                            "prompt",
                            prompt_path,
                            dry_run=True,
                        )
                    else:
                        deps["write_dashboard"](
                            prompt_document, prompt_path, args.overwrite
                        )
                        deps["print_dashboard_export_progress"](
                            args,
                            processed_dashboards,
                            total_dashboards,
                            uid,
                            "prompt",
                            prompt_path,
                            dry_run=False,
                        )
                    item["prompt_path"] = str(prompt_path)
                index_items.append(item)
                exported_dashboards += 1
            except Exception as exc:
                if not should_continue_on_item_error(args):
                    raise
                failure = append_item_failure(
                    failures,
                    "dashboard",
                    uid,
                    str(uid),
                    exc,
                )
                print(
                    "Continuing after dashboard export error uid=%s: %s"
                    % (failure["identity"], failure["error"]),
                    file=sys.stderr,
                )

    if not index_items and not failures:
        print("No dashboards found.", file=deps["sys"].stderr)
        return 0

    raw_index_path = None
    raw_metadata_path = None
    raw_datasources_path = None
    if export_raw:
        raw_variant_dir = (
            output_dir / deps["RAW_EXPORT_SUBDIR"] if all_orgs else raw_dir
        )
        raw_index_path = raw_variant_dir / "index.json"
        raw_metadata_path = raw_variant_dir / deps["EXPORT_METADATA_FILENAME"]
        raw_folders_path = raw_variant_dir / deps["FOLDER_INVENTORY_FILENAME"]
        raw_datasources_path = raw_variant_dir / deps["DATASOURCE_INVENTORY_FILENAME"]
        raw_index = deps["build_variant_index"](
            index_items,
            "raw_path",
            "grafana-web-import-preserve-uid",
        )
        raw_metadata = deps["build_export_metadata"](
            variant=deps["RAW_EXPORT_SUBDIR"],
            dashboard_count=len(raw_index),
            format_name="grafana-web-import-preserve-uid",
            folders_file=deps["FOLDER_INVENTORY_FILENAME"],
            datasources_file=deps["DATASOURCE_INVENTORY_FILENAME"],
        )
        if not args.dry_run:
            deps["write_json_document"](raw_index, raw_index_path)
            deps["write_json_document"](raw_metadata, raw_metadata_path)
            deps["write_json_document"](folder_inventory, raw_folders_path)
            deps["write_json_document"](datasource_inventory, raw_datasources_path)
    prompt_index_path = None
    prompt_metadata_path = None
    if export_prompt:
        prompt_variant_dir = (
            output_dir / deps["PROMPT_EXPORT_SUBDIR"] if all_orgs else prompt_dir
        )
        prompt_index_path = prompt_variant_dir / "index.json"
        prompt_metadata_path = prompt_variant_dir / deps["EXPORT_METADATA_FILENAME"]
        prompt_index = deps["build_variant_index"](
            index_items,
            "prompt_path",
            "grafana-web-import-with-datasource-inputs",
        )
        prompt_metadata = deps["build_export_metadata"](
            variant=deps["PROMPT_EXPORT_SUBDIR"],
            dashboard_count=len(prompt_index),
            format_name="grafana-web-import-with-datasource-inputs",
        )
        if not args.dry_run:
            deps["write_json_document"](prompt_index, prompt_index_path)
            deps["write_json_document"](prompt_metadata, prompt_metadata_path)
    index_path = output_dir / "index.json"
    root_index = deps["build_root_export_index"](
        index_items, raw_index_path, prompt_index_path
    )
    root_metadata_path = output_dir / deps["EXPORT_METADATA_FILENAME"]
    root_metadata = deps["build_export_metadata"](
        variant="root",
        dashboard_count=len(index_items),
    )
    if not args.dry_run:
        deps["write_json_document"](root_index, index_path)
        deps["write_json_document"](root_metadata, root_metadata_path)
    summary_verb = "Would export" if args.dry_run else "Exported"
    summary_parts = [f"{summary_verb} {exported_dashboards} dashboards."]
    if raw_index_path is not None:
        summary_parts.append(f"Raw index: {raw_index_path}")
    if raw_metadata_path is not None:
        summary_parts.append(f"Raw manifest: {raw_metadata_path}")
    if raw_datasources_path is not None:
        summary_parts.append(f"Raw datasources: {raw_datasources_path}")
    if prompt_index_path is not None:
        summary_parts.append(f"Prompt index: {prompt_index_path}")
    if prompt_metadata_path is not None:
        summary_parts.append(f"Prompt manifest: {prompt_metadata_path}")
    summary_parts.append(f"Root index: {index_path}")
    summary_parts.append(f"Root manifest: {root_metadata_path}")
    if failures:
        summary_parts.append("Failed: %s" % len(failures))
    print(" ".join(summary_parts))
    return 1 if failures else 0
