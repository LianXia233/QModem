'use strict';
'require ui';
'require dom';
'require qmodem.compat';

/*
 * QModem 统一前端 —— 公共组件原语（generic 皮肤实现）
 *
 * 本文件原为 luci-app-qmodem-next 的私有组件层，现已被 qmodem.controls 收编：
 *   - 页面不再直接 require 本模块，统一经 `qmodem.controls` 取用；
 *   - 渲染类名与结构改为 generic（qmodem-generic）的 mt-* / theme.js 皮肤，
 *     即"next 的功能、generic 的样子"；
 *   - 原本只服务于已退役视图（overview.js / status/include/11_modem.js）
 *     的 InfoTable、progressbar 已随视图一并删除。
 *
 * 对外契约保持不变，调用方无需改动结构：
 *   ModemSelector(modems, options) -> { section, select, body }
 *     section : 可整块挂载的容器（内部含一张 table，兼容 querySelector('table')）
 *     select  : 原生 <select>
 *     body    : <tbody>，调用方可继续向其中追加自定义行
 */

// --- CSS 能力探测 -----------------------------------------------------------

function hasCssClass(className) {
	var styleSheets = document.styleSheets;
	for (var i = 0; i < styleSheets.length; i++) {
		try {
			var rules = styleSheets[i].cssRules || styleSheets[i].rules;
			if (rules) {
				for (var j = 0; j < rules.length; j++) {
					if (rules[j].selectorText && rules[j].selectorText.indexOf(className) !== -1) {
						return true;
					}
				}
			}
		} catch(e) {
			// 跨域样式表，跳过
		}
	}
	return false;
}

// --- 模组选择器 -------------------------------------------------------------
// generic 皮肤：外层 .mt-modem-bar + 标签 .mt-modem-bar-label + 下拉 .mt-modem-select。
// 保留 next 时代的 table/tbody 结构，使既有页面（sms.js 会向 body 追加行、
// 并用 section.querySelector('table') 取表）无需改动。

function ModemSelector(modems, options) {
	options = options || {};

	var select = E('select', {
		'class': 'cbi-input-select mt-modem-select',
		'id': options.id || 'modem_selector'
	});

	(modems || []).forEach(function(modem) {
		if (!options.filter || options.filter(modem)) {
			select.appendChild(E('option', { 'value': modem.id }, modem.name));
		}
	});

	var labelCell = E('td', {
		'class': 'td left mt-modem-bar-label',
		'width': '33%'
	}, _('Modem Name'));
	var selectCell = E('td', { 'class': 'td' });
	selectCell.appendChild(select);

	var selectorBody = E('tbody', {});
	selectorBody.appendChild(E('tr', { 'class': 'tr' }, [ labelCell, selectCell ]));

	var selectorTable = E('table', { 'class': 'table mt-modem-table' });
	selectorTable.appendChild(selectorBody);

	// generic 皮肤容器；section 仍为可整块挂载的块级元素
	var selectorSection = E('div', { 'class': 'mt-modem-bar mt-ui-card' }, [
		selectorTable
	]);

	return { section: selectorSection, select: select, body: selectorBody };
}

// --- 时间格式化 -------------------------------------------------------------

// 完整时间戳：Date 对象或 epoch 秒 -> 'YYYY-MM-DD HH:mm:ss'
function formatTime(value) {
	var date = (value instanceof Date) ? value : new Date(value * 1000);
	if (isNaN(date.getTime())) return '-';

	var pad = function(n) { return String(n).padStart(2, '0'); };
	return date.getFullYear() + '-' + pad(date.getMonth() + 1) + '-' + pad(date.getDate()) + ' ' +
		pad(date.getHours()) + ':' + pad(date.getMinutes()) + ':' + pad(date.getSeconds());
}

// 相对时间戳：epoch 秒 -> 今天显示 HH:mm / 昨天 / 星期几 / 具体日期
function relativeTime(timestamp) {
	var date = new Date(timestamp * 1000), now = new Date(), days = Math.floor((now - date) / 86400000);
	if (days === 0) return String(date.getHours()).padStart(2, '0') + ':' + String(date.getMinutes()).padStart(2, '0');
	if (days === 1) return _('Yesterday');
	if (days < 7) return [ _('Sunday'), _('Monday'), _('Tuesday'), _('Wednesday'), _('Thursday'), _('Friday'), _('Saturday') ][date.getDay()];
	return date.getFullYear() + '-' + String(date.getMonth() + 1).padStart(2, '0') + '-' + String(date.getDate()).padStart(2, '0');
}

// --- 轮询失败守卫 -----------------------------------------------------------
// 包装轮询失败回调：错误始终写入控制台，界面提示按 context 限频
//（默认同一 context 每 30 秒最多提示一次），避免后台轮询失败刷屏。

var pollGuardLastShown = {};

function pollGuard(context, intervalMs) {
	intervalMs = intervalMs || 30000;

	return function(err) {
		console.error('Poll error [' + context + ']:', err);

		var now = Date.now();
		if (!pollGuardLastShown[context] || (now - pollGuardLastShown[context]) > intervalMs) {
			pollGuardLastShown[context] = now;
			ui.addNotification(null,
				E('p', _('Background refresh failed: %s').format((err && err.message) ? err.message : err)),
				'error');
		}
	};
}

return L.Class.extend({
	hasCssClass: hasCssClass,
	ModemSelector: ModemSelector,
	formatTime: formatTime,
	relativeTime: relativeTime,
	pollGuard: pollGuard
});
