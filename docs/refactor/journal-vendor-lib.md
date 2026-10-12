# journal-vendor-lib：公共解析库与 vendor 试点迁移

- 标签：agent-vendor-lib
- 日期：2026-10-12
- 范围：`cmds/qmodem_lib.sh`（新增）、`vendor/quectel.sh`、`vendor/huawei.sh`（试点迁移）

## 背景

`vendor/*.sh` 共 12,126 行，「query → grep → awk → sed」解析管道复制 13 份；`sed 's/\r//g'` 在 quectel.sh 内出现约 29 次（含变体更多）。本次建立公共解析库并对两个最大厂商试点迁移，其余厂商渐进跟进。

## 新增：cmds/qmodem_lib.sh

POSIX / busybox ash 兼容，仅用 busybox applet（tr/sed/awk），按 `cmds/` 现有 source 约定加载：

```sh
source "${QMODEM_HOME:-/usr/share/qmodem}/cmds/qmodem_lib.sh"
```

| 函数 | 语义 | 替代 |
|---|---|---|
| `ql_at_clean` | 删除流中全部 CR（与 `sed 's/\r//g'` 逐字节等价，tr 实现） | ~50 处 CR 清洗管道 |
| `ql_csv_field <n>` | stdin 每行按 ',' 取第 n 字段（1 起，`awk -F',' -v n` 实现，缺字段输出空行，不清洗 CR） | `awk -F',' '{print $n}'` |
| `ql_regex_pick <ERE>` | 提取首个匹配行的第一个捕获组（busybox sed -E 实现，不依赖 grep -P）；无非空匹配返回 1 | `sed -n 's/.*re.*/\1/p'` |
| `ql_num_or <default>` | 去除空白后校验非负整数，非法输出默认值并返回 0；参数错误返回 2 | 散落的数值兜底 |
| `ql_bool` | 1/true/yes/on/enabled→1，0/false/no/off/disabled→0，否则无输出返回 1；接受 $1 或 stdin | 布尔归一化 |
| `ql_kv_get <key>` | 从 "k: v" 行提取值（key 为 ERE，`.*[^[:space:]]` 定界剥离尾部空白/CR）；无匹配返回 1 | `grep \| sed 's/k: //'` |

失败语义：所有函数 0=成功、1=无匹配/非法值、2=用法错误；均只读 stdin、只写 stdout。

## quectel.sh 迁移点清单（前 → 后）

头部新增 `source .../cmds/qmodem_lib.sh`（与 cmds/quectel.sh 同一加载机制）。共 20 处函数、35 个管道段：

| # | 函数（原行号） | 前 | 后 |
|---|---|---|---|
| 1 | get_mode L71 | `sed 's/\r//g'` | `ql_at_clean` |
| 2 | get_network_prefer_lte L224 | `sed 's/\r//g'` | `ql_at_clean` |
| 3 | get_network_prefer_nr L235 | `awk -F',' '{print $2}' \| sed CR` | `ql_csv_field 2 \| ql_at_clean` |
| 4 | get_voltage L343 | `awk -F',' '{print $3}' \| sed CR` | `ql_csv_field 3 \| ql_at_clean` |
| 5 | base_info L380/L382/L384 | `sed 's/\r//g'` ×3 | `ql_at_clean` ×3 |
| 6 | sim_info imei L426 | `sed 's/\r//g'` | `ql_at_clean` |
| 7 | sim_info imsi L453 | `sed 's/\r//g'` | `ql_at_clean` |
| 8 | get_signal_info rat_num L497 | `awk -F',' '{print $4}' \| sed CR` | `ql_csv_field 4 \| ql_at_clean` |
| 9 | get_signal_info CSQ L502 | `sed 's/\r//g'` | `ql_at_clean` |
| 10 | get_signal_info ambr_ul L525 | `awk -F',' '{print $6}' \| sed CR` | `ql_csv_field 6 \| ql_at_clean` |
| 11 | get_signal_info updown L533 | `sed 's/\r//g'` | `ql_at_clean` |
| 12 | get_neighborcell_qualcomm L962-970 | `awk -F',' '{print $N}' \| sed CR` ×9 | `ql_csv_field N \| ql_at_clean` ×9 |
| 13 | get_neighborcell_lte 锁小区 L1071-1074 | `awk -F',' '{print $N}'`（含 1 处 sed CR） | `ql_csv_field N`（1 处加 `ql_at_clean`） |
| 14 | get_neighborcell_lte LTE 邻区块 L1120-1124 | `awk -F',' '{print $N}'` ×4 | `ql_csv_field N` ×4 |
| 15 | cell_info EN-DC endc_lte_srxlev L1555 | `awk $18 \| sed CR` | `ql_csv_field 18 \| ql_at_clean` |
| 16 | cell_info EN-DC scs_num L1568 | `awk $16 \| sed CR` | `ql_csv_field 16 \| ql_at_clean` |
| 17 | cell_info NR-SA nr_srxlev L1638 | `awk $17 \| sed CR` | `ql_csv_field 17 \| ql_at_clean` |
| 18 | cell_info LTE lte_srxlev L1661 | `awk $20 \| sed CR` | `ql_csv_field 20 \| ql_at_clean` |
| 19 | cell_info WCDMA wcdma_com_mod L1681 | `awk $17 \| sed CR` | `ql_csv_field 17 \| ql_at_clean` |
| 20 | get_current_band nsa scs L1780 | `awk $16 \| sed CR` | `ql_csv_field 16 \| ql_at_clean` |

剩余 1 处 `sed 's/\r//g'` 为注释行，未动。

## huawei.sh 迁移点清单（前 → 后）

头部新增 `. .../cmds/qmodem_lib.sh`（沿 huawei.sh 自身的 `.` 加载风格）。共 11 处：

| # | 函数（原行号） | 前 | 后 |
|---|---|---|---|
| 1 | get_mode L44 | `sed -n '2p' \| sed 's/\r//g'` | `sed -n '2p' \| ql_at_clean` |
| 2 | sim_info imsi L247 | 同上 | `ql_at_clean` |
| 3 | sim_info imei L250 | 同上 | `ql_at_clean` |
| 4 | base_info name L261 | 同上 | `ql_at_clean` |
| 5 | base_info manufacturer L263 | 同上 | `ql_at_clean` |
| 6 | base_info revision L265 | `sed 's/Revision: //g' \| sed CR` | `sed 's/Revision: //g' \| ql_at_clean` |
| 7 | cell_info cell_rat L285 | `awk -F',' '{print $1}'` | `ql_csv_field 1` |
| 8 | cell_info NR nr_sinr L304 | `awk $11 \| sed CR` | `ql_csv_field 11 \| ql_at_clean` |
| 9 | cell_info LTE-NR lte_rxlev L321（及同构 LTE 块） | `awk $10 \| sed CR` | `ql_csv_field 10 \| ql_at_clean` |
| 10 | cell_info LTE-NR second_nr_sinr L325 | `awk $14 \| sed CR` | `ql_csv_field 14 \| ql_at_clean` |

剩余 9 处 CR 清洗（WCDMA/GSM 块、CHIPTEMP 等）待后续批次跟进。

## 验证

1. `sh -n`：cmds/qmodem_lib.sh、vendor/quectel.sh、vendor/huawei.sh 全部通过。
2. 一致性回归（临时脚本 /tmp/ql_lib_regression.sh，验证后已删除）：
   - 方法：从 `testcases/quectel/unisoc/rm500u-cnv-49196729/` 抽取真实 AT fixture JSON（CSQ、QENG=servingcell、QENG=neighbourcell、QNWLOCK=common/lte、QNWCFG=up/down、ATI、CGSN），将 `response_hex` 解码为原始文本（保留 CR）作为 stdin，同输入分别喂迁移前旧管道与迁移后新管道，逐字节比对；另含 ql_num_or/ql_bool 语义单测。
   - 结果：**30/30 PASS，fail=0**，输出逐字节一致。
   - 过程中发现并修复 ql_kv_get 尾部 CR 残留问题（sed 贪婪组不能回吐尾部空白，改用 `.*[^[:space:]]` 定界）；bool invalid 首次失败为测试脚本自身写法错误，非库缺陷。
3. 无新增外部命令依赖：库仅使用 tr/sed/awk/printf（busybox 内置 applet）。

## 后续

- 其余 11 个 vendor 按同模式渐进迁移（优先 fibocom/simcom/openluat 三个最大文件）。
- quectel 中带 `sed 's/"//g'`、`awk -F'"'` 的引号类提取可另设 `ql_quote_field` 收编。
