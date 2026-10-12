# Long-Term Upstream Sync Strategy (FUjr/QModem and modem_feeds)

- **Status**: Active
- **Date**: 2026-10-12
- **Related**: [branch-policy.zh-cn.md](../release/branch-policy.zh-cn.md), [../adr/0001-refactor-phase4-decisions.en.md](../adr/0001-refactor-phase4-decisions.en.md), [../refactor/README.md](../refactor/README.md), [../../REFACTOR_ROADMAP.md](../../REFACTOR_ROADMAP.md)
- **Chinese version**: [upstream-compat.zh-cn.md](upstream-compat.zh-cn.md)

This document defines how this repository (the refactored line) stays in long-term sync with
upstream `FUjr/QModem` (and its hosting repo `FUjr/modem_feeds`, see `version.mk:1`
"Unified version management for modem_feeds" and the README build badge) while preserving the
Phase 0–4 refactoring results. Core principle: **refactored-surface files are never merged
blindly; non-refactored files can be fast-forwarded safely.**

## 1. Sync Cadence and Commands

### 1.1 Remotes and Mirror

Upstream fetches always go through the mirror prefix
`https://gh.acg2.mom/https://github.com/<owner>/<repo>.git` (the current `upstream` remote in
`.git/config` uses this form). This follows the network policy in
[docs/refactor/README.md](../refactor/README.md) §3: mirror/hosts setup is maintained on the
push machine and never written into the repository.

```sh
# Fetch upstream (one remote each for FUjr/QModem and FUjr/modem_feeds)
git fetch upstream main stable --tags
# Review new upstream commits
git log --oneline HEAD..upstream/main
```

### 1.2 Cadence

| Trigger | Action |
| --- | --- |
| Upstream releases a new version (tag/ReleaseNote) | Full sync evaluation: classify per §2 first, then merge class by class |
| Upstream adds modem support (`modem_support.json` / vendor changes) | Follow the §3 absorption flow |
| Upstream CI/packaging fixes | Evaluate, then merge per §2 classification |
| Before landing any refactor journal | Confirm no references to files absent upstream (avoid reverse drift) |

Fetch at least once a month as routine; if two consecutive upstream versions are missed,
catch up before starting any new refactor work package.

### 1.3 Sync Report (Planned)

The repository currently has **no** `upstream-sync-report` workflow (as of 2026-10-12,
`.github/workflows/` contains no such file). Planned shape: a scheduled/`workflow_dispatch`
job that fetches through the mirror, produces a changed-file list for `HEAD..upstream/main`,
and annotates each file with the ownership class from §2, published as an artifact. Until it
lands, use the manual commands above plus the §2 table. **Do not** pretend the workflow
already exists in docs or commit messages.

## 2. File Ownership Classification

### 2.1 Decision Method (Authoritative Source)

A file's class is determined by whether `docs/refactor/journal-*` touches it:

```sh
# List all paths named by the refactor journals (example query)
grep -rhoE '(application|luci|rust|scripts|docs/refactor)/[A-Za-z0-9_./-]+' \
  docs/refactor/journal-*.md | sort -u
```

### 2.2 REFACTORED_OWNED: Manual Three-Way Merge Required

The following files were substantively rewritten during the refactor; overwriting them would
lose security/quality fixes. When syncing, merge each file via `git merge-file` / manual
three-way merge, then pass the gates in [docs/refactor/README.md](../refactor/README.md) §2.

| File/Directory | Refactor Content | Evidence (journal) |
| --- | --- | --- |
| `application/qmodem/files/usr/share/qmodem/cmds/qmodem_lib.sh` | New shared parsing library (csv/regex etc.) | [journal-vendor-lib.md](../refactor/journal-vendor-lib.md), [journal-tests-expand.md](../refactor/journal-tests-expand.md) |
| `application/qmodem/files/usr/share/qmodem/vendor/*.sh` (12 vendor scripts) | Full migration to qmodem_lib, residue cleanup, ql_regex_pick hardening | [journal-vendor-full.md](../refactor/journal-vendor-full.md), [journal-vendor-residue.md](../refactor/journal-vendor-residue.md), [journal-regex-pick.md](../refactor/journal-regex-pick.md) |
| `application/qmodem/files/usr/libexec/rpcd/qmodem` | eval removal, atomic cache, bashism zeroed, get_stats/get_current_band endpoints | [journal-rpcd.md](../refactor/journal-rpcd.md), [journal-rpcd-gap.md](../refactor/journal-rpcd-gap.md), [journal-tests-expand.md](../refactor/journal-tests-expand.md) |
| `rust/` (qmodem-pdu / qmodem-atd / qmodem-smsd) | Rust async core and tests | [journal-rust-core.md](../refactor/journal-rust-core.md), [journal-pdu-conform.md](../refactor/journal-pdu-conform.md), [journal-ffi-ubus.md](../refactor/journal-ffi-ubus.md), [journal-smsd-rs.md](../refactor/journal-smsd-rs.md), [bench-atd.md](../refactor/bench-atd.md) |
| `application/ubus_at_daemon/` | C memory-safety fixes (C1–C7), M1–M5 finishing | [journal-c-core.md](../refactor/journal-c-core.md), [journal-atd-quality.md](../refactor/journal-atd-quality.md) |
| `application/tom_modem/` | Dedup and C14/C15 fixes | [journal-tom-modem.md](../refactor/journal-tom-modem.md) |
| `application/libqmodem-sms/`, `extlib/pdu_decoder.c` | Boundary fixes, audit finishing | [journal-c-core.md](../refactor/journal-c-core.md), [journal-sms-c-audit.md](../refactor/journal-sms-c-audit.md) |
| `application/modem_scan/` (scand/scanc) | Socket permissions, whitelist, write-loop fixes | [journal-c-scan.md](../refactor/journal-c-scan.md), [journal-scan-c-final.md](../refactor/journal-scan-c-final.md) |
| `application/qmodem_smsd/` | C4/C10/C12 fixes | [journal-c-smsd.md](../refactor/journal-c-smsd.md) |
| `application/qmodem_monitor/` | Injection fixes, heartbeat schema, concurrency locks | [journal-monitor-dial.md](../refactor/journal-monitor-dial.md), [journal-monitor-hard.md](../refactor/journal-monitor-hard.md) |
| `modem_dial.sh` / `modem_util.sh` / `modem_hook.sh` / `modem_monitor.sh` | APN/zone fixes, at_tool=2 path, shared DNS constants | [journal-shell-pkg.md](../refactor/journal-shell-pkg.md), [journal-monitor-dial.md](../refactor/journal-monitor-dial.md), [journal-atd-bridge.md](../refactor/journal-atd-bridge.md), [journal-dial-zone.md](../refactor/journal-dial-zone.md) |
| `luci/luci-app-qmodem-next/` | Componentization, ACL, stats/band endpoints, TTL view | [journal-luci-next.md](../refactor/journal-luci-next.md), [journal-luci-comp.md](../refactor/journal-luci-comp.md), [journal-luci-final.md](../refactor/journal-luci-final.md), [journal-luci-gap.md](../refactor/journal-luci-gap.md), [journal-retire-ttl.md](../refactor/journal-retire-ttl.md) |
| Legacy LuCI apps (`luci-app-qmodem`, `-sms`, `-ttl`, `-ttlfw4`, `-mwan`, `-voip`, `-hc`, `-monitor`) | Injection fixes, retirement annotations, Decisions 1–4 handling | [journal-lua-retire.md](../refactor/journal-lua-retire.md), [journal-retire-s1.md](../refactor/journal-retire-s1.md), [journal-retire-mwan.md](../refactor/journal-retire-mwan.md), [journal-retire-voip.md](../refactor/journal-retire-voip.md), [journal-retire-hc.md](../refactor/journal-retire-hc.md) |
| Per-package `Makefile` and init scripts | PKG normalization, dependency review | [journal-shell-pkg.md](../refactor/journal-shell-pkg.md), [journal-pkg-audit-all.md](../refactor/journal-pkg-audit-all.md) |

### 2.3 UPSTREAM_OWNED: Safe to Accept Upstream Overrides

The following paths are untouched by any journal (per the §2.1 query); upstream changes can be
checked out directly:

- Data files: `modem_support.json`, `modem_port_rule.json`, `at_commands_en/zh.json`
  (after merging, run `scripts/update_support_list.py` to verify consistency with
  `docs/support_list.md`, see `.github/workflows/commit_check_supportlist.yaml`).
- Independent vendored components: `application/qfirehose/` (license forbids local
  modification; security fixes follow ADR 0001 Decision 7 via upstream patch tracking, no
  local deep changes), `application/quectel_CM_5G_M/`, `application/ndisc6/`,
  `application/qmodem-seal/`, `application/voipd/`, `application/qmodem_sipd/`.
- Documentation: user-facing content under `docs/` (`support_list.md`, `tom-modem-manual*`,
  etc.); the locally added `docs/refactor/`, `docs/developer/` (async-daemon,
  at-tool-selection, etc.), `docs/adr/`, `docs/release/` are reverse-owned and excluded from
  upstream overrides.
- Untouched upstream scripts and increments in `testcases/`, `tests/`.

Even before overriding, run a gate smoke pass (`scripts/check_all.sh --changed`) to guard
against interface drift between upstream files and the refactored surface.

### 2.4 Consistency Constraints with ADR 0001

- Syncing **must not** resurrect `luci-app-qmodem-ttl` (Decision 1) or pull mwan/voip/HC into
  retirement/migration (Decisions 2–4).
- `at_tool` default stays `'0'` (Decision 5); any upstream AT-path change requires re-running
  fixture replay.
- PDU packaging defaults to the C implementation (Decision 6); upstream SDK changes do not
  trigger an automatic switch.
- The three qfirehose high-severity issues are handled via "upstream patch tracking + usb2tcp
  disabled by default" (Decision 7); re-verify whether upstream has fixed them on each sync.

## 3. Absorption Flow for Upstream New Modems

> The planned `new-modem-quickstart` document and `check_modem_registry` script are **not yet
> in the repository** (no hits as of 2026-10-12). Until they land, the flow below follows the
> current files; update the references in this section once they exist.

1. **Register**: when upstream adds a model to
   `application/qmodem/files/usr/share/qmodem/modem_support.json`, confirm `modem_name`
   matches the output of `get_modem_model()` in `modem_scan.sh` (see
   [docs/How_to_add_modem_cn.md](../How_to_add_modem_cn.md) and
   [docs/developer/modem-adaptation.zh-cn.md](modem-adaptation.zh-cn.md); the English
   adaptation guide is pending).
2. **Vendor scripts**: a new vendor script must `source` `cmds/qmodem_lib.sh` and use the
   shared parsing library (conventions in
   [journal-vendor-lib.md](../refactor/journal-vendor-lib.md)); do not merge upstream
   legacy-style pipelines directly.
3. **Feature gating**: complete or disable features via the `get_disabled_features` case in
   `modem_ctrl.sh`.
4. **Validation**: gated by `commit_check_supportlist.yaml` (JSON validity +
   `docs/support_list.md` diff) and `scripts/update_support_list.py`; the planned
   `check_modem_registry` should replace the manual part of this check.
5. **Fixtures**: when hardware is available, add AT fixtures per `testcases/README.md` and
   include them in replay.

## 4. Feed/SDK Generation Matching Rules (Summary)

Source: `.github/workflows/main.yml` (the "Resolve snapshot SDK" and "Initialization
environment" steps).

- **Snapshot SDK** (non-empty `SDK_PREFIX`): feeds track the `master`/`main` development lines.
- **Release SDK** (e.g. 23.05): feeds must use the `openwrt-X.Y` branch of the same generation
  as the SDK — `feed_ref` is derived from `SDK_NAME` via `sed`, and base/packages/luci/routing
  all use the same `feed_ref`; derivation failure aborts with an error. Rationale: the release
  SDK ships CMake 3.26 while luci master already requires 3.31; cross-generation feeds break
  packages such as `rpcd-mod-luci`.
- This repository is injected into the SDK as the `qmodem` feed via `src-link`
  (`feeds.conf`) and built together with the SDK generation; `version.mk`'s
  `QMODEM_VERSION`/`QMODEM_RELEASE` is the single version source on the feed side
  (currently `3.4.0_rc3`; apk accepts only `_rcN` prerelease suffixes and `PKG_RELEASE` must
  stay numeric).
- Tag/branch constraints: see [branch-policy.zh-cn.md](../release/branch-policy.zh-cn.md) —
  prerelease tags may only point to `main`, stable tags only to `stable`, enforced by the tag
  source validation in `main.yml`.

**Sync implication**: if upstream changes the `feeds.conf` generation logic or bumps the
target OpenWrt generation, sync the whole generation and re-verify on both matrix axes
(ipk/apk, arm64/x64/mmips); never change only half.
