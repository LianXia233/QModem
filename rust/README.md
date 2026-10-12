# QModem Rust 重构区

本目录是 QModem 核心组件向 Rust 迁移的落地区。当前包含两个 crate 的 Cargo workspace：

```
rust/
├── Cargo.toml            # workspace（统一依赖版本）
├── qmodem-pdu/           # 3GPP TS 23.038 SMS PDU 编解码（lib）
│   └── src/lib.rs        #   ← application/libqmodem-sms/src/pdu.c 的修复版重写
└── qmodem-atd/           # tokio 异步 AT 守护进程（bin + lib）
    ├── src/transport.rs  #   SerialPort trait / LoopbackPort / TtyPort(tty feature)
    ├── src/port.rs       #   AtPort actor：命令队列 + oneshot 响应 + 超时 + URC 广播
    ├── src/registry.rs   #   端口注册表
    ├── src/service.rs    #   Unix socket JSON 行协议（service feature，默认开）
    ├── src/ubus_ffi.rs   #   libubus FFI 骨架（ubus feature，默认关）
    └── src/main.rs       #   守护进程入口
```

## 构建

```sh
cd rust
cargo test --workspace      # 24 个测试
cargo build --release       # target/release/qmodem-atd
cargo clippy --workspace    # 0 warning
# 可选 feature：
cargo check -p qmodem-atd --features tty,ubus
```

网络不可达时的降级路径：依赖只有 tokio/serde_json/thiserror/libc（gated）。若 crates.io 不可用，pdu crate 可退化为零依赖（手写 Display impl），atd 需 tokio（设计上无法用 std::thread 等价替代 select! 语义；当前环境 crates.io 可达，未启用降级）。

## 设计总则

1. **安全第一**：PDU 解码全边界检查、永不 panic；异步任务独占资源所有权，共享态只用 Arc/Mutex/channel。
2. **语义对齐 C 版**：端标志匹配规则、默认 end flags、号码 TOA 规则等与 C 实现一一对应，差异（修复项）在两个 crate 的 README 中以 `C 文件:行号` 标注。
3. **feature 门控硬件依赖**：默认构建不触碰串口/libubus，开发机与 CI 全绿。

## 后续迁移计划

1. **ubus FFI 接入**（`feature = "ubus"`）：实现 `ubus_ffi.rs` 的对象注册与 handler 桥接，替换/并列 JSON 服务，对齐 `application/ubus_at_daemon` 的方法表（open/close/sendat/list/event_callback*/load_conf）。交叉编译走 OpenWrt SDK sysroot。
2. **vendor 规则引擎**：迁移 `libqmodem-sms` 其余部分（smsd 的分段重组、存储读写）与 `qmodem_smsd` 的队列调度；UDH 分片重组逻辑直接复用 `qmodem-pdu::Udh`。
3. **smsd**：在 atd 之上加消息组装/去重/持久化任务（tokio actor 扩展），替代 C 版 `qmodem_smsd` 的轮询模型。
4. **TtyPort 加固**：从 O_NONBLOCK 轮询升级为 `tokio::io::unix::AsyncFd` epoll 集成；端口监控线程（C `port_monitor_thread_func`）改为周期任务。
5. **配置加载**：对齐 `config_loader.c` 的 JSON 配置（端口列表、波特率、事件回调脚本），命令注入面用 `tokio::process` + 参数数组消除。

各 crate 的 C→Rust 修复对照详见其 README 与 `docs/refactor/journal-rust-core.md`。
