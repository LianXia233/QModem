# Next → Generic 功能兼容矩阵与验收标准

本文回答一个问题：**把 `luci-app-qmodem-next` 的全部功能迁入 generic 实现后，如何判定「完美兼容」是否达成。**

范围定义、模块与调用路径、测试方式、判定标准四节，每节都给出可复算的判据。文中所有数字均由仓库内脚本机械提取，可重跑：

| 脚本 | 作用 |
| --- | --- |
| `ui_matrix.py` | 提取两套前端的 ubus / uci / exec 调用集 |
| `compat_matrix.py` | 后端能力清单 + 逐视图迁移核对 + 端点解析校验 |
| `unify_layers.py` | next 页面接入 generic 视觉 / 组件 / 数据层 |
| `verify_layers.py` | 语法、导出面、孤儿标识符、require 完整性 |
| `final_check.py` | i18n 覆盖率、样式类覆盖、CRLF |

---

## 1. 需要覆盖的功能范围

### 1.1 界面页面（18 个，菜单 `admin/modem/qmodem`）

| 来源 | 页面 | 视图文件 | 菜单 order |
| --- | --- | --- | --- |
| next | Statistics | `stats.js` | 2 |
| next | Dial and Network | `network_config.js` | 5 |
| next | Messages | `sms.js` | 6 |
| next | SMS Conversation | `sms_conversation.js` | 6 |
| next | SIM Messages | `sms_sim.js` | 6.2 |
| next | SMS Forwarding | `sms_forward.js` | 6.4 |
| next | SIM Switch | `sim_switch.js` | 8 |
| next | Modem Features | `modem_config.js` | 12 |
| next | TTL | `ttl.js` | 13 |
| next | Service and Scan | `service.js` | 15 |
| generic | Overview | `status.js` | 1 |
| generic | Mobile Data | `connection.js` | 3 |
| generic | Radio and Cells | `network.js` | 4 |
| generic | Module and SIM | `system.js` | 7 |
| generic | AT Console | `terminal.js` | 9 |
| generic | Network Monitor | `monitor.js` | 10 |
| generic | Advanced | `advanced.js` | 11 |
| generic | Device Parameters | `device.js` | 14 |

next 独有但**不迁移**的 2 个页面，理由已固化：

| 文件 | 处置 | 理由 |
| --- | --- | --- |
| `next/view/qmodem/overview.js` | 退役 | 与 generic `status.js` 同为主状态页，职责重复 |
| `next/view/qmodem/debug.js` | 退役 | AT 调试能力由 generic `advanced.js` + `terminal.js` 覆盖 |
| `next/view/status/include/11_modem.js` | 退役 | LuCI 状态首页 include 片段，与 `admin_status/index/modem_overview.htm` 重叠 |

### 1.2 端点范围（三层，共 79 个前端端点）

| 类别 | 数量 | 说明 |
| --- | --- | --- |
| next 原有端点 | 61 | **迁移后缺失 0 个** |
| generic 独有端点 | 18 | `qmodem_cache.*` / `qmodem_stats.*` / `qmodem_support.*` / `qos.*` / `network.*` / 频段与流量重置等 |
| 迁移后统一前端合计 | 79 | 61 + 18，两者并集，无删减 |

后端实际提供 111 个端点，覆盖 79 个前端调用后的未解析项仅 4 个，且全部属 LuCI 核心对象、非 QModem 职责：

| 未解析端点 | 归属 |
| --- | --- |
| `network.device.status` | OpenWrt 核心 `netifd` ubus 对象 |
| `network.interface.dump` | 同上 |
| `rc.list` | OpenWrt 核心 `rc` 对象（init.d 控制） |
| `system.board` | OpenWrt 核心 `system` 对象 |

### 1.3 后端能力清单（迁移后需覆盖的调用面）

| ubus 对象 | 来源 | 端点数 |
| --- | --- | --- |
| `qmodem` | `usr/libexec/rpcd/qmodem` | 57 declared / 58 impl |
| `qmodem_cache` | `usr/libexec/rpcd/qmodem_cache` | 20 |
| `qmodem_stats` | `usr/libexec/rpcd/qmodem_stats` | 3（`daily_stats` / `stats_history` / `stats_reset`） |
| `qmodem_support` | `usr/libexec/rpcd/qmodem_support` | 3（`status` / `sync` / `sync_async`） |
| `qos` | `usr/libexec/rpcd/qos` | 3（`qos_info` / `radio_info` / `refresh`） |
| `modem_ctrl` | `usr/libexec/rpcd/modem_ctrl` | 3 declared / 5 impl |
| `qmodem.sms` | C 守护进程 `qmodem-smsd` | 17（`list` / `send` / `delete` / `mark_read` / `modem_list` / `storage_get` …） |

### 1.4 非功能性范围（同样纳入"完美兼容"）

| 维度 | 要求 |
| --- | --- |
| 视觉 | 全站单一样式注入点（`theme.js`），无第二套样式表 |
| 组件 | 单一组件层（`qmodem.controls`），无页面私有组件模块 |
| 数据 | 单一数据入口（`qmodem.controls`，含 `sms` 子命名空间） |
| 国际化 | 中文翻译对迁移后的页面全部生效 |
| 路由 | 菜单项与视图内链接全部可路由，无死链 |
| 权限 | ACL 为 next + generic + monitor + ttlfw4 的并集，无权限回退 |

---

## 2. 涉及的具体模块与调用路径

### 2.1 前端模块拓扑（迁移后）

```
htdocs/luci-static/resources/
├── qmodem/
│   ├── controls.js     统一入口：视觉令牌 + 组件 + 数据层（110 项导出）
│   ├── theme.js        QM UI v2 视觉层（设计令牌 / 组件皮肤 / 响应式 / 旧类适配）
│   ├── compat.js       LuCI 26.246+ 兼容层（String.format、E()、findParent()）
│   ├── ui.js           组件原语实现（internal）：ModemSelector / formatTime / relativeTime / hasCssClass / pollGuard
│   ├── qmodem.js       数据层内部实现：qmodem.* 的 rpc 声明与格式化
│   ├── sms.js          SMS 数据域内部实现：qmodem.sms.* 的 rpc 声明与业务聚合
│   ├── sms-pdu.js      PDU 编解码（纯逻辑，双方共用）
│   └── modem_cfg.js    静态能力数据表（modes / manufacturers / platforms / disabled_features）
└── view/qmodem/        18 个页面
```

`controls.js` 内部依赖链，无环：

```
view/*  ──►  qmodem.controls
                  ├──► qmodem.theme      （视觉注入）
                  ├──► qmodem.compat     （LuCI 版本兼容）
                  ├──► qmodem.ui         （组件原语）
                  ├──► qmodem.sms        ──► { qmodem.ui, qmodem.qmodem }
                  └──► qmodem.qmodem     （rpc 声明）
```

### 2.2 三条调用路径

**路径 A — 视觉注入（每个页面一次，隐式）**

```
页面 require qmodem.controls
  → controls.js 顶部 require qmodem.theme
  → theme.apply() 注入 <style>（设计令牌 + 组件皮肤 + 响应式 + 旧类适配 + 并入的 next 规则）
```

判据：页面内不得出现 `loadCss(...)`；包内不得存在 `.css` 文件。

**路径 B — 数据读取/下发**

```
页面 → controls.<domain 方法>  →  rpc.declare 的 ubus 调用
                                  ├─ 快照优先：qmodem_cache.snapshot / get_task / refresh
                                  ├─ 直读回退：qmodem.<method>
                                  └─ SMS 域：qmodem.sms.<method>
```

**路径 C — 组件渲染**

```
页面 → controls.ModemSelector(modems, options) → { section, select, body }
       section 挂载点（generic 皮肤 .mt-modem-bar）
       body    仍为 <tbody>，页面可继续追加自定义行（契约保持不变）
```

### 2.3 关键名映射（next 旧名 → 统一入口）

| next 旧调用 | 迁移后 | 后端端点 | 等价性核对 |
| --- | --- | --- | --- |
| `qmUI.*`（4 个在用的） | `controls.*` | 无（纯前端） | 签名与返回结构不变 |
| `smsService.*`（20 个） | `controls.sms.*` | `qmodem.sms.*` | 同一 `rpc.declare` 声明 |
| `qmodem.getStats(s)` | `controls.getUsageStats(s)` | `qmodem.get_stats` | 端点一致 |
| `qmodem.callGetSimSwitchCapabilities(s)` | `controls.getSimSwitchCapabilities(s)` | `qmodem.get_sim_switch_capabilities` | 端点一致 |
| `qmodem.rcStatus()` | `controls.rcList()` | `rc.list` | 端点一致 |
| `qmodem.<其余 27 个>` | `controls.<同名>` | 同名端点 | 名称与端点一致 |

`sendSms` / `deleteSms` / `setSmsStorage` 在两侧同名但语义不同（generic 版走 `qmodem.*`，SMS 域走 `qmodem.sms.*`），因此**统一放在 `controls.sms` 子命名空间下**，靠命名空间区分而非靠重名覆盖——这是本次改造唯一一处刻意保留的层级。

---

## 3. 测试多种功能的兼容方式

按"越靠左越便宜、越靠右越可信"的顺序，四层递进。

### 第 1 层 — 静态机械核对（每次改动必跑）

| 检查项 | 工具 | 判据 |
| --- | --- | --- |
| JS 语法 | `node --check` 全量 | FAIL = 0 |
| 端点并集 | `compat_matrix.py` | next 端点缺失 = 0 |
| 调用面解析 | `verify_layers.py` §3 | 未解析 `controls.*` 调用 = 0 |
| 孤儿标识符 | `verify_layers.py` §4 | 页面内不得出现 `qmUI` / `smsService` |
| require 完整性 | `verify_layers.py` §5 | 每个视图"缺 = 无" |
| 死链 | `compat_matrix.py` §6 | 视图内失效链接 = 0 |
| i18n 覆盖 | `final_check.py` §1 | 需要翻译的源串覆盖率不下降 |
| 样式类覆盖 | `final_check.py` §2 | 未定义类不增加 |
| 换行 | `final_check.py` §3 | 含 CRLF 文件 = 0 |
| 包内文件 | 清单比对 | 32 个文件，无遗留 `.css` |

### 第 2 层 — 后端契约测试（实机或 mock）

沿用 generic 仓库既有测试资产（`tests/run-backend-tests.sh`、`tests/mocks/{ubus,uci,jsonfilter}.py`）对 `/usr/libexec/rpcd/*` 做契约测试，验证本次并入的后端脚本（`qmodem_cache` / `qmodem_stats` / `qmodem_support` / `qos`）在 mock 环境下 `list` 与 `call` 均可返回。

### 第 3 层 — 前端单元 / 冒烟

`tests/test_sms_service.js`（随 next 迁入）在 Node 下跑 SMS 数据域逻辑；`scripts/check_all.sh` 的 8 个 stage 门禁（shell / node / js-test / rust / c / crlf / rust-cross / lua-smoke）里 `js-test` 路径已指向新位置。

### 第 4 层 — 实机端到端（最终判据）

在真机（ImmortalWrt SNAPSHOT + FM350-GL）上按功能域逐项走查：

| 功能域 | 走查项 |
| --- | --- |
| 状态总览 | 概览页模组选择器、信号/温度/带宽、最后更新时间 |
| 数据连接 | 拨号/挂断/重拨、连接状态、DNS、IP 详情、拨号日志清空 |
| 无线 | 当前频段、锁定频段、邻区扫描、小区信息、QoS 与射频信息 |
| 短信（核心回归） | 会话列表、会话详情、发送/回复、删除、标记已读、SIM 卡存储短信、存储位置切换、转发配置（Telegram / ServerChan / Webhook / 自定义脚本） |
| SIM | 卡槽读取与切换、切换能力上报 |
| 模组特性 | 模式切换、IMEI 读改、网络偏好、邻区设置、禁用特性、重启能力 |
| 统计 | 用量统计、日统计、统计重置、流量重置计划 |
| 服务与扫描 | 服务状态、模组扫描、支持库同步 |
| 终端 | AT 透传控制台、原始 PDU 发送 |
| TTL | 读取、写入、防火墙生效 |
| 监控 | 阈值配置、动作、恢复脚本 |
| 视觉一致性 | 18 个页面外观统一（同一卡片/表格/按钮皮肤） |

**实机走查要点**：短信域必须用真实卡与真实对端号码各发一条、各收一条，并核对会话气泡样式与已读状态；TTL 必须核对与 generic 版本的字节级一致性。

---

## 4. 判定「完美兼容」的具体标准

### 4.1 硬门禁（全部为 0 或 100%，任一项不满足即判定未达成）

| # | 标准 | 当前实测 |
| --- | --- | --- |
| G1 | next 原有端点在前端缺失数 = 0 | **0** ✓ |
| G2 | 页面未解析 `controls.*` 调用数 = 0 | **0** ✓ |
| G3 | 页面内 `qmUI` / `smsService` 孤儿引用数 = 0 | **0** ✓ |
| G4 | 视图 `node --check` FAIL 数 = 0 | **26/26 PASS** ✓ |
| G5 | 视图内菜单死链数 = 0 | **0** ✓ |
| G6 | 包内独立 `.css` 文件数 = 0 | **0** ✓ |
| G7 | 含 CRLF 文件数 = 0 | **0** ✓ |
| G8 | 原 `.css` 中已定义的样式类在合并后丢失数 = 0 | **0** ✓ |
| G9 | i18n 覆盖率相对迁移前下降 = 0 | **0**（未被翻译的 87 条在迁移前同样未被翻译） ✓ |
| G10 | 后端 `list` 与 `call` 契约测试通过率 = 100% | 待跑（第 2 层） |

### 4.2 软指标（可接受缺口，但必须显式记录）

| 指标 | 当前值 | 说明 |
| --- | --- | --- |
| i18n 覆盖率 | **87.3%**（596 / 683） | 缺 87 条，均为**迁移前就未翻译**的源串，非本次引入；集中在 `monitor.js`（generic 新增监控页）与 next 的 `modem_config.js` 长句 |
| 未定义样式类 | **2**（`sms-message-failed`、`sms-sim-messages`） | 原 next 样式表中同样未定义，属既有问题 |
| 核心对象未解析端点 | **4** | `network.*` / `rc.list` / `system.board`，属 OpenWrt 核心，不归 QModem |

### 4.3 功能对等判定

对 1.1 节的 18 个页面逐个给出结论：

| 结论 | 含义 |
| --- | --- |
| **完整迁移** | next 侧原有全部端点与交互在新页面中均可触达，且视觉已统一 |
| **替代迁移** | 功能由 generic 的等价页面承载（`overview.js` → `status.js`） |
| **有意退役** | 功能不再提供且已在文档中说明理由（`debug.js`、`11_modem.js`） |

本次 10 个 next 页面全部落入「完整迁移」，10 个视图逐条核对结果均为 `OK`（`missing ubus = 0`），1 个页面为「替代迁移」，2 个为「有意退役」。

### 4.4 一句话判据

> **当 G1–G10 全部为 0 或 100%，且第 4 层实机走查中 12 个功能域全部可用、18 个页面外观一致时，判定「完美兼容」达成。**
>
> 当前状态：G1–G9 已达成，G10 待在实机跑测试套件；第 4 层实机走查中「状态总览 / 数据连接 / 无线 / 短信 / SIM / 终端」六个高风险域需优先验证。

---

## 5. 本轮一并修掉的问题

| 问题 | 影响 | 处置 |
| --- | --- | --- |
| 5 个视图仍在引用改名前的 `qmodem/qmodem-next.css` | 样式 404，页面裸奔 | 改为 `qmodem.css`，随后整体并入 `theme.js` |
| 统一包缺 `po/` 目录，而视图有 1665 处 `_()` | 中文界面全部退回英文 | 合并 next 683 条 + generic 1211 条为 1852 条 |
| next 页面自加载样式表 + 使用自有组件层 | 与 generic 页面并存两套视觉语言 | 全部改走 `qmodem.controls`，组件重皮肤为 generic |
| `luci-app-qmodem-ttl` 的 init 脚本调用不存在的 `/etc/firewall.d/qmodem_ttl` | 启动报错 | 删除该行，脚本归入后端包 |
| next 的 `overview.js` 与 generic `status.js` 双主状态页 | 菜单重复、职责不清 | `overview.js` 退役 |
