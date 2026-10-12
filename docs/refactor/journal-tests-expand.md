# journal-tests-expand — 回归测试扩展（agent-tests-expand）

日期：2026-10-12
范围：仅 `tests/**` 与本文档；未触碰任何产品代码。

## 交付内容

### 1. rpcd 新端点测试 — `tests/sh/cases/case_rpcd_endpoints.sh`（新增）

覆盖 agent-rpcd-gap 交付的 `get_stats` / `get_current_band` / `clear_stats` 端点族。

- 手法沿用既有 `case_rpcd_valid_section.sh` 的既定决策：rpcd 脚本与
  generic.sh/quectel.sh 均无法在开发机上整体 source（顶层 jshn.sh 为
  OpenWrt-only 路径、`call)` 分支 `read -r input` 会阻塞 stdin），故用
  行锚定 sed 范围（`^func()/,/^}/`）提取函数体后 source。
- jshn 的 `json_*` API 用"调用序列记录"mock 替代，断言输出的 JSON 结构。
- 实现存在性确认：rpcd methods 表 + dispatch 分支（grep 级钉住）、
  generic.sh `get_current_band` / `get_current_band_capabilities` /
  `get_usage_stats`、quectel.sh `get_usage_stats` 均已在当前树存在；
  若未来函数改名/迁移，`_ep_load` 自动 SKIP（不 FAIL），延续 CI 契约。

### 2. get_stats "增量语义" 钉约（重要澄清）

全仓检索（delta/增量/increment）确认：**端点层没有任何增量/delta 计算**。
`get_stats` 的实际语义是"透传 modem 自身累计计数器"：

- quectel：`+QGDNRCNT/+QGDCNT` 有 marker → `available=1`、
  `updated_at=now`、rx/tx 原样透传（111222/333444 直通断言）；
- 无 marker（ERROR 等）→ `available=0`、`updated_at=0`、计数全零回退，
  非数值计数强制归零；
- generic 兜底实现即全零不可用快照。

测试把"累计透传 + 显式不可用回退"钉为契约，并在用例注释中写明：
增量/差值逻辑归消费方（LuCI stats 视图）负责。若后续有人给端点加
delta 缓存，此契约测试会显式失败提醒。

### 3. qmodem_lib 边界用例 — `case_qmodem_lib.sh` 追加 4 条

- `ql_csv_field`：空字段（`a,,c` → 空输出行）、越界字段（awk $n 契约）、
  引号不特殊（`"a,b",c` 按 `,` 拆分、引号原样保留，钉死"byte-identical
  awk -F','" 文档契约）；
- `ql_regex_pick` 多行输入（见下方发现）。

### 4. modem_util at_tool=2 dispatch 测试 — `case_modem_util.sh` 追加 6 条

任务问"构造 fake curl/nc？函数覆盖不可行则跳过"。结论：**函数覆盖可行**，
分两层实现并记录决策：

- dispatch 层：uci stub 回 `at_tool=2`，在 source modem_util.sh **之后**
  用同名函数覆盖 `qmodem_atd_sendat`（bash 后定义覆盖先定义），验证
  `at()` 传 3000ms / `fastat()` 传 1000ms、stdout 与 rc 透传；
- 传输层：`[ -S sock ]` 是 builtin 无法函数覆盖，故用可选 socat 假服务
  模拟 qmodem-atd JSON line protocol，覆盖 ok/timeout/error 三种 status
  映射；socat 缺失时 SKIP（原因记录在 SKIP 行）。jq/jsonfilter 均缺失
  时同样 SKIP；
- curl 回退分支：把 `nc` 覆盖为空函数（模拟 busybox 无 FEATURE_UNIX_LOCAL），
  断言 `--http0.9` 路径仍交付 ok 响应；
- socket 缺失 → rc 1 且**不回退** at_tool 0/1（bridge 契约）。

### 5. lua smoke（任务 3）

`tests/lua/smoke.sh` 本身即 find 自动发现，无需改动。本次工作树中 5 个
改动的 lua（TTL/HC/MWAN 退役标注）全部在覆盖范围内：`luac -p` 16 个
文件 PASS=16 FAIL=0。

## 过程中发现的问题（留给对应 owner，不在本任务修复）

1. **qmodem_lib.sh 头注释与实现不符**：`ql_regex_pick` 注释称
   "first matching line wins"，实现（`sed -n -E s///p`）实际对每个匹配行
   各输出一个值。测试按实现行为钉约（`test_ql_regex_pick_multiline_all_matches`），
   库 owner 应二选一：修注释或改实现（改实现会破坏本测试，属预期提醒）。
2. **bash 动态作用域陷阱**（记录给写 mock 的后人）：uci mock 内引用测试
   局部变量 `$sock` 会被 `qmodem_atd_socket` 自己的 `local sock` 遮蔽
   （被调函数可见调用链上最近的同名局部），mock 必须用独立命名的全局
   变量（`QMUT_ATD_SOCK`）。同样，`out=$(at ...)` 命令替换子 shell 会
   丢弃 mock 函数写入的全局变量，需用临时文件捕获 stdout。
3. socat 假服务注册了 EXIT trap 兜底回收，避免用例中途 abort 泄漏进程。

## 验证

- `bash -n` / `sh -n` 全部用例文件通过；
- `./tests/sh/run_tests.sh`：**PASS=35 SKIP=0 FAIL=0**（新增 16 条全 PASS；
  本机 socat/jq 可用故无 SKIP；CI 缺 socat 时对应传输层用例自动 SKIP，
  预计 PASS=31 SKIP=4）；
- `./tests/lua/smoke.sh`：PASS=16 FAIL=0；
- 连跑两次结果一致；复跑后 `pgrep socat` 确认无进程泄漏。

## 涉及文件

- 新增 `tests/sh/cases/case_rpcd_endpoints.sh`
- 追加 `tests/sh/cases/case_qmodem_lib.sh`（+4 用例）
- 追加 `tests/sh/cases/case_modem_util.sh`（+6 用例与 atd 辅助函数）
- 更新 `tests/README.md`（用例组清单同步）
