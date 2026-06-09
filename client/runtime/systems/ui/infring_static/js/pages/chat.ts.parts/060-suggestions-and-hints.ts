// Layer ownership: client/runtime/systems/ui (dashboard static UX surface only; no runtime authority).
          var hasContextOverlap = contextKeywords.some(function(keyword) {
            return loweredRaw.indexOf(keyword) >= 0;
          });
          if (!hasContextOverlap) continue;
        }
        var key = String(raw || '').toLowerCase();
        if (seen[key]) continue;
        var duplicate = out.some(function(existing) {
          return isNearDuplicate(existing, raw);
        });
        if (duplicate) continue;
        seen[key] = true;
        out.push(raw);
        if (out.length >= 3) break;
      }
      return out;
    },

    derivePromptSuggestionFallback(agent, hint, gateContext) {
      var context = this.buildPromptSuggestionContextSnapshot();
      var typedHistory = (Array.isArray(context.history) ? context.history : [])
        .map(function(entry, index) {
          var role = String((entry && entry.role) || 'agent').trim().toLowerCase();
          if (role === 'assistant') role = 'agent';
          return {
            key: 'fallback:' + String(index),
            kind: role === 'user' || role === 'agent' ? 'message' : 'synthetic',
            role: role,
            text: String((entry && entry.text) || '').trim()
          };
        })
        .filter(function(entry) {
          return entry.kind === 'message' && !!entry.text;
        });
      var corpus = [
        String(hint || ''),
        String(gateContext || ''),
        String(context.lastUser || ''),
        String(context.lastAgent || ''),
        typedHistory.slice(-3).map(function(entry) { return entry.role + ':' + entry.text; }).join(' || ')
      ].join(' || ').toLowerCase();
      var out = [];
      var add = function(value) {
        var text = String(value || '').trim();
        if (text) out.push(text);
      };
      if (/(connect|pair|token|auth|unauthorized|secure context|device identity|gateway|fetch failed)/.test(corpus)) {
        add('Summarize the fastest recovery step for this connection error');
        add('/apikey');
        add('/help');
      }
      if (/(model|provider|fallback|failover|slow|thinking|reasoning)/.test(corpus)) {
        add('/model');
        add('Continue from the last successful step with a safer model');
      }
      if (/(voice|audio|microphone|dictat|record)/.test(corpus) || this.recording) {
        add('Turn the latest voice note into a concise prompt');
        add('Summarize this chat into a one-line handoff note');
      }
      if (/(agent|session|chat|thread|roster|branch)/.test(corpus) || !typedHistory.length) {
        add('/agents');
        add('/new');
      }
      if (!out.length) {
        add('Give me the next best action from this conversation');
        add('/help');
        add('/model');
      }
      return this.normalizePromptSuggestions(
        out,
        String(gateContext || context.signature || '').trim(),
        this.recentUserSuggestionSamples()
      );
    },

    buildPromptSuggestionContextSnapshot() {
      var out = { lastUser: '', lastAgent: '', history: [], signature: '' };
      var history = Array.isArray(this.messages) ? this.messages : [];
      var compact = function(value, maxLen) {
        var cap = Number(maxLen || 240);
        var text = String(value == null ? '' : value).replace(/\s+/g, ' ').trim();
        if (!text) return '';
        if (text.length > cap) return text.substring(0, Math.max(8, cap - 3)) + '...';
        return text;
      };
	      for (var i = history.length - 1; i >= 0; i--) {
	        var row = history[i];
	        if (!row || row.thinking || row.streaming || row.terminal || row.is_notice) continue;
	        var normalizedRole = compact(row.role || '', 16).toLowerCase();
	        if (!normalizedRole) {
	          normalizedRole = row.user ? 'user' : row.assistant ? 'agent' : 'agent';
	        }
	        if (normalizedRole === 'system') continue;
	        var text = compact(row.text, 240);
	        if (!text) continue;
	        if (/^\[runtime-task\]/i.test(text)) continue;
	        if (/task accepted\.\s*report findings in this thread with receipt-backed evidence/i.test(text)) continue;
	        if (/the user wants exactly 3 actionable next user prompts/i.test(text)) continue;
	        if (String(text || '').toLowerCase() === 'heartbeat_ok') continue;
	        if (out.history.length < 7) {
	          out.history.unshift({
	            role: normalizedRole,
	            text: text
	          });
	        }
	        if (!out.lastUser && normalizedRole === 'user') {
	          out.lastUser = text;
	          continue;
	        }
	        if (!out.lastAgent && (normalizedRole === 'agent' || normalizedRole === 'assistant')) {
	          out.lastAgent = text;
	        }
	        if (out.lastUser && out.lastAgent && out.history.length >= 7) break;
	      }
      if (out.history.length > 7) out.history = out.history.slice(-7);
      out.signature = compact(
        out.history
          .map(function(entry) {
            return compact(entry.role || 'agent', 20) + ':' + compact(entry.text || '', 180);
          })
          .join(' || ') ||
          (String(out.lastUser || '') + '|' + String(out.lastAgent || '')),
        1200
      );
      return out;
    },

    // Backward-compat shim for legacy callers during naming migration.
    collectPromptSuggestionContext() {
      return this.buildPromptSuggestionContextSnapshot();
    },

    recentUserSuggestionSamples() {
      var history = Array.isArray(this.messages) ? this.messages : [];
      var out = [];
      for (var i = history.length - 1; i >= 0; i--) {
        var row = history[i];
        if (!row || row.thinking || row.streaming || row.terminal || row.is_notice) continue;
        if (String(row.role || '').toLowerCase() !== 'user') continue;
        var text = String(row.text == null ? '' : row.text).replace(/\s+/g, ' ').trim();
        if (!text) continue;
        out.unshift(text);
        if (out.length >= 7) break;
      }
      return out;
    },

    hasConversationSuggestionSeed() {
      if (this.isSystemThreadAgent && this.isSystemThreadAgent(this.currentAgent)) return false;
      var context = this.buildPromptSuggestionContextSnapshot();
      var count = Array.isArray(context && context.history) ? context.history.length : 0;
      return count >= 7;
    },

    nextPromptQueueId() {
      this._promptQueueSeq = Number(this._promptQueueSeq || 0) + 1;
      return 'pq-' + String(Date.now()) + '-' + String(this._promptQueueSeq);
    },

    get promptQueueItems() {
      var queue = Array.isArray(this.messageQueue) ? this.messageQueue : [];
      var out = [];
      for (var i = 0; i < queue.length; i++) {
        var row = queue[i];
        if (!row || row.terminal) continue;
        if (!row.queue_id) row.queue_id = this.nextPromptQueueId();
        if (!row.queue_kind) row.queue_kind = 'prompt';
        out.push({
          queue_id: String(row.queue_id),
          queue_index: i,
          text: String(row.text || '').trim(),
          files: Array.isArray(row.files) ? row.files : [],
          images: Array.isArray(row.images) ? row.images : [],
          queued_at: Number(row.queued_at || 0) || Date.now()
        });
      }
      return out;
    },

    get hasPromptQueue() {
      return Array.isArray(this.promptQueueItems) && this.promptQueueItems.length > 0;
    },

    queuePromptPreview(item) {
      var text = String(item && item.text ? item.text : '').replace(/\s+/g, ' ').trim();
      if (!text) return '(queued prompt)';
      return text.length > 140 ? text.substring(0, 137) + '...' : text;
    },

    promptQueueSteerActionLabel(item) {
      var engineId = String(this.selectedAgentRuntimeEngineId || 'infring_native').trim() || 'infring_native';
      var usesRuntimeSocket = typeof this.shouldUseAgentRuntimeSocketPath === 'function'
        ? this.shouldUseAgentRuntimeSocketPath(engineId)
        : false;
      if (!this.sending) return 'Send';
      if (!usesRuntimeSocket) return 'Steer';
      var row = typeof this.activeRuntimeEngineRow === 'function' ? this.activeRuntimeEngineRow() : null;
      if (row && row.supports_live_steering === true) return 'Steer now';
      return 'Queue steer';
    },

    removePromptQueueItem(queueId) {
      var id = String(queueId || '').trim();
      if (!id) return;
      var rows = Array.isArray(this.messageQueue) ? this.messageQueue : [];
      var idx = rows.findIndex(function(row) {
        return !!(row && String(row.queue_id || '').trim() === id);
      });
      if (idx < 0) return;
      rows.splice(idx, 1);
      this.messageQueue = rows.slice();
      if (!this.hasPromptQueue && !this.sending && this.currentAgent) {
        this.refreshPromptSuggestions(false, 'queue-cleared');
      }
      this.scheduleConversationPersist();
    },

    movePromptQueueItem(sourceId, targetId) {
      var src = String(sourceId || '').trim();
      var dst = String(targetId || '').trim();
      if (!src || !dst || src === dst) return;
      var rows = Array.isArray(this.messageQueue) ? this.messageQueue.slice() : [];
      var srcIdx = rows.findIndex(function(row) { return !!(row && String(row.queue_id || '').trim() === src); });
      var dstIdx = rows.findIndex(function(row) { return !!(row && String(row.queue_id || '').trim() === dst); });
      if (srcIdx < 0 || dstIdx < 0) return;
      var moving = rows[srcIdx];
      rows.splice(srcIdx, 1);
      if (dstIdx > srcIdx) dstIdx -= 1;
      rows.splice(dstIdx, 0, moving);
      this.messageQueue = rows;
      this.scheduleConversationPersist();
    },

    onPromptQueueDragStart(queueId, event) {
      var id = String(queueId || '').trim();
      if (!id) return;
      this.promptQueueDragId = id;
      if (event && event.dataTransfer) {
        event.dataTransfer.effectAllowed = 'move';
        try { event.dataTransfer.setData('text/plain', id); } catch(_) {}
      }
    },

    onPromptQueueDrop(targetId, event) {
      if (event && typeof event.preventDefault === 'function') event.preventDefault();
      var sourceId = String(this.promptQueueDragId || '').trim();
      if (!sourceId && event && event.dataTransfer) {
        try {
          sourceId = String(event.dataTransfer.getData('text/plain') || '').trim();
        } catch(_) {}
      }
      var destinationId = String(targetId || '').trim();
      if (sourceId && destinationId) {
        this.movePromptQueueItem(sourceId, destinationId);
      }
      this.promptQueueDragId = '';
    },

    onPromptQueueDragEnd() {
      this.promptQueueDragId = '';
    },

    async steerPromptQueueItem(queueId) {
      var id = String(queueId || '').trim();
      if (!id) return;
      var rows = Array.isArray(this.messageQueue) ? this.messageQueue : [];
      var idx = rows.findIndex(function(row) {
        return !!(row && String(row.queue_id || '').trim() === id);
      });
      if (idx < 0) return;
      var item = rows[idx];
      rows.splice(idx, 1);
      this.messageQueue = rows.slice();
      var text = String(item && item.text ? item.text : '').trim();
      var files = Array.isArray(item && item.files) ? item.files : [];
      var images = Array.isArray(item && item.images) ? item.images : [];
      if (!text) return;
      this.messages.push({
        id: ++msgId,
        role: 'system',
        is_notice: true,
        notice_type: 'info',
        notice_label: 'Steer injected into active workflow.',
        text: 'Steer injected into active workflow.',
        meta: '',
        tools: [],
        system_origin: 'prompt_queue:steer',
        ts: Date.now(),
      });
      this.scrollToBottom();
      this.scheduleConversationPersist();

      if (!this.sending) {
        var liveAgent = this.ensureValidCurrentAgent({ clear_when_missing: true });
        if (!liveAgent || !liveAgent.id) return;
        this.appendUserChatMessage(text, images, { deferPersist: true });
        this.scheduleConversationPersist();
        this._sendPayload(text, files, images, {
          agent_id: liveAgent.id,
          steer_injected: true,
          from_queue: true,
          queue_id: id
        });
        return;
      }

      var selectedRuntimeEngineId = String(this.selectedAgentRuntimeEngineId || 'infring_native').trim() || 'infring_native';
      var usesAgentRuntimeSocket = typeof this.shouldUseAgentRuntimeSocketPath === 'function'
        ? this.shouldUseAgentRuntimeSocketPath(selectedRuntimeEngineId)
        : false;
      if (usesAgentRuntimeSocket) {
        var runtimeAgent = this.ensureValidCurrentAgent({ clear_when_missing: true });
        if (!runtimeAgent || !runtimeAgent.id) return;
        try {
          var steerAck = await InfringAPI.post('/api/shell-socket/agent-runtime/steer', {
            engine_id: selectedRuntimeEngineId,
            agent_id: runtimeAgent.id,
            session_id: String((runtimeAgent && (runtimeAgent.session_id || runtimeAgent.id)) || runtimeAgent.id || ''),
            text: text,
            attachments: files,
            mode: 'auto',
            priority: 'steer',
          });
          this.appendUserChatMessage(text, images, { deferPersist: true });
          this.messages.push({
            id: ++msgId,
            role: 'system',
            is_notice: true,
            notice_type: 'info',
            notice_label: steerAck && steerAck.live_injected ? 'Steer injected into active runtime turn.' : 'Steer queued for next runtime turn.',
            text: steerAck && steerAck.live_injected ? 'Steer injected into active runtime turn.' : 'Steer queued for next runtime turn.',
            meta: '',
            tools: [],
            system_origin: 'prompt_queue:agent_runtime_steer',
            ts: Date.now(),
          });
          this.scrollToBottom();
          this.scheduleConversationPersist();
          return;
        } catch(_) {}
      }

      var wsPayload = { type: 'message', content: text, steer: true, priority: 'steer' };
      if (files.length) wsPayload.attachments = files;
      if (InfringAPI.wsSend(wsPayload)) {
        this.appendUserChatMessage(text, images, { deferPersist: true });
        this.scheduleConversationPersist();
        return;
      }

      if (this.currentAgent && this.currentAgent.id) {
        var reboundAgent = this.ensureValidCurrentAgent({ clear_when_missing: true });
        if (!reboundAgent || !reboundAgent.id) return;
        try {
          await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(reboundAgent.id) + '/message', {
            message: text,
            attachments: files,
            steer: true,
            priority: 'steer',
          });
          this.appendUserChatMessage(text, images, { deferPersist: true });
          this.scheduleConversationPersist();
          return;
        } catch(_) {}
      }

      this.messageQueue.unshift({
        queue_id: id,
        queue_kind: 'prompt',
        text: text,
        files: files,
        images: images,
        queued_at: Number(item && item.queued_at ? item.queued_at : Date.now()),
      });
      this.messages.push({
        id: ++msgId,
        role: 'system',
        is_notice: true,
        notice_type: 'error',
        notice_label: 'Steer injection failed; prompt returned to queue.',
        text: 'Steer injection failed; prompt returned to queue.',
        meta: '',
        tools: [],
        system_origin: 'prompt_queue:steer',
        ts: Date.now(),
      });
      this.scrollToBottom();
      this.scheduleConversationPersist();
    },

    loadPromptSuggestionsPreference() {
      var key = String(this.promptSuggestionsStorageKey || '').trim();
      if (!key) return;
      try {
        var raw = localStorage.getItem(key);
        if (raw == null) return;
        var normalized = String(raw).trim().toLowerCase();
        this.promptSuggestionsEnabled = !(
          normalized === '0' ||
          normalized === 'false' ||
          normalized === 'off' ||
          normalized === 'no'
        );
      } catch (_) {}
    },

    persistPromptSuggestionsPreference() {
      var key = String(this.promptSuggestionsStorageKey || '').trim();
      if (!key) return;
      try {
        localStorage.setItem(key, this.promptSuggestionsEnabled ? '1' : '0');
      } catch (_) {}
    },

    setPromptSuggestionsEnabled(enabled) {
      this.promptSuggestionsEnabled = enabled !== false;
      this.persistPromptSuggestionsPreference();
      if (!this.promptSuggestionsEnabled) {
        this.clearPromptSuggestions();
        return;
      }
      this.refreshPromptSuggestions(true, 'toggle-enabled');
    },

    togglePromptSuggestionsEnabled() {
      this.setPromptSuggestionsEnabled(!this.promptSuggestionsEnabled);
    },

    clearPromptSuggestions() {
      this.promptSuggestions = [];
      this.suggestionsLoading = false;
      this._lastSuggestionsAt = 0;
      this._lastSuggestionsAgentId = '';
    },

    async applyPromptSuggestion(suggestion) {
      var text = String(suggestion == null ? '' : suggestion).trim();
      if (!text) return;
      this.inputText = text;
      this._pendingPromptSuggestionSend = {
        source: 'prompt_suggestion',
        text_preview: text.slice(0, 240),
        created_at: Date.now()
      };
      this.showSlashMenu = false;
      this.showModelPicker = false;
      this.showAttachMenu = false;
      try {
        await this.sendMessage();
      } finally {
        this._pendingPromptSuggestionSend = null;
      }
    },

    promptSuggestionNeedsResize(chip) {
      if (!chip) return false;
      var wasExpanded = chip.classList.contains('is-expanded');
      var wasResizing = chip.classList.contains('is-resizing');
      if (wasExpanded) chip.classList.remove('is-expanded');
      if (wasResizing) chip.classList.remove('is-resizing');
      var needs = false;
      try {
        var text = chip.querySelector('.prompt-suggestion-chip-text');
        if (text) needs = (Number(text.scrollWidth || 0) - Number(text.clientWidth || 0)) > 1;
        if (!needs) needs = (Number(chip.scrollWidth || 0) - Number(chip.clientWidth || 0)) > 1;
      } catch(_) {}
      if (wasExpanded) chip.classList.add('is-expanded');
      if (wasResizing) chip.classList.add('is-resizing');
      return !!needs;
    },

    onPromptSuggestionHoverIn(event) {
      if (!event || !event.currentTarget) return;
      var chip = event.currentTarget;
      if (chip._resizeBlurTimer) {
        clearTimeout(chip._resizeBlurTimer);
        chip._resizeBlurTimer = 0;
      }
      if (!this.promptSuggestionNeedsResize(chip)) {
        chip.classList.remove('is-expanded');
        chip.classList.remove('is-resizing');
        return;
      }
      chip.classList.add('is-expanded');
      chip.classList.add('is-resizing');
      chip._resizeBlurTimer = setTimeout(function() {
