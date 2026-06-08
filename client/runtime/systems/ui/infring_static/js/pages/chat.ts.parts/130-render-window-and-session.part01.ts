    isFreshInitTemplateSelected(templateDef) {
      if (!templateDef) return false;
      var key = String(templateDef.name || '').trim();
      return !!key && key === String(this.freshInitTemplateName || '').trim();
    },

    freshInitTemplateDescription: function(templateDef) {
      if (!templateDef) return '';
      if (templateDef.is_other) {
        var typed = String(this.freshInitOtherPrompt || '').trim();
        if (typed) return this.truncateFreshInitSummary(typed, 86);
      }
      return String(templateDef.description || '').trim();
    },

    truncateFreshInitSummary: function(text, limit) {
      var clean = String(text || '').replace(/\s+/g, ' ').trim();
      if (!clean) return '';
      var max = Number(limit || 0);
      if (!Number.isFinite(max) || max < 12) max = 80;
      if (clean.length <= max) return clean;
      return clean.slice(0, Math.max(8, max - 1)).trimEnd() + '…';
    },

    filteredFreshInitEmojiCatalog: function() {
      var source = Array.isArray(this.drawerEmojiCatalog) ? this.drawerEmojiCatalog : [];
      var query = String(this.freshInitEmojiSearch || '').trim().toLowerCase();
      var self = this;
      var rows = source.filter(function(row) {
        var emoji = String((row && row.emoji) || '');
        if (self.isReservedSystemEmoji && self.isReservedSystemEmoji(emoji)) return false;
        return true;
      });
      if (!query) return rows.slice(0, 24);
      return rows.filter(function(row) {
        var emoji = String((row && row.emoji) || '');
        var name = String((row && row.name) || '').toLowerCase();
        return emoji.indexOf(query) >= 0 || name.indexOf(query) >= 0;
      }).slice(0, 24);
    },

    toggleFreshInitEmojiPicker: function() {
      this.freshInitEmojiPickerOpen = !this.freshInitEmojiPickerOpen;
      if (!this.freshInitEmojiPickerOpen) {
        this.freshInitEmojiSearch = '';
      }
    },

    selectFreshInitEmoji: function(choice) {
      var emoji = String(choice && choice.emoji ? choice.emoji : choice || '').trim();
      if (!emoji) return;
      var sanitized = this.sanitizeAgentEmojiForDisplay
        ? this.sanitizeAgentEmojiForDisplay(this.currentAgent, emoji)
        : emoji;
      if (!sanitized) {
        InfringToast.info('The gear icon is reserved for the System thread.');
        return;
      }
      this.freshInitEmoji = sanitized;
      this.freshInitAvatarUrl = '';
      this.freshInitEmojiPickerOpen = false;
      this.freshInitEmojiSearch = '';
    },

    openFreshInitAvatarPicker: function() {
      if (this.$refs && this.$refs.freshInitAvatarInput) {
        this.$refs.freshInitAvatarInput.click();
      }
    },

    uploadFreshInitAvatar: async function(fileList) {
      if (!this.currentAgent || !this.currentAgent.id) return;
      var files = Array.isArray(fileList) ? fileList : Array.from(fileList || []);
      if (!files.length) return;
      var file = files[0];
      if (!file) return;
      var mime = String(file.type || '').toLowerCase();
      if (mime && mime.indexOf('image/') !== 0) {
        InfringToast.error('Avatar must be an image file.');
        return;
      }
      this.freshInitAvatarUploading = true;
      this.freshInitAvatarUploadError = '';
      try {
        var headers = {
          'Content-Type': file.type || 'application/octet-stream',
          'X-Filename': file.name || 'avatar'
        };
        var token = (typeof InfringAPI !== 'undefined' && typeof InfringAPI.getToken === 'function')
          ? String(InfringAPI.getToken() || '')
          : '';
        if (token) headers.Authorization = 'Bearer ' + token;
        var response = await fetch('/api/agents/' + encodeURIComponent(this.currentAgent.id) + '/avatar', {
          method: 'POST',
          headers: headers,
          body: file
        });
        var payload = null;
        try {
          payload = await response.json();
        } catch (_) {
          payload = null;
        }
        if (!response.ok || !payload || !payload.ok || !payload.avatar_url) {
          throw new Error(String(payload && payload.error ? payload.error : 'avatar_upload_failed'));
        }
        this.freshInitAvatarUrl = String(payload.avatar_url || '').trim();
        this.freshInitEmojiPickerOpen = false;
        this.freshInitEmojiSearch = '';
      } catch (error) {
        var message = (error && error.message) ? String(error.message) : 'avatar_upload_failed';
        this.freshInitAvatarUploadError = message;
        InfringToast.error('Failed to upload avatar: ' + message);
      } finally {
        this.freshInitAvatarUploading = false;
      }
    },

    clearFreshInitAvatar: function() {
      this.freshInitAvatarUrl = '';
      this.freshInitAvatarUploadError = '';
    },

    isFreshInitPersonalitySelected: function(card) {
      if (!card) return false;
      return String(card.id || '') === String(this.freshInitPersonalityId || '');
    },

    selectFreshInitPersonality: function(card) {
      var id = String(card && card.id ? card.id : 'none').trim() || 'none';
      this.freshInitPersonalityId = id;
      if (typeof this.scheduleFreshInitProgressAnchor === 'function') this.scheduleFreshInitProgressAnchor();
    },

    selectedFreshInitPersonality: function() {
      var cards = Array.isArray(this.freshInitPersonalityCards) ? this.freshInitPersonalityCards : [];
      var selectedId = String(this.freshInitPersonalityId || 'none');
      for (var i = 0; i < cards.length; i += 1) {
        if (String(cards[i] && cards[i].id ? cards[i].id : '') === selectedId) return cards[i];
      }
      return cards.length ? cards[0] : null;
    },

    isFreshInitLifespanSelected: function(card) {
      if (!card) return false;
      return String(card.id || '') === String(this.freshInitLifespanId || '');
    },

    selectFreshInitLifespan: function(card) {
      var id = String(card && card.id ? card.id : '1h').trim() || '1h';
      this.freshInitLifespanId = id;
      if (typeof this.scheduleFreshInitProgressAnchor === 'function') this.scheduleFreshInitProgressAnchor();
    },

    selectedFreshInitLifespan: function() {
      var cards = Array.isArray(this.freshInitLifespanCards) ? this.freshInitLifespanCards : [];
      var selectedId = String(this.freshInitLifespanId || '1h');
      var fallback = null;
      for (var i = 0; i < cards.length; i += 1) {
        var cardId = String(cards[i] && cards[i].id ? cards[i].id : '');
        if (cardId === '1h') fallback = cards[i];
        if (cardId === selectedId) return cards[i];
      }
      return fallback || (cards.length ? cards[0] : null);
    },

    async applyChatArchetypeTemplate(templateDef) {
      if (!templateDef) return;
      this.freshInitTemplateDef = templateDef;
      this.freshInitTemplateName = String(templateDef.name || '').trim();
      this.freshInitModelManual = false;
      this.freshInitModelSelection = '';
      this.refreshFreshInitModelSuggestions(templateDef);
      if (templateDef.is_other) {
        this.freshInitAwaitingOtherPrompt = true;
        this.focusChatComposerFromInit(String(this.freshInitOtherPrompt || '').trim());
      } else {
        this.freshInitAwaitingOtherPrompt = false;
      }
      if (typeof this.scheduleFreshInitProgressAnchor === 'function') this.scheduleFreshInitProgressAnchor();
    },

    captureFreshInitOtherPrompt: function() {
      if (!this.showFreshArchetypeTiles || !this.freshInitAwaitingOtherPrompt) return false;
      if (Array.isArray(this.attachments) && this.attachments.length > 0) {
        InfringToast.info('Init prompt does not support file attachments.');
        return false;
      }
      var text = String(this.inputText || '').trim();
      if (!text) {
        InfringToast.info('Describe the special purpose first.');
        this.focusChatComposerFromInit('');
        return false;
      }
      this.freshInitOtherPrompt = text;
      this.freshInitAwaitingOtherPrompt = false;
      this.inputText = '';
      var ta = document.getElementById('msg-input');
      if (ta) ta.style.height = '';
      if (typeof this.scheduleFreshInitProgressAnchor === 'function') this.scheduleFreshInitProgressAnchor('lifespan');
      return true;
    },

    resolveFreshInitSystemPrompt: function(templateDef, agentName, personalityCard, vibeCard) {
      if (!templateDef) return '';
      var basePrompt = '';
      if (templateDef.is_other) {
        var purpose = String(this.freshInitOtherPrompt || '').trim();
        basePrompt = [
          'You are ' + String(agentName || 'the assistant') + '.',
          'Special purpose: ' + purpose,
          'Act as a focused specialist for this purpose. Stay concise, practical, and reliable.',
        ].join('\n');
      } else {
        basePrompt = String(templateDef.system_prompt || '').trim();
      }
      var personalitySuffix = String(personalityCard && personalityCard.system_suffix ? personalityCard.system_suffix : '').trim();
      var vibeSuffix = String(vibeCard && vibeCard.system_suffix ? vibeCard.system_suffix : '').trim();
      var suffixes = [];
      if (personalitySuffix) suffixes.push(personalitySuffix);
      if (vibeSuffix) suffixes.push(vibeSuffix);
      if (suffixes.length) {
        return (basePrompt ? (basePrompt + '\n\n') : '') + suffixes.join('\n');
      }
      return basePrompt;
    },

    resolveFreshInitRole: function(templateDef) {
      var currentRole = String((this.currentAgent && this.currentAgent.role) || '').trim().toLowerCase();
      if (!templateDef) return currentRole || 'analyst';
      var hint = String(
        templateDef.role || templateDef.archetype || templateDef.profile || templateDef.name || ''
      ).trim().toLowerCase();
      if (!hint) return currentRole || 'analyst';
      if (hint.indexOf('teacher') >= 0 || hint.indexOf('tutor') >= 0 || hint.indexOf('mentor') >= 0 || hint.indexOf('coach') >= 0 || hint.indexOf('instructor') >= 0) {
        return 'tutor';
      }
      if (hint.indexOf('code') >= 0 || hint.indexOf('coder') >= 0 || hint.indexOf('engineer') >= 0 || hint.indexOf('developer') >= 0 || hint.indexOf('devops') >= 0 || hint.indexOf('api') >= 0 || hint.indexOf('build') >= 0) {
        return 'engineer';
      }
      if (hint.indexOf('research') >= 0 || hint.indexOf('investig') >= 0) {
        return 'researcher';
      }
      if (hint.indexOf('analyst') >= 0 || hint.indexOf('analysis') >= 0 || hint.indexOf('data') >= 0 || hint.indexOf('meeting') >= 0) {
        return 'analyst';
      }
      if (hint.indexOf('writer') >= 0 || hint.indexOf('editor') >= 0 || hint.indexOf('content') >= 0) {
        return 'writer';
      }
      if (hint.indexOf('design') >= 0 || hint.indexOf('ui') >= 0 || hint.indexOf('ux') >= 0) {
        return 'designer';
      }
      if (hint.indexOf('support') >= 0) {
        return 'support';
      }
      return currentRole || 'analyst';
    },

    resolveFreshInitContractPayload: function(agentName) {
      var selected = this.selectedFreshInitLifespan();
      var mission = 'Initialize and run as ' + String(agentName || 'agent') + '.';
      if (!selected) {
        return {
          mission: mission,
          termination_condition: 'task_or_timeout',
          expiry_seconds: 60 * 60,
          indefinite: false,
          auto_terminate_allowed: true,
          idle_terminate_allowed: true,
        };
      }
      var terminationCondition = String(selected.termination_condition || 'task_or_timeout');
      var expirySeconds = selected.expiry_seconds == null ? null : Number(selected.expiry_seconds);
      var indefinite = selected.indefinite === true;
      var supportsTimeout = terminationCondition === 'timeout' || terminationCondition === 'task_or_timeout';
      return {
        mission: mission,
        termination_condition: terminationCondition,
        expiry_seconds: expirySeconds,
        indefinite: indefinite,
        auto_terminate_allowed: !indefinite && supportsTimeout && expirySeconds != null,
        idle_terminate_allowed: !indefinite && supportsTimeout && expirySeconds != null,
      };
    },

    async launchFreshAgentInitialization() {
      if (!this.currentAgent || !this.currentAgent.id) return;
      if (this.freshInitLaunching) return;
      if (!this.freshInitTemplateDef) {
        InfringToast.info('Select an archetype first.');
        return;
      }
      var agentId = this.currentAgent.id;
      var templateDef = this.freshInitTemplateDef;
      var provider = String(templateDef.provider || '').trim();
      var model = String(templateDef.model || '').trim();
      var selectedModel = this.selectedFreshInitModelSuggestion();
      var selectedModelRef = this.normalizeFreshInitModelRef(selectedModel);
      if (!selectedModelRef) {
        await this.refreshFreshInitModelSuggestions(templateDef);
        selectedModel = this.selectedFreshInitModelSuggestion();
        selectedModelRef = this.normalizeFreshInitModelRef(selectedModel);
      }
      var resolvedModelRef = selectedModelRef;
      if (!resolvedModelRef && provider && model) resolvedModelRef = provider.toLowerCase() + '/' + model;
      var requestedName = String(this.freshInitName || '').trim();
      var requestedEmoji = String(this.freshInitEmoji || '').trim();
      var launchName = requestedName || 'agent';
      if (templateDef.is_other && !String(this.freshInitOtherPrompt || '').trim()) {
        InfringToast.info('Describe the special purpose for Other before launch.');
        this.freshInitAwaitingOtherPrompt = true;
        this.focusChatComposerFromInit('');
        return;
      }
      var selectedPersonality = this.selectedFreshInitPersonality();
      var selectedVibe = this.selectedFreshInitVibe();
      var resolvedSystemPrompt = this.resolveFreshInitSystemPrompt(templateDef, launchName, selectedPersonality, selectedVibe);
      var resolvedContract = this.resolveFreshInitContractPayload(launchName);
      var resolvedPermissions = this.resolveFreshInitPermissionManifest ? this.resolveFreshInitPermissionManifest() : null;
      if (resolvedPermissions && typeof resolvedPermissions === 'object') resolvedContract.permissions_manifest = resolvedPermissions;
      this.freshInitLaunching = true;
      this.freshInitRevealMenu = false;
      this.freshInitEmojiPickerOpen = false;
      try {
        if (resolvedModelRef) {
          await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(agentId) + '/model', {
            model: resolvedModelRef
          });
        }
        var sanitizedRequestedEmoji = this.sanitizeAgentEmojiForDisplay
          ? this.sanitizeAgentEmojiForDisplay(this.currentAgent, requestedEmoji || '')
          : (requestedEmoji || '');
        var identityPayload = {};
        if (String(sanitizedRequestedEmoji || '').trim()) {
          identityPayload.emoji = String(sanitizedRequestedEmoji || '').trim();
        }
        var vibeValue = String(selectedVibe && selectedVibe.id ? selectedVibe.id : '').trim();
        if (vibeValue && vibeValue !== 'none') identityPayload.vibe = vibeValue;
        var configPayload = {
          role: this.resolveFreshInitRole(templateDef),
          identity: identityPayload,
          system_prompt: resolvedSystemPrompt,
          archetype: String(templateDef.archetype || '').trim(),
          profile: String(templateDef.profile || '').trim(),
          contract: resolvedContract,
          termination_condition: resolvedContract.termination_condition,
          expiry_seconds: resolvedContract.expiry_seconds,
          indefinite: resolvedContract.indefinite === true,
        };
        if (requestedName) {
          configPayload.name = requestedName;
        }
        if (!Object.keys(identityPayload).length) {
          delete configPayload.identity;
        }
        if (this.freshInitAvatarUrl) {
          configPayload.avatar_url = String(this.freshInitAvatarUrl || '').trim();
        }
        await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(agentId) + '/config', {
          ...configPayload
        });
        var appliedAgentName = requestedName || String(this.currentAgent.name || this.currentAgent.id || agentId).trim() || 'agent';
        this.addNoticeEvent({
          notice_label: 'Initialized ' + appliedAgentName + ' as ' + String(templateDef.name || 'template'),
          notice_type: 'info',
          ts: Date.now()
        });
        try {
          var store = Alpine.store('app');
          if (store) {
            store.pendingFreshAgentId = null;
            store.pendingAgent = null;
            if (typeof store.refreshAgents === 'function') {
              await store.refreshAgents();
            }
          }
        } catch(_) {}
        await this.syncDrawerAgentAfterChange();
        await this.loadSession(agentId, false);
        this.requestContextTelemetry(true);
        this.freshInitStageToken = Number(this.freshInitStageToken || 0) + 1;
        this.showFreshArchetypeTiles = false;
        this.freshInitTemplateDef = null;
        this.freshInitTemplateName = '';
        this.freshInitLaunching = false; if (typeof this.resetFreshInitPermissions === 'function') this.resetFreshInitPermissions();
        var launchedRole = String((templateDef && (templateDef.name || templateDef.profile || templateDef.archetype)) || 'agent').trim() || 'agent';
        InfringToast.success('Launched ' + String(appliedAgentName || 'agent') + ' as ' + launchedRole);
      } catch (e) {
        this.freshInitLaunching = false;
        this.freshInitRevealMenu = true;
        InfringToast.error('Failed to initialize agent: ' + e.message);
      }
    },

    extractTerminalCommandsFromHistoryText: function(rawText) {
      var text = String(rawText || '');
      if (!text.trim()) return [];
      var lines = text.split('\n');
      var out = [];
      for (var i = 0; i < lines.length; i++) {
        var line = String(lines[i] || '').trim();
        if (!line) continue;
        var marker = line.indexOf(' % ');
        if (marker <= 0) continue;
        var cmd = line.slice(marker + 3).trim();
        if (cmd) out.push(cmd);
      }
      return out;
    },

    normalizeSessionKeyToken: function(value, fallback) {
      var raw = String(value == null ? '' : value).trim().toLowerCase();
      raw = raw.replace(/[^a-z0-9:_-]+/g, '-').replace(/^-+|-+$/g, '');
      if (raw) return raw;
      var fallbackValue = String(fallback == null ? '' : fallback).trim().toLowerCase();
      return fallbackValue || 'main';
    },

    normalizeSessionAgentId: function(value) {
      var raw = String(value == null ? '' : value).trim().toLowerCase();
      raw = raw.replace(/[^a-z0-9_-]+/g, '-').replace(/^-+|-+$/g, '');
      return raw || 'main';
    },

    parseAgentSessionKey: function(sessionKey) {
      var raw = String(sessionKey == null ? '' : sessionKey).trim().toLowerCase();
      if (!raw) return null;
      var parts = raw.split(':').filter(Boolean);
      if (parts.length < 3 || parts[0] !== 'agent') return null;
      var agentId = this.normalizeSessionAgentId(parts[1]);
      var rest = parts.slice(2).join(':');
      if (!rest) return null;
      return {
        agentId: agentId,
        rest: this.normalizeSessionKeyToken(rest, 'main')
      };
    },

    resolveSessionAgentIdFromKey: function(sessionKey, fallbackAgentId) {
      var parsed = this.parseAgentSessionKey(sessionKey);
      if (parsed && parsed.agentId) return parsed.agentId;
      return this.normalizeSessionAgentId(fallbackAgentId);
    },

    isSubagentSessionKey: function(sessionKey) {
      var raw = String(sessionKey == null ? '' : sessionKey).trim().toLowerCase();
      if (!raw) return false;
      if (raw.indexOf('subagent:') === 0) return true;
      var parsed = this.parseAgentSessionKey(raw);
      return !!(parsed && parsed.rest.indexOf('subagent:') === 0);
    },

    resolveSessionRowScopeToken: function(row) {
      var rawKey = String(
        (row && (row.session_key || row.key || row.session_id || row.id || row.main_key)) || ''
      ).trim();
      var parsed = this.parseAgentSessionKey(rawKey);
      if (parsed && parsed.rest) return parsed.rest;
      return this.normalizeSessionKeyToken(rawKey, 'main');
    },

    resolveSessionRowLabel: function(row, fallbackAgentId) {
      var explicitLabel = String((row && (row.label || row.name || row.session_label)) || '').trim();
      if (explicitLabel) return explicitLabel;
      var rawKey = String((row && (row.session_key || row.key || row.session_id || row.id)) || '').trim();
      var parsed = this.parseAgentSessionKey(rawKey);
      var scopeToken = parsed && parsed.rest ? parsed.rest : this.resolveSessionRowScopeToken(row);
      if (scopeToken === 'main') return 'Main';
      if (scopeToken.indexOf('subagent:') === 0) {
        var subagentTail = scopeToken.slice('subagent:'.length).replace(/[:_-]+/g, ' ').trim();
        return subagentTail ? ('Subagent ' + subagentTail) : 'Subagent';
      }
      var normalized = String(scopeToken || '').replace(/[:_-]+/g, ' ').trim();
      if (normalized) return normalized;
      return this.normalizeSessionAgentId(fallbackAgentId);
    },

    normalizeSessionsList: function(rows, fallbackAgentId) {
      var source = Array.isArray(rows) ? rows : [];
      var out = [];
      var seen = {};
      for (var i = 0; i < source.length; i++) {
        var row = source[i];
        if (!row || typeof row !== 'object') continue;
        var rawKey = String((row.session_key || row.key || row.session_id || row.id) || '').trim();
        var agentId = this.resolveSessionAgentIdFromKey(rawKey, row.agent_id || row.agentId || fallbackAgentId);
        var scopeToken = this.resolveSessionRowScopeToken(row);
        var scopeKey = this.normalizeSessionAgentId(agentId) + '|' + scopeToken;
        if (seen[scopeKey]) continue;
        seen[scopeKey] = true;
        out.push(Object.assign({}, row, {
          _agent_id: this.normalizeSessionAgentId(agentId),
          _scope_token: scopeToken,
          _scope_key: scopeKey,
          _label: this.resolveSessionRowLabel(row, agentId),
          _is_subagent: this.isSubagentSessionKey(rawKey),
        }));
      }
      return out;
    },

    resolveCurrentSessionRow: function(agentId) {
      var normalizedAgentId = this.normalizeSessionAgentId(agentId);
      var rows = this.normalizeSessionsList(this.sessions || [], normalizedAgentId);
      var fallback = null;
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i] || {};
        if (!fallback && row._agent_id === normalizedAgentId) fallback = row;
        if (row._agent_id === normalizedAgentId && row.active === true) return row;
      }
      if (fallback) return fallback;
      for (var j = 0; j < rows.length; j++) {
        if (rows[j] && rows[j].active === true) return rows[j];
      }
      return rows.length ? rows[0] : null;
    },

    resolveConversationCacheScopeKey: function(agentId, explicitSessionRow) {
      var normalizedAgentId = this.normalizeSessionAgentId(agentId);
      var row = explicitSessionRow && typeof explicitSessionRow === 'object'
        ? explicitSessionRow
        : this.resolveCurrentSessionRow(normalizedAgentId);
      var scopeToken = row && row._scope_token
        ? row._scope_token
        : this.resolveSessionRowScopeToken(row || {});
      return normalizedAgentId + '|' + this.normalizeSessionKeyToken(scopeToken, 'main');
    },

    applySessionsPayloadSnapshot: function(agentId, payload) {
      var normalizedAgentId = this.normalizeSessionAgentId(agentId);
      var rows = [];
      if (payload && payload.session && Array.isArray(payload.session.sessions)) {
        rows = payload.session.sessions;
      } else if (payload && Array.isArray(payload.sessions)) {
        rows = payload.sessions;
      }
      var normalizedRows = this.normalizeSessionsList(rows, normalizedAgentId);
      if (!normalizedRows.length) return;
      this.sessions = normalizedRows;
      if (!this._sessionsLastLoadedAtByAgent || typeof this._sessionsLastLoadedAtByAgent !== 'object') {
        this._sessionsLastLoadedAtByAgent = {};
      }
      this._sessionsLastLoadedAtByAgent[normalizedAgentId] = Date.now();
    },

    rebuildInputHistoryFromSessionPayload: function(data) {
      var payload = data && typeof data === 'object' ? data : {};
      var fallbackAgentId = String(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '').trim();
      this.applySessionsPayloadSnapshot(fallbackAgentId, payload);
      var state = payload && payload.session && typeof payload.session === 'object' ? payload.session : {};
      var sessions = this.normalizeSessionsList(Array.isArray(state.sessions) ? state.sessions : [], fallbackAgentId);
      var sourceRows = [];
      var seenSessionScopes = {};
      for (var i = 0; i < sessions.length; i++) {
        var session = sessions[i] || {};
        var scopeKey = String(session._scope_key || '').trim();
        if (scopeKey && seenSessionScopes[scopeKey]) continue;
        if (scopeKey) seenSessionScopes[scopeKey] = true;
        var messages = Array.isArray(session.messages) ? session.messages : [];
        for (var j = 0; j < messages.length; j++) sourceRows.push(messages[j]);
      }
      if (Array.isArray(payload.messages)) {
        for (var m = 0; m < payload.messages.length; m++) sourceRows.push(payload.messages[m]);
      }
      if (payload && payload.message_window && Array.isArray(payload.message_window.rows)) {
        for (var mw = 0; mw < payload.message_window.rows.length; mw++) sourceRows.push(payload.message_window.rows[mw]);
      }
      if (!sourceRows.length) {
        this.chatInputHistory = [];
        this.terminalInputHistory = [];
        this.hydrateInputHistoryFromCache('chat');
        this.hydrateInputHistoryFromCache('terminal');
        this.resetInputHistoryNavigation('chat');
        this.resetInputHistoryNavigation('terminal');
        return;
      }

      var normalized = this.normalizeSessionMessages({ messages: sourceRows });
      var maxEntries = Number(this.inputHistoryMaxEntries || 0);
      if (!Number.isFinite(maxEntries) || maxEntries < 20) maxEntries = 120;
      var chatRows = [];
      var terminalRows = [];
      for (var k = 0; k < normalized.length; k++) {
        var row = normalized[k] || {};
        var role = String(row.role || '').toLowerCase();
        var text = String(row.text || '').trim();
        if (!text) continue;
        if (role === 'user') {
          chatRows.push(text);
          continue;
        }
        var isTerminal = !!row.terminal || role === 'terminal';
        if (!isTerminal) continue;
        var source = String(row.terminal_source || '').toLowerCase();
        if (source && source !== 'user') continue;
        var commands = this.extractTerminalCommandsFromHistoryText(text);
        for (var c = 0; c < commands.length; c++) {
          var command = String(commands[c] || '').trim();
          if (command) terminalRows.push(command);
        }
      }
      if (!chatRows.length && !terminalRows.length) {
        this.chatInputHistory = [];
        this.terminalInputHistory = [];
        this.hydrateInputHistoryFromCache('chat');
        this.hydrateInputHistoryFromCache('terminal');
        this.resetInputHistoryNavigation('chat');
        this.resetInputHistoryNavigation('terminal');
        return;
      }
      chatRows = chatRows.filter(function(text, idx, arr) { return idx === 0 || text !== arr[idx - 1]; });
      terminalRows = terminalRows.filter(function(text, idx, arr) { return idx === 0 || text !== arr[idx - 1]; });
      if (chatRows.length > maxEntries) chatRows = chatRows.slice(chatRows.length - maxEntries);
      if (terminalRows.length > maxEntries) terminalRows = terminalRows.slice(terminalRows.length - maxEntries);
