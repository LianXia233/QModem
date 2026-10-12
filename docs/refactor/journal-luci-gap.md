# Journal: LuCI next gap 补齐 — Statistics 页与 band/stats 前端接入

- **agent**: agent-luci-gap
- **date**: 2026-10-12
- **scope**: `luci/luci-app-qmodem-next/**` 仅限（htdocs/**、rpcd acl.d、menu.d、po/**）

## 背景

Phase 1 后端（rpcd qmodem）已提供 `get_stats` / `clear_stats` / `get_current_band`
三个端点，但 luci-app-qmodem-next 前端未接入：ACL 白名单缺项、qmodem.js 无对应
rpc.declare、无流量统计页面、debug 页不展示当前频段。本次补齐该 gap。

## 改动清单

| 文件 | 内容 |
| --- | --- |
| `root/usr/share/rpcd/acl.d/luci-app-qmodem-next.json` | read 段补 `get_stats`、`get_current_band`；write 段补 `clear_stats`（与既有白名单风格一致） |
| `htdocs/luci-static/resources/qmodem/qmodem.js` | 新增 `callGetStats`/`callGetCurrentBand`/`callClearStats` 三个 rpc.declare + 门面 `getStats`/`getCurrentBand`/`clearStats` |
| `htdocs/luci-static/resources/view/qmodem/stats.js` | 新视图：每 modem 一张卡片，RX/TX 速率 CSS 条形图（复用 `cbi-progressbar`/`compat-progressbar` 探测，与 overview 进度条风格一致）+ 累计量；5s 轮询；`qmUI.pollGuard` 限频报错；清零按钮（原生 confirm 后调 `clear_stats`，清零后重置速率基线避免尖峰） |
| `root/usr/share/luci/menu.d/luci-app-qmodem-next.json` | 新增 `admin/modem/qmodem/stats`（order 1.5，紧跟 Overview；本包 menu.d 为多入口结构，非单入口 tab） |
| `htdocs/luci-static/resources/view/qmodem/debug.js` | AT Debug 页新增 Current Band 展示行（尽力而为）：`status=unsupported`/调用失败 → "厂商未支持"；未注册 → "未注册"；正常 → cells 的 band_name+channel 列表 |
| `po/zh_Hans/` + `po/template/` | 追加 14 条 msgid（含"厂商未支持"等），已 grep 查重不与既有条目冲突 |

## 设计取舍

- **不引入 ECharts**：速率条形图直接用进度条 DOM 结构（`<div class=bar><div style=width:N%>`），
  通过 `qmUI.hasCssClass('.cbi-progressbar')` 探测主题类名，fallback 到包内
  `compat-progressbar`，零依赖且主题自适应。
- **速率计算**：前端以相邻两次采样差值 / 时间差计算；计数器回退（清零/重置）时速率归 0。
- **峰值自适应**：条形图满刻度取 `max(当前速率, 上次峰值×0.85, 1KB/s)`，带衰减避免
  速率下降后条形长期贴底；下限 1KB/s 防止除零。
- **厂商不支持**：`get_stats` 对非 Quectel 返回 `available:false`，卡片显示警告条
  "厂商不支持流量统计" 而非报错；`clear_stats` 返回 `result:false` 同样提示。
- **confirm 用原生 `confirm()`**：与 config_advanced.js / settings.js 既有风格一致。

## 验证

- `node --check`：qmodem.js / stats.js / debug.js 全部通过。
- `node tests/test_sms_service.js`：PASS（回归无影响）。
- ACL / menu.d JSON `JSON.parse` 合法。
- `grep qmodem/stats` menu.d 命中；ACL grep 确认 `get_stats`、`get_current_band`、`clear_stats` 入列。
- po msgid 去重校验通过（zh 671 / template 670 条，无重复）。
- 全部改动文件 LF 换行（`file` 无 CRLF 报告）。

## 遗留 / 后续

- `get_traffic_reset_schedule` / `set_traffic_reset_schedule`（自动清零排程）尚未做
  前端入口，可在 Statistics 页追加排程表单（需 ACL read/write 白名单同步补齐）。
- stats.js 的模板 po 行号为近似值，下次 `luci-src-poe2l`/`i18n-scan` 全量重扫时会自动校正。
