    pushSystemMessage: function(entry) {
      var payload = entry && typeof entry === 'object' ? entry : { text: entry };
      var rawText = String(payload && payload.text ? payload.text : '');
      var text = this.normalizeSystemMessageText
        ? this.normalizeSystemMessageText(rawText)
        : rawText.trim();
      if (!text) return null;
      var canonicalText = text.replace(/\s+/g, ' ').trim().toLowerCase();
      if (/^error:\s*/i.test(canonicalText) && canonicalText.indexOf('operation was aborted') >= 0) return null;
      if (payload.allow_chat_injection !== true) {
        if (!Array.isArray(this.systemTelemetry)) this.systemTelemetry = [];
        this.systemTelemetry.push({ text: text, origin: payload.system_origin || payload.systemOrigin || '', ts: Date.now() });
        return null;
      }

      var origin = String(payload.system_origin || payload.systemOrigin || '').trim();
      var tsRaw = Number(payload.ts || 0);
      var ts = Number.isFinite(tsRaw) && tsRaw > 0 ? tsRaw : Date.now();
      var dedupeWindowMs = Number(payload.dedupe_window_ms || payload.dedupeWindowMs || 8000);
      if (!Number.isFinite(dedupeWindowMs) || dedupeWindowMs < 0) dedupeWindowMs = 8000;
      if (dedupeWindowMs > 60000) dedupeWindowMs = 60000;
      var canDedupe = payload.dedupe !== false;
      var systemThreadId = String(this.systemThreadId || 'system').trim() || 'system';
      var activeId = String((this.currentAgent && this.currentAgent.id) || '').trim();
      var targetId = activeId || systemThreadId;
      var isGlobalNotice = !!(
        this.isSystemNotificationGlobalToWorkspace &&
        this.isSystemNotificationGlobalToWorkspace(origin, text)
      );
      var routeToSystem =
        payload.route_to_system === true ||
        (payload.route_to_system !== false && isGlobalNotice);
      if (routeToSystem) targetId = systemThreadId;
      var activeThread = !!activeId && activeId === targetId;
      if (!this._systemMessageDedupeIndex || typeof this._systemMessageDedupeIndex !== 'object') this._systemMessageDedupeIndex = {};

      var targetRows = null;
      var targetCache = null;
      if (activeThread) {
        if (!Array.isArray(this.messages)) this.messages = [];
        targetRows = this.messages;
      } else {
        if (!this.conversationCache || typeof this.conversationCache !== 'object') this.conversationCache = {};
        targetCache = this.conversationCache[targetId];
        if (!targetCache || typeof targetCache !== 'object' || !Array.isArray(targetCache.messages)) {
          targetCache = { saved_at: Date.now(), token_count: 0, messages: [] };
          this.conversationCache[targetId] = targetCache;
        }
        targetRows = targetCache.messages;
      }

      if (!Array.isArray(targetRows)) return null;
      var dedupeKey = targetId + '|' + (origin || '_') + '|' + canonicalText;
      if (canDedupe) {
        for (var idx = targetRows.length - 1, scanned = 0; idx >= 0 && scanned < 24; idx -= 1) {
          var row = targetRows[idx];
          if (!row || row.thinking || row.streaming) continue;
          if (String(row.role || '').toLowerCase() !== 'system' || row.is_notice) continue;
          scanned += 1;
          var rowText = String(row.text || '').replace(/\s+/g, ' ').trim().toLowerCase();
          if (rowText !== canonicalText) continue;
          var rowTs = Number(row.ts || 0);
          if (Number.isFinite(rowTs) && Math.abs(ts - rowTs) > dedupeWindowMs) continue;
          var rowOrigin = String(row.system_origin || '').trim();
          if (rowOrigin && origin && rowOrigin !== origin && !/^error:/i.test(canonicalText)) continue;
          var repeatCount = Number(row._repeat_count || 1);
          if (!Number.isFinite(repeatCount) || repeatCount < 1) repeatCount = 1;
          repeatCount += 1;
          row._repeat_count = repeatCount;
          var priorMeta = String(row.meta || '').trim().replace(/\s*\|\s*repeated x\d+\s*$/i, '').trim();
          row.meta = (priorMeta ? (priorMeta + ' | ') : '') + 'repeated x' + repeatCount;
          row.ts = ts;
          this._systemMessageDedupeIndex[dedupeKey] = { id: row.id, ts: ts };
          if (activeThread) this.scheduleConversationPersist();
          else this.persistConversationCache();
          return row;
        }
      }

      var message = {
        id: ++msgId,
        role: 'system',
        text: text,
        meta: String(payload.meta || ''),
        tools: Array.isArray(payload.tools) ? payload.tools : [],
        system_origin: origin,
        ts: ts
      };
      targetRows.push(message);
      if (canDedupe && canonicalText) this._systemMessageDedupeIndex[dedupeKey] = { id: message.id, ts: ts };

      var store = Alpine.store('app');
      if (store && typeof store.saveAgentChatPreview === 'function') {
        store.saveAgentChatPreview(targetId, targetRows);
      }
      if (activeThread) {
        if (payload.auto_scroll !== false) this.scrollToBottom();
        this.scheduleConversationPersist();
      } else {
        if (targetCache) {
          targetCache.saved_at = Date.now();
          targetCache.token_count = 0;
        }
        this.persistConversationCache();
      }
      return message;
    },

    activateSystemThread: function(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var priorAgentId = String((this.currentAgent && this.currentAgent.id) || '').trim();
      if (priorAgentId && !this.isSystemThreadId(priorAgentId) && typeof this.captureConversationDraft === 'function') {
        this.captureConversationDraft(priorAgentId);
      }
      this.currentAgent = this.makeSystemThreadAgent();
      this.setStoreActiveAgentId(this.currentAgent.id || null);
      this._clearTypingTimeout();
      this._clearPendingWsRequest(this.currentAgent.id || '');
      this.sending = false;
      this._responseStartedAt = 0;
      this.tokenCount = 0;
      this.messageQueue = Array.isArray(this.messageQueue)
        ? this.messageQueue.filter(function(row) { return !row || !row.terminal; })
        : [];
      InfringAPI.wsDisconnect();
      this._wsAgent = null;
      this.sessions = [];
      this.showFreshArchetypeTiles = false;
      this.freshInitRevealMenu = false;
      this.terminalMode = true;
      var restored = this.restoreAgentConversation(this.currentAgent.id);
      if (!restored && opts.preserve_if_empty !== true) {
        this.messages = [];
      }
      if (typeof this.restoreConversationDraft === 'function') {
        this.restoreConversationDraft(this.currentAgent.id, 'terminal');
      }
      this.recomputeContextEstimate();
      this.refreshContextPressure();
      this.clearPromptSuggestions();
      this.$nextTick(() => {
        var input = document.getElementById('msg-input');
        if (input) input.focus();
        this.scrollToBottomImmediate();
        this.stabilizeBottomScroll();
        this.pinToLatestOnOpen(null, { maxFrames: 20 });
        this.scheduleMessageRenderWindowUpdate();
      });
    },

    defaultSlashAliases: function() {
      return {
        '/status': '/status',
        '/opt': '/continuity',
        '/q': '/queue',
        '/ctx': '/context',
        '/mods': '/model',
        '/mem': '/compact'
      };
    },

    normalizeSlashCommandName: function(value) {
      var name = String(value || '').trim().toLowerCase();
      if (!name) return '';
      return name.startsWith('/') ? name : ('/' + name);
    },

    findSlashCommandDefinition: function(value) {
      var target = this.normalizeSlashCommandName(value);
      if (!target) return null;
      var projectedRows = Array.isArray(this.filteredSlashCommands) ? this.filteredSlashCommands : [];
      for (var p = 0; p < projectedRows.length; p += 1) {
        var projected = projectedRows[p] && typeof projectedRows[p] === 'object' ? projectedRows[p] : null;
        if (!projected || projected.row_kind === 'heading' || projected.selectable === false) continue;
        if (this.normalizeSlashCommandName(projected.cmd) === target) return projected;
      }
      var rows = Array.isArray(this.slashCommands) ? this.slashCommands : [];
      for (var i = 0; i < rows.length; i += 1) {
        var row = rows[i] && typeof rows[i] === 'object' ? rows[i] : null;
        if (!row) continue;
        if (this.normalizeSlashCommandName(row.cmd) === target) return row;
      }
      return null;
    },

    formatSlashCommandUsage: function(value) {
      var target = this.normalizeSlashCommandName(value);
      if (!target) return '';
      var def = this.findSlashCommandDefinition(target);
      var desc = String(def && def.desc ? def.desc : '').trim();
      return desc ? ('`' + target + '` — ' + desc) : ('`' + target + '`');
    },

    loadSlashAliases: function() {
      var defaults = this.defaultSlashAliases();
      var persisted = {};
      try {
        var raw = localStorage.getItem(this.slashAliasStorageKey || '');
        if (raw) {
          var parsed = JSON.parse(raw);
          if (parsed && typeof parsed === 'object') persisted = parsed;
        }
      } catch(_) {}
      var merged = {};
      Object.keys(defaults).forEach(function(key) {
        var target = String(defaults[key] || '').trim().toLowerCase();
        var alias = String(key || '').trim().toLowerCase();
        if (!alias.startsWith('/') || !target.startsWith('/')) return;
        merged[alias] = target;
      });
      Object.keys(persisted).forEach(function(key) {
        var alias = String(key || '').trim().toLowerCase();
        var target = String(persisted[key] || '').trim().toLowerCase();
        if (!alias.startsWith('/') || !target.startsWith('/')) return;
        merged[alias] = target;
      });
      this.slashAliasMap = merged;
      return merged;
    },

    saveSlashAliases: function() {
      try {
        localStorage.setItem(
          this.slashAliasStorageKey || '',
          JSON.stringify(this.slashAliasMap || {})
        );
      } catch(_) {}
    },

    resolveSlashAlias: function(inputCmd, cmdArgs) {
      var cmd = this.normalizeSlashCommandName(inputCmd);
      var args = String(cmdArgs || '').trim();
      var aliases = this.slashAliasMap || {};
      var visited = {};
      var expandedCmd = cmd;
      var expandedArgs = args;
      var rendered = cmd + (args ? (' ' + args) : '');
      for (var depth = 0; depth < 5; depth += 1) {
        var target = String(aliases[expandedCmd] || '').trim();
        if (!target) break;
        if (visited[expandedCmd]) break;
        visited[expandedCmd] = true;
        rendered = target + (expandedArgs ? (' ' + expandedArgs) : '');
        var targetParts = target.split(/\s+/).filter(Boolean);
        if (!targetParts.length) break;
        expandedCmd = this.normalizeSlashCommandName(targetParts[0]);
        var trailing = targetParts.slice(1).join(' ').trim();
        if (trailing) {
          expandedArgs = trailing + (expandedArgs ? (' ' + expandedArgs) : '');
        }
      }
      return { cmd: expandedCmd, args: expandedArgs.trim(), expanded: rendered };
    },

    formatSlashAliasRows: function() {
      var self = this;
      var aliases = this.slashAliasMap || {};
      var rows = Object.keys(aliases)
        .sort()
        .map(function(alias) {
          var target = String(aliases[alias] || '').trim();
          var targetCommand = self.normalizeSlashCommandName(target.split(/\s+/)[0] || '');
          var usage = self.formatSlashCommandUsage(targetCommand);
          return '- `' + alias + '` → `' + target + '`' + (usage ? ('\n  ↳ ' + usage) : '');
        });
      return rows.join('\n');
    },

    fetchProactiveTelemetryAlerts: function(notify) {
      var self = this;
      return InfringAPI.get('/api/telemetry/alerts').then(function(payload) {
        var rows = Array.isArray(payload && payload.alerts) ? payload.alerts : [];
        var nextActions = Array.isArray(payload && payload.next_actions) ? payload.next_actions : [];
        var digest = rows.map(function(row) {
          return String((row && row.id) || '') + ':' + String((row && row.message) || '');
        }).join('|');
        self._telemetrySnapshot = payload && typeof payload === 'object' ? payload : null;
        self._continuitySnapshot = payload && payload.continuity ? payload.continuity : null;
        self.telemetryNextActions = nextActions.slice(0, 6);
        if (notify && digest && digest !== String(self._lastTelemetryAlertDigest || '')) {
          var rendered = rows.map(function(row) {
            var severity = String((row && row.severity) || 'info').toUpperCase();
            var message = String((row && row.message) || '').trim();
            var command = String((row && row.recommended_command) || '').trim();
            return '- [' + severity + '] ' + message + (command ? ('\n  ↳ `' + command + '`') : '');
          }).join('\n');
          var nextRendered = nextActions.slice(0, 3).map(function(row) {
            var cmd = String((row && row.command) || '').trim();
            var reason = String((row && row.reason) || '').trim();
            return '- `' + cmd + '`' + (reason ? ('\n  ↳ ' + reason) : '');
          }).join('\n');
          if (rendered) {
            self.pushSystemMessage({
              text: '**Telemetry Alerts**\n' + rendered + (nextRendered ? ('\n\n**Suggested Next Actions**\n' + nextRendered) : ''),
              system_origin: 'telemetry:alerts',
              ts: Date.now(),
              auto_scroll: false
            });
          }
        }
        self._lastTelemetryAlertDigest = digest;
        return payload;
      }).catch(function() {
        self._telemetrySnapshot = null;
        self.telemetryNextActions = [];
        return { ok: false, alerts: [] };
      });
    },

    staleMemoryWarningText: function() {
      return '';
    },

    thinkingTraceRows: function(msg) {
      var rows = [];
      if (!msg || !msg.thinking) return rows;
      var tools = Array.isArray(msg.tools) ? msg.tools : [];
      for (var i = 0; i < tools.length; i++) {
        var tool = tools[i];
        if (!tool || this.isThoughtTool(tool)) continue;
        var state = tool.running ? 'running' : (this.isBlockedTool(tool) ? 'blocked' : (tool.is_error ? 'error' : 'done'));
        rows.push({
          id: String(tool.id || ('trace-tool-' + i)),
          label: this.toolDisplayName(tool),
          state: state,
          state_label: state === 'done' ? 'complete' : state
        });
      }
      if (!rows.length) {
        var status = String(
          typeof this.thinkingStatusText === 'function'
            ? this.thinkingStatusText(msg)
            : (msg.thinking_status || '')
        ).trim();
        if (status) {
          rows.push({
            id: 'trace-status',
            label: status,
            state: 'running',
            state_label: 'active'
          });
        }
      }
      return rows.slice(-4);
    },

    publishSlashCommandFeedback: function(command, options) {
      var row = command && typeof command === 'object' ? command : {};
      var opts = options && typeof options === 'object' ? options : {};
      var cmd = String(row.cmd || row.display_command || opts.display_command || '').trim();
      var title = String(opts.title || row.title || row.label || cmd || 'Slash command').replace(/\s+/g, ' ').trim();
      var status = String(opts.status || row.operational_state || 'completed').replace(/\s+/g, ' ').trim();
      var text = String(
        opts.text ||
        opts.display_text ||
        row.operational_detail ||
        row.desc ||
        row.description ||
        row.operational_label ||
        status ||
        'Command accepted.'
      ).replace(/\s+/g, ' ').trim();
      if (text.length > 260) text = text.slice(0, 257) + '...';
      var noticeType = String(opts.notice_type || '').trim();
      if (!noticeType) {
        noticeType = /failed|error|denied/i.test(status) ? 'error' : (/manual|warning|required/i.test(status) ? 'warning' : 'info');
      }
      if (this._slashCommandFeedbackTimer) {
        clearTimeout(this._slashCommandFeedbackTimer);
        this._slashCommandFeedbackTimer = 0;
      }
      this.slashCommandFeedback = {
        id: 'slash-command-feedback-' + Date.now(),
        type: 'slash_command_feedback_projection',
        command: cmd,
        title: title,
        text: text,
        status: status,
        notice_type: noticeType,
        created_at: Date.now(),
        source_authority: row.source === 'agent_runtime_command_catalog'
          ? 'gateway.agent_runtime_command_catalog'
          : 'client.runtime.slash_command_projection'
      };
      var shouldPersist = opts.persist === true || /manual|required|failed|error|denied/i.test(status);
      if (!shouldPersist) {
        var self = this;
        this._slashCommandFeedbackTimer = setTimeout(function() {
          if (self.slashCommandFeedback && self.slashCommandFeedback.id) self.slashCommandFeedback = null;
          self._slashCommandFeedbackTimer = 0;
        }, 12000);
      }
      return this.slashCommandFeedback;
    },

    dismissSlashCommandFeedback: function() {
      if (this._slashCommandFeedbackTimer) {
        clearTimeout(this._slashCommandFeedbackTimer);
        this._slashCommandFeedbackTimer = 0;
      }
      this.slashCommandFeedback = null;
    },

    emitCommandFailureNotice: function(command, error, fallbackCommands) {
      var cmd = String(command || '').trim() || '/status';
      var message = String(error && error.message ? error.message : error || 'command_failed').trim();
      if (message.length > 220) message = message.slice(0, 217) + '...';
      if (typeof this.publishSlashCommandFeedback === 'function') {
        this.publishSlashCommandFeedback({ cmd: cmd, title: 'Command ' + cmd + ' failed' }, {
          status: 'failed',
          notice_type: 'error',
          text: message,
          persist: true
        });
      }
      var fallbacks = Array.isArray(fallbackCommands) ? fallbackCommands : [];
      var fallbackText = fallbacks
        .map(function(row) { return '`' + String(row || '').trim() + '`'; })
        .filter(Boolean)
        .join(' · ');
      this.messages.push({
        id: ++msgId,
        role: 'system',
        is_notice: true,
        notice_type: 'error',
        notice_label: 'Command ' + cmd + ' failed',
        text:
          'Command `' + cmd + '` failed: ' + message +
          (fallbackText ? ('\nTry recovery: ' + fallbackText) : ''),
        meta: '',
        tools: [],
        system_origin: 'slash:error',
        ts: Date.now()
      });
      this.scrollToBottom();
      this.scheduleConversationPersist();
    },

    get filteredSlashCommands() {
      if (this.showSlashMenu && typeof this.fetchAgentRuntimeSlashCommands === 'function') {
        this.fetchAgentRuntimeSlashCommands(false);
      }
      var base = Array.isArray(this.slashCommands) ? this.slashCommands.slice() : [];
      var aliases = this.slashAliasMap || {};
      Object.keys(aliases).forEach(function(alias) {
        if (!base.some(function(c) { return c && c.cmd === alias; })) {
          base.push({
            cmd: alias,
            desc: 'Alias → ' + String(aliases[alias] || ''),
            source: 'alias'
          });
        }
      });
      var runtimeRows = Array.isArray(this.agentRuntimeSlashCommandRows) ? this.agentRuntimeSlashCommandRows : [];
      var gatewayByCmd = {};
      runtimeRows.forEach(function(row) {
        if (!row || row.group_id !== 'infring_native_commands') return;
        var cmd = String(row.cmd || '').trim().toLowerCase();
        if (cmd) gatewayByCmd[cmd] = row;
      });
      var localOperational = function(cmd) {
        var target = String(cmd || '').trim().toLowerCase();
        if (/^\/(model|apikey|file|folder|stop|usage|think|context|verbose|queue|status|alerts|next|memory|continuity|aliases|alias|opt|clear|help)$/i.test(target)) {
          return {
            operational_state: 'connected',
            operational_label: 'Operational',
            operational_detail: 'Handled by the current InfRing client command path.',
            connected: true,
            fully_operational: true
          };
        }
        return {
          operational_state: 'intent_route_only',
          operational_label: 'Legacy route',
          operational_detail: 'Cataloged as an InfRing slash command; full operational proof is not yet attached.',
          connected: true,
          fully_operational: false
        };
      };
      var commandRow = function(row) {
        var cmd = String(row && row.cmd || '').trim();
        var gateway = gatewayByCmd[cmd.toLowerCase()] || null;
        var meta = gateway || localOperational(cmd);
        return {
          row_kind: 'command',
          cmd: cmd,
          desc: String(row && row.desc || gateway && gateway.desc || '').trim(),
          title: String(row && row.title || gateway && gateway.title || cmd).trim(),
          source: String(row && row.source || 'infring_native_slash'),
          command_id: String(gateway && gateway.command_id || ('infring-local:' + cmd.replace(/^\//, ''))),
          intent_id: String(gateway && gateway.intent_id || ''),
          engine_id: String(gateway && gateway.engine_id || 'infring'),
          group_id: 'infring_native_commands',
          group_title: 'InfRing native / commands',
          execution_kind: String(gateway && gateway.execution_kind || 'client_command_handler'),
          safety_class: String(gateway && gateway.safety_class || 'control'),
          operational_state: String(meta.operational_state || 'stubbed_or_unwired'),
          operational_label: String(meta.operational_label || meta.operational_state || 'Unknown'),
          operational_detail: String(meta.operational_detail || ''),
          connected: meta.connected !== false,
          fully_operational: meta.fully_operational === true,
          selectable: true,
          action_route: String(gateway && gateway.action_route || '')
        };
      };
      var sections = [];
      var f = String(this.slashFilter || '').trim().toLowerCase();
      var matches = function(row) {
        if (!f) return true;
        return String(row.cmd || '').toLowerCase().indexOf(f) !== -1 ||
          String(row.desc || '').toLowerCase().indexOf(f) !== -1 ||
          String(row.title || '').toLowerCase().indexOf(f) !== -1 ||
          String(row.operational_label || '').toLowerCase().indexOf(f) !== -1;
      };
      var addSection = function(label, rows, source) {
        var filtered = rows.filter(matches);
        if (!filtered.length) return;
        sections.push({
          row_kind: 'heading',
          cmd: '__heading_' + source + '_' + sections.length,
          label: label,
          desc: '',
          source: source,
          selectable: false
        });
        Array.prototype.push.apply(sections, filtered);
      };
      var activeRuntimeRows = runtimeRows.filter(function(row) {
        return row && row.group_id === 'runtime_native_commands';
      });
      addSection(
        activeRuntimeRows.length && activeRuntimeRows[0].group_title ? activeRuntimeRows[0].group_title : 'Runtime / commands',
        activeRuntimeRows,
        'agent_runtime_command_catalog'
      );
      var localRows = base.map(commandRow).filter(function(row) { return !!row.cmd; });
      runtimeRows.forEach(function(row) {
        if (!row || row.group_id !== 'infring_native_commands') return;
        var cmd = String(row.cmd || '').trim().toLowerCase();
        if (!cmd) return;
        if (!localRows.some(function(local) { return String(local.cmd || '').trim().toLowerCase() === cmd; })) {
          localRows.push(row);
        }
      });
      addSection('InfRing native / commands', localRows, 'infring_native_slash');
      return sections;
    },
