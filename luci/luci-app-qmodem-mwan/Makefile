# Copyright (C) 2023 Siriling <siriling@qq.com>
# This is free software, licensed under the GNU General Public License v3.

include $(TOPDIR)/rules.mk

# Include unified version
include ../../version.mk

PKG_NAME:=luci-app-qmodem-mwan
LUCI_TITLE:=Luci qwrt modem mwan support (legacy, maintenance mode)
LUCI_PKGARCH:=all
PKG_VERSION:=$(QMODEM_VERSION)
PKG_LICENSE:=GPLv3
PKG_LICENSE_FILES:=LICENSE
PKG_MAINTAINER:=Tom <fjrcn@outlook.com>
# legacy: 硬依赖 +luci-app-qmodem 仅为旧栈 UI 入口；Step 2 后切换为 +qmodem
LUCI_DEPENDS:=+luci-app-qmodem +luci-app-mwan3
		

define Package/luci-app-qmodem-mwan/conffiles
/etc/config/modem_mwan
endef

include $(TOPDIR)/feeds/luci/luci.mk

# call BuildPackage - OpenWrt buildroot signature
