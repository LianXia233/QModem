#!/bin/sh
# Generic AT command wrappers shared by all vendors.
# Vendor scripts must send AT commands only through cmd_* interfaces
# (here and in cmds/<vendor>.sh) so fixtures can be collected and replayed.
# Convention: $1 is always the AT port, remaining args are command parameters.

#query DNS addresses assigned to a PDP context
#$2: pdp_index
cmd_gtdns()
{
    at "$1" "AT+GTDNS=$2"
}

#query PDP context activation state
cmd_cgact_query()
{
    at "$1" 'AT+CGACT?'
}

#query the address of a PDP context
#$2: pdp_index
cmd_cgpaddr()
{
    at "$1" "AT+CGPADDR=$2"
}

#soft reboot the modem
cmd_cfun_soft_reboot()
{
    at "$1" 'AT+CFUN=1,1'
}

#query SMS storage configuration
cmd_cpms_query()
{
    at "$1" 'AT+CPMS?'
}

#set SMS storage; 2 memories ($2,$3) or 3 memories ($2,$3,$4)
cmd_cpms_set()
{
    if [ $# -ge 4 ]; then
        at "$1" "AT+CPMS=\"$2\",\"$3\",\"$4\""
    else
        at "$1" "AT+CPMS=\"$2\",\"$3\""
    fi
}

# ---------------------------------------------------------------------------
# Universal cmd_* wrappers (3GPP TS 27.007 baseline).
# Used by the generic vendor fallback below; vendor cmds/<vendor>.sh override
# these after sourcing, so known modems keep their vendor-specific variants.
# ---------------------------------------------------------------------------
cmd_cgmi() { at "$1" 'AT+CGMI'; }
cmd_cgmm() { at "$1" 'AT+CGMM'; }
cmd_gmr()  { at "$1" 'AT+CGMR'; }
cmd_cgsn() { at "$1" 'AT+CGSN'; }
cmd_cimi() { at "$1" 'AT+CIMI'; }
cmd_cpin_query() { at "$1" 'AT+CPIN?'; }
cmd_cops_query() { at "$1" 'AT+COPS?'; }
cmd_csq() { at "$1" 'AT+CSQ'; }
# best-effort serving cell query; several vendors understand the Quectel style
cmd_qeng_servingcell() { at "$1" 'AT+QENG="servingcell"'; }
# ICCID attempt chain: AT+ICCID? / AT+CCID (vendor specific, try both)
cmd_iccid_query() { at "$1" 'AT+ICCID?'; }
cmd_ccid() { at "$1" 'AT+CCID'; }

# Pick the first meaningful reply line from a raw AT response.
# Drops the command echo, OK/ERROR/CME lines and surrounding quotes; empty
# output means the command failed or is unsupported (graceful degradation).
qmodem_at_pick()
{
    printf '%s' "$1" | tr -d '\r' \
        | grep -v -e '^$' -e '^OK$' -e 'ERROR' -e '^AT' \
        | sed 's/^"\(.*\)"$/\1/' \
        | head -n 1
}

# ---------------------------------------------------------------------------
# Generic vendor implementations.
# These only run when no vendor script was loaded (unknown VID/PID modem,
# manufacturer=generic, resolved via dynamic_load.json -> ../generic.sh).
# Every AT command is a best-effort attempt: unsupported commands yield empty
# values which add_plain_info_entry silently skips.
# ---------------------------------------------------------------------------
base_info()
{
    local name manufacturer revision
    name=$(qmodem_at_pick "$(cmd_cgmm "$at_port")")
    manufacturer=$(qmodem_at_pick "$(cmd_cgmi "$at_port")")
    revision=$(qmodem_at_pick "$(cmd_gmr "$at_port")")
    class="Base Information"
    add_plain_info_entry "name" "$name" "Name"
    add_plain_info_entry "manufacturer" "$manufacturer" "Manufacturer"
    add_plain_info_entry "revision" "$revision" "Revision"
    add_plain_info_entry "at_port" "$at_port" "AT Port"
    get_connect_status
}

sim_info()
{
    local imei sim_status_flag sim_status isp imsi iccid
    imei=$(qmodem_at_pick "$(cmd_cgsn "$at_port")")
    sim_status_flag=$(cmd_cpin_query "$at_port")
    sim_status=$(get_sim_status "$sim_status_flag")
    class="SIM Information"
    add_plain_info_entry "SIM Status" "$sim_status" "SIM Status"
    add_plain_info_entry "IMEI" "$imei" "International Mobile Equipment Identity"
    [ "$sim_status" = "ready" ] || return
    isp=$(cmd_cops_query "$at_port" | tr -d '\r' | sed -n 's/.*+COPS:[^"]*"\([^"]*\)".*/\1/p' | head -n 1)
    imsi=$(qmodem_at_pick "$(cmd_cimi "$at_port")")
    iccid=$(cmd_iccid_query "$at_port" | tr -d '\r' | grep -o '+ICCID:[ ]*[-0-9A-Fa-f]\+' | cut -d ' ' -f 2)
    [ -n "$iccid" ] || iccid=$(cmd_ccid "$at_port" | tr -d '\r' | grep -o '+CCID:[ ]*[-0-9A-Fa-f]\+' | cut -d ' ' -f 2)
    add_plain_info_entry "ISP" "$isp" "Internet Service Provider"
    add_plain_info_entry "IMSI" "$imsi" "International Mobile Subscriber Identity"
    add_plain_info_entry "ICCID" "$iccid" "Integrate Circuit Card Identity"
}

network_info()
{
    local response csq rssi rat_num network_type
    # AT+CSQ is part of TS 27.007 and supported by virtually every modem
    response=$(cmd_csq "$at_port" | tr -d '\r' | grep '+CSQ:' | head -n 1)
    csq=$(echo "$response" | sed 's/.*+CSQ:[ ]*//' | awk -F',' '{print $1}' | tr -d ' "')
    case "$csq" in
        ""|"99") ;;
        *)
            rssi=$((2 * csq - 113))
            [ "$rssi" -lt -113 ] && rssi=-113
            [ "$rssi" -gt -51 ] && rssi=-51
            add_bar_info_entry "RSSI" "$rssi" "Received Signal Strength Indicator" -113 -51 dBm
            ;;
    esac
    # network type from the COPS <AcT> digit (fallback chain: COPS -> get_rat)
    rat_num=$(cmd_cops_query "$at_port" | tr -d '\r"' | grep '+COPS:' | awk -F',' '{print $4}' | tr -d ' ')
    network_type=$(get_rat "$rat_num")
    add_plain_info_entry "Network Type" "$network_type" "Network Type"
    get_connect_status
}

cell_info()
{
    # Best-effort serving cell report via AT+QENG="servingcell". Modems that
    # do not support it reply ERROR and the function degrades to no output.
    local reply rat earfcn pci rsrp rsrq sinr
    reply=$(cmd_qeng_servingcell "$at_port" | tr -d '\r' | grep '^+QENG:' | head -n 1)
    [ -n "$reply" ] || return
    rat=$(echo "$reply" | awk -F',' '{gsub(/[ "]/, "", $3); print $3}')
    earfcn=$(echo "$reply" | awk -F',' '{gsub(/[ "]/, "", $5); print $5}')
    pci=$(echo "$reply" | awk -F',' '{gsub(/[ "]/, "", $6); print $6}')
    rsrp=$(echo "$reply" | awk -F',' '{gsub(/[ "]/, "", $8); print $8}')
    rsrq=$(echo "$reply" | awk -F',' '{gsub(/[ "]/, "", $7); print $7}')
    sinr=$(echo "$reply" | awk -F',' '{gsub(/[ "]/, "", $10); print $10}')
    case "$earfcn" in ''|*[!0-9]*) earfcn="" ;; esac
    case "$pci" in ''|*[!0-9]*) pci="" ;; esac
    case "$rat" in
        "LTE")
            class="Cell Information"
            set_4g_cell_info "" "" "" "" "$earfcn" "$pci" "" "" "" "$rsrp" "$rsrq" "$sinr" "" ""
            ;;
        "NR5G-SA"|"NR5G-NSA")
            class="Cell Information"
            set_5g_cell_info "" "" "" "" "$earfcn" "$pci" "" "" "" "$rsrp" "$rsrq" "$sinr" "" ""
            ;;
    esac
}
