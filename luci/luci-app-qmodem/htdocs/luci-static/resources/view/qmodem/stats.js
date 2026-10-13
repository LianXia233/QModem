'use strict';
'require view';
'require qmodem.compat';
'require poll';
'require ui';
'require dom';
'require uci';
'require qmodem.controls as controls';
;


// Poll interval in seconds: stats are cheap to read and rates need a
// reasonably fresh baseline, same cadence as the backend rate sampler.
var POLL_INTERVAL = 5;

return view.extend({
	load: function() {
		return uci.load('qmodem');
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
					name: displayName
				});
			}
		});

		return modems;
	},

	// 4750 -> '4.63 KB', byte counts stay human readable
	formatBytes: function(bytes) {
		bytes = parseFloat(bytes);
		if (isNaN(bytes) || bytes < 0) return '-';

		var units = [ 'B', 'KB', 'MB', 'GB', 'TB' ];
		var i = 0;
		while (bytes >= 1024 && i < units.length - 1) {
			bytes /= 1024;
			i++;
		}

		return (i === 0 ? Math.round(bytes) : bytes.toFixed(2)) + ' ' + units[i];
	},

	// Build a progress bar in the same visual style as overview.js
	// (cbi-progressbar when the theme provides it, compat fallback otherwise)
	buildBar: function(label, current, peak) {
		var pct = (peak > 0) ? (current / peak) * 100 : 0;
		pct = Math.max(0, Math.min(100, pct));
		if (isNaN(pct)) pct = 0;

		var bar = E('div', {
			'class': controls.hasCssClass('.cbi-progressbar') ? 'cbi-progressbar' : 'compat-progressbar',
			'title': label + ': ' + this.formatBytes(current) + '/s'
		});
		bar.appendChild(E('div', { 'style': 'width:' + pct + '%' }));
		return bar;
	},

	// Render one modem card: RX/TX rate bars + cumulative counters
	updateModemStats: function(modem, card, state) {
		var self = this;

		controls.getUsageStats(modem.id).then(function(stats) {
			if (!stats || stats.available === false) {
				dom.content(card.body, E('div', { 'class': 'alert-message warning' },
					_('Vendor does not support traffic statistics')));
				return;
			}

			var now = Date.now();
			var rx = parseFloat(stats.total_rx_bytes) || 0;
			var tx = parseFloat(stats.total_tx_bytes) || 0;
			var rxRate = 0;
			var txRate = 0;

			if (state.prev) {
				var dt = (now - state.prev.t) / 1000;
				if (dt > 0) {
					// Counter went backwards: stats were reset, report zero rate
					rxRate = (rx >= state.prev.rx) ? (rx - state.prev.rx) / dt : 0;
					txRate = (tx >= state.prev.tx) ? (tx - state.prev.tx) / dt : 0;
				}
			}
			state.prev = { rx: rx, tx: tx, t: now };

			// Auto-scale peaks with decay so idle lines do not stay flat forever
			state.peakRx = Math.max(rxRate, state.peakRx * 0.85, 1024);
			state.peakTx = Math.max(txRate, state.peakTx * 0.85, 1024);

			dom.content(card.body, E('table', { 'class': 'table' }, E('tbody', { 'style': 'width: 100%' }, [
				E('tr', { 'class': 'tr' }, [
					E('td', { 'class': 'td left', 'width': '33%' }, _('RX Rate')),
					E('td', { 'class': 'td' }, self.buildBar('RX', rxRate, state.peakRx))
				]),
				E('tr', { 'class': 'tr' }, [
					E('td', { 'class': 'td left', 'width': '33%' }, _('TX Rate')),
					E('td', { 'class': 'td' }, self.buildBar('TX', txRate, state.peakTx))
				]),
				E('tr', { 'class': 'tr' }, [
					E('td', { 'class': 'td left', 'width': '33%' }, _('Total RX')),
					E('td', { 'class': 'td' }, self.formatBytes(rx))
				]),
				E('tr', { 'class': 'tr' }, [
					E('td', { 'class': 'td left', 'width': '33%' }, _('Total TX')),
					E('td', { 'class': 'td' }, self.formatBytes(tx))
				]),
				E('tr', { 'class': 'tr' }, [
					E('td', { 'class': 'td left', 'width': '33%' }, _('Updated')),
					E('td', { 'class': 'td' }, stats.updated_at ? controls.formatTime(stats.updated_at) : '-')
				])
			])));
		}).catch(controls.pollGuard('qmodem-stats-' + modem.id));
	},

	handleClearStats: function(modem, state) {
		var self = this;

		if (!confirm(_('Are you sure you want to clear the traffic statistics of modem %s?').format(modem.name)))
			return;

		controls.clearStats(modem.id).then(function(result) {
			if (result && result.result === false)
				throw new Error(_('Vendor does not support traffic statistics'));

			// Restart the rate baseline so the next sample is not a spike
			state.prev = null;
			state.peakRx = 1024;
			state.peakTx = 1024;
			ui.addNotification(null, E('p', _('Statistics cleared')), 'info');
		}).catch(function(e) {
			ui.addNotification(null,
				E('p', _('Failed to clear statistics: ') + ((e && e.message) ? e.message : e)),
				'error');
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
		var caption = E('h2', { 'class': 'cbi-map-caption' }, _('Traffic Statistics'));
		container.appendChild(caption);

		var cards = [];

		modems.forEach(function(modem) {
			var fieldset = E('fieldset', { 'class': 'cbi-section' });
			fieldset.appendChild(E('h2', { 'class': 'panel-title' }, modem.name));

			var body = E('div', { 'class': 'spinning' }, _('Loading...'));
			fieldset.appendChild(body);

			var actions = E('div', { 'class': 'cbi-section-actions' });
			actions.appendChild(E('button', {
				'class': 'btn cbi-button-remove',
				'click': function() { self.handleClearStats(modem, card.state); }
			}, _('Clear Statistics')));
			fieldset.appendChild(actions);

			container.appendChild(fieldset);

			var card = {
				modem: modem,
				fieldset: fieldset,
				body: body,
				// Per-modem polling state: previous counter sample + auto-scale peaks
				state: { prev: null, peakRx: 1024, peakTx: 1024 }
			};
			cards.push(card);
		});

		// Initial refresh + 5s polling; failures are rate-limited via pollGuard
		var refresh = function() {
			cards.forEach(function(card) {
				self.updateModemStats(card.modem, card, card.state);
			});
		};

		refresh();
		poll.add(refresh, POLL_INTERVAL);

		return container;
	},

	handleSaveApply: null,
	handleSave: null,
	handleReset: null
});
