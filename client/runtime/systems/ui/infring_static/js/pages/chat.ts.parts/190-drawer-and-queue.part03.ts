    clearTransientThinkingRows: function(options) {
      var opts = options && typeof options === 'object' ? options : {}, force = opts.force === true;
      var preserveRunningTools = !force && opts.preserve_running_tools !== false;
      var pendingAgentId = !force && opts.preserve_pending_ws !== false && this._pendingWsRequest && this._pendingWsRequest.agent_id ? String(this._pendingWsRequest.agent_id || '').trim() : '';
      var rows = Array.isArray(this.messages) ? this.messages : []; if (!rows.length) return 0;
      var kept = [], now = Date.now(), keptPending = false;
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i];
        if (!row || (!row.thinking && !row.streaming)) { kept.push(row); continue; }
        var rowAgentId = String(row.agent_id || '').trim();
        var keep = (preserveRunningTools && this.hasRunningActionableTools(row)) || (!!pendingAgentId && (!rowAgentId || rowAgentId === pendingAgentId));
        if (!keep) continue;
        if (pendingAgentId && (!rowAgentId || rowAgentId === pendingAgentId)) keptPending = true;
        row.thinking = true; row.streaming = true; row._stream_updated_at = now;
        if (!Number.isFinite(Number(row._stream_started_at))) row._stream_started_at = now;
        if (!String(row.thinking_status || '').trim()) {
          var label = typeof this.currentToolDialogLabel === 'function' ? String(this.currentToolDialogLabel(row) || '').trim() : '';
          if (label) row.thinking_status = label;
        }
        kept.push(row);
      }
      if (typeof this.replaceActiveChatMessages === 'function') this.replaceActiveChatMessages(kept);
      else this.messages = kept;
      if (!force && pendingAgentId && !keptPending && typeof this.ensureLiveThinkingRow === 'function') {
        var restored = this.ensureLiveThinkingRow({ agent_id: pendingAgentId, agent_name: this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '' });
        if (restored) {
          restored.thinking = true; restored.streaming = true; restored._stream_updated_at = now;
          if (!Number.isFinite(Number(restored._stream_started_at))) restored._stream_started_at = now;
        }
      }
      return Math.max(0, rows.length - this.messages.length);
    },

    thoughtToolDurationSeconds: function(tool) {
      if (!tool || typeof tool !== 'object') return 0;
      var ms = Number(tool.duration_ms || tool.durationMs || tool.elapsed_ms || 0);
      if (!Number.isFinite(ms) || ms < 0) ms = 0;
      var seconds = Math.round(ms / 1000);
      if (ms > 0 && seconds < 1) seconds = 1;
      return Math.max(0, seconds);
    },

    thoughtToolLabel: function(tool) {
      return 'Thought for ' + this.thoughtToolDurationSeconds(tool) + ' seconds';
    },

    toolRawResultTextIfLoaded: function(tool) {
      var row = tool && typeof tool === 'object' ? tool : {};
      if (!row._detail_loaded || row.result == null) return '';
      if (typeof row.result === 'string') return row.result;
      try {
        return JSON.stringify(row.result);
      } catch (_) {
        return '';
      }
    },

    toolProjectionPreviewText: function(tool) {
      var row = tool && typeof tool === 'object' ? tool : {};
      var raw = this.toolRawResultTextIfLoaded(row);
      if (raw) return raw;
      return String(row.summary || row.result_preview || row.output_preview || row.display_text || row.status || '').trim();
    },

    toolStatusText: function(tool) {
      if (!tool) return '';
      if (tool.running) return 'running...';
      if (tool._detail_loading) return 'loading detail...';
      if (this.isThoughtTool(tool)) return 'thought';
      if (this.isBlockedTool(tool)) return 'blocked';
      if (tool.is_error) return 'error';
      var loadedResult = this.toolRawResultTextIfLoaded(tool);
      if (loadedResult) {
        return loadedResult.length > 500 ? Math.round(loadedResult.length / 1024) + 'KB' : 'done';
      }
      return 'done';
    },

    // Mark chat-rendered error messages for styling
    isErrorMessage: function(msg) {
      if (!msg || !msg.text) return false;
      if (String(msg.role || '').toLowerCase() !== 'system') return false;
      var t = String(msg.text).trim().toLowerCase();
      return t.startsWith('error:');
    },

    messageHasTools: function(msg) {
      return !!(msg && Array.isArray(msg.tools) && msg.tools.length);
    },

    allToolsCollapsed: function(msg) {
      if (!this.messageHasTools(msg)) return true;
      return !msg.tools.some(function(tool) {
        return !!(tool && tool.expanded);
      });
    },

    toggleMessageTools: function(msg) {
      if (!this.messageHasTools(msg)) return;
      var expand = this.allToolsCollapsed(msg);
      msg.tools.forEach(function(tool) {
        if (tool) tool.expanded = expand;
      });
      this.scheduleConversationPersist();
    },

    formatToolOutputForClipboard: function(text) {
      var raw = String(text == null ? '' : text);
      var trimmed = raw.trim();
      if (!trimmed) return '';
      if (trimmed.charAt(0) === '{' || trimmed.charAt(0) === '[') {
        try {
          return '```json\n' + JSON.stringify(JSON.parse(trimmed), null, 2) + '\n```';
        } catch (_) {}
      }
      return raw;
    },

    truncateToolOutputPreview: function(text) {
      var raw = String(text == null ? '' : text).trim();
      if (!raw) return '';
      var allLines = raw.split('\n');
      var maxLines = Number(this.toolPreviewMaxLines || 0);
      if (!Number.isFinite(maxLines) || maxLines < 1) maxLines = 2;
      var maxChars = Number(this.toolPreviewMaxChars || 0);
      if (!Number.isFinite(maxChars) || maxChars < 24) maxChars = 100;
      var preview = allLines.slice(0, maxLines).join('\n');
      if (preview.length > maxChars) return preview.slice(0, maxChars).trimEnd() + '…';
      return allLines.length > maxLines ? preview.trimEnd() + '…' : preview;
    },

    messageCopyMarkdown: function(msg) {
      var row = msg && typeof msg === 'object' ? msg : {};
      var parts = [];
      var label = typeof this.messageActorLabel === 'function'
        ? String(this.messageActorLabel(row) || '').trim()
        : String(row.role || 'Message').trim();
      var stamp = typeof this.messageTimestampLabel === 'function' ? String(this.messageTimestampLabel(row) || '').trim() : '';
      if (label) parts.push('**' + label + '**');
      if (stamp) parts.push('_' + stamp + '_');

      var text = '';
      if (typeof this.extractMessageVisibleText === 'function') {
        text = String(this.extractMessageVisibleText(row) || '').trim();
      }
      if (!text && typeof this.messageVisiblePreviewText === 'function') {
        text = String(this.messageVisiblePreviewText(row) || '').trim();
      }
      if (!text) text = String(row.text || '').trim();
      if (text) parts.push(text);

      if (row.notice_label) {
        var notice = String(row.notice_label || '').trim();
        if (notice) parts.push('Notice: ' + notice);
      }

      var toolLines = [];
      var tools = Array.isArray(row.tools) ? row.tools : [];
      for (var i = 0; i < tools.length; i += 1) {
        var tool = tools[i] || {};
        var toolName = this.toolDisplayName(tool);
        var status = String(tool.status || '').trim();
        var rendered = this.formatToolOutputForClipboard(this.toolProjectionPreviewText(tool) || '');
        var preview = rendered ? this.truncateToolOutputPreview(rendered) : '';
        var line = '- ' + toolName;
        if (status) line += ' (' + status + ')';
        if (preview) line += ': ' + preview;
        toolLines.push(line);
      }
      if (toolLines.length) {
        parts.push('');
        parts.push('Tools:');
        for (var j = 0; j < toolLines.length; j += 1) parts.push(toolLines[j]);
      }

      if (row.file_output && row.file_output.path) parts.push('', 'File: `' + String(row.file_output.path).trim() + '`');
      if (row.folder_output && row.folder_output.path) parts.push('', 'Folder: `' + String(row.folder_output.path).trim() + '`');

      return parts.filter(function(part, idx, arr) {
        if (part !== '') return true;
        return idx > 0 && arr[idx - 1] !== '';
      }).join('\n').trim();
    },

    // Copy message text to clipboard as markdown
    copyMessage: function(msg) {
      if (!msg || msg._copying) return;
      var text = this.messageCopyMarkdown(msg);
      if (!text || !navigator.clipboard || typeof navigator.clipboard.writeText !== 'function') {
        InfringToast.error('Copy failed.');
        return;
      }
      msg._copying = true;
      navigator.clipboard.writeText(text).then(function() {
        msg._copying = false;
        msg._copied = true;
        setTimeout(function() { msg._copied = false; }, 1500);
      }).catch(function() {
        msg._copying = false;
        InfringToast.error('Copy failed.');
      });
    },

    prefersReducedMotion: function() {
      if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return false;
      try {
        return !!window.matchMedia('(prefers-reduced-motion: reduce)').matches;
      } catch (_) {
        return false;
      }
    },

    captureComposerSendMorph: function(textInput) {
      if (this.prefersReducedMotion() || this.terminalMode || this.showFreshArchetypeTiles) return null;
      if (typeof document === 'undefined') return null;
      var shell = document.querySelector('.input-row .composer-shell');
      var input = document.getElementById('msg-input');
      if (!shell || !input) return null;
      var text = String(textInput == null ? '' : textInput).trim();
      if (!text) return null;
      var rect = input.getBoundingClientRect();
      if (!(rect.width > 80 && rect.height > 24)) return null;
      var ghost = document.createElement('div');
      ghost.className = 'composer-send-morph-ghost';
      ghost.textContent = text.length > 260 ? (text.slice(0, 257) + '...') : text;
      ghost.style.left = rect.left + 'px';
      ghost.style.top = rect.top + 'px';
      ghost.style.width = rect.width + 'px';
      ghost.style.minHeight = rect.height + 'px';
      document.body.appendChild(ghost);
      shell.classList.add('composer-shell-send-morph');
      return { shell: shell, ghost: ghost };
    },

    clearComposerSendMorph: function(snapshot) {
      if (!snapshot || typeof snapshot !== 'object') return;
      if (snapshot.shell && snapshot.shell.classList) snapshot.shell.classList.remove('composer-shell-send-morph');
      if (snapshot.ghost && snapshot.ghost.parentNode) snapshot.ghost.parentNode.removeChild(snapshot.ghost);
    },

    playComposerSendMorphToMessage: function(snapshot, messageId) {
      if (!snapshot || !snapshot.ghost) return;
      if (this.prefersReducedMotion()) {
        snapshot.ghost.style.opacity = '0.56';
        setTimeout(this.clearComposerSendMorph.bind(this, snapshot), 240);
        return;
      }
      var row = document.getElementById('chat-msg-' + String(messageId || '').trim());
      var bubble = row ? row.querySelector('.message-bubble') : null;
      if (!bubble) {
        this.clearComposerSendMorph(snapshot);
        return;
      }
      var rect = bubble.getBoundingClientRect();
      if (!(rect.width > 24 && rect.height > 20)) {
        this.clearComposerSendMorph(snapshot);
        return;
      }
      var ghost = snapshot.ghost;
      var self = this;
      ghost.classList.add('in-flight');
      var finish = function() { self.clearComposerSendMorph(snapshot); };
      ghost.addEventListener('transitionend', finish, { once: true });
      requestAnimationFrame(function() {
        ghost.style.left = rect.left + 'px';
        ghost.style.top = rect.top + 'px';
        ghost.style.width = rect.width + 'px';
        ghost.style.minHeight = rect.height + 'px';
        ghost.style.opacity = '0.2';
      });
      setTimeout(finish, 760);
    },

    appendUserChatMessage: function(finalText, msgImages, options) {
      var opts = options && typeof options === 'object' ? options : {};
      var text = String(finalText == null ? '' : finalText);
      var images = Array.isArray(msgImages) ? msgImages : [];
      if (!String(text || '').trim() && !images.length) return;
      var msg = {
        id: ++msgId,
        role: 'user',
        text: text,
        meta: '',
        tools: [],
        images: images,
        ts: Number.isFinite(Number(opts.ts)) ? Number(opts.ts) : Date.now()
      };
      this.messages.push(msg);
      this._stickToBottom = true;
      this.scrollToBottom({ force: true, stabilize: true });
      localStorage.setItem('of-first-msg', 'true');
      this.promptSuggestions = [];
      if (!opts.deferPersist) this.scheduleConversationPersist();
      return msg;
    },

    // Process queued messages after current response completes
    _processQueue: function() {
      if (!this.messageQueue.length || this.sending || this._inflightFailoverInProgress) return;
      var next = this.messageQueue.shift();
      if (next && next.terminal) {
        this._sendTerminalPayload(next.command);
        return;
      }
      var nextText = String(next && next.text ? next.text : '');
      var nextFiles = Array.isArray(next && next.files) ? next.files : [];
      var nextImages = Array.isArray(next && next.images) ? next.images : [];
      if (!nextText.trim() && !nextFiles.length) {
        var self = this;
        this.$nextTick(function() { self._processQueue(); });
        return;
      }
      this.appendUserChatMessage(nextText, nextImages, { deferPersist: true });
      this.scheduleConversationPersist();
      this._sendPayload(nextText, nextFiles, nextImages, {
        from_queue: true,
        queue_id: next && next.queue_id ? String(next.queue_id) : '',
        agent_runtime_engine_id: String((next && next.agent_runtime_engine_id) || this.selectedAgentRuntimeEngineId || 'infring_native')
      });
    },

    _terminalPromptLine: function(cwd, command) {
      var path = String(cwd || this.terminalPromptPath || '/workspace');
      var cmd = String(command || '').trim();
      if (!cmd) return path + ' %';
      return path + ' % ' + cmd;
    },

    _appendTerminalMessage: function(entry) {
      var payload = entry || {};
      var text = String(payload.text || '').replace(/\r\n/g, '\n').replace(/\r/g, '\n').replace(/^\s+|\s+$/g, '');
      var now = Date.now();
      var ts = Number.isFinite(Number(payload.ts)) ? Number(payload.ts) : now;
      var role = payload.role ? String(payload.role) : 'terminal';
      var terminalSource = payload.terminal_source ? String(payload.terminal_source).toLowerCase() : '';
      if (terminalSource !== 'user' && terminalSource !== 'agent' && terminalSource !== 'system') {
        terminalSource = role === 'user' ? 'user' : 'system';
      }
      var cwd = payload.cwd ? String(payload.cwd) : this.terminalPromptPath;
      var meta = payload.meta == null ? '' : String(payload.meta);
      var tools = Array.isArray(payload.tools) ? payload.tools : [];
      var shouldAppendToLast = payload.append_to_last === true;
      var agentId = payload.agent_id ? String(payload.agent_id) : '';
      var agentName = payload.agent_name ? String(payload.agent_name) : '';
      if (terminalSource === 'agent') {
        if (!agentId && this.currentAgent && this.currentAgent.id) agentId = String(this.currentAgent.id);
        if (!agentName && this.currentAgent && this.currentAgent.name) agentName = String(this.currentAgent.name);
      }

      var rows = this.ensureActiveChatMessagesArray();
      var last = rows.length ? rows[rows.length - 1] : null;
      if (shouldAppendToLast && last && !last.thinking && last.terminal) {
        if (text) {
          if (last.text && !/\n$/.test(last.text)) last.text += '\n';
          last.text += text.replace(/^[\r\n]+/, '');
        }
        if (meta) last.meta = meta;
        if (cwd) {
          last.cwd = cwd;
          this.terminalCwd = cwd;
        }
        if (terminalSource) last.terminal_source = terminalSource;
        if (agentId) last.agent_id = agentId;
        if (agentName) last.agent_name = agentName;
        last.ts = ts;
        if (!Array.isArray(last.tools)) last.tools = [];
        if (tools.length) last.tools = last.tools.concat(tools);
        if (typeof this.syncActiveChatMessages === 'function') this.syncActiveChatMessages();
        return last;
      }

      var msg = {
        id: ++msgId,
        role: role,
        text: text,
        meta: meta,
        tools: tools,
        ts: ts,
        terminal: true,
        terminal_source: terminalSource || 'system',
        cwd: cwd
      };
      if (agentId) msg.agent_id = agentId;
      if (agentName) msg.agent_name = agentName;
      this.appendActiveChatMessage(msg);
      if (cwd) this.terminalCwd = cwd;
      return msg;
    },
