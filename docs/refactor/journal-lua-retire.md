# Journal: 旧 Lua 栈止血与退役准备 (agent-lua-retire)

日期: 2026-10-12
范围: 仅 `luci/luci-app-qmodem/**`、`luci/luci-app-qmodem-sms/**`，未触碰
`luci-app-qmodem-next`、`application/`、`rust/`（git status 证实仅 4 个允许文件变更）。

## 修复内容

### H5-高: modemCtrl 命令注入 (luasrc/controller/qmodem.lua)
- 新增 `shq()`（qmodem.lua:15）：Lua 版单引号 shell 转义。
- 新增 `modem_actions` 白名单（qmodem.lua:20-56），动作名集合从
  `application/qmodem/files/usr/share/qmodem/modem_ctrl.sh` 的 case 分支逐一提取，
  共 35 项，覆盖现有 view 层全部调用（info / clear_dial_log /
  get_disabled_features / get_lockband|set_lockband / get_network_prefer /
  get_mode / get_neighborcell / get_imei / get_reboot_caps|do_reboot /
  get_at_cfg|send_at）。
- `modemCtrl()`（qmodem.lua:116）：action 不在白名单、cfg_id 不匹配
  `^[A-Za-z0-9_]+$` 时直接返回 `{"error":...}`；params 一律经 `shq` 再拼接，
  移除原先的裸 `\"..params..\"`。

### H6-高: sendATCommand 注入
- `at()`（qmodem.lua:154）：at_port / at_command 均经 `shq` 后拼接。
- 新增 `valid_at_port()`（qmodem.lua:164）：仅允许 `^/dev/[A-Za-z0-9_/]+$`
  且 `nixio.fs.access()` 确认设备存在。
- 新增 `valid_at_command()`（qmodem.lua:174）：禁止 `\r\n` 与反引号。
- `sendATCommand()`（qmodem.lua:292）：校验失败返回
  `{"error":"invalid at port or command"}`，不再执行。

### H7-高: modem_sms.htm XSS
- `luasrc/view/modem_sms/modem_sms.htm` 全部 16 处 `innerHTML` 改为
  `textContent`（元素均为既有 DOM 节点，纯文本赋值语义不变）。
  覆盖：短信 sender/timestamp/content/part 展示（984-997 区段）、
  warn_msg/log_msg（621/631）、存储选项文本（559、904-905）、
  modem 下拉选项名（932）、表头/清空逻辑（429/430、444/445、479/480）。
  grep 验证 `innerHTML` 残留 = 0。

### H8-中: 全局变量泄漏与 nil 崩溃
- qmodem.lua: `uci` 改 local；`translate_modem_info` 内 modem_info/
  response/key/full_name、`modemCtrl` 内 result/modem_more_info、
  `getOverviews` 内 section_name/modem_name/alias/modem_state/cmd/result/
  json_result/modem_info/tmp_info/title/name/full_name/log_path/log_msg/
  modem_log、`getModemCFG` 内 modem_state 全部补 local。
- 原约 185 行 `modem_device["name"]:upper()` nil 崩溃：改为
  `modem_device["name"] and modem_device["name"]:upper() or
  luci.i18n.translate("Unknown")`（qmodem.lua:267）。

### 退役准备-中: 解除互斥
- `luci/luci-app-qmodem/Makefile`: 删除
  `@(PACKAGE_luci-app-qmodem-next=n)` 互斥依赖，保留 `+luci-compat +qmodem`，
  新旧 UI 可共存由用户选择，为单包交付铺路。
- `luci/luci-app-qmodem-sms/Makefile`: **保留** `+luci-app-qmodem` 依赖，
  原因（已 grep view 层核实）：modem_sms.htm 除自身控制器（走 ubus
  `qmodem.sms`，由 +qmodem-smsd 提供）外，还调用
  `admin/modem/qmodem/get_modem_cfg` 端点，该端点仅由 luci-app-qmodem 的
  Lua 控制器注册，luci-app-qmodem-next 未提供同名路由；若移除依赖，
  SMS 页 modem 选择器将 404。已在 LUCI_TITLE 注明
  "legacy Lua UI, deprecated - prefer luci-app-qmodem-next"。
- PKG_LICENSE_FILES 复核：6 个 Makefile 声明未回退（指向仓库根 LICENSE）。

## 验证记录
- `luac5.3 -p`（apt 安装 lua5.3）：
  - qmodem.lua: syntax OK
  - qmodem_sms.lua: syntax OK（未改动，回归确认）
- `grep -c innerHTML modem_sms.htm` → 0
- `grep 'qmodem-next=n' luci/luci-app-qmodem*/Makefile` → 无匹配（互斥已删）
- git status：变更仅限 4 个允许文件。
