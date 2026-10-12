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
| `ubus_ffi.rs` | `ubus` feature：libubus FFI 声明 + 集成说明骨架，默认不编译、不影响默认构建 |

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
- JSON 服务集成：临时 unix socket 上 sendat/list/subscribe 全流程。

主线程无长阻塞调用：`TtyPort` 用 O_NONBLOCK + 短睡眠轮询读（epoll/AsyncFd 加固见 journal 后续计划）。

## Feature 矩阵

| feature | 默认 | 说明 |
|---|---|---|
| `service` | ✔ | Unix socket JSON 服务 |
| `tty` | ✘ | 真实串口（libc/termios），无串口的机器同样可编译 |
| `ubus` | ✘ | libubus FFI 骨架；交叉编译 OpenWrt 时开启（见 `ubus_ffi.rs` 文档） |
