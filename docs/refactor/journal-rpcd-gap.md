# rpcd 端点 gap 补齐：get_stats / clear_stats / get_current_band

- **agent**: agent-rpcd-gap
- **日期**: 2026-10-12
- **范围**: 仅 `application/qmodem/files/usr/libexec/rpcd/qmodem`（+242/-4）。ACL/前端侧由 agent-luci-gap 同步完成，本任务不触碰。

## 1. 背景与现状复核

`journal-retire-s1.md` gap 表指出 next 前端缺 `get_stats`/`clear_stats`/`get_current_band`。复核发现 rpcd 文件内三个 case 已存在，但全部委托旧栈实现：

| 端点 | 原委托 | 问题 |
|---|---|---|
| get_stats | `get_usage_stats`（generic.sh stub 恒 `available:0`；quectel.sh 走 AT QGDCNT） | 无网卡侧数据源，依赖厂商 AT 计数器，非 quectel 全部不可用 |
| clear_stats | `clear_usage_stats`（vendor AT 计数器清零） | 同上，且与"基线后增量"的 next 需求不匹配 |
| get_current_band | vendor `get_current_band`（generic stub `status:"unsupported"`） | 输出为 rich 结构 `{current_band:{cells:[...]}}`，next 需要扁平 `{rat,band,earfcn,nr_arfcn}` |

## 2. 实现（rpcd 文件内新增函数）

### get_stats / clear_stats（网卡侧统计）

- **section→网卡映射**：复用 modem_dial.sh:583 的既定逻辑（`uci get qmodem.<sec>.path` → `find <path> -name net`），候选项经 `/sys/class/net/<name>` 存在性校验（对齐 `get_network_interfaces` 的枚举校验风格），见 `find_modem_netif`。
- **get_netif_stats**：读 `/sys/class/net/<if>/statistics/{rx,tx}_bytes` 累计值 + `/proc/uptime`（整数秒）+ `date +%s`；输出键 `available/interface/uptime/updated_at/total_rx_bytes/total_tx_bytes`（前四键与旧 API 兼容的超集）+ 新增增量 `rx_bytes/tx_bytes`。
- **clear_netif_stats**：mktemp+mv 原子写 `/var/run/qmodem/<section>_stats_reset`（格式 `<rx_base> <tx_base> <epoch>`），返回重置后状态（增量为 0）。`get_stats` 增量 = 累计值 − 基线，无基线视为从开机累计；计数器回绕（当前 < 基线，如网卡重绑）计 0 防双算。增量全部在脚本内完成，前端只渲染数字。
- **不缓存**（实时性要求），stats 两个 case 不走 try_cache。

### get_current_band（扁平结构）

- `parse_current_band`：能力门控用 `type cmd_qeng_servingcell`（仅 cmds/quectel.sh 提供，generic/其他厂商自然落到 `supported:false`），经 vendor AT 封装取 `AT+QENG="servingcell"` 响应解析。
- 字段布局参照 vendor/quectel.sh 与 testcases fixture（unisoc RM500U hex 解码：SA 行 nr_arfcn=$10/band=$11/pci=$8；LTE 行按 TDD/FDD 字段有无区分两种布局；EN-DC 取 NR5G-NSA 行 arfcn=$4/band=$9）。尽力而为：数值字段经 `_qeng_numeric` 白名单校验，非法置空。
- `current_band_query` 为 try_cache 回调（10s 原子缓存 `/tmp/cache_current_band_<sec>`，与 base_info/info 同风格），输出 `{"modem_info":[{supported,rat,band,earfcn,nr_arfcn}]}`。
- 三个 case 全部走全局 `valid_section` + 函数内显式复验。

## 3. 验证记录（/tmp/qmodem_test/run.sh，53 断言全绿）

1. **sh -n**：bash + dash 双解释器通过。
2. **stub 环境**：jshn.sh stub（真实 JSON 构建，输出经 jq 校验）、uci shim、fake sysfs（`<sec>.path` → net/lo，lo 为容器真实网卡，total 值与 /sys 实读比对）、jsonfilter shim（jq 后端，供 `_rpc_field`/端到端用）。
3. **stats 断言**：基线读取/增量/回绕→0/无基线=累计/非法基线忽略；clear 写文件 3 字段、增量归零；未知/非法 section 返回 `available/result:false`。
4. **band 断言**：fixture NR5G-SA（633984/78）、LTE FDD（1300/3）、EN-DC、无 vendor 门控 supported:false、JSON shape（modem_info[0] 5 键）。
5. **e2e**：`sh rpcd call get_stats|get_current_band` 真实 dispatch 路径输出经 `jq -e .` 与 jsonfilter shim 校验合法。
6. **风格纪律**：bashism grep 清零（无 `[[`/`function `/`&>>`/`|&`）；diff 复核仅目标文件、ASCII/LF 无 CR；零新依赖（仅用 uci/jq/awk/grep/find/stat 既有工具）。

## 4. 行为变更说明（旧栈影响面）

- rpcd `get_stats`/`clear_stats` 数据源从"vendor AT 计数器"切换为"网卡 sysfs 基线增量"。旧栈 LuCI 经 modem_ctrl.sh 直连 vendor 函数，不受影响；rpcd 侧 `save_stats`（usage_stats.sh 持久化循环）保持 vendor 路径不变。
- traffic_reset cron（`ubus call qmodem clear_stats`）语义从"清模组计数器"变为"记基线"，与"用量重置"目标一致。
- `get_current_band` 输出从 `{current_band:{...}}` 变为 `{modem_info:[{...}]}`；rpcd 旧消费方无（旧栈走 modem_ctrl.sh），ACL 放行由 agent-luci-gap 处理。
