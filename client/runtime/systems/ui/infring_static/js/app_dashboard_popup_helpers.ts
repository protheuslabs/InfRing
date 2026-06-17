// Dashboard popup projection helpers keep popup placement fallback logic out of app.ts.
function infringNormalizeSidebarPopupText(page, rawText) {
  var text = String(rawText || '').trim();
  if (!text) return '';
  if (page.isSidebarPopupPlaceholderText(text)) return '';
  return text;
}

function infringIsSidebarPopupPlaceholderText(text) {
  var normalized = String(text || '').trim().toLowerCase();
  return normalized === 'no messages yet'
    || normalized === 'system events and terminal output'
    || normalized === 'no matching text'
    || normalized === 'agent';
}

function infringSidebarPopupMetaOrigin(preview, fallbackLabel) {
  var role = String(preview && preview.role || '').trim().toLowerCase();
  if (role === 'user') return 'User';
  if (role === 'assistant' || role === 'agent') return 'Agent';
  if (role) return role.charAt(0).toUpperCase() + role.slice(1);
  return String(fallbackLabel || 'Sidebar').trim() || 'Sidebar';
}

function infringHideDashboardPopupBySource(page, source) {
  var expected = String(source || '').trim();
  if (!expected) return;
  var popup = page.dashboardPopup || {};
  var currentSource = String(popup.source || '').trim();
  if (currentSource !== expected) return;
  page.hideDashboardPopup(String(popup.id || '').trim());
}

function infringShowCollapsedSidebarAgentPopup(page, agent, ev) {
  if (!page.sidebarCollapsed || !agent) {
    page.hideDashboardPopupBySource('sidebar');
    return;
  }
  var rawId = String(agent.id || '').trim();
  var rawIdLower = rawId.toLowerCase();
  var isSystemThread = (typeof page.isSystemSidebarThread === 'function')
    ? page.isSystemSidebarThread(agent)
    : (agent.is_system_thread === true || rawIdLower === 'system');
  if (isSystemThread || rawIdLower === 'settings') {
    page.hideDashboardPopupBySource('sidebar');
    return;
  }
  var preview = page.chatSidebarPreview(agent) || {};
  var previewText = page.normalizeSidebarPopupText(preview.text || '');
  var title = String(agent.name || rawId).trim();
  if (!rawId || !title || !previewText) {
    page.hideDashboardPopupBySource('sidebar');
    return;
  }
  page.showDashboardPopup('sidebar-agent:' + rawId, title, ev, {
    source: 'sidebar',
    side: 'right',
    body: previewText,
    meta_origin: page.sidebarPopupMetaOrigin(preview, 'Agent'),
    meta_time: typeof page.formatChatSidebarTime === 'function'
      ? String(page.formatChatSidebarTime(preview.ts) || '').trim()
      : '',
    unread: !!preview.unread_response
  });
}

function infringShowCollapsedSidebarNavPopup(page, label, ev) {
  if (!page.sidebarCollapsed) {
    page.hideDashboardPopupBySource('sidebar');
    return;
  }
  var navLabel = String(label || '').trim();
  var navLabelLower = navLabel.toLowerCase();
  if (!navLabel || navLabelLower === 'system' || navLabelLower === 'settings') {
    page.hideDashboardPopupBySource('sidebar');
    return;
  }
  page.showDashboardPopup('sidebar-nav:' + navLabelLower.replace(/[^a-z0-9_-]+/g, '-'), navLabel, ev, {
    source: 'sidebar',
    side: 'right',
    meta_origin: 'Sidebar'
  });
}

function infringDashboardPopupService() {
  var root = typeof window !== 'undefined' ? window : {};
  var services = root && root.InfringSharedShellServices;
  return services && services.popup ? services.popup : null;
}

function infringClearDashboardPopupState(page) {
  var service = page.dashboardPopupService();
  page.dashboardPopup = service && typeof service.emptyState === 'function'
    ? service.emptyState()
    : {
      id: '',
      active: false,
      source: '',
      title: '',
      body: '',
      meta_origin: '',
      meta_time: '',
      unread: false,
      left: 0,
      top: 0,
      side: 'bottom',
      inline_away: 'right',
      block_away: 'bottom',
      compact: false
    };
}

function infringNormalizeDashboardPopupSide(page, sideValue, fallbackSide) {
  var service = page.dashboardPopupService();
  if (service && typeof service.normalizeSide === 'function') {
    return service.normalizeSide(sideValue, fallbackSide);
  }
  var fallback = String(fallbackSide || 'bottom').trim().toLowerCase();
  if (fallback !== 'top' && fallback !== 'left' && fallback !== 'right') fallback = 'bottom';
  var side = String(sideValue || fallback).trim().toLowerCase();
  if (side !== 'top' && side !== 'left' && side !== 'right') side = 'bottom';
  return side;
}

function infringDashboardOppositeSide(page, sideValue) {
  var service = page.dashboardPopupService();
  if (service && typeof service.oppositeSide === 'function') {
    return service.oppositeSide(sideValue);
  }
  var side = page.normalizeDashboardPopupSide(sideValue, 'bottom');
  if (side === 'top') return 'bottom';
  if (side === 'left') return 'right';
  if (side === 'right') return 'left';
  return 'top';
}

function infringDashboardPopupWallAffinity(rect) {
  if (!rect || typeof window === 'undefined') return null;
  var viewportWidth = Number(window.innerWidth || 0);
  var viewportHeight = Number(window.innerHeight || 0);
  if (!Number.isFinite(viewportWidth) || viewportWidth <= 0) viewportWidth = 1;
  if (!Number.isFinite(viewportHeight) || viewportHeight <= 0) viewportHeight = 1;
  var left = Number(rect.left || 0);
  var right = Number(rect.right || 0);
  var top = Number(rect.top || 0);
  var bottom = Number(rect.bottom || 0);
  if (!Number.isFinite(left) || !Number.isFinite(right) || !Number.isFinite(top) || !Number.isFinite(bottom)) {
    return null;
  }
  var width = Math.max(1, Math.abs(right - left));
  var height = Math.max(1, Math.abs(bottom - top));
  var distanceToLeft = Math.max(0, left);
  var distanceToRight = Math.max(0, viewportWidth - right);
  var distanceToTop = Math.max(0, top);
  var distanceToBottom = Math.max(0, viewportHeight - bottom);
  var proximityScore = function(distance) {
    var normalized = Number(distance || 0);
    if (!Number.isFinite(normalized) || normalized < 0) normalized = 0;
    return 1 / (1 + normalized);
  };
  return {
    scores: {
      top: width * proximityScore(distanceToTop),
      bottom: width * proximityScore(distanceToBottom),
      left: height * proximityScore(distanceToLeft),
      right: height * proximityScore(distanceToRight)
    },
    distances: {
      top: distanceToTop,
      bottom: distanceToBottom,
      left: distanceToLeft,
      right: distanceToRight
    }
  };
}

function infringDashboardPopupWallAnchorNode(node) {
  if (!node || typeof node.closest !== 'function') return null;
  try {
    return node.closest(
      '[data-popup-wall-anchor], .global-taskbar, .sidebar, .bottom-dock, .doc-window, .chat-window'
    );
  } catch(_) {
    return null;
  }
}

function infringDashboardPopupWallRectForNode(page, node) {
  var anchor = page.dashboardPopupWallAnchorNode(node);
  if (!anchor || typeof anchor.getBoundingClientRect !== 'function') return null;
  try {
    return anchor.getBoundingClientRect();
  } catch(_) {
    return null;
  }
}

function infringDashboardPopupUsableAnchorRect(node) {
  if (!node || typeof node.getBoundingClientRect !== 'function') return null;
  var rect = null;
  try {
    rect = node.getBoundingClientRect();
  } catch(_) {
    rect = null;
  }
  var width = rect ? Math.abs(Number(rect.right || 0) - Number(rect.left || 0)) : 0;
  var height = rect ? Math.abs(Number(rect.bottom || 0) - Number(rect.top || 0)) : 0;
  if (rect && width > 0 && height > 0) return rect;
  if (node && typeof node.closest === 'function') {
    try {
      var fallback = node.closest('[data-popup-origin-anchor], .composer-menu-pill, .composer-input-pill, .taskbar-text-menu-anchor, .taskbar-hero-menu-anchor, .notif-wrap');
      if (fallback && fallback !== node && typeof fallback.getBoundingClientRect === 'function') {
        rect = fallback.getBoundingClientRect();
        width = rect ? Math.abs(Number(rect.right || 0) - Number(rect.left || 0)) : 0;
        height = rect ? Math.abs(Number(rect.bottom || 0) - Number(rect.top || 0)) : 0;
        if (rect && width > 0 && height > 0) return rect;
      }
    } catch(_) {}
  }
  return null;
}

function infringDashboardPopupSideAwayFromNearestWall(page, rect, fallbackSide) {
  var service = page.dashboardPopupService();
  if (service && typeof service.sideAwayFromNearestWall === 'function') {
    return service.sideAwayFromNearestWall(rect, fallbackSide);
  }
  var fallback = page.normalizeDashboardPopupSide('', fallbackSide);
  var affinity = page.dashboardPopupWallAffinity(rect);
  if (!affinity || !affinity.scores || !affinity.distances) return fallback;
  var scores = affinity.scores;
  var distances = affinity.distances;
  var walls = ['top', 'bottom', 'left', 'right'];
  var fallbackWall = page.dashboardOppositeSide(fallback);
  var winner = walls[0];
  var winnerScore = Number(scores[winner] || 0);
  var epsilon = 0.000001;
  for (var i = 1; i < walls.length; i += 1) {
    var wall = walls[i];
    var score = Number(scores[wall] || 0);
    if (score > winnerScore + epsilon) {
      winner = wall;
      winnerScore = score;
      continue;
    }
    if (Math.abs(score - winnerScore) <= epsilon) {
      if (wall === fallbackWall && winner !== fallbackWall) {
        winner = wall;
        winnerScore = score;
        continue;
      }
      var wallDistance = Number(distances[wall] || 0);
      var winnerDistance = Number(distances[winner] || 0);
      if (wallDistance < winnerDistance) {
        winner = wall;
        winnerScore = score;
      }
    }
  }
  return page.dashboardOppositeSide(winner);
}

function infringDashboardPopupHorizontalAwayFromNearestWall(page, rect, fallbackSide) {
  var service = page.dashboardPopupService();
  if (service && typeof service.horizontalAwayFromNearestWall === 'function') {
    return service.horizontalAwayFromNearestWall(rect, fallbackSide);
  }
  var fallback = String(fallbackSide || 'right').trim().toLowerCase();
  if (fallback !== 'left') fallback = 'right';
  var affinity = page.dashboardPopupWallAffinity(rect);
  if (!affinity || !affinity.distances) return fallback;
  var distances = affinity.distances;
  var nearest = Number(distances.left || 0) <= Number(distances.right || 0)
    ? 'left'
    : 'right';
  return nearest === 'left' ? 'right' : 'left';
}

function infringDashboardPopupVerticalAwayFromNearestWall(page, rect, fallbackSide) {
  var service = page.dashboardPopupService();
  if (service && typeof service.verticalAwayFromNearestWall === 'function') {
    return service.verticalAwayFromNearestWall(rect, fallbackSide);
  }
  var fallback = String(fallbackSide || 'bottom').trim().toLowerCase();
  if (fallback !== 'top') fallback = 'bottom';
  var affinity = page.dashboardPopupWallAffinity(rect);
  if (!affinity || !affinity.distances) return fallback;
  var distances = affinity.distances;
  var nearest = Number(distances.top || 0) <= Number(distances.bottom || 0)
    ? 'top'
    : 'bottom';
  return nearest === 'top' ? 'bottom' : 'top';
}

function infringDashboardPopupAxisAwareSideAway(page, rect, fallbackSide) {
  var service = page.dashboardPopupService();
  if (service && typeof service.axisAwareSideAway === 'function') {
    return service.axisAwareSideAway(rect, fallbackSide);
  }
  var fallback = page.normalizeDashboardPopupSide('', fallbackSide || 'bottom');
  if (fallback === 'left' || fallback === 'right') {
    return page.dashboardPopupHorizontalAwayFromNearestWall(rect, fallback);
  }
  return page.dashboardPopupVerticalAwayFromNearestWall(rect, fallback);
}

function infringTaskbarAnchoredDropdownClass(page, anchorNode, fallbackSide, layoutKey) {
  var fallback = page.normalizeDashboardPopupSide('', fallbackSide || 'bottom');
  var anchorRect = anchorNode && typeof anchorNode.getBoundingClientRect === 'function'
    ? page.dashboardPopupUsableAnchorRect(anchorNode)
    : null;
  var service = page.dashboardPopupService();
  if (service && typeof service.dropdownClass === 'function') {
    return service.dropdownClass(anchorRect, fallback, layoutKey);
  }
  String(layoutKey == null ? '' : layoutKey);
  var side = fallback;
  var inlineAway = 'right';
  var blockAway = 'bottom';
  if (anchorRect) {
    side = page.dashboardPopupAxisAwareSideAway(anchorRect, fallback);
    inlineAway = page.dashboardPopupHorizontalAwayFromNearestWall(anchorRect, 'right');
    blockAway = page.dashboardPopupVerticalAwayFromNearestWall(anchorRect, 'bottom');
  }
  return {
    'taskbar-anchored-dropdown': true,
    'is-side-top': side === 'top',
    'is-side-bottom': side === 'bottom',
    'is-side-left': side === 'left',
    'is-side-right': side === 'right',
    'is-inline-away-left': inlineAway === 'left',
    'is-inline-away-right': inlineAway === 'right',
    'is-block-away-top': blockAway === 'top',
    'is-block-away-bottom': blockAway === 'bottom'
  };
}

function infringDashboardPopupAnchorPoint(page, ev, sideOverride) {
  var preferredSide = page.normalizeDashboardPopupSide(sideOverride, 'bottom');
  var node = ev && ev.currentTarget ? ev.currentTarget : null;
  if (!node && ev && ev.target && typeof ev.target.closest === 'function') {
    try {
      node = ev.target.closest('button,[role="button"],.taskbar-reorder-item');
    } catch(_) {
      node = null;
    }
  }
  if (!node || typeof node.getBoundingClientRect !== 'function') {
    return { left: 0, top: 0, side: preferredSide, inline_away: 'right', block_away: 'bottom' };
  }
  var rect = node.getBoundingClientRect();
  var service = page.dashboardPopupService();
  if (service && typeof service.anchorPoint === 'function') {
    return service.anchorPoint(rect, preferredSide);
  }
  var side = page.dashboardPopupAxisAwareSideAway(rect, preferredSide);
  var inlineAway = page.dashboardPopupHorizontalAwayFromNearestWall(rect, 'right');
  var blockAway = page.dashboardPopupVerticalAwayFromNearestWall(rect, 'bottom');
  var left = Math.round(Number(rect.left || 0));
  var top = Math.round(Number(rect.bottom || 0));
  if (side === 'top') {
    left = inlineAway === 'left'
      ? Math.round(Number(rect.right || 0))
      : Math.round(Number(rect.left || 0));
    top = Math.round(Number(rect.top || 0));
  } else if (side === 'bottom') {
    left = inlineAway === 'left'
      ? Math.round(Number(rect.right || 0))
      : Math.round(Number(rect.left || 0));
    top = Math.round(Number(rect.bottom || 0));
  } else if (side === 'left') {
    left = Math.round(Number(rect.left || 0));
    top = blockAway === 'top'
      ? Math.round(Number(rect.bottom || 0))
      : Math.round(Number(rect.top || 0));
  } else if (side === 'right') {
    left = Math.round(Number(rect.right || 0));
    top = blockAway === 'top'
      ? Math.round(Number(rect.bottom || 0))
      : Math.round(Number(rect.top || 0));
  }
  return {
    left: left,
    top: top,
    side: side,
    inline_away: inlineAway === 'left' ? 'left' : 'right',
    block_away: blockAway === 'top' ? 'top' : 'bottom'
  };
}

function infringShowDashboardPopup(page, id, label, ev, overrides) {
  var popupId = String(id || '').trim();
  var title = String(label || '').trim();
  if (!popupId || !title) {
    page.hideDashboardPopup();
    return;
  }
  var eventType = String((ev && ev.type) || '').toLowerCase();
  if (
    eventType === 'mouseleave' ||
    eventType === 'pointerleave' ||
    eventType === 'blur' ||
    eventType === 'focusout'
  ) {
    page.hideDashboardPopup(popupId);
    return;
  }
  var config = overrides && typeof overrides === 'object' ? overrides : {};
  var anchor = page.dashboardPopupAnchorPoint(ev, config.side);
  if (
    (!anchor || !Number.isFinite(Number(anchor.left)) || !Number.isFinite(Number(anchor.top)) || Number(anchor.left) <= 0 || Number(anchor.top) <= 0) &&
    ev
  ) {
    var fallbackNode = ev.currentTarget || ev.target;
    if (fallbackNode && typeof fallbackNode.getBoundingClientRect === 'function') {
      var fallbackRect = fallbackNode.getBoundingClientRect();
      var fallbackSide = String(config.side || (anchor && anchor.side) || 'bottom').toLowerCase();
      if (fallbackSide !== 'top' && fallbackSide !== 'left' && fallbackSide !== 'right') fallbackSide = 'bottom';
      anchor = {
        left: Math.round(fallbackSide === 'right' ? Number(fallbackRect.right || 0) : Number(fallbackRect.left || 0)),
        top: Math.round(fallbackSide === 'bottom' ? Number(fallbackRect.bottom || 0) : Number(fallbackRect.top || 0)),
        side: fallbackSide,
        inline_away: 'right',
        block_away: 'bottom'
      };
    }
  }
  var service = page.dashboardPopupService();
  page.dashboardPopup = service && typeof service.openState === 'function'
    ? service.openState(popupId, title, config, anchor)
    : {
      id: popupId,
      active: true,
      source: String(config.source || '').trim(),
      title: title,
      body: String(config.body || '').trim(),
      meta_origin: '',
      meta_time: '',
      unread: !!config.unread,
      left: anchor.left,
      top: anchor.top,
      side: anchor.side,
      inline_away: anchor.inline_away === 'left' ? 'left' : 'right',
      block_away: anchor.block_away === 'top' ? 'top' : 'bottom',
      compact: false
    };
}

function infringShowTaskbarNavPopup(page, label, ev) {
  var navLabel = String(label || '').trim();
  if (!navLabel) {
    page.hideDashboardPopup();
    return;
  }
  var navKey = navLabel.toLowerCase().replace(/[^a-z0-9_-]+/g, '-');
  var body = navKey === 'back'
    ? (page.canNavigateBack() ? 'Go to the previous page in this session' : 'No earlier page in this session')
    : (page.canNavigateForward() ? 'Go to the next page in this session' : 'No later page in this session');
  page.showDashboardPopup('taskbar-nav:' + navKey, navLabel, ev, {
    source: 'taskbar',
    side: 'bottom',
    compact: false,
    body: body,
    meta_origin: 'Chat nav'
  });
}

function infringShowTaskbarUtilityPopup(page, label, body, ev) {
  var utilityLabel = String(label || '').trim();
  if (!utilityLabel) {
    page.hideDashboardPopup();
    return;
  }
  page.showDashboardPopup(
    'taskbar-utility:' + utilityLabel.toLowerCase().replace(/[^a-z0-9_-]+/g, '-'),
    utilityLabel,
    ev,
    {
      source: 'taskbar',
      side: 'bottom',
      compact: false,
      body: String(body || '').trim(),
      meta_origin: 'Taskbar'
    }
  );
}

function infringHideDashboardPopup(page, rawId) {
  var service = page.dashboardPopupService();
  if (service && typeof service.closeState === 'function') {
    page.dashboardPopup = service.closeState(page.dashboardPopup, rawId);
    return;
  }
  var popupId = String(rawId || '').trim();
  var currentId = String(page.dashboardPopup && page.dashboardPopup.id || '').trim();
  if (popupId && currentId && popupId !== currentId) return;
  page.clearDashboardPopupState();
}

(function installDashboardPopupDelegatedHover() {
  if (typeof window === 'undefined' || typeof document === 'undefined') return;
  if (window.__infringDashboardPopupDelegatedHoverInstalled) return;
  window.__infringDashboardPopupDelegatedHoverInstalled = true;

  function clean(value) {
    return String(value == null ? '' : value).trim();
  }

  function appStoreService() {
    var services = window.InfringSharedShellServices;
    return services && services.appStore ? services.appStore : null;
  }

  function appStore() {
    var service = appStoreService();
    if (service && typeof service.current === 'function') {
      var current = service.current();
      if (current && typeof current === 'object') return current;
    }
    if (window.InfringApp && typeof window.InfringApp === 'object') return window.InfringApp;
    try {
      if (window.Alpine && typeof window.Alpine.store === 'function') {
        var alpineStore = window.Alpine.store('app');
        if (alpineStore && typeof alpineStore === 'object') return alpineStore;
      }
    } catch (_) {}
    return null;
  }

  function closestPopupTarget(start) {
    var node = start && start.nodeType === 1 ? start : null;
    while (node && node !== document.body && node !== document.documentElement) {
      if (node.matches && node.matches([
        '[data-dashboard-popup-title]',
        '[data-popup-title]',
        '[data-dock-id]',
        '.bottom-dock-btn',
        '.nav-agent-row',
        '.nav-agent-card',
        '.nav-item',
        '.sidebar-tab-item',
        '.chat-map-day',
        '.chat-map-entry',
        '.chat-map-surface',
        '.chat-map-message',
        '.chat-map-item',
        '.chat-map-row',
        '.chat-map-marker',
        '.chat-map-tool-row',
        '.dock-tile'
      ].join(','))) {
        return node;
      }
      node = node.parentElement;
    }
    return null;
  }

  function targetLabel(target) {
    if (!target) return '';
    return clean(
      target.getAttribute('data-dashboard-popup-title') ||
      target.getAttribute('data-popup-title') ||
      target.getAttribute('aria-label') ||
      target.getAttribute('title') ||
      target.textContent ||
      target.getAttribute('data-dock-id') ||
      ''
    );
  }

  function targetBody(target, title) {
    if (!target) return '';
    var body = clean(
      target.getAttribute('data-dashboard-popup-body') ||
      target.getAttribute('data-popup-body') ||
      target.getAttribute('data-tooltip') ||
      target.getAttribute('title') ||
      ''
    );
    return body && body !== title ? body : '';
  }

  function targetSource(target) {
    if (!target) return 'dashboard';
    if (target.closest && target.closest('.bottom-dock, infring-bottom-dock-shell')) return 'bottom_dock';
    if (target.closest && target.closest('.chat-map, .chat-map-rail, .chat-map-surface, .chat-map-scroll, infring-chat-map-shell, infring-chat-map-rail-shell, infring-chat-map-viewport-shell')) return 'chat-map';
    if (target.closest && target.closest('.sidebar, .nav-sidebar, infring-sidebar-agent-list-shell, infring-sidebar-rail-shell')) return 'sidebar';
    return 'dashboard';
  }

  function targetId(target, source, title) {
    var raw = clean(
      target.getAttribute('data-dashboard-popup-id') ||
      target.getAttribute('data-popup-id') ||
      target.getAttribute('data-dock-id') ||
      target.getAttribute('data-agent-id') ||
      target.getAttribute('data-message-id') ||
      target.getAttribute('id') ||
      title
    ).toLowerCase().replace(/[^a-z0-9_-]+/g, '-');
    return 'delegated-' + clean(source || 'dashboard').replace(/[^a-z0-9_-]+/g, '-') + ':' + raw;
  }

  function fallbackAnchor(target, side) {
    var rect = target && typeof target.getBoundingClientRect === 'function'
      ? target.getBoundingClientRect()
      : null;
    if (!rect) return { left: 0, top: 0, side: side || 'bottom', inline_away: 'right', block_away: 'bottom' };
    var chosenSide = clean(side || 'bottom').toLowerCase();
    if (chosenSide !== 'top' && chosenSide !== 'left' && chosenSide !== 'right') chosenSide = 'bottom';
    if (chosenSide === 'top') {
      return { left: Math.round(rect.left), top: Math.round(rect.top), side: 'top', inline_away: 'right', block_away: 'bottom' };
    }
    if (chosenSide === 'left') {
      return { left: Math.round(rect.left), top: Math.round(rect.top), side: 'left', inline_away: 'right', block_away: 'bottom' };
    }
    if (chosenSide === 'right') {
      return { left: Math.round(rect.right), top: Math.round(rect.top), side: 'right', inline_away: 'right', block_away: 'bottom' };
    }
    return { left: Math.round(rect.left), top: Math.round(rect.bottom), side: 'bottom', inline_away: 'right', block_away: 'bottom' };
  }

  function publishPopup(page, id, title, body, source, side, event, target) {
    if (!page || typeof page !== 'object') return;
    var anchor = null;
    try {
      if (typeof page.dashboardPopupAnchorPoint === 'function') {
        anchor = page.dashboardPopupAnchorPoint(event, side);
      }
    } catch (_) {}
    if (
      !anchor ||
      !Number.isFinite(Number(anchor.left)) ||
      !Number.isFinite(Number(anchor.top)) ||
      Number(anchor.left) <= 0 ||
      Number(anchor.top) <= 0
    ) {
      anchor = fallbackAnchor(target, side);
    }
    page.dashboardPopup = {
      id: id,
      active: true,
      source: source,
      title: title,
      body: body,
      meta_origin: '',
      meta_time: '',
      unread: false,
      left: Math.round(Number(anchor.left || 0)),
      top: Math.round(Number(anchor.top || 0)),
      side: clean(anchor.side || side || 'bottom').toLowerCase(),
      inline_away: anchor.inline_away === 'left' ? 'left' : 'right',
      block_away: anchor.block_away === 'top' ? 'top' : 'bottom',
      compact: false
    };
    var service = appStoreService();
    if (service && typeof service.notify === 'function') {
      try { service.notify('dashboard_popup_delegated_hover'); } catch (_) {}
    }
  }

  function show(event) {
    var page = appStore();
    if (!page) return;
    var target = closestPopupTarget(event && event.target);
    var title = targetLabel(target);
    if (!target || !title) return;
    var source = targetSource(target);
    var side = source === 'bottom_dock' ? 'top' : 'right';
    var id = targetId(target, source, title);
    var body = targetBody(target, title);
    if (typeof page.showDashboardPopup === 'function') {
      try {
        page.showDashboardPopup(id, title, event, {
          source: source,
          side: side,
          body: body,
          meta_origin: ''
        });
      } catch (_) {}
    }
    publishPopup(page, id, title, body, source, side, event, target);
  }

  function hide(event) {
    var page = appStore();
    if (!page || typeof page.hideDashboardPopupBySource !== 'function') return;
    var target = closestPopupTarget(event && event.target);
    if (!target) return;
    var related = event && event.relatedTarget && event.relatedTarget.nodeType === 1
      ? event.relatedTarget
      : null;
    if (related && target.contains && target.contains(related)) return;
    if (typeof page.hideDashboardPopupBySource === 'function') {
      page.hideDashboardPopupBySource(targetSource(target));
    } else {
      page.dashboardPopup = { id: '', active: false, source: '', title: '', body: '', meta_origin: '', meta_time: '', unread: false, left: 0, top: 0, side: 'bottom', inline_away: 'right', block_away: 'bottom', compact: false };
    }
    var service = appStoreService();
    if (service && typeof service.notify === 'function') {
      try { service.notify('dashboard_popup_delegated_hide'); } catch (_) {}
    }
  }

  document.addEventListener('mouseover', show, true);
  document.addEventListener('mousemove', show, true);
  document.addEventListener('focusin', show, true);
  document.addEventListener('mouseout', hide, true);
  document.addEventListener('focusout', hide, true);
})();
