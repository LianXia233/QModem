# 更新日志（Changelog）

[English](CHANGELOG.md) | [简体中文](CHANGELOG.zh-CN.md)

本文件记录本项目的所有重要变更。
格式基于 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)；版本号遵循
`version.mk`（main 分支：pre-release，stable 分支：release）。

> 本仓库自 2026-10 起经历了一轮系统性重构（审计 → 止血 → Rust 异步核心 → 单包交付）。
> 详细工程记录见 [docs/refactor/](docs/refactor/README.md)，问题依据见各 journal。

## [Unreleased]

### 变更（Changed）
- **LuCI 前端回归上游 `luci-app-qmodem-next`**：`luci/` 目录整体对齐上游
  FUjr/QModem@c49654ef（即本仓库 fork 点，也是上游最新 main）。删除本地的
  `luci/luci-app-qmodem` 包——包括 Soft Anime Pastel Glass 主题体系、
  `theme.pageHead` 统一化改造及后续全部定制——由上游 `luci-app-qmodem-next`
  现代 JS 前端替代（上游以独立包交付，依赖 `qmodem`、`qmodem-smsd`、
  `sms-forwarder-next`；我们的单包 `qmodem` 已通过 `PKG_PROVIDES` 提供这些
  依赖名）。`luci/` 目录现与上游逐字节一致（新增
  `luci-app-qmodem-monitor` / `-sms` / `-ttl` / `-ttlfw4` / `-next`，
  恢复上游 `luci-app-qmodem`）。构建/CI 引用已同步适配，且仅涉及
  luci 相关包名：`qmodem_package_generic` 改编 `luci-app-qmodem-next`、
  `main.yml` 必需包为 `luci-app-qmodem-next` + `luci-i18n-qmodem-next-zh-cn`、
  `openwrt_package.config` 选中 next 包、`extract-i18n.yml` 与 i18n 脚本
  默认路径指向 `luci/luci-app-qmodem-next`、`check_all.sh` 运行 next 包的
  `test_sms_service.js`。其余组件未做任何修改。

## [1.1.0] - 2026-10-14

> tag `v1.0.1` 从未发布过 Release——编译被 ubuntu-26.04 runner 的
> uutils coreutils 前置检查失败阻断，因此 1.0.0 之后的全部变更都包含在本版本中。

### 新增（Added）
- **短信会话 Pastel Glass 皮肤（theme.js）**：新增 `.qm-app .sms-*`
  规则覆盖旧会话 CSS —— 接收气泡为 `--qm-surface` 柔影卡片，发送气泡
  使用 `--qm-accent-gradient` 渐变，会话区升级为毛玻璃面板
  （backdrop-filter + `--qm-glass-shadow`），未读徽章落在 `--qm-danger`
  上；明暗两模式均由共享令牌驱动。同时新增 ≤560px 手机端微调：
  气泡放宽至 `max-width:88%`，会话区高度收敛为 420px，气泡内删除
  按钮加大为 36px 触控目标，通用 `.btn` / `.cbi-button` 控件提升至
  42px。
- **Rust 目标优先 Cortex-A53**：`build-rust.sh` 对 `aarch64*` 目标显式加
  `-C target-cpu=cortex-a53`。实测（`rustc --print cfg`）默认 `generic` 只有
  `crt-static`/`neon`，指定后追加 `aes`/`crc`/`sha2`/`pmuv3`；主力设备 MT7981
  的 A53 在 `/proc/cpuinfo` 中确认带有 `asimd aes pmull sha1 sha2 crc32`，
  故可安全启用。未实现 ARMv8 Crypto 扩展的 A53 设备可导出
  `QMODEM_RUST_TARGET_CPU=generic` 回退，无需改脚本。C 侧沿用 OpenWrt SDK 的
  `-mcpu=cortex-a53`，两侧一致。
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

- **Rust 组件接入 OpenWrt 包构建（qmodem-atd 随包交付）**：`application/qmodem`
  新增 `build-rust.sh`，`Build/Compile` 第一步用 cargo 交叉编译 `qmodem-atd`
  并装入 `/usr/bin/`。linker 一律取 OpenWrt 自带交叉工具链（而非 musl.cc 等
  第三方工具链），保证与目标 sysroot 的 musl 配置及后续 libubus / libjson-c /
  libsqlite3 的 ABI 一致。目标三元组按 `$(ARCH)` 推导（aarch64 / mipsel /
  x86_64 → `*-unknown-linux-musl`）；其中 **mipsel musl 是 Rust tier 3**
  （`std` 可用但 rustup 无预编译），需 nightly + rust-src +
  `-Z build-std=std,panic_abort`，aarch64 / x86_64 为 tier 2 用稳定版即可，
  `main.yml` 已按矩阵变体预装对应工具链。本地门禁同步升级：
  `check_all.sh` 的 rust stage 在非 Unix 宿主改用预装 musl 目标 std 跑
  `cargo check + clippy --all-features`（不再是 SKIP），shell stage 把包根
  构建脚本与 `scripts/*.sh` 纳入语法检查（覆盖 74 → 114 个文件）。
- **qmodem-atd 系统服务与配置**：新增 `/etc/init.d/qmodem-atd` 与
  `/etc/config/qmodem-atd`，按 `config port` 段在启动时逐个登记 AT 口并把
  socket 固定到 `/var/run/qmodem/atd.sock`（默认 `enabled='0'`，须显式开启）。
  文档中三处旧的 `/var/run/qmodem-atd.sock` 路径已统一为
  `/var/run/qmodem/atd.sock`。
- **voipd 的 AT 适配改走 qmodem-atd unix socket**：`at_daemon_adapter.sh`
  不再调用已退役语义的 ubus `at-daemon` 对象（list/open/sendat），改用
  `{"cmd":"list"}` / `{"cmd":"sendat"}` JSON 行协议（nc -U 优先、curl
  --http0.9 兜底）。端口在守护进程启动时即已登记，故删除了原先
  「未打开则先 `ubus call at-daemon open`」的握手分支；响应字段由
  `status=success / end_flag_matched / response` 对齐为
  `status=ok / end_flag / lines`。
- **AT 默认通路切到 qmodem-atd（Rust）**：`at()`/`fastat()` 在 `at_tool` 未配置
  或为 `2` 时走 qmodem-atd 的 unix socket，`0`（tom_modem）/`1`（sms_tool_q）
  降级为 legacy 显式选项。配套变更：`/etc/config/qmodem` 默认 `at_tool='2'`；
  `/etc/config/qmodem-atd` 默认 `enabled='1'`；`/etc/init.d/qmodem-atd` 在未
  声明任何 `config port` 段时自动收集 `/etc/config/qmodem` 各 modem-device 的
  `at_port`（开箱即用），并把 `qmodem` 加入 reload trigger；新增
  `etc/uci-defaults/99-qmodem-atd-default` 让既有安装升级后同样切到新通路
  （用户已显式设 `at_tool='1'` 的尊重原选择）。回退只需
  `uci set qmodem.main.at_tool='0' && uci commit qmodem`。

### 变更（Changed）
- **次级视图 Pastel 色彩收敛（Soft Anime 视觉重构）**：
  `sms_conversation` / `sim_switch` / `monitor` / `ttl` / `modem_config` /
  `network_config` 中残留的全部硬编码颜色统一为 `--qm-*` 设计令牌 ——
  连接状态圆点（连接 `--qm-ok`、断开 `--qm-danger`、未知 `--qm-warn`、
  加载 `--qm-text-muted`），RAT 制式徽章改为 soft 底色胶囊（NR
  `--qm-ok` / 其他 `--qm-info`），monitor 安全警告与 TTL 重要事项块
  升级为 `--qm-warn-soft` Pastel 警示卡，短信字符计数器与元数据标记
  全部令牌化。至此所有 LuCI 视图不再包含任何裸十六进制/颜色名。
- **LuCI 界面排版与视觉升级**：统一页面宽度——`status` / `network` /
  `system` / `advanced` / `connection` / `device` 六个视图原先各自写死
  `max-width:1160px`（`device` 是 `960px`），现统一读 theme.js 的
  `--qm-maxw` 令牌，一处即可控制所有页面；删除 12 条 `bg-glow` 死规则
  （theme.js 已用 `display:none!important` 屏蔽）与重复 6 次的字体栈，
  收敛为 `--qm-font` 单一令牌并跟随 LuCI 主题的 `--font-family`。
  theme.js 组件视觉升级：滚动列表表头吸顶、长表格斑马纹、行悬停对比度
  加强、按钮按下态。样式段净减 1.4KB。
- **Runner 固定为 `ubuntu-26.04`**：8 个工作流共 11 处 `runs-on` 原先都写
  `ubuntu-latest`，而它将于 2026-10-19 起迁移到 Ubuntu 26.04
  （actions/runner-images#14748），GitHub 已对每次运行弹出该警告。现已显式
  固定为 `ubuntu-26.04`，既消除警告，也把镜像版本的选择变成一次主动决策
  （OpenWrt SDK 构建依赖 `libncurses5-dev` / `gcc-multilib` 等特定 apt 包，
  不应被动跟着标签漂移）。下次升级镜像时再主动改标签。
- **CI 工作流合并（11 → 8）**：把重叠的测试/校验工作流合并为两个，覆盖面不减：
  - `ci.yml`（新增）取代 `quality.yml` + `vendor-fixtures.yml` + `rust-build.yml`。
    合并时同时消掉三处重复检查：shell 语法检查只由 `check_all.sh` 跑一遍
    （原先 vendor-fixtures 又用 `--only shell` 跑第二次）；`cargo test
    --workspace` + clippy 只由 `check_all.sh` 的 rust stage 跑一遍（旧
    rust-build 的 `test` job 与之完全一致）；js 单测只由 js-test stage 跑一遍。
  - `support-list.yml`（新增）取代 `commit_check_supportlist.yaml` +
    `pr_check_supportlist.yaml`，两者校验逻辑本就相同，只是后处理不同（推送后
    自动提交、PR 时发评论）。
  发布链（`main.yml` / `release.yml` / `tagger.yml` /
  `publish-existing-release.yml`）与 `extract-i18n.yml` / `wiki.yml` 保持不变。
  推送 tag 现在只会启动 3 个工作流，而非 5 个。
- **CI 按需编译（不再每次推送就全量构建）**：`main.yml` 原先监听
  `push: branches: ['*']`，任何分支的任何改动都会跑满 arm64 三个变体（约
  15 分钟 × 3）。现在拆成两个工作流——
  - `main.yml`：只在 **① 推送 `v*` 标签 ② 手动 workflow_dispatch ③ PR 到 main**
    （忽略 docs/README/.github）时运行，负责全量编译与发布；
  - `tagger.yml`（新增）：只在 **main/stable 分支且变更包含 `version.mk`** 时
    运行，只做版本号判断与打标签（秒级，不编译）。标签推送再触发 `main.yml`，
    发版链路与原来一致。
  日常提交代码不再触发任何编译。
  `rust-build.yml`（Rust 交叉编译校验）同步收敛：只在标签 / 手动 / PR（含 Rust
  路径变更）时运行；纯文本与业务代码推送仍会跑轻量的 Quality gate 与
  Vendor fixture tests（毫秒级静态检查，非编译）。
- **页签层级细化**：主题会按 `#tabmenu` 递归追加多个 `ul.tabs`（第一行一级、
  第二行当前一级项的子页签）。二级页签现单独降级——更小字号（12px）与内边距
  （5px 12px）、常规字重、弱化文字色、去掉重复分隔线，与一级形成清晰层级。
- **排版与视觉规范统一（全站 18 个页面）**：`theme.js` 新增 `CSS_LAYOUT`
  区块，把原本散落在 7 个视图里的 331 条内联 CSS 收敛到统一规范上——标题字阶
  （h1>h2>h3>h4 + 首元素去上边距）、段落/列表行高、表格（表头底色与字重、
  单元格内边距、悬停高亮、末行去边框）、表单控件高度与说明文字、相邻卡片/网格
  的间距节奏、长文本溢出处理。
- **顶部页签（tab）优化**：新增 `CSS_TABS` 区块，针对主题渲染的
  `#tabmenu > ul.tabs`（menu-aurora.js 产出）做统一：胶囊化外观、6px 间距、
  单行不换行 + 横向滚动（条目多时不再挤压换行）、悬停态、键盘焦点环、
  激活态用「主色淡染底 + 主色描边 + 主题高对比文字色 + 加粗」区分（不用主色
  直接作文字色，避免主色偏浅的主题对比度不足）。
- **术语与文案统一**：补 29 条译文（Bridge Port、Direct modem access、SMS
  Storage、Sender、Result、Notice、Slot %s、Webhook URL 等说明性文案），
  专业术语与品牌名（APN、ARFCN、PCI、SCS、IPv4/IPv6、DNS、URL、USB、
  Webhook、PushDeer、ServerChan 及运营商名）按规范保留原文。
- **菜单整合（一级 16 → 6）**：原一级入口 16 个过密，改为
  `概览 / 移动数据 / 短信 / 模块与 SIM / 网络监控 / 高级` 六个，其余页面下沉为
  所属页面的子页签（拨号与网络、无线与小区 → 移动数据；SIM 短信、短信转发 →
  短信；SIM 切换、设备参数 → 模块与 SIM；AT 控制台、统计 → 网络监控；模组功能、
  TTL、服务与扫描 → 高级）。功能与页面一个不丢，仅调整层级；同步更新
  advanced.js / system.js / status.js 中硬编码的菜单路径引用。
- **CI 提速**：
  - OpenWrt SDK 缓存键不再包含 `github.sha`，SDK 与其 `build_dir` 可跨提交复用，
    不必每次提交都重新下载 SDK、重新拉 feeds、重编依赖（此前最大耗时项）
  - 新增 `swatinem/rust-cache` 步骤缓存 cargo registry 与 `rust/target`，
    第二次构建只重编变动的 crate
  - 新增 `concurrency.cancel-in-progress`：同一分支的新推送会取消排队中的旧运行，
    迭代不再堆积
- **CI 流程汉化**：工作流标题与 job / step 名称改为中文（`自动生成版本标签`、
  `准备发布元数据`、`构建 QModem`、`发布产物` 等）。job id、矩阵名、产物名、
  env 文件名保持原样，因为它们被跨 job 引用
- **CI action 升级到 Node 24 运行时**，消除「Node.js 20 is deprecated … being
  forced to run on Node.js 24」警告：`checkout` v4→v7、`upload-artifact`
  v4→v7、`download-artifact` v4→v8、`cache` v4→v6、`setup-node` v4→v7、
  `setup-python` v5→v7、`softprops/action-gh-release` v2.2.2/v2→v3
- **CI 构建矩阵收敛为 arm64**：`main.yml` 现在只构建 `arm64_ipk` / `arm64_apk` /
  `arm64_apk_immortalwrt`，`mmips_*`（mipsel）与 `x64_*` 移出矩阵。mipsel musl
  是 Rust tier-3 目标，需要 nightly + `-Z build-std`，其 std 链接步骤在 OpenWrt
  SDK 交叉 gcc 下无法解析 musl 启动文件与 `-lunwind`（详见
  `application/qmodem/build-rust.sh` 注释）；tier 3 路径保留实现，待链接链路
  打通后再加回。
- **前端统一（视觉 / 组件 / 数据三层）**：`luci-app-qmodem-generic` 的 LuCI 适配整体
  移植进 `luci-app-qmodem`，原 `luci-app-qmodem-next` 的 10 个业务视图改接统一入口
  `qmodem.controls`，不再各自维护样式层与私有组件层。统一后对外导出 110 项，覆盖
  generic 原面 + 组件层（`ModemSelector` / `formatTime` / `relativeTime` /
  `pollGuard` / `hasCssClass`）+ 数据层（`controls.sms.*`）以及 next 旧名别名
  （`getStats` / `callGetSimSwitchCapabilities` / `rcStatus`）。next 原有 61 个 ubus
  端点零丢失，统一前端合计可解析 79 个端点。验证方法、模块与调用路径、以及
  「完美兼容」的判定标准见
  [docs/refactor/journal-next-into-generic-compat.zh-cn.md](docs/refactor/journal-next-into-generic-compat.zh-cn.md)。
- **后端一体化 `qmodem`**：原 11 个独立包（`libqmodem-sms`、`tom_modem`、
  `sms_tool_q`、`ubus-at-daemon`、`modem_scan`、`qmodem-smsd`、`qmodem-settings`、
  `qmodem_monitor`、`sms-forwarder-next`、`quectel-CM-5G-M`、`qmodem-seal`）物理并入
  `qmodem`，源码置于 `src/<组件>/`，按依赖顺序在单个 `Build/Compile` 内构建。
  用户侧安装简化为 `qmodem` + `luci-app-qmodem`（仅装前端会自动拉入后端）。
  整合范围、未合并项及其原因见
  [docs/refactor/integration-scope.zh-cn.md](docs/refactor/integration-scope.zh-cn.md)。

### 移除（Removed）
- 退役 `luci-app-qmodem-next`、`luci-app-qmodem-monitor`、`luci-app-qmodem-ttlfw4`、
  `luci-app-qmodem-ttl`、`luci-app-qmodem-sms`：视图与服务分别并入
  `luci-app-qmodem`（其中 `luci-app-qmodem-ttl` 是旧 iptables Lua 栈，与 fw4 变体
  共用 `/etc/config/qmodem_ttl`，语义冲突，直接退役）。
- 退役 `qmodem-suite` 元包：其唯一实质职责是逐个 `enable` 后端守护进程，该职责已
  下沉到 `qmodem` 自身。
- 退役旧 Lua 栈 `luci/luci-app-qmodem/luasrc/**`（controller / model / view）；
  其携带的服务脚本、`board.d` / `hotplug.d` 钩子、厂商命令库与 rpcd 插件下沉到
  `qmodem`，落盘路径不变。

### 修复（Fixed）
- **CI：ubuntu-26.04 runner 上 OpenWrt SDK 前置检查恢复通过**：Ubuntu
  26.04 将 GNU coreutils 替换为 Rust 重写的 uutils coreutils，
  `install --version` 输出不再包含 "GNU"，导致 OpenWrt SDK 的前置检查
  （include/prereq-build.mk 先探测 `install --version | grep GNU`，
  再探测 `ginstall --version | grep GNU`）失败——runner 迁移到
  ubuntu-26.04 后，所有构建 job 在「初始化 SDK 环境」步骤报
  `Build dependency: Please install GNU 'install'`。main.yml 现在会向
  `/usr/local/bin` 安装一个 `ginstall` wrapper（OpenWrt 探测的第二候选）：
  `--version` 输出 GNU 字样通过检查，其余调用原样转发给系统 install
  （uutils 支持 OpenWrt 用到的 -d/-m/-D/-p/-t/-T 选项）；步骤会先自检，
  系统 install 本身能通过 GNU 检查时跳过安装 shim。
- **AT 控制台在 AT 独占（qmodem-atd）模式下不可用**：实机联调定位到 6 个叠加
  缺陷，全部修复——
  1. `qmodem-atd` init 用 `config_get "$section" type` 判断节类型，而 UCI 里
     `config modem-device '2_1'` 的 `modem-device` 是**节类型**不是 option，
     取值为空 → 未显式配置 `config port` 时收集不到任何端口，atd 直接不启动；
  2. 出厂 `/etc/config/qmodem-atd` 预置了示例端口 `/dev/ttyUSB2`，会把守护
     进程绑到**错误设备**（本机 AT 口实际为 `/dev/ttyUSB1`）→ 控制台请求在 atd
     侧查不到端口；现改为不预置端口段（自动收集），并跳过不存在的 tty 节点；
  3. `qmodem.send_at` 在调用方未给端口时不解析 UCI，直接 `at "" <cmd>` 必然
     失败（表现为 `status=0` 且无 `res`）；现按 `at_port`/`override_at_port`
     回退解析，并在命令/端口缺失时返回明确错误体；
  4. 前端传 `use_ubus`、后端读 `use_ubus_flag` → "走 ubus"开关被静默忽略；
     后端兼容两种键名，前端统一为 `use_ubus_flag`；
  5. atd 桥接 helper 的 curl 兜底用 `-d` 传正文，而 `-d` 会按 urlencoded 语义
     **删除换行**，atd 按行读取永远等不到完整请求行；同时判停条件写成"文件末尾
     是换行"，会被 HTTP 头行产生的 bad json 回复提前打断。改为 `--data-binary
     @-` + `-N`（禁缓冲）+ 以「出现 `{"status":...}` 应答行」为判停条件；顺带
     把 busybox 不支持的 `sleep 0.1` 换掉，并在 busybox `nc` 无 `-U` 时直接走
     curl（避免一次必然失败的传输）；
  6. Rust atd 打开串口时 `B115200` 常量误写为 `0o100002`（正确值 `0o10002`），
     属于未定义速率 → `tcsetattr` 后串口速率异常，**永远读不到模组字节**；
     已修正，并让 termios 设置失败显式报错（不再静默"能打开但读不到数据"）。
- **AT 独占被遗留 C 组件破坏**：实机 `ls -l /proc/*/fd` 显示遗留
  `ubus-at-daemon` 与 `qmodem-atd` **同时持有** `/dev/ttyUSB1`，导致 atd 收到的
  应答被偷走（形似"AT 超时"）。现在 `qmodem-atd` 启动前会停止并停用遗留
  `ubus-at-daemon`，保证 AT 串口只有一个持有者。
- **流量清零计划保存无效（前后端参数不匹配）**：前端发嵌套 `params` 对象，后端
  `set_traffic_reset_schedule` 接收的是扁平字段（`enabled`/`reset_type`/`hour`/
  `day`）→ 计划从未真正写入；且后端缺少 `minute` 字段，前端的分钟选择器永远
  存不进去。现在前端显式映射为扁平字段，后端补齐 `minute`（校验 0–59、写入
  `usage_reset_minute`、进入 cron 行与返回体）。
- **二级页签未汉化**：菜单标题 `Dial and Network`、`Device Parameters`、
  `Modem Features`、`Service and Scan`、`SIM Messages`、`SMS Forwarding` 在 po
  中没有条目，导致第二行页签显示英文。现已补齐（`TTL` 作为术语保留原文）。
- **页签行内"浅底控件"变成空白块**：hero 会把后代文字强制白色，而
  `.mt-system-temp-box`（模组系统页温度框，经 CARD_CLASSES 拿到白底）与
  `.mtconn-state`（连接页状态徽章，`background:var(--qm-glass)`）是浅底，
  于是白底白字整块看不见（实机截图确认）。二者在 hero 内同样黑化处理，
  温度值/说明与徽章文字一并强制白字。
- **Hero 内胶囊/按钮白底白字**：hero 的 `* { color:#fff !important }` 会把所有
  后代文字强制白色，而 `.qm-hero-op`（运营商）、`.qm-pill-tag`（模式、接口）、
  `.qm-refresh-btn`（刷新）的默认底色是白/浅灰（跟随主题）→ 白底白字不可见
  （实机截图确认）。现统一改为深色半透明底 + 白字，并补齐 hover（加深）与
  disabled（低对比但仍 ≥4.5）状态：白字对比度 8.32–17.94 全部达标。
- **「网络监控」页面与标题未汉化**：monitor.js 中有 61 条文案缺中文译文
  （部分甚至未包 `_()`），且菜单标题 `Network Monitor` 在 po 中完全没有条目。
  现为 17 处硬编码英文补 `_()`（占位符、示例值、AT 命令示例均原样保留），
  并补 po 译文 64 条（模板 13 条）；`URL` 按术语规范保留原文。
- **英雄区（hero）文字看不清**：hero 渐变两端此前直接用主题主色
  （`color-mix(主色 72%,#060b14) → var(--qm-accent)`，深色回退同理），而 hero 文字被
  强制白色；主色偏浅的主题（如 luci-theme-aurora）下渐变末端变浅蓝，白字对
  比度只有 1.51–4.44（aurora 约 2.53），全部不达 WCAG AA。现改为两端都压暗
  （`color-mix(主色 24%/42%,#05080f)`，回退 `#0f172a→#1e40af` / `#0b1220→#14315c`），
  主色只作色调来源；白字对比度 4.96–16.02 全部达标（主色为纯白的极端情况仍 4.96）。
  hero 内徽章底色由半透明白改深色半透明（浅主题下白字 4.27 → 5.9+），副标题/元信息
  文字降为 90% 白以保留层次。
- **qmodem-atd 一启动就崩溃循环（实机实测）**：打包时只用默认 feature（仅
  `service`），缺 `tty` 时守护进程收到 `--port` 直接退出（"--port requires
  building with --features tty"），procd 反复重启、socket 始终不存在，AT 默认
  通路上的全部请求失败。构建改为 `--features service,tty`。
- **LuCI 页面白屏 / "factory yields invalid constructor"**：`qmodem/compat.js`
  是纯副作用模块，结尾 `return {}`，而 LuCI 的 `require()` 会校验工厂返回值是
  合法 class（luci.js：`if (!Class.isSubclass(_class))`），普通对象直接触发该报错，
  所有依赖它的视图停在 Loading view。改为 `return L.Class.extend({})`。
- **`mmips`（mipsel）构建因 Rust 目标三元组为空而失败**：MIPS 平台的 `$(ARCH)` 是
  `mipsel_24kc`（必须带 ISA 后缀），精确匹配的 `ifeq ($(ARCH),mipsel)` 永远不成立，
  `Build/CompileRust` 在 cargo 启动前就中止（且诊断被 OpenWrt 吞掉）。三元组推导改为
  `$(filter)` 前缀匹配，覆盖 `aarch64*` / `mipsel*` / `mips*` / `x86_64*`。
- **`mmips`（mipsel）build-std 链接失败**：`-Zbuild-std` 下链接驱动解析不到 musl
  启动文件（`crt1.o` 等）与 `-lunwind`（build-std 产物为带 hash 的
  `libunwind-<hash>.a`，`-lunwind` 无从绑定）。`build-rust.sh` 现在把交叉工具链的
  真实 sysroot 作为最后一个 `--sysroot` 链接参数追加，并提供空 `libunwind.a` 垫片
  （std 以 `panic_abort` 构建、运行期不需要真 unwinder，垫片合法）。
- **CI 包编译失败不再静默**：`Compile QModem` 步骤失败时用 `make ... V=s -j1` 重跑，
  让被 OpenWrt 默认隐藏的真实错误（如 cargo 交叉编译日志）直接出现在 job 日志中。
- **移除跨包文件冲突**：`qmodem` 与 `luci-app-qmodem` 此前都会安装
  `/usr/share/rpcd/acl.d/luci-app-qmodem.json`。现由前端包单独提供（并把原先位于
  后端 ACL 中的 `luci-mod-status-index → modem_ctrl` 授权合并进来），后端只装
  `qmodem.json`。
- **Release 资产改为可直接安装的散装包**（`main.yml`、
  `publish-existing-release.yml`）：不再把构建产物二次打包为
  `QModem-<arch>_*.tar.gz`，Release 直接上传散装 `.ipk` / `.apk`。
  由于 OpenWrt apk 文件名不含架构，按架构构建的 apk 资产名追加架构后缀
  （如 `qmodem-3.4.0_rc3-r1_aarch64.apk`）。下载后直接
  `opkg install *.ipk` / `apk add --allow-untrusted *.apk`，无需解压。
  v1.0.0 Release 已同步迁移（6 个 tar.gz 资产替换为 80 个散装包）。
- **新增 `.gitattributes` 统一行尾策略**：仓库与 `scripts/check_all.sh` 的 crlf
  stage 都以「一律 LF」为前提，但此前没有 `.gitattributes`，Windows 上
  `core.autocrlf=true` 会在检出时把工作区写成 CRLF，使本地 crlf 门禁报出大量
  与提交内容无关的误报（索引里其实仍是 LF）。现显式声明 `* text=auto eol=lf`，
  检出即 LF；第三方/上游 vendored 目录（`application/qfirehose/`、`driver/`、
  `application/qmodem/src/quectel_CM_5G_M/`、`application/qmodem/src/sms-tool_q/`）
  以 `-text` 保持字节原样，与 crlf stage 的排除名单一致。
- **补交上一轮整合遗漏的失效路径引用**：`docs/refactor/HANDOFF.md` 与
  `docs/refactor/README.md` 的门禁命令块中，JS 冒烟路径与 C 语法检查路径仍指向
  已退役的 `luci/luci-app-qmodem-next/` 与 `application/ubus_at_daemon/{inc,src}`；
  `docs/developer/` 下的异步守护进程导览与上游同步策略表同样引用了合并前的旧
  目录。全部改为合并后路径（`luci/luci-app-qmodem/tests/`、
  `application/qmodem/src/ubus_at_daemon/`、`application/qmodem/files/`）。
- **修复 `crlf` 门禁的判据与实现**：原实现用 `grep -I -q -l $'\r'` 扫工作区文件，
  存在两个反向缺陷——Windows 上 `core.autocrlf=true` 会让工作区检出为 CRLF，与提交
  内容无关却全部命中（误报）；而 MSYS/Cygwin 的 grep 以文本模式读文件会吞掉 CR，
  对含 CR 的文件同样返回不匹配（实测含 36 个 CR 的文件亦不命中，漏报）。现改用
  `git ls-files --eol` 的 `i/<eol>` 字段判定即将提交的内容，确定性且跨平台；同时
  把该 stage 从分钟级（全量扫 688 个文件约 3.7 分钟）降到约 2 秒。已用「向索引注入
  CRLF blob」做过负向验证，确认能正确 FAIL 并点名文件。
- **`rust` 与 `c` 两个 stage 增加「宿主平台不适用」判别**：`rust` 跑
  `cargo test --workspace`，而 `qmodem-atd` 依赖 `std::os::unix`（unix socket 与
  串口），在 Windows 宿主上无法编译；`c` 在 MinGW 下会因 UCRT 不提供 `timegm` /
  `strsep` 等 POSIX 符号而报隐式声明。两者都是平台差异而非代码缺陷，现按严格判据
  降级为 SKIP 并说明原因（`rust`：仅当失败者全为 `qmodem-atd` 且日志确有该平台
  错误；`c`：用一份仅含这些 POSIX 声明的补丁头重试编译，补上声明后即通过，才判定
  为宿主缺口——真实的语法错误仍照常 FAIL）。CI（ubuntu-latest）仍照常执行这两个
  stage。
- **恢复合并过程中丢失的 38 个可执行位**：本机 `core.fileMode=false`（Windows
  默认），git 不记录工作区的 x 位，因此本轮新增/重写的文件一律以 `100644` 入索引。
  其中 37 个是 `Makefile` 中由 `$(INSTALL_BIN)` 安装、落盘必须为 `0755` 的运行时
  脚本（`/etc/init.d/*`、`/etc/uci-defaults/*`、`/usr/bin/*`、`/usr/sbin/*`、
  `/usr/libexec/rpcd/*`、`/usr/lib/qmodem/*`、netifd proto 脚本、
  `sms_forwarder` 通知脚本等）；另 1 个是
  `tests/qmodem-seal/test_qmodem_seal.sh`——它在合并前就是 `100755`，内容逐字节
  未变、仅模式丢失，而 `vendor-fixtures.yml` 直接把它当可执行文件调用，模式丢失
  即导致该工作流在第 3 行 `Permission denied`（exit 126）失败。现对这 38 个文件
  `git update-index --chmod=+x`，索引模式已全部复核为 `100755`，`git diff --cached`
  确认纯模式变更、无内容改动。
  同类审计中另有 `files/usr/share/qmodem/ref-rmnet-dhcp`（原
  `application/quectel_CM_5G_M/files/dhcp`）在旧树里是 `100755`，但它是一份
  dnsmasq 配置参考（`/etc/config/dhcp` 样式），且旧 `quectel_CM_5G_M/Makefile`
  从未安装过它；本轮以 `$(INSTALL_DATA)` 安装（落盘 `0644`），**有意保持
  `100644`**——旧树的 x 位系上游遗留，恢复反而与安装语义不符。

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
