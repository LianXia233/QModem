# Journal: 新模组接入模板与快速上手文档（agent-newmodem-tpl）

- 日期: 2026-10-12
- Agent: agent-newmodem-tpl
- 范围: `cmds/vendor_template.sh`（新增）、
  `docs/developer/new-modem-quickstart.zh-cn.md` / `.en.md`（新增）、
  `docs/How_to_add_modem_cn.md`（追加快速上手链接）、
  `docs/refactor/journal-newmodem-tpl.md`（本文件）

## 目标

降低「新模组接入」门槛：给新厂商一个可直接 source 的脚本骨架（每段标注
「此处填什么」），并配一份从 VID/PID 出发的 6 步双语清单，把既有的
qmodem_collect / fixture 回放 / check_modem_registry 工具串成闭环。

## 交付内容

### 1. cmds/vendor_template.sh

- 按现有 vendor 两层结构组织：`cmd_*` AT 封装层（识别探测
  `cmd_myvendor_identity_query` 仿 `cmds/openluat.sh` 的 case 白名单范式）
  与特性/解析层（`get_imei`/`sim_info`/`base_info`/`network_info` 仿
  `vendor/sierra.sh` 的最小函数集）。
- 所有解析一律走 `cmds/qmodem_lib.sh` 范式：`ql_at_clean`（取行后去 CR）、
  `ql_csv_field`（CESQ 等逗号响应取列）、`ql_regex_pick`（单值消费统一
  `head -n1`，遵循 journal-regex-pick 确认的多行语义）。
- band/锁网/写 IMEI 标注为可选：不支持就删除并在
  `vendor_get_disabled_features` 声明屏蔽，避免框架调用缺失方法。
- 拨号配合点 `get_mode`/`set_mode`：`available_modes` 来自
  modem_support.json 的 `modes`（UCI `qmodem.<section>.modes`）。
- 依赖加载带 `command -v` 守卫：`at`/`ql_at_clean` 已存在时不重复 source，
  因此模板可在独立 /tmp 环境（stub generic 助手）下直接 source。

### 2. new-modem-quickstart 双语文档

6 步：① modem_support.json 登记（VID→`manufacturer_id`、字段含义、
`jq` 验证）→ ② 识别 fixture 采集（`qmodem_collect` + `recognition/pending`
归档 + `expected_identity`，引用 testcases/README.md 与
How_to_add_modem_cn.md）→ ③ 复制模板起步（`sh -n` 验证）→ ④ fixture 回放
（`test_recognition_fixtures.sh` / `test_vendor_fixtures.sh` /
`tests/sh/run_tests.sh`）→ ⑤ `update_support_list.py` +
`check_modem_registry.py` → ⑥ PR 检查单。每步标注自动化工具与验证命令，
并与 ADR-0001（docs/adr/0001-refactor-phase4-decisions）交叉链接。

### 3. How_to_add_modem_cn.md

顶部追加快速上手链接（中英），原文保留为背景说明，不动旧结论。

## 自验证记录

- `sh -n` + `dash -n` 通过（初版函数名占位符 `cmd_<vendor>_…` 含 `<` 破坏
  POSIX 解析，已改为可解析的 `myvendor` 占位，并在注释中说明替换规则）。
- 独立 /tmp 环境：stub `at`/`json_add_*`/`add_plain_info_entry` 等助手后
  `dash` source 模板（sed 替换占位符为 acme），关键函数
  （get_imei/sim_info/base_info/network_info/get_mode/set_mode/
  vendor_get_disabled_features/cmd_*_identity_query）全部 `command -v`
  命中；source 前后 `command -v ql_csv_field` 不变，确认不遮蔽
  qmodem_lib 函数。
- 模板解析管道样例自检：`+CESQ: 47,2,28,60,,` → regex_pick → csv_field 3
  输出 `28`。
- 文档相对链接逐条 `ls` 核对（zh/en 各 7 条，How_to 2 条）全部存在。
- 行尾均为 LF（`file` 无 CRLF 标记）。

## 遗留

- `get_network_prefer` 的示例 code 映射（2/3/38/71）是占位表，接入者须按
  实际模组 AT 手册替换（模板内已标注）。
- CESQ 枚举值到 dB 的映射未内置，模板注释建议改厂商专用指令或映射表。
- 模板未覆盖 SMS/锁小区等扩展方法，接入时参考 vendor/simcom.sh 与
  modem_ctrl.sh 的 feature 名单。
