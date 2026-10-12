include $(TOPDIR)/rules.mk
include $(INCLUDE_DIR)/kernel.mk

# Include unified version
include ../../version.mk

PKG_NAME:=qmodem_monitor
PKG_RELEASE:=$(QMODEM_RELEASE)
PKG_VERSION:=$(QMODEM_VERSION)

include $(INCLUDE_DIR)/package.mk

define Package/$(PKG_NAME)
  PKGARCH:=all
  SECTION:=utils
  CATEGORY:=Utilities
  TITLE:=QModem Network Monitor scripts
  DEPENDS:=+qmodem +flock +jq
endef

define Package/$(PKG_NAME)/description
	QModem Network Monitor scripts
endef

define Build/Prepare
endef

define Build/Compile
endef

define Package/$(PKG_NAME)/install
	$(INSTALL_DIR) $(1)/etc/init.d $(1)/usr/share/qmodem
	$(INSTALL_BIN) ./files/etc/init.d/qmodem_monitor $(1)/etc/init.d/
	$(INSTALL_BIN) ./files/usr/share/qmodem/modem_monitor.sh \
		./files/usr/share/qmodem/monitor_recovery.sh $(1)/usr/share/qmodem/
endef

$(eval $(call BuildPackage,$(PKG_NAME)))
