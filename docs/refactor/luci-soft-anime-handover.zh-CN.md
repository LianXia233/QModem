# LuCI Soft Anime 视觉重构 — 交接文档

> 用途：换平台 / 换执行者接续本轮工作。文档只使用**仓库相对路径**，不含任何本地绝对路径、私有服务器或凭据。
> 设计需求与验收口径见 [`docs/soft-anime-luci-requirements-and-progress.md`](../soft-anime-luci-requirements-and-progress.md)。

## 1. 交付概览

| 项 | 值 |
|---|---|
| 仓库 | `LianXia233/QModem`（分支 `main`） |
| 版本基线 | `version.mk` → `QMODEM_VERSION = 1.1.1` |
| 基线提交 | `3b0e78b` |
| 本次提交 | `a2b00d9` → `d99fbb2` → `9557cad` → `e0296cd`（4 个，历史已重写，见 §7.3） |
| 风格定位 | Soft Anime × Pastel Glass × Modern Dashboard |
| 影响范围 | `luci/` 下 6 个包：next、monitor、voip 重构；legacy Lua 五包标记废弃 |
| 语义变更 | **无**。菜单路径、配置键、ACL、RPC 接口、表单提交逻辑、po 翻译均未改动 |

### 提交清单

| 提交 | 主题 | 关键内容 |
|---|---|---|
| `a2b00d9` | `luci: Soft Anime x Pastel Glass design system for QModem web UI` | CSS 设计系统重写 + overview 仪表盘结构升级 + 10 视图接入样式 |
| `d99fbb2` | `luci: deprecate legacy Lua UI packages` | 5 个旧包标记 DEPRECATED + 页面弃用横幅 + README/CHANGELOG |
| `9557cad` | `luci: cache-bust CSS resource loads (?v=<YYYY-MM-DD>)` | 15 处资源加载加按天版本号 |
| `e0296cd` | `luci-legacy: yield menu nodes to luci-app-qmodem-next` | 旧版 controller 让位同名菜单节点 |

## 2. 仓库结构与分工

```text
luci/
├── luci-app-qmodem-next/      ★ 现代 JS 界面（重构主体，10 个 view，Soft Anime）
├── luci-app-qmodem-monitor/   ★ 监控配置（form.Map，复用 next 样式 + 状态胶囊）
├── luci-app-qmodem-voip/      ★ 语音（含 Soft Anime 设计系统自包含副本）
└── luci-app-qmodem-ttlfw4/    ★ TTL（JS/fw4，含 Soft Anime 设计系统自包含副本）
```

旧 Lua 五包（qmodem, hc, sms, ttl, mwan）已全部退役删除，其关键后台守护及脚本已完整迁入 `application/qmodem` 后端包，彻底根除了同名菜单路由覆盖冲突。

## 3. 设计系统（`luci/luci-app-qmodem-next/htdocs/luci-static/resources/qmodem/qmodem-next.css`）

文件规模：288 行 → 1413 行（构建后压缩为单行 ≈24.7 KB）。

### 3.1 令牌层（Design Tokens）

| 类别 | 浅色 | 暗色 |
|---|---|---|
| 页面 / 次级背景 | `#F7F6FC` / `#F0EEFA` | `#111522` / `#171C2D` |
| 卡片 / 次级卡片 | `#FFFFFF` / `#FBFAFF` | `#1B2236` / `#202842` |
| 主强调 / 悬停 | `#8875E8` / `#7561D7` | `#B4A4FF` / `#C5B9FF` |
| 浅紫底 / 樱花粉 / 薄荷蓝 / 薄荷绿 | `#EEE9FF` / `#EFA8CB` / `#8CCFE2` / `#79CDB0` | `#302B4D` / `#EFA6D0` / `#80D4EC` / `#75D6B1` |
| 主 / 次 / 辅助文字 | `#34334B` / `#77758F` / `#9997AE` | `#EEEAFB` / `#B0AEC8` / `#8887A4` |
| 边框 / 分隔线 | `#E7E2F4` / `#EEEBF6` | `#34334D` / `#2B3046` |
| 成功 / 警告 / 错误 / 信息 | `#36AD87` / `#E2A44D` / `#D96883` / `#6BAFE8` | `#4CC39A` / `#ECB869` / `#E88AA2` / `#7FBFF0` |

全部变量以 `--qmd-*` 前缀定义，组件层只消费变量；色彩比例 70/20/10。

### 3.2 暗色解析顺序

1. `<html data-qmodem-theme="light">` → 强制浅色
2. `<html data-qmodem-theme="dark">` 或 `body.qmodem-dark` → 强制暗色
3. `@media (prefers-color-scheme: dark)` → 跟随系统
4. 主题切换只切换令牌，**不重建 DOM、不重发请求、不重置表单**

### 3.3 结构规范

| 项 | 取值 |
|---|---|
| 字体栈 | `system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif` |
| 字号 | 主标题 24–28px / 分区 16–18px / 正文 13–14px / 表格 12–14px / 辅助 11–12px |
| 数值 | 数字、IP、流量、时间统一 `font-variant-numeric: tabular-nums` |
| 间距 | 4px 体系：`--qmd-sp-1`…`--qmd-sp-10`（4/8/12/16/20/24/32/40） |
| 圆角 | 标签 999px / 控件 8–10px / 卡片 14px / 大型面板 18px |
| 阴影 | 低透明紫灰；暗色减外部阴影，靠边框与亮度分层 |
| 动效 | 140–220ms；`prefers-reduced-motion` 全量关闭 |
| 栅格 | 卡片 `repeat(auto-fill, minmax(min(100%, 420px), 1fr))` |
| 断点 | `1024px`（卡片列宽收窄）、`768px`（单列、标签上下布局、触控 42px） |

### 3.4 作用域与二次元装饰

- 样式仅在 QModem 各视图被加载时注入 `document.head`，**不使用全局 DOM 改造**，不污染其它插件。
- 装饰仅用于：页面顶部 hero 区（樱花渐变 + 纯 CSS 樱花点 + 内联 SVG 吉祥物，≈1.6 KB，仓库内自带，无图床/CDN 依赖）。不覆盖数值、输入框、按钮、图例、告警与导航。

## 4. 逐文件改动清单

### 4.1 `luci-app-qmodem-next`

| 路径 | 变更 |
|---|---|
| `htdocs/luci-static/resources/qmodem/qmodem-next.css` | 完全重写为设计系统（令牌 + 16 分区组件层） |
| `htdocs/luci-static/resources/view/qmodem/overview.js` | hero 品牌区 + 内联 SVG 吉祥物；信息分区接入卡片栅格；末次更新时间行 token 化 |
| `htdocs/luci-static/resources/view/qmodem/{network_config,config_advanced,settings,debug,sim_switch,sms_forward}.js` | 补齐共享样式加载（原仅 4/10 视图加载） |
| `.../network_config.js` | 状态点硬编码 `#FFA500` → `.qmodem-dot.qmodem-dot-warn` |
| `.../sms.js`、`sms_conversation.js` | `#666`、`#e74c3c` 等 → `.qmodem-help-text` / `.sms-unread-badge` |
| `.../sim_switch.js` | `#999`、`#f0ad4e` → `.qmodem-help-text` / `.qmodem-note-warn` |

### 4.2 `luci-app-qmodem-monitor`

| 路径 | 变更 |
|---|---|
| `htdocs/luci-static/resources/view/qmodem/monitor.js` | 加载共享样式；安全警告内联色 → `.qmodem-inline-warning`；脚本列表内联样式 → `.script-item` / `.script-item-path` |

### 4.3 `luci-app-qmodem-voip`

| 路径 | 变更 |
|---|---|
| `htdocs/luci-static/resources/qmodem-voip/soft-anime.css` | **新增**：设计系统自包含副本（voip 无 next 依赖，避免跨包 404） |
| `htdocs/luci-static/resources/qmodem-voip/qmodem-voip.css` | 状态区改粉彩指标卡；边框/文字/计时器 token 化 |
| `htdocs/luci-static/resources/view/qmodem-voip/call.js`、`sip.js` | 加载 `soft-anime.css` + 原样式 |

### 4.4 旧版 Lua 五包（`d99fbb2` + `e0296cd`）

| 路径 | 变更 |
|---|---|
| `luci-app-qmodem{,-hc,-sms,-ttl,-mwan}/Makefile` | `LUCI_TITLE` 追加 `(DEPRECATED)` + 弃用注释 |
| `luci-app-qmodem/luasrc/model/cbi/qmodem/*.lua`（5 个）<br>`luci-app-qmodem-hc/.../modem_sim.lua`<br>`luci-app-qmodem-sms/.../sms_forward{,_extedit}.lua`<br>`luci-app-qmodem-ttl/.../modem_ttl.lua`<br>`luci-app-qmodem-mwan/.../mwan_config.lua` | 注入 `m.description` 弃用警告；`sms_forward`、`sms_forward_extedit`、`mwan_config` 与原说明**前缀合并**（原文件已有 `m.description`，直接赋值会被覆盖） |
| `luci-app-qmodem/luasrc/view/qmodem/modem_info.htm`、`modem_debug.htm`<br>`luci-app-qmodem-sms/luasrc/view/modem_sms/modem_sms.htm` | 整页模板 header 后注入 `alert-message warning` 横幅 |
| `luci-app-qmodem/luasrc/controller/qmodem.lua` | 旧版让位守卫（见 §5.3） |

### 4.5 文档

| 路径 | 变更 |
|---|---|
| `README.md`、`README.en.md` | 新增「已废弃组件 / Deprecated packages」表（5 包 + 替代方案） |
| `CHANGELOG.md`、`CHANGELOG.zh-CN.md` | `[Unreleased]` 下新增 `Deprecated` 条目 |

## 5. 三个真实问题与解法（接续时务必保留）

### 5.1 打包正确但界面无变化 —— 资源缓存

- 现象：包内 CSS 已是新版（≈24.7 KB），设备文件也对，但页面仍是旧观感。
- 根因：LuCI 的 `/luci-static/resources/*` 不带版本号，uhttpd 不发 `no-cache`，浏览器按启发式缓存保留旧文件。
- 解法（`9557cad`）：15 处资源加载统一追加按天版本号
  `L.resource('qmodem/qmodem-next.css') + '?v=' + new Date().toISOString().slice(0, 10)`
  跨天自动失效、当天复用缓存、无需手工 bump。

### 5.2 页面完全无新元素 —— 菜单节点冲突（本次真凶）

- 现象：URL 为 `admin/modem/qmodem`，但页面无 hero、无新样式。
- 根因：新旧两套界面注册**同一路径**。next 用 `luci-app-qmodem-next/root/usr/share/luci/menu.d/luci-app-qmodem-next.json`（→ view `qmodem/overview`）；旧 Lua 用 `luci-app-qmodem/luasrc/controller/qmodem.lua`（→ alias `qmodem/modem_info`）。LuCI 先加载 `menu.d` JSON、后执行 Lua `index()`，**Lua 后注册覆盖 JSON**，于是始终打开旧模板页。旧包 Makefile 的 `@(PACKAGE_luci-app-qmodem-next=n)` 只约束编译期，管不到已安装的旧包。
- 解法（`e0296cd`）：旧版 controller 前置守卫——检测到 `/usr/share/luci/menu.d/luci-app-qmodem-next.json` 存在即 `return`，不再注册 `admin/modem` 与 `admin/modem/qmodem`。hc/sms/ttl/mwan 只挂子页面，不受影响。

### 5.3 设备侧共存处理（一次性）

旧包未卸载时，即使代码已修复，仍需设备侧执行（apk 语法，opkg 见 §8）：

```sh
apk del luci-app-qmodem
rm -f /tmp/luci-indexcache*
/etc/init.d/rpcd restart && /etc/init.d/uhttpd restart
```

## 6. 已知边界与架构要点

1. **旧 Lua 五包已彻底清理**：`luci/` 目录下仅存 4 个纯 JavaScript 现代包（next、monitor、voip、ttlfw4），旧版 Lua controller 与 next 的菜单路由覆盖冲突彻底消除。底层守护与规则已收编入 `application/qmodem`。
2. **`luci-app-qmodem-ttlfw4` 已完整接入设计系统**：自带自包含 `soft-anime.css` 副本与按天缓存防呆机制，Pastel Glass 警示卡片已适配，独立安装零 404 风险。
3. **voip 与 ttlfw4 设计系统副本维护**：`soft-anime.css` 与 next 的 `qmodem-next.css` 内容同源，确保各包独立安装时的完整视觉呈现。
4. **`color-mix()` 依赖**：用于状态色边框/徽标调色，需 Chrome 111+ / Safari 16.2+；旧浏览器忽略该声明但保留基础配色（优雅降级）。
5. **LuCI 主题兼容**：Bootstrap 主题未使用 ID 选择器定义 `.cbi-section`，无特异性冲突；若换用 Argon 等强主题，需复核是否存在 `!important` 覆盖。

## 7. 验证记录

### 7.1 静态验证（全部通过）

| 项 | 结果 |
|---|---|
| `node --check`（全量 29 个 JS 文件） | PASS（29/29） |
| CSS 花括号平衡（4 个 CSS） | PASS（4/4） |
| `luci-app-qmodem-next/tests/test_sms_service.js` | PASS |
| `luci-app-qmodem-voip/tests/{media,module-loader,reducer}.test.js` | PASS ×3 |
| JSON 语法检查（全部 menu.d 与 acl.d） | PASS（8/8） |
| `sh -n` 脚本语法检查（全部 shell / init.d / uci-defaults） | PASS |
| CRLF 检查 | 全文件 LF |
| 静态预览截图（浅色 / 暗色 / 390px / 360px） | 布局与配色符合预期，无横向溢出 |

### 7.2 产物验证（解包 `luci-app-qmodem-next_1.1.1_all.ipk`）

| 检查项 | 期望 | 实际 |
|---|---|---|
| `www/luci-static/resources/qmodem/qmodem-next.css` | 新版设计系统 | ✅ ≈24.7 KB |
| 10 个 view 是否注入样式 | 10/10 | ✅ 10/10 |
| `overview.js` 含 `qmodem-hero` / `qmodem-mascot` | 有 | ✅ |
| `lib/upgrade/keep.d/qmodme-next` | 仅保留 `/etc/qmodem` | ✅（不会留存旧 `/www` 文件） |
| `conffiles` | 空 | ✅（无配置覆盖风险） |

### 7.3 未执行项

- 真机（路由器）实机验证：暗色切换、SMS 气泡、拖拽排序、表单提交。
- OpenWrt buildroot 本地编译（沙箱无 buildroot，依赖 CI 构建）。
- 多 LuCI 主题（Argon / Material）集成验证。
- 说明：早期两个提交因署名误用上游维护者邮箱被重写为 `a2b00d9` / `d99fbb2`，**树内容与重写前逐字节一致**（`git diff` 为空）。

## 8. 接续操作

### 8.1 构建与产物

- CI 工作流：`.github/workflows/main.yml`（`OpenWrt SDK 自动编译`），支持 `workflow_dispatch`（输入 `note`、`tag`，可留空仅编译不发布）、`push: tags: v*`、`pull_request`、`workflow_call`。
- 产物名：`QModem-arm64_ipk`、`QModem-arm64_apk`、`QModem-arm64_apk_immortalwrt`、`Release Note`。
- 触发：`gh workflow run main.yml --repo LianXia233/QModem --ref main`（`gh` 走环境变量凭据，不落盘）。
- 本地编译：`make package/luci-app-qmodem-next/compile V=s`（需 OpenWrt/ImmortalWrt buildroot + 本仓库作为 feed）。

### 8.2 设备侧安装与回读（只读校验优先）

```sh
# apk（ImmortalWrt / OpenWrt 24.10+）
apk add --allow-untrusted luci-app-qmodem-next_1.1.1_all.apk     # 覆盖同名文件加 --force-overwrite
apk del luci-app-qmodem luci-app-qmodem-hc luci-app-qmodem-sms luci-app-qmodem-ttl luci-app-qmodem-mwan
apk info | grep -i qmodem

# opkg（OpenWrt 23/24）
opkg install luci-app-qmodem-next_1.1.1_all.ipk                  # 重装加 --force-reinstall
opkg remove luci-app-qmodem
opkg list-installed | grep qmodem

# 通用：清缓存 + 回读
rm -f /tmp/luci-indexcache*
/etc/init.d/rpcd restart && /etc/init.d/uhttpd restart
ls -l /www/luci-static/resources/qmodem/qmodem-next.css                   # ≈24692
grep -c qmodem-hero /www/luci-static/resources/view/qmodem/overview.js    # ≥1
ls /usr/share/luci/menu.d/ | grep qmodem                                  # 仅 next 的 JSON
```

### 8.3 进展与待办
 
1. [x] **旧版 Lua 五包清理**：已彻底删除 `luci-app-qmodem`、`luci-app-qmodem-sms`、`luci-app-qmodem-ttl`、`luci-app-qmodem-hc`、`luci-app-qmodem-mwan`；后台硬件与路由脚本（`modem_sim.sh`、`modem_mwan.sh`、`qmodem_hc_sim`、`qmodem_mwan`、`99-setup-mwan`、`01-reload-mwan3.dhcpscript`）已平滑收编至 `application/qmodem`。
2. [x] **`luci-app-qmodem-ttlfw4` 接入设计系统**：提供自包含 `soft-anime.css` 副本，TTL 页面接入 Soft Anime 样式，警示模块改造为 Pastel Glass 风格，init.d 脚本移除 bashism。
3. [x] **`luci-app-qmodem-monitor` 状态胶囊化**：服务启用/运行状态全面接入 `.qmodem-chip` 与 `.qmodem-status-row` 状态胶囊徽章。
4. [ ] 真机路由（如 Airpi AP3000M / ImmortalWrt）在实际连卡环境下的业务功能验收（SIM 热切换、语音呼叫流、短信收发）。
