"""Live dashboard and datasource listing helpers."""

import csv
import json
import sys
from typing import Any, Callable, Optional

from ..clients.dashboard_client import GrafanaClient
from .common import (
    DEFAULT_DASHBOARD_TITLE,
    DEFAULT_FOLDER_TITLE,
    DEFAULT_FOLDER_UID,
    DEFAULT_ORG_ID,
    DEFAULT_ORG_NAME,
    DEFAULT_UNKNOWN_UID,
    GrafanaError,
)
from .transformer import (
    build_datasource_catalog,
    collect_datasource_refs,
    is_builtin_datasource_ref,
    is_placeholder_string,
    lookup_datasource,
    resolve_datasource_ref,
    resolve_datasource_type_alias,
)


def format_dashboard_summary_line(summary: dict[str, Any]) -> str:
    """Render one live dashboard summary in a compact operator-readable form."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 41

    record = build_dashboard_summary_record(summary)
    line = (
        "uid={uid} name={name} folder={folder} folderUid={folderUid} "
        "path={path} org={org} orgId={orgId}"
    ).format(**record)
    if record.get("sources"):
        line += " sources={sources}".format(**record)
    return line


def build_dashboard_summary_record(summary: dict[str, Any]) -> dict[str, str]:
    """Normalize a dashboard summary into a stable output record."""
    folder = str(summary.get("folderTitle") or DEFAULT_FOLDER_TITLE)
    record = {
        "uid": str(summary.get("uid") or DEFAULT_UNKNOWN_UID),
        "name": str(summary.get("title") or DEFAULT_DASHBOARD_TITLE),
        "folder": folder,
        "folderUid": str(summary.get("folderUid") or DEFAULT_FOLDER_UID),
        "path": str(summary.get("folderPath") or folder),
        "org": str(summary.get("orgName") or DEFAULT_ORG_NAME),
        "orgId": str(summary.get("orgId") or DEFAULT_ORG_ID),
    }
    if "sources" in summary:
        record["sources"] = ",".join(summary.get("sources") or [])
    if "sourceUids" in summary:
        record["sourceUids"] = ",".join(summary.get("sourceUids") or [])
    return record


def build_folder_path(folder: dict[str, Any], fallback_title: str) -> str:
    """Build a readable folder tree path from Grafana folder metadata."""
    parents = folder.get("parents")
    titles = []
    if isinstance(parents, list):
        for parent in parents:
            if isinstance(parent, dict):
                title = str(parent.get("title") or "").strip()
                if title:
                    titles.append(title)
    title = (
        str(folder.get("title") or fallback_title or DEFAULT_FOLDER_TITLE).strip()
        or DEFAULT_FOLDER_TITLE
    )
    titles.append(title)
    return " / ".join(titles)


def attach_dashboard_folder_paths(
    client: GrafanaClient,
    summaries: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    """Attach a resolved folder tree path to each dashboard summary when possible."""
    folder_paths = {}
    for summary in summaries:
        folder_uid = str(summary.get("folderUid") or "").strip()
        folder_title = str(summary.get("folderTitle") or DEFAULT_FOLDER_TITLE)
        if not folder_uid or folder_uid in folder_paths:
            continue
        folder = client.fetch_folder_if_exists(folder_uid)
        if folder is None:
            folder_paths[folder_uid] = folder_title
            continue
        folder_paths[folder_uid] = build_folder_path(folder, folder_title)

    enriched = []
    for summary in summaries:
        item = dict(summary)
        folder_uid = str(item.get("folderUid") or "").strip()
        folder_title = str(item.get("folderTitle") or DEFAULT_FOLDER_TITLE)
        item["folderPath"] = folder_paths.get(folder_uid, folder_title)
        enriched.append(item)
    return enriched


def describe_datasource_ref(
    ref: Any,
    datasources_by_uid: dict[str, dict[str, Any]],
    datasources_by_name: dict[str, dict[str, Any]],
) -> Optional[str]:
    """Resolve one datasource reference into a display label when possible."""
    if ref is None or is_builtin_datasource_ref(ref):
        return None

    if isinstance(ref, str):
        if is_placeholder_string(ref):
            return None
        datasource = lookup_datasource(
            datasources_by_uid,
            datasources_by_name,
            uid=ref,
            name=ref,
        )
        if datasource is not None:
            label = datasource.get("name") or ref
            if isinstance(label, str) and label:
                return label
        datasource_type = resolve_datasource_type_alias(ref, datasources_by_uid)
        if datasource_type is not None:
            return datasource_type
        return ref

    if isinstance(ref, dict):
        uid = ref.get("uid")
        name = ref.get("name")
        ds_type = ref.get("type")
        has_placeholder = (
            isinstance(uid, str)
            and is_placeholder_string(uid)
            or isinstance(name, str)
            and is_placeholder_string(name)
        )
        if has_placeholder:
            return None
        datasource = lookup_datasource(
            datasources_by_uid,
            datasources_by_name,
            uid=uid,
            name=name,
        )
        if datasource is not None:
            label = datasource.get("name") or name or uid
            if isinstance(label, str) and label:
                return label
        for candidate in (name, uid, ds_type):
            if isinstance(candidate, str) and candidate:
                return candidate
    return None


def resolve_datasource_uid(
    ref: Any,
    datasources_by_uid: dict[str, dict[str, Any]],
    datasources_by_name: dict[str, dict[str, Any]],
) -> Optional[str]:
    """Resolve one datasource reference into a concrete datasource UID when possible."""
    if ref is None or is_builtin_datasource_ref(ref):
        return None

    if isinstance(ref, str):
        if is_placeholder_string(ref):
            return None
        datasource = lookup_datasource(
            datasources_by_uid,
            datasources_by_name,
            uid=ref,
            name=ref,
        )
        if datasource is None:
            return None
        uid = datasource.get("uid")
        if isinstance(uid, str) and uid:
            return uid
        return None

    if isinstance(ref, dict):
        uid = ref.get("uid")
        name = ref.get("name")
        has_placeholder = (
            isinstance(uid, str)
            and is_placeholder_string(uid)
            or isinstance(name, str)
            and is_placeholder_string(name)
        )
        if has_placeholder:
            return None
        datasource = lookup_datasource(
            datasources_by_uid,
            datasources_by_name,
            uid=uid,
            name=name,
        )
        if datasource is not None:
            resolved_uid = datasource.get("uid")
            if isinstance(resolved_uid, str) and resolved_uid:
                return resolved_uid
        if isinstance(uid, str) and uid:
            return uid
    return None


def resolve_dashboard_source_metadata(
    payload: dict[str, Any],
    extract_dashboard_object: Callable[[dict[str, Any], str], dict[str, Any]],
    datasource_error: type,
    datasources_by_uid: dict[str, dict[str, Any]],
    datasources_by_name: dict[str, dict[str, Any]],
) -> tuple[list[str], list[str]]:
    """Collect sorted datasource display names and concrete UIDs from one dashboard payload."""
    dashboard = extract_dashboard_object(
        payload,
        "Unexpected dashboard payload from Grafana.",
    )
    refs: list[Any] = []
    collect_datasource_refs(dashboard, refs)
    source_names = set()
    source_uids = set()
    for ref in refs:
        try:
            resolved = resolve_datasource_ref(
                ref,
                datasources_by_uid=datasources_by_uid,
                datasources_by_name=datasources_by_name,
            )
        except Exception as exc:
            if isinstance(exc, datasource_error):
                resolved = None
            else:
                raise
        if resolved is not None:
            label = getattr(resolved, "input_label", "")
            if isinstance(label, str) and label:
                source_names.add(label)

        label = describe_datasource_ref(
            ref,
            datasources_by_uid=datasources_by_uid,
            datasources_by_name=datasources_by_name,
        )
        if label:
            source_names.add(label)
        uid = resolve_datasource_uid(
            ref,
            datasources_by_uid=datasources_by_uid,
            datasources_by_name=datasources_by_name,
        )
        if uid:
            source_uids.add(uid)
    return sorted(source_names), sorted(source_uids)


def attach_dashboard_sources(
    client: GrafanaClient,
    summaries: list[dict[str, Any]],
    extract_dashboard_object: Callable[[dict[str, Any], str], dict[str, Any]],
    datasource_error: type,
) -> list[dict[str, Any]]:
    """Attach sorted datasource display names to each dashboard summary."""
    datasources_by_uid, datasources_by_name = build_datasource_catalog(
        client.list_datasources()
    )
    enriched = []
    for summary in summaries:
        item = dict(summary)
        uid = str(item.get("uid") or "").strip()
        if uid:
            payload = client.fetch_dashboard(uid)
            sources, source_uids = resolve_dashboard_source_metadata(
                payload,
                extract_dashboard_object=extract_dashboard_object,
                datasource_error=datasource_error,
                datasources_by_uid=datasources_by_uid,
                datasources_by_name=datasources_by_name,
            )
            item["sources"] = sources
            item["sourceUids"] = source_uids
        else:
            item["sources"] = []
            item["sourceUids"] = []
        enriched.append(item)
    return enriched


def attach_dashboard_org(
    client: GrafanaClient,
    summaries: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    """Attach the current Grafana organization to each dashboard summary."""
    org = client.fetch_current_org()
    org_name = str(org.get("name") or DEFAULT_ORG_NAME)
    org_id = str(org.get("id") or DEFAULT_ORG_ID)
    enriched = []
    for summary in summaries:
        item = dict(summary)
        item["orgName"] = org_name
        item["orgId"] = org_id
        enriched.append(item)
    return enriched


def render_dashboard_summary_table(
    summaries: list[dict[str, Any]],
    include_header: bool = True,
) -> list[str]:
    """Render dashboard summaries as a fixed-width table."""
    headers = ["UID", "NAME", "FOLDER", "FOLDER_UID", "FOLDER_PATH", "ORG", "ORG_ID"]
    if summaries and "sources" in summaries[0]:
        headers.append("SOURCES")
    rows = []
    for record in [build_dashboard_summary_record(summary) for summary in summaries]:
        row = [
            record["uid"],
            record["name"],
            record["folder"],
            record["folderUid"],
            record["path"],
            record["org"],
            record["orgId"],
        ]
        if "sources" in record:
            row.append(record["sources"])
        rows.append(row)
    widths = [len(header) for header in headers]
    for row in rows:
        for index, value in enumerate(row):
            widths[index] = max(widths[index], len(value))

    def format_row(values: list[str]) -> str:
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        return "  ".join(
            value.ljust(widths[index]) for index, value in enumerate(values)
        )

    lines = []
    if include_header:
        lines.extend([format_row(headers), format_row(["-" * width for width in widths])])
    lines.extend(format_row(row) for row in rows)
    return lines


def render_dashboard_summary_csv(summaries: list[dict[str, Any]]) -> None:
    """Render dashboard summaries as CSV records."""
    fieldnames = ["uid", "name", "folder", "folderUid", "path", "org", "orgId"]
    if summaries and "sources" in summaries[0]:
        fieldnames.append("sources")
    if summaries and "sourceUids" in summaries[0]:
        fieldnames.append("sourceUids")
    writer = csv.DictWriter(sys.stdout, fieldnames=fieldnames, lineterminator="\n")
    writer.writeheader()
    for summary in summaries:
        writer.writerow(build_dashboard_summary_record(summary))


def render_dashboard_summary_json(summaries: list[dict[str, Any]]) -> str:
    """Render dashboard summaries as JSON."""
    records: list[dict[str, Any]] = []
    for summary in summaries:
        record: dict[str, Any] = build_dashboard_summary_record(summary)
        if "sources" in summary:
            record["sources"] = list(summary.get("sources") or [])
        if "sourceUids" in summary:
            record["sourceUids"] = list(summary.get("sourceUids") or [])
        records.append(record)
    return json.dumps(records, indent=2, sort_keys=False)


def list_dashboards(
    args: Any,
    build_client: Callable[[Any], GrafanaClient],
    extract_dashboard_object: Callable[[dict[str, Any], str], dict[str, Any]],
    datasource_error: type,
) -> int:
    """List live dashboard summaries without exporting dashboard JSON."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 258, 290, 307, 350, 363, 78

    all_orgs = bool(getattr(args, "all_orgs", False))
    org_id = getattr(args, "org_id", None)
    if all_orgs and org_id:
        raise GrafanaError("Choose either --org-id or --all-orgs, not both.")
    client = build_client(args)
    auth_header = client.headers.get("Authorization", "")
    if (all_orgs or org_id) and not auth_header.startswith("Basic "):
        raise GrafanaError(
            "Dashboard org switching does not support API token auth. Use Grafana "
            "username/password login with --basic-user and --basic-password."
        )

    clients = []
    if all_orgs:
        for org in client.list_orgs():
            scoped_org_id = str(org.get("id") or "").strip()
            if scoped_org_id:
                clients.append(client.with_org_id(scoped_org_id))
    elif org_id:
        clients = [client.with_org_id(str(org_id))]
    else:
        clients = [client]

    summaries = []
    for scoped_client in clients:
        scoped_summaries = attach_dashboard_folder_paths(
            scoped_client,
            scoped_client.iter_dashboard_summaries(args.page_size),
        )
        scoped_summaries = attach_dashboard_org(scoped_client, scoped_summaries)
        if args.json or getattr(args, "with_sources", False):
            scoped_summaries = attach_dashboard_sources(
                scoped_client,
                scoped_summaries,
                extract_dashboard_object=extract_dashboard_object,
                datasource_error=datasource_error,
            )
        summaries.extend(scoped_summaries)
    if args.csv:
        render_dashboard_summary_csv(summaries)
        return 0
    if args.json:
        print(render_dashboard_summary_json(summaries))
        return 0
    for line in render_dashboard_summary_table(
        summaries,
        include_header=not bool(getattr(args, "no_header", False)),
    ):
        print(line)
    print("")
    print("Listed {count} dashboard summaries from {url}".format(
        count=len(summaries),
        url=args.url,
    ))
    return 0


def format_data_source_line(datasource: dict[str, Any]) -> str:
    """Render one datasource in a compact operator-readable form."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 448

    record = build_data_source_record(datasource)
    return (
        "uid={uid} name={name} type={type} url={url} isDefault={isDefault}"
    ).format(**record)


def build_data_source_record(datasource: dict[str, Any]) -> dict[str, str]:
    """Normalize one datasource into a stable output record."""
    record = {
        "uid": str(datasource.get("uid") or ""),
        "name": str(datasource.get("name") or ""),
        "type": str(datasource.get("type") or ""),
        "url": str(datasource.get("url") or ""),
        "isDefault": "true" if bool(datasource.get("isDefault")) else "false",
    }
    org = str(datasource.get("org") or "")
    org_id = str(datasource.get("orgId") or "")
    if org or org_id:
        record["org"] = org
        record["orgId"] = org_id
    return record


def data_source_rows_include_org_scope(datasources: list[dict[str, Any]]) -> bool:
    """Return whether any datasource row includes explicit org metadata."""
    return any(
        bool(str(datasource.get("org") or "").strip())
        or bool(str(datasource.get("orgId") or "").strip())
        for datasource in datasources
    )


def build_datasource_inventory_record(
    datasource: dict[str, Any],
    org: dict[str, Any],
) -> dict[str, str]:
    """Normalize one datasource inventory record for raw export metadata."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 448

    record = build_data_source_record(datasource)
    record["access"] = str(datasource.get("access") or "")
    record["org"] = str(org.get("name") or DEFAULT_ORG_NAME)
    record["orgId"] = str(org.get("id") or DEFAULT_ORG_ID)
    return record


def render_data_source_table(
    datasources: list[dict[str, Any]],
    include_header: bool = True,
) -> list[str]:
    """Render datasource summaries as a fixed-width table."""
    # Call graph: see callers/callees.
    #   Upstream callers: 553
    #   Downstream callees: 448, 465, 512

    include_org_scope = data_source_rows_include_org_scope(datasources)
    headers = ["UID", "NAME", "TYPE", "URL", "IS_DEFAULT"]
    if include_org_scope:
        headers.extend(["ORG", "ORG_ID"])
    rows = []
    for record in [build_data_source_record(item) for item in datasources]:
        row = [
            record["uid"],
            record["name"],
            record["type"],
            record["url"],
            record["isDefault"],
        ]
        if include_org_scope:
            row.extend([record.get("org", ""), record.get("orgId", "")])
        rows.append(row)
    widths = [len(header) for header in headers]
    for row in rows:
        for index, value in enumerate(row):
            widths[index] = max(widths[index], len(value))

    def format_row(values: list[str]) -> str:
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        return "  ".join(
            value.ljust(widths[index]) for index, value in enumerate(values)
        )

    lines = []
    if include_header:
        lines.extend([format_row(headers), format_row(["-" * width for width in widths])])
    lines.extend(format_row(row) for row in rows)
    return lines


def render_data_source_csv(datasources: list[dict[str, Any]]) -> None:
    """Render datasource summaries as CSV records."""
    include_org_scope = data_source_rows_include_org_scope(datasources)
    fieldnames = ["uid", "name", "type", "url", "isDefault"]
    if include_org_scope:
        fieldnames.extend(["org", "orgId"])
    writer = csv.DictWriter(
        sys.stdout,
        fieldnames=fieldnames,
        lineterminator="\n",
    )
    writer.writeheader()
    for datasource in datasources:
        writer.writerow(build_data_source_record(datasource))


def render_data_source_json(datasources: list[dict[str, Any]]) -> str:
    """Render datasource summaries as JSON."""
    return json.dumps(
        [build_data_source_record(item) for item in datasources],
        indent=2,
        sort_keys=False,
    )


def list_data_sources(
    args: Any,
    build_client: Callable[[Any], GrafanaClient],
) -> int:
    """List live Grafana datasource summaries."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 486, 528, 544

    client = build_client(args)
    datasources = client.list_datasources()
    if args.csv:
        render_data_source_csv(datasources)
        return 0
    if args.json:
        print(render_data_source_json(datasources))
        return 0
    for line in render_data_source_table(
        datasources,
        include_header=not bool(getattr(args, "no_header", False)),
    ):
        print(line)
    print("")
    print("Listed {count} data source(s) from {url}".format(
        count=len(datasources),
        url=args.url,
    ))
    return 0
