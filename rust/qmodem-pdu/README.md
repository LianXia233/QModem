# qmodem-pdu

3GPP TS 23.038 SMS PDU 编解码库，对应 C 版 `application/libqmodem-sms/src/pdu.c` 的 Rust 重写（修复版语义）。

## 公开 API

```rust
// SMS-DELIVER 解码（SM-RP-DUD，+CMGR 读回内容）
pub fn decode(buf: &[u8]) -> Result<SmsDeliver, PduError>;

// SMS-SUBMIT 编码（AT+CMGS 前的 PDU，pdu.len()-1 即长度参数）
pub fn encode_sms_submit(phone_number: &str, text: &str, dcs: Dcs)
    -> Result<Vec<u8>, PduError>;

pub fn encode_scts(ts: &Timestamp) -> [u8; 7];
```

- `SmsDeliver { smsc, sender, pid, dcs, scts, udh, text, ud }`：SMSC/发送方号码（TOON 解析，国际号带 `+`）、TP-PID/TP-DCS（GSM 7-bit / 8-bit / UCS-2）、SCTS 时间戳、UDH 分片（IEI 0x00/0x08）、解码正文（UCS-2 转 UTF-8、7-bit 位填充含 UDH filler、8-bit 直通为 Latin-1 字符）与原始 UD。
- `PduError::{Truncated, Invalid, TooLong, InvalidNumber}`。

## 相对 C 版修复的缺陷（引用 pdu.c 行号）

| C 版问题 | 位置 | Rust 行为 |
|---|---|---|
| 发送方地址越界读（未校验 buffer 覆盖地址字段） | pdu.c:552-562 | 全部索引走 `get()`，越界返回 `PduError::Truncated` |
| 数字校验恒假 `c < '0' && c > '9'` | pdu.c:335 | `encode_sms_submit` 严格校验纯数字，非法返回 `InvalidNumber` |
| 固定 320 字节栈缓冲 + UTF-8 展开可溢出 | pdu.c:418-421, 301-318 | 7-bit ≤160 septets / UCS-2 ≤70 units / 8-bit ≤140 octets，超限返回 `TooLong` |
| UCS-2 原样拷贝不转码 | pdu.c:616-617 | UTF-16BE → UTF-8，含代理对校验 |
| `DecodePDUMessage_GSM_7bit` 中 `buffer[i-1]` 可为 -1 下标 | pdu.c:115 | 位流重写为累加器式 pack/unpack，无负下标 |
| SMSC/时间戳半八位组无边界检查 | pdu.c:548-577 | 每个字段独立长度校验 |

时区半字节语义：低半字节为 15 分钟步长偏移数，高半字节 bit 0x80 表示负偏移（`Timestamp::timezone_quarters`，含正负偏移往返测试）。

## 测试

`cargo test -p qmodem-pdu` 覆盖：3 个与 C 解码布局交叉验证的手工向量（UCS-2「你好」、7-bit "hellohello" dreamfabric 位布局、UCS-2+UDH 分片）、SCTS 正负时区、7-bit 位填充对齐、UDH filler septet 偏移、代理对、号码奇偶位长编码、长度上限、**截断穷举**（对每个向量每个前缀长度 `decode` 均返回 `Err` 不 panic）、畸形输入（错误 MTI/TOA/半八位组/奇数 UCS-2）。

依赖仅 `thiserror`。
