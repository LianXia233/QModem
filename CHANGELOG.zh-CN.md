# 更新日志（Changelog）

[English](CHANGELOG.md) | [简体中文](CHANGELOG.zh-CN.md)

本文件记录本项目的所有重要变更。
格式基于 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)；版本号遵循
`version.mk`（main 分支：pre-release，stable 分支：release）。

> 本仓库自 2026-10 起经历了一轮系统性重构（审计 → 止血 → Rust 异步核心 → 单包交付）。
> 详细工程记录见 [docs/refactor/](docs/refactor/README.md)，问题依据见各 journal。

## [Unreleased]

### 新增（Added）
- **ImmortalWrt 原生构建变体（`arm64_apk_immortalwrt`）**：新增
  `.github/workflows/arm64_apk_immortalwrt.env`，改用 ImmortalWrt SNAPSHOT SDK
  （`immortalwrt-sdk-mediatek-filogic_`），并以 `SDK_FLAVOR=immortalwrt` 让
  `main.yml` 选取 ImmortalWrt 的 feeds —— base 取 `immortalwrt/immortalwrt`
  （ubus / libubox / libblobmsg-json 的 ABI 来源），packages 与 luci 取
  ImmortalWrt 镜像，routing 与 ImmortalWrt 自身 `feeds.conf.default` 一致仍取
  `openwrt/routing`。此前产物固定由 OpenWrt SDK 与 OpenWrt feeds 构建，其 ubus
  ABI 为 `libubus20260925`，而 ImmortalWrt master 为 `libubus20260628`，导致 apk
  在 ImmortalWrt 设备上因依赖不可满足而无法安装。OpenWrt 各变体行为不变；
  ImmortalWrt 资产在 Release 中带 `-immortalwrt` 后缀，避免覆盖同名的 OpenWrt
  资产。

### 修复（Fixed）
- **Release 资产改为可直接安装的散装包**（`main.yml`、
  `publish-existing-release.yml`）：不再把构建产物二次打包为
  `QModem-<arch>_*.tar.gz`，Release 直接上传散装 `.ipk` / `.apk`。
  由于 OpenWrt apk 文件名不含架构，按架构构建的 apk 资产名追加架构后缀
  （如 `qmodem-3.4.0_rc3-r1_aarch64.apk`）。下载后直接
  `opkg install *.ipk` / `apk add --allow-untrusted *.apk`，无需解压。
  v1.0.0 Release 已同步迁移（6 个 tar.gz 资产替换为 80 个散装包）。
- **`qmodem-suite` 元包恢复构建**：该包此前不在 SDK 构建清单中，导致
  "一次安装交付完整栈"的承诺没有可分发产物。已加入
  `qmodem_package_generic` 与 `main.yml` 的必检清单（其依赖
  `qmodem_monitor` / `luci-app-qmodem-monitor` / `luci-app-qmodem-ttlfw4`
  由同一次编译自动拉入）。

## [1.0.0] - 2026-10-12

### Phase 1 — 依赖精简与单包交付（`f874091`，2026-10-12）

#### 新增（Added）
- **`qmodem-suite` 元包（meta package）**：一次安装即交付完整套件
  （`qmodem` + `luci-app-qmodem-next` + `qmodem-smsd` + `qmodem_monitor` +
  `luci-app-qmodem-monitor`），并附带 uci-defaults 引导脚本，在首次安装时启用
  相关服务。参见 [docs/user/install-suite.zh-cn.md](docs/user/install-suite.zh-cn.md)。
- **`rust/` 异步核心，零外部 crate**：`qmodem-pdu`（3GPP TS 23.038 SMS PDU
  编解码器，对 `libqmodem-sms` 的带边界检查重写）与 `qmodem-atd`
  （thread-actor AT 守护进程：每端口命令队列、单调超时、URC 广播、
  unix-socket JSON 测试 API、按 feature 门控的 `tty`/`ubus`）。
  二进制体积较早期 tokio 构建（748 KB）减小 43.4%。37 个测试。
  参见 [docs/developer/async-daemon.zh-cn.md](docs/developer/async-daemon.zh-cn.md)。
- **`cmds/qmodem_lib.sh`**：共享 POSIX/busybox 解析器库
  （`ql_at_clean`、`ql_csv_field`、`ql_regex_pick`、`ql_num_or` 等）；
  quectel/huawei 厂商脚本作为试点完成迁移（移除 46 条重复管道），
  fixture 回归 30/30 字节级一致。
- **统一质量门禁**：`scripts/check_all.sh`（shell 语法、JS check、
  JS 冒烟测试、cargo test/clippy、C 语法、CRLF —— 自动发现）+
  `.github/workflows/quality.yml`；`vendor-fixtures.yml` 现已复用它。
- **无依赖测试套件**：`tests/sh/`（断言框架 + 18 个用例）
  与 `tests/lua/smoke.sh`（16 个用例），接入真实 `testcases/` fixture。
- **开发者/用户文档**：异步守护进程指南、单包安装指南、
  `docs/refactor/README.md` 索引、双语 README 章节。
- `qmodem` 现已声明其缺失的硬依赖 `+jsonfilter`。

#### 变更（Changed）
- `luci-app-qmodem-next`：共享 `ui.js` 组件层（modem 选择器、
  时间格式化、限流轮询守卫），CSS 去重，对空 ubus 载荷的防御性处理。
- 旧版 Lua 栈解除互斥：`luci-app-qmodem` 与 `luci-app-qmodem-next`
  现在可以共存安装（由用户自行选择）。
- `qmodem_smsd`：URC 触发的同步从 ubus 分发线程移入串行化的 worker
  （不再出现 15 秒的方法阻塞）。

#### 修复 — 安全（Fixed — security）
- **远程（modem 可控）输入**不再进入 root shell：
  `ubus_at_daemon` 事件回调改用 `execl` 位置参数执行脚本，
  取代 `system()` 字符串插值。
- 旧版 LuCI controller：修复经 `io.popen` 参数插值的命令注入
  （action 白名单 + shell 引号处理）；修复短信正文/发送者 XSS
  （innerHTML → textContent）。
- `modem_scand` 控制套接字：`0666` → `0660` + 专用组；新增 section 与
  scan-type 白名单。
- rpcd 插件：阻断缓存路径穿越（所有方法均加 `valid_section` 白名单），
  移除 `delete_sms`/`send_raw_pdu`/`send_sms` 中多余的第二次 `read`
  （这些方法此前会静默返回空结果），`set_imei` 输入校验（15 位数字）。
- `modem_dial.sh`：APN 凭据在使用前先做 shell 引号处理（此前是经 UCI 的
  root 命令注入向量）；SIM PIN 不再以明文记录日志。

#### 修复 — 内存安全与正确性（Fixed — memory safety & correctness）
- `ubus_at_daemon`：提前返回时未初始化的 `at_response_t`；每端口
  引用计数（并发 `sendat` 与 `close` 之间的 use-after-free）；
  `CLOCK_MONOTONIC` AT 超时（NTP 跳变安全）；end-flags 数据竞争；
  写失败时的响应状态清理。
- `tom_modem`：重写 `tty_open_device`（fd 泄漏 / 悬空 `FILE*`）；
  修复操作/错误枚举冲突——该冲突曾使一个非法参数静默执行短信读取。
- `libqmodem-sms` PDU 编解码器：攻击者可控 PDU 上的越界读、
  负索引写、两个恒为假的校验条件（`&&` → `||`）、
  缺失的返回值检查、7-bit 溢出防护。
- `qmodem_smsd`：SMS JSON 输出向栈缓冲区的无界 `sprintf`；
  UCS2→UTF-8 转换越界（含完整多字节截断）；
  NULL 列文本；COMMIT 失败现在会回滚。
- `modem_hook.sh`：修复反转的哨兵条件（post-init AT 序列从未执行）
  与无效的 `delay` 变量。
- `modem_dial.sh`：PIN 比较中的 POSIX 语法错误、未加引号的展开、
  恒真的接口检查、snapshot 的版本检测回退、qmi_dial 重试退避、
  精确的 `wait`。
- `modem_scand`：未检查的 `pthread_create` / 对未初始化线程 join。
- 打包：`qmodem` 的 `$(CP)` 安装 → `INSTALL_*` 宏；
  `PKG_RELEASE` 遵循 `version.mk` autorelease；5 个包中的
  `PKG_LINCESE_FILES` → `PKG_LICENSE_FILES`。
- LuCI next：ACL 读权限范围从 `*` 收窄到 27 个读方法；移除危险的
  `'modem_1'` 静默回退；删除带有 innerHTML XSS 模式的无用 `LuciTable`
  代码；SMS 时间戳改为从 PDU 时区字节解析（移除硬编码的 `−28800`
  hack）；NaN 进度条；扫描与刷新不一致。

### Phase 2 — 厂商迁移收尾与 Rust 集成（`T3`，2026-10-12）

#### 新增（Added）
- AT 工具路径 3：`qmodem.main.at_tool=2` 将 AT 命令经 Rust
  `qmodem-atd` unix-socket JSON 协议转发（opt-in；默认值不变），
  并附配套开发者文档（[AT 工具选择](docs/developer/at-tool-selection.zh-cn.md)）。
- `scripts/cross-build-rust.sh`：Rust 核心的 aarch64/x86_64 musl 静态
  交叉构建（+ `rust-build.yml` CI 矩阵，产物附 sha256）。
- 旧栈退役 Step 1：为 7 个旧包添加维护模式标记，并建立供
  Phase 4 删除使用的功能差距表。

#### 变更（Changed）
- 全部 12 个厂商适配脚本现均使用共享的 `cmds/qmodem_lib.sh`
  解析器（Phase 1 试点覆盖 quectel/huawei；全仓约 280 条重复管道退役）。
- rpcd 插件：移除通用 `eval`（改为白名单命令分发），缓存写入原子化
  （mktemp+mv），全程 POSIX 安全（经 dash 验证），
  合并重复的 `ubus call` 样板代码。
- `ubus_at_daemon`：monitor 线程探测端口时不再持有全局锁；
  修复配置加载器边界；lease token 改为随机。
- `tom_modem`：短信读取路径去重，操作匹配改为表驱动，
  贪婪读取增加绝对截止时间。
- LuCI next：下拉框改用组件 API 读取，列表上限可配置，
  并行 ubus 轮询（allSettled），补齐 i18n 缺口。

### Phase 2.5 — 收尾审查与交付物定稿（`02b4c70`..`777620d`，2026-10-12）

> 证据基线：`ls docs/refactor/` + `git log` 核对。journal 尚未落库/未合入 main 的
> 条目（rpcd gap 端点、vendor 残留清理专项、rust FFI/bench/smsd-rs/pdu-conform）
> 有意不记录，留给编排者合入后补充。

#### 新增（Added）
- **Rust 交叉编译交付物定稿**（`02b4c70`）：
  [journal-rust-cross.md](docs/refactor/journal-rust-cross.md) 落库（musl linker
  兜底链 musl-tools → musl.cc、产物 450,544 B stripped 实测、SDK 源码构建 /
  CI 预编译注入两条集成路线）；`rust/README.md` 增加 Cross compilation 章节。
- **旧栈退役 Step 1 审查结论**（[journal-retire-s1.md](docs/refactor/journal-retire-s1.md)，
  随 `ebb90dd` 合入）：7 个 legacy 包 Makefile 审查 + 维护模式标注，6 个旧栈 htm
  首行退役注释；端点 gap 表（Phase 4 输入）——`get_modem_cfg` luad 端点是子包
  DEPENDS 切 `+qmodem`（Step 2）的关键阻塞；HC SIM 切换、频段/流量统计端点、
  mwan/TTL/voip 页面在 next 栈暂无对应物。

#### 修复（Fixed）
- `scripts/cross-build-rust.sh`：`+crt-static` 改为仅对 `*-musl` target 注入
  （无条件注入曾使原生 gnu 产物 450 KB → 1.4 MB）(`02b4c70`)；可执行位恢复
  `100644 → 100755` (`777620d`)。
- `luci-app-qmodem-ttlfw4`：补缺失的 `PKG_LICENSE_FILES:=LICENSE`（Makefile
  审查发现，随 `ebb90dd` 合入）。
- `ubus_at_daemon`：退出清理路径自死锁修复（持 `ports_mutex` 期间递归加锁
  销毁端口；改为弹出链表头后锁外销毁，见 journal-atd-quality.md C7 收尾）。

### Phase 3 — Rust 深化、差距闭合与退役审查（`T4`，2026-10-12）

#### 新增（Added）
- Rust `ubus_ffi`：纯 Rust blobmsg 编解码器（与 libubox 字节级兼容，
  12 个 golden/健壮性测试）+ 按 feature 门控的 libubus FFI +
  mock-transport 集成测试；`qmodem-smsd` Rust 骨架（Storage trait、claim/ack、
  backoff、scheduler、URC hub —— 零依赖）；PDU 一致性套件 14→64 个测试
  （TS 23.038 向量、与 C 交叉验证的语义、属性鲁棒性）——
  **发现并修复一个真实的 SCTS 时区 bug**（按 TS 23.040
  9.2.3.11 的 semi-octet；+5:30/+8:00/±12:45 此前全部错误）；atd 微基准
  （p50 50ms 往返，31.4k cmd/s，广播 6.8µs）。
- rpcd 端点 `get_stats` / `clear_stats` / `get_current_band` + LuCI next
  Statistics 视图（ACL/menu/po）—— 闭合流量统计与当前频段差距。
- 退役审查结论：mwan（保留）、ttl/ttlfw4（稍后合并）、
  voip（独立保留）、hc（需实机验证）—— 详见各 journal。
- `docs/security/vendored-audit.md`：qfirehose 存在 3 个高危发现
  （经升级包路径的 system() 注入、XML 驱动的溢出、
  未认证 TCP→USB 透传）—— 需在上游修补/隔离。
- UCI 选项参考（73 个选项，zh/en），文档英文翻译 ×3，
  release.yml（changelog 驱动的发布），CI markdownlint。

#### 变更 / 修复（Changed / Fixed）
- 厂商残留清理（huawei.sh 8 处 CR 清理并入 ql_at_clean）；
  dial：确定性的 wan-zone 查找、qmi_dial 退避计数器重置修复；
  at-daemon 与 scand：-Wall -Wextra 清零、pdu_decoder 加边界、uci-get 解析；
  sms-tool_q：修复同源 C5/C6 类 PDU 溢出（ASan fuzz 200k，0 发现）；
  monitor：心跳 schema 版本、每 modem flock、进程组 kill；
  **修复单包交付**：qmodem 此前交付时未安装 5 个 init.d 脚本；
  qmodem-suite uci-defaults 现已启用全部服务。

### Phase 0 — 基于审计的加固（`2568198`，2026-10-12）

- 首轮基于证据的加固，覆盖 C 内存安全、shell 注入面、打包与 LuCI
  （27 个文件）；详情见 [docs/refactor/](docs/refactor/README.md) 各 journal。

### 版本化建议 — [Unreleased] → v1.0.0 拆分

- **Feature（minor 升级动因，1.0.x → 1.1.0）**：
  - `qmodem-suite` 元包（单包交付 + uci-defaults 引导）
  - `rust/` 异步核心（`qmodem-pdu` / `qmodem-atd`）与 `at_tool=2` AT 通路
  - `cmds/qmodem_lib.sh` 公共解析库 + 12 厂商脚本全量迁移
  - `scripts/cross-build-rust.sh` + `rust-build.yml` 交叉编译矩阵
  - `scripts/check_all.sh` 统一门禁 + `tests/sh` / `tests/lua` 回归体系
- **Fix（随 rc1 一并发布）**：Phase 0/1/2 全部安全与内存安全修复（注入、
  路径穿越、XSS、socket 权限、UAF/死锁）及打包修正（`PKG_LICENSE_FILES`、
  `INSTALL_*` 宏、可执行位）。
- **不纳入 rc1**：退役 Step 2（`get_modem_cfg` 等价端点与 DEPENDS 切换）与
  Phase 4 旧栈删除属 v1.2.0 范围；尚未合入 main 的 Phase 2.5+ 条目（rpcd gap
  端点、rust FFI/bench/smsd-rs/pdu-conform）在合入前不写入发布说明。

## [Base] — main@c49654e (2026-09-19)

- 本 fork 重构历史的上游基线
  （同步自 [FUjr/QModem](https://github.com/FUjr/QModem)）。
