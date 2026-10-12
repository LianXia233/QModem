# luci-app-qmodem-voip 退役审查（结论：独立保留）

- **agent**: agent-retire-voip
- **日期**: 2026-10-12
- **范围**: `luci/luci-app-qmodem-voip/**`（Makefile + 两个视图 JS 声明式注释）；应用侧代码仅只读理解，不深审 C。
- **前置**: 承接 journal-retire-s1.md §3 gap 表「语音 voip —— 整体 gap，建议长期独立保留」行，给出结论依据。

## 1. 功能清单

本包是**实验性单路语音**的 LuCI 前端，自成一体的依赖链，与旧栈 `luci-app-qmodem` 无耦合（Makefile 依赖 `+qmodem-voip +qmodem-sipd +luci-base`，无旧 UI 依赖）。

| 组件 | 角色 |
|---|---|
| `htdocs/.../qmodem-voip/contract.js` | ubus 契约常量（object/methods/params/快照字段/错误码） |
| `rpc.js` | 按 contract 批量 `rpc.declare`，另声明 `qmodem.sip status/generate_credentials` |
| `reducer.js` | 呼叫状态机（9 态：disabled/idle/outgoing_setup/incoming_ringing/early_media/active/terminating/recovering/fault）+ revision/restart_epoch 单调性防旧快照回退 |
| `media.js` | 浏览器媒体客户端：WSS WebSocket（自帧格式 magic `QMVP`/48 kHz/PCM16）+ AudioWorklet（Safari 降级 ScriptProcessor） |
| `surface.js` / `call.js` | 拨号盘/DTMF/呼叫历史/来电弹窗 UI；1 s 轮询 + ubus 事件双通道 |
| `sip.js` | `qmodem_sip` UCI CBI：inbound（UDP/TCP LAN）与 outbound（TLS WAN）双向独立配置、凭据生成 |
| `tests/` | reducer/media/module-loader node 测试 + contract.sh 契约自检 |

### 依赖的后端（只读应用侧理解）

| 后端 | ubus 对象 | 方法（ACL 授予） | 说明 |
|---|---|---|---|
| `application/voipd` | `qmodem_voip` | status/capabilities/call_history（读）；enable/disable/originate/answer/reject/hangup/send_dtmf/issue_media_token（写） | 调制解调器呼叫控制（AT 适配 `at_daemon_adapter.sh` + `modem_safety.sh` 安全探针，仅 RM520N-GL 白名单 `modem_profiles.json`） |
| `application/qmodem_sipd` | `qmodem.sip`（另有 `qmodem.sip.inbound/outbound` direction 实例对象） | status；generate_credentials | SIP 注册/信令网关，uci `qmodem_sip` |
| 事件 | `qmodem_voip.call` | — | ubus event，前端快照增量更新 |
| uhttpd | — | — | `web_enabled` 时经 `qmodem_voip_web` 增删 443 HTTPS 监听；WSS 媒体走 uhttpd 反代 |

### 信令面 / 媒体面

- **信令面**：浏览器→rpcd→`qmodem_voip`（呼叫指令）与 `qmodem.sip`（SIP 注册/凭据）；qmodem_sipd 与外部 SIP 服务器走 UDP/TCP(inbound)/TLS(outbound) 5060/5061。
- **媒体面**：voipd 直接驱动 modem 音频（ALSA/串行/RTP `rtp_transport.c`，G.711，RTP 40000-40031），经 `media_socket`（`/var/run/qmodem_voip/media.sock` unix socket）桥接；浏览器侧经 `issue_media_token`（session_id + call_revision + HTTPS origin 绑定，token 仅入 WebSocket 子协议头）拿到一次性凭据，WSS 二进制 PCM 帧收发，页面 origin 强制覆盖广播地址防 LAN/WAN 串流。
- **安全设计**：硬件白名单安全探针（只读 probe 通过才解锁控件）、revision/epoch 状态恢复、token 与 LuCI 会话解耦（media.js:126-234）。

## 2. 与 next 覆盖对照（gap 结论）

实测 `luci-app-qmodem-next` 的 acl.d 与 menu.d 对 `voip|sip` **零匹配**；retire-s1 gap 表判断一致。

| 项目 | 结论 |
|---|---|
| next 对应物 | 无。next 覆盖范围 = 模组信息/拨号/AT/SMS；语音链路（voipd + qmodem_sipd + uhttpd WSS + ACL `issue_media_token` 等 10 方法）完全独立 |
| **结论** | **独立保留**，不纳入 Phase 4 旧栈退役 |
| 理由 | ① 功能正交：无旧栈 UI/luad 端点依赖，退役旧栈不影响它，反之它也不阻塞 next；② 已是维护模式（标题 legacy/maintenance，实验性硬件白名单），无历史包袱；③ ACL 粒度合理（read-only 普通用户 / admin 呼叫控制），无 retire-s1 所述注入/XSS 类高危模式（本包 shell 仅 tests/contract.sh，POSIX 合规） |
| 维护成本估计 | **低**。无 vendor 变体适配面（不像 quectel/huawei 脚本需跟随固件变化）；状态机契约集中在 contract.js 一处；主要维护点为浏览器 API 演进（AudioWorklet 兼容已内置降级）与 RM520N-GL 白名单扩展。估计每季度 <1 人日级补丁；若底层 voipd/qmodem_sipd 迁 Rust（Phase 3+），前端契约（ubus 方法名/快照字段）可原样保留 |

## 3. 本次改动（声明式标注）

| 文件 | 改动 |
|---|---|
| `Makefile` | +5 行 retire-audit 注释（独立保留结论 + 依赖链说明），标题/依赖零改动 |
| `view/qmodem-voip/call.js` | 首行 +2 行声明式注释（本包无 .htm，此为 htm 注释的等价标注） |
| `view/qmodem-voip/sip.js` | 同上 |
| htm | **本包无任何 .htm**（retire-s1 已确认 ttlfw4/voip 为 JS 视图），无插入对象 |

**注入/bashism 扫描**：包内唯一 shell 为 `tests/contract.sh`——POSIX sh、`set -eu`、变量全引号、`$(dirname "$0")` 安全展开，无 `[[`/`$((`/eval，未发现注入点；**无需修改**。全包无 CRLF。

## 4. 验证记录

1. `sh -n tests/contract.sh` → 通过。
2. `node --check` 改动的 call.js / sip.js → 通过（声明在 directive 前的注释不影响 `'use strict'` prologue，且 contract.sh 的 `find -exec node --check` 门禁复跑 9 文件全过）。
3. `tests/contract.sh` 全量复跑 → `PASS: qmodem voip LuCI package contract`。
4. `git diff --stat` 仅 3 文件、全为注释/空行，`git diff` 逐行复核**无逻辑变更**；本包改动前工作区干净（`git status` 为空）。
