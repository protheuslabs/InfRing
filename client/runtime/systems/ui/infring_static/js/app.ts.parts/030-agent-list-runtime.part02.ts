      this.mobileMenuOpen = false;
    },
    setTheme(mode) {
      this.beginInstantThemeFlip();
      this.themeMode = mode;
      localStorage.setItem('infring-theme-mode', mode);
      if (mode === 'system') {
        this.theme = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
      } else {
        this.theme = mode;
      }
    },
    isChatSidebarSearchActive() {
      return String(this.chatSidebarQuery || '').trim().length > 0;
    },
    clearChatSidebarSearch() {
      if (this._chatSidebarSearchTimer) { clearTimeout(this._chatSidebarSearchTimer); this._chatSidebarSearchTimer = 0; }
      this.chatSidebarSearchSeq = Number(this.chatSidebarSearchSeq || 0) + 1;
      this.chatSidebarSearchLoading = false;
      this.chatSidebarSearchError = '';
      this.chatSidebarSearchResults = [];
      this.scheduleSidebarScrollIndicators();
    },
    onChatSidebarQueryInput(value) {
      this.chatSidebarQuery = String(value || '');
      this.chatSidebarVisibleCount = Math.max(1, Math.floor(Number(this.chatSidebarVisibleBase || 7)));
      var query = String(this.chatSidebarQuery || '').trim();
      if (!query) {
        this.clearChatSidebarSearch();
        return;
      }
      this.scheduleChatSidebarSearch();
    },
    scheduleChatSidebarSearch() {
      var query = String(this.chatSidebarQuery || '').trim();
      if (!query) { this.clearChatSidebarSearch(); return; }
      if (this._chatSidebarSearchTimer) { clearTimeout(this._chatSidebarSearchTimer); this._chatSidebarSearchTimer = 0; }
      var self = this;
      var seq = Number(this.chatSidebarSearchSeq || 0) + 1;
      this.chatSidebarSearchSeq = seq;
      this.chatSidebarSearchLoading = true;
      this.chatSidebarSearchError = '';
      this._chatSidebarSearchTimer = setTimeout(function() { self._chatSidebarSearchTimer = 0; self.runChatSidebarSearch(seq); }, 140);
    },
    async runChatSidebarSearch(seq) {
      var token = Number(seq || 0);
      var currentToken = Number(this.chatSidebarSearchSeq || 0);
      if (token !== currentToken) return;
      var query = String(this.chatSidebarQuery || '').trim();
      if (!query) {
        this.clearChatSidebarSearch();
        return;
      }
      try {
        var path = '/api/search/conversations?q=' + encodeURIComponent(query) + '&limit=80';
        var payload = await InfringAPI.get(path);
        if (token !== Number(this.chatSidebarSearchSeq || 0)) return;
        var self = this;
        var serverRows = payload && Array.isArray(payload.sidebar_rows) ? payload.sidebar_rows : null;
        if (serverRows && serverRows.length) {
          this.chatSidebarSearchResults = serverRows.filter(function(agent) {
            return !self.isSidebarArchivedAgent(agent);
          }).map(function(agent) {
            return self.sanitizeSidebarAgentRow(agent);
          });
          this.chatSidebarSearchError = '';
          return;
        }
        var quickRows = payload && Array.isArray(payload.quick_actions) ? payload.quick_actions : [];
        this.chatSidebarSearchResults = quickRows.filter(function(agent) {
          return !self.isSidebarArchivedAgent(agent);
        }).map(function(agent) {
          return self.sanitizeSidebarAgentRow(agent);
        });
        this.chatSidebarSearchError = '';
      } catch (e) {
        if (token !== Number(this.chatSidebarSearchSeq || 0)) return;
        this.chatSidebarSearchResults = [];
        this.chatSidebarSearchError = String(e && e.message ? e.message : 'search_failed');
      } finally {
        if (token === Number(this.chatSidebarSearchSeq || 0)) {
          this.chatSidebarSearchLoading = false;
        }
        this.scheduleSidebarScrollIndicators();
      }
    },
    overlayGlassTemplateNormalized(modeRaw) {
      var mode = String(modeRaw || '').trim().toLowerCase();
      if (mode === 'simple-glass') return 'simple-glass';
      if (mode === 'fogged-glass') return 'fogged-glass';
      if (mode === 'warped-glass' || mode === 'magnified-glass') return 'warped-glass';
      if (mode === 'liquid-glass') return 'fogged-glass';
      return 'simple-glass';
    },
    applyOverlayGlassTemplate(modeRaw, persistRaw) {
      var mode = this.overlayGlassTemplateNormalized(modeRaw);
      this.overlayGlassTemplate = mode;
      var persist = persistRaw !== false;
      if (document && document.documentElement) {
        try {
          document.documentElement.setAttribute('data-overlay-glass-template', mode);
        } catch (_) {}
      }
      if (persist) {
        try { localStorage.setItem('infring-overlay-glass-template', mode); } catch (_) {}
      }
      return mode;
    },
    uiBackgroundTemplateNormalized(modeRaw) {
      var service = this.taskbarDockService ? this.taskbarDockService() : infringTaskbarDockService();
      if (service && typeof service.normalizeBackgroundTemplate === 'function') return service.normalizeBackgroundTemplate(modeRaw);
      var mode = String(modeRaw || '').trim().toLowerCase();
      if (mode === 'unsplash-paper') return 'default-grid';
      if (mode === 'default-grid') return 'default-grid';
      if (mode === 'light-wood') return 'light-wood';
      if (mode === 'sand') return 'sand';
      return 'default-grid';
    },
    applyUiBackgroundTemplate(modeRaw, persistRaw) {
      var mode = this.uiBackgroundTemplateNormalized(modeRaw);
      this.uiBackgroundTemplate = mode;
      var persist = persistRaw !== false;
      if (document && document.documentElement) {
        try {
          document.documentElement.setAttribute('data-ui-background-template', mode);
        } catch (_) {}
      }
      if (persist) {
        try {
          var service = this.taskbarDockService ? this.taskbarDockService() : infringTaskbarDockService();
          if (service && typeof service.writeDisplayBackground === 'function') service.writeDisplayBackground(mode);
          else {
            var rawDisplaySettings = localStorage.getItem('infring-display-settings') || '';
            var displaySettings = rawDisplaySettings ? JSON.parse(rawDisplaySettings) : {};
            displaySettings = displaySettings && typeof displaySettings === 'object' ? displaySettings : {};
            displaySettings.background = mode;
            localStorage.setItem('infring-display-settings', JSON.stringify(displaySettings));
          }
        } catch (_) {}
      }
      return mode;
    },
    beginInstantThemeFlip() {
      var self = this;
      var body = document && document.body ? document.body : null;
      if (!body) return;
      body.classList.add('theme-switching');
      // Force style flush so no-transition styles are applied before theme variables swap.
      void body.offsetHeight;
      if (this._themeSwitchReset) {
        clearTimeout(this._themeSwitchReset);
      }
      this._themeSwitchReset = window.setTimeout(function() {
        body.classList.remove('theme-switching');
        self._themeSwitchReset = 0;
      }, 260);
    },
    toggleTheme() {
      var modes = ['light', 'system', 'dark'];
      var next = modes[(modes.indexOf(this.themeMode) + 1) % modes.length];
      this.setTheme(next);
    },
    toggleSidebar() {
      if (typeof this.shouldSuppressSidebarToggle === 'function' && this.shouldSuppressSidebarToggle()) return;
      var nextCollapsed = !this.sidebarCollapsed;
      var resolveMessagesHost = function() {
        var nodes = document.querySelectorAll('#messages');
        for (var ni = 0; ni < nodes.length; ni++) if (nodes[ni] && nodes[ni].offsetParent !== null) return nodes[ni];
        return nodes && nodes.length ? nodes[0] : null;
      };
      var captureMessageBottomAnchor = function() {
        var host = resolveMessagesHost();
        if (!host || host.offsetParent === null) return null;
        var hostRect = host.getBoundingClientRect();
        var input = document.getElementById('msg-input');
        var alignY = hostRect.bottom;
        if (input && input.offsetParent !== null) {
          var inputRect = input.getBoundingClientRect();
          if (inputRect.top > hostRect.top && inputRect.top < (hostRect.bottom + 140)) alignY = inputRect.top;
        }
        var rows = host.querySelectorAll('.chat-message-block[id], .chat-message-block .message[id]');
        var best = null;
        var bestDiff = Number.POSITIVE_INFINITY;
        for (var i = 0; i < rows.length; i++) {
          var row = rows[i];
          if (!row || row.offsetParent === null) continue;
          var rect = row.getBoundingClientRect();
          if (rect.bottom < (hostRect.top - 40) || rect.top > (hostRect.bottom + 40)) continue;
          var diff = Math.abs(rect.bottom - alignY);
          if (diff < bestDiff) { bestDiff = diff; best = row; }
        }
        return best && best.id ? { id: String(best.id) } : null;
      };
      if (nextCollapsed) this._sidebarChatAnchorForExpand = captureMessageBottomAnchor();
      this.sidebarCollapsed = nextCollapsed;
      localStorage.setItem('infring-sidebar', this.sidebarCollapsed ? 'collapsed' : 'expanded');
      // Always clear stale sidebar popup when toggling sidebar state.
      this.hideDashboardPopupBySource('sidebar');
      if (!nextCollapsed) {
        var anchor = (this._sidebarChatAnchorForExpand && this._sidebarChatAnchorForExpand.id)
          ? this._sidebarChatAnchorForExpand
          : captureMessageBottomAnchor();
        this._sidebarChatAnchorForExpand = null;
        var passes = 4;
        var restoreAnchor = function() {
          var host = resolveMessagesHost();
          if (!host || host.offsetParent === null || !anchor || !anchor.id) return;
          var row = document.getElementById(anchor.id);
          if (!row || !host.contains(row) || row.offsetParent === null) return;
          var hostRect = host.getBoundingClientRect();
          var input = document.getElementById('msg-input');
          var alignY = hostRect.bottom;
          if (input && input.offsetParent !== null) {
            var inputRect = input.getBoundingClientRect();
            if (inputRect.top > hostRect.top && inputRect.top < (hostRect.bottom + 140)) alignY = inputRect.top;
          }
          var alignOffset = Math.max(0, Math.min(Math.max(0, Number(host.clientHeight || 0)), Math.round(alignY - hostRect.top)));
          var rowBottom = Number(row.offsetTop || 0) + Math.max(0, Number(row.offsetHeight || 0));
          var maxTop = Math.max(0, Number(host.scrollHeight || 0) - Math.max(0, Number(host.clientHeight || 0)));
          var nextTop = Math.max(0, Math.min(maxTop, Math.round(rowBottom - alignOffset)));
          host.scrollTop = nextTop;
          if (passes-- > 1 && typeof requestAnimationFrame === 'function') requestAnimationFrame(restoreAnchor);
          try { host.dispatchEvent(new Event('scroll')); } catch (_) {}
        };
        if (typeof requestAnimationFrame === 'function') requestAnimationFrame(restoreAnchor);
        else setTimeout(restoreAnchor, 0);
      }
      this.scheduleSidebarScrollIndicators();
    },
    runtimeFacadeHealthSummary() {
      var summary = this.healthSummary && typeof this.healthSummary === 'object' ? this.healthSummary : null;
      if (!summary) return null;
      var loadedAt = Number(this._healthSummaryLoadedAt || 0);
      if (loadedAt > 0 && (Date.now() - loadedAt) > 60000) return null;
      return summary;
    },
    runtimeFacadeState() {
      var store = this.getAppStore();
      var conn = this.normalizeConnectionIndicatorState(
        this.connectionIndicatorState ||
        ((store && store.connectionState) || this.connectionState || '')
      );
      if (conn === 'connecting') return 'connecting';
      if (conn === 'disconnected') return this.runtimeFacadeHealthSummary() ? 'connecting' : 'down';
      if (this.runtimeEtaSeconds() > 0) return 'active';
      return 'connected';
    },
    runtimeFacadeClass() {
      var state = this.runtimeFacadeState();
      if (state === 'connected' || state === 'active') return 'health-ok';
      if (state === 'connecting') return 'health-connecting';
      return 'health-down';
    },
    runtimeFacadeLabel() {
      var state = this.runtimeFacadeState();
      if (state === 'active') return 'Active';
      if (state === 'connected') {
        var store = this.getAppStore();
        var health = this.runtimeFacadeHealthSummary();
        var agents = ((store && store.agents && store.agents.length) || (store && store.agentCount) || this.agentCount || Number(health && health.agent_count || 0) || Number(health && health.agents && health.agents.length || 0));
        return String(agents) + ' agents';
      }
      if (state === 'connecting' && this.runtimeFacadeHealthSummary()) return 'Reconnecting...';
      if (state === 'connecting') return 'Connecting...';
      return 'Disconnected';
    },
    runtimeFacadeDisplayLabel() {
      var label = String(this.runtimeFacadeLabel() || '').trim();
      if (!label) return '';
      return label.replace(/\s+agents?$/i, '');
    },
    runtimeResponseP95Ms() {
      var store = this.getAppStore();
      var runtime = store && store.runtimeSync && typeof store.runtimeSync === 'object'
        ? store.runtimeSync
        : null;
      if (!runtime) {
        var health = this.runtimeFacadeHealthSummary();
        var durationMs = Number(health && health.durationMs);
        return Number.isFinite(durationMs) && durationMs >= 0 ? Math.round(durationMs) : null;
      }
      var facadeP95 = Number(runtime.facade_response_p95_ms);
      if (Number.isFinite(facadeP95) && facadeP95 > 0) return Math.round(facadeP95);
      var p95 = Number(runtime.receipt_latency_p95_ms);
      if (Number.isFinite(p95) && p95 > 0) return Math.round(p95);
      var p99 = Number(runtime.receipt_latency_p99_ms);
      if (Number.isFinite(p99) && p99 > 0) return Math.round(p99);
      return null;
    },
    runtimeConfidencePercent() {
      var store = this.getAppStore();
      var runtime = store && store.runtimeSync && typeof store.runtimeSync === 'object'
        ? store.runtimeSync
        : null;
      if (!runtime) return this.runtimeFacadeHealthSummary() ? 92 : 80;
      var facadeConfidence = Number(runtime.facade_confidence_percent);
      if (Number.isFinite(facadeConfidence) && facadeConfidence > 0) {
        return Math.max(10, Math.min(100, Math.round(facadeConfidence)));
      }

      var score = 100;
      var queueDepth = Number(runtime.queue_depth || 0);
      var stale = Number(runtime.cockpit_stale_blocks || 0);
      var gaps = Number(runtime.health_coverage_gap_count || 0);
      var conduitSignals = Number(runtime.conduit_signals || 0);
      var targetSignals = Math.max(1, Number(runtime.target_conduit_signals || 4));
      var benchmark = String(runtime.benchmark_sanity_cockpit_status || runtime.benchmark_sanity_status || 'unknown').toLowerCase();
      var spine = Number(runtime.spine_success_rate);

      if (queueDepth > 20) score -= Math.min(20, Math.floor((queueDepth - 20) / 2));
      if (stale > 0) score -= Math.min(20, stale * 2);
      if (gaps > 0) score -= Math.min(20, gaps * 6);
      if (conduitSignals < Math.max(3, Math.floor(targetSignals * 0.5))) score -= 12;
      if (benchmark === 'warn') score -= 8;
      if (benchmark === 'fail' || benchmark === 'error') score -= 20;
      if (Number.isFinite(spine)) {
        if (spine < 0.9) score -= 15;
        if (spine < 0.6) score -= 10;
      }

      score = Math.max(10, Math.min(100, Math.round(score)));
      return score;
    },
    runtimeEtaSeconds() {
      var store = this.getAppStore();
      var runtime = store && store.runtimeSync && typeof store.runtimeSync === 'object'
        ? store.runtimeSync
        : null;
      if (!runtime) return 0;
      var facadeEta = Number(runtime.facade_eta_seconds);
      if (Number.isFinite(facadeEta) && facadeEta >= 0) {
        return Math.max(0, Math.min(300, Math.round(facadeEta)));
      }
      var queueDepth = Math.max(0, Number(runtime.queue_depth || 0));
      if (queueDepth <= 0) return 0;
      // Conservative client-side estimate for "Active" mode only.
      return Math.max(1, Math.min(300, Math.ceil(queueDepth / 8)));
    },
    runtimeFacadeDetail() {
      var state = this.runtimeFacadeState();
      var store = this.getAppStore();
      var bootStage = String((store && store.bootStage) || '').trim();
      var stageSuffix = bootStage ? (' · ' + bootStage.replace(/_/g, ' ')) : '';
      if (state === 'connecting' && this.runtimeFacadeHealthSummary()) return 'HTTP health OK · reconnecting live runtime' + stageSuffix;
      if (state === 'connecting') return 'Establishing runtime link' + stageSuffix;
      if (state === 'down') return 'Runtime unavailable' + stageSuffix;
      var response = this.runtimeResponseP95Ms();
      var confidence = this.runtimeConfidencePercent();
      var health = this.runtimeFacadeHealthSummary();
      var agents = ((store && store.agents && store.agents.length) || (store && store.agentCount) || Number(health && health.agent_count || 0) || Number(health && health.agents && health.agents.length || 0));
      var base = 'Response ' + (response != null ? (response + 'ms') : '—') + ' · Confidence ' + confidence + '%';
      if (store && store.statusDegraded) {
        return base + ' · Status degraded' + stageSuffix;
      }
      if (state === 'active') {
        var eta = this.runtimeEtaSeconds();
        return (eta > 0 ? ('ETA ~' + eta + 's · ') : '') + base;
      }
      return base + ' · ' + agents + ' agent(s)';
    },
    runtimeFacadeTitle() {
      return this.runtimeFacadeLabel();
    },
    taskbarClockParts() {
      var tick = Number(this.clockTick || Date.now());
      var dt = new Date(tick);
      if (!Number.isFinite(dt.getTime())) dt = new Date();
      var dayNames = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
      var monthNames = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
      var dayName = dayNames[dt.getDay()] || '';
      var monthName = monthNames[dt.getMonth()] || '';
      var day = dt.getDate();
      var hours24 = dt.getHours();
      var minutes = dt.getMinutes();
      var suffix = hours24 >= 12 ? 'PM' : 'AM';
      var hours12 = hours24 % 12;
      if (hours12 === 0) hours12 = 12;
      var minuteText = minutes < 10 ? ('0' + minutes) : String(minutes);
      return {
        main: dayName + ' ' + monthName + ' ' + day + ' ' + hours12 + ':' + minuteText,
        meridiem: suffix
      };
    },
    taskbarClockMainLabel() {
      return this.taskbarClockParts().main;
    },
    taskbarClockMeridiemLabel() {
      return this.taskbarClockParts().meridiem;
    },
    taskbarClockLabel() {
      var parts = this.taskbarClockParts();
      return parts.main + ' ' + parts.meridiem;
    },
    toggleAgentChatsSidebar() {
      if (this.sidebarCollapsed) {
        this.sidebarCollapsed = false;
        localStorage.setItem('infring-sidebar', 'expanded');
      }
      this.hideDashboardPopupBySource('sidebar');
      this.scheduleSidebarScrollIndicators();
    },
    closeAgentChatsSidebar() {
      if (this.chatSidebarMode !== 'default') {
        this.chatSidebarMode = 'default';
        this.chatSidebarQuery = '';
        this.clearChatSidebarSearch();
      }
      this.confirmArchiveAgentId = '';
      this.scheduleSidebarScrollIndicators();
    },
    async applyBootChatSelection() {
      if (this.bootSelectionApplied) return;
      var store = this.getAppStore();
      if (!store || store.agentsLoading || !store.agentsHydrated) {
        return;
      }
      var rows = Array.isArray(store.agents) ? store.agents.slice() : [];
      if (!rows.length && typeof InfringAPI !== 'undefined' && InfringAPI && typeof InfringAPI.get === 'function') {
        try {
          var bootstrapRows = await InfringAPI.get('/api/agents?view=sidebar&authority=runtime&compact=1');
          if (Array.isArray(bootstrapRows) && bootstrapRows.length) {
            rows = bootstrapRows.filter(function(agent) {
              return !!(agent && agent.id);
            });
            store.agents = rows.slice();
            store.agentCount = rows.length;
          }
        } catch (_bootstrapError) {}
      }
      if (!rows.length) {
        this.bootSelectionApplied = true;
        if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(null);
        else store.activeAgentId = null;
        this.navigate('chat');
        this.chatSidebarQuery = '';
        this.clearChatSidebarSearch();
        return;
      }
      var target = null;
      if (store.activeAgentId) {
        var saved = String(store.activeAgentId);
        target = rows.find(function(agent) { return agent && String(agent.id) === saved; }) || null;
      }
      if (!target) {
        rows.sort(function(a, b) {
