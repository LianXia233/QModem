# C 层核心修复日志（journal-c-core）

> 本日志由 agent-c-core 维护。覆盖范围：application/ubus_at_daemon/**、application/tom_modem/**、application/libqmodem-sms/**（仅限本代理互斥文件域）。

## C1 sendat 早退路径返回未初始化 response

【问题】
`send_at_command_with_response()` 在 `open_at_port()` 失败时于 `memset(response)` 之前 `return -1`；调用方 `main.c` 的 `ubus_sendat_method()` 在 `result != 0` 分支仍读取 `at_response_t`（约 32KB 栈对象，含 `response[32768]`），构成未初始化内存读取。

【修复方式】
在函数入口处立即 `memset(response, 0, sizeof(at_response_t))`（`response` 非空时），保证所有退出路径返回的缓冲区均已清零；原初始化逻辑保留（幂等）。

【验证结果】
`gcc -fsyntax-only -Wall` 通过，无新增警告；diff 复核确认所有 `return -1` 路径均已被入口 memset 覆盖。

【涉及文件:行号】
- application/ubus_at_daemon/src/at_handler.c:127-129（入口 memset）
- application/ubus_at_daemon/src/at_handler.c:123-146（send_at_command_with_response）

## C2 事件回调 root 命令注入 + 僵尸进程

【问题】
`process_incoming_data()` 将模组串口输出 `data`（远程可控）通过 `snprintf("%s \"%s\"", script, data)` 拼接后交给 `system()` 执行，存在 root 命令注入；且父进程不 `waitpid`，产生僵尸进程。

【修复方式】
- 子进程改用 `execl("/bin/sh", "sh", "-c", callback->callback_script, "qmodem-event", data, (char *)NULL)`：脚本路径放在 `-c` 字符串，`data` 仅作为位置参数 `$2` 传入，绝不拼进 `-c` 字符串；`execl` 失败后 `_exit(127)`。范式对齐 `application/modem_scan/src/modem_scand.c:782`。
- 防僵尸采用 `signal(SIGCHLD, SIG_IGN)` 方案：在 `main()` 初始化处设置一次，子进程退出由内核自动回收。

【验证结果】
语法检查通过；diff 复核确认 `data` 不再进入任何 shell 字符串拼接，`command` 缓冲区已删除。

【涉及文件:行号】
- application/ubus_at_daemon/src/event_callback.c:83-97（execl 替换 system）
- application/ubus_at_daemon/src/main.c:641（signal(SIGCHLD, SIG_IGN)）

## C3 tty_open_device fd 泄漏与悬空 FILE*

【问题】
原实现三次 `open()` 互相覆盖 `fds->tty_fd`（前两次的 fd 泄漏）；`close(fd)` 后又对同一 fd 重新 `open` 并用两个 `fdopen`（fdi/fdo）指向同一 fd，`FILE*` 与 fd 所有权混乱，错误路径资源不清理。

【修复方式】
重写为单一路径 + `goto err` 统一清理：
- 只 `open` 一次（`O_RDWR|O_NOCTTY|O_NONBLOCK`），`tty_set_device` 失败即 goto err；
- `tcflush` 后对原 fd `fdopen` 得到 `fdi`（读方向），`dup()` 出独立 fd 再 `fdopen` 得到 `fdo`（写方向），保证每个 fd 只归属一个 `FILE*`；
- 错误路径 `fclose`/`close` 全部已分配资源并复位 `fds` 字段。
对外语义不变：成功后 `tty_fd`/`fdi`/`fdo` 三者均有效（transport.c 关闭路径继续使用 `tty_fd`）。

【验证结果】
语法检查通过；diff 复核确认所有失败分支均经 `err:` 释放资源，无 fd 双重归属。

【涉及文件:行号】
- application/tom_modem/src/ttydevice.c:80-151（tty_open_device 重写）

## C5 pdu_decode 越界读与 sender 长度 0 下标 -1 写

【问题】
(a) 时间戳 7 字节（`sms_pid_start+2` 起）在长度边界检查之前读取，恶意 PDU 可导致越界读；sender 的 TOA 字节、sender 数据、UCS2 用户数据 memcpy 同样缺少统一边界检查。
(b) `sender_number_length==0` 且 TOA 为 alphanumeric 时，`sender_len1-1` 为 -1 传入 `G7bitToAscii`，且 `output_sender_phone_number[-1]=0` 构成下标 -1 写。

【修复方式】
- 每步读 buffer 前先检查 `offset + 需要长度 <= buffer_length`：sender TOA（`sms_deliver_start+2`）、sender 数据块、PID/DCS/时间戳整体（`sms_pid_start+2+7`）、用户数据长度（`sms_start+1+output_sms_text_length`）；
- `sms_pid_start` 改用已读取并校验过的 `sender_number_length` 计算；
- 增加 `sender_number_length == 0` 提前分支：直接写 `output_sender_phone_number[0] = 0`，不再进入 `G7bitToAscii`。

【验证结果】
语法检查通过；diff 复核确认后续所有字段读取均先经过边界检查。

【涉及文件:行号】
- application/libqmodem-sms/src/pdu.c:562-581（边界检查 + sender==0 分支）
- application/libqmodem-sms/src/pdu.c:607-608（用户数据长度检查）

## C6 pdu_encode 恒假校验、未检查返回值与缓冲区溢出

【问题】
(a) `if (length < 0 && length >= 254)` 恒假（应为 `||`），SC 号编码失败/超长未拦截；
(b) `if (phone_number[i] < '0' && phone_number[i] > '9')` 恒假，号码完全不校验；
(c) 目标号码 `EncodePhoneNumber` 返回 -1 未检查，直接参与 `output_buffer_length` 累加导致负长度下标；
(d) `AsciiToG7bit` 写入 `sms_text_7bit[320]` 前未限制输入长度，超长输入（尤其转义字符 2 字节输出）造成栈缓冲区溢出。

【修复方式】
- (a)(b) `&&` 改 `||`；
- (c) 检查 `EncodePhoneNumber` 返回值，`length < 0` 即 `return -1`；
- (d) 在调用 `AsciiToG7bit` 前检查 `strlen(sms_text) > SMS_MAX_7BIT_TEXT_LENGTH` 即返回 -1（转义字符最坏 2 字节/字符，160 输入上限保证输出不超过 320 字节缓冲区），转换后原有的 `> SMS_MAX_7BIT_TEXT_LENGTH` 检查保留。

【验证结果】
语法检查通过；diff 复核四处修改逻辑正确。

【涉及文件:行号】
- application/libqmodem-sms/src/pdu.c:335（b）
- application/libqmodem-sms/src/pdu.c:381（a）
- application/libqmodem-sms/src/pdu.c:406-407（c）
- application/libqmodem-sms/src/pdu.c:420-426（d）

## C7 ubus_at_daemon 并发问题（关键路径）

【问题】
(a) `ubus_sendat_method` 持裸 `port` 指针执行耗时收发，并发 `close` 方法可 `destroy_port_instance` 直接 `free(port)`，构成 use-after-free；
(b) `parse_end_flags` 在 `response_mutex` 之外写 `port->expected_end_flags/num_end_flags`，与 reader 线程读取存在数据竞争；
(c) `pthread_cond_timedwait` 使用 `CLOCK_REALTIME` 初始化的条件变量配合基于系统时钟的绝对超时，系统时钟跳变会导致等待异常；
(d) `write` 短写返回 -1 的失败路径对 `waiting_for_response` 的清理未持 `response_mutex`，与 reader 线程存在竞态。

【修复方式】
(a) `at_port_instance_t` 新增 `int refcount` / `int pending_destroy`（`g_daemon_ctx.ports_mutex` 保护）；`main.c` 新增 `port_instance_acquire()`（在校验实例仍在 ports 链表内后递增引用）与 `port_instance_release()`；sendat 处理全程持引用，两个退出点均释放；`destroy_port_instance()` 在 `refcount>0` 时置 `pending_destroy` 并延迟销毁，由最后一次 release 触发真正销毁。
(b) `parse_end_flags` 调用移入调用点已有的 `response_mutex` 临界区内。
(c) `create_port_instance()` 用 `pthread_condattr_setclock(CLOCK_MONOTONIC)` 初始化 `response_cond`；等待处改用 `clock_gettime(CLOCK_MONOTONIC)` 计算绝对超时。
(d) write 失败路径对 `waiting_for_response = 0` 纳入 `response_mutex` 保护。

【验证结果】
语法检查通过；diff 复核确认 sendat 全部退出路径均 release、destroy 延迟逻辑闭环、condattr 与等待时钟一致。

【涉及文件:行号】
- application/ubus_at_daemon/src/ubus_at_daemon.h:157-159（refcount/pending_destroy 字段）
- application/ubus_at_daemon/src/main.c:5-31（acquire/release 助手）、240-241、255、318（持引用/释放）、641 另见 C2
- application/ubus_at_daemon/src/port_manager.c:63-69（CLOCK_MONOTONIC condattr）、83-93（延迟销毁）
- application/ubus_at_daemon/src/at_handler.c:150-155（b）、205-209（d）、216-217（c）

## 验证汇总

- 逐文件 `gcc -fsyntax-only -Wall` 全部通过：at_handler.c / event_callback.c / main.c / port_manager.c / ttydevice.c / pdu.c，由本次修改引入的错误为 0。
- libubus/libubox/json-c 本机缺失，使用 /tmp/qstub stub 头目录（未入仓库）完成语法检查；其余警告均为既有问题（如 `ubus_at_daemon.h` 中 static 声明未定义、`main.c:121` timeout 未使用）。
- 所有修改文件保持 LF 换行；diff 已逐处复核逻辑。
