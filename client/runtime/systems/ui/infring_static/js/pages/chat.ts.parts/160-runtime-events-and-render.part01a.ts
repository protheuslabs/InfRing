
    thinkingDisplayText: function(msg) {
      var rawThought = String(msg && msg._thoughtText ? msg._thoughtText : '').trim();
      if (!rawThought) return '';
      if (rawThought) {
        var latestComplete = typeof this.nextThoughtSentenceFrame === 'function'
          ? String(this.nextThoughtSentenceFrame(msg, rawThought) || '').trim()
          : '';
        if (!latestComplete && typeof this.latestCompleteSentence === 'function') {
          latestComplete = String(this.latestCompleteSentence(rawThought) || '').trim();
        }
        if (latestComplete) {
          if (msg && typeof msg === 'object') msg._thought_last_complete_sentence = latestComplete;
          return latestComplete;
        }
        var sticky = String(msg && msg._thought_last_complete_sentence ? msg._thought_last_complete_sentence : '').trim();
        if (sticky) return sticky;
        return '';
      }
      return '';
    },

    thinkingToolStatusSummary: function(msg) {
      var summary = { text: '', hasRunning: false };
      if (!msg || !Array.isArray(msg.tools) || !msg.tools.length) return summary;
      var runningNames = [];
      var completed = 0;
      var errors = 0;
      var blocked = 0;
      var lastFinishedName = '';
      for (var ri = msg.tools.length - 1; ri >= 0; ri--) {
        var recent = msg.tools[ri];
        if (!recent || recent.running || this.isThoughtTool(recent)) continue;
        var recentName = this.toolDisplayName(recent);
        if (recentName) { lastFinishedName = recentName; break; }
      }
      for (var i = 0; i < msg.tools.length; i++) {
        var tool = msg.tools[i];
        if (!tool || this.isThoughtTool(tool)) continue;
        if (tool.running) {
          var runningName = typeof this.toolThinkingActionLabel === 'function'
            ? this.toolThinkingActionLabel(tool)
            : this.toolDisplayName(tool);
          if (runningName) runningNames.push(runningName);
          continue;
        }
        if (this.isBlockedTool(tool)) {
          blocked += 1;
          continue;
        }
        if (tool.is_error) {
          errors += 1;
          continue;
        }
        completed += 1;
      }
      summary.hasRunning = runningNames.length > 0;
      var doneCount = completed + errors + blocked;
      if (summary.hasRunning) {
        summary.text = runningNames.length === 1
          ? (runningNames[0] + '...')
          : ('Running ' + runningNames.length + ' tools...');
        var runningBits = [];
        if (doneCount > 0) runningBits.push(doneCount + ' done');
        if (errors > 0) runningBits.push(errors + ' error');
        if (blocked > 0) runningBits.push(blocked + ' blocked');
        if (runningBits.length) summary.text += ' · ' + runningBits.join(', ');
        return summary;
      }
      if (!doneCount) return summary;
      summary.text = lastFinishedName || 'Tool steps complete';
      var doneBits = [];
      if (errors > 0) doneBits.push(errors + ' error');
      if (blocked > 0) doneBits.push(blocked + ' blocked');
      if (doneBits.length) summary.text += ' · ' + doneBits.join(', ');
      return summary;
    },

    thinkingStatusText: function(msg) {
      if (!msg || !msg.thinking) return '';
      var toolDialog = typeof this.currentToolDialogLabel === 'function'
        ? String(this.currentToolDialogLabel(msg) || '').trim()
        : '';
      if (typeof this.normalizeThinkingStatusCandidate === 'function') {
        toolDialog = this.normalizeThinkingStatusCandidate(toolDialog);
      }
      if (toolDialog) {
        return toolDialog;
      }
      var thoughtLine = typeof this.thinkingDisplayText === 'function'
        ? String(this.thinkingDisplayText(msg) || '').trim()
        : '';
      if (typeof this.normalizeThinkingStatusCandidate === 'function') {
        thoughtLine = this.normalizeThinkingStatusCandidate(thoughtLine);
      }
      if (thoughtLine) {
        return thoughtLine;
      }
      var status = typeof this.normalizeThinkingStatusCandidate === 'function'
        ? this.normalizeThinkingStatusCandidate(msg.thinking_status || msg.status_text || '')
        : String(msg.thinking_status || msg.status_text || '').trim();
      if (status) return status;
      return 'Thinking';
    },
