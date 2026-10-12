# 重构交接文档（HANDOFF.md）

> 面向"下一个接手的人 / 下一个会话"：本页说明当前重构做到哪、怎么验证、怎么继续。
> 各子代理的详细改动记录见 `docs/refactor/journal-*.md`，总进度见根目录 `REFACTOR_ROADMAP.md`。

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

# LuCI next 冒烟
node luci/luci-app-qmodem-next/tests/test_sms_service.js

# Shell 语法（改动的脚本）
sh -n application/qmodem/files/usr/share/qmodem/modem_hook.sh
sh -n application/qmodem/files/usr/share/qmodem/modem_dial.sh
sh -n application/qmodem/files/usr/libexec/rpcd/qmodem

# C 语法级检查
gcc -fsyntax-only -I application/ubus_at_daemon/inc application/ubus_at_daemon/src/*.c
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
| docs/refactor/journal-rust-core.md | W4：rust/ 工作区设计说明与测试结果 |
