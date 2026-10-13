#!/bin/sh
qmodem_voip_adapter_device_exists() { [ -c "$1" ]; }

qmodem_voip_adapter_usb_slot()
{
	section=$(qmodem_voip_adapter_section) || return 1
	path=$(uci -q get "qmodem.$section.path") || return 1
	path=${path%/}
	slot=${path##*/}
	case $slot in ''|*[!0-9.-]*) return 1 ;; esac
	[ -d "${QMODEM_VOIP_SYSFS_ROOT:-/sys/bus/usb/devices}/$slot" ] || return 1
	printf '%s\n' "$slot"
}

qmodem_voip_adapter_section()
{
	match=
	for section in $(uci -q show qmodem 2>/dev/null |
		sed -n 's/^qmodem\.\([A-Za-z0-9_]*\)=modem-device$/\1/p'); do
		[ "$(uci -q get "qmodem.$section.name")" = rm520n-gl ] || continue
		[ "$(uci -q get "qmodem.$section.data_interface")" = usb ] || continue
		[ -n "$(uci -q get "qmodem.$section.at_port")" ] || continue
		[ -n "$(uci -q get "qmodem.$section.voice_pcm_port")" ] || continue
		[ -z "$match" ] || return 1
		match=$section
	done
	[ -n "$match" ] || return 1
	printf '%s\n' "$match"
}

qmodem_voip_adapter_enable_ubus()
{
	section=$(qmodem_voip_adapter_section) || return 1
	[ "$(uci -q get "qmodem.$section.use_ubus" 2>/dev/null)" = 1 ] && return 0
	uci -q set "qmodem.$section.use_ubus=1" || return 1
	uci -q commit qmodem
}

qmodem_voip_adapter_usb_id()
{
	slot=$(qmodem_voip_adapter_usb_slot) || return 1
	root=${QMODEM_VOIP_SYSFS_ROOT:-/sys/bus/usb/devices}
	vendor=$(cat "$root/$slot/idVendor" 2>/dev/null) || return 1
	product=$(cat "$root/$slot/idProduct" 2>/dev/null) || return 1
	case $vendor in ''|*[!0-9a-fA-F]*) return 1 ;; esac
	case $product in ''|*[!0-9a-fA-F]*) return 1 ;; esac
	printf '%s:%s\n' "$vendor" "$product"
}

qmodem_voip_adapter_endpoint()
{
	section=$(qmodem_voip_adapter_section) || return 1
	port=$(uci -q get "qmodem.$section.at_port") || return 1
	case $port in /dev/ttyUSB*|/dev/ttyACM*) ;; *) return 1 ;; esac
	qmodem_voip_adapter_device_exists \
		"${QMODEM_VOIP_DEVICE_ROOT:-/dev}/${port#/dev/}" || return 1
	printf '%s\n' "$port"
}

qmodem_voip_adapter_pcm_endpoint()
{
	section=$(qmodem_voip_adapter_section) || return 1
	port=$(uci -q get "qmodem.$section.voice_pcm_port") || return 1
	case $port in /dev/ttyUSB*|/dev/ttyACM*) ;; *) return 1 ;; esac
	qmodem_voip_adapter_device_exists \
		"${QMODEM_VOIP_DEVICE_ROOT:-/dev}/${port#/dev/}" || return 1
	printf '%s\n' "$port"
}

qmodem_voip_adapter_normalize()
{
	command=$1
	response=$(tr -d '\r' | sed '/^AT/d; /^OK$/d; /^$/d')
	case $command in
		AT+CGMM|AT+CGMR) printf '%s\n' "$response" | sed -n '1p' ;;
		AT+CPIN\?) printf '%s\n' "$response" | sed -n 's/^+CPIN: //p' ;;
		AT+CEREG\?)
			status=$(printf '%s\n' "$response" | sed -n 's/^+CEREG: [0-9]*,//p')
			case $status in 1|5) printf '%s\n' registered ;; *) printf '%s\n' unregistered ;; esac ;;
		AT+C5GREG\?)
			status=$(printf '%s\n' "$response" | sed -n 's/^+C5GREG: [0-9]*,//p')
			case $status in 1|5) printf '%s\n' registered ;; *) printf '%s\n' unregistered ;; esac ;;
		AT+QMBNCFG=\"list\")
			case $response in *CMCC*) printf '%s\n' CMCC ;; *CU*) printf '%s\n' CU ;; *) printf '%s\n' none ;; esac ;;
		AT+QCFG=\"ims\") printf '%s\n' "$response" | sed -n 's/^+QCFG: "ims",\([01]\)\(,.*\)\{0,1\}$/\1/p' ;;
		AT+QCALLCFG=\"voice_disable\") printf '%s\n' "$response" | sed -n 's/^+QCALLCFG: "voice_disable",//p' ;;
		AT+QPCMV\?) printf '%s\n' "$response" | sed -n 's/^+QPCMV: //p' ;;
		AT+QDAI\?) printf '%s\n' "$response" | sed -n 's/^+QDAI: //p' ;;
		AT+QAUDCFG=\"qpcmv_cfg\") printf '%s\n' "$response" | sed -n 's/^+QAUDCFG: "[Qq][Pp][Cc][Mm][Vv]_[Cc][Ff][Gg]",//p' ;;
		AT+QAUDMOD\?) printf '%s\n' "$response" | sed -n 's/^+QAUDMOD: //p' ;;
		AT+QGPSCFG=\"outport\") printf '%s\n' "$response" | sed -n 's/^+QGPSCFG: "outport",//p' ;;
		AT+QCFG=\"usbcfg\") printf '%s\n' "$response" | sed -n 's/^+QCFG: "usbcfg",//p' ;;
		*) printf '%s\n' "$response" ;;
	esac
}

# ---------------------------------------------------------------------------
# qmodem-atd unix socket transport (JSON line protocol).
# Replaces the retired ubus `at-daemon` object: the daemon pre-registers every
# modem port at start (`--port`), so there is no separate `open` handshake.
#   request : {"cmd":"list"} / {"cmd":"sendat","port":..,"at":..,"timeout_ms":..}
#   response: {"status":"ok"|"timeout"|"error","lines":[...],"end_flag":..}
# ---------------------------------------------------------------------------

qmodem_voip_atd_socket()
{
	local sock
	sock=$(uci -q get qmodem-atd.main.socket 2>/dev/null)
	[ -n "$sock" ] || sock=/var/run/qmodem/atd.sock
	printf '%s\n' "$sock"
}

#send one request line, print the single json response line
qmodem_voip_atd_request()
{
	local req=$1 sock res bg poll budget out
	sock=$(qmodem_voip_atd_socket)
	[ -S "$sock" ] || return 1
	res=$(mktemp) || return 1
	#the daemon keeps the connection open after replying, so run the
	#transport in background and stop once one full json line arrived
	budget=50
	if command -v nc >/dev/null 2>&1; then
		printf '%s\n' "$req" | nc -U "$sock" >"$res" 2>/dev/null &
		bg=$!
		poll=0
		while [ "$poll" -lt "$budget" ]; do
			[ "$(tail -c1 "$res" 2>/dev/null | wc -l)" = "1" ] && break
			kill -0 "$bg" 2>/dev/null || break
			sleep 0.1
			poll=$((poll + 1))
		done
		kill "$bg" 2>/dev/null
		wait "$bg" 2>/dev/null
	fi
	if [ ! -s "$res" ] && command -v curl >/dev/null 2>&1; then
		: >"$res"
		#HTTP/0.9 mode: accept the raw reply stream; curl's own header lines
		#produce bad-json replies that precede the real response
		curl -s --http0.9 -m 7 --unix-socket "$sock" -d "$req" \
			http://localhost/ >"$res" 2>/dev/null &
		bg=$!
		poll=0
		while [ "$poll" -lt "$budget" ]; do
			[ "$(tail -c1 "$res" 2>/dev/null | wc -l)" = "1" ] && break
			kill -0 "$bg" 2>/dev/null || break
			sleep 0.1
			poll=$((poll + 1))
		done
		kill "$bg" 2>/dev/null
		wait "$bg" 2>/dev/null
	fi
	out=$(tail -n 1 "$res" 2>/dev/null)
	rm -f "$res"
	case $out in
		'{'*'}') printf '%s\n' "$out"; return 0 ;;
	esac
	return 1
}

#extract a scalar/array with jq first, jsonfilter as fallback
qmodem_voip_atd_get()
{
	local json=$1 path=$2
	if command -v jq >/dev/null 2>&1; then
		printf '%s' "$json" | jq -r "$path // empty" 2>/dev/null && return 0
	fi
	printf '%s' "$json" | jsonfilter -e "$path" 2>/dev/null
}

qmodem_voip_adapter_port_open()
{
	target_port=$1
	daemon_ports=$(qmodem_voip_atd_request '{"cmd":"list"}') || return 1
	[ "$(qmodem_voip_atd_get "$daemon_ports" '.status')" = ok ] || return 1
	#`list` answers {"ports":["/dev/ttyUSB2",...]}; ports come from UCI
	#device paths, which never contain quotes, so a substring match on the
	#quoted name is an exact-enough membership test without a JSON parser
	case $daemon_ports in
		*"\"$target_port\""*) return 0 ;;
	esac
	return 1
}

#join the "lines" json array into one newline-separated response blob
qmodem_voip_adapter_lines_to_text()
{
	local raw
	raw=$(qmodem_voip_atd_get "$1" '.lines') || return 1
	[ -n "$raw" ] || return 1
	printf '%s' "$raw" | sed -e 's/^\[[[:space:]]*//' -e 's/[[:space:]]*\]$//' \
		-e 's/",[[:space:]]*"/\n/g' -e 's/^"//' -e 's/"$//'
}

qmodem_voip_adapter_at()
{
	command=$1
	port=$(qmodem_voip_adapter_endpoint) || return 1
	if ! qmodem_voip_adapter_port_open "$port"; then
		logger -t qmodem_voip "atd: port $port not registered, AT \"$command\" unavailable"
		return 1
	fi
	#port and command come from UCI and the fixed AT vocabulary; escape
	#defensively before embedding into the request json
	escaped_port=$(printf '%s' "$port" | sed 's/\\/\\\\/g; s/"/\\"/g')
	escaped_cmd=$(printf '%s' "$command" | sed 's/\\/\\\\/g; s/"/\\"/g')
	req=$(printf '{"cmd":"sendat","port":"%s","at":"%s","timeout_ms":3000}' \
		"$escaped_port" "$escaped_cmd")
	attempt=0
	while [ "$attempt" -lt 3 ]; do
		result=$(qmodem_voip_atd_request "$req") || result=
		if [ "$(qmodem_voip_atd_get "$result" '.status')" = ok ]; then
			response=$(qmodem_voip_adapter_lines_to_text "$result") || return 1
			[ -n "$response" ] || return 1
			printf '%s\n' "$response" | qmodem_voip_adapter_normalize "$command"
			return $?
		fi
		attempt=$((attempt + 1))
		sleep 1
	done
	return 1
}

qmodem_voip_adapter_rediscover()
{
	requested_slot=$1
	slot=$(qmodem_voip_adapter_usb_slot) || return 1
	[ "$slot" = "$requested_slot" ] || return 1
	qmodem_voip_adapter_endpoint || return 1
	qmodem_voip_adapter_pcm_endpoint
}

qmodem_voip_adapter_wait_reenumeration()
{
	slot=$1
	want_audio=$2
	remaining=${3:-30}
	root=${QMODEM_VOIP_SYSFS_ROOT:-/sys/bus/usb/devices}
	proc=${QMODEM_VOIP_PROC_ASOUND_ROOT:-/proc/asound}
	saw_absent=0
	while [ "$remaining" -gt 0 ]; do
		endpoint=$(qmodem_voip_adapter_endpoint 2>/dev/null) || endpoint=
		pcm_endpoint=$(qmodem_voip_adapter_pcm_endpoint 2>/dev/null) || pcm_endpoint=
		[ -d "$root/$slot" ] && [ -n "$endpoint" ] || saw_absent=1
		has_audio=0
		for card in "$root/$slot"/"$slot":*/sound/card*; do
			[ -d "$card" ] && has_audio=1 && break
		done
		if [ "$saw_absent" = 1 ] && [ -n "$endpoint" ] && [ "$has_audio" = "$want_audio" ]; then
			[ "$want_audio" = 0 ] && [ -n "$pcm_endpoint" ] && return 0
			grep -q 'playback.*capture' "$proc/pcm" 2>/dev/null && return 0
		fi
		sleep 1
		remaining=$((remaining - 1))
	done
	return 1
}

if [ "${0##*/}" = at_daemon_adapter.sh ]; then
	case ${1:-} in
	at)
		[ "$#" -eq 2 ] || exit 64
		qmodem_voip_adapter_at "$2"
		;;
	endpoint)
		[ "$#" -eq 1 ] || exit 64
		qmodem_voip_adapter_endpoint
		;;
	*)
		exit 64
		;;
	esac
fi
