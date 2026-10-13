'use strict';

/*
 * QModem LuCI 旧 API 兼容层（独立模块）
 *
 * 统一前端的全部视图仍沿用 LuCI 25.x 时代的写法（全局 E() / findParent()、
 * String.prototype.format），而 LuCI 26.246+ 已移除这些历史辅助。缺失时按需补齐、
 * 已存在绝不覆盖，避免影响其它插件。
 *
 * 本模块只做副作用注入，不导出任何接口；每个视图与 controls.js 都显式 require 它，
 * 因此不依赖模块加载顺序。后续若视图整体迁移到 L.dom.create / 模板字符串，本模块可删除。
 */

/*
 * ------------------------------------------------------------------ *
 * LuCI 26.x（OpenWrt/ImmortalWrt Master 26.246+）旧 API 兼容层            *
 * ------------------------------------------------------------------ *
 * 新版 LuCI 已移除两个历史全局辅助：
 *   1) String.prototype.format —— 仅保留 String.prototype.format.call()，
 *      直接写 'x'.format(...) 会抛 ".format is not a function"；
 *   2) 全局 E()（旧 dom.create 别名）与 findParent()。
 * 本包视图仍沿用这些写法（共计数百处），在 26.x 上会出现"菜单正常、内容区
 * 永远停在 Loading view"的现象。这里在本模块被 require 时（早于任何视图体
 * 执行）按需补齐，缺失才注入、已存在绝不覆盖，避免影响其它插件。
 * 后续若视图整体迁移到 L.dom.create / 模板字符串，本段可直接删除。
 */
(function installCompat() {
	if (typeof window === 'undefined')
		return;

	if (!String.prototype.format) {
		var NUM_TYPES = 'diouxXfFeEgG';
		String.prototype.format = function() {
			var args = arguments, idx = 0;
			return String(this).replace(/%(\d+\$)?([-+ #0]*)(\d+|\*)?(?:\.(\d+|\*))?([diouxXfFeEgGcs%])/g,
				function(m, pos, flags, width, prec, conv) {
					if (conv === '%') return '%';
					if (pos) idx = parseInt(pos, 10) - 1;
					var v = args[idx++];
					if (v === undefined || v === null) return m;
					if (width === '*') width = args[idx++];
					if (prec === '*') prec = args[idx++];
					var s;
					if (NUM_TYPES.indexOf(conv) >= 0) {
						var n;
						if ('diu'.indexOf(conv) >= 0) n = parseInt(v, 10);
						else if (conv === 'o') n = parseInt(v, 10).toString(8);
						else if (conv === 'x') n = parseInt(v, 10).toString(16);
						else if (conv === 'X') n = parseInt(v, 10).toString(16).toUpperCase();
						else n = parseFloat(v);
						if (prec !== undefined && 'fFeE'.indexOf(conv) >= 0)
							s = parseFloat(v).toFixed(parseInt(prec, 10));
						else
							s = String(n);
					} else if (conv === 'c') {
						s = String(v).charAt(0);
					} else {
						s = String(v);
						if (prec !== undefined) s = s.substring(0, parseInt(prec, 10));
					}
					var w = width ? parseInt(width, 10) : 0;
					if (s.length >= w) return s;
					if (flags && flags.indexOf('-') < 0 && flags.indexOf('0') >= 0 && NUM_TYPES.indexOf(conv) >= 0) {
						var sign = /^[+-]/.test(s) ? s.charAt(0) : '';
						var body = sign ? s.substring(1) : s;
						while (body.length + sign.length < w) body = '0' + body;
						return sign + body;
					}
					while (s.length < w)
						s = (flags && flags.indexOf('-') >= 0) ? s + ' ' : ' ' + s;
					return s;
				});
		};
	}

	if (typeof window.E === 'undefined') {
		window.E = function(elem, attrs, data) {
			if (elem instanceof Node)
				return elem;
			var e = document.createElement(String(elem));
			if (attrs != null && typeof attrs === 'object') {
				for (var k in attrs)
					e.setAttribute(k, attrs[k]);
			}
			if (data != null) {
				if (data instanceof Array)
					data.forEach(function(x) {
						e.appendChild(x instanceof Node ? x : document.createTextNode(String(x)));
					});
				else if (data instanceof Node)
					e.appendChild(data);
				else
					e.innerHTML = String(data);
			}
			return e;
		};
	}

	if (typeof window.findParent === 'undefined') {
		window.findParent = function(node, selector) {
			while (node && node.parentNode) {
				node = node.parentNode;
				if (node instanceof Element && node.matches(selector))
					return node;
			}
			return null;
		};
	}

	/*
	 * 26.x 的 ui.js 在错误提示路径（LuCI.prototype.error → ui.addNotification）
	 * 里直接引用全局 `_()`，而该版本并不提供它：一旦视图加载抛错，先崩在
	 * 提示逻辑上，真正的原因被吞掉，只剩 "_ is not defined"。这里给出轻量兜底
	 * （原样返回，不做翻译），让被掩盖的错误文本能正常显示出来。
	 */
	if (typeof window._ === 'undefined') {
		window._ = function(s) { return String(s == null ? '' : s); };
	}
})();

/*
 * LuCI 的 require() 会把模块体编译成工厂函数，并校验返回值是合法 class
 * （luci.js: `if (!Class.isSubclass(_class))` → "factory yields invalid
 * constructor"）。本模块只做副作用注入，也必须返回一个空 class，不能返回
 * 普通对象（`return {}` 会直接触发上面的报错，整个页面停在 Loading view）。
 * 工厂作用域里 L 可用（工厂签名 function(window, document, L, ...)）。
 */
return L.Class.extend({});
