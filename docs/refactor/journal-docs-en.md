# Journal: 文档英文翻译（agent-docs-en）

- 日期：2026-10-12
- Agent：agent-docs-en（技术文档工程师）
- 范围：仅新增三个 `.en.md` + 三份中文版头部语言切换链接 + README.md 文档清单补链

## 产出

| 文件 | 操作 |
|---|---|
| `docs/user/install-suite.en.md` | 新增（忠实翻译 install-suite.zh-cn.md） |
| `docs/developer/async-daemon.en.md` | 新增（忠实翻译 async-daemon.zh-cn.md） |
| `docs/developer/at-tool-selection.en.md` | 新增（忠实翻译 at-tool-selection.zh-cn.md） |
| `docs/user/install-suite.zh-cn.md` | 头部加 `[简体中文] | [English]` 双向链接 |
| `docs/developer/async-daemon.zh-cn.md` | 同上 |
| `docs/developer/at-tool-selection.zh-cn.md` | 同上 |
| `README.md` | 文档清单补三份英文版链接（`docs/user/install-suite.en.md` 等） |

## 翻译口径

- 术语对照 OpenWrt/LuCI 官方英文：metapackage、feed、autostart、troubleshooting、LuCI frontend、procd/init.d service status、uci commit、redact（脱敏）等。
- 代码块（sh/uci/json）、命令、路径逐字保留，未做任何改写；中文注释译为英文注释。
- 未虚构内容；中文版引用的中文文档链接（如 `../user-guide.zh-cn.md`）在英文版中保留原路径，待对应英文版存在时再切换。

## 验证

- 相对链接：脚本提取全部 `.md` 相对链接并 `normpath` 后核对存在，结果 `MISSING: none`（覆盖 7 个文件）。
- 代码块标点：扫描 sh/uci/json/text 代码块，非注释位置无中文标点混入。
- 英文版 CJK 残留：仅语言切换链接中的「简体中文」，符合预期。
