# QModem UCI Options Reference

> Derived from a repo-wide static survey of `uci get/set` / `config_get` / `option_string` call sites (methodology in `docs/refactor/journal-uci-docs.md`, agent-uci-docs).
> Paths are `file:line`, relative to the repository root.

## Package `qmodem` (`/etc/config/qmodem`)

### Section `main` (config main 'main')

| Option | Type | Default | Range | Consumers |
|---|---|---|---|---|
| enable_dial | bool | `'1'` | 0/1 | `application/qmodem/files/etc/init.d/qmodem_network:45,74`; `application/qmodem/files/usr/share/qmodem/modem_dial.sh:510,534` |
| try_preset_usb | bool | `'1'` | 0/1 | `application/qmodem/files/etc/init.d/qmodem_init:48` |
| try_preset_pcie | bool | `'1'` | 0/1 | `application/qmodem/files/etc/init.d/qmodem_init:49` |
| enable_pcie_scan | bool | `'1'` | 0/1 | `application/qmodem/files/etc/init.d/qmodem_init:46`; default emitted by `application/qmodem/files/etc/uci-defaults/99-qmodem-board:69` |
| start_delay | int (s) | `'0'` | >=0 | `application/qmodem/files/etc/init.d/qmodem_init:72` |
| scan_log_level | string | `'info'` | log level (passed to modem_scand env) | `application/qmodem/files/etc/init.d/qmodem_init:128` |
| scan_workers | int | `'4'` | >0 | `application/modem_scan/src/modem_scand.c:1733` |
| at_probe_workers | int | `'4'` | >0 | `application/modem_scan/src/modem_scand.c:1735` |
| at_timeout_fast | int (s) | `'2'` | >0 | `application/modem_scan/src/modem_scand.c:1737` |
| at_timeout_model | int (s) | `'8'` | >0 | `application/modem_scan/src/modem_scand.c:1739` |
| hotplug_add_delay | int (s) | `'8'` | >=0 | `application/qmodem/files/etc/hotplug.d/usb/20-modem-usb:6`; `application/qmodem/files/etc/hotplug.d/net/20-modem-net:7` |
| add_retry_delay | int (s) | `'8'` | >=0 | `application/modem_scan/src/modem_scand.c:1741` |
| add_retry_max | int | `'5'` | >=0 | `application/modem_scan/src/modem_scand.c:1743` |
| usage_stats_nvram_save | bool | `'0'` | 0/1 | `application/qmodem/files/etc/init.d/qmodem_usage_stats:58` |
| usage_stats_nvram_interval | int (s) | `'300'` | positive int accepted by sanitize_interval | `application/qmodem/files/etc/init.d/qmodem_usage_stats:38,61` |
| at_tool | int | `'0'` | 0=tom_modem/ubus at-daemon; 1=sms_tool_q direct TTY; 2=qmodem-atd (unix socket, no fallback) | `application/qmodem/files/usr/share/qmodem/modem_util.sh:245,255` |
| at_socket | string | `'/var/run/qmodem/atd.sock'` | unix socket path | `application/qmodem/files/usr/share/qmodem/modem_util.sh:111` |
| testcase_collect | bool | `'0'` | 0/1 | `application/qmodem/files/usr/share/qmodem/modem_util.sh:25`; `application/qmodem/files/usr/sbin/qmodem_collect:35` |

### Section `main` (runtime-only, written by uci-defaults; absent from the default config file)

| Option | Type | Default | Range | Consumers |
|---|---|---|---|---|
| board_defaults_board | string | device board name | board id | read: `application/qmodem/files/etc/uci-defaults/99-qmodem-board:81`; write: same file `:97` |
| board_defaults_version | string | `BOARD_DEFAULTS_VERSION` | version string | **write-only** (`99-qmodem-board:98`), no read consumer; kept as a board-defaults version marker |

### Section `modem-device` (generated at runtime by modem_scand, one section per modem)

| Option | Type | Default | Range | Consumers |
|---|---|---|---|---|
| name | string | detected model | model string | `application/qmodem/files/usr/share/qmodem/modem_util.sh:49,73`; `application/qmodem/files/usr/sbin/qmodem_collect:76`; `application/qmodem/files/usr/share/qmodem/vendor/foxconn.sh:11`; `vendor/fibocom.sh:34`; `application/voipd/files/usr/lib/qmodem_voip/at_daemon_adapter.sh:20`; written by `application/modem_scan/src/modem_scand.c` |
| manufacturer | string | detected | vendor id | `modem_ctrl.sh:10`; `rpcd/qmodem:132,167`; `qmodem_collect:74`; `qmodem_usage_stats` (config_get); written by `modem_scand.c` |
| platform | string | detected | platform id | `modem_ctrl.sh:11`; `rpcd/qmodem:134,168`; `qmodem_collect:75` |
| path | string | — | /sys device path | `modem_ctrl.sh:16`; `rpcd/qmodem:139,566`; `application/qmodem/files/etc/init.d/qmodem_led:32`; `at_daemon_adapter.sh:7`; `99-qmodem-board:188` |
| at_port | string | — | /dev/ttyUSB* port | `modem_ctrl.sh:6,104`; `rpcd/qmodem:128,150,197`; `application/qmodem_smsd/src/main.c:220`; `qmodem_led:184`; `at_daemon_adapter.sh:22,53` |
| override_at_port | string | — | /dev/ttyUSB* port | `modem_ctrl.sh:7,103`; `rpcd/qmodem:129,151,196`; `application/qmodem_smsd/src/main.c:218` |
| sms_at_port | string | — | /dev/ttyUSB* port | `modem_ctrl.sh:9`; `rpcd/qmodem:131,155`; `application/qmodem_smsd/src/main.c:219` |
| ports | list (space-joined string) | — | port list | `modem_ctrl.sh:92`; `rpcd/qmodem:185` |
| valid_at_ports | list (space-joined string) | — | valid AT port list | `modem_ctrl.sh:98`; `rpcd/qmodem:191` |
| pdp_index | int | falls back to suggest_pdp_index | PDP index | `modem_ctrl.sh:12`; `rpcd/qmodem:135` |
| suggest_pdp_index | int | written by scan | PDP index | `modem_ctrl.sh:13`; `rpcd/qmodem:136`; `modem_dial.sh:523`; written by `modem_scand.c:1220` |
| use_ubus | bool | `'1'` (smsd default; uci-defaults writes 1) | 0/1 | `modem_ctrl.sh:14`; `application/qmodem/files/usr/share/qmodem/modem_hook.sh:14`; `rpcd/qmodem:137,159`; `application/qmodem/files/etc/init.d/qmodem_led:184`; `application/qmodem_smsd/src/main.c:221`; `application/qmodem_settings/files/usr/sbin/qmodem-settings:121`; `at_daemon_adapter.sh:34-35`; `application/qmodem/files/etc/uci-defaults/91-qmodem-use-ubus:16` |
| sms_mode | string | `database_poll` (use_ubus=1) / `direct` | database_poll / direct / database_urc | `application/qmodem_smsd/src/main.c:223,1143,1346`; `qmodem-settings:114`; `91-qmodem-use-ubus:18` (writes direct) |
| sms_storage_mem1 | string | `'SM'` | storage name ("SM"/"ME") | `application/qmodem_smsd/src/main.c:226` |
| sms_db_path | string | `'/etc/qmodem'` | directory path | `application/qmodem_smsd/src/main.c:228` |
| sms_auto_delete_from_sim | bool | `'1'` (smsd default) | 0/1 | `application/qmodem_smsd/src/main.c:229` |
| sms_forwarding | bool | `'0'` | 0/1 | `application/qmodem_smsd/src/main.c:231` |
| sms_poll_interval | int (s) | `'300'` | 60–3600 (out of range falls back to 300) | `application/qmodem_smsd/src/main.c:232` |
| state | string | scan writes `enabled`/`disabled` | enabled / disabled | `application/qmodem/files/usr/libexec/rpcd/modem_ctrl:5,15,33,42`; `modem_scand.c:1177-1212`; `modem_dial.sh:509`; `qmodem_usage_stats` (config_get) |
| alias | string | written by scan | alias string | written by `modem_scand.c:1201`; read by `application/qmodem/files/usr/share/qmodem/led_scripts/c2000_max.sh:30`, `led_scripts/m60k63.sh:29`, `led_scripts/misectel_modem_status.sh:17`, `qmodem_led:184` |
| metric | int | written by scan | network metric | written by `modem_scand.c:1213`; read by `modem_dial.sh:515,866,899`; `qmodem_led init.d:40` |
| slot | string | — | usb slot (e.g. `2-1`) or pcie slot | `application/qmodem/files/usr/share/qmodem/generic.sh:675`; `modem_dial.sh:53`; `99-qmodem-board:179` |
| associated_usb | string | — | usb slot | `application/modem_scan/src/modem_scand.c:693`; written/rewritten by `99-qmodem-board:300-301` |
| network | string | written by scan | network interface names | `modem_scand.c:1173-1217`; `vendor/fibocom.sh:1987` |
| data_interface | string | written by scan | usb / pcie (slot_type) | written by `modem_scand.c:1206`; read by `at_daemon_adapter.sh:21` |
| voice_pcm_port | string | written by scan | PCM port | written by `modem_scand.c:1240-1241` (deleted on unknown `:1186`); read by `at_daemon_adapter.sh:23,63` |
| led_script | string | written by uci-defaults | led_scripts script name | `99-qmodem-board:150` (read), `:266` (write) |
| modes | string | — | combination of `2g/3g/4g/5g` | `vendor/thales.sh:77`, `vendor/telit.sh:27`, `vendor/simcom.sh:62`, `vendor/sierra.sh:37`, `vendor/quectel.sh:119`, `vendor/openluat.sh:867`, `vendor/meig.sh:66`, `vendor/huawei.sh:58`, `vendor/gosuncn.sh:108`, `vendor/foxconn.sh:87`, `vendor/fibocom.sh:440` |
| sa_band | string | — | `/`-separated NR bands | `vendor/simcom.sh:391,588`, `vendor/quectel.sh:572,680,744`, `vendor/gosuncn.sh:426` |
| nsa_band | string | — | `/`-separated NR bands | `vendor/simcom.sh:392`, `vendor/quectel.sh:573,681,745` |
| lte_band | string | — | `/`-separated LTE bands | `vendor/simcom.sh:393,592`, `vendor/quectel.sh:574,682,746`, `vendor/gosuncn.sh:385,424` |
| wcdma_band | string | — | `/`-separated WCDMA bands | `vendor/simcom.sh:394,588`, `vendor/quectel.sh:575,683,747`, `vendor/gosuncn.sh:422` |
| lockcell_boot_hook_enabled | bool | written to 1 on match | 0/1 | `vendor/simcom.sh:888`, `vendor/quectel.sh:1294`, `vendor/gosuncn.sh:965`, `vendor/fibocom.sh:1322`; `modem_util.sh:368,386` |
| lockcell_boot_hook_delay | int | written by modem_util | delay seconds | `modem_util.sh:369,387` |
| usage_reset_enabled | bool | rpcd-side default | 0/1 | `rpcd/qmodem:435,501` |
| usage_reset_type | string | rpcd built-in default | daily / monthly | `rpcd/qmodem:436,506` (validated nearby 445-447) |
| usage_reset_hour | int | rpcd built-in default | 0–23 | `rpcd/qmodem:437,512` |
| usage_reset_day | int | rpcd built-in default | 1–31 | `rpcd/qmodem:438,518` |
| bridge_port | string | — | bridge scan target port | `modem_dial.sh:67,544`; backup-section write `:215` |

### Section `bridge-port-backup` (created at runtime by modem_dial.sh)

| Option | Type | Default | Range | Consumers |
|---|---|---|---|---|
| modem_config | string | — | source modem-device section | write: `modem_dial.sh:213`; read: `:376` |
| device_section | string | — | source device section | write: `modem_dial.sh:214`; read: `:379` |
| bridge_port | string | — | bridge port | write: `modem_dial.sh:215`; read: `:67,544` |

## Package `qmodem_led` (`/etc/config/qmodem_led`)

### Section `main`

| Option | Type | Default | Range | Consumers |
|---|---|---|---|---|
| poll_interval | int (s) | `'5'` | positive int; `0` treated as 1; non-numeric falls back to 5 | `application/qmodem/files/etc/init.d/qmodem_led:261` |

### Sections `network_detect` / `modem_status` / `internet` / `cellular` (generated by uci-defaults)

| Option | Type | Default | Range | Consumers |
|---|---|---|---|---|
| enabled | bool | `'1'` | 0/1 | `application/qmodem/files/etc/init.d/qmodem_led:132,158`; `99-qmodem-board:113,236` |
| script | string | — | led_scripts script name | `qmodem_led:96`; `99-qmodem-board:117,208,233,242` |
| bind | string | `'any'` | `port` / `any` | `qmodem_led:133`; `99-qmodem-board:119,161,208,291` |
| port | string | — | usb slot (required when bind=port) | `qmodem_led:134`; `99-qmodem-board:123,162` |

## Package `qmodem_sms` (`/etc/config/qmodem_sms`)

### Section `main`

| Option | Type | Default | Range | Consumers |
|---|---|---|---|---|
| database | string | `'/etc/qmodem/sms.sqlite3'` | sqlite file path | `application/qmodem_smsd/files/etc/init.d/qmodem-smsd:9`; `application/qmodem_smsd/src/main.c:178` |
| multipart_wait | int (s) | `'300'` | 1–3600 (out of range falls back to 300) | `application/qmodem_smsd/src/main.c:179` |
| late_fragment_window | int (s) | `'3600'` | 1–86400 (out of range falls back to 3600) | `application/qmodem_smsd/src/main.c:183` |
| received_retention | int (msgs) | `'5000'` | >=1 (invalid falls back to 5000) | `application/qmodem_smsd/src/main.c:187` |
| sent_retention | int (msgs) | `'1000'` | >=1 (invalid falls back to 1000) | `application/qmodem_smsd/src/main.c:191` |

## Package `qmodem_ttl` (config file not shipped under this repo's application/; installed by luci-app-qmodem-ttl / luci-app-qmodem-ttlfw4)

### Section `main`

| Option | Type | Default | Range | Consumers |
|---|---|---|---|---|
| enable | bool | `'0'` | 0/1 | `luci/luci-app-qmodem-ttl/root/etc/init.d/qmodem_ttl:13` (config_get) |
| ttl | int | empty (skipped when unset) | digits only (whitelisted in script) | `luci/luci-app-qmodem-ttl/root/etc/init.d/qmodem_ttl:27` |

## Deprecated option notes

Repo-wide verification: **every option present in the three config files has at least one consumer; no deprecated options were found**. The only write-only entry is `qmodem.main.board_defaults_version` (a version marker, see table above; not part of the default config file).
