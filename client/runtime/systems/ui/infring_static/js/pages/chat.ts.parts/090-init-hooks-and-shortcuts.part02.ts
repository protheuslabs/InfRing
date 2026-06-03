      } else {
        x = targetX;
        y = targetY;
      }
      s.x = x;
      s.y = y;
      s.vx = 0;
      s.vy = 0;
      s.trailX = x;
      s.trailY = y;
      s.anchorMode = 'thinking';
      s.anchorTargetX = targetX;
      s.anchorTargetY = targetY;
      s.anchorLastAt = now;
      this._agentTrailState = s;
      this._agentTrailSeeded = true;
      this._agentTrailLastDotAt = now;
      if (enteredThinking && this._agentTrailOrbEl) {
        // Promote + mark listening before reposition so ensureAgentTrailOrb
        // performs the teleport path instead of easing from the last spot.
        this.setAgentTrailBlinkState(true, this._agentTrailOrbEl);
      }
      var orb = this.ensureAgentTrailOrb(host, x, y);
      this.setAgentTrailBlinkState(true, orb);
      host.style.setProperty('--chat-agent-grid-active', '1');
      host.style.setProperty('--chat-agent-grid-x', Math.round(x) + 'px');
      host.style.setProperty('--chat-agent-grid-y', Math.round(y) + 'px');
      this._agentTrailLastAt = now;
      return true;
    },
    anchorAgentTrailToFreshInit(host, hostRect, now, pad, w, h) {
      if (!host || typeof host.querySelector !== 'function') return false;
      if (!this.showFreshArchetypeTiles || !this.freshInitRevealMenu) return false;
      // Never override active thinking positioning during init.
      var activeThinking = host.querySelector('.message.thinking .message-bubble.message-bubble-thinking');
      if (activeThinking && activeThinking.offsetParent !== null) return false;
      var panel = host.querySelector('.chat-init-panel');
      if (!panel || panel.offsetParent === null) return false;
      var rect = hostRect && Number.isFinite(Number(hostRect.width || 0)) ? hostRect : host.getBoundingClientRect();
      var panelRect = panel.getBoundingClientRect();
      if (!(Number(panelRect.width || 0) > 0 && Number(panelRect.height || 0) > 0)) return false;
      if (panelRect.bottom < rect.top || panelRect.top > rect.bottom || panelRect.right < rect.left || panelRect.left > rect.right) return false;
      // During agent initialization, pin the orb to the initial agent chat panel.
      // Keep it 1rem outside the panel's bottom-left corner.
      var anchor = {
        x: (panelRect.left - rect.left) - 16,
        y: (panelRect.bottom - rect.top) + 16,
      };
      var x = Math.max(pad + 1, Math.min(w - (pad + 1), Number(anchor.x || 0)));
      var y = Math.max(pad + 1, Math.min(h - (pad + 1), Number(anchor.y || 0)));
      var orb = this.ensureAgentTrailOrb(host, x, y);
      this.setAgentTrailBlinkState(true, orb);
      host.style.setProperty('--chat-agent-grid-active', '1');
      host.style.setProperty('--chat-agent-grid-x', Math.round(x) + 'px');
      host.style.setProperty('--chat-agent-grid-y', Math.round(y) + 'px');
      this._agentTrailState = { x: x, y: y, vx: 0, vy: 0, dir: 0, target: 0, turnAt: now + 1000 };
      this._agentTrailSeeded = false;
      this._agentTrailLastAt = now;
      return true;
    },

    get filteredModelPicker() {
      if (!this.modelPickerFilter) return this.modelPickerList.slice(0, 15);
      var f = this.modelPickerFilter;
      return this.modelPickerList.filter(function(m) {
        return m.id.toLowerCase().indexOf(f) !== -1 || (m.display_name || '').toLowerCase().indexOf(f) !== -1 || m.provider.toLowerCase().indexOf(f) !== -1;
      }).slice(0, 15);
    },
    pickModel(modelId) {
      this.showModelPicker = false;
      this.inputText = '/model ' + modelId;
      this.sendMessage();
    },

    loadModelCatalogSafely: function(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var preferCached = opts.prefer_cached !== false;
      var suppressErrors = opts.suppress_errors === true;
      var self = this;
      return InfringAPI.get('/api/shell-socket/models').then(function(data) {
        var models = self.sanitizeModelCatalogRows((data && data.models) || []);
        self._modelCache = models;
        self._modelCacheTime = Date.now();
        self.modelPickerList = models;
        return models;
      }).catch(function(error) {
        var fallback = preferCached ? self.sanitizeModelCatalogRows(self._modelCache || []) : [];
        if (fallback.length) {
          self._modelCache = fallback;
          self.modelPickerList = fallback;
          return fallback;
        }
        if (typeof self.loadProviderModelCatalogSafely === 'function') {
          return self.loadProviderModelCatalogSafely({
            merge_existing: true
          }).then(function(providerModels) {
            if (providerModels.length) return providerModels;
            if (suppressErrors) return [];
            throw error;
          });
        }
        if (suppressErrors) return [];
        throw error;
      });
    },

    describeModelDiscoveryResult: function(resp, catalogRows) {
      var provider = String((resp && resp.provider) || '').trim();
      var inputKind = String((resp && resp.input_kind) || '').trim().toLowerCase();
      var discoveredCount = Number((resp && resp.model_count) || ((resp && resp.models && resp.models.length) || 0));
      if (!Number.isFinite(discoveredCount) || discoveredCount < 0) discoveredCount = 0;
      var availableRows = Array.isArray(catalogRows) ? catalogRows : [];
      var availableCount = this.availableModelRowsCount ? this.availableModelRowsCount(availableRows) : availableRows.length;
      var prefix = '';
      if (inputKind === 'local_path') {
        prefix = provider
          ? ('Indexed local path for `' + provider + '`')
          : 'Indexed local path';
      } else {
        prefix = provider
          ? ('Added provider `' + provider + '`')
          : 'Saved model discovery input';
      }
      prefix += ' (' + discoveredCount + ' discovered';
      if (availableCount > 0) {
        prefix += ', ' + availableCount + ' available now';
      }
      prefix += ').';
      return prefix;
    },

    toggleModelSwitcher() {
      if (this.showModelSwitcher) { this.showModelSwitcher = false; return; }
      var self = this;
      var now = Date.now();
      if (typeof this.closeComposerMenus === 'function') this.closeComposerMenus({ model: true });
      else {
        this.showAttachMenu = false;
        this.closeGitTreeMenu();
      }
      this.modelApiKeyStatus = '';
      var cached = self.sanitizeModelCatalogRows(self._modelCache || []);
      if (cached.length) {
        self._modelCache = cached;
        self.modelPickerList = cached;
      }
      this.modelSwitcherFilter = '';
      this.modelSwitcherProviderFilter = '';
      this.modelSwitcherIdx = 0;
      this.showModelSwitcher = true;
      this.$nextTick(function() {
        var el = document.getElementById('model-switcher-search');
        if (el) el.focus();
      });

      if (!cached.length && typeof self.loadProviderModelCatalogSafely === 'function') {
        self.loadProviderModelCatalogSafely({
          merge_existing: true
        }).catch(function() { return []; });
      }

      var cacheFresh = Array.isArray(this._modelCache) && (now - this._modelCacheTime) < 300000;
      var cachedAvailable = self.availableModelRowsCount ? self.availableModelRowsCount(cached) : 0;
      var shouldRefresh = !cacheFresh || cached.length < 8 || cachedAvailable < 4;
      if (!shouldRefresh) return;
      self.refreshModelCatalogAndGuidance({ discover: true, guidance: true }).catch(function(e) {
        return self.loadModelCatalogSafely({
          prefer_cached: true,
          suppress_errors: true
        }).then(function(models) {
          if (!models.length && (!self.modelPickerList || !self.modelPickerList.length)) {
            var active = self.resolveActiveSwitcherModel([]);
            self.modelPickerList = active ? [active] : [];
          }
          self.modelApiKeyStatus = models.length
            ? 'Unable to refresh model list (showing cached entries)'
            : 'Unable to refresh model list right now';
          InfringToast.error('Failed to refresh models: ' + e.message);
        });
      });
    },

    fallbackRuntimeEngineRows: function() {
      return [
        { engine_id: 'infring_native', display_name: 'InfRing Native', status: 'available', selectable: true, engine_kind: 'native_orchestration' },
        { engine_id: 'codex_cli', display_name: 'Codex CLI', status: 'not_downloaded', selectable: false, download_available: true, display_when_missing: 'download_icon' },
        { engine_id: 'claude_code', display_name: 'Claude Code', status: 'not_downloaded', selectable: false, download_available: true, display_when_missing: 'download_icon' },
        { engine_id: 'openhands', display_name: 'OpenHands', status: 'not_downloaded', selectable: false, download_available: true, display_when_missing: 'download_icon' }
      ];
    },

    normalizeRuntimeEngineRows: function(payload) {
      var rows = payload && Array.isArray(payload.engines) ? payload.engines : [];
      var out = [];
      for (var i = 0; i < rows.length; i += 1) {
        var row = rows[i] && typeof rows[i] === 'object' ? rows[i] : {};
        var id = String(row.engine_id || '').trim();
        if (!id) continue;
        out.push({
          engine_id: id,
          display_name: String(row.display_name || id).trim() || id,
          engine_kind: String(row.engine_kind || '').trim(),
          transport_kind: String(row.transport_kind || '').trim(),
          status: String(row.status || 'unknown').trim(),
          selectable: row.selectable !== false && ['not_downloaded', 'not_configured', 'planned_adapter'].indexOf(String(row.status || '').trim()) < 0,
          capabilities: Array.isArray(row.capabilities) ? row.capabilities : [],
          download_available: row.download_available === true,
          install_action_available: row.install_action_available === true || row.command_line_install_available === true,
          command_line_install_available: row.command_line_install_available === true,
          install_permission_state: String(row.install_permission_state || '').trim(),
          download_action_ref: String(row.download_action_ref || '').trim(),
          preferred_install_method: String(row.preferred_install_method || '').trim(),
          command_line_hint: String(row.command_line_hint || '').trim(),
          browser_fallback_url: String(row.browser_fallback_url || '').trim(),
          display_when_missing: String(row.display_when_missing || '').trim(),
          version_preview: String(row.version_preview || '').trim()
        });
      }
      return out.length ? out : this.fallbackRuntimeEngineRows();
    },

    loadRuntimeEnginesSafely: function(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var self = this;
      if (!opts.force && Array.isArray(this.runtimeEngineRows) && this.runtimeEngineRows.length && (Date.now() - Number(this.runtimeEngineCacheTime || 0)) < 120000) {
        return Promise.resolve(this.runtimeEngineRows);
      }
      this.runtimeEngineLoading = true;
      this.runtimeEngineError = '';
      return InfringAPI.get('/api/shell-socket/agent-runtime/engines').then(function(payload) {
        var rows = self.normalizeRuntimeEngineRows(payload);
        self.runtimeEngineRows = rows;
        self.runtimeEngineCacheTime = Date.now();
        var restoredLocalSelection = typeof self.restoreAgentRuntimeEngineSelection === 'function' ? self.restoreAgentRuntimeEngineSelection() : false;
        var serverSelected = String(payload && (payload.active_engine_id || payload.selected_default_engine_id) || '').trim();
        if (!restoredLocalSelection && serverSelected) self.selectedAgentRuntimeEngineId = serverSelected;
        return rows;
      }).catch(function(e) {
        self.runtimeEngineError = e && e.message ? String(e.message) : 'runtime_engines_unavailable';
        self.runtimeEngineRows = self.runtimeEngineRows && self.runtimeEngineRows.length ? self.runtimeEngineRows : self.fallbackRuntimeEngineRows();
        if (typeof self.restoreAgentRuntimeEngineSelection === 'function') self.restoreAgentRuntimeEngineSelection();
        return self.runtimeEngineRows;
      }).finally(function() {
        self.runtimeEngineLoading = false;
      });
    },

    toggleRuntimeSwitcher: function() {
      if (this.showRuntimeSwitcher) { this.showRuntimeSwitcher = false; return; }
      if (typeof this.closeComposerMenus === 'function') this.closeComposerMenus({ runtime: true });
      else {
        this.showAttachMenu = false;
        this.showModelSwitcher = false;
        this.closeGitTreeMenu();
      }
      this.showRuntimeSwitcher = true;
      this.loadRuntimeEnginesSafely({ force: true }).catch(function() { return []; });
    },

    activeRuntimeEngineRow: function() {
      if (typeof this.restoreAgentRuntimeEngineSelection === 'function') this.restoreAgentRuntimeEngineSelection();
      var id = String(this.selectedAgentRuntimeEngineId || 'infring_native').trim();
      var rows = Array.isArray(this.runtimeEngineRows) && this.runtimeEngineRows.length ? this.runtimeEngineRows : this.fallbackRuntimeEngineRows();
      for (var i = 0; i < rows.length; i += 1) {
        if (String(rows[i] && rows[i].engine_id || '') === id) return rows[i];
      }
      return rows[0] || null;
    },

    restoreAgentRuntimeEngineSelection: function() {
      if (this._agentRuntimeEngineSelectionRestored === true) return this._agentRuntimeEngineSelectionSource === 'local_storage';
      this._agentRuntimeEngineSelectionRestored = true;
      var key = String(this.agentRuntimeEngineStorageKey || 'infring-selected-agent-runtime-engine-v1');
      var saved = '';
      try {
        if (typeof window !== 'undefined' && window.localStorage) saved = String(window.localStorage.getItem(key) || '').trim();
      } catch (_e) {}
      if (!saved) return false;
      this.selectedAgentRuntimeEngineId = saved;
      this._agentRuntimeEngineSelectionSource = 'local_storage';
      return true;
    },

    persistAgentRuntimeEngineSelection: function(engineId) {
      var id = String(engineId || 'infring_native').trim() || 'infring_native';
      this.selectedAgentRuntimeEngineId = id;
      try { window.localStorage.setItem(this.agentRuntimeEngineStorageKey, id); } catch (_e) {}
      InfringAPI.post('/api/shell-socket/agent-runtime/selection', { engine_id: id }).catch(function() {});
      return id;
    },

    runtimeEngineDisplayNameForId: function(engineId, fallbackRow) {
      var id = String(engineId || '').trim() || 'infring_native';
      if (fallbackRow && String(fallbackRow.engine_id || '').trim() === id) {
        return String(fallbackRow.display_name || id).trim() || id;
      }
      var rows = Array.isArray(this.runtimeEngineRows) && this.runtimeEngineRows.length ? this.runtimeEngineRows : this.fallbackRuntimeEngineRows();
      for (var i = 0; i < rows.length; i += 1) {
        if (String(rows[i] && rows[i].engine_id || '').trim() === id) {
          return String(rows[i].display_name || id).trim() || id;
        }
      }
      return id === 'infring_native' ? 'InfRing Native' : id;
    },

    addRuntimeEngineSwitchNotice: function(previousEngineId, nextEngineId, nextRow) {
      var previousId = String(previousEngineId || 'infring_native').trim() || 'infring_native';
      var nextId = String(nextEngineId || 'infring_native').trim() || 'infring_native';
      if (previousId === nextId) return;
      if (typeof this.addNoticeEvent !== 'function') return;
      var fromName = this.runtimeEngineDisplayNameForId(previousId, null);
      var toName = this.runtimeEngineDisplayNameForId(nextId, nextRow);
      this.addNoticeEvent({
        notice_label: 'Changed active engine from ' + fromName + ' to ' + toName,
        notice_type: 'info',
        ts: Date.now()
      });
    },

    runtimeEngineMenuLabel: function() {
      var row = this.activeRuntimeEngineRow();
      return String((row && row.display_name) || 'InfRing Native').trim();
    },

    isRuntimeEngineActive: function(row) {
      return String(row && row.engine_id || '') === String(this.selectedAgentRuntimeEngineId || 'infring_native');
    },

    runtimeEngineStatusLabel: function(row) {
      var status = String(row && row.status || '').trim();
      if (status === 'available') return 'Available';
      if (status === 'adapter_ready') return 'Adapter ready';
      if (status === 'not_downloaded') return 'Not installed';
      if (status === 'not_connected') return 'Not connected';
      if (status === 'planned_adapter') return 'Planned';
      return status || 'Unknown';
    },

    runtimeEngineMeta: function(row) {
      var parts = [];
      var kind = String(row && row.engine_kind || '').replace(/_/g, ' ').trim();
      var status = this.runtimeEngineStatusLabel(row);
      if (kind) parts.push(kind);
      if (status) parts.push(status);
      return parts.join(' · ');
    },

    runtimeEngineActionIcon: function(row) {
      if (this.isRuntimeEngineActive(row)) return '✓';
      var status = String(row && row.status || '').trim();
      if (row && row.download_available && ['available', 'adapter_ready'].indexOf(status) < 0) return '⇩';
      if (row && String(row.status || '') === 'not_connected') return '!';
      return '';
    },

    selectRuntimeEngine: function(row) {
      var engineId = String(row && row.engine_id || '').trim();
      if (!engineId) return;
      if (row && row.selectable === false) {
        if (row.install_action_available || row.command_line_install_available || row.preferred_install_method === 'command_line') {
          this.installRuntimeEngine(row);
          return;
        }
        this.openRuntimeEngineInstall(row);
        return;
      }
      var previousEngineId = String(this.selectedAgentRuntimeEngineId || 'infring_native').trim() || 'infring_native';
      this.persistAgentRuntimeEngineSelection(engineId);
      this.showRuntimeSwitcher = false;
      this.addRuntimeEngineSwitchNotice(previousEngineId, engineId, row);
      InfringToast.success('Agent runtime: ' + String((row && row.display_name) || engineId));
    },

    installRuntimeEngine: function(row) {
      var engineId = String(row && row.engine_id || '').trim();
      if (!engineId) return Promise.resolve(null);
      var previousEngineId = String(this.selectedAgentRuntimeEngineId || 'infring_native').trim() || 'infring_native';
      var self = this;
      var label = String((row && row.display_name) || engineId);
      this.showRuntimeSwitcher = false;
      InfringToast.info('Installing agent runtime: ' + label);
      return InfringAPI.post('/api/shell-socket/agent-runtime/engines/' + encodeURIComponent(engineId) + '/install', {
        engine_id: engineId
      }).then(function(payload) {
        var status = String(payload && payload.status || '').trim();
        if (payload && payload.ok && (status === 'installed_available' || status === 'already_available')) {
          self.persistAgentRuntimeEngineSelection(engineId);
          self.addRuntimeEngineSwitchNotice(previousEngineId, engineId, row);
          InfringToast.success('Agent runtime ready: ' + label);
          return self.loadRuntimeEnginesSafely({ force: true });
        }
        if (status === 'permission_required') {
          InfringToast.error('Install permission is required for ' + label);
          return payload;
        }
        if (payload && payload.command_line_hint) InfringToast.info(String(payload.command_line_hint));
        if (payload && payload.browser_fallback_url && status !== 'command_line_installer_unavailable_for_platform') {
          try { window.open(String(payload.browser_fallback_url), '_blank', 'noopener,noreferrer'); } catch (_e) {}
        } else {
          InfringToast.error('Runtime install did not complete: ' + (status || 'unknown'));
        }
        return payload;
      }).catch(function(e) {
        InfringToast.error(e && e.message ? String(e.message) : 'runtime install failed');
        return null;
      });
    },

    openRuntimeEngineInstall: function(row) {
      var hint = String(row && row.command_line_hint || '').trim();
      var preferred = String(row && row.preferred_install_method || '').trim();
      var url = String(row && row.browser_fallback_url || '').trim();
      if (hint) {
        InfringToast.info(hint);
        if (preferred === 'command_line') return;
      }
      if (url && preferred !== 'command_line') {
        try { window.open(url, '_blank', 'noopener,noreferrer'); } catch (_e) {}
      } else if (!hint) {
        InfringToast.info('Runtime install path is not configured yet.');
      }
    },

    discoverModelsFromApiKey: function() {
      var self = this;
      var entry = String(this.modelApiKeyInput || '').trim();
      if (!entry) {
        InfringToast.error('Enter an API key or local model path first');
        return;
      }
      this.modelApiKeySaving = true;
      this.modelApiKeyStatus = 'Detecting...';
      InfringAPI.post('/api/shell-socket/models/discover', {
        input: entry,
        api_key: entry
      }).then(function(resp) {
        var provider = String((resp && resp.provider) || '').trim();
        var inputKind = String((resp && resp.input_kind) || '').trim().toLowerCase();
        var count = Number((resp && resp.model_count) || ((resp && resp.models && resp.models.length) || 0));
        self.modelApiKeyInput = '';
        if (inputKind === 'local_path') {
          self.modelApiKeyStatus = provider
            ? ('Indexed local path to ' + provider + ' (' + count + ' models)')
            : ('Indexed local path (' + count + ' models)');
        } else {
          self.modelApiKeyStatus = provider ? ('Added ' + provider + ' (' + count + ' models)') : 'API key saved';
        }
        self._modelCache = null;
        self._modelCacheTime = 0;
        return self.loadModelCatalogSafely({
          prefer_cached: false,
          suppress_errors: false
        }).then(function(models) {
          self.modelApiKeyStatus = self.describeModelDiscoveryResult(resp, models);
          return models;
        });
      }).then(function(models) {
        if (self.availableModelRowsCount(models) === 0) {
          self.injectNoModelsGuidance('discover_key');
        }
      }).catch(function(e) {
        self.modelApiKeyStatus = '';
        InfringToast.error('Model discovery failed: ' + (e && e.message ? e.message : e));
      }).finally(function() {
        self.modelApiKeySaving = false;
      });
    },

    resolveModelContextWindowForSwitch: function(targetModelRef) {
      var modelId = '';
      var explicitWindow = 0;
      if (targetModelRef && typeof targetModelRef === 'object') {
        modelId = String(
          targetModelRef.id || targetModelRef.model || targetModelRef.model_name || ''
        ).trim();
        explicitWindow = Number(
          targetModelRef.context_window || targetModelRef.context_window_tokens || 0
        );
      } else {
        modelId = String(targetModelRef || '').trim();
      }
      if (Number.isFinite(explicitWindow) && explicitWindow > 0) {
        return Math.round(explicitWindow);
      }
      var map = this._contextWindowByModel || {};
      var candidates = [];
      if (modelId) {
