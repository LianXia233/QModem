# at-daemon 质量清理日志（journal-atd-quality）

> 本日志由 agent-atd-quality 维护。范围：application/ubus_at_daemon/**（Phase 0 C1/C2/C3/C7 之后的剩余中低危项）。

## C13/C15 write 短写清理复核（无改动，确认完整）

【审查结论】
`send_at_command_with_response()` 的 `written != send_len` 路径（at_handler.c:205-210）：短写与错误统一失败，`waiting_for_response = 0` 在 `response_mutex` 内完成，response 缓冲区由入口 memset（Phase 0 C1）保证全路径清零；`send_at_command_only()` 无等待态无需清理。确认 Phase 0 修复闭环，无需改动。

## C7-残留 port_monitor 持全局锁执行探测 → 两阶段化

【问题】
`port_monitor_thread_func()`（port_manager.c）持 `g_daemon_ctx.ports_mutex` 期间逐端口调用 `check_and_reconnect_port()` → `open_at_port()`/`close_at_port()`（内含 `stop_reader()` 的 `pthread_join`），阻塞所有 ubus `list`/`close`/`open`/`sendat` 的 `find_port_instance()`。

【修复方式】
三阶段（port_manager.c port_monitor_thread_func）：
1. 快照：锁内遍历 ports 链表，跳过 `pending_destroy` 端口，对其余端口 `refcount++`（复用 Phase 0 引用计数），指针存入临时 malloc 数组；临界区仅 O(n) 指针操作。
2. 探测：锁外逐端口 `check_and_reconnect_port()`，open/close/join 不再持全局锁。
3. 回收：逐端口 `port_instance_release()`，引用归零且 pending_destroy 时触发延迟销毁；free 快照数组。malloc 失败时跳过本周期（下一周期重试）。

配套调整：
- `port_instance_acquire()/port_instance_release()` 从 main.c static 提升为跨文件函数（定义移至 port_manager.c，声明加入 ubus_at_daemon.h），与 destroy 逻辑同址，测试可链接。
- 顺带修复 main.c 退出清理路径的既有死锁：原代码持 `ports_mutex` 调用 `destroy_port_instance()`（其内部再次 lock 同一非递归锁，Phase 0 引入延迟销毁检查后即构成自死锁）。改为循环弹出链表头后锁外销毁；`destroy_port_instance()` 的 unlink 查不到已弹出节点为安全 no-op。

【锁序静态复核】
- 监控线程：`ports_mutex`（仅 refcount/快照）→ 释放 → `lifecycle_mutex` → `state_mutex`（open/close 原有顺序）。从不持 ports_mutex 取任何端口锁；`ubus_list_method` 的嵌套顺序为 ports_mutex → state_mutex，两者不构成反序环路。
- `port_instance_release()`：`ports_mutex` 单独持有，释放后才调用 destroy；`destroy_port_instance()` 内部顺序为 ports_mutex（查 refcount，即放）→ 端口各锁（close/queue/callbacks/destroy，期间不持 ports_mutex）→ ports_mutex（unlink）→ free。全程不存在"持端口锁取 ports_mutex"与"持 ports_mutex 取端口锁"同时成立的路径。
- 监控线程探测期间若 ubus close 到来：destroy 仅置 `pending_destroy` 并提前返回，端口内存由监控线程持有的引用保证有效；探测可能对将亡端口做一次无害的 reopen，release 时由延迟销毁兜底关闭。

## C11 config_loader.c 边界与死代码

【问题】
`load_config_from_json()`：`ftell` 失败返回 -1 时 `malloc(0)`/`fread(-1)`/`content[-1]` 未定义行为；`fread` 返回值未检查。全仓库无调用点（dormant 死代码）。

【修复方式】
- `fseek(END)`/`ftell`/`fseek(SET)` 逐一检查，`file_size < 0` 报错并 fclose 清理；
- `malloc((size_t)file_size + 1)`，`fread` 返回值与 `file_size` 比对 + `ferror()` 检查，失败 free/fclose 返回 -1；
- 文件头注释标注 DORMANT（入口走 ubus open，JSON 加载器无调用方），函数加 `__attribute__((unused))`；不删除，保持 API 兼容。

## 低危项

### L1 port_path strncpy 隐式依赖 calloc 清零

`create_port_instance()` 中 `strncpy(port->port_path, port_path, sizeof-1)` 后显式 `port->port_path[sizeof-1] = '\0'`（port_manager.c），截断语义不变（路径超长截断，与原行为一致），不再依赖 calloc 清零这一隐式保证。

### L2 create_port_instance 中途失败无回滚

8 个同步原语（6 mutex + 2 cond）逐一检查返回值，任一失败经 `goto fail_*` 链逆序 `pthread_*_destroy` 已初始化对象并 `free(port)` 返回 NULL，不再泄漏半初始化对象。`response_cond` 保持 CLOCK_MONOTONIC condattr（Phase 0 C7c）。restart_epoch/last_check_time 等字段初始化保留（calloc 清零余下字段，语义不变）。

### L3 lease token 可预测（control.c）

原 `"%llx-%llx"` = 实时纳秒 epoch + 自增序号，本机观察者可猜测。新增 `read_random_token_bytes()` 从 `/dev/urandom` 读 8 字节（短读循环，失败返回 -1）作为随机部分，/dev/urandom 不可用时回退 daemon_epoch；自增 `next_lease_id` 保留作唯一性保证。token 仍为 `"%llx-%llx"` 格式，长度在 `lease->token[96]` 内，`at_lease_renew/release` 的 strcmp 校验无需改动。

## C11/低-5 sendat 超时折叠确认（无改动）

`application/tom_modem/src/ubus_client.c` 位于 tom_modem 文件域，**不在本轮授权范围内，未改动**。daemon 侧确认：at_handler.c 返回 `response->status`（0=成功；ETIMEDOUT 时 -1），main.c ubus_sendat_method 对 `result==0 / -1 / 其他` 分别回复 status=success/timeout/error，区分逻辑完好、本轮无误改。既有观察（仅记录不改）：at_handler.c 的 write 失败路径同样返回 -1，daemon 侧会归入 timeout 语义，属历史行为，如需细分建议后续以 response->status=-2 区分。

## 验证结果

- `gcc -fsyntax-only -Wall -Wextra -std=c99 -D_GNU_SOURCE`（/tmp/qstub stub 头，未入仓库；其中补了 `json_tokener_parse` 原型）：port_manager.c / config_loader.c / control.c / main.c / at_handler.c 全部通过。剩余警告均为既有问题：ubus_at_daemon.h 中 static 方法声明未定义（-Wunused-function）、main.c ubus 回调 unused 参数与 `timeout` set-but-not-used（main.c:122，既有）。
- tests/ 套件（`-Werror` 构建）：test_line_event_queue / test_reader_flood / test_port_lifecycle / test_port_concurrency / test_production_reader_lifecycle / test_uloop_adapter / test_control 全部 PASS。test_response_baseline 与 test_terminal_matching 运行期断言失败为既有问题：二者仅链接 at_handler.c + line_events.c（git 显示本轮无改动），依赖 pty 环境，与本轮 diff 无关。
- diff 逐处复核：5 文件 +225/-80，全部位于 application/ubus_at_daemon/**，LF 换行。

【涉及文件:行号】
- application/ubus_at_daemon/src/port_manager.c:25-51（acquire/release 定义）、68-131（strncpy NUL + 回滚）、434-488（监控线程两阶段）
- application/ubus_at_daemon/src/main.c:8-10（说明注释）、628-641（退出清理去死锁）
- application/ubus_at_daemon/src/ubus_at_daemon.h:230-235（acquire/release 声明）
- application/ubus_at_daemon/src/config_loader.c:1-9（dormant 标注）、22-66（ftell/fread 边界）
- application/ubus_at_daemon/src/control.c:40-57（urandom 读取）、84-98（随机 token）
