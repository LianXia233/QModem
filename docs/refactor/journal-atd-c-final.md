# journal-atd-c-final

> agent: **agent-atd-c-final**
> 范围: application/ubus_at_daemon/**（含新建 extlib/）
> 日期: 2026-10-12

Phase 0/2（journal-c-core / journal-atd-quality）之后的收尾批次：独立 PDU 小工具、dormant 标注、警告清零、超长 port_path 防错配。

## 1. 新建 extlib/pdu_decoder.c（独立小工具）

- 问题：tom_modem/src/extlib/pdu_decoder.c 同型工具对 stdin hex 流用
  `getchar()` 循环无边界写入固定 buffer（`*p++` 无上限），超长输入直接
  溢栈。本目录此前无该工具，按任务要求新建独立版本。
- 修复（application/ubus_at_daemon/extlib/pdu_decoder.c，自包含、无外部依赖）：
  - 硬上限 `SMS_MAX_PDU_LENGTH` 192 字节（TS 23.040 SMS-DELIVER TPDU
    176 字节 + SMSC 头余量）；hex 流超限即 `fprintf(stderr)` + `exit(1)`。
  - 非法 hex 字符、奇数个 hex 位（悬挂半字节）均错误退出。
  - 解码 SMS-DELIVER：SMSC 跳过、发件人（semi-octet swap）、TP-SCTS
    （low nibble 为十位：`low*10+high`，非十六进制交换）、TP-PID/DCS、
    GSM7（打包位流 + UDHI fill-bits 跳过）与 UCS2（→UTF-8）。
  - GSM7 的 TP-UDL 按 septet 计、UCS2 按 octet 计，分别校验；UDHI 时
    septet 偏移相对完整 UD 位流（不能先跳过 UDH octets 再算）。
- 冒烟（/tmp 编译运行，三例全过 + 三例错误路径 exit=1）：
  - GSM7 无 UDH：`hello` ✓；UCS2：`中e` ✓；UDHI 分段（6 octet UDH）：
    `hello` ✓（测试数据用 python 按 LSB-first + fill-bit 生成）。
  - 溢出输入（'AB'*500）→ "input exceeds 384 hex digits"、exit=1；
    非法字符 'Z'、奇数长度 → exit=1。

## 2. dormant 标注（不删代码）

依据：main.c 的 ubus 方法表（main.c:548-559）仅注册
open/sendat/list/close/lease_*/urc_*；以下函数不在注册面且全仓无调用点：

- config_loader.c `load_config_from_json()`：保留既有 `__attribute__((unused))`
  与说明注释，函数前加 `/* dormant: not registered in ubus method table */`。
- at_handler.c `send_at_command()`：grep 全 src/tests 仅
  send_at_command_only/send_at_command_with_response 有调用，此包装函数
  无任何调用点，加同款标注。

## 3. -Wall -Wextra 警告清零

基线（/tmp/qstub stub 头，`gcc -fsyntax-only -Wall -Wextra -std=c99
-D_GNU_SOURCE`，7 个 src 文件）复核出三类，全部修复：

1. `ubus_at_daemon.h:276-302` 9 个 static 方法声明在无定义的 TU 触发
   `-Wunused-function`（其中 event_callback*/load_conf 5 个连 main.c 都
   无定义，属头文件层死代码）。修复：整块声明从共享头移除（它们是
   main.c 内部实现细节，定义自带声明；未注册的 4 个声明一并消失），
   原位留注释说明。
2. main.c：`ubus_open_method` 的 `timeout` set-but-not-used（main.c:92，
   赋值后从未读取，open 方法本来就不消费 timeout）——删除变量与赋值，
   `open_policy` 的 timeout 字段保留（ubus 接口不变）；5 个函数的 unused
   参数（obj/method/msg/argc/argv）按 control.c 既有风格补 `(void)x;`，
   main 改 `int main(void)`。
3. line_events.c 的 `uloop_fd.registered`/`ULOOP_READ`/`container_of`/
   `blobmsg_add_field`/`ubus_send_event` 报错为 stub 缺失假警——重建
   /tmp/qstub（uloop.h 补 registered/uloop_fd_add/uloop_fd_delete、
   libubus.h 补 container_of/ubus_send_event、blobmsg.h 补
   blobmsg_add_field），未改仓库代码。

验证：8 个文件（7 src + extlib/pdu_decoder.c）`gcc -fsyntax-only -Wall
-Wextra -std=c99 -D_GNU_SOURCE -I/tmp/qstub -I/tmp/qstub/libubox
-I/tmp/qstub/json-c` 零输出（0 警告 0 错误）。

host 测试（tests/，-Werror 全绿编译）：8 个用例中
line_event_queue/reader_flood/port_lifecycle/port_concurrency/
production_reader_lifecycle/uloop_adapter/control 7 个 PASS；
test_response_baseline 与 test_terminal_matching 失败——已用
`git stash` 对照未修改基线复现，**既有失败（pty 时序断言），非本批引入**，
遗留待查（记录于 HANDOFF 待办为宜）。

## 4. port_manager.c 超长 port_path 防错配

- 问题：`port_path[MAX_PORT_PATH_SIZE=64]`，create_port_instance 用
  strncpy 截断超长路径；sendat 流程 `find→miss→create` 后，实例存的
  是截断名，后续 `find_port_instance(原始长路径)` 永远 miss，每次
  sendat 都新建一个截断实例（错配 + 实例泄漏式累积）。
- 修复：`find_port_instance()` 与 `create_port_instance()` 入口统一
  校验 `!port_path || strlen(port_path) >= MAX_PORT_PATH_SIZE` 直接
  返回 NULL（find 语义上超长路径不可能被存储，reject 等价 not-found；
  create 返回 NULL 走既有失败路径：ubus open 返回 NO_DATA / config
  loader continue / sendat NO_DATA）。注释说明截断-错配机理。

## 5. 纪律复核

- 只改 application/ubus_at_daemon/**：src 5 文件 + 新建 extlib/pdu_decoder.c；
  /tmp/qstub 为会话外 stub（未入仓库）。git status 确认无其他目录改动。
- 全部文件 LF（`grep -rIl $'\r' src extlib` 为空）。
- stub 头目录 /tmp/qstub 可随时按下述清单复建：json-c/json.h、
  libubox/{blobmsg.h,blobmsg_json.h,uloop.h}、libubus.h（内容含本批
  增补的 uloop_fd_add/delete、container_of、blobmsg_add_field、
  ubus_send_event）。
