# 重构交接文档（HANDOFF.md）

> 面向"下一个接手的人 / 下一个会话"：本页说明当前重构做到哪、怎么验证、怎么继续。
> 各子代理的详细改动记录见 `docs/refactor/journal-*.md`，总进度见根目录 `REFACTOR_ROADMAP.md`。

## 0. 当前状态：Phase 5 单包整合与前端三层统一（2026-10-13）

**仓库形态已从"十余个包"收敛为两个包**，请先读这两份文档再动代码：

| 主题 | 文档 |
|---|---|
| 哪些包被并入谁、哪些保留、为什么，验收判据 | [integration-scope.zh-cn.md](integration-scope.zh-cn.md) |
| 前端三层统一的覆盖范围、调用路径、测试方式、"完美兼容"判据 | [journal-next-into-generic-compat.zh-cn.md](journal-next-into-generic-compat.zh-cn.md) |

要点速览：

- **`qmodem`**（`application/qmodem/`）：一体化后端。C 工程源码在 `src/<组件>/`
  （`libqmodem-sms`、`tom_modem`、`sms-tool_q`、`ubus_at_daemon`、`modem_scan`、
  `qmodem_smsd`、`quectel_CM_5G_M`、`qmodem-seal`），`sms-tool_q` 的两个补丁在
  `patches-src/sms-tool_q/`（**刻意不放在 `patches/`**，避免被 OpenWrt 默认 Patch 阶段
  以错误的 `-p1` 根路径误用）；脚本与服务载荷在 `files/`（120 个文件）。
  Makefile 里 `Build/Compile` 的**顺序即依赖顺序**：`libqmodem-sms` 先构建并发布到
  `$(STAGING_DIR)`，之后 `tom_modem` / `qmodem_smsd` 才能 `-lqmodem-sms` 链接。
- **`luci-app-qmodem`**（`luci/luci-app-qmodem/`）：唯一前端。`qmodem.controls` 是唯一
  对外入口（110 项导出），`qmodem.ui` / `qmodem.theme` / `qmodem.sms` 是内部实现层，
  视图只 `require qmodem.controls`。
- **改完必须跑的门禁**：`node --check` 全量前端 JS、`compat_matrix.py`（端点）、
  `verify_layers.py`（导出面 / 孤儿引用）、`final_check.py`（i18n / 样式类 / CRLF），
  以及 `application/qmodem/tests/*.sh` 中的一致性断言脚本。
- **动手前先确认没有隐藏的跨包同名文件**：两个包都往 `/usr/share/rpcd/acl.d/`、
  `/etc/init.d/` 落文件，历史上曾出现 `luci-app-qmodem.json` 被两包同时安装的冲突，
  现已由前端包单独提供。新增文件前建议先 `grep` 一遍安装规则。

## 1. 本次重构范围与原则

范围：评估报告（高危 26 项）中**不依赖大规模迁移即可落地的全部止血项** + Rust 异步核心的第一块可验证交付（PDU crate + tokio at-daemon 骨架）。

原则：
- 最小化 diff，不做顺手重构；每个改动在对应 journal 里有"问题 → 修复 → 验证"三段记录。
- Rust 代码全部在新 `rust/` 目录，**不替代任何现有组件的打包路径**（避免影响上游构建），作为并行可选项演进。
- LuCI next 的行为修复保持 ubus API 兼容。

## 2. 如何验证（全绿才合并/推送）

```sh
# Rust
cd rust && cargo test --workspace

# LuCI 前端冒烟
node luci/luci-app-qmodem/tests/test_sms_service.js

# Shell 语法（改动的脚本）
sh -n application/qmodem/files/usr/share/qmodem/modem_hook.sh
sh -n application/qmodem/files/usr/share/qmodem/modem_dial.sh
sh -n application/qmodem/files/usr/libexec/rpcd/qmodem

# C 语法级检查
gcc -fsyntax-only -I application/qmodem/src/ubus_at_daemon application/qmodem/src/ubus_at_daemon/*.c
```

## 3. 交接注意事项

- **Token 安全**：本次推送使用环境变量传递 GitHub PAT，任何 token 不得写入仓库文件或 .git/config；会话中暴露过的 token 使用后应立即轮换。
- **已知未修复项**（遗留到 Phase 1-4）：C4 SMS JSON sprintf 边界群、C7 全系竞态（本次只修关键路径）、C8-C14、旧 Lua 栈漏洞（H5-H7，随退役解决）、vendored 上游项目不动。
- **回退方式**：本次改动为单个提交系列，`git revert` 对应提交即可；Rust 目录独立，可直接整体移除。

## 4. 子代理交接日志索引

| 日志 | 覆盖内容 |
|------|----------|
| docs/refactor/journal-c-core.md | W1：at-daemon / tom_modem / libqmodem-sms 的 C 层修复明细 |
| docs/refactor/journal-shell-pkg.md | W2：modem_hook / modem_dial / rpcd / 打包 Makefile 修复明细 |
| docs/refactor/journal-luci-next.md | W3：前端修复明细（死代码、fallback、ACL、时区等） |
| docs/refactor/journal-rust-core.md | W4+W6：rust/ 设计、C→Rust 对照表、零依赖改造（0 外部 crate，二进制 -43.4%） |
| docs/refactor/journal-pkg-suite.md | W7：qmodem-suite 单包交付 meta 包 + 全仓依赖审计表 |
| docs/refactor/journal-vendor-lib.md | W8：cmds/qmodem_lib.sh 公共解析库 + quectel/huawei 试点迁移 |
| docs/refactor/journal-c-smsd.md | W9：smsd C4/C10/C12 修复（含 ASan 自测） |
| docs/refactor/journal-c-scan.md | W10：modem_scand 权限/白名单 + init.d/hotplug 修正 |
| docs/refactor/journal-luci-comp.md | W11：LuCI next 组件化（ui.js / pollGuard） |
| docs/refactor/journal-lua-retire.md | W12：旧栈 H5-H8 止血 + 互斥解除 |
| docs/refactor/journal-monitor-dial.md | W13：monitor/dial 收尾修复 |
| docs/refactor/journal-ci.md | W14：check_all.sh 统一门禁 + quality.yml |
| docs/refactor/journal-tests.md | W16：tests/sh + tests/lua 无依赖测试体系 |

总索引另见 docs/refactor/README.md。
