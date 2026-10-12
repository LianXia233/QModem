# QModem 重构任务路线图（REFACTOR_ROADMAP.md）

> 本文档是本次重构的**唯一权威进度源**，由编排者（主会话）统一维护。
> 各子代理只写自己的交接日志 `docs/refactor/journal-<组件>.md`，不直接改本文档，避免并发冲突。
> 每完成一个里程碑，编排者立即更新本文档并推送。

- 基线：`main@c49654e`（与 FUjr/QModem 同步）
- 目标仓库：`LianXia233/QModem`
- 纪律：所有文件 LF；最小化 diff；每个 Phase 结束可独立构建/回退；验证失败回退已知良好基线。

## 状态总览（最后更新：T1 全部工作包完成，验证门禁全绿）

| # | 工作包 | 负责子代理 | 文件域（互斥，防冲突） | 状态 | 交接日志 |
|---|--------|-----------|------------------------|------|----------|
| W1 | C 层内存安全与注入修复 | agent-c-core | `application/ubus_at_daemon/src/*`、`application/tom_modem/src/*`、`application/libqmodem-sms/src/*` | ✅ 完成（gcc -fsyntax-only 全过） | docs/refactor/journal-c-core.md |
| W2 | Shell 脚本与打包修复 | agent-shell-pkg | `application/qmodem/files/**`、`application/*/Makefile`、`luci/*/Makefile` | ✅ 完成（sh -n 全过，sq() 自检 6 项） | docs/refactor/journal-shell-pkg.md |
| W3 | LuCI next 前端重构 | agent-luci-next | `luci/luci-app-qmodem-next/htdocs/**`、`luci/luci-app-qmodem-next/root/**` | ✅ 完成（node --check 9 文件 + 冒烟 PASS） | docs/refactor/journal-luci-next.md |
| W4 | Rust 异步核心（pdu + atd） | agent-rust-core | `rust/**`（全新目录） | ✅ 完成（cargo test 24 passed，clippy 0 warning） | docs/refactor/journal-rust-core.md |
| W5 | 验证、文档整合、推送 | 编排者 | `REFACTOR_ROADMAP.md`、`docs/refactor/HANDOFF.md` | ✅ 完成（门禁全绿，经 GitHub API 推送） | — |

## 路线图（对应评估报告第 9 节）

```
Phase 0 止血（本次执行）
├─ W1 C 层高危：C1 未初始化响应 / C2 event_callback 注入 / C3 tty fd 泄漏
│        C5/C6 PDU 边界与恒假条件 / C7 引用计数竞态（关键路径）
├─ W2 Shell：S1/S2 modem_hook 反写与变量错位 / S3 PIN 明文 / S4 凭据注入
│        S5 rpcd 路径穿越 / S6 二次 read / 打包 H9/H10/LINCESE 拼写
├─ W3 LuCI next：H1 ACL / H2 modem_1 fallback / H3 死代码 XSS 模式
│        H4 时区硬编码 / M1 CSS 重复 / M5 NaN 进度条 / M4 scan reload
└─ W4 Rust 异步核心：rust/qmodem-pdu（PDU 编解码，cargo test 通过）
         rust/qmodem-atd（tokio 异步 AT 守护进程：per-port 命令队列、
         单调时钟、无裸指针共享态；ubus FFI feature 门控）

Phase 1-4（后续，见评估报告）：vendor 层配置驱动下沉 → smsd/rpcd 迁移 → 旧 Lua 栈退役
```

## 验证门禁（推送前必须全绿）

| 门禁 | 命令 | 状态 |
|------|------|------|
| Rust 测试 | `cargo test --workspace`（rust/ 下） | ✅ 24 passed / 0 failed；`cargo clippy` 0 warning |
| JS 冒烟测试 | `node luci/luci-app-qmodem-next/tests/test_sms_service.js` | ✅ PASS |
| Shell 语法 | `sh -n` 全量改动脚本 | ✅ 3/3 通过 |
| C 编译检查 | `gcc -fsyntax-only`（/tmp stub 头补齐 libubus 等系统头） | ✅ 6 个修改文件全过 |
| LF 检查 | 改动文件无 CRLF（Python 二进制校验 30 个文件） | ✅ 0 违规 |
| 残留检查 | `PKG_LINCESE` / `modem_1'` fallback / `28800` / `system(`(event_callback) | ✅ 全部为 0 |

## 里程碑记录

- **T0（2026-10-12 11:15）**：路线图与交接文档建立，4 个子代理并行启动（W1-W4）。
- **T1（2026-10-12 11:45）**：W1-W4 全部完成并复核；编排者复跑全部门禁通过；分两个提交经 GitHub API 推送至 `LianXia233/QModem` main：
  - commit A `fix: ...`——W1/W2/W3（C 层 + Shell + 打包 + LuCI next，27 文件）
  - commit B `feat(rust): ...`——W4 + 重构文档（rust/ 工作区、REFACTOR_ROADMAP、docs/refactor/*）
