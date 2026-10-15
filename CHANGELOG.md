# Changelog

[English](CHANGELOG.md) | [简体中文](CHANGELOG.zh-CN.md)

All notable changes to this project are documented in this file.
Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning follows
`version.mk` (main branch: pre-release, stable branch: release).

> 本仓库自 2026-10 起经历了一轮系统性重构（审计 → 止血 → Rust 异步核心 → 单包交付）。
> 详细工程记录见 [docs/refactor/](docs/refactor/README.md)，问题依据见各 journal。

## [Unreleased]

### Added
- **Full visual and asynchronous refactor of all 11 core LuCI views**:
  - `overview`: Dynamic cellular-wave SVG antenna & mascot, 4 live KPI metric cards (Signal & RAT, Carrier & Band, Uplink Session, SIM & Hardware), shimmer skeleton placeholders, and concurrent asynchronous data hydration.
  - `network_config`: Animated dual-antenna 5G router SVG hero, resilient timeout-bounded status polling (eliminating blocking page hangs), and live pulsing service indicator chips.
  - `debug`: Rebuilt as a macOS/Anime-inspired Cyber Terminal (`.qmodem-terminal-frame`), animated terminal SVG mascot, diagnostic command preset chips, 1-click clipboard copy, and Enter-key async execution.
  - `config_advanced`: Dynamic tuning gear SVG hero, streamlined modem selection toolbar, and fluid responsive tab panels.
  - `sms`: Animated glowing chat bubble SVG hero, redesigned message thread and storage preference panels.
  - `settings`: Animated microchip & PCB trace SVG hero, resilient timeout-safe parallel RPC fetching preventing cold-start stalls.
  - `monitor`: Animated radar sweep & heartbeat wave SVG hero, cardified watchdog metrics.
  - `ttl`: Animated packet shield SVG hero, 1-click presets for common carrier bypass rules (64 Linux/Android, 128 Windows, 65 Bypass).
  - `sim_switch`: Animated dual-SIM chip SVG hero, visual interactive slot cards (Slot 1 & Slot 2) with active RF state badges.
  - `sms_forward`: Animated paper-airplane messenger SVG hero, multi-channel push configuration cards.
  - `sms_sim`: Animated Micro-SIM memory SVG hero, modern toolbar card, and batch action tables.
- **Soft Anime design system for `luci-app-qmodem-ttlfw4`**: self-contained `soft-anime.css`, Pastel Glass warning cards, daily cache-busting, and bashism cleanup.
- **Enhanced service status chips for `luci-app-qmodem-monitor`**: unified `.qmodem-chip` and `.qmodem-status-row` pill badges for service state.

- **Legacy Lua UI packages removed**: `luci-app-qmodem` (Lua), `luci-app-qmodem-sms`, `luci-app-qmodem-ttl`, `luci-app-qmodem-hc`, `luci-app-qmodem-mwan` have been deleted from `luci/`. Essential hardware and daemon scripts (`modem_sim.sh`, `modem_mwan.sh`, `qmodem_hc_sim`, `qmodem_mwan`, `99-setup-mwan`, `01-reload-mwan3.dhcpscript`) were migrated directly into the all-in-one `qmodem` backend package. This permanently resolves the menu routing conflict between legacy Lua controllers and modern JS views. These five packages were previously flagged `DEPRECATED` in the `[Unreleased]` section; that entry is superseded by this one because they were removed outright rather than kept through a transition period.

### Fixed
- **Missing translations for the new overview hero strings**: the Soft Anime
  overview introduced three new UI strings (`Modem Dashboard`,
  `%d modem(s) detected`, and the hero subtitle) that were never extracted
  into the translation catalog, so a Chinese (`zh_Hans`) interface rendered
  the hero band in English while the rest of the page was translated
  (observed on a live device after installing the 1.1.1 build). The three
  strings are now present in `po/template/qmodem-next.po` and translated in
  `po/zh_Hans/luci-app-qmodem-next.po`.

## [1.1.1] - 2026-10-14

### Fixed
- **CI manual dispatch can now upload Release assets**: `main.yml`'s
  `job_release_artifacts` requires `push_type == 'tag'`, but a
  `workflow_dispatch` run was always classified as `branch`, so the
  "发布产物" (publish artifacts) job was permanently skipped — manual
  builds compiled successfully yet never uploaded ipk/apk to any
  Release. `workflow_dispatch` now accepts an optional `tag` input:
  when provided, the run is treated as a release build and its
  installable packages are uploaded to that existing Release (useful
  for republishing assets after a fix); leaving it empty keeps the
  build-only behaviour.

### Removed
- **WAN hijack uci-defaults (`99-rmnet-init`)**: the package shipped
  `/etc/uci-defaults/99-rmnet-init`, which on every install/upgrade
  unconditionally rewrote `network.wan` to `proto 'rmnet'` +
  `ifname 'wwan0'` (and `wan6` to `rmnet6`), silently clobbering any
  user WAN configuration. On devices where the WAN port should be a
  standard DHCP uplink this breaks WAN entirely: the rmnet proto waits
  for a cellular data-call state file and a `wwan0` interface that do
  not exist, netifd falls back to `proto 'none'`, no DHCP client is
  ever started and the port gets no address (diagnosed on a
  WAN-first ImmortalWrt device: manual `proto 'dhcp'` repair was undone
  by the next qmodem upgrade re-running this script). The same script
  also forced an odhcpd relay setup (`dhcp.lan/wan/wan6` ra/dhcpv6/ndp
  relay + `dhcp.wan6.master=1`) that presumes the cellular-on-wan
  design and leaves LAN IPv6 dead when wan6 carries no real uplink.
  The script and its Makefile install line are removed; qmodem now
  leaves `network` / `dhcp` untouched at install time (its own modem
  interfaces are created by `modem_dial.sh` as separate
  `modem_*` sections and never touched wan). The inert netifd proto
  handlers `rmnet.sh` / `rmnet6.sh` stay for existing configs that
  still reference them. Device-side cleanup for already-infected
  installs: `uci set network.wan.proto='dhcp'`, drop the bogus
  `ifname 'wwan0'` / `defaultroute '0'` remnants on wan/wan6, and
  delete the relay overrides in `/etc/config/dhcp`.

### Changed
- **LuCI frontend reverted to upstream `luci-app-qmodem-next`**: the entire
  `luci/` directory has been realigned to upstream FUjr/QModem@c49654ef
  (the fork point itself, upstream's latest main). The local
  `luci/luci-app-qmodem` package — including the Soft Anime Pastel Glass
  theme system, the `theme.pageHead` unification pass and all downstream
  customizations — is removed and replaced by the upstream
  `luci-app-qmodem-next` modern JS UI (which upstream ships as a separate
  package depending on `qmodem`, `qmodem-smsd` and `sms-forwarder-next`;
  our single `qmodem` package already `PKG_PROVIDES` those). The `luci/`
  tree matches upstream except one deliberate adaptation in
  `luci-app-qmodem-next/Makefile` (the realign adds
  `luci-app-qmodem-monitor` / `-sms` / `-ttl` / `-ttlfw4` / `-next` and
  restores upstream `luci-app-qmodem`): upstream declares
  `+qmodem-smsd +sms-forwarder-next` as build-time dependencies, which
  only exist as separate packages in upstream's multi-package layout —
  here they are folded into the single `qmodem` package (`PKG_PROVIDES`
  covers install-time only), and OpenWrt metadata resolution drops the
  package at defconfig when a build-time dependency name does not exist
  (observed: "missing required release package: luci-app-qmodem-next").
  The dependency list is therefore converged to `+qmodem +luci-base`.
  Build/CI references were adapted
  accordingly and only touch luci-related package names:
  `qmodem_package_generic` now compiles `luci-app-qmodem-next`,
  `main.yml` required packages are `luci-app-qmodem-next` +
  `luci-i18n-qmodem-next-zh-cn`, `openwrt_package.config` selects the
  next packages, `extract-i18n.yml` and the i18n scripts' default paths
  point at `luci/luci-app-qmodem-next`, and `check_all.sh` runs the
  next package's `test_sms_service.js`. No other components were
  modified.

## [1.1.0] - 2026-10-14

> Tag `v1.0.1` never shipped a Release — the build was blocked by the
> ubuntu-26.04 runner's uutils coreutils prereq failure, so everything
> since 1.0.0 is included here.

### Added
- **Pastel Glass SMS conversation skin (theme.js)**: new `.qm-app .sms-*`
  rules override the legacy conversation CSS — received bubbles render as
  `--qm-surface` cards with soft shadows, sent bubbles use the
  `--qm-accent-gradient`, the message area becomes a glass panel
  (backdrop-filter + `--qm-glass-shadow`), and unread badges sit on
  `--qm-danger`. Light/dark both follow the shared tokens. Also ≤560px
  mobile tweaks: bubbles widen to `max-width:88%`, the message area is
  capped at 420px, in-bubble delete buttons become 36px touch targets and
  generic `.btn` / `.cbi-button` controls are raised to 42px.
- **Rust target optimized for Cortex-A53**: `build-rust.sh` now passes
  `-C target-cpu=cortex-a53` for `aarch64*` targets. Measured with
  `rustc --print cfg`, the default `generic` CPU exposes only
  `crt-static`/`neon`, while cortex-a53 adds `aes`/`crc`/`sha2`/`pmuv3`; the
  MT7981 A53 on the reference device reports
  `asimd aes pmull sha1 sha2 crc32` in `/proc/cpuinfo`, so enabling them is
  safe. Devices whose A53 lacks the ARMv8 crypto extensions can fall back with
  `QMODEM_RUST_TARGET_CPU=generic` (no script edit needed). The C side keeps the
  OpenWrt SDK's own `-mcpu=cortex-a53`, so both halves agree.
- **ImmortalWrt-native build variant (`arm64_apk_immortalwrt`)**: QModem is now
  additionally built from the ImmortalWrt SNAPSHOT SDK
  (`immortalwrt-sdk-mediatek-filogic_`) with `SDK_FLAVOR=immortalwrt`, which
  makes `main.yml` select ImmortalWrt feeds: `immortalwrt/immortalwrt` as the
  base feed (the source of the ubus / libubox / libblobmsg-json ABI) plus the
  ImmortalWrt `packages` and `luci` mirrors. Packages produced earlier were
  built from the OpenWrt SDK and OpenWrt feeds, whose ubus ABI is
  `libubus20260925`; ImmortalWrt master ships `libubus20260628`, so those apk
  files could not be installed on an ImmortalWrt device (unsatisfiable
  dependency). The OpenWrt variants are unchanged. ImmortalWrt assets are
  published with an `-immortalwrt` suffix so they do not overwrite the
  identically named OpenWrt assets inside a Release.

- **Rust components are now built into the OpenWrt package (`qmodem-atd` ships
  in the package)**: `application/qmodem` gains `build-rust.sh`, and the first
  step of `Build/Compile` cross-compiles `qmodem-atd` with cargo and installs it
  to `/usr/bin/`. The linker is always the OpenWrt-provided cross toolchain
  (not musl.cc or another third-party toolchain) so the musl configuration of
  the target sysroot — and the ABI of the future libubus / libjson-c /
  libsqlite3 link — match exactly. The target triple is derived from `$(ARCH)`
  (aarch64 / mipsel / x86_64 → `*-unknown-linux-musl`); **mipsel musl is Rust
  tier 3** (`std` is available but rustup ships no prebuilt std), so it needs
  nightly + rust-src + `-Z build-std=std,panic_abort`, while aarch64 /
  x86_64 are tier 2 and only need the stable toolchain — `main.yml` installs
  the right one per matrix variant. The local gate was upgraded accordingly:
  the `rust` stage of `check_all.sh` now runs
  `cargo check + clippy --all-features` against the preinstalled musl target
  std on non-Unix hosts (no longer SKIP), and the `shell` stage now includes
  package-root build scripts and `scripts/*.sh` (coverage 74 → 114 files).
- **qmodem-atd system service and config**: new `/etc/init.d/qmodem-atd` and
  `/etc/config/qmodem-atd`; every `config port` section is registered with
  `--port` at daemon start and the socket is pinned to
  `/var/run/qmodem/atd.sock` (default `enabled='0'`, opt-in). The three stale
  `/var/run/qmodem-atd.sock` paths in the docs were normalised to
  `/var/run/qmodem/atd.sock`.
- **voipd's AT adapter now speaks the qmodem-atd unix socket**: the
  `at_daemon_adapter.sh` no longer calls the ubus `at-daemon` object
  (`list` / `open` / `sendat`) and instead uses the `{"cmd":"list"}` /
  `{"cmd":"sendat"}` JSON line protocol (`nc -U` first, `curl --http0.9`
  fallback). Ports are registered at daemon start, so the old
  "open it first via `ubus call at-daemon open`" handshake branch is gone, and
  the response fields are aligned from `status=success / end_flag_matched /
  response` to `status=ok / end_flag / lines`.
- **The default AT path is now qmodem-atd (Rust)**: `at()` / `fastat()` use the
  qmodem-atd unix socket when `at_tool` is unset or `2`; `0` (tom_modem) and
  `1` (sms_tool_q) become legacy explicit opt-outs. Supporting changes:
  `/etc/config/qmodem` defaults to `at_tool='2'`; `/etc/config/qmodem-atd`
  ships with `enabled='1'`; `/etc/init.d/qmodem-atd` auto-collects `at_port`
  from the `modem-device` sections of `/etc/config/qmodem` when no
  `config port` section is declared (works out of the box) and adds `qmodem`
  to its reload triggers; a new `etc/uci-defaults/99-qmodem-atd-default`
  switches existing installs on upgrade as well (an explicitly set
  `at_tool='1'` is respected). Rolling back is one line:
  `uci set qmodem.main.at_tool='0' && uci commit qmodem`.

### Changed
- **Secondary-view pastel token sweep (Soft Anime visual refactor)**:
  all remaining hardcoded colors in `sms_conversation` / `sim_switch` /
  `monitor` / `ttl` / `modem_config` / `network_config` are now `--qm-*`
  design tokens — connection status dots (connected `--qm-ok`,
  disconnected `--qm-danger`, unknown `--qm-warn`, loading
  `--qm-text-muted`), RAT badges restyled as soft pills (NR `--qm-ok` /
  others `--qm-info`), the monitor security warning and the TTL
  important-notes block upgraded to `--qm-warn-soft` pastel cards, and
  the SMS char counter / meta chips tokenized. Every LuCI view now
  ships zero bare hex or named colors.
- **LuCI layout + visual upgrade**: unified page width across 6 views
  (`status` / `network` / `system` / `advanced` / `connection` / `device`
  each hard-coded `max-width:1160px`, `device` used `960px`) — they now all
  read `--qm-maxw` from theme.js, so one token controls every page.
  Removed 12 dead `bg-glow` rules (theme.js already neutralizes them with
  `display:none!important`) and the font stack duplicated 6× — it is now a
  single `--qm-font` token following the LuCI theme's `--font-family`.
  Upgraded theme.js components: sticky table headers in scrollable lists,
  zebra striping, stronger row-hover contrast, and pressed states for
  buttons. Net effect on the style sections: −1.4KB.
- **Runners pinned to `ubuntu-26.04`**: all 11 `runs-on` entries across the 8
  workflows used `ubuntu-latest`, which migrates to Ubuntu 26.04 from
  2026-10-19 (actions/runner-images#14748) — GitHub was flagging every run
  with that warning. Labels are now pinned explicitly to `ubuntu-26.04`,
  which both silences the warning and makes the image version deliberate
  rather than something that shifts under us (the OpenWrt SDK build depends
  on specific apt packages like `libncurses5-dev` / `gcc-multilib`).
  Bump the label deliberately when we are ready for the next image.
- **CI workflows consolidated (11 → 8)**: merged overlapping test/validation
  workflows into two, without weakening coverage:
  - `ci.yml` (new) replaces `quality.yml` + `vendor-fixtures.yml` +
    `rust-build.yml`. Three duplicated checks were removed in the process:
    the shell syntax check now runs once via `check_all.sh` (vendor-fixtures
    used to run it a second time with `--only shell`), `cargo test --workspace`
    + clippy now run once in `check_all.sh`'s rust stage (the old rust-build
    `test` job was byte-for-byte the same), and the js unit test runs once in
    the js-test stage.
  - `support-list.yml` (new) replaces `commit_check_supportlist.yaml` +
    `pr_check_supportlist.yaml`, which were the same script with different
    post-processing (auto-commit on push, PR comment on PR).
  Release chain (`main.yml` / `release.yml` / `tagger.yml` /
  `publish-existing-release.yml`) and `extract-i18n.yml` / `wiki.yml` are
  untouched. A tag push now starts 3 workflows instead of 5.
- **CI builds are now on-demand (no more build-per-push)**: `main.yml` listened
  to `push: branches: ['*']`, so every commit on any branch ran the full arm64
  matrix (≈15 min × 3). It is now split into two workflows:
  - `main.yml` runs only on **① `v*` tag push ② manual `workflow_dispatch`
    ③ pull requests to main** (docs/README/.github ignored) and owns the full
    build + release;
  - `tagger.yml` (new) runs only on **push to main/stable that touches
    `version.mk`**, and merely validates the version and pushes a tag (seconds,
    no compilation). The tag push then triggers `main.yml`, so the release path
    is unchanged.
  Routine code pushes no longer start any build. `rust-build.yml` follows the
  same policy (tags / manual / PRs touching Rust paths); only the lightweight
  Quality gate and Vendor fixture tests still run on pushes — static checks,
  not compilation.
- **Tab hierarchy refined**: the theme appends multiple `ul.tabs` rows inside
  `#tabmenu` (row 1 = top level, row 2 = children of the active entry). The
  second row is now visually subordinate — smaller type (12px) and padding
  (5px 12px), regular weight, muted color, and no duplicate divider — so the two
  rows read as a clear hierarchy.
- **Unified typography & layout across all 18 pages**: `theme.js` gains a
  `CSS_LAYOUT` block that folds the 331 inline CSS rules scattered over 7 views
  into one spec — heading scale (h1>h2>h3>h4, no leading top margin), paragraph
  and list rhythm, tables (header tint/weight, cell padding, row hover, no
  trailing border), form control height and help text, spacing between adjacent
  cards/grids, and long-text overflow handling.
- **Top tab bar reworked**: new `CSS_TABS` block styling the theme-rendered
  `#tabmenu > ul.tabs` (produced by `menu-aurora.js`): pill look, 6px gaps,
  single-row with horizontal scrolling instead of wrapping, hover state,
  keyboard focus ring, and an active state built from an accent-tinted
  background + accent border + the theme's high-contrast text color + bold
  weight (never the raw accent as text color, which fails AA on light-accent
  themes).
- **Terminology pass**: 29 new translations (Bridge Port, Direct modem access,
  SMS Storage, Sender, Result, Notice, Slot %s, Webhook URL …). Technical terms
  and brand names (APN, ARFCN, PCI, SCS, IPv4/IPv6, DNS, URL, USB, Webhook,
  PushDeer, ServerChan, carrier names) stay in their original form by rule.
- **Menu consolidation (16 top-level entries → 6)**: the top level is now
  `Overview / Mobile Data / Messages / Module and SIM / Network Monitor /
  Advanced`; the remaining pages moved into their owner page as sub-tabs
  (Dial and Network, Radio and Cells → Mobile Data; SIM Messages, SMS
  Forwarding → Messages; SIM Switch, Device Parameters → Module and SIM; AT
  Console, Statistics → Network Monitor; Modem Features, TTL, Service and Scan
  → Advanced). No page or feature was dropped — only the hierarchy changed.
  Hard-coded menu paths in `advanced.js`, `system.js` and `status.js` were
  updated accordingly.
- **Faster CI builds + localized workflow** (`main.yml` and friends):
  - the OpenWrt SDK cache key no longer contains `github.sha`, so the SDK and its
    `build_dir` are reused across pushes instead of being re-downloaded, re-fed
    and rebuilt on every single commit (the dominant cost before);
  - a `swatinem/rust-cache` step caches the cargo registry and `rust/target`, so
    only changed crates are recompiled;
  - `concurrency.cancel-in-progress` drops queued superseded runs so iterations
    no longer pile up behind slow builds.
- **CI workflows localized to Chinese**: job / step names and workflow titles are
  now Chinese (`自动生成版本标签`, `准备发布元数据`, `构建 QModem`, `发布产物`, …).
  Job ids, matrix names, artifact names and env file names are unchanged because
  they are referenced across jobs.
- **CI actions moved to the Node 24 runtime**, removing the
  "Node.js 20 is deprecated … being forced to run on Node.js 24" warning:
  `checkout` v4→v7, `upload-artifact` v4→v7, `download-artifact` v4→v8,
  `cache` v4→v6, `setup-node` v4→v7, `setup-python` v5→v7,
  `softprops/action-gh-release` v2.2.2/v2→v3.
- **CI build matrix narrowed to arm64**: `main.yml` now builds `arm64_ipk` /
  `arm64_apk` / `arm64_apk_immortalwrt` only. The `mmips_*` (mipsel) and `x64_*`
  targets were removed from the matrix. mipsel musl is a Rust tier-3 target that
  needs nightly + `-Z build-std`, and its std link step cannot resolve the musl
  startfiles / `-lunwind` through the OpenWrt SDK cross gcc (see the comments in
  `application/qmodem/build-rust.sh`); the tier-3 path remains implemented and is
  re-enabled once that link chain is fixed.
- **Unified frontend (visual / component / data layers)**: the LuCI adaptation from
  `luci-app-qmodem-generic` was ported wholesale into `luci-app-qmodem`, and the ten
  business views of `luci-app-qmodem-next` now go through the single entry point
  `qmodem.controls` instead of each carrying its own style layer and private component
  layer. The unified surface exports 110 symbols: the full original generic surface, the
  component layer (`ModemSelector` / `formatTime` / `relativeTime` / `pollGuard` /
  `hasCssClass`), the data layer (`controls.sms.*`) and legacy next aliases
  (`getStats` / `callGetSimSwitchCapabilities` / `rcStatus`). All 61 ubus endpoints used
  by next are preserved; the unified frontend resolves 79 endpoints in total. Coverage,
  module/call paths and the acceptance criteria for "fully compatible" are documented in
  [docs/refactor/journal-next-into-generic-compat.zh-cn.md](docs/refactor/journal-next-into-generic-compat.zh-cn.md).
- **Consolidated backend `qmodem`**: eleven previously separate packages
  (`libqmodem-sms`, `tom_modem`, `sms_tool_q`, `ubus-at-daemon`, `modem_scan`,
  `qmodem-smsd`, `qmodem-settings`, `qmodem_monitor`, `sms-forwarder-next`,
  `quectel-CM-5G-M`, `qmodem-seal`) are physically merged into `qmodem`, with sources
  under `src/<component>/` built in dependency order by a single `Build/Compile`.
  Installing is now `qmodem` + `luci-app-qmodem` (installing the frontend pulls the
  backend automatically). Scope, retained packages and the reasons are documented in
  [docs/refactor/integration-scope.zh-cn.md](docs/refactor/integration-scope.zh-cn.md).

### Removed
- Retired `luci-app-qmodem-next`, `luci-app-qmodem-monitor`, `luci-app-qmodem-ttlfw4`,
  `luci-app-qmodem-ttl` and `luci-app-qmodem-sms`: their views and services were merged
  into `luci-app-qmodem` (`luci-app-qmodem-ttl` was the legacy iptables Lua stack sharing
  `/etc/config/qmodem_ttl` with the fw4 variant — an outright conflict, hence retired).
- Retired the `qmodem-suite` meta-package: its only real job was to `enable` the backend
  daemons one by one, which now happens inside `qmodem` itself.
- Retired the legacy Lua stack `luci/luci-app-qmodem/luasrc/**` (controller / model /
  view). Its service scripts, `board.d` / `hotplug.d` hooks, vendor command library and
  rpcd plugins moved into `qmodem` at unchanged install paths.

### Fixed
- **CI: OpenWrt SDK prereq passes again on ubuntu-26.04 runners**: Ubuntu
  26.04 replaced GNU coreutils with the Rust-based uutils coreutils, whose
  `install --version` output no longer contains "GNU". That trips the
  OpenWrt SDK prerequisite check (include/prereq-build.mk probes
  `install --version | grep GNU` and then `ginstall --version | grep GNU`),
  so since the runner migrated to ubuntu-26.04 every build job failed at
  the "初始化 SDK 环境" step with `Build dependency: Please install GNU
  'install'`. main.yml now installs a `ginstall` wrapper (the second
  candidate OpenWrt probes) into `/usr/local/bin`: `--version` prints a
  GNU-style banner to satisfy the check, all other invocations forward to
  the system install (uutils supports the -d/-m/-D/-p/-t/-T options
  OpenWrt uses). The step self-tests first and skips the shim when the
  system install already reports GNU.
- **AT console unusable under the exclusive `qmodem-atd` mode**: device-level
  integration surfaced six stacked defects, all fixed —
  1. the `qmodem-atd` init script tested section type with
     `config_get "$section" type`, but in UCI `config modem-device '2_1'` the
     `modem-device` part is the **section type**, not an option, so the lookup is
     empty and no port was ever collected unless `config port` was declared
     explicitly — atd then did not start at all;
  2. the shipped `/etc/config/qmodem-atd` hard-coded an example port
     `/dev/ttyUSB2`, binding the daemon to the **wrong device** (this board's AT
     port is `/dev/ttyUSB1`); the shipped config no longer pre-declares a port
     (auto-collection is used) and non-existent tty nodes are skipped;
  3. `qmodem.send_at` did not resolve a port when the caller omitted it and ran
     `at "" <cmd>`, which can only fail (`status=0`, no `res`); it now falls back
     to `at_port`/`override_at_port` and returns an explicit error payload when
     the command or port is missing;
  4. the frontend sent `use_ubus` while the backend read `use_ubus_flag`, so the
     "use ubus" toggle was silently ignored; the backend accepts both names and
     the frontend now sends `use_ubus_flag`;
  5. the atd bridge helper's curl fallback used `-d`, which strips newlines from
     the body per urlencoded semantics, so the line-oriented atd never saw a
     complete request line; its completion test ("file ends with a newline") was
     also satisfied by the bad-json replies generated for curl's own HTTP header
     lines, aborting the transfer before the real reply arrived. It now uses
     `--data-binary @-` plus `-N` (unbuffered) and waits for an actual
     `{"status":...}` line; the busybox-incompatible `sleep 0.1` is gone and
     curl is used directly when busybox `nc` lacks `-U`;
  6. the Rust atd opened the tty with `B115200` mis-typed as `0o100002` instead
     of `0o10002` — an undefined speed constant, so after `tcsetattr` the port
     ran at a bogus rate and **no bytes were ever read**; fixed, and termios
     setup failures are now reported instead of silently yielding a port that
     never receives data.
- **Exclusivity broken by a legacy C component**: on the device, `ls -l
  /proc/*/fd` showed the legacy `ubus-at-daemon` holding `/dev/ttyUSB1` at the
  same time as `qmodem-atd`, stealing its replies (looking exactly like "AT
  timeout"). `qmodem-atd` now stops and disables the legacy `ubus-at-daemon`
  before starting, keeping a single owner of the AT port.
- **Traffic-reset schedule never saved (frontend/backend parameter mismatch)**:
  the frontend sent a nested `params` object while the backend
  `set_traffic_reset_schedule` expects flat fields (`enabled`/`reset_type`/
  `hour`/`day`), so the schedule was never written; the backend also lacked a
  `minute` field, making the minute picker impossible to persist. The frontend
  now maps to flat fields and the backend handles `minute` end-to-end (validated
  0–59, stored as `usage_reset_minute`, included in the cron line and response).
- **Second-level tabs were untranslated**: the menu titles `Dial and Network`,
  `Device Parameters`, `Modem Features`, `Service and Scan`, `SIM Messages` and
  `SMS Forwarding` had no po entries, so the second tab row rendered in English.
  All six are now translated (`TTL` stays as a term).
- **Light-background widgets inside hero rendered as blank blocks**: hero forces
  white text on every descendant, but `.mt-system-temp-box` (module-system
  temperature box, which gets a white background via CARD_CLASSES) and
  `.mtconn-state` (connection status badge, `background:var(--qm-glass)`) are
  light — i.e. white on white (confirmed on device). Both are now darkened inside
  hero and their text (temperature value, description, badge label) forced white.
- **White-on-white hero chips and button**: the hero rule
  `* { color:#fff !important }` forces white text on every descendant, while
  `.qm-hero-op` (operator), `.qm-pill-tag` (mode / interface) and
  `.qm-refresh-btn` (refresh) use white or light backgrounds inherited from the
  theme — i.e. white on white (confirmed on device). They now use a translucent
  dark background with white text, plus explicit hover (darker) and disabled
  (dimmed but still ≥4.5) states: white text contrast 8.32–17.94, all AA. <br>`
- **"Network Monitor" page and title untranslated**: 61 strings on the monitor
  page had no Chinese translation (some were not even wrapped in `_()`), and the
  menu title `Network Monitor` had no po entry at all. 17 hard-coded English
  literals are now wrapped in `_()` (placeholders, sample values and AT command
  examples untouched) and 64 po entries were added (13 in the template). `URL` is
  intentionally kept as-is per terminology rules.
- **Hero text unreadable on light-primary themes**: the hero gradient ended in
  the raw theme primary (`color-mix(primary 72%,#060b14) → var(--qm-accent)`, same
  in the dark fallback) while hero text is forced white. With a light primary
  (e.g. luci-theme-aurora) the gradient end became light blue and white text
  scored only 1.51–4.44 (≈2.53 for aurora), all below WCAG AA. Both stops are now
  darkened (`color-mix(primary 24%/42%,#05080f)`, fallbacks `#0f172a→#1e40af` and
  `#0b1220→#14315c`) with the primary used only as a hue source: white text now
  scores 4.96–16.02 (still 4.96 even for a pure-white primary). Hero chips switched
  from translucent white to translucent dark (white text 4.27 → 5.9+ on light
  themes) and hero sub/meta text dropped to 90% white for hierarchy.
- **`qmodem-atd` crash loop on device**: the package was built with default
  features only (`service`), but `--port` requires the `tty` feature — without it
  the daemon exits immediately ("--port requires building with --features tty"),
  so procd respawned it forever, the socket never appeared and every AT request
  on the default path failed. The build now passes `--features service,tty`.
- **LuCI views stuck at "Loading view" with `factory yields invalid constructor`**:
  `qmodem/compat.js` is a side-effect-only module that ended with `return {}`, but
  LuCI's `require()` validates the factory result as a class
  (`luci.js`: `if (!Class.isSubclass(_class))`). Plain objects trip that check and
  every view requiring the module failed to load. It now returns
  `L.Class.extend({})`.
- **`mmips` (mipsel) builds failed with an empty Rust target triple**: `$(ARCH)` on
  MIPS platforms is `mipsel_24kc` (CPU/ISA suffix mandatory), so the exact-match
  `ifeq ($(ARCH),mipsel)` never fired and `Build/CompileRust` aborted before cargo
  ran (with OpenWrt swallowing the diagnostic). The triple lookup now uses
  `$(filter)` prefix matching for `aarch64*` / `mipsel*` / `mips*` / `x86_64*`.
- **`mmips` (mipsel) build-std link failure**: under `-Zbuild-std` the linker driver
  could not resolve the musl startfiles (`crt1.o` etc.) or `-lunwind` (build-std
  artifacts are hash-named, so `-lunwind` has nothing to bind to). `build-rust.sh`
  now appends the cross toolchain's real sysroot as the last `--sysroot` link arg
  and provides an empty `libunwind.a` shim (valid because std is built with
  `panic_abort`, which needs no runtime unwinder).
- **Silent package build failures in CI**: the `Compile QModem` step now retries the
  failing package with `make ... V=s -j1`, so the real error output (e.g. cargo
  cross-compile logs that OpenWrt hides by default) lands directly in the job log.
- **Removed a cross-package file conflict**: both `qmodem` and `luci-app-qmodem` used to
  install `/usr/share/rpcd/acl.d/luci-app-qmodem.json`. It is now provided solely by the
  frontend package (which absorbed the `luci-mod-status-index → modem_ctrl` grant that
  used to live in the backend ACL), and the backend installs only `qmodem.json`.
- **Release assets are now directly installable** (`main.yml`,
  `publish-existing-release.yml`): artifacts are no longer repacked into
  `QModem-<arch>_*.tar.gz`; the Release ships loose `.ipk` / `.apk` files
  instead. Per-arch `apk` asset names carry an arch suffix
  (e.g. `qmodem-3.4.0_rc3-r1_aarch64.apk`) because OpenWrt apk filenames do
  not encode the target arch. Download and install directly —
  `opkg install *.ipk` / `apk add --allow-untrusted *.apk` — no extraction.
  The v1.0.0 Release was migrated accordingly (6 tar.gz assets replaced by
  80 loose packages).
- **Added `.gitattributes` to pin the line-ending policy**: both the repository and
  the `crlf` stage of `scripts/check_all.sh` assume LF everywhere, but no
  `.gitattributes` existed. With `core.autocrlf=true` on Windows the working tree is
  written as CRLF on checkout, so the local `crlf` gate reported many false positives
  unrelated to the committed content (the index was LF all along). The file now
  declares `* text=auto eol=lf`, and third-party/upstream vendored trees
  (`application/qfirehose/`, `driver/`, `application/qmodem/src/quectel_CM_5G_M/`,
  `application/qmodem/src/sms-tool_q/`) are marked `-text` to keep their bytes
  untouched, matching the exclusions in the `crlf` stage.
- **Fixed stale path references missed in the previous consolidation pass**: the gate
  command blocks in `docs/refactor/HANDOFF.md` and `docs/refactor/README.md` still
  pointed at the retired `luci/luci-app-qmodem-next/` and
  `application/ubus_at_daemon/{inc,src}` for the JS smoke test and the C syntax
  check; the async-daemon tour and the upstream-sync table under `docs/developer/`
  likewise referenced pre-merge directories. All now use the post-merge paths
  (`luci/luci-app-qmodem/tests/`, `application/qmodem/src/ubus_at_daemon/`,
  `application/qmodem/files/`).
- **Fixed the `crlf` gate's criterion and implementation**: the previous implementation
  scanned working-tree files with `grep -I -q -l $'\r'`, which failed in both directions —
  on Windows `core.autocrlf=true` checks files out as CRLF, so every file matched
  regardless of the committed content (false positives); and MSYS/Cygwin `grep` reads
  files in text mode and discards CR, so files that do contain CR were reported as clean
  (a file with 36 CRs was measured as not matching — false negatives). The stage now
  decides from the `i/<eol>` field of `git ls-files --eol`, i.e. the content about to be
  committed, which is deterministic and platform-independent; it also drops from
  minutes (a full scan of 688 files took ~3.7 min) to about 2 seconds. Validated
  negatively by injecting a CRLF blob into the index, confirming it FAILs and names
  the file.
- **The `rust` and `c` stages now recognise "not applicable on this host"**: the `rust`
  stage runs `cargo test --workspace`, but `qmodem-atd` depends on `std::os::unix`
  (unix sockets and serial ports) and cannot compile on a Windows host; the `c` stage
  fails under MinGW because the UCRT does not provide `timegm` / `strsep` and similar
  POSIX symbols, producing implicit-declaration errors. Both are host differences, not
  code defects, so they now degrade to SKIP with the reason stated, under strict
  conditions (`rust`: only when the sole failing crate is `qmodem-atd` and the log
  really contains that platform error; `c`: only when recompiling with a header that
  merely declares those POSIX functions succeeds, so genuine syntax errors still
  FAIL). CI (ubuntu-latest) still runs both stages.
- **Restored 38 executable bits lost during the consolidation**: this machine has
  `core.fileMode=false` (the Windows default), so git does not track the working-tree
  x bit and every file added or rewritten this round entered the index as `100644`.
  37 of them are runtime scripts installed via `$(INSTALL_BIN)` and required to land
  as `0755` (`/etc/init.d/*`, `/etc/uci-defaults/*`, `/usr/bin/*`, `/usr/sbin/*`,
  `/usr/libexec/rpcd/*`, `/usr/lib/qmodem/*`, the netifd proto scripts, the
  `sms_forwarder` notifier scripts, …); the last one is
  `tests/qmodem-seal/test_qmodem_seal.sh`, which was already `100755` before the
  merge — byte-identical in content, with only the mode lost — and is invoked
  directly as an executable by `vendor-fixtures.yml`, so losing the mode made that
  workflow fail with `Permission denied` (exit 126) on its third line. All 38 files
  are now restored via `git update-index --chmod=+x`; the index modes have been
  re-verified as `100755` and `git diff --cached` confirms mode-only changes with no
  content edits.
  The same audit also surfaced `files/usr/share/qmodem/ref-rmnet-dhcp` (formerly
  `application/quectel_CM_5G_M/files/dhcp`), which was `100755` in the old tree.
  It is a dnsmasq config reference (an `/etc/config/dhcp`-style file) and the old
  `quectel_CM_5G_M/Makefile` never installed it at all; this round installs it with
  `$(INSTALL_DATA)` (landing as `0644`), so it is **deliberately kept at `100644`** —
  the old x bit was an upstream leftover and restoring it would contradict the
  install semantics.

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
