# Journal: vendor 全量迁移 qmodem_lib（Phase 2）

- **agent**: agent-vendor-full
- **日期**: 2026-10-12
- **范围**: `application/qmodem/files/usr/share/qmodem/vendor/` 下 10 个厂商脚本
  （fibocom/foxconn/gosuncn/meig/neoway/openluat/sierra/simcom/telit/thales）。
  quectel.sh、huawei.sh（Phase 1 试点）与 cmds/** 未改动。

## 目标与范式

沿用 Phase 1 确立的范式：在每个 vendor 头部 `source` 区追加
`source "${QMODEM_HOME:-/usr/share/qmodem}/cmds/qmodem_lib.sh"`，
将逐字节等价的管道替换为库函数：

1. `| tr -d '\r'` / `| sed 's/\r//g'` → `| ql_at_clean`
2. `| awk -F',' '{print $N}'`（含 `-F","`、`-F,`、`{ print $N }` 变体）→ `| ql_csv_field N`
3. 固定正则提取 → `| ql_regex_pick`（仅语义完全等价处）

**不迁移（保持原样，保证输出逐字节一致）**：

- `cut -d',' -fN`：实测与 awk 语义不等价——无逗号行 cut 输出整行而 awk 输出空行
  （ Gosuncn/foxconn/telit 等保留；gosuncn/telit 的 `cut -d:` 为 ':' 分隔，本就不属 ',' CSV）。
- 多字符 `tr -d '\r" '`、`tr -d '\r '`、`tr -d '\r" nNB '`：ql_at_clean 只删 \r，不等价。
- 带条件/循环/NF 逻辑的 awk（`{print (NF>=9)?$9:"0"}`、`{for(...)}`、`{print NF}` 等）。
- 含逗号引号字段处理或非常规定界（`awk -F'"'`、`awk -F'[:,]'`、`awk -F'mcc:'` 等）。
- 锚定正则：openluat.sh:859 `sed -n 's/^\([Mm]ode:\)\(...\)$/...'` 已加注释说明
  ql_regex_pick 的 `.*` 包裹会破坏 `^...$` 锚点与 head -n1 语义。

## 每文件迁移统计表

| 文件 | ql_at_clean | ql_csv_field | ql_regex_pick | 小计 | source | 回归用例 |
|---|---|---|---|---|---|---|
| fibocom.sh | 29 | 132 | 0 | 161 | 已加 | cell_info（208 行输出） |
| foxconn.sh | 3 | 5 | 1 | 9 | 已加 | cell_info + _get_temperature |
| gosuncn.sh | 8 | 4 | 0 | 12 | 已加 | cell_info_lte（73 行输出） |
| meig.sh | 11 | 55 | 0 | 66 | 已加 | cell_info 5G/EN-DC 两态 |
| neoway.sh | 21 | 38 | 0 | 59 | 已加 | cell_info（40 行输出） |
| openluat.sh | 0 | 0 | 0* | 0* | 已加 | get_mode（真实 fixture） |
| sierra.sh | 4 | 0 | 0 | 4 | 已加 | base_info |
| simcom.sh | 26 | 92 | 0 | 118 | 已加 | cell_info（213 行输出） |
| telit.sh | 15 | 6 | 0 | 21 | 已加 | get_lockband（166 行输出） |
| thales.sh | 1 | 7 | 1 | 9 | 已加 | cell_info + _get_temperature |
| **合计** | **118** | **339** | **2** | **459** | 10/10 | 12 用例全 PASS |

\* openluat 本就无 `awk -F','`/纯 CR 管道可迁（仅 1 处锚定 sed，不迁并注释）；其自身迁移点为注释说明。

各文件「重复管道的大多数」均已迁移：fibocom/meig/simcom/neoway 四个大文件
的 `awk -F','` 简单提取形式为 100% 迁移（131/131、66/66、92/92、39/39 → 剩余均为
条件/循环/NF 复杂形式，语义不可等价替换）。

## 迁移要点

- **替换规则由严格正则脚本执行**，仅命中三种精确模式；替换后逐文件 diff 复核，
  确认无计划外改动（meig:319 三元 print 中的 `sed 's/\r//g'` 尾段单独迁移为 `| ql_at_clean`，其余原样）。
- **函数导出与加载方式零变化**：10 个文件新旧函数定义清单逐字节一致
  （`grep -oE '^[a-zA-Z_0-9]+\(\)'` 对比通过），被 modem_dial/modem_util/rpcd
  调用的函数签名均未触碰；各 vendor 原 `source` 行为与顺序不变。

## 验证记录

1. **语法**：`sh -n` 全部 10 个文件 → 全部 OK。
2. **一致性回归**（/tmp/stub 沙箱，用后即删）：
   - 桩环境：jshn.sh / functions.sh / uci / ubus / at 确定性桩（新旧两侧同桩，
     桩只需确定性不需保真）；`at()` 桩按 AT 命令分发构造响应（含 CR/LF 原始格式）。
   - **openluat 用真实 fixture**：`testcases/openluat/unisoc/air724ug-c9340434/AT_SETUSB_-b51511f9.json`
     的 response_hex 解码文本（保留 CR）作为 `cmd_openluat_setusb_query` 输出，
     对比 `get_mode` 新旧输出 → 逐字节一致。
   - **其余 9 厂商用构造样本**（testcases/ 下无对应 fixture，注明）：
     sierra base_info、gosuncn cell_info_lte、foxconn cell_info/_get_temperature、
     thales cell_info/_get_temperature、neoway cell_info、simcom cell_info、
     telit get_lockband、fibocom cell_info、meig cell_info（5G 与 EN-DC 两种响应）。
     每用例覆盖该厂商 ql_ 调用最密集的解析函数，新旧输出 diff 全部为空（12/12 PASS）。
   - 已知桩噪声：sierra 用例中 `tom_modem` 未定义（modem_util.sh 既有缺口，
     非本次范围），新旧两侧一致，不影响对比有效性。
   - foxconn/thales 的 `^DEBUG?` 桩样本为 \n-only：真实响应的 `\r` 会被 xargs
     保留致 case 永不命中（新旧同样不命中，无对比价值），故构造样本去 CR 以覆盖分支。
3. **临时产物清理**：/tmp/orig、/tmp/stub、/tmp/migrate.py、/tmp/out_* 等已删除。

## 遗留与建议

- `cut -d','`（约 30 余处）如需迁移，需先扩展 ql_csv_field 语义或新增
  `ql_csv_field_cut`（保留整行回退），本次按「一致优先」原则不动。
- sierra.sh 头部 `debug_subject="quectel_ctrl"` 为既有问题，未越权修改。
