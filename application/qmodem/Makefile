include $(TOPDIR)/rules.mk
include $(INCLUDE_DIR)/kernel.mk

# Include unified version
include ../../version.mk

PKG_NAME:=qmodem
PKG_RELEASE:=$(QMODEM_RELEASE)
PKG_VERSION:=$(QMODEM_VERSION)

# 被本包吸收的旧独立包名。声明 provides 有两个作用：
#   1) 让仍写着 `+tom_modem` / `+qmodem-smsd` 等依赖的仓库内包（如旧的
#      sms-forwarder）在 defconfig / package-metadata 阶段仍能解析到本包；
#   2) 让已装旧包的用户在升级时其依赖得到满足，不会因为包消失而断链。
PKG_PROVIDES:=tom_modem ubus-at-daemon modem_scan libqmodem-sms \
	qmodem-smsd qmodem-settings qmodem-monitor qmodem-seal \
	sms-tool_q sms-forwarder-next quectel-CM-5G-M

# 这些包的内容已全部并入本包，同时安装会造成同名文件互相覆盖（相同 uid 的
# rpcd 插件、同名 init.d、同名 /usr/bin 工具），因此显式声明互斥。
PKG_CONFLICTS:=tom_modem ubus-at-daemon modem_scan libqmodem-sms \
	qmodem-smsd qmodem-settings qmodem-monitor qmodem-seal \
	sms-tool_q sms-forwarder-next quectel-CM-5G-M sms-forwarder

include $(INCLUDE_DIR)/package.mk

# ---------------------------------------------------------------------------
# 一体化后端包
#
# 本包把原先分散的 11 个组件收编为单一软件包，用户只需安装 qmodem：
#   libqmodem-sms  tom_modem      sms-tool_q     ubus-at-daemon
#   modem_scan     qmodem-smsd    qmodem-settings qmodem_monitor
#   sms-forwarder-next  quectel-CM-5G-M  qmodem-seal
#
# 仍保留为独立包的组件及原因见 docs/refactor/integration-scope.zh-cn.md：
#   kmod 类（vermagic 锁内核，必须走 package/kernel 的 BuildPackage）、
#   ndisc6 / qfirehose（上游独立项目，自带上游构建系统与会话补丁）、
#   qmodem-sipd / voipd（系统依赖重、默认不装）。
#
# 原 application/qmodem-suite（元包，唯一职责是逐个 enable 各守护进程）
# 已退役：其职责下沉到本包自身，见文件末尾的 postinst 与
# files/etc/uci-defaults/99-qmodem-worker-bootstrap —— 安装本包即得到
# 可直接工作的完整后端，不再需要元包。
#
# application/sms_forwarder（旧脚本实现，自带 /etc/config/sms_daemon）
# 保留为独立可选包，但它与本包内置的 sms-forwarder-next 都往
# /etc/init.d/sms_forwarder 落文件，因此继续在 PKG_CONFLICTS 中保持互斥。
# ---------------------------------------------------------------------------

define Package/$(PKG_NAME)
  SECTION:=utils
  CATEGORY:=Utilities
  TITLE:=QModem all-in-one backend (scripts, ubus daemons, dialer, CLI tools)
  DEPENDS:=	\
  		+kmod-usb2 +kmod-usb3 \
		+kmod-usb-serial +kmod-usb-serial-option +kmod-usb-serial-qualcomm \
		+kmod-usb-net +kmod-usb-acm \
		+kmod-usb-wdm \
  		+kmod-usb-net-cdc-ether \
		+kmod-usb-net-cdc-mbim \
		+kmod-usb-net-rndis \
		+kmod-usb-net-cdc-ncm +kmod-usb-net-huawei-cdc-ncm \
		+terminfo \
		+libubus +libubox +libblobmsg-json +libjson-c +libuci +libsqlite3 \
		+libpthread \
		+flock +curl \
		+ucode +ucode-mod-ubus +ucode-mod-uci \
		+jq +bc \
		+jsonfilter \
		+coreutils +coreutils-stat +xxd \
		+PACKAGE_qmodem_INCLUDE_SEAL:libsodium \
		+usbutils \
		+PACKAGE_qmodem_INCLUDE_rdisc6:rdisc6 \
		+PACKAGE_qmodem_INCLUDE_ndisc6:ndisc6 \
		+PACKAGE_qmodem_INCLUDE_generic-qmi-wwan:kmod-usb-net-qmi-wwan \
		+PACKAGE_qmodem_INCLUDE_vendor-qmi-wwan:kmod-qmi_wwan_q \
		+PACKAGE_qmodem_INCLUDE_vendor-qmi-wwan:kmod-qmi_wwan_f \
		+PACKAGE_qmodem_INCLUDE_vendor-qmi-wwan:kmod-qmi_wwan_s \
		+PACKAGE_qmodem_INCLUDE_nss-qmi-wwan:kmod-rmnet-nss \
		+PACKAGE_qmodem_INCLUDE_nss-qmi-wwan:kmod-qmi_wwan_q_nss \
		+PACKAGE_qmodem_VENDOR_MHI_PCIe_DRIVER:kmod-pcie_mhi \
		+PACKAGE_qmodem_GENERIC_MHI_PCIe_DRIVER:kmod-mhi-wwan \
		+PACKAGE_qmodem_GENERIC_MHI_PCIe_DRIVER:kmod-mhi-pci-generic \
		+PACKAGE_qmodem_GENERIC_MHI_PCIe_DRIVER:kmod-mhi-wwan-ctrl \
		+PACKAGE_qmodem_GENERIC_MHI_PCIe_DRIVER:kmod-mhi-wwan-mbim \
		+PACKAGE_qmodem_NSS_MHI_PCIe_DRIVER:kmod-rmnet-nss \
		+PACKAGE_qmodem_NSS_MHI_PCIe_DRIVER:kmod-pcie_mhi_nss \
		+PACKAGE_qmodem_INCLUDE_ADD_PCI_SUPPORT:pciutils \
		+PACKAGE_qmodem_INCLUDE_ADD_MTK_T7XX_SUPPORT:umbim \
		+PACKAGE_qmodem_INCLUDE_ADD_MTK_T7XX_SUPPORT:kmod-mtk-t7xx \
		+PACKAGE_qmodem_INCLUDE_ADD_QFIREHOSE_SUPPORT:qfirehose \
		+PACKAGE_qmodem_USING_QWRT_QUECTEL_CM_5G:quectel-CM-5G \
		+PACKAGE_qmodem_USING_NORMAL_QUECTEL_CM:quectel-cm
endef

define Package/$(PKG_NAME)/config
	config PACKAGE_qmodem_INCLUDE_SEAL
		bool "Include encrypted feedback support (qmodem-seal)"
		default y
		help
		  Build and install qmodem-seal. When omitted, qmodem_collect
		  falls back to an explicitly marked unencrypted archive.

	config PACKAGE_qmodem_TOM_MODEM_EARLY_RETURN
		bool "tom_modem: early return"
		default y

	config PACKAGE_qmodem_TOM_MODEM_ENABLE_UBUS_DAEMON
		bool "tom_modem: enable UBUS daemon support"
		default y
		help
		  Enable UBUS daemon support for tom_modem.
		  When disabled, compiles tom_modem_v1 (TTY-only mode).
		  When enabled, allows switching between TTY and UBUS with -u flag.

	choice
		prompt "IPv6 Neighbor Discovery Selection"
		default PACKAGE_qmodem_INCLUDE_ndisc6

		config PACKAGE_qmodem_INCLUDE_ndisc6
			bool "Include ndisc6 for IPv6 Neighbor Discovery"

		config PACKAGE_qmodem_INCLUDE_rdisc6
			bool "Include rdisc6 for IPv6 Router Discovery"

		config PACKAGE_qmodem_INCLUDE_no_ndisc_rdisc6
			bool "Do not include any IPv6 Neighbor/Router Discovery tool"
	endchoice

	choice
		prompt "Qualcomm QMI WWAN Driver Selection"
		default PACKAGE_qmodem_INCLUDE_vendor-qmi-wwan

		config PACKAGE_qmodem_INCLUDE_vendor-qmi-wwan
			bool "Vendor QMI driver"

		config PACKAGE_qmodem_INCLUDE_generic-qmi-wwan
			bool "Generic QMI driver"

		config PACKAGE_qmodem_INCLUDE_nss-qmi-wwan
			bool "NSS QMI driver"
			depends on TARGET_qualcommax_ipq807x || TARGET_qualcommax_ipq50xx
	endchoice

	choice
		prompt "MHI PCIe Modem Driver Selection"
		default PACKAGE_qmodem_VENDOR_MHI_PCIe_DRIVER
		depends on PACKAGE_qmodem_INCLUDE_ADD_PCI_SUPPORT

		config PACKAGE_qmodem_VENDOR_MHI_PCIe_DRIVER
			bool "Use Vendor MHI PCIe Driver"

		config PACKAGE_qmodem_GENERIC_MHI_PCIe_DRIVER
			bool "Use Generic MHI PCIe Driver"

		config PACKAGE_qmodem_NSS_MHI_PCIe_DRIVER
			bool "Use NSS MHI PCIe Driver"
			depends on TARGET_qualcommax_ipq807x || TARGET_qualcommax_ipq50xx
	endchoice

	choice
		prompt "Quectel Connect Manager Selection"
		default PACKAGE_qmodem_USE_TOM_CUSTOMIZED_QUECTEL_CM

		config PACKAGE_qmodem_USE_TOM_CUSTOMIZED_QUECTEL_CM
			bool "Tom customized Quectel CM (built into this package)"

		config PACKAGE_qmodem_USING_QWRT_QUECTEL_CM_5G
			bool "QWRT Quectel-CM-5G"

		config PACKAGE_qmodem_USING_NORMAL_QUECTEL_CM
			bool "NORMAL Quectel-CM"
	endchoice

	config PACKAGE_qmodem_INCLUDE_ADD_PCI_SUPPORT
		bool "Add PCIe Modem SUPPORT"
		default n

	config PACKAGE_qmodem_INCLUDE_ADD_MTK_T7XX_SUPPORT
		bool "Add MTK-T7XX Modem PCI SUPPORT"
		default n

	config PACKAGE_qmodem_INCLUDE_ADD_QFIREHOSE_SUPPORT
		bool "Add Qfirehose SUPPORT"
		default n
endef

define Package/$(PKG_NAME)/description
  QModem 一体化后端：模组识别与拨号脚本、ubus 数据服务、短信守护进程、
  统计采集、网络监控、SMS 转发，以及配套命令行工具（tom_modem、sms_tool_q、
  ubus-at-daemon、modem_scan、quectel-CM-M、qmodem-seal）。Rust 组件
  qmodem-atd（AT 守护进程，unix socket 通路）随包构建与安装。
endef

define Package/$(PKG_NAME)/conffiles
/etc/config/qmodem
/etc/config/qmodem_led
/etc/config/qmodem_sms
/etc/config/qmodem_ttl
/etc/config/sms_forwarder
/etc/config/ubus-at-daemon
/etc/config/qmodem-atd
endef

# ---------------------------------------------------------------------------
# Rust 组件（仓库根 rust/ workspace）
#
# feed 以 src-link 指向仓库根，包目录为 <feed>/application/qmodem，故 rust/
# 位于 ../../rust；若某天 rust/ 被并入包目录，用 ./rust 兜底。
# 三元组按 $(ARCH) 推导。注意 $(ARCH) 在不同目标下形态不一：aarch64 / x86_64
# 平台是裸值，而 MIPS 平台必须带 ISA 后缀（如 mipsel_24kc / mips_24kc），故
# 一律用 $(filter) 前缀匹配。mipsel 为 tier 3（std 可用但 rustup 无预编译），
# 由 build-rust.sh 切到 nightly + -Z build-std；aarch64 / x86_64 为 tier 2。
# 注意：mipsel 构建需要 CI 预装 nightly + rust-src（见 main.yml）。
# ---------------------------------------------------------------------------
QMODEM_RUST_SRC:=$(firstword $(foreach d,../../rust ./rust,$(if $(wildcard $(d)/Cargo.toml),$(d),)))
ifneq (,$(filter aarch64%,$(ARCH)))
QMODEM_RUST_TRIPLE:=aarch64-unknown-linux-musl
else ifneq (,$(filter mipsel%,$(ARCH)))
QMODEM_RUST_TRIPLE:=mipsel-unknown-linux-musl
else ifneq (,$(filter mips_% mips,$(ARCH)))
QMODEM_RUST_TRIPLE:=mips-unknown-linux-musl
else ifneq (,$(filter x86_64%,$(ARCH)))
QMODEM_RUST_TRIPLE:=x86_64-unknown-linux-musl
endif

define Build/CompileRust
	if [ -z "$(QMODEM_RUST_SRC)" ] || [ -z "$(QMODEM_RUST_TRIPLE)" ]; then \
		echo "!! rust build failed: QMODEM_RUST_SRC='$(QMODEM_RUST_SRC)' QMODEM_RUST_TRIPLE='$(QMODEM_RUST_TRIPLE)' (ARCH=$(ARCH))" >&2; \
		exit 1; \
	fi
	RUST_DIR="$(QMODEM_RUST_SRC)" \
	RUST_TARGET_TRIPLE="$(QMODEM_RUST_TRIPLE)" \
	OUT_DIR="$(PKG_BUILD_DIR)/rust-bin" \
	TARGET_CC="$(TARGET_CC)" \
	STAGING_DIR="$(STAGING_DIR)" \
	sh ./build-rust.sh
endef

# ---------------------------------------------------------------------------
# 构建：源码位于 src/<组件>/，各自带独立 Makefile
# ---------------------------------------------------------------------------
define Build/Prepare
	$(RM) -r $(PKG_BUILD_DIR)
	mkdir -p $(PKG_BUILD_DIR)
	$(CP) ./src/libqmodem-sms $(PKG_BUILD_DIR)/
	$(CP) ./src/tom_modem $(PKG_BUILD_DIR)/
	$(CP) ./src/sms-tool_q $(PKG_BUILD_DIR)/
	$(CP) ./src/ubus_at_daemon $(PKG_BUILD_DIR)/
	$(CP) ./src/modem_scan $(PKG_BUILD_DIR)/
	$(CP) ./src/qmodem_smsd $(PKG_BUILD_DIR)/
	$(CP) ./src/quectel_CM_5G_M $(PKG_BUILD_DIR)/
	$(CP) ./src/qmodem-seal $(PKG_BUILD_DIR)/
	# sms-tool_q 的两个补丁在原独立包中由默认 Patch 阶段以 -p1 应用于构建根。
	# 合并后源码位于子目录，因此在 Prepare 阶段显式应用，保持与合并前一致。
	$(PATCH) -g0 -p1 -f -s -d $(PKG_BUILD_DIR)/sms-tool_q < ./patches-src/sms-tool_q/0001-send_raw_pdu.patch
	$(PATCH) -g0 -p1 -f -s -d $(PKG_BUILD_DIR)/sms-tool_q < ./patches-src/sms-tool_q/0002-add_timeout_option.patch
endef

# 构建顺序即依赖顺序：
#   libqmodem-sms 是公共依赖，先构建并发布到 STAGING_DIR；
#   tom_modem 与 qmodem-smsd 链接 -lqmodem-sms。
define Build/Compile
	$(call Build/CompileRust)
	$(MAKE) -C $(PKG_BUILD_DIR)/libqmodem-sms $(TARGET_CONFIGURE_OPTS) \
		CC="$(TARGET_CC)" \
		CFLAGS="$(TARGET_CFLAGS) -fPIC -Wall -Wextra -Werror -std=gnu99 -I." \
		LDFLAGS="$(TARGET_LDFLAGS)"
	$(INSTALL_DIR) $(STAGING_DIR)/usr/lib $(STAGING_DIR)/usr/include/qmodem-sms
	$(CP) $(PKG_BUILD_DIR)/libqmodem-sms/libqmodem-sms.so.1.0.0 $(STAGING_DIR)/usr/lib/
	ln -sf libqmodem-sms.so.1.0.0 $(STAGING_DIR)/usr/lib/libqmodem-sms.so
	ln -sf libqmodem-sms.so.1.0.0 $(STAGING_DIR)/usr/lib/libqmodem-sms.so.1
	$(CP) $(PKG_BUILD_DIR)/libqmodem-sms/qmodem-sms/pdu.h $(STAGING_DIR)/usr/include/qmodem-sms/
	$(MAKE) -C $(PKG_BUILD_DIR)/tom_modem $(TARGET_CONFIGURE_OPTS) \
		CC="$(TARGET_CC)" \
		CFLAGS="$(TARGET_CFLAGS) -I$(STAGING_DIR)/usr/include $(if $(CONFIG_PACKAGE_$(PKG_NAME)_TOM_MODEM_ENABLE_UBUS_DAEMON),-DENABLE_UBUS_DAEMON)" \
		LDFLAGS="$(TARGET_LDFLAGS) -L$(STAGING_DIR)/usr/lib -lubus -lubox -ljson-c -lblobmsg_json -lqmodem-sms" \
		ENABLE_UBUS_DAEMON=$(if $(CONFIG_PACKAGE_$(PKG_NAME)_TOM_MODEM_ENABLE_UBUS_DAEMON),y,n)
	$(MAKE) -C $(PKG_BUILD_DIR)/sms-tool_q $(TARGET_CONFIGURE_OPTS) \
		CC="$(TARGET_CC)" \
		CFLAGS="$(TARGET_CFLAGS)"
	$(MAKE) -C $(PKG_BUILD_DIR)/ubus_at_daemon $(TARGET_CONFIGURE_OPTS) \
		CC="$(TARGET_CC)" \
		CFLAGS="$(TARGET_CFLAGS) -I$(STAGING_DIR)/usr/include" \
		LDFLAGS="$(TARGET_LDFLAGS) -L$(STAGING_DIR)/usr/lib -lubus -lubox -lblobmsg_json -ljson-c -lpthread"
	$(MAKE) -C $(PKG_BUILD_DIR)/modem_scan $(TARGET_CONFIGURE_OPTS) \
		CC="$(TARGET_CC)" \
		STRIP="$(TARGET_CROSS)strip" \
		CFLAGS="$(TARGET_CFLAGS) -Wall -Wextra -std=c99 -D_GNU_SOURCE -I$(STAGING_DIR)/usr/include -I$(STAGING_DIR)/usr/include/json-c" \
		LDFLAGS="$(TARGET_LDFLAGS) -ljson-c -lpthread"
	$(MAKE) -C $(PKG_BUILD_DIR)/qmodem_smsd $(TARGET_CONFIGURE_OPTS) \
		CC="$(TARGET_CC)" \
		CFLAGS="$(TARGET_CFLAGS) -I$(STAGING_DIR)/usr/include -Wall -Wextra -Werror -std=gnu11" \
		LDFLAGS="$(TARGET_LDFLAGS) -L$(STAGING_DIR)/usr/lib -lubus -lubox -lblobmsg_json -ljson-c -luci -lsqlite3 -lqmodem-sms"
	$(MAKE) -C $(PKG_BUILD_DIR)/quectel_CM_5G_M $(TARGET_CONFIGURE_OPTS) \
		CC="$(TARGET_CC)" \
		CFLAGS="$(TARGET_CFLAGS)" \
		release
	if [ "$(CONFIG_PACKAGE_$(PKG_NAME)_INCLUDE_SEAL)" = "y" ]; then \
		$(TARGET_CC) $(TARGET_CFLAGS) $(TARGET_CPPFLAGS) \
			-Wall -Wextra -Werror -o $(PKG_BUILD_DIR)/qmodem-seal/qmodem-seal \
			$(PKG_BUILD_DIR)/qmodem-seal/qmodem-seal.c $(TARGET_LDFLAGS) -lsodium; \
	fi
endef

# ---------------------------------------------------------------------------
# 安装
# ---------------------------------------------------------------------------
define Package/$(PKG_NAME)/install
	# ---- C 组件产物 ----
	$(INSTALL_DIR) $(1)/usr/lib
	$(INSTALL_BIN) $(PKG_BUILD_DIR)/libqmodem-sms/libqmodem-sms.so.1.0.0 $(1)/usr/lib/
	ln -sf libqmodem-sms.so.1.0.0 $(1)/usr/lib/libqmodem-sms.so.1
	$(INSTALL_DIR) $(1)/usr/bin
	$(INSTALL_BIN) $(PKG_BUILD_DIR)/rust-bin/qmodem-atd $(1)/usr/bin/
	$(INSTALL_BIN) $(PKG_BUILD_DIR)/tom_modem/tom_modem $(1)/usr/bin/
	$(INSTALL_BIN) $(PKG_BUILD_DIR)/sms-tool_q/sms_tool $(1)/usr/bin/sms_tool_q
	$(INSTALL_BIN) $(PKG_BUILD_DIR)/ubus_at_daemon/ubus-at-daemon $(1)/usr/bin/
	$(INSTALL_BIN) $(PKG_BUILD_DIR)/modem_scan/modem_scand $(1)/usr/bin/
	$(INSTALL_BIN) $(PKG_BUILD_DIR)/modem_scan/modem_scanc $(1)/usr/bin/
	$(INSTALL_BIN) $(PKG_BUILD_DIR)/quectel_CM_5G_M/quectel-CM $(1)/usr/bin/quectel-CM-M
	if [ "$(CONFIG_PACKAGE_$(PKG_NAME)_INCLUDE_SEAL)" = "y" ]; then \
		$(INSTALL_BIN) $(PKG_BUILD_DIR)/qmodem-seal/qmodem-seal $(1)/usr/bin/ ; \
	fi
	$(INSTALL_DIR) $(1)/usr/sbin
	$(INSTALL_BIN) $(PKG_BUILD_DIR)/qmodem_smsd/qmodem-smsd $(1)/usr/sbin/

	# ---- 内置守护进程与守护脚本 ----
	$(INSTALL_DIR) $(1)/etc/init.d
	$(INSTALL_BIN) ./files/etc/init.d/qmodem-smsd $(1)/etc/init.d/
	$(INSTALL_BIN) ./files/etc/init.d/ubus-at-daemon $(1)/etc/init.d/
	$(INSTALL_BIN) ./files/etc/init.d/qmodem-atd $(1)/etc/init.d/
	$(INSTALL_BIN) ./files/etc/init.d/qmodem-settings $(1)/etc/init.d/
	$(INSTALL_BIN) ./files/etc/init.d/qmodem_monitor $(1)/etc/init.d/
	$(INSTALL_BIN) ./files/etc/init.d/sms_forwarder $(1)/etc/init.d/
	$(INSTALL_BIN) ./files/usr/sbin/qmodem-settings $(1)/usr/sbin/
	$(INSTALL_BIN) ./files/usr/bin/sms_forwarder_next $(1)/usr/bin/
	$(INSTALL_DIR) $(1)/usr/share/sms_forwarder
	$(INSTALL_BIN) ./files/usr/share/sms_forwarder/sms_forward_custom_example.sh \
		./files/usr/share/sms_forwarder/sms_forward_feishu.sh \
		./files/usr/share/sms_forwarder/sms_forward_pushdeer.sh \
		./files/usr/share/sms_forwarder/sms_forward_serverchan.sh \
		./files/usr/share/sms_forwarder/sms_forward_tgbot.sh \
		./files/usr/share/sms_forwarder/sms_forward_webhook.sh $(1)/usr/share/sms_forwarder/
	$(INSTALL_DIR) $(1)/usr/share/qmodem
	$(INSTALL_BIN) ./files/usr/share/qmodem/modem_monitor.sh \
		./files/usr/share/qmodem/monitor_recovery.sh $(1)/usr/share/qmodem/
	$(INSTALL_DIR) $(1)/lib/netifd/proto $(1)/lib/netifd
	$(INSTALL_BIN) ./files/lib/netifd/proto/rmnet.sh ./files/lib/netifd/proto/rmnet6.sh \
		$(1)/lib/netifd/proto/
	$(INSTALL_BIN) ./files/lib/netifd/rmnet.script ./files/lib/netifd/rmnet6.script \
		$(1)/lib/netifd/
	$(INSTALL_DIR) $(1)/etc/uci-defaults
	$(INSTALL_BIN) ./files/etc/uci-defaults/99-qmodem-atd-default $(1)/etc/uci-defaults/
	$(INSTALL_DIR) $(1)/etc/config
	$(INSTALL_CONF) ./files/etc/config/qmodem_sms $(1)/etc/config/
	$(INSTALL_CONF) ./files/etc/config/sms_forwarder $(1)/etc/config/
	$(INSTALL_CONF) ./files/etc/config/ubus-at-daemon $(1)/etc/config/
	$(INSTALL_CONF) ./files/etc/config/qmodem-atd $(1)/etc/config/
	$(INSTALL_DATA) ./files/usr/share/qmodem/apns.json $(1)/usr/share/qmodem/
	$(INSTALL_DATA) ./files/usr/share/qmodem/ref-rmnet-dhcp $(1)/usr/share/qmodem/
	$(INSTALL_DIR) $(1)/usr/share/rpcd/acl.d
	$(INSTALL_DATA) ./files/usr/share/rpcd/acl.d/qmodem-sms.json $(1)/usr/share/rpcd/acl.d/

	# ---- 原有核心脚本 ----
	$(INSTALL_DIR) $(1)/etc/init.d
	$(INSTALL_BIN) ./files/etc/init.d/qmodem_init \
		./files/etc/init.d/qmodem_led \
		./files/etc/init.d/qmodem_network \
		./files/etc/init.d/qmodem_reboot \
		./files/etc/init.d/qmodem_usage_stats $(1)/etc/init.d/
	$(INSTALL_DIR) $(1)/etc/config
	$(INSTALL_CONF) ./files/etc/config/qmodem $(1)/etc/config/
	$(INSTALL_CONF) ./files/etc/config/qmodem_led $(1)/etc/config/
	$(INSTALL_DIR) $(1)/etc/hotplug.d/iface
	$(INSTALL_DATA) ./files/etc/hotplug.d/iface/80-wwan-led $(1)/etc/hotplug.d/iface/
	$(INSTALL_DIR) $(1)/etc/hotplug.d/net
	$(INSTALL_BIN) ./files/etc/hotplug.d/net/20-modem-net $(1)/etc/hotplug.d/net/
	$(INSTALL_DIR) $(1)/etc/hotplug.d/usb
	$(INSTALL_BIN) ./files/etc/hotplug.d/usb/20-modem-usb $(1)/etc/hotplug.d/usb/
	$(INSTALL_DIR) $(1)/etc/board.d
	$(INSTALL_BIN) ./files/etc/board.d/03_qmodem $(1)/etc/board.d/
	$(INSTALL_DIR) $(1)/etc/uci-defaults
	$(INSTALL_DATA) ./files/etc/uci-defaults/luci-app-qmodem $(1)/etc/uci-defaults/
	$(INSTALL_BIN) ./files/etc/uci-defaults/91-qmodem-use-ubus $(1)/etc/uci-defaults/
	$(INSTALL_BIN) ./files/etc/uci-defaults/99-qmodem-board $(1)/etc/uci-defaults/
	$(INSTALL_DIR) $(1)/lib/functions
	$(INSTALL_DATA) ./files/lib/functions/qmodem-defaults.sh $(1)/lib/functions/
	$(INSTALL_DIR) $(1)/usr/lib/lua/luci/view/admin_status/index
	$(INSTALL_DATA) ./files/usr/lib/lua/luci/view/admin_status/index/modem_overview.htm \
		$(1)/usr/lib/lua/luci/view/admin_status/index/
	# 注意：/usr/share/rpcd/acl.d/luci-app-qmodem.json 由前端包
	# luci-app-qmodem 提供（含完整 UI 权限面）。后端不再安装同名文件，
	# 否则两个包会争抢同一路径；原先此文件里的 luci-mod-status-index
	# → modem_ctrl 授权已合并进前端包的 ACL。
	$(INSTALL_DIR) $(1)/usr/share/rpcd/acl.d
	$(INSTALL_DATA) ./files/usr/share/rpcd/acl.d/qmodem.json $(1)/usr/share/rpcd/acl.d/
	$(INSTALL_DIR) $(1)/usr/libexec/rpcd
	$(INSTALL_BIN) ./files/usr/libexec/rpcd/modem_ctrl $(1)/usr/libexec/rpcd/
	$(INSTALL_BIN) ./files/usr/libexec/rpcd/qmodem $(1)/usr/libexec/rpcd/
	$(INSTALL_DIR) $(1)/usr/sbin
	$(INSTALL_BIN) ./files/usr/sbin/qmodem_collect $(1)/usr/sbin/
	$(INSTALL_DIR) $(1)/usr/share/qmodem
	$(INSTALL_DATA) ./files/usr/share/qmodem/at_commands_en.json \
		./files/usr/share/qmodem/at_commands_zh.json \
		./files/usr/share/qmodem/feedback-recipient.conf \
		./files/usr/share/qmodem/modem_port_rule.json \
		./files/usr/share/qmodem/modem_support.json \
		./files/usr/share/qmodem/usage_stats.sh $(1)/usr/share/qmodem/
	$(INSTALL_BIN) ./files/usr/share/qmodem/generic.sh \
		./files/usr/share/qmodem/modem_ctrl.sh \
		./files/usr/share/qmodem/modem_dial.sh \
		./files/usr/share/qmodem/modem_hook.sh \
		./files/usr/share/qmodem/modem_scan.sh \
		./files/usr/share/qmodem/modem_util.sh \
		./files/usr/share/qmodem/qmodem_led_watch.sh $(1)/usr/share/qmodem/
	$(INSTALL_DIR) $(1)/usr/share/qmodem/cmds
	$(INSTALL_DATA) ./files/usr/share/qmodem/cmds/fibocom.sh \
		./files/usr/share/qmodem/cmds/foxconn.sh \
		./files/usr/share/qmodem/cmds/generic.sh \
		./files/usr/share/qmodem/cmds/gosuncn.sh \
		./files/usr/share/qmodem/cmds/huawei.sh \
		./files/usr/share/qmodem/cmds/meig.sh \
		./files/usr/share/qmodem/cmds/modem_dial.sh \
		./files/usr/share/qmodem/cmds/modem_util.sh \
		./files/usr/share/qmodem/cmds/neoway.sh \
		./files/usr/share/qmodem/cmds/openluat.sh \
		./files/usr/share/qmodem/cmds/qmodem_lib.sh \
		./files/usr/share/qmodem/cmds/quectel.sh \
		./files/usr/share/qmodem/cmds/sierra.sh \
		./files/usr/share/qmodem/cmds/simcom.sh \
		./files/usr/share/qmodem/cmds/telit.sh \
		./files/usr/share/qmodem/cmds/thales.sh $(1)/usr/share/qmodem/cmds/
	$(INSTALL_DIR) $(1)/usr/share/qmodem/led_scripts
	$(INSTALL_DATA) ./files/usr/share/qmodem/led_scripts/connectivity.sh \
		./files/usr/share/qmodem/led_scripts/misectel_led.sh \
		$(1)/usr/share/qmodem/led_scripts/
	$(INSTALL_BIN) ./files/usr/share/qmodem/led_scripts/c2000_max.sh \
		./files/usr/share/qmodem/led_scripts/hh500v_led.sh \
		./files/usr/share/qmodem/led_scripts/m01k43.sh \
		./files/usr/share/qmodem/led_scripts/m02k45.sh \
		./files/usr/share/qmodem/led_scripts/m60k63.sh \
		./files/usr/share/qmodem/led_scripts/misectel_3led.sh \
		./files/usr/share/qmodem/led_scripts/misectel_modem_status.sh \
		./files/usr/share/qmodem/led_scripts/misectel_network_detect.sh \
		$(1)/usr/share/qmodem/led_scripts/
	$(INSTALL_DIR) $(1)/usr/share/qmodem/vendor
	$(INSTALL_DATA) ./files/usr/share/qmodem/vendor/dynamic_load.json \
		./files/usr/share/qmodem/vendor/foxconn.sh \
		./files/usr/share/qmodem/vendor/meig.sh \
		./files/usr/share/qmodem/vendor/openluat.sh \
		./files/usr/share/qmodem/vendor/sierra.sh \
		./files/usr/share/qmodem/vendor/simcom.sh \
		./files/usr/share/qmodem/vendor/telit.sh \
		./files/usr/share/qmodem/vendor/thales.sh $(1)/usr/share/qmodem/vendor/
	$(INSTALL_BIN) ./files/usr/share/qmodem/vendor/fibocom.sh \
		./files/usr/share/qmodem/vendor/gosuncn.sh \
		./files/usr/share/qmodem/vendor/huawei.sh \
		./files/usr/share/qmodem/vendor/neoway.sh \
		./files/usr/share/qmodem/vendor/quectel.sh $(1)/usr/share/qmodem/vendor/

	# ---- 通用后端服务（原 luci-app-qmodem-generic 携带，现归后端包） ----
	$(INSTALL_DIR) $(1)/etc/init.d
	$(INSTALL_BIN) ./files/etc/init.d/qmodem-worker \
		./files/etc/init.d/qmodem-stats-collect \
		./files/etc/init.d/qmodem-modem-support \
		./files/etc/init.d/qmodem-mt5700-fix \
		./files/etc/init.d/qmodem_ttl $(1)/etc/init.d/
	$(INSTALL_DIR) $(1)/etc/config
	$(INSTALL_CONF) ./files/etc/config/qmodem_ttl $(1)/etc/config/
	$(INSTALL_DIR) $(1)/etc/uci-defaults
	$(INSTALL_BIN) ./files/etc/uci-defaults/99-qmodem-worker-bootstrap $(1)/etc/uci-defaults/
	$(INSTALL_DIR) $(1)/usr/bin
	$(INSTALL_BIN) ./files/usr/bin/qmodem-stats-collect \
		./files/usr/bin/qmodem-stats-loop \
		./files/usr/bin/qmodem-task $(1)/usr/bin/
	$(INSTALL_DIR) $(1)/usr/sbin
	$(INSTALL_BIN) ./files/usr/sbin/qmodem-worker \
		./files/usr/sbin/qmodem-mt5700-fix \
		./files/usr/sbin/qmodem-modem-support $(1)/usr/sbin/
	$(INSTALL_DIR) $(1)/usr/lib/qmodem
	$(INSTALL_DATA) ./files/usr/lib/qmodem/qmodem-lib.sh $(1)/usr/lib/qmodem/
	$(INSTALL_BIN) ./files/usr/lib/qmodem/qmodem-at-probe $(1)/usr/lib/qmodem/
	$(INSTALL_DIR) $(1)/usr/libexec/rpcd
	$(INSTALL_BIN) ./files/usr/libexec/rpcd/qmodem_cache $(1)/usr/libexec/rpcd/
	$(INSTALL_BIN) ./files/usr/libexec/rpcd/qmodem_stats $(1)/usr/libexec/rpcd/
	$(INSTALL_BIN) ./files/usr/libexec/rpcd/qmodem_support $(1)/usr/libexec/rpcd/
	$(INSTALL_BIN) ./files/usr/libexec/rpcd/qos $(1)/usr/libexec/rpcd/
	$(INSTALL_DIR) $(1)/usr/share/qmodem
	$(INSTALL_DATA) ./files/usr/share/qmodem/extra_modem_support.json $(1)/usr/share/qmodem/
endef

# ---------------------------------------------------------------------------
# 安装后置：本包现在自带 rpcd 插件、procd 服务与采集脚本，
# 因此需要在安装时补权限、注册服务并重启 rpcd（否则 ubus 上没有
# qmodem_cache / qmodem_stats / qmodem_support / qos 对象，界面首屏为空）。
#
# 原则：postinst 必须**快速返回**——它同步阻塞 opkg/apk 安装流程。
# 因此这里只做 chmod + enable，耗时动作（模组支持库合并）放后台，
# modem 探测一律交给 qmodem-worker 服务。
# 随固件镜像打包时 postinst 不执行，脚本类文件必须以 100755 提交。
# ---------------------------------------------------------------------------
define Package/$(PKG_NAME)/postinst
#!/bin/sh
[ -n "$${IPKG_INSTROOT}" ] && exit 0

for f in \
	/usr/libexec/rpcd/qmodem \
	/usr/libexec/rpcd/qmodem_cache \
	/usr/libexec/rpcd/qmodem_stats \
	/usr/libexec/rpcd/qmodem_support \
	/usr/libexec/rpcd/qos \
	/usr/libexec/rpcd/modem_ctrl \
	/usr/bin/qmodem-stats-collect \
	/usr/bin/qmodem-stats-loop \
	/usr/bin/qmodem-task \
	/usr/bin/sms_forwarder_next \
	/usr/sbin/qmodem-worker \
	/usr/sbin/qmodem-modem-support \
	/usr/sbin/qmodem-mt5700-fix \
	/usr/sbin/qmodem-settings \
	/usr/sbin/qmodem_collect \
	/usr/lib/qmodem/qmodem-at-probe
do
	[ -f "$$f" ] && chmod 755 "$$f" 2>/dev/null
done

mkdir -p /etc/qmodem-stats /tmp/qmodem-cache 2>/dev/null || true

# rpcd 需要重启才会加载新的插件与 ACL
/etc/init.d/rpcd restart 2>/dev/null || true

# 启用并拉起后台服务（各自 procd 实例的 start 只注册实例、立刻返回）
for svc in \
	qmodem_init qmodem_network qmodem_led qmodem_monitor \
	ubus-at-daemon qmodem-smsd qmodem-settings \
	qmodem-worker qmodem-stats-collect qmodem-modem-support qmodem-mt5700-fix \
	sms_forwarder qmodem_reboot qmodem_usage_stats
do
	[ -x "/etc/init.d/$$svc" ] && "/etc/init.d/$$svc" enable 2>/dev/null
done

[ -x /usr/sbin/qmodem-worker ] && /etc/init.d/qmodem-worker start 2>/dev/null
[ -x /usr/bin/qmodem-stats-collect ] && /etc/init.d/qmodem-stats-collect start 2>/dev/null
[ -x /usr/sbin/qmodem-smsd ] && /etc/init.d/qmodem-smsd start 2>/dev/null

# 模组支持库合并：纯文件操作但可能较慢，放后台
if [ -x /usr/sbin/qmodem-modem-support ]; then
	( /usr/sbin/qmodem-modem-support >/dev/null 2>&1 & ) 2>/dev/null
fi
exit 0
endef

$(eval $(call BuildPackage,$(PKG_NAME)))
