# Soft Anime LuCI 界面设计要求与实施进度文档

> **✨ 状态更新（2026-10-15）：本方案已 100% 全量实施完成并通过实机测试验收。**
> 涵盖 `luci-app-qmodem-next` 全量 11 个独立视图模块、`luci-app-qmodem-ttlfw4`、`luci-app-qmodem-voip` 以及全局 Soft Anime Pastel Glass 毛玻璃弹窗体系；并在真机 ImmortalWrt (26.275, aarch64) + 广和通 FM350-GL 5G 模组环境下完成 AT 实时调度与端到端渲染验证。

本文档记录 OpenWrt / ImmortalWrt `luci-app-qmodem` 实施 **Soft Anime × Pastel Glass × Modern Dashboard** 视觉重构的完整需求规范、实施现状与后续工作跟踪。

---

## 一、 设计定位与核心需求规范

### 1.1 设计语言
* **定位**：Soft Anime × Pastel Glass × Modern Dashboard。
* **色彩基调**：淡紫、薰衣草紫、樱花粉、薄荷蓝、薄荷绿。
* **视觉元素**：现代感卡片、圆润圆角（14–16px / 18–22px）、超轻量柔和阴影、细腻边框与微量毛玻璃效果（Glassmorphism）。
* **信息层级**：结构化网格系统、清晰的指标对比、避免视觉杂乱，同时保持路由后台的高可用性与工程专业度。
* **离线与零依赖**：**严禁使用外部 CDN 或联网字体/图床**，必须使用系统内置字体栈，所有插画及图形均使用纯内联矢量 SVG 实现。
* **多端与双模适配**：
  * 响应式覆盖：桌面端（≥1024px）、平板端（680px–980px）、手机端（≤680px，重点 360/390/430px）。
  * 浅色模式（Light Mode）与暗色模式（Dark Mode）全覆盖，支持跟随系统与手动自由切换。

### 1.2 全局 CSS Design Tokens 规范

| 属性类型 | CSS Variable | 浅色模式 (Light) | 暗色模式 (Dark) | 用途与语义 |
| :--- | :--- | :--- | :--- | :--- |
| **背景色** | `--qm-page-bg` / `--qm-bg` | `#F7F6FC` | `#111522` | 页面主背景 |
| **次级背景** | `--qm-surface-sub` | `#F0EEFA` | `#171C2D` | 次级容器/凹陷区/侧边底色 |
| **卡片背景** | `--qm-surface` / `--qm-surface-card` | `#FFFFFF` | `#1B2236` | 主数据卡片/功能面板 |
| **浮层背景** | `--qm-surface-popover` | `#FFFFFF` | `#222A41` | 下拉菜单/浮动层/弹窗 |
| **主强调色** | `--qm-accent` | `#8875E8` | `#B4A4FF` | 主按钮/高亮项/主曲线 |
| **强调悬停** | `--qm-accent-hover` | `#7561D7` | `#C5B9FF` | 按钮悬停/交互激活 |
| **浅紫背景** | `--qm-purple-soft` | `#EEE9FF` | `#302B4D` | 徽章浅底/选中高亮浅底 |
| **樱花粉** | `--qm-sakura` | `#EFA8CB` | `#EFA6D0` | 辅助强调/上行流量/二次元装饰 |
| **薄荷蓝** | `--qm-mint-blue` | `#8CCFE2` | `#80D4EC` | 辅助强调/下行流量/副指标 |
| **薄荷绿** | `--qm-mint-green` | `#79CDB0` | `#75D6B1` | 辅助强调/良好状态 |
| **主文字** | `--qm-text` | `#34334B` | `#EEEAFB` | 标题/关键指标数值 |
| **次级文字** | `--qm-text-soft` | `#77758F` | `#B0AEC8` | 标签/副标题/常规正文 |
| **辅助文字** | `--qm-text-muted` | `#9997AE` | `#8887A4` | 说明提示/禁用文字/单位 |
| **边框色** | `--qm-border` | `#E7E2F4` | `#34334D` | 卡片外框/分割边界 |
| **浅分隔线** | `--qm-border-soft` | `#EEEBF6` | `#2B3046` | 卡片内部分区/细线 |
| **成功色** | `--qm-ok` | `#36AD87` | `#42D8A7` | 连接正常/SIM就绪 |
| **警告色** | `--qm-warn` | `#E2A44D` | `#EBB663` | 降频/漫游/部分异常 |
| **错误色** | `--qm-danger` | `#D96883` | `#E87A95` | 断开/未插卡/硬件故障 |
| **信息色** | `--qm-info` | `#6BAFE8` | `#82C0F7` | 提示信息/一般通知 |

---

## 二、 架构与实现进展清单

### 2.1 全局主题系统与基础设施 (`theme.js`)
- [x] **CSS 变量系统**：在 `theme.js` 中完整定义浅色与暗色模式的全部 Design Tokens，统一命名空间 `--qm-*`。
- [x] **12 列响应式网格**：引入 `.qm-dashboard-grid` 和 `.qm-col-1` 至 `.qm-col-12` 栅格类，支持平板与手机端自适应折叠。
- [x] **矢量二次元吉祥物**：内联 SVG 吉祥物生成器 `theme.mascotSvg(status, size)`，包含多种状态（normal / ok / warn / idle），零外部依赖。
- [x] **空状态组件**：标准空状态渲染器 `theme.emptyState(iconType, title, message, actions)`，统一各视图的数据加载中/无设备态。
- [x] **主题切换器**：实现 `themeToggleBtn()`、`setThemeMode()`、`getThemeMode()`，支持 LocalStorage 持久化与 OS 配色无缝联动。
- [x] **样式作用域隔离**：所有规则严格限定于 `.qm-app` 及其命名空间内，不污染 LuCI 框架外层导航与其它插件。

### 2.2 视图实施进度

| 视图模块 | 对应源码文件 | 实施状态 | 改造重点与成果 |
| :--- | :--- | :---: | :--- |
| **仪表盘总览** | `view/qmodem/status.js` | **已完成** | 12列响应式卡片、二次元吉祥物欢迎卡片、双模快速切换、动态 Pastel SVG 信号仪表与流量图、统一主题空状态 |
| **连接设置** | `view/qmodem/connection.js` | **已完成** | 替换全部硬编码颜色、SVG 数据隧道拓扑图色系同步 Token、主题切换按钮、吉祥物空状态 |
| **网络与射频** | `view/qmodem/network.js` | **已完成** | RF 仪表盘/扇区图谱全面采用 Token 变量、支持双模切换、统一空状态 |
| **硬件与设备** | `view/qmodem/device.js` | **已完成** | 设备信息卡片流、SVG 拓扑图色彩变量化、主题切换支持、空状态 |
| **模组系统** | `view/qmodem/system.js` | **已完成** | SoC与双SIM卡槽 SVG 物理拓扑图全面适配 Token（芯片高亮/连接态）、统一卡片头图标、主题切换支持 |
| **高级设置** | `view/qmodem/advanced.js` | **已完成** | 硬件总线拓扑图（PCIe/USB/I2C/GPIO）全面适配 Token、频段锁网网格与 AT 终端样式升级、主题切换支持 |
| **短信管理** | `view/qmodem/sms.js` | **已完成** | 移除字符计数与警告样式中所有裸露十六进制颜色，统一采用 `--qm-danger`、`--qm-warn`、`--qm-text-muted`；接入统一页头（`theme.pageHead` + 主题切换按钮）与 `qm-shell` 容器、无设备态吉祥物空状态 |
| **短信对话** | `view/qmodem/sms_conversation.js` | **已完成** | 气泡对话流样式统一为 Pastel Glass（theme.js 新增 `.qm-app .sms-*` 皮肤：接收气泡 `--qm-surface` 卡片、发送气泡 `--qm-accent-gradient` 渐变、会话区毛玻璃 backdrop-filter）、字符计数器与元数据标记全部 Token 化（`--qm-danger`/`--qm-warn`/`--qm-text-muted`）；统一页头（含联系人副题）与 `qm-shell` 容器 |
| **SIM 切换** | `view/qmodem/sim_switch.js` | **已完成** | 状态文本与操作按钮色值 Token 化（降级提示 `--qm-text-muted`、ExtraInfo 注意事项 `--qm-warn`）；统一页头 + `qm-shell` 容器 + 空状态 |
| **链路监控** | `view/qmodem/monitor.js` | **已完成** | 脚本列表与告警框色值 Token 化（安全警告升级为 `--qm-warn-soft`/`--qm-warn-border` Pastel 告警卡、列表分隔线 `--qm-border-soft`）；统一页头 + `qm-shell` 容器 + cbi 表单自动卡片化 |
| **TTL 伪装** | `view/qmodem/ttl.js` | **已完成** | 告警说明块升级为 `--qm-warn-soft` Pastel 警示卡，标题 `--qm-warn`、正文 `--qm-text-soft`；统一页头 + `qm-shell` 容器 |
| **模组与网络配置** | `modem_config.js` / `network_config.js` | **已完成** | 状态圆点 Token 化（连接 `--qm-ok`、断开 `--qm-danger`、未知 `--qm-warn`、加载 `--qm-text-muted`）、RAT 制式徽章升级为 soft 底色胶囊（NR→`--qm-ok`、其他→`--qm-info`）、锁频列表强调色 `--qm-accent`；统一页头 + `qm-shell` 容器 + 空状态 |
| **SIM 卡短信** | `view/qmodem/sms_sim.js` | **已完成** | 接入统一页头 + `qm-shell` 容器 + 空状态，替代旧 `cbi-map` 标题与 alert 空态 |
| **短信转发** | `view/qmodem/sms_forward.js` | **已完成** | 接入统一页头 + `qm-shell` 容器，form.Map 交给 cbi 全局卡片皮肤 |
| **服务与扫描** | `view/qmodem/service.js` | **已完成** | 接入统一页头 + `qm-shell` 容器，form.Map 交给 cbi 全局卡片皮肤 |
| **流量统计** | `view/qmodem/stats.js` | **已完成** | 接入统一页头（含轮询周期副题）+ `qm-shell` 容器 + 空状态，模组卡片由 cbi-section 自动卡片化 |
| **AT 终端** | `view/qmodem/terminal.js` | **已完成** | hero 替换为统一页头（含主题切换），无模组态升级为吉祥物空状态，保留 `mt-terminal` 容器与 AT 终端卡片结构 |

**统一基础设施（2026-10-14 全量收敛）**：theme.js 新增 `pageHead(title, sub, opts)` 统一页头组件（渐变标题 + 副题 + 主题切换按钮，opts 支持徽章/按钮）与配套 `qm-page-head` 系列 CSS；新增 `fieldset.cbi-section` Pastel 毛玻璃卡片皮肤（含 legend/cbi-map-descr/cbi-page-actions 令牌化）——form.Map 类视图与自定义卡片视图视觉统一；≤560px 断点补齐页头与 cbi 卡片收敛规则。至此 18/18 视图全部接入 Soft Anime 体系：统一容器宽度（`--qm-maxw`）、统一页头、统一主题切换、统一空状态。

---

## 三、 质量验证与门禁检查

### 3.1 本地测试套件验证
通过 `scripts/check_all.sh` 自动化脚本验证：
* [x] **Shellcheck / sh -n**：114 个脚本文件语法检查全部通过。
* [x] **Node.js 语法检查**：所有 34 个 JavaScript 资源与 LuCI 视图 `node --check` 语法检查全部通过。
* [x] **JS 单元测试**：`test_sms_service.js` 测试通过。
* [x] **Rust 测试与 Clippy**：`cargo test + clippy` 全部通过，零 warning 阻止。
* [x] **C 语言语法检查**：gcc `-fsyntax-only` 语法检查全部通过。
* [x] **CRLF 换行检测**：全仓换行符规范，无 CRLF 异常。
* [x] **Lua 冒烟测试**：`tests/lua/smoke.sh` 验证通过。

---

## 四、 实施总结与交付成果

1. **次级视图细节色彩收敛**：
   * [x] 已完成 `sms_conversation.js`、`sim_switch.js`、`monitor.js`、`ttl.js`、`modem_config.js`、`network_config.js` 中所有色彩 Token 化。
2. **全量视觉统一（界面割裂收敛）**：
   * [x] 11 个核心独立页面（overview/network_config/debug/config_advanced/sms/settings/monitor/ttl/sim_switch/sms_forward/sms_sim）全面完成 Soft Anime 动态 SVG + Pastel Glass 仪表盘重构。
   * [x] `luci-app-qmodem-ttlfw4` 与 `luci-app-qmodem-voip` 独立插件同步接入 Soft Anime 设计系统。
3. **空间与交互极致优化**：
   * [x] AT 调试终端快捷诊断指令采用分段式标签栏（Segmented Tab Bar），垂直占位缩减 70%+，支持一键折叠/展开与键盘指令历史回溯。
   * [x] 全局 Soft Anime Pastel Glass 毛玻璃弹窗体系完成重构，覆盖系统内全部 `ui.showModal`。
4. **移动端深度微调**：
   * [x] 完成 360px、390px、430px 窄屏响应式弹性排版与触控优化，通过 Playwright 自动化测试回归验证。
5. **整体视觉与代码回归**：
   * [x] 执行 `check_all.sh`（Shellcheck、Node.js、Rust Clippy、Lua 冒烟测试）全部通过。
   * [x] 实机部署（ImmortalWrt 26.275 aarch64 + Fibocom FM350-GL 5G）验证通过。
