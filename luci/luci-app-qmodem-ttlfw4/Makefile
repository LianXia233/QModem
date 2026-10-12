# Copyright (C) 2024 Tom <fjrcn@outlook.com>
# This is free software, licensed under the GNU General Public License v3.

include $(TOPDIR)/rules.mk

# Include unified version
include ../../version.mk

PKG_NAME:=luci-app-qmodem-ttlfw4
# legacy: fw4/nft 专用变体，维护模式；与 luci-app-qmodem-ttl 互斥（共用 /etc/config/qmodem_ttl），勿新增功能
LUCI_TITLE:=LuCI App for QModem TTL Configuration (JS LuCI / fw4 Only) (legacy, maintenance mode)
LUCI_DEPENDS:=+luci-base
PKG_CONFLICTS:=luci-app-qmodem-ttl
LUCI_PKGARCH:=all
PKG_VERSION:=$(QMODEM_VERSION)
PKG_RELEASE:=1
PKG_LICENSE:=GPLv3
PKG_LICENSE_FILES:=LICENSE
PKG_MAINTAINER:=Tom <fjrcn@outlook.com>

define Package/luci-app-qmodem-ttlfw4/conffiles
/etc/config/qmodem_ttl
endef

include $(TOPDIR)/feeds/luci/luci.mk

# call BuildPackage - OpenWrt buildroot signature
