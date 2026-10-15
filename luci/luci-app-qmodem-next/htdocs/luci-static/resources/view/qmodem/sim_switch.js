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

		// Soft Anime Hero
		var hero = E('div', { 'class': 'qmodem-hero' }, [
			E('h1', { 'class': 'qmodem-hero-title' }, [
				_('SIM Slot Switcher'),
				E('span', { 'class': 'qmodem-chip qmodem-chip-info' }, _('Hardware Slot Control'))
			]),
			E('p', { 'class': 'qmodem-hero-sub' },
				_('Toggle active SIM physical slots on dual-SIM cellular modems. Some modules may automatically re-register.'))
		]);

		// Animated Dual-SIM Card SVG
		var heroIcon = E('div', { 'class': 'qmodem-mascot qmd-anim-float', 'aria-hidden': 'true' });
		heroIcon.innerHTML =
			'<svg viewBox="0 0 76 60" width="76" height="60" fill="none">' +
			'<rect x="10" y="14" width="24" height="34" rx="4" fill="#FFFFFF" stroke="#8875E8" stroke-width="2.2"/>' +
			'<rect x="42" y="14" width="24" height="34" rx="4" fill="#FFFFFF" stroke="#EFA8CB" stroke-width="2.2"/>' +
			'<circle cx="22" cy="31" r="3" fill="#36AD87" class="qmd-anim-pulse"/>' +
			'<circle cx="54" cy="31" r="3" fill="#8875E8"/>' +
			'<path d="M26 10 L50 10" stroke="#8875E8" stroke-width="2" stroke-linecap="round" stroke-dasharray="2 3"/>' +
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
		container.appendChild(selectorSection);

		// SIM switch interface container
		var interfaceContainer = E('div', { 'id': 'sim_switch_container' });
		container.appendChild(interfaceContainer);

		var self = this;
		var updateInterface = function() {
			var selectedId = select.value;
			var selectedModem = modems.find(function(m) { return m.id === selectedId; });
			if (selectedModem) {
				dom.content(interfaceContainer, self.createSimSwitchInterface(selectedModem));
			}
		};

		select.addEventListener('change', updateInterface);
		updateInterface();

		return container;
	},

	createSimSwitchInterface: function(modem) {
		var self = this;
		var wrap = E('div', {});

		var loadingCard = E('div', { 'class': 'cbi-section qmodem-skeleton-card', 'style': 'padding: 24px; text-align: center;' }, [
			E('div', { 'class': 'spinning' }, _('Probing SIM slot capabilities for %s...').format(modem.name))
		]);
		wrap.appendChild(loadingCard);

		var resultSection = E('div', { 'id': 'sim_result_section_' + modem.id, 'style': 'display: none; margin-bottom: 16px;' });
		wrap.appendChild(resultSection);

		// Load SIM switch support and current slot concurrently
		Promise.all([
			qmodem.callGetSimSwitchCapabilities(modem.id),
			qmodem.getSimSlot(modem.id)
		]).then(function(results) {
			wrap.removeChild(loadingCard);

			var supportResult = results[0];
			var slotResult = results[1];
			var supported = supportResult && supportResult.supportSwitch === '1';
			var currentSlot = slotResult && slotResult.sim_slot ? slotResult.sim_slot : 'N/A';
			var slots = (supportResult && Array.isArray(supportResult.simSlots)) ? supportResult.simSlots : ['1', '2'];

			// Status & Note Header
			var statusCard = E('div', { 'class': 'cbi-section', 'style': 'margin-bottom: 16px; padding: 14px 18px;' });
			var statusRow = E('div', { 'class': 'qmodem-status-row' }, [
				E('span', { 'class': 'qmodem-chip ' + (supported ? 'qmodem-chip-ok' : 'qmodem-chip-warn') }, [
					E('span', { 'class': 'qmodem-dot ' + (supported ? 'qmodem-dot-ok' : 'qmodem-dot-warn') }),
					_('Hardware Support') + ': ' + (supported ? _('Supported') : _('Not Supported'))
				]),
				E('span', { 'class': 'qmodem-chip qmodem-chip-info' }, [
					_('Current Active Slot') + ': ' + self.formatSlotDisplay(currentSlot)
				])
			]);
			statusCard.appendChild(statusRow);

			if (supportResult && supportResult.ExtraInfo) {
				statusCard.appendChild(E('div', { 'class': 'qmodem-inline-warning', 'style': 'margin-top: 12px;' }, [
					E('div', { 'class': 'qmodem-warn-title' }, [
						E('span', { 'class': 'qmodem-dot qmodem-dot-warn' }),
						E('strong', {}, _('Note:'))
					]),
					E('span', {}, _(supportResult.ExtraInfo))
				]));
			}
			wrap.appendChild(statusCard);

			if (!supported) {
				wrap.appendChild(E('div', { 'class': 'alert-message warning' },
					_('This modem model or current firmware does not support software SIM switching.')));
				return;
			}

			// Interactive SIM Slot Visual Cards
			var slotsGrid = E('div', { 'class': 'qmodem-sim-slot-grid' });
			var slotButtons = [];

			slots.forEach(function(slotVal) {
				var isActive = (String(currentSlot) === String(slotVal));
				
				var card = E('div', {
					'class': 'qmodem-sim-slot-card' + (isActive ? ' active' : ''),
					'id': 'sim_card_slot_' + slotVal
				});

				var cardHeader = E('div', { 'style': 'display: flex; align-items: center; justify-content: space-between; margin-bottom: 14px;' }, [
					E('div', { 'style': 'display: flex; align-items: center; gap: 8px;' }, [
						createSvg('<svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="var(--qmd-primary)" stroke-width="2"><path d="M6 3h9l5 5v13H6V3zM10 12h4M10 16h4"/></svg>'),
						E('span', { 'style': 'font-size: 16px; font-weight: 700; color: var(--qmd-text-1);' }, _('SIM Slot %s').format(slotVal))
					]),
					E('span', {
						'class': 'qmodem-chip ' + (isActive ? 'qmodem-chip-ok' : 'qmodem-chip-muted'),
						'id': 'sim_chip_slot_' + slotVal
					}, [
						isActive ? E('span', { 'class': 'qmodem-dot qmodem-dot-ok' }) : null,
						isActive ? _('Active Slot') : _('Standby')
					])
				]);

				var switchBtn = E('button', {
					'class': 'cbi-button ' + (isActive ? 'cbi-button-positive' : 'cbi-button-action'),
					'type': 'button',
					'style': 'width: 100%; min-height: 40px; font-weight: 600; margin-top: 10px;',
					'disabled': isActive,
					'data-slot': slotVal,
					'click': function() {
						self.switchSimSlot(modem.id, slotVal, slotButtons, wrap);
					}
				}, isActive ? _('Currently Active') : _('Switch to Slot %s').format(slotVal));

				slotButtons.push(switchBtn);

				card.appendChild(cardHeader);
				card.appendChild(E('p', { 'style': 'font-size: 12.5px; color: var(--qmd-text-2); margin-bottom: 12px;' },
					isActive ? _('This SIM card is currently serving cellular connections.') : _('Click below to switch the active RF path to this slot.')));
				card.appendChild(switchBtn);
				slotsGrid.appendChild(card);
			});

			wrap.appendChild(slotsGrid);

		}).catch(function(e) {
			wrap.removeChild(loadingCard);
			wrap.appendChild(E('div', { 'class': 'alert-message error' },
				_('Error querying SIM switch capability: %s').format(e.message)));
		});

		return wrap;
	},

	formatSlotDisplay: function(slot) {
		if (!slot) return 'N/A';
		return _('Slot %s').format(slot);
	},

	switchSimSlot: function(modemId, slot, buttons, wrap) {
		var resultSection = document.getElementById('sim_result_section_' + modemId);
		if (resultSection) {
			resultSection.style.display = '';
			dom.content(resultSection, E('div', { 'class': 'cbi-section qmodem-skeleton-card', 'style': 'padding: 18px; text-align: center;' }, [
				E('div', { 'class': 'spinning' }, _('Switching active RF path to Slot %s...').format(slot))
			]));
		}

		(buttons || []).forEach(function(b) { b.disabled = true; });

		qmodem.setSimSlot(modemId, slot).then(function(result) {
			(buttons || []).forEach(function(b) {
				var bs = b.getAttribute('data-slot');
				var isTarget = (String(bs) === String(slot));
				b.disabled = isTarget;
				b.className = 'cbi-button ' + (isTarget ? 'cbi-button-positive' : 'cbi-button-action');
				b.textContent = isTarget ? _('Currently Active') : _('Switch to Slot %s').format(bs);

				var card = document.getElementById('sim_card_slot_' + bs);
				if (card) {
					if (isTarget) card.classList.add('active');
					else card.classList.remove('active');
				}

				var chip = document.getElementById('sim_chip_slot_' + bs);
				if (chip) {
					chip.className = 'qmodem-chip ' + (isTarget ? 'qmodem-chip-ok' : 'qmodem-chip-muted');
					dom.content(chip, [
						isTarget ? E('span', { 'class': 'qmodem-dot qmodem-dot-ok' }) : null,
						isTarget ? _('Active Slot') : _('Standby')
					]);
				}
			});

			if (resultSection) {
				dom.content(resultSection, E('div', { 'class': 'alert-message success' }, [
					E('strong', {}, _('SIM slot switched to Slot %s successfully.').format(slot)),
					E('div', { 'style': 'font-size: 12px; margin-top: 4px;' },
						_('Note: Some modems may take 10-30 seconds to re-register on cellular networks.'))
				]));
			}
		}).catch(function(e) {
			(buttons || []).forEach(function(b) { b.disabled = false; });
			if (resultSection) {
				dom.content(resultSection, E('div', { 'class': 'alert-message error' },
					_('Failed to switch SIM slot: %s').format(e.message)));
			}
		});
	},

	handleSaveApply: null,
	handleSave: null,
	handleReset: null
});
