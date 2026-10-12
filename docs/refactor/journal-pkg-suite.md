# Journal: qmodem-suite meta-package

- **Agent**: agent-pkg-suite
- **Date**: 2026-10-12
- **Scope**: `application/qmodem-suite/**`（全新）+ `application/qmodem/Makefile`（小改）。其余路径未触碰。

## 目标

"单包交付，安装即用"：`opkg install qmodem-suite`（或 `apk add`）后获得完整栈：核心 + LuCI 界面 + 短信守护 + 监控。

## 变更记录

1. **新建 `application/qmodem-suite/Makefile`**（meta-package，无构建输出）
   - `PKG_NAME:=qmodem-suite`，`PKGARCH:=all`，版本走 `../../version.mk` 的 `QMODEM_RELEASE`（即 `QMODEM_AUTORELEASE`），与 `application/qmodem_monitor/Makefile` 模式一致。
   - `DEPENDS`（已逐一核对真实 `Package/$(PKG_NAME)` 名称）:
     - `+qmodem`（application/qmodem）
     - `+luci-app-qmodem-next`（luci/luci-app-qmodem-next）
     - `+qmodem-smsd`（application/qmodem_smsd，**连字符**，目录名是下划线，勿混淆）
     - `+qmodem_monitor`（application/qmodem_monitor，**下划线**）
     - `+luci-app-qmodem-monitor`（luci/luci-app-qmodem-monitor，其自身已依赖 qmodem_monitor 与 next，此处显式声明仅为语义完整）
   - install 段仅安装 `files/etc/uci-defaults/zz-qmodem-suite`：安装/首次开机时启用 `qmodem-smsd`、`qmodem_monitor` 服务并打印提示，系统执行后自删。保持最小，不写 `/etc/config/qmodem-suite`（无独立配置项，避免无用配置文件）。
2. **`application/qmodem/Makefile` 小改**：DEPENDS 补 `+jsonfilter`。原因见审计——脚本 20 处调用 `jsonfilter`（`usr/libexec/rpcd/qmodem`、`usr/share/qmodem/modem_dial.sh`、`usr/share/qmodem/usage_stats.sh`），原 DEPENDS 未声明，属硬缺失。`jsonfilter` 为独立二进制包，busybox 不含。其余 DEPENDS 未动。
3. **CI/构建拾取确认**：目录结构与同级包一致（`application/<pkg>/Makefile`），标准 OpenWrt feed 扫描会自动拾取；`.github/workflows` 未触碰。

## 依赖审计表（外部命令）

| 包 | 命令 | 使用位置 | busybox 内置？ | 现状 | 处置 |
|---|---|---|---|---|---|
| qmodem | awk/sed/grep/cat | 大量 | ✅ | — | 无需处理 |
| qmodem | jq | 155 处 | ❌ 独立包 | 已声明 `+jq` | 硬依赖，OK |
| qmodem | bc | 91 处 | ❌ | 已声明 `+bc` | 硬依赖，OK |
| qmodem | jsonfilter | 20 处（rpcd/dial/stats） | ❌ 独立包 | **缺失** | **本次补 `+jsonfilter`** |
| qmodem | wget | led_scripts 2 文件 | ✅ busybox wget | 未声明 | 正确（软依赖即可） |
| qmodem | xxd | 4 处 | ❌（busybox 默认无 xxd） | 已声明 `+xxd` | OK |
| qmodem | stat | usage_stats 等 | busybox 有，但脚本依赖 GNU stat | 已声明 `+coreutils +coreutils-stat` | OK |
| qmodem | tar/md5sum/sha256sum | qmodem_collect | ✅ | 未声明 | OK |
| qmodem | ip | hotplug/led | ✅（busybox ip；ip-full 可选） | 未声明 | OK |
| qmodem_monitor | jq / flock | 核心逻辑 | jq ❌ / flock ✅ | 已声明 `+jq +flock` | OK |
| qmodem_monitor | curl | `Method=curl` HTTP 探测（modem_monitor.sh:191） | ❌ | **未声明** | 可选路径（默认 ping），建议上游加 `+PACKAGE_...:curl` 式可选依赖；不属本域，仅记录 |
| qmodem-smsd | （C 守护进程） | libubus/libubox/libjson-c/libsqlite3 | — | DEPENDS 完整 | OK |
| luci-app-qmodem-next | — | LUCI_DEPENDS=`+qmodem +qmodem-smsd +sms-forwarder-next +luci-base` | — | 精简 | **无 `+luci-compat` 冗余**（纯 JS 应用，无需 compat 层），报告确认无误 |
| sms-forwarder-next | curl | DEPENDS 已含 `+curl` | ❌ | 已声明 | OK |

## 验证

- qmodem-suite Makefile 与 `qmodem_monitor`、`qmodem-smsd` 逐字段比对：`include ../../version.mk`、`PKG_RELEASE:=$(QMODEM_RELEASE)`、空 `Build/Prepare`/`Build/Compile`、`$(eval $(call BuildPackage,$(PKG_NAME)))` 一致。
- DEPENDS 五个包名均以仓库内 `Package/$(PKG_NAME)` 定义核实（qmodem-smsd 的连字符易错点已双查）。
- 本机无 OpenWrt SDK，`make` 不可用；以上为语法自查 + 逐字段比对。

## 未尽事项（移交）

- qmodem_monitor 的可选 `curl` 依赖（建议上游用可选 CONFIG 依赖）。
- luci-app-qmodem-monitor 的 `PKG_RELEASE:=1` 为硬编码，未走 AUTORELEASE（不属本域，仅提示）。
