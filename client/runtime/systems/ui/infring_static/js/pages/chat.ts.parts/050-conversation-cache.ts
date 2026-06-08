      } catch {
        return false;
      }
    },

    loadConversationCache() {
      try {
        var cacheVersion = localStorage.getItem(this.conversationCacheVersionKey);
        if (cacheVersion !== this.conversationCacheVersion) {
          localStorage.removeItem(this.conversationCacheKey);
          localStorage.setItem(this.conversationCacheVersionKey, this.conversationCacheVersion);
          return {};
        }
        var raw = localStorage.getItem(this.conversationCacheKey);
        if (!raw) return {};
        var parsed = JSON.parse(raw);
        if (!parsed || typeof parsed !== 'object') return {};
        return parsed;
      } catch {
        return {};
      }
    },

    persistConversationCache() {
      try {
        localStorage.setItem(this.conversationCacheVersionKey, this.conversationCacheVersion);
        var sanitized = this.sanitizeConversationCacheForPersistence
          ? this.sanitizeConversationCacheForPersistence(this.conversationCache || {})
          : (this.conversationCache || {});
        this.conversationCache = sanitized;
        localStorage.setItem(this.conversationCacheKey, JSON.stringify(sanitized));
      } catch {}
    },

    sessionNoticeMemoryStorageKey(scopeKey) {
      var normalized = String(scopeKey || '').trim();
      if (!normalized) return '';
      return 'of-chat-session-notices-v1:' + normalized;
    },

    loadSessionNoticeMemory(scopeKey) {
      var storageKey = this.sessionNoticeMemoryStorageKey(scopeKey);
      if (!storageKey) return {};
      try {
        var raw = localStorage.getItem(storageKey);
        if (!raw) return {};
        var parsed = JSON.parse(raw);
        if (!Array.isArray(parsed)) return {};
        var next = {};
        for (var i = 0; i < parsed.length; i++) {
          var key = String(parsed[i] || '').trim();
          if (key) next[key] = true;
        }
        return next;
      } catch (_) {
        return {};
      }
    },

    saveSessionNoticeMemory(scopeKey, nextMemory) {
      var storageKey = this.sessionNoticeMemoryStorageKey(scopeKey);
      if (!storageKey) return;
      var rows = Object.keys(nextMemory || {}).filter(function(key) {
        return !!nextMemory[key];
      });
      try {
        if (!rows.length) {
          localStorage.removeItem(storageKey);
          return;
        }
        localStorage.setItem(storageKey, JSON.stringify(rows));
      } catch (_) {}
    },

    hasSeenSessionNotice(scopeKey, noticeKey) {
      var normalizedNoticeKey = String(noticeKey || '').trim();
      if (!normalizedNoticeKey) return false;
      var memory = this.loadSessionNoticeMemory(scopeKey);
      return memory[normalizedNoticeKey] === true;
    },

    markSeenSessionNotice(scopeKey, noticeKey) {
      var normalizedNoticeKey = String(noticeKey || '').trim();
      if (!normalizedNoticeKey) return;
      var memory = this.loadSessionNoticeMemory(scopeKey);
      memory[normalizedNoticeKey] = true;
      this.saveSessionNoticeMemory(scopeKey, memory);
    },

    clearSeenSessionNotice(scopeKey, noticeKey) {
      var normalizedNoticeKey = String(noticeKey || '').trim();
      if (!normalizedNoticeKey) return;
      var memory = this.loadSessionNoticeMemory(scopeKey);
      if (!memory[normalizedNoticeKey]) return;
      delete memory[normalizedNoticeKey];
      this.saveSessionNoticeMemory(scopeKey, memory);
    },

    estimateTokenCountFromText(text) {
      return Math.max(0, Math.round(String(text || '').length / 4));
    },

    // Backward-compat shim for legacy callers during naming migration.
    estimateTokensFromText(text) {
      return this.estimateTokenCountFromText(text);
    },

    shouldConvertLargePasteToAttachment(rawText) {
      if (!this.pasteToMarkdownEnabled) return false;
      var text = String(rawText == null ? '' : rawText);
      if (!text.trim()) return false;
      var chars = text.trim().length;
      var lines = text.split(/\r?\n/g).length;
      var charThreshold = Number(this.pasteToMarkdownCharThreshold || 2000);
      var lineThreshold = Number(this.pasteToMarkdownLineThreshold || 40);
      if (!Number.isFinite(charThreshold) || charThreshold < 256) charThreshold = 2000;
      if (!Number.isFinite(lineThreshold) || lineThreshold < 8) lineThreshold = 40;
      return chars >= charThreshold || lines >= lineThreshold;
    },

    buildLargePasteTextAttachment(rawText) {
      if (typeof File !== 'function') return null;
      var text = String(rawText == null ? '' : rawText);
      if (!text.trim()) return null;
      var normalized = text.replace(/\r\n?/g, '\n');
      try {
        var file = new File([normalized], 'pastedtext.txt', {
          type: 'text/plain;charset=utf-8',
          lastModified: Date.now()
        });
        return {
          file: file,
          preview: '',
          uploading: false,
          pasted_text: true,
          pasted_text_preview: normalized.slice(0, 12000),
          pasted_text_size: normalized.length
        };
      } catch (_) {
        return null;
      }
    },

    // Backward-compat shim for older send-path callers.
    buildLargePasteMarkdownAttachment(rawText) {
      return this.buildLargePasteTextAttachment(rawText);
    },

    recomputeContextEstimate() {
      var rows = Array.isArray(this.messages) ? this.messages : [];
      var total = 0;
      for (var i = 0; i < rows.length; i++) {
        total += this.estimateTokenCountFromText(rows[i] && rows[i].text ? rows[i].text : '');
      }
      this.contextApproxTokens = total;
      this.refreshContextPressure();
    },

    applyContextTelemetry(data) {
      if (!data || typeof data !== 'object') return;
      var payloadAgentId = String(data.agent_id || '').trim();
      var selectedAgentId = String(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '').trim();
      if (payloadAgentId && selectedAgentId && payloadAgentId !== selectedAgentId) {
        return;
      }
      var pool = data.context_pool && typeof data.context_pool === 'object' ? data.context_pool : null;
      var hasApproxField =
        Object.prototype.hasOwnProperty.call(data, 'context_tokens') ||
        Object.prototype.hasOwnProperty.call(data, 'context_used_tokens') ||
        Object.prototype.hasOwnProperty.call(data, 'context_total_tokens') ||
        (pool && Object.prototype.hasOwnProperty.call(pool, 'active_tokens')) ||
        (pool && Object.prototype.hasOwnProperty.call(pool, 'pool_tokens'));
      var approx = Number(
        data.context_tokens != null ? data.context_tokens :
        (data.context_used_tokens != null ? data.context_used_tokens :
        (data.context_total_tokens != null ? data.context_total_tokens :
        (pool && pool.active_tokens != null ? pool.active_tokens :
        (pool && pool.pool_tokens != null ? pool.pool_tokens : 0))))
      );
      if (hasApproxField && Number.isFinite(approx) && approx >= 0) {
        this.contextApproxTokens = Math.max(0, Math.round(approx));
      } else if (typeof data.message === 'string') {
        var tokenMatch = data.message.match(/~?\s*([0-9,]+)\s+tokens/i);
        if (tokenMatch && tokenMatch[1]) {
          var parsed = Number(String(tokenMatch[1]).replace(/,/g, ''));
          if (Number.isFinite(parsed) && parsed > 0) this.contextApproxTokens = parsed;
        }
      }
      var windowSize = Number(
        data.context_window != null ? data.context_window :
        (data.context_window_tokens != null ? data.context_window_tokens :
        (pool && pool.context_window != null ? pool.context_window : 0))
      );
      if (Number.isFinite(windowSize) && windowSize > 0) {
        this.contextWindow = windowSize;
      }
      var ratio = Number(
        data.context_ratio != null ? data.context_ratio :
        (pool && pool.context_ratio != null ? pool.context_ratio : 0)
      );
      if ((!Number.isFinite(approx) || approx <= 0) && Number.isFinite(ratio) && ratio > 0 && this.contextWindow > 0) {
        this.contextApproxTokens = Math.round(this.contextWindow * ratio);
      }
      var pressure = String(
        data.context_pressure != null ? data.context_pressure :
        (pool && pool.context_pressure != null ? pool.context_pressure : '')
      ).trim();
      if (pressure) {
        this.contextPressure = pressure;
      } else {
        this.refreshContextPressure();
      }
    },

    isAutoModelSelected() {
      return !!(
        this.currentAgent &&
        String(this.currentAgent.model_name || '').trim().toLowerCase() === 'auto'
      );
    },

    formatAutoRouteMeta(route) {
      if (!route || typeof route !== 'object') return '';
      var provider = String(route.provider || route.selected_provider || '').trim();
      var model = String(route.model || route.selected_model || route.selected_model_id || '').trim();
      if (!model) return '';
      var shortModel = model;
      if (shortModel.indexOf('/') >= 0) {
        shortModel = shortModel.split('/').slice(-1)[0];
      }
      var reason = String(route.reason || '').trim();
      if (reason.length > 80) reason = reason.slice(0, 77) + '...';
      var prefix = provider ? ('Auto -> ' + provider + '/' + shortModel) : ('Auto -> ' + shortModel);
      return reason ? (prefix + ' (' + reason + ')') : prefix;
    },

    normalizeAutoModelNoticeName(modelId) {
      var value = String(modelId || '').trim();
      if (!value) return '';
      if (value.indexOf('/') >= 0) {
        value = value.split('/').slice(-1)[0];
      }
      return value.replace(/-\d{8}$/, '');
    },

    formatAutoModelSwitchLabel(modelId) {
      var normalized = this.normalizeAutoModelNoticeName(modelId);
      if (!normalized && this.currentAgent) {
        normalized = this.normalizeAutoModelNoticeName(
          this.currentAgent.runtime_model || this.currentAgent.model_name || ''
        );
      }
      return 'Auto:[' + (normalized || 'unknown') + ']';
    },

    captureAutoModelSwitchBaseline() {
      return '';
    },

    maybeAddAutoModelSwitchNotice(previousLabel, route) {
      void previousLabel;
      void route;
    },

    applyAutoRouteTelemetry(data) {
      if (!data || typeof data !== 'object') return null;
      var route = null;
      if (data.auto_route && typeof data.auto_route === 'object') {
        route = data.auto_route;
      } else if (data.route && typeof data.route === 'object') {
        route = data.route;
      }
      if (!route) return null;
      return route;
    },

    async fetchAutoRoutePreflight(message, uploadedFiles) {
      void message;
      void uploadedFiles;
      return null;
    },

    inferContextWindowFromModelId(modelId) {
      var value = String(modelId || '').toLowerCase();
      if (!value) return 0;
      var explicitK = value.match(/(?:^|[^0-9])([0-9]{2,4})k(?:[^a-z0-9]|$)/);
      if (explicitK && explicitK[1]) {
        var parsedK = Number(explicitK[1]);
        if (Number.isFinite(parsedK) && parsedK > 0) return parsedK * 1000;
      }
      var explicitM = value.match(/(?:^|[^0-9])([0-9]{1,3})m(?:[^a-z0-9]|$)/);
      if (explicitM && explicitM[1]) {
        var parsedM = Number(explicitM[1]);
        if (Number.isFinite(parsedM) && parsedM > 0) return parsedM * 1000000;
      }
      if (value.indexOf('qwen2.5') >= 0 || value.indexOf('qwen3') >= 0) return 131072;
      if (value.indexOf('kimi') >= 0 || value.indexOf('moonshot') >= 0) return 262144;
      if (value.indexOf('llama-3.3') >= 0 || value.indexOf('llama3.3') >= 0) return 131072;
      if (value.indexOf('llama-3.2') >= 0 || value.indexOf('llama3.2') >= 0) return 128000;
      if (value.indexOf('mistral-nemo') >= 0 || value.indexOf('mixtral') >= 0) return 32000;
      return 0;
    },

    contextWindowNeedsFloor(modelId) {
      var value = String(modelId || '').toLowerCase();
      if (!value) return false;
      return value.indexOf('kimi') >= 0 || value.indexOf('moonshot') >= 0;
    },

    collectContextWindowCandidatesFromAgent(agent) {
      var row = agent && typeof agent === 'object' ? agent : {};
      var provider = String(row.model_provider || row.provider || '').trim().toLowerCase();
      var out = [];
      var seen = {};
      var push = function(value) {
        var key = String(value || '').trim();
        if (!key || seen[key]) return;
        seen[key] = true;
        out.push(key);
      };
      var modelName = String(row.model_name || '').trim();
      var runtimeModel = String(row.runtime_model || '').trim();
      push(modelName);
      push(runtimeModel);
      if (provider && modelName && modelName.indexOf('/') < 0) push(provider + '/' + modelName);
      if (provider && runtimeModel && runtimeModel.indexOf('/') < 0) push(provider + '/' + runtimeModel);
      if (modelName.indexOf('/') >= 0) push(modelName.split('/').slice(-1)[0]);
      if (runtimeModel.indexOf('/') >= 0) push(runtimeModel.split('/').slice(-1)[0]);
      return out;
    },

    resolveBestContextWindowFromMap(candidates) {
      var keys = Array.isArray(candidates) ? candidates : [];
      var map = this._contextWindowByModel || {};
      var best = 0;
      for (var i = 0; i < keys.length; i += 1) {
        var fromMap = Number(map[keys[i]] || 0);
        if (!Number.isFinite(fromMap) || fromMap <= 0) continue;
        if (fromMap > best) best = fromMap;
      }
      return best;
    },

    refreshContextWindowMap(models) {
      var next = {};
      var rows = Array.isArray(models) ? models : [];
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i] || {};
        var id = String(row.id || '').trim();
        if (!id) continue;
        var provider = String(row.provider || row.model_provider || '').trim().toLowerCase();
        var windowSize = Number(row.context_window || row.context_window_tokens || 0);
        if (!Number.isFinite(windowSize) || windowSize <= 0) {
          windowSize = this.inferContextWindowFromModelId(id);
        }
        if (Number.isFinite(windowSize) && windowSize > 0) {
          var normalized = Math.round(windowSize);
          var keys = [id];
          if (id.indexOf('/') >= 0) {
            keys.push(id.split('/').slice(-1)[0]);
          } else if (provider) {
            keys.push(provider + '/' + id);
          }
          for (var k = 0; k < keys.length; k += 1) {
            var key = String(keys[k] || '').trim();
            if (!key) continue;
            var prior = Number(next[key] || 0);
            if (!Number.isFinite(prior) || normalized > prior) next[key] = normalized;
          }
        }
      }
      this._contextWindowByModel = next;
    },

    setContextWindowFromCurrentAgent() {
      var agent = this.currentAgent || {};
      var direct = Number(agent.context_window || agent.context_window_tokens || 0);
      var candidates = this.collectContextWindowCandidatesFromAgent(agent);
      var fromMap = this.resolveBestContextWindowFromMap(candidates);
      var inferred = 0;
      var needsFloor = false;
      for (var i = 0; i < candidates.length; i += 1) {
        var key = String(candidates[i] || '').trim();
        if (!key) continue;
        if (this.contextWindowNeedsFloor(key)) needsFloor = true;
        var guess = Number(this.inferContextWindowFromModelId(key) || 0);
        if (Number.isFinite(guess) && guess > inferred) inferred = guess;
      }
      var best = 0;
      if (Number.isFinite(direct) && direct > 0) best = direct;
      if (Number.isFinite(fromMap) && fromMap > best) best = fromMap;
      if (Number.isFinite(inferred) && inferred > 0) {
        if (needsFloor) best = Math.max(best, inferred);
        else if (best <= 0) best = inferred;
      }
      if (!Number.isFinite(best) || best <= 0) best = 128000;
      this.contextWindow = Math.round(best);
      this.refreshContextPressure();
    },

    refreshContextPressure() {
      var windowSize = Number(this.contextWindow || 0);
      var used = Number(this.contextApproxTokens || 0);
      if (!Number.isFinite(windowSize) || windowSize <= 0 || !Number.isFinite(used) || used < 0) {
        this.contextPressure = 'low';
        return;
      }
      var ratio = used / windowSize;
      if (ratio >= 0.96) this.contextPressure = 'critical';
      else if (ratio >= 0.82) this.contextPressure = 'high';
      else if (ratio >= 0.55) this.contextPressure = 'medium';
      else this.contextPressure = 'low';
    },

    normalizePromptSuggestions(rows, contextText, disallowSamples) {
      var source = Array.isArray(rows) ? rows : [];
      var blocked = Array.isArray(disallowSamples) ? disallowSamples : [];
      var seen = {};
      var out = [];
      var contextKeywords = [];
      var wordCount = function(text) {
        return String(text == null ? '' : text).trim().split(/\s+/g).filter(Boolean).length;
      };
      var tokenize = function(value) {
        var stop = {
          can: true, could: true, would: true, should: true, what: true, why: true, how: true, when: true, where: true, who: true,
          the: true, this: true, that: true, with: true, from: true, into: true, your: true, you: true, and: true, for: true,
          then: true, now: true, again: true, please: true
        };
        return String(value == null ? '' : value)
          .toLowerCase()
          .replace(/[^a-z0-9_:-]+/g, ' ')
          .split(/\s+/g)
          .filter(function(token) { return !!(token && token.length >= 3 && !stop[token]); });
      };
      var tokenSimilarity = function(a, b) {
        var left = tokenize(a);
        var right = tokenize(b);
        if (!left.length && !right.length) return 1;
        if (!left.length || !right.length) return 0;
        var leftSet = {};
        var rightSet = {};
        var i;
        for (i = 0; i < left.length; i++) leftSet[left[i]] = true;
        for (i = 0; i < right.length; i++) rightSet[right[i]] = true;
        var overlap = 0;
        var union = {};
        Object.keys(leftSet).forEach(function(token) {
          union[token] = true;
          if (rightSet[token]) overlap += 1;
        });
        Object.keys(rightSet).forEach(function(token) { union[token] = true; });
        var unionSize = Object.keys(union).length || 1;
        return overlap / unionSize;
      };
      var isNearDuplicate = function(a, b) {
        var left = String(a == null ? '' : a).toLowerCase().trim();
        var right = String(b == null ? '' : b).toLowerCase().trim();
        if (!left || !right) return false;
        if (left === right) return true;
        if (left.indexOf(right) >= 0 || right.indexOf(left) >= 0) return true;
        return tokenSimilarity(left, right) >= 0.72;
      };
      var trimTrailingJoiners = function(text) {
        var words = String(text == null ? '' : text).trim().split(/\s+/g).filter(Boolean);
        while (words.length > 1) {
          var tail = String(words[words.length - 1] || '')
            .replace(/[^a-z0-9_-]+/gi, '')
            .toLowerCase();
          if (!tail || /^(and|or|to|with|for|from|via|then|than|versus|vs)$/i.test(tail)) {
            words.pop();
            continue;
          }
          break;
        }
        return words.join(' ');
      };
      var clampWords = function(text, maxWords) {
        var cap = Number(maxWords || 10);
        var words = String(text == null ? '' : text).trim().split(/\s+/g).filter(Boolean);
        if (!words.length) return '';
        if (!Number.isFinite(cap) || cap < 3) cap = 10;
        if (words.length > cap) words = words.slice(0, cap);
        return trimTrailingJoiners(words.join(' '));
      };
      var normalizeVoice = function(value) {
        var row = String(value == null ? '' : value)
          .replace(/\s+/g, ' ')
          .trim();
        if (!row) return '';
        row = row
          .replace(/^\s*[-*0-9.)\]]+\s*/, '')
          .replace(/^\s*\[[^\]\n]{2,96}\]\s*/, '')
          .replace(/^\s*(?:\*\*)?(?:agent|assistant|system|model|ai|jarvis|user|human)(?:\*\*)?\s*:\s*/i, '')
          .replace(/^ask\s+[^.?!]{0,140}?\s+to\s+/i, '')
          .replace(/^ask\s+[^.?!]{0,140}?\s+for\s+/i, '')
          .replace(/^ask\s+for\s+/i, '')
          .replace(/^request\s+/i, '')
          .replace(/^please\s+request\s+/i, '')
          .replace(/\s+/g, ' ')
          .trim();
        // Suggestions must read as user->agent prompts, not agent->user offers.
        row = row
          .replace(/^(?:do you want me to|would you like me to|do you want us to|would you like us to)\s+/i, '')
          .replace(/^(?:want me to|should i|should we)\s+/i, '')
          .replace(/^(?:can i|could i|can we|could we)\s+/i, '')
          .replace(/^(?:i can|i could|i will|i'll|we can|we could|we will|we'll)\s+/i, '')
          .replace(/^(?:let me|let us)\s+/i, '')
          .trim();
        row = clampWords(row, 10);
        row = row.replace(/[.!?]+$/g, '').trim();
        if (!row) return '';
        row = trimTrailingJoiners(row);
        if (!row) return '';
        row = row.replace(/\?+$/g, '').trim();
        if (row.length && /^[a-z]/.test(row.charAt(0))) {
          row = row.charAt(0).toUpperCase() + row.slice(1);
        }
        if (row.length > 180) row = row.substring(0, 177) + '...';
        return row;
      };
      var isLowValue = function(text) {
        var lowered = String(text || '').toLowerCase();
        if (!lowered) return true;
        if (lowered.indexOf('the infring runtime is currently') >= 0) return true;
        if (lowered.indexOf('if you need help') >= 0 || lowered.indexOf('feel free to ask') >= 0) return true;
        if (lowered.indexOf('the user wants exactly 3 actionable next user prompts') >= 0) return true;
        if (lowered.indexOf('json array of strings') >= 0) return true;
        if (lowered.indexOf('output only the') >= 0) return true;
        if (lowered.indexOf('do not include numbering') >= 0) return true;
        if (lowered.indexOf('highest-roi') >= 0) return true;
        if (lowered.indexOf('runbook') >= 0) return true;
        if (lowered.indexOf('reliability remediation') >= 0) return true;
        if (lowered.indexOf('rollback criteria') >= 0) return true;
        if (lowered.indexOf('3-step execution plan') >= 0) return true;
        if (/^(do you want me to|would you like me to|want me to|should i|should we)\b/i.test(lowered)) return true;
        if (lowered.indexOf('this task') >= 0) return true;
        if (lowered === 'thinking...' || lowered === 'thinking..' || lowered === 'thinking.') return true;
        var sentenceCount = (text.match(/[.!?]/g) || []).length;
        if (sentenceCount > 2) return true;
        if (/[\"“”]/.test(text) && text.length > 120) return true;
        if (/^(give me|request|ask for)\b/i.test(text)) return true;
        var words = wordCount(text);
        if (words < 3 || words > 10) return true;
        var actionableStart =
          /^(can|could|would|should|what|why|how|when|where|who|show|fix|check|run|retry|switch|clear|drain|scale|continue|compare|explain|validate|review|open|trace|summarize|draft|outline|tell|list)\b/i.test(text);
        if (!actionableStart && text.indexOf('?') < 0 && /^\s*(the|it|this|that)\b/i.test(text)) return true;
        return false;
      };
      var isGeneric = function(text) {
        var lowered = String(text || '').toLowerCase();
        if (!lowered) return true;
        return (
          lowered.indexOf('continue this and keep the same direction') >= 0 ||
          lowered.indexOf('best next move from here') >= 0 ||
          lowered.indexOf('summarize progress in three concrete bullets') >= 0 ||
          lowered.indexOf('show the first command to run now') >= 0 ||
          lowered.indexOf('turn that into a concrete checklist') >= 0 ||
          lowered.indexOf('take the next step on current task') >= 0 ||
          lowered.indexOf('respond to the latest update') >= 0
        );
      };
      var rawContext = String(contextText == null ? '' : contextText).toLowerCase();
      contextKeywords = rawContext
        .split(/[^a-z0-9_:-]+/g)
        .filter(function(token) {
          return !!(
            token &&
            token.length >= 4 &&
            ['this', 'that', 'with', 'from', 'into', 'your', 'have', 'will', 'when', 'where', 'what'].indexOf(token) === -1
          );
        })
        .slice(0, 10);
      for (var i = 0; i < source.length; i++) {
        var raw = normalizeVoice(source[i]);
        if (!raw || isLowValue(raw)) continue;
        var parrotsSample = blocked.some(function(sample) {
          var cleanSample = normalizeVoice(sample || '');
          if (!cleanSample) return false;
          return isNearDuplicate(raw, cleanSample);
        });
        if (parrotsSample) continue;
        if (isGeneric(raw) && contextKeywords.length) {
          var loweredRaw = String(raw || '').toLowerCase();
