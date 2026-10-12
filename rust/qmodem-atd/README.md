# qmodem-atd

tokio 全异步 AT 命令守护进程，对应 C 版 `application/ubus_at_daemon` 的异步重构骨架。

## 架构：C 线程模型 → tokio actor

C 版为每端口一个 pthread reader + 条件变量 + **单槽** `current_response`（`at_handler.c:138-158`），并发调用会互踩响应；写路径跨线程共享 `port->fd`（`at_handler.c:193-195`）存在 UAF 根因；超时用 `CLOCK_REALTIME`（`at_handler.c:208-215`）受墙钟跳变影响。

Rust 版：

```
JSON/ubus 客户端
   │ mpsc（per-port 命令队列）
   ▼
AtPort actor 任务（tokio::select! 循环，独占 SerialPort 所有权）
   ├── 串行执行队列中的 AT 命令，逐个回发 oneshot 响应
   ├── tokio::time::sleep 单调超时；超时返回已收部分响应 + Timeout
   └── broadcast channel 发布 AtLine{port, line, seq, epoch}（URC/数据行）
```

- **消除互踩**：每个请求一个 oneshot，队列串行执行，响应不会错配。
- **消除 UAF**：fd 归 actor 独占，无跨线程裸共享；关闭后请求得到 `AtError::PortClosed`（drain 队列中所有等待者，不会挂死——C 版等待者挂在无人 signal 的 condvar 上）。
- **单调时钟**：`tokio::time`。
- **URC 广播**：`broadcast` channel + `seq`/`epoch` 计数，对应 C `at_line_event_t`（`ubus_at_daemon.h:66-89`）的语义；订阅滞后通过 `RecvError::Lagged` 上报，替代 C 版 `drop_count` 环形队列。

## 模块

| 模块 | 说明 |
|---|---|
| `transport.rs` | `SerialPort` trait（手写 boxed future，免 async-trait 依赖）；`LoopbackPort` 测试替身（可编程响应/URC 注入/EOF）；`tty` feature 下 `TtyPort`（libc termios 封装，O_NONBLOCK 非阻塞读） |
| `port.rs` | `spawn_port` actor、`AtHandle::send_at/subscribe/close`、端标志匹配（移植 C `check_end_flags` 精确/前缀规则，默认标志 OK/ERROR/+CMS ERROR:/+CME ERROR:/NO CARRIER） |
| `registry.rs` | 端口注册表（替代 C 全局 `g_daemon_ctx.ports` 链表） |
| `service.rs` | `service` feature（默认开）：Unix socket JSON 行协议 |
| `ubus_proto.rs` | 纯 Rust blobmsg 编解码（libubox 兼容 wire format，TLV 往返/畸形输入单测）+ `mock` transport（`MockUbusServer` 分发 `sendat`/`list`，无 libubus 环境同语义） |
| `ubus_ffi.rs` | `ubus` feature：手写 libubus/libubox 最小 FFI（connect/lookup/lookup_id/add_object/send_reply/blobmsg_parse）、`qmodem.atd` 对象方法表注册、`sendat` handler；默认不编译、不影响默认构建 |

## ubus FFI 与 mock transport（同语义双路径）

生产传输是 ubus（`application/ubus_at_daemon` 的 `sendat` 方法，`ubus_at_daemon.h:269-293`）。
主机/CI 没有 libubus，因此拆成两层，**共享同一套请求解码/响应编码代码**：

```
                     ┌─ feature=ubus ──> ubus_ffi.rs：C handler 解码 blob_attr*
ubus 调 sendat ──┐   │                   （复用 ubus_proto::decode_msg +
                 ├──<┤                    mock::encode_sendat_result）
                 │   └─ 默认构建 ────> ubus_proto::mock：MockUbusServer.call("sendat")
blobmsg 请求体 ──┘                        （集成测试走此路径，语义相同）
```

关键设计：libubox 的 blob_attr wire format（4 字节对齐 TLV、`type<<28|len` 头、
blobmsg 名字头、native endian）由 `ubus_proto` **逐字节实现**，单测含字节级
golden 断言；FFI handler 直接用 Rust 解码器切 C 传入的 `blob_attr`，
`tests/ubus_ffi_smoke.rs` 再用 C stub 的 `blobmsg_parse` 反向验证 Rust 编码——
两侧格式一致性有测试锁定。

### 无 libubus 主机（默认构建）

```sh
cargo test -p qmodem-atd --test ubus_mock   # 「ubus 调 sendat」全流程，走 mock
```

### 有 libubus 的交叉编译 / 主机 stub 验证

FFI 声明为手写 extern（无 bindgen），语法检查不需要任何头文件：

```sh
cargo check -p qmodem-atd --features ubus   # syntax check，无 libubus 也能过
```

真链接（OpenWrt SDK sysroot）：

```sh
cargo build --release --features ubus \
  --config 'target.<triplet>.linker="<staging>/...-gcc"' \
  --config 'rustflags=["-L", "native=<staging>/usr/lib", "-lubus", "-lubox"]'
```

主机冒烟（/tmp 最小 stub 共享库，镜像声明签名 + 按 libubox 语义实现
`blobmsg_parse`，stub 本体不入库，见 journal-ffi-ubus.md 附录）：

```sh
RUSTFLAGS="-L native=/tmp/stub-ubus -lubus -lubox" \
LD_LIBRARY_PATH=/tmp/stub-ubus \
cargo test -p qmodem-atd --features ubus --test ubus_ffi_smoke
```

`register_atd_object` 采用 C 惯用的 container_of 嵌入模式
（`AtdUbusObject` 首字段为 `ubus_object`，`offset_of!` 还原），C 可达内存
（方法表/名字串/对象本体）注册后刻意泄漏——libubus 持有指针至连接关闭，
与 C daemon 的静态表同型。

## Unix socket JSON 行协议（ubus FFI 前的测试接口）

```
{"cmd":"sendat","port":"/dev/ttyUSB2","at":"AT+CSQ","timeout_ms":3000,"end_flag":"OK"}
  → {"status":"ok"|"timeout","lines":[...],"end_flag":"OK","elapsed_ms":12}
{"cmd":"list"} → {"status":"ok","ports":["/dev/ttyUSB2",...]}
{"cmd":"subscribe"} → 持续推送 {"event":"line","port":...,"line":...,"seq":...,"epoch":...}
```

说明：该协议是本仓库 luci/app 侧联调用的传输，后续接入 OpenWrt 时由 `feature = "ubus"` 的 ubus 对象（sendat/list/event_callback 系列，对应 `ubus_at_daemon.h:266-293` 的方法表）替代；届时 JSON 协议保留为无 ubus 环境（开发机/CI）的测试通道。

## 测试

`cargo test -p qmodem-atd`：

- 顺序 sendat（echo 抑制、end flag 匹配）；
- 并发 sendat 排队：两任务同时发，各拿各的响应、内容不错配（单槽互踩回归）；
- 超时路径：端口不响应 → `Timeout` + 已收部分响应，且实际耗时贴合 deadline（单调时钟）；
- 自定义 end flag（`>` SMS 提示符类）；
- URC 广播订阅（seq/epoch 元数据）；
- 端口关闭 → `PortClosed`，不 panic 不挂死；
- JSON 服务集成：临时 unix socket 上 sendat/list/subscribe 全流程；
- ubus mock 集成（`--test ubus_mock`）：blobmsg 请求编码 → `MockUbusServer` 分发 → 响应解码，覆盖 ok/timeout/unknown port/invalid argument/unknown method 与 wire format 结构断言；
- FFI 冒烟（`--features ubus --test ubus_ffi_smoke`，stub 链接）：声明层符号解析、`register_atd_object` 方法表布局、Rust 编码 blob 过 C `blobmsg_parse` 往返。

微基准（sendat 往返延迟 / 并发吞吐 / 32 订阅广播 / JSON 1KB）见 [docs/refactor/bench-atd.md](../../docs/refactor/bench-atd.md)，实现于 `tests/bench_atd.rs`。

主线程无长阻塞调用：`TtyPort` 用 O_NONBLOCK + 短睡眠轮询读（epoll/AsyncFd 加固见 journal 后续计划）。

## Feature 矩阵

| feature | 默认 | 说明 |
|---|---|---|
| `service` | ✔ | Unix socket JSON 服务 |
| `tty` | ✘ | 真实串口（libc/termios），无串口的机器同样可编译 |
| `ubus` | ✘ | libubus FFI 实装 + mock transport（见上方「ubus FFI 与 mock transport」章节） |
