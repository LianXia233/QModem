# QModem

[简体中文](README.md) | English

[![OpenWrt SDK build](https://github.com/FUjr/modem_feeds/actions/workflows/main.yml/badge.svg)](https://github.com/FUjr/modem_feeds/actions/workflows/main.yml)

QModem is a cellular modem management and dialing stack for OpenWrt and ImmortalWrt. It discovers USB and PCIe modems, manages AT and data interfaces, and provides dialing, status, SMS, and vendor-specific controls through LuCI.

It is intended for custom 4G/5G routers, embedded gateways, and multi-modem devices. It is not a desktop ModemManager replacement and cannot compensate for kernel drivers missing from the target firmware.

## Capabilities

| Area | Scope |
| --- | --- |
| Discovery | USB and PCIe modems with physical-slot binding |
| Data sessions | QMI, MBIM, NCM, ECM, RNDIS, and others depending on modem and driver |
| Status | SIM, registration, signal, cell, and firmware information |
| Advanced controls | RAT preference, band/cell locking, SIM switching, depending on vendor and model |
| SMS | Send, receive, PDU, and optional forwarding; long-SMS behavior varies by modem |
| OpenWrt integration | UCI, ubus, rpcd, procd, and LuCI |

An entry in the [support list](docs/support_list.md) means that a matching project profile exists. It does not mean every firmware, interface composition, or advanced operation has been verified on hardware by the maintainers.

## 🌸 Soft Anime UI & Modern Dashboard (LuCI Next)

The entire QModem LuCI suite has undergone a full **Soft Anime × Pastel Glass × Modern Dashboard** visual refactoring:

| Module | Route | Highlights & Features |
|---|---|---|
| **Overview Dashboard** | `/qmodem/overview` | Animated cellular-wave SVG antenna & mascot, 4 live KPI metric cards (Signal/RAT, Carrier/Band, Session/IP, SIM Slot), smooth skeleton loading & async hydration |
| **Network Config** | `/qmodem/network_config` | Animated dual-antenna 5G router SVG hero, resilient timeout-bounded status polling, pulsing network service chips |
| **AT Terminal** | `/qmodem/debug` | Cyber Terminal console, segmented preset tabs (**reduces vertical footprint by 70%+**), 1-click collapse, command history with `↑`/`↓` keys |
| **Advanced Config** | `/qmodem/config_advanced` | Tuning gear animated SVG, per-device toolbar card, responsive band/RAT grid panels |
| **SMS Management** | `/qmodem/sms` | Glowing chat bubble SVG hero, thread layout, storage selector, pink unread indicators |
| **Core Settings** | `/qmodem/settings` | Microchip & PCB trace SVG hero, parallel non-blocking RPC fetching |
| **Health Monitor** | `/qmodem/monitor` | Radar sweep animated SVG, cardified multi-WAN watchdog |
| **TTL Rules** | `/qmodem/ttl` | Packet flow shield SVG hero, 1-click bypass presets (64 Linux/Android, 128 Windows, 65 Bypass) |
| **SIM Switch** | `/qmodem/sim_switch` | Dual-SIM chip animated SVG, interactive Slot 1 & Slot 2 cards with active RF state indicators |
| **SMS Forwarding** | `/qmodem/sms_forward` | Paper-airplane messenger SVG hero, Telegram/Webhook/Feishu/ServerChan/PushDeer push channels |
| **SIM Storage** | `/qmodem/sms_sim` | Micro-SIM memory SVG hero, modern cardified batch action table |

- **🎨 Unified Design Tokens**: Flawless light & dark theme adaptation with Lavender Purple (`#8875E8`), Sakura Pink (`#EFA8CB`), Mint Blue (`#8CCFE2`), and Mint Green (`#79CDB0`).
- **✨ Zero External Dependencies**: **100% offline-ready**, zero external CDNs or web fonts; all illustrations are pure inline animated SVGs.
- **🪟 Unified Pastel Glass Modals**: All `ui.showModal` dialogs feature `14px` frosted glass blur, `20px` rounded cards, top pastel gradient accents, and macaron pill buttons.
- **📱 Responsive Layout**: Seamless scaling from high-density desktop dashboards to single-hand mobile views (360px~430px).

## Architecture

```text
LuCI
  |
rpcd / ubus
  |
QModem control and dialer ---- modem_scand discovery daemon
  |
generic + vendor adapters
  |
shared AT command wrappers
  |
ubus-at-daemon / tom_modem
  |
USB / PCIe cellular modem
```

The core package is `qmodem` — an all-in-one backend that embeds what used to be eleven separate packages (`libqmodem-sms`, `tom_modem`, `sms_tool_q`, `ubus-at-daemon`, `modem_scan`, `qmodem-smsd`, `qmodem-settings`, `qmodem_monitor`, `sms-forwarder-next`, `quectel-CM-5G-M`, `qmodem-seal`). The unified pure-JavaScript frontend is `luci-app-qmodem`, which supersedes both the legacy Lua `luci-app-qmodem` UI and `luci-app-qmodem-next`. Installing `luci-app-qmodem` pulls `qmodem` in automatically. Platform-specific extras (`luci-app-qmodem-hc`, `luci-app-qmodem-mwan`) and the experimental voice stack stay separate. See [integration scope](docs/refactor/integration-scope.zh-cn.md).

## Retired legacy packages

Since 2026-10 the following **legacy Lua UI packages have been retired** (their backend daemons and scripts are now incorporated into the all-in-one `qmodem` package); the entire web UI is unified on modern JavaScript:

| Retired Package | Purpose | Modern Replacement |
|---|---|---|
| `luci-app-qmodem` (Lua) | Legacy LuCI UI | `luci-app-qmodem-next` (Soft Anime modern UI) |
| `luci-app-qmodem-hc` | HC-G80 SIM switch | `luci-app-qmodem-next` (SIM switch view) + `qmodem` built-in script |
| `luci-app-qmodem-sms` | Legacy SMS forwarding | Built-in SMS suite in `luci-app-qmodem-next` |
| `luci-app-qmodem-ttl` | Legacy TTL configuration | `luci-app-qmodem-ttlfw4` (Soft Anime modern UI) |
| `luci-app-qmodem-mwan` | Multi-WAN configuration | `qmodem` built-in multi-WAN rules + official `luci-app-mwan3` |

All legacy Lua UI packages are retired, resolving all menu conflicts with `luci-app-qmodem-next`. For all deployments use `luci-app-qmodem-next` and `luci-app-qmodem-ttlfw4` directly.

## Quick start

Add the feed to `feeds.conf.default` in an OpenWrt source tree:

```text
src-git qmodem https://github.com/FUjr/QModem.git;main
```

Then run:

```sh
./scripts/feeds update qmodem
./scripts/feeds install -a -p qmodem
make menuconfig
```

Select `luci-app-qmodem` under `LuCI -> Applications`. The target firmware must also contain USB, serial, QMI, MBIM, or MHI drivers matching the modem, protocol, kernel, and hardware topology.

The detailed English manuals are currently maintained in the existing [user guide](docs/user-guide.md) and [developer guide](docs/developer-guide.md). The new structured documentation is being introduced in Chinese first (three documents are also available in English):

- [User documentation](docs/user/index.zh-cn.md)
- [Developer documentation](docs/developer/index.zh-cn.md)
- [Install guide (qmodem + luci-app-qmodem)](docs/user/install-suite.zh-cn.md) ([English](docs/user/install-suite.en.md))
- [Rust async daemon architecture](docs/developer/async-daemon.zh-cn.md) ([English](docs/developer/async-daemon.en.md))
- [AT tool selection](docs/developer/at-tool-selection.zh-cn.md) ([English](docs/developer/at-tool-selection.en.md))
- [Changelog](CHANGELOG.md) ([简体中文](CHANGELOG.zh-CN.md)) - [Branch and release policy](docs/release/branch-policy.zh-cn.md)
- [Version migration and issue reporting](docs/release/version-migration.zh-cn.md)
- [Supported hardware](docs/support_list.md)
- [rpcd API](docs/qmodem-rpcd-interface.md)
- [AT fixture development](testcases/README.md)

## Rust async daemon (experimental)

`rust/qmodem-atd` is a zero-dependency rewrite of the C `ubus_at_daemon` (no third-party crates in `Cargo.lock`): one std-thread actor per port with an mpsc command queue, monotonic timeouts, URC broadcast, and a Unix-socket JSON test protocol. The release binary is about 731 KiB unstripped (-43.4% versus the earlier tokio build). It ships as an experimental parallel option and does not replace any packaged component yet. See the [Rust async daemon architecture](docs/developer/async-daemon.zh-cn.md) and the [Rust workspace README](rust/README.md).

The AT command path is selectable via `qmodem.main.at_tool`: `0` tom_modem/ubus at-daemon, `1` sms_tool_q direct, or `2` (new in this version) qmodem-atd over a Unix socket. See [AT tool selection (Chinese)](docs/developer/at-tool-selection.zh-cn.md).

## Install

The stack is two packages: `qmodem` (backend, built for the target architecture) and
`luci-app-qmodem` (frontend, `all`). One command is enough — the frontend depends on the
backend:

```sh
opkg update && opkg install luci-app-qmodem    # opkg
apk add luci-app-qmodem                        # apk (ImmortalWrt 25.12+ / OpenWrt snapshot)
```

`qmodem`'s `postinst` and its `uci-defaults` hook enable the backend daemons, fix script
permissions and restart `rpcd`, so the stack works right after install. The guide also
covers post-install service checks, minimal UCI configuration, and troubleshooting: see
the [install guide](docs/user/install-suite.zh-cn.md). The former `qmodem-suite`
meta-package was retired — see
[integration scope](docs/refactor/integration-scope.zh-cn.md).

## Important boundaries

- Do not let multiple management applications read and write the same AT port concurrently.
- Band locking, cell locking, IMEI changes, and modem resets can disrupt service; confirm model support and recovery steps first.
- Kernel packages from a release must match the target firmware ABI. Do not treat `--force-depends` as a compatibility solution.
- A successful package build does not prove that AT operations or data sessions work on a specific modem.

## Reporting issues

Include the router model, firmware and QModem versions, exact modem model, USB/PCIe topology, dialing protocol, and reproducible steps. Redact IMEI, IMSI, ICCID, phone numbers, APNs, credentials, and SMS content before publishing logs or fixtures.

## Contributing

AT commands used by vendor and dialing implementations must be defined through `cmd_*` wrappers under `application/qmodem/files/usr/share/qmodem/cmds/`. CI enforces this boundary. Real-device, redacted fixtures are preferred over screenshots or model-name-only support claims.

## License

The repository [LICENSE](LICENSE) contains the Mozilla Public License 2.0 text plus an additional restriction prohibiting commercial use. It is therefore not an unmodified standard MPL 2.0 grant. Read the complete license before using, distributing, or integrating the project.

## Acknowledgments

QModem uses or builds on work from the 5G-Modem-Support, luci-app-4gmodem, sms_tool, gl-modem-at, sendat, and qosmio/nss-packages communities. Consult the files and Git history in each component for exact provenance and licensing.
