# journal-rpcd（Phase 2：rpcd qmodem 剩余问题清理）

- **agent**: agent-rpcd
- **范围**: 仅 `application/qmodem/files/usr/libexec/rpcd/qmodem`（单文件）
- **日期**: 2026-10-12

## 变更清单

### 1.【S11-中】get_at_cfg 通用 eval → 白名单命令分发
- 原 ~171-186 行：jq `@sh "key=... value=..."` + `eval $cmd`（数据来自 /usr/share/qmodem/at_commands_*.json）。
- 现改为：jq 输出 `<op>\t<key|@sh>\t<value|@sh>` 单行协议，`while IFS=$tab read -r op key value` + `case` 分发；唯一白名单 op 为 `at_cmd`（读代码确认实际命令形态仅"设置 name/value 键值对"一种）；未知 op 直接忽略。
- 新增 `_sh_unquote()`：仅接受裸词或"无内嵌单引号的单引号包裹"两种简单形式，`'\''` 等复杂转义形式一律拒绝（跳过该条目），杜绝任何 shell 解释路径。**全文 eval 清零（grep=0）**。
- 与原实现的差异（记录在案）：
  - `echo -e`（bashism）与 `IFS=$'\n'` 一并消除；
  - 原实现当 platform_cmd 为空时 `echo -e "a\n"` 会产生空行条目（eval 空串后重复 json_add 上一轮 key/value 的隐性问题），新实现自然跳过空行，属修复而非回归；
  - 对 at_commands_en/zh.json 全部实际数据（含 `AT+QCFG="usbnet"` 等含双引号值）做了新旧算法逐字节对比，见验证 #3。
- 注：任务原文建议"参数经 jq @sh 后以位置参数传给 case 分支"。因 POSIX sh 无法在不引入 eval 的前提下展开 `@sh` 引号词（eval 正是本轮要删除的目标），最终采用 tab 分列 + `_sh_unquote` 严格白名单解引方案，注入安全性等价且满足"eval=0"硬性验收。

### 2.【S11-低】bashism 清理
- 原 731 行 `[[ "$status_output" != *"Not Running"* ]]` → `case` 语句（dial_status）。
- `IFS=$'\n'`、`echo -e` 随 get_at_cfg 重写一并消除。`[[` 残留 grep=0。

### 3.【S11/try_cache】缓存写入原子化
- try_cache 与 get_sms 中 `touch + 重定向直写 /tmp/cache_*` 改为：`mktemp "$cache_file.tmp.XXXXXX"`（同目录）→ 完整写入 → `mv` 原子替换；写入/mv 失败 `rm -f` 清理临时文件；TTL 语义（stat 第 14 字段 mtime）不变。并发 RPC 不会再观察到半写缓存。
- get_sms 属同类隐患，按同一模式顺带修复（同文件内，未越界）。

### 4.【S9-低】delete_sms index 校验
- 原 `for i in $index`（未引号 + glob 风险，index 来自 RPC）改为：先 `case` 白名单校验整个 index 仅允许 `0-9`、`,`、空格（非法则输出 `{"result":{"status":"0","error":"invalid index, must be comma/space separated digits"}}` 并 exit 1），再 `tr ',' ' '` 分词、逐项二次数字校验后才调用 tom_modem。

### 5.【S13-低】get_tty_ports 动态设备枚举
- 已核查：modem_scan.sh 无 tty 枚举方法、仓库无 modem_scand 设备清单接口、UCI 无全局 ports 选项，无可复用的现成动态列表。
- 采取优于"上限提到 15"的方案：`/dev/ttyUSB0-7`、`/dev/ttyACM0-3` 硬编码循环改为 `/sys/class/tty/ttyUSB*`、`/sys/class/tty/ttyACM*` glob 动态发现（`[ -e ]` 保护），索引数量不再受限，与 modem_scan 动态发现对齐；mhi/wwan 段原有 glob 保持不变。已知差异：多于 10 个口时字典序排序（ttyUSB10 排在 ttyUSB2 前），不影响正确性。

### 6. 样板收敛（替代原 _ubus_json 计划）
- **grep 全文核实：本文件不存在任何 `ubus call` 实际调用**（仅 traffic_reset_build_line 生成的 cron 行字符串模板含该字样），`_ubus_json` 无 ≥5 处可替换的调用点，强行引入即死代码，故未添加。
- 按同一精神收敛了真正的重复样板：`echo "$x" | jsonfilter -e '@.k'`（12 处）与 `echo "$x" | jq -r '.k'`（6 处）→ 私有函数 `_rpc_field <dot-key> <json>`、`_rpc_jq <filter> <json>`。替换点：call 入口 config_section、delete_sms index、do_reboot params/method、send_at params/at/port/flag、send_raw_pdu cmd、send_sms params/phone/message、set_imei、set_lockband 块、set_mode、set_5g_lan、set_sms_storage、set_sim_slot、traffic_reset_set_schedule ×5。
- 输出逐字节一致性：参数与管道写法逐一对应；仅 `echo` → `printf '%s'`（不再让 dash echo 吞反斜杠转义，对含 `\\` 的输入反而更正确；常规输入无字节差异，jq -r 缺键输出 "null" 的行为与原实现完全一致）。

## 验证记录

1. **语法**：`bash -n` OK；安装 dash 后 `dash -n` OK（先发现并修复了 case 模式括号内空格的语法错误 `*[!0-9, ]*` → `*[!0-9," "]*`）。
2. **grep 终态**：`eval`=0、`[[`=0、`IFS=$'`/`echo -e`=0、`touch "$cache`=0。
3. **/tmp 单元断言**（bash 运行，32/32 PASS，已删除）：
   - valid_section：合法/带连字符/空/分号/64/65 字符 6 项；
   - _sh_unquote：裸词、带空格引号、内嵌双引号、空串、拒绝内嵌 `'`、拒绝 `'\''` 形式 6 项；
   - _rpc_field/_rpc_jq：取值、缺键（jq -r "null" 行为与原管道一致）4 项；
   - try_cache：新鲜 passthrough（大 TTL 规避沙箱 mtime 时钟偏差）、过期重建、缺失重建、20 并发写后缓存为单行合法 JSON 且无 `.tmp.*` 残留 8 项；
   - get_at_cfg 新旧算法逐字节对比：at_commands_en/zh.json × quectel/qualcomm、quectel/unisoc、huawei/general、simcom/general 共 8 组，`cmp` 全等 8 项。
4. jsonfilter 本机不存在，单测中以 `jq -er ".k // empty"` shim 等价模拟（OpenWrt 端为真实 jsonfilter）。

## 遗留 / 后续建议

- at_commands JSON 的 vendor/platform 键名直接内插进 jq 程序（`.${vendor}.${platform}`），来自 UCI；现状值域安全，后续可改 `--arg` 传参消除 jq 注入面（本次未动，避免扩大 diff）。
- `ubus call + jq` 样板若未来在 rpcd 落地（如迁回 ubus 通路），可直接按 `_rpc_field/_rpc_jq` 模式补 `_ubus_json <object> <method> <args-json>`。
