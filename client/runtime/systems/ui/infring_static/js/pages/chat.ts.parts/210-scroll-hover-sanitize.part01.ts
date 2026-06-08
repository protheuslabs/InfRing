        }
      }
      if (handedOffToRecovery) return;
      this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
      this._responseStartedAt = 0;
      this.sending = false;
      this.scrollToBottom();
      var self = this;
      this.$nextTick(function() {
        var el = document.getElementById('msg-input'); if (el) el.focus();
        self._processQueue();
      });
    },
    stopAgent: function() {
      if (!this.currentAgent) return;
      var self = this;
      InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(this.currentAgent.id) + '/stop', {}).then(function(res) {
        self.handleStopResponse(self.currentAgent && self.currentAgent.id ? self.currentAgent.id : '', res || {});
      }).catch(function(e) {
        var raw = String(e && e.message ? e.message : 'stop_failed');
        var lower = raw.toLowerCase();
        if (lower.indexOf('agent_inactive') >= 0 || lower.indexOf('inactive') >= 0) {
          self.handleAgentInactive(
            self.currentAgent && self.currentAgent.id ? self.currentAgent.id : '',
            'inactive',
            { noticeText: 'Agent is now inactive.' }
          );
          return;
        }
        if (lower.indexOf('agent_contract_terminated') >= 0 || lower.indexOf('contract terminated') >= 0) {
          self.handleAgentInactive(
            self.currentAgent && self.currentAgent.id ? self.currentAgent.id : '',
            'contract_terminated',
            { noticeText: 'Agent contract terminated.' }
          );
          return;
        }
        InfringToast.error('Stop failed: ' + raw);
      });
    },
    killAgent() {
      if (!this.currentAgent) return;
      var self = this;
      var name = this.currentAgent.name;
      InfringToast.confirm('Stop Agent', 'Stop agent "' + name + '"? The agent will be shut down.', async function() {
        try {
          self.setAgentLiveActivity(self.currentAgent && self.currentAgent.id, 'idle');
          await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(self.currentAgent.id) + '/archive', { reason: 'user_archive' });
          InfringAPI.wsDisconnect();
          self._wsAgent = null;
          self.currentAgent = null;
          self.setStoreActiveAgentId(null);
          self.messages = [];
          InfringToast.success('Agent "' + name + '" stopped');
          Alpine.store('app').refreshAgents();
        } catch(e) {
          InfringToast.error('Failed to stop agent: ' + e.message);
        }
      });
    },
    _latexTimer: null,
    resolveMessagesScroller: function(preferred) {
      var candidate = preferred || null;
      if (candidate && candidate.id === 'messages' && candidate.offsetParent !== null) return candidate;
      var refNode = this.$refs && this.$refs.messagesEl ? this.$refs.messagesEl : null;
      if (refNode && refNode.offsetParent !== null) return refNode;
      var nodes = document.querySelectorAll('#messages');
      for (var i = 0; i < nodes.length; i++) {
        var node = nodes[i];
        if (node && node.offsetParent !== null) return node;
      }
      return candidate && candidate.id === 'messages' ? candidate : null;
    },
    syncMapSelectionToScroll: function(container) {
      var el = this.resolveMessagesScroller(container);
      if (!el || !this.currentAgent || !Array.isArray(this.messages) || !this.messages.length) return;
      var nodes = el.querySelectorAll('.chat-message-block[id^="chat-msg-"]');
      if (!nodes || !nodes.length) return;
      var viewport = el.getBoundingClientRect();
      var viewportCenterY = viewport.top + (viewport.height / 2);
      var bestNode = null;
      var bestDiff = Number.POSITIVE_INFINITY;
      for (var i = 0; i < nodes.length; i++) {
        var node = nodes[i];
        if (!node || node.offsetParent === null) continue;
        var rect = node.getBoundingClientRect();
        if (rect.height <= 0) continue;
        if (rect.bottom < viewport.top || rect.top > viewport.bottom) continue;
        var nodeCenter = rect.top + (rect.height / 2);
        var diff = Math.abs(nodeCenter - viewportCenterY);
        if (diff < bestDiff) {
          bestDiff = diff;
          bestNode = node;
        }
      }
      if (!bestNode || !bestNode.id) return;
      var domId = String(bestNode.id);
      if (this.selectedMessageDomId !== domId) this.selectedMessageDomId = domId;
      var popup = typeof this.activeDashboardPopupOrigin === 'function'
        ? (this.activeDashboardPopupOrigin() || {})
        : {};
      if (String(popup.source || '').trim() !== 'chat-map') this.hoveredMessageDomId = domId;
      for (var idx = 0; idx < this.messages.length; idx++) {
        if (this.messageDomId(this.messages[idx], idx) === domId) { this.mapStepIndex = idx; break; }
      }
      var chatStore = window.InfringChatStore;
      if (chatStore && typeof chatStore.setThreadProjectionCenter === 'function') {
        chatStore.setThreadProjectionCenter(this.mapStepIndex);
      }
      this.centerChatMapOnMessage(domId, { immediate: true });
    },

    scrollToBottom(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var self = this;
      self.$nextTick(function() {
        if (opts.buttonAnimated) {
          self.scrollToBottomFromButton(opts);
          if (opts.stabilize) self.stabilizeBottomScroll();
          return;
        }
        self.scrollToBottomImmediate(opts);
        if (opts.stabilize) self.stabilizeBottomScroll();
      });
    },

    scrollToBottomFromButton(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var el = this.resolveMessagesScroller(opts.container || null);
      if (!el) return;
      var force = opts.force !== false;
      if (!force && !this._stickToBottom && !isNearLatestMessageBottom(this, el, opts.tolerancePx)) return;
      var startTop = Number(el.scrollTop || 0);
      var targetTop = resolveLatestMessageScrollTop(this, el);
      if (!(targetTop > startTop + 1)) {
        this.scrollToBottomImmediate({ container: el, force: true });
        return;
      }
      if (this._scrollToBottomButtonRaf) {
        try { cancelAnimationFrame(this._scrollToBottomButtonRaf); } catch (_) {}
        this._scrollToBottomButtonRaf = 0;
      }
      this._stickToBottom = true;
      this.showScrollDown = false;
      var self = this;
      var duration = 1000;
      var startedAt = 0;
      var easeOut = function(t) {
        var x = Math.max(0, Math.min(1, Number(t || 0)));
        return 1 - Math.pow(1 - x, 3);
      };
      var step = function(ts) {
        if (!startedAt) startedAt = Number(ts || 0);
        var elapsed = Math.max(0, Number(ts || 0) - startedAt);
        var progress = Math.max(0, Math.min(1, elapsed / duration));
        var eased = easeOut(progress);
        var top = startTop + ((targetTop - startTop) * eased);
        el.scrollTop = top;
        self.syncGridBackgroundOffset(el);
        if (progress < 1) {
          self._scrollToBottomButtonRaf = requestAnimationFrame(step);
          return;
        }
        self._scrollToBottomButtonRaf = 0;
        // Preserve current "blink" completion semantics, but only after the
        // staged 1s glide has completed.
        self.scrollToBottomImmediate({ container: el, force: true });
      };
      this._scrollToBottomButtonRaf = requestAnimationFrame(step);
    },

    scrollToBottomImmediate(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var el = this.resolveMessagesScroller(opts.container || null);
      if (!el) return;
      var force = opts.force !== false;
      if (!force && !this._stickToBottom && !isNearLatestMessageBottom(this, el, opts.tolerancePx)) return;
      el.scrollTop = resolveLatestMessageScrollTop(this, el);
      this.syncGridBackgroundOffset(el);
      this.showScrollDown = false;
      this._stickToBottom = true;
      this.syncMapSelectionToScroll(el);
      this.scheduleMessageRenderWindowUpdate(el);
      if (this._latexTimer) clearTimeout(this._latexTimer);
      this._latexTimer = setTimeout(function() { renderLatex(el); }, 150);
    },

    stabilizeBottomScroll: function() {
      var self = this;
      var tries = 3;
      var tick = function() {
        var el = self.resolveMessagesScroller();
        if (!el) return;
        el.scrollTop = resolveLatestMessageScrollTop(self, el);
        self.syncGridBackgroundOffset(el);
        if (--tries > 0) {
          if (typeof requestAnimationFrame === 'function') requestAnimationFrame(tick);
          else setTimeout(tick, 16);
        }
      };
      if (typeof requestAnimationFrame === 'function') requestAnimationFrame(tick);
      else setTimeout(tick, 0);
    },
    cancelPinToLatestOnOpen: function() {
      cancelPinToLatestOnOpenJob(this);
    },
    pinToLatestOnOpen: function(container, options) {
      runPinToLatestOnOpenJob(this, container, options);
    },
    handleMessagesScroll(e) {
      var el = this.resolveMessagesScroller(e && e.target ? e.target : null);
      if (!el) return;
      this._lastMessagesScrollAt = Date.now();
      var targetTop = resolveLatestMessageScrollTop(this, el);
      scheduleBottomHardCapClamp(this, el, targetTop, 128);
      this.startAgentTrailLoop(el);
      this.syncGridBackgroundOffset(el);
      this.syncDirectHoverAfterScroll(el);
      var hiddenBottom = Math.max(0, targetTop - Number(el.scrollTop || 0));
      this._stickToBottom = hiddenBottom <= resolveBottomFollowTolerancePx(this);
      this.showScrollDown = hiddenBottom > 120;
      var self = this;
      if (typeof requestAnimationFrame === 'function') {
        if (this._scrollSyncFrame) cancelAnimationFrame(this._scrollSyncFrame);
        this._scrollSyncFrame = requestAnimationFrame(function() {
          self._scrollSyncFrame = 0;
          self.syncMapSelectionToScroll(el);
        });
      } else {
        self.syncMapSelectionToScroll(el);
      }
      this.scheduleMessageRenderWindowUpdate(el);
      if (Number(el.scrollTop || 0) === 0 && this._hasMoreMessages && !this._olderMessagesLoading) {
        this.loadOlderMessages();
      }
    },
    resolveHoveredMessageDomIdFromPoint(container, clientX, clientY) {
      var host = this.resolveMessagesScroller(container || null);
      if (!host) return '';
      var x = Number(clientX || 0);
      var y = Number(clientY || 0);
      if (!(x > 0 && y > 0)) return '';
      var currentId = String(this.directHoveredMessageDomId || '').trim();
      var pickFromNode = function(node) {
        if (!node || typeof node.closest !== 'function') return '';
        var blockEl = node.closest('.chat-message-block[id^="chat-msg-"]');
        if (blockEl && host.contains(blockEl)) return String(blockEl.id || '').trim();
        var messageEl = node.closest('.message[id^="chat-msg-"]');
        if (messageEl && host.contains(messageEl)) return String(messageEl.id || '').trim();
        return '';
      };
      var candidateId = '';
      try {
        candidateId = pickFromNode(document.elementFromPoint(x, y));
      } catch (_) {
        candidateId = '';
      }
      if (!candidateId && typeof document.elementsFromPoint === 'function') {
        try {
          var stack = document.elementsFromPoint(x, y) || [];
          for (var i = 0; i < stack.length; i++) {
            candidateId = pickFromNode(stack[i]);
            if (candidateId) break;
          }
        } catch (_) {
          candidateId = '';
        }
      }
      if (candidateId && currentId && candidateId !== currentId) {
        var candidateEl = document.getElementById(candidateId);
        if (candidateEl) {
          var cRect = candidateEl.getBoundingClientRect();
          // Require pointer to move slightly inside the new row to avoid
          // boundary thrash on the split line between adjacent messages.
          if (y <= (cRect.top + 2) || y >= (cRect.bottom - 2)) {
            return currentId;
          }
        }
      }
      if (!candidateId && currentId) {
        var stickyEl = document.getElementById(currentId);
        if (stickyEl && host.contains(stickyEl)) {
          var sRect = stickyEl.getBoundingClientRect();
          var inStickyBand =
            x >= (sRect.left - 2) &&
            x <= (sRect.right + 2) &&
            y >= (sRect.top - 2) &&
            y <= (sRect.bottom + 2);
          if (inStickyBand) return currentId;
        }
      }
      return candidateId;
    },

    syncDirectHoverFromPointer(event) {
      if (!event || !event.currentTarget) return;
      this._lastPointerClientX = Number(event.clientX || 0);
      this._lastPointerClientY = Number(event.clientY || 0);
      var host = this.resolveMessagesScroller(event.currentTarget);
      if (!host) return;
      var domId = this.resolveHoveredMessageDomIdFromPoint(
        host,
        this._lastPointerClientX,
        this._lastPointerClientY
      );
      if (domId) {
        if (this._hoverClearTimer) {
          clearTimeout(this._hoverClearTimer);
          this._hoverClearTimer = 0;
        }
        this.directHoveredMessageDomId = domId;
        this.hoveredMessageDomId = domId;
        return;
      }
    },

    syncDirectHoverAfterScroll(container) {
      var host = this.resolveMessagesScroller(container || null);
      if (!host) return;
      var px = Number(this._lastPointerClientX || 0);
      var py = Number(this._lastPointerClientY || 0);
      if (!(px > 0 && py > 0)) {
        this.directHoveredMessageDomId = '';
        this.hoveredMessageDomId = this.selectedMessageDomId || '';
        return;
      }
      var domId = this.resolveHoveredMessageDomIdFromPoint(host, px, py);
      if (!domId) {
        this.directHoveredMessageDomId = '';
        this.hoveredMessageDomId = this.selectedMessageDomId || '';
        return;
      }
      this.directHoveredMessageDomId = domId;
      this.hoveredMessageDomId = domId;
    },

    currentInputToggleMode() {
      if (this.attachPickerSessionActive) return 'attach';
      return this.recording ? 'voice' : 'send';
    },

    beginAttachPickerSession() {
      if (typeof this.isSystemThreadActive === 'function' && this.isSystemThreadActive()) return;
      if (this.terminalMode) this.toggleTerminalMode();
      this.attachPickerRestoreMode = this.recording ? 'voice' : 'send';
      this.attachPickerSessionActive = true;
      this.showAttachMenu = false;
      this.armAttachPickerFocusTracking();
      var self = this;
      this.$nextTick(function() {
        var input = self.$refs && self.$refs.fileInput ? self.$refs.fileInput : null;
        if (!input || typeof input.click !== 'function') {
          self.endAttachPickerSession();
          return;
        }
        try {
          input.click();
        } catch (_) {
          self.endAttachPickerSession();
        }
      });
    },

    armAttachPickerFocusTracking() {
      var self = this;
      if (this._attachPickerFocusTimer) {
        clearTimeout(this._attachPickerFocusTimer);
        this._attachPickerFocusTimer = 0;
      }
      if (this._attachPickerFocusListener) {
        window.removeEventListener('focus', this._attachPickerFocusListener);
        this._attachPickerFocusListener = null;
      }
      this._attachPickerFocusListener = function() {
        if (self._attachPickerFocusTimer) clearTimeout(self._attachPickerFocusTimer);
        self._attachPickerFocusTimer = setTimeout(function() {
          self._attachPickerFocusTimer = 0;
          if (self.attachPickerSessionActive) self.endAttachPickerSession();
        }, 180);
      };
      window.addEventListener('focus', this._attachPickerFocusListener, { once: true });
    },

    endAttachPickerSession() {
      this.attachPickerSessionActive = false;
      this.showAttachMenu = false;
      if (this._attachPickerFocusTimer) {
        clearTimeout(this._attachPickerFocusTimer);
        this._attachPickerFocusTimer = 0;
      }
      if (this._attachPickerFocusListener) {
        window.removeEventListener('focus', this._attachPickerFocusListener);
        this._attachPickerFocusListener = null;
      }
    },

    handleAttachInputChange(event) {
      var input = event && event.target ? event.target : null;
      var files = input && input.files ? input.files : null;
      if (files && files.length) this.addFiles(files);
      if (input) input.value = '';
      this.endAttachPickerSession();
    },

    handleComposerPaste(event) {
      if (this.terminalMode) return false;
      if (!event || !event.clipboardData || typeof event.clipboardData.getData !== 'function') return false;
      var text = String(event.clipboardData.getData('text/plain') || '');
      if (!text || !this.shouldConvertLargePasteToAttachment || !this.shouldConvertLargePasteToAttachment(text)) return false;
      var attachment = this.buildLargePasteTextAttachment && this.buildLargePasteTextAttachment(text);
      if (!attachment || !attachment.file) return false;
      if (event && typeof event.preventDefault === 'function') event.preventDefault();
      if (!Array.isArray(this.attachments)) this.attachments = [];
      this.attachments.push(attachment);
      if (typeof InfringToast !== 'undefined' && InfringToast && typeof InfringToast.info === 'function') {
        InfringToast.info('Large paste moved to pastedtext.txt');
      }
      return true;
    },

    addFiles(files) {
      var self = this;
      var acceptedMimeTypes = [
        'image/png',
        'image/jpeg',
        'image/gif',
        'image/webp',
        'text/plain',
        'application/pdf',
        'text/markdown',
        'application/json',
        'text/csv'
      ];
      var acceptedExtensions = ['.txt', '.pdf', '.md', '.json', '.csv'];
      var existingKeys = {};
      var rows = Array.isArray(this.attachments) ? this.attachments : [];
      var attachmentKeyFor = function(file) {
        if (!file) return '';
        return [
          String(file.name || '').trim().toLowerCase(),
          Number(file.size || 0),
          Number(file.lastModified || 0)
        ].join('|');
      };
      var isSupportedMimeType = function(mimeType) {
        if (typeof mimeType !== 'string') return false;
        if (mimeType.indexOf('image/') === 0) return true;
        return acceptedMimeTypes.indexOf(mimeType) !== -1;
      };
      var isSupportedFile = function(file) {
        if (!file) return false;
        if (isSupportedMimeType(file.type)) return true;
        var ext = file.name.lastIndexOf('.') !== -1
          ? file.name.substring(file.name.lastIndexOf('.')).toLowerCase()
          : '';
        return acceptedExtensions.indexOf(ext) !== -1;
      };
      for (var existingIdx = 0; existingIdx < rows.length; existingIdx++) {
        var existing = rows[existingIdx];
        if (!existing || !existing.file) continue;
        var existingKey = attachmentKeyFor(existing.file);
        if (existingKey) existingKeys[existingKey] = true;
      }
      for (var i = 0; i < files.length; i++) {
        var file = files[i];
        var dedupeKey = attachmentKeyFor(file);
        if (dedupeKey && existingKeys[dedupeKey]) {
          InfringToast.info('Already attached: ' + file.name);
          continue;
        }
        if (file.size > 10 * 1024 * 1024) {
          InfringToast.warn('File "' + file.name + '" exceeds 10MB limit');
          continue;
        }
        var typeOk = isSupportedFile(file);
        if (!typeOk) {
          InfringToast.warn('File type not supported: ' + file.name);
          continue;
        }
        var preview = null;
        if (isSupportedMimeType(file.type) && file.type.indexOf('image/') === 0) {
          preview = URL.createObjectURL(file);
        }
        self.attachments.push({ file: file, preview: preview, uploading: false });
        if (dedupeKey) existingKeys[dedupeKey] = true;
      }
    },
    removeAttachment(idx) {
      var att = this.attachments[idx];
      if (att && att.preview) URL.revokeObjectURL(att.preview);
      this.attachments.splice(idx, 1);
    },
    handleDrop(e) {
      e.preventDefault();
      if (e.dataTransfer && e.dataTransfer.files && e.dataTransfer.files.length) {
        this.addFiles(e.dataTransfer.files);
      }
    },
    showMessageTitle(msg, idx, rows) {
      if (!msg || msg.is_notice) return false;
      if (msg.terminal) return this.isFirstInSourceRun(idx, rows);
      var role = String(msg.role || '').toLowerCase();
      if (role !== 'agent' && role !== 'system' && role !== 'user') return false;
      return this.isFirstInSourceRun(idx, rows);
    },
    messageMetaVisible(msg, idx, rows) {
      var service = typeof this.messageMetadataService === 'function' ? this.messageMetadataService() : null;
      if (service && typeof service.visible === 'function') {
        return service.visible(msg, this.isMessageMetaCollapsed(msg, idx, rows));
      }
      return !!(msg && !msg.is_notice && !msg.thinking && !this.isMessageMetaCollapsed(msg, idx, rows));
    },
    isMessageMetaCollapsed(msg, idx, rows) {
      if (!msg || msg.is_notice || msg.thinking) return true;
      return !this.isDirectHoveredMessage(msg, idx);
    },
    isGrouped(idx, rows) {
      var list = Array.isArray(rows) ? rows : this.messages;
      if (!Array.isArray(list) || idx <= 0 || idx >= list.length) return false;
      var prev = list[idx - 1];
      var curr = list[idx];
      if (!prev || !curr || prev.is_notice || curr.is_notice) return false;
      if (curr.thinking || prev.thinking) return false;
      return !this.isFirstInSourceRun(idx, list);
    },
    messageHasTailBlockingBox(msg) {
      if (!msg || typeof msg !== 'object') return false;
      if (this.messageHasTools(msg)) return true;
      if (msg.file_output && msg.file_output.path) return true;
      if (msg.folder_output && msg.folder_output.path) return true;
      if (this.messageProgress(msg)) return true;
      return false;
    },
    showMessageTail(msg, idx, rows) {
      if (!msg || msg.is_notice) return false;
      var role = this.messageGroupRole(msg);
      if (role !== 'user' && role !== 'agent' && role !== 'system') return false;
      // Tail only shows when this bubble is the terminal visible item in its source run.
      var list = Array.isArray(rows) ? rows : this.messages;
      if (!Array.isArray(list) || idx < 0 || idx >= list.length) return true;
      return this.isLastInSourceRun(idx, list);
    },

    sanitizeToolText: function(text) {
      if (!text) return text;
      text = text.replace(/<function=[^>]+>[\s\S]*?<\/function>/gi, '');
      text = text.replace(/<\/?function[^>]*>/gi, '');
      text = text.replace(/<cache_control[^>]*\/>/gi, '');
      text = text.replace(/<cache_control[^>]*>[\s\S]*?<\/cache_control>/gi, '');
      text = text.replace(/<\/?cache_control[^>]*>/gi, '');
      text = text
        .split('\n')
        .filter(function(line) {
          var lowered = String(line || '').toLowerCase();
          return !(lowered.includes('stable_hash=') && (lowered.includes('cache_control') || lowered.includes('cache control')));
        })
        .join('\n');
      text = text.replace(/\s*\w+<\/function[=,]?\s*\{[\s\S]*$/gmi, '');
      text = text.replace(/\s*<function=[^>]*>\s*\{[\s\S]*$/gmi, '');
      text = text.replace(/\s*\w+\{"type"\s*:\s*"function"[\s\S]*$/gmi, '');
      text = text.replace(/<\|[\w_:-]+\|>/g, '');
      text = text.replace(/\n{3,}/g, '\n\n');
      return text.trim();
    },
    collectStreamedAssistantEnvelope: function() {
      var streamedText = '';
      var streamedTools = [];
      var streamedThought = '';
      var appendThought = function(value) {
        var clean = String(value || '').trim();
        if (!clean) return;
        if (streamedThought) streamedThought += '\n';
        streamedThought += clean;
      };
      var rows = Array.isArray(this.messages) ? this.messages : [];
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i];
        if (!row || row.role !== 'agent' || (!row.streaming && !row.thinking)) continue;
        if (!row.thinking) {
          streamedText += (typeof row._cleanText === 'string') ? row._cleanText : (row.text || '');
        }
        if (row._thoughtText) appendThought(row._thoughtText);
        if (row._reasoning) appendThought(row._reasoning);
        if (row.thinking && row.text) {
          var pendingThought = String(row.text || '').replace(/^\*+|\*+$/g, '').trim();
          if (pendingThought && pendingThought.toLowerCase() !== 'thinking...') appendThought(pendingThought);
