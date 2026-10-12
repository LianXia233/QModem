#
# Copyright (C) 2024 Tom <fjrcn@outlook.com>
# This is free software, licensed under the GNU General Public License v3.
#
# qmodem-suite: meta-package, single-install delivery of the full QModem stack
# (core + LuCI UI + SMS daemon + network monitor). No build output of its own.
#
# Dependency audit (see docs/refactor/journal-pkg-suite.md):
#   - luci-app-qmodem-next pulls qmodem + qmodem-smsd + sms-forwarder-next +
#     luci-base transitively; no redundant +luci-compat found (JS-only app).
#   - qmodem_monitor needs curl only for its optional HTTP probe method
#     (Method=curl); ping is the default, so curl stays a soft dependency.
#   - Real package names verified: "qmodem-smsd" (hyphen) vs "qmodem_monitor"
#     (underscore) -- do NOT normalize these.
#
include $(TOPDIR)/rules.mk

# Include unified version
include ../../version.mk

PKG_NAME:=qmodem-suite
PKG_RELEASE:=$(QMODEM_RELEASE)
PKG_VERSION:=$(QMODEM_VERSION)

include $(INCLUDE_DIR)/package.mk

define Package/$(PKG_NAME)
  PKGARCH:=all
  SECTION:=utils
  CATEGORY:=Utilities
  TITLE:=QModem complete management stack (meta-package)
  DEPENDS:=	\
	+qmodem \
	+luci-app-qmodem-next \
	+qmodem-smsd \
	+qmodem_monitor \
	+luci-app-qmodem-monitor
endef

define Package/$(PKG_NAME)/description
	One-shot install of the full QModem modem management stack:
	core scripts, LuCI modern UI, SMS daemon and network monitor.
	一键安装完整 QModem 模组管理栈：核心脚本 + LuCI 现代界面 +
	短信守护进程 + 网络监控，安装即用，无需逐个挑选软件包。
endef

define Build/Prepare
endef

define Build/Compile
endef

define Package/$(PKG_NAME)/install
	$(INSTALL_DIR) $(1)/etc/uci-defaults
	$(INSTALL_BIN) ./files/etc/uci-defaults/zz-qmodem-suite $(1)/etc/uci-defaults/
endef

$(eval $(call BuildPackage,$(PKG_NAME)))
