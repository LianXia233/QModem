# QModem Rust 重构区

本目录是 QModem 核心组件向 Rust 迁移的落地区。当前包含两个 crate 的 Cargo workspace，
**外部 crate 依赖数为 0**（零依赖、单包交付）：

```
rust/
├── Cargo.toml            # workspace（无 [workspace.dependencies]）
├── qmodem-pdu/           # 3GPP TS 23.038 SMS PDU 编解码（lib，零依赖）
│   └── src/lib.rs        #   ← application/libqmodem-sms/src/pdu.c 的修复版重写
└── qmodem-atd/           # std::thread actor AT 守护进程（bin + lib，零依赖）
    ├── src/transport.rs  #   SerialPort trait(同步+读超时) / LoopbackPort / TtyPort(tty feature，内联 termios FFI)
    ├── src/port.rs       #   AtPort actor 线程：命令队列 + oneshot + 单调超时 + URC 广播
    ├── src/line.rs       #   AtLine 事件 + 自研 broadcast hub（Lagged/drop_count 语义）
    ├── src/json.rs       #   自研最小 JSON parse/serialize（含 \uXXXX 代理对/转义）
    ├── src/registry.rs   #   端口注册表
    ├── src/service.rs    #   Unix socket JSON 行协议（service feature，默认开；每连接一线程）
    ├── src/ubus_ffi.rs   #   libubus FFI 骨架（ubus feature，默认关）
    ├── src/main.rs       #   守护进程入口
    └── tests/            #   集成测试 + 零依赖守卫测试（zero_dep.rs）
```

## 依赖矩阵

| crate | 外部依赖 | 说明 |
|---|---|---|
| qmodem-pdu | 0 | `PduError` 手写 `Display`/`Error`（原 thiserror） |
| qmodem-atd | 0 | tokio → 每端口 `std::thread` actor；serde_json → `src/json.rs`；broadcast/oneshot → std mpsc + Condvar 自研；libc → 内联 `extern "C"` termios 声明（tty feature） |

全部 feature（`service`/`tty`/`ubus`）均不引入任何 crate；`Cargo.lock` 中无第三方包。

## 构建

```sh
cd rust
cargo test --workspace          # 37 个测试（原 24 个语义不变 + json/broadcast/零依赖新测试）
cargo build --release           # target/release/qmodem-atd
cargo clippy --workspace        # 0 warning
cargo test -p qmodem-atd --no-default-features
cargo check -p qmodem-atd --features tty,ubus
```

## 体积对比（cargo build --release，未 strip，本机实测）

| 产物 | tokio 版 | 零依赖版 | 变化 |
|---|---|---|---|
| qmodem-atd 二进制 | 1,322,696 B | 748,176 B | **-43.4%** |
| libqmodem_pdu.rlib | 95,882 B | 94,954 B | ≈持平 |

运行时收益：无异步运行时线程池与调度开销，常驻内存仅 = 主线程 + 每端口 1 actor 线程
+ 每连接 1 线程（1-4 模组场景下 RSS 显著低于 tokio multi-thread runtime）。

## 设计总则

1. **安全第一**：PDU 解码全边界检查、永不 panic；actor 线程独占资源所有权，共享态只用 Arc/Mutex/channel。
2. **语义对齐 C 版**：端标志匹配规则、默认 end flags、号码 TOA 规则等与 C 实现一一对应，差异（修复项）在两个 crate 的 README 中以 `C 文件:行号` 标注。
3. **feature 门控硬件依赖**：默认构建不触碰串口/libubus，开发机与 CI 全绿。
4. **零外部依赖**：并发原语与 JSON 均为 std + 自研实现，crates.io 不可达时构建路径不受影响。

## 后续迁移计划

1. **ubus FFI 接入**（`feature = "ubus"`）：实现 `ubus_ffi.rs` 的对象注册与 handler 桥接，替换/并列 JSON 服务，对齐 `application/ubus_at_daemon` 的方法表（open/close/sendat/list/event_callback*/load_conf）。交叉编译走 OpenWrt SDK sysroot。
2. **vendor 规则引擎**：迁移 `libqmodem-sms` 其余部分（smsd 的分段重组、存储读写）与 `qmodem_smsd` 的队列调度；UDH 分片重组逻辑直接复用 `qmodem-pdu::Udh`。
3. **smsd**：在 atd 之上加消息组装/去重/持久化 actor 线程，替代 C 版 `qmodem_smsd` 的轮询模型。
4. **TtyPort 加固**：从 O_NONBLOCK 轮询升级为 poll/epoll 集成（仍零依赖，`std::os::fd` + libc syscall 声明）；端口监控线程（C `port_monitor_thread_func`）改为周期任务。
5. **配置加载**：对齐 `config_loader.c` 的 JSON 配置（端口列表、波特率、事件回调脚本），命令注入面用 `std::process::Command` 参数数组消除。

各 crate 的 C→Rust 修复对照详见其 README 与 `docs/refactor/journal-rust-core.md`。

## Cross compilation

```sh
# aarch64 musl 静态二进制（自动获取 musl 工具链）
bash scripts/cross-build-rust.sh --target aarch64-unknown-linux-musl
# x86_64 musl
bash scripts/cross-build-rust.sh --target x86_64-unknown-linux-musl
# 原生（本机验证用）
bash scripts/cross-build-rust.sh
```

产物输出到 `scripts/dist/<target>/qmodem-atd`（stripped，含 sha256 与 `file`
自检）。release profile：`opt-level="z"` + `lto` + `strip` + `codegen-units=1`，
本机实测 450,544 B（x86_64-gnu 原生）；纯 std 零依赖，musl 下全静态链接，
无运行时 so 依赖。

集成到 OpenWrt 包的两条路线：

1. **SDK 源码构建**：feed 包 Makefile 直接调用 cargo（SDK 带 rust 工具链时优先，
   构建可复现）；
2. **预编译注入**：CI（`.github/workflows/rust-build.yml`）按 target 产出静态
   二进制 artifact，`qmodem-suite` 按 ARCH 注入（适合 SDK 无 rust 的代际）。

详见 docs/refactor/journal-rust-cross.md。
