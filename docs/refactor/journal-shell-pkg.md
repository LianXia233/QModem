# Shell 与打包修复日志

> 由 agent-shell-pkg 维护。本文件记录本轮 shell 脚本安全修复与打包规范化改动。

## S1 modem_hook.sh at_port 哨兵条件反写

【问题】原条件 `if [ -f "$at_port" ] || [ -z "$at_port" ]; then exit 1; fi` 为反写：at_port 正常存在（是文件）时也会 `exit 1`，导致 hook 从不执行 AT 命令。

【修复方式】改为 `if [ -z "$at_port" ] || [ ! -f "$at_port" ]; then exit 1; fi`：仅在 at_port 为空或不存在时退出。

【验证结果】`sh -n` 通过；逻辑真值表人工核对（空→退出，不存在→退出，存在→继续）。

【文件:行号】application/qmodem/files/usr/share/qmodem/modem_hook.sh:63

## S2 modem_hook.sh delay 变量名不一致

【问题】`config_get ${cfg_prefix}_delay $config_section delay` 写入 `post_init_delay`/`pre_dial_delay` 变量，后续却读取从未赋值的 `$delay`，UCI delay 配置失效。

【修复方式】统一变量名：`config_get delay $config_section delay`，第 69-71 行的 `[ -n "$delay" ] && sleep $delay` 读取同一变量。

【验证结果】`sh -n` 通过；grep 确认文件内仅剩 `delay` 一个变量名。

【文件:行号】application/qmodem/files/usr/share/qmodem/modem_hook.sh:59

## S3 modem_dial.sh sim_lock 语法错误与 PIN 泄露

【问题】`[ "$pin" == "$(cat $sim_lock_file)"]` 存在三处错误：`==` 非 POSIX、`]` 前缺空格导致语法错误、`$sim_lock_file` 未加引号。该 if 永远走 else。另：解锁成功/失败日志均把明文 PIN 写入 syslog；pincode 临时文件无权限保护。

【修复方式】改为 `[ "$pin" = "$(cat "$sim_lock_file")" ]`；成功与失败日志均去掉明文 PIN（失败日志仅保留 "unlock sim card failed,block try until nextboot"）；写入 pincode 后 `chmod 600 $sim_lock_file`。

【验证结果】`sh -n` 通过；grep 确认 unlock_sim 函数内日志不再包含 `$pin`。

【文件:行号】application/qmodem/files/usr/share/qmodem/modem_dial.sh:433-446

## S4 modem_dial.sh APN/用户名/密码注入

【问题】qmi_dial 中 `apn`/`username`/`password` 来自 UCI，无引号拼接进 `cmd_line` 后作为命令执行（约 1176-1184 行），值中含空格/引号可改变命令参数甚至注入；umbim 分支 `--apn $apn` 同样裸拼。

【修复方式】文件头部新增 POSIX 单引号转义函数 `sq()`（值中 `'` 转义为 `'\''`），qmi_dial 中三者改为 `-s $(sq "$apn")`、`$(sq "$username")`、`$(sq "$password")`；umbim 分支改为 `--apn "$apn"`。AT 指令路径（AT+CGDCONT 等）本就处于带引号 AT 字符串内，未改动。

【验证结果】`sh -n` 通过；/tmp 临时测试脚本验证 sq 对含单引号/分号/反引号输入的输出正确，eval round-trip 后原值无损、注入 payload 未被执行，测试后已删除临时文件。

【文件:行号】application/qmodem/files/usr/share/qmodem/modem_dial.sh:21-25（sq）、1180-1188（qmi_dial）、1418（umbim）

## S5 rpcd qmodem config_section 路径穿越

【问题】`config_section` 直接取自 RPC JSON，拼入 `/tmp/cache_*`、`/var/run/qmodem/${config_section}_dir` 及 uci 路径，`../` 或绝对值可造成路径穿越写。原仅 get_monitor_status 有白名单校验。

【修复方式】文件头部提取通用校验函数 `valid_section()`（非空、仅 `[A-Za-z0-9_]`、长度 ≤64）；call 分支解析 config_section 后统一校验（非空即校验，非法返回 `{"error":"invalid modem section"}`）；set_lockband/set_neighborcell/set_network_prefer 的二次解析与 traffic_reset_set_schedule 亦补充校验。

【验证结果】`sh -n` 通过；所有使用 config_section 拼路径的代码路径均位于校验之后（call 分支全局校验覆盖 base_info/cell_info/info/network_info/sim_info/get_sms/delete_sms/clear_dial_log/get_dial_log/remove_modem/dial_status 等）。

【文件:行号】application/qmodem/files/usr/libexec/rpcd/qmodem:15-21（valid_section）、668-671（call 全局校验）、398-399 前后（traffic_reset）、1015-1021（set_lockband 等二次解析校验）

## S6 delete_sms/send_raw_pdu/send_sms 二次 read 耗尽 stdin

【问题】rpcd 脚本在 call 分支开头已 `read -r input` 读入整个 JSON 请求；delete_sms、send_raw_pdu、send_sms 内各自的第二次 `read -r input` 因 stdin 已耗尽得到空值，导致 index/cmd/params 全为空。

【修复方式】删除三处二次 `read -r input`，复用首次读入的 `$input`，保留原有 jsonfilter/jq 提取逻辑（index、cmd、params.phone_number/message_content）。

【验证结果】`sh -n` 通过；grep 确认 call 分支内仅剩开头一处 `read -r input`。

【文件:行号】application/qmodem/files/usr/libexec/rpcd/qmodem:873（delete_sms）、951（send_raw_pdu）、975（send_sms）

## S11 set_imei 入参不校验

【问题】set_imei 直接把 RPC 传入的 imei 透传到 AT 层（set_imei "$imei"），任意字符串直达 AT 命令。

【修复方式】增加格式校验：非空、仅数字（case `*[!0-9]*`）、长度恰 15（`${#imei}`）；不合法返回 status 0 与错误信息并 exit 1，不再调用 AT 层。

【验证结果】`sh -n` 通过；校验逻辑真值表人工核对（14/16 位、含字母、空串均拒绝）。

【文件:行号】application/qmodem/files/usr/libexec/rpcd/qmodem:1001-1020

## H9 qmodem Makefile 规范化安装

【问题】`Package/$(PKG_NAME)/install` 使用 `$(CP) ./files/* $(1)/` 整拷，未使用 OpenWrt INSTALL 宏，权限与 strip 处理不规范。

【修复方式】改为 `INSTALL_DIR` + 逐目录安装：可执行文件（init.d、hotplug.d/net|usb、board.d、uci-defaults 91/99、rpcd 二进制位脚本、usr/sbin、usr/share/qmodem 根及 vendor/led_scripts 中原 755 的 .sh）用 `INSTALL_BIN`；数据文件（etc/config、acl.d JSON、cmds/*.sh、JSON/HTM/配置及原 644 文件）用 `INSTALL_DATA`，与原 files 目录实际权限一一对应，全部文件无遗漏。

【验证结果】对照 `find files -type f` 清单逐项核对安装列表；Makefile 内 INSTALL* 引用 37 处，覆盖全部 5 个子目录与根目录。

【文件:行号】application/qmodem/Makefile:150-213

## H10 luci-app-qmodem-next PKG_RELEASE 改用 AUTORELEASE

【问题】PKG_RELEASE 写死为 4，不随 commitcount 自增。

【修复方式】改为 `PKG_RELEASE:=$(QMODEM_AUTORELEASE)`（version.mk 已在 luci.mk 之前 include，与 qmodem 主包用法一致，兼容 luci.mk 体系）。

【验证结果】grep 确认无写死值；include 顺序核对（rules.mk → version.mk → luci.mk）。

【文件:行号】luci/luci-app-qmodem-next/Makefile:16

## 拼写修正 LICENSE 声明变量

【问题】5 个 Makefile 使用错误变量名（LICENSE 拼成了 LINCESE，即 `PKG_LI` + `NCESE_FILES`），LICENSE 文件声明失效。

【修复方式】统一替换为正确的 `PKG_LICENSE_FILES`。

【验证结果】`grep -rn "PKG_LI*NCESE_FILES"`（正则）全仓库无残留。

【文件:行号】luci/luci-app-qmodem/Makefile:15、luci/luci-app-qmodem-hc/Makefile:14、luci/luci-app-qmodem-mwan/Makefile:14、luci/luci-app-qmodem-sms/Makefile:14、luci/luci-app-qmodem-ttl/Makefile:14

## 验证记录汇总

- `sh -n`：modem_hook.sh、modem_dial.sh、rpcd/qmodem 全部通过（输出 "sh -n ALL OK"）。
- `grep -rn "PKG_LI*NCESE_FILES"`（正则）：仓库无残留。
- sq() 转义函数自检（/tmp/test_sq.sh，已删除）：普通串、含单引号、分号+反引号、注入 payload 共 6 项断言全部通过（"SQ TESTS ALL PASS"），eval round-trip 后注入 payload 未执行。
- 改动文件均为 LF 换行（file 检查无 CRLF）。
