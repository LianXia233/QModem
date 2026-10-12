# ADR 0001：Phase 4 重构决策记录

- **状态**：已接受（Accepted）
- **日期**：2026-10-12
- **关联**：`REFACTOR_ROADMAP.md`（W23 旧栈退役 Step 1）、`docs/refactor/journal-retire-s1.md` §3 端点 Gap 表、各 retire/audit journal（见各决策引用）

## 背景

Phase 0–3 已完成 C 层内存安全修复、LuCI next 前端、Rust 异步核心（qmodem-pdu / qmodem-atd）与 qmodem-suite 元包。Phase 4（旧栈退役与打包收尾）面临 7 项待决问题：TTL 双包重叠、mwan/voip/HC 的去留、AT 通路与 PDU 打包的 Rust 切换时机、qfirehose 三项高危漏洞的处置。本 ADR 依据各 retire journal 与 gap 表给出最终决策。

## 决策 1：TTL 并入 next 生态（ttlfw4 为载体，纳入 qmodem-suite）

**背景**：`luci-app-qmodem-ttl`（CBI lua + iptables）与 `luci-app-qmodem-ttlfw4`（JS 视图 + fw4/nft）功能重叠约 90%，共用 uci 节 `qmodem_ttl` 并双向 `PKG_CONFLICTS`；`luci-app-qmodem-next` 无任何 TTL 视图与端点（journal-retire-ttl.md §2、journal-retire-s1.md gap 表"整体 gap"行）。

**决策**：
1. 以 ttlfw4 为唯一载体（保留 JS UI + fw4/nft 后端），纳入 `qmodem-suite` 元包依赖；
2. next 增加 TTL 视图（纯 uci + `rc`/`luci setInitAction`，无需新 ubus 端点）；
3. `luci-app-qmodem-ttl`（iptables 旧栈变体）在 Phase 4 退役；
4. 修复 ttlfw4 init.d:34 对不存在的 `/etc/firewall.d/qmodem_ttl` 的残留调用（journal-retire-ttl.md §1 遗留项）。

**理由**：两包同源数据模型，双包维护 + CONFLICTS 互斥纯成本；fw4/nft 是 OpenWrt 主线方向，ttlfw4 依赖更轻（仅 `+luci-base`）；TTL 功能（绕过运营商共享限制）仍有真实用户需求，不宜直接弃用。

**被否方案与理由**：
- *保留双包维持现状*：双份维护成本与互斥安装体验无收益；
- *以旧栈 ttl 为载体*：iptables 随 fw3 时代淘汰，且依赖旧栈 UI 入口 `+luci-app-qmodem`，与退役方向相反；
- *整体弃用不迁移*：功能有需求，弃用迫使依赖此功能的用户自行手写 nft/iptables 规则。

**后果**：suite 安装面多一项 fw4 依赖；退役 ttl 后需删除双向 CONFLICTS 中已失效的一侧；next TTL 视图成为 Phase 4 的前端交付项。

## 决策 2：mwan 保留 legacy 维护模式（不迁移、不弃用、不入 suite）

**背景**：`luci-app-qmodem-mwan` 是纯 uci→mwan3 的离线配置生成器，无 ubus/HTTP 自定义端点；next 无对应页面且 ACL 未含 `mwan3`/`qmodem_mwan`（journal-retire-mwan.md §2）；功能与上游 `luci-app-mwan3` 官方 UI 高度重叠，但 `99-setup-mwan` 为 HC-G80/ailf gs2410/huasifei ws3006 三款量产设备预置多拨成员，且 `modem_mwan.sh` 的 `add_by=modem` 增量管理（不碰用户手写 mwan3 节）上游不具备。

**决策**：保持 legacy 维护模式随旧栈共存：不迁移 next、不弃用、不纳入 qmodem-suite；未来若 next 需要 mwan 能力，优先引导用户直接使用 `luci-app-mwan3`。

**被否方案与理由**：
- *迁移到 next*：需新建 JS 视图 + ACL + 后端联动，成本高而自研页面相对上游价值有限；
- *直接弃用*：破坏三款量产设备的出厂预置行为；
- *纳入 suite*：与上游 mwan3 生态重叠，收进元包会引入无谓的依赖耦合。

**后果**：遗留观察项——Makefile conffiles 误写 `/etc/config/modem_mwan`（实际为 `/etc/config/qmodem_mwan`），涉及打包行为变更，在 Phase 4 收尾时一并处理（journal-retire-mwan.md §3.5）。

## 决策 3：voip 独立保留（不入 suite，单包安装）

**背景**：`luci-app-qmodem-voip` 是实验性单路语音前端，依赖链自足（`+qmodem-voip +qmodem-sipd +luci-base`），与旧栈 `luci-app-qmodem` 零耦合；next 的 acl.d/menu.d 对 `voip|sip` 零匹配（journal-retire-voip.md §2）；ACL 粒度合理、无注入/XSS 高危模式，维护成本估计每季度 <1 人日。

**决策**：独立保留，不纳入 Phase 4 旧栈退役范围，不纳入 qmodem-suite，维持单包安装；后端（voipd/qmodem_sipd）未来迁 Rust 时前端 ubus 契约（方法名/快照字段）原样保留。

**被否方案与理由**：
- *并入 suite*：实验性 + 硬件白名单（仅 RM520N-GL）限定，并入会扩大元包安装面与依赖，违背 suite"核心即用"定位；
- *迁移 next*：功能正交，next 覆盖范围（模组信息/拨号/AT/SMS）不含语音链路，迁移无收益；
- *弃用*：成本极低且无安全包袱，弃用损失大于维护支出。

**后果**：浏览器 API 演进（AudioWorklet 降级已内置）与白名单扩展是长期维护点；网络面隔离（voipd/sipd 沙箱）按 vendored-audit §4 建议另行推进。

## 决策 4：HC——set_sim_slot 等效性实机验证前，luci-app-qmodem-hc 保留

**背景**：HC-G80 的 SIM 切换走 `fw_printenv`/`fw_setenv`（uboot env `sim2`）+ GPIO 直写（`/sys/class/gpio/sim`、`/sys/class/gpio/4g` 脉冲复位），与 next 的 `set_sim_slot`（rpcd → vendor AT 脚本如 `AT+QUIMSLOT`）是完全不同的硬件路径；next ACL 无 qmodem_hc 条目，gap 成立（journal-retire-hc.md §1–2）。

**决策**：在完成 journal-retire-hc.md §2 的 7 步实机验证清单（vendor 是否实现 set_sim_slot → GPIO/env 基线采集 → next 路径驱动观察 → next boot 槽写入 → 复位语义对比 → 重启持久性 → 结论落盘）之前，`luci-app-qmodem-hc` 保留（legacy 标注 + 设备级 `@TARGET_mediatek_...hc-g80` 限定不变）。若 1–6 步证明不等效，需在 next 增加 HC 专用 ubus 方法（如 `qmodem.hc set_sim`）或将本包逻辑收编为 vendor script，之后方可退役。

**被否方案与理由**：
- *直接随旧栈退役*：破坏 HC-G80 切卡功能与 uboot env/GPIO 出厂一致性；
- *立即将 GPIO 逻辑移植进 next*：未验证 vendor AT 路径能否驱动 HC 硬件（清单第 1、3 步未做），移植可能白费且引入设备专用分支。

**后果**：Phase 4 退役清单中 HC 行状态为"验证阻塞"；验证工作依赖实机，需安排设备排期；已知逻辑疑点（modem_sim.sh `_get_netdev` 的 config_load 三参误用）留档待验证时一并确认（journal-retire-hc.md §1.3.4）。

## 决策 5：at_tool 默认保持 0（tom_modem），切 2（Rust atd）须满足灰度标准

**背景**：`qmodem.main.at_tool` 三通路：0=tom_modem（默认，行为不变）、1=sms_tool_q、2=qmodem-atd（Rust，unix socket JSON 行协议，socket 缺失直接报错不回退，journal-atd-bridge.md）。Rust 侧质量建设完成（journal-atd-quality、bench-atd），但实机数据不足。

**决策**：
1. `at_tool` 默认值保持 `'0'`（tom_modem）；
2. 将默认切为 `'2'` 的灰度标准（两项同时满足）：
   - 连续 N 天（建议 N=7，由 CI 门禁定义）fixture 回放零差异（at()/fastat() 三通路输出逐字节一致）；
   - 实机 72h 稳定运行：守护进程无崩溃、无异常回落、拨号/短信/监控链路无回归。

**被否方案与理由**：
- *立即默认 2*：at_tool=2 语义是"选了不存在的工具就报错"，Rust atd 未装机时开箱即坏；且 timeout_ms（1s/3s）与外层看门狗（2s/8s）的组合依赖经验证但样本少；
- *不定标准、凭感觉切换*：灰度不可度量，回退无判据。

**后果**：灰度门禁需固化为自动化检查（fixture 回放进 CI 日任务）；切换时同步更新 at-tool-selection 文档与 UCI 注释；socket 不可达回落 tom_modem 的增强（本版本已引入）不改变该标准。

## 决策 6：PDU 打包切换——SDK 带 Rust 工具链时代切 rust/qmodem-pdu

**背景**：rust/qmodem-pdu 已达 64 测试全绿（TS 23.038 一致性 + 对 C pdu.c 的语义交叉对照 + property/往返，journal-pdu-conform.md），并修复了 C 版未覆盖的 SCTS 时区语义 bug（>±2:30 时区全部错值）；C 版 `libqmodem-sms/pdu.c` 仍是打包默认；构建 SDK 是否含 Rust 工具链不可控（交叉编译脚本 `scripts/cross-build-rust.sh` 已备，journal-rust-cross.md）。

**决策**：默认打包维持 C 版实现；当目标构建 SDK 提供 Rust 工具链（`cross-build-rust.sh` 可用）时，打包切换为 `rust/qmodem-pdu`；C 侧若后续补时区解析，直接采用标准 semi-octet 语义（TS 23.040 §9.2.3.11），不复刻旧错误。

**被否方案与理由**：
- *立即全量切 Rust*：无工具链的 SDK 打包直接失败，破坏发布确定性；
- *双实现长期并行维护*：已知语义分歧（C 将 total≤1 清零、C 无国际 TOA `+` 前缀、C 忽略时区八位组）会随时间漂移，双份一致性成本高。

**后果**：qmodem-pdu README:32 的旧时区语义描述待更新（journal-pdu-conform.md §4 遗留）；切换时需在打包层做工具链探测开关。

## 决策 7：qfirehose 三项高危的处置（来源校验 + 默认禁用 usb2tcp，上游补丁跟踪）

**背景**：`docs/security/vendored-audit.md` 表 #1–#3（均为上游 QFirehose V1.4.21 专有代码）：① `firehose_protocol.c` 等 8 处 `system(zip_cmd_buf)` 恶意升级包 shell 注入（高）；② XML `strncpy` 无 NUL + `fh_xml_set_value` strcpy 回写固定 512 缓冲溢出（高）；③ `usb2tcp.c:189-216` `bind(INADDR_ANY)` 无鉴权 TCP→USB 直通管道，远端可刷机（高，CLI 传 `usb2tcp_port` 时启用）。

**决策**：
1. 短期运行时缓解：升级包来源校验（内置 md5 之外追加签名校验）；默认禁用 usb2tcp（仅 CLI 显式传 `usb2tcp_port` 且文档警示时可用）；
2. 长期：作为上游补丁跟踪项（QFirehose_Linux_Android V1.4.21），随上游版本更新修复；
3. 运行约束：仅 root 手动执行。

**被否方案与理由**：
- *本地深改专有源码*：QFirehose 许可禁逆向/修改，且本地 fork 无法随上游升级，维护死路；
- *直接移除 qfirehose*：高通平台刷机功能依赖该组件，移除属于功能回退而非安全处置。

**后果**：需在打包层落实"默认禁用"（不安装启用 usb2tcp 的 wrapper/服务）；上游跟踪项录入 issue 并在 ReleaseNote 版本变更时复核三项是否修复。

## 总体后果与检查点

- Phase 4 退役范围收敛为：luci-app-qmodem-ttl（决策 1）+ next TTL 视图交付；mwan/voip/HC 按上表保留。
- 灰度与工具链两个"条件切换"（决策 5、6）均以可度量门禁为准，不允许无判据切换。
- qfirehose 安全缓解（决策 7）属发布阻断项，先于其余决策落地。

## 参考

- docs/refactor/journal-retire-s1.md（gap 表）
- docs/refactor/journal-retire-ttl.md / journal-retire-mwan.md / journal-retire-voip.md / journal-retire-hc.md
- docs/refactor/journal-atd-bridge.md / journal-atd-quality.md / bench-atd.md / journal-tom-modem.md
- docs/refactor/journal-pdu-conform.md / journal-rust-cross.md / journal-pkg-suite.md
- docs/security/vendored-audit.md
