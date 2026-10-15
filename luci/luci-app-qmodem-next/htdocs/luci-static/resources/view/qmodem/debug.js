'use strict';
'require view';
'require ui';
'require dom';
'require qmodem.qmodem as qmodem';

// Load shared QModem Soft Anime design system with daily cache-bust
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

		var rightStatus = E('span', { 'class': 'qmodem-chip qmodem-chip-info' }, _('Serial Ready'));
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
		var self = this;
		var wrap = E('div', {});

		// Loading indicator
		var loadingCard = E('div', { 'class': 'cbi-section qmodem-skeleton-card', 'style': 'padding: 24px; text-align: center;' }, [
			E('div', { 'class': 'spinning' }, _('Initializing AT interface for %s...').format(modem.name))
		]);
		wrap.appendChild(loadingCard);

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

			// Configuration Card: Port selection + Ubus Mode
			var configCard = E('div', { 'class': 'cbi-section', 'style': 'margin-bottom: 16px; padding: 14px 18px;' });
			var configRow = E('div', { 'style': 'display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 16px;' });

			// Port Selector
			var portField = E('div', { 'style': 'display: flex; align-items: center; gap: 8px;' });
			portField.appendChild(E('label', { 'style': 'font-weight: 600; font-size: 13px; color: var(--qmd-text-2);' }, _('AT Port') + ':'));
			
			var portSelect = E('select', {
				'class': 'cbi-input-select',
				'id': 'at_port_' + modem.id,
				'style': 'min-width: 180px;'
			});
			var currentPort = cfg.at_cfg.using_port || (ports.length > 0 ? ports[0] : '');
			ports.forEach(function(p) {
				var opt = E('option', { 'value': p }, p);
				if (p === currentPort) opt.selected = true;
				portSelect.appendChild(opt);
			});
			portField.appendChild(portSelect);
			configRow.appendChild(portField);

			// Ubus Mode Toggle
			var ubusField = E('div', { 'style': 'display: flex; align-items: center; gap: 8px;' });
			var ubusCheckbox = E('input', {
				'type': 'checkbox',
				'id': 'use_ubus_' + modem.id,
				'checked': modem.use_ubus !== '0'
			});
			ubusField.appendChild(ubusCheckbox);
			ubusField.appendChild(E('label', { 'for': 'use_ubus_' + modem.id, 'style': 'font-size: 13px; cursor: pointer;' },
				_('Use Ubus AT Daemon (non-blocking)')));
			configRow.appendChild(ubusField);
			configCard.appendChild(configRow);
			wrap.appendChild(configCard);

			// Quick Commands Section
			var quickCard = E('div', { 'class': 'cbi-section', 'style': 'margin-bottom: 16px; padding: 12px 18px;' });
			quickCard.appendChild(E('div', { 'style': 'font-size: 12.5px; font-weight: 600; color: var(--qmd-text-2); margin-bottom: 8px;' },
				_('Quick Diagnostic Presets')));
			
			var quickChips = E('div', { 'style': 'display: flex; flex-wrap: wrap; gap: 8px;' });
			
			// Built-in popular AT commands
			var standardCmds = [
				{ name: 'ATI (Info)', value: 'ATI' },
				{ name: 'AT+CSQ (Signal)', value: 'AT+CSQ' },
				{ name: 'AT+CPIN? (SIM)', value: 'AT+CPIN?' },
				{ name: 'AT+CEREG? (EPS)', value: 'AT+CEREG?' },
				{ name: 'AT+COPS? (Operator)', value: 'AT+COPS?' },
				{ name: 'AT+CIMI (IMSI)', value: 'AT+CIMI' }
			];

			// Merge custom commands from cfg
			if (cfg.at_cfg.cmds && Array.isArray(cfg.at_cfg.cmds)) {
				cfg.at_cfg.cmds.forEach(function(c) {
					if (c.name && c.value) standardCmds.push(c);
				});
			}

			standardCmds.forEach(function(cmd) {
				var chip = E('button', {
					'class': 'qmodem-preset-btn',
					'type': 'button',
					'click': function() {
						var input = document.getElementById('at_command_' + modem.id);
						if (input) {
							input.value = cmd.value;
							input.focus();
						}
					}
				}, [
					createSvg('<svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2"><polyline points="9 18 15 12 9 6"/></svg>'),
					E('span', {}, cmd.name)
				]);
				quickChips.appendChild(chip);
			});
			quickCard.appendChild(quickChips);
			wrap.appendChild(quickCard);

			// macOS / Soft Anime Cyber Terminal Frame
			var terminalFrame = E('div', { 'class': 'qmodem-terminal-frame' });

			// Terminal Window Topbar
			var termTopbar = E('div', { 'class': 'qmodem-terminal-topbar' });
			var dotsWrap = E('div', { 'class': 'qmodem-terminal-dots' }, [
				E('span', { 'class': 'qmodem-terminal-dot dot-pink' }),
				E('span', { 'class': 'qmodem-terminal-dot dot-yellow' }),
				E('span', { 'class': 'qmodem-terminal-dot dot-mint' }),
				E('span', { 'style': 'font-size: 12px; font-weight: 600; color: var(--qmd-text-2); margin-left: 8px;' },
					_('Serial Console') + ' — ' + modem.name)
			]);

			var termActions = E('div', { 'style': 'display: flex; gap: 8px;' });
			var copyBtn = E('button', {
				'class': 'qmodem-preset-btn',
				'type': 'button',
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
				createSvg('<svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>'),
				E('span', {}, _('Copy Output'))
			]);

			var clearBtn = E('button', {
				'class': 'qmodem-preset-btn',
				'type': 'button',
				'click': function() {
					var resp = document.getElementById('at_response_' + modem.id);
					if (resp) resp.value = '';
				}
			}, [
				createSvg('<svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>'),
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
				'placeholder': _('Ready. Type an AT command below or click a diagnostic preset.')
			});
			terminalFrame.appendChild(responseTextarea);

			// Terminal Prompt Input Row
			var promptRow = E('div', { 'class': 'qmodem-terminal-prompt-row' });
			promptRow.appendChild(E('span', { 'class': 'qmodem-terminal-prompt-label' }, 'AT>'));
			
			var cmdInput = E('input', {
				'type': 'text',
				'class': 'cbi-input-text',
				'id': 'at_command_' + modem.id,
				'style': 'flex: 1; font-family: monospace; font-size: 13.5px; height: 38px;',
				'placeholder': 'AT+CSQ'
			});

			var executeCommand = function() {
				var port = portSelect.value;
				var cmd = cmdInput.value.trim();
				var useUbus = ubusCheckbox.checked ? '1' : '0';

				if (!cmd) {
					ui.addNotification(null, E('p', _('Please enter an AT command')), 'warning');
					return;
				}

				sendBtn.disabled = true;
				sendBtn.classList.add('spinning');
				responseTextarea.value += (responseTextarea.value ? '\n\n' : '') + '>>> ' + cmd + '\n... sending ...';
				responseTextarea.scrollTop = responseTextarea.scrollHeight;

				qmodem.sendAt(modem.id, port, cmd, useUbus).then(function(result) {
					sendBtn.disabled = false;
					sendBtn.classList.remove('spinning');
					var text = '';
					if (result && result.at_cfg) {
						var statusStr = result.at_cfg.status === '1' ? 'OK' : 'ERROR';
						text = '[' + statusStr + ']\n' + (result.at_cfg.res || _('No response content'));
					} else {
						text = _('[No response received]');
					}
					// Replace sending line with response
					responseTextarea.value = responseTextarea.value.replace(/\n\.\.\. sending \.\.\.$/, '\n' + text);
					responseTextarea.scrollTop = responseTextarea.scrollHeight;
				}).catch(function(e) {
					sendBtn.disabled = false;
					sendBtn.classList.remove('spinning');
					responseTextarea.value = responseTextarea.value.replace(/\n\.\.\. sending \.\.\.$/, '\n[ERROR: ' + e.message + ']');
					responseTextarea.scrollTop = responseTextarea.scrollHeight;
				});
			};

			var sendBtn = E('button', {
				'class': 'cbi-button cbi-button-action',
				'type': 'button',
				'style': 'min-height: 38px; padding: 0 20px; font-weight: 600;',
				'click': executeCommand
			}, _('Send Command'));

			// Enter key trigger
			cmdInput.addEventListener('keydown', function(ev) {
				if (ev.key === 'Enter') {
					ev.preventDefault();
					executeCommand();
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
