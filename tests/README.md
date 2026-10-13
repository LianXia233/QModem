# tests/ — 轻量无依赖回归测试

本目录为"单包交付"回归测试入口，不引入 pytest 等框架，仅依赖 busybox/标准工具（bash、sed、awk、grep、jq；lua 检查工具缺失时自动 SKIP）。

## 本地运行

```sh
# shell 函数级单测（微型断言框架 + 用例组）
./tests/sh/run_tests.sh

# 旧栈 lua 语法冒烟（luac -p 或 lua loadfile；工具缺失则 SKIP）
./tests/lua/smoke.sh
```

输出每条用例一行 `PASS/SKIP/FAIL`，末尾汇总 `PASS=x SKIP=y FAIL=z`。

## CI 接口约定（供 ci.yml 调用方使用）

| 入口 | 退出码 0 | 退出码 1 |
| --- | --- | --- |
| `./tests/sh/run_tests.sh` | 全部 PASS 或 SKIP | 任一 FAIL |
| `./tests/lua/smoke.sh` | 语法全过或工具缺失（SKIP） | 任一文件语法错误 |

- SKIP 不影响退出码，用于"依赖未交付"（如 `cmds/qmodem_lib.sh` 未落地）或"工具缺失"（如 luac/lua）场景，保证 CI 不因执行顺序失败。
- 建议在 ci.yml 中两条命令分别调用并合并判定，不要加任何参数。

## 目录结构

- `sh/framework.sh` — 几十行的 assert_eq/assert_rc/assert_contains/assert_nonempty 断言器；测试在子 shell 中执行，`fail()`/`skip()` 只中止当前用例。
- `sh/cases/` — 用例组（`case_*.sh` 自动发现加载）：
  - `case_modem_util.sh` — modem_util.sh 的 at 命令下发/清洗行为、at_tool=2（qmodem-atd unix socket）dispatch 与传输层 mock（socat 假服务，缺失则 SKIP）、testcase 路径 slug 清洗、布尔判定；
  - `case_rpcd_valid_section.sh` — rpcd/qmodem 的 `valid_section`（sed 提取函数体后 source，规避脚本顶层 jshn 依赖与 `call` 分支的 `read` 阻塞；提取失败则 SKIP）；
  - `case_rpcd_endpoints.sh` — rpcd 新端点 `get_stats`/`get_current_band` 家族：mock jshn json_* API + sed 提取 generic/quectel 实现函数，钉住累计计数透传与不可用回退、supported:false 分支（脚本或函数缺失则 SKIP）；
  - `case_qmodem_lib.sh` — `cmds/qmodem_lib.sh` 的 ql_* 助手（库文件未交付或函数缺失时逐条 SKIP）；
  - `case_fixture_parse.sh` — 用 testcases/ 两个真实 AT+CGSN fixture 做"解码 response_hex → 喂 vendor get_imei()"解析一致性冒烟（路径硬编码）。
- `lua/smoke.sh` — `luci/*/luasrc/**/*.lua` 语法检查（luci-app-qmodem 已改为纯 JS 前端，无 luasrc）。
- `browser/` — 既有 Playwright 前端冒烟（独立运行，不属于本套件）。

## 用例发现机制

`run_tests.sh` 只 source `sh/cases/case_*.sh`；新用例组按现有文件的 `t test_xxx` 注册模式追加即可，无需改入口。
