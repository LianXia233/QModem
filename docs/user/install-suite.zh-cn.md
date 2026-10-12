# qmodem-suite 单包安装指南

[简体中文](install-suite.zh-cn.md) | [English](install-suite.en.md)

> 状态：**新增元包**。`qmodem-suite` 是元包（metapackage，`PKGARCH:=all`，见 `application/qmodem-suite/Makefile`），用于把 QModem 核心与推荐前端一次装齐。若你的固件源中尚无该包，请直接参考本文末尾的"逐包手动安装"一节；安装后各服务的状态核查与故障排查命令在任何安装方式下都适用。

## 1. 一键安装

qmodem-suite 会自动拉起以下组件（依赖关系由包管理器解析）：

- `qmodem`：核心控制与拨号栈（含 modem_scand、rpcd 脚本）
- `luci-app-qmodem-next`：推荐的纯 JavaScript LuCI 前端
- `qmodem-smsd`：短信守护进程
- `qmodem_monitor` + `luci-app-qmodem-monitor`：网络监控及其 LuCI 页面
- `luci-app-qmodem-ttlfw4`：TTL 管理（fw4/nft 现代防火墙栈），默认关闭，LuCI 中按需开启

### opkg（OpenWrt 21.02 / 23.05）

```sh
opkg update
opkg install qmodem-suite
```

### apk（OpenWrt 24.10 起默认）

```sh
apk update
apk add qmodem-suite
```

> 注意：两套 LuCI 主界面不应同时安装。suite 已指向 `luci-app-qmodem-next`，请勿再手动安装旧版 `luci-app-qmodem`。

### TTL 组件说明

suite 内置的 TTL 组件为 `luci-app-qmodem-ttlfw4`（fw4/nft 现代栈）。它与旧版 iptables 变体 `luci-app-qmodem-ttl` **互斥**（两者共用 `/etc/config/qmodem_ttl`），因此 suite 不包含也不依赖 `luci-app-qmodem-ttl`；OpenWrt 包管理器会在依赖解析层面拒绝二者共存，若设备上已装旧版 `luci-app-qmodem-ttl`，请先执行 `opkg remove luci-app-qmodem-ttl`（或 `apk del luci-app-qmodem-ttl`）再安装 suite。

TTL 功能默认关闭（`/etc/config/qmodem_ttl` 的默认配置节随 `luci-app-qmodem-ttlfw4` 包自带，安装 suite 即就位），无需额外配置；在 LuCI 中按需开启即可。启用后会改写 br-lan 出站报文的 TTL/Hop Limit 并关闭硬件流量分载，详见该包 LuCI 页面说明。

### 语音（voip）组件需单独安装

实验性语音栈（`qmodem-voip` + `qmodem-sipd` + `luci-app-qmodem-voip`）**不包含在 suite 中**，需单独安装：

```sh
opkg install luci-app-qmodem-voip   # 或 apk add luci-app-qmodem-voip
```

### 从 GitHub Release 直接安装（无软件源时）

若设备没有可用的 QModem 软件源，可从 [Releases](https://github.com/LianXia233/QModem/releases) 下载**散装 ipk/apk**（无需解压）后直接安装。资产名带架构后缀，请按设备架构选择：

| 资产名后缀 | 架构 | 典型设备 |
|---|---|---|
| `_aarch64_cortex-a53`（ipk）/ `_aarch64`（apk） | ARM 64-bit | Airpi AP3000M 等 Cortex-A53 主板 |
| `_x86_64` | x86-64 | x64 软路由 / 虚拟机 |
| `_mipsel_24kc` | MIPS 小端 24Kc | MediaTek MT76xx/MT8621 等 |
| `_all` | 架构无关 | LuCI 应用与 i18n，各架构通用，只需装一次 |

```sh
# OpenWrt（以 aarch64 为例；apk 文件名去掉了 ipk 的 `_cortex-a53` 细粒度）
cd /tmp
opkg install qmodem-suite_*_all.ipk
opkg install qmodem_*_aarch64_cortex-a53.ipk
opkg install qmodem-smsd_*_aarch64_cortex-a53.ipk
opkg install qmodem_monitor_*_aarch64_cortex-a53.ipk
opkg install luci-app-qmodem-next_*_all.ipk
opkg install luci-app-qmodem-monitor_*_all.ipk
opkg install luci-app-qmodem-ttlfw4_*_all.ipk

# ImmortalWrt（apk）
apk add --allow-untrusted qmodem-suite-*.apk   # 元包会自动拉起上述依赖
```

> 逐包安装时务必装齐 suite 的全部依赖（见上方"一键安装"清单），否则 LuCI 页面与监控会缺功能。

该前端会自动拉起 `qmodem-voip`（呼叫控制守护 voipd）与 `qmodem-sipd`（SIP 信令网关）。voip 为实验性功能、硬件白名单受限（当前仅 RM520N-GL），且处于独立维护状态（决策依据 `docs/refactor/journal-retire-voip.md`），故不纳入单包交付。

## 2. 安装后状态核查

### 服务状态（/etc/init.d/）

```sh
# 开机自启是否注册
/etc/init.d/qmodem_init enabled && echo enabled

# 各服务状态
/etc/init.d/qmodem_init status      # 主服务：modem_scand 扫描 + 初始化
/etc/init.d/qmodem_network status   # 拨号网络
/etc/init.d/qmodem_led status       # 信号/状态 LED
/etc/init.d/qmodem_reboot status    # 定时/条件重启
/etc/init.d/qmodem_usage_stats status  # 流量统计持久化
```

### ubus 对象

```sh
ubus list | grep qmodem

# 示例：查询 modem1 的基础信息（节名见 /etc/config/qmodem 的 modem-device 节）
ubus call qmodem base_info '{"config_section":"modem1"}'
ubus call qmodem network_info '{"config_section":"modem1"}'
```

完整方法表见 [rpcd 接口](../qmodem-rpcd-interface.zh-cn.md)。

### LuCI 菜单路径

安装 `luci-app-qmodem-next` 后，Web 界面出现独立菜单：**Modem → QModem → Overview**（路由 `admin/modem`）。看不到菜单时，先确认浏览器账号已获得 `luci-app-qmodem-next` 的 ACL，然后重新登录。

## 3. 最小可用配置

QModem 支持自动扫描发现模组；多数场景下安装后重启即可在 LuCI 中看到模组。手动写一个最小 `modem-device` 节用于固定配置（写入 `/etc/config/qmodem`）：

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

字段说明与更多选项（锁频、SIM 切换、自定义初始化命令等）见[旧版完整用户手册](../user-guide.zh-cn.md)的"模组设备配置"一节。修改后执行：

```sh
/etc/init.d/qmodem_init restart
```

## 4. 故障排查速查

| 现象 | 命令 / 位置 |
|---|---|
| 服务日志 | `logread -t qmodem`（也可 `logread -e qmodem` 过滤 QModem 全部条目） |
| 扫描器日志 | `logread -e modem_scand` |
| 拨号日志 | LuCI：Modem → QModem → Overview 内的拨号日志面板；或 rpcd `ubus call qmodem get_dial_log '{"config_section":"modem1"}'` |
| 内核/驱动层 | `logread -e usb -e mhi -e tty -e qmi -e mbim` |
| 端口枚举 | `ubus call qmodem get_tty_ports '{}'` |

日志与 AT 响应可能包含 IMEI、IMSI、ICCID、手机号、APN 或短信内容，公开前必须脱敏。更完整的排查流程见[故障排查](troubleshooting.zh-cn.md)。

## 5. 与逐包手动安装的对比

| 维度 | qmodem-suite（单包） | 逐包手动安装 |
|---|---|---|
| 命令数 | 1 条（`opkg install qmodem-suite`） | 每个包一条，需自行确定清单 |
| 依赖解析 | 包管理器自动 | 手动按顺序：`qmodem` → 前端 → 可选包 |
| 前端选择 | 固定为推荐的 `luci-app-qmodem-next` | 可自选，但容易误装两套主界面 |
| 升级 | 元包整体升级，依赖随之更新 | 需逐个 `upgrade`，易漏 |
| 精细裁剪 | 弱（多余的可选包也会被拉入） | 强（只装所需，如仅 `qmodem` + SMS 包） |
| 适用场景 | 首次部署、整机交付 | 定制固件、存储受限设备 |

逐包手动安装的包清单与构建来源见[安装指南](installation.zh-cn.md)。

## 6. AT 通路选择

QModem 的 AT 命令走哪条底层通路由 `qmodem.main.at_tool` 全局决定：`0`/未配置为默认的 tom_modem（可经 ubus `at-daemon`），`1` 为 sms_tool_q 直连，`2`（本版本引入）为 `qmodem-atd` unix socket（配套选项 `qmodem.main.at_socket` 指定 socket 路径，默认 `/var/run/qmodem-atd.sock`）。三通路的适用场景、超时语义、故障切换与排查命令见 [AT 通路选择](../developer/at-tool-selection.zh-cn.md)。

安装 qmodem-suite 后，无需任何额外配置即使用通路 0；通路 2 需按下一节启用 Rust 组件后再切换。

## 7. Rust 组件可选启用（qmodem-atd）

`qmodem-suite` 默认不包含 Rust 异步 AT 守护进程 `qmodem-atd`。如需启用通路 2，自行交叉编译二进制并放入固件或 suite 安装目录：

1. **交叉编译**：使用统一入口脚本 `scripts/cross-build-rust.sh`——

   ```sh
   # aarch64（路由器常见目标）
   bash scripts/cross-build-rust.sh --target aarch64-unknown-linux-musl
   # 产物：scripts/dist/aarch64-unknown-linux-musl/qmodem-atd
   ```

   产物为全静态 musl 二进制（零外部 crate，约 731 KiB，未 strip），`file` 确认 `statically linked` 后可直接拷入目标机。构建细节与手动等价命令见 [Rust 异步守护进程架构](../developer/async-daemon.zh-cn.md) 第 4 节。

2. **部署**：将二进制放入设备 PATH（如 `/usr/bin/qmodem-atd`），启动时指定 socket 路径与端口（见 async-daemon 文档第 3 节），默认 socket 路径与 `qmodem.main.at_socket` 一致。

3. **切换通路**：按上一节把 `at_tool` 置为 `2` 并重启 `qmodem_init`；socket 不可达时自动回落 tom_modem 通路。
