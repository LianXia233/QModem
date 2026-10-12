# QModem v1.0.0 发布说明（中文）

- 发布日期：2026-10-12
- 版本类型：Release Candidate（预发布，main 分支）
- 基线：`main@c49654e`；完整变更清单见 [CHANGELOG.md](../../CHANGELOG.md) /
  [CHANGELOG.zh-CN.md](../../CHANGELOG.zh-CN.md)

## 本次 rc1 的三大主线

### 1. 止血加固（审计驱动的安全与内存安全修复）

一轮覆盖 C 内存安全、shell 注入面、打包与 LuCI 的系统性加固（27+ 文件）：

- **远端（modem 可控）输入不再可达 root shell**：`ubus_at_daemon` 事件回调改用
  `execl` 位置参数；`modem_dial.sh` APN 凭据 shell 引号处理；旧 LuCI controller
  命令注入与短信 XSS 修复；`modem_scand` 控制套接字 `0666` → `0660` + 白名单。
- **内存安全**：use-after-free、自死锁、越界读/写、无界 `sprintf`、
  UCS2→UTF-8 转换越界等一批修复；`-Wall -Wextra` 清零。
- **正确性**：SMS SCTS 时区 bug（+5:30/+8:00/±12:45 此前全错）修复；
  rpcd `delete_sms`/`send_raw_pdu`/`send_sms` 静默返回空修复。

### 2. Rust 零依赖异步核心

- `rust/` 新增 `qmodem-pdu`（3GPP TS 23.038 SMS PDU 编解码，带边界检查，
  64 个一致性测试）与 `qmodem-atd`（线程-actor AT 守护进程：每端口命令队列、
  单调时钟超时、URC 广播、unix-socket JSON 测试 API）。
- **零外部 crate**：二进制体积较此前 tokio 构建缩小 43.4%（748 KB → 更小）；
  微基准 p50 往返 50ms、31.4k cmd/s。
- 新增 AT 通路 `qmodem.main.at_tool=2`，将 AT 命令路由到 `qmodem-atd`
  （详见 [AT 工具选择](../developer/at-tool-selection.zh-cn.md)）。

### 3. 单包交付

- 新增 `qmodem-suite` 元包：一次安装交付全栈
  （`qmodem` + `luci-app-qmodem-next` + `qmodem-smsd` + `qmodem_monitor` +
  `luci-app-qmodem-monitor`），uci-defaults 引导在首次安装时启用全部服务。
- 修复旧缺陷：`qmodem` 此前安装时不落地 5 个 init.d 脚本。
- 安装指南：[install-suite.zh-cn.md](../user/install-suite.zh-cn.md)。

## 用户升级注意事项

1. **`qmodem` 现在会安装 init.d 脚本并启用服务**。修复了此前"装了包但服务
   不启动"的问题。从旧版本升级时，升级流程会自动启用 qmodem 相关服务；
   如果你此前依赖手动启动/停用行为，请在升级后检查
   `/etc/init.d/qmodem*` 的 enabled 状态。
2. **`at_tool=2` 是可选新通路，默认不变**。默认 AT 通路仍走原有实现；
   只有显式设置 `qmodem.main.at_tool=2` 才会切换到 Rust `qmodem-atd`
   的 unix-socket JSON 协议。rc1 阶段建议先在非关键设备上试用。
3. **TTL 功能由 `ttlfw4` 提供**。TTL 相关能力在 next 栈中暂无独立对应物，
   继续使用 `luci-app-qmodem-ttlfw4`（后续版本计划合并）。
4. 二进制/固件产物获取方式见 Release 页面附录（main.yml SDK 构建、
   rust-build.yml 交叉编译，产物不直接挂在 Release 上）。

## 已知限制

- **qfirehose 上游存在 3 个高危问题，待上游补丁/隔离**：升级包路径的
  `system()` 命令注入、XML 驱动的缓冲区溢出、未认证 TCP→USB 直通。
  详见 [docs/security/vendored-audit.md](../security/vendored-audit.md)。
  在补丁落地前，请勿将不可信固件包交给 qfirehose 刷写。
- **ubus FFI（`ubus_ffi` + libubus）需 SDK 实机验证**：纯 Rust blobmsg 编解码
  已通过字节级兼容测试，但 libubus FFI 绑定尚未在真实 OpenWrt SDK
  构建的设备上完成验证，`at_tool=2` 的 ubus feature 默认关闭。
- 旧栈退役 Step 2（`get_modem_cfg` 等价端点与子包 DEPENDS 切换）及 Phase 4
  旧栈删除属 v1.2.0 范围，本次 rc1 保留双栈共存。
