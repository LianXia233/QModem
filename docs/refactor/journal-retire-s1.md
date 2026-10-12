# 旧栈退役 Step 1：声明式标记 + 依赖关系梳理

- **agent**: agent-retire-s1
- **日期**: 2026-10-12
- **范围**: `luci/luci-app-qmodem{,-sms,-hc,-mwan,-ttl,-ttlfw4,-voip}` 的 Makefile 与各包 `luasrc/view/*.htm` 首部退役注释；不碰 `luci-app-qmodem-next/**`，不碰 `.lua` 逻辑文件，不删功能。

## 1. Makefile 审查结果（7 包）

| 包 | PKG_LICENSE_FILES | legacy 标注 | DEPENDS 变更 | 备注 |
|---|---|---|---|---|
| luci-app-qmodem | ✓ 已有 | ✓ 标题追加 | 无硬依赖改动（+luci-compat +qmodem） | Phase 1 互斥删除已复核未回退（与 next 可共存，见 Makefile 注释）；**无 CONFLICTS 残留** |
| luci-app-qmodem-sms | ✓ 已有 | ✓ 标题原已含 legacy/deprecated | 保留 `+luci-app-qmodem`，加退役计划注释 | 见 §2 |
| luci-app-qmodem-hc | ✓ 已有 | ✓ 标题追加 | 保留 `+luci-app-qmodem`，加注释 | 设备限定 @TARGET_mediatek_...hc-g80 |
| luci-app-qmodem-mwan | ✓ 已有 | ✓ 标题追加 | 保留 `+luci-app-qmodem`，加注释 | 另依赖 +luci-app-mwan3 |
| luci-app-qmodem-ttl | ✓ 已有 | ✓ 标题追加 | 保留 `+luci-app-qmodem`，加注释 | PKG_CONFLICTS:=ttlfw4 为**功能互斥**（共用 `/etc/config/qmodem_ttl`），保留不删；与 next 无冲突声明 |
| luci-app-qmodem-ttlfw4 | **修复：补 `PKG_LICENSE_FILES:=LICENSE`（原先缺失）** | ✓ 标题追加 | +luci-base，无 qmodem 依赖 | PKG_CONFLICTS:=ttl 同上保留 |
| luci-app-qmodem-voip | ✓ 已有 | ✓ 标题追加 | +qmodem-voip +qmodem-sipd +luci-base，无 luci-app-qmodem 依赖 | 实验性语音 |

**硬依赖退役计划（注释已写入各 Makefile）**：`+luci-app-qmodem` 仅提供旧栈 UI 入口（菜单挂载 + `get_modem_cfg` HTTP 端点）。Step 2 在 `luci-app-qmodem-next` 提供等价端点后，各子包 DEPENDS 切换为 `+qmodem`（及 menu.d ACL），并移除对旧 UI 的依赖。

## 2. htm 退役注释

旧栈 htm 实际共 **6 个**（mwan/ttl 为 CBI lua 无 htm；ttlfw4/voip 为 JS 视图无 htm），均已在**文件首行**插入：

```
<!-- legacy: superseded by luci-app-qmodem-next; scheduled for removal in Phase 4 -->
```

- luci-app-qmodem: `dial_overview.htm`、`modem_config_add.htm`、`modem_debug.htm`、`modem_info.htm`
- luci-app-qmodem-sms: `modem_sms.htm`
- luci-app-qmodem-hc: `modem_sim.htm`

插入位置均在任何 `<%...%>` 模板标签之前（首行独立注释），LuCI 模板按原文输出，不影响解析与 DOM。

## 3. 退役清单 / 端点 Gap 表（Phase 4 输入）

依据：旧栈 `controller/*.lua` 后端调用（modem_ctrl.sh 白名单、`qmodem.sms` ubus）对照 next 的 `acl.d/luci-app-qmodem-next.json` 与 `qmodem.js`/`sms.js` 的 `rpc.declare`。

| 功能（旧栈） | next 对应物 | 迁移阻塞点 |
|---|---|---|
| 模组信息页 modem_info（base_info/cell_info/sim_info/info/network_info via modem_ctrl.sh） | `overview.js`（qmodem ubus 同名方法） | 无（ACL 已覆盖） |
| 拨号总览/配置 dial_overview+dial_config（dial_status/get_dial_log/clear_dial_log/modem_dial/hang/redial） | `overview.js` | 无（ACL 已覆盖） |
| AT 调试 modem_debug（send_at/send_raw_pdu/get_at_cfg/get_disabled_features） | `debug.js` | 无（ACL 已覆盖） |
| **get_modem_cfg HTTP 端点**（模组列表，`qmodem.lua` luad call，非 ubus） | next 用 `qmodem.sms modem_list` + `qmodem get_available_devices/get_tty_ports` 替代 | **Step 2 关键阻塞**：旧栈子包（尤其 sms）依赖此 luad 端点取模组列表；next 无同名端点，需确认 modem_list 覆盖 at_port/cfg 字段后再切换 `+qmodem` |
| SMS 收发/删除/存储（qmodem.sms list/send/delete/sync/storage_set） | `sms.js`/`sms_conversation.js`/`sms_sim.js` | 无（qmodem.sms 全覆盖） |
| SMS 转发 CBI（uci sms_forwarder） | `sms_forward.js` | 无 |
| HC-G80 SIM 切换（fw_printenv/setenv + GPIO 直写，硬件专用） | `sim_switch.js`（qmodem get_sim_switch_capabilities/set_sim_slot） | **gap**：next 无 HC 专用 ubus 端点，需验证 set_sim_slot 在 HC-G80 固件路径（uboot env/GPIO）等效 |
| SIM 槽能力/当前频段（get_current_band、get_current_band_capabilities） | 无 | **gap**：不在 next acl.d，需后端+ACL 补齐或确认弃用 |
| 流量统计（get_stats、clear_stats） | 无 | **gap**：next 无统计端点 |
| mwan 多拨管理（CBI + mwan3 联动） | 无对应页面 | **整体 gap**：next menu.d 无 mwan；依赖 luci-app-mwan3，Phase 4 前需决定去留 |
| TTL 管理 ttl/ttlfw4（uci qmodem_ttl） | 无对应页面 | **整体 gap**：next 无 TTL 视图与 ubus 端点 |
| 语音 voip（qmodem-voip/qmodem-sipd WebRTC） | 无对应页面 | **整体 gap**：实验性，建议长期独立保留 |

## 4. 验证记录

1. **luac -p 不适用**：本次未修改任何 `.lua` 文件（`git diff` 确认仅 Makefile 与 htm）。
2. **HTML 注释语法自查**：6 个 htm 首行均为完整单行 `<!-- ... -->`（无嵌套 `--`），awk 校验 6/6 注释位于首个 `<%` 之前；与未改文件对比，插入位置为模板 raw-text 区，安全。
3. **git diff 复核**：本次改动 = 7 个 Makefile（标题标注、注释、ttlfw4 补 PKG_LICENSE_FILES）+ 6 个 htm 各 +1 行注释；**无任何逻辑改动**。diff 中其余文件（vendor/*.sh、application/*、next JS 等）为工作区既有未提交改动，本任务未触碰。
4. **CONFLICTS 复核**：luci-app-qmodem 无冲突声明；ttl↔ttlfw4 互斥为功能约束保留。
