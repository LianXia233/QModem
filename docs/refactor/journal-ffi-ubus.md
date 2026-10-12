# journal-ffi-ubus — qmodem-atd ubus FFI 实装 + 纯 Rust blobmsg codec + mock transport

> 由 agent-ffi-ubus 维护。
> 日期：2026-10-12
> 范围：`rust/qmodem-atd/**` + 本 journal。零新增外部 crate（仅 std + 系统 cc 编译 /tmp stub）。

## 1. 交付物

| 路径 | 内容 |
|---|---|
| `src/ubus_proto.rs` | 纯 Rust blobmsg 编解码（libubox 兼容 wire format）+ `mock` transport（`MockUbusServer`、`dispatch_sendat`/`dispatch_list`、客户端 encode/decode 助手）；12 个单元测试 |
| `src/ubus_ffi.rs` | 手写 libubus/libubox 最小 FFI（`ubus_connect/free/lookup/lookup_id/add_object/send_reply/blobmsg_parse`）、`repr(C)` 方法表、`sendat`/`list` handler、RAII `UbusConnection`、`register_atd_object`（container_of 嵌入）；全部在 `feature="ubus"` 门控下 |
| `tests/ubus_mock.rs` | 默认构建的「ubus 调 sendat」集成测试（8 个：ok/custom end flag/timeout/unknown port/missing at/乱码 blob/list/unknown method/响应 wire format 结构） |
| `tests/ubus_ffi_smoke.rs` | `--features ubus` 冒烟（3 个：符号解析+RAII、方法表布局、Rust 编码 blob 过 C `blobmsg_parse` 往返） |
| `README.md` | FFI 章节改写为「ubus FFI 与 mock transport（同语义双路径）」 |
| `/tmp/stub-ubus/` | stub 验证用共享库（不入库，源码见附录） |

## 2. 核心设计：同语义双路径

主机/CI 无 libubus，拆两层但共享一套请求解码/响应编码代码：

```
ubus 调 sendat ──┬─ feature=ubus ──> C handler → ubus_proto::decode_msg 切 C blob
                 └─ 默认构建 ──────> MockUbusServer.call → dispatch_sendat
```

`dispatch_sendat`/`encode_sendat_result`/`dispatch_list` 只实现一次（`ubus_proto::mock`），
C handler 直接调用，两条传输路径行为一致 —— mock 集成测试即覆盖真实 handler 语义。

### wire format（libubox blob.h/blobmsg.h 逐字节兼容）

- 4 字节对齐 TLV：头字 `type<<28 | data_len`（native endian）；
- 命名属性带 blobmsg 头：`u16 namelen + name + NUL`，`pad4(2+namelen+1)` 对齐；
- 字符串含尾 NUL；嵌套为子 TLV 串联；`data_len` 含名字头（同 `blob_len`）；
- 结构性约定：TABLE 子节点命名、ARRAY 子节点无名（对应 `blobmsg_open_table/array` 用法）；
- 字节级 golden 测试锁格式（`golden_bytes_named_string_attr`）。

### 关键决策

| 决策 | 理由 |
|---|---|
| 手写 extern 声明，不用 bindgen | 零依赖；`extern` 块无符号引用时编译/语法检查不需要 libubus 头文件或库 |
| C handler 复用 Rust 解码器（而非 C 端 `blobmsg_parse`） | wire format 一致即可直接切指针；`blobmsg_parse` 仅在 FFI 冒烟测试中反向验证格式 |
| Registry 经 `AtdUbusObject{obj, registry}` 嵌入 + `offset_of!` 还原 | C 惯用 container_of 模式，避免全局指针 |
| 注册相关 C 可达内存刻意 `Box::leak` | libubus 持指针至连接关闭，与 C daemon 静态方法表同型 |
| policy 表运行时构建（非 static） | 含原始指针的类型不是 `Sync`，static 需 unsafe impl；运行时构建零 unsafe 收益 |

## 3. 验证记录（本机实测，cargo 1.93 / gcc 12）

| 命令 | 结果 |
|---|---|
| `cargo test -p qmodem-atd`（默认构建） | 47/47 绿（lib 26 + atd_integration 8 + zero_dep 4 + ubus_mock 8 + …），0 warning |
| `cargo check -p qmodem-atd --features ubus` | 通过（无需任何 stub 头文件 —— 手写 extern 无符号引用即可编译） |
| `cargo check -p qmodem-atd --features tty` | 通过 |
| `cc -Wall -Wextra -fPIC -shared` 编译 /tmp stub `libubus.so`/`libubox.so` | 通过（0 警告） |
| `RUSTFLAGS="-L native=/tmp/stub-ubus -lubus -lubox" LD_LIBRARY_PATH=/tmp/stub-ubus cargo test -p qmodem-atd --features ubus` | 52/52 绿（含 FFI 冒烟 3 个），0 warning |
| `cargo clippy -p qmodem-atd --features ubus --all-targets` | 本工作包文件 0 警告（bench_atd.rs 2 条属另一在途工作包） |

FFI 冒烟亮点：`rust_encoded_blob_parses_via_c_blobmsg_parse` 用 C stub 按 libubox
语义实现的 `blobmsg_parse`（policy 匹配 + name 头校验 + 4 字节对齐推进）解析 Rust
编码的 sendat 请求，逐字段比对 payload 字节 —— Rust↔C wire format 一致性有跨语言测试锁定。

## 4. 已知边界

- stub 库镜像的是**声明签名**，不代表真实 libubus 行为（`ubus_add_object` 等返回罐头值）；
  `ubus_object` 的 subscriber list_head 等内部字段按 `ubus.h` 布局近似（opaque `[u8;16]`），
  真机链接需在 OpenWrt SDK 上以 `--features ubus` 构建并运行验证（`blobmsg_parse` 签名/布局
  语义已按 libubox 源码语义实现，风险集中在 `ubus_object` 精确布局）。
- blobmsg 类型只实现 atd 方法表所需子集（STRING/INT32/INT64/TABLE/ARRAY），
  其余类型返回 `UnsupportedType`（错误可恢复，不 panic）；后续方法移植时按需扩展。
- 数值 endian 跟随宿主（同 libubox）；OpenWrt 目标均为 LE，与主机测试环境一致。
- `tests/bench_atd.rs`（他人在途工作包）存在 2 条 clippy 警告，不属于本工作包范围。

## 附录：/tmp stub 源码要点（不入库）

- `stub_ubox.c`：`blobmsg_parse` 按真实语义实现——遍历 4 字节对齐 TLV、`u16 namelen`
  名字头 `pad4(2+namelen+1)`、NUL 校验、policy 名字匹配写 `tb[]`、未知字段跳过、
  `hdr==0` 终止符；越界/截断返回 -1。
- `stub_ubus.c`：`ubus_connect`(malloc)/`ubus_free`/`ubus_lookup_id`(*id=1)/
  `ubus_add_object`/`ubus_send_reply` 罐头成功。
- 复现：`cc -Wall -Wextra -fPIC -shared -o libubox.so stub_ubox.c && cc ... -o libubus.so stub_ubus.c`
