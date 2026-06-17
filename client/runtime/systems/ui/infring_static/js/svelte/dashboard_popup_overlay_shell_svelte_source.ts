const COMPONENT_TAG = 'infring-dashboard-popup-overlay-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-dashboard-popup-overlay-shell', shadow: 'none' }} />
<script>
  import { onMount, onDestroy } from 'svelte';

  let timer = 0;
  let popup = emptyPopup();

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

  function popupService() {
    const services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
    return services && services.popup ? services.popup : null;
  }

  function appStoreService() {
    const services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
    return services && services.appStore ? services.appStore : null;
  }

  function appStore() {
    if (typeof window === 'undefined') return null;
    const service = appStoreService();
    return service && typeof service.current === 'function'
      ? service.current()
      : (window.InfringApp && typeof window.InfringApp === 'object' ? window.InfringApp : null);
  }

  function appStoreCandidates() {
    if (typeof window === 'undefined') return [];
    const candidates = [];
    const push = function(value) {
      if (!value || typeof value !== 'object') return;
      if (candidates.indexOf(value) >= 0) return;
      candidates.push(value);
    };
    push(appStore());
    push(window.InfringApp);
    try {
      if (window.Alpine && typeof window.Alpine.store === 'function') push(window.Alpine.store('app'));
    } catch (_) {}
    return candidates;
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
    const rect = element.getBoundingClientRect();
    const side = text((origin && origin.side) || preferredSide || 'bottom').toLowerCase();
    let left = Math.round(Number(rect.left || 0));
    let top = Math.round(Number(rect.bottom || 0));
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
      left,
      top,
      side: side === 'top' || side === 'left' || side === 'right' ? side : 'bottom',
      inline_away: (origin && origin.inline_away) || 'right',
      block_away: (origin && origin.block_away) || 'bottom'
    }));
  }

  function recoverPopupOrigin(service, app, origin) {
    if (!app || !origin || !origin.active || origin.ready || !text(origin.title)) return origin;
    const rawPopup = app.dashboardPopup && typeof app.dashboardPopup === 'object' ? app.dashboardPopup : {};
    const popupId = text(rawPopup.id);
    let element = null;
    if (popupId.indexOf('bottom-dock:') === 0) {
      const dockId = safeAttr(popupId.slice('bottom-dock:'.length));
      element = document.querySelector('.bottom-dock [data-dock-id="' + dockId + '"], .bottom-dock .dock-tile-slot[data-dock-slot-id="' + dockId + '"]');
    } else if (popupId.indexOf('delegated-bottom_dock:') === 0) {
      const dockId = safeAttr(popupId.slice('delegated-bottom_dock:'.length));
      element = document.querySelector('.bottom-dock [data-dock-id="' + dockId + '"], .bottom-dock [aria-label="' + dockId + '"]');
    }
    if (!element) {
      const label = safeAttr(origin.title);
      if (label) element = document.querySelector('.bottom-dock [aria-label="' + label + '"]');
    }
    return elementAnchorOrigin(service, origin, element, origin.side || 'top');
  }

  function bottomDockOrigin(service, app) {
    if (!app) return emptyPopup();
    const label = text(app.bottomDockPreviewText);
    const left = Math.round(Number(app.bottomDockPreviewX || 0));
    const top = Math.round(Number(app.bottomDockPreviewY || 0));
    if (!app.bottomDockPreviewVisible || !label) return serviceOrigin(service);
    const side = typeof app.bottomDockOpenSide === 'function' ? app.bottomDockOpenSide() : 'top';
    return serviceOrigin(service, {
      source: 'bottom_dock',
      active: true,
      ready: left > 0 && top > 0,
      side,
      inline_away: 'center',
      block_away: 'center',
      left,
      top,
      compact: false,
      title: label
    });
  }

  function recoverBottomDockOrigin(service, app, origin) {
    if (!app || !origin || !origin.active || origin.ready || !text(origin.title)) return origin;
    const dockId = safeAttr(app.bottomDockHoverId || '');
    const label = safeAttr(origin.title);
    const element = dockId
      ? document.querySelector('.bottom-dock [data-dock-id="' + dockId + '"], .bottom-dock .dock-tile-slot[data-dock-slot-id="' + dockId + '"]')
      : (label ? document.querySelector('.bottom-dock [aria-label="' + label + '"]') : null);
    return elementAnchorOrigin(service, origin, element, origin.side || 'top');
  }

  function activePopupOrigin() {
    const service = popupService();
    const apps = appStoreCandidates();
    for (const app of apps) {
      const shared = recoverPopupOrigin(service, app, stateOrigin(service, app));
      if (shared.active && shared.ready) return shared;
    }
    for (const app of apps) {
      const dock = recoverBottomDockOrigin(service, app, bottomDockOrigin(service, app));
      if (dock.active && dock.ready) return dock;
    }
    return serviceOrigin(service);
  }

  function classString(map) {
    const result = [];
    for (const key in map || {}) {
      if (Object.prototype.hasOwnProperty.call(map, key) && map[key]) result.push(key);
    }
    return result.join(' ');
  }

  function overlayClasses() {
    const service = popupService();
    const map = service && typeof service.overlayClass === 'function'
      ? service.overlayClass(popup, 'fogged-glass')
      : { 'fogged-glass': true, 'is-visible': !!(popup.active && popup.ready && popup.title) };
    return 'dashboard-popup-surface dashboard-preview-surface dashboard-popup-overlay ' + classString(map);
  }

  function overlayStyle() {
    const service = popupService();
    if (service && typeof service.overlayStyle === 'function') return service.overlayStyle(popup);
    if (!popup.active || !popup.ready) return 'left:-9999px;top:-9999px;';
    return 'left:' + Math.round(Number(popup.left || 0)) + 'px;top:' + Math.round(Number(popup.top || 0)) + 'px;';
  }

  function refresh() {
    popup = activePopupOrigin();
  }

  onMount(function() {
    refresh();
    timer = window.setInterval(refresh, 80);
    window.addEventListener('resize', refresh, { passive: true });
    window.addEventListener('scroll', refresh, true);
  });

  onDestroy(function() {
    if (timer) window.clearInterval(timer);
    window.removeEventListener('resize', refresh);
    window.removeEventListener('scroll', refresh, true);
  });

</script>

<div class={overlayClasses()} style={overlayStyle()} aria-hidden="true">
  <span class="dashboard-popup-title">{popup.title || ''}</span>
  {#if text(popup.body).length > 0}
    <span class:preview-unread={!!popup.unread} class="dashboard-popup-body">{popup.body}</span>
  {/if}
</div>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
