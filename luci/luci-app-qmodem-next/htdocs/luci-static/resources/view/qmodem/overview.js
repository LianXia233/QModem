'use strict';
'require view';
'require poll';
'require ui';
'require dom';
'require uci';
'require qmodem.qmodem as qmodem';

// Load shared CSS with daily cache-bust
document.head.appendChild(E('link', {
	'rel': 'stylesheet',
	'type': 'text/css',
	'href': L.resource('qmodem/qmodem-next.css') + '?v=' + new Date().toISOString().slice(0, 10)
}));

// Check if cbi-progressbar class exists
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

// Helper for dynamic SVGs
function createSvg(html) {
	var div = document.createElement('div');
	div.innerHTML = html.trim();
	return div.firstChild;
}

// LuciTable class for rendering modem info tables
var LuciTable = function() {
	this.rows = [];
	this.tbody = null;
	this.fieldset = null;
	this.initTable();
};

LuciTable.prototype = {
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
				var percentage = ((value - min) / (max - min)) * 100;
				
				percentage = Math.max(0, Math.min(100, percentage));
				
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
		} else if (row_length > value_length) {
			for (var i = value_length; i < row_length; i++) {
				this.tbody.removeChild(this.rows[i].row);
			}
			this.rows = this.rows.slice(0, value_length);
		}
		
		for (var i = 0; i < value.length; i++) {
			this.newTr(value[i], i);
		}
	},

	setObjectData: function(value) {
		var row_length = this.rows.length;
		var value_length = Object.keys(value).length;
		
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
		} else if (row_length > value_length) {
			for (var i = value_length; i < row_length; i++) {
				this.tbody.removeChild(this.rows[i].row);
			}
			this.rows = this.rows.slice(0, value_length);
		}
		
		var index = 0;
		for (var key in value) {
			this.rows[index].left.textContent = key;
			this.rows[index].right.textContent = value[key];
			index++;
		}
	}
};

return view.extend({
	load: function() {
		return Promise.all([
			uci.load('qmodem')
		]);
	},

	getModemList: function() {
		var modems = [];
		var sections = uci.sections('qmodem', 'modem-device');
		
		sections.forEach(function(section) {
			if (section.state !== 'disabled' && section.at_port) {
				var name = section.name ? section.name.toUpperCase() : 'Unknown';
				var displayName = section.alias ? section.alias + ' (' + name + ')' : name;
				
				modems.push({
					id: section['.name'],
					name: displayName,
					at_port: section.at_port
				});
			}
		});
		
		return modems;
	},

	getSectionOrder: function(modemId) {
		var key = 'qmodem_section_order_' + modemId;
		var order = localStorage.getItem(key);
		return order ? JSON.parse(order) : [];
	},

	saveSectionOrder: function(modemId, order) {
		var key = 'qmodem_section_order_' + modemId;
		localStorage.setItem(key, JSON.stringify(order));
	},

	getCollapsedState: function(modemId, className) {
		var key = 'qmodem_collapsed_' + modemId + '_' + className;
		return localStorage.getItem(key) === 'true';
	},

	saveCollapsedState: function(modemId, className, isCollapsed) {
		var key = 'qmodem_collapsed_' + modemId + '_' + className;
		localStorage.setItem(key, isCollapsed ? 'true' : 'false');
	},

	attachSectionHandlers: function(fieldset, className, modemId, infoContainer) {
		var self = this;
		var title = fieldset.querySelector('.panel-title');
		
		var toggleCollapse = function(e) {
			if (fieldset.classList.contains('dragging')) return;
			fieldset.classList.toggle('collapsed');
			var isCollapsed = fieldset.classList.contains('collapsed');
			self.saveCollapsedState(modemId, className, isCollapsed);
		};
		
		if (title) title.addEventListener('click', toggleCollapse);
		
		fieldset.addEventListener('dragstart', function(e) {
			fieldset.classList.add('dragging');
			e.dataTransfer.effectAllowed = 'move';
			e.dataTransfer.setData('text/plain', className);
		});
		
		fieldset.addEventListener('dragend', function(e) {
			fieldset.classList.remove('dragging');
			var sections = infoContainer.querySelectorAll('.cbi-section.draggable');
			sections.forEach(function(section) {
				section.classList.remove('drag-over');
			});
		});
		
		fieldset.addEventListener('dragover', function(e) {
			e.preventDefault();
			e.dataTransfer.dropEffect = 'move';
			var draggingElement = infoContainer.querySelector('.dragging');
			if (draggingElement && draggingElement !== fieldset) {
				fieldset.classList.add('drag-over');
			}
		});
		
		fieldset.addEventListener('dragleave', function(e) {
			fieldset.classList.remove('drag-over');
		});
		
		fieldset.addEventListener('drop', function(e) {
			e.preventDefault();
			fieldset.classList.remove('drag-over');
			
			var draggedClassName = e.dataTransfer.getData('text/plain');
			if (draggedClassName === className) return;
			
			var draggedSection = infoContainer.querySelector('.dragging');
			if (draggedSection) {
				infoContainer.insertBefore(draggedSection, fieldset);
			}
			
			var newOrder = Array.from(infoContainer.querySelectorAll('.cbi-section.draggable')).map(function(section) {
				var title = section.querySelector('.panel-title');
				return (title && title.textContent) || '';
			}).filter(function(name) {
				return name !== '';
			});
			
			self.saveSectionOrder(modemId, newOrder);
		});
	},

	// Extract primary indicators from all_info
	extractKpi: function(all_info) {
		var kpi = {
			mode: '—',
			signal: '—',
			signalBars: 0,
			operator: '—',
			band: '—',
			ip: '—',
			connected: false,
			sim: '—',
			slot: '—'
		};

		if (!Array.isArray(all_info)) return kpi;

		all_info.forEach(function(item) {
			if (!item || !item.key) return;
			var k = (item.key || '').toLowerCase();
			var v = (item.value != null ? String(item.value) : '').trim();
			if (!v || v === 'N/A' || v === 'null') return;

			if (k.indexOf('connect_status') !== -1 || k.indexOf('connect') !== -1) {
				kpi.connected = (v.toLowerCase() === 'yes' || v.toLowerCase() === 'connected');
			}
			if (k === 'network_type' || k === 'rat' || k === 'mode' || k === 'network_mode') {
				kpi.mode = v;
			}
			if (k === 'csq' || k === 'signal_strength' || k === 'signal_level' || k === 'rssi') {
				kpi.signal = v;
				var num = parseInt(v, 10);
				if (!isNaN(num)) {
					if (num >= 22) kpi.signalBars = 5;
					else if (num >= 16) kpi.signalBars = 4;
					else if (num >= 11) kpi.signalBars = 3;
					else if (num >= 5) kpi.signalBars = 2;
					else if (num > 0) kpi.signalBars = 1;
				}
			}
			if (k.indexOf('rsrp') !== -1) {
				kpi.signal += ' | RSRP ' + v;
			}
			if (k === 'operator' || k === 'operator_name' || k === 'cops' || k === 'network_provider') {
				kpi.operator = v;
			}
			if (k === 'band' || k === 'primary_band' || k === 'earfcn' || k === 'channel') {
				kpi.band = v;
			}
			if (k === 'ip' || k === 'ip_address' || k === 'ipv4' || k === 'wan_ip') {
				kpi.ip = v;
			}
			if (k === 'sim_state' || k === 'sim_status' || k === 'cpin') {
				kpi.sim = v;
			}
			if (k === 'sim_slot' || k === 'slot') {
				kpi.slot = v;
			}
		});

		return kpi;
	},

	updateKpiDisplay: function(kpiNodes, kpi) {
		if (!kpiNodes) return;
		
		// 1. Signal Card
		if (kpiNodes.modeVal) kpiNodes.modeVal.textContent = kpi.mode !== '—' ? kpi.mode : (kpi.connected ? 'Cellular' : _('Disconnected'));
		if (kpiNodes.signalSub) kpiNodes.signalSub.textContent = kpi.signal !== '—' ? kpi.signal : _('No signal data');
		if (kpiNodes.signalBarsSvg) {
			var bars = kpiNodes.signalBarsSvg.querySelectorAll('.qmd-sig-bar');
			bars.forEach(function(bar, idx) {
				if (idx < kpi.signalBars) {
					bar.setAttribute('fill', 'var(--qmd-ok)');
					bar.setAttribute('opacity', '1');
				} else {
					bar.setAttribute('fill', 'var(--qmd-text-3)');
					bar.setAttribute('opacity', '0.22');
				}
			});
		}

		// 2. Carrier Card
		if (kpiNodes.carrierVal) kpiNodes.carrierVal.textContent = kpi.operator;
		if (kpiNodes.bandSub) kpiNodes.bandSub.textContent = kpi.band !== '—' ? (_('Band') + ': ' + kpi.band) : _('Carrier Aggregation Idle');

		// 3. IP / Uplink Card
		if (kpiNodes.ipVal) kpiNodes.ipVal.textContent = kpi.ip;
		if (kpiNodes.connSub) {
			kpiNodes.connSub.textContent = kpi.connected ? _('Uplink Ready') : _('No Active Session');
			kpiNodes.connChip.className = 'qmodem-chip ' + (kpi.connected ? 'qmodem-chip-ok' : 'qmodem-chip-warn');
			kpiNodes.connChip.textContent = kpi.connected ? _('Connected') : _('Offline');
		}

		// 4. SIM Card
		if (kpiNodes.simVal) kpiNodes.simVal.textContent = kpi.sim !== '—' ? kpi.sim : _('Ready');
		if (kpiNodes.simSub) kpiNodes.simSub.textContent = kpi.slot !== '—' ? (_('Slot %s').format(kpi.slot)) : _('Active SIM Slot');
	},

	updateModemInfo: function(modemId, tables_map, infoContainer, updateTimeElement, copyrightElement, kpiNodes, refreshBtn) {
		var self = this;
		
		if (refreshBtn) {
			refreshBtn.classList.add('spinning');
		}

		// Asynchronous non-blocking fetch of all modem info
		return Promise.all([
			qmodem.getBaseInfo(modemId).catch(function(){ return null; }),
			qmodem.getSimInfo(modemId).catch(function(){ return null; }),
			qmodem.getNetworkInfo(modemId).catch(function(){ return null; }),
			qmodem.getCellInfo(modemId).catch(function(){ return null; }),
			qmodem.getCopyright(modemId).catch(function(){ return null; })
		]).then(function(results) {
			if (refreshBtn) {
				refreshBtn.classList.remove('spinning');
			}

			var all_info = [];
			for (var i = 0; i < results.length - 1; i++) {
				if (results[i] && results[i].modem_info) {
					all_info = all_info.concat(results[i].modem_info);
				}
			}
			
			// Update top KPI cards
			var kpi = self.extractKpi(all_info);
			self.updateKpiDisplay(kpiNodes, kpi);

			// Handle copyright
			var copyrightData = results[results.length - 1];
			if (copyrightElement && copyrightData && copyrightData.copyright) {
				var copyrightInfo = copyrightData.copyright;
				var copyrightText = [];
				for (var key in copyrightInfo) {
					copyrightText.push(_(key) + ': ' + copyrightInfo[key]);
				}
				copyrightElement.textContent = copyrightText.join(' | ');
				copyrightElement.style.display = '';
			} else if (copyrightElement) {
				copyrightElement.style.display = 'none';
			}

			// Group by class
			var grouped = {};
			all_info.forEach(function(entry) {
				if (!entry || entry.type === 'warning_message') return;
				var className = entry['class'] || 'General';
				if (!grouped[className]) grouped[className] = [];
				grouped[className].push(entry);
			});

			// Clear initial loading skeleton
			var skeletons = infoContainer.querySelectorAll('.qmodem-skeleton-card');
			skeletons.forEach(function(sk) { sk.remove(); });

			// Remove obsolete tables
			for (var existingClass in tables_map) {
				if (!grouped[existingClass]) {
					if (tables_map[existingClass].fieldset.parentNode) {
						infoContainer.removeChild(tables_map[existingClass].fieldset);
					}
					delete tables_map[existingClass];
				}
			}

			// Section order preservation
			var sectionOrder = self.getSectionOrder(modemId);
			var orderedClasses = [];
			sectionOrder.forEach(function(className) {
				if (grouped[className]) orderedClasses.push(className);
			});
			for (var className in grouped) {
				if (orderedClasses.indexOf(className) === -1) orderedClasses.push(className);
			}

			orderedClasses.forEach(function(className) {
				if (!tables_map[className]) {
					tables_map[className] = new LuciTable();
					infoContainer.appendChild(tables_map[className].fieldset);
					self.attachSectionHandlers(tables_map[className].fieldset, className, modemId, infoContainer);
				}
				tables_map[className].setTitle(className);
				tables_map[className].setData(grouped[className]);
				
				var collapsedState = self.getCollapsedState(modemId, className);
				if (collapsedState) {
					tables_map[className].fieldset.classList.add('collapsed');
				} else {
					tables_map[className].fieldset.classList.remove('collapsed');
				}
			});
			
			if (updateTimeElement) {
				var now = new Date();
				var timeStr = String(now.getHours()).padStart(2, '0') + ':' + 
					String(now.getMinutes()).padStart(2, '0') + ':' + 
					String(now.getSeconds()).padStart(2, '0');
				updateTimeElement.textContent = _('Last update') + ': ' + timeStr;
			}
		}).catch(function(e) {
			if (refreshBtn) refreshBtn.classList.remove('spinning');
			console.error('Error fetching modem info:', e);
		});
	},

	render: function() {
		var self = this;
		var modems = this.getModemList();
		
		if (modems.length === 0) {
			return E('div', { 'class': 'alert-message warning' }, 
				_('No modems configured or all modems are disabled.'));
		}

		var container = E('div', { 'class': 'cbi-map' });

		// Soft Anime Hero with Animated Dynamic SVG Antenna & Mascot
		var hero = E('div', { 'class': 'qmodem-hero' }, [
			E('h1', { 'class': 'qmodem-hero-title' }, [
				_('Modem Dashboard'),
				E('span', { 'class': 'qmodem-chip qmodem-chip-info' },
					_('%d modem(s) detected').format(modems.length))
			]),
			E('p', { 'class': 'qmodem-hero-sub' },
				_('Real-time signal diagnostics, carrier aggregation and hardware overview. Drag cards to reorder.'))
		]);

		// Animated Mascot & Signal Waves
		var mascot = E('div', { 'class': 'qmodem-mascot qmd-anim-float', 'aria-hidden': 'true' });
		mascot.innerHTML =
			'<svg viewBox="0 0 88 68" xmlns="http://www.w3.org/2000/svg" width="88" height="68">' +
			'<defs>' +
			'<linearGradient id="qmdMascotGrad" x1="0" y1="0" x2="1" y2="1">' +
			'<stop offset="0%" stop-color="#FFFFFF"/>' +
			'<stop offset="100%" stop-color="#EFE9FF"/>' +
			'</linearGradient>' +
			'</defs>' +
			'<circle cx="70" cy="16" r="3" fill="#8875E8" class="qmd-anim-pulse"/>' +
			'<path d="M64 10 A9 9 0 0 1 76 10" fill="none" stroke="#8875E8" stroke-width="1.8" stroke-linecap="round" opacity="0.6"/>' +
			'<path d="M60 6 A15 15 0 0 1 80 6" fill="none" stroke="#EFA8CB" stroke-width="1.8" stroke-linecap="round" opacity="0.4"/>' +
			'<path d="M22 36 Q20 12 32 14 Q38 15 40 24 L48 24 Q50 15 56 14 Q68 12 66 36 Q66 60 44 60 Q22 60 22 36 Z" fill="url(#qmdMascotGrad)"/>' +
			'<path d="M32 14 Q38 15 40 24 L35 26 Q33 19 32 14 Z" fill="#EFA8CB"/>' +
			'<path d="M56 14 Q50 15 48 24 L53 26 Q55 19 56 14 Z" fill="#EFA8CB"/>' +
			'<circle cx="35" cy="38" r="3.2" fill="#34334B"/>' +
			'<circle cx="53" cy="38" r="3.2" fill="#34334B"/>' +
			'<ellipse cx="30" cy="45" rx="4.2" ry="2.5" fill="#EFA8CB" opacity="0.75"/>' +
			'<ellipse cx="58" cy="45" rx="4.2" ry="2.5" fill="#EFA8CB" opacity="0.75"/>' +
			'<path d="M41 46 Q44 49 47 46" stroke="#34334B" stroke-width="1.8" fill="none" stroke-linecap="round"/>' +
			'</svg>';
		hero.appendChild(mascot);
		container.appendChild(hero);

		// Modern Toolbar Card: Modem Selector + Live Refresh Button
		var toolbar = E('div', {
			'class': 'cbi-section',
			'style': 'display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 12px; margin-bottom: 16px; padding: 12px 18px;'
		});

		var selectorWrap = E('div', { 'style': 'display: flex; align-items: center; gap: 10px; flex-wrap: wrap;' });
		selectorWrap.appendChild(E('span', { 'class': 'qmodem-chip qmodem-chip-muted', 'style': 'font-weight: 600;' }, _('Active Modem')));
		
		var select = E('select', {
			'class': 'cbi-input-select',
			'id': 'modem_selector',
			'style': 'min-width: 200px;'
		});
		modems.forEach(function(modem) {
			select.appendChild(E('option', { 'value': modem.id }, modem.name));
		});
		selectorWrap.appendChild(select);
		toolbar.appendChild(selectorWrap);

		var actionsWrap = E('div', { 'style': 'display: flex; align-items: center; gap: 10px;' });
		
		var refreshBtn = E('button', {
			'class': 'qmodem-preset-btn',
			'id': 'qmodem_refresh_btn',
			'type': 'button',
			'title': _('Refresh modem data')
		}, [
			createSvg('<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M21.5 2v6h-6M21.34 15.57a10 10 0 1 1-.57-8.38l5.67-5.67"/></svg>'),
			E('span', {}, _('Refresh'))
		]);

		var updateTimeDiv = E('span', {
			'id': 'update_time_display',
			'class': 'qmodem-chip qmodem-chip-muted'
		}, _('Last update') + ': -');

		actionsWrap.appendChild(refreshBtn);
		actionsWrap.appendChild(updateTimeDiv);
		toolbar.appendChild(actionsWrap);
		container.appendChild(toolbar);

		// 4 Quick Status KPI Cards with Dynamic Animated SVGs
		var kpiGrid = E('div', { 'class': 'qmodem-kpi-grid' });

		// Card 1: Signal & RAT
		var sigBarsSvg = createSvg(
			'<svg viewBox="0 0 32 20" width="32" height="20" fill="none">' +
			'<rect class="qmd-sig-bar" x="2" y="15" width="4" height="5" rx="1.5" fill="var(--qmd-text-3)" opacity="0.25"/>' +
			'<rect class="qmd-sig-bar" x="8" y="11" width="4" height="9" rx="1.5" fill="var(--qmd-text-3)" opacity="0.25"/>' +
			'<rect class="qmd-sig-bar" x="14" y="7" width="4" height="13" rx="1.5" fill="var(--qmd-text-3)" opacity="0.25"/>' +
			'<rect class="qmd-sig-bar" x="20" y="3" width="4" height="17" rx="1.5" fill="var(--qmd-text-3)" opacity="0.25"/>' +
			'<rect class="qmd-sig-bar" x="26" y="0" width="4" height="20" rx="1.5" fill="var(--qmd-text-3)" opacity="0.25"/>' +
			'</svg>'
		);
		var modeVal = E('div', { 'class': 'qmodem-kpi-val' }, '—');
		var signalSub = E('div', { 'class': 'qmodem-kpi-sub' }, _('Waiting for signal data'));
		var cardSignal = E('div', { 'class': 'qmodem-kpi-card' }, [
			E('div', { 'class': 'qmodem-kpi-header' }, [
				E('div', { 'class': 'qmodem-kpi-title' }, [
					E('div', { 'class': 'qmodem-kpi-icon' }, [
						createSvg('<svg viewBox="0 0 24 24" fill="none" stroke="var(--qmd-primary)" stroke-width="2"><path d="M12 20V10M18 20V4M6 20v-4"/></svg>')
					]),
					E('span', {}, _('RAT & Signal'))
				]),
				sigBarsSvg
			]),
			modeVal,
			signalSub
		]);

		// Card 2: Carrier & Band
		var carrierVal = E('div', { 'class': 'qmodem-kpi-val' }, '—');
		var bandSub = E('div', { 'class': 'qmodem-kpi-sub' }, _('Primary Band'));
		var cardCarrier = E('div', { 'class': 'qmodem-kpi-card' }, [
			E('div', { 'class': 'qmodem-kpi-header' }, [
				E('div', { 'class': 'qmodem-kpi-title' }, [
					E('div', { 'class': 'qmodem-kpi-icon', 'style': 'background: var(--qmd-blue);' }, [
						createSvg('<svg viewBox="0 0 24 24" fill="none" stroke="#FFFFFF" stroke-width="2"><path d="M12 2L4 22h16L12 2zM12 11v6"/></svg>')
					]),
					E('span', {}, _('Cellular Carrier'))
				])
			]),
			carrierVal,
			bandSub
		]);

		// Card 3: Uplink & IP
		var ipVal = E('div', { 'class': 'qmodem-kpi-val' }, '—');
		var connSub = E('span', {}, _('Checking session...'));
		var connChip = E('span', { 'class': 'qmodem-chip qmodem-chip-muted' }, _('Status'));
		var cardIp = E('div', { 'class': 'qmodem-kpi-card' }, [
			E('div', { 'class': 'qmodem-kpi-header' }, [
				E('div', { 'class': 'qmodem-kpi-title' }, [
					E('div', { 'class': 'qmodem-kpi-icon', 'style': 'background: var(--qmd-green);' }, [
						createSvg('<svg viewBox="0 0 24 24" fill="none" stroke="#FFFFFF" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/></svg>')
					]),
					E('span', {}, _('Network Session'))
				]),
				connChip
			]),
			ipVal,
			E('div', { 'class': 'qmodem-kpi-sub' }, [connSub])
		]);

		// Card 4: SIM & Hardware
		var simVal = E('div', { 'class': 'qmodem-kpi-val' }, '—');
		var simSub = E('div', { 'class': 'qmodem-kpi-sub' }, _('Slot 1'));
		var cardSim = E('div', { 'class': 'qmodem-kpi-card' }, [
			E('div', { 'class': 'qmodem-kpi-header' }, [
				E('div', { 'class': 'qmodem-kpi-title' }, [
					E('div', { 'class': 'qmodem-kpi-icon', 'style': 'background: var(--qmd-pink);' }, [
						createSvg('<svg viewBox="0 0 24 24" fill="none" stroke="#FFFFFF" stroke-width="2"><path d="M6 3h9l5 5v13H6V3zM10 12h4M10 16h4"/></svg>')
					]),
					E('span', {}, _('SIM Card'))
				])
			]),
			simVal,
			simSub
		]);

		kpiGrid.appendChild(cardSignal);
		kpiGrid.appendChild(cardCarrier);
		kpiGrid.appendChild(cardIp);
		kpiGrid.appendChild(cardSim);
		container.appendChild(kpiGrid);

		var kpiNodes = {
			modeVal: modeVal,
			signalSub: signalSub,
			signalBarsSvg: sigBarsSvg,
			carrierVal: carrierVal,
			bandSub: bandSub,
			ipVal: ipVal,
			connSub: connSub,
			connChip: connChip,
			simVal: simVal,
			simSub: simSub
		};

		// Detailed Info Container (Grid of Collapsible & Draggable Cards)
		var infoContainer = E('div', { 'id': 'modem_info_container', 'class': 'qmodem-cards' });
		
		// Initial Skeleton Placeholders
		for (var k = 0; k < 2; k++) {
			infoContainer.appendChild(E('fieldset', {
				'class': 'cbi-section qmodem-skeleton-card',
				'style': 'height: 140px;'
			}, [
				E('div', { 'class': 'qmodem-skeleton', 'style': 'width: 40%; height: 20px; margin-bottom: 14px;' }),
				E('div', { 'class': 'qmodem-skeleton', 'style': 'width: 100%; height: 16px; margin-bottom: 8px;' }),
				E('div', { 'class': 'qmodem-skeleton', 'style': 'width: 80%; height: 16px;' })
			]));
		}
		container.appendChild(infoContainer);

		// Copyright notice
		var copyrightDiv = E('div', { 'class': 'copyright-section', 'style': 'display: none;' });
		container.appendChild(copyrightDiv);

		var tables_map = {};

		var updateInfo = function(clearTables) {
			var selectedModem = select.value;
			if (clearTables) {
				for (var className in tables_map) {
					if (tables_map[className].fieldset.parentNode) {
						infoContainer.removeChild(tables_map[className].fieldset);
					}
				}
				tables_map = {};
			}
			self.updateModemInfo(selectedModem, tables_map, infoContainer, updateTimeDiv, copyrightDiv, kpiNodes, refreshBtn);
		};

		select.addEventListener('change', function() { updateInfo(true); });
		refreshBtn.addEventListener('click', function() { updateInfo(false); });

		// Initial asynchronous load
		updateInfo(false);

		// Non-blocking background poll (every 10s)
		poll.add(function() {
			var selectedModem = select.value;
			self.updateModemInfo(selectedModem, tables_map, infoContainer, updateTimeDiv, copyrightDiv, kpiNodes, null);
		}, 10);

		return container;
	},

	handleSaveApply: null,
	handleSave: null,
	handleReset: null
});
