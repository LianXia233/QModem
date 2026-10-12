# Journal: qmodem_monitor 加固（monitor-hard）

- **agent**: agent-monitor-hard
- **date**: 2026-10-12
- **scope**: `application/qmodem_monitor/files/**`（未触碰其他路径）

## M1 心跳 JSON schema 加 version 字段

- `monitor_recovery.sh` `nm_state()`：jq 对象新增顶层
  `"version":1`（`--argjson version 1`），与既有 `api_version:1` 并存。
- 语义：`version` 是心跳 schema 自身的修订号；消费方以
  `(.version // 0) >= 1` 做兼容判断，缺字段视为 legacy v0（旧文件可读）。
- 消费方影响分析：仓库内读取该 JSON 的仅 `nm_status()`（读
  heartbeat/pid/connectivity/phase，均不受新增字段影响）与 LED 脚本
  （`led_scripts/*.sh` 读 `connectivity`，grep 确认无 `version`/`api_version`
  引用）；新增字段为纯增量，向后兼容。

## M2 CFUN 恢复动作与 at_tool=2 通路兼容性确认（记录依据）

- 结论：**自动兼容，无需改代码**。
- 依据：`nm_at_command()` 唯一入口为 `at "$AT_PORT" "$1"`；
  `modem_util.sh`（application/qmodem）中 `at()` 统一转发
  `qmodem_at_dispatch()`，其按 `qmodem.main.at_tool` 分发：
  `1) sms_tool_q` / `2) qmodem_atd_sendat`（atd unix-socket JSON line
  协议，3000ms）`*) tom_modem $use_ubus_flag`。at_tool=2 的响应语义
  （lines + exit code）在 `qmodem_atd_sendat()` 内映射，调用方无感。
- 已在 `nm_at_command()` 注释中记录该依据。legacy `modem_monitor.sh` 的
  `_send_at_command` 同样走 `at`，一并覆盖。

## M3 legacy modem_monitor.sh 与 recovery 防并发

- 评估：共享全局 `/var/run/qmodem/monitor.lock` 会串行化不同 modem 的
  巡检，代价大于收益；`flock` 各持锁但无互斥则无意义。
- 选型：**共享同名 per-modem 锁** —— legacy 启动时获取与 recovery 相同的
  `/var/run/qmodem_monitor/$Modem_ID.lock`（fd 9，`flock -n`），
  持锁整个生命周期，实现同 modem 互斥、跨 modem 并行。
- 新增 `Modem_ID` 字符白名单校验（`[A-Za-z0-9_-]`）防锁文件名注入；
  `flock -n` 失败 `log` 后 exit 1，由 procd respawn 节流（3600 5 5）
  兜底重试（profile 切换/手工双启场景为瞬时冲突）。
- recovery 原有 `exec 9>` + `flock -n` 语义不变，两侧对同一文件竞争。

## M4 巡检超时与进程组 kill 边界

- 泄漏点分析：原 `nm_run` 仅按 `/proc/<pid>/task/<pid>/children` 递归
  kill；孙进程在父进程先退（或扫描后被 reparent）时逃逸。
- 修复：`nm_run` 增加 `group|fn` 显式模式参数（6 处调用点全部更新）：
  - `group`（ping×2、netifd redial 等外部命令）：`setsid` 起独立进程组，
    超时走 `nm_kill_group`（负 pid TERM+KILL 整组 + `nm_kill_tree`
    兜底），自然退出后也做组扫尾，reparented 孙进程可清；
  - `fn`（`nm_at_command`/`nm_switch` 等 shell 函数）：保持 fork 子壳
    语义（`setsid` 是 exec，无法执行函数），继续 tree-walk；残余竞态
    （父进程在 kill 中途死亡）已在注释中声明。
- 设计取舍：评估过全局 `set -m` 让所有后台作业独立成组，但会改变守护
  进程整体作业控制语义，爆炸半径大，弃用。
- 附带加固：`nm_kill_tree` 读 `children` 文件加 `2>/dev/null`（实测存在
  `No such file or directory` 竞态噪音）。

## 验证

1. `sh -n`：bash + dash 双解释器，两脚本全过；LF 无 CR（grep `\r` = 0）。
2. /tmp 单元断言（用完即删 `/tmp/nmtest`）：
   - 心跳 schema：`version=1`/`api_version=1`/heartbeat 数值型/phase，
     以及消费方兼容表达式 `(.version // 0) >= 1` 对新旧 JSON 的真假分支；
   - 锁互斥：同 `flock -n 9` 语义二实例竞争（第二实例 BUSY）、释放后
     再获取成功、flock CLI 与脚本 fd 9 跨持有者互斥；
   - 进程组：group 模式超时 rc=124 且孙进程零泄漏、自然退出路径组扫尾
     零泄漏、fn 模式函数可执行且退出码透传（rc=7）、fn 超时 tree-walk
     清理可见子进程；bash 与 dash 双跑全 PASS。
   - kill 负 pid 兼容性：dash 内建 kill 实测支持 `kill -TERM "-$p"` 组杀；
     busybox kill 走 `bb_strtoi` 接受负 pid，且 `nm_kill_tree` 兜底保证
     异常 shell 下退化为原有语义（无回归）。
3. `git diff` 复核：仅 scope 内两文件，无逻辑外溢。
