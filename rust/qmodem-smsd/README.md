# qmodem-smsd (Rust skeleton)

Rust 骨架，对应 C 守护进程 `application/qmodem/src/qmodem_smsd`（`qmodem_smsd` 二进制，ubus 对象 `qmodem.sms`）。
零新增外部 crate；PDU 解码复用 workspace 内 [`qmodem-pdu`](../qmodem-pdu)。

## 模块结构

| 模块 | 对应 C 源 | 说明 |
|---|---|---|
| `storage` | `sms_db.c` | `trait Storage`（去重导入 / sync_runs / URC epoch / deliveries 状态机）+ `InMemoryStorage` 参考实现 |
| `import` | `sync_modem` 导入段 | `import_pdu_segment`: hex → `qmodem_pdu::decode` → 填充 sender/text/UDH → 幂等导入 |
| `scheduler` | `scheduler_cb` + `SCHEDULER_TICK_MS` | `std::thread` + `recv_timeout` 的 30s tick；`last_sync_at + poll_interval <= now` 触发 `poll` 同步 |
| `urc` | `urc_event` + `urc_sync_worker` | `UrcEvent`（owner=`qmodem.sms`、`urc_id=sms-new`、epoch/sequence）+ `UrcSource`/`UrcStream` trait + fan-out hub |
| `daemon` | `main.c` 运行时拼装 | `sync_modem`（含 `sync_runs` 落库）、URC worker（事件合并 + gap 检测 → `urc`/`urc_gap` trigger）、delivery worker（claim → hook → ack） |

## 与 C 版功能对照

| C 功能 | 状态 | Rust 对应 |
|---|---|---|
| 消息导入去重（`UNIQUE(modem_id,storage,source_index,pdu)`，重复刷新 `imported_at`） | ✅ | `Storage::import_segment`（`duplicate`/`safe_to_delete` 语义一致） |
| PDU 来源解码 | ✅（改用 `qmodem-pdu`） | `import::decode_pdu_hex` / `segment_from_pdu`（C 信任 tom_modem JSON 字段） |
| multipart 聚合发布 + `multipart_wait`/`late_fragment_window` 超时发布 | ✅（简化） | `import_segment`（按 sender+reference+total 分组）+ `publish_expired` |
| 发送队列 claim/ack（回收 >300s 陈旧 claim；失败退避 `30*2^min(attempts,7)` 封顶 3600s） | ✅ | `Storage::claim_delivery` / `complete_delivery` + `daemon::run_delivery_pass` |
| 30s 调度 tick | ✅ | `Scheduler`（tick/`poll_interval` 可注入，测试用短周期） |
| URC 事件订阅（`qmodem.at.urc`，合并 pending 事件，gap=epoch 变化或 seq≠prev+1） | ✅ | `UrcSource` trait + `daemon::spawn_urc_worker` |
| `sync_runs` 记录（trigger: poll/manual/urc/urc_gap/settings_reapplied） | ✅ | `Storage::record_sync`（daemon 在 driver 返回后落一行） |
| ubus 17 方法对外接口 | ❌ 未覆盖 | 见下表 |
| SQLite 持久化 | ❌ 未覆盖（trait 预留） | 见下节 |
| UCI 配置（`qmodem_sms`、`qmodem`、`sms_forwarder`） | ❌ 未覆盖 | `port_map`/`poll_interval` 由构造方注入 |
| legacy 目录迁移（`legacy_migrate.c`） | ❌ 未覆盖 | — |
| 发送编码与 `send`/`send_managed`（`pdu_encode_ucs2_parts`、request_id 幂等） | ❌ 未覆盖 | 未来用 `qmodem-pdu` 补 encode 路径 |
| `tom_modem` 子进程调用（15s 超时读管道） | ❌ 未覆盖 | `SyncDriver`/`DeliveryHook` 由实现方承接 |
| `modem_list` 状态文件读取、`storage_get/set` AT 交互 | ❌ 未覆盖 | — |
| 保留策略 prune（received/sent 条数、孤儿清理、sync_runs 30 天清理） | ❌ 未覆盖 | — |

## 未覆盖项 / SQLite 路线

- **SQLite**：`Storage` trait 即持久化边界。真实实现（建议名 `SqliteStorage`）实现同一 trait、
  以 `Arc<dyn Storage>` 注入 daemon 即可，无需改上层。建表语句与迁移直接照抄
  `sms_db.c` 的 `schema_sql` + `apply_migrations`（v1–v3）；WAL + `busy_timeout(5000)` +
  `FULLMUTEX`；`import_segment` 用 `BEGIN IMMEDIATE` 包裹以匹配 C 的并发语义。
- **ubus 面**：17 个方法中 14 个是纯存储查询/变更，等 SQLite 落地后薄封装即可；
  `send`/`send_managed` 依赖出站 PDU encode 与 `tom_modem` 通路。
- **URC 对接 qmodem-atd**：写一个 `UrcSource` adapter，把
  `qmodem_atd::line::LineHub::subscribe()` 的 `AtLine { port, seq, epoch, line }`
  映射为 `UrcEvent`（`line` 含 `+CMT`/`+CMTI` 时 `urc_id="sms-new"`），交给
  `spawn_urc_worker`。hub 广播 + `drop_count` 语义由 atd 保证。

## 测试

`cargo test -p qmodem-smsd` 覆盖：去重幂等（重复导入零新增消息）、claim 不重复下发
（claim→ack(done) 后不再出现；失败退避与陈旧 claim 回收）、调度 tick 触发（短周期 +
计数 hook）、URC fan-out 与 gap→`urc_gap`、delivery pass 只下发一次。
