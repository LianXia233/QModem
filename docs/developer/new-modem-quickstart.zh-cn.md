# 新模组快速上手：从 VID/PID 到合入

> English: [new-modem-quickstart.en.md](new-modem-quickstart.en.md)

本文是「拿到一台新模组（知道 USB VID/PID）→ 代码合入」的最短路径清单。
背景与旧版说明见 [How_to_add_modem_cn.md](../How_to_add_modem_cn.md)，
架构与分层约定见 [modem-adaptation.zh-cn.md](modem-adaptation.zh-cn.md)，
Phase 4 决策记录见 [ADR-0001](../adr/0001-refactor-phase4-decisions.zh-cn.md)，
本模板的落地过程记录见 [journal-newmodem-tpl](../refactor/journal-newmodem-tpl.md)。

## 步骤总览

| 步骤 | 产出 | 自动化工具 | 验证命令 |
| --- | --- | --- | --- |
| ① 登记 | modem_support.json 条目 | 手工编辑 + `jq` | `jq` 校验 + `check_modem_registry.py` |
| ② 采集 | recognition fixture | `qmodem_collect` | `qmodem_collect status` |
| ③ 起步 | cmds/vendor 脚本 | `cp vendor_template.sh` | `sh -n` |
| ④ 回放 | 回放通过的解析层 | fixture 回放 runner | `test_*_fixtures.sh` / `run_tests.sh` |
| ⑤ 表格 | support_list.md 条目 | `update_support_list.py` | `check_modem_registry.py` |
| ⑥ 提交 | PR | git + CI | CI 全绿 |

## ① modem_support.json 登记条目

VID/PID 是起点：USB VID 对应 `manufacturer_id`（小写十六进制，如 `1e0e`），
PID 决定同一 VID 下的条目名（如 `air724ug`）。往
`application/qmodem/files/usr/share/qmodem/modem_support.json` 的 `modem_support.usb`
下新增条目：

```json
"my-model": {
    "manufacturer_id": "1e0e",
    "manufacturer": "simcom",
    "platform": "qualcomm",
    "data_interface": "usb",
    "pdp_index": "1",
    "wcdma_band": "0",
    "lte_band": "1/3/5/8",
    "nsa_band": "0",
    "sa_band": "0",
    "modes": ["ecm", "rndis"]
}
```

字段含义：`manufacturer` 必须与 `vendor/dynamic_load.json` 的厂商名及
`cmds/<vendor>.sh` 文件名一致；`platform`（qualcomm/mediatek/unisoc/…）
决定 fixture 目录第二级；`modes` 是拨号方式白名单（qmi/ecm/mbim/rndis）。
`model` 的 UCI `name` 就是条目名（modem_name）。

验证：

```sh
jq -e '.modem_support.usb["my-model"]' \
  application/qmodem/files/usr/share/qmodem/modem_support.json
python3 scripts/check_modem_registry.py   # 此时②⑤未完成，允许 fixture 提示
```

## ② 识别 fixture 采集（真机，一次性）

在没有 `cmds/<vendor>.sh` 之前，识别流程会发送探测指令并落入
`recognition/pending/`。设备端采集（详见
[testcases/README.md](../../testcases/README.md)）：

```sh
uci set qmodem.main.testcase_collect=1 && uci commit qmodem
# 触发一次插拔/重扫，让识别流程跑完；再在 LuCI/ubus 触发 base_info 等功能
qmodem_collect status
qmodem_collect pack        # 默认脱敏并加密，输出 review key
```

开发机导入：

```sh
scp root@<device>:/tmp/qmodem_feedback_*.tar .
# 维护者 qmodem-seal decrypt 后：
scripts/import_testcases.sh qmodem_testcases_*.tar.gz
```

采集器会按最终识别出的 vendor/platform/model 把记录归档到
`testcases/recognition/<vendor>/<platform>/<model>/` 并写入
`expected_identity`；无法归属的 pending 记录导入脚本会拒绝。
检查归档结果：

```sh
ls testcases/recognition/<vendor>/<platform>/<model>/
jq -e '.expected_identity' testcases/recognition/*/*/*/ATI-*.json
```

## ③ 复制模板起步

```sh
cd application/qmodem/files/usr/share/qmodem
cp cmds/vendor_template.sh cmds/<vendor>.sh
cp cmds/vendor_template.sh vendor/<vendor>.sh   # 或按头部注释拆两层
```

模板内每段都有「此处填什么」注释：识别探测（`cmd_<vendor>_identity_query`）、
`get_imei`/`sim_info`/`base_info`/`network_info`（信号解析一律走
`cmds/qmodem_lib.sh` 的 `ql_at_clean`/`ql_csv_field`/`ql_regex_pick` 范式）、
可选的 band/锁网（`get_lockband`/`set_network_prefer`，不支持就在
`vendor_get_disabled_features` 声明屏蔽）、拨号配合点（`get_mode`/`set_mode`，
`modes` 与 ① 的 JSON 字段一致）。把手写的 `myvendor`/占位符统一替换为厂商名。

验证：

```sh
sh -n cmds/<vendor>.sh && sh -n vendor/<vendor>.sh
```

老厂商新增型号则跳过本步，只做 ①（并按需在既有 `cmds/<vendor>.sh` 补封装）。

## ④ fixture 回放验证（无硬件）

```sh
# recognition fixture 回放 + vendor 只读方法 JSON 合法性 + expected 黄金比对
bash application/qmodem/tests/test_recognition_fixtures.sh
bash application/qmodem/tests/test_vendor_fixtures.sh
# 仓库级 shell 回归（含 qmodem_lib 边界用例）
bash tests/sh/run_tests.sh
```

三层校验：路径与元数据一致、按设备画像回放 `at`/`fastat`、
输出 JSON 且与 `expected/<method>.json` 精确比对。想给解析函数补黄金输出，
在 `testcases/<vendor>/<platform>/<model>-<md5前8位>/expected/` 下放
`<method>.json`。测试分层说明见
[testing.zh-cn.md](testing.zh-cn.md)。

## ⑤ support_list.md 登记

```sh
python3 scripts/update_support_list.py docs/support_list.md
python3 scripts/check_modem_registry.py --fail-on-fixtures
```

`check_modem_registry.py` 交叉核对 modem_support.json、support_list.md、
vendor 函数表与 recognition fixture 四处注册源，缺失会逐条报告。

## ⑥ PR 检查单

- [ ] `modem_support.json` 条目字段齐全，`modes` 与实机一致（jq 校验通过）
- [ ] `cmds/<vendor>.sh` 只含 `cmd_*` 封装；`sh -n` 通过；无 bashism
- [ ] `vendor/<vendor>.sh` 实现 `get_imei`/`sim_info`/`base_info`/`network_info`，解析走 `ql_*`
- [ ] 不支持的功能已在 `vendor_get_disabled_features` 声明
- [ ] recognition fixture 已入库且 `expected_identity` 正确（回放脚本通过）
- [ ] `scripts/check_modem_registry.py` 退出码 0
- [ ] `docs/support_list.md` 已更新（脚本生成）
- [ ] CI（quality.yml / vendor-fixtures.yml）全绿；声明硬件验证层级

提交前用 `git diff --check` 确认无 CRLF/行尾空白（本仓库统一 LF）。
