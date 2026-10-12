# journal — modem_scan C 最终加固（agent-scan-c-final）

日期：2026-10-12
范围：仅 `application/modem_scan/src/{modem_scanc.c, modem_scand.c}`

## 变更清单

### modem_scanc.c
1. **write short-write 处理**（send_request）：原实现单次 `write(fd, line, strlen(line))`，短写时请求被截断且换行丢失，daemon 端可能收到不完整命令。改为循环写：请求体按 `sent` 偏移补写至完成，换行单独循环写，`EINTR` 重试，其余错误打印 `strerror` 并返回 1。
2. **SIGPIPE 屏蔽**：`main()` 入口 `signal(SIGPIPE, SIG_IGN)`。此前 daemon 提前关闭连接时客户端会被 SIGPIPE 直接杀死，无错误输出。
3. **连接失败错误信息区分**：重试耗尽后按 errno 分类——`ENOENT/ECONNREFUSED` 提示 "modem_scand not running"，`EACCES/EPERM` 提示权限不足（对应 0660 socket 的组权限问题），其余走通用 connect 错误；均带 socket 路径。`socket()` 失败的 `perror` 一并改为 `strerror` 格式统一。

### modem_scand.c
4. **find_slot_section 引号剥离健壮化（原 396-402 行）**：旧代码从 `uci show` 文本解析 value 并手工剥引号，`uci show` 对含引号值会重新加引号/转义（`'"'"'`），导致解析错位误匹配。调用处（get_slot_option 等 3 处）均只传普通 slot 串，允许改写：现仅解析行 KEY 部分（`qmodem.<sec>.slot=` 截断取节名），value 改用 `uci -q get qmodem.<sec>.slot` 取原始未加引号值再比较。
5. **JSON 拼接端口白名单（原 886 行）**：`probe_port_thread` 用 snprintf 把 `ctx->port` 直接拼进 ubus JSON `{ "at_port": "%s" }`，无转义。新增 `valid_port_name()`：白名单 `[A-Za-z0-9_/]`、非空、≤64 字符；不合规端口跳过探测并 `LOG_L_WARN` 记录端口名（来源为 /dev 目录枚举，仅作纵深防御）。
6. **socket 权限回退路径日志完整性复核（apply_socket_permissions）**：
   - 组不存在回退分支：补充 `errno=%d` 与 socket 路径，明确提示 "only root clients will be able to connect"。
   - chown/chmod 改为 else-if 链：任一失败记 LOG_ERR，全部成功补一条 `root:<gid> mode 0660` 的 LOG_INFO，便于核对实际生效属主（旧实现 chown 失败后仍执行 chmod，日志无法判断最终状态）。

## 验证
- `gcc -fsyntax-only -Wall -Wextra -I/tmp/qstub -I src modem_scand.c` → PASS（SCAND-OK，无警告；json-c 用 /tmp/qstub 头）
- `gcc -fsyntax-only -Wall -Wextra modem_scanc.c` → PASS（SCANC-OK）
- 功能自测：编译 modem_scanc 连接不存在 socket → `modem_scand not running (socket ...: No such file or directory)`，exit=2（ENOENT 分支生效）；本地 echo UNIX socket server 端到端 → `echo:status` 应答正常，exit=0（写循环/换行/读回放路径生效）。
- LF 检查：两文件 0 个 CR；最小 diff，未触碰范围外文件。

## 遗留
- `uci show` 仍用于枚举节名（KEY 部分不含引号转义问题），如需彻底零解析可换 `uci -q get qmodem.@modem[i]` 但稳定性更差，维持现状。
