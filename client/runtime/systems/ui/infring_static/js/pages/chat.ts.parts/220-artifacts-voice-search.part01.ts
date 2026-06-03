    deriveUserFacingFromThought: function(thoughtText) {
      var thought = String(thoughtText || '').replace(/\s+/g, ' ').trim();
      if (!thought) return '';
      var skip = /^(alright|okay|ok|hmm|let me|i need to|i should|i will|first[, ]|to answer this|it seems|we need to)\b/i;
      var sentences = thought
        .split(/(?<=[.!?])\s+/)
        .map(function(part) { return String(part || '').trim(); })
        .filter(function(part) { return !!part; });
      var keep = [];
      for (var i = 0; i < sentences.length; i++) {
        var sentence = sentences[i];
        var lower = sentence.toLowerCase();
        if (skip.test(sentence) && lower.indexOf('queue depth') < 0 && lower.indexOf('scale') < 0 && lower.indexOf('recommend') < 0 && lower.indexOf('command') < 0) {
          continue;
        }
        if (lower.indexOf('user') >= 0 && lower.indexOf('request') >= 0) continue;
        if (sentence.length < 20) continue;
        keep.push(sentence);
      }
      if (!keep.length) {
        var queueLine = thought.match(/queue depth[^.?!]*[.?!]?/i);
        if (queueLine && queueLine[0]) keep.push(String(queueLine[0]).trim());
        var scaleLine = thought.match(/scale[^.?!]*instances?[^.?!]*[.?!]?/i);
        if (scaleLine && scaleLine[0]) keep.push(String(scaleLine[0]).trim());
      }
      if (!keep.length) return '';
      var message = keep.slice(0, 2).join(' ').replace(/\s+/g, ' ').trim();
      if (!message) return '';
      if (!/[.?!]$/.test(message)) message += '.';
      if (message.length > 300) message = message.slice(0, 297) + '...';
      return message;
    },
    extractArtifactDirectives: function(text) {
      var value = String(text || '');
      if (!value) return [];
      var rx = /\[\[\s*(file|folder)\s*:\s*([^\]]+?)\s*\]\]/gi;
      var out = [];
      var match;
      while ((match = rx.exec(value)) && out.length < 4) {
        var kind = String(match[1] || '').toLowerCase();
        var targetPath = String(match[2] || '').trim();
        if (!targetPath) continue;
        out.push({ kind: kind, path: targetPath });
      }
      return out;
    },
    stripArtifactDirectivesFromText: function(text) {
      var value = String(text || '');
      if (!value) return '';
      return value.replace(/\[\[\s*(file|folder)\s*:\s*[^\]]+?\s*\]\]/gi, '').replace(/\n{3,}/g, '\n\n').trim();
    },
    resolveArtifactDirectives: async function(directives) {
      if (!this.currentAgent || !this.currentAgent.id) return;
      var rows = Array.isArray(directives) ? directives : [];
      if (!rows.length) return;
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i] || {};
        var targetPath = String(row.path || '').trim();
        if (!targetPath) continue;
        try {
          if (row.kind === 'file') {
            var fileRes = await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(this.currentAgent.id) + '/artifacts/file/read', {
              path: targetPath
            });
            var fileMeta = fileRes && fileRes.file ? fileRes.file : null;
            if (fileMeta && fileMeta.ok) {
              this.messages.push({
                id: ++msgId,
                role: 'agent',
                text: '',
                meta: (Number(fileMeta.bytes || 0) > 0 ? (Number(fileMeta.bytes || 0) + ' bytes') : ''),
                tools: [],
                ts: Date.now(),
                file_output: {
                  path: String(fileMeta.path || targetPath),
                  content: String(fileMeta.content || ''),
                  truncated: !!fileMeta.truncated,
                  bytes: Number(fileMeta.bytes || 0)
                }
              });
            }
          } else if (row.kind === 'folder') {
            var folderRes = await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(this.currentAgent.id) + '/artifacts/folder/export', {
              path: targetPath
            });
            var folderMeta = folderRes && folderRes.folder ? folderRes.folder : null;
            var archiveMeta = folderRes && folderRes.archive ? folderRes.archive : null;
            if (folderMeta && folderMeta.ok) {
              this.messages.push({
                id: ++msgId,
                role: 'agent',
                text: '',
                meta: Number(folderMeta.entries || 0) + ' entries',
                tools: [],
                ts: Date.now(),
                folder_output: {
                  path: String(folderMeta.path || targetPath),
                  tree: String(folderMeta.tree || ''),
                  entries: Number(folderMeta.entries || 0),
                  truncated: !!folderMeta.truncated,
                  download_url: archiveMeta && archiveMeta.download_url ? String(archiveMeta.download_url) : '',
                  archive_name: archiveMeta && archiveMeta.file_name ? String(archiveMeta.file_name) : '',
                  archive_bytes: Number(archiveMeta && archiveMeta.bytes ? archiveMeta.bytes : 0)
                }
              });
            }
          }
        } catch (_) {}
      }
      this.scrollToBottom();
      this.scheduleConversationPersist();
    },

    // Remove disclosure/speaker prefixes injected by model/backend responses.
    // Examples:
    //   "[openai/gpt-5] hello" -> "hello"
    //   "Agent: hello" -> "hello"
    //   "**Assistant:** hello" -> "hello"
    stripModelPrefix: function(text) {
      if (!text) return text;
      var out = String(text);
      var lowered = out.toLowerCase();
      var recallIdx = lowered.indexOf('recalled context:');
      if (recallIdx >= 0) {
        var prefix = lowered.slice(0, recallIdx);
        var looksLikeMemoryMeta = prefix.indexOf('persistent memory') >= 0 ||
          prefix.indexOf('stored messages') >= 0 ||
          prefix.indexOf('session(s)') >= 0 ||
          prefix.indexOf(' sessions') >= 0;
        if (looksLikeMemoryMeta) {
          var leakedTail = out.slice(recallIdx + 'recalled context:'.length).trim();
          var finalIdx = leakedTail.toLowerCase().indexOf('final answer:');
          if (finalIdx >= 0) {
            out = leakedTail.slice(finalIdx + 'final answer:'.length).trim();
          } else {
            out = '';
          }
        }
      }
      if (/persistent memory is enabled for this agent across/i.test(out)) {
        var finalAnswerMatch = out.match(/(?:^|\n)\s*final answer\s*:\s*/i);
        if (finalAnswerMatch && Number.isFinite(Number(finalAnswerMatch.index))) {
          out = out.slice(Number(finalAnswerMatch.index) + String(finalAnswerMatch[0] || '').length).trim();
        } else {
          var strippedLines = out.split(/\r?\n/).filter(function(line) {
            var value = String(line || '').trim().toLowerCase();
            if (!value) return false;
            if (/^e2e-\d+-res$/.test(value)) return false;
            if (value.indexOf('persistent memory is enabled for this agent across') === 0) return false;
            if (value.indexOf('recalled context:') === 0) return false;
            if (value.indexOf('stored messages') >= 0) return false;
            return true;
          });
          out = strippedLines.join('\n').trim();
        }
      }
      for (var i = 0; i < 6; i++) {
        var prior = out;
        out = out.replace(/^\s*\[[^\]\n]{2,96}\]\s*/, '');
        // Strip leaked transcript wrappers like "User: ... Agent: <answer>".
        var transcriptLead = out.match(
          /^\s*(?:[-*]\s*)?(?:\*\*)?(?:user|human|you)(?:\*\*)?\s*:\s*[\s\S]{0,1200}?(?:\*\*)?(?:agent|assistant|model|ai|jarvis)(?:\*\*)?\s*:\s*/i
        );
        if (transcriptLead && transcriptLead[0]) {
          out = out.slice(transcriptLead[0].length);
          continue;
        }
        out = out.replace(
          /^\s*(?:[-*]\s*)?(?:\*\*)?(?:agent|assistant|system|model|ai|jarvis|user|human|you)(?:\*\*)?\s*:\s*/i,
          ''
        );
        if (out === prior) break;
      }
      out = out.replace(/^e2e-\d+-res\s*/i, '').trim();
      out = out.replace(
        /\s*i could not produce a final answer this turn\.\s*please retry or clarify what you want next\.\s*/ig,
        '\n'
      ).trim();
      return out;
    },

    formatToolJson: function(text) {
      if (!text) return '';
      try { return JSON.stringify(JSON.parse(text), null, 2); }
      catch(e) { return text; }
    },

    // Voice: start recording
    startRecording: async function() {
      if (this.recording) return;
      try {
        var stream = await navigator.mediaDevices.getUserMedia({ audio: true });
        var mimeType = MediaRecorder.isTypeSupported('audio/webm;codecs=opus') ? 'audio/webm;codecs=opus' :
                       MediaRecorder.isTypeSupported('audio/webm') ? 'audio/webm' : 'audio/ogg';
        this._audioChunks = [];
        this._mediaRecorder = new MediaRecorder(stream, { mimeType: mimeType });
        var self = this;
        this._mediaRecorder.ondataavailable = function(e) {
          if (e.data.size > 0) self._audioChunks.push(e.data);
        };
        this._mediaRecorder.onstop = function() {
          stream.getTracks().forEach(function(t) { t.stop(); });
          self._handleRecordingComplete();
        };
        this._mediaRecorder.start(250);
        this.recording = true;
        this.recordingTime = 0;
        this._recordingTimer = setInterval(function() { self.recordingTime++; }, 1000);
      } catch(e) {
        if (typeof InfringToast !== 'undefined') InfringToast.error('Microphone access denied');
      }
    },

    // Voice: stop recording
    stopRecording: function() {
      if (!this.recording || !this._mediaRecorder) return;
      this._mediaRecorder.stop();
      this.recording = false;
      if (this._recordingTimer) { clearInterval(this._recordingTimer); this._recordingTimer = null; }
    },

    // Voice: handle completed recording — upload and transcribe
    _handleRecordingComplete: async function() {
      var voiceAgent = this.ensureValidCurrentAgent({ clear_when_missing: true });
      if (!this._audioChunks.length || !voiceAgent || !voiceAgent.id) return;
      var blob = new Blob(this._audioChunks, { type: this._audioChunks[0].type || 'audio/webm' });
      this._audioChunks = [];
      if (blob.size < 100) return; // too small

      // Show a temporary "Transcribing..." message
      this.messages.push({ id: ++msgId, role: 'system', is_notice: true, notice_type: 'info', notice_label: 'Transcribing audio...', text: 'Transcribing audio...', thinking: true, ts: Date.now(), tools: [], system_origin: 'voice:transcribe' });
      this.scrollToBottom();

      try {
        // Upload audio file
        var ext = blob.type.includes('webm') ? 'webm' : blob.type.includes('ogg') ? 'ogg' : 'mp3';
        var file = new File([blob], 'voice_' + Date.now() + '.' + ext, { type: blob.type });
        var upload = await InfringAPI.upload(voiceAgent.id, file);

        // Remove the "Transcribing..." message
        this.messages = this.messages.filter(function(m) { return !m.thinking || m.role !== 'system'; });

        // Use server-side transcription if available, otherwise fall back to placeholder
        var text = (upload.transcription && upload.transcription.trim())
          ? upload.transcription.trim()
          : '[Voice message - audio: ' + upload.filename + ']';
        this._sendPayload(text, [upload], [], { agent_id: voiceAgent.id });
      } catch(e) {
        this.messages = this.messages.filter(function(m) { return !m.thinking || m.role !== 'system'; });
        if (typeof InfringToast !== 'undefined') InfringToast.error('Failed to upload audio: ' + (e.message || 'unknown error'));
      }
    },

    // Voice: format recording time as MM:SS
    formatRecordingTime: function() {
      var m = Math.floor(this.recordingTime / 60);
      var s = this.recordingTime % 60;
      return (m < 10 ? '0' : '') + m + ':' + (s < 10 ? '0' : '') + s;
    },

    // Search: toggle open/close
    toggleSearch: function() {
      this.searchOpen = !this.searchOpen;
      if (this.searchOpen) {
        var self = this;
        this.$nextTick(function() {
          var el = document.getElementById('chat-search-input');
          if (el) el.focus();
        });
      } else {
        this.searchQuery = '';
      }
    },

    messageDisplayScopeKey: function() {
      var agentId = String((this.currentAgent && this.currentAgent.id) || '').trim();
      var sessionId = '';
      if (Array.isArray(this.sessions)) {
        for (var i = 0; i < this.sessions.length; i += 1) {
          var row = this.sessions[i];
          if (row && row.active) {
            sessionId = String((row.session_id || row.id || '')).trim();
            break;
          }
        }
      }
      var search = String(this.searchQuery || '').trim().toLowerCase();
      return agentId + '|' + sessionId + '|' + search;
    },

    // Backward-compat shim for legacy callers during naming migration.
    _messageDisplayScopeKey: function() {
      return this.messageDisplayScopeKey();
    },

    ensureMessageDisplayWindow: function(totalCount) {
      var total = Number(totalCount || 0);
      if (!Number.isFinite(total) || total < 0) total = 0;
      var key = this.messageDisplayScopeKey();
      if (String(this._messageDisplayKey || '') !== key) {
        this._messageDisplayKey = key;
        this.messageDisplayCount = Number(this.messageDisplayInitialLimit || 10);
      }
      var rawQuery = String(this.searchQuery || '').trim();
      if (!rawQuery) {
        // Normal chat mode should keep full history visible; virtualization handles perf.
        this.messageDisplayCount = total;
        return;
      }
      var base = Number(this.messageDisplayInitialLimit || 10);
      if (!Number.isFinite(base) || base < 1) base = 10;
      if (!Number.isFinite(Number(this.messageDisplayCount))) {
        this.messageDisplayCount = base;
      }
      if (this.messageDisplayCount < base) this.messageDisplayCount = base;
      if (this.messageDisplayCount > total) this.messageDisplayCount = total;
    },

    get canExpandDisplayedMessages() {
      var total = Array.isArray(this.allFilteredMessages) ? this.allFilteredMessages.length : 0;
      this.ensureMessageDisplayWindow(total);
      return total > Number(this.messageDisplayCount || 0);
    },

    get expandRemainingCount() {
      var total = Array.isArray(this.allFilteredMessages) ? this.allFilteredMessages.length : 0;
      var visible = Number(this.messageDisplayCount || 0);
      if (!Number.isFinite(visible)) visible = 0;
      return Math.max(0, total - visible);
    },

    expandDisplayedMessages: function() {
      var total = Array.isArray(this.allFilteredMessages) ? this.allFilteredMessages.length : 0;
      this.ensureMessageDisplayWindow(total);
      if (total <= Number(this.messageDisplayCount || 0)) return;
      var step = Number(this.messageDisplayStep || 5);
      if (!Number.isFinite(step) || step < 1) step = 5;
      this.messageDisplayCount = Math.min(total, Number(this.messageDisplayCount || 0) + step);
    },

    // Search: full filtered message set before display-window capping.
    get allFilteredMessages() {
      var query = String(this.searchQuery || '').trim();
      if (!query) return this.messages;
      var self = this;
      var filtered = this.messages.filter(function(m) {
        if (typeof self.messageMatchesSearchQuery === 'function') return self.messageMatchesSearchQuery(m, query);
        var text = typeof (m && m.text) === 'string' ? m.text : String((m && m.text) || '');
        return text.toLowerCase().indexOf(query.toLowerCase()) !== -1;
      });
      if (filtered.length > 0) return filtered;
      // Avoid "blank thread" states from stale hidden query filters.
      if (!this.searchOpen && Array.isArray(this.messages) && this.messages.length > 0) {
        return this.messages;
      }
      return filtered;
    },

    // Search: filter messages by query + apply incremental display capping.
    get filteredMessages() {
      var all = Array.isArray(this.allFilteredMessages) ? this.allFilteredMessages : [];
      this.ensureMessageDisplayWindow(all.length);
      if (!all.length) return all;
      var visible = Number(this.messageDisplayCount || 0);
      if (!Number.isFinite(visible) || visible < 1 || visible >= all.length) return all;
      return all.slice(Math.max(0, all.length - visible));
    },

    // Search: highlight matched text in a string
    highlightSearch: function(html) {
      if (!this.searchQuery.trim() || !html) return html;
      var q = this.searchQuery.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      var regex = new RegExp('(' + q + ')', 'gi');
      return html.replace(regex, '<mark style="background:var(--warning);color:var(--bg);border-radius:2px;padding:0 2px">$1</mark>');
    },

    messageVisibleLineWindow: function(msg, idx) {
      var row = msg && typeof msg === 'object' ? msg : {};
      var text = String(row.text || '');
      if (!text || row._typingVisual || row.isHtml) return { text: text, truncated: false, key: '', shown: 0, total: 0 };
      var step = Number(this.messageLineExpandStep || 20);
      if (!Number.isFinite(step) || step < 20) step = 20;
      var key = String(row.id || '').trim() || ('line:' + String(row.ts || '') + ':' + String(idx || 0));
      if (!this.messageLineExpandState || typeof this.messageLineExpandState !== 'object') this.messageLineExpandState = {};
      var shown = Number(this.messageLineExpandState[key] || step);
      if (!Number.isFinite(shown) || shown < step) shown = step;
      var lines = text.split(/\r?\n/);
      var total = lines.length;
      if (shown >= total) return { text: text, truncated: false, key: key, shown: total, total: total };
      return { text: lines.slice(0, shown).join('\n'), truncated: true, key: key, shown: shown, total: total };
    },

    // Backward-compat shim for legacy callers during naming migration.
    messageLineWindow: function(msg, idx) {
      return this.messageVisibleLineWindow(msg, idx);
    },

    messageHasLineOverflow: function(msg, idx) {
      return !!this.messageVisibleLineWindow(msg, idx).truncated;
    },

    expandMessageLines: function(msg, idx) {
      var window = this.messageVisibleLineWindow(msg, idx);
      if (!window.truncated || !window.key) return;
      var step = Number(this.messageLineExpandStep || 20);
      if (!Number.isFinite(step) || step < 20) step = 20;
      this.messageLineExpandState[window.key] = Math.min(window.total, window.shown + step);
    },

    messageBubbleHtml: function(msg, idx) {
      if (!msg || typeof msg !== 'object') return '';
      if (msg._typingVisual) {
        if (typeof msg._typingVisualHtml === 'string' && msg._typingVisualHtml.trim()) {
          return msg._typingVisualHtml;
        }
        return this.escapeHtml(String(msg.text || ''));
      }
      var lineWindow = this.messageVisibleLineWindow(msg, idx);
      var displayText = String(lineWindow.text || '');
      var role = String(msg.role || '').trim().toLowerCase();
      var baseHtml = '';
      if (msg.isHtml) {
        baseHtml = String(displayText || '');
      } else if ((role === 'agent' || role === 'assistant' || role === 'system') && !msg.thinking) {
        baseHtml = this.renderMarkdown(String(displayText || ''));
      } else {
        baseHtml = this.escapeHtml(String(displayText || ''));
      }
      return this.highlightSearch(baseHtml);
    },
    messageTypingReserveStyle: function(msg) {
      if (!msg || typeof msg !== 'object' || !msg._typingVisual) return '';
      var finalText = String(msg._typewriterFinalText || msg.text || '');
      if (!finalText.trim()) return '--typing-reserve-height:72px;';
      var hardLines = finalText.split(/\r?\n/).length;
      var softWrapLines = Math.ceil(Math.max(0, finalText.length - (hardLines * 20)) / 92);
      var visualLines = Math.max(1, hardLines + softWrapLines);
      var reserveHeight = 20 + (visualLines * 25);
      reserveHeight = Math.max(72, Math.min(980, Math.round(reserveHeight)));
      return '--typing-reserve-height:' + reserveHeight + 'px;';
    },

    renderMarkdown: renderMarkdown,
    escapeHtml: escapeHtml
  };
}

function cancelPinToLatestOnOpenJob(page) {
  if (!page || typeof page !== 'object') return;
  if (page._openPinRaf && typeof cancelAnimationFrame === 'function') {
    cancelAnimationFrame(page._openPinRaf);
  }
  if (page._openPinTimer) {
    clearTimeout(page._openPinTimer);
  }
  page._openPinRaf = 0;
  page._openPinTimer = 0;
}

function runPinToLatestOnOpenJob(page, container, options) {
  if (!page || typeof page !== 'object') return;
  var opts = options || {};
  var maxFrames = Number(opts.maxFrames || 18);
  if (!Number.isFinite(maxFrames) || maxFrames < 4) maxFrames = 18;
  if (maxFrames > 64) maxFrames = 64;
  var stableFramesNeeded = Number(opts.stableFrames || 2);
  if (!Number.isFinite(stableFramesNeeded) || stableFramesNeeded < 1) stableFramesNeeded = 2;
  if (stableFramesNeeded > 6) stableFramesNeeded = 6;
  var token = Number(page._openPinToken || 0) + 1;
  var frame = 0;
  var stable = 0;
  var lastTop = -1;
  var lastHeight = -1;
  var lastClient = -1;
  var target = container || null;
  page._openPinToken = token;
  cancelPinToLatestOnOpenJob(page);
  var schedule = function() {
    if (Number(page._openPinToken || 0) !== token) return;
    if (typeof requestAnimationFrame === 'function') {
      page._openPinRaf = requestAnimationFrame(tick);
    } else {
      page._openPinTimer = setTimeout(tick, 16);
    }
  };
  var tick = function() {
    if (Number(page._openPinToken || 0) !== token) return;
    page._openPinRaf = 0;
    page._openPinTimer = 0;
    var el = typeof page.resolveMessagesScroller === 'function'
      ? page.resolveMessagesScroller(target)
      : null;
    if (el) {
      var scrollHeight = Math.max(0, Number(el.scrollHeight || 0));
      var clientHeight = Math.max(0, Number(el.clientHeight || 0));
      var targetTop = resolveLatestMessageScrollTop(page, el);
      el.scrollTop = targetTop;
      if (typeof page.syncGridBackgroundOffset === 'function') page.syncGridBackgroundOffset(el);
      page.showScrollDown = false;
      if (typeof page.syncMapSelectionToScroll === 'function') page.syncMapSelectionToScroll(el);
      if (typeof page.scheduleMessageRenderWindowUpdate === 'function') page.scheduleMessageRenderWindowUpdate(el);
      var top = Math.round(Number(el.scrollTop || 0));
      var height = Math.round(scrollHeight);
      var client = Math.round(clientHeight);
      var nearBottom = Math.abs(top - targetTop) <= 2 || height <= (client + 2);
      if (nearBottom && top === lastTop && height === lastHeight && client === lastClient) {
        stable += 1;
      } else if (nearBottom) {
        stable = 1;
      } else {
        stable = 0;
