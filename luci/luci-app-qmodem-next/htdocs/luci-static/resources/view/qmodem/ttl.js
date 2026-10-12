// QModem next: TTL management view (fw4/nft backend).
// Ported from luci-app-qmodem-ttlfw4 htdocs/luci-static/resources/view/qmodem/ttl.js;
// the backend (init.d + /etc/config/qmodem_ttl) still ships with the ttlfw4
// package until Phase 4 retirement. See docs/refactor/journal-next-ttl.md.
'use strict';
'require view';
'require form';
'require uci';

return view.extend({
	load: function() {
		return uci.load('qmodem_ttl');
	},

	render: function() {
		var m, s, o;

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

		// Warning section
		s = m.section(form.NamedSection, 'main', 'main', _('Important Notes'));
		s.anonymous = true;

		o = s.option(form.DummyValue, '_warning');
		o.rawhtml = true;
		o.cfgvalue = function() {
			return E('div', { 'class': 'cbi-value-description' }, [
				E('p', { 'style': 'color: #c00; font-weight: bold;' }, _('Warning:')),
				E('ul', {}, [
					E('li', {}, _('TTL rewriting is implemented with the fw4/nftables backend (/etc/nftables.d/99-reset-ttl-from-br-lan.nft).')),
					E('li', {}, _('Enabling TTL modification will disable hardware flow offloading for proper packet modification.')),
					E('li', {}, _('NSS ECM, SFE, and other acceleration modules will be disabled when TTL modification is active.')),
					E('li', {}, _('This may affect network performance on some devices.')),
					E('li', {}, _('Settings will take effect after saving and applying changes.'))
				])
			]);
		};

		return m.render();
	}
});
