# journal: 模组注册一致性门禁（agent-support-sync）

日期：2026-10-12 ｜ 任务：建立「模组注册一致性」门禁，保证新模组三处登记齐全、上游同步不遗漏。

## 交付物

1. `scripts/check_modem_registry.py`（新建，仅标准库，LF）——交叉核对四处数据：
   - `modem_support.json`：usb/pcie 条目必需字段校验（manufacturer/platform/data_interface/pdp_index/modes，modes 非空 list）；device bus 校验 platform→vendor_id/product_id 映射；同 bus 内查重（跨 usb/pcie 同名属合法双 bus 支持，不算重复）。
   - `docs/support_list.md`：解析 `# <厂商>` 分节 + `Model | Platform | USB | PCIe` 表格行。
   - `vendor/dynamic_load.json` + `vendor/<mfr>.sh`：json 中每个 manufacturer 必须有 dynamic_load 映射、脚本文件存在、且定义框架必需函数（`get_imei/get_mode/network_info/sim_info/base_info`，与 rpcd `load_vendor_script` 后的调用约定对应，见 `application/qmodem/files/usr/libexec/rpcd/qmodem:107`）。
   - `testcases/recognition/<mfr>/<platform>/<model>-*/`：recognition fixture 存在性。
2. `.github/workflows/quality.yml`：追加 stage `modem-registry consistency gate`（`if: hashFiles(...)` 存在才跑），触发 paths 追加 `scripts/check_modem_registry.py`、`docs/support_list.md`。
3. `docs/refactor/modem-registry-baseline.md`：首跑基线报告。

## CLI 与退出码

- 无参数：严格模式，退出码 0（一致）/ 1（不一致）。
- `--report`：markdown 矩阵报告到 stdout（四个类别分节 + 汇总表）。
- `--baseline`：只报告恒返回 0，不触发门禁 FAIL。
- `--fail-on-fixtures`：fixture 缺失计入失败（默认 fixture 缺失仅报告——覆盖是增量建设目标，当前 2/91）。

## 首跑基线结论

- json (usb/pcie) 91 条 ↔ md 91 行：**完全对齐**（0/0 差异）；schema 错误 0。
- 存量不一致仅 2 处 vendor 函数缺口：`vendor/neoway.sh` 缺 `get_mode()`、`vendor/telit.sh` 缺 `get_imei()`（属 vendor 功能面缺口，非登记遗漏）。
- recognition fixture 缺 89 个（信息项）。
- 因此 CI 首跑采用 `--baseline` 模式：**不会**因上述存量缺口 FAIL。

## 如何转严格（后续步骤）

1. 修复 neoway.sh `get_mode()` 与 telit.sh `get_imei()`（或评审调整 `REQUIRED_VENDOR_FUNCS`）；
2. 移除 quality.yml 中该 stage 的 `--baseline` 参数；
3. 此后任何 json/md/vendor 登记遗漏即门禁 FAIL（严格模式已在本地验证退出码 1 路径）。

## 验证记录

- `python3 -m py_compile` 通过；脚本/文档/workflow 均 LF（grep CR = 0）。
- 退出码矩阵实测：严格=1（存量缺口）、baseline=0、strict+`--fail-on-fixtures`=1。
- 负路径注入测试：临时向 json 注入 `test-ghost-modem` → 报告 `In json but not in md (1)` 且退出码 1，随后恢复原文件（diff 确认还原）。
- `yaml.safe_load` 校验 quality.yml：stage 位于第 10 步，条件与命令符合预期。
- 注意：`--report` 与 `--report --baseline` 的 stdout 均为完整 markdown；基线文档由后者生成。
