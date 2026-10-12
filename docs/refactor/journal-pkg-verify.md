# Journal: qmodem-suite 打包验证与断链修复

- **Agent**: agent-pkg-verify
- **Date**: 2026-10-12
- **Scope**: `application/qmodem-suite/**` + `application/qmodem/Makefile`（init.d 安装断链修复）

## 1) DEPENDS 逐包核对（真实 Package 名）

| suite DEPENDS | 仓库定义 | 结论 |
|---|---|---|
| `+qmodem` | application/qmodem `PKG_NAME:=qmodem` | OK |
| `+luci-app-qmodem-next` | luci/luci-app-qmodem-next | OK |
| `+qmodem-smsd` | application/qmodem_smsd（目录下划线、包名**连字符**） | OK |
| `+qmodem_monitor` | application/qmodem_monitor（包名**下划线**） | OK |
| `+luci-app-qmodem-monitor` | luci/luci-app-qmodem-monitor | OK |

传递依赖实名复核：`qmodem-settings`（qmodem_settings）、`ubus-at-daemon`（ubus_at_daemon）、`tom_modem`、`sms-tool_q`、`modem_scan`、`sms-forwarder-next`（sms_forwarder_next）、`qmodem-seal`、`rdisc6/ndisc6` —— 连字符/下划线均与 Makefile `PKG_NAME` 一致，**无断链**。

## 2) 发现的断链（均已修复）

- **断链 A**：`qmodem` 包 5 个 init 脚本（qmodem_init/led/network/reboot/usage_stats）存在于 `files/etc/init.d/`，但 Makefile install 段**从未安装** → `Package/qmodem/install` 补 `$(INSTALL_DIR) $(1)/etc/init.d` + 5 个 `$(INSTALL_BIN)`。
- **断链 B**：全仓库无任何 uci-defaults enable `qmodem_init/qmodem_network/qmodem_led/ubus-at-daemon`（qmodem 自带 uci-defaults 仅 enable reboot/usage_stats；旧 HC 包 enable 的是遗留名 modem/modeminit）→ `zz-qmodem-suite` 补齐 6 个 enable（上述 4 个 + 原有 qmodem-smsd、qmodem_monitor），`[ -x ]` 守卫容忍部分安装。

## 3) 「首装即用」核对表（安装 → 可用）

| 项目 | 归属 | 状态 |
|---|---|---|
| LuCI 菜单 `admin/modem` | luci-app-qmodem-next `root/usr/share/luci/menu.d/luci-app-qmodem-next.json` | OK |
| LuCI 菜单（monitor） | luci-app-qmodem-monitor `menu.d/luci-app-qmodem-monitor.json` | OK |
| UCI 默认 `/etc/config/qmodem`、`qmodem_led` | qmodem 包 install 段 | OK |
| UCI 默认 `/etc/config/qmodem_sms` | qmodem-smsd 包 | OK |
| UCI 默认 `/etc/config/ubus-at-daemon` | ubus-at-daemon 包 | OK |
| `/var/run/qmodem` 创建者 | `qmodem_init`（start_service mkdir + boot + flock oneshot）；qmodem_network 亦 mkdir RUNDIR | OK（经修复后 init 脚本才真正存在并被 enable；无 tmpfile 方案，依赖 enable 链路） |
| 守护 enable | qmodem-smsd/qmodem_monitor/ubus-at-daemon/qmodem_init/network/led | OK（suite uci-defaults 补齐） |
| sms-forwarder-next | 安装即注册 init（默认 enable=0 门控，设计使然） | OK |

## 验证

- `sh -n zz-qmodem-suite` 通过；`file` 确认 LF（无 CRLF）。
- grep 确认 enable 循环存在于 uci-defaults；init.d 安装段 grep `init.d` 命中 6 处。
- qmodem Makefile 与同仓库 qmodem-smsd/ubus-at-daemon 的 init.d 安装模式逐字段比对一致。
- 本机无 OpenWrt SDK，`make` 不可用；静态核查 + 同构比对。

## 遗留提示

- qmodem_monitor 可选 curl 依赖（默认 ping 路径）维持软依赖，见 journal-pkg-suite.md。
