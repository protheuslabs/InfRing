      var self = this;
      if (this._persistTimer) clearTimeout(this._persistTimer);
      this._persistTimer = setTimeout(function() {
        self.cacheCurrentConversation();
      }, 80);
    },

    countAvailableModelRows: function(rows) {
      var list = Array.isArray(rows) ? rows : [];
      var count = 0;
      for (var i = 0; i < list.length; i += 1) {
        var row = list[i] || {};
        if (row.available !== false) count += 1;
      }
      return count;
    },

    // Backward-compat shim for legacy callers during naming migration.
    availableModelRowsCount: function(rows) {
      return this.countAvailableModelRows(rows);
    },

    providerPayloadToModelCatalogRows: function(payload) {
      var providers = payload && Array.isArray(payload.providers) ? payload.providers : [];
      var out = [];
      for (var i = 0; i < providers.length; i += 1) {
        var providerRow = providers[i] && typeof providers[i] === 'object' ? providers[i] : {};
        var provider = String(providerRow.id || '').trim().toLowerCase();
        if (!provider || provider === 'auto') continue;
        var isLocal = providerRow.is_local === true;
        var reachable = providerRow.reachable === true;
        var supportsChat = providerRow.supports_chat !== false;
        var needsKey = providerRow.needs_key === true;
        var authStatus = String(providerRow.auth_status || '').trim().toLowerCase();
        var authConfigured = authStatus === 'configured' || authStatus === 'set' || authStatus === 'ok';
        var profiles = providerRow.model_profiles && typeof providerRow.model_profiles === 'object'
          ? providerRow.model_profiles
          : {};
        var names = Object.keys(profiles);
        for (var j = 0; j < names.length; j += 1) {
          var modelName = String(names[j] || '').trim();
          if (!modelName) continue;
          var modelRef = provider + '/' + modelName;
          if (this.isPlaceholderModelRef(modelRef)) continue;
          var profile = profiles[modelName] && typeof profiles[modelName] === 'object' ? profiles[modelName] : {};
          var deployment = String(profile.deployment_kind || '').trim().toLowerCase();
          var rowLocal = isLocal || deployment === 'local' || deployment === 'ollama';
          var available = supportsChat && (rowLocal ? reachable : (!needsKey || authConfigured || reachable));
          out.push({
            id: modelRef,
            provider: provider,
            model: modelName,
            model_name: modelName,
            runtime_model: modelName,
            display_name: String(profile.display_name || modelName).trim() || modelName,
            available: !!available,
            reachable: !!reachable,
            supports_chat: supportsChat,
            needs_key: !!needsKey,
            auth_status: authStatus || 'unknown',
            is_local: rowLocal,
            deployment_kind: deployment || (rowLocal ? 'local' : 'api'),
            context_window: Number(profile.context_window || profile.context_size || profile.context_tokens || 0) || 0,
            context_window_tokens: Number(profile.context_window || profile.context_size || profile.context_tokens || 0) || 0,
            power_rating: Number(profile.power_rating || 3) || 3,
            cost_rating: Number(profile.cost_rating || (rowLocal ? 1 : 3)) || (rowLocal ? 1 : 3),
            specialty: String(profile.specialty || 'general').trim().toLowerCase() || 'general',
            specialty_tags: Array.isArray(profile.specialty_tags) ? profile.specialty_tags : ['general'],
            param_count_billion: Number(profile.param_count_billion || 0) || 0,
            download_available: profile.download_available === true || rowLocal,
            local_download_path: String(profile.local_download_path || '').trim(),
            max_output_tokens: Number(profile.max_output_tokens || 0) || 0,
          });
        }
      }
      return out;
    },

    mergeModelCatalogRows: function(primaryRows, fallbackRows) {
      var merged = [];
      var seen = {};
      var add = function(row) {
        var id = String(row && row.id ? row.id : '').trim();
        if (!id) return;
        var key = id.toLowerCase();
        if (seen[key]) return;
        seen[key] = true;
        merged.push(row);
      };
      var primary = Array.isArray(primaryRows) ? primaryRows : [];
      var fallback = Array.isArray(fallbackRows) ? fallbackRows : [];
      for (var i = 0; i < primary.length; i += 1) add(primary[i]);
      for (var j = 0; j < fallback.length; j += 1) add(fallback[j]);
      return merged;
    },

    fallbackModelCatalogRows: function() {
      var seeds = [
              [
                      "openai",
                      "gpt-5.5",
                      "GPT-5.5"
              ],
              [
                      "openai",
                      "gpt-5.4",
                      "GPT-5.4"
              ],
              [
                      "openai",
                      "gpt-5.4-mini",
                      "GPT-5.4 Mini"
              ],
              [
                      "openai",
                      "gpt-5.4-nano",
                      "GPT-5.4 Nano"
              ],
              [
                      "openai",
                      "gpt-5.3-codex",
                      "GPT-5.3 Codex"
              ],
              [
                      "anthropic",
                      "claude-opus-4-8",
                      "Claude Opus 4.8"
              ],
              [
                      "anthropic",
                      "claude-sonnet-4-6",
                      "Claude Sonnet 4.6"
              ],
              [
                      "anthropic",
                      "claude-haiku-4-5-20251001",
                      "Claude Haiku 4.5"
              ],
              [
                      "google",
                      "gemini-3",
                      "Gemini 3"
              ],
              [
                      "deepseek",
                      "deepseek-chat",
                      "DeepSeek Chat"
              ],
              [
                      "deepseek",
                      "deepseek-reasoner",
                      "DeepSeek Reasoner"
              ],
              [
                      "ollama",
                      "qwen2.5:3b-instruct",
                      "Qwen 2.5 3B Instruct"
              ]
      ];
      return this.sanitizeModelCatalogRows(seeds.map(function(seed) {
        var provider = seed[0];
        var model = seed[1];
        return {
          id: provider + '/' + model,
          provider: provider,
          model: model,
          model_name: model,
          runtime_model: model,
          display_name: seed[2],
          available: true,
          shell_catalog_seed: true
        };
      }));
    },

    loadProviderModelCatalogSafely: function(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var self = this;
      var cachedRows = self.sanitizeModelCatalogRows(self._modelCache || self.modelPickerList || []);
      var useRows = function(rows) {
        var models = self.sanitizeModelCatalogRows(rows);
        self._modelCache = models;
        self._modelCacheTime = Date.now();
        self.modelPickerList = models;
        return models;
      };
      var timeoutMs = Number(opts.timeout_ms || 2000);
      var timeoutFallback = new Promise(function(resolve) {
        setTimeout(function() { resolve(null); }, timeoutMs > 0 ? timeoutMs : 2000);
      });
      return Promise.race([
        InfringAPI.get('/api/providers'),
        timeoutFallback
      ]).then(function(providersPayload) {
        if (!providersPayload) {
          return useRows(cachedRows.length ? cachedRows : self.fallbackModelCatalogRows());
        }
        var providerRows = self.sanitizeModelCatalogRows(
          self.providerPayloadToModelCatalogRows(providersPayload)
        );
        if (!providerRows.length) {
          return useRows(cachedRows.length ? cachedRows : self.fallbackModelCatalogRows());
        }
        var existingRows = opts.merge_existing === false
          ? []
          : cachedRows;
        var models = self.mergeModelCatalogRows(existingRows, providerRows);
        return useRows(models);
      }).catch(function() {
        return useRows(cachedRows.length ? cachedRows : self.fallbackModelCatalogRows());
      });
    },

    modelCatalogRows: function(rows) {
      var list = Array.isArray(rows) && rows.length
        ? rows
        : (
          Array.isArray(this.modelPickerList) && this.modelPickerList.length
            ? this.modelPickerList
            : (Array.isArray(this._modelCache) ? this._modelCache : [])
        );
      return this.sanitizeModelCatalogRows(list);
    },

    resolveModelCatalogOption: function(value, providerHint, rows) {
      var list = this.modelCatalogRows(rows);
      var raw = value && typeof value === 'object'
        ? String(value.id || value.model || value.model_name || value.runtime_model || '').trim()
        : String(value || '').trim();
      var provider = value && typeof value === 'object'
        ? String(value.provider || value.model_provider || providerHint || '').trim().toLowerCase()
        : String(providerHint || '').trim().toLowerCase();
      if (!raw || this.isPlaceholderModelRef(raw)) return null;

      var candidates = [];
      var seen = {};
      var addCandidate = function(candidate) {
        var next = String(candidate || '').trim();
        if (!next) return;
        var key = next.toLowerCase();
        if (seen[key]) return;
        seen[key] = true;
        candidates.push(next);
      };
      addCandidate(raw);
      if (provider && raw.indexOf('/') < 0) addCandidate(provider + '/' + raw);
      if (raw.indexOf('/') >= 0) addCandidate(raw.split('/').slice(-1)[0]);

      var fallbackMatches = [];
      for (var i = 0; i < list.length; i += 1) {
        var row = list[i] || {};
        var rowId = String(row.id || '').trim();
        var rowProvider = String(row.provider || row.model_provider || '').trim().toLowerCase();
        var rowModel = String(row.model || row.model_name || row.runtime_model || '').trim();
        var rowDisplay = String(row.display_name || '').trim();
        for (var j = 0; j < candidates.length; j += 1) {
          var candidate = candidates[j];
          var candidateLower = candidate.toLowerCase();
          if (rowId && rowId.toLowerCase() === candidateLower) return row;
          if (rowModel && rowModel.toLowerCase() === candidateLower) {
            if (!provider || rowProvider === provider) return row;
            fallbackMatches.push(row);
          }
          if (rowDisplay && rowDisplay.toLowerCase() === candidateLower) {
            if (!provider || rowProvider === provider) return row;
            fallbackMatches.push(row);
          }
        }
      }
      if (provider) {
        for (var k = 0; k < fallbackMatches.length; k += 1) {
          var fallback = fallbackMatches[k] || {};
          if (String(fallback.provider || fallback.model_provider || '').trim().toLowerCase() === provider) {
            return fallback;
          }
        }
      }
      return fallbackMatches.length ? fallbackMatches[0] : null;
    },

    resolveProviderScopedModelCatalogOption: function(providerValue, modelValue, rows) {
      var provider = String(providerValue || '').trim().toLowerCase();
      var list = this.modelCatalogRows(rows);
      if (!provider) return this.resolveModelCatalogOption(modelValue, '', list);
      var resolved = this.resolveModelCatalogOption(modelValue, provider, list);
      if (resolved && String(resolved.provider || resolved.model_provider || '').trim().toLowerCase() === provider) {
        return resolved;
      }
      var rawModel = String(modelValue || '').trim();
      var targetModel = rawModel.indexOf('/') >= 0 ? rawModel.split('/').slice(-1)[0] : rawModel;
      var matches = [];
      for (var i = 0; i < list.length; i += 1) {
        var row = list[i] || {};
        var rowProvider = String(row.provider || row.model_provider || '').trim().toLowerCase();
        if (rowProvider !== provider) continue;
        if (!targetModel) {
          matches.push(row);
          continue;
        }
        var rowModel = String(row.model || row.model_name || row.runtime_model || '').trim();
        var rowId = String(row.id || '').trim();
        var exactId = rowId && rowId.toLowerCase() === (provider + '/' + targetModel).toLowerCase();
        var exactModel = rowModel && rowModel.toLowerCase() === targetModel.toLowerCase();
        if (exactId || exactModel) return row;
        matches.push(row);
      }
      if (!matches.length) return resolved || null;
      for (var j = 0; j < matches.length; j += 1) {
        if (matches[j] && matches[j].available !== false) return matches[j];
      }
      return matches[0];
    },

    dedupeFallbackModelList: function(entries, options) {
      var list = Array.isArray(entries) ? entries : [];
      var opts = options && typeof options === 'object' ? options : {};
      var rows = this.modelCatalogRows(opts.rows);
      var primary = this.resolveModelCatalogOption(opts.primary_id || '', opts.primary_provider || '', rows);
      var primaryId = String(primary && primary.id ? primary.id : '').trim().toLowerCase();
      var out = [];
      var seen = {};
      for (var i = 0; i < list.length; i += 1) {
        var entry = list[i];
        var raw = entry && typeof entry === 'object' ? entry : { model: entry };
        var provider = String(raw.provider || raw.model_provider || '').trim();
        var model = String(raw.model || raw.model_name || raw.runtime_model || raw.id || '').trim();
        if (!model || this.isPlaceholderModelRef(model)) continue;
        var resolved = provider
          ? this.resolveProviderScopedModelCatalogOption(provider, model, rows)
          : this.resolveModelCatalogOption(model, '', rows);
        var normalizedProvider = String(
          (resolved && (resolved.provider || resolved.model_provider)) || provider || ''
        ).trim();
        var normalizedModel = String(
          (resolved && (resolved.model || resolved.model_name || resolved.runtime_model)) || model
        ).trim();
        var normalizedId = String(
          (resolved && resolved.id) ||
          (normalizedProvider && normalizedModel ? (normalizedProvider + '/' + normalizedModel) : normalizedModel)
        ).trim();
        if (!normalizedId || this.isPlaceholderModelRef(normalizedId)) continue;
        var dedupeKey = normalizedId.toLowerCase();
        if (primaryId && dedupeKey === primaryId) continue;
        if (seen[dedupeKey]) continue;
        seen[dedupeKey] = true;
        out.push({
          provider: normalizedProvider || String(provider || '').trim(),
          model: normalizedModel
        });
      }
      return out;
    },

    noModelsGuidanceText: function() {
      return [
        "I don't have any usable models yet.",
        '',
        'To enable models now:',
        '1. Install Ollama: https://ollama.com/download',
        '2. Start it: `ollama serve`',
        '3. Pull any model you choose from the Ollama library',
        '4. Or add an API key with `/apikey <key>`',
        '',
        'Useful links:',
        '- Ollama library: https://ollama.com/library',
        '- OpenRouter keys: https://openrouter.ai/keys',
        '- OpenAI keys: https://platform.openai.com/api-keys',
        '- Anthropic keys: https://console.anthropic.com/settings/keys'
      ].join('\n');
    },

    injectNoModelsGuidance: function(reason) {
      if (!this.currentAgent || (this.isSystemThreadAgent && this.isSystemThreadAgent(this.currentAgent))) {
        return null;
      }
      if (!this._noModelsGuidanceByAgent || typeof this._noModelsGuidanceByAgent !== 'object') {
        this._noModelsGuidanceByAgent = {};
      }
      var agentId = String((this.currentAgent && this.currentAgent.id) || '').trim();
      if (!agentId) return null;
      if (this._noModelsGuidanceByAgent[agentId]) return null;
      var text = this.noModelsGuidanceText();
      var row = {
        id: ++msgId,
        role: 'agent',
        text: text,
        meta: '',
        tools: [],
        ts: Date.now(),
        agent_id: agentId,
        agent_name: String((this.currentAgent && this.currentAgent.name) || 'Agent'),
        system_origin: 'models:no_models_available'
      };
      var pushed = this.pushAgentMessageDeduped(row, { dedupe_window_ms: 120000 }) || row;
      this._noModelsGuidanceByAgent[agentId] = {
        ts: Date.now(),
        reason: String(reason || ''),
        id: pushed && pushed.id ? pushed.id : row.id
      };
      this.scrollToBottom();
      this.scheduleConversationPersist();
      return pushed;
    },

    addNoModelsRecoveryNotice: function(reason, actionKind) {
      if (!this.currentAgent || (this.isSystemThreadAgent && this.isSystemThreadAgent(this.currentAgent))) {
        return null;
      }
      if (typeof this.addNoticeEvent !== 'function') return null;
      if (!this._noModelsRecoveryNoticeByAgent || typeof this._noModelsRecoveryNoticeByAgent !== 'object') {
        this._noModelsRecoveryNoticeByAgent = {};
      }
      var agentId = String((this.currentAgent && this.currentAgent.id) || '').trim();
      if (!agentId) return null;
      var now = Date.now();
      var prev = this._noModelsRecoveryNoticeByAgent[agentId];
      if (prev && Number(prev.ts || 0) > 0 && (now - Number(prev.ts || 0)) < 20000) {
        return null;
      }
      var desiredKind = String(actionKind || '').trim().toLowerCase();
      if (!desiredKind) desiredKind = 'model_discover';
      var action = null;
      if (desiredKind === 'open_url') {
        action = {
          kind: 'open_url',
          label: 'Install Ollama',
          url: 'https://ollama.com/download'
        };
      } else {
        action = {
          kind: 'model_discover',
          label: 'Discover models',
          reason: String(reason || 'chat_send_gate').trim()
        };
      }
      this.addNoticeEvent({
        notice_label: desiredKind === 'open_url'
          ? 'No runnable models detected. Install Ollama, then run model discovery.'
          : 'No runnable models detected. Discover models to unlock chat.',
        notice_type: 'warn',
        notice_icon: '\u26a0',
        notice_action: action,
        ts: now
      });
      this._noModelsRecoveryNoticeByAgent[agentId] = {
        ts: now,
        reason: String(reason || ''),
        action_kind: desiredKind
      };
      return true;
    },

    currentAvailableModelCount: function() {
      var rows = [];
      if (Array.isArray(this.modelPickerList) && this.modelPickerList.length) {
        rows = this.modelPickerList;
      } else if (Array.isArray(this._modelCache) && this._modelCache.length) {
        rows = this._modelCache;
      } else {
        rows = [];
      }
      rows = this.sanitizeModelCatalogRows(rows);
      return this.countAvailableModelRows(rows);
    },

    ensureUsableModelsForChatSend: async function(reason) {
      var available = this.currentAvailableModelCount();
      if (available > 0) return available;
      try {
        var models = await this.refreshModelCatalogAndGuidance({ discover: true, guidance: true });
        available = this.countAvailableModelRows(models);
      } catch (_) {
        available = this.currentAvailableModelCount();
      }
      if (available <= 0) {
        this.injectNoModelsGuidance(reason || 'chat_send_gate');
        this.addNoModelsRecoveryNotice(reason || 'chat_send_gate', 'model_discover');
      }
      return available;
    },

    refreshModelCatalogAndGuidance: async function(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var discoverFirst = opts.discover !== false;
      var includeGuidance = opts.guidance !== false;
      try {
        if (discoverFirst) {
          await InfringAPI.post('/api/shell-socket/models/discover', { input: '__auto__' }).catch(function() { return null; });
        }
        var data = await InfringAPI.get('/api/shell-socket/models');
        var models = this.sanitizeModelCatalogRows((data && data.models) || []);
        var available = this.countAvailableModelRows(models);
        // Recover from partial catalog responses by rebuilding rows from provider model_profiles.
        if (models.length < 8 || available < 4) {
          var providerFallbackRows = await this.loadProviderModelCatalogSafely({
            merge_existing: true
          }).catch(function() { return []; });
          if (providerFallbackRows.length) {
            models = this.mergeModelCatalogRows(models, providerFallbackRows);
            available = this.countAvailableModelRows(models);
          }
        }
        this._modelCache = models;
        this._modelCacheTime = Date.now();
        this.modelPickerList = models;
        if (includeGuidance && available === 0) {
          this.injectNoModelsGuidance('refresh');
        }
        return models;
      } catch (err) {
        if (includeGuidance && (!this.modelPickerList || !this.modelPickerList.length)) {
          this.injectNoModelsGuidance('refresh_error');
        }
        throw err;
      }
    },

    sanitizeConversationForCache(messages) {
      var source = Array.isArray(messages) ? messages : [];
      var out = [];
      var compactText = function(value, limit) {
        var max = Number(limit || 0);
        if (!Number.isFinite(max) || max < 1) max = 240;
        var text = '';
        if (value == null) {
          text = '';
        } else if (typeof value === 'string') {
          text = value;
        } else {
          try {
            text = JSON.stringify(value);
          } catch(_) {
            text = String(value || '');
          }
        }
        text = text.replace(/\s+/g, ' ').trim();
        if (text.length > max) text = text.slice(0, max - 1) + '…';
        return text;
      };
      var cleanRef = function(value) {
        var text = String(value || '').trim();
        return text.length > 300 ? text.slice(0, 300) : text;
      };
      var sanitizeTools = function(tools) {
        if (!Array.isArray(tools) || !tools.length) return [];
        var rows = [];
        for (var ti = 0; ti < tools.length && ti < 12; ti++) {
          var tool = tools[ti];
          if (!tool || typeof tool !== 'object') continue;
          rows.push({
            id: cleanRef(tool.id || tool.tool_id || tool.detail_ref || ''),
            name: compactText(tool.name || tool.tool || 'tool', 80),
            status: compactText(tool.status || (tool.running ? 'running' : (tool.is_error ? 'error' : 'done')), 48),
            running: false,
            blocked: tool.blocked === true,
            is_error: tool.is_error === true,
            summary: compactText(tool.summary || tool.label || tool.status || '', 160),
            detail_ref: cleanRef(tool.detail_ref || tool.result_ref || tool.input_ref || ''),
            input_ref: cleanRef(tool.input_ref || tool.detail_ref || ''),
            result_ref: cleanRef(tool.result_ref || tool.detail_ref || '')
          });
        }
        return rows;
      };
      var sanitizeArtifactRef = function(value) {
        if (!value || typeof value !== 'object') return null;
        return {
          path: compactText(value.path || value.file_name || value.name || '', 160),
          truncated: value.truncated === true,
          bytes: Number(value.bytes || value.archive_bytes || 0) || 0,
          entries: Number(value.entries || 0) || 0,
          detail_ref: cleanRef(value.detail_ref || value.download_url || '')
        };
      };
      for (var i = 0; i < source.length; i++) {
        var msg = source[i];
        if (!msg || typeof msg !== 'object') continue;
        if (msg.thinking || msg.streaming || (msg.terminal && msg.thinking)) continue;
        var roleRaw = String(msg.role || msg.type || '').trim().toLowerCase();
        if (roleRaw.indexOf('assistant') >= 0) roleRaw = 'agent';
        else if (roleRaw.indexOf('user') >= 0) roleRaw = 'user';
        else if (roleRaw.indexOf('system') >= 0) roleRaw = 'system';
        else if (msg.terminal) roleRaw = 'terminal';
        else roleRaw = roleRaw || 'agent';
        var rawText = msg.text;
        if (rawText == null && msg.content != null) rawText = msg.content;
        if (rawText == null && msg.message != null) rawText = msg.message;
        if (rawText == null && msg.assistant != null) rawText = msg.assistant;
        if (rawText == null && msg.user != null && roleRaw === 'user') rawText = msg.user;
        var tools = sanitizeTools(msg.tools);
        var fileOutput = sanitizeArtifactRef(msg.file_output);
        var folderOutput = sanitizeArtifactRef(msg.folder_output);
        var progress = msg.progress && typeof msg.progress === 'object'
          ? {
              label: compactText(msg.progress.label || msg.progress.status || '', 120),
              value: Number(msg.progress.value || msg.progress.percent || 0) || 0,
              total: Number(msg.progress.total || 0) || 0
            }
          : null;
        var preview = {
          id: compactText(msg.id || ('cached-message-' + i), 120),
          role: roleRaw,
          text: compactText(rawText, 1200),
          content_preview: compactText(rawText, 320),
          search_text: compactText(rawText, 500),
          meta: compactText(msg.meta || '', 160),
          ts: Number(msg.ts || msg.timestamp || Date.now()) || Date.now(),
          tools: tools,
          tool_summary_count: Array.isArray(msg.tools) ? msg.tools.length : tools.length,
          artifact_summary_count: (fileOutput ? 1 : 0) + (folderOutput ? 1 : 0),
          detail_ref: cleanRef(msg.detail_ref || '')
        };
        if (msg.terminal) preview.terminal = true;
        if (msg.is_notice || msg.notice_label || msg.notice_type || msg.notice_action) {
          preview.is_notice = true;
          preview.notice_label = compactText(msg.notice_label || '', 160);
          preview.notice_type = compactText(msg.notice_type || '', 40);
          preview.notice_icon = compactText(msg.notice_icon || '', 24);
          preview.notice_action = msg.notice_action && typeof msg.notice_action === 'object'
            ? {
                type: compactText(msg.notice_action.type || '', 60),
                label: compactText(msg.notice_action.label || '', 80),
                value: compactText(msg.notice_action.value || '', 160)
              }
            : null;
        }
        if (fileOutput) preview.file_output = fileOutput;
        if (folderOutput) preview.folder_output = folderOutput;
        if (progress) preview.progress = progress;
        var hasNotice = !!preview.is_notice;
        var hasText = typeof preview.text === 'string' && preview.text.trim().length > 0;
        var hasTools = Array.isArray(preview.tools) && preview.tools.length > 0;
        var hasArtifacts = !!(preview.file_output || preview.folder_output);
        var hasProgress = !!preview.progress;
        var hasTerminal = !!preview.terminal;
        if (!hasNotice && !hasText && !hasTools && !hasArtifacts && !hasProgress && !hasTerminal) {
          continue;
        }
        out.push(preview);
      }
      return out;
    },
    sanitizeConversationCacheForPersistence(cache) {
      var source = cache && typeof cache === 'object' ? cache : {};
      var out = {};
      var keys = Object.keys(source);
      for (var i = 0; i < keys.length; i += 1) {
        var key = String(keys[i] || '').trim();
        if (!key) continue;
        var row = source[key] && typeof source[key] === 'object' ? source[key] : {};
        out[key] = {
          saved_at: Number(row.saved_at || Date.now()) || Date.now(),
          session_scope_key: String(row.session_scope_key || '').slice(0, 160),
          session_label: String(row.session_label || '').slice(0, 120),
          token_count: Number(row.token_count || 0) || 0,
          default_terminal: row.default_terminal === true,
          draft_terminal: chatSanitizeConversationDraftText(row.draft_terminal || ''),
          draft_chat: chatSanitizeConversationDraftText(row.draft_chat || ''),
          messages: this.sanitizeConversationForCache(row.messages || [])
        };
      }
      return out;
    },
    restoreAgentConversation(agentId) {
      if (!agentId || !this.conversationCache) return false;
      const cached = this.conversationCache[String(agentId)];
      if (!cached || !Array.isArray(cached.messages)) return false;
      var scopeKey = typeof this.resolveConversationCacheScopeKey === 'function'
        ? this.resolveConversationCacheScopeKey(agentId)
        : String(agentId || '').trim();
      var cachedScopeKey = String(cached.session_scope_key || '').trim();
      if (scopeKey && cachedScopeKey && scopeKey !== cachedScopeKey) return false;
      try {
        if (this.applyConversationInputMode) this.applyConversationInputMode(agentId);
        var rawCachedMessages = cached.messages || [];
        var sanitized = this.sanitizeConversationForCache(cached.messages || []);
        var cacheChanged = false;
        try {
          cacheChanged = JSON.stringify(sanitized) !== JSON.stringify(rawCachedMessages);
        } catch(_) {
          cacheChanged = sanitized.length !== rawCachedMessages.length;
        }
        this.messages = this.mergeModelNoticesForAgent(
          agentId,
          this.normalizeSessionMessages({ messages: sanitized })
        );
        this.tokenCount = Number(cached.token_count || 0);
        this.sending = false;
        this._responseStartedAt = 0;
        this._clearPendingWsRequest();
        if (cacheChanged) {
          this.conversationCache[String(agentId)].messages = sanitized;
          this.persistConversationCache();
        }
        this.recomputeContextEstimate();
        if (typeof this.restoreConversationDraft === 'function') {
          this.restoreConversationDraft(agentId);
        }
        this.$nextTick(() => this.scrollToBottomImmediate());
        return true;
