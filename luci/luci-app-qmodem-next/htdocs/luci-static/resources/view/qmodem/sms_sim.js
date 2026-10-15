'use strict';
'require view';
'require poll';
'require ui';
'require dom';
'require qmodem.sms as smsService';
'require qmodem.qmodem as qmodem';

// Load CSS
document.head.appendChild(E('link', {
	'rel': 'stylesheet',
	'type': 'text/css',
	'href': L.resource('qmodem/qmodem-next.css') + '?v=' + new Date().toISOString().slice(0, 10)
}));

return view.extend({
	load: function() {
		return qmodem.getModemSections();
	},

	render: function(modems) {
		var self = this;
		var container = E('div', { 'class': 'cbi-map' });

		// Filter enabled modems
		modems = modems.filter(function(m) { return m.enabled; });

		if (modems.length === 0) {
			container.appendChild(E('div', { 'class': 'alert-message warning' }, 
				_('No modems configured or all modems are disabled.')));
			return container;
		}

		// Soft Anime Hero
		var hero = E('div', { 'class': 'qmodem-hero' }, [
			E('h1', { 'class': 'qmodem-hero-title' }, [
				_('SIM Card SMS Storage'),
				E('span', { 'class': 'qmodem-chip qmodem-chip-info' }, _('Direct SIM Access'))
			]),
			E('p', { 'class': 'qmodem-hero-sub' },
				_('Inspect raw SMS records stored directly on SIM memory, view message indices, and perform cleanup.'))
		]);
		var heroIcon = E('div', { 'class': 'qmodem-mascot qmd-anim-float', 'aria-hidden': 'true' });
		heroIcon.innerHTML =
			'<svg viewBox="0 0 76 60" width="76" height="60" fill="none">' +
			'<path d="M18 10 L48 10 L58 20 L58 50 L18 50 Z" fill="#FFFFFF" stroke="#8875E8" stroke-width="2.5" stroke-linejoin="round"/>' +
			'<rect x="26" y="22" width="24" height="20" rx="3" stroke="#EFA8CB" stroke-width="2"/>' +
			'<line x1="38" y1="22" x2="38" y2="42" stroke="#EFA8CB" stroke-width="2"/>' +
			'<line x1="26" y1="32" x2="50" y2="32" stroke="#EFA8CB" stroke-width="2"/>' +
			'<circle cx="58" cy="18" r="3.5" fill="#36AD87" class="qmd-anim-pulse"/>' +
			'</svg>';
		hero.appendChild(heroIcon);
		container.appendChild(hero);

		// Modern Toolbar Card
		var selectorSection = E('div', {
			'class': 'cbi-section',
			'style': 'display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 12px; margin-bottom: 16px; padding: 12px 18px;'
		});
		var leftBar = E('div', { 'style': 'display: flex; align-items: center; gap: 10px; flex-wrap: wrap;' });
		leftBar.appendChild(E('span', { 'class': 'qmodem-chip qmodem-chip-muted', 'style': 'font-weight: 600;' }, _('Active Modem')));
		var select = E('select', {
			'class': 'cbi-input-select',
			'id': 'sms_sim_modem_selector',
			'style': 'min-width: 220px;'
		});
		modems.forEach(function(modem) {
			select.appendChild(E('option', { 'value': modem.id }, modem.name));
		});
		leftBar.appendChild(select);
		selectorSection.appendChild(leftBar);
		container.appendChild(selectorSection);

		// Main content area
		var contentArea = E('div', { 'id': 'sms-sim-content' });
		container.appendChild(contentArea);

		// Loading indicator
		var loadingDiv = E('div', { 'class': 'spinning' }, _('Loading...'));
		contentArea.appendChild(loadingDiv);

		// Load SIM SMS list with selected modem
		var selectedModem = select.value;
		this.loadSimSmsList(contentArea, loadingDiv, selectedModem);

		// Selector change handler
		select.addEventListener('change', function() {
			var loading = E('div', { 'class': 'spinning' }, _('Loading...'));
			dom.content(contentArea, loading);
			self.loadSimSmsList(contentArea, loading, select.value);
		});

		// Start polling for updates
		poll.add(function() {
			self.pollSimSmsList(contentArea, select.value);
		}, 10);

		return container;
	},

	loadSimSmsList: function(contentArea, loadingDiv, configSection) {
		var self = this;

		smsService.getSimSms(configSection).then(function(result) {
			// Remove loading indicator
			if (loadingDiv && loadingDiv.parentNode) {
				dom.content(contentArea, null);
			}

			if (result.error) {
				contentArea.appendChild(E('div', { 'class': 'alert-message error' }, 
					_('Error loading SIM SMS: ') + result.error));
				return;
			}

			// Render SIM SMS list
			self.renderSimSmsList(contentArea, result.messages || [], configSection);
		}).catch(function(err) {
			if (loadingDiv && loadingDiv.parentNode) {
				dom.content(contentArea, null);
			}
			contentArea.appendChild(E('div', { 'class': 'alert-message error' }, 
				_('Failed to load SIM SMS: ') + err.message));
		});
	},

	pollSimSmsList: function(contentArea, configSection) {
		var self = this;
		
		smsService.getSimSms(configSection).then(function(result) {
			if (!result.error && result.messages) {
				self.renderSimSmsList(contentArea, result.messages, configSection);
			}
		}).catch(function(err) {
			console.error('Poll error:', err);
		});
	},

	renderSimSmsList: function(contentArea, messages, configSection) {
		var self = this;

		// Clear content
		dom.content(contentArea, null);

		// Create messages container
		var messagesDiv = E('div', { 'class': 'sms-sim-messages' });

		if (!messages || messages.length === 0) {
			messagesDiv.appendChild(E('div', { 'class': 'alert-message info' }, 
				_('No messages found on SIM card')));
		} else {
			// Create messages list
			var listDiv = E('div', { 'class': 'cbi-section' });
			var listFieldset = E('fieldset', { 'class': 'cbi-section' });
			var listLegend = E('legend', {}, _('SMS Messages on SIM Card'));
		
			listFieldset.appendChild(listLegend);

			// Create table
			var table = E('table', { 'class': 'table cbi-section-table' });
			var thead = E('thead', {}, [
				E('tr', { 'class': 'tr cbi-section-table-titles' }, [
					E('th', { 'class': 'th', 'style': 'width: 5%' }, _('Index')),
					E('th', { 'class': 'th', 'style': 'width: 15%' }, _('Sender')),
					E('th', { 'class': 'th', 'style': 'width: 15%' }, _('Time')),
					E('th', { 'class': 'th', 'style': 'width: 50%' }, _('Content')),
					E('th', { 'class': 'th', 'style': 'width: 10%' }, _('Type')),
					E('th', { 'class': 'th', 'style': 'width: 5%' }, _('Actions'))
				])
			]);
			table.appendChild(thead);

			var tbody = E('tbody', {});

			// Add message rows
			for (var i = 0; i < messages.length; i++) {
				var msg = messages[i];
				
				var index = msg.index != null ? String(msg.index) : '-';
				var sender = smsService.formatPhoneNumber(msg.sender || '-');
				var time = msg.timestamp ? smsService.formatTimestamp(msg.timestamp) : '-';
				var content = smsService.truncateMessage(msg.content || msg.sms_text || '', 80);
				
				// Determine message type
				var type = '';
				if (msg.reference && msg.total > 1) {
					type = _('Part') + ' ' + (msg.part || '?') + '/' + (msg.total || '?');
				} else {
					type = _('Single');
				}
				
				var deleteBtn = E('button', {
					'class': 'cbi-button cbi-button-remove',
					'click': L.bind(function(idx) {
						self.handleDeleteSimSms(configSection, idx, contentArea);
					}, this, msg.index)
				}, _('Delete'));

				var row = E('tr', { 
					'class': 'tr cbi-section-table-row'
				}, [
					E('td', { 'class': 'td' }, index),
					E('td', { 'class': 'td' }, sender),
					E('td', { 'class': 'td' }, time),
					E('td', { 'class': 'td', 'style': 'word-break: break-word;' }, content),
					E('td', { 'class': 'td' }, type),
					E('td', { 'class': 'td' }, deleteBtn)
				]);

				tbody.appendChild(row);
			}

			table.appendChild(tbody);
			listFieldset.appendChild(table);
			listDiv.appendChild(listFieldset);
			messagesDiv.appendChild(listDiv);
		}

		// Add summary info
		var summaryDiv = E('div', { 'class': 'cbi-section' });
		var summaryFieldset = E('fieldset', { 'class': 'cbi-section' });
		summaryFieldset.appendChild(E('legend', {}, _('Summary')));
		
		var summaryTable = E('table', { 'class': 'table' });
		var summaryBody = E('tbody', {}, [
			E('tr', { 'class': 'tr' }, [
				E('td', { 'class': 'td', 'width': '33%' }, _('Total Messages')),
				E('td', { 'class': 'td' }, String(messages.length))
			])
		]);
		
		summaryTable.appendChild(summaryBody);
		summaryFieldset.appendChild(summaryTable);
		summaryDiv.appendChild(summaryFieldset);
		messagesDiv.appendChild(summaryDiv);

		// Add action buttons
		var buttonDiv = E('div', { 'class': 'cbi-page-actions' });
		
		var refreshBtn = E('button', {
			'class': 'cbi-button cbi-button-action',
			'click': L.bind(function(ev) {
				ev.preventDefault();
				var loading = E('div', { 'class': 'spinning' }, _('Loading...'));
				dom.content(contentArea, loading);
				this.loadSimSmsList(contentArea, loading, configSection);
			}, this)
		}, _('Refresh'));
		
		var deleteAllBtn = E('button', {
			'class': 'cbi-button cbi-button-remove',
			'click': L.bind(function(ev) {
				ev.preventDefault();
				this.handleDeleteAllSimSms(configSection, messages, contentArea);
			}, this)
		}, _('Delete All'));
		
		buttonDiv.appendChild(refreshBtn);
		if (messages.length > 0) {
			buttonDiv.appendChild(deleteAllBtn);
		}
		messagesDiv.appendChild(buttonDiv);

		contentArea.appendChild(messagesDiv);
	},

	handleDeleteSimSms: function(configSection, index, contentArea) {
		var self = this;
		
		if (index == null) {
			ui.addNotification(null, E('p', _('Invalid message index')), 'error');
			return;
		}

		ui.showModal(_('Confirm Delete'), [
			E('p', _('Are you sure you want to delete this message from SIM card?')),
			E('div', { 'class': 'right' }, [
				E('button', {
					'class': 'cbi-button cbi-button-neutral',
					'click': function() {
						ui.hideModal();
					}
				}, _('Cancel')),
				' ',
				E('button', {
					'class': 'cbi-button cbi-button-negative',
					'click': function() {
						ui.hideModal();
						ui.showModal(_('Deleting...'), E('div', { 'class': 'spinning' }, _('Deleting message...')));
						
						smsService.deleteSimSms(configSection, index).then(function(result) {
							ui.hideModal();
							
							if (result.error) {
								ui.addNotification(null, E('p', _('Failed to delete SMS: ') + result.error), 'error');
								return;
							}
							
							if (result.success) {
								ui.addNotification(null, E('p', _('Message deleted successfully')), 'info');
								// Reload the list
								var loading = E('div', { 'class': 'spinning' }, _('Loading...'));
								dom.content(contentArea, loading);
								self.loadSimSmsList(contentArea, loading, configSection);
							} else {
								ui.addNotification(null, E('p', _('Failed to delete message')), 'error');
							}
						}).catch(function(err) {
							ui.hideModal();
							ui.addNotification(null, E('p', _('Error deleting SMS: ') + err.message), 'error');
						});
					}
				}, _('Delete'))
			])
		]);
	},

	handleDeleteAllSimSms: function(configSection, messages, contentArea) {
		var self = this;
		
		if (!messages || messages.length === 0) {
			ui.addNotification(null, E('p', _('No messages to delete')), 'info');
			return;
		}

		ui.showModal(_('Confirm Delete All'), [
			E('p', _('Are you sure you want to delete ALL %d messages from SIM card? This action cannot be undone.').format(messages.length)),
			E('div', { 'class': 'right' }, [
				E('button', {
					'class': 'cbi-button cbi-button-neutral',
					'click': function() {
						ui.hideModal();
					}
				}, _('Cancel')),
				' ',
				E('button', {
					'class': 'cbi-button cbi-button-negative',
					'click': function() {
						ui.hideModal();
						ui.showModal(_('Deleting...'), E('div', { 'class': 'spinning' }, 
							_('Deleting all messages... This may take a while.')));
						
						// Delete messages one by one
						var deletePromises = [];
						for (var i = 0; i < messages.length; i++) {
							if (messages[i].index != null) {
								deletePromises.push(smsService.deleteSimSms(configSection, messages[i].index));
							}
						}
						
						Promise.all(deletePromises).then(function(results) {
							ui.hideModal();
							
							var successCount = 0;
							var failCount = 0;
							
							for (var i = 0; i < results.length; i++) {
								if (results[i].success) {
									successCount++;
								} else {
									failCount++;
								}
							}
							
							if (failCount === 0) {
								ui.addNotification(null, E('p', _('All %d messages deleted successfully').format(successCount)), 'info');
							} else {
								ui.addNotification(null, E('p', _('%d messages deleted, %d failed').format(successCount, failCount)), 'warning');
							}
							
							// Reload the list
							var loading = E('div', { 'class': 'spinning' }, _('Loading...'));
							dom.content(contentArea, loading);
							self.loadSimSmsList(contentArea, loading, configSection);
						}).catch(function(err) {
							ui.hideModal();
							ui.addNotification(null, E('p', _('Error deleting messages: ') + err.message), 'error');
						});
					}
				}, _('Delete All'))
			])
		]);
	},

	handleSaveApply: null,
	handleSave: null,
	handleReset: null
});
