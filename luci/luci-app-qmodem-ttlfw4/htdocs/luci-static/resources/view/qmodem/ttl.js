'use strict';
'require view';
'require form';
'require uci';
'require rpc';
'require ui';

// Load QModem Soft Anime design system (self-contained within package)
document.head.appendChild(E('link', {
	'rel': 'stylesheet',
	'type': 'text/css',
	'href': L.resource('qmodem-ttlfw4/soft-anime.css') + '?v=' + new Date().toISOString().slice(0, 10)
}));

var callInitAction = rpc.declare({
	object: 'luci',
	method: 'setInitAction',
	params: ['name', 'action'],
	expect: { result: false }
});

var callRcStatus = rpc.declare({
	object: 'rc',
	method: 'list',
	params: ['name'],
	expect: { }
});

return view.extend({
	load: function() {
		var self = this;
		return Promise.all([
			uci.load('qmodem_ttl'),
		]).then(function(results) {
			return results;
		});
	},

	render: function() {
		var m, s, o;
		var self = this;

		m = new form.Map('qmodem_ttl', _('TTL Configuration'),
			_('Configure TTL/Hop Limit settings to modify outgoing packet TTL values. This can help bypass carrier restrictions on tethering.'));

		// Global TTL Configuration
		s = m.section(form.NamedSection, 'main', 'main', _('Global TTL Settings'));
		s.anonymous = true;

		// Enable TTL modification
		o = s.option(form.Flag, 'enable', _('Enable TTL Modification'));
		o.default = '0';
		o.rmempty = false;
		o.description = _('Enable or disable TTL/Hop Limit modification for outgoing packets.');

		// TTL Value
		o = s.option(form.Value, 'ttl', _('TTL Value'));
		o.datatype = 'range(1,255)';
		o.default = '64';
		o.placeholder = '64';
		o.description = _('Set the TTL (Time To Live) value for IPv4 packets and Hop Limit for IPv6 packets. Common values: 64 (Linux default), 128 (Windows default).');

		// Preset buttons helper
		var presets = s.option(form.DummyValue, '_presets', _('Common Presets'));
		presets.rawhtml = true;
		presets.cfgvalue = function() {
			var wrap = E('div', { 'style': 'display: flex; gap: 8px; flex-wrap: wrap; margin-top: 4px;' });
			var options = [
				{ label: _('64 (Linux / Android / macOS)'), val: '64' },
				{ label: _('128 (Windows)'), val: '128' },
				{ label: _('65 (Carrier Bypass +1)'), val: '65' }
			];
			options.forEach(function(opt) {
				wrap.appendChild(E('button', {
					'class': 'qmodem-preset-btn',
					'type': 'button',
					'click': function(ev) {
						ev.preventDefault();
						var input = document.querySelector('input[id*="ttl"]');
						if (input) {
							input.value = opt.val;
							input.dispatchEvent(new Event('change', { bubbles: true }));
						}
					}
				}, opt.label));
			});
			return wrap;
		};

		// Warning section
		s = m.section(form.NamedSection, 'main', 'main', _('Important Notes'));
		s.anonymous = true;

		var warning = s.option(form.DummyValue, '_warning');
		warning.rawhtml = true;
		warning.cfgvalue = function() {
			return E('div', { 'class': 'qmodem-inline-warning' }, [
				E('div', { 'class': 'qmodem-warn-title' }, [
					E('span', { 'class': 'qmodem-dot qmodem-dot-warn' }),
					E('strong', {}, _('Important Warning:'))
				]),
				E('ul', { 'class': 'qmodem-warn-list' }, [
					E('li', {}, _('Enabling TTL modification will disable hardware flow offloading for proper packet modification.')),
					E('li', {}, _('This may affect network performance on some devices.')),
					E('li', {}, _('NSS ECM, SFE, and other acceleration modules will be disabled when TTL modification is active.')),
					E('li', {}, _('Settings will take effect after saving and applying changes.'))
				])
			]);
		};

		return m.render().then(function(rendered) {
			var hero = E('div', { 'class': 'qmodem-hero' }, [
				E('h1', { 'class': 'qmodem-hero-title' }, [
					_('Packet TTL / Hop Limit Rules'),
					E('span', { 'class': 'qmodem-chip qmodem-chip-info' }, _('Firewall4 / NFT'))
				]),
				E('p', { 'class': 'qmodem-hero-sub' },
					_('Customize outgoing IPv4 Time-To-Live and IPv6 Hop Limit packets to bypass tethering and hotspot throttling.'))
			]);
			var heroIcon = E('div', { 'class': 'qmodem-mascot qmd-anim-float', 'aria-hidden': 'true' });
			heroIcon.innerHTML =
				'<svg viewBox="0 0 76 60" width="76" height="60" fill="none">' +
				'<path d="M38 10 L58 18 L58 36 C58 48 38 54 38 54 C38 54 18 48 18 36 L18 18 Z" fill="#FFFFFF" stroke="#8875E8" stroke-width="2.5" stroke-linejoin="round"/>' +
				'<path d="M38 16 L52 22 L52 35 C52 44 38 48 38 48 C38 48 24 44 24 35 L24 22 Z" fill="#EEE9FF"/>' +
				'<path d="M32 32 L36 36 L44 26" stroke="#36AD87" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" class="qmd-anim-pulse"/>' +
				'</svg>';
			hero.appendChild(heroIcon);
			if (rendered.firstChild) {
				rendered.insertBefore(hero, rendered.firstChild);
			} else {
				rendered.appendChild(hero);
			}
			return rendered;
		});
	},
});
