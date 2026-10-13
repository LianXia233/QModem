# Copyright (C) 2024 Tom <fjrcn@outlook.com>
# This is free software, licensed under the GNU General Public License v3.

include $(TOPDIR)/rules.mk

# Include unified version
include ../../version.mk

# ---------------------------------------------------------------------------
# QModem 唯一前端包
#
# 本包由 luci-app-qmodem-generic（视觉层 + 组件层基座）与 luci-app-qmodem-next
# （独有业务视图）合并而来，三层统一：
#   视觉层 theme.js / 组件层 ui.js + controls.js / 数据层 controls.sms
#
# 运行时只依赖 +qmodem：一体化后端自带 rpcd 插件（qmodem / qmodem_cache /
# qmodem_stats / qmodem_support / qos / modem_ctrl）、采集服务、拨号脚本、
# 短信守护进程与 SMS 转发，因此本包不再依赖任何独立的 *-next / *-smsd /
# *-settings 包。整合与保留清单见 docs/refactor/integration-scope.zh-cn.md。
#
# 关闭 JS 压缩：本包视图使用较新的 JS 语法（箭头函数、模板字符串等），
# 开启 jsmin 会在部分模式下误改写标识符导致运行时白屏。
LUCI_MINIFY_JS:=0

PKG_NAME:=luci-app-qmodem
PKG_VERSION:=$(QMODEM_VERSION)
PKG_RELEASE:=$(QMODEM_AUTORELEASE)
PKG_LICENSE:=Apache-2.0
PKG_LICENSE_FILES:=../../LICENSE
PKG_MAINTAINER:=Tom <fjrcn@outlook.com>

LUCI_TITLE:=QModem modem management (unified UI)
LUCI_DESCRIPTION:=Unified LuCI (JavaScript) Web UI for any modem managed by QModem. \
	All data is read through the `qmodem` ubus object, so the UI is not bound to a \
	specific module model.
# 兼容基线：OpenWrt / ImmortalWrt 25.12 及更新版本（不再支持 24.10 及更早）。
# 注意：OpenWrt buildroot 的 DEPENDS 不支持 `(>= 版本)` 约束语法（package-metadata.pl
# 会把 `(>=` / `25.12)` 当作不存在的包名，导致 make defconfig 失败）。因此 25.12+
# 基线由 CI 编译矩阵（仅 25.12 + snapshot）与文档强制，此处保持无版本约束写法。
LUCI_DEPENDS:=+luci-base +qmodem
LUCI_PKGARCH:=all

# 旧的两套前端已被本包吸收：提供 qmodem-luci-ui 以便历史依赖解析，
# 声明 conflict 防止 generic / next 与本包同时安装造成菜单与 ACL 重复注册。
PKG_PROVIDES:=qmodem-luci-ui
PKG_CONFLICTS:=luci-app-qmodem-generic luci-app-qmodem-next

# 安装后重启 rpcd：本包（与 qmodem 后端包）携带的 ACL 与 rpcd 插件需要在
# rpcd 启动时扫描 acl.d / 注册插件；同时清掉 LuCI 的资源缓存索引，
# 避免旧的缓存清单指向已被替换或删除的静态资源。
#
# 重要：本包随固件镜像打包时 postinst 不会被执行，镜像里的文件会保持源码
# 自身的权限位。因此随包提交的脚本类文件必须以 100755 提交；这里的 chmod
# 只是通过包管理器安装时的兜底，不能作为唯一保障。
define Package/luci-app-qmodem/postinst
#!/bin/sh
[ -n "$${IPKG_INSTROOT}" ] && exit 0

/etc/init.d/rpcd restart 2>/dev/null || true
rm -rf /tmp/luci-*cache 2>/dev/null || true
exit 0
endef

include $(TOPDIR)/feeds/luci/luci.mk

# call BuildPackage - OpenWrt buildroot signature
