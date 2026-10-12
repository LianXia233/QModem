#!/usr/bin/env python3
"""Modem registry consistency gate.

Cross-checks three registration sources so that a newly added modem is
registered everywhere and upstream syncs are not dropped:

  1. application/qmodem/files/usr/share/qmodem/modem_support.json  (source of truth)
  2. docs/support_list.md                                          (human-readable table)
  3. vendor/dynamic_load.json + vendor/<mfr>.sh                    (vendor function map)
  4. testcases/recognition/<mfr>/<platform>/<model>-*/             (recognition fixtures)

Report matrix:
  - in json but not in md
  - in md but not in json
  - vendor function missing (mfr not in dynamic_load.json / script file missing /
    required shell function not defined)
  - recognition fixture missing (informational unless --fail-on-fixtures)

Exit codes: 0 = consistent (or --baseline), 1 = inconsistency found (strict).

Python standard library only. LF line endings.
"""

import argparse
import json
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
JSON_PATH = REPO_ROOT / "application/qmodem/files/usr/share/qmodem/modem_support.json"
MD_PATH = REPO_ROOT / "docs/support_list.md"
VENDOR_DIR = REPO_ROOT / "application/qmodem/files/usr/share/qmodem/vendor"
DYNAMIC_LOAD = VENDOR_DIR / "dynamic_load.json"
RECOGNITION_DIR = REPO_ROOT / "testcases/recognition"

# Functions the rpcd framework invokes after load_vendor_script() sources the
# vendor script (see application/qmodem/files/usr/libexec/rpcd/qmodem).
REQUIRED_VENDOR_FUNCS = (
    "get_imei",
    "get_mode",
    "network_info",
    "sim_info",
    "base_info",
)

# Required fields for usb/pcie modem entries in modem_support.json.
REQUIRED_ENTRY_FIELDS = (
    "manufacturer",
    "platform",
    "data_interface",
    "pdp_index",
    "modes",
)


def load_json_entries():
    """Return dict[(mfr_lower, model_lower)] -> {model, mfr, platform, bus} plus schema errors."""
    data = json.loads(JSON_PATH.read_text(encoding="utf-8"))
    modem_support = data["modem_support"]
    entries = {}
    schema_errors = []
    seen_per_bus = {}
    for bus in ("usb", "pcie"):
        for model, info in modem_support.get(bus, {}).items():
            missing = [f for f in REQUIRED_ENTRY_FIELDS if f not in info]
            if missing:
                schema_errors.append(f"{bus}/{model}: missing required fields: {', '.join(missing)}")
                continue
            if not isinstance(info.get("modes"), list) or not info["modes"]:
                schema_errors.append(f"{bus}/{model}: 'modes' must be a non-empty list")
                continue
            mfr = info["manufacturer"].lower()
            key = (mfr, model.lower())
            # Same model under usb and pcie is legitimate (dual-bus support);
            # only a true duplicate within one bus is a schema error.
            if (bus, key) in seen_per_bus:
                schema_errors.append(f"{bus}/{model}: duplicate (manufacturer, model) entry")
                continue
            seen_per_bus[(bus, key)] = True
            entries.setdefault(key, {
                "model": model,
                "mfr": mfr,
                "platform": info["platform"].lower(),
                "bus": bus,
            })
    # device bus: platform -> {vendor_id, product_id} scan table
    for mfr, platforms in modem_support.get("device", {}).items():
        if not isinstance(platforms, dict) or not platforms:
            schema_errors.append(f"device/{mfr}: expected platform->ids mapping")
            continue
        for plat, ids in platforms.items():
            if not isinstance(ids, dict) or not ids.get("vendor_id") or not ids.get("product_id"):
                schema_errors.append(f"device/{mfr}/{plat}: missing vendor_id/product_id list")
    return entries, schema_errors


def parse_support_list():
    """Return dict[(mfr_lower, model_lower)] -> source line info."""
    found = {}
    current_mfr = None
    for lineno, raw in enumerate(MD_PATH.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.rstrip("\n")
        stripped = line.strip()
        if stripped.startswith("# ") and not stripped.startswith("# --"):
            current_mfr = stripped[2:].strip().lower()
            continue
        if stripped.startswith("#") or current_mfr is None or " | " not in stripped:
            continue
        cells = [c.strip() for c in stripped.split("|")]
        if not cells or not cells[0]:
            continue
        name = cells[0]
        if name == "Model" or set(name) <= {"-"}:
            continue
        key = (current_mfr, name.lower())
        found[key] = {"model": name, "mfr": current_mfr, "line": lineno}
    return found


def parse_shell_functions(path):
    """Return set of function names defined at top level in a shell script."""
    funcs = set()
    pattern = re.compile(r"^(?:function\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*\(\s*\)")
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        m = pattern.match(line.strip())
        if m:
            funcs.add(m.group(1))
    return funcs


def check_vendors(entries):
    """Check dynamic_load.json mapping and required functions for every referenced mfr."""
    problems = []
    if not DYNAMIC_LOAD.is_file():
        return [f"dynamic_load.json not found at {DYNAMIC_LOAD}"]
    mapping = json.loads(DYNAMIC_LOAD.read_text(encoding="utf-8"))
    mfrs = sorted({e["mfr"] for e in entries.values()})
    for mfr in mfrs:
        script = mapping.get(mfr)
        if not script:
            problems.append(f"manufacturer '{mfr}' present in modem_support.json but missing from dynamic_load.json")
            continue
        script_path = VENDOR_DIR / script
        if not script_path.is_file():
            problems.append(f"manufacturer '{mfr}': dynamic_load.json points to missing script vendor/{script}")
            continue
        funcs = parse_shell_functions(script_path)
        missing = [fn for fn in REQUIRED_VENDOR_FUNCS if fn not in funcs]
        if missing:
            problems.append(f"vendor/{script}: missing required functions: {', '.join(missing)}")
    return problems


def check_fixtures(entries):
    """Return list of models without a recognition fixture directory."""
    missing = []
    for key, e in sorted(entries.items()):
        fixture_dir = RECOGNITION_DIR / e["mfr"] / e["platform"] / f"{e['model']}-*"
        if not list(RECOGNITION_DIR.glob(f"{e['mfr']}/{e['platform']}/{e['model']}-*")):
            missing.append(f"{e['mfr']}/{e['platform']}/{e['model']}")
    return missing


def build_report(entries, md_entries, schema_errors, vendor_problems, fixtures_missing):
    """Build the markdown matrix report (list of lines)."""
    json_not_md = sorted(k for k in entries if k not in md_entries)
    md_not_json = sorted(k for k in md_entries if k not in entries)
    lines = []
    lines.append("# Modem registry consistency report")
    lines.append("")
    lines.append(f"- json entries (usb/pcie): {len(entries)}")
    lines.append(f"- support_list.md rows: {len(md_entries)}")
    lines.append(f"- schema errors: {len(schema_errors)}")
    lines.append(f"- vendor problems: {len(vendor_problems)}")
    lines.append(f"- fixture missing: {len(fixtures_missing)}")
    lines.append("")
    lines.append("| Category | Count |")
    lines.append("| --- | --- |")
    lines.append(f"| in json not in md | {len(json_not_md)} |")
    lines.append(f"| in md not in json | {len(md_not_json)} |")
    lines.append(f"| vendor function/mapping missing | {len(vendor_problems)} |")
    lines.append(f"| recognition fixture missing | {len(fixtures_missing)} |")
    lines.append("")

    def section(title, items):
        lines.append(f"## {title} ({len(items)})")
        lines.append("")
        if items:
            for item in items:
                lines.append(f"- {item}")
        else:
            lines.append("- (none)")
        lines.append("")

    section("In json but not in md", [f"{entries[k]['mfr']}/{entries[k]['model']}" for k in json_not_md])
    section("In md but not in json", [f"{md_entries[k]['mfr']}/{md_entries[k]['model']} (line {md_entries[k]['line']})" for k in md_not_json])
    section("Vendor function/mapping problems", vendor_problems)
    section("Schema errors", schema_errors)
    section("Recognition fixture missing", fixtures_missing)
    return lines


def main():
    parser = argparse.ArgumentParser(description="Modem registry consistency gate")
    parser.add_argument("--report", action="store_true", help="print markdown report to stdout")
    parser.add_argument("--baseline", action="store_true",
                        help="baseline mode: report only, always exit 0 (no gate FAIL)")
    parser.add_argument("--fail-on-fixtures", action="store_true",
                        help="treat missing recognition fixtures as a failure in strict mode")
    args = parser.parse_args()

    entries, schema_errors = load_json_entries()
    md_entries = parse_support_list()
    vendor_problems = check_vendors(entries)
    fixtures_missing = check_fixtures(entries)

    json_not_md = [k for k in entries if k not in md_entries]
    md_not_json = [k for k in md_entries if k not in entries]

    if args.report:
        report = build_report(entries, md_entries, schema_errors, vendor_problems, fixtures_missing)
        sys.stdout.write("\n".join(report) + "\n")

    blocking = bool(schema_errors or json_not_md or md_not_json or vendor_problems)
    if args.fail_on_fixtures and fixtures_missing:
        blocking = True

    if args.baseline:
        # Baseline mode documents current state without failing the gate.
        if args.report:
            sys.stdout.write("\n> baseline mode: report only, gate not enforced (exit 0)\n")
        return 0

    if blocking:
        if not args.report:
            print("modem-registry: inconsistency detected (use --report for details)", file=sys.stderr)
            for k in json_not_md:
                print(f"  in json not in md: {entries[k]['mfr']}/{entries[k]['model']}", file=sys.stderr)
            for k in md_not_json:
                print(f"  in md not in json: {md_entries[k]['mfr']}/{md_entries[k]['model']}", file=sys.stderr)
            for p in vendor_problems + schema_errors:
                print(f"  {p}", file=sys.stderr)
        return 1

    print("modem-registry: OK", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
