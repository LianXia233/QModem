# 与上游（FUjr/QModem 及其 modem_feeds）长期同步策略

- **状态**：现行策略（Active）
- **日期**：2026-10-12
- **关联**：[branch-policy.zh-cn.md](../release/branch-policy.zh-cn.md)、[../adr/0001-refactor-phase4-decisions.zh-cn.md](../adr/0001-refactor-phase4-decisions.zh-cn.md)、[../refactor/README.md](../refactor/README.md)、[../../REFACTOR_ROADMAP.md](../../REFACTOR_ROADMAP.md)

本文定义本仓库（重构线）如何从上游 `FUjr/QModem`（及其承载仓库 `FUjr/modem_feeds`，见
`version.mk:1` "Unified version management for modem_feeds" 与 README 构建徽章）长期同步，
同时保住 Phase 0–4 的重构成果。核心原则：**重构面文件永不盲合，未重构文件可安全快进。**

## 1. 同步节奏与命令

### 1.1 远端与镜像

上游 fetch 统一走镜像前缀 `https://gh.acg2.mom/https://github.com/<owner>/<repo>.git`
（当前 `.git/config` 中 `upstream` 远端即此形式）。这遵循
[docs/refactor/README.md](../refactor/README.md) §3 的网络策略：镜像/hosts 方案由推送机维护，
不写入仓库文件。

```sh
# 拉取上游（FUjr/QModem 与 FUjr/modem_feeds 各配一个 remote）
git fetch upstream main stable --tags
# 查看上游新增提交
git log --oneline HEAD..upstream/main
```

### 1.2 节奏

| 时机 | 动作 |
| --- | --- |
| 上游发布新版本（tag/ReleaseNote） | 全量同步评估：先跑 §2 分类，再逐类合并 |
| 上游新增模组支持（`modem_support.json` / vendor 变更） | 走 §3 吸收流程 |
| 上游 CI/打包修复 | 评估后按 §2 分类合入 |
| 每次重构 journal 落盘前 | 确认没有引用上游尚未包含的文件（避免反向漂移） |

建议每月至少一次例行 fetch；连续两个上游版本未同步时，须先补齐再开始新的重构工作包。

### 1.3 同步报告（规划中）

仓库目前**尚无** `upstream-sync-report` 工作流（截至 2026-10-12，`.github/workflows/` 无此文件）。
规划形态：定时 `workflow_dispatch`/schedule 任务，fetch 镜像后输出
`HEAD..upstream/main` 的变更文件清单，并按 §2 分类表标注每份文件的归属建议，作为 artifact 发布。
在它落地前，用上面的手工命令 + §2 表格替代，**不得**在文档或提交信息中假装该工作流已存在。

## 2. 文件所有权分类

### 2.1 判定方法（权威来源）

一个文件属于哪一类，以 `docs/refactor/journal-*` 是否涉及为准：

```sh
# 列出所有被重构 journal 点名的路径（示例检索）
grep -rhoE '(application|luci|rust|scripts|docs/refactor)/[A-Za-z0-9_./-]+' \
  docs/refactor/journal-*.md | sort -u
```

### 2.2 REFACTORED_OWNED：必须人工三方合并

以下文件在本轮重构中被实质改写，直接覆盖会丢失安全/质量修复。同步时逐文件
`git merge-file` / 手工三方合并，合并后必须过 [docs/refactor/README.md](../refactor/README.md) §2 门禁。

| 文件/目录 | 重构内容 | 证据（journal） |
| --- | --- | --- |
| `application/qmodem/files/usr/share/qmodem/cmds/qmodem_lib.sh` | 新建公共解析库（csv/regex 等） | [journal-vendor-lib.md](../refactor/journal-vendor-lib.md)、[journal-tests-expand.md](../refactor/journal-tests-expand.md) |
| `application/qmodem/files/usr/share/qmodem/vendor/*.sh`（12 个厂商脚本） | 全量迁移 qmodem_lib、残留清理、ql_regex_pick 加固 | [journal-vendor-full.md](../refactor/journal-vendor-full.md)、[journal-vendor-residue.md](../refactor/journal-vendor-residue.md)、[journal-regex-pick.md](../refactor/journal-regex-pick.md) |
| `application/qmodem/files/usr/libexec/rpcd/qmodem` | 去 eval、原子缓存、bashism 清零、get_stats/get_current_band 新端点 | [journal-rpcd.md](../refactor/journal-rpcd.md)、[journal-rpcd-gap.md](../refactor/journal-rpcd-gap.md)、[journal-tests-expand.md](../refactor/journal-tests-expand.md) |
| `rust/`（qmodem-pdu / qmodem-atd / qmodem-smsd） | Rust 异步核心与测试 | [journal-rust-core.md](../refactor/journal-rust-core.md)、[journal-pdu-conform.md](../refactor/journal-pdu-conform.md)、[journal-ffi-ubus.md](../refactor/journal-ffi-ubus.md)、[journal-smsd-rs.md](../refactor/journal-smsd-rs.md)、[bench-atd.md](../refactor/bench-atd.md) |
| `application/qmodem/src/ubus_at_daemon/` | C 层内存安全修复（C1–C7）、M1–M5 收尾 | [journal-c-core.md](../refactor/journal-c-core.md)、[journal-atd-quality.md](../refactor/journal-atd-quality.md) |
| `application/qmodem/src/tom_modem/` | 去重与 C14/C15 修复 | [journal-tom-modem.md](../refactor/journal-tom-modem.md) |
| `application/qmodem/src/libqmodem-sms/`、`extlib/pdu_decoder.c` | 边界修复、审计收尾 | [journal-c-core.md](../refactor/journal-c-core.md)、[journal-sms-c-audit.md](../refactor/journal-sms-c-audit.md) |
| `application/qmodem/src/modem_scan/`（scand/scanc） | socket 权限、白名单、写循环修复 | [journal-c-scan.md](../refactor/journal-c-scan.md)、[journal-scan-c-final.md](../refactor/journal-scan-c-final.md) |
| `application/qmodem/src/qmodem_smsd/` | C4/C10/C12 修复 | [journal-c-smsd.md](../refactor/journal-c-smsd.md) |
| `application/qmodem/files/`（原 `application/qmodem_monitor/`） | 注入修复、心跳 schema、并发锁 | [journal-monitor-dial.md](../refactor/journal-monitor-dial.md)、[journal-monitor-hard.md](../refactor/journal-monitor-hard.md) |
| `modem_dial.sh` / `modem_util.sh` / `modem_hook.sh` / `modem_monitor.sh` | APN/zone 修复、at_tool=2 通路、公共 DNS 收编 | [journal-shell-pkg.md](../refactor/journal-shell-pkg.md)、[journal-monitor-dial.md](../refactor/journal-monitor-dial.md)、[journal-atd-bridge.md](../refactor/journal-atd-bridge.md)、[journal-dial-zone.md](../refactor/journal-dial-zone.md) |
| `luci/luci-app-qmodem/` | 组件化、ACL、stats/band 端点、TTL 视图 | [journal-luci-next.md](../refactor/journal-luci-next.md)、[journal-luci-comp.md](../refactor/journal-luci-comp.md)、[journal-luci-final.md](../refactor/journal-luci-final.md)、[journal-luci-gap.md](../refactor/journal-luci-gap.md)、[journal-retire-ttl.md](../refactor/journal-retire-ttl.md) |
| 旧栈 luci 包（`luci-app-qmodem`、`-sms`、`-ttl`、`-ttlfw4`、`-mwan`、`-voip`、`-hc`、`-monitor`） | 注入修复、退役标注、决策 1–4 处置 | [journal-lua-retire.md](../refactor/journal-lua-retire.md)、[journal-retire-s1.md](../refactor/journal-retire-s1.md)、[journal-retire-mwan.md](../refactor/journal-retire-mwan.md)、[journal-retire-voip.md](../refactor/journal-retire-voip.md)、[journal-retire-hc.md](../refactor/journal-retire-hc.md) |
| 各包 `Makefile` 与 init 脚本 | PKG 规范化、依赖梳理 | [journal-shell-pkg.md](../refactor/journal-shell-pkg.md)、[journal-pkg-audit-all.md](../refactor/journal-pkg-audit-all.md) |

### 2.3 UPSTREAM_OWNED：可安全接受上游覆盖

以下路径未被任何 journal 涉及（以 §2.1 检索结果为准），上游变更可直接 checkout 上游版本：

- 数据文件：`modem_support.json`、`modem_port_rule.json`、`at_commands_en/zh.json`
  （合入后跑 `scripts/update_support_list.py` 校验与 `docs/support_list.md` 一致，
  见 `.github/workflows/support-list.yml`）。
- 独立 vendored 组件：`application/qfirehose/`（许可禁本地修改，安全修复按 ADR 0001
  决策 7 走上游补丁跟踪，不本地深改）、`application/qmodem/src/quectel_CM_5G_M/`、`application/ndisc6/`、
  `application/qmodem/src/qmodem-seal/`、`application/voipd/`、`application/qmodem_sipd/`。
- 文档：`docs/` 中用户向内容（`support_list.md`、`tom-modem-manual*` 等）；本仓库新增的
  `docs/refactor/`、`docs/developer/`（async-daemon、at-tool-selection 等）、`docs/adr/`、
  `docs/release/` 属反向，不参与上游覆盖。
- 未涉及的上游脚本与 `testcases/`、`tests/` 增量。

覆盖前仍须跑一次门禁冒烟（`scripts/check_all.sh --changed`），防止上游文件与重构面产生接口漂移。

### 2.4 与 ADR 0001 的一致性约束

- 同步**不得**复活 `luci-app-qmodem-ttl`（决策 1）或把 mwan/voip/HC 拉入退役/迁移（决策 2–4）。
- `at_tool` 默认值保持 `'0'`（决策 5）；上游若改动 AT 通路须重跑 fixture 回放。
- PDU 打包默认仍为 C 版（决策 6），上游 SDK 变更不触发自动切换。
- qfirehose 三项高危以"上游补丁跟踪 + 默认禁用 usb2tcp"处理（决策 7），同步时复核上游是否已修复。

## 3. 上游新增模组的吸收流程

> 规划中的 `new-modem-quickstart` 文档与 `check_modem_registry` 检查脚本**尚未入库**（2026-10-12
> 检索无果）。在它们建立之前，吸收流程以现行文件为准；两者落地后应更新本节引用。

1. **登记**：上游在 `application/qmodem/files/usr/share/qmodem/modem_support.json` 新增型号时，
   确认 `modem_name` 与 `modem_scan.sh` 的 `get_modem_model()` 输出一致（流程详见
   [docs/How_to_add_modem_cn.md](../How_to_add_modem_cn.md) 与
   [docs/developer/modem-adaptation.zh-cn.md](modem-adaptation.zh-cn.md)）。
2. **厂商脚本**：新增 vendor 时，脚本须 `source` `cmds/qmodem_lib.sh` 并走公共解析库
   （[journal-vendor-lib.md](../refactor/journal-vendor-lib.md) 的约定），不得直接合入上游旧式管道写法。
3. **能力屏蔽**：按 `modem_ctrl.sh` 的 `get_disabled_features` case 补齐或屏蔽 feature。
4. **校验**：合入后由 `support-list.yml`（JSON 合法性 + `docs/support_list.md` 差异）
   与 `scripts/update_support_list.py` 把关；规划中的 `check_modem_registry` 应替代其中的手工核对部分。
5. **fixture**：有实机条件时按 `testcases/README.md` 补 AT fixture，纳入回放。

## 4. feed/SDK 代际匹配规则摘要

来源：`.github/workflows/main.yml`（Resolve snapshot SDK 与 Initialization environment 两步）。

- **snapshot SDK**（`SDK_PREFIX` 非空）：feeds 取 `master`/`main` 开发线。
- **release SDK**（如 23.05）：feeds 必须取与 SDK 同代的 `openwrt-X.Y` 分支——
  `SDK_NAME` 经 `sed` 推导 `feed_ref`，base/packages/luci/routing 全部使用同一 `feed_ref`；
  推导失败直接报错退出。原因：release SDK 自带 CMake 3.26，而 luci master 已要求 3.31，
  跨代喂 feed 会破坏 `rpcd-mod-luci` 等包。
- 本仓库作为 `qmodem` feed 以 `src-link` 注入 SDK（`feeds.conf`），随 SDK 代际一起构建；
  `version.mk` 的 `QMODEM_VERSION`/`QMODEM_RELEASE` 是 feed 侧唯一版本源
  （当前 `3.4.0_rc3`，apk 仅接受 `_rcN` 预发布后缀，`PKG_RELEASE` 保持纯数字）。
- 标签与分支约束见 [branch-policy.zh-cn.md](../release/branch-policy.zh-cn.md)：
  预发布标签只能指向 `main`，稳定标签只能指向 `stable`，`main.yml` 的 tag 来源校验强制执行。

**同步含义**：上游若调整 `feeds.conf` 生成逻辑或升级目标 OpenWrt 代际，须整代同步并在
双代际（ipk/apk、arm64/x64/mmips）矩阵上重新验证，禁止只改一半。
