# Changelog

[English](CHANGELOG.md) | [简体中文](CHANGELOG.zh-CN.md)

All notable changes to this project are documented in this file.
Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning follows
`version.mk` (main branch: pre-release, stable branch: release).

> 本仓库自 2026-10 起经历了一轮系统性重构（审计 → 止血 → Rust 异步核心 → 单包交付）。
> 详细工程记录见 [docs/refactor/](docs/refactor/README.md)，问题依据见各 journal。

## [Unreleased]

### Fixed
- **Release assets are now directly installable** (`main.yml`,
  `publish-existing-release.yml`): artifacts are no longer repacked into
  `QModem-<arch>_*.tar.gz`; the Release ships loose `.ipk` / `.apk` files
  instead. Per-arch `apk` asset names carry an arch suffix
  (e.g. `qmodem-3.4.0_rc3-r1_aarch64.apk`) because OpenWrt apk filenames do
  not encode the target arch. Download and install directly —
  `opkg install *.ipk` / `apk add --allow-untrusted *.apk` — no extraction.
  The v1.0.0 Release was migrated accordingly (6 tar.gz assets replaced by
  80 loose packages).
- **`qmodem-suite` meta-package is built again**: it was missing from the SDK
  build list, so the "single install delivers the full stack" promise had no
  shippable artifact. Added to `qmodem_package_generic` and to the
  required-package gate in `main.yml` (its `DEPENDS`
  `qmodem_monitor` / `luci-app-qmodem-monitor` / `luci-app-qmodem-ttlfw4`
  are pulled in by the same compile).

## [1.0.0] - 2026-10-12

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

### Phase 2.5 — Wrap-up review & deliverable finalization (`02b4c70`..`777620d`, 2026-10-12)

> Evidence base: `ls docs/refactor/` + `git log` 核对。journal 尚未落库/未合入 main 的
> 条目（rpcd gap 端点、vendor 残留清理专项、rust FFI/bench/smsd-rs/pdu-conform）
> 有意不记录，留给编排者合入后补充。

#### Added
- **Rust cross-build deliverables finalized** (`02b4c70`):
  [journal-rust-cross.md](docs/refactor/journal-rust-cross.md) 落库（musl linker
  兜底链 musl-tools → musl.cc、产物 450,544 B stripped 实测、SDK 源码构建 /
  CI 预编译注入两条集成路线）；`rust/README.md` 增加 Cross compilation 章节。
- **旧栈退役 Step 1 审查结论**（[journal-retire-s1.md](docs/refactor/journal-retire-s1.md)，
  随 `ebb90dd` 合入）：7 个 legacy 包 Makefile 审查 + 维护模式标注，6 个旧栈 htm
  首行退役注释；端点 gap 表（Phase 4 输入）——`get_modem_cfg` luad 端点是子包
  DEPENDS 切 `+qmodem`（Step 2）的关键阻塞；HC SIM 切换、频段/流量统计端点、
  mwan/TTL/voip 页面在 next 栈暂无对应物。

#### Fixed
- `scripts/cross-build-rust.sh`：`+crt-static` 改为仅对 `*-musl` target 注入
  （无条件注入曾使原生 gnu 产物 450 KB → 1.4 MB）(`02b4c70`)；可执行位恢复
  `100644 → 100755` (`777620d`)。
- `luci-app-qmodem-ttlfw4`：补缺失的 `PKG_LICENSE_FILES:=LICENSE`（Makefile
  审查发现，随 `ebb90dd` 合入）。
- `ubus_at_daemon`：退出清理路径自死锁修复（持 `ports_mutex` 期间递归加锁
  销毁端口；改为弹出链表头后锁外销毁，见 journal-atd-quality.md C7 收尾）。

### Phase 3 — Rust deepening, gap closure, retirement review (`T4`, 2026-10-12)

#### Added
- Rust `ubus_ffi`: pure-Rust blobmsg codec (byte-compatible with libubox,
  12 golden/robustness tests) + feature-gated libubus FFI + mock-transport
  integration tests; `qmodem-smsd` Rust skeleton (Storage trait, claim/ack,
  backoff, scheduler, URC hub — zero deps); PDU conformance suite 14→64 tests
  (TS 23.038 vectors, C-cross-checked semantics, property robustness) —
  **found & fixed a real SCTS timezone bug** (semi-octet per TS 23.040
  9.2.3.11; +5:30/+8:00/±12:45 were all wrong); atd micro-benchmarks
  (p50 50ms roundtrip, 31.4k cmd/s, broadcast 6.8µs).
- rpcd endpoints `get_stats` / `clear_stats` / `get_current_band` + LuCI next
  Statistics view (ACL/menu/po) — closes the traffic-stats & current-band gaps.
- Retirement review conclusions for mwan (keep), ttl/ttlfw4 (merge later),
  voip (keep standalone), hc (needs on-device verification) — see journals.
- `docs/security/vendored-audit.md`: qfirehose has 3 high-risk findings
  (system() injection via upgrade package paths, XML-driven overflow,
  unauthenticated TCP→USB passthrough) — patch/isolate upstream.
- UCI options reference (73 options, zh/en), docs EN translations ×3,
  release.yml (changelog-driven releases), CI markdownlint.

#### Changed / Fixed
- Vendor residue cleanup (huawei.sh 8 CR-cleanups folded into ql_at_clean);
  dial: deterministic wan-zone lookup, qmi_dial backoff counter reset fix;
  at-daemon & scand: -Wall -Wextra clean, bounded pdu_decoder, uci-get parsing;
  sms-tool_q: fixed same-origin C5/C6-class PDU overflows (ASan fuzz 200k, 0);
  monitor: heartbeat schema version, per-modem flock, process-group kill;
  **single-package delivery fixed**: qmodem was shipping without installing 5
  init.d scripts; qmodem-suite uci-defaults now enables all services.
### Phase 0 — Audit-driven hardening (`2568198`, 2026-10-12)

- Initial evidence-based hardening pass covering C memory safety, shell
  injection surfaces, packaging and LuCI (27 files); details in
  [docs/refactor/](docs/refactor/README.md) journals.

### 版本化建议 — [Unreleased] → v1.0.0 拆分

- **Feature（minor 升级动因，1.0.x → 1.1.0）**：
  - `qmodem-suite` 元包（单包交付 + uci-defaults 引导）
  - `rust/` 异步核心（`qmodem-pdu` / `qmodem-atd`）与 `at_tool=2` AT 通路
  - `cmds/qmodem_lib.sh` 公共解析库 + 12 厂商脚本全量迁移
  - `scripts/cross-build-rust.sh` + `rust-build.yml` 交叉编译矩阵
  - `scripts/check_all.sh` 统一门禁 + `tests/sh` / `tests/lua` 回归体系
- **Fix（随 rc1 一并发布）**：Phase 0/1/2 全部安全与内存安全修复（注入、
  路径穿越、XSS、socket 权限、UAF/死锁）及打包修正（`PKG_LICENSE_FILES`、
  `INSTALL_*` 宏、可执行位）。
- **不纳入 rc1**：退役 Step 2（`get_modem_cfg` 等价端点与 DEPENDS 切换）与
  Phase 4 旧栈删除属 v1.2.0 范围；尚未合入 main 的 Phase 2.5+ 条目（rpcd gap
  端点、rust FFI/bench/smsd-rs/pdu-conform）在合入前不写入发布说明。

## [Base] — main@c49654e (2026-09-19)

- Upstream baseline this fork's refactor history starts from
  (synced from [FUjr/QModem](https://github.com/FUjr/QModem)).
