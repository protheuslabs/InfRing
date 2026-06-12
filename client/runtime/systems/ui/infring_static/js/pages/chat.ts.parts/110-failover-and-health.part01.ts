    // Fetch dynamic slash commands from server
    fetchCommands: function() {
      var self = this;
      InfringAPI.get('/api/commands').then(function(data) {
        if (data.commands && data.commands.length) {
          // Build a set of known cmds to avoid duplicates
          var existing = {};
          self.slashCommands.forEach(function(c) { existing[c.cmd] = true; });
          data.commands.forEach(function(c) {
            if (!existing[c.cmd]) {
              self.slashCommands.push({ cmd: c.cmd, desc: c.desc || '', source: c.source || 'server' });
              existing[c.cmd] = true;
            }
          });
        }
      }).catch(function() { /* silent — use hardcoded list */ });
    },

    normalizeAgentRuntimeSlashCommandRow: function(row, group) {
      if (!row || typeof row !== 'object') return null;
      var displayCommand = String(row.display_command || row.command || '').trim();
      if (!displayCommand) return null;
      var title = String(row.title || row.intent_id || displayCommand).trim();
      var description = String(row.description || '').trim();
      var operationalState = String(row.operational_state || 'stubbed_or_unwired').trim();
      var operationalLabel = String(row.operational_label || operationalState).trim();
      return {
        row_kind: 'command',
        cmd: displayCommand,
        desc: description || title,
        title: title,
        source: 'agent_runtime_command_catalog',
        command_id: String(row.command_id || ''),
        intent_id: String(row.intent_id || ''),
        engine_id: String(row.engine_id || ''),
        group_id: String(row.group_id || (group && group.group_id) || ''),
        group_title: String((group && group.title) || ''),
        native_command: String(row.native_command || ''),
        native_command_kind: String(row.native_command_kind || ''),
        execution_kind: String(row.execution_kind || ''),
        safety_class: String(row.safety_class || ''),
        operational_state: operationalState,
        operational_label: operationalLabel,
        operational_detail: String(row.operational_detail || ''),
        connected: row.connected !== false,
        fully_operational: row.fully_operational === true,
        selectable: true,
        action_route: String(row.action_route || '/api/shell-socket/agent-runtime/commands/execute'),
        default_passthrough_allowed: false,
        chat_memory_eligible: false
      };
    },

    normalizeAgentRuntimeSlashCommandProjection: function(payload) {
      var groups = Array.isArray(payload && payload.groups) ? payload.groups : [];
      var rows = [];
      for (var i = 0; i < groups.length; i += 1) {
        var group = groups[i] && typeof groups[i] === 'object' ? groups[i] : {};
        var commands = Array.isArray(group.commands) ? group.commands : [];
        for (var j = 0; j < commands.length; j += 1) {
          var normalized = this.normalizeAgentRuntimeSlashCommandRow(commands[j], group);
          if (normalized) rows.push(normalized);
        }
      }
      return rows;
    },

    fetchAgentRuntimeSlashCommands: function(force) {
      var self = this;
      var engineId = String(this.selectedAgentRuntimeEngineId || 'infring_native').trim() || 'infring_native';
      var now = Date.now();
      if (
        force !== true &&
        this.agentRuntimeSlashCommandProjection &&
        this._agentRuntimeSlashCommandProjectionEngineId === engineId &&
        now - Number(this._agentRuntimeSlashCommandProjectionAt || 0) < 10000
      ) {
        return Promise.resolve(this.agentRuntimeSlashCommandProjection);
      }
      if (this._agentRuntimeSlashCommandProjectionLoading && force !== true) {
        return Promise.resolve(this.agentRuntimeSlashCommandProjection);
      }
      this._agentRuntimeSlashCommandProjectionLoading = true;
      return InfringAPI.get('/api/shell-socket/agent-runtime/commands?engine_id=' + encodeURIComponent(engineId)).then(function(payload) {
        self.agentRuntimeSlashCommandProjection = payload || null;
        self.agentRuntimeSlashCommandRows = self.normalizeAgentRuntimeSlashCommandProjection(payload);
        self._agentRuntimeSlashCommandProjectionAt = Date.now();
        self._agentRuntimeSlashCommandProjectionEngineId = engineId;
        self._agentRuntimeSlashCommandProjectionLoading = false;
        return payload;
      }).catch(function() {
        self._agentRuntimeSlashCommandProjectionLoading = false;
        return null;
      });
    },

    executeAgentRuntimeSlashCommand: async function(row, cmdArgs) {
      var command = row && typeof row === 'object' ? row : {};
      var engineId = String(command.engine_id || this.selectedAgentRuntimeEngineId || 'infring_native').trim() || 'infring_native';
      var payload = {
        engine_id: engineId,
        intent_id: String(command.intent_id || ''),
        display_command: String(command.cmd || command.display_command || ''),
        args: String(cmdArgs || ''),
        client_surface: 'dashboard',
        source: 'slash_command_menu'
      };
      if (typeof this.publishSlashCommandFeedback === 'function') {
        this.publishSlashCommandFeedback(command, {
          status: 'pending',
          notice_type: 'info',
          text: 'Sending command intent to Gateway.'
        });
      }
      try {
        var res = await InfringAPI.post('/api/shell-socket/agent-runtime/commands/execute', payload);
        var text = String(res && (res.display_text || res.message || res.status || '') || '').trim();
        if (typeof this.publishSlashCommandFeedback === 'function') {
          this.publishSlashCommandFeedback(command, {
            status: String(res && (res.terminal_outcome || res.status) || 'completed'),
            notice_type: res && res.status === 'manual_action_required' ? 'warning' : 'info',
            text: text || 'Gateway accepted the runtime command intent.',
            persist: !!(res && res.status === 'manual_action_required')
          });
        }
        if (text) {
          this.messages.push({
            id: ++msgId,
            role: 'system',
            is_notice: true,
            notice_type: res && res.status === 'manual_action_required' ? 'warning' : 'info',
            notice_label: String(command.title || command.cmd || 'Runtime command'),
            text: text,
            meta: '',
            tools: [],
            system_origin: 'slash:agent-runtime-command',
            ts: Date.now()
          });
          this.scrollToBottom();
          this.scheduleConversationPersist();
        }
        return res;
      } catch (error) {
        var message = String(error && error.message ? error.message : error || 'runtime_command_failed').trim();
        if (typeof this.publishSlashCommandFeedback === 'function') {
          this.publishSlashCommandFeedback(command, {
            status: 'failed',
            notice_type: 'error',
            text: message,
            persist: true
          });
        }
        if (typeof this.emitCommandFailureNotice === 'function') {
          this.emitCommandFailureNotice(command.cmd || payload.display_command, message, ['/help']);
        }
        return null;
      }
    },

    // Keep thinking indicators alive while work is still in-flight.
    // Only hard-timeout once no pending activity remains or the request is
    // genuinely stale far beyond expected runtime.
    _resetTypingTimeout: function() {
      var self = this;
      if (self._typingTimeout) clearTimeout(self._typingTimeout);
      self._typingTimeout = setTimeout(function() {
        var hasPending = typeof self.hasLivePendingResponse === 'function'
          ? self.hasLivePendingResponse()
          : false;
        var hardStale = typeof self.pendingResponseExceededHardTimeout === 'function'
          ? self.pendingResponseExceededHardTimeout()
          : false;
        if (hasPending && !hardStale) {
          self._resetTypingTimeout();
          return;
        }
        // Transport timeout: do not fabricate assistant content.
        self._clearStreamingTypewriters();
        typeof self.clearTransientThinkingRows === 'function' ? self.clearTransientThinkingRows({ force: true }) : (self.messages = self.messages.filter(function(m) { return !m.thinking && !m.streaming; }));
        // Do not inject transport-authored text into the chat transcript.
        self.sending = false;
        self._responseStartedAt = 0;
        self.tokenCount = 0;
        self._inflightPayload = null;
        self._clearPendingWsRequest();
        self.setAgentLiveActivity(self.currentAgent && self.currentAgent.id ? self.currentAgent.id : '', 'idle', { optimistic: true, source: 'shell_display_hint' });
        self.scheduleConversationPersist();
      }, 120000);
    },

    hasLivePendingResponse: function() {
      var rows = Array.isArray(this.messages) ? this.messages : [];
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i];
        if (!row) continue;
        if (row.thinking || row.streaming || (row.terminal && row.thinking)) {
          return true;
        }
      }
      return !!(this._pendingWsRequest && this._pendingWsRequest.agent_id);
    },

    pendingResponseExceededHardTimeout: function() {
      var now = Date.now();
      var startedAt = Number(this._responseStartedAt || 0);
      if ((!Number.isFinite(startedAt) || startedAt <= 0) && this._pendingWsRequest) {
        startedAt = Number(this._pendingWsRequest.started_at || 0);
      }
      if (!Number.isFinite(startedAt) || startedAt <= 0) {
        var rows = Array.isArray(this.messages) ? this.messages : [];
        for (var i = rows.length - 1; i >= 0; i--) {
          var row = rows[i];
          if (!row) continue;
          if (!(row.thinking || row.streaming || (row.terminal && row.thinking))) continue;
          var rowStartedAt = Number(row._stream_started_at || row._stream_updated_at || row.ts || 0);
          if (Number.isFinite(rowStartedAt) && rowStartedAt > 0) {
            startedAt = rowStartedAt;
            break;
          }
        }
      }
      if (!Number.isFinite(startedAt) || startedAt <= 0) return false;
      return Math.max(0, now - startedAt) >= 900000;
    },

    _clearTypingTimeout: function() {
      if (this._typingTimeout) {
        clearTimeout(this._typingTimeout);
        this._typingTimeout = null;
      }
    },

    _resolveLiveMessageRef: function(message) {
      if (!message || typeof message !== 'object') return null;
      var msgId = message.id;
      if (!Array.isArray(this.messages) || !this.messages.length) return message;
      if (msgId == null) return message;
      for (var i = this.messages.length - 1; i >= 0; i--) {
        var row = this.messages[i];
        if (!row || typeof row !== 'object') continue;
        if (String(row.id) === String(msgId)) return row;
      }
      return message;
    },

    _clearMessageTypewriter: function(message, options) {
      var liveMessage = this._resolveLiveMessageRef(message);
      if (!liveMessage || typeof liveMessage !== 'object') return;
      var opts = options && typeof options === 'object' ? options : {};
      var preserveTypingVisual = opts.preserveTypingVisual === true;
      var preservePartialText = opts.preservePartialText === true;
      var clearFinalText = opts.clearFinalText !== false;
      if (liveMessage._typewriterTimer) {
        clearTimeout(liveMessage._typewriterTimer);
        liveMessage._typewriterTimer = null;
      }
      if (message && message !== liveMessage && message._typewriterTimer) {
        clearTimeout(message._typewriterTimer);
        message._typewriterTimer = null;
      }
      liveMessage._typewriterRunning = false;
      if (message && message !== liveMessage) message._typewriterRunning = false;
      if (!preserveTypingVisual) {
        if (
          !preservePartialText &&
          liveMessage._typingVisual &&
          typeof liveMessage._typewriterFinalText === 'string'
        ) {
          liveMessage.text = String(liveMessage._typewriterFinalText || '');
        }
        liveMessage._typingVisual = false;
        if (message && message !== liveMessage) message._typingVisual = false;
      }
      if (clearFinalText && !preserveTypingVisual) {
        liveMessage._typewriterFinalText = '';
        if (message && message !== liveMessage) message._typewriterFinalText = '';
      }
      if (!preserveTypingVisual) {
        liveMessage._typingVisualHtml = '';
        liveMessage._typingVisualHtmlStable = '';
        liveMessage._typingVisualHtmlActive = '';
        liveMessage._typingVisualHtmlActiveStable = '';
        if (message && message !== liveMessage) message._typingVisualHtml = '';
        if (message && message !== liveMessage) message._typingVisualHtmlStable = '';
        if (message && message !== liveMessage) message._typingVisualHtmlActive = '';
        if (message && message !== liveMessage) message._typingVisualHtmlActiveStable = '';
      }
    },

    _clearStreamingTypewriters: function() {
      var rows = Array.isArray(this.messages) ? this.messages : [];
      for (var i = 0; i < rows.length; i++) {
        this._clearMessageTypewriter(rows[i], {
          preserveTypingVisual: false,
          preservePartialText: false,
        });
      }
    },

    _hasActiveTypewriterVisual: function() {
      var rows = Array.isArray(this.messages) ? this.messages : [];
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i];
        if (!row || typeof row !== 'object') continue;
        if (row._typingVisual || row._typewriterRunning || row._typewriterTimer) return true;
      }
      return false;
    },

    _queueStreamTypingRender: function(message, nextText) {
      if (!message || typeof message !== 'object') return;
      var targetText = String(nextText || '');
      message._streamTargetText = targetText;
      message._typewriterFinalText = '';
      if (message._typewriterRunning) return;
      var self = this;
      message._typewriterRunning = true;

      var step = function() {
        if (!message || !message.streaming) {
          self._clearMessageTypewriter(message);
          return;
        }
        var target = String(message._streamTargetText || '');
        var current = String(message.text || '');
        if (target === current) {
          self._clearMessageTypewriter(message);
          return;
        }
        // If sanitization trims or rewrites content, snap to the newest safe value.
        if (target.length < current.length || target.indexOf(current) !== 0) {
          message.text = target;
          self._clearMessageTypewriter(message);
          self.scrollToBottom();
          return;
        }
        var remaining = target.length - current.length;
        var take = Math.max(1, Math.min(8, Math.ceil(remaining / 4)));
        message.text = target.slice(0, current.length + take);
        self.scrollToBottom();
        if (message.text.length < target.length) {
          message._typewriterTimer = setTimeout(step, 1);
          return;
        }
        self._clearMessageTypewriter(message);
      };

      step();
    },

    _resolveTypingDelayForToken: function(baseDelay, emittedToken, fullText, emittedIndex) {
      var base = Number(baseDelay || 1);
      if (!Number.isFinite(base) || base < 0) base = 1;
      var token = String(emittedToken || '');
      if (!/[.!?]/.test(token)) return base;
      var source = String(fullText || '');
      var idx = Number(emittedIndex || 0);
      if (!Number.isFinite(idx) || idx < 0) idx = 0;
      var next = source.charAt(idx + 1);
      if (!next || /\s|["')\]]/.test(next)) {
        return base * 2;
      }
      return base;
    },

    // Backward-compat shim for legacy callers during naming migration.
    _typingDelayForToken: function(baseDelay, emittedToken, fullText, emittedIndex) {
      return this._resolveTypingDelayForToken(baseDelay, emittedToken, fullText, emittedIndex);
    },

    _resolveTypingWordCadenceMs: function(fallbackDelayMs) {
      var cadenceMs = Number(this.typingWordCadenceMs);
      if (!Number.isFinite(cadenceMs) || cadenceMs <= 0) cadenceMs = Number(fallbackDelayMs);
      if (!Number.isFinite(cadenceMs) || cadenceMs <= 0) cadenceMs = 1;
      cadenceMs = Math.max(1, Math.min(2000, cadenceMs));
      return cadenceMs;
    },

    _escapeTypingVisualTokenHtml: function(token) {
      var raw = String(token == null ? '' : token);
      var escaped = '';
      if (typeof this.escapeHtml === 'function') escaped = this.escapeHtml(raw);
      else escaped = raw
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#39;');
      escaped = escaped.replace(/\t/g, '    ');
      escaped = escaped.replace(/\n/g, '<br>');
      return escaped;
    },

    _queueFinalWordTypingRender: function(message, finalText, wordDelayMs) {
      var baseMessage = this._resolveLiveMessageRef(message);
      if (!baseMessage || typeof baseMessage !== 'object') return;
      var targetText = String(finalText || '');
      this._clearMessageTypewriter(baseMessage, {
        preserveTypingVisual: false,
        preservePartialText: false,
      });
      baseMessage._typingVisual = false;
      if (!targetText.trim()) {
        baseMessage._typewriterFinalText = '';
        baseMessage._typingVisualHtml = '';
        baseMessage._typingVisualHtmlStable = '';
        baseMessage._typingVisualHtmlActive = '';
        baseMessage._typingVisualHtmlActiveStable = '';
        baseMessage.text = targetText;
        if (typeof this.scheduleConversationPersist === 'function') this.scheduleConversationPersist();
        return;
      }
      var segments = [];
      var segmentPattern = /\S+\s*/g;
      var segmentMatch;
      var typingRenderText = typeof normalizeChatMarkdownListBreaks === 'function'
        ? normalizeChatMarkdownListBreaks(targetText)
        : targetText;
      while ((segmentMatch = segmentPattern.exec(typingRenderText)) !== null) {
        segments.push({
          text: String(segmentMatch[0] || ''),
          index: Number(segmentMatch.index || 0)
        });
      }
      var leadingWhitespaceMatch = typingRenderText.match(/^\s+/);
      if (leadingWhitespaceMatch && segments.length) {
        var leadingWhitespace = String(leadingWhitespaceMatch[0] || '');
        segments[0].text = leadingWhitespace + String(segments[0].text || '');
        segments[0].index = 0;
      }
      if (!Array.isArray(segments) || !segments.length) {
        baseMessage._typewriterFinalText = '';
        baseMessage._typingVisualHtml = '';
        baseMessage._typingVisualHtmlStable = '';
        baseMessage._typingVisualHtmlActive = '';
        baseMessage._typingVisualHtmlActiveStable = '';
        baseMessage.text = targetText;
        baseMessage._typingVisual = false;
        if (typeof this.scheduleConversationPersist === 'function') this.scheduleConversationPersist();
        return;
      }
      baseMessage._typewriterFinalText = targetText;
      baseMessage.text = '';
      baseMessage._typingVisualHtml = '';
      baseMessage._typingVisualHtmlStable = '';
      baseMessage._typingVisualHtmlActive = '';
      baseMessage._typingVisualHtmlActiveStable = '';
      baseMessage._typingVisual = true;
      baseMessage._typewriterRunning = true;
      var self = this;
      var index = 0;
      var markdownState = { bold: false, italic: false };
      var cadenceMs = typeof this._resolveTypingWordCadenceMs === 'function'
        ? this._resolveTypingWordCadenceMs(wordDelayMs)
        : 1;
      var maxTokensPerTick = 24;
      var nextTickAt = Date.now();
      var keepPinnedToBottom = function() {
        try {
          if (typeof self.scrollToBottomImmediate === 'function') {
            self.scrollToBottomImmediate({ force: false });
          } else {
            self.scrollToBottom();
          }
        } catch (_) {}
      };
      var step = function() {
        var liveMessage = self._resolveLiveMessageRef(baseMessage);
        if (!liveMessage) {
          self._clearMessageTypewriter(baseMessage);
          return;
        }
        if (!liveMessage._typewriterRunning) {
          self._clearMessageTypewriter(liveMessage, {
            preserveTypingVisual: false,
            preservePartialText: false,
          });
          if (typeof self.scheduleConversationPersist === 'function') self.scheduleConversationPersist();
          return;
        }
        if (index >= segments.length) {
          liveMessage.text = targetText;
          liveMessage._typingVisual = false;
          liveMessage._typingVisualHtmlStable = '';
          liveMessage._typingVisualHtmlActive = '';
          liveMessage._typingVisualHtmlActiveStable = '';
          liveMessage._typingVisualHtml = '';
          if (baseMessage !== liveMessage) {
            baseMessage._typingVisual = false;
            baseMessage._typingVisualHtmlStable = '';
            baseMessage._typingVisualHtmlActive = '';
            baseMessage._typingVisualHtmlActiveStable = '';
            baseMessage._typingVisualHtml = '';
          }
          liveMessage._typewriterRunning = false;
          liveMessage._typewriterTimer = null;
          if (baseMessage !== liveMessage) {
            baseMessage._typewriterRunning = false;
            baseMessage._typewriterTimer = null;
          }
          if (typeof self.scheduleConversationPersist === 'function') self.scheduleConversationPersist();
          return;
        }
        var now = Date.now();
        if (now < nextTickAt) {
          var waitMs = Math.max(1, Math.min(2000, Math.round(nextTickAt - now)));
          var waitTimer = setTimeout(step, waitMs);
          liveMessage._typewriterTimer = waitTimer;
          if (baseMessage !== liveMessage) baseMessage._typewriterTimer = waitTimer;
          return;
        }
        var emitted = 0;
        var stableHtml = String(liveMessage._typingVisualHtmlStable || '') + String(liveMessage._typingVisualHtmlActiveStable || '');
        var activeHtml = '';
        var activeStable = '';
        var hasFutureMarkdownDelimiter = function(sourceText, delimiter, absoluteIndex) {
          var source = String(sourceText || '');
          var marker = String(delimiter || '');
          if (!marker) return false;
          var startAt = Math.max(0, Number(absoluteIndex || 0) + marker.length);
          return source.indexOf(marker, startAt) >= 0;
        };
        while (index < segments.length && emitted < maxTokensPerTick) {
          now = Date.now();
          if (now < nextTickAt) break;
          cadenceMs = typeof self._resolveTypingWordCadenceMs === 'function'
            ? self._resolveTypingWordCadenceMs(wordDelayMs)
            : cadenceMs;
          var segment = segments[index] || { text: '', index: 0 };
          var token = String(segment.text || '');
          index += 1;
          emitted += 1;
          liveMessage.text = String(liveMessage.text || '') + token;
          var tokenEndIndex = Number(segment.index || 0) + Math.max(0, token.length - 1);
          var nextDelay = typeof self._resolveTypingDelayForToken === 'function'
            ? self._resolveTypingDelayForToken(cadenceMs, token, typingRenderText, tokenEndIndex)
            : cadenceMs;
          if (!Number.isFinite(nextDelay) || nextDelay <= 0) nextDelay = cadenceMs;
          nextDelay = Math.max(1, Math.min(2000, nextDelay));
          nextTickAt += nextDelay;
          var tokenHtmlStable = '';
          var tokenHtmlActive = '';
          var tokenState = { bold: !!markdownState.bold, italic: !!markdownState.italic };
          var appendChunk = function(chunk, isActiveChunk) {
            if (!chunk) return;
            var chunkHtml = self._escapeTypingVisualTokenHtml(chunk);
            if (tokenState.bold) chunkHtml = '<strong>' + chunkHtml + '</strong>';
            if (tokenState.italic) chunkHtml = '<em>' + chunkHtml + '</em>';
            if (isActiveChunk) {
              tokenHtmlActive +=
                '<span class="typing-word-active" style="--typing-word-fade-ms:' +
                '1000ms">' +
                chunkHtml +
                '</span>';
              tokenHtmlStable += chunkHtml;
              return;
            }
            tokenHtmlStable += chunkHtml;
            tokenHtmlActive += chunkHtml;
          };
          var cursor = 0;
          while (cursor < token.length) {
            var absoluteCursor = Number(segment.index || 0) + cursor;
            if (token.charAt(cursor) === '\\' && (cursor + 1) < token.length && token.charAt(cursor + 1) === '*') {
              appendChunk('*', true);
              cursor += 2;
              continue;
            }
            if ((cursor + 1) < token.length && token.charAt(cursor) === '*' && token.charAt(cursor + 1) === '*') {
              if (tokenState.bold || hasFutureMarkdownDelimiter(typingRenderText, '**', absoluteCursor)) {
                tokenState.bold = !tokenState.bold;
              } else {
                appendChunk('**', true);
              }
              cursor += 2;
              continue;
            }
            if (token.charAt(cursor) === '*') {
              if (tokenState.italic || hasFutureMarkdownDelimiter(typingRenderText, '*', absoluteCursor)) {
                tokenState.italic = !tokenState.italic;
              } else {
                appendChunk('*', true);
              }
              cursor += 1;
              continue;
            }
            var start = cursor;
            while (cursor < token.length) {
              if (token.charAt(cursor) === '\\' && (cursor + 1) < token.length && token.charAt(cursor + 1) === '*') break;
              if ((cursor + 1) < token.length && token.charAt(cursor) === '*' && token.charAt(cursor + 1) === '*') break;
              if (token.charAt(cursor) === '*') break;
              cursor += 1;
            }
            var chunk = token.slice(start, cursor);
            if (!chunk) continue;
            if (!/\S/.test(chunk)) {
              appendChunk(chunk, false);
              continue;
            }
            var leadMatch = chunk.match(/^\s+/);
            var trailMatch = chunk.match(/\s+$/);
            var lead = leadMatch ? String(leadMatch[0] || '') : '';
            var trail = trailMatch ? String(trailMatch[0] || '') : '';
            var coreStart = lead.length;
            var coreEnd = chunk.length - trail.length;
            if (coreEnd < coreStart) {
              coreEnd = coreStart;
              trail = '';
            }
            var core = chunk.slice(coreStart, coreEnd);
            if (lead) appendChunk(lead, false);
            if (core) appendChunk(core, true);
            if (trail) appendChunk(trail, false);
          }
          markdownState.bold = !!tokenState.bold;
          markdownState.italic = !!tokenState.italic;
          if (activeStable) stableHtml += activeStable;
          activeStable = tokenHtmlStable;
          activeHtml = tokenHtmlActive;
        }
        liveMessage._typingVisualHtmlStable = stableHtml;
        liveMessage._typingVisualHtmlActive = activeHtml;
        liveMessage._typingVisualHtmlActiveStable = activeStable;
        liveMessage._typingVisualHtml = stableHtml + activeHtml;
        if (baseMessage !== liveMessage) {
          baseMessage._typingVisualHtmlStable = liveMessage._typingVisualHtmlStable;
          baseMessage._typingVisualHtmlActive = liveMessage._typingVisualHtmlActive;
          baseMessage._typingVisualHtmlActiveStable = liveMessage._typingVisualHtmlActiveStable;
          baseMessage._typingVisualHtml = liveMessage._typingVisualHtml;
        }
        if (emitted > 0) keepPinnedToBottom();
        if (index < segments.length) {
          var timerDelay = Math.max(1, Math.min(2000, Math.round(nextTickAt - Date.now())));
          var timerId = setTimeout(step, timerDelay);
          liveMessage._typewriterTimer = timerId;
          if (baseMessage !== liveMessage) baseMessage._typewriterTimer = timerId;
          return;
        }
        liveMessage.text = targetText;
        liveMessage._typingVisual = false;
        liveMessage._typingVisualHtmlStable = '';
        liveMessage._typingVisualHtmlActive = '';
        liveMessage._typingVisualHtmlActiveStable = '';
        liveMessage._typingVisualHtml = '';
        if (baseMessage !== liveMessage) {
          baseMessage._typingVisual = false;
          baseMessage._typingVisualHtmlStable = '';
          baseMessage._typingVisualHtmlActive = '';
          baseMessage._typingVisualHtmlActiveStable = '';
          baseMessage._typingVisualHtml = '';
        }
        liveMessage._typewriterRunning = false;
        liveMessage._typewriterTimer = null;
        if (baseMessage !== liveMessage) {
          baseMessage._typewriterRunning = false;
          baseMessage._typewriterTimer = null;
        }
        keepPinnedToBottom();
        if (typeof self.scheduleConversationPersist === 'function') self.scheduleConversationPersist();
      };
      step();
    },
    _reconcileSendingState: function() {
      if (!this.sending) return false;
      var pending = this._pendingWsRequest && this._pendingWsRequest.agent_id ? this._pendingWsRequest : null;
      var hasPendingWs = !!pending;
      var inflight = this._inflightPayload && typeof this._inflightPayload === 'object'
        ? this._inflightPayload
        : null;
      var pendingStatusText = pending && String(pending.status_text || '').trim()
        ? String(pending.status_text || '').trim()
        : '';
      var rows = Array.isArray(this.messages) ? this.messages : [];
      var hasVisiblePending = false;
      var now = Date.now();
      var currentAgentId = String(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '').trim();
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i];
        if (!row) continue;
        if (row.thinking || row.streaming || (row.terminal && row.thinking)) {
          if (
            (!String(row.thinking_status || '').trim() ||
              (
                typeof this.isThinkingPlaceholderText === 'function' &&
                this.isThinkingPlaceholderText(row.thinking_status)
              )) &&
            (!pending || !pending.agent_id || !String(row.agent_id || '').trim() || String(row.agent_id || '').trim() === String(pending.agent_id || '').trim())
          ) {
            row.thinking_status = pendingStatusText;
          }
          hasVisiblePending = true;
        }
      }
      if (!hasVisiblePending && hasPendingWs && typeof this.ensureLiveThinkingRow === 'function') {
        var keepRow = this.ensureLiveThinkingRow({
          agent_id: String(pending.agent_id || ''),
          agent_name: this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '',
          status_text: pendingStatusText
        });
        if (keepRow) {
          keepRow.thinking = true;
          keepRow.streaming = true;
          if (!Number.isFinite(Number(keepRow._stream_started_at))) keepRow._stream_started_at = now;
          keepRow._stream_updated_at = now;
          if (!String(keepRow.text || '').trim()) keepRow.text = '';
          if (
            !String(keepRow.thinking_status || '').trim() ||
            (
              typeof this.isThinkingPlaceholderText === 'function' &&
              this.isThinkingPlaceholderText(keepRow.thinking_status)
            )
          ) {
            keepRow.thinking_status = pendingStatusText;
          }
          hasVisiblePending = true;
        }
      }
      if (pending) {
        var pendingAgentId = String(pending.agent_id || '');
        var pendingAgeMs = Math.max(0, now - Number(pending.started_at || now));
        if (currentAgentId && pendingAgentId && pendingAgentId !== currentAgentId) {
          hasPendingWs = false;
          if (!this._pendingWsRecovering) {
            this._recoverPendingWsRequest('cross_agent_pending');
          }
        } else if (pendingAgeMs >= 12000) {
          if (!this._pendingWsRecovering) {
            this._recoverPendingWsRequest('stale_pending');
          }
          if (pendingAgeMs >= 900000) {
            this._clearPendingWsRequest();
            hasPendingWs = false;
          }
        }
      }
      if (!hasVisiblePending && !hasPendingWs && inflight) {
        var inflightAgentId = String(inflight.agent_id || currentAgentId || '').trim();
        var inflightStartedAt = Number(
          this._responseStartedAt ||
          (pending && pending.started_at) ||
          inflight.created_at ||
          0
        );
        var hasRecentInflightReply = false;
        if (
          Number.isFinite(inflightStartedAt) &&
          inflightStartedAt > 0 &&
          typeof this._recentAgentReplyObserved === 'function'
        ) {
          hasRecentInflightReply = this._recentAgentReplyObserved(rows, inflightStartedAt);
        }
        if (inflightAgentId && !hasRecentInflightReply) {
          this._setPendingWsRequest(
            inflightAgentId,
            String(inflight.final_text || ''),
            {
              started_at: Number.isFinite(inflightStartedAt) && inflightStartedAt > 0
                ? inflightStartedAt
                : Date.now(),
              status_text: pendingStatusText
            }
          );
          pending = this._pendingWsRequest;
          hasPendingWs = !!pending;
          if (!Number.isFinite(Number(this._responseStartedAt)) || Number(this._responseStartedAt) <= 0) {
            this._responseStartedAt = Number(
              (pending && pending.started_at) ||
              inflight.created_at ||
              Date.now()
            );
          }
          if (typeof this.ensureLiveThinkingRow === 'function') {
            var restoredPending = this.ensureLiveThinkingRow({
              agent_id: inflightAgentId,
              agent_name: this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '',
              status_text: pendingStatusText
            });
            if (restoredPending) {
              restoredPending.thinking = true;
              restoredPending.streaming = true;
              restoredPending._stream_updated_at = now;
              if (!Number.isFinite(Number(restoredPending._stream_started_at))) {
                restoredPending._stream_started_at = now;
              }
              hasVisiblePending = true;
            }
          }
        }
      }
      if (hasVisiblePending || hasPendingWs) {
        var keepBusyAgentId = '';
        if (pending && pending.agent_id) keepBusyAgentId = String(pending.agent_id || '').trim();
        if (!keepBusyAgentId) keepBusyAgentId = String(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '').trim();
        if (keepBusyAgentId) this.setAgentLiveActivity(keepBusyAgentId, 'working', { optimistic: true, source: 'shell_display_hint' });
      }
      if (hasVisiblePending || hasPendingWs) return false;
      this.sending = false;
      this._responseStartedAt = 0;
      this.tokenCount = 0;
      this._clearTypingTimeout();
      this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '', 'idle', { optimistic: true, source: 'shell_display_hint' });
      return true;
    },
    _setPendingWsRequest: function(agentId, messageText, options) {
      var id = String(agentId || '').trim();
      if (!id) return;
      var opts = options && typeof options === 'object' ? options : {};
      var startedAt = Number(opts.started_at || 0);
      if (!Number.isFinite(startedAt) || startedAt <= 0) startedAt = Date.now();
      var statusText = String(opts.status_text || '').trim();
      this._pendingWsRequest = {
        agent_id: id,
        message_text: String(messageText || '').trim(),
        status_text: statusText,
        started_at: startedAt,
      };
      this._pendingWsRecovering = false;
    },

    _setPendingWsStatusText: function(agentId, statusText) {
      if (!this._pendingWsRequest) return;
      var pendingAgentId = String(this._pendingWsRequest.agent_id || '').trim();
      var targetAgentId = String(agentId || '').trim();
      if (targetAgentId && pendingAgentId && pendingAgentId !== targetAgentId) return;
      var nextStatus = String(statusText || '').trim();
      if (!nextStatus) return;
      if (typeof this.normalizeThinkingStatusCandidate === 'function') {
        var normalized = this.normalizeThinkingStatusCandidate(nextStatus);
        if (normalized) nextStatus = normalized;
      }
      if (!nextStatus) return;
      this._pendingWsRequest.status_text = nextStatus;
    },

    _clearPendingWsRequest: function(agentId) {
      if (!this._pendingWsRequest) return;
      if (!agentId) {
        this._pendingWsRequest = null;
        this._pendingWsRecovering = false;
        return;
      }
      var current = String(this._pendingWsRequest.agent_id || '').trim();
      if (current && current === String(agentId)) {
        this._pendingWsRequest = null;
        this._pendingWsRecovering = false;
      }
    },

    _markAgentPreviewUnread: function(agentId, unread) {
      var id = String(agentId || '').trim();
      if (!id) return;
      try {
        var store = Alpine.store('app');
        if (!store) return;
        if (typeof store.markAgentPreviewUnread === 'function') {
          store.markAgentPreviewUnread(id, unread !== false);
        } else if (store.agentChatPreviews && store.agentChatPreviews[id]) {
          store.agentChatPreviews[id].unread_response = unread !== false;
        }
      } catch(_) {}
    },

    _pendingRequestReplyObserved: function(normalizedMessages, pendingRequest, startedAt) {
      var rows = Array.isArray(normalizedMessages) ? normalizedMessages : [];
      if (!rows.length) return false;

      var pendingText = this.sanitizeToolText(String(
        pendingRequest && pendingRequest.message_text ? pendingRequest.message_text : ''
      )).trim();
      var started = Number(startedAt || (pendingRequest && pendingRequest.started_at) || 0);
      var skewToleranceMs = 15000;
      var lastMatchingUserIndex = -1;

      if (pendingText) {
        for (var i = 0; i < rows.length; i++) {
          var userMsg = rows[i] || {};
          var userRole = String(userMsg.role || '').toLowerCase();
          if (userRole !== 'user') continue;
          var userText = this.sanitizeToolText(String(userMsg.text || '')).trim();
          if (userText && userText === pendingText) {
            lastMatchingUserIndex = i;
          }
        }
      }

      for (var j = 0; j < rows.length; j++) {
        var msg = rows[j] || {};
        var role = String(msg.role || '').toLowerCase();
        var text = String(msg.text || '').trim();
        var hasToolPayload = Array.isArray(msg.tools) && msg.tools.length > 0;
        var isAgentRole = role === 'agent' || role === 'assistant';
        if (!isAgentRole || (!text && !hasToolPayload)) continue;

        var ts = Number(msg.ts || 0);
        if (started > 0 && ts > 0 && (ts + skewToleranceMs) >= started) {
          return true;
        }
        if (lastMatchingUserIndex >= 0 && j > lastMatchingUserIndex) {
          return true;
        }
      }
      return false;
    },

    _recentAgentReplyObserved: function(rows, startedAt) {
      var list = Array.isArray(rows) ? rows : [];
      if (!list.length) return false;
      var started = Number(startedAt || 0);
      var skewToleranceMs = 20000;
      for (var i = list.length - 1; i >= 0; i -= 1) {
        var msg = list[i] || {};
        if (msg.thinking || msg.streaming) continue;
        var role = String(msg.role || '').toLowerCase();
        if (role !== 'agent' && role !== 'assistant') continue;
        var text = String(msg.text || '').trim();
        var hasToolPayload = Array.isArray(msg.tools) && msg.tools.length > 0;
        if (!text && !hasToolPayload) continue;
        if (msg._auto_fallback) continue;
        if (text && /^thinking\.\.\.$/i.test(text)) continue;
        var ts = Number(msg.ts || 0);
        if (started > 0 && ts > 0 && (ts + skewToleranceMs) < started) continue;
        return true;
      }
      return false;
    },

    _recoverPendingWsRequest: async function(reason) {
      if (this._pendingWsRecovering) return;
      var pending = this._pendingWsRequest;
      if (!pending || !pending.agent_id) return;
      this._pendingWsRecovering = true;
      var recoverySeq = Number(this._pendingWsRecoverySeq || 0) + 1;
      this._pendingWsRecoverySeq = recoverySeq;
      var agentId = String(pending.agent_id);
      var startedAt = Number(pending.started_at || Date.now());
      var recoverStartedAt = Date.now();
      var maxRecoverMs = 15000;
      var resolved = false;
      var self = this;
      var recoveryStillCurrent = function() {
        if (Number(self._pendingWsRecoverySeq || 0) !== recoverySeq) return false;
        if (!self._pendingWsRequest || String(self._pendingWsRequest.agent_id || '') !== agentId) return false;
        return true;
      };
      for (var attempt = 0; attempt < 30; attempt++) {
        if (!recoveryStillCurrent()) {
          break;
        }
        if ((Date.now() - recoverStartedAt) > maxRecoverMs) {
          break;
        }
        try {
          var sessionData = await InfringAPI.get('/api/agents/' + encodeURIComponent(agentId) + '/session');
          if (!recoveryStillCurrent()) break;
          var normalized = this.normalizeSessionMessages(sessionData);
          var hasFreshAgentReply =
            this._pendingRequestReplyObserved(normalized, pending, startedAt) ||
            this._recentAgentReplyObserved(normalized, startedAt);
          if (!hasFreshAgentReply) {
            await new Promise(function(resolve) { setTimeout(resolve, 650); });
            continue;
          }
          if (!this.conversationCache) this.conversationCache = {};
          var normalizedPreview = this.sanitizeConversationForCache(normalized || []);
          this.conversationCache[String(agentId)] = {
            saved_at: Date.now(),
            token_count: Number(this.contextApproxTokens || 0),
            messages: normalizedPreview,
          };
          try {
            var appStore = Alpine.store('app');
            if (appStore && typeof appStore.saveAgentChatPreview === 'function') {
              appStore.saveAgentChatPreview(agentId, normalizedPreview);
            }
          } catch(_) {}
          var isActive = !!(this.currentAgent && String(this.currentAgent.id || '') === agentId);
          if (isActive) {
            this.messages = this.mergeModelNoticesForAgent(agentId, JSON.parse(JSON.stringify(normalized || [])));
            this.scrollToBottom();
          } else {
            this._markAgentPreviewUnread(agentId, true);
          }
          this.persistConversationCache();
          resolved = true;
          break;
        } catch(_) {
          if (!recoveryStillCurrent()) break;
          await new Promise(function(resolve) { setTimeout(resolve, 500); });
        }
      }

      if (!recoveryStillCurrent()) {
        this._pendingWsRecovering = false;
        return;
      }
      var stillActiveAgent = !!(this.currentAgent && String(this.currentAgent.id || '') === agentId);
      if (!resolved && stillActiveAgent) {
        var localRows = Array.isArray(this.messages) ? this.messages : [];
        if (this._pendingRequestReplyObserved(localRows, pending, startedAt)) {
          resolved = true;
        }
        if (!resolved && this._recentAgentReplyObserved(localRows, startedAt)) {
          resolved = true;
        }
        if (!resolved && this._recentAgentReplyObserved(localRows, Math.max(0, startedAt - 120000))) {
          resolved = true;
        }
      }
      var pendingAgeMs = Math.max(0, Date.now() - Number(startedAt || Date.now()));
      if (!resolved && stillActiveAgent && pendingAgeMs < 900000) {
        this._pendingWsRecovering = false;
        return;
      }
      if (!resolved && stillActiveAgent) {
        typeof this.clearTransientThinkingRows === 'function' ? this.clearTransientThinkingRows({ force: true }) : (this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; }));
        // Do not inject transport-authored text into the chat transcript.
      }
      if (!resolved && !stillActiveAgent) {
        this._pendingWsRecovering = false;
        return;
      }
      this.setAgentLiveActivity(agentId, 'idle', { optimistic: true, source: 'shell_display_hint' });
      if (stillActiveAgent) {
        this.sending = false;
        this._responseStartedAt = 0;
        this.tokenCount = 0;
        var self = this;
        this.$nextTick(function() {
          var el = document.getElementById('msg-input'); if (el) el.focus();
          self._processQueue();
        });
      }
      this._clearPendingWsRequest(agentId);
      this._pendingWsRecovering = false;
    },

    async executeSlashCommand(cmd, cmdArgs, commandRow) {
      this.showSlashMenu = false;
