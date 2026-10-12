# Refactor Journal — qmodem-smsd (Rust 骨架)

- Agent: `agent-smsd-rs`
- Date: 2026-10-12
- Scope: `rust/qmodem-smsd/**`（新增）+ `rust/Cargo.toml` members 一行 + 本 journal
- 对应 C: `application/qmodem_smsd`（main.c 1493 行，17 个 ubus 方法）

## 语义对齐来源（先读后写）

| C 语义 | 位置 | Rust 落点 |
|---|---|---|
| 去重键 `UNIQUE(modem_id,storage,source_index,pdu)`；重复导入仅刷新 `imported_at`，`safe_to_delete=committed` | `sms_db.c:450` `sms_db_import_segment` | `storage::import_segment`，duplicate 时零消息变更 |
| claim 先回收 `claimed_at<now-300` 的陈旧 claim → pending；取 `pending AND available_at<=now ORDER BY id LIMIT 1`；`attempts+1` | `main.c:1183` `delivery_claim_method` | `storage::claim_delivery`（`CLAIM_TIMEOUT_SECS=300`） |
| ack: 成功→done；失败→`pending, available_at=now+MIN(3600,30*(1<<MIN(attempts,7)))`，仅 `state='claimed'` 可 ack | `main.c:1236` `delivery_complete_method` | `storage::complete_delivery`（`BACKOFF_BASE/MAX_SECS`） |
| 30s 调度 tick；`last_sync_at+poll_interval<=now` 才 poll；跳过 database_urc/direct | `main.c:1382` `scheduler_cb` | `scheduler::Scheduler`（std thread + `recv_timeout`），tick/interval 可注入 |
| URC 过滤 `owner=qmodem.sms && urc_id=sms-new`；pending 合并；gap=`epoch!=prev || seq!=prev+1`（prev_epoch!=0）；trigger `urc`/`urc_gap` | `main.c:1320` `urc_event` + `sms_db.c` `record_event` | `urc::UrcSource/UrcStream` + `daemon::spawn_urc_worker` |
| `sync_runs(modem_id,trigger,started_at,finished_at,imported,error)` + `modems.last_sync_at` 更新 | `sms_db.c:637` `record_sync` | `storage::record_sync` |
| multipart 聚合（sender+reference+total）齐包发布；`multipart_wait` 超时以 `complete=0` 发布 | `sms_db.c` `find_group/publish_expired` | `storage::import_segment` 分组 + `publish_expired` |

## 实现

- **零新增外部 crate**：仅 path 依赖 `qmodem-pdu`；sqlite 按 trait 路线预留（`trait Storage` =
  持久化边界，README 记录建表/迁移/WAL 照抄 `sms_db.c` 的方案）。
- **PDU 解码走 `qmodem-pdu::decode`**：`import::segment_from_pdu` 从 hex 解出
  sender/text/UDH（C 版信任 tom_modem JSON 字段，Rust 版以 PDU 为准、metadata 仅兜底）。
- **线程模型**：scheduler（tick 循环）/ urc worker（250ms 超时轮询 + stop flag）/
  delivery worker（claim→hook→ack 循环），均为 `std::thread`，可 join 停止。
- **测试**：10 个。去重幂等（重复导入零新增消息）、claim 不重复下发（done 后不再出现；
  失败退避窗口内不可 claim；陈旧 claim 300s 后回收且 attempts 递增）、调度 tick 触发
  （20ms tick + 计数 hook）、URC fan-out/过滤/gap→`urc_gap`、delivery pass 一次下发。
  PDU fixture 复用 qmodem-pdu cross-vector（UCS-2 你好）。

## 验证

- `cargo test --workspace`：全绿快照 —— qmodem-smsd 10、qmodem-pdu 14（旧基线保持）、
  qmodem-atd 39（lib 26 + integration 8 + bench 4 + zero_dep 1）。37 个旧测试无回归。
- `cargo test -p qmodem-smsd`：10/10 稳定全绿。
- `cargo clippy -p qmodem-smsd`：0 warning（pdu 剩 2 个同类 cast 警告属并行
  `agent-pdu-conform` 任务正在修改的文件，未越界处理）。
- 注：收尾时并行任务向 pdu 新增 `ts23038_conformance`/`cross_c_pdu_semantics`
  集成测试尚在中间态（`to_tp_dcs is private`），与 smsd 无关；smsd 编译与测试
  不受影响（仅依赖 pdu lib，lib 本体正常）。

## 过程备注

- workspace 期间 `qmodem-pdu`/`qmodem-atd` 一度处于并行任务中间态（编译失败/新增测试失败），
  等其收敛后复测通过；本任务未触碰这两个 crate 的源码。
- 初版 `urc.rs` mock 的 publisher/subscriber 通道未连通，重写为 hub fan-out
  （与 `qmodem_atd::line::LineHub` 同构：订阅后发布的事件全量可见）。
- URC worker 内 ingest 为单线程串行（recv→ingest→下一个 recv），C 版双线程的
  pending 合并标志保留但不会触发；多消费者 adapter 落地时该路径生效。

## 未覆盖项（移交后续）

ubus 17 方法对外面、SQLite 真实现（SqliteStorage）、UCI 配置、legacy 迁移、
出站 encode（send/send_managed）、tom_modem 子进程、prune 保留策略、
`qmodem-atd` LineHub→UrcSource adapter。路线图见 `rust/qmodem-smsd/README.md`。
