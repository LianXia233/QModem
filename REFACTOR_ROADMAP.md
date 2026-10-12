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
- **T1（2026-10-12 11:45）**：W1-W4 全部完成并复核；编排者复跑全部门禁通过；分两个提交经 git 原生推送（hosts 直连）至 `LianXia233/QModem` main：
  - commit A `fix: ...`——W1/W2/W3（C 层 + Shell + 打包 + LuCI next，27 文件）
  - commit B `feat(rust): ...`——W4 + 重构文档（rust/ 工作区、REFACTOR_ROADMAP、docs/refactor/*）
- **T2（2026-10-12 12:50）**：Phase 1 —— 11 个子代理并行（W6-W16），**依赖收缩与单包交付**战略落地：

| # | 工作包 | 子代理 | 状态 |
|---|--------|--------|------|
| W6 | Rust 零依赖改造（去 tokio/serde/thiserror/libc → 纯 std 线程-actor + 自研 JSON） | agent-rust-zero | ✅ 37 tests；二进制 -43.4%（748KB） |
| W7 | 单包交付 qmodem-suite meta 包 + 依赖审计（qmodem 补 +jsonfilter） | agent-pkg-suite | ✅ |
| W8 | 厂商解析公共库 cmds/qmodem_lib.sh + quectel/huawei 试点（46 处管道） | agent-vendor-lib | ✅ fixture 回归 30/30 |
| W9 | smsd C4/C10/C12/COMMIT 修复（agent-c-smsd 中断后由 -v2 续完） | agent-c-smsd-v2 | ✅ ASan 自测 17/17 |
| W10 | modem_scand 安全加固（socket 0660+组、白名单）+ init.d/hotplug 修正 | agent-c-scan | ✅ |
| W11 | LuCI next 组件化（ui.js 共享组件、pollGuard、CSS 收敛） | agent-luci-comp | ✅ |
| W12 | 旧栈止血（H5-H8 注入/XSS/全局泄漏）+ 解除互斥依赖 | agent-lua-retire | ✅ luac 16/16 |
| W13 | legacy monitor 白名单 + dial 脚本 S9/S12/S13 收尾 | agent-monitor | ✅ 18/18 断言 |
| W14 | CI 统一门禁 scripts/check_all.sh + quality.yml | agent-ci | ✅ 全绿 |
| W15 | 文档：async-daemon 开发文档、install-suite 指南、README 双语小节 | agent-docs | ✅ |
| W16 | 无依赖测试体系 tests/sh + tests/lua | agent-tests | ✅ 18+16 用例 |

  教训记录：agent-c-smsd 首轮因命令失败后原样重试陷入循环，留下 `&db->` 半成品；由 agent-c-smsd-v2 按「绝对路径 + 失败先分析再重试」纪律续完。
  门禁复核（编排者）：check_all 6/6 PASS（sh 69、node 27、rust 37 tests、crlf 0）、tests/sh 18 PASS、lua smoke 16 PASS；rpcd `valid_section` 的 `${#$1}`→`${#1}`（dash 兼容）由编排者直接修复。

## Phase 2-4（后续）：vendor 全量迁移 → smsd/rpcd 迁 Rust → 旧 Lua 栈退役

- **T3（2026-10-12 17:45）**：Phase 2 —— 10 个子代理并行（W17-W26），vendor 全量迁移 + Rust 接入链路落地：

| # | 工作包 | 子代理 | 状态 |
|---|--------|--------|------|
| W17 | 剩余 10 个厂商脚本迁移 qmodem_lib（simcom 113 处/meig 64 处等） | agent-vendor-full | ✅ fixture 回归通过 |
| W18 | rpcd 瘦身：去 eval/原子缓存/POSIX 化/ubus 样板收敛 | agent-rpcd | ✅ bash+dash 双语法检查 |
| W19 | cross-build-rust.sh（aarch64/x86_64 musl）+ 产物 dist | agent-rust-cross | ✅ native 实测通过 |
| W20 | at-daemon 剩余清理（monitor 持锁两阶段、config_loader 边界、token 随机化） | agent-atd-quality | ✅ |
| W21 | tom_modem 去重（sms_read 合并、表驱动 match）+ C14 deadline | agent-tom-modem | ✅ |
| W22 | LuCI next 收尾（Dropdown API 化、列表上限参数化、i18n 补齐、Promise.allSettled） | agent-luci-final | ✅ |
| W23 | 旧栈退役 Step1（Makefile 标注 + gap 表，Phase 4 输入） | agent-retire-s1 | ✅ |
| W24 | at_tool=2 接入 qmodem-atd unix socket（安装即用的 Rust AT 通路） | agent-atd-bridge | ✅ 端到端模拟验证 |
| W25 | 文档：at-tool-selection 三通路对照 + install-suite 追加 | agent-docs-p2 | ✅ |
| W26 | CI：rust-build.yml 交叉编译矩阵 + check_all 新 stage | agent-ci-p2 | ✅ |

  门禁复核（编排者）：check_all 8 stage 全 PASS/SKIP 合规、cargo 37 tests、tests/sh 18 PASS、at_tool=2 端到端验证、vendor lib 调用覆盖 12/12 厂商文件；scripts/dist/ 构建产物已 gitignore。

- **T4（2026-10-12 18:20）**：Phase 3 —— 20 个子代理并行（W27-W44）+ 3 个补发（W45-W47），Rust 深化与 gap 补齐：

| # | 工作包 | 子代理 | 关键产出 |
|---|--------|--------|----------|
| W27 | ubus FFI 实装（blobmsg 纯 Rust 编解码 + mock transport 全流程测试） | agent-ffi-ubus | 47/52 tests，跨语言格式锁定 |
| W28 | qmodem-smsd Rust 骨架（Storage trait/claim/退避/调度/URC） | agent-smsd-rs | 10 tests，零依赖 |
| W29 | PDU 一致性测试 14→64（TS 23.038 向量/交叉/property） | agent-pdu-conform | **发现并修复 SCTS 时区真 bug** |
| W30 | atd 微基准（往返 p50=50ms/吞吐 31.4k cmd/s/广播 6.8µs） | agent-atd-bench | docs/refactor/bench-atd.md |
| W31 | rpcd gap 端点：get_stats/clear_stats/get_current_band | agent-rpcd-gap | 53 断言全绿 |
| W32 | next 前端 stats 视图 + ACL/menu + debug band 行 | agent-luci-gap | gap「流量统计/当前频段」闭环 |
| W33-36 | 旧栈退役审查 mwan/ttl/voip/hc（结论：mwan 保留、ttl 合并待迁、voip 独立、hc 需实机验证） | agent-retire-* | 顺带修注入/bashism |
| W37 | vendor 残留清理（重复分支 0、huawei 8 处收编） | agent-vendor-residue | 字节级对比证据 |
| W38 | dial wan zone 确定性遍历 + qmi_dial 退避计数器清零修正 | agent-dial-zone | mock uci 5 组断言 |
| W39-40 | at-daemon/scand C 收尾（警告清零、pdu_decoder 有界、uci get 取值） | agent-atd-c-final / scan-c-final | -Wall -Wextra 0 |
| W41 | libqmodem-sms + sms-tool_q 审计修复（pdu_lib 同源 C5/C6 高危） | agent-sms-c-audit | ASan 模糊 20 万轮 0 报告 |
| W42 | vendored 上游审计（**qfirehose 3 个高危**：system 注入/XML 溢出/usb2tcp 无鉴权） | agent-vendored-audit | docs/security/vendored-audit.md |
| W43 | release.yml（CHANGELOG→Release 自动化）+ markdownlint | agent-ci-release | 8 workflow 校验 |
| W44 | 文档英文化 ×3 + README 链接 | agent-docs-en | 双向互链 |
| W45 | UCI 选项参考文档（73 选项/中英）+ config 注释 | agent-uci-docs | docs/reference/ |
| W46 | monitor 心跳 schema version + per-modem 互斥锁 + 进程组 kill | agent-monitor-hard | dash 双跑断言 |
| W47 | **单包交付断链修复**：qmodem 5 个 init.d 未被安装、suite enable 缺失 | agent-pkg-verify | 首装即用闭环 |

  编排者收尾：pdu README SCTS 语义更新、ql_regex_pick 注释失实修正。
  门禁复核：check_all 7 PASS+1 SKIP 合规、cargo test 113（pdu 64/atd 39/smsd 10）、tests/sh 35 PASS、lua 16 PASS。
