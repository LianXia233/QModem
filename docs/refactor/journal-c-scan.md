# journal-c-scan（modem_scand + 打包/init 修复）

agent: agent-c-scan
date: 2026-10-12
范围: application/modem_scan/**, application/qmodem/files/etc/{init.d,hotplug.d,uci-defaults}/**

## 修复清单

### 1.【Sec2-高】控制套接字权限（modem_scand.c）
- 原 `chmod(QMODEM_SCAND_SOCKET, 0666)` 允许任意本地用户连接 root 守护进程，触发
  uci set/commit 与 network restart。
- 重写为 `apply_socket_permissions()`（modem_scand.c:1649）：
  - `chmod 0660`；
  - `getgrnam("qmodem")` 取 gid 后 `chown(0, gid)`；组不存在回退 root:root 并
    `syslog(LOG_WARNING)`；
  - chown / chmod 失败路径分别记录 `LOG_ERR`，不中断守护进程。
- 组创建：`init.d/qmodem_init` 新增 `_ensure_qmodem_group()`，start_service 首行
  `grep -q "^qmodem:" /etc/group || echo "qmodem:x:<maxgid+1>" >> /etc/group`，成功后
  logger 记录。已注明：正式构建应由包 postinst 处理，此为运行时兜底。

### 2.【Sec2/Sec11】输入白名单
- `section_from_slot()`（modem_scand.c:428）改为返回 int：仅允许 [A-Za-z0-9_]，
  `.`/`:`/`-` 映射为 `_`，长度 ≤64；空串/非法字符/超长返回 -1。
  调用方 `add_modem()`、`disable_slot()` 检查返回值，非法直接拒绝并 syslog。
- 新增 `valid_section_name()`（remove/disable 参数白名单）与 `valid_slot_type()`
  （scan 枚举：usb/pcie/all）。
- `handle_client()`：add 的 slot_type 限定 usb/pcie、slot 必须可安全映射；
  remove/disable 的 section 必须过白名单；scan type 必须属已知枚举；全部拒绝路径
  输出 `{"code":1,...}` 并 syslog(LOG_WARNING)。

### 3.【C8-中】线程创建/回收（modem_scand.c）
- `validate_ports()`：检查 `pthread_create` 返回值，失败时 `ctx[i].port = NULL`
  标记槽位无效（跳过），join 循环仅 join 有效线程（`if (ctx[i].port)`）。
- `main()`：worker 线程逐个检查创建返回值，统计 `g.scan_workers_started`；
  全部失败则释放资源并退出；join 循环按实际创建数执行。

### 4.【打包/init】qmodem_init 与 hotplug
- bashism：4 处 `==` 改 `=`（_try_pcie_slot/_try_usb_slot/_try_usb_device/
  _try_pcie_device）。
- boot() 派生的无跟踪子 shell：新增 `_locked_oneshot(delay, tag, cmd...)`，
  子进程统一走 `flock -w 120 /var/run/qmodem/scan.lock`（串行化，避免重复/并发扫描）
  + `logger -t qmodem_init` 记录 start/done/skipped；boot()、_scan_usb、_scan_pcie、
  _try_slot、_try_device 全部改经该封装。USE_PROCD=1 的 scand 实例（_start_scand）
  保持不变；一次性扫描仍由 `/etc/init.d/qmodem_init boot` 触发。
- hotplug（20-modem-net / 20-modem-usb）：`[ "${manual}" -eq 1 ]` 改为
  `[ "${manual:-0}" -eq 1 ]`，消除空变量 test 报错；start_service 中
  block_auto_probe 同步加 `:-0` 兜底。

### 5.【Sec10-低】qmodem_collect --raw
- `usage()` 增加 --raw 隐私警告行。
- `cmd_pack()`：解析出 `--raw` 后立即向 stderr 输出 WARNING 并
  `logger -t qmodem_collect`；打包完成处原静默 raw 分支改为显式 WARNING（stderr +
  logger），非 raw 分支保留 sanitized 提示。默认（非显式 --raw）始终走
  `mask_digits` 脱敏，行为不变。

## 验证记录

| 项 | 命令 | 结果 |
|---|---|---|
| C 语法 | `gcc -fsyntax-only -Wall -Wextra -I/tmp/qstub -I src modem_scand.c`（json-c stub 头） | PASS（SYNTAX-OK，无警告输出） |
| shell 语法 | `sh -n` 对 qmodem_init / 20-modem-net / 20-modem-usb / qmodem_collect | 全部 PASS |
| bashism 复查 | `grep -n '==' qmodem_init`（排除 =~） | 无残留 |
| chown/chmod 逻辑静态复核 | 复核 apply_socket_permissions：0660 + getgrnam 回退 root:root + 双失败路径日志 | PASS |
| join 有效性 | 复核 validate_ports：port=NULL 标记跳过 join；main 按 scan_workers_started join | PASS |

## 备注
- qmodem 组的 gid 取 /etc/group 最大 gid+1；postinst 接管后可删除 _ensure_qmodem_group。
- add_modem 的 section 白名单校验位于 uci 写入之前，非法 slot 不会产生任何 uci 操作。
