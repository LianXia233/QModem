# tom_modem 重构 Journal（agent-tom-modem）

日期：2026-10-12
范围：仅 `application/tom_modem/**`
验证基线：本机 gcc（TTY-only 模式全量链接 + /tmp stub 对比）

## 修复清单

### 1.【重复-中】operations.c：sms_read / sms_read_unread 抽公共实现
- 新增 `static int sms_read_impl(PROFILE_T *profile, void *transport_ptr, const char *read_sms_cmd, const char *label)`（operations.c:74），两处约 70 行逐字重复的解析/JSON 输出逻辑合并。
- `sms_read` → `sms_read_impl(..., READ_ALL_SMS, "SMS")`；`sms_read_unread` → `sms_read_impl(..., READ_UNREAD_SMS, "unread SMS")`。
- dbg 文案通过 label 参数拼回原句（"Error reading SMS/unread SMS"、"Read SMS/unread SMS success"），日志逐字节一致。

### 2.【重复-中】utils.c：match_option / match_operation 表驱动
- 新增 `name_map_t {short_name, long_name, value}` 与静态表 `g_option_map`（14 项）、`g_operation_map`（含 `#ifdef USE_SEMAPHORE` 的 cleanup 项）。
- 公共循环匹配 `match_name_map()`；match 结果与错误语义不变：option 未命中返回 -1；operation 空串返回 SUCCESS、未命中返回 INVALID_PARAM。

### 3.【C9-残留】main.c parse_user_input 返回值 + usage() 去 exit
- 复核确认：main.c 原先**未**处理 `parse_user_input` 返回值（main.c:253 直接丢弃）。
- main() 现检查返回值，非 SUCCESS 直接 `return result`。
- `usage()`（utils.c）内部 `exit(-1)` 已移除，只打印帮助并 `return INVALID_PARAM`，由调用方（parse_user_input / main）统一退出。
- 附带修复：`-o <非法操作>` 此前 `match_operation` 返回 3（=SMS_READ_OP 别名）被静默当作 sms_read 执行；现 parse 阶段即拦截，打印 "Invalid operation" + usage 并返回 INVALID_PARAM。

### 4.【C14-中残留】ttydevice.c tty_read_keyword
- 贪婪读：原 `start_time = time(NULL)` 在每次有新数据时重置导致永不超时；现 start_time 作为**绝对 deadline 锚点不再重置**，无论是否有新数据，总时长超过 `profile->timeout` 必返回（TIMEOUT_WAITING_NEWLINE）。
- `#ifdef EARLY_RETURN` 段删除：该宏在仓库内无任何编译路径注入（Makefile 仅有未接线的 CONFIG 符号），且把 read_flag 当计数器（>500 才触发）属死逻辑。
- 外部行为三态不变：关键字命中 SUCCESS / 关键词不匹配 KEYWORD_NOT_MATCH / 超时 TIMEOUT_WAITING_NEWLINE；无任何数据仍为 COMM_ERROR。

### 5.【低】modem_types.h 枚举冲突
- grep 确认调用链：ubus_at_daemon 不解析 tom_modem 退出码数值；shell/C 调用方（modem_util.sh、qmodem_smsd、sms_forwarder）仅区分 0/非 0。
- tom_modem 侧存在真实暴露面（`profile->op = match_operation(...)` 将错误码写入操作码空间），故将 `INVALID_PARAM` 从 3 移至独立命名空间值 **200**；其余错误码显式保留旧值（KEYWORD_NOT_MATCH=1、TIMEOUT_WAITING_NEWLINE=2、INVALID_HEX=4、UNKNOWN_ERROR=5、BUFFER_OVERFLOW=6、SEND_SMS_FAILED=7），所有 switch 使用点（run_op 等）均按常量比较，行为兼容。

## 验证记录

1. **语法检查**：`gcc -fsyntax-only -Wall` 对 main.c / utils.c / operations.c / ttydevice.c 全部通过（0 警告）。
2. **全量链接**：TTY-only 模式（main/utils/operations/transport/ttydevice + libqmodem-sms 的 pdu.c/ucs2_to_utf8.c）链接成功；仅 transport.c 存在 5 条既有的 -Wswitch 警告（未修改文件，TTY-only 分支固有）。
3. **重构前后输出对比**（/tmp stub 测试，stub transport_send_at_with_response/pdu_decode/ucs2_to_utf8，debug 开启捕获全部日志路径）：
   - sms_read（含 1 条解码失败 → COMM_ERROR 路径）：stdout/stderr diff 均 SAME，返回码一致（-1）。
   - sms_read_unread（正常解码 → JSON 输出路径）：stdout/stderr diff 均 SAME，返回码一致（0）。
   - 测试文件已按要求用后删除。
4. **退出码行为对比**（新旧二进制 A/B）：
   | 场景 | 旧 | 新 | 说明 |
   |---|---|---|---|
   | `-o bogus` | 255（静默当 sms_read 执行） | 200（parse 拦截 + usage） | 修复枚举别名 bug |
   | 未知选项 | 255（usage 内 exit(-1)） | 200（INVALID_PARAM） | 退出码语义统一 |
   | 正常路径设备错误 | 255 | 255 | 不变 |
   - 外部调用方均只判 0/非 0，退出码 255→200 的变化兼容。

## 影响面与风险
- 退出码 3→200（INVALID_PARAM 场景）、255→200（参数错误场景）：已核对全部仓内调用方（仅判成败），无数值依赖。
- greedy_read 不再无限续期：长输出场景现在受总超时约束（原设计意图即超时保护），若上层需要更长窗口应显式调大 `-t`。
- 删除 EARLY_RETURN 死代码后，Makefile 中未接线的 `PACKAGE_tom_modem_EARLY_RETURN` 配置符号成为空操作（原本即未注入编译）。
