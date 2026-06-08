
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
        proposal_arguments: request.proposal_arguments && typeof request.proposal_arguments === 'object' ? request.proposal_arguments : null,
        capability: String(request.capability || ''),
        reason: String(request.reason || ''),
        gatekeeper_kind: 'user'
      }).then(function(payload) {
        self.pendingAgentRuntimePermissionRequests = (Array.isArray(self.pendingAgentRuntimePermissionRequests) ? self.pendingAgentRuntimePermissionRequests : [])
          .filter(function(item) { return String(item && item.approval_id || '') !== approvalId; });
        if (choice === 'deny') {
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
          if (payload && payload.execution_result && payload.execution_result.ok) {
            if (typeof self.addNoticeEvent === 'function') {
              self.addNoticeEvent({
                notice_label: 'Approved ' + self.permissionRequestPreview(request) + '; executed approved action.',
                notice_type: 'info',
                ts: Date.now()
              });
            }
            self.appendAgentRuntimeApprovalExecutionMessage(request, payload.execution_result);
            return payload;
          }
          var resume = request.resume_payload && typeof request.resume_payload === 'object' ? request.resume_payload : null;
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
                decision_receipt_ref: String(payload && payload.decision_receipt_ref || '').trim()
              });
              }, 0);
            }
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
        var kind = String(event.activity_kind || event.kind || '').trim();
        var text = String(event.display_text || event.text || event.summary || '').replace(/\r\n/g, '\n').trim();
        var providerType = String(event.provider_event_type || event.event_type || '').trim();
        if (!text && !providerType) continue;
        if (kind === 'assistant_delta' && text.length > 900) continue;
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
          activity_text: text || providerType,
          activity_status: String(event.status || 'completed').trim() || 'completed',
          agent_activity_event: true,
          agent_runtime_live_activity: true,
          agent_runtime_activity_dialog_text: text || providerType,
          agent_runtime_activity_latest: false,
          notice_type: kind,
          running: false,
          ts: Date.now()
        });
      }
      return out;
    },

    agentRuntimeActivityEventsToDecisionDialog: function(events) {
      var rows = Array.isArray(events) ? events : [];
      var lines = [];
      var seen = Object.create(null);
      for (var i = 0; i < rows.length && lines.length < 160; i += 1) {
        var event = rows[i] && typeof rows[i] === 'object' ? rows[i] : {};
        var text = String(event.display_text || event.text || event.summary || '').replace(/\r\n/g, '\n').trim();
        var providerType = String(event.provider_event_type || event.event_type || '').trim();
        if (!text && providerType) text = this.agentRuntimeProviderEventLabel(providerType);
        if (!text) continue;
        var status = String(event.status || '').trim();
        var line = text.split('\n').map(function(part) {
          return String(part || '').replace(/\s+/g, ' ').trim();
        }).filter(Boolean).join('\n');
        if (!line) continue;
        var lowerLine = line.toLowerCase();
        if (
          lowerLine.indexOf('final answer is shown in the message') >= 0 ||
          lowerLine.indexOf('assistant draft streamed') >= 0 ||
          lowerLine.indexOf('returned completed') >= 0
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

    agentRuntimeActivityEventsToDecisionTool: function(events, engineId, durationMs, extraDialog) {
      var dialog = this.agentRuntimeActivityEventsToDecisionDialog(events);
      var extra = String(extraDialog || '').trim();
      if (extra && dialog.indexOf(extra) < 0) dialog = dialog ? (dialog + '\n\n' + extra) : extra;
      if (!dialog) return null;
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
      target.tools = target.tools.concat(tools).slice(-40);
      var latestTool = tools[tools.length - 1] || {};
      var latestText = String(latestTool.display_text || latestTool.summary || latestTool.name || '').trim();
      target.text = latestText || 'Working through runtime activity...';
      target.thinking_status = latestText || 'Working through runtime activity...';
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

    async _sendAgentRuntimeSocketPayload(targetAgentId, finalText, uploadedFiles, msgImages, runtimeEngineId, resumeOptions) {
      var engineId = String(runtimeEngineId || this.selectedAgentRuntimeEngineId || '').trim();
      if (!engineId) return;
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
        var approvalResume = resumeOptions && typeof resumeOptions === 'object' ? resumeOptions : null;
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
        var turnRequest = {
          engine_id: engineId,
          agent_id: targetAgentId,
          session_id: String((this.currentAgent && (this.currentAgent.session_id || this.currentAgent.id)) || targetAgentId || ''),
          message: String(finalText || ''),
          cwd: typeof this.activeWorkspacePath === 'function' ? this.activeWorkspacePath() : '',
          active_workspace: typeof this.activeWorkspaceTurnContextProjection === 'function' ? this.activeWorkspaceTurnContextProjection() : null,
      model_provider_context: {
        source_authority: 'shell_selected_model_projection',
        provider: String((this.currentAgent && (this.currentAgent.model_provider || this.currentAgent.provider || this.currentAgent.selected_provider)) || '').slice(0, 120),
        model: String((this.currentAgent && (this.currentAgent.model_name || this.currentAgent.runtime_model || this.currentAgent.selected_model || this.currentAgent.model)) || '').slice(0, 240),
        runtime_model: String((this.currentAgent && (this.currentAgent.runtime_model || this.currentAgent.model_name)) || '').slice(0, 240),
        selected_runtime_engine_id: engineId,
        secrets_included: false
      },
          attachments: Array.isArray(uploadedFiles) ? uploadedFiles.slice(0, 12) : [],
          permission_policy: this.agentRuntimePermissionPolicyProjection(),
          approval_resume: approvalResume ? {
            approval_id: String(approvalResume.approval_id || '').slice(0, 260),
            resume_token: String(approvalResume.resume_token || '').slice(0, 260),
            approved_tool_id: String(approvalResume.approved_tool_id || '').slice(0, 120),
            approval_decision: String(approvalResume.approval_decision || '').slice(0, 80),
            approval_resume_action: String(approvalResume.approval_resume_action || '').slice(0, 160),
            decision_receipt_ref: String(approvalResume.decision_receipt_ref || '').slice(0, 240),
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
        typeof this.clearTransientThinkingRows === 'function'
          ? this.clearTransientThinkingRows({ force: true })
          : (this.messages = this.messages.filter(function(m) { return !m.thinking; }));
        if (res && res.pending_permission_request) {
          var pendingPermissionRequest = res.pending_permission_request;
          pendingPermissionRequest.projection_kind = 'permission_request';
          pendingPermissionRequest.projection_schema_version = 1;
          var pendingRuntimeDurationMs = Math.max(0, Date.now() - startedAt);
          var pendingRuntimeDuration = this.formatResponseDuration(pendingRuntimeDurationMs);
          var pendingResponseActivityEvents = Array.isArray(res && res.agent_activity_events) ? res.agent_activity_events : [];
          var pendingFinalActivityEvents = pendingResponseActivityEvents.length ? pendingResponseActivityEvents : streamedActivityEvents;
          var pendingDialogExtra = typeof this.agentRuntimePermissionRequestToDecisionDialog === 'function'
            ? this.agentRuntimePermissionRequestToDecisionDialog(pendingPermissionRequest)
            : '';
          var pendingDecisionTool = this.agentRuntimeActivityEventsToDecisionTool(
            pendingFinalActivityEvents,
            engineId,
            pendingRuntimeDurationMs,
            pendingDialogExtra
          );
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
            paused_reason: 'waiting_for_permission_decision'
          };
          pendingPermissionRequest.status = 'paused_pending_approval';
          this.enqueueAgentRuntimePermissionRequest(pendingPermissionRequest);
          var pendingRuntimeMessage = {
            id: ++msgId,
            role: 'agent',
            text: 'Waiting for approval: ' + pendingToolId,
            meta: pendingMeta,
            tools: pendingDecisionTool ? [pendingDecisionTool] : [],
            agent_activity_events: pendingFinalActivityEvents.slice(-80),
            agent_activity_event_count: Number(res && res.activity_event_count) || pendingFinalActivityEvents.length,
            ts: Date.now(),
            result_ref: String(res && res.result_ref || '').slice(0, 240),
            receipt_ref: String(res && res.receipt_ref || '').slice(0, 240),
            agent_id: targetAgentId,
            agent_name: this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '',
            isHtml: false,
            _typingVisual: false,
            agent_runtime_engine_id: engineId,
            pending_permission_request: pendingPermissionRequest,
            projection_kind: 'permission_request',
            projection_schema_version: 1
          };
          var pushedPendingRuntimeMessage = this.pushAgentMessageDeduped(pendingRuntimeMessage, { dedupe_window_ms: 90000 }) || pendingRuntimeMessage;
          this.markAgentMessageComplete(pushedPendingRuntimeMessage);
          this._clearPendingWsRequest(targetAgentId);
          this._inflightPayload = null;
          this.sending = false;
          this._responseStartedAt = 0;
          this.tokenCount = 0;
          this._clearTypingTimeout();
          this.setAgentLiveActivity(targetAgentId, 'idle', { optimistic: true, source: 'agent_runtime_permission_wait' });
          this.scheduleConversationPersist();
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
        var responseActivityEvents = Array.isArray(res && res.agent_activity_events) ? res.agent_activity_events : [];
        var finalActivityEvents = responseActivityEvents.length ? responseActivityEvents : streamedActivityEvents;
        var runtimeDecisionTool = this.agentRuntimeActivityEventsToDecisionTool(finalActivityEvents, engineId, runtimeDurationMs);
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
      var useRuntimeSocketPath = typeof this.shouldUseAgentRuntimeSocketPath === 'function'
        ? this.shouldUseAgentRuntimeSocketPath(runtimeEngineId)
        : (typeof this.isExternalAgentRuntimeEngineSelected === 'function' && this.isExternalAgentRuntimeEngineSelected(runtimeEngineId));
      if (useRuntimeSocketPath) {
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
