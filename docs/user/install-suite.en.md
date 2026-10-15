# QModem installation guide

[简体中文](install-suite.zh-cn.md) | English

> Status: **two-package layout**. After the component consolidation the whole stack is just
> `qmodem` (backend, built for the target architecture) and `luci-app-qmodem-next` (frontend,
> `all`). The former `qmodem-suite` meta-package and the standalone packages it pulled in
> are retired, and the legacy Lua `luci-app-qmodem` UI has been deleted from the repository
> (never built, never released); see the
> [integration scope](../refactor/integration-scope.zh-cn.md) for the reasons and the
> replacement mapping.

## 1. Packages

| Package | Arch | Role | Required |
| --- | --- | --- | --- |
| `qmodem` | target | All-in-one backend: modem discovery and dialing scripts, ubus data services, SMS daemon, traffic statistics, network monitor, SMS forwarding, all CLI tools | yes |
| `luci-app-qmodem-next` | `all` | The single frontend (pure JavaScript); all data is read through the `qmodem` ubus object | recommended |
| `luci-i18n-qmodem-next-zh-cn` | `all` | Simplified Chinese translations | optional |

`luci-app-qmodem-next` declares `DEPENDS:=+qmodem`, so **installing the frontend is enough** —
the backend is pulled in automatically.

> `qmodem` now embeds what used to be eleven separate packages: `libqmodem-sms`,
> `tom_modem`, `sms_tool_q`, `ubus-at-daemon`, `modem_scan`, `qmodem-smsd`,
> `qmodem-settings`, `qmodem_monitor`, `sms-forwarder-next`, `quectel-CM-5G-M` and
> `qmodem-seal`. Those names are kept as `PKG_PROVIDES` so existing dependencies still
> resolve, and they are listed in `PKG_CONFLICTS` so the old and new packages cannot be
> installed side by side.

## 2. Installation

### 2.1 From a package feed (recommended)

```sh
# opkg (OpenWrt 23.05 and older)
opkg update
opkg install luci-app-qmodem-next luci-i18n-qmodem-next-zh-cn

# apk (OpenWrt 24.10 / ImmortalWrt 25.12 and newer)
apk update
apk add luci-app-qmodem-next luci-i18n-qmodem-next-zh-cn
```

`opkg install qmodem luci-app-qmodem-next` is equivalent if you want to pin the backend
explicitly.

> Compatibility baseline: OpenWrt / ImmortalWrt **25.12 and newer** (24.10 and older are no
> longer supported). The baseline is enforced by the CI build matrix and the documentation;
> the package metadata carries no version-constraint expression because the OpenWrt
> buildroot `DEPENDS` syntax does not support `(>= version)`.

### 2.2 Directly from the GitHub Release (no feed available)

If the device has no QModem feed, download the **loose ipk/apk files** (no extraction
needed) from [Releases](https://github.com/LianXia233/QModem/releases). Asset names carry an
arch suffix — pick the one matching your device:

| Asset suffix | Arch | Typical device |
|---|---|---|
| `_aarch64_cortex-a53` (ipk) / `_aarch64` (apk) | ARM 64-bit | Airpi AP3000M and other Cortex-A53 boards |
| `_x86_64` | x86-64 | x64 soft routers / VMs |
| `_mipsel_24kc` | MIPS little-endian 24Kc | MediaTek MT76xx / MT8621 |
| `_all` | arch-independent | LuCI apps and i18n — install once, works everywhere |

```sh
# OpenWrt (aarch64 shown; apk filenames drop the ipk `_cortex-a53` granularity)
cd /tmp
opkg install qmodem_*_aarch64_cortex-a53.ipk
opkg install luci-app-qmodem-next_*_all.ipk
opkg install luci-i18n-qmodem-next-zh-cn_*_all.ipk

# ImmortalWrt (apk)
apk add --allow-untrusted qmodem-*_aarch64.apk luci-app-qmodem-next-*.apk
```

> Assets built natively for ImmortalWrt carry an `-immortalwrt` suffix. Do not mix them
> with OpenWrt assets — the two have different ubus ABIs.

### 2.3 TTL support

TTL management (fw4/nft backend) is part of the unified packages: the service
`/etc/init.d/qmodem_ttl` and the default config `/etc/config/qmodem_ttl` ship with
`qmodem`, and the UI is the **TTL** page in `luci-app-qmodem-next`.

- **Disabled by default**: TTL rewriting disables hardware flow offloading, so neither the
  `qmodem` uci-defaults hook nor its `postinst` enables `qmodem_ttl`. Turn it on explicitly
  in LuCI when you need it.
- **Mutually exclusive with the legacy iptables variant**: the old `luci-app-qmodem-ttl`
  (iptables stack) shares `/etc/config/qmodem_ttl` with the fw4 variant — a real conflict —
  and has been retired. If the old package is still installed, run
  `opkg remove luci-app-qmodem-ttl` (or `apk del luci-app-qmodem-ttl`) first.

### 2.4 Voice (voip) is a separate install

The experimental voice stack (`qmodem-voip` + `qmodem-sipd` + `luci-app-qmodem-voip`) is
**not** part of the default install: it pulls in heavy libraries (`libpjsip`, `libsoxr`,
`alsa-lib`) and is limited to a hardware allow-list (currently RM520N-GL only). Install it
separately if needed:

```sh
opkg install luci-app-qmodem-voip   # or apk add luci-app-qmodem-voip
```

This frontend pulls in `qmodem-voip` (call-control daemon voipd) and `qmodem-sipd` (SIP
signalling gateway). See
[docs/refactor/journal-retire-voip.md](../refactor/journal-retire-voip.md) for why it is
kept separate rather than merged.

## 3. Post-install verification

### 3.1 Service status

`qmodem`'s `postinst` (package-manager installs) and
`/etc/uci-defaults/99-qmodem-worker-bootstrap` (firmware images / first boot) fix script
permissions, enable the daemons and restart `rpcd`. Confirm with:

```sh
for s in qmodem_init qmodem_network qmodem_led qmodem_reboot qmodem_usage_stats \
         qmodem-smsd qmodem-settings qmodem_monitor qmodem-worker qmodem-stats-collect \
         ubus-at-daemon; do
	printf '%-24s %s\n' "$s" "$(/etc/init.d/$s enabled && echo enabled || echo DISABLED)"
done

/etc/init.d/qmodem_init status            # discovery + init
/etc/init.d/qmodem_network status         # dialing
/etc/init.d/qmodem_led status             # status LED
/etc/init.d/qmodem_reboot status          # scheduled reboot
/etc/init.d/qmodem_usage_stats status     # traffic statistics persistence
/etc/init.d/qmodem-smsd status            # SMS daemon
/etc/init.d/ubus-at-daemon status         # serial AT channel daemon
/etc/init.d/qmodem-worker status          # status cache collector
/etc/init.d/qmodem-stats-collect status   # traffic sampling
```

### 3.2 ubus objects

`qmodem`, `qmodem_cache`, `qmodem_stats`, `qmodem_support` and `qos` are provided by this
package and only register after `/etc/init.d/rpcd` restarts once (handled by `postinst`):

```sh
ubus list | grep -E 'qmodem|qos'

ubus call qmodem base_info '{"config_section":"modem1"}'
ubus call qmodem network_info '{"config_section":"modem1"}'
```

The full method table is in the [rpcd interface](../qmodem-rpcd-interface.zh-cn.md) (Chinese).

### 3.3 LuCI menu

With `luci-app-qmodem-next` installed, a dedicated menu appears at **Modem → QModem → Overview**
(route `admin/modem/qmodem/overview`). There are 18 pages: Overview, Statistics, Mobile
Data, Radio and Cells, Dial and Network, Messages, SIM Messages, SMS Forwarding, Module and
SIM, SIM Switch, AT Console, Network Monitor, Advanced, Modem Features, TTL, Device
Parameters, Service and Scan, plus the SMS conversation page (not in the menu — reached
from Messages).

If the menu is missing, check that the browser account has the `luci-app-qmodem-next` ACL, then
log in again; if necessary `rm -rf /tmp/luci-*cache` and hard-refresh the browser.

## 4. Minimal configuration

QModem discovers modems automatically; in most cases a reboot is enough for LuCI to show
the modem. To pin a configuration manually, add a minimal `modem-device` section to
`/etc/config/qmodem`:

```uci
config modem-device 'modem1'
    option enabled '1'
    option data_interface 'usb'
    option slot '/sys/bus/usb/devices/2-1.2'
    option at_port '/dev/ttyUSB2'
    option manufacturer 'Quectel'
    option enable_dial '1'
    option pdp_type 'IPv4/IPv6'
    option apn ''          # empty = auto-detect from the carrier
```

See the “modem device configuration” section of the
[full user guide](../user-guide.md) for the field reference and further options (band
locking, SIM switching, custom init commands, ...). After editing:

```sh
/etc/init.d/qmodem_init restart
```

## 5. Troubleshooting quick reference

| Symptom | Command / location |
|---|---|
| Service logs | `logread -t qmodem` (or `logread -e qmodem` for all QModem entries) |
| Scanner logs | `logread -e modem_scand` |
| Dial log | LuCI: Modem → QModem → Overview dial-log panel; or `ubus call qmodem get_dial_log '{"config_section":"modem1"}'` |
| Kernel / driver | `logread -e usb -e mhi -e tty -e qmi -e mbim` |
| Port enumeration | `ubus call qmodem get_tty_ports '{}'` |
| UI reports “modem support library not detected” | `ls -l /usr/sbin/qmodem-modem-support /usr/libexec/rpcd/qmodem_support` should be `755`; then run `/usr/sbin/qmodem-modem-support` once by hand |

Logs and AT responses may contain IMEI, IMSI, ICCID, phone numbers, APNs or SMS bodies —
redact before publishing. See [troubleshooting](troubleshooting.zh-cn.md) (Chinese) for the
full flow.

## 6. AT path selection

Which low-level path AT commands take is decided globally by `qmodem.main.at_tool`:

| Value | Path | Notes |
| --- | --- | --- |
| `0` / unset | `tom_modem` | default; can be serialized through `ubus-at-daemon` |
| `1` | `sms_tool_q` | direct serial |
| `2` | `qmodem-atd` | unix socket (Rust async daemon; deploy manually) |

See [AT tool selection](../developer/at-tool-selection.en.md) for the trade-offs, timeout
semantics, failover and diagnostics. A fresh install uses path 0 with no extra
configuration; path 2 needs the Rust component from the next section.

## 7. Optional Rust component (qmodem-atd)

The default install does not include the Rust async AT daemon `qmodem-atd`. To use path 2,
cross-compile the binary and ship it with the firmware:

1. **Cross-compile** with the unified entry script `scripts/cross-build-rust.sh`:

   ```sh
   bash scripts/cross-build-rust.sh --target aarch64-unknown-linux-musl
   # output: scripts/dist/aarch64-unknown-linux-musl/qmodem-atd
   ```

   The result is a fully static musl binary (zero external crates). Confirm with `file`
   that it is `statically linked`, then copy it to the target. See section 4 of the
   [Rust async daemon architecture](../developer/async-daemon.en.md) for build details and
   the equivalent manual commands.

2. **Deploy**: put the binary in the device PATH (e.g. `/usr/bin/qmodem-atd`) and start it
   with the socket path and port described in section 3 of the async-daemon document. The
   default socket path matches `qmodem.main.at_socket`.

3. **Switch**: set `at_tool` to `2` and restart `qmodem_init`. If the socket is
   unreachable, QModem falls back to the `tom_modem` path automatically.

## 8. Upgrading from an older layout

Older releases installed `qmodem` alongside a set of standalone packages (`qmodem-smsd`,
`qmodem_monitor`, `tom_modem`, `ubus-at-daemon`, `sms-forwarder-next`, ...) and used
`luci-app-qmodem-next` or the retired legacy Lua `luci-app-qmodem` as the frontend.
Migration:

```sh
# 1) remove the absorbed standalone packages first so they cannot fight over
#    identically named files
opkg remove qmodem-suite qmodem-smsd qmodem_monitor qmodem-settings tom_modem \
	ubus-at-daemon sms-forwarder-next qmodem-seal 2>/dev/null
# the retired legacy Lua frontend (if still present on the device) must go too,
# otherwise it fights over the menu and the LuCI cache
opkg remove luci-app-qmodem luci-app-qmodem-sms luci-app-qmodem-ttl \
	luci-app-qmodem-mwan luci-app-qmodem-hc 2>/dev/null

# 2) install the new layout
opkg install luci-app-qmodem-next luci-i18n-qmodem-next-zh-cn
```

`/etc/config/qmodem`, `/etc/config/qmodem_sms`, `/etc/config/sms_forwarder`,
`/etc/config/qmodem_ttl` and `/etc/config/ubus-at-daemon` are conffiles and are preserved
across upgrades. The SMS database `/etc/qmodem/sms.sqlite3` is preserved too (`keep.d`
declares `/etc/qmodem`).
