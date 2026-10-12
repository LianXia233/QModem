# Journal — Release 资产散装化与 qmodem-suite 恢复构建

日期：2026-10-12
范围：`.github/workflows/`（main.yml / release.yml / publish-existing-release.yml）、
`qmodem_package_generic`、CHANGELOG 双语、v1.0.0 Release 资产替换

## 背景（问题）

用户反馈："需要直接交付 APK/IPK 能直接安装的，不要压缩包"。

核查 v1.0.0 Release 资产（`gh release view v1.0.0 --json assets`）：

| 资产名 | 类型 | 大小 |
|---|---|---|
| QModem-arm64_apk.tar.gz | application/gzip | 465,974 |
| QModem-arm64_ipk.tar.gz | application/gzip | 470,616 |
| QModem-mmips_apk.tar.gz | application/gzip | 433,484 |
| QModem-mmips_ipk.tar.gz | application/gzip | 437,596 |
| QModem-x64_apk.tar.gz | application/gzip | 465,853 |
| QModem-x64_ipk.tar.gz | application/gzip | 467,767 |

6/6 资产均为 tar.gz——用户下载后必须手动解压，与"安装即用"目标相悖。

根因：`main.yml` 的 `job_release_artifacts` 有 "Package artifacts into tar.gz"
步骤，把每个 `QModem-<arch>_<pkgtype>` artifact 目录重新打包；
`publish-existing-release.yml` 同样打包 `qmodem-<tag>.tar.gz`。

## 附带发现：qmodem-suite 元包缺失

解压 tar.gz 后发现产物只有 16 个包族，**没有 qmodem-suite**。
排查：`.github/workflows/qmodem_package_generic`（generic 构建清单）只含
`luci-app-qmodem-next / qmodem / qmodem_smsd / sms_forwarder_next`，
arch 清单只含 `tom_modem / ubus_at_daemon`。qmodem-suite 及其 DEPENDS
（qmodem_monitor / luci-app-qmodem-monitor / luci-app-qmodem-ttlfw4）
从未被编译——Phase 1"单包交付"承诺没有可分发产物。

## 修复

### 1. CI 不再打包压缩文件

- `main.yml`：删除 "Package artifacts into tar.gz"，替换为
  "Collect installable packages"（`find artifacts -type f \( -name '*.ipk' -o -name '*.apk' \)`
  拷入 `releases/`，并强制校验 ipk/apk 两类都存在，否则 fail）。
- `publish-existing-release.yml`：同样删除 tar.gz 打包，`files` 改为
  `releases/*.ipk releases/*.apk`。
- `release.yml`：更新 artifact 提示段，明确"资产为可直接安装的散装包"并给出
  opkg/apk 安装命令。
- 三份 workflow 均通过 `yaml.safe_load` 校验。

### 2. qmodem-suite 恢复构建

- `qmodem_package_generic` 追加 `qmodem-suite`（其 DEPENDS 由
  `make package/qmodem-suite/compile` 自动拉入编译）。
- `main.yml` `required_packages` 增加 `qmodem-suite` 门禁，缺失即构建失败。

### 3. v1.0.0 Release 资产替换（手工执行）

- 下载 6 个 tar.gz（GitHub release 直连限速，单批约 3-7 分钟，需重试补漏）。
- 发现 **apk 文件名不含架构**（`qmodem-3.4.0_rc3-r1.apk`），三个架构同名不同
  内容（md5 逐对比对确认），直接上传会同名冲突。处理：apk 资产名追加架构后缀
  `_aarch64` / `_x86_64` / `_mipsel_24kc`；ipk 原名已含架构，保持不变；
  `_all` 架构无关包（luci-app-qmodem-next / luci-i18n）三架构 md5 一致，去重上传一份。
- `gh release delete-asset` 删除 6 个 tar.gz；`gh release upload --clobber`
  分 4 批上传 80 个散装包（38 ipk + 42 apk，26 秒完成）。
- 复核 `gh release view v1.0.0 --json assets` → assets = 80，无 tar.gz 残留。

## 验证

| 项 | 结果 |
|---|---|
| yaml.safe_load（3 份 workflow） | OK |
| release 资产数 | 80（38 ipk + 42 apk） |
| tar.gz 残留 | 0 |
| apk 架构后缀冲突 | 无（3×14 全部唯一） |
| CHANGELOG 双语段落 | 已同步（Unreleased → Fixed） |

### 包格式核查（重要，排除误报）

初次校验时 `ar t *.ipk` 与 `gzip -t *.apk` **全部失败**，一度怀疑产物损坏。
查证后确认这是**本地工具与这代 SDK 包格式不匹配造成的误报**，包本身完好：

| 格式 | 实际结构 | 依据 | 设备端安装 |
|---|---|---|---|
| `.ipk` | `gzip(tar(debian-binary + control.tar.gz + data.tar.gz))` | OpenWrt 现行 `ipkg-build`（`tar … \| gzip -n - > pkg.ipk`，openwrt commit 7a732213）；CSDN 拆包教程同样记载"该文件实质是 .tar.gz" | `opkg install` 正常 |
| `.apk` | APK v3 容器：`ADB` 魔数 + 压缩模式字节（`d`=Deflate level 0）+ raw deflate 流，解压后为 `ADB.pckg` schema | Alpine apk-tools v3（OpenWrt 25.12 起默认，snapshot SDK 已切换）；raw deflate 无 gzip 头，`gzip -t` 必然失败 | `apk add`（apk-tools 3.x）正常 |

按真实规范重写校验（`scripts/verify_release_packages.py`）：38 ipk + 42 apk
**全部通过**（ipk 三件套齐全且 control.tar.gz 内含 control 元数据；apk
ADB 流可完整解压且 schema 为 pckg）。该脚本已入库，作为未来 Release 产物的
格式门禁，避免再次误判。

## 待办

- qmodem-suite 构建产物（ipk + apk）由推送后触发的 main.yml 重建产出，
  需从新 run 下载并补传 v1.0.0 Release，随后更新 release notes。
- apk 资产名带架构后缀是 GitHub 单层资产列表的权宜之计；若未来迁移到
  apk 官方索引（或按目录分层交付），可改回标准命名。
