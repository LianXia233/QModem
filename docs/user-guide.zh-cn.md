# QModem 用户手册

本指南全面概述了适用于 OpenWRT 的 QModem 软件包，内容包括安装、功能说明、基于源代码的详细配置选项以及故障排除步骤。

## 1. 快速入门

### 编译方法

要从源码构建 QModem，请将自定义软件源添加到您的 OpenWRT 构建环境中：

```bash
# 添加 QModem 软件源
echo 'src-git qmodem https://github.com/FUjr/QModem.git;main' >> feeds.conf.default

# 更新并安装软件源包
./scripts/feeds update qmodem
./scripts/feeds install -a -p qmodem

# (可选) 强制安装以覆盖现有的驱动程序/应用
./scripts/feeds install -a -f -p qmodem

# 进入构建配置菜单
make menuconfig
```

在 `make menuconfig` 中，导航到 `LuCI -> Applications` 以选择 QModem 软件包。

最后，构建固件：

```bash
make -j$(nproc)
```

### 软件包

> **状态**：自 2026-10 旧 Lua 栈退役后，整套只交付两个核心包——`qmodem`（一体化后端）
> 与 `luci-app-qmodem-next`（唯一前端）。下列旧版 Lua 分体包已从仓库删除，不再编译与产出；
> 本手册保留其字段说明作为历史参考。当前包清单与安装命令见
> [安装指南](user/install-suite.zh-cn.md)。

* **`qmodem` (后端)**: 一体化后端，包含模组识别、拨号脚本、ubus 数据服务、短信守护与全部命令行工具。
* **`luci-app-qmodem-next` (核心前端)**: 纯 JavaScript 界面，提供模组管理、状态监控和拨号的核心 Web 界面，已 `DEPENDS:=+qmodem`。
* **`luci-app-qmodem-ttlfw4`**: TTL 规则界面（fw4/nft 后端），按需单独安装。

### 从 Release 安装

从 Release 安装预编译的 `.ipk` 软件包时，请注意内核依赖性。

**重要提示**：您的路由器的内核版本必须与用于构建软件包的内核版本兼容。如果遇到有关内核版本不匹配的错误，您可以尝试强制安装，但这可能会导致不稳定或功能无法使用。

```bash
# 安装示例（前端会自动拉入后端）
opkg install luci-app-qmodem-next.ipk

# 强制安装示例 (请谨慎使用)
opkg install luci-app-qmodem-next.ipk --force-depends
```

## 2. 功能介绍

### `luci-app-qmodem-next` (核心)

主插件为管理您的模组提供了全面的界面。

* **模组信息**: 显示详细状态，包括制造商、型号、固件、IMEI、信号质量 (RSSI, RSRP, RSRQ, SINR) 和网络注册详情。
  ![模组信息](../imgs/modem_info.png)
* **拨号控制**: 允许您配置和控制模组的数据连接。
  ![拨号总览](../imgs/dial_overview.png)
* **高级调试**: 提供锁频段、锁小区和发送自定义 AT 命令的工具。
  ![高级设置](../imgs/modem_debug_lock_band.png)

### 短信（`luci-app-qmodem-next` 内置）

原独立包 `luci-app-qmodem-sms` 已退役，短信门户并入核心前端的 SMS 页面。

* 发送和接收短信。
* 查看消息历史记录。
* 支持 PDU 模式以进行原始消息发送。
  ![短信界面](../imgs/modem_sms.png)

### 多 WAN（历史：`luci-app-qmodem-mwan`，已退役）

原独立包已退役，多拨规则下沉到 `qmodem` 内置脚本，界面配合官方 `luci-app-mwan3` 使用。

* 设置负载均衡规则以在多个互联网连接之间分配流量。
* 配置故障转移以在主连接失败时自动切换到备用连接。
  ![MWAN 配置](../imgs/modem_mwan.png)

### TTL（`luci-app-qmodem-ttlfw4`）

允许您修改 IP 包的生存时间 (TTL) 值。这有时用于防止运营商检测和限制热点或网络共享数据的使用。TTL 界面以独立包 `luci-app-qmodem-ttlfw4` 交付（原 `luci-app-qmodem-ttl` 已退役）。

### SIM 卡硬件切换（历史：`luci-app-qmodem-hc`，已退役）

原独立包已退役，HC-G80 等 GPIO 控制的 SIM 卡槽切换能力由核心前端的 SIM 切换视图与 `qmodem` 内置脚本提供。

## 3. 配置选项

本节详细介绍了 QModem LuCI 界面中可用的配置选项，这些选项均在源代码中定义。

### 模组设备配置 (`qmodem.modem-device`)

这些设置针对每个检测到的模组。


| 选项                                        | 类型      | 描述                                                        |
| :------------------------------------------ | :-------- | :---------------------------------------------------------- |
| **固定设备** (`is_fixed_device`)            | `Flag`    | 如果启用，模组的配置在连接/断开事件时不会自动更新。         |
| **插槽路径** (`slot`)                       | `List`    | 模组连接的物理 USB 或 PCIe 插槽路径。                       |
| **接口类型** (`data_interface`)             | `List`    | 连接接口类型 (USB 或 PCIe)。                                |
| **别名** (`alias`)                          | `Text`    | 模组的自定义、用户友好名称。                                |
| **软重启** (`soft_reboot`)                  | `Flag`    | 启用模组的软重启，而不是完全断电重启。                      |
| **PDP 上下文索引** (`pdp_index`)       | `Integer` | 用于连接的 PDP 上下文索引 (默认为 1)。                      |
| **制造商** (`manufacturer`)                 | `List`    | 模组的制造商 (例如，Quectel, Fibocom)。                     |
| **平台** (`platform`)                       | `List`    | 模组的芯片组平台 (例如，SDX55, SDX62)。                     |
| **AT 端口** (`at_port`)                     | `Text`    | AT 命令端口的设备路径 (例如，`/dev/ttyUSB2`)。              |
| **支持的模式** (`modes`)                    | `List`    | 模组支持的网络模式。                                        |
| **启用** (`enabled`)                        | `Flag`    | 启用或禁用此模组配置。                                      |
| **禁用的功能** (`disabled_features`)        | `List`    | 允许您为此模组隐藏 UI 中不支持的功能 (例如，锁频段、短信)。 |
| **频段配置** (`wcdma_band`, `lte_band`, 等) | `Text`    | 指定用于锁定的频段 (例如，"1/3/5")。                        |
| **初始化后/拨号前延迟**                     | `Integer` | 初始化后或拨号前等待执行自定义命令的秒数。                  |
| **初始化后/拨号前 AT 命令**                 | `List`    | 在连接过程的不同阶段执行的自定义 AT 命令列表。              |

### 拨号配置 (`qmodem.modem-device`)

这些设置控制数据连接配置文件。


| 选项                                      | 标签页 | 类型      | 描述                                                                     |
| :---------------------------------------- | :----- | :-------- | :----------------------------------------------------------------------- |
| **启用拨号** (`enable_dial`)              | 常规   | `Flag`    | 启用或禁用此模组拨号功能的主开关。                                       |
| **模组别名** (`alias`)                    | 常规   | `Text`    | 此拨号配置文件的自定义名称。                                             |
| **AT 端口 / 短信 AT 端口**                | 常规   | `List`    | 选择用于数据和短信功能的已验证 AT 端口。                                 |
| **DNS** (`dns_list`)                      | 常规   | `List`    | 要使用的自定义 DNS 服务器列表。如果为空，则使用运营商的 DNS。            |
| **桥接模式** (`en_bridge`)                | 高级   | `Flag`    | 启用桥接/透传模式。(注意：仅适用于某些 Quectel 5G 模组)。                |
| **桥接端口** (`bridge_port`)              | 高级   | `Text`    | 透传桥接使用的端口。设备级 `bridge_port` 会覆盖槽位默认桥接端口。        |
| **不修改 resolv.conf** (`do_not_add_dns`) | 高级   | `Flag`    | 防止`quectel-cm` 自动将 DNS 服务器添加到 `/etc/resolv.conf`。            |
| **RA Master** (`ra_master`)               | 高级   | `Flag`    | 如果启用，此接口将成为 IPv6 路由器通告主接口。只能有一个接口作为主接口。 |
| **扩展前缀** (`extend_prefix`)            | 高级   | `Flag`    | 如果启用，委派的 IPv6 前缀将应用于 LAN 区域。                            |
| **PDP 类型** (`pdp_type`)                 | 高级   | `List`    | 连接的 IP 协议 (`IPv4`, `IPv6`, 或 `IPv4/IPv6`)。                        |
| **APN / APN 2** (`apn`, `apn2`)           | 高级   | `Text`    | SIM 卡槽 1 和 2 的接入点名称。可以留空以进行自动检测。                   |
| **认证类型** (`auth`)                     | 高级   | `List`    | 认证协议 (`NONE`, `PAP`, `CHAP`, 或 `MsChapV2`)。                        |
| **PAP/CHAP 用户名/密码**                  | 高级   | `Text`    | 如果需要，用于认证的凭据。                                               |
| **PIN 码** (`pincode`)                    | 高级   | `Text`    | SIM 卡的 PIN 码，如果已锁定。                                            |
| **跃点数** (`metric`)                     | 高级   | `Integer` | 此接口的路由跃点数。值越小，优先级越高。                                 |

### MWAN 配置 (`qmodem_mwan`)


| 选项                              | 类型      | 描述                                                    |
| :-------------------------------- | :-------- | :------------------------------------------------------ |
| **启用 MWAN** (`enable_mwan`)     | `Flag`    | 启用或禁用 MWAN 功能的主开关。                          |
| **粘性模式** (`sticky_mode`)      | `Flag`    | 启用后，来自同一源 IP 的连接将始终使用相同的 WAN 接口。 |
| **粘性超时** (`sticky_timeout`)   | `Integer` | 维持粘性连接的持续时间（秒）。                          |
| **成员接口** (`member_interface`) | `List`    | 要添加到 MWAN 池的网络接口。                            |
| **跟踪 IP** (`member_track_ip`)   | `List`    | 用于 Ping 以监控连接健康状况的 IP 地址列表。            |
| **优先级** (`member_priority`)    | `Integer` | 接口的优先级 (1-255)。数字越小，优先级越高。            |
| **权重** (`member_weight`)        | `Integer` | 负载均衡权重 (1-255)。数字越大，接收的流量越多。        |

### TTL 配置 (`qmodem_ttl`)


| 选项                | 类型      | 描述                                      |
| :------------------ | :-------- | :---------------------------------------- |
| **启用** (`enable`) | `Flag`    | 启用或禁用 TTL 修改。                     |
| **TTL** (`ttl`)     | `Integer` | 要应用于数据包的 TTL 值 (例如，64, 128)。 |

## 4. 故障排除

当您遇到问题时，提供详细的日志对于诊断至关重要。在创建 GitHub Issue 之前，请按照以下步骤收集信息。

### 必需信息收集

在创建 GitHub Issue 之前，请通过 SSH 执行指定命令收集以下信息：

#### 1. 基本系统信息

在 Issue 描述中提供您的路由器型号、OpenWRT 版本和模组型号。

#### 2. 模组配置

```bash
uci show qmodem
# 显示模组配置
```

#### 3. 网络配置

```bash
uci show network
# 显示网络配置
```

#### 4. 系统日志

```bash
logread
# 查看系统日志
```

#### 5. 内核日志

```bash
dmesg
# 查看内核日志
```

#### 6. USB 设备信息（仅适用于 USB 模组）

```bash
lsusb
# 列出 USB 设备
```

#### 7. PCIe 设备信息（仅适用于 PCIe 模组）

```bash
lspci
# 列出 PCIe 设备
```

#### 8. 模组扫描（如果出现扫描问题）

```bash
/usr/share/qmodem/modem_scan.sh scan
# 扫描模组
```

### 创建 GitHub Issue

1. 访问 [QModem GitHub Issues 页面](https://github.com/Fujr/QModem/issues)。
2. 使用"Bug Report"模板。
3. 使用上面收集的信息填写所有必需的部分。
4. 将命令输出粘贴到模板中相应的可折叠部分。
