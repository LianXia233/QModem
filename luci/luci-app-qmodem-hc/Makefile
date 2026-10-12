# Copyright (C) 2024 Tom <fjrcn@outlook.com>
# This is free software, licensed under the GNU General Public License v3.

include $(TOPDIR)/rules.mk

# Include unified version
include ../../version.mk

PKG_NAME:=luci-app-qmodem-hc
LUCI_TITLE:=Luci qwrt modem sim switch (legacy, maintenance mode)
LUCI_PKGARCH:=all
PKG_VERSION:=$(QMODEM_VERSION)
PKG_LICENSE:=GPLv3
PKG_LICENSE_FILES:=LICENSE
PKG_MAINTAINER:=Tom <fjrcn@outlook.com>
# legacy: 硬依赖 +luci-app-qmodem 仅为旧栈 UI 入口；Step 2 后切换为 +qmodem
LUCI_DEPENDS:=@TARGET_mediatek_mt7981_DEVICE_hc-g80||TARGET_mediatek_filogic_DEVICE_hc-g80 +luci-app-qmodem

# legacy: HC-G80 专用 SIM 切换链路（本包独有，全仓库唯一 fw_printenv/fw_setenv 使用点）：
#   - GPIO 直写 /sys/class/gpio/sim/value（1=SIM1, 0=SIM2）与 /sys/class/gpio/4g/value（模组复位）
#   - uboot env sim2（fw_printenv -n sim2 / fw_setenv sim2 [1]）决定 next boot 槽位
# gap: luci-app-qmodem-next 的 set_sim_slot 走 vendor AT 路径（如 AT+QUIMSLOT），
#      无 fw_setenv/GPIO 等效 ubus 端点；迁移前需实机等效性验证，
#      见 docs/refactor/journal-retire-hc.md（agent-retire-hc）
		

define Package/luci-app-qmodem-hc/conffiles
/etc/config/qmodem_hc_sim
endef

include $(TOPDIR)/feeds/luci/luci.mk

# call BuildPackage - OpenWrt buildroot signature
