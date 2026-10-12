-- Copyright 2024 Siriling <siriling@qq.com>
-- Copyright 2024 FJR <fjrcn@outlook.com>
module("luci.controller.qmodem", package.seeall)
local http = require "luci.http"
local fs = require "nixio.fs"
local json = require("luci.jsonc")
local uci = luci.model.uci.cursor()
local script_path="/usr/share/qmodem/"
local run_path="/tmp/run/qmodem/"
local modem_ctrl = "/usr/share/qmodem/modem_ctrl.sh "

--[[
@Description shell 单引号转义，防止 HTTP 参数注入命令
]]
local function shq(s)
	return "'" .. s:gsub("'", [['\'']]) .. "'"
end

-- modem_ctrl.sh 已注册的动作名白名单
local modem_actions = {
	base_info = true,
	cell_info = true,
	clear_dial_log = true,
	clear_stats = true,
	delete_sms = true,
	do_reboot = true,
	get_at_cfg = true,
	get_copyright = true,
	get_current_band = true,
	get_current_band_capabilities = true,
	get_disabled_features = true,
	get_dns = true,
	get_imei = true,
	get_lockband = true,
	get_mode = true,
	get_neighborcell = true,
	get_network_prefer = true,
	get_reboot_caps = true,
	get_sim_slot = true,
	get_sim_switch_capabilities = true,
	get_sms = true,
	get_stats = true,
	info = true,
	network_info = true,
	send_at = true,
	send_raw_pdu = true,
	send_sms = true,
	set_imei = true,
	set_lockband = true,
	set_mode = true,
	set_neighborcell = true,
	set_network_prefer = true,
	set_sim_slot = true,
	set_sms_storage = true,
	sim_info = true
}

function index()
    if not nixio.fs.access("/etc/config/qmodem") then
        return
    end
	entry({"admin", "modem"}, firstchild(), _("Modem"), 25).dependent=false
	entry({"admin", "modem", "qmodem"}, alias("admin", "modem", "qmodem", "modem_info"), luci.i18n.translate("QModem"), 100).dependent = true
	--模块信息
	entry({"admin", "modem", "qmodem", "modem_info"}, template("qmodem/modem_info"), luci.i18n.translate("QModem Information"),2).leaf = true
	entry({"admin", "modem", "qmodem", "get_modem_cfg"}, call("getModemCFG"), nil).leaf = true
	entry({"admin", "modem", "qmodem", "modem_ctrl"}, call("modemCtrl")).leaf = true
	--拨号配置
	entry({"admin", "modem", "qmodem", "dial_overview"},cbi("qmodem/dial_overview"),luci.i18n.translate("Dial Overview"),3).leaf = true
	entry({"admin", "modem", "qmodem", "dial_config"}, cbi("qmodem/dial_config")).leaf = true
	entry({"admin", "modem", "qmodem", "modems_dial_overview"}, call("getOverviews"), nil).leaf = true
	--模块调试
	entry({"admin", "modem", "qmodem", "modem_debug"},template("qmodem/modem_debug"),luci.i18n.translate("Advance Modem Settings"),4).leaf = true
	entry({"admin", "modem", "qmodem", "send_at_command"}, call("sendATCommand"), nil).leaf = true

	--Qmodem设置
	entry({"admin", "modem", "qmodem", "settings"}, cbi("qmodem/settings"), luci.i18n.translate("QModem Settings"),100).leaf = true
	entry({"admin", "modem", "qmodem", "slot_config"}, cbi("qmodem/slot_config")).leaf = true
	entry({"admin", "modem", "qmodem", "modem_config"}, cbi("qmodem/modem_config")).leaf = true
end

--[[
@Description 执行Shell脚本
@Params
	command sh命令
]]
function shell(command)
	local odpall = io.popen(command)
	local odp = odpall:read("*a")
	odpall:close()
	return odp
end

function translate_modem_info(result)
	local modem_info = result["modem_info"]
	local response = {}
	for k,entry in pairs(modem_info) do
		if type(entry) == "table" then
			local key = entry["key"]
			local full_name = entry["full_name"]
			if full_name then
				full_name = luci.i18n.translate(full_name)
			elseif key then
				full_name = luci.i18n.translate(key)
			end
			entry["full_name"] = full_name
			if entry["class"] then
				entry["class"] = luci.i18n.translate(entry["class"])
			end
			table.insert(response, entry)
		end
	end
	return response
end

function modemCtrl()
	local action = http.formvalue("action")
	local cfg_id = http.formvalue("cfg")
	local params = http.formvalue("params")
	local translate = http.formvalue("translate")
	-- 白名单校验：action 必须是已注册动作名
	if type(action) ~= "string" or not modem_actions[action] then
		luci.http.prepare_content("application/json")
		luci.http.write('{"error":"invalid action"}')
		return
	end
	-- cfg_id 仅允许字母数字下划线
	if type(cfg_id) ~= "string" or not cfg_id:match("^[A-Za-z0-9_]+$") then
		luci.http.prepare_content("application/json")
		luci.http.write('{"error":"invalid cfg"}')
		return
	end
	local result
	if params then
		result = shell(modem_ctrl..action.." "..cfg_id.." "..shq(params))
	else
		result = shell(modem_ctrl..action.." "..cfg_id)
	end
	if translate == "1" then
		local modem_more_info = json.parse(result)
		modem_more_info = translate_modem_info(modem_more_info)
		result = json.stringify(modem_more_info)
	end
	luci.http.prepare_content("application/json")
	luci.http.write(result)
end

--[[
@Description 执行AT命令
@Params
	at_port AT串口
	at_command AT命令
]]
function at(at_port,at_command)
	local command="source "..script_path.."modem_util.sh && at "..shq(at_port).." "..shq(at_command)
	local result=shell(command)
	result=string.gsub(result, "\r", "")
	return result
end

--[[
@Description AT 串口校验：仅允许 /dev/ 下字母数字下划线斜杠，且设备必须存在
]]
local function valid_at_port(at_port)
	if type(at_port) ~= "string" or not at_port:match("^/dev/[A-Za-z0-9_/]+$") then
		return false
	end
	return fs.access(at_port) ~= nil
end

--[[
@Description AT 命令校验：禁止换行符与反引号
]]
local function valid_at_command(at_command)
	if type(at_command) ~= "string" or at_command == "" then
		return false
	end
	if at_command:find("[\r\n]") or at_command:find("`", 1, true) then
		return false
	end
	return true
end


--[[
@Description 获取模组信息
]]
function getOverviews()
	-- 获取所有模组
	local modems={}
	local logs={}
	uci:foreach("qmodem", "modem-device", function (modem_device)
		local section_name = modem_device[".name"]
		local modem_name = modem_device["name"] or luci.i18n.translate("Unknown")
		local alias = modem_device["alias"]
		local modem_state = modem_device["state"]
		if modem_state == "disabled" then
			return
		end
--模组信息部分
		local cmd = modem_ctrl.."base_info "..section_name
		local result = shell(cmd)
		local json_result = json.parse(result) or "{}"
		local modem_info = json_result["modem_info"]
		local tmp_info = {}
		local title
		if alias then
			title = alias .. "("..modem_name..")"
		else
			title = modem_name
		end
		local name = {
			type = "plain_text",
			key = "name",
			value = title
		}
		table.insert(tmp_info, name)
		for k,v in pairs(modem_info) do
			local full_name = v["full_name"]
			if full_name then
				v["full_name"] = luci.i18n.translate(full_name)
			end
			table.insert(tmp_info, v)
		end
		table.insert(modems, tmp_info)
	--拨号日志部分
	local log_path = run_path..section_name.."_dir/dial_log"
	if fs.access(log_path) then
		local log_msg = fs.readfile(log_path)
		local modem_log = {}
		modem_log["log_msg"] = log_msg
		modem_log["section_name"] = section_name
		if alias then
			modem_log["name"] = alias .. "("..modem_name..")"
		else
			modem_log["name"] = modem_name
		end
		table.insert(logs, modem_log)
	end
	end)
	
	-- 设置值
	local data={}
	data["modems"]=modems
	data["logs"]=logs
	luci.http.prepare_content("application/json")
	luci.http.write_json(data)
end

function getModemCFG()

	local cfgs={}
	local translation={}

	uci:foreach("qmodem", "modem-device", function (modem_device)
		local modem_state = modem_device["state"]
		if modem_state == "disabled" then
			return
		end
		--获取模组的备注
		local network=modem_device["modem"]
		local alias=modem_device["alias"]
		local config_name=modem_device[".name"]
		--设置模组AT串口
		local cfg = modem_device[".name"]
		if modem_device["at_port"] ~= nil then
			local at_port=modem_device["at_port"]
			local name = modem_device["name"] and modem_device["name"]:upper() or luci.i18n.translate("Unknown")
			local config = {}
			if alias then
				config["name"] = alias .. "("..name..")"
			else
				config["name"] = name
			end
			config["at_port"] = at_port
			config["cfg"] = cfg
			table.insert(cfgs, config)
		end
	end)

	-- 设置值
	local data={}
	data["cfgs"]=cfgs
	data["translation"]=translation

	-- 写入Web界面
	luci.http.prepare_content("application/json")
	luci.http.write_json(data)
end



function sendATCommand()
    local at_port = http.formvalue("port")
	local at_command = http.formvalue("command")

	local response={}
    if at_port and at_command and valid_at_port(at_port) and valid_at_command(at_command) then
		response["response"]=at(at_port,at_command)
		response["time"]=os.date("%Y-%m-%d %H:%M:%S")
	else
		response["error"]="invalid at port or command"
    end

	-- 写入Web界面
	luci.http.prepare_content("application/json")
	luci.http.write_json(response)
end
