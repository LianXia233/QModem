# Journal: qmodem_monitor + modem_dial 安全/健壮性修复

- **agent**: agent-monitor
- **date**: 2026-10-12
- **scope**: `application/qmodem_monitor/files/**`、`application/qmodem/files/usr/share/qmodem/{modem_dial.sh,modem_util.sh,generic.sh}`（未触碰其他路径）

## S10【中】modem_monitor.sh 注入面收敛

1. `_run_script`：`$script_path $@` 未引号执行 UCI 配置的脚本路径 →
   增加 case 白名单（仅允许 `/usr/share/qmodem/` 前缀）+ `[ -f ]` 存在性校验，
   通过后以 `"$script_path" "$@"` 引号执行；拒绝时 `log`（logger -t qmodem_monitor）。
2. `switch_sim_slot`：三处 `ubus call ... '{"config_section": "'$Modem_ID'"}'`
   变量内插 JSON → 改为 `jq -nc --arg` 构造 payload，
   对齐 `monitor_recovery.sh` 的 `nm_switch`/`nm_switch_supported` 范式
   （get_sim_switch_capabilities / get_sim_slot / set_sim_slot 均参数化）。

## S12【中】modem_dial.sh 关键写操作失败检查

- `set_if`：`uci commit network` 失败、`/etc/init.d/network restart` 失败、
  `ifup ${interface_name}`/`ifup ${interface6_name}` 失败、
  `uci commit firewall|dhcp` 与对应 init 脚本 restart 失败 →
  均增加返回值检查，失败走 `logger -t qmodem "error: ..."`。
- `flush_if`：`uci commit network|dhcp|firewall`、firewall/network restart 同样加固。

## S9【中】modem_dial.sh shell 缺陷

1. `wwan_hang`：`[ -n $pid ]` → `[ -n "$pid" ]`，`kill` 同步加引号。
2. `set_if`：`[ -n "/sys/class/net/$ethernet_5g" ]` 恒真 → `[ -d ... ]`。
3. `append_to_fw_zone`：snapshot 无 `VERSION_ID` 时 `[ "" -le 21 ]` test 报错 →
   `${VERSION_ID:-snapshot}` 兜底后 `${os_version%%.*}` 取主版本，
   非数值 case 兜底为 99（视为新版走 add_list 路径），等价还原 `:0:2` 语义。
4. `qmi_dial`：`wait`（无参，等待所有后台作业）→ 记录 `qmi_dial_pid` 后
   `wait "$qmi_dial_pid"`；新增连续失败计数，≥3 次时
   `logger -t qmodem` 记 error 并 `sleep 30` 退避后清零重试。

## S13【低】公共 DNS 常量收编

- `modem_util.sh` 头部新增单一事实源：
  `QMODEM_FALLBACK_DNS{1,2}_IPV4` / `QMODEM_FALLBACK_DNS{1,2}_IPV6`。
- `generic.sh:get_dns` 与 `modem_dial.sh:ip_change_fm350` 原地改为引用常量。
  source 顺序核实：`generic.sh:4` source `modem_util.sh`，
  `modem_monitor.sh:3` 亦 source `modem_util.sh`，均在函数调用前定义，无顺序风险。

## S9【低】`sed 's/at/mbim/g'` 隐式端口转换加固

- 新增 `resolve_mbim_port()`（modem_dial.sh，紧跟 `sq()` 之后）：
  注释说明假设（mbim 字符设备名与 at 端口同构，仅前缀 at→mbim 替换，mtk_pcie 场景），
  转换后做 `[ -c "$port" ]` 字符设备校验，失败 `logger -t qmodem` 记 error
  并仍返回派生名（交由 umbim 报错）。仓库内无现成 mbim 探测逻辑
  （已 grep cdc-wdm/mbim-control，无命中），按约定"仅校验+日志"。
- `check_ip`、`at_dial`、`ip_change_fm350` 三处调用点统一替换。

## 验证记录

1. `sh -n`：modem_monitor.sh / monitor_recovery.sh / modem_dial.sh /
   modem_util.sh / generic.sh 全部 OK。
2. `/tmp/qmodem_test.sh` 单元断言（sh，18/18 通过后已删除）：
   - 白名单 case 匹配/拒绝/缺文件分支
   - `jq --arg` 对含 `"` 的 Modem_ID 正确转义
   - os-release 版本兜底（23.05→23、21.02→21、snapshot→99）且数值 test 不报错
   - `[ -n "$pid" ]` 引号语义、`[ -d ]` 设备目录判断
   - qmi_dial 退避计数（7 次失败触发 2 次退避）与 `wait $pid` 定向等待
3. journal 本身即交付物，无独立 journal 运行项。

## 风险与后续

- `resolve_mbim_port` 校验失败时保留原行为（返回派生名），
  若后续在 target 层补充 mbim-control 枚举可升级为自动回退探测。
- snapshot 版本判定为"非数值→按新版处理"，与 OpenWrt fw3/fw4 分界（21.02）一致。
