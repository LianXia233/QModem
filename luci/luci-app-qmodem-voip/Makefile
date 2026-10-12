include $(TOPDIR)/rules.mk

include ../../version.mk

# retire-audit (agent-retire-voip): 实验性语音包，luci-app-qmodem-next 无任何对应物
# （ACL/menu 均无 voip/sip 覆盖），Phase 4 结论为独立保留而非退役。
# 自带独立依赖链 +qmodem-voip(voipd) +qmodem-sipd，与旧栈 luci-app-qmodem 无耦合。
# gap 表与维护成本估计见 docs/refactor/journal-retire-voip.md。
LUCI_TITLE:=LuCI support for experimental QModem voice (legacy, maintenance mode)
LUCI_DEPENDS:=+qmodem-voip +qmodem-sipd +luci-base
LUCI_PKGARCH:=all
PKG_VERSION:=$(QMODEM_VERSION)
PKG_RELEASE:=$(QMODEM_RELEASE)
PKG_LICENSE:=GPL-3.0-or-later
PKG_LICENSE_FILES:=LICENSE

include $(TOPDIR)/feeds/luci/luci.mk

# call BuildPackage - OpenWrt buildroot signature
