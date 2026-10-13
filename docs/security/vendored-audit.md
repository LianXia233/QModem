# Vendored 上游项目安全审计报告

- **日期**: 2026-10-12 · **审计人**: agent-vendored-audit · **性质**: 只读审计，未改动任何源码
- **范围**: `application/quectel_CM_5G_M`、`application/qfirehose`、`application/voipd` + `application/qmodem_sipd`
- **方法**: grep 定位 `system/popen/sprintf/strcpy/setuid/recv` 等模式后逐一人工确认；所有路径为仓库内绝对相对路径，行号以当前工作树为准

## 1. 上游来源与版本线索

| 项目 | 上游 | 版本线索 |
| --- | --- | --- |
| quectel_CM_5G_M | Quectel QConnectManager_Linux | `src/ReleaseNote.txt` `[V1.6.5] 2023-07-03`；`src/main.c:985` `QConnectManager_Linux_V1.6.5`；`src/NOTICE` 公版声明 |
| qfirehose | Quectel QFirehose（专有许可，禁逆向） | `src/ReleaseNote.txt` `[QFirehose_Linux_Android_V1.4.21] 2024-08-14` |
| voipd / qmodem_sipd | 无 NOTICE/版本宏，含 `tests/` 与 qmodem-* 一致命名 → 疑为仓库内自研/深度本地化，**provenance 待确认** | — |

高危模式统计：setuid/euid/gid 全部 0 处；CM（system 35 / popen 2 / sprintf 32 / strcpy 27）；qfirehose（system 16 / sprintf 4 / strcpy 5）；voipd+sipd（system/popen/sprintf 0，strcpy 7）。

## 2. 最严重点（人工确认，≤10）

| # | 位置 | 模式 | 触发面 | 严重度 |
| --- | --- | --- | --- | --- |
| 1 | `application/qfirehose/src/firehose_protocol.c:547,990,1639-1976`（同型 `qfirehose.c:236,540`、`md5.c:322,421`、`sahara.c:452`） | `system(zip_cmd_buf)`，路径含升级包内 `file_name_image_dir` | 恶意升级包 shell 元字符注入 | 高 |
| 2 | `application/qfirehose/src/firehose_protocol.c:276`（strncpy 无 NUL）+ `:242-266` `fh_xml_set_value` strcpy 回写固定 512 `fh_cmd.xml_original_data` | 缓冲区溢出 | 构造 rawprogram XML（本地包） | 高 |
| 3 | `application/qfirehose/src/usb2tcp.c:189-216` | `bind(INADDR_ANY)` 无鉴权 TCP → USB 直通管道 | 远端主机直连可刷机 | 高（启用条件：CLI 传 `usb2tcp_port`） |
| 4 | `application/qmodem/src/quectel_CM_5G_M/src/udhcpc.c:278` | `while(!system(shell_cmd));` 忙循环重试 | 逻辑缺陷/卡死（非注入） | 中 |
| 5 | `application/qmodem/src/quectel_CM_5G_M/src/udhcpc_script.c:84-101` | `system()` 拼接 `getenv(IFUP_SCRIPT)` | 环境变量可控命令执行（本地） | 中 |
| 6 | `application/qmodem/src/quectel_CM_5G_M/src/udhcpc.c:112-116,259-304`、`atc.c:610` | `system()` 拼接 ifname/ip/gw | 变量均经 `ipv4Str/ipv6Str/inet_ntop` 或 sysfs 枚举，无网络文本 | 低-中 |
| 7 | `application/qmodem/src/quectel_CM_5G_M/src/qrtr.c:171,238,321` + `QCQMUX.c` TLV 解析 | 信任 modem 侧长度字段 | modem 被攻陷 → host 半可信边界 | 中（架构性） |
| 8 | `application/qmodem_sipd/src/rtp_transport.c:178-190`（voipd 同型 `rtp_transport.c:178`） | recvfrom 仅校验源 IP，未校验源端口 | 同 IP 伪造 RTP 流 | 低 |
| 9 | `application/qmodem_sipd/src/sip_consumer.c:1946` `strcpy(app.lan_address, argv[4])` | argv 无长度拷贝 | 前置 `:1899` `valid_lan_address` 校验，缓解 | 低 |
| 10 | `application/qfirehose/src/usb_linux.c:292,305` sprintf 拼接 sysfs 路径 | sprintf 到固定缓冲 | udev 枚举输入，路径长度受内核限制 | 低 |

已排除的疑似点：`QCQMUX.c:334` `sprintf(unknow[40],"unknow_%x")`（UINT 有界）；`sip_gateway.c:417-419/479` strcpy（前置 `strlen >= sizeof` 拒绝）；`sip_gateway.c:185-188` 凭据 strcpy（root 属主 + ha1 32-hex 校验）；`md5.c:447` sprintf（64 字节有界）。

## 3. 数据面暴露评估

| 组件 | 网络侧输入 | 暴露面 |
| --- | --- | --- |
| quectel_CM_5G_M | QMI/QRTR/MBIM/AT 报文（USB/QRTR 内核套接字），无 IP 监听 | modem→host 半可信边界；shell 拼接均本地来源 |
| qfirehose | EDL 响应 + **唯一可能监听 IP 端口**（usb2tcp） | 本地升级包 + 可选远程 TCP |
| voipd/qmodem_sipd | SIP/RTP UDP，nonce 防重放（`sip_gateway.c:380-402`）、源 IP pin | 自研，防御成熟度最高 |

## 4. 结论分级

| 项目 | 结论 | 依据与建议 |
| --- | --- | --- |
| quectel_CM_5G_M V1.6.5 | **维持 vendored（推荐）** | 无网络文本注入面；`udhcpc.c:278` 忙循环、`udhcpc_script.c` getenv 拼接建议随上游 1.6.x 更新修复 |
| qfirehose V1.4.21 | **需上游补丁** | 表 #1-#3 均在上游代码；短期运行时缓解：仅 root 手动执行、禁用 usb2tcp、升级包来源校验（md5 已内置，建议追加签名） |
| voipd + qmodem_sipd | **建议隔离运行（网络面独立/沙箱）** | 疑为自研非 vendored，维持独立演进；RTP 源端口校验可后续加固 |

---
*本报告为只读审计产物，未修改任何源码文件。*
