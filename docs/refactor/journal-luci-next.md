# journal-luci-next

由 agent-luci-next 维护。范围：luci/luci-app-qmodem-next 的 htdocs/** 与 root/**。
所有文件保持 LF 换行；未引入新依赖；ubus API 调用保持兼容。

---

## H1: ACL read 段 `qmodem: ["*"]` 全开收敛

【问题】root/usr/share/rpcd/acl.d/luci-app-qmodem-next.json read 段对 qmodem 对象使用 `"*"`，任何登录用户可调用全部 qmodem 方法（含 send_at、modem_dial 等写方法，因为 rpcd ACL 的 read/write 是独立授权，`*` 使 read 权限覆盖所有方法）。

【修复方式】grep htdocs 内全部 rpc.declare（qmodem.js 服务层 + settings.js 视图层），得到实际使用的只读方法白名单（base_info/cell_info/info/network_info/sim_info/get_at_cfg/get_imei/get_mode/get_lockband/get_neighborcell/get_network_prefer/get_dns/get_sms/get_disabled_features/get_reboot_caps/get_copyright/dial_status/get_connect_status/get_dial_log/get_sim_switch_capabilities/get_sim_slot/get_pcie_devices/get_usb_devices/get_leds/get_network_interfaces/get_tty_ports/get_available_devices），替换 `"*"`。send_at/send_sms/set_*/scan_*/modem_dial 等全部留在 write 段。另将前端声明但原先仅靠 read `*` 放行的 clear_dial_log、set_sim_slot、remove_modem 补入 write 段（write 段其余保持不动），保证既有调用不因收敛而 403。

【验证结果】python3 json.load 解析通过；grep 确认 read 段无 `"*"`；node --check 与测试不受影响。

【文件:行号】root/usr/share/rpcd/acl.d/luci-app-qmodem-next.json:5-13（read 段）、:27-48（write 段）

---

## H2: sms.js 服务层 15 处 `|| 'modem_1'` 静默 fallback

【问题】qmodem/sms.js 所有方法在 configSection 缺失时静默回退到硬编码 'modem_1'，多 modem 场景下会静默读写错误设备。

【修复方式】与该文件现有错误处理风格（Promise.reject(new Error(...))）一致：新增 requireModem(configSection) 辅助，缺失时返回 rejected Promise（Error: 'configSection (modem_id) is required'），15 处调用点全部改为 requireModem(configSection)。调用方（view 层）均从 modem 下拉框 select.value 取值且在 modems 为空时提前返回，不存在未传参调用点，无需补逻辑。

【验证结果】grep htdocs/ 确认 'modem_1' 零残留；node --check 通过；node tests/test_sms_service.js PASS（测试以 listSms('modem_1') 显式传参，行为不变）。

【文件:行号】htdocs/luci-static/resources/qmodem/sms.js:37-40（requireModem）、:83、:86、:95、:110、:113、:115、:117、:121、:124、:127、:130、:149、:152、:155

---

## H3: sms-pdu.js 死代码 LuciTable ES6 class 删除

【问题】sms-pdu.js 尾部约 141 行的 `class LuciTable{...}` 无任何引用（全仓库 grep：仅 application 老版 .htm 与 luci-app-qmodem-sms 各有独立副本，本文件无引用方），且包含 innerHTML 操作，属于遗留死代码。

【修复方式】先全仓库 grep 确认 htdocs 外无引用后整体删除，其余 PDU 功能（parse/generate/encode/decode 等）不动。

【验证结果】grep LuciTable 与 innerHTML 在 sms-pdu.js 内零命中；node --check 通过。

【文件:行号】htdocs/luci-static/resources/qmodem/sms-pdu.js（原 :391-531 已删除，现文件 389 行）

---

## H4: SCTS 时区硬编码 -28800 hack 修复

【问题】sms-pdu.js 对 TP-SCTS 第 7 字节（时区）解析错误，视图层用 `timestamp - 28800`（固定 UTC+8）+ localStorage 开关补偿，换算结果因时区/季节错误。

【修复方式】按 3GPP TS 23.038 重写解析：保留原始 7 字节 hex（rawTimestamp），时区字节低半字节为时区值十位、高半字节 bit0-2 为个位、bit3 为符号（1=西半球负偏移），单位 15 分钟；`UTC = 本地时间 - offset`，parse() 返回正确 UTC epoch 的 Date。视图层 view/qmodem/sms.js 与 sms_conversation.js 删除 getUtcOffsetSetting/setUtcOffsetSetting/adjustTimestampForUtcOffset 三组重复函数、两处时区 checkbox UI 及 change handler、loadSmsList/pollSmsList/loadConversation/pollConversation 中全部 -28800 补偿块；时间戳直接使用服务层返回值，无需共用换算函数（故未在 sms.js 服务层新增导出）。

【验证结果】grep 28800/utc_offset/adjustTimestamp/UtcOffsetSetting 全仓库 htdocs 零命中；node --check 通过；sms 服务测试 PASS。

【文件:行号】htdocs/luci-static/resources/qmodem/sms-pdu.js:46-66；htdocs/luci-static/resources/view/qmodem/sms.js:1-20、:56-58、:208-214、:241-270；htdocs/luci-static/resources/view/qmodem/sms_conversation.js:1-16、:56-77、:275-297

---

## M1: 4 个视图顶层 CSS link 无去重

【问题】overview.js、sms.js、sms_conversation.js、sms_sim.js 顶层直接 document.head.appendChild 注入 qmodem-next.css，视图反复切换会重复注入 <link>。

【修复方式】qmodem.js 服务层导出 loadCss(href)，内部按 href 判重（loadedCss map），4 个视图改为调用 qmodem.loadCss(L.resource('qmodem/qmodem-next.css'))；sms.js 与 sms_conversation.js 视图补 'require qmodem.qmodem as qmodem'。

【验证结果】grep loadCss 确认 4 个视图均已改用服务层方法且无残留 document.head.appendChild 注入 CSS；node --check 通过。

【文件:行号】htdocs/luci-static/resources/qmodem/qmodem.js:258-272；view/qmodem/overview.js:9-10、sms.js:9-10、sms_conversation.js:9-10、sms_sim.js:9-10

---

## M4: settings.js 扫描后 reload 行为不一致

【问题】PCIe 扫描失败才 reload、USB/ALL 扫描成功才 reload，行为疑似写反，设备列表刷新不可预期。

【修复方式】保守统一方案：三个扫描按钮的 .then 分支无论 res.code 成败，先 ui.addNotification（成功 info/失败 warning），随后统一 window.location.reload()，保持三处一致；.catch（RPC 层异常）仅提示不 reload。

【验证结果】node --check 通过；人工核对三段 then 分支结构一致（成功/失败均提示后 reload）。注意：中途一次缩进整理脚本误伤全文件，已 git checkout 回滚后用精确 Edit 重做，最终 diff 仅 17 行增删。

【文件:行号】htdocs/luci-static/resources/view/qmodem/settings.js:184、:211、:236（三处 window.location.reload() 统一移至 then 分支末尾）

---

## M5: overview.js 进度条除零 NaN

【问题】progress_bar 类型当 max==min（或区间退化）时 `(value-min)/(max-min)*100` 产生 NaN%，进度条宽度失效。

【修复方式】max-min>1 时按原公式计算，否则直接取 `value > min ? 100 : 0`；保留原有 0-100 clamp，并追加 isNaN 兜底归 0。

【验证结果】node --check 通过；value=min/max/min+1、max==min、max<min 等输入手工推演均得到 0-100 内确定值。

【文件:行号】htdocs/luci-static/resources/view/qmodem/overview.js:98-106

---

## M6: overview.js localStorage key 使用译文文本

【问题】拖拽排序持久化时从 legend/panel-title 的 textContent（经 _() 翻译）提取 key 存入 localStorage，切换语言后旧 key 失配、面板顺序与折叠状态失效。

【修复方式】面板 fieldset 写入稳定标识 data-section-key（未翻译的原始 class 名，与折叠状态 getCollapsedState 用的 className 同源），drop handler 的 newOrder 改为读取 section.getAttribute('data-section-key')。

【验证结果】node --check 通过；grep 确认 newOrder 提取不再依赖 textContent。旧的多语言遗留 key 会自然失配回退为默认顺序，无副作用。

【文件:行号】htdocs/luci-static/resources/view/qmodem/overview.js:344-346（写入 data-section-key）、:340-344（drop handler 读取）

---

## i18n: 硬编码英文补 _()

【问题】qmodem.js formatSignal 硬编码 (Excellent)/(Good)/(Fair)/(Poor)；debug.js 'Status: '/'Success'/'Failed'/'Command: '/'Response:\n'/'No response' 未包 _()。

【修复方式】formatSignal 16 处评级文案拆出并包 _()（保留 ' dBm '/' dB ' 前缀不动）；debug.js 6 处包 _()。

【验证结果】node --check 通过；tests/test_sms_service.js 的 _ 以恒等函数注入，不受影响（PASS）。

【文件:行号】htdocs/luci-static/resources/qmodem/qmodem.js:495-514；htdocs/luci-static/resources/view/qmodem/debug.js:198-201

---

## 验证汇总

- `node --check`：9 个修改的 .js 全部通过。
- `node luci/luci-app-qmodem-next/tests/test_sms_service.js`：PASS（SMS service propagates sync errors and renders refreshed messages）。
- grep 验证：htdocs 内 'modem_1' fallback、28800/utc_offset 补偿、sms-pdu.js innerHTML、LuciTable 死代码全部零残留。
- ACL JSON：python3 json.load 解析通过，read 段无 `"*"`。
- 换行符：`file` 确认全部修改文件为 LF（ASCII/UTF-8 text，无 CRLF）。
