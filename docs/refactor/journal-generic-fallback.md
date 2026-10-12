# journal-generic-fallback — 未知/新模组兜底加固

> 维护者：agent-generic-fallback（代码交付）+ 编排者（收尾验证与补记；代理因会话中断未及写 journal，本文件依据工作区实际 diff 补记）

## 问题

识别失败的模组（未知 VID:PID 且型号不匹配 modem_support.json）会被 `add_modem`
直接丢弃（`goto out_fail`），导致整个槽位不可用：无 modem_device 节、无 AT 通路、
无拨号配置 —— 新模组即使硬件完全兼容也无法起步。

## 修复（实际 diff，均已验证）

| 文件 | 改动 |
|------|------|
| `modem_scand.c:1161`（add_modem） | detect_profile 失败不再丢弃：注册 generic profile（name/manufacturer="generic"，pdp_index=1，空 modes 由 dial 流程补全），日志降为 WARN 并注明 fallback |
| `vendor/dynamic_load.json` | 新增 `"generic": "../generic.sh"` 显式映射，兜底厂商可被动态加载器解析 |
| `cmds/generic.sh` | 新增 TS 27.007 基线 cmd_* 包装层（CGMI/CGMM/CGMR/CGSN/CIMI/CPIN?/COPS?/CSQ + QENG servingcell 尽力而为 + ICCID?/CCID 尝试链）：vendor 侧同名函数覆盖之，已知模组行为不变；附「取首个有意义响应行」辅助（剔除回显/OK/ERROR/CME/引号） |

## 兼容性矩阵（新逻辑下的四类模组）

1. 已登记 + vendor 齐全 → 原路径（行为不变）
2. 已登记 + vendor 函数缺失 → registry 门禁报告（scripts/check_modem_registry.py 基线）
3. **未登记 → generic 兜底**：发现/AT/基础信息/拨号可用（本次修复）
4. 完全非 AT 设备 → AT 命令超时，dial 退避（modem_dial.sh qmi_dial 退避兜底）

## 验证

- `sh -n` generic.sh 通过；`gcc -fsyntax-only -Wall` modem_scand.c 通过（/tmp/qstub 头）
- 行为级验证（fixture 回放/实机）列入 scripts/check_modem_registry.py 的 fixture 覆盖建设项

## 关联

- 注册一致性：docs/refactor/modem-registry-baseline.md（91/91 对齐，neoway get_mode / telit get_imei 两缺口）
- 新模组接入：docs/developer/new-modem-quickstart.zh-cn.md + cmds/vendor_template.sh
