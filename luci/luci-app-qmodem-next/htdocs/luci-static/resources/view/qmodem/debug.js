'use strict';
'require view';
'require ui';
'require dom';
'require qmodem.qmodem as qmodem';
'require qmodem.ui as qmUI';

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
		var title = E('h2', { 'class': 'cbi-map-caption' }, _('AT Debug'));
		container.appendChild(title);

	// Create modem selector section (shared component)
	var selector = qmUI.ModemSelector(modems, {
		leftAlignLabel: true,
		filter: function(modem) { return modem.enabled; }
	});
	var select = selector.select;
	container.appendChild(selector.section);

		// Create AT interface container
		var interfaceContainer = E('div', { 'id': 'at_interface_container' });
		container.appendChild(interfaceContainer);

		var self = this;
		
		// Update function to show selected modem's interface
		var updateInterface = function() {
			var selectedId = select.value;
			var selectedModem = modems.find(function(m) { return m.id === selectedId; });
			
			if (selectedModem) {
				dom.content(interfaceContainer, self.createAtInterface(selectedModem));
			}
		};

		// Selector change handler
		select.addEventListener('change', updateInterface);

		// Initial display
		updateInterface();

		return container;
	},

	createAtInterface: function(modem) {
		var self = this;
		var container = E('div', { 'class': 'cbi-section-node' });

		// AT Port Configuration
		var portSection = E('div', { 'class': 'cbi-value' });
		portSection.appendChild(E('label', { 'class': 'cbi-value-title' }, _('Available AT Ports')));
		var portInfo = E('div', { 'class': 'cbi-value-field' });
		portInfo.appendChild(E('div', { 'class': 'spinning' }, _('Loading...')));
		portSection.appendChild(portInfo);
		container.appendChild(portSection);

		// Current band display (best effort; unsupported vendors show a hint)
		var bandSection = E('div', { 'class': 'cbi-value' });
		bandSection.appendChild(E('label', { 'class': 'cbi-value-title' }, _('Current Band')));
		var bandField = E('div', { 'class': 'cbi-value-field' });
		bandField.appendChild(E('div', { 'class': 'spinning' }, _('Loading...')));
		bandSection.appendChild(bandField);
		container.appendChild(bandSection);

		qmodem.getCurrentBand(modem.id).then(function(res) {
			var band = res && res.current_band;

			if (!band || band.status === 'unsupported') {
				dom.content(bandField, _('Vendor does not support band query'));
				return;
			}

			if (band.status !== 'ok') {
				dom.content(bandField, _('Not registered'));
				return;
			}

			var names = (band.cells || []).map(function(cell) {
				var name = cell.band_name || ((cell.rat || '') + ' B' + (cell.band || ''));
				return cell.channel ? name + ' (' + cell.channel + ')' : name;
			});

			dom.content(bandField, names.length ? names.join(', ') : _('N/A'));
		}).catch(function() {
			dom.content(bandField, _('Vendor does not support band query'));
		});

		// Load AT configuration
		qmodem.getAtCfg(modem.id).then(function(cfg) {
			if (!cfg || !cfg.at_cfg) {
				dom.content(portInfo, _('Failed to load AT configuration'));
				return;
			}

			var info = [];
			info.push(E('div', {}, [
				E('strong', {}, _('Current Port') + ': '),
				E('span', {}, cfg.at_cfg.using_port || _('N/A'))
			]));

			if (cfg.at_cfg.ports && cfg.at_cfg.ports.length > 0) {
				info.push(E('div', {}, [
					E('strong', {}, _('Configured Ports') + ': '),
					E('span', {}, cfg.at_cfg.ports.join(', '))
				]));
			}

			if (cfg.at_cfg.other_ttys && cfg.at_cfg.other_ttys.length > 0) {
				info.push(E('div', {}, [
					E('strong', {}, _('Detected Ports') + ': '),
					E('span', {}, cfg.at_cfg.other_ttys.join(', '))
				]));
			}

			dom.content(portInfo, info);

			// Port selection dropdown
			var ports = [];
			if (cfg.at_cfg.ports) ports = ports.concat(cfg.at_cfg.ports);
			if (cfg.at_cfg.other_ttys) ports = ports.concat(cfg.at_cfg.other_ttys);

			var portSelectSection = E('div', { 'class': 'cbi-value' });
			portSelectSection.appendChild(E('label', { 'class': 'cbi-value-title' }, _('Select AT Port')));
			var portSelectField = E('div', { 'class': 'cbi-value-field' });
			
			var portDropdown = new ui.Dropdown(cfg.at_cfg.using_port || (ports.length > 0 ? ports[0] : ''),
				ports.reduce(function(obj, port) {
					obj[port] = port;
					return obj;
				}, {}), {
					id: 'at_port_' + modem.id,
					sort: false
				});
			/* 持有实例与渲染节点引用：取值走实例 API，回退走节点内查询，
			 * 不依赖 LuCI cbi-dropdown 内部渲染结构，也避免全局 getElementById 多实例串扰 */
			var portDropdownNode = portDropdown.render();

			var getSelectedPort = function() {
				if (portDropdown && typeof portDropdown.getValue === 'function')
					return portDropdown.getValue();
				var sel = portDropdownNode.querySelector('li[data-selected], li.selected');
				return sel ? (sel.getAttribute('data-value') || sel.textContent.trim()) : '';
			};

			portSelectField.appendChild(portDropdownNode);
			portSelectSection.appendChild(portSelectField);
			container.appendChild(portSelectSection);

			// Use Ubus flag option
			var ubusSection = E('div', { 'class': 'cbi-value' });
			ubusSection.appendChild(E('label', { 'class': 'cbi-value-title' }, _('Use Ubus AT Daemon')));
			var ubusField = E('div', { 'class': 'cbi-value-field' });
			
			var ubusCheckbox = E('input', {
				'type': 'checkbox',
				'id': 'use_ubus_' + modem.id,
				'checked': modem.use_ubus !== '0'
			});
			
			ubusField.appendChild(ubusCheckbox);
			ubusField.appendChild(document.createTextNode(' '));
			ubusField.appendChild(E('span', {}, _('Enable to use Ubus AT daemon instead of direct serial port access')));
			ubusSection.appendChild(ubusField);
			container.appendChild(ubusSection);

			// AT Command input
			var cmdSection = E('div', { 'class': 'cbi-value' });
			cmdSection.appendChild(E('label', { 'class': 'cbi-value-title' }, _('AT Command')));
			var cmdField = E('div', { 'class': 'cbi-value-field' });
			var cmdInput = E('input', {
				'type': 'text',
				'class': 'cbi-input-text',
				'id': 'at_command_' + modem.id,
				'placeholder': 'AT+CIMI'
			});
			cmdField.appendChild(cmdInput);
			cmdSection.appendChild(cmdField);
			container.appendChild(cmdSection);

			// Response area
			var responseSection = E('div', { 'class': 'cbi-value' });
			responseSection.appendChild(E('label', { 'class': 'cbi-value-title' }, _('Response')));
			var responseField = E('div', { 'class': 'cbi-value-field' });
			
			var responseDiv = E('textarea', {
				'id': 'at_response_' + modem.id,
				'style': 'padding: 10px; overflow-y: auto; font-family: monospace; white-space: pre-wrap; width: 80%;',
				'rows': 20,
				'readonly': 'readonly',
			}, _('Click "Send AT Command" to execute'));

			var sendBtn = E('button', {
				'class': 'btn cbi-button-action',
				'click': function() {
					var port = getSelectedPort();
					var cmd = document.getElementById('at_command_' + modem.id).value.trim();
					var useUbus = document.getElementById('use_ubus_' + modem.id).checked ? '1' : '0';

					if (!cmd) {
						responseDiv.textContent = _('Error: Please enter AT command');
						return;
					}

					responseDiv.textContent = _('Sending command...');

					// Use the use_ubus flag when sending AT command
					qmodem.sendAt(modem.id, port, cmd, useUbus).then(function(result) {
						if (result && result.at_cfg) {
							var text = '';
							text += _('Status: ') + (result.at_cfg.status === '1' ? _('Success') : _('Failed')) + '\n';
							text += _('Command: ') + (result.at_cfg.cmd || '') + '\n';
							text += _('Response:\n') + (result.at_cfg.res || _('No response'));
							responseDiv.textContent = text;
						} else {
							responseDiv.textContent = _('No response received');
						}
					}).catch(function(e) {
						responseDiv.textContent = _('Error: %s').format(e.message);
					});
				}
			}, _('Send AT Command'));

			responseField.appendChild(sendBtn);
			responseField.appendChild(E('br'));
			responseField.appendChild(E('br'));
			responseField.appendChild(responseDiv);
			responseSection.appendChild(responseField);
			container.appendChild(responseSection);

			// Quick commands
			if (cfg.at_cfg.cmds && cfg.at_cfg.cmds.length > 0) {
				var quickSection = E('div', { 'class': 'cbi-value' });
				quickSection.appendChild(E('label', { 'class': 'cbi-value-title' }, _('Quick Commands')));
				var quickField = E('div', { 'class': 'cbi-value-field cbi-section-actions' });
				
				cfg.at_cfg.cmds.forEach(function(cmd) {
					if (cmd.name && cmd.value) {
						quickField.appendChild(E('button', {
							'class': 'btn cbi-button-action',
							'click': function() {
								document.getElementById('at_command_' + modem.id).value = cmd.value;
							}
						}, cmd.name));
						quickField.appendChild(document.createTextNode(' '));
					}
				});
				
				quickSection.appendChild(quickField);
				container.appendChild(quickSection);
			}
		}).catch(function(e) {
			dom.content(portInfo, _('Error loading AT configuration: %s').format(e.message));
		});

		return container;
	},

	handleSaveApply: null,
	handleSave: null,
	handleReset: null
});
