# AT 通路选择：tom_modem / sms_tool_q / qmodem-atd

[简体中文](at-tool-selection.zh-cn.md) | [English](at-tool-selection.en.md)

> 范围说明：通路 0（tom_modem/ubus at-daemon）与通路 1（sms_tool_q）为既有实现，代码入口见 `application/qmodem/files/usr/share/qmodem/modem_util.sh` 的 `at()`/`fastat()`。**通路 2（qmodem-atd unix socket）为本版本引入的新通路**，其 shell 侧接线与实测记录对应 journal 尚在发布流程中，发布后将补入 [docs/refactor/](../refactor/README.md) 索引；以本文为准的描述在 journal 发布前以设计口径阅读。

## 1. 三通路对照表

所有 QModem shell 脚本（vendor 适配、拨号、短信）统一经 `at()` / `fastat()` 封装发 AT，二者按 `qmodem.main.at_tool` 的取值选择底层通路：

| | 通路 0 | 通路 1 | 通路 2（本版本引入） |
|---|---|---|---|
| 工具 | `tom_modem`（`$use_ubus_flag` 置位时经 ubus `at-daemon`） | `sms_tool_q` 直连串口 | `qmodem-atd` unix socket JSON 协议 |
| 串口占用 | tom_modem 直连时独占；ubus 模式由 at-daemon 排队 | shell 进程直连，独占 | 由 qmodem-atd 的 per-port actor 独占并串行排队 |
| 并发行为 | ubus 模式下 at-daemon 命令队列保证串行 | 并发调用各自开端口，可能互踩 | per-port mpsc 队列串行，每请求私有响应通道，不错配 |
| 超时语义 | `at()` 用 tom_modem 默认超时（`$options` 可附加参数）；`fastat()` 固定 `-t 1` | 同左：`at()` 默认、`fastat()` 追加 `-t 1` | 每请求显式 `timeout_ms`，单调时钟；超时返回已收部分行 + `timeout` 状态 |
| 依赖 | tom_modem / ubus 守护进程 | 仅 sms_tool_q 二进制 | qmodem-atd 二进制 + unix socket |
| 适用场景 | 默认通路，兼容面最广 | 轻量设备 / tom_modem 表现不佳的模组 | 多模组并发、需要 URC 订阅与稳定超时语义的场景 |
| 配置 | `at_tool='0'` 或未配置 | `at_tool='1'` | `at_tool='2'`（`at_socket` 指定 socket 路径） |

`fastat()` 在三条通路下语义一致：固定 1 秒超时（通路 2 换算为 `timeout_ms: 1000`），不附加 `$options`，用于轮询类短命令。

## 2. 配置方法

写入 `/etc/config/qmodem` 后重启主服务生效：

```sh
# 通路 2：qmodem-atd unix socket（本版本引入）
# 1) 先启用守护进程并登记 AT 口（/etc/config/qmodem-atd 的 port 段）
uci set qmodem-atd.main.enabled='1'
uci set qmodem-atd.main.socket='/var/run/qmodem/atd.sock'
uci commit qmodem-atd
/etc/init.d/qmodem-atd restart

# 2) 再把 AT 通路切到 qmodem-atd
uci set qmodem.main.at_tool='2'
uci set qmodem.main.at_socket='/var/run/qmodem/atd.sock'
uci commit qmodem

# 回到默认通路
uci set qmodem.main.at_tool='0'
uci commit qmodem
```

说明：

- `at_tool` 是全局选项，作用于所有模组的 `at()`/`fastat()` 调用；`luci-app-qmodem` 的 Settings 页提供 `Alternative AT Tools` 开关，对应 `at_tool='1'`（见 `luci/luci-app-qmodem/htdocs/luci-static/resources/view/qmodem/settings.js`）。
- 通路 2 要求 `qmodem-atd` 已运行并注册目标端口。守护进程的启动方式、端口注册与 JSON 协议细节见 [Rust 异步守护进程架构](async-daemon.zh-cn.md)。

## 3. 故障切换行为

- **通路 2 → 通路 0 回落**：`at_tool='2'` 时，socket 连接失败（守护进程未启动、路径错误、权限不足）的请求回落到 tom_modem 通路执行，并通过 `logger -t qmodem` 记录回落事件；回落不改变命令与超时语义。守护进程恢复后新请求自动回到 socket 通路。（本版本引入的行为，实测数据以对应 journal 发布为准。）
- **通路 0 / 通路 1 无自动切换**：工具缺失或命令失败按既有返回码向上传递，由调用方（拨号/短信脚本）处理重试。
- qmodem-atd 端口关闭或 actor 退出时，排队中的请求收到 `PortClosed` 错误而非挂死；这些错误同样触发上述回落。

## 4. 排查命令

```sh
# 日志：回落事件、通路选择与 AT 错误
logread -t qmodem
logread -e qmodem-atd

# 通路 0：确认 at-daemon ubus 对象存在（ubus 模式）
ubus list | grep at-daemon
ubus call at-daemon sendat '{"at_port":"/dev/ttyUSB2","at_cmd":"AT"}'

# 通路 2：socket 探活（列出 qmodem-atd 已注册端口）
curl --unix-socket /var/run/qmodem-atd.sock -d '{"cmd":"list"}' http://localhost/

# 通路 2：AT 探活
curl --unix-socket /var/run/qmodem-atd.sock \
     -d '{"cmd":"sendat","port":"/dev/ttyUSB2","at":"AT","timeout_ms":1000}' \
     http://localhost/
# 正常响应：{"status":"ok","lines":["OK"],"end_flag":"OK","elapsed_ms":...}
# {"status":"error","error":"unknown port"} → 端口未注册，见 async-daemon 文档的启动方式
```

日志与 AT 响应可能包含 IMEI、IMSI、手机号等敏感信息，公开前先脱敏。

## 5. 相关文档

- [Rust 异步守护进程架构](async-daemon.zh-cn.md)：qmodem-atd 的 actor 模型、JSON 协议与交叉编译
- [Rust 工作区 README](../../rust/README.md)：零依赖设计、体积数据与测试
- [AT fixture 格式与回放](../../testcases/README.md)：通路无关的 AT 响应回放测试
- [RPCD AT Daemon 用户指南](../rpcd-at-daemon-userguide-cn.md)：at-daemon 的 ubus/rpcd 调用方式
