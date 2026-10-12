# journal: UCI 选项参考文档与配置注释（agent-uci-docs）

日期：2026-10-12
范围约束：仅改动 `application/qmodem/files/etc/config/**`、`application/qmodem_smsd/files/etc/config/**`，新建 `docs/reference/uci-options.zh-cn.md` 与 `.en.md`。

## 交付物

1. `docs/reference/uci-options.zh-cn.md` / `docs/reference/uci-options.en.md`
   - 覆盖 4 个包：`qmodem`（main / main 运行期项 / modem-device / bridge-port-backup）、`qmodem_led`（main / uci-defaults 生成节）、`qmodem_sms`（main）、`qmodem_ttl`（config 文件不在本仓库 application/ 下，随 luci-app-qmodem-ttl / ttlfw4 安装）。
   - 每行：名称、类型、默认值、取值域、消费者（文件:行号）。
   - 中英文表格逐行对齐（73 行，bridge_port 在两个节中各出现一次，属预期）。

2. `application/qmodem/files/etc/config/qmodem`：为全部 18 个 option 补独立注释行（说明语义、消费者与取值域），保留原有 AT 工具链注释，UCI 语法保持有效。

## 调研方法

- `Grep`：全仓库 `uci (get|set|add|toggle)` 含 qmodem/qmodem_sms/qmodem_ttl 的调用点（约 200 处）。
- 补充 `config_get` / `config_get_bool`（init.d、led_scripts）与 C 侧 `option_string`（qmodem_smsd/src/main.c）、`uci_get/uci_set`（modem_scand.c）。

## 废弃 option 核查结论

- 三个配置文件中现有 option 均存在消费者，**未发现废弃项**，故无"仅标注"条目。
- 唯一"只写不读"：`qmodem.main.board_defaults_version`（`99-qmodem-board:98` 写入，无读取方；板级默认值版本标记，非配置文件缺省项，已在文档表格中标注）。

## 验证

- Python 解析三个 config 文件：config/option 配对完整，无未知 token（qmodem 1 节 18 项、qmodem_led 1 节 1 项、qmodem_sms 1 节 5 项）→ PASS。
- 文档表格行数与解析结果一致：zh=en=73 选项行，集合完全一致 → PASS。
- 逐项与 grep 消费者对照（含 modem_scand.c、smsd main.c、rpcd、init.d、hotplug、uci-defaults、vendor 脚本）→ 无虚构条目。
- 所有新文件 LF 行尾（file 检查无 CRLF）→ PASS。

## 影响面

- 仅注释与文档新增，无运行时行为变更。
