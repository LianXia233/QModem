# Journal: TTL / TTL-fw4 双包审查与退役准备 (agent-retire-ttl)

日期: 2026-10-12
范围: 仅 `luci/luci-app-qmodem-ttl/**`、`luci/luci-app-qmodem-ttlfw4/**` 与本 journal；
声明式标注为主，顺带修复注入/bashism，不改任何功能逻辑。

## 1. 功能清单与后端依赖

两包共用 uci 节 `qmodem_ttl`（`config main 'main'`：`enable`、`ttl`），功能一致：
将 br-lan 出站报文 TTL（IPv4）/ Hop Limit（IPv6）改写为指定值，绕过运营商共享限制；
启用时停用 NSS ECM / SFE 加速模块，停用时恢复。

| 项 | luci-app-qmodem-ttl（旧栈） | luci-app-qmodem-ttlfw4（fw4 变体） |
|---|---|---|
| UI | CBI lua（`modem_ttl.lua` + controller，菜单挂旧栈 `admin/modem/qmodem`，order 22） | JS view（`ttl.js`，menu.d/acl.d 标准三件套，order 22） |
| 依赖 | `+luci-app-qmodem`（旧栈 UI 入口） | `+luci-base` |
| 后端 | **iptables**：`iptables/-t mangle POSTROUTING TTL --ttl-set` + `ip6tables ... HL --hl-set`，写入 `/etc/firewall.d/qmodem_ttl` 后执行；依赖 uci-defaults 注册 firewall `include type=script` + ucitrack | **nft/fw4**：生成 `/etc/nftables.d/99-reset-ttl-from-br-lan.nft`（`iifname "br-lan" ip ttl set / ip6 hoplimit set`，filter hook postrouting priority mangle）；依赖 fw4 内建 nftables.d include；另强制关闭 flow_offloading(_hw) |
| 互斥 | `PKG_CONFLICTS:=luci-app-qmodem-ttlfw4`（共用 `/etc/config/qmodem_ttl`） | 同左，双向 |

已知遗留（未改，属逻辑变更）：ttlfw4 init.d:34 仍执行 `/etc/firewall.d/qmodem_ttl`，
但该包既不创建也不注册该文件（fw4 走 nftables.d include），疑似旧版残留（失败静默）。

## 2. 重叠度分析与去留建议

- **重叠度 ≈ 90%**：同一 uci 数据模型、同一功能语义、init.d 逻辑同源；差异仅在 UI 技术栈（CBI vs JS）与防火墙后端（iptables vs fw4/nft）。
- **next 覆盖情况**：`luci-app-qmodem-next` 无任何 TTL 视图、菜单项或 ubus 端点（grep 仅命中 `allSettled` 等无关词）；journal-retire-s1 端点 Gap 表亦记录"整体 gap"。
- **建议**：
  - 两包**合并为一个包**（保留 ttlfw4 的 JS UI + fw4/nft 后端，iptables 变体随 fw3 时代淘汰），可消除 CONFLICTS 互斥与双份维护；
  - 在 next 提供 TTL 视图（纯 uci + `rc`/`luci setInitAction` 即可，无需新 ubus 端点）后**整体退役**两包（Phase 4）；
  - 迁移前**维持现状**：两包均 legacy/maintenance mode，不再新增功能。ttlfw4 依赖更轻（仅 luci-base），若需立即减包可先弃用 ttl 旧栈包。

## 3. 本次改动（声明式标注 + 顺带修复）

1. **Makefile**：ttl 与 ttlfw4 的 `LUCI_TITLE` 均已含 `(legacy, maintenance mode)`（journal-retire-s1 已完成，本次复核无需再改）；为 ttlfw4 Makefile 补一行 legacy 维护模式中文注释（ttl 侧已有对应注释，对齐）。
2. **htm 退役注释**：两包**均无 htm**（ttl 为 CBI lua、ttlfw4 为 JS 视图）。按 journal-lua-retire/mwan 先例做等价标注：
   - `ttl/luasrc/controller/qmodem_ttl.lua:1`、`ttl/luasrc/model/cbi/qmodem/modem_ttl.lua:1` 加 `-- legacy: superseded by luci-app-qmodem-next (no ttl page there); scheduled for removal in Phase 4`
   - `ttlfw4/htdocs/luci-static/resources/view/qmodem/ttl.js:1` 加同义 `//` 注释（置于 `'use strict'` 之前的指令序言区，合法且不影响求值）。
3. **bashism 修复**（两包 init.d 同一处）：`[ "$enable" == 0 ]` → `[ "$enable" = 0 ]`（POSIX test；busybox ash 虽容忍 `==`，仍按规范写法）。
4. **注入修复**（两包 init.d `set_if_ttl`）：uci 读取的 `$ttl` 此前未校验即拼接进
   - ttl：`echo "... --ttl-set $ttl ..." >> /etc/firewall.d/qmodem_ttl` 随后**执行该 shell 脚本** → shell 注入；
   - ttlfw4：heredoc 写入 nft 规则 → nft 语法注入。
   统一加 `case "$ttl" in ''|*[!0-9]*) logger ...; return;; esac` 纯数字白名单（UI 的 `datatype=uinteger`/`range(1,255)` 仅为前端约束）。

## 4. 验证记录

1. `sh -n` 通过：两包 init.d、uci-defaults（共 3 个 shell 文件）。
2. `git diff` 复核：仅上述 7 个文件的注释/校验插入，**无功能逻辑变更**；diff 中其余文件为工作区既有未提交改动，本任务未触碰。
3. JS 注释位置确认位于 `'use strict'` 前（指令序言允许前置注释）。
4. 未触碰 `luci-app-qmodem-next/**`；两包 Makefile 的 `PKG_CONFLICTS` 互斥关系保持不变。
