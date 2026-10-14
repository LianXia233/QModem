# Copyright (C) 2024 Tom <fjrcn@outlook.com>
# This is free software, licensed under the GNU General Public License v3.

include $(TOPDIR)/rules.mk

# Include unified version
include ../../version.mk

LUCI_TITLE:=LuCI Modern UI for QModem
LUCI_DESCRIPTION:=LuCI modern (JavaScript) interface for QModem.
# 本仓库适配（2026-10-14）：上游按多包架构声明 +qmodem-smsd +sms-forwarder-next，
# 本仓库已将二者并入单包 qmodem（PKG_PROVIDES 含其包名，安装期依赖可满足），
# 但 OpenWrt 构建期元数据解析要求依赖名真实存在于包树，否则 defconfig 会
# 丢弃本包（报 missing required release package）。故构建期依赖收敛为 +qmodem。
LUCI_DEPENDS:=+qmodem +luci-base
LUCI_PKGARCH:=all
LUCI_MAINTAINER:=Tom <fjrcn@outlook.com>

PKG_VERSION:=$(QMODEM_VERSION)
PKG_RELEASE:=4
PKG_LICENSE:=GPLv3
PKG_PROVIDES:=qmodem-luci-ui

include $(TOPDIR)/feeds/luci/luci.mk

# call BuildPackage - OpenWrt buildroot signature
