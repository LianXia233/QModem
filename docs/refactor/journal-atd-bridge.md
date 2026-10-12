# journal-atd-bridge（at_tool=2：rust qmodem-atd 可选接入 AT 工具链）

agent: agent-atd-bridge
date: 2026-10-12
范围: application/qmodem/files/usr/share/qmodem/modem_util.sh,
      application/qmodem/files/etc/config/qmodem
目标: 让 rust 版 qmodem-atd（unix socket JSON 行协议）成为第三条 AT 通路
     （uci `qmodem.main.at_tool=2`），**可选项、安装即用、不改默认行为**。

## 背景与协议确认

- 协议权威来源：`rust/qmodem-atd/README.md` 与 `rust/qmodem-atd/src/service.rs`。
  请求 `{"cmd":"sendat","port":...,"at":...,"timeout_ms":...}`（每行一个 JSON），
  响应 `{"status":"ok"|"timeout"|"error","lines":[...],"error":...}`。
- 关键实测（见验证节）：service.rs 的 `handle_conn` 是**纯 JSON 行协议**而非 HTTP，
  且响应后**不主动关连接**（循环继续 read_line）。README 中的 `curl --unix-socket`
  示例与实现不符：curl 自身的 6 行 HTTP 头每行触发一条 `bad json` 错误回复，
  sendat 响应经 curl 无法可靠取回（裸 socket 重放 curl 字节流可取回 7 行完整响应，
  证明是 curl 客户端侧行为差异）。
- 现有 at() 返回语义（调用方 modem_dial.sh/cmds/*.sh 的用法）：成功输出响应文本
  （调用方以 grep/awk 解析内容为主）；失败返回非零。tom_modem 默认超时 3s
  （main.c:162-164），fastat 为 1s（`-t 1`）。

## 变更清单

### 1. modem_util.sh：新增 atd bridge 与 dispatch 抽取

- 新增 `qmodem_atd_socket()`：socket 路径取 uci `qmodem.main.at_socket`，
  默认 `/var/run/qmodem/atd.sock`（与 rust atd 文档约定一致）。
- 新增 `qmodem_atd_sendat(port, atcmd, timeout_ms)`：
  - 请求构造：优先 `jq -cn --arg`（qmodem DEPENDS 已含 +jq，转义安全兜底）；
    无 jq 时回退 sed 转义（`\` 与 `"` 两字符）手工构造 JSON，port/at 字段必转义。
  - 传输：**nc -U 为主**（后台运行 + 0.1s 轮询响应文件，读到"末字节为换行"的
    完整 JSON 行即杀进程——因服务端不关连接，必须提前收割）；
    nc 不可用（如 busybox 未编 FEATURE_UNIX_LOCAL 的 -U）或零输出时，回退
    `curl --http0.9`（尽力而为）；两者都失败 → 报错返回 1，**不回退 at_tool 0/1**
    （与 at_tool 语义一致：选了不存在的工具就报错，UCI 注释已说明）。
  - 响应解析：`.status`==ok → 逐行输出 `.lines[]`、返回 0；==timeout → 输出已收
    部分行（对齐 tom_modem 超时仍吐部分响应的行为）、返回 1；error/传输失败 →
    经 m_debug 记录 `.error`、返回 1。解析优先 jq，回退 jsonfilter（同为硬依赖）。
- 新增 `qmodem_at_dispatch()` / `qmodem_fastat_dispatch()`：把 at()/fastat() 中
  6 处重复的 `if at_tool==1 … else …` 收敛为单一 case 分发（1=sms_tool_q，
  2=qmodem_atd_sendat 3000ms/1000ms，其他=tom_modem 保持原参数含
  `$use_ubus_flag`/`$options`），at()/fastat() 的 testcase_collect 分支同步收敛，
  行为零变化。

### 2. UCI 默认（/etc/config/qmodem）

- 新增 `option at_tool '0'`（默认 0=tom_modem，显式化但行为不变），注释说明
  三种取值：0=tom_modem(ubus)、1=sms_tool_q(直连 TTY)、2=qmodem-atd(unix
  socket JSON，可选)；并注明 at_tool=2 时 socket 缺失直接报错、不回退。
- 新增 `option at_socket '/var/run/qmodem/atd.sock'`（仅 at_tool=2 使用）。

### 3. modem_scand 探测链路（C 代码零改动）

- `at_command()`（modem_scand.c:795）通过 `execl("/bin/sh", … modem_util.sh …)`
  调 at()/fastat()（modem_scand.c:825），自动获得 at_tool=2 通路，无需改动。
- scand 的 AT 超时 `qmodem.main.at_timeout_fast/at_timeout_model`
  （modem_scand.c:1696-1708，默认 2s/8s）是外层 select+kill 看门狗
  （modem_scand.c:856-865），与工具选择无关，天然透传，无需新增配置。
- 注：外层看门狗会在 timeout 秒杀整条 shell 链，atd 内部 timeout_ms(1000/3000)
  必须小于外层值，现默认组合（1s/3s vs 2s/8s）兼容。

### 4. 未改动项

- modem_hook.sh 无需改动（经 at()/fastat() 自动生效）。
- rust/qmodem-atd、README、docs/developer/async-daemon.zh-cn.md 不在授权范围内；
  README 的 curl 示例与 service.rs 行为的偏差建议后续单独修正（见下"遗留"）。

## 验证记录（sandbox，/tmp 模拟 atd，已清理）

环境：python3 mock 服务端复刻 service.rs 行为（逐行 JSON、bad json 回复、
不主动关连接）+ uci 桩；用例见 /tmp/qmtest/run.sh（验证后已删除）。

1. `sh -n` / `bash -n` modem_util.sh 全部通过。
2. nc -U 主路径（mock 场景 ok）：
   `at` 输出 `+CSQ: 25,99`+`OK`、rc=0；`fastat` rc=0；请求 timeout_ms 断言
   at()=3000 / fastat()=1000；含双引号命令 `AT+CMGS="+86…"` 经 jq 转义往返无损。
3. mock 场景 timeout：输出已收部分行 `+CSQ: 25,99`、rc=1（对齐 tom_modem 超时
   语义）。
4. 连接失败（socket 不存在）：rc=1、stdout 空、m_debug 记录 "socket … not found"，
   且确认**无** tom_modem/sms_tool_q 回退尝试（grep 零命中）。
5. curl 回退路径（PATH 隐藏 nc 强制走 curl --http0.9）：实测取不回 sendat 响应
   （与手工 curl 实验一致）→ rc=1 报错。XFAIL 观察项：curl 对该行协议不可靠属
   客户端侧行为（裸 socket 重放同字节流可取回完整 7 行响应），回退仅作最后尝试，
   失败保持报错，不影响主路径。
6. at_tool=0/1 回归：桩环境确认分发仍路由至 tom_modem / sms_tool_q（沙箱无该
   二进制，出现 "not found" 即证明路由正确）。
7. 全程 pass=22（curl XFAIL 2 项除外），失败 0。

## 风险与遗留

- busybox nc 若未编 FEATURE_UNIX_LOCAL（OpenWrt 默认配置通常未编），`nc -U`
  会立即报错 → 自动落入 curl 回退；curl 不可靠时保持报错。建议后续给
  qmodem-atd 侧（或单独小工具）提供"响应后关连接"或 HTTP 封装选项，
  即可让 curl/README 示例真正可用。
- README/async-daemon.zh-cn.md 的 curl 示例待修正（超出本次授权范围，未改）。
- atd 侧 `end_flag` 参数未透传（服务端默认 end flags 与 C 版一致，够用；
  如后续 fastat 需要自定义 end flag 再扩展 dispatch 签名）。
