# 重构工程索引（docs/refactor/）

> 本目录存放 QModem 第一轮重构（Phase 0 止血 + Rust 异步核心）的工程过程文档。总进度见根目录 [REFACTOR_ROADMAP.md](../../REFACTOR_ROADMAP.md)，"如何验证、如何继续"见 [HANDOFF.md](HANDOFF.md)。

## 1. 文件清单

| 文件 | 一句话摘要 |
|---|---|
| [REFACTOR_ROADMAP.md](../../REFACTOR_ROADMAP.md) | 唯一权威进度源：工作包状态、路线图、验证门禁、里程碑记录 |
| [HANDOFF.md](HANDOFF.md) | 交接页：重构范围与原则、验证命令、token 安全与回退方式、日志索引 |
| [journal-c-core.md](journal-c-core.md) | W1：ubus_at_daemon / tom_modem / libqmodem-sms 的 C 层修复明细（C1-C7，问题 → 修复 → 验证） |
| [journal-shell-pkg.md](journal-shell-pkg.md) | W2：modem_hook / modem_dial / rpcd 脚本安全修复与打包 Makefile 规范化明细 |
| [journal-luci-next.md](journal-luci-next.md) | W3：luci-app-qmodem-next 前端修复明细（ACL 收敛、fallback、时区、CSS 等） |
| [journal-rust-core.md](journal-rust-core.md) | W4：rust/ 工作区设计说明、C→Rust 逐项对照（P1-P7 / A1-A7）与测试结果 |

> 上表为 `ls docs/refactor/` 实际文件（截至 2026-10-12）。后续新增 journal（如 CI、cmd 库迁移）请同步补入本清单。

## 2. 门禁命令

统一入口脚本为 `scripts/check_all.sh`（一站式跑 shell / node / js-test / rust / c / crlf 各 stage，`--only <stage>` 单跑、`--changed` 仅检查改动文件，任一 FAIL 退出码非 0）；未覆盖到的检查项以 [HANDOFF.md](HANDOFF.md) 第 2 节的逐项命令为准，推送前必须全绿：

| 门禁 | 命令 | 工作目录 |
|---|---|---|
| Rust 测试 | `cargo test --workspace` | `rust/` |
| Rust lint | `cargo clippy --workspace --all-targets` | `rust/` |
| JS 冒烟 | `node luci/luci-app-qmodem-next/tests/test_sms_service.js` | 仓库根 |
| Shell 语法 | `sh -n` 逐个改动脚本 | 仓库根 |
| C 语法检查 | `gcc -fsyntax-only -I application/ubus_at_daemon/inc application/ubus_at_daemon/src/*.c` | 仓库根 |
| LF 检查 | 改动文件无 CRLF | 仓库根 |
| 残留检查 | `PKG_LINCESE` / `modem_1'` fallback / `28800` / `system(`(event_callback) 计数为 0 | 仓库根 |

## 3. 推送策略

- **网络直连**：CI 环境对 `github.com` 无 TLS 直连时，通过 hosts 固定 IP 直连绕过解析问题（推送机维护，不写入仓库）。
- **原生 git 推送**：确认门禁全绿后，用 `git push` 直接推送提交到 `LianXia233/QModem` 的 main（参考 REFACTOR_ROADMAP.md 里程碑 T1 的两个提交拆分：C/Shell/LuCI 修复一个提交，rust/ + 文档一个提交）。
- **Token 安全**（强制，见 [HANDOFF.md](HANDOFF.md) 第 3 节）：GitHub PAT 只经环境变量传递，不得写入仓库文件或 `.git/config`；会话中暴露过的 token 用后立即轮换。
- **回退**：改动以提交系列为粒度，`git revert` 对应提交即可；`rust/` 目录独立，可整体移除。
