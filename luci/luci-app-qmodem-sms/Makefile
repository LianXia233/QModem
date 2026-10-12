# Copyright (C) 2023 Siriling <siriling@qq.com>
# This is free software, licensed under the GNU General Public License v3.

include $(TOPDIR)/rules.mk

# Include unified version
include ../../version.mk

PKG_NAME:=luci-app-qmodem-sms
LUCI_TITLE:=Luci qwrt modem sms support (legacy Lua UI, deprecated - prefer luci-app-qmodem-next)
PKG_VERSION:=$(QMODEM_VERSION)
LUCI_PKGARCH:=all
PKG_LICENSE:=GPLv3
PKG_LICENSE_FILES:=LICENSE
PKG_MAINTAINER:=Tom <fjrcn@outlook.com>
# legacy: 硬依赖 +luci-app-qmodem 仅为旧栈 UI 页面入口；Step 2 将在
# luci-app-qmodem-next 提供 get_modem_cfg 端点后切换为 +qmodem 并移除 UI 依赖
LUCI_DEPENDS:=+luci-app-qmodem +qmodem-smsd +sms-forwarder-next +libubus-lua

include $(TOPDIR)/feeds/luci/luci.mk

# call BuildPackage - OpenWrt buildroot signature
