'use strict';
'require ui';
'require dom';

/*
 * Shared UI components for luci-app-qmodem-next views.
 *
 * Scope note: view/status/include/11_modem.js shares InfoTable/progressbar
 * rendering with view/qmodem/overview.js through this module. The legacy
 * luci-app-qmodem package ships its own duplicate of 11_modem.js; that copy
 * is intentionally untouched here and will be consolidated during the
 * legacy-package retirement phase.
 */

// --- CSS feature detection --------------------------------------------------

function hasCssClass(className) {
	var styleSheets = document.styleSheets;
	for (var i = 0; i < styleSheets.length; i++) {
		try {
			var rules = styleSheets[i].cssRules || styleSheets[i].rules;
			if (rules) {
				for (var j = 0; j < rules.length; j++) {
					if (rules[j].selectorText && rules[j].selectorText.indexOf(className) !== -1) {
						return true;
					}
				}
			}
		} catch(e) {
			// Cross-origin stylesheet, skip
		}
	}
	return false;
}

var progressbar_className = hasCssClass('.cbi-progressbar') ? 'cbi-progressbar' : 'compat-progressbar';

// --- Info table (used by overview.js and status/include/11_modem.js) --------

var InfoTable = function() {
	this.rows = [];
	this.tbody = null;
	this.fieldset = null;
	this.initTable();
};

InfoTable.prototype = {
	initTable: function() {
		this.fieldset = E('fieldset', { 'class': 'cbi-section collapsible draggable', 'draggable': 'true' });
		this.title_span = E('h2', { 'class': 'panel-title' });
		var table = E('table', { 'class': 'table' });
		this.tbody = E('tbody', { 'style': 'width: 100%' });

		table.appendChild(this.tbody);
		this.fieldset.appendChild(this.title_span);
		this.fieldset.appendChild(table);
	},

	setTitle: function(value) {
		var translatedValue = _(value);
		this.title_span.textContent = translatedValue;
	},

	newTr: function(data, index) {
		var type = data.type;
		switch(type) {
			case 'plain_text':
				var key = data.key;
				var value = data.value;
				var full_name = data.full_name || key;
				var extra_info = data.extra_info;

				// Translate full_name and extra_info
				var translatedName = _(full_name);
				var displayName = extra_info ? translatedName + ' (' + extra_info + ')' : translatedName;

				this.rows[index].left.textContent = displayName;
				this.rows[index].right.textContent = value;

				if (value == null || value == '') {
					this.rows[index].row.style.display = 'none';
				} else {
					this.rows[index].row.style.display = '';
				}
				break;

			case 'progress_bar':
				var key = data.key;
				var full_name = data.full_name || key;
				var extra_info = data.extra_info;
				var value = parseFloat(data.value);
				var min = parseFloat(data.min_value);
				var max = parseFloat(data.max_value);
				var unit = data.unit || '';
				var title = '(' + data.value + '/' + data.max_value + unit + ')';
				// Guard against division by zero / degenerate ranges
				var percentage = (max - min > 1) ?
					((value - min) / (max - min)) * 100 :
					(value > min ? 100 : 0);

				// Clamp percentage between 0 and 100
				percentage = Math.max(0, Math.min(100, percentage));
				if (isNaN(percentage)) percentage = 0;

				// Translate full_name and extra_info
				var translatedName = _(full_name);
				var displayName = extra_info ? translatedName + ' (' + extra_info + ')' : translatedName;

				this.rows[index].left.textContent = displayName;

				var progress_bar = E('div', {
					'class': progressbar_className,
					'title': title
				});
				var progress_bar_bar = E('div', { 'style': 'width:' + percentage + '%' });
				progress_bar.appendChild(progress_bar_bar);

				this.rows[index].right.innerHTML = '';
				this.rows[index].right.appendChild(progress_bar);

				this.rows[index].row.style.display = '';
				break;
		}
	},

	setData: function(value) {
		if (value == null) return;

		if (Array.isArray(value)) {
			this.setArrayData(value);
		} else {
			this.setObjectData(value);
		}
	},

	setArrayData: function(value) {
		var row_length = this.rows.length;
		var value_length = value.length;

		// Add missing rows
		if (row_length < value_length) {
			for (var i = row_length; i < value_length; i++) {
				var row = E('tr', { 'class': 'tr' });
				var cell_left = E('td', { 'class': 'td left', 'width': '33%' });
				var cell_right = E('td', { 'class': 'td' });
				row.appendChild(cell_left);
				row.appendChild(cell_right);
				this.tbody.appendChild(row);

				this.rows.push({
					row: row,
					left: cell_left,
					right: cell_right
				});
			}
		}
		// Remove extra rows
		else if (row_length > value_length) {
			for (var i = value_length; i < row_length; i++) {
				this.tbody.removeChild(this.rows[i].row);
			}
			this.rows = this.rows.slice(0, value_length);
		}

		// Update row content
		for (var i = 0; i < value.length; i++) {
			this.newTr(value[i], i);
		}
	},

	setObjectData: function(value) {
		var row_length = this.rows.length;
		var value_length = Object.keys(value).length;

		// Add missing rows
		if (row_length < value_length) {
			for (var i = row_length; i < value_length; i++) {
				var row = E('tr', { 'class': 'tr' });
				var cell_left = E('td', { 'class': 'td left', 'width': '33%' });
				var cell_right = E('td', { 'class': 'td' });
				row.appendChild(cell_left);
				row.appendChild(cell_right);
				this.tbody.appendChild(row);

				this.rows.push({
					row: row,
					left: cell_left,
					right: cell_right
				});
			}
		}
		// Remove extra rows
		else if (row_length > value_length) {
			for (var i = value_length; i < row_length; i++) {
				this.tbody.removeChild(this.rows[i].row);
			}
			this.rows = this.rows.slice(0, value_length);
		}

		// Update row content
		var index = 0;
		for (var key in value) {
			this.rows[index].left.textContent = key;
			this.rows[index].right.textContent = value[key];
			index++;
		}
	}
};

// --- Progress bar (used by status/include/11_modem.js) ----------------------

function progressbar(value, max, min, unit) {
	var val = parseInt(value) || 0,
		maximum = parseInt(max),
		minimum = parseInt(min),
		u = unit || '';

	if (isNaN(maximum)) maximum = 100;
	if (isNaN(minimum)) minimum = 0;

	// Guard against division by zero / degenerate ranges
	var pc = (maximum - minimum > 1) ?
		((val - minimum) / (maximum - minimum)) * 100 :
		(val > minimum ? 100 : 0);
	pc = Math.max(0, Math.min(100, pc));
	if (isNaN(pc)) pc = 0;

	return E('div', {
		'class': progressbar_className,
		'title': '%s / %s%s (%d%%)'.format(val, maximum, u, Math.round(pc))
	}, E('div', { 'style': 'width:' + pc + '%' }));
}

// --- Modem selector ----------------------------------------------------------
// Single source of the "Modem Name" selector template previously duplicated in
// overview.js / debug.js / config_advanced.js / sim_switch.js.

function ModemSelector(modems, options) {
	options = options || {};

	var select = E('select', {
		'class': 'cbi-input-select',
		'id': options.id || 'modem_selector'
	});

	(modems || []).forEach(function(modem) {
		if (!options.filter || options.filter(modem)) {
			select.appendChild(E('option', { 'value': modem.id }, modem.name));
		}
	});

	var labelCell = E('td', {
		'class': options.leftAlignLabel ? 'td left' : 'td',
		'width': '33%'
	}, _('Modem Name'));
	var selectCell = E('td', { 'class': 'td' });
	selectCell.appendChild(select);

	var selectorBody = E('tbody', {});
	selectorBody.appendChild(E('tr', { 'class': 'tr' }, [ labelCell, selectCell ]));

	var selectorTable = E('table', { 'class': 'table' });
	selectorTable.appendChild(selectorBody);

	var selectorSection = E('fieldset', { 'class': 'cbi-section' });
	selectorSection.appendChild(selectorTable);

	return { section: selectorSection, select: select, body: selectorBody };
}

// --- Time formatting ---------------------------------------------------------

// Full timestamp: Date object or epoch seconds -> 'YYYY-MM-DD HH:mm:ss'
function formatTime(value) {
	var date = (value instanceof Date) ? value : new Date(value * 1000);
	if (isNaN(date.getTime())) return '-';

	var pad = function(n) { return String(n).padStart(2, '0'); };
	return date.getFullYear() + '-' + pad(date.getMonth() + 1) + '-' + pad(date.getDate()) + ' ' +
		pad(date.getHours()) + ':' + pad(date.getMinutes()) + ':' + pad(date.getSeconds());
}

// Relative timestamp: epoch seconds -> HH:mm today / Yesterday / weekday / date
function relativeTime(timestamp) {
	var date = new Date(timestamp * 1000), now = new Date(), days = Math.floor((now - date) / 86400000);
	if (days === 0) return String(date.getHours()).padStart(2, '0') + ':' + String(date.getMinutes()).padStart(2, '0');
	if (days === 1) return _('Yesterday');
	if (days < 7) return [ _('Sunday'), _('Monday'), _('Tuesday'), _('Wednesday'), _('Thursday'), _('Friday'), _('Saturday') ][date.getDay()];
	return date.getFullYear() + '-' + String(date.getMonth() + 1).padStart(2, '0') + '-' + String(date.getDate()).padStart(2, '0');
}

// --- Poll guard ---------------------------------------------------------------
// Wraps polling failure handlers: logs once and rate-limits the UI
// notification (default: at most one notification every 30s per context)
// so background poll errors do not spam the message area.

var pollGuardLastShown = {};

function pollGuard(context, intervalMs) {
	intervalMs = intervalMs || 30000;

	return function(err) {
		console.error('Poll error [' + context + ']:', err);

		var now = Date.now();
		if (!pollGuardLastShown[context] || (now - pollGuardLastShown[context]) > intervalMs) {
			pollGuardLastShown[context] = now;
			ui.addNotification(null,
				E('p', _('Background refresh failed: %s').format((err && err.message) ? err.message : err)),
				'error');
		}
	};
}

return L.Class.extend({
	hasCssClass: hasCssClass,
	InfoTable: InfoTable,
	progressbar: progressbar,
	ModemSelector: ModemSelector,
	formatTime: formatTime,
	relativeTime: relativeTime,
	pollGuard: pollGuard
});
