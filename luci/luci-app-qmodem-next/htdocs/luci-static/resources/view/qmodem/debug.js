'use strict';
'require view';
'require ui';
'require dom';
'require qmodem.qmodem as qmodem';

// Load shared QModem Soft Anime design system with cache-bust
document.head.appendChild(E('link', {
	'rel': 'stylesheet',
	'type': 'text/css',
	'href': L.resource('qmodem/qmodem-next.css') + '?v=' + new Date().toISOString().slice(0, 10)
}));

function createSvg(html) {
	var div = document.createElement('div');
	div.innerHTML = html.trim();
	return div.firstChild;
}

function formatTime() {
	var now = new Date();
	var pad = function(n) { return (n < 10 ? '0' : '') + n; };
	return pad(now.getHours()) + ':' + pad(now.getMinutes()) + ':' + pad(now.getSeconds());
}

return view.extend({
	load: function() {
		return qmodem.getModemSections();
	},

	render: function(modems) {
		if (!modems || modems.length === 0) {
			return E('div', { 'class': 'alert-message warning' }, 
				_('No modem configured.'));
		}

		var container = E('div', { 'class': 'cbi-map' });

		// Soft Anime Terminal Hero Header
		var hero = E('div', { 'class': 'qmodem-hero' }, [
			E('h1', { 'class': 'qmodem-hero-title' }, [
				_('AT Command Terminal'),
				E('span', { 'class': 'qmodem-chip qmodem-chip-info' }, _('Interactive Modem Shell'))
			]),
			E('p', { 'class': 'qmodem-hero-sub' },
				_('Execute 3GPP diagnostic commands, test cellular module responses, and inspect raw serial streams.'))
		]);

		// Animated Cyber Terminal SVG Mascot
		var heroIcon = E('div', { 'class': 'qmodem-mascot qmd-anim-float', 'aria-hidden': 'true' });
		heroIcon.innerHTML =
			'<svg viewBox="0 0 76 60" width="76" height="60" fill="none">' +
			'<rect x="8" y="10" width="60" height="42" rx="7" fill="#FFFFFF" stroke="#8875E8" stroke-width="2.5"/>' +
			'<rect x="14" y="16" width="48" height="30" rx="4" fill="#1B2236"/>' +
			'<text x="18" y="27" fill="#36AD87" font-family="monospace" font-size="8.5" font-weight="bold">AT&gt; OK</text>' +
			'<rect x="18" y="32" width="6" height="7" fill="#EFA8CB" class="qmd-anim-pulse"/>' +
			'<circle cx="60" cy="13" r="1.6" fill="#36AD87"/>' +
			'<circle cx="55" cy="13" r="1.6" fill="#E2A44D"/>' +
			'<circle cx="50" cy="13" r="1.6" fill="#D96883"/>' +
			'</svg>';
		hero.appendChild(heroIcon);
		container.appendChild(hero);

		// Modem Selector Toolbar
		var selectorSection = E('div', {
			'class': 'cbi-section',
			'style': 'display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 12px; margin-bottom: 16px; padding: 12px 18px;'
		});

		var leftBar = E('div', { 'style': 'display: flex; align-items: center; gap: 10px; flex-wrap: wrap;' });
		leftBar.appendChild(E('span', { 'class': 'qmodem-chip qmodem-chip-muted', 'style': 'font-weight: 600;' }, _('Active Modem')));
		
		var select = E('select', {
			'class': 'cbi-input-select',
			'id': 'modem_selector',
			'style': 'min-width: 220px;'
		});
		
		modems.forEach(function(modem) {
			if (modem.enabled) {
				select.appendChild(E('option', { 'value': modem.id }, modem.name));
			}
		});
		leftBar.appendChild(select);
		selectorSection.appendChild(leftBar);

		var rightStatus = E('span', { 'class': 'qmodem-chip qmodem-chip-success' }, [
			E('span', { 'class': 'qmodem-dot qmd-pulse', 'style': 'margin-right: 6px;' }),
			_('Serial Engine Online')
		]);
		selectorSection.appendChild(rightStatus);
		container.appendChild(selectorSection);

		// Main AT Interface Container
		var interfaceContainer = E('div', { 'id': 'at_interface_container' });
		container.appendChild(interfaceContainer);

		var self = this;
		var updateInterface = function() {
			var selectedId = select.value;
			var selectedModem = modems.find(function(m) { return m.id === selectedId; });
			if (selectedModem) {
				dom.content(interfaceContainer, self.createAtInterface(selectedModem));
			}
		};

		select.addEventListener('change', updateInterface);
		updateInterface();

		return container;
	},

	createAtInterface: function(modem) {
		var wrap = E('div', {});

		// Loading indicator
		var loadingCard = E('div', { 'class': 'cbi-section qmodem-skeleton-card', 'style': 'padding: 24px; text-align: center;' }, [
			E('div', { 'class': 'spinning' }, _('Initializing AT interface for %s...').format(modem.name))
		]);
		wrap.appendChild(loadingCard);

		// Command History State
		var cmdHistory = [];
		var historyIndex = -1;

		// Load AT configuration
		qmodem.getAtCfg(modem.id).then(function(cfg) {
			wrap.removeChild(loadingCard);
			if (!cfg || !cfg.at_cfg) {
				wrap.appendChild(E('div', { 'class': 'alert-message warning' }, _('Failed to load AT configuration')));
				return;
			}

			var ports = [];
			if (cfg.at_cfg.ports) ports = ports.concat(cfg.at_cfg.ports);
			if (cfg.at_cfg.other_ttys) ports = ports.concat(cfg.at_cfg.other_ttys);

			// Configuration Card: Port selection + Info Chips
			var configCard = E('div', { 'class': 'cbi-section', 'style': 'margin-bottom: 12px; padding: 12px 18px;' });
			var configRow = E('div', { 'style': 'display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 12px;' });

			// Port Selector
			var portField = E('div', { 'style': 'display: flex; align-items: center; gap: 10px;' });
			portField.appendChild(E('label', { 'style': 'font-weight: 600; font-size: 13px; color: var(--qmd-text-2);' }, _('AT Port') + ':'));
			
			var portSelect = E('select', {
				'class': 'cbi-input-select',
				'id': 'at_port_' + modem.id,
				'style': 'min-width: 190px;'
			});
			var currentPort = cfg.at_cfg.using_port || (ports.length > 0 ? ports[0] : '');
			ports.forEach(function(p) {
				var opt = E('option', { 'value': p }, p);
				if (p === currentPort) opt.selected = true;
				portSelect.appendChild(opt);
			});
			portField.appendChild(portSelect);
			configRow.appendChild(portField);

			// Port Info Badges
			var infoBadges = E('div', { 'style': 'display: flex; align-items: center; gap: 8px; flex-wrap: wrap;' }, [
				E('span', { 'class': 'qmodem-chip qmodem-chip-muted' }, '115200 8N1'),
				E('span', { 'class': 'qmodem-chip qmodem-chip-info', 'id': 'active_port_badge_' + modem.id }, 
					currentPort ? currentPort : _('Default Port'))
			]);
			configRow.appendChild(infoBadges);
			configCard.appendChild(configRow);
			wrap.appendChild(configCard);

			// Update Port Badge on change
			portSelect.addEventListener('change', function() {
				var badge = document.getElementById('active_port_badge_' + modem.id);
				if (badge) badge.textContent = portSelect.value || _('Default Port');
			});

			// Declare executeCommand so preset dblclick can trigger it safely
			var executeCommand;

			// Categorized Diagnostic Presets (Compact Tabbed Layout)
			var quickCard = E('div', { 'class': 'cbi-section qmodem-preset-card' });

			// Read collapse and active tab preferences from localStorage
			var isCollapsed = localStorage.getItem('qmodem_debug_preset_collapsed') === '1';
			var activeTabKey = localStorage.getItem('qmodem_debug_preset_tab') || 'device';

			var toggleIcon = createSvg(isCollapsed 
				? '<svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="6 9 12 15 18 9"/></svg>'
				: '<svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="18 15 12 9 6 15"/></svg>');
			var toggleLabel = E('span', {}, isCollapsed ? _('Expand') : _('Collapse'));

			var toggleBtn = E('button', {
				'class': 'qmodem-preset-toggle',
				'type': 'button',
				'title': _('Toggle Quick Diagnostic Presets'),
				'click': function() {
					var body = quickCard.querySelector('.qmodem-preset-body');
					if (!body) return;
					var currentlyCollapsed = body.classList.contains('collapsed');
					if (currentlyCollapsed) {
						body.classList.remove('collapsed');
						localStorage.setItem('qmodem_debug_preset_collapsed', '0');
						toggleLabel.textContent = _('Collapse');
						toggleIcon.innerHTML = '<polyline points="18 15 12 9 6 15"/>';
					} else {
						body.classList.add('collapsed');
						localStorage.setItem('qmodem_debug_preset_collapsed', '1');
						toggleLabel.textContent = _('Expand');
						toggleIcon.innerHTML = '<polyline points="6 9 12 15 18 9"/>';
					}
				}
			}, [toggleIcon, toggleLabel]);

			var headerRow = E('div', { 'class': 'qmodem-preset-header' }, [
				E('div', { 'class': 'qmodem-preset-title' }, [
					createSvg('<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="var(--qmd-primary)" stroke-width="2.2"><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/></svg>'),
					E('span', {}, _('Quick Diagnostic Presets')),
					E('span', { 'class': 'qmodem-preset-hint' }, '(' + _('Click to input, double-click to execute') + ')')
				]),
				toggleBtn
			]);
			quickCard.appendChild(headerRow);

			var presetBody = E('div', { 'class': 'qmodem-preset-body' + (isCollapsed ? ' collapsed' : '') });

			var presetGroups = [
				{
					id: 'device',
					category: _('Device Info'),
					colorClass: 'qmodem-preset-device',
					icon: '<svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2"><rect x="5" y="2" width="14" height="20" rx="2" ry="2"/><line x1="12" y1="18" x2="12.01" y2="18"/></svg>',
					cmds: [
						{ name: 'ATI', label: _('Overview'), desc: _('Device Overview') },
						{ name: 'AT+CGMM', label: _('Model'), desc: _('Device Model') },
						{ name: 'AT+CGMI', label: _('Vendor'), desc: _('Manufacturer') },
						{ name: 'AT+CGMR', label: _('Firmware'), desc: _('Revision & Version') },
						{ name: 'AT+CGSN', label: _('IMEI'), desc: _('IMEI Number') }
					]
				},
				{
					id: 'signal',
					category: _('Signal & Network'),
					colorClass: 'qmodem-preset-signal',
					icon: '<svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2"><path d="M2 20h.01"/><path d="M7 20v-4"/><path d="M12 20v-8"/><path d="M17 20V4"/></svg>',
					cmds: [
						{ name: 'AT+CSQ', label: _('Signal CSQ'), desc: _('Signal Quality (0-31)') },
						{ name: 'AT+CESQ', label: _('Ext Signal'), desc: _('Extended Signal (RSRP/RSRQ)') },
						{ name: 'AT+COPS?', label: _('Operator'), desc: _('Current Network Operator') },
						{ name: 'AT+CEREG?', label: _('EPS Reg'), desc: _('Network Registration Status') },
						{ name: 'AT+CGPADDR', label: _('PDP IP'), desc: _('Modem IP Address') }
					]
				},
				{
					id: 'sim',
					category: _('SIM & Card Slot'),
					colorClass: 'qmodem-preset-sim',
					icon: '<svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="4" width="18" height="16" rx="2"/><line x1="7" y1="8" x2="7" y2="8"/><line x1="7" y1="12" x2="7" y2="12"/></svg>',
					cmds: [
						{ name: 'AT+CPIN?', label: _('SIM Status'), desc: _('PIN / SIM Ready State') },
						{ name: 'AT+CIMI', label: _('IMSI'), desc: _('Subscriber Identity') },
						{ name: 'AT+CCID', label: _('ICCID'), desc: _('SIM Card ICCID') },
						{ name: 'AT+GTDUALSIM?', label: _('Dual SIM'), desc: _('SIM Slot Status') }
					]
				},
				{
					id: 'hw',
					category: _('Hardware & RF'),
					colorClass: 'qmodem-preset-hw',
					icon: '<svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>',
					cmds: [
						{ name: 'AT+CBC', label: _('Voltage'), desc: _('Supply Voltage') },
						{ name: 'AT+GTSENRDTEMP=1', label: _('Temp'), desc: _('Module Temperature') },
						{ name: 'AT+CFUN?', label: _('RF State'), desc: _('Radio Power Function') },
						{ name: 'AT+CFUN=1', label: _('Restart RF'), desc: _('Full Radio Reset') }
					]
				}
			];

			// Append custom commands if present in configuration
			if (cfg.at_cfg.cmds && Array.isArray(cfg.at_cfg.cmds) && cfg.at_cfg.cmds.length > 0) {
				var customCmds = [];
				cfg.at_cfg.cmds.forEach(function(c) {
					if (c.name && c.value) {
						var cleanLabel = c.name.replace(/>.*$/, '').replace(/•.*$/, '').trim();
						if (!cleanLabel) cleanLabel = c.name;
						customCmds.push({ name: c.value, label: cleanLabel, desc: c.name });
					}
				});
				if (customCmds.length > 0) {
					presetGroups.push({
						id: 'custom',
						category: _('Custom Commands'),
						colorClass: 'qmodem-preset-custom',
						icon: '<svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/></svg>',
						cmds: customCmds
					});
				}
			}

			// Add "All" tab to the list
			var tabList = presetGroups.concat([{
				id: 'all',
				category: _('All'),
				icon: '<svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/></svg>'
			}]);

			var validIds = tabList.map(function(t) { return t.id; });
			if (validIds.indexOf(activeTabKey) === -1) {
				activeTabKey = 'device';
			}

			// Render Tab Bar
			var tabsBar = E('div', { 'class': 'qmodem-preset-tabs' });
			var pillsContainer = E('div', { 'class': 'qmodem-preset-pills-container' });

			var updateActiveTab = function(newId) {
				activeTabKey = newId;
				localStorage.setItem('qmodem_debug_preset_tab', newId);
				tabsBar.querySelectorAll('.qmodem-preset-tab').forEach(function(btn) {
					if (btn.getAttribute('data-tab-id') === newId) {
						btn.classList.add('active');
					} else {
						btn.classList.remove('active');
					}
				});
				pillsContainer.querySelectorAll('.qmodem-preset-group-row').forEach(function(row) {
					if (newId === 'all' || row.getAttribute('data-group-id') === newId) {
						row.style.display = 'flex';
					} else {
						row.style.display = 'none';
					}
				});
			};

			tabList.forEach(function(tab) {
				var tabBtn = E('button', {
					'class': 'qmodem-preset-tab' + (tab.id === activeTabKey ? ' active' : ''),
					'type': 'button',
					'data-tab-id': tab.id,
					'click': function() {
						updateActiveTab(tab.id);
					}
				}, [
					createSvg(tab.icon),
					E('span', {}, tab.category)
				]);
				tabsBar.appendChild(tabBtn);
			});
			presetBody.appendChild(tabsBar);

			// Render pill rows
			presetGroups.forEach(function(group) {
				var groupRow = E('div', {
					'class': 'qmodem-preset-group-row',
					'data-group-id': group.id,
					'style': 'display: ' + ((activeTabKey === 'all' || activeTabKey === group.id) ? 'flex' : 'none') + '; flex-wrap: wrap; gap: 6px; margin-top: 8px;'
				});

				group.cmds.forEach(function(cmd) {
					var labelElements = [
						E('span', { 'style': 'font-weight: 700; font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;' }, cmd.name)
					];
					if (cmd.label && cmd.label !== cmd.name) {
						labelElements.push(E('span', { 'style': 'font-size: 11px; opacity: 0.75; font-weight: 500;' }, '• ' + cmd.label));
					}
					var pill = E('button', {
						'class': 'qmodem-preset-pill ' + group.colorClass,
						'type': 'button',
						'title': cmd.desc + ' (' + cmd.name + ')',
						'click': function() {
							var input = document.getElementById('at_command_' + modem.id);
							if (input) {
								input.value = cmd.name;
								input.focus();
							}
						},
						'dblclick': function() {
							var input = document.getElementById('at_command_' + modem.id);
							if (input) {
								input.value = cmd.name;
								if (typeof executeCommand === 'function') {
									executeCommand();
								}
							}
						}
					}, labelElements);
					groupRow.appendChild(pill);
				});
				pillsContainer.appendChild(groupRow);
			});

			presetBody.appendChild(pillsContainer);
			quickCard.appendChild(presetBody);
			wrap.appendChild(quickCard);

			// macOS / Soft Anime Cyber Terminal Frame
			var terminalFrame = E('div', { 'class': 'qmodem-terminal-frame' });

			// Terminal Window Topbar
			var termTopbar = E('div', { 'class': 'qmodem-terminal-topbar' });
			var dotsWrap = E('div', { 'class': 'qmodem-terminal-dots' }, [
				E('span', { 'class': 'qmodem-terminal-dot dot-pink' }),
				E('span', { 'class': 'qmodem-terminal-dot dot-yellow' }),
				E('span', { 'class': 'qmodem-terminal-dot dot-mint' }),
				E('span', { 'style': 'font-size: 12.5px; font-weight: 600; color: var(--qmd-text-2); margin-left: 10px;' }, [
					_('Serial Console'),
					E('span', { 'style': 'opacity: 0.6; margin: 0 4px;' }, '—'),
					E('span', { 'style': 'color: var(--qmd-primary); font-weight: 700;' }, modem.name)
				])
			]);

			var termActions = E('div', { 'style': 'display: flex; gap: 8px;' });
			
			// Copy Output Button
			var copyBtn = E('button', {
				'class': 'qmodem-preset-btn',
				'type': 'button',
				'title': _('Copy terminal content to clipboard'),
				'click': function() {
					var resp = document.getElementById('at_response_' + modem.id);
					if (resp && resp.value) {
						navigator.clipboard.writeText(resp.value).then(function() {
							ui.addNotification(null, E('p', _('Output copied to clipboard')), 'info');
						}).catch(function() {
							ui.addNotification(null, E('p', _('Failed to copy output')), 'error');
						});
					}
				}
			}, [
				createSvg('<svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>'),
				E('span', {}, _('Copy Output'))
			]);

			// Clear Output Button
			var clearBtn = E('button', {
				'class': 'qmodem-preset-btn',
				'type': 'button',
				'title': _('Clear terminal screen'),
				'click': function() {
					var resp = document.getElementById('at_response_' + modem.id);
					if (resp) resp.value = '';
				}
			}, [
				createSvg('<svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="2"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>'),
				E('span', {}, _('Clear'))
			]);

			termActions.appendChild(copyBtn);
			termActions.appendChild(clearBtn);
			termTopbar.appendChild(dotsWrap);
			termTopbar.appendChild(termActions);
			terminalFrame.appendChild(termTopbar);

			// Terminal Body Textarea
			var responseTextarea = E('textarea', {
				'id': 'at_response_' + modem.id,
				'class': 'qmodem-terminal-body',
				'readonly': 'readonly',
				'rows': 16,
				'placeholder': _('Ready. Type an AT command below or select a diagnostic preset above.\nUse ↑ / ↓ arrow keys to cycle through previous command history.')
			});
			terminalFrame.appendChild(responseTextarea);

			// Terminal Prompt Input Row
			var promptRow = E('div', { 'class': 'qmodem-terminal-prompt-row' });
			promptRow.appendChild(E('span', { 'class': 'qmodem-terminal-prompt-label' }, 'AT>'));
			
			var cmdInput = E('input', {
				'type': 'text',
				'class': 'cbi-input-text',
				'id': 'at_command_' + modem.id,
				'style': 'flex: 1; font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace; font-size: 13.5px; height: 38px; border-radius: 8px;',
				'placeholder': 'ATI, AT+CSQ, AT+CPIN?...'
			});

			executeCommand = function() {
				var port = portSelect.value;
				var cmd = cmdInput.value.trim();

				if (!cmd) {
					ui.addNotification(null, E('p', _('Please enter an AT command')), 'warning');
					return;
				}

				// Push into history
				if (cmdHistory[cmdHistory.length - 1] !== cmd) {
					cmdHistory.push(cmd);
				}
				historyIndex = cmdHistory.length;

				sendBtn.disabled = true;
				var originalBtnHtml = sendBtn.innerHTML;
				sendBtn.innerHTML = '<span class="spinning"></span> ' + _('Executing...');

				var timestamp = formatTime();
				var sendEntry = '[' + timestamp + '] >>> ' + cmd + '\n... sending ...';
				responseTextarea.value += (responseTextarea.value ? '\n\n' : '') + sendEntry;
				responseTextarea.scrollTop = responseTextarea.scrollHeight;

				qmodem.sendAt(modem.id, port, cmd).then(function(result) {
					sendBtn.disabled = false;
					sendBtn.innerHTML = originalBtnHtml;

					var text = '';
					if (result && result.at_cfg) {
						var isOk = result.at_cfg.status === '1';
						var statusTag = isOk ? '[OK]' : '[ERROR]';
						var resContent = (result.at_cfg.res || '').trim();
						if (!resContent) {
							resContent = isOk ? 'OK' : (result.at_cfg.error || _('No response content'));
						}
						text = statusTag + '\n' + resContent;
					} else {
						text = _('[No response received from modem]');
					}

					responseTextarea.value = responseTextarea.value.replace(/\n\.\.\. sending \.\.\.$/, '\n' + text);
					responseTextarea.scrollTop = responseTextarea.scrollHeight;
				}).catch(function(e) {
					sendBtn.disabled = false;
					sendBtn.innerHTML = originalBtnHtml;
					responseTextarea.value = responseTextarea.value.replace(/\n\.\.\. sending \.\.\.$/, '\n[RPC ERROR: ' + e.message + ']');
					responseTextarea.scrollTop = responseTextarea.scrollHeight;
				});
			};

			// Send Button
			var sendBtn = E('button', {
				'class': 'qmodem-btn-send',
				'type': 'button',
				'click': executeCommand
			}, [
				createSvg('<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.2"><line x1="22" y1="2" x2="11" y2="13"/><polygon points="22 2 15 22 11 13 2 9 22 2"/></svg>'),
				E('span', {}, _('Send Command'))
			]);

			// Keyboard Listeners: Enter to execute, Up/Down for command history
			cmdInput.addEventListener('keydown', function(ev) {
				if (ev.key === 'Enter') {
					ev.preventDefault();
					executeCommand();
				} else if (ev.key === 'ArrowUp') {
					ev.preventDefault();
					if (cmdHistory.length > 0 && historyIndex > 0) {
						historyIndex--;
						cmdInput.value = cmdHistory[historyIndex];
					}
				} else if (ev.key === 'ArrowDown') {
					ev.preventDefault();
					if (historyIndex < cmdHistory.length - 1) {
						historyIndex++;
						cmdInput.value = cmdHistory[historyIndex];
					} else {
						historyIndex = cmdHistory.length;
						cmdInput.value = '';
					}
				}
			});

			promptRow.appendChild(cmdInput);
			promptRow.appendChild(sendBtn);
			terminalFrame.appendChild(promptRow);
			wrap.appendChild(terminalFrame);

		}).catch(function(e) {
			wrap.removeChild(loadingCard);
			wrap.appendChild(E('div', { 'class': 'alert-message error' },
				_('Error loading AT configuration: %s').format(e.message)));
		});

		return wrap;
	},

	handleSaveApply: null,
	handleSave: null,
	handleReset: null
});
