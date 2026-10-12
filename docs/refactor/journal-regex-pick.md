# Journal: ql_regex_pick 多行语义调用点审计（agent-regex-pick）

- 日期: 2026-10-12
- Agent: agent-regex-pick
- 范围: `application/qmodem/files/usr/share/qmodem/vendor/*.sh` 中
  `ql_regex_pick` 调用点；固定用例追加至 `tests/sh/cases/case_qmodem_lib.sh`

## 背景

tests-expand 已确认 `cmds/qmodem_lib.sh` 中 `ql_regex_pick` 的实际语义为
**「每个匹配行输出一行」**（`sed -n -E "s/.*<ERE>.*/\1/p"`），而非
「首行」。库头部注释已修正，并要求单值消费方自行 `head -n1` 或确保输入
至多一行匹配。本任务审计全仓 3 处出现点并消除单值消费方被多行污染的风险。

## 逐点定位与结论

### 1. vendor/foxconn.sh:467（真实调用，已加固）

- 上游: `cmd_temp_query "$at_port" "$at_pre"` → `at "$1" "${2}temp?"`，
  即 `AT+temp?` 或 `AT^temp?`（依平台 at_pre）。
- 模式: `'TSENS: ([0-9]*)C'`，调用方将结果放入单值
  `temperature`，再写 `"temperature" "$temperature C"`。
- 证据: 仓库内唯一温度类 fixture
  `testcases/quectel/unisoc/rm500u-cnv-49196729/AT_QTEMP-81361007.json`
  的 `response_hex` 解码为 **每传感器一行**（soc/pa/pa5g/board 共 4 行
  `+QTEMP:`），证明温度类 AT 命令逐行多传感器输出是本仓库实际观察到的
  常态；foxconn/thales 的 TSENS 查询属同一域。多行匹配可能性成立，
  而调用方只取单值 → **按预案加 `| head -n1`**。

### 2. vendor/thales.sh:483（真实调用，已加固）

- 上游: `cmd_temp_query "$at_port"` → `AT^TEMP?`（注释引用手册 15.13）。
- 同一模式、同一单值消费方式；无 thales fixture，但同上按温度域多行
  常态处理 → **加 `| head -n1`**。

### 3. vendor/openluat.sh:858（非调用，注释）

- 内容为「不迁移 ql_regex_pick」的决策注释（模式含 `^...$` 锚点），
  无代码调用，无需改动。

## 输出一致性论证

`head -n1` 对单行输出逐字节透传（仅按行取前 1 行，不改内容），
命令替换本就吞掉尾部换行，故单行匹配场景下新旧输出逐字节一致；
多行匹配场景下由 `45\n50` 收敛为 `45`，与迁移前「取首行」语义对齐，
避免 `"temperature": "45\n50 C"` 类污染。

## 验证记录

1. `sh -n`：foxconn.sh / thales.sh / case_qmodem_lib.sh 全部通过；
   LF 检查无 CRLF。
2. 套件：`bash tests/sh/run_tests.sh` → PASS=36 SKIP=0 FAIL=0，
   含新增用例 `test_ql_regex_pick_single_value_consumer`。
3. 构造样本前后对比（/tmp，bash + source qmodem_lib.sh）：
   - 单行 `TSENS: 33C`：旧 `[33]` / 新 `[33]` —— 逐字节一致；
   - 双匹配行 `TSENS: 45C` + `TSENS: 50C`：旧 `[45\n50]` /
     新 `[45]` —— 多行风险消除。
   - 双传感器但仅首行匹配 TSENS（`PA:` 行）：新旧均为 `[45]`。

## 固定用例

`tests/sh/cases/case_qmodem_lib.sh` 追加
`test_ql_regex_pick_single_value_consumer`：多传感器构造样本经
`ql_regex_pick 'TSENS: ([0-9]*)C' | head -n1` 仅取首值，且单行输入
逐字节透传，钉死 vendor 调用点契约。
