# journal-luci-final

由 agent-luci-final 维护。范围：luci/luci-app-qmodem-next 的 htdocs/** 与 tests/**（本轮未改 root/**）。
所有文件保持 LF 换行；未引入新依赖；ubus API 调用保持兼容。承接 journal-luci-next.md 与 journal-luci-comp.md。

---

## F1【M8-中】: debug.js AT 口下拉框取值脱离 LuCI 内部渲染实现

【问题】view/qmodem/debug.js 约 125-134 行 `new ui.Dropdown(...)` 创建后仅 `select.render()` 挂载，约 184 行（改前）用 `document.getElementById('at_port_' + modem.id).value` 直接取值。cbi-dropdown 的 value 由 LuCI ui.js 内部渲染结构（JS 实例属性 / li[data-selected]）承载，渲染 div 并非原生表单控件，`.value` 依赖实现细节；且全局 id 查询在多 modem 实例切换时有串扰风险。

【修复方式】本仓库内无可读的 luci-base htdocs/luci-static/resources/ui.js（只有各 app 包），无法直接核对 Dropdown API 方法名，按预案采用「实例引用优先 + 保存节点引用回退」：创建后同时持有 `portDropdown` 实例与 `portDropdownNode`（render() 返回节点），新增闭包内 `getSelectedPort()`——优先调用实例 `getValue()`（LuCI ui.js Dropdown 公开 API，19.07 起稳定），API 不存在时回退 `portDropdownNode.querySelector('li[data-selected], li.selected')` 读取 data-value/文本。sendBtn click handler 与 Dropdown 创建在同一 then() 闭包内，局部引用即可保证多实例隔离，无需全局表。debug.js 剩余 3 处 getElementById（at_command input 的 .value、use_ubus checkbox 的 .checked）均为原生控件属性，不属于内部渲染依赖，保留。

【验证结果】node --check 通过；grep 确认 debug.js 已无「getElementById 取 dropdown 值」的调用（仅余 input/checkbox 原生属性访问各 2 处）。

【文件:行号】htdocs/luci-static/resources/view/qmodem/debug.js:106-124（实例引用 + getSelectedPort）、:175（改用 getSelectedPort）

---

## F2【低】: sms.js 服务层 callList 500 条硬编码参数化

【问题】qmodem/sms.js listRaw 内 `callList(modem, 500, 0)` 硬编码 500 条上限，超长消息列表无法调整。

【修复方式】qmodem.js 服务层新增模块级 `smsListLimit`（默认 500）并暴露 `getListLimit()` / `setListLimit(limit)`（parseInt 校验、>=1 才生效、返回当前值，便于调用方确认）；sms.js listRaw 改为 `callList(modem, qmodem.getListLimit(), 0)` 并加注释，文件头补 `'require qmodem.qmodem as qmodem'`（qmodem.js 不依赖 sms.js，无循环 require）。视图层 UI 接线按任务要求预留，本轮只做参数化。测试 harness（tests/test_sms_service.js）以 `new Function('rpc','L','_','qmodem', source)` 注入 getListLimit stub，原有断言不受影响。

【验证结果】node --check 通过；tests/test_sms_service.js PASS。

【文件:行号】htdocs/luci-static/resources/qmodem/qmodem.js:271-289（getListLimit/setListLimit）、htdocs/luci-static/resources/qmodem/sms.js:1-4、:47-49；tests/test_sms_service.js:23-27

---

## F3【i18n 收尾】: 硬编码用户可见英文补 _()（10 处）

【问题】前轮已覆盖 Status:/Success/Failed/Command:/formatSignal 评级等，本轮全量 grep htdocs 复扫（赋值型、E() 文本、placeholder、通知文案、Error 串），剩余 10 处未包裹的用户可见字符串。

【修复方式】逐处补 _()：
- 'N/A' 显示值 4 处：qmodem.js formatSignal fallback、debug.js 当前 AT 口、sim_switch.js formatSlotDisplay（:95 的 `currentSlot === 'N/A'` 为逻辑比较，保持不译）、11_modem.js 两处 '? ' 占位（统一改为 _('N/A')，语义更明确）；
- 'Unknown' 1 处：overview.js getModemList 无名 modem 展示名；
- 服务层错误串 5 处（经视图 catch 以 e.message 展示给用户）：qmodem/sms.js 的 requireModem 提示、'SMS backend error'、'SMS synchronization failed'、'SMS configuration failed'。
不译项说明：placeholder 均为数据示例（AT+CIMI、IMEI 位数、webhook URL 等）；LTE/NR/GET/POST/15KHZ 为技术术语；po 模板按约定不重新抽取，待下次 l10n 流程统一 i18n-extract。

【验证结果】node --check 全部通过；test_sms_service.js 的 _ 为恒等注入，PASS；grep 复扫 E()/textContent/addNotification 路径无新增漏网可见串。

【文件:行号】qmodem/qmodem.js:495；view/qmodem/debug.js:78；view/qmodem/sim_switch.js:157；view/status/include/11_modem.js:97、:100；view/qmodem/overview.js:29；qmodem/sms.js:40、:48、:55、:91

---

## F4【性能】: overview.js 等 3 处 ubus 调用 allSettled 并行化 + 分项占位

【问题】1) overview.js updateModemInfo 每 tick 5 个 ubus 调用已用 Promise.all 并行，但任一失败即整体 reject → 整页只剩错误提示；2) 11_modem.js load 每 modem Promise.all([base_info, cell_info])，一个 modem 失败导致整页 data 缺失；3) sim_switch.js Promise.all 2 个调用，单失败整面板无状态。轮询复核：overview 轮询间隔 10s、每 tick 5 次 ubus 调用，量级合理，维持不变。

【修复方式】三处改 Promise.allSettled + 分项占位：
- overview.js：fulfilled 项照常合并 modem_info；部分失败 push 占位 entry（type plain_text，General 分组，「部分数据源加载失败/下次刷新重试」，与 InfoTable.newTr 渲染格式兼容）；全部失败取首个 rejected reason 走原整页 alert-message warning；copyright 失败仅隐藏版权行（原有 else 分支天然覆盖）；.catch 保留兜底同步异常。
- 11_modem.js：单 modem 内两源 allSettled，失败源跳过，info 为空时 render 已有 'No modem information available' 占位。
- sim_switch.js：capability/slot 失败置 null，走既有 Not Supported / N/A 降级分支（hideButtons 逻辑不变），并追加一条灰色占位提示；catch 兜底保留。

【验证结果】node --check 三文件通过；人工推演：5 源全失败→整页错误、部分失败→占位行 + 正常分区渲染、copyright 失败→隐藏、11_modem 单 modem 双失败→空表占位、sim_switch 双失败→Not Supported + 占位提示。

【文件:行号】view/qmodem/overview.js:158-198、:215-217（copyright 解包）；view/status/include/11_modem.js:18-43；view/qmodem/sim_switch.js:88-96、:114-118

---

## F5【视觉一致】: qmodem-next.css 作用域冲突复核

【问题】Phase 1 迁入的样式含多条全局 LuCI 通用类规则，多页面共享加载时存在越界命中：1) `.cbi-section-table .label` / `.cbi-section-table-row:hover` 会命中 settings.js（form.Map 渲染 cbi-section-table-row）等页面的表格行与 .label；2) `.cbi-value-field label { user-select: none }` 波及 settings.js 等页的表单 label；3) 顺带发现 `#neighbor_list_modem h4` 选择器与动态 id（`neighbor_list_ + modem.id`，如 neighbor_list_modem_1）永不匹配的存量 bug。

【修复方式】
- 邻区表 3 条规则（h4、.label、row:hover）加 `[id^="neighbor_list_"]` 作用域前缀，同时修复动态 id 匹配 bug；
- user-select:none 收紧为 `#tab_content_container .cbi-value-field label`（仅 config_advanced tab 内容），cursor:pointer 保留全局（纯增益）；
- 复核结论记录：`.cbi-tabmenu` 系列保留全局——全包仅 config_advanced.js:116 一处自建 tabmenu（network_config/settings 无 tab），且本文件后加载、同特异性覆盖主题为设计意图；`.cbi-section.collapsible/.draggable`、`#lockband_content_modem h3`、`.copyright-section`、`.qmodem-update-time`、`.sms-*`、`.compat-progressbar` 前缀或条件类均已自持，无跨页冲突。

【验证结果】CSS 语法结构人工复核通过；grep 确认无残留裸 `.cbi-section-table .label` / `#neighbor_list_modem` 选择器；改动仅影响邻区表与 config_advanced label 行为，settings.js 表单不再被波及。

【文件:行号】htdocs/luci-static/resources/qmodem/qmodem-next.css:68-82（checkbox/label 作用域）、:94-115（邻区表作用域 + id bug 修复）

---

## 验证汇总

- `node --check`：htdocs 下全部 17 个 .js 逐一通过（find 全量，非仅改动文件）。
- `node luci/luci-app-qmodem-next/tests/test_sms_service.js`：PASS（SMS service propagates sync errors and renders refreshed messages）。
- grep 验证：debug.js 无「getElementById 直接取 dropdown 值」——仅剩 at_command input（.value/.value 赋值）与 use_ubus checkbox（.checked）原生属性共 3 处。
- 换行符：`file` 确认全部 8 个改动文件 LF、无 CRLF。
- 未触碰：root/**、旧包 luci-app-qmodem、application/**、po 模板（按约定不重新抽取，待统一 i18n-extract）。
