# journal: vendored 上游项目风险分级审计

- **agent**: agent-vendored-audit
- **日期**: 2026-10-12
- **性质**: 只读审计，未修改任何源码；仅创建本文件与 `docs/security/vendored-audit.md`

## 审计对象

| 项目 | 行数 | 角色 |
| --- | --- | --- |
| `/workspace/QModem/application/quectel_CM_5G_M` | ~23.5k (src/*.c,*.h 实测 23484) | QMI/MBIM/QRTR/AT 数据面拨号 |
| `/workspace/QModem/application/qfirehose` | ~7.7k | EDL 刷机工具 |
| `/workspace/QModem/application/voipd` + `qmodem_sipd` | ~10k | VoIP 语音守护 |

## 过程记录

1. **上游来源与版本线索**
   - quectel_CM_5G_M：`src/ReleaseNote.txt` 最新条目 `[V1.6.5] 7/3/2023`；`src/main.c:985` 打印 `QConnectManager_Linux_V1.6.5`；`src/NOTICE` 为 Quectel 公版声明（public domain）。上游 = Quectel QConnectManager_Linux。
   - qfirehose：`src/ReleaseNote.txt` 最新条目 `[QFirehose_Linux_Android_V1.4.21] 08/14/2024`；`src/README.md`/`NOTICE` 为 Quectel 专有许可（二进制分发许可，禁止逆向）。上游 = Quectel QFirehose。
   - voipd / qmodem_sipd：无 NOTICE/ReleaseNote/版本宏，目录含 `tests/` 与 `files/etc,usr`，命名与仓库内 qmodem-* 系列一致；判定为**仓库内自研或深度本地化组件**，非典型 vendored 上游（provenance 需维护者确认）。

2. **高危模式扫描**（grep system/popen/sprintf/strcpy/setuid/recv 定位后逐一人工确认）
   - setuid/seteuid/setgid：四个项目均为 **0** 处。
   - quectel_CM_5G_M：system 35、popen 2、sprintf 32、strcpy 27。集中在 `udhcpc.c`/`udhcpc_script.c` 的 shell 拼接（ifname/ip/gw 均经 `ipv4Str`/`ipv6Str`/`inet_ntop` 数字化，或来自 sysfs 枚举，非网络文本）。`udhcpc.c:278` `while(!system(shell_cmd));` 为忙循环重试 ifconfig（逻辑缺陷非注入）。`QCQMUX.c:334` `sprintf(unknow[40], "unknow_%x", type)` 有界安全。`atc.c:610` `system("ifconfig %s up")` 仅设备名。
   - qfirehose：system 16 处，`firehose_protocol.c:547/990/1639…1976` 与 `md5.c:322/421`、`qfirehose.c:236/540`、`sahara.c:452` 均为 `snprintf(zip_cmd_buf)` 后 `system()`，其中 `file_name_image_dir` 取自升级包内 rawprogram XML/zip 清单 → 存在包内容驱动的 shell 元字符注入面。`firehose_protocol.c:276` `strncpy(...,512)` 不保证 NUL，`fh_xml_set_value`（:242-266）`strcpy` 回写固定 512 的 `fh_cmd.xml_original_data`，构造 XML 可越界。`usb2tcp.c:189-216` 绑定 `INADDR_ANY` 的无鉴权 TCP 服务，远端对等端直通 USB 管道（可刷机），启用依赖 CLI `usb2tcp_port`。
   - voipd/qmodem_sipd：无 system/popen/sprintf。`sip_gateway.c:417-419/479`、`:185-188`、`sip_consumer.c:1946` 的 strcpy 均有前置长度/格式校验（`strlen >= sizeof` 拒绝、root 属主文件、ha1 32-hex 校验）。`rtp_transport.c:178-190` recvfrom 校验源 IP==peer（未校验源端口，同 IP 伪造源可注入 RTP，风险低）。

3. **数据面暴露评估**
   - CM：处理 modem→host 的 QMI/QRTR/MBIM/AT 报文（`qrtr.c:171/238/321` recvfrom 有界；`QCQMUX.c` 按 TLV 解析，信任 modem 长度字段）。无 IP 监听，暴露面 = USB/QRTR 半可信边界（modem 被攻陷场景）。
   - qfirehose：处理 EDL 协议响应 + **可选 TCP 监听**（usb2tcp），是三者中唯一可能暴露 IP 网络端口的组件。
   - voipd/sipd：SIP/RTP UDP 面，源 IP 校验 + nonce 防重放（`sip_gateway.c:380-402`），防御成熟度显著高于上游件。

## 结论

- quectel_CM_5G_M V1.6.5：**维持 vendored（推荐）**，跟随上游；system 拼接均为本地设备名/数字 IP，无网络文本注入。
- qfirehose V1.4.21：**需上游补丁/本地加固**（zip 命令注入、xml_original_data 越界、usb2tcp INADDR_ANY），运行面限定 root 刷机场景并禁用 usb2tcp。
- voipd/qmodem_sipd：非 vendored，维持独立演进。

详细分级表见 `docs/security/vendored-audit.md`。
