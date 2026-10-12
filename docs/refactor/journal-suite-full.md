# Journal: qmodem-suite 最优单包交付扩展（TTL/fw4 纳入）

- **Agent**: agent-suite-full
- **Date**: 2026-10-12
- **Scope**: `application/qmodem-suite/**`（Makefile）+ `docs/user/install-suite.zh-cn.md` + `docs/user/install-suite.en.md` + 本 journal。其余路径未触碰。

## 目标

按「最优单包交付」扩展 qmodem-suite：纳入 TTL 管理（fw4/nft 变体），排除实验性 voip，并同步文档。

## 变更记录

1. **`application/qmodem-suite/Makefile`**
   - `DEPENDS` 追加 `+luci-app-qmodem-ttlfw4`（TTL 管理，fw4/nft 现代栈）。
   - `description` 中英更新：英文列点补 TTL management (fw4/nft)，声明 voip 不含、需单独安装；中文同步。
   - 头部依赖审计注释补三条：ttlfw4 纳入与 ttl 互斥（suite 不含 ttl）、qmodem_ttl 默认节由 ttlfw4 自带（suite uci-defaults 无需补）、voip 排除依据。
2. **`docs/user/install-suite.zh-cn.md` / `install-suite.en.md`（同步）**
   - §1 组件清单补 `luci-app-qmodem-ttlfw4` 行。
   - 新增「TTL 组件说明 / TTL component notes」小节：fw4/nft 定位、与旧版 `luci-app-qmodem-ttl` 互斥（共用 `/etc/config/qmodem_ttl`）、先卸旧版再装 suite 的操作、默认关闭与默认节随包就位说明。
   - 新增「语音（voip）组件需单独安装 / Voice (voip) component requires separate installation」小节：三包名、单独安装命令、实验性 + 白名单（RM520N-GL）+ 独立维护说明。
3. **suite uci-defaults 未改动**：复核确认 `luci/luci-app-qmodem-ttlfw4/root/etc/config/qmodem_ttl`（内容 `config main 'main'`，enable 未置即默认关闭）随包安装且登记为 conffile，suite 无需补默认配置；TTL 服务默认 `enable=0`，不纳入 zz-qmodem-suite 的 enable 列表（与现有语义一致：仅启用守护类服务）。

## 验证记录

1. **DEPENDS 实名核对**：`luci/luci-app-qmodem-ttlfw4/Makefile:9` `PKG_NAME:=luci-app-qmodem-ttlfw4`，与 DEPENDS 逐字一致（连字符，非下划线）。
2. **CONFLICTS 传递性检查**：ttlfw4 `PKG_CONFLICTS:=luci-app-qmodem-ttl`（ttlfw4 Makefile:13），ttl 侧反向声明同在（ttl Makefile:19），双向互斥成立。suite DEPENDS 不含 ttl，自身无冲突；若设备已装 ttl，包管理器层面报冲突拒绝共装——OpenWrt 依赖解析无法表达「装 suite 时自动替换/移除 ttl」，故仅在 docs 写明「先卸载旧版再装 suite」，已在双语文档落实。
3. **Makefile 与现有包比对**：结构未动，仍为元包模式（`PKGARCH:=all`、空 `Build/Prepare`/`Build/Compile`、仅装 uci-defaults、`include ../../version.mk`），与 journal-pkg-suite.md 记录的比对基线一致；仅 DEPENDS/description/注释三处增改。
4. **链接核对**：双语文档新增内容未引入新 markdown 链接；提及的路径（`docs/refactor/journal-retire-voip.md`）为仓库内真实文件；文档既有链接未触碰。voip 三包名核对：`qmodem-voip`（application/voipd）、`qmodem-sipd`（application/qmodem_sipd）、`luci-app-qmodem-voip`（luci/，LUCI_DEPENDS 即拉前两者）。
5. **行尾检查**：全部改动文件 LF（`file`/`grep -c $'\r'` 复核通过，见下方验证命令输出）。
6. 本机无 OpenWrt SDK，`make` 不可用；Makefile 为语法自查（`make -f` 解析不可行，逐行比对替代）。

## 未尽事项（移交）

- ttlfw4 init.d:41 仍调用不存在的 `/etc/firewall.d/qmodem_ttl`（journal-retire-ttl.md 已记录的旧版残留，静默失败），属 ttlfw4 包域，本任务未触碰。
- voip 若未来转正，可评估纳入 suite（届时需同步本 Makefile 与双语文档）。
