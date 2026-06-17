(function() {
  'use strict';

  var TAG = 'infring-dashboard-popup-overlay-shell';

  function text(value) {
    return String(value == null ? '' : value).trim();
  }

  function emptyPopup() {
    return {
      source: '',
      active: false,
      ready: false,
      side: 'top',
      inline_away: 'right',
      block_away: 'bottom',
      left: 0,
      top: 0,
      compact: false,
      title: '',
      body: '',
      meta_origin: '',
      meta_time: '',
      unread: false
    };
  }

  function services() {
    return typeof window !== 'undefined' && window.InfringSharedShellServices
      ? window.InfringSharedShellServices
      : null;
  }

  function popupService() {
    var svc = services();
    return svc && svc.popup ? svc.popup : null;
  }

  function appStoreService() {
    var svc = services();
    return svc && svc.appStore ? svc.appStore : null;
  }

  function appStore() {
    if (typeof window === 'undefined') return null;
    var service = appStoreService();
    if (service && typeof service.current === 'function') {
      var current = service.current();
      if (current && typeof current === 'object') return current;
    }
    return window.InfringApp && typeof window.InfringApp === 'object' ? window.InfringApp : null;
  }

  function appStoreCandidates() {
    if (typeof window === 'undefined') return [];
    var result = [];
    function push(value) {
      if (!value || typeof value !== 'object') return;
      if (result.indexOf(value) >= 0) return;
      result.push(value);
    }
    push(appStore());
    push(window.InfringApp);
    try {
      if (window.Alpine && typeof window.Alpine.store === 'function') push(window.Alpine.store('app'));
    } catch (_) {}
    return result;
  }

  function serviceOrigin(service, overrides) {
    return service && typeof service.origin === 'function'
      ? service.origin(overrides)
      : Object.assign(emptyPopup(), overrides || {});
  }

  function stateOrigin(service, app) {
    if (!service || !app || typeof service.stateOrigin !== 'function') return emptyPopup();
    return service.stateOrigin(app.dashboardPopup);
  }

  function safeAttr(value) {
    return text(value).replace(/\\/g, '\\\\').replace(/"/g, '\\"');
  }

  function elementAnchorOrigin(service, origin, element, preferredSide) {
    if (!element || typeof element.getBoundingClientRect !== 'function') return origin || emptyPopup();
    var rect = element.getBoundingClientRect();
    var side = text((origin && origin.side) || preferredSide || 'bottom').toLowerCase();
    var left = Math.round(Number(rect.left || 0));
    var top = Math.round(Number(rect.bottom || 0));
    if (side === 'top') {
      top = Math.round(Number(rect.top || 0));
    } else if (side === 'right') {
      left = Math.round(Number(rect.right || 0));
      top = Math.round(Number(rect.top || 0));
    } else if (side === 'left') {
      top = Math.round(Number(rect.top || 0));
    }
    return serviceOrigin(service, Object.assign({}, origin || {}, {
      active: true,
      ready: left > 0 && top > 0,
      left: left,
      top: top,
      side: side === 'top' || side === 'left' || side === 'right' ? side : 'bottom',
      inline_away: origin && origin.inline_away || 'right',
      block_away: origin && origin.block_away || 'bottom'
    }));
  }

  function dockElementFromIdOrTitle(rawId, title) {
    var id = safeAttr(rawId || '');
    var label = safeAttr(title || '');
    if (id) {
      var byId = document.querySelector('.bottom-dock [data-dock-id="' + id + '"], .bottom-dock .dock-tile-slot[data-dock-slot-id="' + id + '"]');
      if (byId) return byId;
    }
    if (label) return document.querySelector('.bottom-dock [aria-label="' + label + '"]');
    return null;
  }

  function recoverPopupOrigin(service, app, origin) {
    if (!app || !origin || !origin.active || origin.ready || !text(origin.title)) return origin;
    var rawPopup = app.dashboardPopup && typeof app.dashboardPopup === 'object' ? app.dashboardPopup : {};
    var popupId = text(rawPopup.id);
    var element = null;
    if (popupId.indexOf('bottom-dock:') === 0) {
      element = dockElementFromIdOrTitle(popupId.slice('bottom-dock:'.length), origin.title);
    } else if (popupId.indexOf('delegated-bottom_dock:') === 0) {
      element = dockElementFromIdOrTitle(popupId.slice('delegated-bottom_dock:'.length), origin.title);
    }
    if (!element) element = dockElementFromIdOrTitle('', origin.title);
    return elementAnchorOrigin(service, origin, element, origin.side || 'top');
  }

  function bottomDockOrigin(service, app) {
    if (!app) return emptyPopup();
    var label = text(app.bottomDockPreviewText);
    var left = Math.round(Number(app.bottomDockPreviewX || 0));
    var top = Math.round(Number(app.bottomDockPreviewY || 0));
    if (!app.bottomDockPreviewVisible || !label) return serviceOrigin(service);
    var side = typeof app.bottomDockOpenSide === 'function' ? app.bottomDockOpenSide() : 'top';
    return serviceOrigin(service, {
      source: 'bottom_dock',
      active: true,
      ready: left > 0 && top > 0,
      side: side,
      inline_away: 'center',
      block_away: 'center',
      left: left,
      top: top,
      compact: false,
      title: label
    });
  }

  function recoverBottomDockOrigin(service, app, origin) {
    if (!app || !origin || !origin.active || origin.ready || !text(origin.title)) return origin;
    var element = dockElementFromIdOrTitle(app.bottomDockHoverId || '', origin.title);
    return elementAnchorOrigin(service, origin, element, origin.side || 'top');
  }

  function activePopupOrigin() {
    var service = popupService();
    var apps = appStoreCandidates();
    for (var i = 0; i < apps.length; i += 1) {
      var shared = recoverPopupOrigin(service, apps[i], stateOrigin(service, apps[i]));
      if (shared.active && shared.ready) return shared;
    }
    for (var j = 0; j < apps.length; j += 1) {
      var dock = recoverBottomDockOrigin(service, apps[j], bottomDockOrigin(service, apps[j]));
      if (dock.active && dock.ready) return dock;
    }
    return serviceOrigin(service);
  }

  function classString(map) {
    var result = [];
    for (var key in map || {}) {
      if (Object.prototype.hasOwnProperty.call(map, key) && map[key]) result.push(key);
    }
    return result.join(' ');
  }

  function overlayClasses(popup) {
    var service = popupService();
    var map = service && typeof service.overlayClass === 'function'
      ? service.overlayClass(popup, 'fogged-glass')
      : { 'fogged-glass': true, 'is-visible': !!(popup.active && popup.ready && popup.title) };
    return 'dashboard-popup-surface dashboard-preview-surface dashboard-popup-overlay ' + classString(map);
  }

  function overlayStyle(popup) {
    var service = popupService();
    if (service && typeof service.overlayStyle === 'function') return service.overlayStyle(popup);
    if (!popup.active || !popup.ready) return 'left:-9999px;top:-9999px;';
    return 'left:' + Math.round(Number(popup.left || 0)) + 'px;top:' + Math.round(Number(popup.top || 0)) + 'px;';
  }

  function appendText(parent, tag, className, value) {
    var node = document.createElement(tag || 'span');
    if (className) node.className = className;
    node.textContent = value || '';
    parent.appendChild(node);
    return node;
  }

  class DashboardPopupOverlayShell extends HTMLElement {
    connectedCallback() {
      if (!this._surface) {
        this._surface = document.createElement('div');
        this.appendChild(this._surface);
      }
      this.refresh = this.refresh.bind(this);
      this.refresh();
      this._timer = window.setInterval(this.refresh, 80);
      window.addEventListener('resize', this.refresh, { passive: true });
      window.addEventListener('scroll', this.refresh, true);
    }

    disconnectedCallback() {
      if (this._timer) window.clearInterval(this._timer);
      this._timer = 0;
      window.removeEventListener('resize', this.refresh);
      window.removeEventListener('scroll', this.refresh, true);
    }

    refresh() {
      var popup = activePopupOrigin();
      var surface = this._surface;
      if (!surface) return;
      surface.className = overlayClasses(popup);
      surface.setAttribute('style', overlayStyle(popup));
      surface.setAttribute('aria-hidden', 'true');
      while (surface.firstChild) surface.removeChild(surface.firstChild);
      appendText(surface, 'span', 'dashboard-popup-title', popup.title || '');
      if (text(popup.body)) {
        var body = appendText(surface, 'span', 'dashboard-popup-body', popup.body);
        if (popup.unread) body.classList.add('preview-unread');
      }
    }
  }

  if (typeof customElements !== 'undefined' && !customElements.get(TAG)) {
    customElements.define(TAG, DashboardPopupOverlayShell);
  }
})();
