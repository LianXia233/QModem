# Journal: 全包打包审计与修复（application + luci）

- **Agent**: agent-pkg-audit-all
- **Date**: 2026-10-12
- **Scope**: `application/*/Makefile` + `luci/*/Makefile`（**排除 application/qmodem-suite**，其已在 journal-pkg-verify.md 审计）；不触碰 files/ 下脚本内容
- **背景**: journal-pkg-verify.md 发现 qmodem 曾漏装 5 个 init.d；本次将同类断链核查扩展到全部包
- **方法**: `/tmp` 交叉核对脚本：`find files -type f` 逐文件 vs install 段 `./files/...` 声明（含 `*` 通配展开）；DEPENDS 引用 vs 全部 `PKG_NAME` 求差；脚本用毕已删

## 1) 逐包核对表

### application（17 包）

| 包（目录） | files/ 实际 | install 覆盖 | init.d→INSTALL_BIN / config→INSTALL_CONF | 结论 |
|---|---|---|---|---|
| libqmodem-sms | 无 files/ | 产物 `.so.1.0.0` + `ln -sf .so.1` | — | OK |
| modem_scan | 无 files/ | 产物 modem_scand/scanc | — | OK |
| ndisc6 | 无 files/ | 多包（ndisc6/rdisc6/traceroute6/rdnssd），上游风格 | — | OK |
| qfirehose | 无 files/ | 产物 QFirehose | — | OK |
| qmodem | 71 | 70 → **断链：`cmds/qmodem_lib.sh` 未装** | config 原用 INSTALL_DATA | **修复 F1/F2** |
| qmodem-seal | 无 files/ | 产物 qmodem-seal | — | OK |
| qmodem_settings | 2 | 2/2 | init.d INSTALL_BIN ✓ | OK |
| qmodem_monitor | 3 | 3/3 但 `$(CP) ./files/*` | init.d 经 CP 非 INSTALL_BIN | **修复 F3** |
| qmodem_sipd | 7 | 7/7（含 `usr/sbin/*` 通配） | INSTALL_CONF + conffiles ✓ | OK |
| qmodem_smsd | 3 | 3/3 | 规范基线（INSTALL_BIN/INSTALL_CONF/conffiles） | OK |
| quectel_CM_5G_M | 7 | **仅 1（apns.json）**，Makefile 却预建了 `/lib/netifd/proto`、`/lib/netifd` 目录 | — | **修复 F4** |
| sms-tool_q | 无 files/ | 产物 sms_tool→sms_tool_q | — | OK |
| sms_forwarder | 8 | 8/8（`sms_forward_*.sh` 通配） | INSTALL_CONF + conffiles ✓ | OK |
| sms_forwarder_next | 9 | 9/9（init 改名安装、example→config） | INSTALL_CONF + conffiles ✓ | OK |
| tom_modem | 无 files/ | 产物 tom_modem | — | OK |
| ubus_at_daemon | 2 | 2/2，INSTALL_BIN/INSTALL_CONF ✓ | **缺 conffiles** | **修复 F5** |
| voipd | 11 | 11/11，INSTALL_BIN/INSTALL_CONF ✓ | **缺 conffiles** | **修复 F6** |

### luci（9 包）

全部走 `feeds/luci/luci.mk`（自动安装 `htdocs/`→/www、`luasrc/`→lua、`root/`→/、构建 po），无需手写 install 段。逐包核对 `root/` 内容均被自动打包，**无漏装**。`tests/` 目录不打包，正常。

| 包 | root/ 内容 | conffiles | 结论 |
|---|---|---|---|
| luci-app-qmodem (legacy) | 无 root/ | 声明 /etc/config/qmodem（实属 qmodem 包） | legacy 仅记录：冗余声明，无害不改 |
| luci-app-qmodem-hc (legacy) | qmodem_hc_sim 配置/init/uci-defaults/modem_sim.sh | qmodem_hc_sim ✓ | OK |
| luci-app-qmodem-monitor | menu.d + acl.d | — | OK |
| luci-app-qmodem-mwan (legacy) | qmodem_mwan 配置/init/uci-defaults/udhcpc.user.d/modem_mwan.sh | **声明 modem_mwan，实际文件 qmodem_mwan** | legacy 仅记录：声明失效不改 |
| luci-app-qmodem-next | menu.d + acl.d + keep.d + qmodem/.gitkeep | — | OK |
| luci-app-qmodem-sms (legacy) | 无 root/ | — | OK |
| luci-app-qmodem-ttl (legacy) | qmodem_ttl 配置/init/uci-defaults | qmodem_ttl ✓ | OK |
| luci-app-qmodem-ttlfw4 (legacy) | qmodem_ttl 配置/init + menu.d/acl.d | qmodem_ttl ✓ | OK |
| luci-app-qmodem-voip | menu.d + acl.d | — | OK |

## 2) DEPENDS 实名核对

脚本对全部 `DEPENDS`/`LUCI_DEPENDS` 中 `+xxx` 与全仓库 `PKG_NAME` 求差：

- 唯一告警 `luci-app-qmodem-monitor -> luci-app-qmodem-next` 为**误报**：next 包未显式 `PKG_NAME:=`，由 luci.mk 按目录名派生，实名一致。
- 连字符/下划线易混点复核：`qmodem-smsd`(qmodem_smsd)、`qmodem_monitor`(下划线)、`ubus-at-daemon`(ubus_at_daemon)、`qmodem-settings`(qmodem_settings)、`sms-forwarder-next`(sms_forwarder_next)、`qmodem-voip`(voipd)、`qmodem-sipd`(qmodem_sipd)、`quectel-CM-5G-M`(quectel_CM_5G_M) —— **全部一致，无断链**。
- 条件依赖 `PACKAGE_qmodem_*:xxx`（qmodem）与 `PACKAGE_libwebsockets-full:...`（voipd）语法正确。

## 3) 修复清单（对齐 qmodem_smsd 规范写法）

- **F1 断链（严重）**：`qmodem` 漏装 `files/usr/share/qmodem/cmds/qmodem_lib.sh`。10 个 vendor 脚本运行时 `source ${QMODEM_HOME:-/usr/share/qmodem}/cmds/qmodem_lib.sh`（vendor/quectel.sh:9 等）→ 全部 vendor 路径断链。已补进 cmds `INSTALL_DATA` 组。同 journal-pkg-verify 断链 A 同类。
- **F2**：`qmodem` conffiles 补 `/etc/config/qmodem`（原先仅声明 qmodem_led；旧栈 luci-app-qmodem 曾代为声明，脱离 legacy UI 后配置不保）；两个 uci config `INSTALL_DATA`→`INSTALL_CONF`。
- **F3**：`qmodem_monitor` 弃 `$(CP) ./files/* $(1)/`，改显式 `INSTALL_DIR` + `INSTALL_BIN`（init.d 与监控脚本），排除打包无关文件风险并对齐规范。
- **F4（严重）**：`quectel_CM_5G_M` 7 个 files 仅装 apns.json。Makefile 预建 `/lib/netifd/proto`、`/lib/netifd` 目录但从未安装——明确回归证据。补装：`rmnet.sh`/`rmnet6.sh`→`/lib/netifd/proto/`，`rmnet.script`/`rmnet6.script`→`/lib/netifd/`（rmnet.sh:18 硬引用该路径），`rmnet_init.sh`→`/etc/uci-defaults/99-rmnet-init`（内容为一次性 `uci set`+`commit`，属 uci-defaults 语义）。
- **F5**：`ubus_at_daemon` 补 conffiles `/etc/config/ubus-at-daemon`（install 已用 INSTALL_CONF，仅缺 sysupgrade 保留声明）。
- **F6**：`voipd` 补 conffiles `/etc/config/qmodem_voip`。

**有意不装（记录）**：`quectel_CM_5G_M/files/dhcp` 为整机 dnsmasq 配置样例，安装为 `/etc/config/dhcp` 会覆盖系统 dnsmasq 配置（opkg conffile 冲突），维持现状。

## 4) legacy 包边界

`luci-app-qmodem`/`-hc`/`-mwan`/`-sms`/`-ttl`/`-ttlfw4`/`-voip`（Makefile 标注 legacy, maintenance mode）按纪律**只查漏装不做其他改动**：确认 luci.mk 自动打包无漏装；两处 conffiles 瑕疵（mwan 声明名不匹配、qmodem 冗余声明）仅记录在 §1 表格。

## 验证

- `/tmp/verify_final.sh`（展开通配符逐文件比对）：修复后唯一 MISS 为有意不装的 `quectel_CM_5G_M/files/dhcp`；脚本用毕已删（`/tmp/audit_cmp.sh`、`/tmp/dep_check.sh`、`/tmp/verify_final.sh`、`/tmp/audit_out.txt`）。
- `git diff --stat` 复核：仅 5 个 Makefile 变更（qmodem/qmodem_monitor/quectel_CM_5G_M/ubus_at_daemon/voipd），均为最小 diff。
- `file` 确认 5 个 Makefile 均为 LF，无 CRLF。
- 本机无 OpenWrt SDK，`make` 不可用；静态核查 + 同构比对（qmodem_smsd 规范）。

## 遗留提示

- `quectel_CM_5G_M/files/dhcp` 后续若确认属某设备 overlay，应移入对应设备配方而非本包。
- legacy 两处 conffiles 瑕疵待退役批次（journal-retire-*.md）统一清理。
