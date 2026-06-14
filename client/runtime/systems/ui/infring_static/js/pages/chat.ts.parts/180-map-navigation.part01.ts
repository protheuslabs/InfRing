      }
      if (!visibleIndexes.length) visibleIndexes = fallbackIndexes;
      if (!visibleIndexes.length) return;

      var activePos = -1;
      var anchorDomId = String(this.selectedMessageDomId || '');
      if (anchorDomId) {
        for (var p = 0; p < visibleIndexes.length; p++) {
          var vi = visibleIndexes[p];
          if (this.messageDomId(list[vi], vi) === anchorDomId) {
            activePos = p;
            break;
          }
        }
      }
      if (activePos < 0) {
        for (var p2 = 0; p2 < visibleIndexes.length; p2++) {
          if (visibleIndexes[p2] === this.mapStepIndex) {
            activePos = p2;
            break;
          }
        }
      }

      if (activePos < 0) {
        activePos = dir > 0 ? 0 : (visibleIndexes.length - 1);
      } else {
        activePos = activePos + (dir > 0 ? 1 : -1);
        if (activePos < 0) activePos = 0;
        if (activePos > visibleIndexes.length - 1) activePos = visibleIndexes.length - 1;
      }

      var next = visibleIndexes[activePos];
      var msg = list[next];
      if (!msg) return;
      this.setHoveredMessage(msg, next);
      this.jumpToMessage(msg, next);
      this.centerChatMapOnMessage(this.messageDomId(msg, next));
      var self = this;
      this._mapPreviewSuppressTimer = setTimeout(function() {
        self.suppressMapPreview = false;
      }, 220);
    },

    chatMapPopupSource: function() {
      return 'chat-map';
    },

    messageMapPopupTitle: function(msg) {
      if (!msg) return 'Message';
      return this.messageActorLabel(msg);
    },

    messageMapPopupBody: function(msg) {
      if (!msg) return '';
      var preview = typeof this.messageVisiblePreviewText === 'function' ? this.messageVisiblePreviewText(msg) : '';
      if (!preview && typeof this.messageMapPreview === 'function') preview = this.messageMapPreview(msg);
      return String(preview || '').trim();
    },

    showMapItemPopup: function(msg, idx, ev) {
      if (!msg) return;
      var domId = this.messageDomId(msg, idx);
      this.forceMessageRender(msg, idx, 9000);
      this.suppressMapPreview = false;
      this.selectedMessageDomId = domId;
      this.mapStepIndex = idx;
      this.setHoveredMessage(msg, idx, { direct: false });
      if (typeof this.showDashboardPopup !== 'function') return;
      this.showDashboardPopup('chat-map-item:' + domId, this.messageMapPopupTitle(msg), ev, {
        source: this.chatMapPopupSource(),
        side: 'left',
        body: this.messageMapPopupBody(msg),
        meta_origin: 'Chat map',
        meta_time: String(this.messageTimestampLabel(msg) || '').trim()
      });
    },

    hideMapItemPopup: function() {
      if (typeof this.hideDashboardPopupBySource === 'function') {
        this.hideDashboardPopupBySource(this.chatMapPopupSource());
      }
      this.clearHoveredMessage();
    },

    showMapDayPopup: function(msg, ev) {
      if (!msg) return;
      this.suppressMapPreview = false;
      if (typeof this.showDashboardPopup !== 'function') return;
      this.showDashboardPopup('chat-map-day:' + this.messageDayKey(msg), this.messageDayLabel(msg), ev, {
        source: this.chatMapPopupSource(),
        side: 'left',
        body: this.isMessageDayCollapsed(msg)
          ? 'Expand this day in the chat map'
          : 'Collapse this day in the chat map',
        meta_origin: 'Chat map'
      });
    },

    hideMapDayPopup: function() {
      if (typeof this.hideDashboardPopupBySource === 'function') {
        this.hideDashboardPopupBySource(this.chatMapPopupSource());
      }
    },

    setHoveredMessage: function(msg, idx, options) {
      if (this._hoverClearTimer) {
        clearTimeout(this._hoverClearTimer);
        this._hoverClearTimer = 0;
      }
      if (!msg && msg !== 0) {
        this.hoveredMessageDomId = this.selectedMessageDomId || '';
        this.directHoveredMessageDomId = '';
        return;
      }
      var domId = this.messageDomId(msg, idx);
      this.hoveredMessageDomId = domId;
      if (options && options.direct) {
        this.directHoveredMessageDomId = domId;
      } else {
        this.directHoveredMessageDomId = '';
      }
    },

    clearHoveredMessage: function() {
      if (this._hoverClearTimer) clearTimeout(this._hoverClearTimer);
      var self = this;
      this._hoverClearTimer = setTimeout(function() {
        self._hoverClearTimer = 0;
        self.hoveredMessageDomId = self.selectedMessageDomId || '';
        self.directHoveredMessageDomId = '';
      }, 42);
    },

    clearHoveredMessageHard: function() {
      if (this._hoverClearTimer) {
        clearTimeout(this._hoverClearTimer);
        this._hoverClearTimer = 0;
      }
      if (typeof this.hideDashboardPopupBySource === 'function') {
        this.hideDashboardPopupBySource(this.chatMapPopupSource());
      }
      this.hoveredMessageDomId = '';
      this.directHoveredMessageDomId = '';
      this.selectedMessageDomId = '';
    },

    isHoveredMessage: function(msg, idx) {
      if (!this.hoveredMessageDomId) return false;
      return this.hoveredMessageDomId === this.messageDomId(msg, idx);
    },

    isDirectHoveredMessage: function(msg, idx) {
      if (!this.directHoveredMessageDomId) return false;
      return this.directHoveredMessageDomId === this.messageDomId(msg, idx);
    },

    centerChatMapOnMessage: function(domId, options) {
      if (!domId) return;
      var immediate = !!(options && options.immediate);
      var map = null;
      var maps = document.querySelectorAll('.chat-map-scroll');
      for (var i = 0; i < maps.length; i++) {
        var candidate = maps[i];
        if (candidate && candidate.offsetParent !== null) {
          map = candidate;
          break;
        }
      }
      if (!map) return;
      var host = map.closest('.chat-map') || map;
      var item = host.querySelector('.chat-map-item[data-msg-dom-id="' + domId + '"]');
      if (!item) return;
      var topGuard = 28;
      var bottomGuard = 28;
      var viewport = Math.max(20, map.clientHeight - topGuard - bottomGuard);
      var desired = item.offsetTop + (item.offsetHeight / 2) - (viewport / 2) - topGuard;
      var max = Math.max(0, map.scrollHeight - map.clientHeight);
      var nextTop = Math.max(0, Math.min(max, desired));
      var diff = Math.abs(map.scrollTop - nextTop);
      if (diff < 3) return;
      map.scrollTo({ top: nextTop, behavior: (immediate || this.suppressMapPreview) ? 'auto' : 'smooth' });
    },

    filteredDrawerEmojiCatalog: function() {
      var source = Array.isArray(this.drawerEmojiCatalog) ? this.drawerEmojiCatalog : [];
      var query = String(this.drawerEmojiSearch || '').trim().toLowerCase();
      var self = this;
      var allowSystemReserved = this.isSystemThreadAgent && this.isSystemThreadAgent(this.currentAgent);
      var rows = source.filter(function(row) {
        var emoji = String((row && row.emoji) || '');
        if (!allowSystemReserved && self.isReservedSystemEmoji && self.isReservedSystemEmoji(emoji)) return false;
        return true;
      });
      if (!query) return rows.slice(0, 24);
      return rows.filter(function(row) {
        var emoji = String((row && row.emoji) || '');
        var name = String((row && row.name) || '').toLowerCase();
        return emoji.indexOf(query) >= 0 || name.indexOf(query) >= 0;
      }).slice(0, 24);
    },

    defaultFreshEmojiForAgent: function(agentRef) {
      void agentRef;
      return '∞';
    },

    suggestedFreshIdentityForAgent: function(agentRef, templateDef) {
      var agent = agentRef && typeof agentRef === 'object' ? agentRef : {};
      var id = String(agent.id || agentRef || '').trim();
      var name = String(agent.name || '').trim();
      var emoji = String((agent.identity && agent.identity.emoji) || '').trim();
      if (!emoji) {
        emoji = this.defaultFreshEmojiForAgent(id || name || 'agent');
      }
      if (templateDef && templateDef.category) {
        var category = String(templateDef.category).toLowerCase();
        if (category.indexOf('development') >= 0) emoji = '🧑\u200d💻';
        else if (category.indexOf('research') >= 0) emoji = '🔬';
        else if (category.indexOf('operations') >= 0 || category.indexOf('ops') >= 0) emoji = '🛠️';
        else if (category.indexOf('writing') >= 0) emoji = '📝';
      }
      emoji = this.sanitizeAgentEmojiForDisplay ? this.sanitizeAgentEmojiForDisplay(agent, emoji) : emoji;
      if (!emoji) emoji = '∞';
      return {
        name: name || String(id || '').trim(),
        emoji: String(emoji || '∞').trim() || '∞',
      };
    },

    toggleDrawerEmojiPicker: function() {
      this.drawerEmojiPickerOpen = !this.drawerEmojiPickerOpen;
      if (!this.drawerEmojiPickerOpen) {
        this.drawerEmojiSearch = '';
      } else {
        this.drawerAvatarUrlPickerOpen = false;
        this.drawerEditingEmoji = true;
      }
    },

    toggleDrawerAvatarUrlPicker: function() {
      this.drawerAvatarUrlPickerOpen = !this.drawerAvatarUrlPickerOpen;
      if (this.drawerAvatarUrlPickerOpen) {
        this.drawerEmojiPickerOpen = false;
        this.drawerAvatarUploadError = '';
        this.drawerAvatarUrlDraft = String(
          (this.drawerConfigForm && this.drawerConfigForm.avatar_url) ||
          (this.agentDrawer && this.agentDrawer.avatar_url) ||
          ''
        ).trim();
      } else {
        this.drawerAvatarUrlDraft = '';
      }
    },

    applyDrawerAvatarUrl: async function() {
      if (!this.agentDrawer || !this.agentDrawer.id) return;
      var draft = String(this.drawerAvatarUrlDraft || '').trim();
      if (!draft) {
        this.drawerAvatarUploadError = 'avatar_url_required';
        InfringToast.error('Avatar URL is required.');
        return;
      }
      var parsed = null;
      try {
        parsed = new URL(draft);
      } catch (_) {
        parsed = null;
      }
      if (!parsed || (parsed.protocol !== 'http:' && parsed.protocol !== 'https:')) {
        this.drawerAvatarUploadError = 'avatar_url_invalid';
        InfringToast.error('Avatar URL must start with http:// or https://');
        return;
      }
      if (!this.drawerConfigForm || typeof this.drawerConfigForm !== 'object') {
        this.drawerConfigForm = {};
      }
      var normalized = String(parsed.toString()).trim();
      this.drawerConfigForm.avatar_url = normalized;
      if (this.agentDrawer && typeof this.agentDrawer === 'object') {
        this.agentDrawer.avatar_url = normalized;
      }
      this.drawerAvatarUploadError = '';
      this.drawerEmojiPickerOpen = false;
      this.drawerAvatarUrlPickerOpen = false;
      this.drawerAvatarUrlDraft = '';
      this.drawerEditingEmoji = false;
      await this.saveDrawerIdentity('avatar');
    },

    selectDrawerEmoji: function(choice) {
      var emoji = String(choice && choice.emoji ? choice.emoji : choice || '').trim();
      if (!emoji) return;
      var sanitized = this.sanitizeAgentEmojiForDisplay
        ? this.sanitizeAgentEmojiForDisplay(this.agentDrawer || this.currentAgent, emoji)
        : emoji;
      if (!sanitized) {
        InfringToast.info('The gear icon is reserved for the System thread.');
        return;
      }
      if (!this.drawerConfigForm || typeof this.drawerConfigForm !== 'object') {
        this.drawerConfigForm = {};
      }
      this.drawerConfigForm.emoji = sanitized;
      // Choosing emoji explicitly switches away from image avatar mode.
      this.drawerConfigForm.avatar_url = '';
      if (this.agentDrawer && typeof this.agentDrawer === 'object') {
        this.agentDrawer.avatar_url = '';
      }
      this.drawerEmojiPickerOpen = false;
      this.drawerEmojiSearch = '';
      this.drawerEditingEmoji = false;
    },

    openDrawerAvatarPicker: function() {
      this.drawerAvatarUrlPickerOpen = false;
      this.drawerAvatarUrlDraft = '';
      if (this.$refs && this.$refs.drawerAvatarInput) {
        this.$refs.drawerAvatarInput.click();
      }
    },

    uploadDrawerAvatar: async function(fileList) {
      if (!this.agentDrawer || !this.agentDrawer.id) return;
      var files = Array.isArray(fileList) ? fileList : Array.from(fileList || []);
      if (!files.length) return;
      var file = files[0];
      if (!file) return;
      var mime = String(file.type || '').toLowerCase();
      if (mime && mime.indexOf('image/') !== 0) {
        InfringToast.error('Avatar must be an image file.');
        return;
      }
      this.drawerAvatarUploading = true;
      this.drawerAvatarUploadError = '';
      try {
        var headers = {
          'Content-Type': file.type || 'application/octet-stream',
          'X-Filename': file.name || 'avatar'
        };
        var token = (typeof InfringAPI !== 'undefined' && typeof InfringAPI.getToken === 'function')
          ? String(InfringAPI.getToken() || '')
          : '';
        if (token) headers.Authorization = 'Bearer ' + token;
        var response = await fetch('/api/agents/' + encodeURIComponent(this.agentDrawer.id) + '/avatar', {
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
          var reason = payload && payload.error ? payload.error : 'avatar_upload_failed';
          throw new Error(String(reason));
        }
        if (!this.drawerConfigForm || typeof this.drawerConfigForm !== 'object') {
          this.drawerConfigForm = {};
        }
        this.drawerConfigForm.avatar_url = String(payload.avatar_url || '').trim();
        this.agentDrawer.avatar_url = String(payload.avatar_url || '').trim();
        this.drawerEditingEmoji = false;
        this.drawerEmojiPickerOpen = false;
        this.drawerAvatarUrlPickerOpen = false;
        this.drawerAvatarUrlDraft = '';
        InfringToast.success('Avatar uploaded');
        await this.saveDrawerIdentity('avatar');
      } catch (error) {
        var message = (error && error.message) ? String(error.message) : 'avatar_upload_failed';
        this.drawerAvatarUploadError = message;
        InfringToast.error('Failed to upload avatar: ' + message);
      } finally {
        this.drawerAvatarUploading = false;
      }
    },

    async openAgentDrawer() {
      if (!this.currentAgent || !this.currentAgent.id) return;
      if (this.isSystemThreadAgent && this.isSystemThreadAgent(this.currentAgent)) return;
      if (this.isCurrentAgentArchived && this.isCurrentAgentArchived()) return;
      this.showAgentDrawer = true;
      this.agentDrawerLoading = true;
      this.drawerTab = 'info';
      this.drawerEditingModel = false;
      this.drawerEditingProvider = false;
      this.drawerEditingFallback = false;
      this.drawerEditingName = false;
      this.drawerEditingEmoji = false;
      this.drawerEmojiPickerOpen = false;
      this.drawerEmojiSearch = '';
      this.drawerAvatarUrlPickerOpen = false;
      this.drawerAvatarUrlDraft = '';
      this.drawerAvatarUploading = false;
      this.drawerAvatarUploadError = '';
      this.drawerIdentitySaving = false;
      this.drawerSavePending = false;
      this.drawerNewModelValue = '';
      this.drawerNewProviderValue = '';
      this.drawerNewFallbackValue = '';
      var base = this.resolveAgent(this.currentAgent) || this.currentAgent;
      this.agentDrawer = Object.assign({}, base, {
        _fallbacks: Array.isArray(base && base._fallbacks) ? base._fallbacks : []
      });
      this.drawerConfigForm = {
        name: this.agentDrawer.name || '',
        system_prompt: this.agentDrawer.system_prompt || '',
        emoji: this.sanitizeAgentEmojiForDisplay
          ? this.sanitizeAgentEmojiForDisplay(this.agentDrawer, (this.agentDrawer.identity && this.agentDrawer.identity.emoji) || '')
          : ((this.agentDrawer.identity && this.agentDrawer.identity.emoji) || ''),
        avatar_url: this.agentDrawer.avatar_url || '',
        color: (this.agentDrawer.identity && this.agentDrawer.identity.color) || '#2563EB',
        archetype: (this.agentDrawer.identity && this.agentDrawer.identity.archetype) || '',
        vibe: (this.agentDrawer.identity && this.agentDrawer.identity.vibe) || '',
      };
      try {
        var full = await InfringAPI.get('/api/agents/' + this.currentAgent.id);
        this.agentDrawer = Object.assign({}, base, full || {}, {
          _fallbacks: Array.isArray(full && full.fallback_models) ? full.fallback_models : []
        });
        this.drawerConfigForm = {
          name: this.agentDrawer.name || '',
          system_prompt: this.agentDrawer.system_prompt || '',
          emoji: this.sanitizeAgentEmojiForDisplay
            ? this.sanitizeAgentEmojiForDisplay(this.agentDrawer, (this.agentDrawer.identity && this.agentDrawer.identity.emoji) || '')
            : ((this.agentDrawer.identity && this.agentDrawer.identity.emoji) || ''),
          avatar_url: this.agentDrawer.avatar_url || '',
          color: (this.agentDrawer.identity && this.agentDrawer.identity.color) || '#2563EB',
          archetype: (this.agentDrawer.identity && this.agentDrawer.identity.archetype) || '',
          vibe: (this.agentDrawer.identity && this.agentDrawer.identity.vibe) || '',
        };
      } catch(e) {
        // Keep best-effort drawer data from current agent/store.
      } finally {
        this.agentDrawerLoading = false;
      }
    },

    closeAgentDrawer() {
      this.showAgentDrawer = false;
      this.drawerEditingName = false;
      this.drawerEditingEmoji = false;
      this.drawerEmojiPickerOpen = false;
      this.drawerEmojiSearch = '';
      this.drawerAvatarUrlPickerOpen = false;
      this.drawerAvatarUrlDraft = '';
      this.drawerAvatarUploadError = '';
    },

    toggleAgentDrawer() {
      if (this.isCurrentAgentArchived && this.isCurrentAgentArchived()) return;
      if (this.showAgentDrawer) {
        this.closeAgentDrawer();
        return;
      }
      this.openAgentDrawer();
    },

    async reviveCurrentArchivedAgent() {
      var agent = this.currentAgent && typeof this.currentAgent === 'object' ? this.currentAgent : null;
      if (!agent || !agent.id) return;
      if (!(this.isArchivedAgentRecord && this.isArchivedAgentRecord(agent))) return;
      var agentId = String(agent.id || '').trim();
      if (!agentId) return;
      try {
        await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(agentId) + '/revive', {
          role: String(agent.role || 'analyst')
        });
        this.currentAgent = Object.assign({}, agent, {
          archived: false,
          state: 'running'
        });
        var store = Alpine.store('app');
        if (store) {
          if (Array.isArray(store.agents)) {
            store.agents = store.agents.map(function(row) {
              if (!row || String((row && row.id) || '') !== agentId) return row;
              return Object.assign({}, row, { archived: false, state: 'running' });
            });
          }
          if (store.pendingAgent && String((store.pendingAgent && store.pendingAgent.id) || '') === agentId) {
            store.pendingAgent = null;
            store.pendingFreshAgentId = null;
          }
          if (Array.isArray(store.archivedAgentIds)) {
            store.archivedAgentIds = store.archivedAgentIds.filter(function(id) {
              return String(id || '') !== agentId;
            });
            if (typeof store.persistArchivedAgentIds === 'function') {
              store.persistArchivedAgentIds();
            } else {
              try {
                localStorage.setItem('infring-archived-agent-ids', JSON.stringify(store.archivedAgentIds));
              } catch(_) {}
            }
          }
          if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(agentId);
          else store.activeAgentId = agentId;
          if (typeof store.refreshAgents === 'function') {
