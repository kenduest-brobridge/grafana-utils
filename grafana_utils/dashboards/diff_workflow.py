from pathlib import Path

import sys

from ..batch_error_policy import append_item_failure, should_continue_on_item_error


def run_diff_dashboards(args, deps):
    client = deps["build_client"](args)
    import_dir = Path(args.import_dir)
    deps["load_export_metadata"](import_dir, expected_variant=deps["RAW_EXPORT_SUBDIR"])
    dashboard_files = deps["discover_dashboard_files"](import_dir)
    differences = 0
    failures = []

    for dashboard_file in dashboard_files:
        try:
            document = deps["load_json_file"](dashboard_file)
            uid = deps["resolve_dashboard_uid_for_import"](document)
            local_compare = deps["build_local_compare_document"](
                document,
                args.import_folder_uid,
            )
            remote_payload = client.fetch_dashboard_if_exists(uid)
            if remote_payload is None:
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
                print("Diff same %s -> uid=%s" % (dashboard_file, uid))
                continue

            print("Diff different %s -> uid=%s" % (dashboard_file, uid))
            print(
                "\n".join(
                    deps["build_compare_diff_lines"](
                        remote_compare,
                        local_compare,
                        uid,
                        dashboard_file,
                        args.context_lines,
                    )
                )
            )
            differences += 1
        except Exception as exc:
            if not should_continue_on_item_error(args):
                raise
            failure = append_item_failure(
                failures,
                "dashboard",
                str(dashboard_file.name),
                str(dashboard_file),
                exc,
            )
            print(
                "Continuing after dashboard diff error file=%s: %s"
                % (failure["source"], failure["error"]),
                file=sys.stderr,
            )

    if differences or failures:
        print(
            "Found %s dashboard differences across %s files; failed %s file(s)."
            % (differences, len(dashboard_files), len(failures))
        )
        return 1

    print("No dashboard differences across %s files." % len(dashboard_files))
    return 0
