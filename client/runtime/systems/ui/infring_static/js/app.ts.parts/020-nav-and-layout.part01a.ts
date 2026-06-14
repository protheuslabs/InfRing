          result.indexOf('approval') >= 0 ||
          result.indexOf('permission') >= 0 ||
          result.indexOf('fail-closed') >= 0;
        if (blocked) return 'warning';
        if (tool.is_error) return 'error';
        return 'success';
      };
      var summarizeTools = function(tools) {
        if (!Array.isArray(tools) || !tools.length) return { has_tools: false, tool_state: '', tool_label: '' };
        var state = 'success';
        for (var ti = 0; ti < tools.length; ti++) {
          var s = classifyTool(tools[ti]) || 'success';
          if ((toolStateRank[s] || 0) > (toolStateRank[state] || 0)) state = s;
        }
        var label = state === 'error'
          ? 'Tool error'
          : (state === 'warning' ? 'Tool warning' : 'Tool success');
        return { has_tools: true, tool_state: state, tool_label: label };
      };
      for (var i = list.length - 1; i >= 0; i--) {
        var msg = list[i] || {};
        var text = '';
        var toolInfo = summarizeTools(msg.tools);
        if (typeof msg.text === 'string' && msg.text.trim()) {
          text = msg.text.replace(/\s+/g, ' ').trim();
        } else if (Array.isArray(msg.tools) && msg.tools.length) {
          text = '[Processes] ' + msg.tools.map(function(tool) {
            return tool && tool.name ? tool.name : 'tool';
          }).join(', ');
        }
        if (text) {
          preview.text = text;
          preview.ts = Number(msg.ts || Date.now());
          preview.role = String(msg.role || 'agent');
          preview.has_tools = !!toolInfo.has_tools;
          preview.tool_state = toolInfo.tool_state || '';
          preview.tool_label = toolInfo.tool_label || '';
          break;
        }
      }
      if (preview.role === 'agent') {
        preview.unread_response = String(this.activeAgentId || '') !== previewKey;
      } else if (String(this.activeAgentId || '') === previewKey) {
        preview.unread_response = false;
      }
      var previewChanged = !!existingPreview && (
        Number(preview.ts || 0) > Number(existingPreview.ts || 0) ||
        String(preview.text || '') !== String(existingPreview.text || '') ||
        String(preview.role || '') !== String(existingPreview.role || '') ||
        String(preview.tool_state || '') !== String(existingPreview.tool_state || '')
      );
      var inactiveAgent = String(this.activeAgentId || '') !== previewKey;
      if (previewChanged && inactiveAgent && preview.role === 'agent' && String(preview.text || '').trim()) {
        var label = 'Agent';
        if (Array.isArray(this.agents)) {
          var found = this.agents.find(function(row) {
            return row && String(row.id || '') === previewKey;
          });
          if (found) {
            var foundName = String(found.name || '').trim();
            if (foundName) label = foundName;
          }
        }
        var compact = String(preview.text || '').replace(/\s+/g, ' ').trim();
        if (compact.length > 120) compact = compact.slice(0, 117) + '...';
        this.addNotification({
          type: 'info',
          message: label + ': ' + compact,
          agent_id: previewKey,
          page: 'chat',
          source: 'agent_preview',
          ts: Number(preview.ts || Date.now())
        });
      }
      this.agentChatPreviews[previewKey] = preview;
    },

    getAgentChatPreview(agentId) {
      if (!agentId) return null;
      return this.agentChatPreviews[String(agentId)] || null;
    },

    coerceAgentTimestamp(value) {
      if (value === null || typeof value === 'undefined' || value === '') return 0;
      if (typeof value === 'number') {
        if (!Number.isFinite(value)) return 0;
        return value < 1e12 ? Math.round(value * 1000) : Math.round(value);
      }
      var asNum = Number(value);
      if (Number.isFinite(asNum) && String(value).trim() !== '') {
        return asNum < 1e12 ? Math.round(asNum * 1000) : Math.round(asNum);
      }
      var asDate = Number(new Date(value).getTime());
      return Number.isFinite(asDate) ? asDate : 0;
    },

    agentLastActivityTs(agent) {
      if (!agent) return 0;
      var latest = 0;
      var keys = ['last_active_at', 'last_activity_at', 'last_message_at', 'last_seen_at', 'updated_at'];
      for (var i = 0; i < keys.length; i++) {
        var ts = this.coerceAgentTimestamp(agent[keys[i]]);
        if (ts > latest) latest = ts;
      }
      if (agent.id) {
        var preview = this.getAgentChatPreview(agent.id);
        var previewTs = this.coerceAgentTimestamp(preview && preview.ts);
        if (previewTs > latest) latest = previewTs;
      }
      return latest;
    },

    agentStatusFreshness(agent) {
      var raw = agent && agent.sidebar_status_freshness && typeof agent.sidebar_status_freshness === 'object'
        ? agent.sidebar_status_freshness
        : {};
      var source = String((raw.source || (agent && agent.sidebar_status_source) || '')).trim();
      var sourceSequence = String((raw.source_sequence || (agent && agent.sidebar_status_source_sequence) || '')).trim();
      var ageRaw = Number(
        typeof raw.age_seconds !== 'undefined'
          ? raw.age_seconds
          : (agent && agent.sidebar_status_age_seconds)
      );
      var ageSeconds = Number.isFinite(ageRaw) && ageRaw >= 0 ? ageRaw : 0;
      var staleRaw = raw.stale;
      if (typeof staleRaw !== 'boolean' && agent && typeof agent.sidebar_status_stale === 'boolean') {
        staleRaw = agent.sidebar_status_stale;
      }
      var stale = staleRaw === true;
      return {
        source: source,
        source_sequence: sourceSequence,
        age_seconds: ageSeconds,
        stale: stale
      };
    },

    agentStatusState(agent) {
      if (!agent) return 'offline';
      var rawServerState = (typeof agent.sidebar_status_state === 'string')
        ? agent.sidebar_status_state
        : '';
      var serverState = String(rawServerState).trim().toLowerCase();
      if (serverState === 'active' || serverState === 'idle' || serverState === 'offline') return serverState;
      var freshness = this.agentStatusFreshness(agent);
      if (freshness.stale) return 'offline';
      return 'offline';
    },

    agentStatusLabel(agent) {
      var rawServerLabel = (agent && typeof agent.sidebar_status_label === 'string')
        ? agent.sidebar_status_label
        : '';
      var serverLabel = String(rawServerLabel).trim().toLowerCase();
      if (serverLabel === 'active' || serverLabel === 'idle' || serverLabel === 'offline') return serverLabel;
      var rawServerState = (agent && typeof agent.sidebar_status_state === 'string')
        ? agent.sidebar_status_state
        : '';
      var serverState = String(rawServerState).trim().toLowerCase();
      if (serverState === 'active' || serverState === 'idle' || serverState === 'offline') return serverState;
      var freshness = this.agentStatusFreshness(agent);
      if (freshness.stale) return 'offline';
      return 'offline';
    },

    setAgentLiveActivity(agentId, state, options) {
      var id = String(agentId || '').trim();
      if (!id) return;
      var normalized = String(state || '').trim().toLowerCase();
      var opts = options && typeof options === 'object' ? options : {};
      var ts = Date.now();
      if (!normalized || normalized === 'idle' || normalized === 'done' || normalized === 'stop' || normalized === 'stopped') {
        if (this.agentLiveActivity && Object.prototype.hasOwnProperty.call(this.agentLiveActivity, id)) {
          delete this.agentLiveActivity[id];
          this.agentLiveActivity = Object.assign({}, this.agentLiveActivity);
        }
        return;
      }
      this.agentLiveActivity = Object.assign({}, this.agentLiveActivity || {}, {
        [id]: {
          state: normalized,
          ts: ts,
          source: String(opts.source || 'shell_display_hint'),
          optimistic: opts.optimistic !== false,
          projection_source: String(opts.projection_source || 'shell_socket_projection_hint')
        }
      });
    },

    clearAgentLiveActivity(agentId) {
      this.setAgentLiveActivity(agentId, 'idle', { optimistic: true, source: 'shell_display_hint' });
    },

    isAgentLiveBusy(agent) {
      if (!agent || !agent.id) return false;
      var id = String(agent.id);
      var entry = this.agentLiveActivity ? this.agentLiveActivity[id] : null;
      if (entry) {
        var state = String(entry.state || '').toLowerCase();
        var ts = Number(entry.ts || 0);
        var busyState = state.indexOf('typing') >= 0 || state.indexOf('working') >= 0 || state.indexOf('processing') >= 0;
        // Allow longer-lived busy windows so long tool/reasoning phases keep
        // the avatar pulse visible until completion events clear the state.
        if (busyState && Number.isFinite(ts) && (Date.now() - ts) <= 180000) return true;
      }
      return false;
    },

    formatNotificationTime(ts) {
      if (!ts) return '';
      var d = new Date(ts);
      if (Number.isNaN(d.getTime())) return '';
      return d.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
    },

    clearApiKey() {
      InfringAPI.setAuthToken('');
      localStorage.removeItem('infring-api-key');
    }
  };
  var appStoreBridge = infringShellAppStoreBridge();
  if (appStoreBridge && typeof appStoreBridge.registerAlpineStore === 'function') {
    appStoreBridge.registerAlpineStore(Alpine, 'app', appStoreDefinition);
  } else {
    var alpineRuntime = Alpine;
    if (alpineRuntime && typeof alpineRuntime.store === 'function') {
      alpineRuntime.store('app', appStoreDefinition);
      window.InfringApp = alpineRuntime.store('app');
    } else {
      window.InfringApp = appStoreDefinition;
    }
  }
});

function infringTaskbarDockService() {
  var services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
  return services && services.taskbarDock ? services.taskbarDock : null;
}

function infringShellLayoutDefaultProfile() {
  var service = infringTaskbarDockService();
  if (service && typeof service.defaultProfile === 'function') return service.defaultProfile();
  var raw = '';
  try {
    raw = String((navigator && (navigator.userAgent || navigator.platform)) || '').toLowerCase();
  } catch(_) {}
  if (raw.indexOf('mac') >= 0 || raw.indexOf('darwin') >= 0) return 'mac';
  if (raw.indexOf('win') >= 0) return 'windows';
  if (raw.indexOf('linux') >= 0 || raw.indexOf('x11') >= 0) return 'linux';
  return 'other';
}

function infringShellLayoutDefaultConfig() {
  var service = infringTaskbarDockService();
  if (service && typeof service.defaultLayoutConfig === 'function') return service.defaultLayoutConfig();
  var profile = infringShellLayoutDefaultProfile();
  var macLike = profile === 'mac';
  return {
    version: 1,
    profile: profile,
    dock: {
      placement: 'center',
      wallLock: macLike ? '' : 'bottom',
      order: ['chat', 'overview', 'agents', 'scheduler', 'skills', 'runtime', 'settings']
    },
    taskbar: {
      edge: macLike ? 'top' : 'bottom',
      orderLeft: ['nav_cluster'],
      orderRight: ['connectivity', 'theme', 'notifications', 'search', 'auth']
    },
    chatMap: { placementX: 1, placementY: 0.38, wallLock: 'right' },
    chatBar: { placementX: 1, placementY: 0.5, placementTopPx: null, wallLock: 'right' }
  };
}

function infringLocalStorageHasAny(keys) {
  var service = infringTaskbarDockService();
  if (service && typeof service.hasAnyStorage === 'function') return service.hasAnyStorage(keys);
  try {
    for (var i = 0; i < keys.length; i += 1) {
      if (localStorage.getItem(keys[i]) !== null) return true;
    }
  } catch(_) {}
  return false;
}

function infringReadShellLayoutConfig() {
  var service = infringTaskbarDockService();
  if (service && typeof service.readLayoutConfig === 'function') return service.readLayoutConfig();
  var key = 'infring-shell-layout-config';
  var config = null;
  try {
    var raw = localStorage.getItem(key);
    config = raw ? JSON.parse(raw) : null;
  } catch(_) {
    config = null;
  }
  if (!config || typeof config !== 'object') config = infringShellLayoutDefaultConfig();
  var defaults = infringShellLayoutDefaultConfig();
  config.dock = config.dock && typeof config.dock === 'object' ? config.dock : {};
  config.taskbar = config.taskbar && typeof config.taskbar === 'object' ? config.taskbar : {};
  config.chatMap = config.chatMap && typeof config.chatMap === 'object' ? config.chatMap : {};
  config.chatBar = config.chatBar && typeof config.chatBar === 'object' ? config.chatBar : {};
  config.dock.placement = String(config.dock.placement || defaults.dock.placement);
  config.dock.wallLock = String(config.dock.wallLock || defaults.dock.wallLock || '');
  config.taskbar.edge = String(config.taskbar.edge || defaults.taskbar.edge);
  config.chatMap.placementX = Number.isFinite(Number(config.chatMap.placementX)) ? Number(config.chatMap.placementX) : defaults.chatMap.placementX;
  config.chatMap.placementY = Number.isFinite(Number(config.chatMap.placementY)) ? Number(config.chatMap.placementY) : defaults.chatMap.placementY;
  config.chatMap.wallLock = String(config.chatMap.wallLock || defaults.chatMap.wallLock || '');
  config.chatBar.placementX = Number.isFinite(Number(config.chatBar.placementX)) ? Number(config.chatBar.placementX) : defaults.chatBar.placementX;
  config.chatBar.placementY = Number.isFinite(Number(config.chatBar.placementY)) ? Number(config.chatBar.placementY) : defaults.chatBar.placementY;
  config.chatBar.placementTopPx = Number.isFinite(Number(config.chatBar.placementTopPx)) ? Number(config.chatBar.placementTopPx) : null;
  config.chatBar.wallLock = String(config.chatBar.wallLock || defaults.chatBar.wallLock || '');
  if (!Array.isArray(config.dock.order)) config.dock.order = defaults.dock.order.slice();
  if (!Array.isArray(config.taskbar.orderLeft)) config.taskbar.orderLeft = defaults.taskbar.orderLeft.slice();
  if (!Array.isArray(config.taskbar.orderRight)) config.taskbar.orderRight = defaults.taskbar.orderRight.slice();
  return config;
}

function infringWriteShellLayoutConfig(config) {
  var service = infringTaskbarDockService();
  if (service && typeof service.writeLayoutConfig === 'function') {
    service.writeLayoutConfig(config);
    return;
  }
  try {
    localStorage.setItem('infring-shell-layout-config', JSON.stringify(config));
  } catch(_) {}
}

function infringUpdateShellLayoutConfig(mutator) {
  var service = infringTaskbarDockService();
  if (service && typeof service.updateLayoutConfig === 'function') {
    infringShellLayoutConfig = service.updateLayoutConfig(mutator);
    return;
  }
  var config = infringReadShellLayoutConfig();
  try { mutator(config); } catch(_) {}
  infringShellLayoutConfig = config;
  infringWriteShellLayoutConfig(config);
}

function infringSeedShellLayoutConfig() {
  var service = infringTaskbarDockService();
  if (service && typeof service.seedLayoutConfig === 'function') return service.seedLayoutConfig();
  var config = infringReadShellLayoutConfig();
  var existed = false;
  try { existed = localStorage.getItem('infring-shell-layout-config') !== null; } catch(_) {}
  if (!existed) {
    var dockKeys = ['infring-bottom-dock-placement', 'infring-bottom-dock-wall-lock', 'infring-bottom-dock-order'];
    var taskbarKeys = ['infring-taskbar-dock-edge', 'infring-taskbar-order-left', 'infring-taskbar-order-right'];
    var chatMapKeys = ['infring-chat-map-placement-x', 'infring-chat-map-placement-y', 'infring-chat-map-wall-lock'];
    var chatBarKeys = ['infring-chat-sidebar-placement-x', 'infring-chat-sidebar-placement-y', 'infring-chat-sidebar-placement-top-px', 'infring-chat-sidebar-wall-lock'];
    try {
      if (localStorage.getItem(dockKeys[0])) config.dock.placement = localStorage.getItem(dockKeys[0]);
      if (localStorage.getItem(dockKeys[1])) config.dock.wallLock = localStorage.getItem(dockKeys[1]);
      if (localStorage.getItem(dockKeys[2])) config.dock.order = JSON.parse(localStorage.getItem(dockKeys[2]) || '[]');
      if (localStorage.getItem(taskbarKeys[0])) config.taskbar.edge = localStorage.getItem(taskbarKeys[0]);
      if (localStorage.getItem(taskbarKeys[1])) config.taskbar.orderLeft = JSON.parse(localStorage.getItem(taskbarKeys[1]) || '[]');
      if (localStorage.getItem(taskbarKeys[2])) config.taskbar.orderRight = JSON.parse(localStorage.getItem(taskbarKeys[2]) || '[]');
      if (localStorage.getItem(chatMapKeys[0])) config.chatMap.placementX = Number(localStorage.getItem(chatMapKeys[0]));
      if (localStorage.getItem(chatMapKeys[1])) config.chatMap.placementY = Number(localStorage.getItem(chatMapKeys[1]));
      if (localStorage.getItem(chatMapKeys[2])) config.chatMap.wallLock = localStorage.getItem(chatMapKeys[2]);
      if (localStorage.getItem(chatBarKeys[0])) config.chatBar.placementX = Number(localStorage.getItem(chatBarKeys[0]));
      if (localStorage.getItem(chatBarKeys[1])) config.chatBar.placementY = Number(localStorage.getItem(chatBarKeys[1]));
      if (localStorage.getItem(chatBarKeys[2])) config.chatBar.placementTopPx = Number(localStorage.getItem(chatBarKeys[2]));
      if (localStorage.getItem(chatBarKeys[3])) config.chatBar.wallLock = localStorage.getItem(chatBarKeys[3]);
    } catch(_) {}
  }
  try {
    if (!infringLocalStorageHasAny(['infring-bottom-dock-placement'])) localStorage.setItem('infring-bottom-dock-placement', String(config.dock.placement || 'center'));
    if (!infringLocalStorageHasAny(['infring-bottom-dock-wall-lock', 'infring-bottom-dock-smash-wall']) && config.dock.wallLock) localStorage.setItem('infring-bottom-dock-wall-lock', String(config.dock.wallLock));
    if (!infringLocalStorageHasAny(['infring-bottom-dock-order'])) localStorage.setItem('infring-bottom-dock-order', JSON.stringify(config.dock.order || []));
    if (!infringLocalStorageHasAny(['infring-taskbar-dock-edge'])) localStorage.setItem('infring-taskbar-dock-edge', String(config.taskbar.edge || 'top'));
    if (!infringLocalStorageHasAny(['infring-taskbar-order-left'])) localStorage.setItem('infring-taskbar-order-left', JSON.stringify(config.taskbar.orderLeft || []));
    if (!infringLocalStorageHasAny(['infring-taskbar-order-right'])) localStorage.setItem('infring-taskbar-order-right', JSON.stringify(config.taskbar.orderRight || []));
    if (!infringLocalStorageHasAny(['infring-chat-map-placement-x'])) localStorage.setItem('infring-chat-map-placement-x', String(config.chatMap.placementX));
    if (!infringLocalStorageHasAny(['infring-chat-map-placement-y'])) localStorage.setItem('infring-chat-map-placement-y', String(config.chatMap.placementY));
    if (!infringLocalStorageHasAny(['infring-chat-map-wall-lock', 'infring-chat-map-smash-wall']) && config.chatMap.wallLock) localStorage.setItem('infring-chat-map-wall-lock', String(config.chatMap.wallLock));
    if (!infringLocalStorageHasAny(['infring-chat-sidebar-placement-x'])) localStorage.setItem('infring-chat-sidebar-placement-x', String(config.chatBar.placementX));
    if (!infringLocalStorageHasAny(['infring-chat-sidebar-placement-y'])) localStorage.setItem('infring-chat-sidebar-placement-y', String(config.chatBar.placementY));
    if (!infringLocalStorageHasAny(['infring-chat-sidebar-placement-top-px']) && Number.isFinite(Number(config.chatBar.placementTopPx))) localStorage.setItem('infring-chat-sidebar-placement-top-px', String(config.chatBar.placementTopPx));
    if (!infringLocalStorageHasAny(['infring-chat-sidebar-wall-lock', 'infring-chat-sidebar-smash-wall']) && config.chatBar.wallLock) localStorage.setItem('infring-chat-sidebar-wall-lock', String(config.chatBar.wallLock));
  } catch(_) {}
  infringWriteShellLayoutConfig(config);
  return config;
}

var infringShellLayoutConfig = infringSeedShellLayoutConfig();

// Main app component
function app() {
  return {
    page: 'agents',
    themeMode: localStorage.getItem('infring-theme-mode') || 'system',
    overlayGlassTemplate: 'simple-glass',
    uiBackgroundTemplate: (() => {
      var service = infringTaskbarDockService();
      if (service && typeof service.readDisplayBackground === 'function') return service.readDisplayBackground();
      var mode = 'default-grid';
      try {
        var migrationKey = 'infring-ui-background-template-default-grid-migrated';
        var rawDisplaySettings = localStorage.getItem('infring-display-settings') || '';
        var displaySettings = rawDisplaySettings ? JSON.parse(rawDisplaySettings) : {};
        mode = String(displaySettings && displaySettings.background ? displaySettings.background : mode);
        if (mode === 'light-wood' && localStorage.getItem(migrationKey) !== '1') {
          mode = 'default-grid';
          localStorage.setItem(migrationKey, '1');
        }
        if (mode === 'sand') {
          mode = 'light-wood';
          displaySettings = displaySettings && typeof displaySettings === 'object' ? displaySettings : {};
          displaySettings.background = mode;
          localStorage.setItem('infring-display-settings', JSON.stringify(displaySettings));
        }
        if (!rawDisplaySettings || !displaySettings.background) {
          displaySettings = displaySettings && typeof displaySettings === 'object' ? displaySettings : {};
          displaySettings.background = mode;
          localStorage.setItem('infring-display-settings', JSON.stringify(displaySettings));
        }
      } catch (_) {}
      if (mode === 'unsplash-paper') mode = 'default-grid';
      if (mode !== 'default-grid' && mode !== 'light-wood' && mode !== 'sand') mode = 'default-grid';
      try {
        document.documentElement.setAttribute('data-ui-background-template', mode);
      } catch (_) {}
      return mode;
    })(),
    theme: (() => {
      var mode = localStorage.getItem('infring-theme-mode') || 'system';
      if (mode === 'system') return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
      return mode;
    })(),
    sidebarCollapsed: localStorage.getItem('infring-sidebar') === 'collapsed',
    mobileMenuOpen: false,
    chatSidebarMode: 'default',
    chatSidebarQuery: '',
    chatSidebarSearchResults: [],
    chatSidebarSearchLoading: false,
    chatSidebarSearchError: '',
    chatSidebarSearchSeq: 0,
    _chatSidebarSearchTimer: 0,
    agentChatsSectionCollapsed: false,
    chatSidebarSortMode: (() => {
      try {
        var saved = String(localStorage.getItem('infring-chat-sidebar-sort-mode') || '').trim().toLowerCase();
        return saved === 'topology' ? 'topology' : 'age';
      } catch(_) {
        return 'age';
      }
    })(),
    chatSidebarTopologyOrder: (() => {
      try {
        var raw = localStorage.getItem('infring-chat-sidebar-topology-order');
        var parsed = raw ? JSON.parse(raw) : [];
        if (!Array.isArray(parsed)) return [];
        return parsed.map(function(id) { return String(id || '').trim(); }).filter(Boolean);
      } catch(_) {
        return [];
      }
    })(),
    chatSidebarDragAgentId: '',
