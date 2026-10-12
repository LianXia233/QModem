# 旧栈退役 Step 2 前置：luci-app-qmodem-mwan 审查与去留结论

- **agent**: agent-retire-mwan
- **日期**: 2026-10-12
- **范围**: `luci/luci-app-qmodem-mwan/**`（仅声明式标注 + 注入/bashism 修复）+ 本 journal；不碰 next、不改功能逻辑。

## 1. 包体功能清单与后端依赖表

| 文件 | 功能 | 后端依赖 |
|---|---|---|
| `luasrc/controller/qmodem_mwan.lua` | 菜单入口 `admin/modem/qmodem/mwan_config`（依赖 `/etc/config/qmodem_mwan` 存在才挂载） | LuCI dispatcher；无 ubus/自定义端点 |
| `luasrc/model/cbi/qmodem/mwan_config.lua` | CBI 配置页：`qmodem_mwan` 全局开关/sticky 模式/超时 + ipv4 成员表（接口/track IP/优先级/权重），接口候选来自 `network` uci | uci: `qmodem_mwan`（读写）、`network`（只读枚举） |
| `root/etc/init.d/qmodem_mwan` | procd init：读 `enable_mwan`，调用 modem_mwan.sh 生成/回收 mwan3 配置 | 外部命令: `/usr/share/qmodem/modem_mwan.sh`、`/etc/init.d/mwan3`、logger；uci: `qmodem_mwan` |
| `root/usr/share/qmodem/modem_mwan.sh` | 核心生成器：按 `qmodem_mwan` ipv4 节批量写 `mwan3` uci（interface/member/policy/rule，均带 `add_by=modem` 标记，可整体 flush） | 外部命令: `uci`（batch/set/add_list/delete/commit）、`/etc/init.d/mwan3 stop|start`、logger、config_load/config_foreach（/lib/functions.sh）；uci 节: `mwan3` 全量、`qmodem_mwan` 只读 |
| `root/etc/udhcpc.user.d/01-reload-mwan3.dhcpscript` | DHCP bound 且 mwan3_hook 缺失时 restart mwan3 | 外部命令: iptables、uci、mwan3 |
| `root/etc/uci-defaults/99-setup-mwan` | 三款设备（HC-G80/ailf gs2410/huasifei ws3006）预置多拨成员 | uci: `qmodem_mwan`、board_name |
| `root/etc/uci-defaults/luci-app-modem-mwan` | ucitrack 注册 + 清 LuCI 缓存 | uci: `ucitrack` |
| `root/etc/config/qmodem_mwan` | 默认配置（wan 单成员 + 全局开关） | — |
| `po/*` | zh-cn 翻译 | — |

**无 ubus 方法调用、无 HTTP 自定义端点**：整个包是纯 uci→mwan3 的离线生成器，仅依赖 `+luci-app-mwan3` 提供的 mwan3 引擎。

## 2. 对照 luci-app-qmodem-next 的覆盖结论（更新 journal-retire-s1 §3 gap 表 mwan 行）

核查依据：`luci-app-qmodem-next` 的 `menu.d/luci-app-qmodem-next.json`（8 个页面，无任何 mwan 路径）、`acl.d/luci-app-qmodem-next.json`（uci 白名单仅 `qmodem`/`sms_forwarder`，无 `mwan3`/`qmodem_mwan`）、`htdocs/.../qmodem.js`（540 行，`grep -i mwan` 零命中）。

**journal-retire-s1 gap 表 mwan 行更新结论：`保留`（legacy, maintenance mode，不迁移、不弃用）**

- **理由（不迁移）**：next 无对应页面且 ACL 未含 mwan3/qmodem_mwan uci，迁移需新建 JS 视图 + ACL + 后端联动，成本高；而本包功能与上游 `luci-app-mwan3` 官方 UI 高度重叠，自研页面价值有限。
- **理由（不弃用）**：99-setup-mwan 为 HC-G80 等 3 款量产设备预置多拨成员，弃用直接破坏设备出厂行为；且 modem_mwan.sh 的 `add_by=modem` 增量管理逻辑（不碰用户手写 mwan3 节）上游不具备。
- **Phase 4 建议**：保持现状随旧栈共存（Makefile 已含 legacy 标注 + 注释说明依赖仅作旧栈 UI 入口），不阻塞 next 主线；未来若 next 需要 mwan 能力，优先引导用户直接使用 luci-app-mwan3 而非迁移本包。

## 3. 本次改动（声明式标注 + 安全修复）

1. **Makefile**：`LUCI_TITLE` 已由 agent-retire-s1 追加 `(legacy, maintenance mode)`（Makefile:10），本次复核无重复、无需改动。
2. **htm 退役注释**：本包**无任何 htm**（CBI lua 栈，s1 journal 已确认 mwan/ttl 无 htm），等价声明式注释以 Lua 注释形式写入两处 lua 文件首部，不改任何逻辑：
   - `luasrc/controller/qmodem_mwan.lua:1`
   - `luasrc/model/cbi/qmodem/mwan_config.lua:3`
3. **Phase 0 未覆盖的 bashism 修复**（journal-shell-pkg 未涉及本包脚本）：
   - `root/etc/udhcpc.user.d/01-reload-mwan3.dhcpscript:1,3`：`[ x == y ]` ×2 → `=`（POSIX）。
4. **Phase 0 未覆盖的注入面修复**（modem_mwan.sh 将 uci 配置值无校验拼入 `uci batch` heredoc）：
   - 新增 `valid_name()`（仅 `[A-Za-z0-9_]`），`handle_config` 对 `member_interface` 校验，非法记 syslog 并跳过（ modem_mwan.sh:6-12、:137-140）；
   - heredoc 内值全部加引号：`interface/metric/weight`（:56-58）、`sticky`（:123）、`add_list ...track_ip="$ip"`（:41），阻断含空格/换行/分号的值改变 batch 语义。
5. **观察项（未改，留档）**：Makefile conffiles 写的是 `/etc/config/modem_mwan`，实际配置文件为 `/etc/config/qmodem_mwan`，疑似历史笔误；涉及打包行为变更，超出本次"声明式标注"范围，建议 Phase 4 退役时一并处理。

## 4. 验证记录

1. `sh -n`：包内全部 5 个 shell 脚本（modem_mwan.sh / init.d / dhcpscript / 2×uci-defaults）通过；`luac -p` 两个改动 lua 通过。
2. `git diff -- luci/luci-app-qmodem-mwan` 复核：4 文件，+21/−7；删除行仅 6 处 bashism/未引号值原行，新增行仅 lua 注释 2 行 + valid_name/校验/引号化；**无可观测行为变化**（合法配置下生成结果与原逻辑逐字节一致）。
3. 编码检查：`grep -rl $'\r'` 无命中，全包 LF；`file` 确认 POSIX shell script / ASCII text。
