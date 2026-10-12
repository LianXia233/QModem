# Changelog

All notable changes to this project are documented in this file.
Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning follows
`version.mk` (main branch: pre-release, stable branch: release).

> 本仓库自 2026-10 起经历了一轮系统性重构（审计 → 止血 → Rust 异步核心 → 单包交付）。
> 详细工程记录见 [docs/refactor/](docs/refactor/README.md)，问题依据见各 journal。

## [Unreleased]

### Phase 1 — Dependency reduction & single-package delivery (`f874091`, 2026-10-12)

#### Added
- **`qmodem-suite` meta package**: one install delivers the full stack
  (`qmodem` + `luci-app-qmodem-next` + `qmodem-smsd` + `qmodem_monitor` +
  `luci-app-qmodem-monitor`), with a uci-defaults bootstrap that enables the
  services on first install. See [docs/user/install-suite.zh-cn.md](docs/user/install-suite.zh-cn.md).
- **`rust/` async core, zero external crates**: `qmodem-pdu` (3GPP TS 23.038 SMS
  PDU codec, bounds-checked rewrite of `libqmodem-sms`) and `qmodem-atd`
  (thread-actor AT daemon: per-port command queue, monotonic timeouts, URC
  broadcast, unix-socket JSON test API, feature-gated `tty`/`ubus`).
  Binary size −43.4% vs the earlier tokio build (748 KB). 37 tests.
  See [docs/developer/async-daemon.zh-cn.md](docs/developer/async-daemon.zh-cn.md).
- **`cmds/qmodem_lib.sh`**: shared POSIX/busybox parser library
  (`ql_at_clean`, `ql_csv_field`, `ql_regex_pick`, `ql_num_or`, ...);
  quectel/huawei vendor scripts migrated as pilot (46 duplicated pipelines
  removed), fixture regression 30/30 byte-identical.
- **Unified quality gate**: `scripts/check_all.sh` (shell syntax, JS check,
  JS smoke test, cargo test/clippy, C syntax, CRLF — auto-discovered) +
  `.github/workflows/quality.yml`; `vendor-fixtures.yml` now reuses it.
- **Dependency-free test suites**: `tests/sh/` (assert framework + 18 cases)
  and `tests/lua/smoke.sh` (16 cases), integrated with real `testcases/`
  fixtures.
- **Developer/user docs**: async daemon guide, single-package install guide,
  `docs/refactor/README.md` index, bilingual README sections.
- `qmodem` now declares its missing hard dependency `+jsonfilter`.

#### Changed
- `luci-app-qmodem-next`: shared `ui.js` component layer (modem selector,
  time formatting, rate-limited poll guard), CSS deduplication, defensive
  handling of empty ubus payloads.
- Legacy Lua stack de-mutexed: `luci-app-qmodem` and `luci-app-qmodem-next`
  can now be co-installed (user's choice).
- `qmodem_smsd`: URC-triggered sync moved off the ubus dispatch thread into a
  serialized worker (no more 15 s method stalls).

#### Fixed — security
- **Remote (modem-controlled) input** no longer reaches root shells:
  `ubus_at_daemon` event callbacks execute scripts via `execl` positional
  arguments instead of `system()` string interpolation.
- Legacy LuCI controller: command injection via `io.popen` parameter
  interpolation fixed (action whitelist + shell quoting); SMS body/sender XSS
  fixed (innerHTML → textContent).
- `modem_scand` control socket: `0666` → `0660` + dedicated group; section and
  scan-type whitelists added.
- rpcd plugin: cache path traversal blocked (`valid_section` whitelist on all
  methods), dead second `read` in `delete_sms`/`send_raw_pdu`/`send_sms`
  removed (these methods silently returned empty before), `set_imei` input
  validated (15 digits).
- `modem_dial.sh`: APN credentials are shell-quoted before use (was a root
  command-injection vector via UCI); SIM PIN no longer logged in plaintext.

#### Fixed — memory safety & correctness
- `ubus_at_daemon`: uninitialized `at_response_t` on early return; per-port
  refcounting (use-after-free between concurrent `sendat` and `close`);
  `CLOCK_MONOTONIC` AT timeouts (NTP-jump safe); end-flags data race; write
  failure response-state cleanup.
- `tom_modem`: `tty_open_device` rewritten (fd leak / dangling `FILE*`);
  operation/error enum collision that made an invalid argument silently
  execute SMS read.
- `libqmodem-sms` PDU codec: out-of-bounds reads on attacker-controlled PDU,
  negative-index write, two always-false validation conditions (`&&` → `||`),
  missing return-value checks, 7-bit overflow guard.
- `qmodem_smsd`: unbounded `sprintf` into stack buffers for SMS JSON output;
  UCS2→UTF-8 conversion overrun (incl. complete-multibyte truncation);
  NULL column text; COMMIT failure now rolls back.
- `modem_hook.sh`: inverted sentinel condition (post-init AT sequences never
  ran) and dead `delay` variable fixed.
- `modem_dial.sh`: POSIX syntax error in PIN comparison, unquoted expansion,
  always-true interface check, version detection fallback for snapshots,
  qmi_dial retry backoff, targeted `wait`.
- `modem_scand`: unchecked `pthread_create` / join-on-uninitialized.
- Packaging: `$(CP)` install → `INSTALL_*` macros for `qmodem`;
  `PKG_RELEASE` honors `version.mk` autorelease; `PKG_LINCESE_FILES` →
  `PKG_LICENSE_FILES` in 5 packages.
- LuCI next: ACL read scope narrowed from `*` to 27 read methods; dangerous
  `'modem_1'` silent fallbacks removed; dead `LuciTable` code carrying an
  innerHTML XSS pattern deleted; SMS timestamps parsed from the PDU timezone
  byte (removes the hard-coded `−28800` hack); NaN progress bar; scan-reload
  inconsistency.

### Phase 2 — Vendor migration completion & Rust integration (`T3`, 2026-10-12)

#### Added
- AT tool path 3: `qmodem.main.at_tool=2` routes AT commands through the Rust
  `qmodem-atd` unix-socket JSON protocol (opt-in; defaults unchanged), with
  matching developer docs ([AT tool selection](docs/developer/at-tool-selection.zh-cn.md)).
- `scripts/cross-build-rust.sh`: aarch64/x86_64 musl static cross-builds of the
  Rust core (+ `rust-build.yml` CI matrix, artifacts with sha256).
- Legacy stack retirement Step 1: maintenance-mode markers on 7 legacy packages
  and a feature-gap table feeding Phase 4 removal.

#### Changed
- All 12 vendor adapter scripts now use the shared `cmds/qmodem_lib.sh`
  parsers (pilot in Phase 1 covered quectel/huawei; ~280 duplicated pipelines
  retired repo-wide).
- rpcd plugin: generic `eval` removed (whitelisted command dispatch), cache
  writes made atomic (mktemp+mv), POSIX-safe throughout (dash verified),
  repeated `ubus call` boilerplate consolidated.
- `ubus_at_daemon`: monitor thread no longer holds the global lock while
  probing ports; config loader bounds fixed; lease tokens now random.
- `tom_modem`: SMS read paths de-duplicated, operation matching table-driven,
  greedy read gets an absolute deadline.
- LuCI next: dropdown reads via component API, list limit configurable,
  parallel ubus polling (allSettled), i18n gaps closed.

### Phase 0 — Audit-driven hardening (`2568198`, 2026-10-12)

- Initial evidence-based hardening pass covering C memory safety, shell
  injection surfaces, packaging and LuCI (27 files); details in
  [docs/refactor/](docs/refactor/README.md) journals.

## [Base] — main@c49654e (2026-09-19)

- Upstream baseline this fork's refactor history starts from
  (synced from [FUjr/QModem](https://github.com/FUjr/QModem)).
