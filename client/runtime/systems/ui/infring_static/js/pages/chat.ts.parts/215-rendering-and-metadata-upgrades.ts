    resolveMessageToolRows: function(msg) {
      if (!msg || !Array.isArray(msg.tools)) return [];
      return msg.tools.filter(function(tool) {
        return !!tool && String(tool.name || '').toLowerCase() !== 'thought_process';
      });
    },

    // Backward-compat shim for legacy callers during naming migration.
    _messageToolRows: function(msg) {
      return this.resolveMessageToolRows(msg);
    },

    _collectSourceCandidatesFromValue: function(value, out, seen, depth) {
      if (!value || !out || !seen) return;
      var nextDepth = Number(depth || 0);
      if (!Number.isFinite(nextDepth) || nextDepth < 0) nextDepth = 0;
      if (nextDepth > 4 || out.length >= 24) return;
      if (typeof value === 'string') {
        var text = String(value || '').trim();
        if (/^https?:\/\//i.test(text)) {
          if (!seen[text]) {
            seen[text] = true;
            out.push({ url: text, label: '', source: '' });
          }
        }
        return;
      }
      if (Array.isArray(value)) {
        for (var ai = 0; ai < value.length && out.length < 24; ai += 1) {
          this._collectSourceCandidatesFromValue(value[ai], out, seen, nextDepth + 1);
        }
        return;
      }
      if (typeof value !== 'object') return;
      var url = String(
        value.url ||
        value.href ||
        value.link ||
        value.source_url ||
        value.final_url ||
        value.resolved_url ||
        ''
      ).trim();
      if (url && /^https?:\/\//i.test(url) && !seen[url]) {
        seen[url] = true;
        out.push({
          url: url,
          label: String(value.title || value.name || value.label || '').trim(),
          source: String(value.source || value.provider || value.domain || '').trim()
        });
      }
      var keys = Object.keys(value);
      for (var ki = 0; ki < keys.length && out.length < 24; ki += 1) {
        var key = keys[ki];
        if (!Object.prototype.hasOwnProperty.call(value, key)) continue;
        if (key === 'url' || key === 'href' || key === 'link' || key === 'source_url' || key === 'final_url' || key === 'resolved_url') continue;
        if (key === 'content' || key === 'result' || key === 'output' || key === 'payload' || key === 'data') {
          this._collectSourceCandidatesFromValue(value[key], out, seen, nextDepth + 1);
          continue;
        }
        if (nextDepth <= 2 && typeof value[key] === 'object') {
          this._collectSourceCandidatesFromValue(value[key], out, seen, nextDepth + 1);
        }
      }
    },

    _normalizeMessageSourceChip: function(row, idx) {
      var entry = row && typeof row === 'object' ? row : {};
      var url = String(entry.url || entry.href || entry.link || '').trim();
      if (!url || !/^https?:\/\//i.test(url)) return null;
      var label = String(entry.label || entry.title || entry.name || '').trim();
      var host = '';
      try {
        host = new URL(url).hostname.replace(/^www\./i, '');
      } catch (_) {
        host = '';
      }
      var source = String(entry.source || '').trim();
      if (!label) label = source || host || ('Source ' + (Number(idx || 0) + 1));
      if (label.length > 64) label = label.slice(0, 61).trim() + '...';
      return {
        id: 'src-' + (idx + 1) + '-' + label.toLowerCase().replace(/[^a-z0-9]+/g, '-'),
        label: label,
        host: host,
        source: source,
        url: url
      };
    },

    assistantTurnMetadataFromPayload: function(payload, tools) {
      var data = payload && typeof payload === 'object' ? payload : {};
      var out = {};
      if (data.response_workflow && typeof data.response_workflow === 'object') out.response_workflow = data.response_workflow;
      var finalization = typeof this.responseFinalizationFromPayload === 'function'
        ? this.responseFinalizationFromPayload(data)
        : (data.response_finalization && typeof data.response_finalization === 'object' ? data.response_finalization : null);
      if (finalization) out.response_finalization = finalization;
      if (data.turn_transaction && typeof data.turn_transaction === 'object') out.turn_transaction = data.turn_transaction;
      if (Array.isArray(data.terminal_transcript) && data.terminal_transcript.length) out.terminal_transcript = data.terminal_transcript.slice(0, 48);
      if (data.attention_queue && typeof data.attention_queue === 'object') out.attention_queue = data.attention_queue;
      if (Array.isArray(data.sources) && data.sources.length) out.sources = data.sources.slice(0, 16);
      if (Array.isArray(data.citations) && data.citations.length) out.citations = data.citations.slice(0, 24);
      if (Array.isArray(data.reference_links) && data.reference_links.length) out.reference_links = data.reference_links.slice(0, 24);
      var failureSummary = typeof this.readableToolFailureSummary === 'function'
        ? this.readableToolFailureSummary(data, tools)
        : '';
      if (failureSummary) out.tool_failure_summary = failureSummary;
      return out;
    },

    messageSourceChips: function(msg) {
      var row = msg && typeof msg === 'object' ? msg : {};
      var signature = [
        String(row.id || ''),
        String(row.text || '').length,
        Array.isArray(row.tools) ? row.tools.length : 0,
        row.response_workflow ? 'wf1' : 'wf0',
        row.response_finalization ? 'rf1' : 'rf0',
        row.turn_transaction ? 'tx1' : 'tx0'
      ].join('|');
      if (row._source_chip_signature === signature && Array.isArray(row._source_chips_cached)) {
        return row._source_chips_cached;
      }
      var candidates = [];
      var seenUrls = {};
      this._collectSourceCandidatesFromValue(row.sources, candidates, seenUrls, 0);
      this._collectSourceCandidatesFromValue(row.citations, candidates, seenUrls, 0);
      this._collectSourceCandidatesFromValue(row.response_workflow && row.response_workflow.citations, candidates, seenUrls, 0);
      this._collectSourceCandidatesFromValue(row.response_workflow && row.response_workflow.sources, candidates, seenUrls, 0);
      this._collectSourceCandidatesFromValue(row.response_finalization && row.response_finalization.citations, candidates, seenUrls, 0);
      this._collectSourceCandidatesFromValue(row.response_finalization && row.response_finalization.sources, candidates, seenUrls, 0);
      this._collectSourceCandidatesFromValue(row.turn_transaction && row.turn_transaction.citations, candidates, seenUrls, 0);
      this._collectSourceCandidatesFromValue(row.turn_transaction && row.turn_transaction.evidence, candidates, seenUrls, 0);
      if (Array.isArray(row.tools)) {
        for (var i = 0; i < row.tools.length && candidates.length < 24; i += 1) {
          var tool = row.tools[i] || {};
          var parsedResult = null;
          var loadedResult = this.toolRawResultTextIfLoaded(tool);
          if (loadedResult && typeof loadedResult === 'string') {
            var trimmed = String(loadedResult || '').trim();
            if (trimmed && (trimmed.charAt(0) === '{' || trimmed.charAt(0) === '[')) {
              try { parsedResult = JSON.parse(trimmed); } catch (_) {}
            }
          } else if (tool && tool._detail_loaded && tool.result && typeof tool.result === 'object') {
            parsedResult = tool.result;
          }
          this._collectSourceCandidatesFromValue(parsedResult, candidates, seenUrls, 0);
          if (Array.isArray(tool._imageUrls)) {
            for (var ui = 0; ui < tool._imageUrls.length && candidates.length < 24; ui += 1) {
              this._collectSourceCandidatesFromValue(tool._imageUrls[ui], candidates, seenUrls, 0);
            }
          }
        }
      }
      var chips = [];
      for (var ci = 0; ci < candidates.length && chips.length < 8; ci += 1) {
        var normalized = this._normalizeMessageSourceChip(candidates[ci], ci);
        if (!normalized) continue;
        chips.push(normalized);
      }
      row._source_chip_signature = signature;
      row._source_chips_cached = chips;
      return chips;
    },

    messageHasSourceChips: function(msg) {
      return this.messageSourceChips(msg).length > 0;
    },

    messageToolTraceSummary: function(msg) {
      var rows = this.resolveMessageToolRows(msg);
      var summary = {
        visible: false,
        running: false,
        total: 0,
        done: 0,
        blocked: 0,
        errored: 0,
        label: '',
        detail: ''
      };
      if (!rows.length) return summary;
      summary.visible = true;
      summary.total = rows.length;
      for (var i = 0; i < rows.length; i += 1) {
        var tool = rows[i];
        if (!tool) continue;
        if (tool.running) {
          summary.running = true;
          continue;
        }
        if (this.isBlockedTool(tool)) {
          summary.blocked += 1;
          continue;
        }
        if (tool.is_error) {
          summary.errored += 1;
          continue;
        }
        summary.done += 1;
      }
      summary.label = summary.running ? 'Tool trace running' : 'Tool trace complete';
      var bits = [];
      if (summary.done > 0) bits.push(summary.done + ' done');
      if (summary.errored > 0) bits.push(summary.errored + ' error');
      if (summary.blocked > 0) bits.push(summary.blocked + ' blocked');
      if (summary.running) bits.push((summary.total - (summary.done + summary.errored + summary.blocked)) + ' in progress');
      if (!bits.length) bits.push(summary.total + ' recorded');
      summary.detail = bits.join(' · ');
      return summary;
    },

    messageToolTraceRows: function(msg) {
      var rows = this.resolveMessageToolRows(msg);
      var out = [];
      for (var i = 0; i < rows.length && out.length < 6; i += 1) {
        var tool = rows[i] || {};
        var label = this.toolDisplayName(tool);
        var state = tool.running
          ? 'running'
          : (this.isBlockedTool(tool) ? 'blocked' : (tool.is_error ? 'error' : 'done'));
        out.push({
          id: String(tool.id || tool.attempt_id || (label + '-' + i)).trim(),
          label: label,
          state: state,
          detail: String(tool.status || '').trim()
        });
      }
      return out;
    },

    isThinkingShimmerText: function(msg) {
      if (!msg || !msg.thinking) return false;
      var status = typeof this.thinkingStatusText === 'function'
        ? String(this.thinkingStatusText(msg) || '').trim()
        : String(msg.thinking_status || msg.status_text || '').trim();
      if (!status) return true;
      if (typeof this.isThinkingPlaceholderText === 'function' && this.isThinkingPlaceholderText(status)) return true;
      return true;
    },

    thinkingPhaseText: function(msg) {
      if (!msg || !msg.thinking) return '';
      var primary = typeof this.thinkingStatusText === 'function'
        ? String(this.thinkingStatusText(msg) || '').trim()
        : '';
      var primaryNorm = primary.toLowerCase().replace(/\s+/g, ' ').trim();
      var summary = this.thinkingToolStatusSummary(msg);
      if (summary && summary.text) {
        var summaryText = String(summary.text || '').trim();
        var summaryNorm = summaryText.toLowerCase().replace(/\s+/g, ' ').trim();
        if (
          summaryNorm &&
          primaryNorm &&
          (summaryNorm === primaryNorm || summaryNorm.indexOf(primaryNorm) >= 0 || primaryNorm.indexOf(summaryNorm) >= 0)
        ) {
          return '';
        }
        return summaryText;
      }
      if (primaryNorm && primaryNorm !== 'thinking') {
        // Prevent duplicate waiting/workflow status lines.
        return '';
      }
      if (this._pendingWsRequest && this._pendingWsRequest.agent_id) return 'Thinking';
      return 'Thinking';
    },

    thinkingTraceSummary: function(msg) {
      if (!msg || !msg.thinking) return '';
      var rows = this.messageToolTraceRows(msg);
      if (!rows.length) return '';
      var running = rows.filter(function(row) { return row.state === 'running'; });
      if (running.length) {
        return running.slice(0, 2).map(function(row) { return row.label; }).join(' · ');
      }
      var failed = rows.filter(function(row) { return row.state === 'error' || row.state === 'blocked'; });
      if (failed.length) {
        return failed.slice(0, 2).map(function(row) { return row.label + ' (' + row.state + ')'; }).join(' · ');
      }
      return rows.slice(0, 2).map(function(row) { return row.label; }).join(' · ');
    },

    thinkingBubbleTraceRows: function(msg) {
      if (!msg || !msg.thinking) return [];
      var sourceRows = Array.isArray(msg.agent_runtime_live_trace_rows) ? msg.agent_runtime_live_trace_rows : [];
      var rawRows = [];
      var seen = Object.create(null);
      var fileActionVerb = function(kind) {
        return String(kind || '') === 'write' ? 'Edited' : 'Read';
      };
      var extractFileTraceLabel = function(text, kind) {
        var value = String(text || '').replace(/\r\n/g, '\n').trim();
        if (!value) return '';
        var quoted = value.match(/`([^`\n]+)`/);
        if (quoted && quoted[1]) return quoted[1].trim();
        var colon = value.match(/(?:file|path|changed|edited|wrote|written|created|updated|read|reading)\s*:?\s+(.+)$/i);
        if (colon && colon[1]) return colon[1].trim();
        var stripped = value.replace(/^(?:working on|completed|finished|failed|started|running|ran)\s+/i, '');
        stripped = stripped.replace(/^(?:read|reading|edit|edited|editing|write|wrote|writing|created|creating|updated|updating)\s+(?:file\s+|files\s+)?/i, '');
        stripped = stripped.replace(/^(?:file change|file)\s*:?\s*/i, '');
        stripped = stripped.trim();
        if (!stripped || stripped === value) return value;
        return stripped;
      };
      var compactFileTraceRows = function(rows) {
        var compacted = [];
        var group = null;
        var flush = function() {
          if (!group) return;
          var count = group.children.length;
          var verb = fileActionVerb(group.line_kind);
          compacted.push({
            id: group.id,
            text: verb + ' ' + count + ' ' + (count === 1 ? 'file' : 'files'),
            line_kind: group.line_kind,
            state: group.state,
            shimmer: false,
            children: group.children
          });
          group = null;
        };
        for (var ri = 0; ri < rows.length; ri += 1) {
          var item = rows[ri];
          var kind = String(item && item.line_kind || '').trim();
          if (kind !== 'read' && kind !== 'write') {
            flush();
            compacted.push(item);
            continue;
          }
          var label = extractFileTraceLabel(item.text, kind);
          var child = {
            id: String(item.id || (kind + '-file-' + ri)).slice(0, 180) + '-detail',
            text: fileActionVerb(kind) + ' ' + label,
            line_kind: kind,
            state: item.state || 'done',
            shimmer: false
          };
          if (!group || group.line_kind !== kind || group.state !== item.state) {
            flush();
            group = {
              id: 'file-group-' + kind + '-' + String(item.id || ri).slice(0, 140),
              line_kind: kind,
              state: item.state || 'done',
              children: []
            };
          }
          group.children.push(child);
        }
        flush();
        return compacted;
      };
      for (var i = 0; i < sourceRows.length && rawRows.length < 48; i += 1) {
        var row = sourceRows[i] && typeof sourceRows[i] === 'object' ? sourceRows[i] : {};
        var text = String(row.text || row.display_text || row.summary || '').replace(/\r\n/g, '\n').trim();
        if (!text) continue;
        var lineKind = String(row.line_kind || row.kind || '').trim() || 'status';
        var state = String(row.state || row.status || '').trim() || 'done';
        var key = (lineKind + '|' + text).toLowerCase();
        if (seen[key]) continue;
        seen[key] = true;
        rawRows.push({
          id: String(row.id || (lineKind + '-' + i)).slice(0, 180),
          text: text,
          line_kind: lineKind,
          state: state,
          shimmer: false
        });
      }
      var out = typeof this.compactThoughtTraceRows === 'function'
        ? this.compactThoughtTraceRows(rawRows)
        : compactFileTraceRows(rawRows);
      var latestShimmerIdx = -1;
      for (var j = out.length - 1; j >= 0; j -= 1) {
        if (out[j].line_kind !== 'dialog') {
          latestShimmerIdx = j;
          break;
        }
      }
      if (latestShimmerIdx >= 0) out[latestShimmerIdx].shimmer = true;
      return out;
    },

    thinkingBubbleSizeStyle: function(msg) {
      void msg;
      return '';
    },

    scheduleThinkingBubbleSizeStabilization: function(msg) {
      if (!msg || !msg.thinking || typeof document === 'undefined') return;
      var self = this;
      var run = function() {
        try {
          self.stabilizeThinkingBubbleSize(msg);
        } catch (_) {}
      };
      if (typeof this.$nextTick === 'function') this.$nextTick(run);
      else setTimeout(run, 0);
    },

    stabilizeThinkingBubbleSize: function(msg) {
      if (!msg || !msg.thinking || typeof document === 'undefined') return;
      var id = String(msg.id || '').trim();
      if (!id) return;
      var selectorId = typeof CSS !== 'undefined' && CSS.escape ? CSS.escape(id) : id.replace(/"/g, '\\"');
      var bubble = document.querySelector('[data-thinking-bubble-id="' + selectorId + '"]');
      if (!bubble) return;
      var previousInline = bubble.style.inlineSize;
      var previousMaxInline = bubble.style.maxInlineSize;
      bubble.style.inlineSize = 'fit-content';
      bubble.style.maxInlineSize = 'min(var(--message-bubble-readable-width), calc(100vw - 48px))';
      var rect = bubble.getBoundingClientRect ? bubble.getBoundingClientRect() : null;
      var naturalWidth = rect && Number.isFinite(Number(rect.width)) ? Number(rect.width) : 0;
      bubble.style.inlineSize = previousInline;
      bubble.style.maxInlineSize = previousMaxInline;
      if (!Number.isFinite(naturalWidth) || naturalWidth <= 0) return;
      var previous = Number(msg._thinking_bubble_width_px || 0);
      var tolerance = Number(msg._thinking_bubble_width_tolerance_px || 18);
      if (!Number.isFinite(tolerance) || tolerance < 4) tolerance = 18;
      if (!Number.isFinite(previous) || previous <= 0) {
        msg._thinking_bubble_width_px = Math.round(naturalWidth);
        return;
      }
      var delta = naturalWidth - previous;
      if (Math.abs(delta) <= tolerance) {
        msg._thinking_bubble_width_px = Math.round(naturalWidth);
        return;
      }
      msg._thinking_bubble_width_px = Math.round(previous + (delta > 0 ? tolerance : -tolerance));
    },

    thinkingWorkflowStatusLine: function(msg) {
      if (!msg || !msg.thinking) return '';
      var toolDialog = typeof this.currentToolDialogLabel === 'function'
        ? String(this.currentToolDialogLabel(msg) || '').trim()
        : '';
      if (typeof this.normalizeThinkingStatusCandidate === 'function') {
        toolDialog = this.normalizeThinkingStatusCandidate(toolDialog);
      }
      if (toolDialog) return toolDialog;
      var explicitStatus = String(msg.thinking_status || msg.status_text || '').trim();
      if (typeof this.normalizeThinkingStatusCandidate === 'function') {
        explicitStatus = this.normalizeThinkingStatusCandidate(explicitStatus);
      }
      if (typeof this.isThinkingPlaceholderText === 'function' && this.isThinkingPlaceholderText(explicitStatus)) {
        return '';
      }
      return explicitStatus;
    },

    thinkingInnerDialogLine: function(msg) {
      if (!msg || !msg.thinking) return '';
      var thought = typeof this.thinkingDisplayText === 'function'
        ? String(this.thinkingDisplayText(msg) || '').trim()
        : '';
      if (!thought) {
        thought = String(msg._reasoning || msg._thoughtText || '').trim();
      }
      if (!thought && msg && msg.thoughtStreaming) {
        thought = String(msg._thought_latest_chunk || '').trim();
      }
      if (typeof this.normalizeThinkingStatusCandidate === 'function') {
        thought = this.normalizeThinkingStatusCandidate(thought);
      }
      if (!thought) return '';
      var lowered = thought.toLowerCase().replace(/\s+/g, ' ').trim();
      if (!lowered || lowered === 'thinking') return '';
      if (thought.length > 180) thought = thought.slice(0, 177).trim() + '...';
      return thought;
    },

    thinkingBubbleLineText: function(msg) {
      if (!msg || !msg.thinking) return '';
      var primary = typeof this.thinkingWorkflowStatusLine === 'function'
        ? String(this.thinkingWorkflowStatusLine(msg) || '').trim()
        : '';
      var primaryNorm = primary.toLowerCase().replace(/\s+/g, ' ').trim();
      var thought = typeof this.thinkingInnerDialogLine === 'function'
        ? String(this.thinkingInnerDialogLine(msg) || '').trim()
        : '';
      var thoughtNorm = thought.toLowerCase().replace(/\s+/g, ' ').trim();
      if (primary && primaryNorm && primaryNorm !== 'thinking') {
        if (
          thought &&
          thoughtNorm &&
          thoughtNorm !== primaryNorm &&
          thoughtNorm.indexOf(primaryNorm) === -1 &&
          primaryNorm.indexOf(thoughtNorm) === -1
        ) {
          var composedPrimary = primary.replace(/(\.\.\.|…)+$/g, '').trim();
          if (composedPrimary && !/[.!?:]$/.test(composedPrimary)) composedPrimary += '...';
          else if (composedPrimary && /[.!?:]$/.test(composedPrimary) && !/(\.\.\.|…)$/.test(composedPrimary)) composedPrimary += ' ';
          return (composedPrimary + ' ' + thought).replace(/\s+/g, ' ').trim();
        }
        return primary;
      }
      if (thought) return thought;
      var phase = typeof this.thinkingPhaseText === 'function'
        ? String(this.thinkingPhaseText(msg) || '').trim()
        : '';
      if (phase) return phase;
      var trace = typeof this.thinkingTraceSummary === 'function'
        ? String(this.thinkingTraceSummary(msg) || '').trim()
        : '';
      if (trace) return trace;
      if (primary) return primary;
      return 'Thinking';
    },

    _workspaceState: function() {
      if (!this._messageWorkspaceState || typeof this._messageWorkspaceState !== 'object') {
        this._messageWorkspaceState = {
          open: false,
          payload: null
        };
      }
      return this._messageWorkspaceState;
    },

    isWorkspacePanelOpen: function() {
      var state = this._workspaceState();
      return !!state.open && !!state.payload;
    },

    closeWorkspacePanel: function() {
      var state = this._workspaceState();
      state.open = false;
      state.payload = null;
    },

    _messageTextPreviewForWorkspace: function(msg) {
      var text = '';
      if (typeof this.extractMessageVisibleText === 'function') {
        text = String(this.extractMessageVisibleText(msg) || '').trim();
      }
      if (!text) text = String(msg && msg.text || '').trim();
      if (text.length > 420) text = text.slice(0, 417).trim() + '...';
      return text;
    },

    _messageArtifactsForWorkspace: function(msg) {
      var row = msg && typeof msg === 'object' ? msg : {};
      var out = [];
      if (row.file_output && row.file_output.path) {
        out.push({ id: 'file-' + String(row.file_output.path), type: 'File', label: String(row.file_output.path), detail: String(row.file_output.bytes || '') });
      }
      if (row.folder_output && row.folder_output.path) {
        out.push({ id: 'folder-' + String(row.folder_output.path), type: 'Folder', label: String(row.folder_output.path), detail: String(row.folder_output.entries || '') + ' entries' });
      }
      if (Array.isArray(row.images) && row.images.length) {
        out.push({ id: 'images-' + row.images.length, type: 'Images', label: String(row.images.length) + ' uploaded image(s)', detail: '' });
      }
      return out;
    },

    openWorkspacePanelForMessage: function(msg, idx, rows) {
      var row = msg && typeof msg === 'object' ? msg : {};
      var state = this._workspaceState();
      var trace = this.messageToolTraceRows(row);
      state.payload = {
        id: String(row.id || ('msg-' + String(idx || 0))).trim(),
        actor: typeof this.messageActorLabel === 'function' ? this.messageActorLabel(row) : String(row.role || 'Message'),
        timestamp: typeof this.messageTs === 'function' ? this.messageTs(row) : '',
        preview: this._messageTextPreviewForWorkspace(row),
        sources: this.messageSourceChips(row),
        trace: trace,
        artifacts: this._messageArtifactsForWorkspace(row),
        rows_count: Array.isArray(rows) ? rows.length : 0
      };
      state.open = true;
    },

    workspacePanelPayload: function() {
      var state = this._workspaceState();
      if (state.payload && typeof state.payload === 'object') return state.payload;
      return {
        id: '',
        actor: '',
        timestamp: '',
        preview: '',
        sources: [],
        trace: [],
        artifacts: [],
        rows_count: 0
      };
    },

    messageMetadataService: function() {
      var services = typeof InfringSharedShellServices !== 'undefined' ? InfringSharedShellServices : null;
      return services && services.messageMeta ? services.messageMeta : null;
    },

    messageMetadataShellState: function(msg, idx, rows) {
      var service = this.messageMetadataService();
      var list = Array.isArray(rows) ? rows : (Array.isArray(this.messages) ? this.messages : []);
      var model = service && typeof service.viewModel === 'function' ? service.viewModel({
        row: msg,
        index: idx,
        rows: list,
        agent: this.currentAgent,
        shouldRender: typeof this.shouldRenderMessageContent === 'function' ? this.shouldRenderMessageContent(msg, idx, list) : true,
        collapsed: typeof this.isMessageMetaCollapsed === 'function' ? this.isMessageMetaCollapsed(msg, idx, list) : false,
        copied: !!(msg && msg._copied),
        hasTools: typeof this.messageHasTools === 'function' ? this.messageHasTools(msg) : !!(msg && Array.isArray(msg.tools) && msg.tools.length),
        toolsCollapsed: typeof this.allToolsCollapsed === 'function' ? this.allToolsCollapsed(msg) : true,
        timestamp: typeof this.messageTs === 'function' ? this.messageTs(msg) : '',
        responseTimeMs: typeof this.messageStatResponseTimeMs === 'function' ? this.messageStatResponseTimeMs(msg) : 0,
        responseTimeFormatter: typeof this.formatResponseDuration === 'function' ? this.formatResponseDuration.bind(this) : null,
        burnTotalTokens: typeof this.messageStatBurnTotalTokens === 'function' ? this.messageStatBurnTotalTokens(msg) : 0,
        burnFormatter: typeof this.formatTokenK === 'function' ? this.formatTokenK.bind(this) : null
      }) : { shouldRender: false };
      try { return JSON.stringify(model); } catch (_) { return '{"shouldRender":false}'; }
    },

    handleMessageMetaAction: function(event, msg, idx, rows) {
      var action = String(event && event.detail && event.detail.action || '').trim();
      var handlers = {
        copy: this.copyMessage.bind(this, msg),
        report: this.reportIssueFromMeta.bind(this, msg, idx),
        'toggle-tools': this.toggleMessageTools.bind(this, msg),
        retry: this.retryMessageFromMeta.bind(this, msg, idx, rows),
        reply: this.replyToMessageFromMeta.bind(this, msg, idx, rows),
        fork: this.forkMessageFromMeta.bind(this, msg, idx, rows)
      };
      var handler = handlers[action];
      if (typeof handler === 'function') return handler();
    },

    messageCanRetryFromMeta: function(msg, idx, rows) {
      var service = this.messageMetadataService();
      var list = Array.isArray(rows) ? rows : (Array.isArray(this.messages) ? this.messages : []);
      return !!(service && typeof service.canRetry === 'function' && service.canRetry(msg, idx, list));
    },

    _resolveMessageIndexFromMeta: function(msg, idx, rows) {
      var service = this.messageMetadataService();
      var list = Array.isArray(rows) ? rows : (Array.isArray(this.messages) ? this.messages : []);
      return service && typeof service.resolveIndex === 'function' ? service.resolveIndex(msg, idx, list) : -1;
    },

    messageIsLatestAgentFromMeta: function(msg, idx, rows) {
      var service = this.messageMetadataService();
      var list = Array.isArray(rows) ? rows : (Array.isArray(this.messages) ? this.messages : []);
      return !!(service && typeof service.isLatestAgent === 'function' && service.isLatestAgent(msg, idx, list));
    },

    messageCanReplyFromMeta: function(msg, idx, rows) {
      var service = this.messageMetadataService();
      var list = Array.isArray(rows) ? rows : (Array.isArray(this.messages) ? this.messages : []);
      return !!(service && typeof service.canReply === 'function' && service.canReply(msg, idx, list));
    },

    replyToMessageFromMeta: function(msg, idx, rows) {
      void msg;
      void idx;
      void rows;
      if (typeof InfringToast !== 'undefined') InfringToast.info('Reply requires a backend quote-by-reference contract.');
    },

    messageCanForkFromMeta: function(msg) {
      var service = this.messageMetadataService();
      return !!(service && typeof service.canFork === 'function' && service.canFork(msg, this.currentAgent));
    },

    retryMessageFromMeta: async function(msg, idx, rows) {
      if (this.sending) return;
      var allowed = this.messageCanRetryFromMeta(msg, idx, rows);
      if (!allowed) return;
      void msg;
      void idx;
      void rows;
      if (typeof InfringToast !== 'undefined') InfringToast.info('Retry requires a backend replay contract.');
    },

    forkMessageFromMeta: async function(msg, idx, rows) {
      if (!this.currentAgent || !this.currentAgent.id || this.sending) return;
      void idx;
      void rows;
      if (typeof this.messageCanForkFromMeta === 'function' && !this.messageCanForkFromMeta(msg)) return;
      var sourceAgent = this.currentAgent && typeof this.currentAgent === 'object' ? this.currentAgent : {};
      var sourceAgentId = String(sourceAgent.id || '').trim();
      if (!sourceAgentId) return;
      try {
        this.cacheCurrentConversation();
        var created = await InfringAPI.post(
          '/api/shell-socket/agents/' + encodeURIComponent(sourceAgentId) + '/clone',
          {}
        );
        var forkedAgentId = String(
          (created && (created.agent_id || created.id)) ||
          ''
        ).trim();
        if (!forkedAgentId) {
          throw new Error('agent_clone_failed');
        }
        var forkedAgentName = String((created && created.name) || forkedAgentId).trim();
        var appStoreBridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
          ? InfringSharedShellServices.appStore
          : null;
        var refreshAgents = appStoreBridge && typeof appStoreBridge.method === 'function'
          ? appStoreBridge.method('refreshAgents')
          : null;
        if (typeof refreshAgents === 'function') {
          await refreshAgents({ force: true });
        }
        var resolvedForkedAgent = this.resolveAgent(forkedAgentId);
        if (!resolvedForkedAgent) {
          resolvedForkedAgent = {
            id: forkedAgentId,
            name: forkedAgentName,
            role: String(sourceAgent.role || 'analyst')
          };
        }
        this.selectAgent(resolvedForkedAgent);
        if (typeof InfringToast !== 'undefined') {
          InfringToast.success('Forked to new agent "' + forkedAgentName + '"');
        }
      } catch (e) {
        if (typeof InfringToast !== 'undefined') InfringToast.error('Failed to fork message: ' + (e && e.message ? e.message : 'unknown error'));
      }
    },
