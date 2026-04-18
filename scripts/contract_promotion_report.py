#!/usr/bin/env python3
"""Report overlap and gaps between schema manifests and runtime output contracts."""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Optional, Union


REPO_ROOT = Path(__file__).resolve().parents[1]
MANIFESTS_DIR = REPO_ROOT / "schemas" / "manifests"
RUNTIME_REGISTRY_PATH = REPO_ROOT / "scripts" / "contracts" / "output-contracts.json"


@dataclass(frozen=True)
class ManifestContract:
    family: str
    contract_id: str
    golden_sample: str
    title: str


@dataclass(frozen=True)
class RuntimeContract:
    family: str
    contract_id: str
    fixture: str
    kind: Optional[str]


@dataclass(frozen=True)
class FamilyReport:
    family: str
    contract_count: int
    shared: tuple[str, ...]
    left_only: tuple[str, ...]
    right_families: tuple[str, ...]


@dataclass(frozen=True)
class PromotionReport:
    manifest_families: tuple[FamilyReport, ...]
    runtime_families: tuple[FamilyReport, ...]
    shared_contract_ids: tuple[str, ...]
    manifest_only_contract_ids: tuple[str, ...]
    runtime_only_contract_ids: tuple[str, ...]


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def load_manifest_contracts(manifests_dir: Path = MANIFESTS_DIR) -> list[ManifestContract]:
    contracts: list[ManifestContract] = []
    seen_contract_ids: set[str] = set()
    for family_dir in sorted(path for path in manifests_dir.iterdir() if path.is_dir()):
        contract_path = family_dir / "contracts.json"
        if not contract_path.is_file():
            continue
        payload = load_json(contract_path)
        if not isinstance(payload, list):
            raise TypeError(f"{contract_path} must contain a list")
        for entry in payload:
            if not isinstance(entry, dict):
                raise TypeError(f"{contract_path} contains a non-object contract entry")
            contract_id = str(entry.get("contractId") or "")
            golden_sample = str(entry.get("goldenSample") or "")
            title = str(entry.get("title") or "")
            if not contract_id:
                raise TypeError(f"{contract_path} contains a contract without contractId")
            if not golden_sample:
                raise TypeError(f"{contract_path} contains contract {contract_id!r} without goldenSample")
            if not title:
                raise TypeError(f"{contract_path} contains contract {contract_id!r} without title")
            if contract_id in seen_contract_ids:
                raise TypeError(f"{contract_path} duplicates contractId {contract_id!r}")
            seen_contract_ids.add(contract_id)
            contracts.append(
                ManifestContract(
                    family=family_dir.name,
                    contract_id=contract_id,
                    golden_sample=golden_sample,
                    title=title,
                )
            )
    if not contracts:
        raise SystemExit("no schema manifests were found")
    return contracts


def runtime_family(name: str) -> str:
    return name.split("-", 1)[0] if name else ""


def load_runtime_contracts(registry_path: Path = RUNTIME_REGISTRY_PATH) -> list[RuntimeContract]:
    payload = load_json(registry_path)
    contracts = payload.get("contracts")
    if not isinstance(contracts, list):
        raise TypeError(f"{registry_path} contracts must be a list")
    records: list[RuntimeContract] = []
    seen_contract_ids: set[str] = set()
    for entry in contracts:
        if not isinstance(entry, dict):
            raise TypeError(f"{registry_path} contains a non-object contract entry")
        name = str(entry.get("name") or "")
        fixture = str(entry.get("fixture") or "")
        if not name:
            raise TypeError(f"{registry_path} contains a contract without name")
        if not fixture:
            raise TypeError(f"{registry_path} contains contract {name!r} without fixture")
        if name in seen_contract_ids:
            raise TypeError(f"{registry_path} duplicates contract name {name!r}")
        seen_contract_ids.add(name)
        records.append(
            RuntimeContract(
                family=runtime_family(name),
                contract_id=name,
                fixture=fixture,
                kind=str(entry.get("kind") or "") or None,
            )
        )
    if not records:
        raise SystemExit("no runtime output contracts were found")
    return records


Record = Union[ManifestContract, RuntimeContract]


def _group_by_family(records: list[Record]) -> dict[str, list[Any]]:
    grouped: dict[str, list[Any]] = {}
    for record in records:
        grouped.setdefault(record.family, []).append(record)
    return grouped


def build_promotion_report(
    manifest_contracts: list[ManifestContract],
    runtime_contracts: list[RuntimeContract],
) -> PromotionReport:
    manifest_by_id: dict[str, ManifestContract] = {}
    runtime_by_id: dict[str, RuntimeContract] = {}

    for record in manifest_contracts:
        if record.contract_id:
            manifest_by_id[record.contract_id] = record
    for record in runtime_contracts:
        if record.contract_id:
            runtime_by_id[record.contract_id] = record

    shared_ids = tuple(sorted(set(manifest_by_id) & set(runtime_by_id)))
    manifest_only_ids = tuple(sorted(set(manifest_by_id) - set(runtime_by_id)))
    runtime_only_ids = tuple(sorted(set(runtime_by_id) - set(manifest_by_id)))

    manifest_family_groups = _group_by_family(manifest_contracts)
    runtime_family_groups = _group_by_family(runtime_contracts)

    shared_by_manifest_family: dict[str, list[str]] = {}
    runtime_families_by_manifest_family: dict[str, set[str]] = {}
    shared_by_runtime_family: dict[str, list[str]] = {}
    manifest_families_by_runtime_family: dict[str, set[str]] = {}

    for contract_id in shared_ids:
        manifest_record = manifest_by_id[contract_id]
        runtime_record = runtime_by_id[contract_id]
        shared_by_manifest_family.setdefault(manifest_record.family, []).append(contract_id)
        shared_by_runtime_family.setdefault(runtime_record.family, []).append(contract_id)
        runtime_families_by_manifest_family.setdefault(manifest_record.family, set()).add(
            runtime_record.family
        )
        manifest_families_by_runtime_family.setdefault(runtime_record.family, set()).add(
            manifest_record.family
        )

    manifest_reports: list[FamilyReport] = []
    for family in sorted(manifest_family_groups):
        contract_ids = sorted(record.contract_id for record in manifest_family_groups[family])
        shared = tuple(sorted(shared_by_manifest_family.get(family, [])))
        left_only = tuple(contract_id for contract_id in contract_ids if contract_id not in shared)
        manifest_reports.append(
            FamilyReport(
                family=family,
                contract_count=len(contract_ids),
                shared=shared,
                left_only=left_only,
                right_families=tuple(sorted(runtime_families_by_manifest_family.get(family, set()))),
            )
        )

    runtime_reports: list[FamilyReport] = []
    for family in sorted(runtime_family_groups):
        contract_ids = sorted(record.contract_id for record in runtime_family_groups[family])
        shared = tuple(sorted(shared_by_runtime_family.get(family, [])))
        left_only = tuple(contract_id for contract_id in contract_ids if contract_id not in shared)
        runtime_reports.append(
            FamilyReport(
                family=family,
                contract_count=len(contract_ids),
                shared=shared,
                left_only=left_only,
                right_families=tuple(sorted(manifest_families_by_runtime_family.get(family, set()))),
            )
        )

    return PromotionReport(
        manifest_families=tuple(manifest_reports),
        runtime_families=tuple(runtime_reports),
        shared_contract_ids=shared_ids,
        manifest_only_contract_ids=manifest_only_ids,
        runtime_only_contract_ids=runtime_only_ids,
    )


def render_text_report(
    report: PromotionReport,
    manifest_contracts: list[ManifestContract],
    runtime_contracts: list[RuntimeContract],
    verbose: bool = False,
) -> str:
    manifest_by_id = {record.contract_id: record for record in manifest_contracts}
    runtime_by_id = {record.contract_id: record for record in runtime_contracts}

    lines: list[str] = [
        "contract promotion report",
        f"  manifest families: {len(report.manifest_families)}",
        f"  runtime families: {len(report.runtime_families)}",
        f"  shared contract ids: {len(report.shared_contract_ids)}",
        f"  manifest-only contract ids: {len(report.manifest_only_contract_ids)}",
        f"  runtime-only contract ids: {len(report.runtime_only_contract_ids)}",
        "",
        "manifest families:",
    ]

    for family_report in report.manifest_families:
        overlap = ", ".join(family_report.right_families) if family_report.right_families else "-"
        lines.append(
            f"  {family_report.family}: total={family_report.contract_count} shared={len(family_report.shared)} "
            f"only={len(family_report.left_only)} runtime={overlap}"
        )
        if verbose:
            if family_report.shared:
                lines.append("    shared:")
                for contract_id in family_report.shared:
                    lines.append(
                        f"      - {contract_id} ({manifest_by_id[contract_id].golden_sample})"
                    )
            if family_report.left_only:
                lines.append("    manifest-only:")
                for contract_id in family_report.left_only:
                    lines.append(
                        f"      - {contract_id} ({manifest_by_id[contract_id].golden_sample})"
                    )

    lines.extend(["", "runtime families:"])
    for family_report in report.runtime_families:
        overlap = ", ".join(family_report.right_families) if family_report.right_families else "-"
        lines.append(
            f"  {family_report.family}: total={family_report.contract_count} shared={len(family_report.shared)} "
            f"only={len(family_report.left_only)} manifest={overlap}"
        )
        if verbose:
            if family_report.shared:
                lines.append("    shared:")
                for contract_id in family_report.shared:
                    lines.append(f"      - {contract_id} ({runtime_by_id[contract_id].fixture})")
            if family_report.left_only:
                lines.append("    runtime-only:")
                for contract_id in family_report.left_only:
                    lines.append(f"      - {contract_id} ({runtime_by_id[contract_id].fixture})")

    return "\n".join(lines)


def main(argv: Optional[list[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        description="Report overlap and gaps between schema manifests and runtime output contracts."
    )
    parser.add_argument(
        "--manifests-dir",
        default=str(MANIFESTS_DIR),
        help="Path to schemas/manifests.",
    )
    parser.add_argument(
        "--registry",
        default=str(RUNTIME_REGISTRY_PATH),
        help="Path to scripts/contracts/output-contracts.json.",
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="Print contract-level references under each family.",
    )
    args = parser.parse_args(argv)

    manifests_dir = Path(args.manifests_dir)
    registry_path = Path(args.registry)

    manifest_contracts = load_manifest_contracts(manifests_dir)
    runtime_contracts = load_runtime_contracts(registry_path)
    report = build_promotion_report(manifest_contracts, runtime_contracts)
    print(render_text_report(report, manifest_contracts, runtime_contracts, verbose=args.verbose))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
