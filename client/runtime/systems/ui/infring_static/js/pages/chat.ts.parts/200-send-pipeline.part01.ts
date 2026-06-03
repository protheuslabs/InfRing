
    async sendTerminalMessage() {
      if (this.showFreshArchetypeTiles) {
        InfringToast.info('Launch agent initialization before running terminal commands.');
        return;
      }
      var activeAgent = this.ensureValidCurrentAgent({ clear_when_missing: true });
      if (!activeAgent || !this.inputText.trim()) return;
      if (!this.isSystemThreadAgent(activeAgent) && this.isArchivedAgentRecord && this.isArchivedAgentRecord(activeAgent)) {
        InfringToast.info('This agent is archived. Revive it to run commands.');
        return;
      }
      this.showFreshArchetypeTiles = false;
      var command = this.inputText.trim();
      this.pushInputHistoryEntry('terminal', command);
      this.inputText = '';
      this.terminalSelectionStart = 0;

      var ta = document.getElementById('msg-input');
      if (ta) ta.style.height = '';

      if (this.sending) {
        this._reconcileSendingState();
      }
      if (this.sending) {
        this.messageQueue.push({
          queue_id: this.nextPromptQueueId(),
          queue_kind: 'terminal',
          queued_at: Date.now(),
          terminal: true,
          command: command
        });
        return;
      }

      this._sendTerminalPayload(command, activeAgent.id);
    },

    async sendMessage() {
      if (this.terminalMode) {
        await this.sendTerminalMessage();
        return;
      }
      if (this.showFreshArchetypeTiles && !this.freshInitLaunching) {
        if (this.freshInitAwaitingOtherPrompt) {
          this.captureFreshInitOtherPrompt();
          return;
        }
        InfringToast.info('Launch agent initialization before chatting.');
        return;
      }
      var activeAgent = this.ensureValidCurrentAgent({ clear_when_missing: true });
      if (!activeAgent || (!this.inputText.trim() && !this.attachments.length)) return;
      if (this.isArchivedAgentRecord && this.isArchivedAgentRecord(activeAgent)) {
        InfringToast.info('This agent is archived. Revive it to continue this chat.');
        return;
      }
      if (this.isSystemThreadAgent(activeAgent)) {
        if (Array.isArray(this.attachments) && this.attachments.length) {
          InfringToast.info('System thread does not accept file attachments.');
          this.attachments = [];
        }
        await this.sendTerminalMessage();
        return;
      }
      this.showFreshArchetypeTiles = false;
      var rawInput = String(this.inputText == null ? '' : this.inputText);
      var text = rawInput.trim();
      var condensedLargePaste = false;
      if (text && this.shouldConvertLargePasteToAttachment && this.shouldConvertLargePasteToAttachment(rawInput)) {
        var largePasteAttachment = this.buildLargePasteMarkdownAttachment && this.buildLargePasteMarkdownAttachment(rawInput);
        if (largePasteAttachment && largePasteAttachment.file) {
          if (!Array.isArray(this.attachments)) this.attachments = [];
          this.attachments.push(largePasteAttachment);
          text = '';
          condensedLargePaste = true;
        }
      }
      if (text || condensedLargePaste) this.pushInputHistoryEntry('chat', text || '[File: Pasted markdown.md]');
      if (condensedLargePaste) InfringToast.info('Large paste moved to Pasted markdown.md');
      if (text.startsWith('/') && !this.attachments.length) {
        var cmd = text.split(' ')[0].toLowerCase();
        var cmdArgs = text.substring(cmd.length).trim();
        var aliasResolution = this.resolveSlashAlias(cmd, cmdArgs);
        var routedCmd = String(aliasResolution && aliasResolution.cmd ? aliasResolution.cmd : cmd).toLowerCase();
        var routedArgs = String(aliasResolution && typeof aliasResolution.args === 'string' ? aliasResolution.args : cmdArgs).trim();
        var matched = this.slashCommands.find(function(c) { return c.cmd === routedCmd; });
        if (matched) {
          this.executeSlashCommand(matched.cmd, routedArgs);
          return;
        }
      }
      if (typeof this.restoreAgentRuntimeEngineSelection === 'function') this.restoreAgentRuntimeEngineSelection();
      var selectedRuntimeEngineId = String(this.selectedAgentRuntimeEngineId || 'infring_native').trim() || 'infring_native';
      var usesExternalRuntime = typeof this.isExternalAgentRuntimeEngineSelected === 'function'
        ? this.isExternalAgentRuntimeEngineSelected(selectedRuntimeEngineId)
        : selectedRuntimeEngineId !== 'infring_native';
      if (!usesExternalRuntime) {
        var availableModels = typeof this.ensureUsableModelsForChatSend === 'function'
          ? await this.ensureUsableModelsForChatSend('chat_send')
          : (typeof this.currentAvailableModelCount === 'function' ? this.currentAvailableModelCount() : 0);
        if (availableModels <= 0) {
          if (typeof this.injectNoModelsGuidance === 'function') this.injectNoModelsGuidance('chat_send');
          if (typeof this.addNoModelsRecoveryNotice === 'function') this.addNoModelsRecoveryNotice('chat_send', 'model_discover');
          return;
        }
      }
      this.inputText = '';
      var ta = document.getElementById('msg-input');
      if (ta) ta.style.height = '';
      var fileRefs = [];
      var uploadedFiles = [];
      if (this.attachments.length) {
        for (var i = 0; i < this.attachments.length; i++) {
          var att = this.attachments[i];
          att.uploading = true;
          try {
            var uploadRes = await InfringAPI.upload(activeAgent.id, att.file);
            fileRefs.push('[File: ' + att.file.name + ']');
            uploadedFiles.push({ file_id: uploadRes.file_id, filename: uploadRes.filename, content_type: uploadRes.content_type });
          } catch(e) {
            var reason = (e && e.message) ? String(e.message) : 'upload_failed';
            InfringToast.error('Failed to upload ' + att.file.name + ': ' + reason);
            fileRefs.push('[File: ' + att.file.name + ' (upload failed)]');
          }
          att.uploading = false;
        }
        for (var j = 0; j < this.attachments.length; j++) {
          if (this.attachments[j].preview) URL.revokeObjectURL(this.attachments[j].preview);
        }
        this.attachments = [];
      }
      var finalText = text;
      if (fileRefs.length) {
        finalText = (text ? text + '\n' : '') + fileRefs.join('\n');
      }
      var msgImages = uploadedFiles.filter(function(f) { return f.content_type && f.content_type.startsWith('image/'); });
      if (this.sending) {
        this._reconcileSendingState();
      }
      if (this.sending) {
        this.messageQueue.push({
          queue_id: this.nextPromptQueueId(),
          queue_kind: 'prompt',
          queued_at: Date.now(),
          text: finalText,
          files: uploadedFiles,
          images: msgImages,
          agent_runtime_engine_id: selectedRuntimeEngineId
        });
        this.scheduleConversationPersist();
        return;
      }
      var shouldMorphSend = !!(text && !uploadedFiles.length && !msgImages.length && !fileRefs.length && !this.sending);
      var morphSnapshot = shouldMorphSend && this.captureComposerSendMorph
        ? this.captureComposerSendMorph(text)
        : null;
      var appended = this.appendUserChatMessage(finalText, msgImages, { deferPersist: true });
      if (morphSnapshot && appended && appended.id != null && this.playComposerSendMorphToMessage) {
        var self = this;
        this.$nextTick(function() {
          self.playComposerSendMorphToMessage(morphSnapshot, appended.id);
        });
      } else if (morphSnapshot && this.clearComposerSendMorph) {
        this.clearComposerSendMorph(morphSnapshot);
      }
      this.scheduleConversationPersist();
      this._sendPayload(finalText, uploadedFiles, msgImages, {
        agent_id: activeAgent.id,
        agent_runtime_engine_id: selectedRuntimeEngineId
      });
    },

    isExternalAgentRuntimeEngineSelected: function(engineId) {
      var id = String(engineId || this.selectedAgentRuntimeEngineId || 'infring_native').trim() || 'infring_native';
      return !!id && id !== 'infring_native';
    },

    loadAgentRuntimePermissionPolicy: function() {
      if (this.agentRuntimePermissionPolicy && typeof this.agentRuntimePermissionPolicy === 'object') return this.agentRuntimePermissionPolicy;
      var policy = { always_allowed_tool_calls: [], revoked_default_read_tools: [], default_allow_read_tools: true, gatekeeper_kind: 'user' };
      try {
        var raw = window.localStorage.getItem(this.agentRuntimePermissionPolicyStorageKey);
        var parsed = raw ? JSON.parse(raw) : null;
        if (parsed && typeof parsed === 'object') {
          policy.always_allowed_tool_calls = Array.isArray(parsed.always_allowed_tool_calls) ? parsed.always_allowed_tool_calls : [];
          policy.revoked_default_read_tools = Array.isArray(parsed.revoked_default_read_tools) ? parsed.revoked_default_read_tools : [];
          policy.default_allow_read_tools = parsed.default_allow_read_tools !== false;
          policy.gatekeeper_kind = String(parsed.gatekeeper_kind || 'user');
        }
      } catch (_e) {}
      this.agentRuntimePermissionPolicy = policy;
      return policy;
    },

    saveAgentRuntimePermissionPolicy: function(policy) {
      var next = policy && typeof policy === 'object' ? policy : this.loadAgentRuntimePermissionPolicy();
      this.agentRuntimePermissionPolicy = next;
      try { window.localStorage.setItem(this.agentRuntimePermissionPolicyStorageKey, JSON.stringify(next)); } catch (_e) {}
      return next;
    },

    agentRuntimePermissionPolicyProjection: function() {
      var policy = this.loadAgentRuntimePermissionPolicy();
      return {
        gatekeeper_kind: 'user',
        default_allow_read_tools: policy.default_allow_read_tools !== false,
        revoked_default_read_tools: Array.isArray(policy.revoked_default_read_tools) ? policy.revoked_default_read_tools.slice(0, 24) : [],
        always_allowed_tool_calls: Array.isArray(policy.always_allowed_tool_calls) ? policy.always_allowed_tool_calls.slice(0, 48) : []
      };
    },

    permissionRequestPreview: function(row) {
      var request = row && typeof row === 'object' ? row : {};
      var tool = String(request.tool_id || request.capability || 'tool call');
      var engine = String(request.engine_id || this.selectedAgentRuntimeEngineId || 'runtime');
      return engine + ' requests ' + tool;
    },

    permissionRequestReason: function(row) {
      return String(row && row.reason || 'Permission is required before this tool call can proceed.');
    },

    enqueueAgentRuntimePermissionRequest: function(request) {
      var row = request && typeof request === 'object' ? request : null;
      if (!row || !row.approval_id) return;
      var pending = Array.isArray(this.pendingAgentRuntimePermissionRequests) ? this.pendingAgentRuntimePermissionRequests.slice() : [];
      var id = String(row.approval_id);
      pending = pending.filter(function(item) { return String(item && item.approval_id || '') !== id; });
      pending.unshift(row);
      this.pendingAgentRuntimePermissionRequests = pending.slice(0, 5);
      InfringToast.info('Permission requested: ' + this.permissionRequestPreview(row));
    },

    submitAgentRuntimePermissionDecision: function(row, decision) {
      var request = row && typeof row === 'object' ? row : {};
      var approvalId = String(request.approval_id || '').trim();
      var choice = String(decision || '').trim();
      if (!approvalId || ['allow_once', 'deny', 'always_allow_tool_call'].indexOf(choice) < 0) return Promise.resolve(null);
      if (choice === 'always_allow_tool_call') {
        var policy = this.loadAgentRuntimePermissionPolicy();
        var toolId = String(request.tool_id || '').trim();
        if (toolId) {
          var always = Array.isArray(policy.always_allowed_tool_calls) ? policy.always_allowed_tool_calls.slice() : [];
          if (always.indexOf(toolId) < 0) always.push(toolId);
          policy.always_allowed_tool_calls = always;
          this.saveAgentRuntimePermissionPolicy(policy);
        }
      }
      var self = this;
      return InfringAPI.post('/api/shell-socket/approvals/' + encodeURIComponent(approvalId) + '/decision', {
        decision: choice,
        tool_id: String(request.tool_id || ''),
        tool_call_ref: String(request.tool_call_ref || ''),
        engine_id: String(request.engine_id || this.selectedAgentRuntimeEngineId || ''),
        session_id: String(request.session_id || (this.currentAgent && (this.currentAgent.session_id || this.currentAgent.id)) || ''),
        gatekeeper_kind: 'user'
      }).then(function(payload) {
        self.pendingAgentRuntimePermissionRequests = (Array.isArray(self.pendingAgentRuntimePermissionRequests) ? self.pendingAgentRuntimePermissionRequests : [])
          .filter(function(item) { return String(item && item.approval_id || '') !== approvalId; });
        if (choice === 'deny') InfringToast.info('Permission denied for ' + self.permissionRequestPreview(request));
        else InfringToast.success(choice === 'always_allow_tool_call' ? 'Always allowed: ' + String(request.tool_id || 'tool call') : 'Permission allowed once');
        return payload;
      }).catch(function(e) {
        InfringToast.error(e && e.message ? String(e.message) : 'permission decision failed');
        return null;
      });
    },

    agentRuntimeActivityKindLabel: function(kind) {
      var value = String(kind || '').trim();
      if (value === 'reasoning_summary') return 'Reasoning summary';
      if (value === 'plan_update') return 'Plan update';
      if (value === 'tool_call_event') return 'Tool call';
      if (value === 'command_event') return 'Command';
      if (value === 'file_change_event') return 'File change';
      if (value === 'permission_event') return 'Permission';
      if (value === 'assistant_delta') return 'Assistant draft';
      if (value === 'error') return 'Runtime error';
      if (value === 'started') return 'Started';
      if (value === 'completed') return 'Completed';
      return value ? value.replace(/_/g, ' ') : 'Activity';
    },

    agentRuntimeActivityEventsToTools: function(events, engineId) {
      var rows = Array.isArray(events) ? events : [];
      var out = [];
      for (var i = 0; i < rows.length && out.length < 40; i += 1) {
        var event = rows[i] && typeof rows[i] === 'object' ? rows[i] : {};
        var kind = String(event.activity_kind || event.kind || '').trim();
        var text = String(event.display_text || event.text || event.summary || '').replace(/\r\n/g, '\n').trim();
        var providerType = String(event.provider_event_type || event.event_type || '').trim();
        if (!text && !providerType) continue;
        if (kind === 'assistant_delta' && text.length > 900) continue;
        var label = this.agentRuntimeActivityKindLabel(kind);
        out.push({
          name: label,
          status: String(event.status || 'completed').trim() || 'completed',
          input: providerType ? ('event: ' + providerType) : '',
          result: text || providerType,
          output: text || providerType,
          summary: text || providerType,
          tool_call_ref: String(event.item_id || event.sequence_no || ('activity-' + i)).slice(0, 160),
          agent_runtime_engine_id: String(event.engine_id || engineId || '').trim(),
          agent_activity_event: true,
          notice_type: kind,
          ts: Date.now()
        });
      }
      return out;
    },

    appendAgentRuntimeActivityToThinkingRow: function(event, engineId) {
      var rows = Array.isArray(this.messages) ? this.messages : [];
      var target = null;
      for (var i = rows.length - 1; i >= 0; i -= 1) {
        var row = rows[i];
        if (!row || !row.thinking) continue;
        if (String(row.agent_runtime_engine_id || '') !== String(engineId || '')) continue;
        target = row;
        break;
      }
      if (!target) return;
      var tools = this.agentRuntimeActivityEventsToTools([event], engineId);
      if (!tools.length) return;
      if (!Array.isArray(target.tools)) target.tools = [];
      target.tools = target.tools.concat(tools).slice(-40);
      target.text = 'Working through runtime activity...';
      target.meta = 'Agent runtime: ' + String(engineId || 'runtime') + ' | streaming activity';
      target._stream_updated_at = Date.now();
      if (typeof this.syncActiveChatMessages === 'function') this.syncActiveChatMessages();
      if (typeof this.scheduleMessageRenderWindowUpdate === 'function') this.scheduleMessageRenderWindowUpdate();
      this.scrollToBottom();
    },

    postAgentRuntimeTurnStreaming: async function(body, onActivity) {
      if (typeof fetch !== 'function' || typeof TextDecoder === 'undefined') {
        return InfringAPI.post('/api/shell-socket/agent-runtime/turn', body);
      }
      var response = await fetch('/api/shell-socket/agent-runtime/turn/stream', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Accept': 'application/x-ndjson'
        },
        body: JSON.stringify(body || {})
      });
      if (!response.ok || !response.body || typeof response.body.getReader !== 'function') {
        throw new Error('agent_runtime_stream_unavailable');
      }
      var reader = response.body.getReader();
      var decoder = new TextDecoder();
      var buffer = '';
      var finalPayload = null;
      var drainLine = function(line) {
        var text = String(line || '').trim();
        if (!text) return;
        var parsed = null;
        try { parsed = JSON.parse(text); } catch (_e) { return; }
        if (!parsed || typeof parsed !== 'object') return;
        if (parsed.type === 'activity' && parsed.event && typeof onActivity === 'function') {
          onActivity(parsed.event);
          return;
        }
        if (parsed.type === 'final') finalPayload = parsed.payload || {};
      };
      while (true) {
        var chunk = await reader.read();
        if (chunk.done) break;
        buffer += decoder.decode(chunk.value, { stream: true });
        var lines = buffer.split(/\r?\n/);
        buffer = lines.pop() || '';
        for (var i = 0; i < lines.length; i += 1) drainLine(lines[i]);
      }
      buffer += decoder.decode();
      drainLine(buffer);
      return finalPayload || { ok: false, error: 'agent_runtime_stream_missing_final' };
    },

    async _sendAgentRuntimeSocketPayload(targetAgentId, finalText, uploadedFiles, msgImages, runtimeEngineId) {
      var engineId = String(runtimeEngineId || this.selectedAgentRuntimeEngineId || '').trim();
      if (!engineId || engineId === 'infring_native') return;
      var startedAt = Date.now();
      this._responseStartedAt = startedAt;
      var thinkingMessage = {
        id: ++msgId,
        role: 'agent',
        text: '',
        meta: 'Agent runtime: ' + engineId,
        thinking: true,
        tools: [],
        ts: Date.now(),
        agent_runtime_engine_id: engineId
      };
      this.messages.push(thinkingMessage);
      this.scrollToBottom();
      this.scheduleConversationPersist();

      try {
        var contextRows = [];
        var historyRows = Array.isArray(this.messages) ? this.messages : [];
        var currentTextForContext = String(finalText || '').trim();
        for (var ctxIdx = Math.max(0, historyRows.length - 64); ctxIdx < historyRows.length; ctxIdx++) {
          var ctxMsg = historyRows[ctxIdx] || {};
          if (ctxMsg.thinking) continue;
          var ctxRole = String(ctxMsg.role || ctxMsg.origin_kind || '').trim() || 'message';
          var ctxText = '';
          if (typeof this.extractMessageVisibleText === 'function') {
            ctxText = String(this.extractMessageVisibleText(ctxMsg) || '');
          } else {
            ctxText = String(ctxMsg.text || ctxMsg.message || ctxMsg.content || '');
          }
          ctxText = ctxText.replace(/\r\n/g, '\n').replace(/[ \t]+\n/g, '\n').trim();
          if (!ctxText) continue;
          if (ctxIdx === historyRows.length - 1 && ctxRole === 'user' && ctxText.trim() === currentTextForContext) continue;
          if (ctxRole === 'agent' || ctxRole === 'ai') ctxRole = 'assistant';
          if (ctxRole === 'human') ctxRole = 'user';
          contextRows.push({
            id: String(ctxMsg.id || ('message-' + ctxIdx)).slice(0, 160),
            role: ctxRole.slice(0, 40),
            text_preview: ctxText.slice(0, 2400),
            detail_ref: String(ctxMsg.detail_ref || ctxMsg.id || '').slice(0, 240),
            timestamp: ctxMsg.ts || ctxMsg.timestamp || null,
            source_kind: ctxMsg.tool_call_ref || ctxRole === 'tool' ? 'tool_result_bundle' : 'interaction_unit'
          });
        }
        contextRows = contextRows.slice(-49);
        var turnRequest = {
          engine_id: engineId,
          agent_id: targetAgentId,
          session_id: String((this.currentAgent && (this.currentAgent.session_id || this.currentAgent.id)) || targetAgentId || ''),
          message: String(finalText || ''),
          attachments: Array.isArray(uploadedFiles) ? uploadedFiles.slice(0, 12) : [],
          permission_policy: this.agentRuntimePermissionPolicyProjection(),
          context_projection: {
            schema_version: 1,
            source: 'shell_bounded_message_projection',
            fanout_target: 7,
            rows: contextRows
          }
        };
        var streamedActivityEvents = [];
        var selfRuntimeStream = this;
        var res = await this.postAgentRuntimeTurnStreaming(turnRequest, function(event) {
          streamedActivityEvents.push(event);
          selfRuntimeStream.appendAgentRuntimeActivityToThinkingRow(event, engineId);
        }).catch(function() {
          return InfringAPI.post('/api/shell-socket/agent-runtime/turn', turnRequest);
        });
        typeof this.clearTransientThinkingRows === 'function'
          ? this.clearTransientThinkingRows({ force: true })
          : (this.messages = this.messages.filter(function(m) { return !m.thinking; }));
        if (res && res.pending_permission_request) {
          this.enqueueAgentRuntimePermissionRequest(res.pending_permission_request);
        }
        var runtimePayloadText = String((res && (res.display_text || res.output_text || res.text || res.response || res.output_preview)) || '').trim();
        var runtimeText = this.stripModelPrefix(this.sanitizeToolText(runtimePayloadText || ''));
        var runtimeDurationMs = Math.max(0, Date.now() - startedAt);
        var runtimeDuration = this.formatResponseDuration(runtimeDurationMs);
        var runtimeMeta = 'runtime ' + engineId;
        if (res && res.status) runtimeMeta += ' | ' + String(res.status);
        if (runtimeDuration) runtimeMeta += ' | ' + runtimeDuration;
        if (res && res.result_ref) runtimeMeta += ' | result';
        var responseActivityEvents = Array.isArray(res && res.agent_activity_events) ? res.agent_activity_events : [];
        var finalActivityEvents = responseActivityEvents.length ? responseActivityEvents : streamedActivityEvents;
        var runtimeActivityTools = this.agentRuntimeActivityEventsToTools(finalActivityEvents, engineId);
        if (!String(runtimeText || '').trim()) {
          if (!(res && res.pending_permission_request)) InfringToast.info('Agent runtime returned no display text.');
          this._clearPendingWsRequest(targetAgentId);
          this._inflightPayload = null;
          this.sending = false;
          this._responseStartedAt = 0;
          this.tokenCount = 0;
          this._clearTypingTimeout();
          this.setAgentLiveActivity(targetAgentId, 'idle', { optimistic: true, source: 'agent_runtime_socket' });
          this.scheduleConversationPersist();
          return;
        }
        var runtimeMessage = {
          id: ++msgId,
          role: 'agent',
          text: runtimeText,
          meta: runtimeMeta,
          tools: runtimeActivityTools,
          agent_activity_events: finalActivityEvents.slice(-80),
          agent_activity_event_count: Number(res && res.activity_event_count) || runtimeActivityTools.length,
          ts: Date.now(),
          agent_id: targetAgentId,
          agent_name: this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '',
          isHtml: false,
          _typingVisual: false,
          agent_runtime_engine_id: engineId
        };
        var pushedRuntimeMessage = this.pushAgentMessageDeduped(runtimeMessage, { dedupe_window_ms: 90000 }) || runtimeMessage;
        this.markAgentMessageComplete(pushedRuntimeMessage);
        this._clearPendingWsRequest(targetAgentId);
        this._inflightPayload = null;
        this.scheduleConversationPersist();
      } catch (e) {
        typeof this.clearTransientThinkingRows === 'function'
          ? this.clearTransientThinkingRows({ force: true })
          : (this.messages = this.messages.filter(function(m) { return !m.thinking; }));
        InfringToast.error(e && e.message ? e.message : 'agent runtime failed');
      } finally {
        this.sending = false;
        this._responseStartedAt = 0;
        this.tokenCount = 0;
        this._clearTypingTimeout();
        this.setAgentLiveActivity(targetAgentId, 'idle', { optimistic: true, source: 'agent_runtime_socket' });
      }
    },

    async _sendTerminalPayload(command, agentIdOverride) {
      var targetAgentId = String(agentIdOverride || (this.currentAgent && this.currentAgent.id) || '').trim();
      if (!targetAgentId) return;
      if (this.isSystemThreadId(targetAgentId)) {
        await this._sendSystemTerminalPayload(command);
        return;
      }
      var terminalAgent = this.resolveAgent ? (this.resolveAgent(targetAgentId) || this.currentAgent) : this.currentAgent;
      if (terminalAgent && this.isArchivedAgentRecord && this.isArchivedAgentRecord(terminalAgent)) {
        this.sending = false;
        this._responseStartedAt = 0;
        this._clearPendingWsRequest(targetAgentId);
        InfringToast.info('Archived conversations are read-only. Revive this agent to run commands.');
        return;
      }
      this.sending = true;
      this.setAgentLiveActivity(targetAgentId, 'working');
      this._responseStartedAt = Date.now();
      this._appendTerminalMessage({
        role: 'terminal',
        text: this._terminalPromptLine(this.terminalPromptPath, command),
        meta: this.terminalPromptPath,
        tools: [],
        ts: Date.now(),
        terminal_source: 'user',
        cwd: this.terminalPromptPath
      });
      this.recomputeContextEstimate();
      this.scrollToBottom();
      this.scheduleConversationPersist();

      try {
        var ack = await InfringAPI.post('/api/shell-socket/terminal/commands', {
          agent_id: targetAgentId,
          command: command,
          cwd: this.terminalPromptPath,
        });
        if (!ack || ack.rejected) throw new Error(String((ack && ack.reason_code) || 'terminal_command_rejected'));
        this.sending = false;
        this._responseStartedAt = 0;
        this.setAgentLiveActivity(targetAgentId, 'idle', { optimistic: true, source: 'shell_socket_terminal_ack' });
      } catch (e) {
        this.sending = false;
        this._responseStartedAt = 0;
        this._clearPendingWsRequest(targetAgentId);
        InfringToast.error(e && e.message ? e.message : 'command failed');
      }
    },

    async _sendPayload(finalText, uploadedFiles, msgImages, options) {
      var opts = options && typeof options === 'object' ? options : {};
      var ensuredAgent = this.ensureValidCurrentAgent({ clear_when_missing: true });
      if (!ensuredAgent && !opts.agent_id) {
        this.sending = false;
        this._responseStartedAt = 0;
        return;
      }
      this.sending = true;
      var targetAgentId = String(
        opts.agent_id || (ensuredAgent && ensuredAgent.id) || (this.currentAgent && this.currentAgent.id) || ''
      ).trim();
      if (!targetAgentId) {
        this.sending = false;
        this._responseStartedAt = 0;
        return;
      }
      var targetAgent = ensuredAgent || (this.resolveAgent ? this.resolveAgent(targetAgentId) : null) || this.currentAgent;
      if (!this.isSystemThreadId(targetAgentId) && targetAgent && this.isArchivedAgentRecord && this.isArchivedAgentRecord(targetAgent)) {
        this.sending = false;
        this._responseStartedAt = 0;
        this._clearPendingWsRequest(targetAgentId);
        this._inflightPayload = null;
        InfringToast.info('Archived conversations are read-only. Revive this agent to continue this chat.');
        return;
      }
      this.setAgentLiveActivity(targetAgentId, 'typing');
      var safeFiles = Array.isArray(uploadedFiles) ? uploadedFiles.slice() : [];
      var safeImages = Array.isArray(msgImages) ? msgImages.slice() : [];
      var runtimeEngineId = String(opts.agent_runtime_engine_id || this.selectedAgentRuntimeEngineId || 'infring_native').trim() || 'infring_native';
      if (
        !opts.retry_from_failover ||
        !this._inflightPayload ||
        String(this._inflightPayload.agent_id || '') !== targetAgentId
      ) {
        this._inflightPayload = {
          agent_id: targetAgentId,
          final_text: String(finalText || ''),
          uploaded_files: safeFiles,
          msg_images: safeImages,
          agent_runtime_engine_id: runtimeEngineId,
          failover_attempted: !!opts.retry_from_failover,
          created_at: Date.now()
        };
      } else {
        this._inflightPayload.final_text = String(finalText || '');
        this._inflightPayload.uploaded_files = safeFiles;
        this._inflightPayload.msg_images = safeImages;
        this._inflightPayload.agent_runtime_engine_id = runtimeEngineId;
        this._inflightPayload.retry_started_at = Date.now();
      }
      this._pendingAutoModelSwitchBaseline = '';
      if (typeof this.isExternalAgentRuntimeEngineSelected === 'function' && this.isExternalAgentRuntimeEngineSelected(runtimeEngineId)) {
        await this._sendAgentRuntimeSocketPayload(targetAgentId, finalText, safeFiles, safeImages, runtimeEngineId);
        return;
      }
      var preflightRoute = null;
      var preflightMeta = '';
      if (!InfringAPI.isWsConnected() || String(this._wsAgent || '') !== targetAgentId) {
        this.connectWs(targetAgentId);
        var waitStarted = Date.now();
        while ((!InfringAPI.isWsConnected() || String(this._wsAgent || '') !== targetAgentId) && (Date.now() - waitStarted) < 1500) {
          await new Promise(function(resolve) { setTimeout(resolve, 75); });
        }
      }
      var wsPayload = { type: 'message', content: finalText, agent_runtime_engine_id: runtimeEngineId };
      if (uploadedFiles && uploadedFiles.length) wsPayload.attachments = uploadedFiles;
      if (InfringAPI.wsSend(wsPayload)) {
        this._setPendingWsRequest(targetAgentId, finalText);
        this._responseStartedAt = Date.now();
        this.messages.push({
          id: ++msgId,
          role: 'agent',
          text: '',
          meta: preflightMeta || '',
          thinking: true,
          streaming: true,
          tools: [],
          ts: Date.now()
        });
        this.scrollToBottom();
        this.scheduleConversationPersist();
        return;
      }
      this._clearPendingWsRequest(targetAgentId);
      if (!InfringAPI.isWsConnected()) {
        InfringToast.info('Using HTTP mode (no streaming)');
      }
      this.messages.push({
        id: ++msgId,
        role: 'agent',
        text: '',
        meta: preflightMeta || '',
        thinking: true,
        tools: [],
        ts: Date.now()
      });
      this.scrollToBottom();
      this.scheduleConversationPersist();
      var httpStartedAt = Date.now();
      var handedOffToRecovery = false;

      try {
        var httpBody = { message: finalText, agent_runtime_engine_id: runtimeEngineId };
        if (uploadedFiles && uploadedFiles.length) httpBody.attachments = uploadedFiles;
        var httpAutoSwitchPrevious = String(this._pendingAutoModelSwitchBaseline || '').trim();
        if (!httpAutoSwitchPrevious) httpAutoSwitchPrevious = this.captureAutoModelSwitchBaseline();
        var res = await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(targetAgentId) + '/message', httpBody);
        this.applyContextTelemetry(res);
        var httpRoute = this.applyAutoRouteTelemetry(res);
        typeof this.clearTransientThinkingRows === 'function' ? this.clearTransientThinkingRows({ force: true }) : (this.messages = this.messages.filter(function(m) { return !m.thinking; }));
        var httpMeta = (res.input_tokens || 0) + ' in / ' + (res.output_tokens || 0) + ' out';
        if (res.cost_usd != null) httpMeta += ' | $' + res.cost_usd.toFixed(4);
        if (res.iterations) httpMeta += ' | ' + res.iterations + ' iter';
        var httpDurationMs = Math.max(0, Date.now() - httpStartedAt);
        var httpDuration = this.formatResponseDuration(httpDurationMs);
        if (httpDuration) httpMeta += ' | ' + httpDuration;
        var httpRouteMeta = this.formatAutoRouteMeta(httpRoute || preflightRoute);
        if (httpRouteMeta) httpMeta += ' | ' + httpRouteMeta;
        var httpTools = typeof this.responseToolRowsFromPayload === 'function'
          ? this.responseToolRowsFromPayload(res, 'http-tool')
          : [];
        var httpHasToolCompletion = typeof this.responseHasAuthoritativeToolCompletion === 'function'
          ? this.responseHasAuthoritativeToolCompletion(res, httpTools)
          : httpTools.length > 0;
        var httpMessageMetadata = typeof this.assistantTurnMetadataFromPayload === 'function' ? this.assistantTurnMetadataFromPayload(res, httpTools) : {};
        var httpPayloadText = typeof this.assistantTextFromPayload === 'function'
          ? this.assistantTextFromPayload(res)
          : String(res.response || '');
        var httpText = this.stripModelPrefix(this.sanitizeToolText(httpPayloadText || ''));
        var httpArtifactDirectives = this.extractArtifactDirectives(httpText);
        var httpSplit = this.extractThinkingLeak(httpText);
        if (httpSplit.thought) {
          httpTools.unshift(this.makeThoughtToolCard(httpSplit.thought, httpDurationMs));
          httpText = httpSplit.content || '';
        }
        httpText = this.stripArtifactDirectivesFromText(httpText);
        var httpCompact = String(httpText || '').replace(/\s+/g, ' ').trim();
        if (
          typeof this.isThinkingPlaceholderText === 'function' &&
          this.isThinkingPlaceholderText(httpCompact)
        ) {
          httpText = '';
        }
        var httpToolFailureSummary = httpMessageMetadata && typeof httpMessageMetadata.tool_failure_summary === 'string' ? String(httpMessageMetadata.tool_failure_summary || '').trim() : '';
        var httpToolSummary = httpHasToolCompletion && typeof this.completedToolOnlySummary === 'function'
          ? String(this.completedToolOnlySummary(httpTools) || '').trim()
          : '';
        var httpWorkflowFallbackSummary = typeof this.fallbackAssistantTextFromPayload === 'function'
          ? String(this.fallbackAssistantTextFromPayload(res, httpTools) || '').trim()
          : '';
        var httpReplaceableFinalText =
          !!httpCompact &&
          (
            (typeof this.textLooksNoFindingsPlaceholder === 'function' && this.textLooksNoFindingsPlaceholder(httpCompact)) ||
            (typeof this.textLooksToolAckWithoutFindings === 'function' && this.textLooksToolAckWithoutFindings(httpCompact))
          );
        if (httpReplaceableFinalText && httpWorkflowFallbackSummary && httpWorkflowFallbackSummary !== httpCompact) {
          httpText = httpWorkflowFallbackSummary;
          httpCompact = String(httpText || '').replace(/\s+/g, ' ').trim();
        }
        if (!String(httpText || '').trim()) {
          // Policy: do not inject system-authored fallback text into chat.
          this.maybeAddAutoModelSwitchNotice(httpAutoSwitchPrevious, httpRoute || preflightRoute);
          this._pendingAutoModelSwitchBaseline = '';
          this._clearPendingWsRequest(targetAgentId);
          this._inflightPayload = null;
          this.sending = false;
          this._responseStartedAt = 0;
          this.tokenCount = 0;
          this._clearTypingTimeout();
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
          this.scheduleConversationPersist();
          return;
        }
        var httpFailure = httpHasToolCompletion ? null : this.extractRecoverableBackendFailure(httpText);
        if (httpFailure) {
          this._clearPendingWsRequest(targetAgentId);
          this._pendingAutoModelSwitchBaseline = '';
          this.sending = false;
          this._responseStartedAt = 0;
          this.tokenCount = 0;
          this._clearTypingTimeout();
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
          handedOffToRecovery = await this.attemptAutomaticFailoverRecovery('http_response', httpText, {
            remove_last_agent_failure: false
          });
          if (handedOffToRecovery) {
            this.scheduleConversationPersist();
            return;
          }
        }
        var httpMessage = Object.assign({
          id: ++msgId,
          role: 'agent',
          text: httpText,
          meta: httpMeta,
          tools: httpTools,
          ts: Date.now(),
          agent_id: res && res.agent_id ? String(res.agent_id) : (this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : ''),
          agent_name: res && res.agent_name ? String(res.agent_name) : (this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '')
        }, httpMessageMetadata || {});
        var pushedHttpMessage = this.pushAgentMessageDeduped(httpMessage, { dedupe_window_ms: 90000 }) || httpMessage;
        this.markAgentMessageComplete(pushedHttpMessage);
        if (pushedHttpMessage && typeof this._queueFinalWordTypingRender === 'function') {
          this._queueFinalWordTypingRender(pushedHttpMessage, String(pushedHttpMessage.text || ''), 10);
        }
        this.maybeAddAutoModelSwitchNotice(httpAutoSwitchPrevious, httpRoute || preflightRoute);
        this._pendingAutoModelSwitchBaseline = '';
        this._clearPendingWsRequest(targetAgentId);
        this._inflightPayload = null;
        if (httpArtifactDirectives && httpArtifactDirectives.length) {
          this.resolveArtifactDirectives(httpArtifactDirectives);
        }
        this.scheduleConversationPersist();
      } catch(e) {
        typeof this.clearTransientThinkingRows === 'function' ? this.clearTransientThinkingRows({ force: true }) : (this.messages = this.messages.filter(function(m) { return !m.thinking; }));
        this._clearPendingWsRequest(targetAgentId);
        this._pendingAutoModelSwitchBaseline = '';
        this.sending = false;
        this._responseStartedAt = 0;
        this.tokenCount = 0;
        this._clearTypingTimeout();
        this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
        var rawHttpError = String(e && e.message ? e.message : e || '');
        var lowerHttpError = rawHttpError.toLowerCase();
        var isAbortError =
          (e && String(e.name || '').toLowerCase() === 'aborterror') ||
          lowerHttpError.indexOf('this operation was aborted') >= 0 ||
          lowerHttpError.indexOf('operation was aborted') >= 0;
        if (isAbortError) {
