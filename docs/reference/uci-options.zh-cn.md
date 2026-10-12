# QModem UCI 选项参考

> 来源：全仓库 `uci get/set` / `config_get` / `option_string` 调用点静态梳理（生成方式见 `docs/refactor/journal-uci-docs.md`，agent-uci-docs）。
> 路径均为 `文件:行号`，相对仓库根目录。

## 包 `qmodem`（`/etc/config/qmodem`）

### 节 `main`（config main 'main'）

| 选项 | 类型 | 默认值 | 取值域 | 消费者 |
|---|---|---|---|---|
| enable_dial | bool | `'1'` | 0/1 | `application/qmodem/files/etc/init.d/qmodem_network:45,74`；`application/qmodem/files/usr/share/qmodem/modem_dial.sh:510,534` |
| try_preset_usb | bool | `'1'` | 0/1 | `application/qmodem/files/etc/init.d/qmodem_init:48` |
| try_preset_pcie | bool | `'1'` | 0/1 | `application/qmodem/files/etc/init.d/qmodem_init:49` |
| enable_pcie_scan | bool | `'1'` | 0/1 | `application/qmodem/files/etc/init.d/qmodem_init:46`；默认值由 `application/qmodem/files/etc/uci-defaults/99-qmodem-board:69` 生成 |
| start_delay | int（秒） | `'0'` | >=0 | `application/qmodem/files/etc/init.d/qmodem_init:72` |
| scan_log_level | string | `'info'` | 日志级别（透传给 modem_scand 环境变量） | `application/qmodem/files/etc/init.d/qmodem_init:128` |
| scan_workers | int | `'4'` | >0 | `application/modem_scan/src/modem_scand.c:1733` |
| at_probe_workers | int | `'4'` | >0 | `application/modem_scan/src/modem_scand.c:1735` |
| at_timeout_fast | int（秒） | `'2'` | >0 | `application/modem_scan/src/modem_scand.c:1737` |
| at_timeout_model | int（秒） | `'8'` | >0 | `application/modem_scan/src/modem_scand.c:1739` |
| hotplug_add_delay | int（秒） | `'8'` | >=0 | `application/qmodem/files/etc/hotplug.d/usb/20-modem-usb:6`；`application/qmodem/files/etc/hotplug.d/net/20-modem-net:7` |
| add_retry_delay | int（秒） | `'8'` | >=0 | `application/modem_scan/src/modem_scand.c:1741` |
| add_retry_max | int | `'5'` | >=0 | `application/modem_scan/src/modem_scand.c:1743` |
| usage_stats_nvram_save | bool | `'0'` | 0/1 | `application/qmodem/files/etc/init.d/qmodem_usage_stats:58` |
| usage_stats_nvram_interval | int（秒） | `'300'` | sanitize_interval 允许的正整数 | `application/qmodem/files/etc/init.d/qmodem_usage_stats:38,61` |
| at_tool | int | `'0'` | 0=tom_modem/ubus at-daemon；1=sms_tool_q 直连 TTY；2=qmodem-atd（unix socket，不回退） | `application/qmodem/files/usr/share/qmodem/modem_util.sh:245,255` |
| at_socket | string | `'/var/run/qmodem/atd.sock'` | unix socket 路径 | `application/qmodem/files/usr/share/qmodem/modem_util.sh:111` |
| testcase_collect | bool | `'0'` | 0/1 | `application/qmodem/files/usr/share/qmodem/modem_util.sh:25`；`application/qmodem/files/usr/sbin/qmodem_collect:35` |

### 节 `main`（仅运行期由 uci-defaults 写入，默认配置文件中不存在）

| 选项 | 类型 | 默认值 | 取值域 | 消费者 |
|---|---|---|---|---|
| board_defaults_board | string | 设备 board 名 | 板级标识 | 读：`application/qmodem/files/etc/uci-defaults/99-qmodem-board:81`；写：同文件 `:97` |
| board_defaults_version | string | `BOARD_DEFAULTS_VERSION` | 版本串 | **仅写入**（`99-qmodem-board:98`），无读取消费者；作为板级默认值版本标记保留 |

### 节 `modem-device`（运行期由 modem_scand 生成，每调制解调器一节）

| 选项 | 类型 | 默认值 | 取值域 | 消费者 |
|---|---|---|---|---|
| name | string | 扫描识别的型号 | 型号串 | `application/qmodem/files/usr/share/qmodem/modem_util.sh:49,73`；`application/qmodem/files/usr/libexec/rpcd/qmodem:76` 所在 qmodem_collect `:76`；`application/qmodem/files/usr/share/qmodem/vendor/foxconn.sh:11`；`vendor/fibocom.sh:34`；`application/voipd/files/usr/lib/qmodem_voip/at_daemon_adapter.sh:20`；写：`application/modem_scan/src/modem_scand.c` |
| manufacturer | string | 扫描识别 | 厂商标识 | `application/qmodem/files/usr/share/qmodem/modem_ctrl.sh:10`；`rpcd/qmodem:132,167`；`qmodem_collect:74`；`qmodem_usage_stats`（config_get）；写：`modem_scand.c` |
| platform | string | 扫描识别 | 平台标识 | `modem_ctrl.sh:11`；`rpcd/qmodem:134,168`；`qmodem_collect:75` |
| path | string | — | /sys 设备路径 | `modem_ctrl.sh:16`；`rpcd/qmodem:139,566`；`application/qmodem/files/etc/init.d/qmodem_led:32`；`at_daemon_adapter.sh:7`；`99-qmodem-board:188` |
| at_port | string | — | /dev/ttyUSB* 端口 | `modem_ctrl.sh:6,104`；`rpcd/qmodem:128,150,197`；`application/qmodem_smsd/src/main.c:220`；`qmodem_led:184`；`at_daemon_adapter.sh:22,53` |
| override_at_port | string | — | /dev/ttyUSB* 端口 | `modem_ctrl.sh:7,103`；`rpcd/qmodem:129,151,196`；`application/qmodem_smsd/src/main.c:218` |
| sms_at_port | string | — | /dev/ttyUSB* 端口 | `modem_ctrl.sh:9`；`rpcd/qmodem:131,155`；`application/qmodem_smsd/src/main.c:219` |
| ports | list（空格分隔串） | — | 端口列表 | `modem_ctrl.sh:92`；`rpcd/qmodem:185` |
| valid_at_ports | list（空格分隔串） | — | AT 有效端口列表 | `modem_ctrl.sh:98`；`rpcd/qmodem:191` |
| pdp_index | int | 回退 suggest_pdp_index | PDU 序号 | `modem_ctrl.sh:12`；`rpcd/qmodem:135` |
| suggest_pdp_index | int | 扫描写入 | PDU 序号 | `modem_ctrl.sh:13`；`rpcd/qmodem:136`；`modem_dial.sh:523`；写：`modem_scand.c:1220` |
| use_ubus | bool | `'1'`（smsd 侧默认；uci-defaults 写 1） | 0/1 | `modem_ctrl.sh:14`；`application/qmodem/files/usr/share/qmodem/modem_hook.sh:14`；`rpcd/qmodem:137,159`；`application/qmodem/files/etc/init.d/qmodem_led:184`；`application/qmodem_smsd/src/main.c:221`；`application/qmodem_settings/files/usr/sbin/qmodem-settings:121`；`at_daemon_adapter.sh:34-35`；`application/qmodem/files/etc/uci-defaults/91-qmodem-use-ubus:16` |
| sms_mode | string | `database_poll`（use_ubus=1）/`direct` | database_poll / direct / database_urc | `application/qmodem_smsd/src/main.c:223,1143,1346`；`qmodem-settings:114`；`91-qmodem-use-ubus:18`（写 direct） |
| sms_storage_mem1 | string | `'SM'` | SIM/ME 存储名（"SM"/"ME"） | `application/qmodem_smsd/src/main.c:226` |
| sms_db_path | string | `'/etc/qmodem'` | 目录路径 | `application/qmodem_smsd/src/main.c:228` |
| sms_auto_delete_from_sim | bool | `'1'`（smsd 侧默认） | 0/1 | `application/qmodem_smsd/src/main.c:229`（config_get 经 option_string） |
| sms_forwarding | bool | `'0'` | 0/1 | `application/qmodem_smsd/src/main.c:231` |
| sms_poll_interval | int（秒） | `'300'` | 60–3600（越界回落 300） | `application/qmodem_smsd/src/main.c:232` |
| state | string | 扫描写入 `enabled`/`disabled` | enabled / disabled | `application/qmodem/files/usr/libexec/rpcd/modem_ctrl:5,15,33,42`；`modem_scand.c:1177-1212`；`modem_dial.sh:509`；`qmodem_usage_stats`（config_get） |
| alias | string | 扫描写入 | 别名串 | 写：`modem_scand.c:1201`；读：`application/qmodem/files/usr/share/qmodem/led_scripts/c2000_max.sh:30`、`led_scripts/m60k63.sh:29`、`led_scripts/misectel_modem_status.sh:17`、`qmodem_led:184` |
| metric | int | 扫描写入 | 网络跃点 | 写：`modem_scand.c:1213`；读：`modem_dial.sh:515,866,899`；`qmodem_led init.d:40` |
| slot | string | — | usb slot（如 `2-1`）或 pcie slot | `application/qmodem/files/usr/share/qmodem/generic.sh:675`；`modem_dial.sh:53`；`99-qmodem-board:179` |
| associated_usb | string | — | usb slot | `application/modem_scan/src/modem_scand.c:693`；写/改：`99-qmodem-board:300-301` |
| network | string | 扫描写入 | 网卡名列表 | `modem_scand.c:1173-1217`；`vendor/fibocom.sh:1987` |
| data_interface | string | 扫描写入 | usb / pcie 等 slot_type | 写：`modem_scand.c:1206`；读：`at_daemon_adapter.sh:21` |
| voice_pcm_port | string | 扫描写入 | PCM 端口 | 写：`modem_scand.c:1240-1241`（未识别时删除 `:1186`）；读：`at_daemon_adapter.sh:23,63` |
| led_script | string | uci-defaults 写入 | led_scripts 脚本名 | `99-qmodem-board:150`（读）、`:266`（写） |
| modes | string | — | `2g/3g/4g/5g` 组合 | `vendor/thales.sh:77`、`vendor/telit.sh:27`、`vendor/simcom.sh:62`、`vendor/sierra.sh:37`、`vendor/quectel.sh:119`、`vendor/openluat.sh:867`、`vendor/meig.sh:66`、`vendor/huawei.sh:58`、`vendor/gosuncn.sh:108`、`vendor/foxconn.sh:87`、`vendor/fibocom.sh:440` |
| sa_band | string | — | `/` 分隔 NR 频段 | `vendor/simcom.sh:391,588`、`vendor/quectel.sh:572,680,744`、`vendor/gosuncn.sh:426` |
| nsa_band | string | — | `/` 分隔 NR 频段 | `vendor/simcom.sh:392`、`vendor/quectel.sh:573,681,745` |
| lte_band | string | — | `/` 分隔 LTE 频段 | `vendor/simcom.sh:393,592`、`vendor/quectel.sh:574,682,746`、`vendor/gosuncn.sh:385,424` |
| wcdma_band | string | — | `/` 分隔 WCDMA 频段 | `vendor/simcom.sh:394,588`、`vendor/quectel.sh:575,683,747`、`vendor/gosuncn.sh:422` |
| lockcell_boot_hook_enabled | bool | 命中后由 modem_util 写 1 | 0/1 | `vendor/simcom.sh:888`、`vendor/quectel.sh:1294`、`vendor/gosuncn.sh:965`、`vendor/fibocom.sh:1322`；`modem_util.sh:368,386` |
| lockcell_boot_hook_delay | int | modem_util 写入 | 延迟秒数 | `modem_util.sh:369,387` |
| usage_reset_enabled | bool | rpcd 侧默认 | 0/1 | `rpcd/qmodem:435,501` |
| usage_reset_type | string | rpcd 内置默认 | daily / monthly | `rpcd/qmodem:436,506`（校验 445-447、479 附近） |
| usage_reset_hour | int | rpcd 内置默认 | 0–23 | `rpcd/qmodem:437,512` |
| usage_reset_day | int | rpcd 内置默认 | 1–31 | `rpcd/qmodem:438,518` |
| bridge_port | string | — | 桥接扫描目标端口 | `modem_dial.sh:67,544`；备份节写：`:215` |

### 节 `bridge-port-backup`（运行期由 modem_dial.sh 创建）

| 选项 | 类型 | 默认值 | 取值域 | 消费者 |
|---|---|---|---|---|
| modem_config | string | — | 源 modem-device 节名 | 写：`modem_dial.sh:213`；读：`:376` |
| device_section | string | — | 源设备节名 | 写：`modem_dial.sh:214`；读：`:379` |
| bridge_port | string | — | 桥接端口 | 写：`modem_dial.sh:215`；读：`:67,544` |

## 包 `qmodem_led`（`/etc/config/qmodem_led`）

### 节 `main`

| 选项 | 类型 | 默认值 | 取值域 | 消费者 |
|---|---|---|---|---|
| poll_interval | int（秒） | `'5'` | 正整数；`0` 视为 1；非数字回落 5 | `application/qmodem/files/etc/init.d/qmodem_led:261` |

### 节 `network_detect` / `modem_status` / `internet` / `cellular`（uci-defaults 生成）

| 选项 | 类型 | 默认值 | 取值域 | 消费者 |
|---|---|---|---|---|
| enabled | bool | `'1'` | 0/1 | `application/qmodem/files/etc/init.d/qmodem_led:132,158`；`99-qmodem-board:113,236` |
| script | string | — | led_scripts 脚本名 | `qmodem_led:96`；`99-qmodem-board:117,208,233,242` |
| bind | string | `'any'` | `port` / `any` | `qmodem_led:133`；`99-qmodem-board:119,161,208,291` |
| port | string | — | usb slot（bind=port 时必填） | `qmodem_led:134`；`99-qmodem-board:123,162` |

## 包 `qmodem_sms`（`/etc/config/qmodem_sms`）

### 节 `main`

| 选项 | 类型 | 默认值 | 取值域 | 消费者 |
|---|---|---|---|---|
| database | string | `'/etc/qmodem/sms.sqlite3'` | sqlite 文件路径 | `application/qmodem_smsd/files/etc/init.d/qmodem-smsd:9`；`application/qmodem_smsd/src/main.c:178` |
| multipart_wait | int（秒） | `'300'` | 1–3600（越界回落 300） | `application/qmodem_smsd/src/main.c:179` |
| late_fragment_window | int（秒） | `'3600'` | 1–86400（越界回落 3600） | `application/qmodem_smsd/src/main.c:183` |
| received_retention | int（条） | `'5000'` | >=1（越界回落 5000） | `application/qmodem_smsd/src/main.c:187` |
| sent_retention | int（条） | `'1000'` | >=1（越界回落 1000） | `application/qmodem_smsd/src/main.c:191` |

## 包 `qmodem_ttl`（配置文件不在本仓库 application/ 下，随 luci-app-qmodem-ttl / luci-app-qmodem-ttlfw4 安装）

### 节 `main`

| 选项 | 类型 | 默认值 | 取值域 | 消费者 |
|---|---|---|---|---|
| enable | bool | `'0'` | 0/1 | `luci/luci-app-qmodem-ttl/root/etc/init.d/qmodem_ttl:13`（config_get） |
| ttl | int | 空（未配置则跳过） | 纯数字（脚本内白名单校验） | `luci/luci-app-qmodem-ttl/root/etc/init.d/qmodem_ttl:27` |

## 废弃 option 标注

全仓库核对结果：**三个配置文件中现有 option 均存在消费者，未发现废弃项**。仅 `qmodem.main.board_defaults_version` 为"只写不读"的版本标记（见上表，非配置文件缺省项）。
