'use strict';
'require view';
'require poll';
'require ui';
'require dom';
'require uci';
'require qmodem.qmodem as qmodem';
'require qmodem.ui as qmUI';

// Load CSS (deduplicated)
qmodem.loadCss(L.resource('qmodem/qmodem-next.css'));

// LuciTable rendering and progressbar class detection live in qmodem/ui.js
// (shared with view/status/include/11_modem.js).

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
				var name = section.name ? section.name.toUpperCase() : _('Unknown');
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

	// Get section order from localStorage
	getSectionOrder: function(modemId) {
		var key = 'qmodem_section_order_' + modemId;
		var order = localStorage.getItem(key);
		return order ? JSON.parse(order) : [];
	},

	// Save section order to localStorage
	saveSectionOrder: function(modemId, order) {
		var key = 'qmodem_section_order_' + modemId;
		localStorage.setItem(key, JSON.stringify(order));
	},

	// Get collapsed state from localStorage
	getCollapsedState: function(modemId, className) {
		var key = 'qmodem_collapsed_' + modemId + '_' + className;
		return localStorage.getItem(key) === 'true';
	},

	// Save collapsed state to localStorage
	saveCollapsedState: function(modemId, className, isCollapsed) {
		var key = 'qmodem_collapsed_' + modemId + '_' + className;
		localStorage.setItem(key, isCollapsed ? 'true' : 'false');
	},

	// Attach collapse and drag handlers to a section
	attachSectionHandlers: function(fieldset, className, modemId, infoContainer) {
		var self = this;
		
		// Collapse/expand handler
		var legend = fieldset.querySelector('legend');
		var title = fieldset.querySelector('.panel-title');
		
		var toggleCollapse = function(e) {
			// Don't toggle if dragging
			if (fieldset.classList.contains('dragging')) {
				return;
			}
			
			fieldset.classList.toggle('collapsed');
			var isCollapsed = fieldset.classList.contains('collapsed');
			self.saveCollapsedState(modemId, className, isCollapsed);
		};
		
		if (legend) legend.addEventListener('click', toggleCollapse);
		if (title) title.addEventListener('click', toggleCollapse);
		
		// Drag and drop handlers
		fieldset.addEventListener('dragstart', function(e) {
			fieldset.classList.add('dragging');
			e.dataTransfer.effectAllowed = 'move';
			e.dataTransfer.setData('text/plain', className);
		});
		
		fieldset.addEventListener('dragend', function(e) {
			fieldset.classList.remove('dragging');
			// Remove all drag-over classes
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
			if (draggedClassName === className) {
				return;
			}
			
			// Get all sections in current order
			var sections = Array.from(infoContainer.querySelectorAll('.cbi-section.draggable'));
			var draggedSection = infoContainer.querySelector('.dragging');
			
			// Reorder in DOM
			if (draggedSection) {
				infoContainer.insertBefore(draggedSection, fieldset);
			}
			
			// Save new order
			var newOrder = Array.from(infoContainer.querySelectorAll('.cbi-section.draggable')).map(function(section) {
				// Use the stable data-section-key attribute (language-independent)
				return section.getAttribute('data-section-key') || '';
			}).filter(function(name) {
				return name !== '';
			});
			
			self.saveSectionOrder(modemId, newOrder);
		});
	},

	updateModemInfo: function(modemId, tables_map, infoContainer, updateTimeElement, copyrightElement) {
		var self = this;
		
		// Show loading state
		if (Object.keys(tables_map).length === 0) {
			dom.content(infoContainer, E('div', { 'class': 'spinning' }, _('Loading modem information...')));
		}
		
		// Fetch all modem info in parallel; a single failed source must not
		// blank the whole page, so use allSettled with per-item placeholders
		Promise.allSettled([
			qmodem.getBaseInfo(modemId),
			qmodem.getSimInfo(modemId),
			qmodem.getNetworkInfo(modemId),
			qmodem.getCellInfo(modemId),
			qmodem.getCopyright(modemId)
		]).then(function(results) {
			// Merge fulfilled modem_info arrays (exclude copyright)
			var all_info = [];
			var failed = 0;
			for (var i = 0; i < results.length - 1; i++) {
				if (results[i].status === 'fulfilled' && results[i].value && results[i].value.modem_info) {
					all_info = all_info.concat(results[i].value.modem_info);
				} else {
					failed++;
				}
			}

			// All info sources failed: fall back to the full-page error
			if (failed === results.length - 1) {
				var firstError = results.find(function(r) { return r.status === 'rejected'; });
				var message = firstError && firstError.reason ? firstError.reason.message : 'unknown error';
				console.error('Error fetching modem info:', message);
				dom.content(infoContainer, E('div', { 'class': 'alert-message warning' },
					_('Error loading modem information: %s').format(message)));
				return;
			}

			// Partial failure: keep the successful sections and add a placeholder row
			if (failed > 0) {
				all_info.push({
					type: 'plain_text',
					'class': 'General',
					full_name: _('Some data sources failed to load'),
					value: _('Will retry on next refresh')
				});
			}
			
			// Handle copyright (last result); failure just hides the section
			var copyrightResult = results[results.length - 1];
			var copyrightData = copyrightResult.status === 'fulfilled' ? copyrightResult.value : null;
			if (copyrightElement && copyrightData && copyrightData.copyright) {
				// Format copyright object into display string
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
				if (entry.type === 'warning_message') {
					// Handle warning messages
					return;
				}
				
				var className = entry['class'] || 'General';
				if (!grouped[className]) {
					grouped[className] = [];
				}
				grouped[className].push(entry);
			});

			// Clear loading animation if present
			var loadingDiv = infoContainer.querySelector('.spinning');
			if (loadingDiv) {
				infoContainer.removeChild(loadingDiv);
			}

			// Remove obsolete tables from DOM and map
			for (var existingClass in tables_map) {
				if (!grouped[existingClass]) {
					if (tables_map[existingClass].fieldset.parentNode) {
						infoContainer.removeChild(tables_map[existingClass].fieldset);
					}
					delete tables_map[existingClass];
				}
			}

			// Get section order from localStorage
			var sectionOrder = self.getSectionOrder(modemId);
			var orderedClasses = [];
			
			// First add classes in saved order
			sectionOrder.forEach(function(className) {
				if (grouped[className]) {
					orderedClasses.push(className);
				}
			});
			
			// Then add any new classes not in saved order
			for (var className in grouped) {
				if (orderedClasses.indexOf(className) === -1) {
					orderedClasses.push(className);
				}
			}
			
			// Update or create tables in order
			orderedClasses.forEach(function(className) {
				if (!tables_map[className]) {
					tables_map[className] = new qmUI.InfoTable();
					// Store the stable (untranslated) class name for persistence
					tables_map[className].fieldset.setAttribute('data-section-key', className);
					infoContainer.appendChild(tables_map[className].fieldset);
					self.attachSectionHandlers(tables_map[className].fieldset, className, modemId, infoContainer);
				}
				tables_map[className].setTitle(className);
				tables_map[className].setData(grouped[className]);
				
				// Restore collapsed state
				var collapsedState = self.getCollapsedState(modemId, className);
				if (collapsedState) {
					tables_map[className].fieldset.classList.add('collapsed');
				} else {
					tables_map[className].fieldset.classList.remove('collapsed');
				}
			});
			
			// Update refresh time
			if (updateTimeElement) {
				updateTimeElement.textContent = _('Last update') + ': ' + qmUI.formatTime(new Date());
			}
		}).catch(function(e) {
			console.error('Error fetching modem info:', e);
			dom.content(infoContainer, E('div', { 'class': 'alert-message warning' },
				_('Error loading modem information: %s').format(e.message)));
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
		
		// Create modem selector section (shared component)
		var selector = qmUI.ModemSelector(modems);
		var select = selector.select;
		container.appendChild(selector.section);
		
		// Create copyright display
		var copyrightDiv = E('div', { 
			'class': 'copyright-section',
			'style': 'display: none;'
		});
		container.appendChild(copyrightDiv);
		
		// Create update time display
		var updateTimeSection = E('fieldset', { 'class': 'cbi-section' });
		var updateTimeDiv = E('div', {
			'class': 'qmodem-update-time',
			'id': 'update_time_display'
		}, _('Last update') + ': -');
		updateTimeSection.appendChild(updateTimeDiv);
		container.appendChild(updateTimeSection);
		
		// Create info container
		var infoContainer = E('div', { 'id': 'modem_info_container' });
		container.appendChild(infoContainer);
		
		// Tables map to store LuciTable instances
		var tables_map = {};
		
		// Update function
		var updateInfo = function(clearTables) {
			var selectedModem = select.value;
			
			// Clear tables when switching modem
			if (clearTables) {
				for (var className in tables_map) {
					if (tables_map[className].fieldset.parentNode) {
						infoContainer.removeChild(tables_map[className].fieldset);
					}
				}
				tables_map = {};
			}
			
			self.updateModemInfo(selectedModem, tables_map, infoContainer, updateTimeDiv, copyrightDiv);
		};
		
		// Selector change handler
		select.addEventListener('change', function() {
			updateInfo(true); // Clear tables when switching
		});
		
		// Initial update
		updateInfo(false);
		
		// Start polling (every 10 seconds)
		poll.add(function() {
			var selectedModem = select.value;
			self.updateModemInfo(selectedModem, tables_map, infoContainer, updateTimeDiv, copyrightDiv);
		}, 10);
		
		return container;
	},

	handleSaveApply: null,
	handleSave: null,
	handleReset: null
});
