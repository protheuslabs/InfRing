    fetchModelContextWindows(force) {
      var now = Date.now();
      if (!force && this._contextModelsFetchedAt && (now - this._contextModelsFetchedAt) < 300000) {
        this.setContextWindowFromCurrentAgent();
        return Promise.resolve();
      }
      var self = this;
      return InfringAPI.get('/api/shell-socket/models').then(function(data) {
        self.refreshContextWindowMap(data && data.models ? data.models : []);
        self._contextModelsFetchedAt = Date.now();
        self.setContextWindowFromCurrentAgent();
      }).catch(function() {});
    },

    requestContextTelemetry(force) {
      if (!this.currentAgent || !InfringAPI.isWsConnected()) return false;
      var now = Date.now();
      if (!force && (now - Number(this._lastContextRequestAt || 0)) < 2500) return false;
      this._lastContextRequestAt = now;
      return !!InfringAPI.wsSend({ type: 'command', command: 'context', silent: true });
    },

    normalizeModelUsageKey: function(modelId) {
      return String(modelId || '').trim().toLowerCase();
    },

    loadModelUsageCache: function() {
      try {
        var raw = localStorage.getItem(this.modelUsageCacheKey);
        if (!raw) {
          this.modelUsageCache = {};
          return;
        }
        var parsed = JSON.parse(raw);
        this.modelUsageCache = parsed && typeof parsed === 'object' ? parsed : {};
      } catch {
        this.modelUsageCache = {};
      }
    },

    persistModelUsageCache: function() {
      try {
        localStorage.setItem(this.modelUsageCacheKey, JSON.stringify(this.modelUsageCache || {}));
      } catch {}
    },

    modelUsageTimestamp: function(modelId) {
      var key = this.normalizeModelUsageKey(modelId);
      if (!key || !this.modelUsageCache || typeof this.modelUsageCache !== 'object') return 0;
      var ts = Number(this.modelUsageCache[key] || 0);
      return Number.isFinite(ts) && ts > 0 ? ts : 0;
    },

    // Backward-compat shim for legacy callers during naming migration.
    modelUsageTs: function(modelId) {
      return this.modelUsageTimestamp(modelId);
    },

    recordModelUsageTimestamp: function(modelId, ts) {
      var key = this.normalizeModelUsageKey(modelId);
      if (!key) return;
      if (!this.modelUsageCache || typeof this.modelUsageCache !== 'object') {
        this.modelUsageCache = {};
      }
      var stamp = Number(ts || Date.now());
      this.modelUsageCache[key] = Number.isFinite(stamp) && stamp > 0 ? stamp : Date.now();
      this.persistModelUsageCache();
    },

    // Backward-compat shim for legacy callers during naming migration.
    touchModelUsage: function(modelId, ts) {
      this.recordModelUsageTimestamp(modelId, ts);
    },

    loadModelNoticeCache: function() {
      try {
        var raw = localStorage.getItem(this.modelNoticeCacheKey);
        if (!raw) {
          this.modelNoticeCache = {};
          return;
        }
        var parsed = JSON.parse(raw);
        this.modelNoticeCache = (parsed && typeof parsed === 'object') ? parsed : {};
      } catch {
        this.modelNoticeCache = {};
      }
    },

    persistModelNoticeCache: function() {
      try {
        localStorage.setItem(this.modelNoticeCacheKey, JSON.stringify(this.modelNoticeCache || {}));
      } catch {}
    },

    normalizeNoticeType: function(value, fallbackType) {
      var fallback = String(fallbackType || 'info').toLowerCase();
      if (fallback !== 'model' && fallback !== 'info') fallback = 'info';
      var raw = String(value || '').toLowerCase().trim();
      if (raw === 'model' || raw === 'info') return raw;
      return fallback;
    },

    isModelSwitchNoticeLabel: function(label) {
      var text = String(label || '').trim();
      if (!text) return false;
      return /^Model switched (?:to\b|from\b)/i.test(text);
    },

    rememberModelNotice: function(agentId, label, ts, noticeType, noticeIcon) {
      if (!agentId || !label) return;
      if (!this.modelNoticeCache || typeof this.modelNoticeCache !== 'object') {
        this.modelNoticeCache = {};
      }
      var key = String(agentId);
      if (!Array.isArray(this.modelNoticeCache[key])) this.modelNoticeCache[key] = [];
      var list = this.modelNoticeCache[key];
      var tsNum = Number(ts || Date.now());
      var normalizedType = this.normalizeNoticeType(
        noticeType,
        this.isModelSwitchNoticeLabel(label) ? 'model' : 'info'
      );
      var normalizedIcon = String(noticeIcon || '').trim();
      var exists = list.some(function(entry) {
        return (
          entry &&
          entry.label === label &&
          Number(entry.ts || 0) === tsNum &&
          String(entry.type || '') === normalizedType
        );
      });
      if (!exists) list.push({ label: label, ts: tsNum, type: normalizedType, icon: normalizedIcon });
      if (list.length > 120) this.modelNoticeCache[key] = list.slice(list.length - 120);
      this.persistModelNoticeCache();
    },

    mergeModelNoticesForAgent: function(agentId, rows) {
      var list = Array.isArray(rows) ? rows.slice() : [];
      if (!agentId || !this.modelNoticeCache) return list;
      var notices = this.modelNoticeCache[String(agentId)];
      if (!Array.isArray(notices) || !notices.length) return list;
      var existing = {};
      var self = this;
      list.forEach(function(msg) {
        if (!msg) return;
        var label = msg.notice_label || '';
        if (!label && msg.role === 'system' && typeof msg.text === 'string' && self.isModelSwitchNoticeLabel(msg.text.trim())) {
          label = msg.text.trim();
        }
        if (!label) return;
        var type = self.normalizeNoticeType(
          msg.notice_type,
          self.isModelSwitchNoticeLabel(label) ? 'model' : 'info'
        );
        existing[type + '|' + label + '|' + Number(msg.ts || 0)] = true;
      });
      for (var i = 0; i < notices.length; i++) {
        var n = notices[i] || {};
        var nLabel = String(n.label || '').trim();
        if (!nLabel) continue;
        var nTs = Number(n.ts || 0) || Date.now();
        var nType = this.normalizeNoticeType(
          n.type || n.notice_type,
          this.isModelSwitchNoticeLabel(nLabel) ? 'model' : 'info'
        );
        var nIcon = String(n.icon || n.notice_icon || '').trim();
        var nKey = nType + '|' + nLabel + '|' + nTs;
        if (existing[nKey]) continue;
        list.push({
          id: ++msgId,
          role: 'system',
          text: '',
          meta: '',
          tools: [],
          system_origin: 'notice:' + nType,
          is_notice: true,
          notice_label: nLabel,
          notice_type: nType,
          notice_icon: nIcon,
          ts: nTs
        });
      }
      list.sort(function(a, b) {
        return Number((a && a.ts) || 0) - Number((b && b.ts) || 0);
      });
      return list;
    },

    normalizeMessageRoleForGrouping: function(role) {
      var lower = String(role || '').trim().toLowerCase();
      if (!lower) return 'agent';
      if (lower.indexOf('user') >= 0) return 'user';
      if (lower.indexOf('system') >= 0) return 'system';
      if (lower === 'tool' || lower === 'toolresult' || lower === 'tool_result' || lower === 'toolcall' || lower === 'tool_call') return 'tool';
      if (lower.indexOf('assistant') >= 0 || lower.indexOf('agent') >= 0) return 'agent';
      return lower;
    },

    extractMessageRawText: function(message) {
      var msg = message && typeof message === 'object' ? message : {};
      if (typeof msg.content === 'string') return msg.content;
      if (Array.isArray(msg.content)) {
        var parts = msg.content.map(function(part) {
          return part && part.type === 'text' && typeof part.text === 'string' ? part.text : '';
        }).filter(function(part) { return !!part; });
        if (parts.length) return parts.join('\n');
      }
      if (typeof msg.text === 'string') return msg.text;
      if (typeof msg.message === 'string') return msg.message;
      return '';
    },

    extractMessageThinkingText: function(message) {
      var msg = message && typeof message === 'object' ? message : {};
      if (typeof msg.thinking_text === 'string' && msg.thinking_text.trim()) return msg.thinking_text.trim();
      if (Array.isArray(msg.content)) {
        var parts = msg.content.map(function(part) {
          return part && part.type === 'thinking' && typeof part.thinking === 'string' ? part.thinking.trim() : '';
        }).filter(function(part) { return !!part; });
        if (parts.length) return parts.join('\n');
      }
      var raw = this.extractMessageRawText(msg);
      if (!raw) return '';
      var matches = Array.from(raw.matchAll(/<\s*think(?:ing)?\s*>([\s\S]*?)<\s*\/\s*think(?:ing)?\s*>/gi));
      return matches.map(function(match) { return String((match && match[1]) || '').trim(); }).filter(function(part) { return !!part; }).join('\n');
    },

    extractMessageVisibleText: function(message) {
      var msg = message && typeof message === 'object' ? message : {};
      var raw = typeof msg.text === 'string' && msg.text.trim() ? msg.text : this.extractMessageRawText(msg);
      raw = String(raw || '').replace(/<\s*think(?:ing)?\s*>[\s\S]*?<\s*\/\s*think(?:ing)?\s*>/gi, ' ');
      if (typeof this.stripModelPrefix === 'function') raw = this.stripModelPrefix(raw);
      if (typeof this.sanitizeToolText === 'function') raw = this.sanitizeToolText(raw);
      if (typeof this.stripArtifactDirectivesFromText === 'function') raw = this.stripArtifactDirectivesFromText(raw);
      return raw.replace(/\s+/g, ' ').trim();
    },

    messageMatchesSearchQuery: function(message, query) {
      var normalizedQuery = String(query || '').trim().toLowerCase();
      if (!normalizedQuery) return true;
      var msg = message && typeof message === 'object' ? message : {};
      var parts = [];
      var visible = typeof msg.search_text === 'string' && msg.search_text.trim() ? msg.search_text.trim() : this.extractMessageVisibleText(msg);
      var thinking = typeof msg.thinking_text === 'string' && msg.thinking_text.trim() ? msg.thinking_text.trim() : this.extractMessageThinkingText(msg);
      if (visible) parts.push(visible);
      if (thinking) parts.push(thinking);
      if (msg.notice_label) parts.push(String(msg.notice_label));
      if (Array.isArray(msg.tools)) {
        for (var i = 0; i < msg.tools.length; i += 1) {
          var tool = msg.tools[i] || {};
          if (tool.name) parts.push(String(tool.name));
        }
      }
      return parts.join('\n').toLowerCase().indexOf(normalizedQuery) >= 0;
    },

    normalizeSessionMessages(data) {
      var source = [];
      if (data && Array.isArray(data.messages)) {
        source = data.messages;
      } else if (data && data.message_window && Array.isArray(data.message_window.rows)) {
        source = data.message_window.rows;
      } else if (data && Array.isArray(data.turns)) {
        var turns = data.turns;
        var turnRows = [];
        turns.forEach(function(turn) {
          var ts = turn && turn.ts ? turn.ts : Date.now();
          if (turn && typeof turn.user === 'string' && turn.user.trim()) {
            turnRows.push({ role: 'User', content: turn.user, ts: ts });
          }
          if (turn && typeof turn.assistant === 'string' && turn.assistant.trim()) {
            turnRows.push({ role: 'Agent', content: turn.assistant, ts: ts });
          }
        });
        source = turnRows;
      } else {
        source = [];
      }
      var self = this;
      return source.map(function(m) {
        var roleRaw = String((m && (m.role || m.type)) || '').toLowerCase();
        var isTerminal = roleRaw.indexOf('terminal') >= 0 || !!(m && m.terminal);
        var role = isTerminal ? 'terminal' : self.normalizeMessageRoleForGrouping(roleRaw);
        var textSource = m && (m.content != null ? m.content : (m.text != null ? m.text : m.message));
        if (role === 'user' && m && m.user != null) textSource = m.user;
        if (role === 'agent' && m && m.assistant != null) textSource = m.assistant;
        if (role !== 'user' && !isTerminal && typeof self.assistantTextFromPayload === 'function') {
          var structuredText = self.assistantTextFromPayload(m);
          if (structuredText || Array.isArray(textSource) || (textSource && typeof textSource === 'object')) {
            textSource = structuredText;
          }
        }
        var visibleText = self.extractMessageVisibleText(m);
        if ((!textSource || Array.isArray(textSource) || (textSource && typeof textSource === 'object')) && visibleText) {
          textSource = visibleText;
        }
        var text = typeof textSource === 'string' ? textSource : JSON.stringify(textSource || '');
        text = self.sanitizeToolText(text);
        if (isTerminal) {
          text = String(text || '')
            .replace(/\r\n/g, '\n')
            .replace(/\r/g, '\n')
            .replace(/^\s+|\s+$/g, '');
        }
        if (role === 'agent') text = self.stripModelPrefix(text);
        var derivedSystemOrigin = '';
        if (role === 'user' && /^\s*infring(?:-ops)?\s+/i.test(String(text || ''))) {
          role = 'system';
          derivedSystemOrigin = 'runtime:ops_command';
        }
        if (role === 'user' && /^\s*\[runtime-task\]/i.test(String(text || ''))) {
          role = 'system';
          if (!derivedSystemOrigin) derivedSystemOrigin = 'runtime:task';
        }

        var tools = typeof self.responseToolRowsFromPayload === 'function'
          ? self.responseToolRowsFromPayload(m, 'hist-tool')
          : ((m && Array.isArray(m.tools) ? m.tools : []).map(function(t, idx) {
              return {
                id: (t.name || 'tool') + '-hist-' + idx,
                name: t.name || 'unknown',
                running: false,
                expanded: false,
                status: t.status || (t.is_error ? 'error' : 'done'),
                summary: t.summary || t.label || t.status || '',
                input_ref: t.input_ref || t.detail_ref || '',
                result_ref: t.result_ref || t.detail_ref || '',
                detail_ref: t.detail_ref || t.result_ref || t.input_ref || '',
                is_error: !!t.is_error
              };
            }));
        if (role === 'agent' && !isTerminal) {
          var repairedToolText = '';
          var needsRepair =
            !String(text || '').trim() ||
            (typeof self.textLooksNoFindingsPlaceholder === 'function' && self.textLooksNoFindingsPlaceholder(text)) ||
            (typeof self.textLooksToolAckWithoutFindings === 'function' && self.textLooksToolAckWithoutFindings(text));
          if (needsRepair && typeof self.fallbackAssistantTextFromPayload === 'function') {
            repairedToolText = String(self.fallbackAssistantTextFromPayload(m, tools) || '').trim();
          }
          if (
            repairedToolText &&
            repairedToolText !== String(text || '').trim() &&
            !(typeof self.textLooksNoFindingsPlaceholder === 'function' && self.textLooksNoFindingsPlaceholder(repairedToolText))
          ) {
            text = repairedToolText;
          }
        }
        var messageMetadata = typeof self.assistantTurnMetadataFromPayload === 'function'
          ? self.assistantTurnMetadataFromPayload(m, tools)
          : {};
        var images = (m && Array.isArray(m.images) ? m.images : []).map(function(img) {
          return { file_id: img.file_id, filename: img.filename || 'image' };
        });
        var tsRaw = m && (m.ts || m.timestamp || m.created_at || m.createdAt) ? (m.ts || m.timestamp || m.created_at || m.createdAt) : null;
        var ts = null;
        if (typeof tsRaw === 'number') {
          ts = tsRaw;
        } else if (typeof tsRaw === 'string') {
          var parsedTs = Date.parse(tsRaw);
          ts = Number.isNaN(parsedTs) ? null : parsedTs;
        }
        var meta = typeof (m && m.meta) === 'string' ? m.meta : '';
        if (!meta && m && (m.input_tokens || m.output_tokens)) {
          meta = (m.input_tokens || 0) + ' in / ' + (m.output_tokens || 0) + ' out';
        }
        var isNotice = false;
        var noticeLabel = '';
        var noticeType = '';
        var noticeIcon = '';
        var noticeAction = null;
        if (m && (m.is_notice || m.notice_label || m.notice_type)) {
          var explicitLabel = String(m.notice_label || '').trim();
          var inferredLabel = typeof text === 'string' ? text.trim() : '';
          noticeLabel = explicitLabel || inferredLabel;
          if (noticeLabel) {
            isNotice = true;
            text = '';
            noticeType = self.normalizeNoticeType(
              m.notice_type,
              self.isModelSwitchNoticeLabel(noticeLabel) ? 'model' : 'info'
            );
            noticeIcon = String(m.notice_icon || '').trim();
            noticeAction = self.normalizeNoticeAction(m.notice_action || m.noticeAction || null);
          }
        }
        if (!isNotice && role === 'system' && typeof text === 'string') {
          var compact = text.trim();
          if (self.isModelSwitchNoticeLabel(compact)) {
            isNotice = true;
            noticeLabel = compact;
            text = '';
            noticeType = 'model';
          }
        }
        var systemOrigin = m && m.system_origin ? String(m.system_origin) : derivedSystemOrigin;
        var compactText = typeof text === 'string' ? text.trim() : '';
        if (
          role === 'system' &&
          !isNotice &&
          !systemOrigin &&
          (
            /^\[runtime-task\]/i.test(compactText) ||
            /^task accepted\.\s*report findings in this thread with receipt-backed evidence\.?$/i.test(compactText)
          )
        ) {
          // Legacy synthetic runtime-task chatter (no origin tag) is noise; skip rendering.
          return null;
        }
        if (
          role === 'system' &&
          !isNotice &&
          self.isSystemNotificationGlobalToWorkspace &&
          self.isSystemNotificationGlobalToWorkspace(systemOrigin, compactText) &&
          !(self.isSystemThreadAgent && self.isSystemThreadAgent(self.currentAgent))
        ) {
          // Keep global/system-wide notices out of non-system chats.
          return null;
        }
        var thinkingText = self.extractMessageThinkingText(m);
        return Object.assign({
          id: ++msgId,
          role: role,
          text: text,
          search_text: visibleText || compactText,
          thinking_text: thinkingText,
          meta: meta,
          tools: tools,
          images: images,
          ts: ts,
          is_notice: isNotice,
          notice_label: noticeLabel,
          notice_type: noticeType,
          notice_icon: noticeIcon,
          notice_action: noticeAction,
          terminal: isTerminal,
          terminal_source: m && m.terminal_source ? String(m.terminal_source).toLowerCase() : (isTerminal ? 'user' : ''),
          cwd: m && m.cwd ? String(m.cwd) : '',
          agent_id: m && m.agent_id ? String(m.agent_id) : '',
          agent_name: m && m.agent_name ? String(m.agent_name) : '',
          source_agent_id: m && m.source_agent_id ? String(m.source_agent_id) : '',
          agent_origin: m && m.agent_origin ? String(m.agent_origin) : '',
          system_origin: systemOrigin,
          actor_id: m && m.actor_id ? String(m.actor_id) : '',
          actor: m && m.actor ? String(m.actor) : '',
          render_height_px: Number.isFinite(Number(m && m.render_height_px)) ? Math.max(0, Math.round(Number(m.render_height_px))) : 0,
          render_width_bucket_px: Number.isFinite(Number(m && m.render_width_bucket_px)) ? Math.max(0, Math.round(Number(m.render_width_bucket_px))) : 0,
          render_measured_at: Number.isFinite(Number(m && m.render_measured_at)) ? Math.max(0, Math.round(Number(m.render_measured_at))) : 0
        }, messageMetadata || {});
      }).filter(function(row) { return !!row; });
    },

    isSystemNotificationGlobalToWorkspace: function(systemOrigin, text) {
      var origin = String(systemOrigin || '').trim().toLowerCase();
      var msg = String(text || '').trim().toLowerCase();
      if (!origin && !msg) return false;
      if (
        origin.indexOf('telemetry:') === 0 ||
        origin.indexOf('continuity:') === 0 ||
        origin === 'slash:alerts' ||
        origin === 'slash:next' ||
        origin === 'slash:memory' ||
        origin === 'slash:continuity' ||
        origin === 'slash:opt'
      ) {
        return true;
      }
      if (
        msg.indexOf('memory-backed session context') >= 0 ||
        msg.indexOf('stale memory context') >= 0 ||
        msg.indexOf('continuity cleanup') >= 0 ||
        msg.indexOf('cross-channel continuity') >= 0
      ) {
        return true;
      }
      return false;
    },

    installComposerPlusClickFallback: function() {
      if (typeof document === 'undefined' || this._composerPlusClickFallbackInstalled) return;
      this._composerPlusClickFallbackInstalled = true;
      var self = this;
      document.addEventListener('click', function(event) {
        var target = event && event.target;
        var anchor = target && target.closest ? target.closest('#composer-plus-menu-anchor') : null;
        if (!anchor) return;
        var previous = !!self.showAttachMenu;
        setTimeout(function() {
          if (!!self.showAttachMenu !== previous) return;
          if (typeof self.closeComposerMenus === 'function') self.closeComposerMenus({ attach: true });
          else {
            self.showModelSwitcher = false;
            self.showRuntimeSwitcher = false;
            if (typeof self.closeGitTreeMenu === 'function') self.closeGitTreeMenu();
            else self.showGitTreeMenu = false;
          }
          self.showAttachMenu = !previous;
          if (self.showAttachMenu && typeof self.loadActiveWorkspaceProjection === 'function') {
            Promise.resolve(self.loadActiveWorkspaceProjection({ force: false })).catch(function() {});
          }
        }, 0);
      }, true);
    },

    publishChatPageBridge: function() {
      if (typeof window === 'undefined') return;
      window.InfringChatPage = this;
      try {
        if (document && document.body) {
          document.body.__infringChatPage = this;
          document.body.setAttribute('data-infring-chat-page-bridge', 'ready');
        }
        var wrappers = document && document.querySelectorAll ? document.querySelectorAll('.chat-wrapper') : [];
        for (var i = 0; i < wrappers.length; i += 1) {
          wrappers[i].__infringChatPage = this;
          wrappers[i].setAttribute('data-infring-chat-page-bridge', 'ready');
        }
      } catch (_) {}
    },

    init() {
      var self = this;
      this.publishChatPageBridge();
      this.installComposerPlusClickFallback();

      if (typeof window !== 'undefined') {
        var persistedCache = this.loadConversationCache();
        var runtimeCache = window.__infringChatCache && typeof window.__infringChatCache === 'object'
          ? this.sanitizeConversationCacheForPersistence(window.__infringChatCache)
          : {};
