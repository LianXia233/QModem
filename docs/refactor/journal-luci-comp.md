# journal-luci-comp

由 agent-luci-comp 维护。范围：luci/luci-app-qmodem-next 的 htdocs/** 与 root/**（本轮未改 root/**）。
所有文件保持 LF 换行；未引入新依赖；ubus API 调用保持兼容。承接 journal-luci-next.md。

---

## C1: 新建共享组件模块 qmodem/ui.js（ModemSelector 收敛 7 处模板）

【问题】"Modem Name" 选择器模板（fieldset>table>tr>td+select）在 7 个视图中逐字重复：overview.js、debug.js、config_advanced.js、sim_switch.js、sms.js、sms_conversation.js、sms_sim.js，各处仅在 select id / label 是否 left / 是否过滤 enabled 上有差异。

【修复方式】新建 htdocs/luci-static/resources/qmodem/ui.js，导出 ModemSelector(modems, options)，options 支持 id（默认 'modem_selector'）、leftAlignLabel、filter，返回 { section, select, body }（body 暴露以便 sms.js 向同一 tbody 追加 SMS Mode/Poll Interval/Auto Delete 行，DOM 结构不变）。7 个视图全部改为调用组件：
- overview.js：默认参数（不 filter、label 不 left），与原行为一致；
- debug.js / config_advanced.js / sim_switch.js：leftAlignLabel + filter enabled；
- sms.js / sms_conversation.js / sms_sim.js：id 分别为 sms_modem_selector / sms_modem_selector / sms_sim_modem_selector。
grep 确认 _('Modem Name') 模板全仓仅存 ui.js 一份。

【验证结果】node --check 全部通过；grep 确认视图层零残留（仅 ui.js 一份模板）。

【文件:行号】qmodem/ui.js:225-267（ModemSelector）；view/qmodem/overview.js:373-377、debug.js:23-29、config_advanced.js:24-30、sim_switch.js:27-33、sms.js:32-38、sms_conversation.js:48-52、sms_sim.js:31-35

---

## C2: 时间格式化收敛（formatTime / relativeTime）

【问题】三份时间格式化实现重复：qmodem/sms.js 服务层 formatTimestamp（相对时间：今天 HH:mm / Yesterday / 星期 / 日期）、sms_conversation.js 视图 formatFullTimestamp（完整 'YYYY-MM-DD HH:mm:ss'）、overview.js 内联的"Last update"完整时间拼接（同一逻辑第三份）。

【修复方式】ui.js 导出 formatTime(value)（Date 或 epoch 秒 → 'YYYY-MM-DD HH:mm:ss'，非法时间戳返回 '-'）与 relativeTime(timestamp)（原服务层相对时间逻辑原样迁入）。sms.js 服务层 formatTimestamp 改为一行委托 qmUI.relativeTime（'require qmodem.ui as qmUI' 已加入该模块，测试 harness 不调用此方法不受影响）；sms_conversation.js 删除 formatFullTimestamp，渲染处改 qmUI.formatTime(msg.timestamp)；overview.js "Last update" 改 qmUI.formatTime(new Date())。

【验证结果】node --check 通过；tests/test_sms_service.js PASS；grep 确认 padStart 时间拼接实现仅存 ui.js 一份。

【文件:行号】qmodem/ui.js:269-296；qmodem/sms.js:130-133；view/qmodem/sms_conversation.js:338、:436（原 formatFullTimestamp 已删）；view/qmodem/overview.js:414-417

---

## C3: pollGuard 统一轮询失败处理（限频通知）

【问题】三处轮询失败仅 console.error，用户无感知且不便排障：sms.js 视图 pollSmsList、sms_conversation.js pollConversation、sms_sim.js pollSimSmsList。若直接弹通知，10s/5s 轮询间隔会刷屏。

【修复方式】ui.js 导出 pollGuard(context, intervalMs=30000)：返回 catch 处理器，console.error 记录原始错误，ui.addNotification('Background refresh failed: %s', 'error') 按 context 限频（默认 30s 一次，模块级 pollGuardLastShown 时间戳表）。三处 catch 改为 .catch(qmUI.pollGuard('sms_list'/'sms_conversation'/'sms_sim'))。

【验证结果】node --check 通过；grep 确认 console.error('Poll error') 零残留。

【文件:行号】qmodem/ui.js:298-319；view/qmodem/sms.js:247-254、sms_conversation.js:293-301、sms_sim.js:117-125

---

## C4: 11_modem 状态页共享渲染逻辑收敛（注明旧包退役）

【问题】view/status/include/11_modem.js（next 包）自带 progressbar 实现，与 view/qmodem/overview.js 的 InfoTable/progressbar 渲染逻辑重叠；且旧包 luci-app-qmodem 另有一份同名 11_modem.js（本次未触碰）。

【修复方式】next 版的 InfoTable（原 overview.js 内 LuciTable 类）与 hasCssClass/progressbar_className 探测上移至 ui.js；overview.js 改用 new qmUI.InfoTable()。11_modem.js 删除本地 progressbar，改 qmUI.progressbar（顺带获得 overview 同款的除零/退化区间防护，原实现 100/(max-min) 无防护），并在文件头注明：旧包同名文件将在退役阶段统一处理。原 progressbar 的 class 从硬编码 'cbi-progressbar' 改为 compat 探测（主题缺类时回退 compat-progressbar），为此 11_modem.js 顶部补 qmodem.loadCss 确保 compat 样式可用。

【验证结果】node --check 通过；grep 确认 11_modem.js 内无本地 progressbar 定义。

【文件:行号】qmodem/ui.js:13-223（InfoTable/progressbar/hasCssClass）；view/status/include/11_modem.js:1-15、:83；view/qmodem/overview.js:12-14、:236

---

## C5: ubus 返回缺字段防御（settings.js 典型 2 处 + sms 视图兜底）

【问题】1) settings.js 三个扫描按钮 onclick 直接访问 res.code/res.message/res.output，ubus 返回空 payload（null/{}）时 TypeError 崩溃；2) settings.js handleAdd 直接取 availableDevices[selectedKey].type/path，键不存在时 uci.add 写入 undefined；3) sms.js / sms_conversation.js render 对 getModems 结果无条件 .filter，后端异常返回 undefined 时崩溃。

【修复方式】1) 三处改 `res && res.code === 0`，消息兜底 `(res && res.message) || _('...scan failed')`；2) deviceInfo 缺失时 hideModal + addNotification('No data available for the selected device. Please rescan and try again.', 'error') 占位返回；3) 两处 sms 视图加 `modems = modems || []`，空列表沿用既有 'No modems configured' 警告占位 UI。

【验证结果】node --check 通过；手工推演 res=null/res={} /deviceInfo=undefined/modems=undefined 均走提示分支不抛异常。

【文件:行号】view/qmodem/settings.js:177-188、:201-212、:224-235、:283-291；view/qmodem/sms.js:21-23、sms_conversation.js:39-41

---

## C6: CSS 判重复核 + 内联样式迁移

【问题】上一轮 loadCss 仅覆盖 overview/sms/sms_conversation/sms_sim；config_advanced.js 使用 qmodem-next.css 的 .cbi-tabmenu/.band-container/#lockband_content_modem h3 等规则却从未加载该文件，独立安装/直接访问该页时样式缺失。另有多处大块内联 style 散落视图。

【修复方式】config_advanced.js 补 qmodem.loadCss；11_modem.js 补 loadCss（配合 C4 的 compat-progressbar 回退）。内联 style 迁移到 qmodem-next.css（视觉不变）：overview 更新时间 → .qmodem-update-time；sms 会话行 cursor/bold → .sms-conversation-row/.sms-unread-conversation，未读徽标 → .sms-unread-badge；字符计数行 → .sms-counter-row（sms 弹窗 + 会话回复表单共用）；会话消息区 margin-top 并入既有 .sms-messages-area 规则，会话头按钮/h2 → .sms-conversation-header button/h2，回复表单 → .sms-reply-form；SIM 列表内容格 → .sms-sim-content-cell。保留少量状态切换型内联样式（copyright display 切换、徽标色值等由 CSS 承接后不再保留）。

【验证结果】grep 确认 6 个文件调用 loadCss；迁移点处 inline style 零残留（状态切换型除外）；视觉属性逐一比对与原内联值一致。

【文件:行号】view/qmodem/config_advanced.js:5-7、view/status/include/11_modem.js:5-7；qmodem-next.css:132-186（新增规则）

---

## 验证汇总

- `node --check`：11 个改动 .js（ui.js、qmodem/sms.js、10 个视图中的 9 个改动文件）全部通过。
- `node luci/luci-app-qmodem-next/tests/test_sms_service.js`：PASS。
- grep：_('Modem Name') 选择器模板仅存 qmodem/ui.js 一份；padStart 时间格式化实现仅存 ui.js 一份；console.error('Poll error') 零残留。
- loadCss 覆盖：6 处（overview/sms/sms_conversation/sms_sim/config_advanced/11_modem）。
- 换行符：全部改动文件 LF、无 CRLF。
- 未触碰：旧包 luci-app-qmodem、application/**、root/**。
