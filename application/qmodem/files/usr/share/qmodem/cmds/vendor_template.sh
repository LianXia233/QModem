#!/bin/sh
# vendor_template.sh — 新厂商接入模板（copy → rename → 填空）。
#
# QModem 厂商脚本分两层（参见 docs/developer/modem-adaptation.zh-cn.md）：
#   1) cmds/<vendor>.sh   — cmd_* AT 指令封装层：只发 AT，不做解析。
#                           fixture 采集/回放（testcases/）以这些封装为单位。
#   2) vendor/<vendor>.sh — 特性/解析层：调用 cmd_* 封装，把响应解析成
#                           JSON（get_imei / sim_info / base_info / network_info…）。
#
# 本模板把两层写在同一文件里并分段标注；接入时按注释拆成上面两份，
# 把 <vendor> 全部替换为厂商名（与 modem_support.json 的 manufacturer、
# vendor/dynamic_load.json 条目、cmds/ 与 vendor/ 文件名保持一致）。
#
# 约束（与所有 vendor 脚本一致）：
#   - busybox ash / POSIX sh；只使用 busybox applet。
#   - AT 指令只能通过 cmd_* 封装发出（$1 恒为 AT 口）。
#   - 解析一律走 cmds/qmodem_lib.sh 的 ql_* 范式，禁止手写一次性 sed 逻辑。
#
# 自检（修改模板后必须重跑）：
#   sh -n cmds/vendor_template.sh
#   # 独立 /tmp 环境.source（见 docs/developer/new-modem-quickstart.zh-cn.md 第④步）

# ---------------------------------------------------------------------------
# 0) 头部与依赖加载（vendor 层头部；cmd 层单独成文件时不需要这一段）
# ---------------------------------------------------------------------------
# 此处填什么：厂商元信息。_Vendor 必须与 modem_support.json 的
# "manufacturer" 字段完全一致；_Author/_Maintainer 填接入负责人。
_Vendor="<vendor>"
_Author="<author>"
_Maintainer="<author> <email>"
debug_subject="<vendor>_ctrl"

# QMODEM_HOME 允许测试环境重定向；未设置时用固件内默认路径。
: "${QMODEM_HOME:=/usr/share/qmodem}"

# 已在环境中（如 fixture 回放 stub、其他 cmds 脚本）提供时不再重复加载，
# 保证本文件可被独立 source 而不产生函数冲突。
command -v at >/dev/null 2>&1 || {
    [ -f "$QMODEM_HOME/generic.sh" ] && . "$QMODEM_HOME/generic.sh"
}
command -v ql_at_clean >/dev/null 2>&1 || {
    [ -f "$QMODEM_HOME/cmds/qmodem_lib.sh" ] && . "$QMODEM_HOME/cmds/qmodem_lib.sh"
}

# ---------------------------------------------------------------------------
# 1) cmd_* 层：AT 指令封装（cmds/<vendor>.sh 部分）
# ---------------------------------------------------------------------------
# 此处填什么：每个模组用到的 AT 指令一个封装函数，命名 cmd_<vendor>_<含义>（拷贝后把下文 myvendor 统一替换为厂商名）。
# 函数体只有一行 at 调用：$1 是 AT 口，$2 起是指令参数。不要在这里解析。
# 若某指令与通用指令相同（AT+CGSN/AT+CPIN? 等），优先复用 cmds/generic.sh
# 与其他 cmds/*.sh 已有封装，不要重复定义。

# 识别探测：vendor 尚未识别时按命令名调用（回放机制见 testcases/README.md 的
# recognition/ 一节）。此处填什么：模组对 ATI / AT+CGMM / AT+CGMI 的真实支持
# 情况——列在 case 里才会被识别流程采集；不支持返回 1。
cmd_myvendor_identity_query()
{
    case "$2" in
        AT+CGMM|AT+CGMI|AT+CGMR) at "$1" "$2" ;;
        *) return 1 ;;
    esac
}

# 此处填什么：固件版本查询（base_info 的 Revision 来源）。
cmd_myvendor_ati()
{
    at "$1" "ATI"
}

# 此处填什么：SIM 状态查询（sim_info 的 SIM Status 来源）。
cmd_myvendor_cpin_query()
{
    at "$1" "AT+CPIN?"
}

# 此处填什么：信号质量查询（network_info 的信号来源）。
# 示例用通用 AT+CESQ；若厂商有专用指令（如 AT^HCSQ、AT+QNWINFO）换成它。
cmd_myvendor_cesq()
{
    at "$1" "AT+CESQ"
}

# 此处填什么：运营商/网络注册信息查询。
cmd_myvendor_cops_query()
{
    at "$1" "AT+COPS?"
}

# 此处填什么（可选）：带宽查询/设置。不实现锁频就把整段删掉，
# 并在 vendor_get_disabled_features 里声明 LockBand。
cmd_myvendor_band_query()
{
    at "$1" "AT+<BANDCMD>?"
}

# 此处填什么（可选）：锁网（网络偏好）设置；不实现则删掉并声明 DialMode/NetSel。
cmd_myvendor_cnmp_set()
{
    at "$1" "AT+<NETPREF>=$2"
}

# ---------------------------------------------------------------------------
# 2) 特性层：识别后对外暴露的方法（vendor/<vendor>.sh 部分）
# ---------------------------------------------------------------------------
# 约定：读方法输出 JSON（调用方已建好 json 对象，只需 add 字段/子对象）；
# 写方法把结果写进 JSON 返回；解析只用 ql_* 范式：
#   ql_at_clean    — 去 \r（任何 sed -n 'Np' 取行之后都应接一次）
#   ql_csv_field   — 逗号分隔响应取第 n 列
#   ql_regex_pick  — ERE 捕获组提取（多行输入逐行输出，单值消费加 head -n1）

# IMEI：AT+CGSN 第 2 行是 15 位数字。
# 此处填什么：若响应格式不同，改 sed -n 行号或改用 ql_regex_pick 提取。
get_imei()
{
    imei=$(cmd_cgsn "$at_port" | sed -n '2p' | ql_at_clean | head -n1)
    json_add_string "imei" "$imei"
}

# 此处填什么（可选）：写 IMEI。不支持就删除本函数，
# 并在 vendor_get_disabled_features 里声明 "IMEI"。
set_imei()
{
    local imei="$1"
    local res
    res=$(cmd_myvendor_set_imei "$at_port" "$imei")
    json_add_string "set_imei" "$res"
    get_imei
}

# SIM 状态：+CPIN: READY / SIM PIN / SIM PUK ...
# 此处填什么：若厂商响应含前缀（^CPIN: 等），调整 ERE；此函数必须实现，
# 否则首页显示"无 SIM"警告（见 docs/How_to_add_modem_cn.md）。
sim_info()
{
    class="SIM Information"
    sim_status=$(cmd_myvendor_cpin_query "$at_port" | ql_regex_pick '\+CPIN: *([A-Za-z ]+)' | head -n1 | ql_at_clean)
    add_plain_info_entry "SIM Status" "$sim_status" "SIM Status"
}

# 基础信息（首页第一屏）：名称 / 制造商 / 固件版本。
# 此处填什么：把 cmd_cgmm/cmd_cgmi 换成厂商实际响应行号正确的封装；
# 有电压/温度就仿照 vendor/sierra.sh 补 _get_temperature/_get_voltage。
base_info()
{
    name=$(cmd_cgmm "$at_port" | sed -n '2p' | ql_at_clean)
    manufacturer=$(cmd_cgmi "$at_port" | sed -n '2p' | ql_at_clean)
    revision=$(cmd_myvendor_ati "$at_port" | ql_regex_pick 'Revision: *(.*)' | head -n1 | ql_at_clean)
    class="Base Information"
    add_plain_info_entry "name" "$name" "Name"
    add_plain_info_entry "manufacturer" "$manufacturer" "Manufacturer"
    add_plain_info_entry "revision" "$revision" "Revision"
    add_plain_info_entry "at_port" "$at_port" "AT Port"
    get_connect_status
}

# 网络信息（信号区）：+CESQ: <rxlev>,<ber>,<rsrq>,<rsrp>（4G 段为第 3/4 列）。
# 此处填什么：5G 模组通常改用厂商专用指令（+QESQ/^HCSQ/+CPSI 等），
# 用 ql_csv_field 取列、add_bar_info_entry 给出量程下限/上限/单位。
network_info()
{
    class="Network Information"
    local cesq
    cesq=$(cmd_myvendor_cesq "$at_port" | ql_regex_pick '\+CESQ: *(.*)' | head -n1 | ql_at_clean)
    local rsrq rsrp
    rsrq=$(printf '%s\n' "$cesq" | ql_csv_field 3 | ql_num_or 0)
    rsrp=$(printf '%s\n' "$cesq" | ql_csv_field 4 | ql_num_or 0)
    # +CESQ 的 rsrq/rsrp 是枚举值；此处填什么：换成厂商提供的映射表或专用指令。
    add_bar_info_entry "RSRQ" "$rsrq" "RSRQ" -19.5 -3 "dB"
    add_bar_info_entry "RSRP" "$rsrp" "RSRP" -140 -44 "dBm"
}

# 此处填什么（可选）：网络偏好查询。响应形如 +<X>: <code>，
# 用 ql_csv_field 取第 1 列再 case 映射；不实现就删除并声明 NetSel。
get_network_prefer()
{
    local code
    code=$(cmd_myvendor_cnmp_query "$at_port" | ql_regex_pick '\+<NETPREF>: *([0-9]+)' | head -n1 | ql_at_clean)
    local network_prefer_3g="0" network_prefer_4g="0" network_prefer_5g="0"
    case "$code" in
        2)  network_prefer_3g="1" ;;
        3)  network_prefer_3g="1"; network_prefer_4g="1" ;;
        38) network_prefer_4g="1" ;;
        71) network_prefer_5g="1" ;;
        *)  network_prefer_3g="1"; network_prefer_4g="1"; network_prefer_5g="1" ;;
    esac
    json_add_object "network_prefer"
    json_add_string "3G" "$network_prefer_3g"
    json_add_string "4G" "$network_prefer_4g"
    json_add_string "5G" "$network_prefer_5g"
    json_close_object
}

# 此处填什么（可选）：锁频锁小区。参考 vendor/simcom.sh 的
# get_lockband/set_lockband 与 cmd_ccellcfg_* 封装；不支持就在
# vendor_get_disabled_features 声明 "LockBand"/"NeighborCell"。

# ---------------------------------------------------------------------------
# 3) 拨号配合点（get_mode/set_mode）
# ---------------------------------------------------------------------------
# 此处填什么：模组 USB 组合模式切换。available_modes 来自
# modem_support.json 的 "modes" 字段（qmi/ecm/mbim/rndis）；
# 若模组固定模式，删除本函数并只在该字段里登记实际模式。
get_mode()
{
    local available_modes
    available_modes=$(uci -q get "qmodem.$config_section.modes")
    json_add_object "mode"
    for available_mode in $available_modes; do
        json_add_string "$available_mode" "1"
    done
    json_close_object
}

set_mode()
{
    local mode="$1"
    # 此处填什么：mode -> USB 接口掩码/AT 指令的映射（参考 vendor/sierra.sh）。
    case "$mode" in
        ecm|rndis|mbim|qmi) : ;;
        *) return 1 ;;
    esac
    json_add_string "mode" "$mode"
}

# ---------------------------------------------------------------------------
# 4) 能力屏蔽声明（必须保留）
# ---------------------------------------------------------------------------
# 此处填什么：本厂商不支持的功能名，逐行 json_add_string "" "<Feature>"。
# 可用名参考 modem_ctrl.sh 的 get_disabled_features（IMEI/DialMode/
# LockBand/NeighborCell/NetSel/SMS/...）。未声明但未实现的读取方法
# 会在 fixture 回放与真机上报错。
vendor_get_disabled_features()
{
    json_add_string "" "IMEI"
}
