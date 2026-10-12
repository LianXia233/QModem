# Journal: vendor 残留症状清理（agent-vendor-residue）

日期: 2026-10-12
范围: 仅 `application/qmodem/files/usr/share/qmodem/vendor/*.sh`（12 个）
解释器基线: busybox ash；本机验证用 dash（POSIX 等价）+ sh -n

## 1. 扫描方法

- **重复 case 分支**: 解析每文件 `case…esac`（含 if/for 嵌套栈跟踪），对每分支模式行
  `^pattern(|pattern)*)` 按未转义 `|` 拆分后查重（同 case 内重复 `)` 模式 = 后者不可达）。
- **恒等管道**: 全量提取 `s/LHS/RHS/flags`，筛 `LHS==RHS` 及 `s/(x)/\1/` 捕获组恒等；同法查 `tr A A`。
- **裸 CR 清理**: grep `sed 's/\r//g'` 全部变体（含 `s/\r$//`、双引号、`tr -d '\r'`）。

## 2. 扫描结论（全 12 文件）

| 症状 | 真实命中 | 备注 |
|---|---|---|
| 同 case 重复 `)` 模式 | 0 | openluat.sh:1021/1027 为误报：模式是 `LTE\|*` 与 `GSM\|*`（转义竖线，非多分支），逐字节不同 |
| 恒等 sed/tr | 0 | `s/\.*0*$//`、`s/^\./0./` 等均为有效变换，非恒等 |
| catch-all `*` 前置遮蔽 | 0 | 无 |
| 裸 `sed 's/\r//g'`（活代码） | 8，全部在 huawei.sh | 见 §3 |
| 裸 `sed 's/\r//g'`（注释） | 2（huawei.sh:268、quectel.sh:386） | 注释不执行，不改 |

## 3. 修改点：huawei.sh 裸 CR 清理收编为 ql_at_clean（8 处）

`ql_at_clean()` 定义于 cmds/qmodem_lib.sh:12，实现为 `tr -d '\r'`，与 `sed 's/\r//g'` 逐字节等价。
huawei.sh:8 已 source qmodem_lib.sh；同文件 cell_info() 内已有 6 处既用 `ql_at_clean`，本批为 Phase 2 未覆盖的残留。

| 行(前) | 变量 | 修改 |
|---|---|---|
| 359 | ura | `\| sed 's/\r//g'` → `\| ql_at_clean` |
| 428 | nr_sinr | 同上 |
| 444 | endc_lte_rxlev | 同上 |
| 448 | endc_nr_sinr | 同上 |
| 462 | lte_rxlev | 同上 |
| 477 | wcdma_ura | 同上 |
| 492 | gsm_ta | 同上 |
| 691 | response (_get_temperature, ^CHIPTEMP) | 同上（含行尾多余空格消除） |

## 4. 对比证据

构造样本（`printf '\r\n^MONSC: …,789\r\n'` 与多行/空字段/双 CR 样本 cr2.txt），
dash 下对 12 号字段执行 `awk | sed 's/\r//g'` vs `awk | tr -d '\r'`，od -An -tx1 输出一致:
- `old=0a3738390a / new=0a3738390a` → BYTE-EQUAL
- 多行含空字段与 `z\r\r\n`: BYTE-EQUAL-2
- ^CHIPTEMP 全链路（`grep \^CHIPTEMP | awk '{print $6}'`）: CHIPTEMP-BYTE-EQUAL

fixture 复核: testcases 仅覆盖 quectel/openluat（无 huawei fixture）；
RM500U 的 QNWLOCK fixture（AT_QNWLOCK__common_5g_-84579503.json）解码为
`+QNWLOCK: "common/5g"`（无锁定参数，expected/get_neighborcell.json 中 NR/LTE 均 unlock），
无法证明锁定态 5G 字段布局，故未改任何字段索引（见 §5）。

## 5. 可疑字段复核（未改，证据不足）

- quectel.sh:965 `nr_lock_pci` 取 field 2 与 nr_lock_status 同源，疑似抄错；但同文件写侧格式互相矛盾
  （1310 行 `common/5g",$pci,$arfcn,scs,band` 无 stat vs 1414 行 `common/5g",1,$pci,…`），fixture 仅含
  unlock 响应，无法定 3/4/5 哪个正确 → 按"不确定的不改"原则保留，仅记录。
- get_neighborcell_unisoc（1149-1150）NR 侧 freq=2/pci=3 与同函数 LTE 侧 freq=2/pci=3 布局不一致同理存疑。
- huawei MONSC/CSERSI/CHIPTEMP 字段索引无 fixture 可对照，未发现自相矛盾证据，不动。

## 6. 验证

- `sh -n` 12/12 全过。
- `git diff`: 仅 huawei.sh，8 insertions / 8 deletions，无其它 vendor 文件变更；全文件 LF 无 CRLF。
- 重复分支重扫: 真实重复 0（openluat 命中为 `LTE\|*`/`GSM\|*` 误报，已逐字节核实为不同模式）。
- tests/test_vendor_fixtures.sh 在本机因缺 `xxd` 及 fixture 覆盖缺口失败（预先存在，与本改动无关；
  失败项均为 "command heads without fixtures" 清单，非断言失败）。
