'use strict';
'require view';
'require poll';
'require ui';
'require dom';
'require qmodem.qmodem as qmodem';
'require qmodem.sms as smsService';
'require qmodem.ui as qmUI';

// Load CSS (deduplicated)
qmodem.loadCss(L.resource('qmodem/qmodem-next.css'));

return view.extend({
	load: function() {
		// Get contact from URL hash or path
		var hash = window.location.hash || window.location.pathname;
		var match = hash.match(/\/conversation\/([^\/\?#]+)/);
		var contact = match ? decodeURIComponent(match[1]) : null;

		return Promise.all([
			smsService.getModems(),
			Promise.resolve(contact)
		]);
	},

	render: function(loadResult) {
		var self = this;
		var modems = loadResult[0];
		var contact = loadResult[1];

		if (!contact) {
			return E('div', { 'class': 'cbi-map' }, [
				E('h2', {}, _('SMS Conversation')),
				E('div', { 'class': 'alert-message error' }, 
					_('Invalid contact'))
			]);
		}

		// Filter enabled modems (defensive: backend may return no list)
		modems = modems || [];
		modems = modems.filter(function(m) { return m.enabled; });

		if (modems.length === 0) {
			return E('div', { 'class': 'alert-message warning' }, 
				_('No modems configured or all modems are disabled.'));
		}

		var container = E('div', { 'class': 'cbi-map' });

		// Modem selector (shared component)
		var selector = qmUI.ModemSelector(modems, { id: 'sms_modem_selector' });
		var select = selector.select;
		container.appendChild(selector.section);

		// Header with back button
		var headerDiv = E('div', { 'class': 'sms-conversation-header' }, [
			E('button', {
				'class': 'cbi-button cbi-button-neutral',
				'click': function(ev) {
					ev.preventDefault();
					window.location.href = L.url('admin/modem/qmodem/sms');
				}
			}, '← ' + _('Back')),
			E('h2', {},
				_('Conversation with ') + smsService.formatPhoneNumber(contact))
		]);
		container.appendChild(headerDiv);

		// Messages area
		var messagesArea = E('div', {
			'id': 'sms-messages-area',
			'class': 'sms-messages-area'
		});
		container.appendChild(messagesArea);

		// Loading indicator
		var loadingDiv = E('div', { 'class': 'spinning' }, _('Loading messages...'));
		messagesArea.appendChild(loadingDiv);

		// Get selected modem
		var selectedModem = select.value;

		// Reply form
		var replyForm = this.createReplyForm(contact, selectedModem, messagesArea, loadingDiv, select);
		container.appendChild(replyForm);

		// Load conversation
		this.loadConversation(messagesArea, loadingDiv, selectedModem, contact);

		// Selector change handler
		select.addEventListener('change', function() {
			var loading = E('div', { 'class': 'spinning' }, _('Loading...'));
			dom.content(messagesArea, loading);
			self.loadConversation(messagesArea, loading, select.value, contact);
		});

		// Start polling for new messages
		poll.add(function() {
			self.pollConversation(messagesArea, select.value, contact);
		}, 5);

		return container;
	},

	createReplyForm: function(contact, configSection, messagesArea, loadingDiv, select) {
		var self = this;

		var formDiv = E('div', {
			'class': 'cbi-section sms-reply-form'
		});

		var fieldset = E('fieldset', { 'class': 'cbi-section' });
		var legend = E('legend', {}, _('Send Message'));
		fieldset.appendChild(legend);

		// Message input
		var inputDiv = E('div', { 'class': 'cbi-value' });
		var inputLabel = E('label', { 'class': 'cbi-value-title' }, _('Message'));
		var inputField = E('div', { 'class': 'cbi-value-field' });
		var textarea = E('textarea', {
			'id': 'sms-reply-text',
			'class': 'cbi-input-textarea',
			'rows': '3',
			'placeholder': _('Enter your message here...')
		});
		inputField.appendChild(textarea);
		inputDiv.appendChild(inputLabel);
		inputDiv.appendChild(inputField);
		fieldset.appendChild(inputDiv);

		// Encoding selection
		var encodingDiv = E('div', { 'class': 'cbi-value' });
		var encodingLabel = E('label', { 'class': 'cbi-value-title' }, _('Encoding'));
		var encodingField = E('div', { 'class': 'cbi-value-field' });
		var encodingSelect = E('select', {
			'id': 'sms-encoding',
			'class': 'cbi-input-select'
		}, [
			E('option', { 'value': '7bit' }, _('7-bit (ASCII)')),
			E('option', { 'value': '16bit', 'selected': 'selected' }, _('16-bit (Unicode)'))
		]);
		encodingField.appendChild(encodingSelect);
		encodingDiv.appendChild(encodingLabel);
		encodingDiv.appendChild(encodingField);
		fieldset.appendChild(encodingDiv);

		// Character counter
		var counterDiv = E('div', {
			'class': 'cbi-value sms-counter-row'
		});
		var counter = E('span', { 'id': 'sms-char-counter' }, '0/160');
		counterDiv.appendChild(E('label', { 'class': 'cbi-value-title' }, _('Length')));
		counterDiv.appendChild(E('div', { 'class': 'cbi-value-field' }, counter));
		fieldset.appendChild(counterDiv);

		// Update character counter
		textarea.addEventListener('input', function() {
			var text = textarea.value;
			var encoding = encodingSelect.value;
			var maxLength = encoding === '7bit' ? 160 : 70;
			var parts = Math.ceil(text.length / maxLength);
			
			if (parts > 1) {
				maxLength = encoding === '7bit' ? 153 : 66;
				parts = Math.ceil(text.length / maxLength);
			}
			
			counter.textContent = text.length + ' / ' + maxLength + 
			                      (parts > 1 ? ' (' + parts + ' parts)' : '');
			
			if (text.length > maxLength * 5) {
				counter.style.color = 'red';
			} else if (parts > 1) {
				counter.style.color = 'orange';
			} else {
				counter.style.color = '#666';
			}
		});

		// Buttons
		var buttonsDiv = E('div', { 'class': 'cbi-page-actions' });
		var sendBtn = E('button', {
			'class': 'cbi-button cbi-button-save',
			'click': function(ev) {
				ev.preventDefault();
				var currentModem = select ? select.value : configSection;
				self.sendMessage(textarea, encodingSelect, contact, currentModem, 
				                messagesArea, loadingDiv, sendBtn, select);
			}
		}, _('Send'));
		
		var clearBtn = E('button', {
			'class': 'cbi-button cbi-button-reset',
			'click': function(ev) {
				ev.preventDefault();
				textarea.value = '';
				textarea.dispatchEvent(new Event('input'));
			}
		}, _('Clear'));

		buttonsDiv.appendChild(sendBtn);
		buttonsDiv.appendChild(clearBtn);
		fieldset.appendChild(buttonsDiv);

		formDiv.appendChild(fieldset);
		return formDiv;
	},

	sendMessage: function(textarea, encodingSelect, contact, configSection, 
	                      messagesArea, loadingDiv, sendBtn, select) {
		var self = this;
		var message = textarea.value.trim();
		var encoding = encodingSelect.value;

		if (!message) {
			ui.addNotification(null, E('p', _('Please enter a message')), 'error');
			return;
		}

		// Disable send button
		sendBtn.disabled = true;
		sendBtn.textContent = _('Sending...');

		smsService.sendSms(configSection, contact, message, encoding).then(function(result) {
			sendBtn.disabled = false;
			sendBtn.textContent = _('Send');

			if (result.error) {
				ui.addNotification(null, E('p', _('Failed to send SMS: ') + result.error), 'error');
				return;
			}

			if (result.success) {
				ui.addNotification(null, E('p', _('Message sent successfully')), 'info');
				textarea.value = '';
				textarea.dispatchEvent(new Event('input'));
				
				// Reload conversation with current modem
				var currentModem = select ? select.value : configSection;
				self.loadConversation(messagesArea, loadingDiv, currentModem, contact);
			}
		}).catch(function(err) {
			sendBtn.disabled = false;
			sendBtn.textContent = _('Send');
			ui.addNotification(null, E('p', _('Failed to send SMS: ') + err.message), 'error');
		});
	},

	loadConversation: function(messagesArea, loadingDiv, configSection, contact) {
		var self = this;

		smsService.getConversation(configSection, contact).then(function(result) {
			// Auto-mark all received messages as read
			if (result.messages && result.messages.length > 0) {
				var unreadIds = [];
				result.messages.forEach(function(msg) {
					if (msg.type === 'received' && msg.is_read === false) {
						if (msg.part_ids && msg.part_ids.length > 0) {
							// For multipart messages, mark all parts as read
							unreadIds = unreadIds.concat(msg.part_ids);
						} else {
							unreadIds.push(msg.id);
						}
					}
				});
				if (unreadIds.length > 0) {
					smsService.markRead(configSection, unreadIds);
				}
			}

			// Remove loading indicator
			if (loadingDiv && loadingDiv.parentNode) {
				dom.content(messagesArea, null);
			}

			if (result.error) {
				messagesArea.appendChild(E('div', { 'class': 'alert-message error' },
					_('Error loading messages: ') + result.error));
				return;
			}

			// Render messages
			self.renderMessages(messagesArea, result.messages || [], configSection);
		}).catch(function(err) {
			if (loadingDiv && loadingDiv.parentNode) {
				dom.content(messagesArea, null);
			}
			messagesArea.appendChild(E('div', { 'class': 'alert-message error' }, 
				_('Failed to load messages: ') + err.message));
		});
	},

	pollConversation: function(messagesArea, configSection, contact) {
		var self = this;

		smsService.getConversation(configSection, contact).then(function(result) {
			if (!result.error && result.messages) {
				self.renderMessages(messagesArea, result.messages, configSection);
			}
		}).catch(qmUI.pollGuard('sms_conversation'));
	},

	renderMessages: function(messagesArea, messages, configSection) {
		var self = this;

		// Clear content
		dom.content(messagesArea, null);

		if (!messages || messages.length === 0) {
			messagesArea.appendChild(E('div', { 'class': 'alert-message info' }, 
				_('No messages in this conversation')));
			return;
		}

		// Create messages list
		var messagesDiv = E('div', { 'class': 'sms-messages-list' });

		for (var i = 0; i < messages.length; i++) {
			var msg = messages[i];
			var isReceived = msg.type === 'received';
			var isSent = msg.type === 'sent';

			// Message bubble
			var bubbleClass = 'sms-message-bubble ' + 
			                  (isReceived ? 'sms-message-received' : 'sms-message-sent');
			
			var bubble = E('div', { 'class': bubbleClass });

			// Message content
			var contentDiv = E('div', { 'class': 'sms-message-content' }, msg.content);
			bubble.appendChild(contentDiv);

			// Message metadata
			var metaDiv = E('div', { 'class': 'sms-message-meta' });
			
			var timeStr = qmUI.formatTime(msg.timestamp);
			var timeSpan = E('span', { 'class': 'sms-message-time' }, timeStr);
			metaDiv.appendChild(timeSpan);

			// Show multipart info if applicable
			if (msg.multipart) {
				var multipartSpan = E('span', { 
					'class': 'sms-message-info',
					'style': 'margin-left: 10px;'
				}, _('Multi-part'));
				metaDiv.appendChild(multipartSpan);
			}

			// Show incomplete warning
			if (msg.incomplete) {
				var incompleteSpan = E('span', { 
					'class': 'sms-message-warning',
					'style': 'margin-left: 10px; color: orange;'
				}, _('Incomplete (part ') + msg.part + '/' + msg.total + ')');
				metaDiv.appendChild(incompleteSpan);
			}

			// Show failed indicator for sent messages
			if (isSent && msg.is_success === false) {
				var failedSpan = E('span', { 
					'class': 'sms-message-failed',
					'style': 'margin-left: 10px; color: #e74c3c;',
					'title': _('Failed to send')
				}, '✗ ' + _('Failed'));
				metaDiv.appendChild(failedSpan);
			}

			// Delete button for all messages
			var deleteBtn = E('button', {
				'class': 'cbi-button cbi-button-remove',
				'style': 'margin-left: 10px; font-size: 11px; padding: 2px 6px;',
				'click': L.bind(function(message, contact, ev) {
					ev.preventDefault();
					if (confirm(_('Delete this message?'))) {
						var msgContact = message.type === 'received' ? message.sender : message.recipient;
						this.deleteMessage(configSection, message, msgContact, messagesArea);
					}
				}, this, msg, (isReceived ? msg.sender : msg.recipient))
			}, _('Delete'));
			metaDiv.appendChild(deleteBtn);

			bubble.appendChild(metaDiv);

			// Wrapper for alignment
			var wrapperClass = 'sms-message-wrapper ' + 
			                   (isReceived ? 'sms-message-wrapper-left' : 'sms-message-wrapper-right');
			var wrapper = E('div', { 'class': wrapperClass }, bubble);

			messagesDiv.appendChild(wrapper);
		}

		messagesArea.appendChild(messagesDiv);

		// Scroll to bottom
		setTimeout(function() {
			messagesArea.scrollTop = messagesArea.scrollHeight;
		}, 100);
	},

	deleteMessage: function(configSection, message, contact, messagesArea) {
		var self = this;

		// Determine which IDs to delete
		var idsToDelete;
		if (message.part_ids && message.part_ids.length > 0) {
			// For multipart messages, delete all parts
			idsToDelete = message.part_ids;
		} else {
			// For single messages, delete by ID
			idsToDelete = [message.id];
		}

		smsService.deleteSms(
			configSection,
			message.type,
			idsToDelete
		).then(function(result) {
			if (result.error) {
				ui.addNotification(null, E('p', _('Failed to delete SMS: ') + result.error), 'error');
				return;
			}

			ui.addNotification(null, E('p', _('Message deleted')), 'info');
			
			// Reload conversation
			var loading = E('div', { 'class': 'spinning' }, _('Loading...'));
			dom.content(messagesArea, loading);
			self.loadConversation(messagesArea, loading, configSection, contact);
		}).catch(function(err) {
			ui.addNotification(null, E('p', _('Failed to delete SMS: ') + err.message), 'error');
		});
	},

	// Full timestamp formatting is provided by qmUI.formatTime (qmodem/ui.js)

	handleSaveApply: null,
	handleSave: null,
	handleReset: null
});
