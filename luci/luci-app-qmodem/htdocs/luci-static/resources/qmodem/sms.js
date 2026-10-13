'use strict';
'require rpc';
'require qmodem.ui as qmUI';
'require qmodem.qmodem as qmodem';

var callList = rpc.declare({ object: 'qmodem.sms', method: 'list', params: [ 'modem_id', 'limit', 'offset' ], expect: { } });
var callSync = rpc.declare({ object: 'qmodem.sms', method: 'sync', params: [ 'modem_id' ], expect: { } });
var callSend = rpc.declare({ object: 'qmodem.sms', method: 'send', params: [ 'modem_id', 'recipient', 'content' ], expect: { } });
var callDelete = rpc.declare({ object: 'qmodem.sms', method: 'delete', params: [ 'modem_id', 'id', 'index' ], expect: { } });
var callMarkRead = rpc.declare({ object: 'qmodem.sms', method: 'mark_read', params: [ 'modem_id', 'id' ], expect: { } });
var callStorageGet = rpc.declare({ object: 'qmodem.sms', method: 'storage_get', params: [ 'modem_id' ], expect: { } });
var callStorageSet = rpc.declare({ object: 'qmodem.sms', method: 'storage_set', params: [ 'modem_id', 'mem1', 'mem2', 'mem3' ], expect: { } });
var callConfigure = rpc.declare({ object: 'qmodem.sms', method: 'configure', params: [ 'modem_id', 'mode', 'poll_interval', 'forwarding', 'auto_delete' ], expect: { } });
var callModemList = rpc.declare({ object: 'qmodem.sms', method: 'modem_list', expect: { } });
var callConfigGet = rpc.declare({ object: 'qmodem.sms', method: 'config_get', params: [ 'modem_id' ], expect: { } });

function messagesFrom(result) {
	return (result.messages || result.msg || result.received || []).map(function(message) {
		if (!message.type) message.type = 'received';
		if (message.is_read == null) message.is_read = false;
		if (message.success != null && message.is_success == null) message.is_success = message.success;
		return message;
	});
}

function conversations(messages) {
	var grouped = {};
	messages.forEach(function(message) {
		var contact = message.type === 'received' ? message.sender : message.recipient;
		if (!grouped[contact]) grouped[contact] = { contact: contact, messages: [], last_timestamp: 0, unread_count: 0 };
		grouped[contact].messages.push(message);
		grouped[contact].last_timestamp = Math.max(grouped[contact].last_timestamp, message.timestamp || 0);
		if (message.type === 'received' && !message.is_read) grouped[contact].unread_count++;
	});
	return Object.keys(grouped).map(function(key) { return grouped[key]; })
		.sort(function(a, b) { return b.last_timestamp - a.last_timestamp; });
}

function requireModem(configSection) {
	if (!configSection)
		return Promise.reject(new Error(_('configSection (modem_id) is required')));
	return configSection;
}

function listRaw(modem) {
	// Fetch limit is configurable via qmodem.setListLimit()/getListLimit() (default 500)
	return callList(modem, qmodem.getListLimit(), 0).then(function(result) {
		if (result.status === 'error') return Promise.reject(new Error(result.error || _('SMS backend error')));
		return result;
	});
}

function syncRaw(modem) {
	return callSync(modem).then(function(result) {
		if (result.status === 'error') return Promise.reject(new Error(result.error || _('SMS synchronization failed')));
		return result;
	});
}

function deleteMany(modem, ids) {
	ids = Array.isArray(ids) ? ids : [ ids ];
	return Promise.all(ids.map(function(id) { return callDelete(modem, id, id); })).then(function(results) {
		return { success: results.every(function(result) { return result.status === 'success'; }), deleted: results.length };
	});
}

function parseStorage(result) {
	var storage = { mem1: result.configured || 'SM', mem2: 'SM', mem3: 'SM', ME: { used: 0, total: 0 }, SM: { used: 0, total: 0 } };
	var match = (result.response || '').match(/\+CPMS:\s*"?([^",]+)"?,(\d+),(\d+),"?([^",]+)"?,(\d+),(\d+)(?:,"?([^",]+)"?,(\d+),(\d+))?/);
	if (!match) return storage;
	storage.mem1 = match[1]; storage.mem2 = match[4]; storage.mem3 = match[7] || match[4];
	[ [ match[1], match[2], match[3] ], [ match[4], match[5], match[6] ], [ match[7], match[8], match[9] ] ].forEach(function(values) {
		if (values[0] === 'SM' || values[0] === 'ME') storage[values[0]] = { used: +values[1] || 0, total: +values[2] || 0 };
	});
	return storage;
}

return L.Class.extend({
	getModems: function() {
		return callModemList().then(function(result) {
			return (result.modems || []).map(function(modem) {
				return { id: modem.modem_id, name: modem.name || modem.modem_id, enabled: !!modem.enabled };
			});
		});
	},
	getConfig: function(configSection) { return callConfigGet(requireModem(configSection)); },
	configure: function(configSection, mode, pollInterval, forwarding, autoDelete) {
		var modem = requireModem(configSection);
		return callConfigGet(modem).then(function(current) {
			return callConfigure(modem, mode, pollInterval,
				forwarding == null ? !!current.forwarding : forwarding, autoDelete);
		}).then(function(result) {
			if (result.status === 'error') return Promise.reject(new Error(result.error || _('SMS configuration failed')));
			return result;
		});
	},
	listSms: function(configSection) {
		var modem = requireModem(configSection);
		return listRaw(modem).then(function(first) {
			var refresh = first.mode === 'database_poll' ? syncRaw(modem) : Promise.resolve();
			return refresh.then(function() { return first.mode === 'database_poll' ? listRaw(modem) : first; });
		}).then(function(result) {
			var messages = messagesFrom(result);
			return { conversations: conversations(messages), total: messages.length, mode: result.mode };
		});
	},
	getConversation: function(configSection, contact) {
		return this.listSms(configSection).then(function(result) {
			var found = result.conversations.filter(function(item) { return item.contact === contact; })[0];
			return { messages: found ? found.messages : [] };
		});
	},
	sync: function(configSection) { return syncRaw(requireModem(configSection)); },
	sendSms: function(configSection, recipient, message) {
		return callSend(requireModem(configSection), recipient, message).then(function(result) { result.success = result.status === 'success'; return result; });
	},
	deleteSms: function(configSection, type, ids) { return deleteMany(requireModem(configSection), ids); },
	markRead: function(configSection, ids) {
		var modem = requireModem(configSection); ids = Array.isArray(ids) ? ids : [ ids ];
		return Promise.all(ids.map(function(id) { return callMarkRead(modem, id); })).then(function() { return { success: true, marked: ids.length }; });
	},
	getSentHistory: function(configSection) {
		return listRaw(requireModem(configSection)).then(function(result) { return { messages: messagesFrom(result).filter(function(message) { return message.type === 'sent'; }) }; });
	},
	getReceivedHistory: function(configSection) {
		return listRaw(requireModem(configSection)).then(function(result) { return { messages: messagesFrom(result).filter(function(message) { return message.type === 'received'; }) }; });
	},
	clearSentHistory: function(configSection) {
		var modem = requireModem(configSection);
		return this.getSentHistory(modem).then(function(result) { return deleteMany(modem, result.messages.map(function(message) { return message.id; })); });
	},
	clearReceivedHistory: function(configSection) {
		var modem = requireModem(configSection);
		return this.getReceivedHistory(modem).then(function(result) { return deleteMany(modem, result.messages.map(function(message) { return message.id; })); });
	},
	formatTimestamp: function(timestamp) {
		// Delegates to the shared implementation in qmodem/ui.js
		return qmUI.relativeTime(timestamp);
	},
	formatPhoneNumber: function(number) {
		if (!number) return '';
		var digits = number.replace(/\D/g, '');
		return digits.length === 11 && digits.startsWith('1') ? digits.substring(0, 3) + ' ' + digits.substring(3, 7) + ' ' + digits.substring(7) : number;
	},
	truncateMessage: function(content, maximum) {
		maximum = maximum || 50; return !content || content.length <= maximum ? (content || '') : content.substring(0, maximum) + '...';
	},
	getSmsStorage: function(configSection) {
		return callStorageGet(requireModem(configSection)).then(function(result) { return result.status === 'error' ? { error: result.error } : { storage: parseStorage(result) }; });
	},
	setSmsStorage: function(configSection, mem1, mem2, mem3) {
		return callStorageSet(requireModem(configSection), mem1, mem2, mem3 || mem2).then(function(result) { result.success = result.status !== 'error'; return result; });
	},
	getSimSms: function(configSection) {
		return listRaw(requireModem(configSection)).then(function(result) { return { messages: result.msg || result.received || result.messages || [] }; });
	},
	deleteSimSms: function(configSection, index) {
		return callDelete(requireModem(configSection), index, index).then(function(result) { result.success = result.status === 'success'; return result; });
	}
});
