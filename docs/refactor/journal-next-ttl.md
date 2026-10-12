# Journal: TTL 管理视图合入 next (agent-next-ttl)

日期: 2026-10-12
范围: 仅 `luci/luci-app-qmodem-next/**`（htdocs、menu.d、acl.d、po）与本 journal；
`luci-app-qmodem-ttlfw4` 仅只读参考，未做任何修改。

## 1. 参考实现与 uci 语义（只读分析）

- 参考 `luci/luci-app-qmodem-ttlfw4/htdocs/luci-static/resources/view/qmodem/ttl.js`：
  `form.Map('qmodem_ttl')` + `NamedSection 'main'/'main'` × 2 + `Flag enable` +
  `Value ttl (datatype range(1,255), default 64)` + `DummyValue _warning` 警告清单。
- 后端语义（ttlfw4 `root/etc/init.d/qmodem_ttl` + `root/etc/config/qmodem_ttl`）：
  `config main 'main'` 仅 `enable`（'0'/'1'）与 `ttl`（1-255）；启用时生成
  `/etc/nftables.d/99-reset-ttl-from-br-lan.nft`（fw4/nftables.d include，
  `iifname "br-lan" ip ttl set / ip6 hoplimit set`），并 `flow_offloading(_hw)=0`、
  停止 qca-nss-ecm、rmmod shortcut-fe-cm / fast-classifier；停用时清理恢复。
  init.d 已 `procd_add_reload_trigger "qmodem_ttl"`，uci commit 后自动 reload。

## 2. uci 写入口结论（无 gap，无需新 rpcd 端点）

next 现有视图（如 settings.js）均通过标准 `form.Map` → LuCI `uci` rpc（rpcd
`uci add/set/apply`）写 uci，无自定义写端点。TTL 视图沿用同一机制，仅需在
`acl.d/luci-app-qmodem-next.json` 的 `read.uci`/`write.uci`（及 `read.ubus.uci`/
`write.ubus.uci`，与既有 `sms_forwarder` 声明方式对齐）追加 `"qmodem_ttl"`。
**未虚构任何 rpcd 端点**；ttlfw4 ACL 里的 `luci setInitAction` / `rc init`
未迁移（其视图从未调用，见 §3 diff）。

## 3. 与 ttlfw4 视图逻辑 diff 对照

| 项 | ttlfw4 ttl.js | next ttl.js | 说明 |
|---|---|---|---|
| rpc 声明 | `callInitAction`(luci.setInitAction)、`callRcStatus`(rc.list) | **删除** | 两者在 ttlfw4 视图中声明后从未调用（死代码）；且 reload 由 procd trigger 自动完成 |
| 数据模型 | Map/NamedSection/Flag/Value/DummyValue 完全一致 | 一致 | `enable` 默认 '0'、rmempty=false；`ttl` range(1,255)、默认/占位 64 |
| 警告文案 | 4 条（无后端说明） | 5 条，**新增第 1 条** fw4/nftables 后端说明（`/etc/nftables.d/99-reset-ttl-from-br-lan.nft`） | 呼应任务"说明文案（fw4/nft 后端）"；NSS/SFE 停用提示保留 |
| load() | `Promise.all([uci.load]).then(r=>r)` | 直接 `return uci.load('qmodem_ttl')` | 等价简化 |
| 依赖 | view/form/uci/rpc/ui | view/form/uci | rpc/ui 不再使用，去掉 require |
| ui.js 复用 | 无 | 无（有意决策） | `qmodem/ui.js` 导出 InfoTable/progressbar/ModemSelector/formatTime/relativeTime/pollGuard，均面向信息展示/轮询场景；纯表单页按既有 `settings.js` 风格使用标准 luci form 组件（form.Map/Flag/Value/DummyValue），不引入假依赖 |

## 4. 本次改动清单

1. `htdocs/luci-static/resources/view/qmodem/ttl.js`（新增）：enable 开关 + TTL 值
   输入（1-255 校验）+ fw4/nft 后端与 NSS/SFE 停用警告；ES5 + LuCI require 指令序言，风格对齐 settings.js。
2. `root/usr/share/luci/menu.d/luci-app-qmodem-next.json`：新增
   `admin/modem/qmodem/ttl`（order 5.7，位于 SIM Switch 5.6 与 Settings 6 之间，
   呼应 ttlfw4 原 order 22 挂载点语义），depends acl luci-app-qmodem-next。
3. `root/usr/share/rpcd/acl.d/luci-app-qmodem-next.json`：read/write 两级 uci 数组
   追加 `qmodem_ttl`。
4. `po/template/qmodem-next.po`、`po/zh_Hans/luci-app-qmodem-next.po`：追加 14 条
   TTL 文案（模板 + 简中翻译）；无 msgid 重复（grep 复核）。

## 5. 已知 gap / 后续

- **后端归属**：`/etc/config/qmodem_ttl` 与 `init.d/qmodem_ttl` 仍由 ttlfw4 包提供；
  next 单装时前端可写 uci 但无后端生效（Phase 4 退役合并时需迁移 init.d + 默认
  config + uci-defaults/firewall include，见 journal-retire-ttl.md 的合并建议）。
  本次未在 next 内复制后端文件（越界且与 ttlfw4 冲突：共用 uci 节双向 PKG_CONFLICTS）。
- ttlfw4 init.d 中 `/etc/firewall.d/qmodem_ttl` 疑似残留调用（journal-retire-ttl.md
  已记录），迁移 Phase 4 时一并清理。

## 6. 自验证记录

- `node --check view/qmodem/ttl.js` → OK。
- `node tests/test_sms_service.js` → `PASS: SMS service propagates sync errors and renders refreshed messages`。
- ACL / menu.d JSON `JSON.parse` → OK。
- menu 链接 grep：`admin/modem/qmodem/ttl` 与 `"path": "qmodem/ttl"` 均命中（menu.d:117,122）。
- LF 复核：新增/修改文件无 CRLF。
