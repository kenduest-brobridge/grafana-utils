"""
Dashboard diff workflow orchestration and contract normalization.
"""

from pathlib import Path
import json


def _discover_provisioning_dashboard_files(input_dir):
    """Discover dashboard JSON files in a Grafana file-provisioning tree."""
    root = Path(input_dir)
    if root.name == "provisioning" and (root / "dashboards").is_dir():
        root = root / "dashboards"
    return sorted(
        path
        for path in root.rglob("*.json")
        if path.name not in {"export-metadata.json", "index.json", "folders.json", "datasources.json", "permissions.json"}
    )


def run_diff_dashboards(args, deps):
    """Compare exported dashboards against live Grafana and report deltas."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    client = deps["build_client"](args)
    import_dir = Path(args.import_dir)
    if getattr(args, "input_format", "raw") == "provisioning":
        dashboard_files = _discover_provisioning_dashboard_files(import_dir)
    else:
        deps["load_export_metadata"](import_dir, expected_variant=deps["RAW_EXPORT_SUBDIR"])
        dashboard_files = deps["discover_dashboard_files"](import_dir)
    differences = 0
    records = []

    for dashboard_file in dashboard_files:
        document = deps["load_json_file"](dashboard_file)
        uid = deps["resolve_dashboard_uid_for_import"](document)
        local_compare = deps["build_local_compare_document"](
            document,
            args.import_folder_uid,
        )
        remote_payload = client.fetch_dashboard_if_exists(uid)
        if remote_payload is None:
            records.append(
                {
                    "file": str(dashboard_file),
                    "uid": uid,
                    "status": "missing-remote",
                    "diff": [],
                }
            )
            if getattr(args, "output_format", "text") == "text":
                print("Diff missing-remote %s -> uid=%s" % (dashboard_file, uid))
            differences += 1
            continue

        remote_compare = deps["build_remote_compare_document"](
            remote_payload,
            args.import_folder_uid,
        )
        if deps["serialize_compare_document"](local_compare) == deps[
            "serialize_compare_document"
        ](remote_compare):
            records.append(
                {
                    "file": str(dashboard_file),
                    "uid": uid,
                    "status": "same",
                    "diff": [],
                }
            )
            if getattr(args, "output_format", "text") == "text":
                print("Diff same %s -> uid=%s" % (dashboard_file, uid))
            continue

        diff_lines = deps["build_compare_diff_lines"](
            remote_compare,
            local_compare,
            uid,
            dashboard_file,
            args.context_lines,
        )
        records.append(
            {
                "file": str(dashboard_file),
                "uid": uid,
                "status": "different",
                "diff": diff_lines,
            }
        )
        if getattr(args, "output_format", "text") == "text":
            print("Diff different %s -> uid=%s" % (dashboard_file, uid))
            print("\n".join(diff_lines))
        differences += 1

    if getattr(args, "output_format", "text") == "json":
        print(
            json.dumps(
                {
                    "kind": "grafana-utils-dashboard-diff",
                    "inputDir": str(import_dir),
                    "differenceCount": differences,
                    "fileCount": len(dashboard_files),
                    "records": records,
                },
                indent=2,
            )
        )
        return 1 if differences else 0

    if differences:
        print(
            "Found %s dashboard differences across %s files."
            % (differences, len(dashboard_files))
        )
        return 1

    print("No dashboard differences across %s files." % len(dashboard_files))
    return 0
