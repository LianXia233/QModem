# QModem 安装指南

[简体中文](install-suite.zh-cn.md) | [English](install-suite.en.md)

> 状态：**两包形态**。自组件整合后，整套只剩两个包——`qmodem`（后端，按目标架构编译）
> 与 `luci-app-qmodem`（前端，架构 `all`）。原 `qmodem-suite` 元包与其拉起的一批
> 独立包均已退役，相关原因与替代关系见
> [整合范围说明](../refactor/integration-scope.zh-cn.md)。

## 1. 包清单

| 包 | 架构 | 职责 | 是否必需 |
| --- | --- | --- | --- |
| `qmodem` | 目标架构 | 一体化后端：模组识别与拨号脚本、ubus 数据服务、短信守护、统计采集、网络监控、SMS 转发、全部命令行工具 | 是 |
| `luci-app-qmodem` | `all` | 唯一前端（纯 JavaScript），全部数据经 `qmodem` ubus 对象读取 | 推荐 |
| `luci-i18n-qmodem-zh-cn` | `all` | 简体中文翻译 | 可选 |

`luci-app-qmodem` 已 `DEPENDS:=+qmodem`，所以**只装前端即可**，后端会被自动拉入。

> 后端 `qmodem` 内部已包含原先分散的 11 个包：`libqmodem-sms`、`tom_modem`、
> `sms_tool_q`、`ubus-at-daemon`、`modem_scan`、`qmodem-smsd`、`qmodem-settings`、
> `qmodem_monitor`、`sms-forwarder-next`、`quectel-CM-5G-M`、`qmodem-seal`。
> 这些名字仍以 `PKG_PROVIDES` 形式保留，旧依赖可以继续解析；同时它们被列入
> `PKG_CONFLICTS`，不会与新包同时安装。

## 2. 安装

### 2.1 从软件源安装（推荐）

```sh
# opkg（OpenWrt 23.05 及更早）
opkg update
opkg install luci-app-qmodem luci-i18n-qmodem-zh-cn

# apk（OpenWrt 24.10 / ImmortalWrt 25.12 及更新）
apk update
apk add luci-app-qmodem luci-i18n-qmodem-zh-cn
```

想显式确认后端版本也可写成 `opkg install qmodem luci-app-qmodem`，效果一致。

> 兼容基线：OpenWrt / ImmortalWrt **25.12 及更新**（不再支持 24.10 及更早）。
> 25.12+ 基线由 CI 编译矩阵与文档强制，包元数据中没有版本约束表达式
> （OpenWrt buildroot 的 `DEPENDS` 不支持 `(>= 版本)` 语法）。

### 2.2 从 GitHub Release 直接安装（无软件源时）

若设备没有可用的 QModem 软件源，可从
[Releases](https://github.com/LianXia233/QModem/releases) 下载**散装 ipk/apk**（无需解压）
后直接安装。资产名带架构后缀，请按设备架构选择：

| 资产名后缀 | 架构 | 典型设备 |
|---|---|---|
| `_aarch64_cortex-a53`（ipk）/ `_aarch64`（apk） | ARM 64-bit | Airpi AP3000M 等 Cortex-A53 主板 |
| `_x86_64` | x86-64 | x64 软路由 / 虚拟机 |
| `_mipsel_24kc` | MIPS 小端 24Kc | MediaTek MT76xx / MT8621 等 |
| `_all` | 架构无关 | LuCI 应用与 i18n，各架构通用，只需装一次 |

```sh
# OpenWrt（以 aarch64 为例；apk 文件名去掉了 ipk 的 `_cortex-a53` 细粒度）
cd /tmp
opkg install qmodem_*_aarch64_cortex-a53.ipk
opkg install luci-app-qmodem_*_all.ipk
opkg install luci-i18n-qmodem-zh-cn_*_all.ipk

# ImmortalWrt（apk）
apk add --allow-untrusted qmodem-*_aarch64.apk luci-app-qmodem-*.apk
```

> ImmortalWrt 原生构建的资产带 `-immortalwrt` 后缀，请勿与 OpenWrt 的资产混装
> （两者 ubus ABI 不同）。

### 2.3 TTL 组件说明

TTL 管理（fw4/nft 后端）已并入统一包：服务 `/etc/init.d/qmodem_ttl` 与默认配置
`/etc/config/qmodem_ttl` 由 `qmodem` 携带，界面在 `luci-app-qmodem` 的 **TTL** 页。

- **默认关闭**：TTL 改写会关闭硬件流量分载，因此 `qmodem` 的 uci-defaults 与
  `postinst` 都刻意**不**启用 `qmodem_ttl`，需要时在 LuCI 中显式打开。
- **与旧 iptables 变体互斥**：旧版 `luci-app-qmodem-ttl`（iptables 栈）与 fw4 变体
  共用 `/etc/config/qmodem_ttl`，语义冲突，该包已退役。若设备上残留旧包，请先
  `opkg remove luci-app-qmodem-ttl`（或 `apk del luci-app-qmodem-ttl`）。

### 2.4 语音（voip）组件需单独安装

实验性语音栈（`qmodem-voip` + `qmodem-sipd` + `luci-app-qmodem-voip`）**不在默认安装
范围内**：它依赖 `libpjsip` / `libsoxr` / `alsa-lib` 等较重的库，体积大，且硬件白名单
受限（当前仅 RM520N-GL）。需要时单独安装：

```sh
opkg install luci-app-qmodem-voip   # 或 apk add luci-app-qmodem-voip
```

该前端会自动拉起 `qmodem-voip`（呼叫控制守护 voipd）与 `qmodem-sipd`（SIP 信令网关）。
独立保留而非合并的决策依据见
[docs/refactor/journal-retire-voip.md](../refactor/journal-retire-voip.md)。

## 3. 安装后状态核查

### 3.1 服务状态

`qmodem` 的 `postinst`（包管理器安装时）与
`/etc/uci-defaults/99-qmodem-worker-bootstrap`（随镜像打包 / 首次开机时）会自动补脚本
权限、`enable` 各守护进程并重启 `rpcd`。安装后可用以下命令确认：

```sh
# 开机自启是否注册
for s in qmodem_init qmodem_network qmodem_led qmodem_reboot qmodem_usage_stats \
         qmodem-smsd qmodem-settings qmodem_monitor qmodem-worker qmodem-stats-collect \
         ubus-at-daemon; do
	printf '%-24s %s\n' "$s" "$(/etc/init.d/$s enabled && echo enabled || echo DISABLED)"
done

# 运行状态
/etc/init.d/qmodem_init status        # 主服务：模组扫描 + 初始化
/etc/init.d/qmodem_network status     # 拨号网络
/etc/init.d/qmodem_led status         # 信号/状态 LED
/etc/init.d/qmodem_reboot status      # 定时/条件重启
/etc/init.d/qmodem_usage_stats status # 流量统计持久化
/etc/init.d/qmodem-smsd status        # 短信守护进程
/etc/init.d/ubus-at-daemon status     # 串口 AT 通道守护
/etc/init.d/qmodem-worker status      # 状态缓存采集
/etc/init.d/qmodem-stats-collect status # 流量统计采样
```

### 3.2 ubus 对象

`qmodem`、`qmodem_cache`、`qmodem_stats`、`qmodem_support`、`qos` 由本包提供，
依赖 `/etc/init.d/rpcd` 在安装后重启一次才会注册（`postinst` 已处理）：

```sh
ubus list | grep -E 'qmodem|qos'

# 示例：查询 modem1 的基础信息（节名见 /etc/config/qmodem 的 modem-device 节）
ubus call qmodem base_info '{"config_section":"modem1"}'
ubus call qmodem network_info '{"config_section":"modem1"}'
```

完整方法表见 [rpcd 接口](../qmodem-rpcd-interface.zh-cn.md)。

### 3.3 LuCI 菜单路径

安装 `luci-app-qmodem` 后，Web 界面出现独立菜单：**Modem → QModem → Overview**
（路由 `admin/modem/qmodem/overview`）。共 18 个页面：

概览 / 统计 / 移动数据 / 无线与小区 / 拨号与网络 / 消息 / SIM 短信 / SMS 转发 /
模组与 SIM / SIM 切换 / AT 控制台 / 网络监控 / 高级 / 模组特性 / TTL / 设备参数 /
服务与扫描 / 短信会话（会话页无菜单项，由消息页进入）。

看不到菜单时，先确认浏览器账号已获得 `luci-app-qmodem` 的 ACL，然后重新登录；
必要时 `rm -rf /tmp/luci-*cache` 并强制刷新浏览器缓存。

## 4. 最小可用配置

QModem 支持自动扫描发现模组；多数场景下安装后重启即可在 LuCI 中看到模组。手动写一个
最小 `modem-device` 节用于固定配置（写入 `/etc/config/qmodem`）：

```uci
config modem-device 'modem1'
    option enabled '1'
    option data_interface 'usb'
    option slot '/sys/bus/usb/devices/2-1.2'
    option at_port '/dev/ttyUSB2'
    option manufacturer 'Quectel'
    option enable_dial '1'
    option pdp_type 'IPv4/IPv6'
    option apn ''          # 留空自动匹配运营商
```

字段说明与更多选项（锁频、SIM 切换、自定义初始化命令等）见
[完整用户手册](../user-guide.zh-cn.md)的“模组设备配置”一节。修改后执行：

```sh
/etc/init.d/qmodem_init restart
```

## 5. 故障排查速查

| 现象 | 命令 / 位置 |
|---|---|
| 服务日志 | `logread -t qmodem`（也可 `logread -e qmodem` 过滤 QModem 全部条目） |
| 扫描器日志 | `logread -e modem_scand` |
| 拨号日志 | LuCI：Modem → QModem → 概览 内的拨号日志面板；或 rpcd `ubus call qmodem get_dial_log '{"config_section":"modem1"}'` |
| 内核/驱动层 | `logread -e usb -e mhi -e tty -e qmi -e mbim` |
| 端口枚举 | `ubus call qmodem get_tty_ports '{}'` |
| 界面报“未检测到模组支持库” | `ls -l /usr/sbin/qmodem-modem-support /usr/libexec/rpcd/qmodem_support` 应为 `755`，然后 `/usr/sbin/qmodem-modem-support` 手工跑一次 |

日志与 AT 响应可能包含 IMEI、IMSI、ICCID、手机号、APN 或短信内容，公开前必须脱敏。
更完整的排查流程见[故障排查](troubleshooting.zh-cn.md)。

## 6. AT 通路选择

QModem 的 AT 命令走哪条底层通路由 `qmodem.main.at_tool` 全局决定：

| 取值 | 通路 | 说明 |
| --- | --- | --- |
| `0` / 未配置 | `tom_modem` | 默认；可经 `ubus-at-daemon` 串行化 |
| `1` | `sms_tool_q` | 直连串口 |
| `2` | `qmodem-atd` | unix socket（Rust 异步守护，需自行部署） |

三通路的适用场景、超时语义、故障切换与排查命令见
[AT 通路选择](../developer/at-tool-selection.zh-cn.md)。安装后无需额外配置即使用通路 0；
通路 2 需按下一节启用 Rust 组件后再切换。

## 7. Rust 组件可选启用（qmodem-atd）

默认安装不包含 Rust 异步 AT 守护进程 `qmodem-atd`。如需启用通路 2，自行交叉编译二进制
并放入固件：

1. **交叉编译**：使用统一入口脚本 `scripts/cross-build-rust.sh`——

   ```sh
   # aarch64（路由器常见目标）
   bash scripts/cross-build-rust.sh --target aarch64-unknown-linux-musl
   # 产物：scripts/dist/aarch64-unknown-linux-musl/qmodem-atd
   ```

   产物为全静态 musl 二进制（零外部 crate），`file` 确认 `statically linked` 后可直接
   拷入目标机。构建细节与手动等价命令见
   [Rust 异步守护进程架构](../developer/async-daemon.zh-cn.md) 第 4 节。

2. **部署**：将二进制放入设备 PATH（如 `/usr/bin/qmodem-atd`），启动时指定 socket 路径
   与端口（见 async-daemon 文档第 3 节），默认 socket 路径与 `qmodem.main.at_socket`
   一致。

3. **切换通路**：把 `at_tool` 置为 `2` 并重启 `qmodem_init`；socket 不可达时自动回落
   `tom_modem` 通路。

## 8. 从旧版本升级

旧版本会把 `qmodem` 与一批独立包（`qmodem-smsd`、`qmodem_monitor`、`tom_modem`、
`ubus-at-daemon`、`sms-forwarder-next`…）分别安装，前端则是 `luci-app-qmodem-next`
或旧 Lua 版 `luci-app-qmodem`。升级路径：

```sh
# 1) 先卸掉被吸收的旧独立包，避免与新包争抢同名文件
opkg remove qmodem-suite qmodem-smsd qmodem_monitor qmodem-settings tom_modem \
	ubus-at-daemon sms-forwarder-next qmodem-seal 2>/dev/null
opkg remove luci-app-qmodem-next luci-app-qmodem-monitor luci-app-qmodem-ttlfw4 \
	luci-app-qmodem-ttl luci-app-qmodem-sms 2>/dev/null

# 2) 安装新形态
opkg install luci-app-qmodem luci-i18n-qmodem-zh-cn
```

`/etc/config/qmodem`、`/etc/config/qmodem_sms`、`/etc/config/sms_forwarder`、
`/etc/config/qmodem_ttl`、`/etc/config/ubus-at-daemon` 均为 conffile，升级时不覆盖，
配置保留。短信数据库 `/etc/qmodem/sms.sqlite3` 也保留（`keep.d` 已声明 `/etc/qmodem`）。
