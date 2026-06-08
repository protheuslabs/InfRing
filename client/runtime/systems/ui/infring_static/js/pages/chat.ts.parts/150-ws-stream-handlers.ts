        case 'text_delta':
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'typing');
          var last = this.messages.length ? this.messages[this.messages.length - 1] : null;
          if (last && last.streaming) {
            if (!Number.isFinite(Number(last._stream_started_at))) last._stream_started_at = Date.now();
            if (last._toolTextDetected) break;
            var deltaText = String(data.content || '');
            last._streamRawText = String(last._streamRawText || '') + deltaText;
            last._stream_updated_at = Date.now();
            var streamingSplit = this.extractThinkingLeak(last._streamRawText);
            var visibleText = this.stripModelPrefix(streamingSplit.content || '');
            last._cleanText = visibleText;
            last._thoughtText = streamingSplit.thought || '';
            if (streamingSplit.thought && typeof this.appendNativeWorkflowActivityToThinkingRow === 'function') {
              this.appendNativeWorkflowActivityToThinkingRow(last, {
                activity_kind: 'decision_dialog',
                provider_event_type: 'native.text_delta.thought',
                status: 'running',
                display_text: streamingSplit.thought,
                text: streamingSplit.thought,
                item_id: 'native-text-delta-thought'
              });
            }
            if (streamingSplit.thought && !visibleText.trim()) {
              this._clearMessageTypewriter(last);
              last.isHtml = true;
              last.thoughtStreaming = true;
              last.text = this.renderLiveThoughtHtml(streamingSplit.thought, last);
            } else {
              if (last.isHtml) last.isHtml = false;
              last.thoughtStreaming = false;
              this._clearMessageTypewriter(last);
              last._typingVisual = false;
              last.text = visibleText;
            }
            var toolScanText = String(last._cleanText || '');
            var fcIdx = toolScanText.search(/\w+<\/function[=,>]/);
            if (fcIdx === -1) fcIdx = toolScanText.search(/<function=\w+>/);
            if (fcIdx !== -1) {
              var fcPart = toolScanText.substring(fcIdx);
              var toolMatch = fcPart.match(/^(\w+)<\/function/) || fcPart.match(/^<function=(\w+)>/);
              var trimmedVisible = toolScanText.substring(0, fcIdx).trim();
              if (streamingSplit.thought && !trimmedVisible) {
                this._clearMessageTypewriter(last);
                last.isHtml = true;
                last.thoughtStreaming = true;
                last.text = this.renderLiveThoughtHtml(streamingSplit.thought, last);
              } else {
                if (last.isHtml) last.isHtml = false;
                last.thoughtStreaming = false;
                this._clearMessageTypewriter(last);
                last.text = trimmedVisible;
              }
              last._cleanText = trimmedVisible;
              last._toolTextDetected = true;
              if (toolMatch) {
                var inputMatch = fcPart.match(/[=,>]\s*(\{[\s\S]*)/);
                var leakTool = this.ensureStreamingToolCard(last, toolMatch[1], inputMatch ? inputMatch[1].replace(/<\/function>?\s*$/, '').trim() : '', { running: true });
                var leakLabel = typeof this.toolThinkingActionLabel === 'function'
                  ? this.toolThinkingActionLabel(leakTool || { name: toolMatch[1], input: '' })
                  : String(toolMatch[1] || 'tool');
                if (leakLabel && last.thinking_status !== leakLabel) last.thinking_status = leakLabel;
                if (leakLabel && typeof this._setPendingWsStatusText === 'function') {
                  this._setPendingWsStatusText(last.agent_id || (this.currentAgent && this.currentAgent.id), leakLabel);
                }
              }
            }
            this.tokenCount = Math.round(String(last._cleanText || '').length / 4);
          } else {
            var firstChunk = this.stripModelPrefix(data.content || '');
            var firstSplit = this.extractThinkingLeak(firstChunk);
            var firstVisible = firstSplit.content || '';
            var firstMessage = {
              id: ++msgId, role: 'agent', text: '', meta: '', thinking: true, streaming: true, thinking_status: '', tools: [],
              _streamRawText: firstChunk, _cleanText: firstVisible, _thoughtText: firstSplit.thought || '',
              _stream_started_at: Date.now(), _stream_updated_at: Date.now(), thoughtStreaming: false, ts: Date.now(),
              agent_id: data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : ''),
              agent_name: data && data.agent_name ? String(data.agent_name) : (this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '')
            };
            if (firstSplit.thought && !firstVisible.trim()) {
              firstMessage.isHtml = true;
              firstMessage.thoughtStreaming = true;
              firstMessage.text = this.renderLiveThoughtHtml(firstSplit.thought, firstMessage);
            }
            this.messages.push(firstMessage);
            if (firstSplit.thought && typeof this.appendNativeWorkflowActivityToThinkingRow === 'function') {
              this.appendNativeWorkflowActivityToThinkingRow(firstMessage, {
                activity_kind: 'decision_dialog',
                provider_event_type: 'native.text_delta.thought',
                status: 'running',
                display_text: firstSplit.thought,
                text: firstSplit.thought,
                item_id: 'native-text-delta-thought'
              });
            }
            if (!firstMessage.isHtml) {
              this._clearMessageTypewriter(firstMessage);
              firstMessage._typingVisual = false;
              firstMessage.text = firstVisible;
            }
          }
          this.scrollToBottom();
          break;
        case 'tool_start':
          var toolStartAgentId = String(data && data.agent_id ? data.agent_id : (this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '')).trim(); if (toolStartAgentId) this.setAgentLiveActivity(toolStartAgentId, 'working');
          var lastMsg = this.messages.length ? this.messages[this.messages.length - 1] : null;
          if (!lastMsg || !(lastMsg.thinking || lastMsg.streaming)) {
            lastMsg = {
              id: ++msgId, role: 'agent', text: '', meta: '', thinking: true, streaming: true, thinking_status: '', tools: [],
              _stream_started_at: Date.now(), _stream_updated_at: Date.now(), ts: Date.now(),
              agent_id: data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : ''),
              agent_name: data && data.agent_name ? String(data.agent_name) : (this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '')
            };
            this.messages.push(lastMsg);
          }
          lastMsg.thinking = true;
          lastMsg.streaming = true;
          this.ensureStreamingToolCard(lastMsg, data.tool, data.input || '', { running: true, attempt_id: data.attempt_id, attempt_sequence: data.attempt_sequence });
          lastMsg._stream_updated_at = Date.now();
          if (!Number.isFinite(Number(lastMsg._stream_started_at))) lastMsg._stream_started_at = Date.now(); var receiptStartLabel = String(data && data.tool_status ? data.tool_status : '').trim();
          if (receiptStartLabel && typeof this.normalizeThinkingStatusCandidate === 'function') receiptStartLabel = this.normalizeThinkingStatusCandidate(receiptStartLabel); var startLabel = receiptStartLabel || (typeof this.toolThinkingActionLabel === 'function' ? this.toolThinkingActionLabel({ name: data.tool, input: data.input || '' }) : String(data.tool || 'tool'));
          if (startLabel && lastMsg.thinking_status !== startLabel) lastMsg.thinking_status = startLabel;
          if (startLabel && typeof this.appendNativeWorkflowActivityToThinkingRow === 'function') {
            this.appendNativeWorkflowActivityToThinkingRow(lastMsg, {
              activity_kind: 'tool_call_event',
              provider_event_type: 'native.tool_start',
              status: 'running',
              display_text: startLabel,
              text: startLabel,
              item_id: data && (data.attempt_id || data.tool) ? String(data.attempt_id || data.tool) : 'native-tool-start'
            });
          }
          if (startLabel && typeof this._setPendingWsStatusText === 'function') this._setPendingWsStatusText(toolStartAgentId, startLabel);
          this._resetTypingTimeout();
          this.scrollToBottom();
          break;
        case 'tool_end':
          var toolEndAgentId = String(data && data.agent_id ? data.agent_id : (this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '')).trim(); if (toolEndAgentId) this.setAgentLiveActivity(toolEndAgentId, 'working');
          var lastMsg2 = this.messages.length ? this.messages[this.messages.length - 1] : null;
          if (lastMsg2) {
            var endedTool = this.ensureStreamingToolCard(lastMsg2, data.tool, data.input || '', { running: false, no_create: true, attempt_id: data.attempt_id, attempt_sequence: data.attempt_sequence });
            if (endedTool) endedTool.running = false;
            var activeToolLabel = typeof this.currentToolDialogLabel === 'function' ? String(this.currentToolDialogLabel(lastMsg2) || '').trim() : '';
            if (activeToolLabel && lastMsg2.thinking_status !== activeToolLabel) {
              lastMsg2.thinking_status = activeToolLabel;
            } else if (!activeToolLabel) {
              lastMsg2.thinking_status = 'Thinking';
            }
            if (typeof this.appendNativeWorkflowActivityToThinkingRow === 'function') {
              var endedToolLabel = endedTool && typeof this.toolDisplayName === 'function'
                ? this.toolDisplayName(endedTool)
                : String(data && data.tool || 'tool');
              this.appendNativeWorkflowActivityToThinkingRow(lastMsg2, {
                activity_kind: 'tool_call_event',
                provider_event_type: 'native.tool_end',
                status: 'completed',
                display_text: 'Completed ' + endedToolLabel,
                text: 'Completed ' + endedToolLabel,
                item_id: data && (data.attempt_id || data.tool) ? String(data.attempt_id || data.tool) + ':end' : 'native-tool-end'
              });
            }
            if (typeof this._setPendingWsStatusText === 'function') {
              this._setPendingWsStatusText(toolEndAgentId, lastMsg2.thinking_status || activeToolLabel || 'Thinking');
            }
            lastMsg2._stream_updated_at = Date.now();
            if (!Number.isFinite(Number(lastMsg2._stream_started_at))) lastMsg2._stream_started_at = Date.now();
          }
          this._resetTypingTimeout();
          this.scrollToBottom();
          break;
        case 'tool_result':
          var toolResultAgentId = String(data && data.agent_id ? data.agent_id : (this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '')).trim(); if (toolResultAgentId) this.setAgentLiveActivity(toolResultAgentId, 'working');
          var lastMsg3 = this.messages.length ? this.messages[this.messages.length - 1] : null;
          if (lastMsg3) {
            var resultTool = this.ensureStreamingToolCard(lastMsg3, data.tool, data.input || '', { running: true, attempt_id: data.attempt_id, attempt_sequence: data.attempt_sequence });
            if (resultTool) {
              resultTool.running = false;
              resultTool.result = data.result || '';
              resultTool.is_error = !!data.is_error;
              if ((data.tool === 'image_generate' || data.tool === 'browser_screenshot') && !data.is_error) {
                try {
                  var parsed = JSON.parse(data.result);
                  if (parsed.image_urls && parsed.image_urls.length) resultTool._imageUrls = parsed.image_urls;
                } catch(e) {}
              }
              if (data.tool === 'text_to_speech' && !data.is_error) {
                try {
                  var ttsResult = JSON.parse(data.result);
                  if (ttsResult.saved_to) {
                    resultTool._audioFile = ttsResult.saved_to;
                    resultTool._audioDuration = ttsResult.duration_estimate_ms;
                  }
                } catch(e) {}
              }
            }
            lastMsg3._stream_updated_at = Date.now();
            if (!Number.isFinite(Number(lastMsg3._stream_started_at))) lastMsg3._stream_started_at = Date.now();
            var nextActiveToolLabel = typeof this.currentToolDialogLabel === 'function' ? String(this.currentToolDialogLabel(lastMsg3) || '').trim() : '';
            if (nextActiveToolLabel && lastMsg3.thinking_status !== nextActiveToolLabel) {
              lastMsg3.thinking_status = nextActiveToolLabel;
            } else if (!nextActiveToolLabel) {
              lastMsg3.thinking_status = 'Thinking';
            }
            if (typeof this.appendNativeWorkflowActivityToThinkingRow === 'function') {
              var resultToolLabel = resultTool && typeof this.toolDisplayName === 'function'
                ? this.toolDisplayName(resultTool)
                : String(data && data.tool || 'tool');
              this.appendNativeWorkflowActivityToThinkingRow(lastMsg3, {
                activity_kind: data && data.is_error ? 'error' : 'tool_call_event',
                provider_event_type: data && data.is_error ? 'native.tool_result.error' : 'native.tool_result',
                status: data && data.is_error ? 'failed' : 'completed',
                display_text: (data && data.is_error ? 'Failed ' : 'Completed ') + resultToolLabel,
                text: (data && data.is_error ? 'Failed ' : 'Completed ') + resultToolLabel,
                item_id: data && (data.attempt_id || data.tool) ? String(data.attempt_id || data.tool) + ':result' : 'native-tool-result'
              });
            }
            if (typeof this._setPendingWsStatusText === 'function') {
              this._setPendingWsStatusText(toolResultAgentId, lastMsg3.thinking_status || nextActiveToolLabel || 'Thinking');
            }
          }
          this._resetTypingTimeout();
          this.scrollToBottom();
          break;
        case 'response':
          var responsePendingRequest = this._pendingWsRequest && this._pendingWsRequest.agent_id
            ? this._pendingWsRequest
            : null;
          var responseAgentId = String(
            (data && data.agent_id) ||
            (responsePendingRequest && responsePendingRequest.agent_id) ||
            (this.currentAgent && this.currentAgent.id) ||
            ''
          ).trim();
          var responseTurnStartedAt = Number(
            this._responseStartedAt ||
            (responsePendingRequest && responsePendingRequest.started_at) ||
            Date.now()
          );
          if (!Number.isFinite(responseTurnStartedAt) || responseTurnStartedAt <= 0) {
            responseTurnStartedAt = Date.now();
          }
          this._clearTypingTimeout();
          this._clearStreamingTypewriters();
          this.applyContextTelemetry(data);
          var wsAutoSwitchPrevious = String(this._pendingAutoModelSwitchBaseline || '').trim(); if (!wsAutoSwitchPrevious) wsAutoSwitchPrevious = this.captureAutoModelSwitchBaseline();
          var wsRoute = this.applyAutoRouteTelemetry(data);
          var envelope = this.collectStreamedAssistantEnvelope();
          var streamedText = envelope.text;
          var streamedTools = envelope.tools;
          var streamedThought = envelope.thought;
          var responseTools = typeof this.responseToolRowsFromPayload === 'function' ? this.responseToolRowsFromPayload(data, 'ws-tool') : [];
          var responseHasToolCompletion = typeof this.responseHasAuthoritativeToolCompletion === 'function' ? this.responseHasAuthoritativeToolCompletion(data, responseTools.length ? responseTools : streamedTools) : (responseTools.length > 0 || streamedTools.length > 0);
          var hasAgentTerminalTranscript = !!(Array.isArray(data.terminal_transcript) && data.terminal_transcript.length && typeof this.appendAgentTerminalTranscript === 'function' && this.appendAgentTerminalTranscript(data.terminal_transcript));
          if (hasAgentTerminalTranscript) responseTools = responseTools.filter(function(t) { var n = String((t && t.name) || '').toLowerCase(); return !(n === 'terminal_exec' || n === 'run_terminal' || n === 'terminal' || n === 'shell_exec'); });
          if ((!Array.isArray(streamedTools) || !streamedTools.length) && responseTools.length) streamedTools = responseTools;
          var messageMetadata = typeof this.assistantTurnMetadataFromPayload === 'function' ? this.assistantTurnMetadataFromPayload(data, streamedTools) : {};
          if (!streamedThought && responseTools.length) {
            var thoughtTool = responseTools.find(function(rtool) { return !!(rtool && String(rtool.name || '').toLowerCase() === 'thought_process'); });
            if (thoughtTool) streamedThought = String(thoughtTool.input || thoughtTool.result || '').trim();
          }
          streamedTools.forEach(function(t) {
            t.running = false;
            if (t.id && t.id.indexOf('-txt-') !== -1 && !t.result) {
              t.result = 'Model attempted this call as text (not executed via tool system)';
              t.is_error = true;
            }
          });
          var meta = (data.input_tokens || 0) + ' in / ' + (data.output_tokens || 0) + ' out';
          if (data.cost_usd != null) meta += ' | $' + data.cost_usd.toFixed(4);
          if (data.iterations) meta += ' | ' + data.iterations + ' iter';
          if (data.fallback_model) meta += ' | fallback: ' + data.fallback_model;
          var wsDurationMs = Number(data.duration_ms || data.elapsed_ms || data.response_ms || 0);
          if (!wsDurationMs && this._responseStartedAt) wsDurationMs = Math.max(0, Date.now() - this._responseStartedAt);
          var wsDuration = this.formatResponseDuration(wsDurationMs); if (wsDuration) meta += ' | ' + wsDuration;
          var wsRouteMeta = this.formatAutoRouteMeta(wsRoute);
          if (wsRouteMeta) meta += ' | ' + wsRouteMeta;
          var payloadText = typeof this.assistantTextFromPayload === 'function'
            ? this.assistantTextFromPayload(data)
            : '';
          var finalText = (payloadText && payloadText.trim()) ? payloadText : streamedText;
          finalText = this.stripModelPrefix(finalText);
          var artifactDirectives = this.extractArtifactDirectives(finalText);
          var finalSplit = this.extractThinkingLeak(finalText);
          if (finalSplit.thought) {
            if (!streamedThought) streamedThought = finalSplit.thought;
            else if (streamedThought.indexOf(finalSplit.thought) === -1) streamedThought += '\n' + finalSplit.thought;
            finalText = finalSplit.content || '';
          }
          finalText = this.sanitizeToolText(finalText);
          finalText = this.stripArtifactDirectivesFromText(finalText);
          var collapsedThought = String(streamedThought || '').trim();
          var compactFinal = String(finalText || '').replace(/\s+/g, ' ').trim();
          var maybePlaceholder = /^(thinking|processing|working)\.\.\.$/i.test(compactFinal);
          if (typeof this.isThinkingPlaceholderText === 'function' && this.isThinkingPlaceholderText(compactFinal)) maybePlaceholder = true;
          if (maybePlaceholder) finalText = '';
          var nativeTraceSourceMessage = this.messages.length ? this.messages[this.messages.length - 1] : null;
          var nativeDecisionTool = typeof this.nativeWorkflowDecisionToolFromMessage === 'function'
            ? this.nativeWorkflowDecisionToolFromMessage(nativeTraceSourceMessage, wsDurationMs, collapsedThought)
            : null;
          if (
            nativeDecisionTool &&
            !streamedTools.some(function(tool) { return !!(tool && (tool.agent_runtime_activity_trace || tool.agent_runtime_decision_dialog)); })
          ) {
            streamedTools.unshift(nativeDecisionTool);
          } else if (collapsedThought && !streamedTools.some(function(tool) { return !!(tool && String(tool.name || '').toLowerCase() === 'thought_process'); })) {
            streamedTools.unshift(this.makeThoughtToolCard(collapsedThought, wsDurationMs));
          }
          var nativeActivityEvents = nativeTraceSourceMessage && Array.isArray(nativeTraceSourceMessage.agent_runtime_live_events)
            ? nativeTraceSourceMessage.agent_runtime_live_events.slice(-80)
            : [];
          var usedFallback = false;
          var toolFailureSummary = messageMetadata && typeof messageMetadata.tool_failure_summary === 'string' ? String(messageMetadata.tool_failure_summary || '').trim() : '';
          var toolOnlySummary = responseHasToolCompletion && typeof this.completedToolOnlySummary === 'function'
            ? String(this.completedToolOnlySummary(streamedTools) || '').trim()
            : '';
          var workflowFallbackSummary = typeof this.fallbackAssistantTextFromPayload === 'function'
            ? String(this.fallbackAssistantTextFromPayload(data, streamedTools) || '').trim()
            : '';
          var replaceableFinalText =
            !!compactFinal &&
            (
              (typeof this.textLooksNoFindingsPlaceholder === 'function' && this.textLooksNoFindingsPlaceholder(compactFinal)) ||
              (typeof this.textLooksToolAckWithoutFindings === 'function' && this.textLooksToolAckWithoutFindings(compactFinal))
            );
          if (replaceableFinalText && workflowFallbackSummary && workflowFallbackSummary !== compactFinal) {
            finalText = workflowFallbackSummary;
            compactFinal = String(finalText || '').replace(/\s+/g, ' ').trim();
            usedFallback = true;
          }
          if (!finalText.trim()) {
            // Policy: do not inject system-authored fallback text into chat.
            usedFallback = false;
          }
          var finalMessage = Object.assign({
            id: ++msgId,
            role: 'agent',
            text: finalText,
            meta: meta,
            tools: streamedTools,
            agent_activity_events: nativeActivityEvents,
            agent_activity_event_count: nativeActivityEvents.length,
            agent_runtime_engine_id: nativeActivityEvents.length ? 'infring_native' : '',
            ts: Date.now(),
            _turn_started_at: responseTurnStartedAt,
            _auto_fallback: usedFallback,
            agent_id: data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : ''),
            agent_name: data && data.agent_name ? String(data.agent_name) : (this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '')
          }, messageMetadata || {});
          var renderedFinalMessage = finalMessage;
          var lastStable = this.messages.length ? this.messages[this.messages.length - 1] : null;
          if (!usedFallback && lastStable && lastStable.role === 'agent' && lastStable._auto_fallback) {
            this.messages[this.messages.length - 1] = finalMessage;
            renderedFinalMessage = finalMessage;
          } else {
            renderedFinalMessage = this.pushAgentMessageDeduped(finalMessage, { dedupe_window_ms: 90000 }) || finalMessage;
          }
          typeof this.clearTransientThinkingRows === 'function' ? this.clearTransientThinkingRows({ force: true }) : (this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; }));
          this.markAgentMessageComplete(renderedFinalMessage);
          if (renderedFinalMessage && typeof this._queueFinalWordTypingRender === 'function') {
            this._queueFinalWordTypingRender(renderedFinalMessage, String(renderedFinalMessage.text || ''), 10);
          }
          var wsFailure = responseHasToolCompletion ? null : this.extractRecoverableBackendFailure(finalText);
          if (responseAgentId) this._clearPendingWsRequest(responseAgentId);
          else this._clearPendingWsRequest();
          this.setAgentLiveActivity(responseAgentId || (this.currentAgent && this.currentAgent.id), 'idle');
          this.sending = false;
          this._responseStartedAt = 0;
          this.tokenCount = 0;
          this.scrollToBottom();
          this.requestContextTelemetry(false);
          this.maybeAddAutoModelSwitchNotice(wsAutoSwitchPrevious, wsRoute);
          this._pendingAutoModelSwitchBaseline = '';
          if (artifactDirectives && artifactDirectives.length) {
            this.resolveArtifactDirectives(artifactDirectives);
          }
          var self3 = this;
          if (wsFailure) {
            this.attemptAutomaticFailoverRecovery('ws_response', finalText, {
              remove_last_agent_failure: true
            }).then(function(recovered) {
              if (recovered) return;
              self3._inflightPayload = null;
              self3.refreshPromptSuggestions(true, 'post-response-failed-recover');
              self3.$nextTick(function() {
                var el = document.getElementById('msg-input'); if (el) el.focus();
                self3._processQueue();
              });
            });
          } else {
            this._inflightPayload = null;
            this.refreshPromptSuggestions(true, 'post-response');
            this.$nextTick(function() {
              var el = document.getElementById('msg-input'); if (el) el.focus();
              self3._processQueue();
            });
          }
          break;
        case 'silent_complete':
          // Agent intentionally chose not to reply (NO_REPLY)
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
          this._clearPendingWsRequest(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '');
          this._clearTypingTimeout();
          this._clearStreamingTypewriters();
          this._inflightPayload = null;
          this._pendingAutoModelSwitchBaseline = '';
          var nowTs = Date.now();
          var hasRecentSubstantiveAgentReply = false;
          for (var si = this.messages.length - 1; si >= 0; si--) {
            var stable = this.messages[si];
            if (!stable) continue;
            if (stable.thinking || stable.streaming) continue;
            if (String(stable.role || '').toLowerCase() !== 'agent') continue;
            var stableText = String(stable.text || '').trim();
            if (!stableText) continue;
            if (stable._auto_fallback) continue;
            var stableAge = Math.max(0, nowTs - Number(stable.ts || nowTs));
            if (stableAge <= 20000) {
              hasRecentSubstantiveAgentReply = true;
            }
            break;
          }
          if (hasRecentSubstantiveAgentReply) {
            typeof this.clearTransientThinkingRows === 'function' ? this.clearTransientThinkingRows({ force: true }) : (this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; }));
            this.sending = false;
            this._responseStartedAt = 0;
            this.tokenCount = 0;
            var selfSilentSkip = this;
            this.$nextTick(function() { selfSilentSkip._processQueue(); });
            this.refreshPromptSuggestions(true, 'post-silent-skip');
            break;
          }
          var silentEnvelope = this.collectStreamedAssistantEnvelope();
          var silentThought = String(silentEnvelope.thought || '').trim();
          var silentTools = silentEnvelope.tools || [];
          if (silentThought) {
            silentTools.unshift(this.makeThoughtToolCard(silentThought, Number(data && data.duration_ms ? data.duration_ms : 0)));
          }
          typeof this.clearTransientThinkingRows === 'function' ? this.clearTransientThinkingRows({ force: true }) : (this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; }));
          this.sending = false;
          this._responseStartedAt = 0;
          this.tokenCount = 0;
          var selfSilent = this;
          this.$nextTick(function() { selfSilent._processQueue(); });
          this.refreshPromptSuggestions(true, 'post-silent-no-reply');
          break;
        case 'error':
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
          this._clearPendingWsRequest(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '');
          this._clearTypingTimeout();
          this._clearStreamingTypewriters();
          this._pendingAutoModelSwitchBaseline = '';
          var rawError = String(data && data.content ? data.content : 'unknown_error');
          var errorText = 'Error: ' + rawError;
          var lowerError = rawError.toLowerCase();
          if (
            lowerError.indexOf('this operation was aborted') >= 0 ||
            lowerError.indexOf('operation was aborted') >= 0
          ) {
            typeof this.clearTransientThinkingRows === 'function' ? this.clearTransientThinkingRows({ force: true }) : (this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; }));
            this.sending = false;
            this._responseStartedAt = 0;
            this.tokenCount = 0;
            this._inflightPayload = null;
            this.refreshPromptSuggestions(true, 'post-ws-abort');
            break;
          }
          if (lowerError.indexOf('backend_http_404') >= 0) {
            // Soft-ignore noisy command-surface 404s so they do not get injected
            // into the conversation stream after a successful agent response.
            typeof this.clearTransientThinkingRows === 'function' ? this.clearTransientThinkingRows({ preserve_running_tools: true, preserve_pending_ws: true }) : (this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; }));
            this.sending = false;
            this._responseStartedAt = 0;
            this.tokenCount = 0;
            this._inflightPayload = null;
            this.requestContextTelemetry(false);
            var selfSuppressed = this;
            this.$nextTick(function() {
              var el = document.getElementById('msg-input'); if (el) el.focus();
              selfSuppressed._processQueue();
            });
            this.refreshPromptSuggestions(true, 'post-suppressed-404');
            break;
          }
          if (lowerError.indexOf('agent contract terminated') !== -1 || lowerError.indexOf('agent_contract_terminated') !== -1) {
            this.handleAgentInactive(
              this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '',
              'contract_terminated',
              { noticeText: errorText }
            );
            break;
          }
          if (lowerError.indexOf('agent is inactive') !== -1 || lowerError.indexOf('agent_inactive') !== -1) {
            this.handleAgentInactive(
              this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '',
              'inactive',
              { noticeText: errorText }
            );
            break;
          }
          if (lowerError.indexOf('agent not found') !== -1 || lowerError.indexOf('agent_not_found') !== -1) {
            typeof this.clearTransientThinkingRows === 'function' ? this.clearTransientThinkingRows({ preserve_running_tools: true, preserve_pending_ws: true }) : (this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; }));
            this.sending = false;
            this._responseStartedAt = 0;
            this.tokenCount = 0;
            var priorAgentId = this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : '';
            var inflight = this._inflightPayload && typeof this._inflightPayload === 'object' ? this._inflightPayload : null;
            var rawNotFound = rawError;
            var selfRebound = this;
            Promise.resolve()
              .then(function() {
                return selfRebound.rebindCurrentAgentAuthoritative({
                  preferred_id: priorAgentId,
                  clear_when_missing: true
                });
              })
              .then(function(reboundAgent) {
                var reboundAgentId = reboundAgent && reboundAgent.id ? String(reboundAgent.id) : '';
                if (
                  reboundAgentId &&
                  reboundAgentId !== priorAgentId &&
                  inflight &&
                  !inflight._agent_rebind_attempted
                ) {
                  inflight._agent_rebind_attempted = true;
                  inflight.agent_id = reboundAgentId;
                  selfRebound.addNoticeEvent({
                    notice_label:
                      'Active agent reference expired. Switched to ' +
                      String(reboundAgent.name || reboundAgent.id || reboundAgentId) +
                      ' and retried.',
                    notice_type: 'warn',
                    ts: Date.now(),
                  });
                  return selfRebound._sendPayload(
                    inflight.final_text || '',
                    Array.isArray(inflight.uploaded_files) ? inflight.uploaded_files : [],
                    Array.isArray(inflight.msg_images) ? inflight.msg_images : [],
                    { agent_id: reboundAgentId, retry_from_agent_rebind: true }
                  );
                }
                return selfRebound
                  .attemptAutomaticFailoverRecovery('ws_error', rawNotFound, {
                    remove_last_agent_failure: false
                  })
                  .then(function(recovered) {
                    if (recovered) return;
                    selfRebound.pushSystemMessage({
                      text: 'Error: ' + rawNotFound,
                      meta: '',
                      tools: [],
                      system_origin: 'ws:error',
                      ts: Date.now(),
                      dedupe_window_ms: 12000
                    });
                    selfRebound._inflightPayload = null;
                  });
              })
              .catch(function() {});
            break;
          }
          typeof this.clearTransientThinkingRows === 'function' ? this.clearTransientThinkingRows({ preserve_running_tools: true, preserve_pending_ws: true }) : (this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; }));
          this.sending = false;
          this._responseStartedAt = 0;
          this.tokenCount = 0;
          var self2 = this;
          this.attemptAutomaticFailoverRecovery('ws_error', rawError, {
            remove_last_agent_failure: false
          }).then(function(recovered) {
            if (recovered) return;
            self2.pushSystemMessage({
