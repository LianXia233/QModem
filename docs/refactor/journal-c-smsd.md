# journal-c-smsd（agent-c-smsd-v2）

> 接手前任代理的半成品（C4/C10/C12 进行中状态），本轮完成收尾。
> 范围限定：`application/qmodem_smsd/**`。

## 接手时的半成品状态

- `src/utils.c` / `src/utils.h`：新文件（C4 修复），内容未经前任验证 → 本轮通读 + ASan 实测验证（见下）。
- `src/Makefile`：已加入 `utils.o`，无需改动。
- `src/main.c`：`sync_modem` 已参数化接收 `sms_db_t *db`，但存在 8 处 sed 残留 `&db->`，且 **4 个调用点未适配新签名**（前任只改了函数体，漏改调用方）。

## 本轮修改

### 1. main.c：残留修复 + sync_modem 参数化补全

- 修复 8 处 sed 残留：`sms_db_*(&db->, ...)` → `sms_db_*(db, ...)`（行 454/492/500/506/510/513/523/529）。
- 补全 4 个调用点的 `&app.db` 实参（前任遗漏）：
  - `sync_method`（manual 触发）
  - `urc_event`（改写为异步路径，见 C10）
  - `control_event`（settings_reapplied）
  - `scheduler_cb`（poll 路径）
- 确认自洽：`app.db` 为 `app_context` 内嵌 `sms_db_t`，main 中 `sms_db_open`/`sms_db_close` 生命周期不变；`sync_modem` 内部经 `prepare_database_mode` 仍引用 `app.db`（与形参 `db` 恒指向同一对象，语义一致）。

### 2. C10（中）：urc_event 不再阻塞 ubus 分发线程

`urc_event` 原在 ubus/uloop 线程内同步执行 `sms_db_record_event + sync_modem`（`read_child_output` 单次子进程 deadline 15s），阻塞分发线程。最小化方案：

- 新增后台工作线程 `urc_sync_worker` + pending 槽（`pthread_mutex_t/cond_t` 保护）。
- `urc_event` 仅做 UCI 匹配（快速、无 DB 操作），把 `{section, epoch, sequence, has_epoch}` 写入 pending 槽、`pthread_cond_signal` 后立即返回；pending 期间新事件直接合并丢弃（在途 sync 已覆盖）。
- 工作线程串行执行 `sms_db_record_event` → `sync_modem`（gap 判定改在线程内，据此选 trigger `urc`/`urc_gap`），**DB 操作全部保留在工作线程内串行**。
- sqlite 线程归属：`sms_db_open` 已使用 `SQLITE_OPEN_FULLMUTEX`（sms_db.c:123）+ WAL + busy_timeout 5s，uloop 线程（ubus 方法）与工作线程跨线程使用同一句柄在 API 层安全；事务竞争表现为一方 `SQLITE_BUSY` 失败返回 -1，无损坏。另 scheduler_cb 显式跳过 `database_urc` 模式 modem，poll 与 urc 不同步同一对象。
- main() 中 `pthread_create` 失败时退化为原同步行为；uloop 退出后置 stop 标志、signal、join，线程干净终止。

### 3. C12（中）：sqlite3_column_text NULL 传入 snprintf

- `publish_group`（原 338-339 行）：`snprintf(modem_id/sender, ..., "%s", sqlite3_column_text(...))` → 局部 `const char *value` + `value ? value : ""`，与同文件 `bind_text`（sms_db.c:205）的 NULL 兜底风格一致。
- 同类模式全文件排查：`apply_migrations` 中 `strcmp((const char*)sqlite3_column_text(stmt,1), "last_event_epoch")`（原 85 行）同样可遇 NULL → 加 `column &&` 保护；`sms_db_migration_error` 已有 `SQLITE_NULL` 类型守卫、`group_content`/`publish_group` 的 old_content 比较已有 `?: ""` 兜底，均无需改动。

### 4. 低：COMMIT 失败补 ROLLBACK（全文件同类统一）

COMMIT 失败（如 SQLITE_BUSY）时事务仍活跃，原代码直接 return 导致连接停留在打开事务状态。统一修复 7 处：

- `sms_db_import_segment`（2 处，原 485/559）
- `sms_db_record_sync`（原 651）
- `sms_db_publish_expired`（原 604，顺带消除 `out_no_rollback` 标签，成功路径不再多余执行 ROLLBACK）
- `sms_db_prune`（原 765）
- `sms_db_delete_message`（原 835）
- `sms_db_delete_modem_messages`（原 864）

### 5. utils.c/utils.h 验证（前任未验证内容）

通读结论：边界逻辑正确，未发现需要修改的问题。
- `smsd_escape_json`：`used + length + 1 > output_size` 保证任何单次写入与结尾 NUL 均有空间；截断路径 NUL 终止后返回 -1。
- `json_buf_appendf`：堆上增长，vsnprintf 重试式扩容，硬顶 `SMSD_JSON_MAX_SIZE`（256KiB），失败路径由 `json_buf_free` 释放。
- `smsd_display_sms_in_json`：`escaped_text` 按 6×TEXT_SIZE+1 分配（最坏 \uXXXX 膨胀），尾逗号仅在存在条目时删除；空列表输出 `{"msg":[]}`。
- `smsd_decode_pdu` UCS2 分支：成对读取（`i + 1 < sizeof(sms_text)`）、`offset + length > size - 1` 时整码点截断（不切半多字节序列）、surrogate/NUL 单元跳过；GSM7 分支写入有界。

## 验证记录

- `gcc -fsyntax-only -Wall -D_GNU_SOURCE -I src src/*.c`：**0 error / 0 warning**。环境：libsqlite3-dev、libjson-c-dev（apt 安装，不入仓库）；ubox/ubus/uci 头用 /tmp stub；pdu.h 取自仓库内 `application/libqmodem-sms/src/qmodem-sms/pdu.h`（拷贝至 /tmp stub 目录）。
- ASan/UBSan/LeakSanitizer 独立自测（/tmp 编译运行后删除）：17/17 断言 PASS，0 泄漏 0 越界。覆盖：
  - escape_json 最坏 6x 膨胀填满精确尺寸缓冲、小缓冲截断 NUL 终止；
  - 手工构造 SMS-DELIVER PDU 的 **长 UCS2 短信**（246 × U+4E2D）：截断停在 354 字节 = 118 个完整 3 字节序列（%3==0，无半字符、无越界）；
  - **长 GSM7 短信**（160 septet）解码有界；
  - 3 条满长消息输出经 json-c 解析合法、数组完整、文档闭合；
  - 空列表输出 `{"msg":[]}`；128 条控制字符消息超 256KiB cap → 返回 -1 且无泄漏。
- `grep -n '&db->' src/main.c`：**结果为空**（exit 1）。
- 测试程序与 stub 头仅存于 /tmp，未入仓库。
