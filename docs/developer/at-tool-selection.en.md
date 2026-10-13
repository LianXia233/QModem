# AT Tool Selection: tom_modem / sms_tool_q / qmodem-atd

[简体中文](at-tool-selection.zh-cn.md) | English

> Scope note: path 0 (tom_modem/ubus at-daemon) and path 1 (sms_tool_q) are existing implementations; for the code entry points see `at()`/`fastat()` in `application/qmodem/files/usr/share/qmodem/modem_util.sh`. **Path 2 (the qmodem-atd unix socket) is a new path introduced in this version**; the shell-side wiring, measured test records, and the corresponding journal are still in the release pipeline and will be added to the [docs/refactor/](../refactor/README.md) index once published. Descriptions based on this document should be read as design statements until the journal is published.

## 1. Comparison of the three paths

All QModem shell scripts (vendor adapters, dialing, SMS) send AT commands uniformly through the `at()` / `fastat()` wrappers; the two functions choose the underlying path based on the value of `qmodem.main.at_tool`:

| | Path 0 | Path 1 | Path 2 (introduced in this version) |
|---|---|---|---|
| Tool | `tom_modem` (via ubus `at-daemon` when `$use_ubus_flag` is set) | `sms_tool_q` direct serial connection | `qmodem-atd` unix socket JSON protocol |
| Serial port occupancy | Exclusive when tom_modem connects directly; queued by at-daemon in ubus mode | The shell process connects directly, exclusive | Owned exclusively by the qmodem-atd per-port actor and queued serially |
| Concurrency behavior | In ubus mode, the at-daemon command queue guarantees serial execution | Concurrent calls each open the port themselves and may trample each other | per-port mpsc queue executes serially, private response channel per request, no mismatch |
| Timeout semantics | `at()` uses the tom_modem default timeout (`$options` can append arguments); `fastat()` fixes `-t 1` | Same as left: `at()` default, `fastat()` appends `-t 1` | Explicit `timeout_ms` per request, monotonic clock; on timeout, returns the partially received lines + `timeout` status |
| Dependencies | tom_modem / ubus daemon | Only the sms_tool_q binary | qmodem-atd binary + unix socket |
| Suitable scenarios | Default path, widest compatibility | Lightweight devices / modems where tom_modem performs poorly | Scenarios with concurrent multi-modem operation, URC subscription, and stable timeout semantics |
| Configuration | `at_tool='0'` or unset | `at_tool='1'` | `at_tool='2'` (`at_socket` specifies the socket path) |

`fastat()` has consistent semantics across the three paths: a fixed 1-second timeout (converted to `timeout_ms: 1000` on path 2), no `$options` appended; intended for polling-style short commands.

## 2. Configuration

Write to `/etc/config/qmodem` and restart the main service to take effect:

```sh
# Path 2: qmodem-atd unix socket (introduced in this version)
uci set qmodem.main.at_tool='2'
# Optional, default /var/run/qmodem/atd.sock
uci set qmodem.main.at_socket='/var/run/qmodem/atd.sock'
uci commit qmodem
/etc/init.d/qmodem_init restart

# Back to the default path
uci set qmodem.main.at_tool='0'
uci commit qmodem && /etc/init.d/qmodem_init restart
```

Notes:

- `at_tool` is a global option that applies to the `at()`/`fastat()` calls of all modems; the Settings page of `luci-app-qmodem` provides an `Alternative AT Tools` toggle corresponding to `at_tool='1'` (see `luci/luci-app-qmodem/htdocs/luci-static/resources/view/qmodem/settings.js`).
- Path 2 requires `qmodem-atd` to be running and to have registered the target port. For daemon startup, port registration, and JSON protocol details, see [Rust async daemon architecture](async-daemon.zh-cn.md).

## 3. Failover behavior

- **Path 2 → Path 0 fallback**: when `at_tool='2'`, requests whose socket connection fails (daemon not started, wrong path, insufficient permissions) fall back to the tom_modem path for execution, and the fallback event is logged via `logger -t qmodem`; the fallback does not change command or timeout semantics. After the daemon recovers, new requests automatically return to the socket path. (Behavior introduced in this version; measured data is subject to the corresponding journal when published.)
- **Paths 0 / 1 have no automatic switching**: missing tools or failed commands propagate the existing return codes upward, and the caller (dialing/SMS scripts) handles retries.
- When a qmodem-atd port closes or the actor exits, queued requests receive a `PortClosed` error instead of hanging; these errors likewise trigger the fallback above.

## 4. Diagnostic commands

```sh
# Logs: fallback events, path selection, and AT errors
logread -t qmodem
logread -e qmodem-atd

# Path 0: confirm the at-daemon ubus object exists (ubus mode)
ubus list | grep at-daemon
ubus call at-daemon sendat '{"at_port":"/dev/ttyUSB2","at_cmd":"AT"}'

# Path 2: socket liveness probe (list ports registered by qmodem-atd)
curl --unix-socket /var/run/qmodem/atd.sock -d '{"cmd":"list"}' http://localhost/

# Path 2: AT liveness probe
curl --unix-socket /var/run/qmodem/atd.sock \
     -d '{"cmd":"sendat","port":"/dev/ttyUSB2","at":"AT","timeout_ms":1000}' \
     http://localhost/
# Normal response: {"status":"ok","lines":["OK"],"end_flag":"OK","elapsed_ms":...}
# {"status":"error","error":"unknown port"} → port not registered, see the async-daemon documentation for startup
```

Logs and AT responses may contain IMEI, IMSI, phone numbers, and other sensitive information; redact before publishing.

## 5. Related documents

- [Rust async daemon architecture](async-daemon.zh-cn.md): qmodem-atd's actor model, JSON protocol, and cross-compilation
- [Rust workspace README](../../rust/README.md): zero-dependency design, size data, and tests
- [AT fixture format and replay](../../testcases/README.md): path-agnostic AT response replay tests
- [RPCD AT Daemon user guide](../rpcd-at-daemon-userguide-cn.md): ubus/rpcd invocation of at-daemon
