(function() {
    "use strict";
    var B = __dom;
    var nodeCache = {};

    // ============================================================
    // Event System
    // ============================================================
    var eventListeners = {};

    function getListeners(nid, type) {
        var key = nid + ":" + type;
        if (!eventListeners[key]) eventListeners[key] = [];
        return eventListeners[key];
    }

    function addEventListenerImpl(nid, type, fn, opts) {
        if (typeof fn !== "function") return;
        var listeners = getListeners(nid, type);
        for (var i = 0; i < listeners.length; i++) {
            if (listeners[i].fn === fn) return;
        }
        var capture = false;
        var once = false;
        if (typeof opts === "boolean") capture = opts;
        else if (opts) { capture = !!opts.capture; once = !!opts.once; }
        listeners.push({ fn: fn, capture: capture, once: once });
    }

    function removeEventListenerImpl(nid, type, fn) {
        var listeners = getListeners(nid, type);
        for (var i = 0; i < listeners.length; i++) {
            if (listeners[i].fn === fn) {
                listeners.splice(i, 1);
                return;
            }
        }
    }

    function dispatchEventImpl(nid, evt) {
        if (!evt) return true;
        var type = evt.type || evt;
        if (typeof type !== "string") return true;
        var listeners = getListeners(nid, type);
        var toRemove = [];
        for (var i = 0; i < listeners.length; i++) {
            try { listeners[i].fn.call(null, evt); } catch(e) { console.error(e); }
            if (listeners[i].once) toRemove.push(i);
        }
        for (var j = toRemove.length - 1; j >= 0; j--) {
            listeners.splice(toRemove[j], 1);
        }
        return !(evt && evt.defaultPrevented);
    }

    // Global event targets: document=-1, window=-2
    var DOC_EID = -1;
    var WIN_EID = -2;

    // ============================================================
    // MutationObserver (basic stub)
    // ============================================================
    var mutationObservers = [];

    function MutationObserverImpl(callback) {
        this._callback = callback;
        this._targets = [];
        this._records = [];
    }
    MutationObserverImpl.prototype.observe = function(target, opts) {
        this._targets.push({ target: target, opts: opts || {} });
        mutationObservers.push(this);
    };
    MutationObserverImpl.prototype.disconnect = function() {
        this._targets = [];
        var idx = mutationObservers.indexOf(this);
        if (idx >= 0) mutationObservers.splice(idx, 1);
    };
    MutationObserverImpl.prototype.takeRecords = function() {
        var r = this._records;
        this._records = [];
        return r;
    };

    globalThis.MutationObserver = MutationObserverImpl;

    // ============================================================
    // Event / CustomEvent constructors
    // ============================================================
    function EventImpl(type, opts) {
        opts = opts || {};
        this.type = type;
        this.bubbles = !!opts.bubbles;
        this.cancelable = !!opts.cancelable;
        this.composed = !!opts.composed;
        this.defaultPrevented = false;
        this.target = null;
        this.currentTarget = null;
        this.timeStamp = Date.now();
        this.isTrusted = false;
    }
    EventImpl.prototype.preventDefault = function() { this.defaultPrevented = true; };
    EventImpl.prototype.stopPropagation = function() {};
    EventImpl.prototype.stopImmediatePropagation = function() {};
    EventImpl.prototype.initEvent = function(type, bubbles, cancelable) {
        this.type = type;
        this.bubbles = !!bubbles;
        this.cancelable = !!cancelable;
    };

    function CustomEventImpl(type, opts) {
        EventImpl.call(this, type, opts);
        this.detail = (opts && opts.detail !== undefined) ? opts.detail : null;
    }
    CustomEventImpl.prototype = Object.create(EventImpl.prototype);
    CustomEventImpl.prototype.constructor = CustomEventImpl;

    globalThis.Event = EventImpl;
    globalThis.CustomEvent = CustomEventImpl;

    // ============================================================
    // Style proxy with cssText
    // ============================================================
    function createStyleProxy(nid) {
        var obj = {};
        Object.defineProperty(obj, 'cssText', {
            get: function() { return B.getAttribute(nid, 'style') || ''; },
            set: function(v) { B.setAttribute(nid, 'style', v); },
            configurable: true
        });
        obj.getPropertyValue = function(prop) {
            var css = B.getAttribute(nid, 'style') || '';
            var re = new RegExp('(?:^|;)\\s*' + prop.replace(/[.*+?^${}()|[\]\\]/g, '\\$&') + '\\s*:\\s*([^;]+)', 'i');
            var m = css.match(re);
            return m ? m[1].trim() : '';
        };
        obj.setProperty = function(prop, value) {
            var css = B.getAttribute(nid, 'style') || '';
            var re = new RegExp('((?:^|;)\\s*)' + prop.replace(/[.*+?^${}()|[\]\\]/g, '\\$&') + '\\s*:[^;]*(;|$)', 'i');
            if (re.test(css)) {
                css = css.replace(re, '$1' + prop + ': ' + value + '$2');
            } else {
                css = css ? css.replace(/;?\s*$/, '; ' + prop + ': ' + value) : prop + ': ' + value;
            }
            B.setAttribute(nid, 'style', css);
        };
        obj.removeProperty = function(prop) {
            var old = obj.getPropertyValue(prop);
            var css = B.getAttribute(nid, 'style') || '';
            var re = new RegExp('\\s*' + prop.replace(/[.*+?^${}()|[\]\\]/g, '\\$&') + '\\s*:[^;]*(;|$)', 'gi');
            B.setAttribute(nid, 'style', css.replace(re, '').replace(/^;\s*/, ''));
            return old;
        };
        return obj;
    }

    // ============================================================
    // Dataset proxy
    // ============================================================
    function createDatasetProxy(nid) {
        var handler = {
            get: function(target, prop) {
                if (typeof prop !== 'string') return undefined;
                var attrName = 'data-' + prop.replace(/([A-Z])/g, '-$1').toLowerCase();
                return B.getAttribute(nid, attrName);
            },
            set: function(target, prop, value) {
                if (typeof prop !== 'string') return true;
                var attrName = 'data-' + prop.replace(/([A-Z])/g, '-$1').toLowerCase();
                B.setAttribute(nid, attrName, String(value));
                return true;
            }
        };
        if (typeof Proxy !== 'undefined') {
            return new Proxy({}, handler);
        }
        var obj = {};
        var dataAttrs = B.getDataAttributes(nid);
        for (var i = 0; i < dataAttrs.length; i++) {
            var camelKey = dataAttrs[i][0].replace(/-([a-z])/g, function(_, c) { return c.toUpperCase(); });
            obj[camelKey] = dataAttrs[i][1];
        }
        return obj;
    }

    // ============================================================
    // Node / Element wrappers
    // ============================================================
    function wrapNode(nid) {
        if (nid < 0) return null;
        if (nodeCache[nid]) return nodeCache[nid];
        var isElem = B.isElement(nid);
        var obj = isElem ? wrapElement(nid) : wrapTextNode(nid);
        nodeCache[nid] = obj;
        return obj;
    }

    function wrapNodeList(ids) {
        var list = [];
        for (var i = 0; i < ids.length; i++) {
            list.push(wrapNode(ids[i]));
        }
        list.item = function(i) { return this[i] || null; };
        list.forEach = function(fn) { for (var i = 0; i < this.length; i++) fn(this[i], i, this); };
        return list;
    }

    function wrapTextNode(nid) {
        var obj = {};
        obj.__nid = nid;
        obj.nodeType = B.getNodeType(nid);
        obj.nodeName = B.getNodeName(nid);
        Object.defineProperty(obj, 'textContent', {
            get: function() { return B.getTextContent(nid); },
            set: function(v) { B.setTextContent(nid, String(v)); },
            configurable: true
        });
        Object.defineProperty(obj, 'nodeValue', {
            get: function() { return obj.nodeType === 3 ? B.getTextContent(nid) : null; },
            set: function(v) { if (obj.nodeType === 3) B.setTextContent(nid, String(v)); },
            configurable: true
        });
        Object.defineProperty(obj, 'parentNode', {
            get: function() { return wrapNode(B.getParent(nid)); },
            configurable: true
        });
        Object.defineProperty(obj, 'parentElement', {
            get: function() {
                var p = B.getParent(nid);
                return (p >= 0 && B.isElement(p)) ? wrapNode(p) : null;
            },
            configurable: true
        });
        Object.defineProperty(obj, 'nextSibling', {
            get: function() { return wrapNode(B.getNextSibling(nid)); },
            configurable: true
        });
        Object.defineProperty(obj, 'previousSibling', {
            get: function() { return wrapNode(B.getPrevSibling(nid)); },
            configurable: true
        });
        obj.addEventListener = function(type, fn, opts) { addEventListenerImpl(nid, type, fn, opts); };
        obj.removeEventListener = function(type, fn) { removeEventListenerImpl(nid, type, fn); };
        obj.dispatchEvent = function(evt) { return dispatchEventImpl(nid, evt); };
        obj.contains = function(other) {
            if (!other || other.__nid === undefined) return false;
            var cur = other.__nid;
            while (cur >= 0) {
                if (cur === nid) return true;
                cur = B.getParent(cur);
            }
            return false;
        };
        return obj;
    }

    function wrapElement(nid) {
        var obj = wrapTextNode(nid);
        obj.nodeType = 1;
        obj.tagName = B.getTagName(nid);
        obj.nodeName = obj.tagName;

        Object.defineProperty(obj, 'innerHTML', {
            get: function() { return B.getInnerHTML(nid); },
            set: function(v) {
                B.setInnerHTML(nid, v);
                invalidateChildCache(nid);
            },
            configurable: true
        });
        Object.defineProperty(obj, 'outerHTML', {
            get: function() { return B.getOuterHTML(nid); },
            configurable: true
        });
        Object.defineProperty(obj, 'className', {
            get: function() { return B.getAttribute(nid, 'class') || ''; },
            set: function(v) { B.setAttribute(nid, 'class', v); },
            configurable: true
        });
        Object.defineProperty(obj, 'id', {
            get: function() { return B.getAttribute(nid, 'id') || ''; },
            set: function(v) { B.setAttribute(nid, 'id', v); },
            configurable: true
        });
        Object.defineProperty(obj, 'children', {
            get: function() { return wrapNodeList(B.getChildElements(nid)); },
            configurable: true
        });
        Object.defineProperty(obj, 'childNodes', {
            get: function() { return wrapNodeList(B.getChildren(nid)); },
            configurable: true
        });
        Object.defineProperty(obj, 'firstChild', {
            get: function() { return wrapNode(B.getFirstChild(nid)); },
            configurable: true
        });
        Object.defineProperty(obj, 'lastChild', {
            get: function() { return wrapNode(B.getLastChild(nid)); },
            configurable: true
        });
        Object.defineProperty(obj, 'firstElementChild', {
            get: function() {
                var kids = B.getChildElements(nid);
                return kids.length > 0 ? wrapNode(kids[0]) : null;
            },
            configurable: true
        });
        Object.defineProperty(obj, 'lastElementChild', {
            get: function() {
                var kids = B.getChildElements(nid);
                return kids.length > 0 ? wrapNode(kids[kids.length - 1]) : null;
            },
            configurable: true
        });
        Object.defineProperty(obj, 'nextElementSibling', {
            get: function() { return wrapNode(B.getNextElementSibling(nid)); },
            configurable: true
        });
        Object.defineProperty(obj, 'previousElementSibling', {
            get: function() { return wrapNode(B.getPrevElementSibling(nid)); },
            configurable: true
        });
        Object.defineProperty(obj, 'childElementCount', {
            get: function() { return B.getChildElements(nid).length; },
            configurable: true
        });

        obj.getAttribute = function(name) { return B.getAttribute(nid, name); };
        obj.setAttribute = function(name, val) { B.setAttribute(nid, name, String(val)); };
        obj.removeAttribute = function(name) { B.removeAttribute(nid, name); };
        obj.hasAttribute = function(name) { return B.hasAttribute(nid, name); };
        obj.getAttributeNames = function() {
            var all = B.getAllAttributes(nid);
            return all.map(function(a) { return a[0]; });
        };

        obj.querySelector = function(sel) { return wrapNode(B.querySelector(nid, sel)); };
        obj.querySelectorAll = function(sel) { return wrapNodeList(B.querySelectorAll(nid, sel)); };
        obj.getElementsByTagName = function(tag) { return wrapNodeList(B.querySelectorAll(nid, tag)); };
        obj.getElementsByClassName = function(cls) {
            var parts = cls.split(/\s+/).filter(function(c) { return c; });
            var sel = parts.map(function(c) { return '.' + c; }).join('');
            return wrapNodeList(B.querySelectorAll(nid, sel || '.__none__'));
        };

        obj.appendChild = function(child) {
            if (child && child.__nid !== undefined) {
                B.appendChild(nid, child.__nid);
                delete nodeCache[child.__nid];
            }
            return child;
        };
        obj.removeChild = function(child) {
            if (child && child.__nid !== undefined) {
                B.removeChild(nid, child.__nid);
            }
            return child;
        };
        obj.insertBefore = function(newChild, refChild) {
            if (newChild && newChild.__nid !== undefined) {
                var refNid = (refChild && refChild.__nid !== undefined) ? refChild.__nid : -1;
                B.insertBefore(nid, newChild.__nid, refNid);
                delete nodeCache[newChild.__nid];
            }
            return newChild;
        };
        obj.replaceChild = function(newChild, oldChild) {
            if (newChild && oldChild && newChild.__nid !== undefined && oldChild.__nid !== undefined) {
                B.insertBefore(nid, newChild.__nid, oldChild.__nid);
                B.removeChild(nid, oldChild.__nid);
            }
            return oldChild;
        };
        obj.hasChildNodes = function() { return B.getFirstChild(nid) >= 0; };
        obj.normalize = function() {};

        obj.cloneNode = function(deep) {
            var newNid = B.cloneNode(nid, !!deep);
            return wrapNode(newNid);
        };

        obj.classList = {
            _nid: nid,
            _getClasses: function() { return (B.getAttribute(nid, 'class') || '').split(/\s+/).filter(function(x){return x;}); },
            contains: function(cls) { return this._getClasses().indexOf(cls) >= 0; },
            add: function() {
                var classes = this._getClasses();
                for (var i = 0; i < arguments.length; i++) {
                    if (classes.indexOf(arguments[i]) < 0) classes.push(arguments[i]);
                }
                B.setAttribute(nid, 'class', classes.join(' '));
            },
            remove: function() {
                var args = Array.prototype.slice.call(arguments);
                var classes = this._getClasses().filter(function(c) { return args.indexOf(c) < 0; });
                B.setAttribute(nid, 'class', classes.join(' '));
            },
            toggle: function(cls, force) {
                var has = this.contains(cls);
                if (force !== undefined) {
                    if (force) { this.add(cls); return true; }
                    else { this.remove(cls); return false; }
                }
                if (has) { this.remove(cls); return false; }
                else { this.add(cls); return true; }
            },
            replace: function(old, nw) {
                if (!this.contains(old)) return false;
                this.remove(old); this.add(nw); return true;
            },
            get length() { return this._getClasses().length; },
            item: function(i) { return this._getClasses()[i] || null; },
            toString: function() { return B.getAttribute(nid, 'class') || ''; },
            forEach: function(fn) { this._getClasses().forEach(fn); }
        };

        Object.defineProperty(obj, 'style', {
            get: function() { return createStyleProxy(nid); },
            configurable: true
        });

        Object.defineProperty(obj, 'dataset', {
            get: function() { return createDatasetProxy(nid); },
            configurable: true
        });

        obj.getBoundingClientRect = function() {
            return { top: 0, left: 0, right: 0, bottom: 0, width: 0, height: 0, x: 0, y: 0 };
        };
        obj.getClientRects = function() { return [obj.getBoundingClientRect()]; };
        Object.defineProperty(obj, 'offsetWidth', { get: function() { return 0; }, configurable: true });
        Object.defineProperty(obj, 'offsetHeight', { get: function() { return 0; }, configurable: true });
        Object.defineProperty(obj, 'offsetTop', { get: function() { return 0; }, configurable: true });
        Object.defineProperty(obj, 'offsetLeft', { get: function() { return 0; }, configurable: true });
        Object.defineProperty(obj, 'clientWidth', { get: function() { return 0; }, configurable: true });
        Object.defineProperty(obj, 'clientHeight', { get: function() { return 0; }, configurable: true });
        Object.defineProperty(obj, 'scrollWidth', { get: function() { return 0; }, configurable: true });
        Object.defineProperty(obj, 'scrollHeight', { get: function() { return 0; }, configurable: true });
        obj.scrollTop = 0;
        obj.scrollLeft = 0;
        obj.scrollIntoView = function() {};
        obj.focus = function() {};
        obj.blur = function() {};
        obj.click = function() { this.dispatchEvent(new EventImpl('click')); };

        obj.closest = function(sel) {
            var cur = nid;
            while (cur >= 0) {
                if (B.isElement(cur)) {
                    var p = B.getParent(cur);
                    if (p >= 0) {
                        var all = B.querySelectorAll(p, sel);
                        if (all.indexOf(cur) >= 0) return wrapNode(cur);
                    }
                }
                cur = B.getParent(cur);
            }
            return null;
        };
        obj.matches = function(sel) {
            var p = B.getParent(nid);
            if (p < 0) return false;
            return B.querySelectorAll(p, sel).indexOf(nid) >= 0;
        };

        obj.insertAdjacentHTML = function(position, html) {
            var temp = B.createElement('div');
            B.setInnerHTML(temp, html);
            var children = B.getChildren(temp);
            var parent = B.getParent(nid);
            for (var i = 0; i < children.length; i++) {
                if (position === 'beforebegin' && parent >= 0) {
                    B.insertBefore(parent, children[i], nid);
                } else if (position === 'afterbegin') {
                    var fc = B.getFirstChild(nid);
                    if (fc >= 0) B.insertBefore(nid, children[i], fc);
                    else B.appendChild(nid, children[i]);
                } else if (position === 'beforeend') {
                    B.appendChild(nid, children[i]);
                } else if (position === 'afterend' && parent >= 0) {
                    var ns = B.getNextSibling(nid);
                    if (ns >= 0) B.insertBefore(parent, children[i], ns);
                    else B.appendChild(parent, children[i]);
                }
            }
        };
        obj.insertAdjacentElement = function(position, elem) {
            if (!elem || elem.__nid === undefined) return null;
            var parent = B.getParent(nid);
            if (position === 'beforebegin' && parent >= 0) B.insertBefore(parent, elem.__nid, nid);
            else if (position === 'afterbegin') {
                var fc = B.getFirstChild(nid);
                if (fc >= 0) B.insertBefore(nid, elem.__nid, fc);
                else B.appendChild(nid, elem.__nid);
            }
            else if (position === 'beforeend') B.appendChild(nid, elem.__nid);
            else if (position === 'afterend' && parent >= 0) {
                var ns = B.getNextSibling(nid);
                if (ns >= 0) B.insertBefore(parent, elem.__nid, ns);
                else B.appendChild(parent, elem.__nid);
            }
            return elem;
        };

        obj.remove = function() {
            var parent = B.getParent(nid);
            if (parent >= 0) B.removeChild(parent, nid);
        };

        return obj;
    }

    function invalidateChildCache(nid) {
        var kids = B.getChildren(nid);
        for (var i = 0; i < kids.length; i++) {
            delete nodeCache[kids[i]];
        }
    }

    // ============================================================
    // Document object
    // ============================================================
    var docRoot = B.getDocumentNode();
    var document = {
        nodeType: 9,
        nodeName: '#document',
        __nid: docRoot,
        readyState: 'loading',
        characterSet: 'UTF-8',
        charset: 'UTF-8',
        contentType: 'text/html',
        compatMode: 'CSS1Compat',
        cookie: '',
        domain: '',
        referrer: '',
        hidden: false,
        visibilityState: 'visible',

        getElementById: function(id) { return wrapNode(B.getElementById(id)); },
        querySelector: function(sel) { return wrapNode(B.querySelector(docRoot, sel)); },
        querySelectorAll: function(sel) { return wrapNodeList(B.querySelectorAll(docRoot, sel)); },
        createElement: function(tag) { return wrapNode(B.createElement(tag)); },
        createTextNode: function(text) { return wrapNode(B.createTextNode(text)); },
        createComment: function(text) { return wrapNode(B.createTextNode('')); },
        createDocumentFragment: function() {
            var frag = { nodeType: 11, childNodes: [], __nid: -999 };
            frag.appendChild = function(c) { this.childNodes.push(c); return c; };
            frag.querySelectorAll = function() { return []; };
            frag.querySelector = function() { return null; };
            return frag;
        },
        getElementsByTagName: function(tag) { return wrapNodeList(B.getElementsByTagName(tag)); },
        getElementsByClassName: function(cls) { return wrapNodeList(B.getElementsByClassName(cls)); },
        getElementsByName: function() { return []; },
        createEvent: function(type) { return new EventImpl(type || ''); },
        createRange: function() {
            return { selectNodeContents: function() {}, createContextualFragment: function(h) {
                var tmp = B.createElement('div'); B.setInnerHTML(tmp, h);
                var f = document.createDocumentFragment();
                var kids = B.getChildren(tmp);
                for (var i = 0; i < kids.length; i++) f.appendChild(wrapNode(kids[i]));
                return f;
            }};
        },
        createTreeWalker: function() { return { nextNode: function() { return null; } }; },
        addEventListener: function(type, fn, opts) { addEventListenerImpl(DOC_EID, type, fn, opts); },
        removeEventListener: function(type, fn) { removeEventListenerImpl(DOC_EID, type, fn); },
        dispatchEvent: function(evt) { return dispatchEventImpl(DOC_EID, evt); },
        contains: function(other) {
            if (!other || other.__nid === undefined) return false;
            return true;
        },
        adoptNode: function(node) { return node; },
        importNode: function(node) { return node; },
        getSelection: function() { return { rangeCount: 0, getRangeAt: function() { return null; } }; },
        hasFocus: function() { return true; },
        currentScript: null,
        __setCurrentScript: function(nid) {
            document.currentScript = (nid !== null && nid !== undefined) ? wrapNode(nid) : null;
        }
    };

    Object.defineProperty(document, 'body', {
        get: function() { return wrapNode(B.body()); },
        configurable: true
    });
    Object.defineProperty(document, 'head', {
        get: function() { return wrapNode(B.head()); },
        configurable: true
    });
    Object.defineProperty(document, 'documentElement', {
        get: function() { return wrapNode(B.documentElement()); },
        configurable: true
    });
    Object.defineProperty(document, 'title', {
        get: function() { return B.title(); },
        set: function(val) {
            var head = document.head;
            if (!head) return;
            var titles = head.getElementsByTagName('title');
            var t;
            if (titles.length > 0) {
                t = titles[0];
            } else {
                t = document.createElement('title');
                head.appendChild(t);
            }
            t.textContent = String(val);
        },
        configurable: true
    });
    Object.defineProperty(document, 'activeElement', {
        get: function() { return document.body; },
        configurable: true
    });

    globalThis.document = document;

    // ============================================================
    // Window event delegation
    // ============================================================
    globalThis.addEventListener = function(type, fn, opts) { addEventListenerImpl(WIN_EID, type, fn, opts); };
    globalThis.removeEventListener = function(type, fn) { removeEventListenerImpl(WIN_EID, type, fn); };
    globalThis.dispatchEvent = function(evt) { return dispatchEventImpl(WIN_EID, evt); };

    // Common window stubs
    globalThis.getComputedStyle = function(el) {
        return {
            getPropertyValue: function(prop) {
                if (el && el.__nid !== undefined) return createStyleProxy(el.__nid).getPropertyValue(prop);
                return '';
            },
            display: 'block', visibility: 'visible', opacity: '1'
        };
    };
    globalThis.matchMedia = function(q) {
        return { matches: false, media: q, addEventListener: function(){}, removeEventListener: function(){}, addListener: function(){}, removeListener: function(){} };
    };
    globalThis.requestIdleCallback = function(cb) { return setTimeout(cb, 0); };
    globalThis.cancelIdleCallback = function(id) { clearTimeout(id); };
    globalThis.IntersectionObserver = function() {
        this.observe = function() {};
        this.unobserve = function() {};
        this.disconnect = function() {};
    };
    globalThis.ResizeObserver = function() {
        this.observe = function() {};
        this.unobserve = function() {};
        this.disconnect = function() {};
    };
    globalThis.XMLHttpRequest = function() {
        this.open = function() {};
        this.send = function() {};
        this.setRequestHeader = function() {};
        this.addEventListener = function() {};
        this.readyState = 0;
        this.status = 0;
        this.responseText = '';
    };
    globalThis.Image = function() {
        this.src = '';
        this.onload = null;
        this.onerror = null;
    };
    globalThis.innerWidth = 1920;
    globalThis.innerHeight = 1080;
    globalThis.outerWidth = 1920;
    globalThis.outerHeight = 1080;
    globalThis.devicePixelRatio = 1;
    globalThis.screenX = 0;
    globalThis.screenY = 0;
    globalThis.scrollX = 0;
    globalThis.scrollY = 0;
    globalThis.pageXOffset = 0;
    globalThis.pageYOffset = 0;
    globalThis.scrollTo = function() {};
    globalThis.scrollBy = function() {};
    globalThis.scroll = function() {};
    globalThis.open = function() { return null; };
    globalThis.close = function() {};
    globalThis.postMessage = function() {};
    globalThis.history = { pushState: function(){}, replaceState: function(){}, go: function(){}, back: function(){}, forward: function(){}, length: 1, state: null };
    globalThis.localStorage = globalThis.sessionStorage = (function() {
        var store = {};
        return {
            getItem: function(k) { return store.hasOwnProperty(k) ? store[k] : null; },
            setItem: function(k, v) { store[k] = String(v); },
            removeItem: function(k) { delete store[k]; },
            clear: function() { store = {}; },
            get length() { return Object.keys(store).length; },
            key: function(i) { return Object.keys(store)[i] || null; }
        };
    })();

    // ============================================================
    // URLSearchParams
    // ============================================================
    var URLSearchParamsImpl = (function() {
        function decode(s) {
            return decodeURIComponent(s.replace(/\+/g, ' '));
        }
        function encode(s) {
            return encodeURIComponent(s).replace(/%20/g, '+');
        }

        function USP(init) {
            this._entries = [];
            if (typeof init === 'string') {
                if (init.charAt(0) === '?') init = init.slice(1);
                if (init.length > 0) {
                    var pairs = init.split('&');
                    for (var i = 0; i < pairs.length; i++) {
                        var idx = pairs[i].indexOf('=');
                        if (idx === -1) {
                            this._entries.push([decode(pairs[i]), '']);
                        } else {
                            this._entries.push([decode(pairs[i].slice(0, idx)), decode(pairs[i].slice(idx + 1))]);
                        }
                    }
                }
            } else if (Array.isArray(init)) {
                for (var j = 0; j < init.length; j++) {
                    this._entries.push([String(init[j][0]), String(init[j][1])]);
                }
            } else if (init && typeof init === 'object') {
                var keys = Object.keys(init);
                for (var k = 0; k < keys.length; k++) {
                    this._entries.push([keys[k], String(init[keys[k]])]);
                }
            }
        }

        USP.prototype.append = function(name, value) {
            this._entries.push([String(name), String(value)]);
        };
        USP.prototype.delete = function(name) {
            this._entries = this._entries.filter(function(e) { return e[0] !== String(name); });
        };
        USP.prototype.get = function(name) {
            name = String(name);
            for (var i = 0; i < this._entries.length; i++) {
                if (this._entries[i][0] === name) return this._entries[i][1];
            }
            return null;
        };
        USP.prototype.getAll = function(name) {
            name = String(name);
            var result = [];
            for (var i = 0; i < this._entries.length; i++) {
                if (this._entries[i][0] === name) result.push(this._entries[i][1]);
            }
            return result;
        };
        USP.prototype.has = function(name) {
            name = String(name);
            for (var i = 0; i < this._entries.length; i++) {
                if (this._entries[i][0] === name) return true;
            }
            return false;
        };
        USP.prototype.set = function(name, value) {
            name = String(name); value = String(value);
            var found = false;
            var out = [];
            for (var i = 0; i < this._entries.length; i++) {
                if (this._entries[i][0] === name) {
                    if (!found) { out.push([name, value]); found = true; }
                } else {
                    out.push(this._entries[i]);
                }
            }
            if (!found) out.push([name, value]);
            this._entries = out;
        };
        USP.prototype.sort = function() {
            this._entries.sort(function(a, b) { return a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0; });
        };
        USP.prototype.toString = function() {
            return this._entries.map(function(e) { return encode(e[0]) + '=' + encode(e[1]); }).join('&');
        };
        USP.prototype.forEach = function(callback, thisArg) {
            for (var i = 0; i < this._entries.length; i++) {
                callback.call(thisArg, this._entries[i][1], this._entries[i][0], this);
            }
        };
        USP.prototype.keys = function() {
            var idx = 0, entries = this._entries;
            return { next: function() {
                return idx < entries.length ? { value: entries[idx++][0], done: false } : { done: true };
            } };
        };
        USP.prototype.values = function() {
            var idx = 0, entries = this._entries;
            return { next: function() {
                return idx < entries.length ? { value: entries[idx++][1], done: false } : { done: true };
            } };
        };
        USP.prototype.entries = function() {
            var idx = 0, entries = this._entries;
            return { next: function() {
                return idx < entries.length ? { value: [entries[idx][0], entries[idx++][1]], done: false } : { done: true };
            } };
        };
        if (typeof Symbol !== 'undefined' && Symbol.iterator) {
            USP.prototype[Symbol.iterator] = USP.prototype.entries;
        }
        Object.defineProperty(USP.prototype, 'size', {
            get: function() { return this._entries.length; }
        });

        return USP;
    })();
    globalThis.URLSearchParams = URLSearchParamsImpl;

    // ============================================================
    // URL
    // ============================================================
    var URLImpl = (function() {
        var URL_RE = /^([a-zA-Z][a-zA-Z0-9+\-.]*):\/\/(?:([^:@]*)(?::([^@]*))?@)?([^:\/?#]*)(?::(\d+))?(\/[^?#]*)?(\?[^#]*)?(#.*)?$/;
        var DEFAULT_PORTS = { 'http:': '80', 'https:': '443', 'ftp:': '21' };

        function parseURL(input, base) {
            input = String(input).trim();
            if (base !== undefined) {
                var b = (typeof base === 'string') ? parseURL(base) : base;
                if (!b) throw new TypeError("Invalid base URL");
                if (!/^[a-zA-Z][a-zA-Z0-9+\-.]*:/.test(input)) {
                    if (input.charAt(0) === '/') {
                        if (input.charAt(1) === '/') {
                            input = b.protocol + input;
                        } else {
                            input = b.protocol + '//' +
                                (b.username ? b.username + (b.password ? ':' + b.password : '') + '@' : '') +
                                b.host + input;
                        }
                    } else {
                        var basePath = b.pathname;
                        var lastSlash = basePath.lastIndexOf('/');
                        var dir = lastSlash >= 0 ? basePath.slice(0, lastSlash + 1) : '/';
                        input = b.protocol + '//' +
                            (b.username ? b.username + (b.password ? ':' + b.password : '') + '@' : '') +
                            b.host + dir + input;
                    }
                }
            }

            var m = URL_RE.exec(input);
            if (!m) throw new TypeError("Invalid URL: " + input);
            return {
                protocol: m[1].toLowerCase() + ':',
                username: m[2] ? decodeURIComponent(m[2]) : '',
                password: m[3] ? decodeURIComponent(m[3]) : '',
                hostname: m[4] || '',
                port: m[5] || '',
                pathname: m[6] || '/',
                search: m[7] || '',
                hash: m[8] || ''
            };
        }

        function buildHost(parts) {
            if (!parts.hostname) return '';
            return parts.port ? parts.hostname + ':' + parts.port : parts.hostname;
        }

        function buildHref(parts) {
            var userinfo = '';
            if (parts.username) {
                userinfo = encodeURIComponent(parts.username);
                if (parts.password) userinfo += ':' + encodeURIComponent(parts.password);
                userinfo += '@';
            }
            return parts.protocol + '//' + userinfo + buildHost(parts) + parts.pathname + parts.search + parts.hash;
        }

        function buildOrigin(parts) {
            return parts.protocol + '//' + buildHost(parts);
        }

        function URLCtor(url, base) {
            var p = parseURL(url, base);
            this._parts = p;
            this._searchParams = null;
        }

        var props = ['protocol', 'username', 'password', 'hostname', 'port', 'pathname', 'search', 'hash'];
        for (var i = 0; i < props.length; i++) {
            (function(prop) {
                Object.defineProperty(URLCtor.prototype, prop, {
                    get: function() { return this._parts[prop]; },
                    set: function(v) {
                        this._parts[prop] = String(v);
                        if (prop === 'search' && this._searchParams) {
                            this._searchParams = new URLSearchParamsImpl(this._parts.search);
                        }
                    }
                });
            })(props[i]);
        }

        Object.defineProperty(URLCtor.prototype, 'host', {
            get: function() { return buildHost(this._parts); },
            set: function(v) {
                var idx = v.indexOf(':');
                if (idx === -1) {
                    this._parts.hostname = v;
                    this._parts.port = '';
                } else {
                    this._parts.hostname = v.slice(0, idx);
                    this._parts.port = v.slice(idx + 1);
                }
            }
        });

        Object.defineProperty(URLCtor.prototype, 'href', {
            get: function() { return buildHref(this._parts); },
            set: function(v) { this._parts = parseURL(v); this._searchParams = null; }
        });

        Object.defineProperty(URLCtor.prototype, 'origin', {
            get: function() { return buildOrigin(this._parts); }
        });

        Object.defineProperty(URLCtor.prototype, 'searchParams', {
            get: function() {
                if (!this._searchParams) {
                    this._searchParams = new URLSearchParamsImpl(this._parts.search);
                }
                return this._searchParams;
            }
        });

        URLCtor.prototype.toString = function() { return this.href; };
        URLCtor.prototype.toJSON = function() { return this.href; };

        return URLCtor;
    })();
    globalThis.URL = URLImpl;

    // DOMParser
    globalThis.DOMParser = function() {};
    globalThis.DOMParser.prototype.parseFromString = function(str, type) {
        return document;
    };

    // ============================================================
    // Trigger DOMContentLoaded and load
    // ============================================================
    globalThis.__fireDOMContentLoaded = function() {
        document.readyState = 'interactive';
        var evt = new EventImpl('DOMContentLoaded', { bubbles: true });
        dispatchEventImpl(DOC_EID, evt);
    };

    globalThis.__fireLoad = function() {
        document.readyState = 'complete';
        var evt = new EventImpl('load');
        dispatchEventImpl(WIN_EID, evt);
        if (typeof globalThis.onload === 'function') {
            try { globalThis.onload(evt); } catch(e) { console.error(e); }
        }
    };
})();
