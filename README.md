# QModem

简体中文 | [English](README.en.md)

[![OpenWrt SDK build](https://github.com/FUjr/modem_feeds/actions/workflows/main.yml/badge.svg)](https://github.com/FUjr/modem_feeds/actions/workflows/main.yml)

QModem 是面向 OpenWrt 和 ImmortalWrt 的蜂窝模组管理与拨号组件。它负责发现 USB、PCIe 模组，管理 AT 端口和数据接口，并通过 LuCI 提供拨号、状态监控、短信和厂商高级功能。

QModem 适合自制 4G/5G 路由器、工业网关和多模组设备。它不是通用桌面 ModemManager，也不能替代目标固件缺失的内核驱动。

## 主要能力

| 能力 | 说明 |
| --- | --- |
| 模组发现 | USB、PCIe，按物理卡槽稳定绑定 |
| 数据连接 | QMI、MBIM、NCM、ECM、RNDIS 等，具体取决于模组和驱动 |
| 模组状态 | SIM、注册网络、信号、小区和固件信息 |
| 高级控制 | 网络制式、锁频、锁小区、SIM 切换等，具体取决于厂商和型号 |
| 短信 | 收发、PDU 和可选转发；部分模组的长短信行为存在差异 |
| 多模组 | 独立配置、拨号、状态和物理卡槽管理 |
| OpenWrt 集成 | UCI、ubus、rpcd、procd 和 LuCI |

模组出现在[支持列表](docs/support_list.md)中，表示项目中存在对应配置，不等同于所有固件、接口模式和高级功能都经过维护者实机验证。

## 🌸 Soft Anime 视觉与现代交互界面（LuCI Next）

QModem LuCI 全系列插件已完成全新 **Soft Anime 柔和二次元 × Pastel Glass 毛玻璃 × Modern Dashboard 现代仪表盘** 视觉重构：

| 模块名称 | 页面路径 | 核心视觉与交互特性 |
|---|---|---|
| **总览仪表盘** | `/qmodem/overview` | 动态蜂窝波天线与吉祥物，4 大实时指标卡（信号/RAT、运营商/频段、会话/IP、卡槽），平滑骨架屏与异步水合 |
| **网络配置** | `/qmodem/network_config` | 双天线 5G 路由器动态 SVG，超时防挂死异步轮询，网络服务脉冲呼吸指示胶囊 |
| **AT 调试终端** | `/qmodem/debug` | 赛博终端视窗，分段式诊断标签栏（**节省 70%+ 纵向空间**），一键折叠，键盘 `↑`/`↓` 历史指令循环 |
| **高级配置** | `/qmodem/config_advanced` | 齿轮调谐动态 SVG，按物理设备分段调优，频段/制式响应式卡片网格 |
| **短信管理** | `/qmodem/sms` | 发光气泡对话流 SVG，消息时区/存储独立卡片，未读消息粉红提示徽章 |
| **核心设置** | `/qmodem/settings` | 微芯片与 PCB 脉冲走线 SVG，RPC 并行安全加载防冷启动阻塞 |
| **链路监控** | `/qmodem/monitor` | 雷达扫描波 SVG，多拨链路监控守护卡片化 |
| **TTL 规则** | `/qmodem/ttl` | 数据包护盾 SVG，内置 1-Click 常用预设（64 Linux/Android、128 Windows、65 绕过热点） |
| **SIM 切换** | `/qmodem/sim_switch` | 双卡芯片动态切换 SVG，Slot 1 / Slot 2 独立交互卡片与呼吸灯指示 |
| **短信转发** | `/qmodem/sms_forward` | 纸飞机信使动态 SVG，集成 Telegram/Webhook/飞书/ServerChan/PushDeer 多通道 |
| **SIM 短信** | `/qmodem/sms_sim` | Micro-SIM 存储 SVG，现代工具栏与卡片化消息管理 |

- **🎨 统一色彩 Token**：浅色模式（薰衣草紫 `#8875E8`、樱花粉 `#EFA8CB`、薄荷蓝 `#8CCFE2`、薄荷绿 `#79CDB0`）与暗色模式无缝自适应。
- **✨ 零外部依赖**：**100% 离线可用**，不调用任何外网 CDN 或外部网络字体，所有图标与吉祥物均为轻量内联矢量 SVG。
- **🪟 全局毛玻璃弹窗**：系统内所有 `ui.showModal` 均升级为 `14px` 毛玻璃虚化背景、`20px` 圆角卡片与马卡龙胶囊按钮。
- **📱 响应式适配**：兼顾桌面宽屏高密度监控与手机端（360px~430px）单手易操作布局。

## 软件组成

```text
LuCI
  |
rpcd / ubus
  |
QModem 控制与拨号层 ---- modem_scand 设备扫描器
  |
generic + vendor 厂商适配层
  |
cmds 统一 AT 命令层
  |
ubus-at-daemon / tom_modem
  |
USB / PCIe 蜂窝模组
```

核心包为 `qmodem` —— 一体化后端，已把原先分散的 11 个包（`libqmodem-sms`、`tom_modem`、`sms_tool_q`、`ubus-at-daemon`、`modem_scan`、`qmodem-smsd`、`qmodem-settings`、`qmodem_monitor`、`sms-forwarder-next`、`quectel-CM-5G-M`、`qmodem-seal`）收编进包内。统一前端为纯 JavaScript 的 `luci-app-qmodem-next`（旧 Lua 版 `luci-app-qmodem` 已于 2026-10 退役删除，不再编译、不再产出）。安装 `luci-app-qmodem-next` 会自动拉入 `qmodem`。TTL 界面另有 `luci-app-qmodem-ttlfw4`，实验性语音栈仍为独立包。详见[整合范围说明](docs/refactor/integration-scope.zh-cn.md)。

## 已移除的旧版组件（Retired Packages）

自 2026-10 起，以下 **旧版 Lua 界面包已正式移除**（其后台脚本及守护进程已完整收编入 `qmodem` 一体化后端包），全量统一采用纯 JavaScript 的现代 LuCI 界面：

| 原包名 | 说明 | 现代替代方案 |
|---|---|---|
| `luci-app-qmodem`（Lua 版） | 旧版 LuCI 界面 | `luci-app-qmodem-next`（Soft Anime 现代界面） |
| `luci-app-qmodem-hc` | HC-G80 SIM 切换 | `luci-app-qmodem-next`（SIM 切换视图）+ `qmodem` 内置脚本 |
| `luci-app-qmodem-sms` | 旧版短信转发 | `luci-app-qmodem-next` 内置 SMS 全家桶 |
| `luci-app-qmodem-ttl` | 旧版 TTL 配置 | `luci-app-qmodem-ttlfw4`（Soft Anime 现代界面） |
| `luci-app-qmodem-mwan` | 多 WAN 配置 | `qmodem` 内置多拨规则 + 官方 `luci-app-mwan3` |

旧版 Lua 界面包已从仓库删除，不再参与编译与发布产出，也不再出现在 menuconfig 中；新部署直接使用 `luci-app-qmodem-next` 与 `luci-app-qmodem-ttlfw4`。

## 快速开始

在 OpenWrt 源码树的 `feeds.conf.default` 中添加：

```text
src-git qmodem https://github.com/FUjr/QModem.git;main
```

然后更新 Feed：

```sh
./scripts/feeds update qmodem
./scripts/feeds install -a -p qmodem
make menuconfig
```

在 `LuCI -> Applications` 中选择 `luci-app-qmodem-next`。目标设备仍需具备与模组、拨号协议和当前内核匹配的 USB、串口、QMI、MBIM 或 MHI 驱动。

完整步骤见[安装指南](docs/user/installation.zh-cn.md)和[首次配置](docs/user/first-setup.zh-cn.md)。

## 使用边界

- 同一 AT 端口不应由多个模组管理程序同时读写，否则可能出现响应残缺、超时和模组掉线。
- 锁频、锁小区、改 IMEI 和模组重启会改变无线行为，执行前应确认目标型号及恢复方法。
- Release 中的内核驱动包必须与固件的内核 ABI 匹配，不建议使用 `--force-depends` 绕过内核依赖。
- 构建成功只能证明软件包可生成，不能证明具体模组的 AT 命令和数据连接已经通过实机测试。

## 文档

### 用户

- [用户文档入口](docs/user/index.zh-cn.md)
- [安装指南](docs/user/installation.zh-cn.md)
- [首次配置](docs/user/first-setup.zh-cn.md)
- [故障排查](docs/user/troubleshooting.zh-cn.md)
- [硬件支持列表](docs/support_list.md)
- [旧版完整用户手册](docs/user-guide.zh-cn.md)

### 开发者

- [开发者文档入口](docs/developer/index.zh-cn.md)
- [系统架构](docs/developer/architecture.zh-cn.md)
- [适配新模组](docs/developer/modem-adaptation.zh-cn.md)
- [测试与验证](docs/developer/testing.zh-cn.md)
- [rpcd 接口](docs/qmodem-rpcd-interface.zh-cn.md)
- [AT fixture 格式与回放](testcases/README.md)
- [更新日志](CHANGELOG.zh-CN.md) ([English](CHANGELOG.md))

## Rust 异步守护进程（实验性）

`rust/qmodem-atd` 是 C 版 `ubus_at_daemon` 的零依赖重写（`Cargo.lock` 无任何第三方 crate）：每端口一个 std::thread actor（mpsc 命令队列 + 单调超时 + URC 广播），并提供 Unix socket JSON 测试协议。产物二进制约 731 KiB（未 strip，较早期 tokio 版本 -43.4%）。当前以实验性并行可选项交付，不替换任何已打包组件。详见 [Rust 异步守护进程架构](docs/developer/async-daemon.zh-cn.md) 与 [Rust 工作区 README](rust/README.md)。

AT 命令通路可经 `qmodem.main.at_tool` 选择：`0` tom_modem/ubus at-daemon、`1` sms_tool_q 直连、`2`（本版本引入）qmodem-atd unix socket。详见 [AT 通路选择](docs/developer/at-tool-selection.zh-cn.md)。

## 安装

整套只剩两个包：`qmodem`（后端，按目标架构编译）与 `luci-app-qmodem-next`（前端，架构 `all`）。
前端依赖后端，因此一条命令即可：

```sh
opkg update && opkg install luci-app-qmodem-next    # opkg
apk add luci-app-qmodem-next                        # apk（ImmortalWrt 25.12+ / OpenWrt snapshot）
```

`qmodem` 的 `postinst` 与其 `uci-defaults` 钩子会补脚本权限、启用各后端守护进程并重启
`rpcd`，装完即可用。安装后服务核查、最小 UCI 配置与故障排查见
[安装指南](docs/user/install-suite.zh-cn.md)。原 `qmodem-suite` 元包已退役，
原因见[整合范围说明](docs/refactor/integration-scope.zh-cn.md)。

## 反馈问题

提交 Issue 前，请先阅读[故障排查](docs/user/troubleshooting.zh-cn.md)。至少提供路由器型号、固件版本、QModem 版本、模组完整型号、USB/PCIe 连接方式、拨号协议和复现步骤。日志、配置和 AT 响应可能包含 IMEI、IMSI、ICCID、手机号、APN 或短信内容，公开前必须脱敏。

QModem 支持在设备上采集 AT fixture 和反馈包。开发者应优先提交可回放、已脱敏的 fixture，而不是只提交截图。

## 贡献

新增厂商或型号前，请阅读[模组适配流程](docs/developer/modem-adaptation.zh-cn.md)。Vendor 和拨号实现发送的 AT 指令必须通过 `application/qmodem/files/usr/share/qmodem/cmds/` 中的 `cmd_*` 封装；CI 会检查该边界。

## 许可证

仓库的 [LICENSE](LICENSE) 以 Mozilla Public License 2.0 正文为基础，并附加了禁止商业使用的条款。因此它不是未经修改的标准 MPL 2.0 授权。使用、分发或集成本项目之前，请阅读完整许可证；项目维护者后续应统一许可证名称和 README 表述。

## 致谢

QModem 使用或参考了 5G-Modem-Support、luci-app-4gmodem、sms_tool、gl-modem-at、sendat 和 qosmio/nss-packages 等社区项目。具体来源和许可证应以各目录中的文件及 Git 历史为准。
