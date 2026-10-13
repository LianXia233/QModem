# QModem 包整合范围与保留清单

本文件回答三个问题：

1. 这次整合把哪些包合并进了哪两个包；
2. 哪些包**没有**被合并，为什么（系统依赖 / 架构约束 / 语义冲突）；
3. 合并后各包各自的边界是什么，验收判据是什么。

前端三层统一（视觉 / 组件 / 数据）的兼容性验证方法与门禁判据另见
[`journal-next-into-generic-compat.zh-cn.md`](./journal-next-into-generic-compat.zh-cn.md)，
本文件不重复。

---

## 一、目标形态：两个包

| 包名 | 架构 | 位置 | 职责 |
| --- | --- | --- | --- |
| `qmodem` | 目标架构（编译 C 代码） | `application/qmodem/` | 一体化后端：模组识别与拨号脚本、ubus 数据服务、短信守护、统计采集、网络监控、SMS 转发、全部命令行工具 |
| `luci-app-qmodem` | `all` | `luci/luci-app-qmodem/` | 唯一前端：统一视觉层、组件层、数据层，全部数据经 `qmodem` ubus 对象读取 |

用户侧安装命令退化为两条：

```sh
# opkg 环境
opkg update && opkg install qmodem luci-app-qmodem

# apk 环境（ImmortalWrt 25.12+ / OpenWrt snapshot）
apk add qmodem luci-app-qmodem
```

`luci-app-qmodem` 已 `DEPENDS:=+qmodem`，因此**只装前端一条命令即可**（后端会被自动拉入）：

```sh
opkg install luci-app-qmodem
```

### 为什么是两个包而不是一个

`qmodem` 里有 8 个 C 工程，必须对目标架构交叉编译，产出 ELF 可执行文件；
`luci-app-qmodem` 全是 JS / JSON / PO，架构无关（`LUCI_PKGARCH:=all`）。
两者架构不同，在 OpenWrt 打包模型中无法合成单个包。
这是物理下限，不是取舍。

---

## 二、被并入 `qmodem` 的内容

### 2.1 原独立后端包（11 个 → 1 个）

| 旧包名 | 旧路径 | 现位置 | 说明 |
| --- | --- | --- | --- |
| `libqmodem-sms` | `application/libqmodem-sms/` | `src/libqmodem-sms/` | 短信 PDU / UCS2 公共库，产出 `libqmodem-sms.so.1.0.0` |
| `tom_modem` | `application/tom_modem/` | `src/tom_modem/` | AT 命令行工具，链接 `-lqmodem-sms` |
| `sms-tool_q` | `application/sms-tool_q/` | `src/sms-tool_q/` | 短信工具，两个补丁改放 `patches-src/sms-tool_q/` |
| `ubus-at-daemon` | `application/ubus_at_daemon/` | `src/ubus_at_daemon/` | 串口 AT 通道守护进程 |
| `modem_scan` | `application/modem_scan/` | `src/modem_scan/` | `modem_scand` / `modem_scanc` |
| `qmodem-smsd` | `application/qmodem_smsd/` | `src/qmodem_smsd/` | 短信数据库守护进程，链接 `-lqmodem-sms` |
| `quectel-CM-5G-M` | `application/quectel_CM_5G_M/` | `src/quectel_CM_5G_M/` | 定制 Quectel 连接管理器，装为 `/usr/bin/quectel-CM-M` |
| `qmodem-seal` | `application/qmodem-seal/` | `src/qmodem-seal/` | 反馈加密工具，由 `PACKAGE_qmodem_INCLUDE_SEAL` 条件编译 |
| `qmodem-settings` | `application/qmodem_settings/` | `files/{etc/init.d,usr/sbin}/qmodem-settings` | 纯脚本，无编译产物 |
| `qmodem_monitor` | `application/qmodem_monitor/` | `files/etc/init.d/qmodem_monitor` + `files/usr/share/qmodem/{modem_monitor.sh,monitor_recovery.sh}` | 纯脚本 |
| `sms-forwarder-next` | `application/sms_forwarder_next/` | `files/usr/bin/sms_forwarder_next` + `files/etc/{init.d,config}/sms_forwarder` + `files/usr/share/sms_forwarder/` | C 实现 + 6 个转发脚本 |

### 2.2 原属 `luci-app-qmodem-generic` 的运行时资产

generic 是「前端包顺带携带后端运行时」的形态。这些资产与界面无关，
属于后端能力，因此下沉到 `qmodem`：

| 资产 | 旧位置（generic 包内） | 现位置 |
| --- | --- | --- |
| 状态缓存采集 | `root/usr/libexec/rpcd/qmodem_cache` | `files/usr/libexec/rpcd/qmodem_cache` |
| 流量统计 | `root/usr/libexec/rpcd/qmodem_stats`、`root/usr/bin/qmodem-stats-*` | `files/usr/libexec/rpcd/qmodem_stats`、`files/usr/bin/qmodem-stats-*` |
| 模组支持库 | `root/usr/libexec/rpcd/qmodem_support`、`root/usr/sbin/qmodem-modem-support` | `files/...` 同路径 |
| QoS | `root/usr/libexec/rpcd/qos` | `files/usr/libexec/rpcd/qos` |
| worker / task | `root/usr/sbin/qmodem-worker`、`root/usr/bin/qmodem-task` | `files/...` 同路径 |
| AT 探测 | `root/usr/lib/qmodem/qmodem-at-probe`、`qmodem-lib.sh` | `files/usr/lib/qmodem/` |
| MT5700 修复 | `root/usr/sbin/qmodem-mt5700-fix`、`root/etc/init.d/qmodem-mt5700-fix` | `files/...` 同路径 |
| 服务启用钩子 | `root/etc/uci-defaults/99_luci-app-qmodem-generic` | 合并进 `files/etc/uci-defaults/99-qmodem-worker-bootstrap` |
| 模组支持库数据 | `root/usr/share/qmodem-generic/extra_modem_support.json` | `files/usr/share/qmodem/extra_modem_support.json` |

> 落盘路径保持不变（`/usr/libexec/rpcd/*`、`/usr/sbin/*`、`/etc/init.d/*`），
> 因此 rpcd 的 `[ -x ]` 判定、procd 启动、ACL 授权全部无感迁移。
> 只有「谁提供这些文件」变了。

### 2.3 并入的旧 Lua 栈后端脚本

原 `luci-app-qmodem`（Lua 栈）被退役，但它携带的服务脚本是后端能力，
与前端无关，因此保住并归入 `qmodem`：

- `files/etc/init.d/`：`qmodem_init` / `qmodem_led` / `qmodem_network` / `qmodem_reboot` / `qmodem_usage_stats` / `modem_ctrl`
- `files/etc/config/`：`qmodem` / `qmodem_led`
- `files/etc/board.d/03_qmodem`、`files/etc/hotplug.d/{iface,net,usb}/*`
- `files/lib/functions/qmodem-defaults.sh`
- `files/usr/share/qmodem/`：`cmds/`（17 个厂商脚本）、`vendor/`（12 个厂商脚本）、`led_scripts/`（10 个）、`at_commands_*.json`、`modem_support.json`、`modem_port_rule.json`、`apns.json`、`usage_stats.sh` 等
- `files/usr/libexec/rpcd/qmodem`、`files/usr/libexec/rpcd/modem_ctrl`

### 2.4 新增的安装后置动作

`qmodem-suite` 退役后，「让各守护进程开机自启」这件事必须有人做。
现在由 `qmodem` 自己承担，两条路径互为兜底：

- `Package/qmodem/postinst`（包管理器安装时）
- `files/etc/uci-defaults/99-qmodem-worker-bootstrap`（随镜像打包 / 首次开机时）

两者都会 `chmod 755` 脚本类文件、`enable` 守护进程、重启 `rpcd`、拉起采集服务。
`qmodem_ttl` **刻意不在自启列表中**：TTL 改写会关闭硬件分载，必须由用户在界面上显式打开。

---

## 三、被并入 `luci-app-qmodem` 的内容

| 旧包 | 旧路径 | 吸收方式 |
| --- | --- | --- |
| `luci-app-qmodem-generic` | 独立仓库 `LianXia233/luci-app-qmodem-generic` | 作为**统一基座**：`theme.js`（视觉层）、`ui.js`（组件层）、`controls.js`（数据层 + 唯一对外入口）整体迁入 |
| `luci-app-qmodem-next` | `luci/luci-app-qmodem-next/` | 10 个业务视图改接统一入口；自有样式层与私有组件层退役 |
| `luci-app-qmodem-monitor` | `luci/luci-app-qmodem-monitor/` | 视图 `view/qmodem/monitor.js` 并入，菜单项合并 |
| `luci-app-qmodem-ttlfw4` | `luci/luci-app-qmodem-ttlfw4/` | 视图 `view/qmodem/ttl.js`（fw4/nft 后端）+ 服务 `init.d/qmodem_ttl` + `/etc/config/qmodem_ttl` 并入 |
| `luci-app-qmodem-ttl` | `luci/luci-app-qmodem-ttl/` | 退役（旧 iptables Lua 栈，与 fw4 变体共用同一份配置，语义冲突） |
| `luci-app-qmodem-sms` | `luci/luci-app-qmodem-sms/` | 退役（旧 Lua 短信视图，能力被统一前端的短信模块完全覆盖） |
| `luci-app-qmodem`（旧 Lua 栈） | `luci/luci-app-qmodem/luasrc/**` | 退役；后端脚本下沉到 `qmodem`（见 2.3），`rpcd/acl.d` 与 `menu.d` 由新前端包接管 |

统一入口的对外契约（共 110 项导出）分三类：

1. **generic 原面**：`controls.*` 全部原有导出，保持不变；
2. **组件层兼容面**：`controls.hasCssClass / ModemSelector / formatTime / relativeTime / pollGuard`（由 `qmodem.ui` 收编为内部实现）；
3. **数据层兼容面**：`controls.sms.*`（SMS 数据域），以及 next 旧名别名 `controls.getStats` / `controls.callGetSimSwitchCapabilities` / `controls.rcStatus`（端点一致，仅改名）。

> `sendSms` / `deleteSms` / `setSmsStorage` 在两侧**同名但语义不同**
> （generic 直发与 next 的队列/存储语义不同）。统一时刻意**不覆盖重名**，
> 而是收进命名空间区分：`controls.sms.sendSms`。这是唯一保留的层级。

---

## 四、刻意保留为独立包的部分及原因

### 4.1 无法合并：内核模块（架构/工具链硬约束）

`kmod-*` 系列（`kmod-usb-serial-option`、`kmod-qmi_wwan_q`、`kmod-pcie_mhi` 等）
是 `qmodem` 的 `DEPENDS`，但它们**不可能**被合并进来：

- 必须走 `$(INCLUDE_DIR)/kernel.mk` + `KCONFIG:=` + `PKG_ARCH:=$(LINUX_KARCH)`，
  否则不会进入 `kernel_menuconfig`、不会生成 `kmod` 包元数据；
- vermagic 把模块与**具体内核版本**锁死，只能内核构建期产出；
- 源码在 `package/kernel/` 下，不属于本仓库。

因此 `qmodem` 保留对它们的 `DEPENDS`，不复制、不内联。

### 4.2 保留为独立包：上游项目与实验性栈

| 包 | 原因 |
| --- | --- |
| `ndisc6` / `rdisc6` | 上游独立项目（IPv6 邻居/路由发现），自带上游构建系统；`qmodem` 通过 `PACKAGE_qmodem_INCLUDE_ndisc6` 二选一依赖 |
| `qfirehose` | 上游独立项目（Quectel 固件刷写），`PACKAGE_qmodem_INCLUDE_ADD_QFIREHOSE_SUPPORT` 可选依赖 |
| `quectel-cm` / `quectel-CM-5G` | 通用 Quectel 连接管理器，由 feed 提供；`qmodem` 内置的是定制版 `quectel-CM-M`，两者可按需二选一 |
| `qmodem-voip`（voipd）、`qmodem-sipd`、`luci-app-qmodem-voip` | 实验性语音栈，依赖 `libpjsip` / `libsoxr` / `alsa-lib` 等重依赖，体积大且默认不需要；独立保留，按需安装 |
| `sms-forwarder`（旧脚本实现） | 与 `qmodem` 内置的 `sms-forwarder-next` **共用 `/etc/init.d/sms_forwarder` 路径**，语义上也互斥。保留给依赖 `/etc/config/sms_daemon` 的老用户，双方在 `PKG_CONFLICTS` 中互相排斥，不会同时安装 |
| `luci-app-qmodem-hc` | HC-G80 专用 SIM 切换链路（GPIO 直写 + `fw_setenv` uboot env），全仓库唯一的 `fw_printenv/fw_setenv` 使用点，且带 `@TARGET_mediatek_*_DEVICE_hc-g80` 依赖门 |
| `luci-app-qmodem-mwan` | 依赖 `luci-app-mwan3`，属多 WAN 叠加场景，与通用管理界面无耦合 |

### 4.3 被吸收包的旧名兼容

`qmodem` 声明 `PKG_PROVIDES`，把 11 个旧包名（`tom_modem`、`qmodem-smsd`、
`sms-forwarder-next`、`quectel-CM-5G-M` …）映射到自己身上，作用有两个：

1. 仓库内仍写着 `+tom_modem` 这样旧依赖的包（如 `application/sms_forwarder/`）
   在 `defconfig` / `package-metadata` 阶段仍能解析到提供者；
2. 已装旧包的用户升级时依赖得到满足，不会因为包名消失而断链。

同时 `PKG_CONFLICTS` 列同一批名字 + `sms-forwarder`，防止旧包与新包**同时**
安装导致同名文件互相覆盖（相同 ubus 名的 rpcd 插件、同名 `init.d`、同名 `/usr/bin` 工具）。

`luci-app-qmodem` 侧：

- `PKG_PROVIDES:=qmodem-luci-ui` —— 沿用 `luci-app-qmodem-next` 原本的虚拟名，历史依赖可解析；
- `PKG_CONFLICTS:=luci-app-qmodem-generic luci-app-qmodem-next` —— 防止与两套被吸收的前端同时安装造成菜单与 ACL 重复注册。

### 4.4 已退役的包

| 包 | 退役理由 |
| --- | --- |
| `qmodem-suite` | 元包，唯一实质职责是逐个 `enable` 守护进程。该职责已下沉到 `qmodem` 自身（见 2.4），剩下只是「依赖两个包」，与直接 `apk add qmodem luci-app-qmodem` 等价 |
| `luci-app-qmodem-next` | 内容全部并入 `luci-app-qmodem` |
| `luci-app-qmodem-monitor` | 同上 |
| `luci-app-qmodem-ttlfw4` | 同上 |
| `luci-app-qmodem-ttl` | 旧 iptables Lua 栈，与 fw4 变体共用 `/etc/config/qmodem_ttl`，保留会造成语义冲突 |
| `luci-app-qmodem-sms` | 旧 Lua 短信视图，能力被统一前端覆盖 |

---

## 五、验收判据

整合是否成立，按以下硬门禁判定（全部为可执行脚本，不靠人工目测）：

| 编号 | 判据 | 通过标准 |
| --- | --- | --- |
| G1 | JS 语法 | `node --check` 覆盖全部前端 JS，26/26 通过 |
| G2 | 视图 ↔ 端点 | 18 个视图引用的 ubus 端点 0 缺失（`compat_matrix.py`） |
| G3 | 统一入口导出面 | 页面中未解析的 `controls.*` 调用为 0（`verify_layers.py`） |
| G4 | 孤儿标识符 | 页面残留 `qmUI.*` / `smsService.*` / `qmodem.*` 引用为 0 |
| G5 | require 完整性 | 每个页面的 `require` 都能被解析，无悬空引用 |
| G6 | 菜单死链 | `menu.d` 中每个 `path` 都有对应 `view/**/*.js`，失效链接 0 |
| G7 | 样式类覆盖 | 页面使用的 CSS 类在全包样式表中均有定义（已知 2 项历史遗留除外） |
| G8 | i18n | 合并后条目 1852 条，需翻译条目覆盖率不低于迁移前的 87.3% |
| G9 | 包内独立 CSS | 统一后端之后包内 `.css` 文件数为 0（样式全部内联进 `theme.js`） |
| G10 | CRLF | 非 vendor 目录 0 个 CRLF 文件 |

补充的一致性检查：

- `application/qmodem/tests/test_sms_forwarder_conflicts.sh` —— 互斥声明与共享路径；
- `application/qmodem/tests/test_use_ubus_default.sh` —— `use_ubus` 默认值在
  创建路径（`modem_scand.c`）、运行路径（`modem_dial.sh` / rpcd 脚本）、
  界面路径（`network_config.js` / `service.js`）、数据库路径（`qmodem_smsd/main.c`）
  五处一致；
- `application/qmodem/tests/test_recognition_fixtures.sh` —— 识别用例命令仍存在于
  新的 `modem_scand.c` 路径下。

---

## 六、相关文档

- 前端三层统一的验证方法与「完美兼容」判据：
  [`journal-next-into-generic-compat.zh-cn.md`](./journal-next-into-generic-compat.zh-cn.md)
- 逐阶段改造记录：本目录下 `journal-*.md`
- 用户侧安装说明：`../user/installation.zh-cn.md`、`../user/install-suite.zh-cn.md`
- 版本变更：`../../CHANGELOG.zh-CN.md`
