# journal-rust-core

> 由 agent-rust-core 维护。

日期：2026-10-12
范围：`rust/`（新建 workspace，仅新增文件）+ 本 journal。

## 1. 交付物

| 路径 | 内容 |
|---|---|
| `rust/Cargo.toml` | workspace：`members = ["qmodem-pdu", "qmodem-atd"]`，workspace 统一依赖版本（tokio 1 / serde / serde_json / thiserror 2 / libc gated） |
| `rust/qmodem-pdu` | 3GPP TS 23.038 PDU 编解码 lib，依赖仅 thiserror，14 个测试 |
| `rust/qmodem-atd` | tokio 异步 AT 守护进程（bin+lib），8 个集成测试 + 2 个 transport 单测 |
| `rust/README.md`、两个 crate README | 设计说明 + C→Rust 修复对照（C 文件:行号） |

约束遵守：除本 journal 外未触碰 `rust/` 以外的任何路径。

## 2. qmodem-pdu：C→Rust 修复对照

C 源：`application/libqmodem-sms/src/pdu.c`

| # | C 缺陷 | 位置 | Rust 修复 |
|---|---|---|---|
| P1 | 发送方地址/SMSC 字段越界读，`sms_deliver_start` 推进无覆盖校验 | pdu.c:548-577 | 全部字段 `get()` 边界检查，越界 → `PduError::Truncated`；截断穷举测试（每前缀长度断言 Err 不 panic） |
| P2 | 数字校验恒假 `c < '0' && c > '9'`（应为 `||`） | pdu.c:335 | `encode_sms_submit` 严格 ASCII 数字校验 → `PduError::InvalidNumber` |
| P3 | 固定 `sms_text_7bit[320]` 栈缓冲，`AsciiToG7bit` 对多字节 UTF-8 展开可写越界；仅事后判长 | pdu.c:418-421, 301-318 | 上限前置：7-bit ≤160 septets（转义字符计 2）、UCS-2 ≤70 UTF-16 units、8-bit ≤140 octets，超限 → `TooLong` |
| P4 | UCS-2 解码原样 memcpy 到输出，未转码 | pdu.c:616-617 | UTF-16BE → UTF-8，含代理对配对/孤立代理校验 |
| P5 | `DecodePDUMessage_GSM_7bit` 余数路径 `buffer[i-1]` 在 i=0 时为 -1 下标；位运算跨字节对齐易错 | pdu.c:114-115 | 累加器式 pack/unpack（LSB-first），无下标运算；与 dreamfabric "hellohello" 公开向量 E8 32 9B FD 46 97 D9 EC 37 逐字节断言 |
| P6 | `G7bitToAscii` 原地 memmove 压缩转义序列，边界算术脆弱 | pdu.c:253-272 | 纯函数 septets→String，escape 消耗下一个 septet |
| P7 | 时区半字节完全未解析（丢弃偏移信息） | pdu.c:569-575 | `Timestamp::timezone_quarters`：低半字节 = 15 分钟步长数，bit 0x80 = 负偏移；正负往返测试 |

SMSC 长度按八位组（含 TOA）、TP-DA 长度按数字位、SCTS 半八位组“十位在低 nibble”等字段语义按 TS 24.011/23.040/23.038 实现，并以 3 个手工交叉验证向量锁定（UCS-2「你好」/ 7-bit "hellohello" / UCS-2+UDH ref 0xAB part 2/3）。

## 3. qmodem-atd：C→Rust 架构对照

C 源：`application/ubus_at_daemon/src/`

| # | C 缺陷/模式 | 位置 | Rust 设计 |
|---|---|---|---|
| A1 | 每端口 pthread reader + 跨线程共享 `port->fd`（write 路径与 reader 并发用 fd） | at_handler.c:193-195, 307-320 | `AtPort` actor `tokio::select!` 循环独占 `SerialPort`（Box<dyn>），所有权消除 UAF 根因 |
| A2 | 单槽 `current_response` + `waiting_for_response`，并发 sendat 互踩/串响应 | at_handler.c:138-158, 322-334 | per-port mpsc 命令队列串行执行，每请求私有 oneshot；并发测试断言两请求各拿各的响应、内容零错配 |
| A3 | `CLOCK_REALTIME` 绝对超时，墙钟跳变即提前/滞后 | at_handler.c:208-215 | `tokio::time::sleep` 单调时钟；超时返回已收部分行 + `Timeout`（测试断言耗时贴 deadline） |
| A4 | line_events 固定环形队列 + drop_count + notify_pipe | line_events.c, ubus_at_daemon.h:78-89 | `broadcast::channel`，`AtLine{port,line,seq,epoch}` 保留 seq/restart_epoch 语义，滞后经 `RecvError::Lagged` 上报 |
| A5 | 端口关闭后等待者挂在无人 signal 的 condvar | reader_thread_func 退出路径 | actor 退出时 drain：pending/队列中所有请求回 `AtError::PortClosed`，不 panic 不挂死（测试覆盖） |
| A6 | 端标志匹配规则（精确/前缀/冒号特例）散落手写 | at_handler.c:79-121, 28-77 | `Pending::matches_end_flag` 移植同规则；默认 5 个 end flags 一致 |
| A7 | ubus 强依赖，host 编译需 QMODEM_HOST_TEST 垫片 | ubus_at_daemon.h:15-23 | `feature = "service"`（默认）Unix socket JSON 行协议做测试接口；`feature = "ubus"` 预留 FFI 骨架（只声明不引用，默认构建不受影响） |

echo 抑制策略与 C 有意不同：C 按任意 "AT" 前缀丢行（at_handler.c:354，会误杀 "AT+..." 开头的正常响应行），Rust 改为精确匹配最近写入的命令文本。

## 4. 验证记录（本机 cargo 1.93.0）

```
$ cd rust && cargo test --workspace
test result: ok. 2 passed; 0 failed   (transport 单测)
test result: ok. 8 passed; 0 failed   (atd 集成测试)
test result: ok. 14 passed; 0 failed  (qmodem-pdu)
（+ 3 组 doc-test 空跑，0 failed）—— 合计 24 通过，0 失败

$ cargo build --release
Finished `release` profile [optimized]
target/release/qmodem-atd          1,322,696 B（未 strip）
target/release/libqmodem_pdu.rlib     95,882 B

$ cargo clippy --workspace --all-targets
0 warning, 0 error

$ cargo check -p qmodem-atd --features tty,ubus   → ok
$ cargo check -p qmodem-atd --no-default-features → ok
```

feature 矩阵：`service`（默认）、`tty`（libc/termios，无串口可编译）、`ubus`（FFI 骨架）均可独立编译。

## 5. 后续接入计划

1. **ubus FFI**（`--features ubus`）：按 `ubus_ffi.rs` 中原型实现 `sendat/list/event_callback*` 方法表（对齐 ubus_at_daemon.h:266-293），handler 经无阻塞桥接投递到 AtPort 队列；OpenWrt SDK 交叉编译时链接 libubus/libubox。
2. **vendor 规则引擎 / smsd**：`qmodem-pdu::Udh` 复用为分片重组核心；在 atd 上层加消息组装/去重/持久化 actor，替代 `qmodem_smsd` 轮询模型。
3. **TtyPort 加固**：O_NONBLOCK 轮询 → `AsyncFd` epoll；端口监控线程改周期任务（对齐 port_manager.c 的重连逻辑）。
4. **配置**：对齐 config_loader.c 的端口/回调 JSON，事件回调以 `tokio::process` 参数数组执行（消除 C event_callback.c 的命令注入面，C 侧已在 journal-c-core.md 中同步修复）。
