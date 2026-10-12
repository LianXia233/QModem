# qmodem-atd 异步 AT 守护进程架构（异步核心 · 实验性）

> 状态：**实验性（experimental）**。`rust/qmodem-atd` 是 `application/ubus_at_daemon`（C 版）的并行可选项，目前**不替换任何现有组件的打包路径**。设计说明与修复对照的权威来源是 `rust/README.md` 与 `docs/refactor/journal-rust-core.md`，本文是面向开发者的架构导览。

## 1. 线程-actor 模型

`qmodem-atd` 基于 tokio 多线程运行时，核心结构是"**每端口一个 actor**"：

```
                Unix socket JSON 服务（feature "service"，默认开）
                     │  sendat / list / subscribe
                     ▼
              Registry（端口注册表，Arc 共享）
                     │  查找端口句柄
        ┌────────────┼────────────┐
        ▼            ▼            ▼
   ┌─────────┐  ┌─────────┐  ┌─────────┐
   │ AtPort  │  │ AtPort  │  │ AtPort  │   ← 每端口一个 actor 任务
   │ (ttyUSB0)  │ (ttyUSB1)  │ (ttyUSB2)  │      独占 Box<dyn SerialPort>
   │ mpsc 命令队列（串行执行）          │
   │ 每请求私有 oneshot 回应            │
   │ tokio::time::sleep 单调超时        │
   └────┬────┘  └────┬────┘  └────┬────┘
        └────────────┼────────────┘
                     ▼
          broadcast::channel（URC 广播，容量 256）
          AtLine { port, line, seq, epoch }
```

要点：

| 机制 | 实现 | 位置（rust/qmodem-atd/src/） |
|---|---|---|
| 端口所有权 | `Box<dyn SerialPort>` 由 actor 独占，读写路径零共享 | `transport.rs` / `port.rs` |
| 命令串行化 | per-port mpsc 队列；并发请求各自持有 oneshot，响应不串线 | `port.rs` |
| 超时 | `tokio::select!` + `tokio::time::sleep`（**单调时钟**），超时返回已收部分行 + `Timeout` | `port.rs` |
| URC 广播 | `tokio::sync::broadcast`，`AtLine{port,line,seq,epoch}` 保留 C 版 seq/重启 epoch 语义；订阅者滞后经 `RecvError::Lagged` 上报丢行数 | `service.rs` |
| 端口关闭 | actor 退出时 drain：所有 pending 与排队请求收到 `AtError::PortClosed`，不 panic 不挂死 | `port.rs` |
| 端标志匹配 | `Pending::matches_end_flag` 移植 C 版精确/前缀/冒号特例规则，默认 5 个 end flags 一致 | `port.rs` |
| echo 抑制 | **有意与 C 不同**：精确匹配最近写入的命令文本（C 版按任意 `AT` 前缀丢行，会误杀 `AT+...` 开头的正常响应行，见 journal-rust-core.md 第 3 节） | `port.rs` |

默认构建（`service` feature）不触碰串口与 libubus，可在开发机/CI 全绿编译测试；`--loopback NAME` 可在进程内起环回模组做冒烟测试，`--port PATH` 需 `--features tty`。

## 2. 与 C 版 ubus_at_daemon 的对照

C 源：`application/ubus_at_daemon/src/`。逐项"问题 → Rust 消除方式"素材取自 `docs/refactor/journal-rust-core.md` 第 3 节：

| # | C 问题 | C 位置 | Rust 设计如何消除 |
|---|---|---|---|
| A1 | 每端口 pthread reader，write 路径与 reader 并发使用同一 `port->fd` | at_handler.c:193-195, 307-320 | `AtPort` actor `tokio::select!` 循环独占 `SerialPort`，所有权模型直接消除 UAF 根因 |
| A2 | 单槽 `current_response` + `waiting_for_response`，并发 sendat 互踩、响应串线 | at_handler.c:138-158, 322-334 | per-port mpsc 命令队列串行执行，每请求私有 oneshot；并发测试断言两请求各拿各的响应、内容零错配 |
| A3 | `CLOCK_REALTIME` 绝对超时，墙钟跳变即提前/滞后 | at_handler.c:208-215 | `tokio::time::sleep` 单调时钟；超时返回已收部分行 + `Timeout`（测试断言耗时贴 deadline） |
| A4 | line_events 固定环形队列 + drop_count + notify_pipe | line_events.c, ubus_at_daemon.h:78-89 | `broadcast::channel` 保留 seq/restart_epoch 语义，滞后经 `RecvError::Lagged` 显式上报 |
| A5 | 端口关闭后等待者挂在无人 signal 的 condvar | reader_thread_func 退出路径 | actor 退出时 drain：所有等待请求回 `AtError::PortClosed`（测试覆盖） |
| A6 | 端标志匹配规则散落手写 | at_handler.c:79-121, 28-77 | 集中到 `Pending::matches_end_flag`，默认 end flags 与 C 一致 |
| A7 | ubus 强依赖，host 编译需 QMODEM_HOST_TEST 垫片 | ubus_at_daemon.h:15-23 | `feature = "service"`（默认）用 Unix socket JSON 行协议做测试接口；`feature = "ubus"` 预留 FFI 骨架，默认构建不受影响 |

C 侧的内存安全修复（未初始化 response、fd 泄漏、引用计数竞态等）见 `docs/refactor/journal-c-core.md`。

## 3. Unix socket JSON 测试协议

启动（需先 `cargo build --release`，见 `rust/README.md`）：

```sh
cd rust
cargo build --release
./target/release/qmodem-atd --socket /var/run/qmodem-atd.sock --loopback loop0
```

协议为**每行一个 JSON 对象**，共三个命令：`sendat`、`list`、`subscribe`（`subscribe` 只持续推送事件，不回应请求行）。

### sendat

```sh
curl --unix-socket /var/run/qmodem-atd.sock \
     -d '{"cmd":"sendat","port":"/dev/ttyUSB2","at":"AT+CSQ","timeout_ms":3000,"end_flag":"OK"}' \
     http://localhost/
```

响应：

```json
{"status":"ok","lines":["+CSQ: 25,99","OK"],"end_flag":"OK","elapsed_ms":12}
```

`status` 为 `ok` / `timeout` / `error`；`timeout` 时 `lines` 携带已收到的部分行；端口不存在为 `{"status":"error","error":"unknown port"}`。

### list

```sh
curl --unix-socket /var/run/qmodem-atd.sock \
     -d '{"cmd":"list"}' http://localhost/
# {"status":"ok","ports":["/dev/ttyUSB2","loop0"]}
```

### subscribe（URC 流）

```sh
curl -N --unix-socket /var/run/qmodem-atd.sock \
     -d '{"cmd":"subscribe"}' http://localhost/
# {"event":"line","port":"/dev/ttyUSB2","line":"+CMTI: \"SM\",3","seq":7,"epoch":1}
# 消费滞后时：{"event":"lagged","dropped":n}
```

## 4. 交叉编译（aarch64 / x86_64 musl）

默认 feature 集（`service`）的依赖只有 tokio / serde / serde_json / thiserror（外加 `tty` feature 门控的 libc/termios），**不链接任何系统 C 库**（无 libubus/libubox），因此可用 musl 目标产出全静态二进制：

```sh
rustup target add aarch64-unknown-linux-musl x86_64-unknown-linux-musl

# aarch64（路由器常见目标）
cargo build --release -p qmodem-atd --target aarch64-unknown-linux-musl
# x86_64
cargo build --release -p qmodem-atd --target x86_64-unknown-linux-musl

file target/aarch64-unknown-linux-musl/release/qmodem-atd
# statically linked → 可直接拷入固件运行
```

说明（与 `rust/README.md` 的降级路径一致）：

- `qmodem-pdu` 仅依赖 thiserror，极端场景可退化为零依赖（手写 Display impl）；
- `qmodem-atd` 依赖 tokio 的 `select!` 语义，设计上无法用 `std::thread` 等价替代，因此 tokio 是不可降级的最小外部 crate 集；
- 无 musl 工具链时可改用 `-Z build-std` 的 nightly 方式或 OpenWrt SDK 的 sysroot，产物等价。

## 5. ubus FFI 路线图

`src/ubus_ffi.rs` 已有 FFI 骨架（`ubus` feature，默认关，只声明不引用）。接入顺序（对齐 `rust/README.md` 与 journal-rust-core.md 第 5 节）：

1. **ubus 方法表桥接**：按 `ubus_at_daemon.h:266-293` 的方法表实现 `sendat` / `list` / `event_callback*` / `open` / `close` / `load_conf`；ubus handler 不阻塞，统一投递到 AtPort 的 mpsc 队列后由 oneshot 收结果。
2. **与 JSON 服务并列**：`service`（unix socket）继续作为测试接口保留，`ubus` 作为生产接口并列演进，最终替换 C 版 `ubus_at_daemon` 的打包路径。
3. **配置加载**：对齐 `config_loader.c` 的 JSON 配置（端口列表、波特率、事件回调脚本）；事件回调改用 `tokio::process` 参数数组执行，消除 C 版 event_callback.c 的命令注入面（C 侧已在 journal-c-core.md 同步修复）。
4. **交叉编译集成**：`--features ubus` 需链接 libubus/libubox，走 OpenWrt SDK sysroot（此时不再适用第 4 节的纯 musl 零系统库路径）。

后续更大的迁移（vendor 规则引擎、smsd、TtyPort AsyncFd 加固）见 `rust/README.md` 的"后续迁移计划"。
