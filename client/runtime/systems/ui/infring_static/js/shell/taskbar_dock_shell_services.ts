'use strict';

var InfringSharedShellServices = (function(existing) {
  var services = existing && typeof existing === 'object' ? existing : {};
  var dockOrder = ['chat', 'overview', 'agents', 'scheduler', 'skills', 'runtime', 'settings'];
  var taskbarLeftOrder = ['nav_cluster'];
  var taskbarRightOrder = ['connectivity', 'theme', 'notifications', 'search', 'auth'];
  var backgroundTemplates = { 'default-grid': true, 'light-wood': true, sand: true };

  function trimString(value) {
    return String(value == null ? '' : value).trim();
  }

  function numericOr(value, fallback) {
    var numeric = Number(value);
    return Number.isFinite(numeric) ? numeric : fallback;
  }

  function storageGet(key) {
    try { return localStorage.getItem(key); } catch(_) { return null; }
  }

  function storageSet(key, value) {
    try { localStorage.setItem(key, String(value)); } catch(_) {}
  }

  function storageRemove(key) {
    try { localStorage.removeItem(key); } catch(_) {}
  }

  function readJsonStorage(key, fallback) {
    try {
      var raw = localStorage.getItem(key);
      return raw ? JSON.parse(raw) : fallback;
    } catch(_) {
      return fallback;
    }
  }

  function writeJsonStorage(key, value) {
    try { localStorage.setItem(key, JSON.stringify(value)); } catch(_) {}
  }

  function hasAnyStorage(keys) {
    try {
      for (var i = 0; i < keys.length; i += 1) {
        if (localStorage.getItem(keys[i]) !== null) return true;
      }
    } catch(_) {}
    return false;
  }

  function normalizeWall(wallRaw) {
    var service = services.dragbar;
    if (service && typeof service.normalizeWall === 'function') return service.normalizeWall(wallRaw);
    var wall = trimString(wallRaw).toLowerCase();
    if (wall === 'left' || wall === 'right' || wall === 'top' || wall === 'bottom') return wall;
    return '';
  }

  function defaultProfile() {
    var raw = '';
    try {
      raw = String((navigator && (navigator.userAgent || navigator.platform)) || '').toLowerCase();
    } catch(_) {}
    if (raw.indexOf('mac') >= 0 || raw.indexOf('darwin') >= 0) return 'mac';
    if (raw.indexOf('win') >= 0) return 'windows';
    if (raw.indexOf('linux') >= 0 || raw.indexOf('x11') >= 0) return 'linux';
    return 'other';
  }

  function defaultLayoutConfig(profileRaw) {
    var profile = trimString(profileRaw || defaultProfile()).toLowerCase() || 'other';
    var macLike = profile === 'mac';
    return {
      version: 1,
      profile: profile,
      dock: { placement: 'center', wallLock: macLike ? '' : 'bottom', order: dockOrder.slice() },
      taskbar: { edge: macLike ? 'top' : 'bottom', orderLeft: taskbarLeftOrder.slice(), orderRight: taskbarRightOrder.slice() },
      chatMap: { placementX: 1, placementY: 0.38, wallLock: 'right' },
      chatBar: { placementX: 1, placementY: 0.5, placementTopPx: null, wallLock: 'right' }
    };
  }

  function normalizeTaskbarEdge(raw) {
    return trimString(raw).toLowerCase() === 'bottom' ? 'bottom' : 'top';
  }

  function normalizeDockPlacement(raw) {
    var key = trimString(raw).toLowerCase();
    var allowed = {
      left: true,
      center: true,
      right: true,
      'top-left': true,
      'top-center': true,
      'top-right': true,
      'left-top': true,
      'left-bottom': true,
      'right-top': true,
      'right-bottom': true
    };
    if (allowed[key]) return key;
    if (key === 'left-center') return 'left-top';
    if (key === 'right-center') return 'right-top';
    return 'center';
  }

  function normalizeOrder(rawOrder, defaultsRaw) {
    var defaults = Array.isArray(defaultsRaw) && defaultsRaw.length ? defaultsRaw : [];
    var source = Array.isArray(rawOrder) ? rawOrder : [];
    var seen = {};
    var ordered = [];
    for (var i = 0; i < source.length; i += 1) {
      var id = trimString(source[i]);
      if (!id || seen[id] || defaults.indexOf(id) < 0) continue;
      seen[id] = true;
      ordered.push(id);
    }
    for (var j = 0; j < defaults.length; j += 1) {
      var fallbackId = defaults[j];
      if (seen[fallbackId]) continue;
      seen[fallbackId] = true;
      ordered.push(fallbackId);
    }
    return ordered;
  }

  function readLayoutConfig() {
    var defaults = defaultLayoutConfig();
    var config = readJsonStorage('infring-shell-layout-config', null);
    if (!config || typeof config !== 'object') config = defaultLayoutConfig();
    config.dock = config.dock && typeof config.dock === 'object' ? config.dock : {};
    config.taskbar = config.taskbar && typeof config.taskbar === 'object' ? config.taskbar : {};
    config.chatMap = config.chatMap && typeof config.chatMap === 'object' ? config.chatMap : {};
    config.chatBar = config.chatBar && typeof config.chatBar === 'object' ? config.chatBar : {};
    config.dock.placement = normalizeDockPlacement(config.dock.placement || defaults.dock.placement);
    config.dock.wallLock = normalizeWall(config.dock.wallLock || defaults.dock.wallLock || '');
    config.dock.order = normalizeOrder(config.dock.order, dockOrder);
    config.taskbar.edge = normalizeTaskbarEdge(config.taskbar.edge || defaults.taskbar.edge);
    config.taskbar.orderLeft = normalizeOrder(config.taskbar.orderLeft, taskbarLeftOrder);
    config.taskbar.orderRight = normalizeOrder(config.taskbar.orderRight, taskbarRightOrder);
    config.chatMap.placementX = numericOr(config.chatMap.placementX, defaults.chatMap.placementX);
    config.chatMap.placementY = numericOr(config.chatMap.placementY, defaults.chatMap.placementY);
    config.chatMap.wallLock = normalizeWall(config.chatMap.wallLock || defaults.chatMap.wallLock || '');
    config.chatBar.placementX = numericOr(config.chatBar.placementX, defaults.chatBar.placementX);
    config.chatBar.placementY = numericOr(config.chatBar.placementY, defaults.chatBar.placementY);
    config.chatBar.placementTopPx = Number.isFinite(Number(config.chatBar.placementTopPx)) ? Number(config.chatBar.placementTopPx) : null;
    config.chatBar.wallLock = normalizeWall(config.chatBar.wallLock || defaults.chatBar.wallLock || '');
    return config;
  }

  function writeLayoutConfig(config) {
    writeJsonStorage('infring-shell-layout-config', config);
  }

  function updateLayoutConfig(mutator) {
    var config = readLayoutConfig();
    try { mutator(config); } catch(_) {}
    writeLayoutConfig(config);
    return config;
  }

  function seedLayoutConfig() {
    var config = readLayoutConfig();
    var existed = storageGet('infring-shell-layout-config') !== null;
    if (!existed) {
      if (storageGet('infring-bottom-dock-placement')) config.dock.placement = normalizeDockPlacement(storageGet('infring-bottom-dock-placement'));
      if (storageGet('infring-bottom-dock-wall-lock')) config.dock.wallLock = normalizeWall(storageGet('infring-bottom-dock-wall-lock'));
      if (storageGet('infring-bottom-dock-order')) config.dock.order = normalizeOrder(readJsonStorage('infring-bottom-dock-order', []), dockOrder);
      if (storageGet('infring-taskbar-dock-edge')) config.taskbar.edge = normalizeTaskbarEdge(storageGet('infring-taskbar-dock-edge'));
      if (storageGet('infring-taskbar-order-left')) config.taskbar.orderLeft = normalizeOrder(readJsonStorage('infring-taskbar-order-left', []), taskbarLeftOrder);
      if (storageGet('infring-taskbar-order-right')) config.taskbar.orderRight = normalizeOrder(readJsonStorage('infring-taskbar-order-right', []), taskbarRightOrder);
      if (storageGet('infring-chat-map-placement-x')) config.chatMap.placementX = numericOr(storageGet('infring-chat-map-placement-x'), config.chatMap.placementX);
      if (storageGet('infring-chat-map-placement-y')) config.chatMap.placementY = numericOr(storageGet('infring-chat-map-placement-y'), config.chatMap.placementY);
      if (storageGet('infring-chat-map-wall-lock')) config.chatMap.wallLock = normalizeWall(storageGet('infring-chat-map-wall-lock'));
      if (storageGet('infring-chat-sidebar-placement-x')) config.chatBar.placementX = numericOr(storageGet('infring-chat-sidebar-placement-x'), config.chatBar.placementX);
      if (storageGet('infring-chat-sidebar-placement-y')) config.chatBar.placementY = numericOr(storageGet('infring-chat-sidebar-placement-y'), config.chatBar.placementY);
      if (storageGet('infring-chat-sidebar-placement-top-px')) config.chatBar.placementTopPx = numericOr(storageGet('infring-chat-sidebar-placement-top-px'), config.chatBar.placementTopPx);
      if (storageGet('infring-chat-sidebar-wall-lock')) config.chatBar.wallLock = normalizeWall(storageGet('infring-chat-sidebar-wall-lock'));
    }
    if (!hasAnyStorage(['infring-bottom-dock-placement'])) storageSet('infring-bottom-dock-placement', config.dock.placement || 'center');
    if (!hasAnyStorage(['infring-bottom-dock-wall-lock', 'infring-bottom-dock-smash-wall']) && config.dock.wallLock) storageSet('infring-bottom-dock-wall-lock', config.dock.wallLock);
    if (!hasAnyStorage(['infring-bottom-dock-order'])) writeJsonStorage('infring-bottom-dock-order', config.dock.order || []);
    if (!hasAnyStorage(['infring-taskbar-dock-edge'])) storageSet('infring-taskbar-dock-edge', config.taskbar.edge || 'top');
    if (!hasAnyStorage(['infring-taskbar-order-left'])) writeJsonStorage('infring-taskbar-order-left', config.taskbar.orderLeft || []);
    if (!hasAnyStorage(['infring-taskbar-order-right'])) writeJsonStorage('infring-taskbar-order-right', config.taskbar.orderRight || []);
    if (!hasAnyStorage(['infring-chat-map-placement-x'])) storageSet('infring-chat-map-placement-x', config.chatMap.placementX);
    if (!hasAnyStorage(['infring-chat-map-placement-y'])) storageSet('infring-chat-map-placement-y', config.chatMap.placementY);
    if (!hasAnyStorage(['infring-chat-map-wall-lock', 'infring-chat-map-smash-wall']) && config.chatMap.wallLock) storageSet('infring-chat-map-wall-lock', config.chatMap.wallLock);
    if (!hasAnyStorage(['infring-chat-sidebar-placement-x'])) storageSet('infring-chat-sidebar-placement-x', config.chatBar.placementX);
    if (!hasAnyStorage(['infring-chat-sidebar-placement-y'])) storageSet('infring-chat-sidebar-placement-y', config.chatBar.placementY);
    if (!hasAnyStorage(['infring-chat-sidebar-placement-top-px']) && Number.isFinite(Number(config.chatBar.placementTopPx))) storageSet('infring-chat-sidebar-placement-top-px', config.chatBar.placementTopPx);
    if (!hasAnyStorage(['infring-chat-sidebar-wall-lock', 'infring-chat-sidebar-smash-wall']) && config.chatBar.wallLock) storageSet('infring-chat-sidebar-wall-lock', config.chatBar.wallLock);
    writeLayoutConfig(config);
    return config;
  }

  function normalizeBackgroundTemplate(raw) {
    var mode = trimString(raw || 'default-grid').toLowerCase();
    if (mode === 'unsplash-paper') mode = 'default-grid';
    return backgroundTemplates[mode] ? mode : 'default-grid';
  }

  function readDisplayBackground() {
    var raw = readJsonStorage('infring-display-settings', {});
    var settings = raw && typeof raw === 'object' ? raw : {};
    var migrationKey = 'infring-ui-background-template-default-grid-migrated';
    var mode = normalizeBackgroundTemplate(settings.background || 'default-grid');
    if (mode === 'light-wood' && storageGet(migrationKey) !== '1') {
      mode = 'default-grid';
      storageSet(migrationKey, '1');
    }
    settings.background = mode;
    writeJsonStorage('infring-display-settings', settings);
    try { document.documentElement.setAttribute('data-ui-background-template', mode); } catch(_) {}
    return mode;
  }

  function writeDisplayBackground(modeRaw) {
    var mode = normalizeBackgroundTemplate(modeRaw);
    var settings = readJsonStorage('infring-display-settings', {});
    settings = settings && typeof settings === 'object' ? settings : {};
    settings.background = mode;
    writeJsonStorage('infring-display-settings', settings);
    try { document.documentElement.setAttribute('data-ui-background-template', mode); } catch(_) {}
    return mode;
  }

  function taskbarOrderDefaults(group) {
    return trimString(group).toLowerCase() === 'right' ? taskbarRightOrder.slice() : taskbarLeftOrder.slice();
  }

  function taskbarStorageKey(group) {
    return trimString(group).toLowerCase() === 'right' ? 'infring-taskbar-order-right' : 'infring-taskbar-order-left';
  }

  function readTaskbarOrder(group) {
    return normalizeOrder(readJsonStorage(taskbarStorageKey(group), []), taskbarOrderDefaults(group));
  }

  function persistTaskbarOrder(group, rawOrder) {
    var key = trimString(group).toLowerCase() === 'right' ? 'right' : 'left';
    var normalized = normalizeOrder(rawOrder, taskbarOrderDefaults(key));
    writeJsonStorage(taskbarStorageKey(key), normalized);
    updateLayoutConfig(function(config) {
      if (key === 'right') config.taskbar.orderRight = normalized.slice();
      else config.taskbar.orderLeft = normalized.slice();
    });
    return normalized;
  }

  function orderIndex(item, rawOrder, defaultsRaw) {
    var id = trimString(item);
    if (!id) return 999;
    var defaults = Array.isArray(defaultsRaw) ? defaultsRaw : [];
    var idx = normalizeOrder(rawOrder, defaults).indexOf(id);
    if (idx >= 0) return idx;
    var fallback = defaults.indexOf(id);
    return fallback >= 0 ? fallback : 999;
  }

  function dockTileConfig() {
    return {
      chat: { icon: 'messages', tone: 'message', tooltip: 'Messages', label: 'Messages' },
      overview: { icon: 'home', tone: 'bright', tooltip: 'Home', label: 'Home' },
      agents: { icon: 'agents', tone: 'bright', tooltip: 'Agents', label: 'Agents' },
      scheduler: { icon: 'automation', tone: 'muted', tooltip: 'Automation', label: 'Automation', animation: ['automation-gears', 1200] },
      skills: { icon: 'apps', tone: 'default', tooltip: 'Apps', label: 'Apps' },
      runtime: { icon: 'system', tone: 'bright', tooltip: 'System', label: 'System', animation: ['system-terminal', 2000] },
      settings: { icon: 'settings', tone: 'muted', tooltip: 'Settings', label: 'Settings', animation: ['spin', 4000] }
    };
  }

  function dockDefaultOrder(registryRaw) {
    var registry = registryRaw && typeof registryRaw === 'object' ? registryRaw : dockTileConfig();
    var ids = Object.keys(registry);
    return ids.length ? ids : dockOrder.slice();
  }

  function readDockOrder(registryRaw) {
    return normalizeOrder(readLayoutConfig().dock.order, dockDefaultOrder(registryRaw));
  }

  function persistDockOrder(rawOrder, registryRaw) {
    var normalized = normalizeOrder(rawOrder, dockDefaultOrder(registryRaw));
    writeJsonStorage('infring-bottom-dock-order', normalized);
    updateLayoutConfig(function(config) { config.dock.order = normalized.slice(); });
    return normalized;
  }

  function dockSlotStyle(id, rawOrder, weightRaw, registryRaw) {
    var weight = numericOr(weightRaw, 0);
    if (weight < 0) weight = 0;
    if (weight > 1) weight = 1;
    return 'order:' + orderIndex(id, rawOrder, dockDefaultOrder(registryRaw)) + ';--bottom-dock-hover-weight:' + weight.toFixed(4);
  }

  function dockTaskbarContained(wallRaw, taskbarEdgeRaw, draggingRaw, draggedWallRaw) {
    var wall = normalizeWall(wallRaw);
    if (wall !== 'top' && wall !== 'bottom') return false;
    if (draggingRaw && normalizeWall(draggedWallRaw) === wall) return true;
    return wall === normalizeTaskbarEdge(taskbarEdgeRaw);
  }

  function dockTaskbarContainedAnchorX(optionsRaw) {
    var options = optionsRaw && typeof optionsRaw === 'object' ? optionsRaw : {};
    var viewportWidth = Math.max(1, numericOr(options.viewportWidth, 1440));
    var dockWidth = Math.max(1, numericOr(options.dockWidth, 1));
    var left = Math.max(0, numericOr(options.leftAnchor, 16));
    var minX = dockWidth / 2;
    var maxX = Math.max(minX, viewportWidth - minX - 10);
    return Math.max(minX, Math.min(maxX, left + (dockWidth / 2)));
  }

  function dockTaskbarContainedMetrics(optionsRaw) {
    var options = optionsRaw && typeof optionsRaw === 'object' ? optionsRaw : {};
    var edge = normalizeTaskbarEdge(options.edge);
    var viewportHeight = Math.max(1, numericOr(options.viewportHeight, 900));
    var fallbackHeight = Math.max(1, numericOr(options.fallbackHeight, 32));
    var height = Math.max(1, numericOr(options.groupHeight, fallbackHeight));
    var centerY = Number.isFinite(Number(options.groupTop))
      ? numericOr(options.groupTop, 0) + (height / 2)
      : (edge === 'bottom' ? viewportHeight - 23 : 23);
    if (options.dragging && Number.isFinite(Number(options.dragY))) {
      centerY = numericOr(options.dragY, 0) + (Math.max(1, numericOr(options.taskbarHeight, 46)) / 2);
    }
    return { height: height, centerY: centerY };
  }

  function taskbarContainerStyle(optionsRaw) {
    var options = optionsRaw && typeof optionsRaw === 'object' ? optionsRaw : {};
    var edge = normalizeTaskbarEdge(options.edge);
    var dragging = !!options.dragging;
    var transitionMs = dragging ? 0 : Math.max(0, Math.round(numericOr(options.transitionMs, 220)));
    var styles = [];
    if (options.page !== 'chat') styles.push('background:transparent;border-bottom:none;box-shadow:none;-webkit-backdrop-filter:none;backdrop-filter:none;');
    styles.push('--taskbar-dock-transition:' + transitionMs + 'ms;');
    if (dragging) styles.push('top:' + Math.round(numericOr(options.dragY, 0)) + 'px;bottom:auto;');
    else if (edge === 'bottom') styles.push('top:auto;bottom:0;');
    else styles.push('top:0;bottom:auto;');
    return styles.join('');
  }

  services.taskbarDock = Object.assign({}, services.taskbarDock || {}, {
    defaultProfile: defaultProfile,
    defaultLayoutConfig: defaultLayoutConfig,
    hasAnyStorage: hasAnyStorage,
    readLayoutConfig: readLayoutConfig,
    writeLayoutConfig: writeLayoutConfig,
    updateLayoutConfig: updateLayoutConfig,
    seedLayoutConfig: seedLayoutConfig,
    normalizeBackgroundTemplate: normalizeBackgroundTemplate,
    readDisplayBackground: readDisplayBackground,
    writeDisplayBackground: writeDisplayBackground,
    normalizeTaskbarEdge: normalizeTaskbarEdge,
    normalizeDockPlacement: normalizeDockPlacement,
    normalizeOrder: normalizeOrder,
    taskbarOrderDefaults: taskbarOrderDefaults,
    taskbarStorageKey: taskbarStorageKey,
    readTaskbarOrder: readTaskbarOrder,
    persistTaskbarOrder: persistTaskbarOrder,
    orderIndex: orderIndex,
    dockTileConfig: dockTileConfig,
    dockDefaultOrder: dockDefaultOrder,
    readDockOrder: readDockOrder,
    persistDockOrder: persistDockOrder,
    dockSlotStyle: dockSlotStyle,
    dockTaskbarContained: dockTaskbarContained,
    dockTaskbarContainedAnchorX: dockTaskbarContainedAnchorX,
    dockTaskbarContainedMetrics: dockTaskbarContainedMetrics,
    taskbarContainerStyle: taskbarContainerStyle
  });

  return services;
})(typeof InfringSharedShellServices === 'object' ? InfringSharedShellServices : null);

if (typeof window !== 'undefined') {
  window.InfringSharedShellServices = InfringSharedShellServices;
}

function infringTaskbarDockService() {
  var services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
  return services && services.taskbarDock ? services.taskbarDock : null;
}

function infringShellLayoutDefaultProfile() {
  var service = infringTaskbarDockService();
  return service && typeof service.defaultProfile === 'function' ? service.defaultProfile() : 'other';
}

function infringShellLayoutDefaultConfig() {
  var service = infringTaskbarDockService();
  return service && typeof service.defaultLayoutConfig === 'function'
    ? service.defaultLayoutConfig()
    : { version: 1, profile: infringShellLayoutDefaultProfile(), dock: {}, taskbar: {}, chatMap: {}, chatBar: {} };
}

function infringLocalStorageHasAny(keys) {
  var service = infringTaskbarDockService();
  return service && typeof service.hasAnyStorage === 'function' ? service.hasAnyStorage(keys) : false;
}

function infringReadShellLayoutConfig() {
  var service = infringTaskbarDockService();
  return service && typeof service.readLayoutConfig === 'function'
    ? service.readLayoutConfig()
    : infringShellLayoutDefaultConfig();
}

function infringWriteShellLayoutConfig(config) {
  var service = infringTaskbarDockService();
  if (service && typeof service.writeLayoutConfig === 'function') service.writeLayoutConfig(config);
}

function infringUpdateShellLayoutConfig(mutator) {
  var service = infringTaskbarDockService();
  if (service && typeof service.updateLayoutConfig === 'function') {
    infringShellLayoutConfig = service.updateLayoutConfig(mutator);
    return;
  }
  infringShellLayoutConfig = infringReadShellLayoutConfig();
}

function infringSeedShellLayoutConfig() {
  var service = infringTaskbarDockService();
  return service && typeof service.seedLayoutConfig === 'function'
    ? service.seedLayoutConfig()
    : infringShellLayoutDefaultConfig();
}

var infringShellLayoutConfig = infringSeedShellLayoutConfig();
