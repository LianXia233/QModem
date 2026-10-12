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
#   - No package auto-enables its init.d scripts; qmodem's own uci-defaults
#     only cover qmodem_reboot/qmodem_usage_stats. The suite bootstrap
#     (zz-qmodem-suite) enables the rest: qmodem_init (mkdir /var/run/qmodem
#     + modem scan), qmodem_network (dial), qmodem_led, ubus-at-daemon,
#     qmodem-smsd, qmodem_monitor. Verified: docs/refactor/journal-pkg-verify.md
#   - luci-app-qmodem-ttlfw4 (TTL mgmt, fw4/nft stack) added for full-stack
#     delivery; it CONFLICTS with the legacy iptables variant luci-app-qmodem-ttl
#     (shared /etc/config/qmodem_ttl), so the suite intentionally does NOT
#     pull ttl -- users with ttl installed must remove it first.
#     The uci default section (config main 'main', enable=0) ships inside the
#     ttlfw4 package itself (root/etc/config/qmodem_ttl, conffile), so the
#     suite uci-defaults needs no TTL supplement.
#   - voip stack (qmodem-voip + qmodem-sipd + luci-app-qmodem-voip) is
#     experimental and NOT included; see docs/refactor/journal-retire-voip.md
#     (independent retention, separate install). Verified: docs/refactor/journal-suite-full.md
#
include $(TOPDIR)/rules.mk

# Include unified version
include ../../version.mk

PKG_NAME:=qmodem-suite
PKG_RELEASE:=$(QMODEM_RELEASE)
PKG_VERSION:=$(QMODEM_VERSION)

include $(INCLUDE_DIR)/package.mk

define Package/$(PKG_NAME)
  SECTION:=utils
  CATEGORY:=Utilities
  TITLE:=QModem complete management stack (meta-package)
  DEPENDS:=	\
	+qmodem \
	+luci-app-qmodem-next \
	+qmodem-smsd \
	+qmodem_monitor \
	+luci-app-qmodem-monitor \
	+luci-app-qmodem-ttlfw4
endef

define Package/$(PKG_NAME)/description
	One-shot install of the full QModem modem management stack:
	core scripts, LuCI modern UI, SMS daemon, network monitor and
	TTL management (fw4/nft). The experimental voice (voip) stack is
	not included and must be installed separately.
	一键安装完整 QModem 模组管理栈：核心脚本 + LuCI 现代界面 +
	短信守护进程 + 网络监控 + TTL 管理（fw4/nft），安装即用，无需
	逐个挑选软件包。实验性语音（voip）组件不包含在内，需单独安装。
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
