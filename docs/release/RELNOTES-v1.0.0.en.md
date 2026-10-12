# QModem v1.0.0 Release Notes (English)

- Release date: 2026-10-12
- Release type: Release Candidate (pre-release, main branch)
- Baseline: `main@c49654e`; full change list in
  [CHANGELOG.md](../../CHANGELOG.md) / [CHANGELOG.zh-CN.md](../../CHANGELOG.zh-CN.md)

## Three main lines of this rc1

### 1. Stop-the-bleed hardening (audit-driven security & memory-safety fixes)

A systematic hardening pass across C memory safety, shell injection surfaces,
packaging and LuCI (27+ files):

- **Remote (modem-controlled) input no longer reaches root shells**:
  `ubus_at_daemon` event callbacks now use `execl` positional arguments;
  `modem_dial.sh` shell-quotes APN credentials; legacy LuCI controller command
  injection and SMS XSS fixed; `modem_scand` control socket `0666` → `0660`
  plus whitelists.
- **Memory safety**: fixes for use-after-free, self-deadlock,
  out-of-bounds read/write, unbounded `sprintf`, UCS2→UTF-8 conversion
  overrun, and more; `-Wall -Wextra` clean.
- **Correctness**: SMS SCTS timezone bug fixed (+5:30/+8:00/±12:45 were all
  wrong); rpcd `delete_sms`/`send_raw_pdu`/`send_sms` no longer silently
  return empty.

### 2. Rust zero-dependency async core

- New `rust/` tree: `qmodem-pdu` (3GPP TS 23.038 SMS PDU codec, bounds-checked,
  64 conformance tests) and `qmodem-atd` (thread-actor AT daemon: per-port
  command queue, monotonic timeouts, URC broadcast, unix-socket JSON test API).
- **Zero external crates**: binary size 43.4% smaller than the earlier tokio
  build; micro-benchmarks p50 50ms roundtrip, 31.4k cmd/s.
- New AT path `qmodem.main.at_tool=2` routes AT commands through
  `qmodem-atd` (see [AT tool selection](../developer/at-tool-selection.zh-cn.md)).

### 3. Single-package delivery

- New `qmodem-suite` meta package: one install delivers the full stack
  (`qmodem` + `luci-app-qmodem-next` + `qmodem-smsd` + `qmodem_monitor` +
  `luci-app-qmodem-monitor`), with a uci-defaults bootstrap that enables all
  services on first install.
- Fixed a legacy defect: `qmodem` previously shipped without installing
  5 init.d scripts.
- Install guide: [install-suite.zh-cn.md](../user/install-suite.zh-cn.md).

## Upgrade notes

1. **`qmodem` now installs init.d scripts and enables the service.** This
   fixes the previous "package installed but services never started" gap.
   When upgrading from an older version, the upgrade flow automatically
   enables the qmodem services; if you relied on manual start/stop, check
   the enabled state of `/etc/init.d/qmodem*` after upgrading.
2. **`at_tool=2` is an optional new path; defaults are unchanged.** The
   default AT path still uses the legacy implementation; only setting
   `qmodem.main.at_tool=2` explicitly switches to the Rust `qmodem-atd`
   unix-socket JSON protocol. During rc1, try it on non-critical devices first.
3. **TTL is provided by `ttlfw4`.** TTL functionality has no standalone
   counterpart in the next stack yet; keep using `luci-app-qmodem-ttlfw4`
   (a merge is planned for a later release).
4. Binary/firmware artifacts: see the appendix on the Release page
   (built by main.yml SDK build and rust-build.yml cross-builds; artifacts
   are not attached to the Release itself).

## Known limitations

- **qfirehose has 3 upstream high-risk findings pending patch/isolation**:
  `system()` command injection via upgrade package paths, XML-driven buffer
  overflow, and unauthenticated TCP→USB passthrough. See
  [docs/security/vendored-audit.md](../security/vendored-audit.md).
  Until patched, do not flash untrusted firmware packages via qfirehose.
- **ubus FFI (`ubus_ffi` + libubus) needs SDK on-device verification**: the
  pure-Rust blobmsg codec passes byte-compatibility tests, but the libubus
  FFI bindings have not been verified on devices built from a real OpenWrt
  SDK; the `ubus` feature of `at_tool=2` is off by default.
- Legacy-stack retirement Step 2 (equivalent `get_modem_cfg` endpoints and
  sub-package DEPENDS switch) and Phase 4 legacy removal belong to v1.2.0;
  this rc1 keeps both stacks co-installable.
