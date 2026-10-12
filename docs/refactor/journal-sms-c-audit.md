# SMS C 代码审计与修复日志（journal-sms-c-audit）

> 本日志由 agent-sms-c-audit 维护。范围：application/libqmodem-sms/** 与 application/sms-tool_q/**。
> 语义对照参考：rust/qmodem-pdu（pack_septets / encode_sms_submit / decode 及其测试向量）。
> 前置：Phase 0（journal-c-core.md C5/C6）已修 libqmodem-sms/src/pdu.c 的 pdu_decode/pdu_encode 主要边界。

## S1 EncodePDUMessage 尾字节缺陷（两处副本，已修）

【问题】
尾字节判定 `if (i <= sms_text_length)` 过宽：循环经 carry 复位跳过退出时 i == length，仍会读
`sms_text[length]`（越界读，字符串调用时恰为 NTL）并多写一个垃圾 octet。实测触发条件为
`length % 8 == 0`（n=0、8、16…64 差分测试全部命中）；当 `buffer_size` 恰为 `(7n+7)/8` 时构成
公共 API 越界写（仓库内现有调用点侥幸未触发）。与 rust pack_septets 的 `ceil(7n/8)` 契约不符。

【修复方式】
收紧为 `if (i == sms_text_length - 1)`：仅当循环以自身条件退出（最后一个 septet 未打包）时补写
尾 octet；n≡0 (mod 8) 与 n=0 均不再产生多余字节。两份副本同步修复。

【验证结果】
差分测试 n=0..64（伪随机 septet，对照 rust pack_septets 等价 Python 实现）：修复前 9 处不匹配，
修复后 0 处。libqmodem tests（test_ucs2_parts）通过。

【涉及文件:行号】
- application/libqmodem-sms/src/pdu.c:81-88（条件收紧+注释）
- application/sms-tool_q/src/pdu_lib/pdu.c:82-89（同）

## S2 libqmodem-sms 剩余边界（已修）

(a) `DecodePDUMessage_GSM_7bit`：`buffer_length == 0 && sms_text_length > 1` 时尾部 remainder
    读 `buffer[i-1]` == buffer[0] 越界（当前调用方恰不触发）→ 补 `buffer_length > 0` 守卫
    （pdu.c:96-98）。
(b) `G7bitToAscii`：扩展表 128 项但以 `(unsigned char)` 全八位索引 → 加 `& 0x7F` 掩码
    （pdu.c:263-265）。
(c) `pdu_decode` 时间戳 `struct tm` 栈对象部分字段（tm_isdst 等）未初始化即传 `timegm` →
    改 `= {0}`（pdu.c:588-589）。
(d) ucs2_to_utf8.c 范围边界差一：`ucs2 < 0xFFFF`（0xFFFF 被误拒）、`ucs2 < 0x10FFFF`
    （0x10FFFF 被误拒）→ 改 `<=`（ucs2_to_utf8.c:31,42）。注：现调用链仅传 16 位 UCS-2，
    0x10000 分支不可达，属防御性修复。

## S3 sms-tool_q/src/pdu_lib/pdu.c（Phase 0 未覆盖，已修）

同源于 journal-c-core C5/C6 的旧版缺陷，本次在本体修复：
(a) `pdu_encode`：`length < 0 && length >= 254` 恒假 → `||`（pdu.c:276）；
(b) `EncodePhoneNumber`：`< '0' && > '9'` 恒假 → `||`（pdu.c:227）；
(c) `pdu_encode`：目标号码 `EncodePhoneNumber` 返回 -1 未检查即累加 → 检查后 return -1
    （pdu.c:298-299）；
(d) `pdu_encode`：`AsciiToG7bit` 无输入上限，超长/转义文本栈溢出 `sms_text_7bit[320]` →
    输入 `> SMS_MAX_7BIT_TEXT_LENGTH` 提前 return -1（pdu.c:325-328）；
(e) `pdu_decode`：补齐 sender TOA、sender 数据块、PID/DCS/时间戳整体、UDH 长度字节与 UDH
    本体、用户数据长度的边界检查；sender_number_length==0 时 alphanumeric 分支直写 NUL
    （修复 `sender_len1-1` 下标 -1）；`sms_pid_start` 改用已校验的 sender_number_length；
    `ceil(x*1.0/2)` 浮点换算改 `(x+1)/2`（pdu.c:348-390、393-398、405-407）；
(f) `G7bitToAscii`：末尾 ESC 无界读 `buffer[i+1]`、memmove 多搬 1 字节（读越界 1 字节）→
    对齐 libqmodem 版（末尾 ESC 收缩返回、索引 `& 0x7F`、memmove 长度 -i-2 且带 i+2 边界）
    （pdu.c:151-163）；
(g) S1 同款尾字节修复（pdu.c:82-89）；
(h) `pdu_decode` 时间戳 `struct tm` 补 `= {0}`（pdu.c:382）。

## S4 sms-tool_q/src/sms_main.c（已修）

(a) 高危：recv 的 CMGL PDU 行 hex 解析 `for(i=0;i<l;i+=2) pdu[i/2]=…` 无长度上限，模组返回
    超长行（buf 1024）可写越界 `pdu[256]` → 加 `l/2 > sizeof(pdu)` 拒绝（sms_main.c:356-360）；
(b) 高危：ussd 解析 `sscanf "%[^\"]"` 无宽度限制（两处：CUSD 首行与 multiline 续行），模组
    输出可栈溢出 `ussd_buf[320]` → 改 `%255[^\"]`（sms_main.c:563、697）；
(c) 高危：ussd UCS-2 通路 `utf_pos += ucs2_to_utf8(...)` 未检查负返回值，遇代理区/非法码点
    返回负数造成 `ussd_txt[负下标]` 写 → 检查 `len < 0` 与上界后 break（sms_main.c:688-693）；
(d) 高危链：`send` 消息 >160 字符仅告警不退出，叠加 (S3d) 旧库无上限会栈溢出 → 告警后
    `exit(2)`（sms_main.c:197-200）；`pdu_encode` 失败仍继续读未初始化 `pdu[0]` → return 1
    （sms_main.c:254-258）；
(e) recv `idx[1024]` 无 count 上界（+CMGL 行数模组可控）→ 超界 break（sms_main.c:322-326）；
(f) ussd 7-bit 解码 `l == sizeof(ussd_txt)` 时字符串未 NUL 终结即 printf → 越界钳位
    （sms_main.c:671-673）；
(g) DCS case 15 `lower & 0x4 == 0` 运算符优先级恒真 → `(lower & 0x4) == 0`
    （sms_main.c:629）；
(h) setserial `tcgetattr` 失败后继续使用未初始化 `t` → 提前 return（sms_main.c:53-58）；
(i) 错误前缀差一：`+CME ERROR:`/`+CMS ERROR:` 为 11 字符，`buf+12` 吞掉首字符 → `buf+11`
    （sms_main.c:481、554）。

## 仅记录（中低危，未改动）

- R1 pdu_decoder.c `sms_decode()`：stdin 逐字符入 `buffer[516]` 无上界，超长输入行可栈溢出；
  属本地独立小工具（非网络面），建议后续加界或以 extlib 重写替代（另有 agent 在做独立实现）。
- R2 sms_main.c recv：hex 解析不校验非法字符（char_to_hex 返回 -1 静默产出垃圾字节），现依赖
  S3e 边界兜底；可改为拒绝该条消息。
- R3 sms_main.c recv：`fgets` 读 PDU 行失败仅告警，继续用上一轮 buf（陈旧数据），建议 continue。
- R4 cmdstr[100]：非 raw ussd 的 `AT+CUSD=1,"<hex>"` 经 snprintf 截断，超长 CUSD 发送不完整
  （安全无虞，功能受限）。
- R5 signal handler `timeout()` 内调用 `fprintf/exit` 非 async-signal-safe；`fputs` 系列返回值
  未检查（串口写失败静默）。均为沿用多年的上游模式，风险低。
- R6 delete "all" 固定遍历 0..49 未按实际存储容量；仅功能限制。

## 验证汇总

- 语法：`gcc -fsyntax-only -Wall -Wextra`（libqmodem 侧含 `-Werror -std=gnu99`，对其 Makefile
  等价）通过：libqmodem-sms/src/pdu.c、src/ucs2_to_utf8.c；sms-tool_q/src/sms_main.c、
  pdu_lib/pdu.c、pdu_lib/pdu_decoder.c、pdu_lib/ucs2_to_utf8.c。无新增警告（残留均为既有：
  signedness、char 下标、未用变量等）。
- 差分：EncodePDUMessage 两副本 n=0..64 对照 rust pack_septets 等价实现 0 差异。
- 回归：libqmodem tests/test_ucs2_parts PASS；pdu_decoder 以 Makefile 内置 UCS-2 向量解码正确
  （中文短信输出正常）。
- ASan：两版 pdu_decode 各 200,000 轮随机/截断 PDU 模糊，0 报告。
- 换行：4 个改动文件均为 LF。diff 量：+94/-28，逐处人工复核。
- system/popen：sms-tool_q 目录内 grep 为 0（无该类调用）。
