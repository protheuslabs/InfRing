
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
      var sendTrigger = this._pendingPromptSuggestionSend && typeof this._pendingPromptSuggestionSend === 'object'
        ? this._pendingPromptSuggestionSend
        : null;
      var sendTriggerSource = sendTrigger ? 'prompt_suggestion' : 'composer';
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
        var largePasteAttachment = this.buildLargePasteTextAttachment
          ? this.buildLargePasteTextAttachment(rawInput)
          : (this.buildLargePasteMarkdownAttachment && this.buildLargePasteMarkdownAttachment(rawInput));
        if (largePasteAttachment && largePasteAttachment.file) {
          if (!Array.isArray(this.attachments)) this.attachments = [];
          this.attachments.push(largePasteAttachment);
          text = '';
          condensedLargePaste = true;
        }
      }
      if (text || condensedLargePaste) this.pushInputHistoryEntry('chat', text || '[File: pastedtext.txt]');
      if (condensedLargePaste) InfringToast.info('Large paste moved to pastedtext.txt');
      if (text.startsWith('/') && !this.attachments.length) {
        var cmd = text.split(' ')[0].toLowerCase();
        var cmdArgs = text.substring(cmd.length).trim();
        var aliasResolution = this.resolveSlashAlias(cmd, cmdArgs);
        var routedCmd = String(aliasResolution && aliasResolution.cmd ? aliasResolution.cmd : cmd).toLowerCase();
        var routedArgs = String(aliasResolution && typeof aliasResolution.args === 'string' ? aliasResolution.args : cmdArgs).trim();
        if (typeof this.fetchAgentRuntimeSlashCommands === 'function') {
          await this.fetchAgentRuntimeSlashCommands(false);
        }
        var matched = typeof this.findSlashCommandDefinition === 'function'
          ? this.findSlashCommandDefinition(routedCmd)
          : this.slashCommands.find(function(c) { return c.cmd === routedCmd; });
        if (matched) {
          this.executeSlashCommand(matched.cmd, routedArgs, matched);
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
            var uploadedFileRow = {
              file_id: uploadRes.file_id,
              filename: uploadRes.filename,
              content_type: uploadRes.content_type
            };
            if (att && att.pasted_text) {
              uploadedFileRow.source_kind = 'pasted_text_attachment';
              uploadedFileRow.size_bytes = Number(att.pasted_text_size || (att.file && att.file.size) || 0) || 0;
              uploadedFileRow.content_preview = String(att.pasted_text_preview || '').slice(0, 12000);
              uploadedFileRow.prompt_instruction = 'Read this pasted text attachment as supplemental user-provided context; do not require the user to paste it again.';
            }
            uploadedFiles.push(uploadedFileRow);
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
        var steerQueueId = this.nextPromptQueueId();
        this.messageQueue.push({
          queue_id: steerQueueId,
          queue_kind: 'prompt',
          queued_at: Date.now(),
          text: finalText,
          files: uploadedFiles,
          images: msgImages,
          agent_runtime_engine_id: selectedRuntimeEngineId,
          trigger_source: sendTriggerSource,
          runtime_steer_direct: false,
          silent_steer_notice: false
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
      if (usesExternalRuntime) {
        if (typeof this.syncActiveChatMessages === 'function') this.syncActiveChatMessages();
        if (typeof this.scheduleMessageRenderWindowUpdate === 'function') this.scheduleMessageRenderWindowUpdate();
      }
      this._sendPayload(finalText, uploadedFiles, msgImages, {
        agent_id: activeAgent.id,
        agent_runtime_engine_id: selectedRuntimeEngineId,
        trigger_source: sendTriggerSource
      });
    },

    isExternalAgentRuntimeEngineSelected: function(engineId) {
      var id = String(engineId || this.selectedAgentRuntimeEngineId || 'infring_native').trim() || 'infring_native';
      return !!id && id !== 'infring_native';
    },

    shouldUseAgentRuntimeSocketPath: function(engineId) {
      var id = String(engineId || this.selectedAgentRuntimeEngineId || 'infring_native').trim() || 'infring_native';
      return !!id;
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

    appendAgentRuntimeApprovalExecutionMessage: function(request, result) {
      var row = result && typeof result === 'object' ? result : {};
      if (!row.ok) return false;
      var engineId = String((request && request.engine_id) || this.selectedAgentRuntimeEngineId || '').trim();
      var text = String(row.display_text || '').trim();
      if (!text) text = row.path ? 'Created ' + String(row.path) + '.' : 'Approved action completed.';
      var message = {
        id: ++msgId,
        role: 'agent',
        text: text,
        meta: 'runtime ' + (engineId || 'agent_runtime') + ' | approval executed',
        tools: [],
        ts: Date.now(),
        result_ref: String(row.result_ref || '').slice(0, 240),
        receipt_ref: String(row.receipt_ref || '').slice(0, 240),
        agent_id: String((request && request.agent_id) || (this.currentAgent && (this.currentAgent.id || this.currentAgent.session_id)) || '').slice(0, 160),
        agent_name: this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '',
        isHtml: false,
        _typingVisual: false,
        agent_runtime_engine_id: engineId
      };
      var pushed = this.pushAgentMessageDeduped(message, { dedupe_window_ms: 90000 }) || message;
      this.markAgentMessageComplete(pushed);
      this.scheduleConversationPersist();
      return true;
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
      var alreadyPending = pending.some(function(item) { return String(item && item.approval_id || '') === id; });
      pending = pending.filter(function(item) { return String(item && item.approval_id || '') !== id; });
      pending.unshift(row);
      this.pendingAgentRuntimePermissionRequests = pending.slice(0, 5);
      if (!alreadyPending) InfringToast.info('Permission requested: ' + this.permissionRequestPreview(row));
    },

    syncAgentRuntimePendingApprovals: function(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var now = Date.now();
      if (!opts.force && this._agentRuntimePendingApprovalSyncAt && (now - this._agentRuntimePendingApprovalSyncAt) < 4000) {
        return Promise.resolve(this.pendingAgentRuntimePermissionRequests || []);
      }
      this._agentRuntimePendingApprovalSyncAt = now;
      var self = this;
      return InfringAPI.get('/api/shell-socket/approvals/pending').then(function(payload) {
        var rows = payload && Array.isArray(payload.pending_requests) ? payload.pending_requests : [];
        rows.slice(0, 5).forEach(function(row) {
          if (row && row.approval_id) self.enqueueAgentRuntimePermissionRequest(row);
        });
        return rows;
      }).catch(function() {
        return self.pendingAgentRuntimePermissionRequests || [];
      });
    },

    ensureAgentRuntimePendingApprovalSyncLoop: function() {
      if (this._agentRuntimePendingApprovalSyncLoop) return;
      var self = this;
      this._agentRuntimePendingApprovalSyncLoop = window.setInterval(function() {
        if (typeof self.syncAgentRuntimePendingApprovals === 'function') {
          self.syncAgentRuntimePendingApprovals({ force: false }).catch(function() {});
        }
      }, 5000);
      this.syncAgentRuntimePendingApprovals({ force: true }).catch(function() {});
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
        proposal_arguments: request.proposal_arguments && typeof request.proposal_arguments === 'object' ? request.proposal_arguments : null,
        capability: String(request.capability || ''),
        reason: String(request.reason || ''),
        gatekeeper_kind: 'user'
      }).then(function(payload) {
        self.pendingAgentRuntimePermissionRequests = (Array.isArray(self.pendingAgentRuntimePermissionRequests) ? self.pendingAgentRuntimePermissionRequests : [])
          .filter(function(item) { return String(item && item.approval_id || '') !== approvalId; });
        var liveRow = typeof self.findAgentRuntimeLiveThinkingRow === 'function'
          ? self.findAgentRuntimeLiveThinkingRow(request.engine_id, request.live_message_id)
          : null;
        var liveEngineId = String(request.engine_id || self.selectedAgentRuntimeEngineId || '').trim();
        if (choice === 'deny') {
          var denyText = 'Permission denied: ' + self.permissionRequestPreview(request);
          if (liveRow && typeof self.appendAgentRuntimeActivityToThinkingRow === 'function') {
            self.appendAgentRuntimeActivityToThinkingRow({
              activity_kind: 'permission_event',
              provider_event_type: 'permission.denied',
              status: 'blocked',
              display_text: denyText,
              text: denyText,
              engine_id: liveEngineId,
              item_id: approvalId
            }, liveEngineId);
            var denyEvents = Array.isArray(liveRow.agent_runtime_live_events) ? liveRow.agent_runtime_live_events.slice(-80) : [];
            var denyDurationMs = Math.max(0, Date.now() - Number(liveRow._stream_started_at || liveRow.ts || Date.now()));
            var denyTool = self.agentRuntimeActivityEventsToDecisionTool(denyEvents, liveEngineId, denyDurationMs, denyText);
            self.finalizeAgentRuntimeThinkingRow(liveRow, {
              text: denyText,
              meta: 'runtime ' + (liveEngineId || 'agent_runtime') + ' | permission denied',
              tools: denyTool ? [denyTool] : [],
              agent_activity_events: denyEvents,
              agent_activity_event_count: denyEvents.length,
              pending_permission_request: request,
              projection_kind: 'permission_request'
            });
          }
          if (typeof self.addNoticeEvent === 'function') {
            self.addNoticeEvent({
              notice_label: 'Denied ' + self.permissionRequestPreview(request),
              notice_type: 'warning',
              ts: Date.now()
            });
          }
          InfringToast.info('Permission denied for ' + self.permissionRequestPreview(request));
        }
        else {
          InfringToast.success(choice === 'always_allow_tool_call' ? 'Always allowed: ' + String(request.tool_id || 'tool call') : 'Permission allowed once');
          var resume = request.resume_payload && typeof request.resume_payload === 'object' ? request.resume_payload : null;
          var executionResult = payload && payload.execution_result && payload.execution_result.ok
            ? payload.execution_result
            : null;
          if (executionResult && typeof self.addNoticeEvent === 'function') {
            self.addNoticeEvent({
              notice_label: 'Approved ' + self.permissionRequestPreview(request) + '; applied approved effect.',
              notice_type: 'info',
              ts: Date.now()
            });
          }
          if (resume && typeof self._sendAgentRuntimeSocketPayload === 'function') {
            var resumeAgentId = String(resume.agent_id || request.agent_id || (self.currentAgent && (self.currentAgent.id || self.currentAgent.session_id)) || '').trim();
            var resumeText = String(resume.final_text || resume.message || resume.text || '').trim();
            var resumeEngineId = String(resume.agent_runtime_engine_id || request.engine_id || self.selectedAgentRuntimeEngineId || '').trim();
            var resumeFiles = Array.isArray(resume.uploaded_files) ? resume.uploaded_files.slice(0, 12) : [];
            var resumeImages = Array.isArray(resume.msg_images) ? resume.msg_images.slice(0, 12) : [];
            if (resumeAgentId && resumeText && resumeEngineId) {
              if (typeof self.addNoticeEvent === 'function') {
                self.addNoticeEvent({
                  notice_label: 'Approved ' + self.permissionRequestPreview(request) + '; resuming ' + resumeEngineId + '.',
                  notice_type: 'info',
                  ts: Date.now()
                });
              }
              self.sending = true;
              if (typeof self.setAgentLiveActivity === 'function') {
                self.setAgentLiveActivity(resumeAgentId, 'working', { optimistic: true, source: 'agent_runtime_permission_resume' });
              }
              setTimeout(function() {
                self._sendAgentRuntimeSocketPayload(resumeAgentId, resumeText, resumeFiles, resumeImages, resumeEngineId, {
                  approval_id: approvalId,
                  resume_token: String(payload && payload.resume_token || request.resume_token || '').trim(),
                  approved_tool_id: String(request.tool_id || '').trim(),
                  approval_decision: choice,
                  approval_resume_action: String(payload && payload.resume_action || '').trim(),
                  decision_receipt_ref: String(payload && payload.decision_receipt_ref || '').trim(),
                  approved_effect_executed: !!executionResult,
                  approved_effect_path: String(executionResult && executionResult.path || '').trim(),
                  approved_effect_artifact_ref: String(executionResult && executionResult.artifact_ref || '').trim(),
                  approved_effect_result_ref: String(executionResult && executionResult.result_ref || '').trim(),
                  approved_effect_receipt_ref: String(executionResult && executionResult.receipt_ref || '').trim(),
                  approved_effect_display_text: String(executionResult && executionResult.display_text || '').trim(),
                  live_message_id: String(request.live_message_id || '').trim()
                });
              }, 0);
              return payload;
            }
          }
          if (executionResult) {
            if (liveRow && typeof self.appendAgentRuntimeActivityToThinkingRow === 'function') {
              var executedText = String(executionResult.display_text || executionResult.path || 'Approved action completed.').trim();
              if (executionResult.path && executedText === String(executionResult.path).trim()) executedText = 'Created ' + executedText + '.';
              var execEvents = Array.isArray(liveRow.agent_runtime_live_events) ? liveRow.agent_runtime_live_events.slice(-80) : [];
              self.finalizeAgentRuntimeThinkingRow(liveRow, {
                text: executedText,
                meta: 'runtime ' + (liveEngineId || 'agent_runtime') + ' | approval executed',
                tools: [],
                agent_activity_events: execEvents,
                agent_activity_event_count: execEvents.length,
                result_ref: String(executionResult.result_ref || ''),
                receipt_ref: String(executionResult.receipt_ref || ''),
                pending_permission_request: request,
                projection_kind: 'permission_request'
              });
              return payload;
            }
            if (typeof self.addNoticeEvent === 'function') {
              self.addNoticeEvent({
                notice_label: 'Approved ' + self.permissionRequestPreview(request) + '; executed approved action.',
                notice_type: 'info',
                ts: Date.now()
              });
            }
            self.appendAgentRuntimeApprovalExecutionMessage(request, executionResult);
            return payload;
          }
        }
        return payload;
      }).catch(function(e) {
        InfringToast.error(e && e.message ? String(e.message) : 'permission decision failed');
        return null;
      });
    },

    agentRuntimeActivityKindLabel: function(kind) {
      var value = String(kind || '').trim();
      if (value === 'decision_dialog') return 'Decision dialog';
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

    agentRuntimeProviderEventLabel: function(providerType) {
      var value = String(providerType || '').trim();
      if (!value) return '';
      var lower = value.toLowerCase();
      if (lower.indexOf('tool') >= 0 && lower.indexOf('call') >= 0) return 'Tool call';
      if (lower.indexOf('tool') >= 0 && (lower.indexOf('result') >= 0 || lower.indexOf('output') >= 0)) return 'Tool result';
      if (lower.indexOf('command') >= 0 || lower.indexOf('exec') >= 0 || lower.indexOf('shell') >= 0) return 'Command';
      if (lower.indexOf('permission') >= 0 || lower.indexOf('approval') >= 0) return 'Permission';
      if (lower.indexOf('assistant') >= 0 || lower.indexOf('message') >= 0 || lower.indexOf('response') >= 0 || lower.indexOf('delta') >= 0) return 'Assistant draft';
      if (lower.indexOf('error') >= 0 || lower.indexOf('fail') >= 0) return 'Runtime error';
      if (lower.indexOf('start') >= 0) return 'Started';
      if (lower.indexOf('complete') >= 0 || lower.indexOf('finish') >= 0 || lower.indexOf('done') >= 0) return 'Completed';
      return value.replace(/[._:/-]+/g, ' ').replace(/\s+/g, ' ').trim();
    },

    agentRuntimeActivityEventsToTools: function(events, engineId) {
      var rows = Array.isArray(events) ? events : [];
      var out = [];
      for (var i = 0; i < rows.length && out.length < 40; i += 1) {
        var event = rows[i] && typeof rows[i] === 'object' ? rows[i] : {};
        if (typeof this.agentRuntimeActivityVisibleInThinkingBubble === 'function' && !this.agentRuntimeActivityVisibleInThinkingBubble(event)) continue;
        var kind = String(event.activity_kind || event.kind || '').trim();
        var text = String(event.display_text || event.text || event.summary || '').replace(/\r\n/g, '\n').trim();
        var providerType = String(event.provider_event_type || event.event_type || '').trim();
        if (!text && !providerType) continue;
        if (kind === 'assistant_delta' && text.length > 900) continue;
        var lineKind = this.agentRuntimeActivityLineKind(event);
        var providerLabel = this.agentRuntimeProviderEventLabel(providerType);
        var label = (!kind || kind === 'activity') && providerLabel
          ? providerLabel
          : this.agentRuntimeActivityKindLabel(kind);
        var visibleName = text
          ? text.split('\n')[0].replace(/\s+/g, ' ').trim().slice(0, 140)
          : label;
        out.push({
          name: visibleName || label,
          status: String(event.status || 'completed').trim() || 'completed',
          input: providerType ? ((label && label !== visibleName ? label + ' | ' : '') + 'event: ' + providerType) : '',
          result: text || providerType,
          output: text || providerType,
          summary: text || providerType,
          display_text: text || providerType,
          input_preview: providerType ? ('event: ' + providerType) : '',
          result_preview: text || providerType,
          receipt_ref: String(event.receipt_ref || '').slice(0, 240),
          result_ref: String(event.result_ref || '').slice(0, 240),
          tool_call_ref: String(event.item_id || event.sequence_no || ('activity-' + i)).slice(0, 160),
          agent_runtime_engine_id: String(event.engine_id || engineId || '').trim(),
          projection_kind: 'runtime_activity',
          projection_schema_version: 1,
          activity_line_kind: lineKind,
          activity_text: text || providerType,
          activity_status: String(event.status || 'completed').trim() || 'completed',
          agent_activity_event: true,
          agent_runtime_live_activity: true,
          agent_runtime_dialog_line: lineKind === 'dialog',
          agent_runtime_activity_dialog_text: text || providerType,
          agent_runtime_activity_latest: false,
          notice_type: kind,
          running: false,
          ts: Date.now()
        });
      }
      return out;
    },

    agentRuntimeActivityLineKind: function(event) {
      var row = event && typeof event === 'object' ? event : {};
      var kind = String(row.activity_kind || row.kind || row.type || '').toLowerCase();
      var providerType = String(row.provider_event_type || row.event_type || '').toLowerCase();
      var timelineRole = String(row.timeline_role || row.role || '').toLowerCase();
      if (
        kind === 'user_steer' ||
        providerType === 'steering.user_message' ||
        timelineRole === 'user_steer'
      ) {
        return 'dialog';
      }
      if (
        kind === 'permission_event' ||
        kind === 'permission_request' ||
        providerType.indexOf('permission.') === 0 ||
        providerType.indexOf('approval.') === 0
      ) {
        return 'status';
      }
      var text = String(row.display_text || row.text || row.summary || '').toLowerCase();
      var joined = kind + ' ' + providerType + ' ' + text;
      if (
        joined.indexOf('decision') >= 0 ||
        joined.indexOf('reasoning') >= 0 ||
        joined.indexOf('thought') >= 0 ||
        joined.indexOf('plan') >= 0 ||
        joined.indexOf('assistant_delta') >= 0
      ) {
        return 'dialog';
      }
      if (
        joined.indexOf('file') >= 0 ||
        joined.indexOf('patch') >= 0 ||
        joined.indexOf('write') >= 0 ||
        joined.indexOf('edit') >= 0 ||
        joined.indexOf('diff') >= 0 ||
        /\b(apply_patch|tee|touch|mkdir|rm|mv|cp)\b/.test(joined) ||
        />\s*[^&|;]+/.test(joined)
      ) {
        return 'write';
      }
      if (
        joined.indexOf('search') >= 0 ||
        joined.indexOf('grep') >= 0 ||
        joined.indexOf('find') >= 0 ||
        joined.indexOf('read') >= 0 ||
        joined.indexOf('open') >= 0 ||
        /\b(rg|grep|sed|cat|ls|find|pwd|head|tail|wc)\b/.test(joined)
      ) {
        return 'read';
      }
      if (
        joined.indexOf('command') >= 0 ||
        joined.indexOf('exec') >= 0 ||
        joined.indexOf('shell') >= 0 ||
        joined.indexOf('tool') >= 0 ||
        joined.indexOf('permission') >= 0 ||
        joined.indexOf('approval') >= 0 ||
        joined.indexOf('error') >= 0
      ) {
        return 'tool';
      }
      return 'status';
    },

    agentRuntimeActivityTraceTarget: function(row, text) {
      var event = row && typeof row === 'object' ? row : {};
      var rawText = String(text || event.display_text || event.text || event.summary || '').replace(/\r\n/g, '\n').trim();
      var itemIdTarget = String(event.target || event.path || event.file_path || event.filename || event.tool_name || event.name || '').trim();
      if (itemIdTarget) return itemIdTarget.slice(0, 900);
      var patterns = [
        /^(?:running|ran|failed running)\s+([\s\S]+)$/i,
        /^(?:writing|wrote|failed writing)\s+([\s\S]+)$/i,
        /^(?:searching|searched|failed searching)\s+([\s\S]+)$/i,
        /^(?:reading|read|failed reading)\s+([\s\S]+)$/i,
        /^Runtime event:\s*([\s\S]+)$/i
      ];
      for (var i = 0; i < patterns.length; i += 1) {
        var match = rawText.match(patterns[i]);
        if (match && match[1]) return String(match[1]).replace(/\s+/g, ' ').trim().slice(0, 900);
      }
      return rawText.split('\n')[0].replace(/\s+/g, ' ').trim().slice(0, 900);
    },

    agentRuntimeActivityTraceKey: function(row, lineKind, target) {
      var event = row && typeof row === 'object' ? row : {};
      var itemId = String(event.item_id || event.itemId || event.tool_call_ref || '').trim();
      if (itemId) return 'item|' + itemId.slice(0, 180);
      var normalizedTarget = String(target || '').replace(/\s+/g, ' ').trim().toLowerCase();
      if (normalizedTarget) return [lineKind || 'activity', normalizedTarget].join('|').slice(0, 240);
      return '';
    },

    agentRuntimeActivityTraceText: function(row, lineKind, state, target) {
      var event = row && typeof row === 'object' ? row : {};
      var cleanTarget = String(target || '').replace(/\s+/g, ' ').trim();
      var done = state === 'done';
      var error = state === 'error';
      if (lineKind === 'write') {
        return (error ? 'failed writing ' : done ? 'wrote ' : 'writing ') + (cleanTarget || 'file');
      }
      if (lineKind === 'read') {
        return (error ? 'failed reading ' : done ? 'read ' : 'reading ') + (cleanTarget || 'workspace');
      }
      if (lineKind === 'tool') {
        return (error ? 'failed running ' : done ? 'ran ' : 'running ') + (cleanTarget || 'tool call');
      }
      return String(event.display_text || event.text || event.summary || cleanTarget || '').replace(/\r\n/g, '\n').trim();
    },

    agentRuntimeActivityEventToTraceLine: function(event, engineId) {
      var row = event && typeof event === 'object' ? event : {};
      var text = String(row.display_text || row.text || row.summary || '').replace(/\r\n/g, '\n').trim();
      var providerType = String(row.provider_event_type || row.event_type || '').trim();
      if (!text && providerType) text = this.agentRuntimeProviderEventLabel(providerType);
      if (!text) return null;
      var status = String(row.status || '').trim();
      var kind = this.agentRuntimeActivityLineKind(row);
      var state = status === 'failed' || status === 'error'
        ? 'error'
        : status === 'paused_pending_approval'
          ? 'blocked'
          : status === 'running' || status === 'started' || status === 'activity'
            ? 'running'
            : 'done';
      var target = this.agentRuntimeActivityTraceTarget(row, text);
      var displayText = this.agentRuntimeActivityTraceText(row, kind, state, target);
      var normalizedDisplayText = String(displayText || '').replace(/\s+/g, ' ').trim().toLowerCase();
      if (
        kind === 'status' &&
        (
          normalizedDisplayText.indexOf('final answer is shown in the message') >= 0 ||
          normalizedDisplayText.indexOf('assistant draft streamed') >= 0 ||
          normalizedDisplayText.indexOf('returned completed') >= 0 ||
          normalizedDisplayText.indexOf('completed the turn') >= 0 ||
          normalizedDisplayText.indexOf('finished its turn') >= 0
        )
      ) {
        return null;
      }
      var activityKey = this.agentRuntimeActivityTraceKey(row, kind, target);
      return {
        id: String(activityKey || row.item_id || row.sequence_no || (kind + '-' + Date.now() + '-' + Math.random())).slice(0, 180),
        text: String(displayText || text).split('\n').map(function(part) {
          return String(part || '').replace(/\s+/g, ' ').trim();
        }).filter(Boolean).join('\n').slice(0, 1400),
        line_kind: kind,
        state: state,
        status: status,
        activity_key: activityKey,
        activity_target: target,
        engine_id: String(row.engine_id || engineId || '').trim(),
        ts: Date.now()
      };
    },

    appendActivityTraceLineToMessage: function(target, traceLine) {
      if (!target || !traceLine || !traceLine.text) return false;
      var traceRows = Array.isArray(target.agent_runtime_live_trace_rows) ? target.agent_runtime_live_trace_rows.slice(-47) : [];
      var lastTrace = traceRows.length ? traceRows[traceRows.length - 1] : null;
      var nextText = String(traceLine.text || '').trim();
      var nextKind = String(traceLine.line_kind || 'status');
      var nextKey = String(traceLine.activity_key || '').trim();
      var nextTarget = String(traceLine.activity_target || '').replace(/\s+/g, ' ').trim().toLowerCase();
      if ((nextKey || nextTarget) && nextKind !== 'dialog') {
        for (var i = traceRows.length - 1; i >= 0; i -= 1) {
          var rowKey = String(traceRows[i].activity_key || '').trim();
          var rowKind = String(traceRows[i].line_kind || 'status');
          var rowTarget = String(traceRows[i].activity_target || '').replace(/\s+/g, ' ').trim().toLowerCase();
          if (
            rowKey === nextKey ||
            (nextTarget && rowTarget === nextTarget && rowKind === nextKind)
          ) {
            traceRows[i] = Object.assign({}, traceRows[i], traceLine, { id: traceRows[i].id || traceLine.id });
            target.agent_runtime_live_trace_rows = traceRows.slice(-48);
            if (typeof this.scheduleThinkingBubbleSizeStabilization === 'function') this.scheduleThinkingBubbleSizeStabilization(target);
            return true;
          }
        }
      }
      if (
        lastTrace &&
        String(lastTrace.line_kind || '') === nextKind &&
        nextKind === 'dialog' &&
        (
          nextText.indexOf(String(lastTrace.text || '').trim()) === 0 ||
          String(lastTrace.text || '').trim().indexOf(nextText) === 0
        )
      ) {
        if (nextText.length >= String(lastTrace.text || '').length) lastTrace.text = nextText;
        lastTrace.state = traceLine.state || lastTrace.state;
        lastTrace.status = traceLine.status || lastTrace.status;
        lastTrace.ts = traceLine.ts;
      } else if (
        lastTrace &&
        String(lastTrace.text || '').trim() === nextText &&
        String(lastTrace.line_kind || '') === nextKind
      ) {
        lastTrace.state = traceLine.state || lastTrace.state;
        lastTrace.status = traceLine.status || lastTrace.status;
        lastTrace.ts = traceLine.ts;
      } else {
        traceRows.push(traceLine);
      }
      target.agent_runtime_live_trace_rows = traceRows.slice(-48);
      if (typeof this.scheduleThinkingBubbleSizeStabilization === 'function') this.scheduleThinkingBubbleSizeStabilization(target);
      return true;
    },

    appendNativeWorkflowActivityToThinkingRow: function(target, event) {
      if (!target || typeof target !== 'object') return false;
      var row = event && typeof event === 'object' ? event : {};
      var engineId = String(row.engine_id || target.agent_runtime_engine_id || 'infring_native').trim() || 'infring_native';
      var normalized = Object.assign({}, row, {
        type: 'agent_activity_event',
        source: String(row.source || 'infring_native_workflow_projection'),
        engine_id: engineId,
        sequence_no: Number(row.sequence_no || 0) || ((Array.isArray(target.agent_runtime_live_events) ? target.agent_runtime_live_events.length : 0) + 1)
      });
      var traceLine = this.agentRuntimeActivityEventToTraceLine(normalized, engineId);
      if (traceLine) this.appendActivityTraceLineToMessage(target, traceLine);
      var liveEvents = Array.isArray(target.agent_runtime_live_events) ? target.agent_runtime_live_events.slice(-79) : [];
      liveEvents.push(normalized);
      target.agent_runtime_live_events = liveEvents;
      var liveDialog = this.agentRuntimeActivityEventsToDecisionDialog(liveEvents);
      if (liveDialog) target.agent_runtime_decision_dialog_text = liveDialog;
      target.agent_runtime_engine_id = engineId;
      target._stream_updated_at = Date.now();
      return true;
    },

    nativeWorkflowDecisionToolFromMessage: function(msg, durationMs, extraDialog) {
      var row = msg && typeof msg === 'object' ? msg : {};
      var events = Array.isArray(row.agent_runtime_live_events) ? row.agent_runtime_live_events.slice(-80) : [];
      var extra = String(extraDialog || row._thoughtText || row._reasoning || '').trim();
      if (!events.length && !extra) return null;
      if (typeof this.agentRuntimeActivityEventsToDecisionTool === 'function') {
        return this.agentRuntimeActivityEventsToDecisionTool(events, row.agent_runtime_engine_id || 'infring_native', durationMs, extra);
      }
      return extra && typeof this.makeThoughtToolCard === 'function' ? this.makeThoughtToolCard(extra, durationMs) : null;
    },

    agentRuntimeActivityEventsToDecisionDialog: function(events) {
      var rows = Array.isArray(events) ? events : [];
      var lines = [];
      var seen = Object.create(null);
      for (var i = 0; i < rows.length && lines.length < 160; i += 1) {
        var event = rows[i] && typeof rows[i] === 'object' ? rows[i] : {};
        if (typeof this.agentRuntimeActivityVisibleInThinkingBubble === 'function' && !this.agentRuntimeActivityVisibleInThinkingBubble(event)) continue;
        var text = String(event.display_text || event.text || event.summary || '').replace(/\r\n/g, '\n').trim();
        var providerType = String(event.provider_event_type || event.event_type || '').trim();
        if (!text && providerType) text = this.agentRuntimeProviderEventLabel(providerType);
        if (!text) continue;
        var status = String(event.status || '').trim();
        var line = text.split('\n').map(function(part) {
          return String(part || '').replace(/\s+/g, ' ').trim();
        }).filter(Boolean).join('\n');
        if (!line) continue;
        var lineKind = typeof this.agentRuntimeActivityLineKind === 'function'
          ? String(this.agentRuntimeActivityLineKind(event) || '').trim()
          : '';
        if (lineKind && lineKind !== 'dialog' && lineKind !== 'status') continue;
        var lowerLine = line.toLowerCase();
        if (
          /^(?:working on|completed|failed)\s+(?:command|file change|search|tool):/i.test(line) ||
          lowerLine.indexOf('final answer is shown in the message') >= 0 ||
          lowerLine.indexOf('assistant draft streamed') >= 0 ||
          lowerLine.indexOf('returned completed') >= 0 ||
          lowerLine.indexOf('completed the turn') >= 0 ||
          lowerLine.indexOf('finished its turn') >= 0
        ) {
          continue;
        }
        if (status && status !== 'completed' && line.toLowerCase().indexOf(status.toLowerCase()) < 0) {
          line += ' [' + status + ']';
        }
        var key = line.toLowerCase();
        if (seen[key]) continue;
        seen[key] = true;
        lines.push(line);
      }
      return lines.join('\n');
    },

    agentRuntimeActivityEventsToTraceRows: function(events, engineId) {
      var rows = Array.isArray(events) ? events : [];
      var target = { agent_runtime_live_trace_rows: [] };
      for (var i = 0; i < rows.length; i += 1) {
        var event = rows[i] && typeof rows[i] === 'object' ? rows[i] : {};
        if (typeof this.agentRuntimeActivityVisibleInThinkingBubble === 'function' && !this.agentRuntimeActivityVisibleInThinkingBubble(event)) continue;
        var traceLine = this.agentRuntimeActivityEventToTraceLine(event, engineId);
        if (traceLine && traceLine.text) this.appendActivityTraceLineToMessage(target, traceLine);
      }
      return Array.isArray(target.agent_runtime_live_trace_rows) ? target.agent_runtime_live_trace_rows.slice(-48) : [];
    },

    agentRuntimePermissionRequestToDecisionDialog: function(request) {
      var row = request && typeof request === 'object' ? request : {};
      var lines = [];
      var toolId = String(row.tool_id || '').trim();
      var capability = String(row.capability || '').trim();
      var reason = String(row.reason || '').trim();
      if (toolId) lines.push('Permission requested: ' + toolId);
      if (capability) lines.push('Capability: ' + capability);
      if (reason) lines.push('Reason: ' + reason);
      var args = row.proposal_arguments && typeof row.proposal_arguments === 'object'
        ? row.proposal_arguments
        : null;
      if (args) {
        try {
          lines.push('Proposal arguments:');
          lines.push('```json');
          lines.push(JSON.stringify(args, null, 2));
          lines.push('```');
        } catch (_) {}
      }
      return lines.join('\n');
    },

    agentRuntimeActivityVisibleInThinkingBubble: function(event) {
      var row = event && typeof event === 'object' ? event : {};
      if (row.display_in_thinking_bubble === false || row.thinking_bubble_visible === false) return false;
      var providerType = String(row.provider_event_type || row.event_type || row.type || '').toLowerCase();
      var kind = String(row.activity_kind || row.kind || row.type || '').toLowerCase();
      var text = String(row.display_text || row.text || row.summary || '').toLowerCase();
      var joined = providerType + ' ' + kind + ' ' + text;
      if (/decision|reasoning|thought|plan|permission|approval|error|failed/.test(joined)) return true;
      if (/command|exec|shell|bash|tool|mcp|function|file|edit|patch|diff|write|search|grep|find/.test(joined)) return true;
      if (
        providerType === 'external_cli.launch' ||
        providerType.indexOf('context.') >= 0 ||
        providerType.indexOf('availability') >= 0 ||
        providerType.indexOf('health') >= 0 ||
        providerType.indexOf('session.') >= 0 ||
        providerType.indexOf('prepare') >= 0 ||
        providerType.indexOf('launch') >= 0 ||
        providerType.indexOf('thread.started') >= 0 ||
        providerType.indexOf('turn.started') >= 0 ||
        providerType.indexOf('turn.completed') >= 0
      ) {
        return false;
      }
      if (
        /^preparing\b/.test(text) ||
        /^loaded \d+ prior context row/.test(text) ||
        /^checking .* availability/.test(text) ||
        /^starting .* session/.test(text) ||
        /^launching .* turn with bounded context pack/.test(text) ||
        /^launching .* cli\b/.test(text) ||
        /^runtime thread started/.test(text) ||
        /^runtime turn started/.test(text) ||
        /^runtime completed the turn/.test(text)
      ) {
        return false;
      }
      return true;
    },

    agentRuntimeActivityEventsToDecisionTool: function(events, engineId, durationMs, extraDialog) {
      var dialog = this.agentRuntimeActivityEventsToDecisionDialog(events);
      var traceRows = this.agentRuntimeActivityEventsToTraceRows(events, engineId);
      var extra = String(extraDialog || '').trim();
      if (extra && dialog.indexOf(extra) < 0) dialog = dialog ? (dialog + '\n\n' + extra) : extra;
      if (!dialog && !traceRows.length) return null;
      var tool = typeof this.makeThoughtToolCard === 'function'
        ? this.makeThoughtToolCard(dialog, durationMs)
        : {
            id: 'agent-decision-' + Date.now(),
            name: 'thought_process',
            running: false,
            expanded: false,
            input: dialog,
            result: '',
            is_error: false,
            duration_ms: Math.max(0, Number(durationMs || 0))
          };
      tool.agent_decision_dialog = true;
      tool.agent_runtime_decision_dialog = true;
      tool.agent_runtime_activity_trace = true;
      tool.projection_kind = 'decision_dialog';
      tool.projection_schema_version = 1;
      tool.agent_decision_dialog_text = dialog;
      tool.agent_runtime_trace_rows = traceRows;
      tool.agent_runtime_engine_id = String(engineId || '').trim();
      tool.status = 'completed';
      tool.input = dialog;
      tool.input_preview = dialog;
      tool.result_preview = dialog;
      tool.summary = 'Agent decision dialog';
      tool.display_text = dialog;
      tool.expanded = false;
      return tool;
    },

    createAgentRuntimeSteerThinkingRow: function(engineId, agentId, steerText) {
      var runtimeId = String(engineId || this.selectedAgentRuntimeEngineId || 'infring_native').trim() || 'infring_native';
      var text = String(steerText || '').trim();
      var now = Date.now();
      var steerEvent = {
        activity_kind: 'user_steer',
        provider_event_type: 'steering.user_message',
        timeline_role: 'user_steer',
        status: 'completed',
        display_text: text ? ('User steer: ' + text) : 'User steer received.',
        text: text ? ('User steer: ' + text) : 'User steer received.',
        engine_id: runtimeId,
        item_id: 'steer-' + now
      };
      var traceLine = typeof this.agentRuntimeActivityEventToTraceLine === 'function'
        ? this.agentRuntimeActivityEventToTraceLine(steerEvent, runtimeId)
        : null;
      var row = {
        id: ++msgId,
        role: 'agent',
        text: 'thinking',
        thinking_status: 'thinking',
        meta: 'Agent runtime: ' + runtimeId + ' | steering',
        thinking: true,
        streaming: true,
        tools: [],
        ts: now,
        _stream_started_at: now,
        _stream_updated_at: now,
        agent_id: String(agentId || '').trim(),
        agent_name: this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '',
        agent_runtime_engine_id: runtimeId,
        agent_runtime_steer_pending: true,
        agent_runtime_steer_text: text,
        agent_runtime_live_events: [steerEvent],
        agent_runtime_live_trace_rows: traceLine ? [traceLine] : []
      };
      this.messages.push(row);
      this.scrollToBottom();
      this.scheduleConversationPersist();
      return row;
    },

    agentRuntimeAggregateLiveThinkingEvents: function(engineId, preferredRow) {
      var runtimeId = String(engineId || '').trim();
      var rows = Array.isArray(this.messages) ? this.messages : [];
      var out = [];
      var seen = Object.create(null);
      for (var i = 0; i < rows.length; i += 1) {
        var row = rows[i];
        if (!row || typeof row !== 'object') continue;
        if (runtimeId && String(row.agent_runtime_engine_id || '') !== runtimeId) continue;
        if (!row.thinking && row !== preferredRow) continue;
        var events = Array.isArray(row.agent_runtime_live_events) && row.agent_runtime_live_events.length
          ? row.agent_runtime_live_events
          : (Array.isArray(row.agent_activity_events) ? row.agent_activity_events : []);
        for (var j = 0; j < events.length; j += 1) {
          var event = events[j] && typeof events[j] === 'object' ? events[j] : null;
          if (!event) continue;
          var key = [
            event.item_id || event.sequence_no || '',
            event.provider_event_type || event.event_type || '',
            event.activity_kind || event.kind || '',
            event.status || '',
            event.display_text || event.text || event.summary || ''
          ].join('|');
          if (seen[key]) continue;
          seen[key] = true;
          out.push(event);
        }
      }
      return out.slice(-80);
    },

    removeAgentRuntimePeerThinkingRows: function(target, engineId) {
      var runtimeId = String(engineId || (target && target.agent_runtime_engine_id) || '').trim();
      var targetId = String(target && target.id || '').trim();
      if (!runtimeId || !targetId || !Array.isArray(this.messages)) return;
      this.messages = this.messages.filter(function(row) {
        if (!row || String(row.id || '') === targetId) return true;
        if (!row.thinking) return true;
        return String(row.agent_runtime_engine_id || '') !== runtimeId;
      });
    },

    appendAgentRuntimeActivityToThinkingRow: function(event, engineId) {
      if (typeof this.agentRuntimeActivityVisibleInThinkingBubble === 'function' && !this.agentRuntimeActivityVisibleInThinkingBubble(event)) return;
      var rows = Array.isArray(this.messages) ? this.messages : [];
      var target = null;
      var eventRow = event && typeof event === 'object' ? event : {};
      for (var i = rows.length - 1; i >= 0; i -= 1) {
        var row = rows[i];
        if (!row || !row.thinking) continue;
        if (String(row.agent_runtime_engine_id || '') !== String(engineId || '')) continue;
        target = row;
        break;
      }
      if (!target) return;
      var tools = this.agentRuntimeActivityEventsToTools([event], engineId);
      var traceLine = this.agentRuntimeActivityEventToTraceLine(event, engineId);
      if (!tools.length && !traceLine) return;
      if (!Array.isArray(target.tools)) target.tools = [];
      target.tools = target.tools.map(function(tool) {
        if (tool && (tool.agent_runtime_live_activity || tool.agent_activity_event)) {
          return Object.assign({}, tool, {
            running: false,
            agent_runtime_activity_latest: false,
            status: tool.status === 'running' ? 'completed' : (tool.status || 'completed')
          });
        }
        return tool;
      });
      if (traceLine && traceLine.text) this.appendActivityTraceLineToMessage(target, traceLine);
      var liveEvents = Array.isArray(target.agent_runtime_live_events) ? target.agent_runtime_live_events.slice(-79) : [];
      liveEvents.push(event);
      target.agent_runtime_live_events = liveEvents;
      var liveDialog = this.agentRuntimeActivityEventsToDecisionDialog(liveEvents);
      if (liveDialog) target.agent_runtime_decision_dialog_text = liveDialog;
      tools = tools.map(function(tool, index) {
        var isLatest = index === tools.length - 1;
        return Object.assign({}, tool, {
          running: isLatest,
          status: isLatest ? 'running' : (tool.status || 'completed'),
          agent_runtime_activity_latest: isLatest
        });
      });
      if (tools.length) target.tools = target.tools.concat(tools).slice(-40);
      var latestTool = tools[tools.length - 1] || {};
      var latestText = String(
        (traceLine && traceLine.line_kind !== 'dialog' && traceLine.text) ||
        latestTool.display_text ||
        latestTool.summary ||
        latestTool.name ||
        ''
      ).trim();
      target.text = latestText || 'Working through runtime activity...';
      target.thinking_status = latestText || 'Working through runtime activity...';
      target.meta = 'Agent runtime: ' + String(engineId || 'runtime') + ' | streaming activity';
      target._stream_updated_at = Date.now();
      if (typeof this.syncActiveChatMessages === 'function') this.syncActiveChatMessages();
      if (typeof this.scheduleMessageRenderWindowUpdate === 'function') this.scheduleMessageRenderWindowUpdate();
      this.scrollToBottom();
    },

    findAgentRuntimeLiveThinkingRow: function(engineId, messageId) {
      var rows = Array.isArray(this.messages) ? this.messages : [];
      var targetId = String(messageId || '').trim();
      var runtimeId = String(engineId || '').trim();
      for (var i = rows.length - 1; i >= 0; i -= 1) {
        var row = rows[i];
        if (!row || !row.thinking) continue;
        if (targetId && String(row.id || '') === targetId) return row;
        if (!targetId && (!runtimeId || String(row.agent_runtime_engine_id || '') === runtimeId)) return row;
      }
      return null;
    },

    finalizeAgentRuntimeThinkingRow: function(row, payload) {
      var target = row && typeof row === 'object' ? row : null;
      var data = payload && typeof payload === 'object' ? payload : {};
      if (!target) return false;
      target.thinking = false;
      target.streaming = false;
      target.text = String(data.text || '').trim();
      target.meta = String(data.meta || '').trim();
      target.tools = Array.isArray(data.tools) ? data.tools : [];
      target.agent_activity_events = Array.isArray(data.agent_activity_events) ? data.agent_activity_events : [];
      target.agent_activity_event_count = Number(data.agent_activity_event_count || 0) || target.agent_activity_events.length;
      target.result_ref = String(data.result_ref || '').slice(0, 240);
      target.receipt_ref = String(data.receipt_ref || '').slice(0, 240);
      target.agent_runtime_steer_pending = false;
      target.isHtml = false;
      target._typingVisual = false;
      target.pending_permission_request = data.pending_permission_request || null;
      if (data.projection_kind) target.projection_kind = data.projection_kind;
      target.projection_schema_version = data.projection_schema_version || 1;
      target._stream_updated_at = Date.now();
      this.markAgentMessageComplete(target);
      this.removeAgentRuntimePeerThinkingRows(target, target.agent_runtime_engine_id);
      if (typeof this.syncActiveChatMessages === 'function') this.syncActiveChatMessages();
      if (typeof this.scheduleMessageRenderWindowUpdate === 'function') this.scheduleMessageRenderWindowUpdate();
      this.scheduleConversationPersist();
      return true;
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

    async _sendAgentRuntimeSocketPayload(targetAgentId, finalText, uploadedFiles, msgImages, runtimeEngineId, resumeOptions) {
      var engineId = String(runtimeEngineId || this.selectedAgentRuntimeEngineId || '').trim();
      if (!engineId) return;
      var startedAt = Date.now();
      this._responseStartedAt = startedAt;
      var opts = resumeOptions && typeof resumeOptions === 'object' ? resumeOptions : {};
      var approvalResume = (opts.approval_id || opts.resume_token || opts.approval_decision)
        ? opts
        : null;
      var thinkingMessage = this.findAgentRuntimeLiveThinkingRow
        ? this.findAgentRuntimeLiveThinkingRow(engineId, approvalResume && approvalResume.live_message_id)
        : null;
      if (thinkingMessage) {
        thinkingMessage.thinking = true;
        thinkingMessage.streaming = true;
        thinkingMessage.meta = 'Agent runtime: ' + engineId + ' | resuming';
        thinkingMessage.agent_runtime_engine_id = engineId;
        thinkingMessage.agent_id = targetAgentId;
        thinkingMessage.agent_name = this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '';
        thinkingMessage._stream_updated_at = Date.now();
        if (!Number.isFinite(Number(thinkingMessage._stream_started_at))) thinkingMessage._stream_started_at = startedAt;
      } else {
        thinkingMessage = {
          id: ++msgId,
          role: 'agent',
          text: '',
          meta: 'Agent runtime: ' + engineId,
          thinking: true,
          streaming: true,
          tools: [],
          ts: Date.now(),
          _stream_started_at: startedAt,
          _stream_updated_at: startedAt,
          agent_id: targetAgentId,
          agent_name: this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '',
          agent_runtime_engine_id: engineId
        };
        this.messages.push(thinkingMessage);
      }
      this.scrollToBottom();
      this.scheduleConversationPersist();
      var drainQueueAfterRuntimeTurn = true;

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
          var ctxSourceKind = ctxRole === 'user'
            ? 'user_message'
            : ctxRole === 'assistant'
              ? 'assistant_message'
              : (ctxMsg.tool_call_ref || ctxRole === 'tool')
                ? 'tool_receipt'
                : ctxRole === 'system'
                  ? 'system_event'
                  : 'message_event';
          contextRows.push({
            id: String(ctxMsg.id || ('message-' + ctxIdx)).slice(0, 160),
            role: ctxRole.slice(0, 40),
            text_preview: ctxText.slice(0, 2400),
            detail_ref: String(ctxMsg.detail_ref || ctxMsg.id || '').slice(0, 240),
            timestamp: ctxMsg.ts || ctxMsg.timestamp || null,
            source_kind: ctxSourceKind,
            record_type: ctxSourceKind,
            source_authority: 'shell_bounded_message_projection',
            speaker_label: String(ctxMsg.agent_name || ctxMsg.origin_display_name || ctxMsg.name || ctxRole).slice(0, 120),
            receipt_ref: String(ctxMsg.receipt_ref || '').slice(0, 240),
            result_ref: String(ctxMsg.result_ref || '').slice(0, 240)
          });
        }
        contextRows = contextRows.slice(-24);
        var selectedProviderForRuntime = String((this.currentAgent && (this.currentAgent.model_provider || this.currentAgent.provider || this.currentAgent.selected_provider)) || '').slice(0, 120);
        var selectedModelForRuntime = String((this.currentAgent && (this.currentAgent.model_name || this.currentAgent.runtime_model || this.currentAgent.selected_model || this.currentAgent.model)) || '').slice(0, 240);
        var selectedRuntimeModelForRuntime = String((this.currentAgent && (this.currentAgent.runtime_model || this.currentAgent.model_name)) || '').slice(0, 240);
        var modelProviderContext = {
          source_authority: 'shell_agent_runtime_model_projection',
          selected_runtime_engine_id: engineId,
          secrets_included: false
        };
        if (engineId === 'infring_native') {
          modelProviderContext.source_authority = 'shell_selected_model_projection';
          modelProviderContext.provider = selectedProviderForRuntime;
          modelProviderContext.model = selectedModelForRuntime;
          modelProviderContext.runtime_model = selectedRuntimeModelForRuntime;
        } else {
          var runtimeEngineRowForModel = typeof this.activeRuntimeEngineRow === 'function' ? this.activeRuntimeEngineRow() : null;
          if (!runtimeEngineRowForModel || String(runtimeEngineRowForModel.engine_id || '').trim() !== engineId) {
            var runtimeRowsForModel = Array.isArray(this.runtimeEngineRows) ? this.runtimeEngineRows : [];
            for (var rtm = 0; rtm < runtimeRowsForModel.length; rtm += 1) {
              if (String(runtimeRowsForModel[rtm] && runtimeRowsForModel[rtm].engine_id || '').trim() === engineId) {
                runtimeEngineRowForModel = runtimeRowsForModel[rtm];
                break;
              }
            }
          }
          var runtimeMenuForModel = runtimeEngineRowForModel && runtimeEngineRowForModel.available_models && typeof runtimeEngineRowForModel.available_models === 'object'
            ? runtimeEngineRowForModel.available_models
            : runtimeEngineRowForModel && runtimeEngineRowForModel.model_menu && typeof runtimeEngineRowForModel.model_menu === 'object'
            ? runtimeEngineRowForModel.model_menu
            : null;
          var runtimeModelRowsForModel = runtimeMenuForModel && Array.isArray(runtimeMenuForModel.rows)
            ? runtimeMenuForModel.rows
            : runtimeMenuForModel && Array.isArray(runtimeMenuForModel.model_rows)
            ? runtimeMenuForModel.model_rows
            : [];
          var runtimeModelCandidateForModel = String(selectedRuntimeModelForRuntime || selectedModelForRuntime || '').trim().toLowerCase();
          var matchedRuntimeModelForModel = null;
          if (runtimeMenuForModel && (runtimeMenuForModel.framework_native_models || runtimeMenuForModel.source === 'framework_native') && runtimeModelCandidateForModel) {
            for (var rmfm = 0; rmfm < runtimeModelRowsForModel.length && !matchedRuntimeModelForModel; rmfm += 1) {
              var runtimeModelRowForModel = runtimeModelRowsForModel[rmfm] || {};
              var runtimeModelIdsForModel = [
                runtimeModelRowForModel.id,
                runtimeModelRowForModel.qualified_model_ref,
                runtimeModelRowForModel.model,
                runtimeModelRowForModel.model_name,
                runtimeModelRowForModel.adapter_model_arg,
                runtimeModelRowForModel.display_name
              ];
              for (var rmid = 0; rmid < runtimeModelIdsForModel.length; rmid += 1) {
                if (String(runtimeModelIdsForModel[rmid] || '').trim().toLowerCase() === runtimeModelCandidateForModel) {
                  matchedRuntimeModelForModel = runtimeModelRowForModel;
                  break;
                }
              }
            }
          }
          if (matchedRuntimeModelForModel) {
            modelProviderContext.source_authority = 'shell_agent_runtime_framework_model_projection';
            modelProviderContext.provider = String(matchedRuntimeModelForModel.provider || selectedProviderForRuntime || '').slice(0, 120);
            modelProviderContext.model = String(matchedRuntimeModelForModel.adapter_model_arg || matchedRuntimeModelForModel.model || matchedRuntimeModelForModel.model_name || matchedRuntimeModelForModel.id || '').slice(0, 240);
            modelProviderContext.runtime_model = modelProviderContext.model;
            modelProviderContext.runtime_model_source = 'framework_native';
          } else if (runtimeMenuForModel && (runtimeMenuForModel.inherit_active_llm_when_unconfigured || runtimeMenuForModel.credential_inheritance_allowed || runtimeMenuForModel.source === 'inherited_infring')) {
            modelProviderContext.source_authority = 'shell_agent_runtime_inherited_model_projection';
            modelProviderContext.provider = selectedProviderForRuntime;
            modelProviderContext.model = selectedModelForRuntime;
            modelProviderContext.runtime_model = selectedRuntimeModelForRuntime;
            modelProviderContext.runtime_model_source = 'inherited_infring_llm';
          } else {
            modelProviderContext.runtime_model_source = 'framework_default';
            modelProviderContext.framework_default = true;
          }
        }
        var turnRequest = {
          engine_id: engineId,
          agent_id: targetAgentId,
          session_id: String((this.currentAgent && (this.currentAgent.session_id || this.currentAgent.id)) || targetAgentId || ''),
          message: String(finalText || ''),
          input_source: String(opts.trigger_source || 'composer').slice(0, 80),
          turn_trigger_source: String(opts.trigger_source || 'composer').slice(0, 80),
          cwd: typeof this.activeWorkspacePath === 'function' ? this.activeWorkspacePath() : '',
          active_workspace: typeof this.activeWorkspaceTurnContextProjection === 'function' ? this.activeWorkspaceTurnContextProjection() : null,
          model_provider_context: modelProviderContext,
          attachments: Array.isArray(uploadedFiles) ? uploadedFiles.slice(0, 12) : [],
          permission_policy: this.agentRuntimePermissionPolicyProjection(),
          approval_resume: approvalResume ? {
            approval_id: String(approvalResume.approval_id || '').slice(0, 260),
            resume_token: String(approvalResume.resume_token || '').slice(0, 260),
            approved_tool_id: String(approvalResume.approved_tool_id || '').slice(0, 120),
            approval_decision: String(approvalResume.approval_decision || '').slice(0, 80),
            approval_resume_action: String(approvalResume.approval_resume_action || '').slice(0, 160),
            decision_receipt_ref: String(approvalResume.decision_receipt_ref || '').slice(0, 240),
            approved_effect_executed: approvalResume.approved_effect_executed === true,
            approved_effect_path: String(approvalResume.approved_effect_path || '').slice(0, 600),
            approved_effect_artifact_ref: String(approvalResume.approved_effect_artifact_ref || '').slice(0, 600),
            approved_effect_result_ref: String(approvalResume.approved_effect_result_ref || '').slice(0, 600),
            approved_effect_receipt_ref: String(approvalResume.approved_effect_receipt_ref || '').slice(0, 600),
            approved_effect_display_text: String(approvalResume.approved_effect_display_text || '').slice(0, 1000),
            source_authority: 'shell_permission_resume_projection'
          } : null,
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
        var latestRuntimeThinkingMessage = this.findAgentRuntimeLiveThinkingRow
          ? this.findAgentRuntimeLiveThinkingRow(engineId)
          : null;
        if (latestRuntimeThinkingMessage) thinkingMessage = latestRuntimeThinkingMessage;
        var projectedPendingPermissionRequest = res && res.pending_permission_request
          ? res.pending_permission_request
          : (res && res.permission_request ? res.permission_request : null);
        if (projectedPendingPermissionRequest) {
          var pendingPermissionRequest = projectedPendingPermissionRequest;
          pendingPermissionRequest.projection_kind = 'permission_request';
          pendingPermissionRequest.projection_schema_version = 1;
          var pendingRuntimeDurationMs = Math.max(0, Date.now() - startedAt);
          var pendingRuntimeDuration = this.formatResponseDuration(pendingRuntimeDurationMs);
          var pendingResponseActivityEvents = Array.isArray(res && res.agent_activity_events)
            ? res.agent_activity_events
            : (Array.isArray(res && res.activity_events) ? res.activity_events : []);
          var pendingFinalActivityEvents = pendingResponseActivityEvents.length ? pendingResponseActivityEvents : streamedActivityEvents;
          var pendingLiveEvents = Array.isArray(thinkingMessage.agent_runtime_live_events) ? thinkingMessage.agent_runtime_live_events.slice(-80) : [];
          if (pendingLiveEvents.length) pendingFinalActivityEvents = pendingLiveEvents;
          var pendingAggregatedEvents = typeof this.agentRuntimeAggregateLiveThinkingEvents === 'function'
            ? this.agentRuntimeAggregateLiveThinkingEvents(engineId, thinkingMessage)
            : [];
          if (pendingAggregatedEvents.length) pendingFinalActivityEvents = pendingAggregatedEvents;
          var pendingDialogExtra = typeof this.agentRuntimePermissionRequestToDecisionDialog === 'function'
            ? this.agentRuntimePermissionRequestToDecisionDialog(pendingPermissionRequest)
            : '';
          var pendingDecisionTool = this.agentRuntimeActivityEventsToDecisionTool(
            pendingFinalActivityEvents,
            engineId,
            pendingRuntimeDurationMs,
            pendingDialogExtra
          );
          var pendingLiveTraceRows = Array.isArray(thinkingMessage.agent_runtime_live_trace_rows)
            ? thinkingMessage.agent_runtime_live_trace_rows.slice(-80)
            : [];
          if (pendingDecisionTool && pendingLiveTraceRows.length) {
            pendingDecisionTool.agent_runtime_trace_rows = typeof this.normalizeThoughtTraceRows === 'function'
              ? this.normalizeThoughtTraceRows(
                pendingLiveTraceRows.concat(Array.isArray(pendingDecisionTool.agent_runtime_trace_rows) ? pendingDecisionTool.agent_runtime_trace_rows : []),
                pendingDecisionTool.agent_decision_dialog_text || pendingDecisionTool.display_text || ''
              )
              : pendingLiveTraceRows.concat(Array.isArray(pendingDecisionTool.agent_runtime_trace_rows) ? pendingDecisionTool.agent_runtime_trace_rows : []);
          }
          var pendingToolId = String(pendingPermissionRequest.tool_id || pendingPermissionRequest.capability || 'tool call').trim();
          var pendingMeta = 'runtime ' + engineId + ' | permission required';
          if (pendingRuntimeDuration) pendingMeta += ' | ' + pendingRuntimeDuration;
          pendingPermissionRequest.resume_payload = {
            agent_id: targetAgentId,
            final_text: String(finalText || ''),
            uploaded_files: Array.isArray(uploadedFiles) ? uploadedFiles.slice(0, 12) : [],
            msg_images: Array.isArray(msgImages) ? msgImages.slice(0, 12) : [],
            agent_runtime_engine_id: engineId,
            session_id: String(turnRequest.session_id || ''),
            approval_id: String(pendingPermissionRequest.approval_id || ''),
            resume_token: String(pendingPermissionRequest.resume_token || ''),
            tool_id: String(pendingPermissionRequest.tool_id || ''),
            proposal_ref: String(pendingPermissionRequest.proposal_ref || ''),
            decision_receipt_ref: '',
            live_message_id: String(thinkingMessage && thinkingMessage.id || ''),
            paused_reason: 'waiting_for_permission_decision'
          };
          pendingPermissionRequest.live_message_id = String(thinkingMessage && thinkingMessage.id || '');
          pendingPermissionRequest.status = 'paused_pending_approval';
          this.enqueueAgentRuntimePermissionRequest(pendingPermissionRequest);
          if (typeof this.appendAgentRuntimeActivityToThinkingRow === 'function') {
            this.appendAgentRuntimeActivityToThinkingRow({
              activity_kind: 'permission_request',
              provider_event_type: 'permission.requested',
              status: 'paused_pending_approval',
              display_text: 'Waiting for approval: ' + pendingToolId,
              text: 'Waiting for approval: ' + pendingToolId,
              engine_id: engineId,
              item_id: String(pendingPermissionRequest.approval_id || '')
            }, engineId);
          }
          thinkingMessage.text = 'Waiting for approval: ' + pendingToolId;
          thinkingMessage.thinking_status = 'Waiting for approval: ' + pendingToolId;
          thinkingMessage.meta = pendingMeta;
          thinkingMessage.tools = pendingDecisionTool ? [pendingDecisionTool] : (Array.isArray(thinkingMessage.tools) ? thinkingMessage.tools : []);
          thinkingMessage.agent_activity_events = pendingFinalActivityEvents.slice(-80);
          thinkingMessage.agent_activity_event_count = Number(res && res.activity_event_count) || pendingFinalActivityEvents.length;
          thinkingMessage.result_ref = String(res && res.result_ref || '').slice(0, 240);
          thinkingMessage.receipt_ref = String(res && res.receipt_ref || '').slice(0, 240);
          thinkingMessage.agent_id = targetAgentId;
          thinkingMessage.agent_name = this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '';
          thinkingMessage.agent_runtime_engine_id = engineId;
          thinkingMessage.pending_permission_request = pendingPermissionRequest;
          thinkingMessage.approval_pause_active = true;
          thinkingMessage.projection_kind = 'permission_request';
          thinkingMessage.projection_schema_version = 1;
          thinkingMessage.thinking = true;
          thinkingMessage.streaming = true;
          thinkingMessage._stream_updated_at = Date.now();
          this._clearPendingWsRequest(targetAgentId);
          this._inflightPayload = null;
          this.sending = false;
          this._responseStartedAt = 0;
          this.tokenCount = 0;
          this._clearTypingTimeout();
          this.setAgentLiveActivity(targetAgentId, 'idle', { optimistic: true, source: 'agent_runtime_permission_wait' });
          this.scheduleConversationPersist();
          if (typeof this.syncActiveChatMessages === 'function') this.syncActiveChatMessages();
          if (typeof this.scheduleMessageRenderWindowUpdate === 'function') this.scheduleMessageRenderWindowUpdate();
          drainQueueAfterRuntimeTurn = false;
          return;
        }
        var runtimePayloadText = String((res && (res.display_text || res.output_text || res.text || res.response || res.output_preview)) || '').trim();
        var runtimeText = this.stripModelPrefix(this.sanitizeToolText(runtimePayloadText || ''));
        var runtimeDurationMs = Math.max(0, Date.now() - startedAt);
        var runtimeDuration = this.formatResponseDuration(runtimeDurationMs);
        var runtimeMeta = 'runtime ' + engineId;
        if (res && res.status) runtimeMeta += ' | ' + String(res.status);
        if (runtimeDuration) runtimeMeta += ' | ' + runtimeDuration;
        if (res && res.result_ref) runtimeMeta += ' | result';
        var responseActivityEvents = Array.isArray(res && res.agent_activity_events)
          ? res.agent_activity_events
          : (Array.isArray(res && res.activity_events) ? res.activity_events : []);
        var finalActivityEvents = responseActivityEvents.length ? responseActivityEvents : streamedActivityEvents;
        var accumulatedLiveEvents = Array.isArray(thinkingMessage.agent_runtime_live_events) ? thinkingMessage.agent_runtime_live_events.slice(-80) : [];
        if (accumulatedLiveEvents.length) finalActivityEvents = accumulatedLiveEvents;
        var aggregatedLiveEvents = typeof this.agentRuntimeAggregateLiveThinkingEvents === 'function'
          ? this.agentRuntimeAggregateLiveThinkingEvents(engineId, thinkingMessage)
          : [];
        if (aggregatedLiveEvents.length) finalActivityEvents = aggregatedLiveEvents;
        var runtimeDecisionTool = this.agentRuntimeActivityEventsToDecisionTool(finalActivityEvents, engineId, runtimeDurationMs);
        var responseTraceRows = res && res.activity_trace && Array.isArray(res.activity_trace.rows) ? res.activity_trace.rows : [];
        var finalLiveTraceRows = Array.isArray(thinkingMessage.agent_runtime_live_trace_rows)
          ? thinkingMessage.agent_runtime_live_trace_rows.slice(-80)
          : [];
        if (runtimeDecisionTool && (responseTraceRows.length || finalLiveTraceRows.length)) {
          runtimeDecisionTool.agent_runtime_trace_rows = typeof this.normalizeThoughtTraceRows === 'function'
            ? this.normalizeThoughtTraceRows(
              finalLiveTraceRows.concat(responseTraceRows).concat(Array.isArray(runtimeDecisionTool.agent_runtime_trace_rows) ? runtimeDecisionTool.agent_runtime_trace_rows : []),
              runtimeDecisionTool.agent_decision_dialog_text || runtimeDecisionTool.display_text || ''
            )
            : finalLiveTraceRows.concat(responseTraceRows).concat(Array.isArray(runtimeDecisionTool.agent_runtime_trace_rows) ? runtimeDecisionTool.agent_runtime_trace_rows : []);
        }
        var runtimeActivityTools = runtimeDecisionTool ? [runtimeDecisionTool] : [];
        if (runtimeActivityTools.length && (res && (res.receipt_ref || res.result_ref))) {
          runtimeActivityTools = runtimeActivityTools.map(function(row) {
            return Object.assign({}, row, {
              receipt_ref: String(res.receipt_ref || row.receipt_ref || '').slice(0, 240),
              result_ref: String(res.result_ref || row.result_ref || '').slice(0, 240)
            });
          });
        }
        if (!String(runtimeText || '').trim()) {
          var hadPermissionPause = !!(res && (res.pending_permission_request || res.permission_request));
          if (!hadPermissionPause) InfringToast.info('Agent runtime returned no display text.');
          var emptyText = 'Agent runtime returned no display text.';
          if (typeof this.appendAgentRuntimeActivityToThinkingRow === 'function') {
            this.appendAgentRuntimeActivityToThinkingRow({
              activity_kind: 'error',
              provider_event_type: 'turn.no_display_text',
              status: 'failed',
              display_text: emptyText,
              text: emptyText,
              engine_id: engineId,
              item_id: 'turn-no-display-text'
            }, engineId);
          }
          var emptyEvents = Array.isArray(thinkingMessage.agent_runtime_live_events) ? thinkingMessage.agent_runtime_live_events.slice(-80) : finalActivityEvents;
          var emptyTool = this.agentRuntimeActivityEventsToDecisionTool(emptyEvents, engineId, runtimeDurationMs, emptyText);
          this.finalizeAgentRuntimeThinkingRow(thinkingMessage, {
            text: emptyText,
            meta: runtimeMeta + ' | no display text',
            tools: emptyTool ? [emptyTool] : runtimeActivityTools,
            agent_activity_events: emptyEvents,
            agent_activity_event_count: emptyEvents.length,
            result_ref: String(res && res.result_ref || '').slice(0, 240),
            receipt_ref: String(res && res.receipt_ref || '').slice(0, 240)
          });
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
        if (!this.finalizeAgentRuntimeThinkingRow(thinkingMessage, {
          text: runtimeText,
          meta: runtimeMeta,
          tools: runtimeActivityTools,
          agent_activity_events: finalActivityEvents.slice(-80),
          agent_activity_event_count: Number(res && res.activity_event_count) || runtimeActivityTools.length,
          result_ref: String(res && res.result_ref || '').slice(0, 240),
          receipt_ref: String(res && res.receipt_ref || '').slice(0, 240)
        })) {
          var runtimeMessage = {
            id: ++msgId,
            role: 'agent',
            text: runtimeText,
            meta: runtimeMeta,
            tools: runtimeActivityTools,
            agent_activity_events: finalActivityEvents.slice(-80),
            agent_activity_event_count: Number(res && res.activity_event_count) || runtimeActivityTools.length,
            ts: Date.now(),
            result_ref: String(res && res.result_ref || '').slice(0, 240),
            receipt_ref: String(res && res.receipt_ref || '').slice(0, 240),
            agent_id: targetAgentId,
            agent_name: this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '',
            isHtml: false,
            _typingVisual: false,
            agent_runtime_engine_id: engineId
          };
          var pushedRuntimeMessage = this.pushAgentMessageDeduped(runtimeMessage, { dedupe_window_ms: 90000 }) || runtimeMessage;
          this.markAgentMessageComplete(pushedRuntimeMessage);
        }
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
        if (
          drainQueueAfterRuntimeTurn &&
          Array.isArray(this.messageQueue) &&
          this.messageQueue.length &&
          typeof this._processQueue === 'function'
        ) {
          var selfRuntimeQueueDrain = this;
          this.$nextTick(function() {
            if (!selfRuntimeQueueDrain.sending) selfRuntimeQueueDrain._processQueue();
          });
        }
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
          trigger_source: String(opts.trigger_source || 'composer').slice(0, 80),
          failover_attempted: !!opts.retry_from_failover,
          created_at: Date.now()
        };
      } else {
        this._inflightPayload.final_text = String(finalText || '');
        this._inflightPayload.uploaded_files = safeFiles;
        this._inflightPayload.msg_images = safeImages;
        this._inflightPayload.agent_runtime_engine_id = runtimeEngineId;
        this._inflightPayload.trigger_source = String(opts.trigger_source || this._inflightPayload.trigger_source || 'composer').slice(0, 80);
        this._inflightPayload.retry_started_at = Date.now();
      }
      this._pendingAutoModelSwitchBaseline = '';
      var useRuntimeSocketPath = typeof this.shouldUseAgentRuntimeSocketPath === 'function'
        ? this.shouldUseAgentRuntimeSocketPath(runtimeEngineId)
        : (typeof this.isExternalAgentRuntimeEngineSelected === 'function' && this.isExternalAgentRuntimeEngineSelected(runtimeEngineId));
      if (useRuntimeSocketPath) {
        await this._sendAgentRuntimeSocketPayload(targetAgentId, finalText, safeFiles, safeImages, runtimeEngineId, {
          trigger_source: String(opts.trigger_source || 'composer').slice(0, 80)
        });
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
