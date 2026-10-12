# journal-pdu-conform — qmodem-pdu TS 23.038 一致性测试扩展

> 维护者：agent-pdu-conform
> 范围：`rust/qmodem-pdu/tests/**`（新增）+ `rust/qmodem-pdu/src/lib.rs`（仅修测试暴露的真 bug）
> 日期：2026-10-12

## 1. 目标与结果

将 PDU codec 测试从 **14 → 64 个**（≥40 达标），`cargo test -p qmodem-pdu` 全绿，
`cargo test --workspace` 全绿，`cargo clippy --all-targets` 0 警告，零外部依赖
（property 测试自带 xorshift64* PRNG），全部文件 LF。

| 测试文件 | 数量 | 内容 |
|---|---|---|
| `tests/ts23038_conformance.rs` | 19 | TS 23.038/23.040 固定向量：7bit 基本表/转义表/控制符（3+）、8bit 裸字节与 UDH（2）、UCS2 ASCII/CJK/代理对（3）、SCTS ±12:45 边界 + 全域 0..79 与 12 组已知八位组、UDH IEI 0x00/0x08/未知 IE 跳过/IE 溢出 |
| `tests/cross_c_pdu_semantics.rs` | 12 | 从 `application/libqmodem-sms/src/pdu.c` 语义推导的交叉对照向量，每条注明源码行号（见下） |
| `tests/property_robustness.rs` | 10 | 全前缀截断、5000 次随机字节、单字节角值变异（0 前缀长度/0xFF 病态 SMSC/符号位极值）、UDL=0、UDL>实际、UDL<实际、SMSC len 0/1/0x7F/0xFF、UDH 病态（UDHI 空载荷/UDHL 谎报/UDHL=0）、随机 SCTS、错误 TP-MTI |
| `tests/roundtrip.rs` | 9 | encode_sms_submit→DELIVER→decode 往返：7bit ASCII/转义/160 septets/0x00 特例、UCS2 CJK/70 units、8bit、SCTS -79..=79 全偏移往返、地址奇偶位/TOA `+` 前缀 |
| `tests/common/mod.rs` | — | 独立位串式 pack7（不依赖被测实现）、deliver/deliver_with_udh 组包器、xorshift64* |

## 2. C 语义交叉对照行号索引（pdu.c）

| 测试 | 依据 |
|---|---|
| c_7bit_packing_hellohello | pdu.c:44-55（dreamfabric 注释样例 E8 32 9B FD 46 97 D9 EC 37）、EncodePDUMessage pdu.c:59-85 |
| c_7bit_unpack_boundaries | DecodePDUMessage_GSM_7bit pdu.c:89-119（carry 位重组 + 余数）；(len*7+7)/8 容量式 pdu.c:63 |
| c_gsm7_latin1_table | gsm7bits_to_latin1 pdu.c:123-132（0x00 '@'、0x01 '£'、0x0a '\n'、0x1c-0x1e Ææß） |
| c_gsm7_extend_table | gsm7bits_extend_to_latin1 pdu.c:134-143 + G7bitToAscii 转义对解析 pdu.c:255-263 |
| c_parse_udh_concat_ie00 | parse_udh pdu.c:205-208 |
| c_parse_udh_concat_ie08 | parse_udh pdu.c:209-213（16-bit ref 大端拼接） |
| c_parse_udh_total_le_1_divergence | pdu.c:216-218（C 将 total≤1 清零；Rust 有意保留原始 IE，已在注释声明分歧） |
| c_sender_semi_octet_and_plus_prefix | DecodePhoneNumber pdu.c:351-363；分歧：Rust 对国际 TOA 加 `+`（C 无，pdu.c:576-577） |
| c_dcs_group_routing | pdu.c:614 `(*tp_dcs/4)%4`：0x00→7bit、0x04→default(不处理)、0x08→UCS2；0x0C 亦归 7bit |
| c_encode_sms_submit_layout | pdu_encode pdu.c:366-437：SMSC len 385-386、0x11 391、MR 392、TOA 397-401、PID/DCS/VP 414-416、UDL 428 |
| c_udh_skip_septets_formula | pdu.c:622-624 `(skip*8+6)/7` 填充 septet 公式（6 octet UDHL → 8 filler） |
| c_ascii_to_g7bit_npc | NPC '?' pdu.c:274 + AsciiToG7bit pdu.c:295-319（UTF-8 多字节折叠为单 NPC） |

## 3. 测试暴露的真 bug（src 修改记录）

**SCTS 时区解码/编码违反 TS 23.040 §9.2.3.11**（`lib.rs` parse_scts / encode_scts）。

- 旧实现：`count = tz & 0x0f`（低半字节直接当 15 分钟数，仅能表达 ±3:45），
  `0x80` 当负号。
- 正确语义（三方独立来源交叉确认）：时区为 quarter-hour 数的 semi-octet 编码——
  低半字节 = 十位数字 + 符号位 0x08，高半字节 = 个位数字，范围 0..=79
  （±19:45）。+8:00(32q)=0x23、-2:00(8q)=0x88、+12:45(51q)=0x15、-12:45=0x1D。
  依据：TS 23.040 §9.2.3.11 原文（"In the first of the two semi-octets, bit 3
  …sign"，含 DST 例 01000000=+1:00=0x40）；warthog618/sms `encoding/bcd/bcd.go`
  EncodeSigned/DecodeSigned；Android AOSP `SmsMessage.getSCTimestampMillis`
  （sign = tzByte & 0x08）。
- 实害：>±2:30 的时区全部错值——印度 +5:30(22q) 被解成 +0:30，中国 +8:00(32q)
  被解成 +2:00，任务要求的 ±12:45(51q) 完全无法表达；且旧编码把个位放进低半
  字节，与其它 BCD 字段（年月日时分秒）方向不一致。
- 附带修复：units 半字节 > 9（0xA..0xF，非 BCD）现在返回
  `Invalid("bad scts field")`（与 Android "digit 视为 0" 的宽松策略不同，
  与本 crate 严格校验月/日/时的既有风格一致）。
- 同步修正 `lib.rs` 内 3 处受影响断言/向量：`scts_timezone_polarity` 的 +8q
  八位组 0x08→0x80；`cross_vector_ucs2_nihao` 向量时区 0x08→0x23（北京时间
  +8:00=32q，博客原向量把 0x08 误当 +8:00，属同源 Folk 错误）；
  `truncated_input_never_panics` 向量同步。

## 4. 遗留事项

- `rust/qmodem-pdu/README.md:32` 仍描述旧时区语义（"低半字节为 15 分钟步长
  偏移数，高半字节 bit 0x80 表示负偏移"）——README 不在本 agent 修改范围
  （仅允许 tests/** 与 src），**需后续编排者更新为 semi-octet 十位/符号 0x08
  语义**。
- C 版 pdu.c 完全忽略时区八位组（pdu.c:586-592 未解析），故该修复不构成
  C/Rust 行为分歧；若 C 侧后续补时区解析，应直接采用标准语义。

## 5. 验证记录

- `cargo test -p qmodem-pdu`：64/64 通过（lib 14 + conformance 19 + cross 12
  + property 10 + roundtrip 9；其中 scts_range_and_roundtrip 含 0..79 全偏移
  ×2 方向共 271 组子断言）。
- `cargo test --workspace`：全部通过（qmodem-atd 等 crate 无回归）。
- `cargo clippy -p qmodem-pdu --all-targets`：0 警告。
- 新增文件均为 LF、零依赖（tests 仅依赖 `qmodem_pdu` 本体 + std）。
