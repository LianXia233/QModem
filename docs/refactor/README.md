# 重构工程索引（docs/refactor/）

> 本目录存放 QModem 各轮重构的工程过程文档：Phase 0 止血 / Phase 1-3 依赖收缩与 Rust 异步核心 /
> Phase 4 旧栈退役决策 / **Phase 5 单包整合与前端三层统一**。总进度见根目录
> [REFACTOR_ROADMAP.md](../../REFACTOR_ROADMAP.md)，"如何验证、如何继续"见 [HANDOFF.md](HANDOFF.md)。

## 1. 文件清单

| 文件 | 一句话摘要 |
|---|---|
| [REFACTOR_ROADMAP.md](../../REFACTOR_ROADMAP.md) | 唯一权威进度源：工作包状态、路线图、验证门禁、里程碑记录 |
| [HANDOFF.md](HANDOFF.md) | 交接页：重构范围与原则、验证命令、token 安全与回退方式、日志索引 |
| [integration-scope.zh-cn.md](integration-scope.zh-cn.md) | **Phase 5 交付**：包整合范围与保留清单（哪些并入 `qmodem` / `luci-app-qmodem-next`，哪些因系统依赖保留及原因，验收判据 G1-G10） |
| [journal-next-into-generic-compat.zh-cn.md](journal-next-into-generic-compat.zh-cn.md) | **Phase 5 交付**：前端三层统一的覆盖范围、模块与调用路径、四层测试方式、"完美兼容"判定标准 |
| [journal-c-core.md](journal-c-core.md) | W1：ubus_at_daemon / tom_modem / libqmodem-sms 的 C 层修复明细（C1-C7，问题 → 修复 → 验证） |
| [journal-shell-pkg.md](journal-shell-pkg.md) | W2：modem_hook / modem_dial / rpcd 脚本安全修复与打包 Makefile 规范化明细 |
| [journal-luci-next.md](journal-luci-next.md) | W3：luci-app-qmodem-next 前端修复明细（ACL 收敛、fallback、时区、CSS 等） |
| [journal-rust-core.md](journal-rust-core.md) | W4：rust/ 工作区设计说明、C→Rust 逐项对照（P1-P7 / A1-A7）与测试结果 |
| [journal-ci.md](journal-ci.md) | CI/DevOps 门禁固化（agent-ci） |
| [journal-c-scan.md](journal-c-scan.md) | modem_scand + 打包/init 修复（agent-c-scan） |
| [journal-c-smsd.md](journal-c-smsd.md) | C 层 smsd 收尾（C4/C10/C12，agent-c-smsd-v2 接手前任半成品） |
| [journal-lua-retire.md](journal-lua-retire.md) | 旧 Lua 栈止血与退役准备（agent-lua-retire） |
| [journal-luci-comp.md](journal-luci-comp.md) | luci-app-qmodem-next htdocs/** 组件修复（agent-luci-comp） |
| [journal-monitor-dial.md](journal-monitor-dial.md) | qmodem_monitor + modem_dial 安全/健壮性修复（agent-monitor） |
| [journal-pkg-suite.md](journal-pkg-suite.md) | qmodem-suite 元包（agent-pkg-suite） |
| [journal-tests.md](journal-tests.md) | tests/ 轻量回归测试（agent-tests） |
| [journal-vendor-lib.md](journal-vendor-lib.md) | 公共解析库与 vendor 试点迁移（agent-vendor-lib） |
| [journal-vendor-full.md](journal-vendor-full.md) | W17：剩余 10 厂商脚本全量迁移 qmodem_lib（agent-vendor-full） |
| [journal-rpcd.md](journal-rpcd.md) | W18：rpcd 去 eval/原子缓存/POSIX 化（agent-rpcd） |
| [journal-rust-cross.md](journal-rust-cross.md) | W19：Rust 交叉编译脚本、musl 工具链兜底与体积实测（agent-rust-cross + 编排者收尾） |
| [journal-atd-quality.md](journal-atd-quality.md) | W20：at-daemon 剩余清理（agent-atd-quality） |
| [journal-tom-modem.md](journal-tom-modem.md) | W21：tom_modem 去重与 C14 修复（agent-tom-modem） |
| [journal-luci-final.md](journal-luci-final.md) | W22：LuCI next 收尾优化（agent-luci-final） |
| [journal-retire-s1.md](journal-retire-s1.md) | W23：旧栈退役 Step1 与 gap 表（agent-retire-s1） |
| [journal-atd-bridge.md](journal-atd-bridge.md) | W24：at_tool=2 接入 Rust atd（agent-atd-bridge） |
| [journal-ci-p2.md](journal-ci-p2.md) | W26：rust-build.yml 交叉编译矩阵（agent-ci-p2） |
| [journal-vendor-residue.md](journal-vendor-residue.md) | vendor 残留症状清理与字段复核（agent-vendor-residue） |
| [journal-release-assets.md](journal-release-assets.md) | Release 资产散装化（去 tar.gz）+ qmodem-suite 恢复构建（编排者） |
| [../developer/upstream-compat.zh-cn.md](../developer/upstream-compat.zh-cn.md)（[en](../developer/upstream-compat.en.md)） | 与上游（FUjr/QModem 及 modem_feeds）长期同步策略：文件所有权分类（REFACTORED_OWNED 表附 journal 证据）、模组吸收流程、feed/SDK 代际匹配 |

> 上表为 `ls docs/refactor/` 实际文件（截至 2026-10-12）。后续新增 journal（如 CI、cmd 库迁移）请同步补入本清单。

## 2. 门禁命令

统一入口脚本为 `scripts/check_all.sh`（一站式跑 shell / node / js-test / rust / c / crlf 各 stage，`--only <stage>` 单跑、`--changed` 仅检查改动文件，任一 FAIL 退出码非 0）；未覆盖到的检查项以 [HANDOFF.md](HANDOFF.md) 第 2 节的逐项命令为准，推送前必须全绿：

| 门禁 | 命令 | 工作目录 |
|---|---|---|
| Rust 测试 | `cargo test --workspace` | `rust/` |
| Rust lint | `cargo clippy --workspace --all-targets` | `rust/` |
| JS 冒烟 | `node luci/luci-app-qmodem-next/tests/test_sms_service.js` | 仓库根 |
| Shell 语法 | `sh -n` 逐个改动脚本 | 仓库根 |
| C 语法检查 | `gcc -fsyntax-only -I application/qmodem/src/ubus_at_daemon application/qmodem/src/ubus_at_daemon/*.c` | 仓库根 |
| LF 检查 | `sh scripts/check_all.sh --only crlf`（判据为索引中的 `i/crlf`，非工作区行尾） | 仓库根 |
| 残留检查 | `PKG_LINCESE` / `modem_1'` fallback / `28800` / `system(`(event_callback) 计数为 0 | 仓库根 |

## 3. 推送策略

- **网络直连**：CI 环境对 `github.com` 无 TLS 直连时，通过 hosts 固定 IP 直连绕过解析问题（推送机维护，不写入仓库）。
- **原生 git 推送**：确认门禁全绿后，用 `git push` 直接推送提交到 `LianXia233/QModem` 的 main（参考 REFACTOR_ROADMAP.md 里程碑 T1 的两个提交拆分：C/Shell/LuCI 修复一个提交，rust/ + 文档一个提交）。
- **Token 安全**（强制，见 [HANDOFF.md](HANDOFF.md) 第 3 节）：GitHub PAT 只经环境变量传递，不得写入仓库文件或 `.git/config`；会话中暴露过的 token 用后立即轮换。
- **回退**：改动以提交系列为粒度，`git revert` 对应提交即可；`rust/` 目录独立，可整体移除。
