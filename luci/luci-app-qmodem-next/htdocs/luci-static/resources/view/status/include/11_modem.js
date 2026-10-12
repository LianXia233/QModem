'use strict';
'require baseclass';
'require qmodem.qmodem as qmodem';
'require qmodem.ui as qmUI';

// Load CSS (deduplicated) so compat-progressbar styles are available when the
// active theme does not provide .cbi-progressbar.
qmodem.loadCss(L.resource('qmodem/qmodem-next.css'));

// Note: the legacy luci-app-qmodem package ships its own 11_modem.js with a
// duplicated progressbar/table implementation. It is intentionally untouched
// here and will be consolidated during the legacy-package retirement phase;
// the shared rendering helpers now live in qmodem/ui.js.

return baseclass.extend({
	title: _('Modem Info'),

	load: function() {
		return qmodem.getModemSections().then(function(sections) {
			var promises = sections.map(function(section) {
				// allSettled: one failed source degrades to an empty section
				// (renders as "No modem information available") instead of
				// blanking every modem on the status page
				return Promise.allSettled([
					qmodem.getBaseInfo(section.id),
					qmodem.getCellInfo(section.id)
				]).then(function(results) {
					var allInfo = [];
					results.forEach(function(result) {
						if (result.status === 'fulfilled' && result.value && result.value.modem_info) {
							allInfo = allInfo.concat(result.value.modem_info);
						}
					});
					return {
						section: section,
						info: allInfo
					};
				});
			});
			return Promise.all(promises);
		});
	},

	render: function(data) {
		var container = E('div', {});

		if (!data || data.length === 0) {
			var table = E('table', { 'class': 'table' });
			table.appendChild(E('tr', { 'class': 'tr' }, [
				E('td', { 'class': 'td left', 'width': '100%' }, [ _('No modem information available') ])
			]));
			return table;
		}

		try {
			for (var m = 0; m < data.length; m++) {
				var modem = data[m];
				var table = E('table', { 'class': 'table' });
				var fields = [];

				// Add section header
				if (modem.section && modem.section.name) {
					table.appendChild(E('tr', { 'class': 'tr table-titles' }, [
						E('th', { 'class': 'th', 'colspan': '2' }, [ modem.section.name ])
					]));
				}

				var infoArray = modem.info || [];
				for (var i = 0; i < infoArray.length; i++) {
					var entry = infoArray[i];
					var full_name = entry.full_name;
					var extra_info = entry.extra_info;
					
					if (entry.value == null) {
						continue;
					}
					
					if ((entry.class == 'Base Information') || 
						(entry.class == 'Cell Information' && entry.type == 'progress_bar')) {
						fields.push(extra_info ? '%s (%s)'.format(_(full_name), extra_info) : _(full_name));
						fields.push(entry);
					}
				}

				if (fields.length == 0) {
					table.appendChild(E('tr', { 'class': 'tr' }, [
						E('td', { 'class': 'td left', 'width': '100%' }, [ _('No modem information available') ])
					]));
					container.appendChild(table);
					continue;
				}

				for (var i = 0; i < fields.length; i += 2) {
					var entry = fields[i + 1];
					var type = entry.type;
					var value;
					if (type == 'progress_bar') {
					value = E('td', { 'class': 'td left' }, [
						(entry.value != null) ? qmUI.progressbar(entry.value, entry.max_value, entry.min_value, entry.unit) : _('N/A')
					]);
				} else {
					value = E('td', { 'class': 'td left' }, [ (entry.value != null) ? entry.value : _('N/A') ]);
					}

					table.appendChild(E('tr', { 'class': 'tr' }, [
						E('td', { 'class': 'td left', 'width': '33%' }, [ fields[i] ]),
						value
					]));
				}

				container.appendChild(table);
			}

			return container;
		}
		catch (e) {
			var table = E('table', { 'class': 'table' });
			table.appendChild(E('tr', { 'class': 'tr' }, [
				E('td', { 'class': 'td left', 'width': '100%' }, [ _('No modem information available') ])
			]));
			return table;
		}
	}
});
