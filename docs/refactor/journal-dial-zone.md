# Journal: modem_dial.sh wan zone 确定性遍历 + qmi_dial 退避复核（agent-dial-zone）

日期: 2026-10-12
范围: `application/qmodem/files/usr/share/qmodem/modem_dial.sh`（唯一改动文件）

## 背景

S9-低任务：`set_if()` 中 `num=$(uci show firewall | grep "name='wan'" | wc -l)`
把"名为 wan 的 zone 个数"当作"wan zone 的下标"使用，多 zone 且 wan 非最后
一个时下标错误，会把接口挂到错误 zone；firewall 无 wan zone 时 `grep|wc -l`
得 0，静默默认 @zone[0]。

## 改动

1. 新增 `find_wan_zone_index()`（约 modem_dial.sh:705）：按用户给定的确定性
   遍历模板 `i=0; while uci -q get firewall.@zone[$i] ...` 逐个比对
   `.name`，找到写 `wan_zone_index` 返回 0；遍历完返回 1。
2. `set_if()`（约 :854）改为调用上述函数：未找到时
   `logger -t qmodem "error: no firewall zone named 'wan' found, skip zone
   operations for $interface_name"`，`num` 保持空串，跳过全部 zone 相关
   操作（不默认 0）。
3. 新增 `zone_has_network()`（约 :722）：遍历 zone 的 network 列表做成员
   判断，替换原有 `grep -w | wc -l` 两处（wwan/wwan6）；调用点守卫
   `[ -n "$num" ]`，与"未找到即跳过"语义一致。
4. 同文件其余 `grep|wc -l` 排查：`check_logfile_line` 的 `wc -l $log_file`
   是日志行数统计、`ensure_bridge_management_interface` 的 `wc -w` 是
   octet 计数，均非下标推算，不属本模式，保留。
5. qmi_dial 退避复核（Phase 1 遗留）：原逻辑对 quectel-CM 的**每次退出**
   都计失败，长时间在线后掉线也会累积到 3 次触发 30s 退避，违背
   "失败计数器在成功后要清零"。改为记录 `date +%s` 起始时间，`wait` 返回
   后计算运行时长：≥60s 视为一次成功会话（清零计数器，仅记掉线重连）；
   <60s 才计失败，连续 3 次退避 30s 后清零。targeted wait 复核结论：
   `_hang` 先 `procd_kill` 杀整个 dial 实例（qmodem_network:99）再跑
   hang，循环内无需响应信号退出，边界无遗漏。

## 验证

- `bash -n` + `dash -n` 均通过；`file` 确认 POSIX shell script、UTF-8（LF）。
- /tmp uci mock（临时 UCI_CONFIG_DIR + PATH 前置 mock uci，按
  `key=value` 平面文件模拟匿名 zone 段），dash 与 bash 双解释器运行
  5 组断言，全部 PASS：
  - wan 在 4 zone 的下标 1 → idx=1；
  - wan 在 3 zone 的下标 2（旧 grep|wc 会得 1 的 off-by-one 场景）→ idx=2；
  - wan zone 不含接口 → membership=no；
  - 无 wan zone → idx=none（zone 操作跳过，不默认 0）；
  - 空 firewall → idx=none。
  - 临时目录 /tmp/qmodem_uci_test 与测试脚本已删除。
- `git diff` 复核：仅本文件，+50/-10，无其他文件变动。
