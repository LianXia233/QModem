# qmodem-suite Single-Package Installation Guide

[简体中文](install-suite.zh-cn.md) | English

> Status: **new metapackage**. `qmodem-suite` is a metapackage (`PKGARCH:=all`, see `application/qmodem-suite/Makefile`) that installs the QModem core together with the recommended frontends in one step. If the package is not yet available in your firmware feed, refer directly to the "Per-package manual installation" section at the end of this document. The post-install service status checks and troubleshooting commands apply regardless of the installation method.

## 1. One-command installation

qmodem-suite automatically pulls in the following components (dependencies are resolved by the package manager):

- `qmodem`: core control and dialing stack (including modem_scand and rpcd scripts)
- `luci-app-qmodem-next`: the recommended pure-JavaScript LuCI frontend
- `qmodem-smsd`: SMS daemon
- `qmodem_monitor` + `luci-app-qmodem-monitor`: network monitoring and its LuCI pages

### opkg (OpenWrt 21.02 / 23.05)

```sh
opkg update
opkg install qmodem-suite
```

### apk (default since OpenWrt 24.10)

```sh
apk update
apk add qmodem-suite
```

> Note: the two LuCI main interfaces should not be installed at the same time. The suite already points to `luci-app-qmodem-next`; do not additionally install the legacy `luci-app-qmodem`.

## 2. Post-installation status checks

### Service status (/etc/init.d/)

```sh
# Whether autostart is registered
/etc/init.d/qmodem_init enabled && echo enabled

# Status of each service
/etc/init.d/qmodem_init status      # Main service: modem_scand scanning + initialization
/etc/init.d/qmodem_network status   # Dialing network
/etc/init.d/qmodem_led status       # Signal/status LED
/etc/init.d/qmodem_reboot status    # Scheduled/conditional reboot
/etc/init.d/qmodem_usage_stats status  # Traffic statistics persistence
```

### ubus objects

```sh
ubus list | grep qmodem

# Example: query basic info of modem1 (section name refers to a modem-device section in /etc/config/qmodem)
ubus call qmodem base_info '{"config_section":"modem1"}'
ubus call qmodem network_info '{"config_section":"modem1"}'
```

See [rpcd interface](../qmodem-rpcd-interface.zh-cn.md) for the full method table.

### LuCI menu path

After installing `luci-app-qmodem-next`, a dedicated menu appears in the web interface: **Modem → QModem → Overview** (route `admin/modem`). If the menu is missing, first confirm that the browser account has been granted the ACL of `luci-app-qmodem-next`, then log in again.

## 3. Minimal working configuration

QModem supports automatic scanning and discovery of modems; in most scenarios a reboot after installation is enough for the modems to show up in LuCI. To pin down the configuration, write a minimal `modem-device` section manually (into `/etc/config/qmodem`):

```uci
config modem-device 'modem1'
    option enabled '1'
    option data_interface 'usb'
    option slot '/sys/bus/usb/devices/2-1.2'
    option at_port '/dev/ttyUSB2'
    option manufacturer 'Quectel'
    option enable_dial '1'
    option pdp_type 'IPv4/IPv6'
    option apn ''          # Leave empty to auto-match the carrier
```

For field descriptions and more options (band locking, SIM switching, custom initialization commands, etc.), see the "Modem device configuration" section of the [legacy full user manual](../user-guide.zh-cn.md). After making changes, run:

```sh
/etc/init.d/qmodem_init restart
```

## 4. Troubleshooting quick reference

| Symptom | Command / location |
|---|---|
| Service logs | `logread -t qmodem` (or `logread -e qmodem` to filter all QModem entries) |
| Scanner logs | `logread -e modem_scand` |
| Dialing logs | LuCI: the dialing log panel inside Modem → QModem → Overview; or rpcd `ubus call qmodem get_dial_log '{"config_section":"modem1"}'` |
| Kernel/driver layer | `logread -e usb -e mhi -e tty -e qmi -e mbim` |
| Port enumeration | `ubus call qmodem get_tty_ports '{}'` |

Logs and AT responses may contain IMEI, IMSI, ICCID, phone numbers, APNs, or SMS content; they must be redacted before publishing. For a more complete troubleshooting workflow, see [Troubleshooting](troubleshooting.zh-cn.md).

## 5. Comparison with per-package manual installation

| Dimension | qmodem-suite (single package) | Per-package manual installation |
|---|---|---|
| Number of commands | 1 (`opkg install qmodem-suite`) | One per package; the list must be determined yourself |
| Dependency resolution | Automatic by the package manager | Manual, in order: `qmodem` → frontend → optional packages |
| Frontend choice | Fixed to the recommended `luci-app-qmodem-next` | Free choice, but easy to mistakenly install both main interfaces |
| Upgrades | The metapackage upgrades as a whole, dependencies follow | Each package needs a separate `upgrade`; easy to miss one |
| Fine-grained trimming | Weak (unneeded optional packages are pulled in too) | Strong (install only what is needed, e.g. only `qmodem` + SMS package) |
| Suitable scenarios | First deployment, full-device delivery | Custom firmware, storage-constrained devices |

For the package list and build sources of per-package manual installation, see the [Installation guide](installation.zh-cn.md).

## 6. AT path selection

Which underlying path QModem uses for AT commands is determined globally by `qmodem.main.at_tool`: `0`/unset for the default tom_modem (optionally via ubus `at-daemon`), `1` for sms_tool_q direct connection, and `2` (introduced in this version) for the `qmodem-atd` unix socket (with the companion option `qmodem.main.at_socket` specifying the socket path, default `/var/run/qmodem-atd.sock`). For the applicable scenarios, timeout semantics, failover, and diagnostic commands of the three paths, see [AT tool selection](../developer/at-tool-selection.zh-cn.md).

After installing qmodem-suite, path 0 is used without any extra configuration; path 2 requires enabling the Rust component as described in the next section before switching.

## 7. Optional Rust component (qmodem-atd)

`qmodem-suite` does not include the Rust async AT daemon `qmodem-atd` by default. To enable path 2, cross-compile the binary yourself and place it into the firmware or the suite installation directory:

1. **Cross-compilation**: use the unified entry script `scripts/cross-build-rust.sh`—

   ```sh
   # aarch64 (common router target)
   bash scripts/cross-build-rust.sh --target aarch64-unknown-linux-musl
   # Output: scripts/dist/aarch64-unknown-linux-musl/qmodem-atd
   ```

   The output is a fully static musl binary (zero external crates, about 731 KiB, unstripped); after `file` confirms `statically linked`, it can be copied directly to the target device. For build details and the equivalent manual commands, see section 4 of [Rust async daemon architecture](../developer/async-daemon.zh-cn.md).

2. **Deployment**: put the binary into the device PATH (e.g. `/usr/bin/qmodem-atd`), and specify the socket path and port at startup (see section 3 of the async-daemon documentation); the default socket path matches `qmodem.main.at_socket`.

3. **Switch the path**: as described in the previous section, set `at_tool` to `2` and restart `qmodem_init`; when the socket is unreachable, it automatically falls back to the tom_modem path.
