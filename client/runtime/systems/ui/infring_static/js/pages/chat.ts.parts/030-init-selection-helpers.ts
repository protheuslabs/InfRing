      for (var i = 0; i < rows.length; i += 1) {
        if (this.normalizeFreshInitModelRef(rows[i]) === selected) return rows[i];
      }
      return rows.length ? rows[0] : null;
    },
    isFreshInitVibeSelected: function(card) {
      if (!card) return false;
      return String(card.id || '') === String(this.freshInitVibeId || '');
    },
    selectFreshInitVibe: function(card) {
      var id = String(card && card.id ? card.id : 'none').trim() || 'none';
      this.freshInitVibeId = id;
      this.scheduleFreshInitProgressAnchor();
    },
    scheduleFreshInitProgressAnchor: function(forcedAnchor) {
      var anchor = String(forcedAnchor || '').trim();
      if (!anchor) {
        if (this.freshInitCanLaunch) anchor = 'launch';
        else if (this.freshInitTemplateDef) anchor = 'lifespan';
        else anchor = 'role';
      }
      var self = this;
      this.$nextTick(function() {
        var scroller = typeof self.resolveMessagesScroller === 'function' ? self.resolveMessagesScroller(null) : null;
        if (!scroller || typeof scroller.getBoundingClientRect !== 'function') return;
        var panel = scroller.querySelector('.chat-init-panel');
        if (!panel) return;
        var target = panel.querySelector('[data-init-anchor=\"' + anchor + '\"]');
        if (!target || typeof target.getBoundingClientRect !== 'function') return;
        var hostRect = scroller.getBoundingClientRect();
        var targetRect = target.getBoundingClientRect();
        var delta = (targetRect.bottom + 92) - hostRect.bottom;
        if (Math.abs(delta) < 2) return;
        scroller.scrollTo({ top: Math.max(0, scroller.scrollTop + delta), behavior: 'smooth' });
      });
    },
    selectedFreshInitVibe: function() {
      var cards = Array.isArray(this.freshInitVibeCards) ? this.freshInitVibeCards : [];
      var selectedId = String(this.freshInitVibeId || 'none');
      for (var i = 0; i < cards.length; i += 1) {
        if (String(cards[i] && cards[i].id ? cards[i].id : '') === selectedId) return cards[i];
      }
      return cards.length ? cards[0] : null;
    },
    modelSpecialtyTagsForScoring: function(model) {
      var tags = model && model.specialty_tags;
      if (!Array.isArray(tags)) return [];
      var seen = {};
      var out = [];
      for (var i = 0; i < tags.length; i += 1) {
        var tag = String(tags[i] || '').trim().toLowerCase();
        if (!tag || seen[tag]) continue;
        seen[tag] = true;
        out.push(tag);
      }
      return out;
    },
    scoreFreshInitModelForRole: function(model, roleKey) {
      var row = model || {};
      var role = String(roleKey || 'general').trim().toLowerCase() || 'general';
      var power = this.modelPowerLevel(row);
      var cost = this.modelCostLevel(row);
      var contextWindow = Number(row && row.context_window != null ? row.context_window : 0);
      var contextScore = 0;
      if (Number.isFinite(contextWindow) && contextWindow > 0) {
        contextScore = Math.max(0, Math.min(2.4, Math.log2(Math.max(4096, contextWindow) / 4096)));
      }
      var paramsB = this.modelParamCountB(row);
      var specialty = String(row && row.specialty ? row.specialty : '').trim().toLowerCase();
      var tags = this.modelSpecialtyTagsForScoring(row);
      var name = this.freshInitModelName(row).toLowerCase();
      var local = this.modelDeploymentKind(row) === 'local';
      var score = (power * 1.25) + ((6 - cost) * 0.7) + (contextScore * 0.45);
      if (local) score += 0.35;
      if (role === 'coding') {
        if (specialty === 'coding') score += 3.1;
        if (tags.indexOf('coding') >= 0) score += 1.6;
        if (/\b(code|coder|codex|codestral|deepseek|starcoder|qwen.*coder)\b/i.test(name)) score += 1.5;
        score += power * 0.35;
      } else if (role === 'reasoning') {
        if (specialty === 'reasoning') score += 3.0;
        if (tags.indexOf('reasoning') >= 0) score += 1.2;
        score += contextScore * 1.15;
        if (/\b(reason|o3|r1|sonnet|opus|think)\b/i.test(name)) score += 0.9;
      } else if (role === 'creative') {
        score += Math.max(0, 1.8 - Math.abs(power - 3) * 0.7);
        score += contextScore * 0.8;
        if (specialty === 'coding') score -= 0.5;
      } else if (role === 'support') {
        score += (6 - cost) * 1.05;
        if (/\b(mini|flash|instant|turbo|haiku)\b/i.test(name)) score += 1.0;
        if (Number.isFinite(paramsB) && paramsB > 60) score -= 0.8;
      } else {
        score += power * 0.35;
        score += contextScore * 0.55;
      }
      if (Number.isFinite(paramsB) && paramsB > 0) {
        if (role === 'support' && paramsB > 80) score -= 1.0;
        if (role === 'coding' && paramsB > 100) score -= 0.6;
      }
      var usageBonus = this.modelUsageTs(this.normalizeFreshInitModelRef(row)) > 0 ? 0.25 : 0;
      score += usageBonus;
      return Number(score.toFixed(6));
    },
    refreshFreshInitModelSuggestions: async function(templateDef) {
      var template = templateDef || this.freshInitTemplateDef || null;
      if (!template) {
        this.freshInitModelSuggestions = [];
        this.freshInitModelSelection = '';
        this.freshInitModelSuggestLoading = false;
        return;
      }
      this.freshInitModelSuggestLoading = true;
      try {
        var rows = await this.ensureFailoverModelCache();
        var roleKey = this.freshInitRoleKey(template);
        var ranked = (Array.isArray(rows) ? rows : [])
          .filter(function(row) {
            return !!(row && row.available !== false && String(row.id || '').trim() && String(row.id || '').trim().toLowerCase() !== 'auto');
          })
          .map((row) => ({
            ...(row && typeof row === 'object' ? row : {}),
            _fresh_role_score: this.scoreFreshInitModelForRole(row, roleKey),
          }))
          .sort((left, right) => {
            var a = Number(left && left._fresh_role_score != null ? left._fresh_role_score : 0);
            var b = Number(right && right._fresh_role_score != null ? right._fresh_role_score : 0);
            if (b !== a) return b - a;
            var lName = this.normalizeFreshInitModelRef(left).toLowerCase();
            var rName = this.normalizeFreshInitModelRef(right).toLowerCase();
            return lName.localeCompare(rName);
          })
          .slice(0, 5);
        if (!ranked.length) {
          var fallbackProvider = String(template.provider || '').trim().toLowerCase();
          var fallbackModel = String(template.model || '').trim();
          if (fallbackProvider && fallbackModel) {
            ranked = [{
              id: fallbackProvider + '/' + fallbackModel,
              display_name: fallbackModel,
              provider: fallbackProvider,
              context_window: 0,
              available: true,
              power_rating: 3,
              cost_rating: fallbackProvider === 'ollama' || fallbackProvider === 'llama.cpp' ? 1 : 3,
              specialty: 'general',
              specialty_tags: ['general'],
            }];
          }
        }
        this.freshInitModelSuggestions = ranked;
        var current = String(this.freshInitModelSelection || '').trim();
        var hasCurrent = ranked.some((row) => this.normalizeFreshInitModelRef(row) === current);
        if (!this.freshInitModelManual || !hasCurrent) {
          this.freshInitModelSelection = ranked.length ? this.normalizeFreshInitModelRef(ranked[0]) : '';
        }
      } catch (_) {
        if (!this.freshInitModelManual && template) {
          var provider = String(template.provider || '').trim();
          var model = String(template.model || '').trim();
          this.freshInitModelSelection = provider && model ? (provider.toLowerCase() + '/' + model) : '';
        }
      } finally {
        this.freshInitModelSuggestLoading = false;
      }
    },
    get modelDisplayName() {
      var readModelField = function(agent, keys) {
        var row = agent && typeof agent === 'object' ? agent : null;
        if (!row) return '';
        for (var i = 0; i < keys.length; i += 1) {
          var key = String(keys[i] || '').trim();
          if (!key) continue;
          var value = String(row[key] || '').trim();
          if (value) return value;
        }
        return '';
      };
      var store = typeof this.getAppStore === 'function' ? this.getAppStore() : null;
      var currentId = String((this.currentAgent && this.currentAgent.id) || '').trim();
      var storeAgent = null;
      if (store && Array.isArray(store.agents) && currentId) {
        for (var ai = 0; ai < store.agents.length; ai += 1) {
          var row = store.agents[ai];
          if (row && String(row.id || '').trim() === currentId) {
            storeAgent = row;
            break;
          }
        }
      }
      var selected = readModelField(this.currentAgent, ['model_name', 'selected_model', 'model', 'selected_model_id']);
      var runtime = readModelField(this.currentAgent, ['runtime_model', 'current_model', 'resolved_model']);
      var modelOverride = readModelField(this.currentAgent, ['model_override', 'active_model_ref']);
      var storeSelected = readModelField(storeAgent, ['model_name', 'selected_model', 'model', 'selected_model_id']);
      var storeRuntime = readModelField(storeAgent, ['runtime_model', 'current_model', 'resolved_model']);
      var storeOverride = readModelField(storeAgent, ['model_override', 'active_model_ref']);
      var suggestion = this.selectedFreshInitModelSuggestion ? this.selectedFreshInitModelSuggestion() : null;
      var suggestionRef = this.normalizeFreshInitModelRef ? this.normalizeFreshInitModelRef(suggestion) : '';
      var providerFallback = readModelField(this.currentAgent, ['model_provider', 'provider', 'selected_provider']);
      if (!providerFallback) providerFallback = readModelField(storeAgent, ['model_provider', 'provider', 'selected_provider']);
      providerFallback = String(providerFallback || '').trim().toLowerCase();
      if (this.isPlaceholderModelRef(selected)) selected = '';
      if (this.isPlaceholderModelRef(runtime)) runtime = '';
      if (this.isPlaceholderModelRef(modelOverride)) modelOverride = '';
      if (this.isPlaceholderModelRef(storeSelected)) storeSelected = '';
      if (this.isPlaceholderModelRef(storeRuntime)) storeRuntime = '';
      if (this.isPlaceholderModelRef(storeOverride)) storeOverride = '';
      if (this.isPlaceholderModelRef(suggestionRef)) suggestionRef = '';
      var runtimeEngineId = String(this.selectedAgentRuntimeEngineId || 'infring_native').trim() || 'infring_native';
      if (runtimeEngineId !== 'infring_native' && Array.isArray(this.runtimeEngineRows)) {
        var runtimeEngineRow = null;
        for (var ri = 0; ri < this.runtimeEngineRows.length; ri += 1) {
          var runtimeCandidate = this.runtimeEngineRows[ri] || {};
          if (String(runtimeCandidate.engine_id || '').trim() === runtimeEngineId) {
            runtimeEngineRow = runtimeCandidate;
            break;
          }
        }
        if (!runtimeEngineRow) {
          var pendingRuntimeLabel = this.runtimeEngineDisplayNameForId
            ? this.runtimeEngineDisplayNameForId(runtimeEngineId, null)
            : runtimeEngineId;
          pendingRuntimeLabel = String(pendingRuntimeLabel || runtimeEngineId || 'Agent runtime').trim();
          var compactPendingRuntimeLabel = this.truncateModelLabel(pendingRuntimeLabel);
          if (compactPendingRuntimeLabel) return compactPendingRuntimeLabel.length > 24 ? compactPendingRuntimeLabel.substring(0, 22) + '\u2026' : compactPendingRuntimeLabel;
        }
        var availableModels = runtimeEngineRow && runtimeEngineRow.available_models && typeof runtimeEngineRow.available_models === 'object'
          ? runtimeEngineRow.available_models
          : runtimeEngineRow && runtimeEngineRow.model_menu && typeof runtimeEngineRow.model_menu === 'object'
          ? runtimeEngineRow.model_menu
          : null;
        var availableRows = availableModels && Array.isArray(availableModels.rows)
          ? availableModels.rows
          : (availableModels && Array.isArray(availableModels.model_rows) ? availableModels.model_rows : []);
        if (availableModels && (availableModels.framework_native_models || availableModels.source === 'framework_native')) {
          var runtimeCandidates = [runtime, selected, modelOverride, storeRuntime, storeSelected, storeOverride, suggestionRef];
          var matchedRuntimeModel = null;
          for (var rci = 0; rci < runtimeCandidates.length && !matchedRuntimeModel; rci += 1) {
            var runtimeCandidateId = String(runtimeCandidates[rci] || '').trim().toLowerCase();
            if (!runtimeCandidateId) continue;
            for (var rmi = 0; rmi < availableRows.length; rmi += 1) {
              var runtimeModelRow = availableRows[rmi] || {};
              var rowIds = [
                runtimeModelRow.id,
                runtimeModelRow.qualified_model_ref,
                runtimeModelRow.model,
                runtimeModelRow.model_name,
                runtimeModelRow.adapter_model_arg,
                runtimeModelRow.display_name
              ];
              for (var ridx = 0; ridx < rowIds.length; ridx += 1) {
                if (String(rowIds[ridx] || '').trim().toLowerCase() === runtimeCandidateId) {
                  matchedRuntimeModel = runtimeModelRow;
                  break;
                }
              }
              if (matchedRuntimeModel) break;
            }
          }
          var displayRuntimeModel = matchedRuntimeModel || availableRows[0] || {};
          var runtimeModelLabel = String(displayRuntimeModel.display_name || displayRuntimeModel.model_name || displayRuntimeModel.model || displayRuntimeModel.id || '').trim();
          if (!runtimeModelLabel && availableModels.default_selection_policy && typeof availableModels.default_selection_policy === 'object') {
            runtimeModelLabel = String(availableModels.default_selection_policy.current_model || '').trim();
          }
          if (!runtimeModelLabel) runtimeModelLabel = String(runtimeEngineRow.display_name || runtimeEngineId || 'Agent runtime').trim();
          var compactRuntimeModel = this.truncateModelLabel(runtimeModelLabel);
          if (compactRuntimeModel) return compactRuntimeModel.length > 24 ? compactRuntimeModel.substring(0, 22) + '\u2026' : compactRuntimeModel;
        }
        if (availableModels && !(availableModels.inherit_active_llm_when_unconfigured || availableModels.credential_inheritance_allowed || availableModels.source === 'inherited_infring')) {
          var runtimeLabel = String(runtimeEngineRow && (runtimeEngineRow.display_name || runtimeEngineRow.engine_id) || runtimeEngineId || 'Agent runtime').trim();
          var compactRuntimeLabel = this.truncateModelLabel(runtimeLabel);
          if (compactRuntimeLabel) return compactRuntimeLabel.length > 24 ? compactRuntimeLabel.substring(0, 22) + '\u2026' : compactRuntimeLabel;
        }
      }
      if (selected.toLowerCase() === 'auto') {
        var resolved = this.truncateModelLabel(runtime);
        var autoLabel = resolved ? ('Auto: ' + resolved) : 'Auto';
        return autoLabel.length > 24 ? autoLabel.substring(0, 22) + '\u2026' : autoLabel;
      }
      if (runtime && selected && runtime.toLowerCase() !== selected.toLowerCase()) {
        var runtimeLabel = this.truncateModelLabel(runtime);
        if (runtimeLabel) {
          return runtimeLabel.length > 24 ? runtimeLabel.substring(0, 22) + '\u2026' : runtimeLabel;
        }
      }
      var active = this.resolveActiveSwitcherModel ? this.resolveActiveSwitcherModel(this._modelCache || []) : null;
      var activeId = String((active && active.id) || '').trim();
      var candidates = [runtime, selected, modelOverride, storeRuntime, storeSelected, storeOverride, suggestionRef, activeId];
      for (var ci = 0; ci < candidates.length; ci += 1) {
        var compactCandidate = this.truncateModelLabel(candidates[ci]);
        if (!compactCandidate) continue;
        return compactCandidate.length > 24 ? compactCandidate.substring(0, 22) + '\u2026' : compactCandidate;
      }
      if (providerFallback === 'auto' || !providerFallback) return 'Auto';
      return providerFallback.length > 24 ? providerFallback.substring(0, 22) + '\u2026' : providerFallback;
    },

    get menuModelLabel() {
      var label = String(this.modelDisplayName || '').trim();
      if (!label) label = 'Auto';
      if (label.length > 7) return label.substring(0, 7) + '...';
      return label;
    },

    modelRowsForActiveRuntimeEngine: function(rows, runtimeEngineId) {
    var list = Array.isArray(rows) ? rows : [];
    var engineId = String(runtimeEngineId || this.selectedAgentRuntimeEngineId || 'infring_native').trim() || 'infring_native';
    if (!engineId || engineId === 'infring_native') return list;
    var runtimeRows = Array.isArray(this.runtimeEngineRows) ? this.runtimeEngineRows : [];
    var engineRow = null;
    for (var i = 0; i < runtimeRows.length; i += 1) {
      var row = runtimeRows[i] || {};
      if (String(row.engine_id || '').trim() === engineId) {
        engineRow = row;
        break;
      }
    }
    var menu = engineRow && engineRow.available_models && typeof engineRow.available_models === 'object'
      ? engineRow.available_models
      : engineRow && engineRow.model_menu && typeof engineRow.model_menu === 'object'
      ? engineRow.model_menu
      : null;
    if (!menu || menu.show_in_llm_menu === false) return [];
    var sanitize = typeof this.sanitizeModelCatalogRows === 'function'
      ? this.sanitizeModelCatalogRows.bind(this)
      : function(value) { return Array.isArray(value) ? value : []; };
    var menuRows = Array.isArray(menu.rows) ? menu.rows : (Array.isArray(menu.model_rows) ? menu.model_rows : []);
    if (menu.framework_native_models || menu.source === 'framework_native') {
      return sanitize(menuRows.map(function(modelRow) {
        return Object.assign({}, modelRow || {}, {
          runtime_engine_id: engineId,
          runtime_model_source: 'framework_native'
        });
      }));
    }
    if (menu.inherit_active_llm_when_unconfigured || menu.credential_inheritance_allowed || menu.source === 'inherited_infring') {
      return list.map(function(modelRow) {
        return Object.assign({}, modelRow || {}, {
          runtime_engine_id: engineId,
          runtime_model_source: 'inherited_infring_llm'
        });
      });
    }
    return [];
  },

  get switcherViewState() {
      var modelsRef = Array.isArray(this._modelCache) ? this._modelCache : [];
      if (!modelsRef.length && this.showModelSwitcher && typeof this.fallbackModelCatalogRows === 'function') {
        modelsRef = this.fallbackModelCatalogRows();
        this._modelCache = modelsRef;
        this._modelCacheTime = Date.now();
        this.modelPickerList = modelsRef;
        this._modelSwitcherViewCache = null;
      }
      var runtimeEngineId = String(this.selectedAgentRuntimeEngineId || 'infring_native').trim() || 'infring_native';
      modelsRef = this.modelRowsForActiveRuntimeEngine(modelsRef, runtimeEngineId);
      var providerFilter = String(this.modelSwitcherProviderFilter || '').trim();
      var textFilter = String(this.modelSwitcherFilter || '').trim().toLowerCase();
      var cacheTime = Number(this._modelCacheTime || 0);
      var cache = this._modelSwitcherViewCache;
      if (
        cache &&
        cache.modelsRef === modelsRef &&
        cache.runtimeEngineId === runtimeEngineId &&
        cache.providerFilter === providerFilter &&
        cache.textFilter === textFilter &&
        cache.cacheTime === cacheTime
      ) {
        return cache.value;
      }

      var seenProviders = {};
      for (var pi = 0; pi < modelsRef.length; pi += 1) {
        var providerName = String(modelsRef[pi] && modelsRef[pi].provider ? modelsRef[pi].provider : '').trim();
        if (providerName) seenProviders[providerName] = true;
      }
      var providers = Object.keys(seenProviders).sort();

      var filtered = modelsRef.filter(function(m) {
        var row = m || {};
        var rowProvider = String(row.provider || '').trim();
        var rowId = String(row.id || '').trim();
        var rowDisplay = String(row.display_name || '').trim();
        if (providerFilter && rowProvider !== providerFilter) return false;
        if (!textFilter) return true;
        return rowId.toLowerCase().indexOf(textFilter) !== -1 ||
          rowDisplay.toLowerCase().indexOf(textFilter) !== -1 ||
          rowProvider.toLowerCase().indexOf(textFilter) !== -1;
      });

      var self = this;
      var activeIds = self.activeModelCandidateIds();
      var activeMap = {};
      for (var ai = 0; ai < activeIds.length; ai += 1) {
        activeMap[String(activeIds[ai] || '').trim()] = true;
      }
      var usageCache = {};
      var usageFor = function(id) {
        var key = String(id || '').trim();
        if (!key) return 0;
        if (Object.prototype.hasOwnProperty.call(usageCache, key)) return usageCache[key];
        var ts = self.modelUsageTs(key);
        usageCache[key] = ts;
        return ts;
      };

      filtered.sort(function(a, b) {
        var aId = String((a && a.id) || '').trim();
        var bId = String((b && b.id) || '').trim();
        var aAvailable = !(a && a.available === false) ? 1 : 0;
        var bAvailable = !(b && b.available === false) ? 1 : 0;
        if (bAvailable !== aAvailable) return bAvailable - aAvailable;
        var aUsage = usageFor(aId);
        var bUsage = usageFor(bId);
        if (bUsage !== aUsage) return bUsage - aUsage;
        var aActive = aId && activeMap[aId] ? 1 : 0;
        var bActive = bId && activeMap[bId] ? 1 : 0;
        if (bActive !== aActive) return bActive - aActive;
        var aProvider = String((a && a.provider) || '').toLowerCase();
        var bProvider = String((b && b.provider) || '').toLowerCase();
        if (aProvider !== bProvider) return aProvider.localeCompare(bProvider);
        return aId.toLowerCase().localeCompare(bId.toLowerCase());
      });

      var maxVisible = (textFilter || providerFilter) ? 240 : 120;
      var rendered = filtered.length > maxVisible ? filtered.slice(0, maxVisible) : filtered.slice();
      var groups = [];
      var cursor = 0;
      var active = self.resolveActiveSwitcherModel(rendered.length ? rendered : filtered);
      var activeId = '';
      if (active) {
        activeId = String(active.id || '').trim();
        groups.push({
          provider: 'Active',
          models: [Object.assign({}, active, { _switcherIndex: cursor++ })]
        });
      }
      var recent = rendered.filter(function(m) {
        var id = String((m && m.id) || '').trim();
        return !activeId || id !== activeId;
      });
      if (recent.length) {
        groups.push({
          provider: 'Recent',
          models: recent.map(function(row) {
            return Object.assign({}, row, { _switcherIndex: cursor++ });
          })
        });
      } else if (!groups.length && rendered.length) {
        groups.push({
          provider: 'Recent',
          models: rendered.map(function(row) {
            return Object.assign({}, row, { _switcherIndex: cursor++ });
          })
        });
      }

      var value = {
        providers: providers,
        filtered: filtered,
        rendered: rendered,
        grouped: groups,
        totalCount: filtered.length,
        truncatedCount: Math.max(0, filtered.length - rendered.length),
      };
      this._modelSwitcherViewCache = {
        modelsRef: modelsRef,
        runtimeEngineId: runtimeEngineId,
        providerFilter: providerFilter,
        textFilter: textFilter,
        cacheTime: cacheTime,
        value: value,
      };
      return value;
    },
    get switcherProviders() {
      return this.switcherViewState.providers;
    },
    get filteredSwitcherModels() {
      return this.switcherViewState.filtered;
    },
    get renderedSwitcherModels() {
      return this.switcherViewState.rendered;
    },
    get modelSwitcherTruncatedCount() {
      return this.switcherViewState.truncatedCount;
    },
    isPlaceholderModelRef: function(value) {
      var id = String(value || '').trim().toLowerCase();
      if (!id) return true;
      if (id === 'model' || id === '<model>' || id === '(model)') return true;
      if (id.indexOf('/') >= 0) {
        var tail = String(id.split('/').slice(-1)[0] || '').trim();
        if (!tail) return true;
        return tail === 'model' || tail === '<model>' || tail === '(model)';
      }
      return false;
    },
    buildQualifiedModelRef: function(modelValue, providerValue) {
      var model = String(modelValue || '').trim();
      var provider = String(providerValue || '').trim().toLowerCase();
      if (!model || this.isPlaceholderModelRef(model)) return '';
      if (!provider) return model;
      var normalizedPrefix = provider + '/';
      if (model.toLowerCase().indexOf(normalizedPrefix) === 0) return model;
      if (model.indexOf('/') >= 0) return model;
      return provider + '/' + model;
    },
    normalizeModelOverrideValue: function(modelValue) {
      if (!modelValue || typeof modelValue !== 'object') {
        return {
          kind: '',
          value: String(modelValue || '').trim()
        };
      }
      var kind = String(modelValue.kind || '').trim().toLowerCase();
      var value = String(
        modelValue.value ||
        modelValue.model ||
        modelValue.id ||
        ''
      ).trim();
      return {
        kind: kind === 'qualified' || kind === 'raw' ? kind : '',
        value: value
      };
    },
    normalizeQualifiedModelRef: function(modelValue, providerValue, rows) {
      var override = this.normalizeModelOverrideValue(modelValue);
      var raw = String(override.value || '').trim();
      if (!raw || this.isPlaceholderModelRef(raw)) return '';
      if (override.kind === 'qualified') return raw;
      if (typeof this.resolveModelCatalogOption === 'function') {
        var resolved = this.resolveModelCatalogOption(raw, providerValue || '', rows);
        var resolvedId = String(resolved && resolved.id ? resolved.id : '').trim();
        if (resolvedId) return resolvedId;
      }
      return this.buildQualifiedModelRef(raw, providerValue);
    },
    formatQualifiedModelDisplay: function(value) {
      var ref = String(value || '').trim();
      if (!ref || this.isPlaceholderModelRef(ref)) return '';
      if (ref.indexOf('/') < 0) return ref;
      var parts = ref.split('/');
      var provider = String(parts[0] || '').trim();
      var model = String(parts.slice(1).join('/') || '').trim();
      if (!model) return provider || ref;
      if (!provider) return model;
      return model + ' · ' + provider;
    },
    truncateModelLabel: function(value) {
      var raw = this.normalizeQualifiedModelRef(value, '', this._modelCache || []);
      if (!raw || this.isPlaceholderModelRef(raw)) return '';
      var compact = raw;
      if (raw.indexOf('/') >= 0) {
        var parts = raw.split('/');
        var tail = String(parts[parts.length - 1] || '').trim();
        var head = String(parts[0] || '').trim();
        compact = tail || head;
      }
      compact = String(compact || '').trim();
      if (!compact || this.isPlaceholderModelRef(compact)) return '';
      return compact.replace(/-\d{8}$/, '');
    },

    // Backward-compat shim for legacy callers during naming migration.
    compactModelLabel: function(value) {
      return this.truncateModelLabel(value);
    },
    sanitizeModelCatalogRows: function(rows) {
      var list = Array.isArray(rows) ? rows : [];
      var out = [];
      var seen = {};
      for (var i = 0; i < list.length; i += 1) {
        var row = list[i] && typeof list[i] === 'object' ? list[i] : {};
        var provider = String(row.provider || row.model_provider || '').trim();
        var modelName = String(row.model || row.model_name || row.runtime_model || row.id || '').trim();
        var id = this.buildQualifiedModelRef(row.id || modelName, provider);
        if (!id || this.isPlaceholderModelRef(id)) continue;
        if (!provider && id.indexOf('/') >= 0) provider = String(id.split('/')[0] || '').trim();
        if (!provider) provider = 'unknown';
        var key = id.toLowerCase();
        if (seen[key]) continue;
        seen[key] = true;
        var normalizedModelName = modelName;
        if (!normalizedModelName && id.indexOf('/') >= 0) {
          normalizedModelName = String(id.split('/').slice(1).join('/') || '').trim();
        }
        out.push(Object.assign({}, row, {
          id: id,
          provider: provider,
          model: normalizedModelName || id,
          model_name: normalizedModelName || id,
          display_name: String(row.display_name || normalizedModelName || this.formatQualifiedModelDisplay(id) || id).trim(),
          available: row.available !== false
        }));
      }
      return out;
    },
    activeModelCandidateIds: function() {
      var out = [];
      var seen = {};
      var self = this;
      var add = function(value) {
        var id = self.normalizeQualifiedModelRef(value, provider, self._modelCache || []);
        if (!id || self.isPlaceholderModelRef(id) || seen[id]) return;
        seen[id] = true;
        out.push(id);
      };
      var agent = this.currentAgent || {};
      var selected = String(agent.model_name || '').trim();
      var runtime = String(agent.runtime_model || '').trim();
      var provider = String(agent.model_provider || '').trim().toLowerCase();
      if (selected) add(selected);
      if (runtime) add(runtime);
      return out;
    },
    isSwitcherModelActive: function(model) {
      var id = String(model && model.id ? model.id : '').trim();
      if (!id) return false;
      return this.activeModelCandidateIds().indexOf(id) >= 0;
    },
    resolveActiveSwitcherModel: function(filtered) {
      var rows = Array.isArray(filtered) ? filtered : [];
      var activeIds = this.activeModelCandidateIds();
      for (var i = 0; i < activeIds.length; i++) {
        var id = activeIds[i];
        for (var j = 0; j < rows.length; j++) {
          var row = rows[j];
          if (row && String(row.id || '').trim() === id) return row;
        }
      }
      var agent = this.currentAgent || null;
      if (!agent) return null;
      var selected = String(agent.model_name || '').trim();
      var runtime = String(agent.runtime_model || '').trim();
      var provider = String(agent.model_provider || '').trim();
      var activeId = selected.toLowerCase() === 'auto' && runtime ? runtime : (selected || runtime);
      activeId = this.normalizeQualifiedModelRef(activeId, provider, rows);
      if (!activeId || this.isPlaceholderModelRef(activeId)) return null;
      return {
        id: activeId,
        provider: provider || (activeId.indexOf('/') >= 0 ? activeId.split('/')[0] : 'unknown'),
        display_name: activeId.indexOf('/') >= 0 ? activeId.split('/').slice(-1)[0] : activeId,
        tier: 'Active',
        context_window: Number(agent.context_window || 0) || null,
        is_local: provider.toLowerCase() === 'ollama' || provider.toLowerCase() === 'llama.cpp',
        deployment: (provider.toLowerCase() === 'ollama' || provider.toLowerCase() === 'llama.cpp') ? 'local' : (provider.toLowerCase() === 'cloud' ? 'cloud' : 'api'),
        power_rating: 3,
        cost_rating: provider.toLowerCase() === 'ollama' || provider.toLowerCase() === 'llama.cpp' ? 1 : 3,
        specialty: 'general',
        specialty_tags: ['general'],
        local_download_path: '',
        download_available: false,
      };
    },
    get groupedSwitcherModels() {
      return this.switcherViewState.grouped;
    },
    modelSwitcherItemName: function(m) {
      var model = m || {};
      var provider = String(model.provider || '').trim();
      var id = String(model.id || '').trim();
      var display = String(model.display_name || id).trim();
      var isAutoRow = provider.toLowerCase() === 'auto' || id.toLowerCase() === 'auto';
      if (!isAutoRow) return display || id || 'model';
      var activeAuto = this.currentAgent && String(this.currentAgent.model_name || '').trim().toLowerCase() === 'auto';
      var runtime = activeAuto ? String(this.currentAgent.runtime_model || '').trim() : '';
      if (!runtime) return 'Auto';
      var short = runtime.replace(/-\d{8}$/, '');
      return short ? ('Auto: ' + short) : 'Auto';
    },
    modelLogoFamilyKey: function(model) {
      var row = model && typeof model === 'object' ? model : {};
      var combined = String(
        row.id || row.display_name || row.model_name || row.name || ''
      ).toLowerCase();
      var provider = String(row.provider || row.model_provider || '').toLowerCase();
      var haystack = (provider + ' ' + combined).trim();
      if (!haystack) return 'unknown';
      if (haystack.indexOf('openai') >= 0 || haystack.indexOf('chatgpt') >= 0 || haystack.indexOf('gpt') >= 0) return 'openai';
      if (haystack.indexOf('anthropic') >= 0 || haystack.indexOf('claude') >= 0 || haystack.indexOf('frontier_provider') >= 0) return 'anthropic';
      if (haystack.indexOf('gemini') >= 0 || haystack.indexOf('google') >= 0) return 'gemini';
      if (haystack.indexOf('qwen') >= 0) return 'qwen';
      if (haystack.indexOf('deepseek') >= 0) return 'deepseek';
      if (haystack.indexOf('kimi') >= 0 || haystack.indexOf('moonshot') >= 0) return 'kimi';
      if (haystack.indexOf('llama') >= 0 || haystack.indexOf('meta') >= 0) return 'llama';
      if (haystack.indexOf('mistral') >= 0 || haystack.indexOf('mixtral') >= 0) return 'mistral';
      if (haystack.indexOf('grok') >= 0 || haystack.indexOf('xai') >= 0) return 'xai';
      return 'unknown';
    },
    modelLogoSimpleIconUrl: function(slug) {
      var key = String(slug || '').trim().toLowerCase();
      if (!key) return '';
      return 'https://cdn.simpleicons.org/' + encodeURIComponent(key);
    },
    modelLogoClearbitUrl: function(domain) {
      var value = String(domain || '').trim().toLowerCase();
      if (!value) return '';
      return 'https://logo.clearbit.com/' + encodeURIComponent(value) + '?size=64&format=png';
    },
    pushUniqueLogoCandidate: function(list, url) {
      var value = String(url || '').trim();
      if (!value) return;
      if (list.indexOf(value) >= 0) return;
      list.push(value);
    },
    modelLogoCandidates: function(model) {
      var key = this.modelLogoFamilyKey(model);
      var out = [];
      if (key === 'openai') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('openai'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('openai.com'));
      } else if (key === 'anthropic') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('anthropic'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('anthropic.com'));
      } else if (key === 'gemini') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('googlegemini'));
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('google'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('google.com'));
      } else if (key === 'qwen') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('qwen'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('alibabacloud.com'));
      } else if (key === 'deepseek') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('deepseek'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('deepseek.com'));
      } else if (key === 'kimi') {
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('moonshot.ai'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('moonshot.cn'));
      } else if (key === 'llama') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('meta'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('meta.com'));
      } else if (key === 'mistral') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('mistralai'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('mistral.ai'));
      } else if (key === 'xai') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('x'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('x.ai'));
      }
      return out;
    },
    modelLogoFailMap: function(kind) {
      var scope = String(kind || '').trim().toLowerCase() === 'source' ? 'source' : 'model';
      if (scope === 'source') {
        if (!this._modelSourceLogoFailIndex || typeof this._modelSourceLogoFailIndex !== 'object') {
          this._modelSourceLogoFailIndex = {};
        }
        return this._modelSourceLogoFailIndex;
      }
      if (!this._modelLogoFailIndex || typeof this._modelLogoFailIndex !== 'object') {
        this._modelLogoFailIndex = {};
      }
      return this._modelLogoFailIndex;
    },
    modelLogoUrl: function(model) {
      var key = this.modelLogoFamilyKey(model);
      if (!key || key === 'unknown') return '';
      var candidates = this.modelLogoCandidates(model);
      if (!candidates.length) return '';
      var map = this.modelLogoFailMap('model');
      var index = Number(map[key] || 0);
      if (!Number.isFinite(index) || index < 0) index = 0;
      if (index >= candidates.length) return '';
      return String(candidates[index] || '');
    },
    modelLogoTooltip: function(model) {
      var key = this.modelLogoFamilyKey(model);
      if (key === 'unknown') return 'Model family';
      if (key === 'openai') return 'Model family: OpenAI';
      if (key === 'anthropic') return 'Model family: Anthropic';
      if (key === 'gemini') return 'Model family: Gemini';
      if (key === 'qwen') return 'Model family: Qwen';
      if (key === 'deepseek') return 'Model family: DeepSeek';
      if (key === 'kimi') return 'Model family: Kimi';
      if (key === 'llama') return 'Model family: Llama';
      if (key === 'mistral') return 'Model family: Mistral';
      if (key === 'xai') return 'Model family: xAI';
      return 'Model family';
    },
    modelSourceLogoKey: function(model) {
      var row = model && typeof model === 'object' ? model : {};
      var provider = String(row.provider || row.model_provider || '').trim().toLowerCase();
      if (!provider) {
        var deployment = this.modelDeploymentKind(row);
        if (deployment === 'local') return 'local';
        if (deployment === 'cloud') return 'cloud';
        if (deployment === 'api') return 'direct';
        return 'unknown';
      }
      if (provider.indexOf('ollama') >= 0) return 'ollama';
      if (provider.indexOf('huggingface') >= 0 || provider === 'hf') return 'huggingface';
      if (provider.indexOf('openrouter') >= 0) return 'openrouter';
      if (provider.indexOf('openai') >= 0) return 'openai';
      if (provider.indexOf('frontier_provider') >= 0 || provider.indexOf('anthropic') >= 0) return 'anthropic';
      if (provider.indexOf('google') >= 0 || provider.indexOf('gemini') >= 0) return 'google';
      if (provider.indexOf('moonshot') >= 0 || provider.indexOf('kimi') >= 0) return 'moonshot';
      if (provider.indexOf('deepseek') >= 0) return 'deepseek';
      if (provider.indexOf('groq') >= 0) return 'groq';
      if (provider.indexOf('xai') >= 0) return 'xai';
      if (provider.indexOf('cloud') >= 0) return 'cloud';
      return 'direct';
    },
    modelSourceLogoCandidates: function(model) {
      var key = this.modelSourceLogoKey(model);
      var out = [];
      if (key === 'ollama') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('ollama'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('ollama.com'));
      } else if (key === 'huggingface') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('huggingface'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('huggingface.co'));
      } else if (key === 'openrouter') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('openrouter'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('openrouter.ai'));
      } else if (key === 'openai') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('openai'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('openai.com'));
      } else if (key === 'anthropic') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('anthropic'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('anthropic.com'));
      } else if (key === 'google') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('google'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('google.com'));
      } else if (key === 'moonshot') {
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('moonshot.ai'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('moonshot.cn'));
      } else if (key === 'deepseek') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('deepseek'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('deepseek.com'));
      } else if (key === 'groq') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('groq'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('groq.com'));
      } else if (key === 'xai') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('x'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('x.ai'));
      } else if (key === 'local') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('docker'));
      } else if (key === 'cloud') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('icloud'));
      } else if (key === 'direct') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('chainlink'));
      }
      return out;
    },
    modelSourceLogoUrl: function(model) {
      var key = this.modelSourceLogoKey(model);
      if (!key || key === 'unknown') return '';
      var candidates = this.modelSourceLogoCandidates(model);
      if (!candidates.length) return '';
      var map = this.modelLogoFailMap('source');
      var index = Number(map[key] || 0);
      if (!Number.isFinite(index) || index < 0) index = 0;
      if (index >= candidates.length) return '';
      return String(candidates[index] || '');
    },
    onModelLogoLoad: function(event) {
      var target = event && event.target ? event.target : null;
      if (!target || !target.style) return;
      target.style.visibility = '';
    },
    onModelLogoError: function(kind, model, event) {
      var scope = String(kind || '').trim().toLowerCase() === 'source' ? 'source' : 'model';
      var key = scope === 'source' ? this.modelSourceLogoKey(model) : this.modelLogoFamilyKey(model);
      var candidates = scope === 'source' ? this.modelSourceLogoCandidates(model) : this.modelLogoCandidates(model);
      var target = event && event.target ? event.target : null;
      if (!key || !candidates.length) {
        if (target && target.style) target.style.visibility = 'hidden';
        return;
      }
      var map = this.modelLogoFailMap(scope);
      var current = Number(map[key] || 0);
      if (!Number.isFinite(current) || current < 0) current = 0;
      var next = current + 1;
      map[key] = next;
      var replacement = next < candidates.length ? String(candidates[next] || '') : '';
      if (!target || !target.style) return;
      if (replacement) {
        target.style.visibility = '';
        target.src = replacement;
        return;
      }
      target.style.visibility = 'hidden';
    },
    modelSourceLogoTooltip: function(model) {
      var key = this.modelSourceLogoKey(model);
      if (key === 'unknown') return 'Model source';
      if (key === 'ollama') return 'Source: Ollama';
      if (key === 'huggingface') return 'Source: Hugging Face';
      if (key === 'openrouter') return 'Source: OpenRouter';
      if (key === 'openai') return 'Source: OpenAI direct';
      if (key === 'anthropic') return 'Source: Anthropic direct';
      if (key === 'google') return 'Source: Google direct';
      if (key === 'moonshot') return 'Source: Moonshot direct';
      if (key === 'deepseek') return 'Source: DeepSeek direct';
      if (key === 'groq') return 'Source: Groq';
      if (key === 'xai') return 'Source: xAI direct';
      if (key === 'local') return 'Source: Local runtime';
      if (key === 'cloud') return 'Source: Cloud runtime';
      if (key === 'direct') return 'Source: Direct provider';
      return 'Model source';
    },
    modelDeploymentKind: function(model) {
      var row = model || {};
      var deployment = String(row.deployment || row.deployment_kind || '').trim().toLowerCase();
      if (deployment === 'local' || deployment === 'cloud' || deployment === 'api') return deployment;
      if (row.is_local === true) return 'local';
      var provider = String(row.provider || '').trim().toLowerCase();
      if (provider === 'ollama' || provider === 'llama.cpp') return 'local';
      if (provider === 'cloud') return 'cloud';
      return 'api';
    },
    modelDeploymentLabel: function(model) {
      var kind = this.modelDeploymentKind(model);
      if (kind === 'local') return 'Local model';
      if (kind === 'api') return 'API model';
      return 'Cloud model';
    },
    normalizeModelRating: function(value, fallback) {
      var level = Number(value);
      var base = Number(fallback);
      if (!Number.isFinite(base)) base = 3;
      if (!Number.isFinite(level)) level = base;
      level = Math.round(level);
      if (level < 1) level = 1;
      if (level > 5) level = 5;
      return level;
    },
    modelPowerLevel: function(model) {
      return this.normalizeModelRating(model && model.power_rating, 3);
    },
    modelCostLevel: function(model) {
      return this.normalizeModelRating(model && model.cost_rating, 3);
    },
    modelContextWindowLabel: function(model) {
      var raw = Number(model && model.context_window != null ? model.context_window : 0);
      if (!Number.isFinite(raw) || raw <= 0) return '? ctx';
      return this.formatTokenK(raw) + ' ctx';
    },
    inferModelParamsFromId: function(model) {
      var id = String((model && (model.display_name || model.id)) || '').toLowerCase();
      if (!id) return 0;
      var pair = id.match(/([0-9]+(?:\.[0-9]+)?)x([0-9]+(?:\.[0-9]+)?)b/i);
      if (pair && pair[1] && pair[2]) {
        var left = Number(pair[1]);
        var right = Number(pair[2]);
        if (Number.isFinite(left) && Number.isFinite(right) && left > 0 && right > 0) return left * right;
      }
      var bMatch = id.match(/(?:^|[^a-z0-9])([0-9]+(?:\.[0-9]+)?)b(?:[^a-z0-9]|$)/i);
      if (bMatch && bMatch[1]) {
        var b = Number(bMatch[1]);
        if (Number.isFinite(b) && b > 0) return b;
      }
      var mMatch = id.match(/(?:^|[^a-z0-9])([0-9]{3,5})m(?:[^a-z0-9]|$)/i);
      if (mMatch && mMatch[1]) {
        var m = Number(mMatch[1]);
        if (Number.isFinite(m) && m > 0) return m / 1000;
      }
      return 0;
    },
    modelParamCountB: function(model) {
      var raw = Number(model && model.param_count_billion != null ? model.param_count_billion : 0);
      if (Number.isFinite(raw) && raw > 0) return raw;
      return this.inferModelParamsFromId(model);
    },
    modelParamLabel: function(model) {
      var params = this.modelParamCountB(model);
      if (!Number.isFinite(params) || params <= 0) return '? params';
      if (params >= 100) return Math.round(params) + 'B';
      if (params >= 10) return (Math.round(params * 10) / 10).toFixed(1).replace(/\.0$/, '') + 'B';
      if (params >= 1) return (Math.round(params * 100) / 100).toFixed(2).replace(/0$/, '').replace(/\.$/, '') + 'B';
      return Math.max(1, Math.round(params * 1000)) + 'M';
    },
    modelSpecialtyLabel: function(model) {
      var raw = String(model && model.specialty ? model.specialty : '').trim().toLowerCase();
      if (!raw) return 'General';
      if (raw === 'coding') return 'Coding';
      if (raw === 'reasoning') return 'Reasoning';
      if (raw === 'vision') return 'Vision';
      if (raw === 'speed') return 'Fast';
      return raw.charAt(0).toUpperCase() + raw.slice(1);
    },
    modelDownloadProgressValue: function(model) {
      var key = this.modelDownloadKey(model);
      if (!key || !this.modelDownloadProgress) return 0;
      var raw = Number(this.modelDownloadProgress[key] || 0);
      if (!Number.isFinite(raw) || raw <= 0) return 0;
      if (raw >= 100) return 100;
      return Math.max(1, Math.min(99, Math.round(raw)));
    },
    modelDownloadProgressStyle: function(model) {
      return 'width:' + this.modelDownloadProgressValue(model) + '%';
    },
    setModelDownloadProgress: function(key, value) {
      if (!key) return;
