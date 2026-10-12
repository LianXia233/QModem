# 旧栈退役（HC 分支）：luci-app-qmodem-hc 审查 + 声明式标注

- **agent**: agent-retire-hc
- **日期**: 2026-10-12
- **范围**: 仅 `luci/luci-app-qmodem-hc/**`。代码改动限于注入/bashism 修复，其余为声明式标注；不删功能、不改行为语义。

## 1. HC-G80 SIM 切换链路分析

### 1.1 切换链路（双通道）

本包是**全仓库唯一**使用 `fw_printenv`/`fw_setenv` 的位置（`grep -rln` 验证）：

| 通道 | 路径 | 语义 |
|---|---|---|
| GPIO 直写（当前槽） | `/sys/class/gpio/sim/value` | `1`=SIM1（近电源），`0`=SIM2 |
| GPIO 直写（模组复位） | `/sys/class/gpio/4g/value` | `0`→`1` 脉冲复位模组，切卡后必做 |
| uboot env（下次启动槽） | `fw_printenv -n sim2` / `fw_setenv sim2 [1]` | 变量缺省（被删除）=SIM1 启动；`sim2=1`=SIM2 启动 |

写入口有三处：
- `luasrc/controller/qmodem_hc.lua` `setSIM()`（HTTP 端点，UI 触发）
- `root/usr/share/qmodem/modem_sim.sh` `switch_sim()`（自动切换守护，ping/AT+CPIN/AT+CGPADDR 三判据，`fail_times >= judge_time*2` 触发取反翻转 GPIO）
- 两者语义一致：GPIO 取反 + 模组复位；controller 额外维护 uboot env

### 1.2 本包端点清单（非 ubus）

本包**没有任何 ubus 端点**，全走 LuCI HTTP dispatcher（`luasrc/controller/qmodem_hc.lua:7-9`）：

| 端点 | 类型 | 功能 |
|---|---|---|
| `admin/modem/qmodem/modem_sim` | cbi 页 | SIM 自动切换配置（qmodem_hc_sim.main）+ 手动切卡视图 |
| `admin/modem/qmodem/set_sim` | call `setSIM` | 切卡（GPIO + fw_setenv + 模组复位），JSON 应答 |
| `admin/modem/qmodem/get_sim` | call `getSIM` | 读当前槽（GPIO）+ next boot 槽（fw_printenv），JSON 应答 |

对照 `luci-app-qmodem-next/root/usr/share/rpcd/acl.d/luci-app-qmodem-next.json`：**无 qmodem_hc / modem_sim 任何条目**，即 next 未覆盖。

### 1.3 硬件假设（不可移植点）

1. DT 已导出名为 `sim`、`4g` 的 gpiochip sysfs 节点（mt7981/filogic hc-g80 DTS 专属，Makefile DEPENDS 已设备级限定）。
2. `fw_setenv` 可用且 `/etc/fw_env.config` 指向的 MTD 分区含 `sim2` 变量（uboot 侧配合）。
3. GPIO `sim` 初始电平决定上电槽位，与 uboot env 需人工保持一致（脚本不做一致性检查）。
4. `modem_sim.sh` 依赖 `at` 工具（modem_util.sh）与 `qmodem` UCI 的 modem-device 段；`_get_netdev` 中 `config_load modemconfig $1 modem_config` 疑似错用 `config_load` 三参数形式（config_load 只接受一个参数），netdev 关联可能失效——**逻辑问题，本次仅记录不修**。
5. `ping_dest` 为空时走拨号判据分支；`ping_monitor` 的 `$netdev` 依赖第 4 条。

## 2. next 覆盖方案：set_sim_slot 等效性验证清单

next 的 `set_sim_slot`（rpcd `application/qmodem/files/usr/libexec/rpcd/qmodem:1175`）→ vendor 脚本（如 quectel `AT+QUIMSLOT`），与 HC 的 uboot env/GPIO 是**完全不同的硬件路径**，且切换成功后 next 会触发 `qmodem_network redial`（HC 包是 GPIO 脉冲复位模组）。等效性验证清单（**TODO：以下均为实机操作，本任务不执行**）：

1. [ ] 确认 HC-G80 所载模组 vendor 是否存在 `set_sim_slot` 实现（无则 next 路径直接 404/失败，gap 成立）。
2. [ ] 基线采集：`fw_printenv sim2`、`cat /sys/class/gpio/sim/value`，记录旧栈 UI 切换前后两者变化。
3. [ ] 用 next UI/ubus 调 `set_sim_slot`（slot=1/2），观察 GPIO `sim` value 是否变化——若不变，证明 next 无法驱动 HC 硬件。
4. [ ] 验证 next boot 槽：next 路径切换后 `fw_printenv sim2` 是否被写入（预期：否，需确认固件是否有其他写 env 机制）。
5. [ ] 复位语义对比：next 的 redial 与 HC 的 GPIO `4g` 脉冲复位是否都能让 `AT+CPIN?` 反映新槽。
6. [ ] 持久性：切换后重启，验证实际槽位与 env/GPIO 一致。
7. [ ] 结论落盘：若 1-6 证明不等效，Phase 4 前需在 next 增加 HC 专用 ubus 方法（如 `qmodem.hc set_sim`）或将本包逻辑收编为 vendor script，否则本包**不可退役**。

## 3. 本次代码改动（注入/bashism 修复）

| 文件 | 改动 | 类型 |
|---|---|---|
| `luasrc/controller/qmodem_hc.lua` | `setSIM` 增加 slot 白名单（SIM1/SIM2 之外 400 拒绝）；sysfs 写入由 `shell("echo X > path")` 字符串拼接改为 `fs.writefile`；`sysfs_cmd`/`fw_setenv_cmd` 收敛为 local；fw_setenv 参数确认为固定字面量并加注释 | 注入防护（安全修复，行为等价） |
| `root/usr/share/qmodem/modem_sim.sh` | 4 处 `[ x == y ]` → `[ x = y ]`（at_dial_monitor 1 处、precheck 3 处） | bashism |
| `root/etc/init.d/qmodem_hc_sim` | `== 1` → `= 1` | bashism |
| `Makefile` / `modem_sim.htm` | 链路与 gap 声明式注释（htm 注释内无 `--`，置于 raw-text 区） | 标注 |

注：`shell()`（io.popen）现仅接收固定字面量命令（`fw_printenv -n sim2`、`fw_setenv sim2 [1]`），无用户输入可达。

## 4. 验证记录

1. `sh -n`：modem_sim.sh / qmodem_hc_sim / uci-defaults 全部通过。
2. `luac -p`：controller 与 cbi 两个 lua 文件通过。
3. bashism 清零：`grep -rn '==' root/` 无残留。
4. `git diff -- luci/luci-app-qmodem-hc` 复核：lua 改动为白名单分支（新增拒绝路径）+ writefile 替换（等价写语义）+ 注释；shell 改动仅 `==`→`=`；其余为注释。无行为逻辑变更。
