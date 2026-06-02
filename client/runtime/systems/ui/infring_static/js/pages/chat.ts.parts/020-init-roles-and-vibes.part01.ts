      {
        id: 'direct',
        name: 'Direct',
        description: 'Straight to the point with minimal fluff.',
        system_suffix: 'Vibe directive: be direct and concise; prioritize concrete recommendations over exposition.',
      },
      {
        id: 'professional',
        name: 'Professional',
        description: 'Structured and businesslike.',
        system_suffix: 'Vibe directive: maintain a professional, structured tone suitable for business operations.',
      },
      {
        id: 'analytical',
        name: 'Analytical',
        description: 'Evidence-driven and detail-focused.',
        system_suffix: 'Vibe directive: reason from evidence, make assumptions explicit, and surface tradeoffs.',
      },
      {
        id: 'creative',
        name: 'Creative',
        description: 'Inventive and exploratory.',
        system_suffix: 'Vibe directive: propose inventive options and novel angles while staying practical.',
      }
    ],
    slashCommands: [
      { cmd: '/help', desc: 'Show available commands' },
      { cmd: '/agents', desc: 'Switch to Agents page' },
      { cmd: '/new', desc: 'Reset session (clear history)' },
      { cmd: '/compact', desc: 'Trigger LLM session compaction' },
      { cmd: '/model', desc: 'Show or switch model (/model [name])' },
      { cmd: '/apikey', desc: 'Add API key or local model path (/apikey [key|path])' },
      { cmd: '/file', desc: 'Render full file output in chat (/file [path])' },
      { cmd: '/folder', desc: 'Render folder tree + downloadable archive (/folder [path])' },
      { cmd: '/stop', desc: 'Cancel current agent run' },
      { cmd: '/usage', desc: 'Show session token usage & cost' },
      { cmd: '/think', desc: 'Toggle extended thinking (/think [on|off|stream])' },
      { cmd: '/context', desc: 'Show context window usage & pressure' },
      { cmd: '/verbose', desc: 'Cycle tool detail level (/verbose [off|on|full])' },
      { cmd: '/queue', desc: 'Check if agent is processing' },
      { cmd: '/status', desc: 'Show system status' },
      { cmd: '/alerts', desc: 'Show proactive telemetry alerts' },
      { cmd: '/next', desc: 'Show predicted next high-ROI actions' },
      { cmd: '/memory', desc: 'Show memory hygiene + cleanup recommendations' },
      { cmd: '/continuity', desc: 'Show pending actions across channels/sessions/tasks' },
      { cmd: '/aliases', desc: 'List active slash aliases' },
      { cmd: '/alias', desc: 'Create custom alias (/alias /short /target ...)' },
      { cmd: '/opt', desc: 'Suggest worker optimization / hibernation actions' },
      { cmd: '/clear', desc: 'Clear chat display' },
      { cmd: '/exit', desc: 'Disconnect from agent' },
      { cmd: '/budget', desc: 'Show spending limits and current costs' },
      { cmd: '/peers', desc: 'Show OFP peer network status' },
      { cmd: '/a2a', desc: 'List discovered external A2A agents' }
    ],
    tokenCount: 0,
    promptSuggestions: [],
    suggestionsLoading: false,
    promptSuggestionsEnabled: true,
    promptSuggestionsStorageKey: 'infring-chat-prompt-suggestions-enabled-v1',
    _suggestionFetchSeq: 0,
    _lastSuggestionsAt: 0,
    _lastSuggestionsAgentId: '',
    _pointerTrailLastAt: 0,
    _pointerTrailRaf: 0,
    _pointerTrailLastX: 0,
    _pointerTrailLastY: 0,
    _pointerTrailSeeded: false,
    _pointerTrailHeadLastAt: 0,
    _pointerOrbEl: null,
    _agentTrailRaf: 0,
    _agentTrailHost: null,
    _agentTrailState: null,
    _agentTrailLastAt: 0,
    _agentTrailLastDotAt: 0,
    _agentTrailSeeded: false,
    _agentTrailOrbEl: null,
    _agentTrailOrbElevated: false,
    _agentTrailTeleportTimer: 0,
    _agentTrailTeleportTargetX: NaN,
    _agentTrailTeleportTargetY: NaN,
    _agentTrailTeleportToggleIndex: true,
    _agentTrailListenTimer: 0,
    _agentTrailListening: false,
    chatResizeBlurActive: false,
    _chatResizeBlurTimer: 0,
    _chatResizeObserver: null,
    _chatResizeLastWidth: 0,
    _chatInputOverlayObserver: null,
    _chatInputOverlayResizeHandler: null,
    _progressCache: {},
    _freshInitThreadShownFor: '',
    _releaseCheckInFlight: false,
    _releaseUpdateNoticeKey: '',
    systemUpdateBusy: false,

    // ── Tip Bar ──
    tipIndex: 0,
    tips: ['Type / for commands', '/think on for reasoning', 'Ctrl+Shift+F for focus mode', 'Ctrl+T or Ctrl+\\ for terminal mode', 'Ctrl+F to add files', '/model to switch models', '/context to check usage', '/continuity to see pending work'],
    tipTimer: null,
    get currentTip() {
      if (localStorage.getItem('of-tips-off') === 'true') return '';
      return this.tips[this.tipIndex % this.tips.length];
    },
    dismissTips: function() { localStorage.setItem('of-tips-off', 'true'); },
    startTipCycle: function() {
      var self = this;
      if (this.tipTimer) clearInterval(this.tipTimer);
      this.tipTimer = setInterval(function() {
        self.tipIndex = (self.tipIndex + 1) % self.tips.length;
      }, 30000);
    },

    // Backward compat helper
    get thinkingEnabled() { return this.thinkingMode !== 'off'; },

    get terminalPromptPath() {
      return this.terminalCwd || '/workspace';
    },

    get terminalPromptPrefix() {
      return this.terminalPromptPath + ' % ';
    },

    get terminalPromptChars() {
      var len = this.terminalPromptPrefix.length;
      if (!Number.isFinite(len)) return 18;
      if (len < 18) return 18;
      return len;
    },

    get terminalCursorIndex() {
      var text = String(this.inputText || '');
      var max = text.length;
      var raw = Number(this.terminalSelectionStart);
      if (!Number.isFinite(raw)) return max;
      if (raw < 0) return 0;
      if (raw > max) return max;
      return Math.floor(raw);
    },

    get terminalCursorRow() {
      var text = String(this.inputText || '');
      if (!text) return 0;
      var upto = text.slice(0, this.terminalCursorIndex);
      var parts = upto.split('\n');
      return Math.max(0, parts.length - 1);
    },

    get terminalCursorColumn() {
      var text = String(this.inputText || '');
      if (!text) return 0;
      var upto = text.slice(0, this.terminalCursorIndex);
      var parts = upto.split('\n');
      return (parts[parts.length - 1] || '').length;
    },

    get terminalCursorStyle() {
      return '--terminal-cursor-ch:' + this.terminalCursorColumn +
        '; --terminal-cursor-row:' + this.terminalCursorRow + ';';
    },

    formatTokenThousands(value) {
      var raw = Number(value || 0);
      if (!Number.isFinite(raw) || raw <= 0) return '0k';
      var k = raw / 1000;
      if (k >= 100) return Math.round(k) + 'k';
      if (k >= 10) return (Math.round(k * 10) / 10).toFixed(1).replace(/\.0$/, '') + 'k';
      return (Math.round(k * 100) / 100).toFixed(2).replace(/0$/, '').replace(/\.$/, '') + 'k';
    },

    // Backward-compat shim for legacy callers during naming migration.
    formatTokenK(value) {
      return this.formatTokenThousands(value);
    },

    get contextUsagePercent() {
      var windowSize = Number(this.contextWindow || 0);
      var used = Number(this.contextApproxTokens || 0);
      if (windowSize > 0 && used >= 0) {
        var ratio = Math.round((used / windowSize) * 100);
        if (ratio < 0) return 0;
        if (ratio > 95) return 95;
        return ratio;
      }
      switch (this.contextPressure) {
        case 'critical': return 95;
        case 'high': return 80;
        case 'medium': return 55;
        default: return 25;
      }
    },

    get contextRingArcLength() {
      // 330deg sweep: starts at 1 o'clock and ends at 12 o'clock at 100%.
      var maxArc = 91.6667;
      var usage = this.contextUsagePercent;
      if (!Number.isFinite(usage) || usage <= 0) return 0;
      if (usage >= 100) return maxArc;
      return Number(((usage / 100) * maxArc).toFixed(3));
    },

    get contextRingProgressStyle() {
      return 'stroke-dasharray: ' + this.contextRingArcLength + ' 100; stroke-dashoffset: 0;';
    },

    get contextRingTooltip() {
      return 'Context window\n' +
        this.contextUsagePercent + '% full\n' +
        ' ' + this.formatTokenThousands(this.contextApproxTokens) + ' / ' + this.formatTokenThousands(this.contextWindow) + ' tokens used\n\n' +
        ' Infring dynamically prunes its context';
    },

    get contextRingCompactLabel() {
      return 'Context: ' + this.contextUsagePercent + '%, ' +
        this.formatTokenThousands(this.contextApproxTokens) + '/' + this.formatTokenThousands(this.contextWindow);
    },

    get activeGitBranchLabel() {
      var agentBranch = this.currentAgent && this.currentAgent.git_branch
        ? String(this.currentAgent.git_branch).trim()
        : '';
      if (agentBranch) return agentBranch;
      try {
        var store = Alpine.store('app');
        var branch = store && store.gitBranch ? String(store.gitBranch).trim() : '';
        return branch || '';
      } catch(_) {
        return '';
      }
    },

    get activeGitBranchMenuLabel() {
      var label = String(this.activeGitBranchLabel || '').trim();
      return label || 'main';
    },

    normalizeBranchName: function(value) {
      var raw = String(value == null ? '' : value).trim();
      if (!raw) return '';
      var normalized = raw
        .replace(/[^A-Za-z0-9._/-]+/g, '-')
        .replace(/\/+/g, '/')
        .replace(/^[-./]+|[-./]+$/g, '');
      return normalized;
    },

    closeComposerMenus: function(options) {
      var keep = options && typeof options === 'object' ? options : {};
      if (!keep.attach) this.showAttachMenu = false;
      if (!keep.model) this.showModelSwitcher = false;
      if (!keep.runtime) this.showRuntimeSwitcher = false;
      if (!keep.git) this.closeGitTreeMenu();
    },

    toggleAttachMenu: function() {
      var nextOpen = !this.showAttachMenu;
      this.closeComposerMenus(nextOpen ? { attach: true } : {});
      this.showAttachMenu = nextOpen;
    },

    closeGitTreeMenu: function() {
      this.showGitTreeMenu = false;
      this.gitTreeMenuError = '';
    },

    async refreshGitTreeMenu(force) {
      if (!this.currentAgent || !this.currentAgent.id) {
        this.gitTreeMenuItems = [];
        this.gitTreeMenuError = '';
        return;
      }
      if (!force && this.gitTreeMenuLoading) return;
      this.gitTreeMenuLoading = true;
      this.gitTreeMenuError = '';
      try {
        var payload = await InfringAPI.get('/api/agents/' + encodeURIComponent(this.currentAgent.id) + '/git-trees');
        var options = Array.isArray(payload && payload.options) ? payload.options : [];
        this.gitTreeMenuItems = options.map(function(row) {
          return {
            branch: String((row && row.branch) || '').trim(),
            current: !!(row && row.current),
            main: !!(row && row.main),
            kind: String((row && row.kind) || '').trim(),
            in_use_by_agents: Number((row && row.in_use_by_agents) || 0) || 0
          };
        }).filter(function(row) { return !!row.branch; });
        this.applyAgentGitTreeState(this.currentAgent, payload && payload.current ? payload.current : {});
      } catch (e) {
        this.gitTreeMenuItems = [];
        this.gitTreeMenuError = (e && e.message) ? String(e.message) : 'failed_to_load_git_trees';
      } finally {
        this.gitTreeMenuLoading = false;
      }
    },

    async toggleGitTreeMenu() {
      if (!this.currentAgent || !this.currentAgent.id) return;
      if (this.showGitTreeMenu) {
        this.closeGitTreeMenu();
        return;
      }
      this.closeComposerMenus({ git: true });
      this.showGitTreeMenu = true;
      await this.refreshGitTreeMenu(true);
    },

    async switchAgentGitTree(branchName, options) {
      if (!this.currentAgent || !this.currentAgent.id || this.gitTreeSwitching) return;
      var branch = this.normalizeBranchName(branchName);
      if (!branch) return;
      var requireNew = !!(options && options.requireNew === true);
      var current = this.normalizeBranchName(this.activeGitBranchLabel);
      if (!requireNew && current && current === branch) {
        this.closeGitTreeMenu();
        return;
      }
      this.gitTreeSwitching = true;
      this.gitTreeMenuError = '';
      try {
        var result = await InfringAPI.post(
          '/api/shell-socket/agents/' + encodeURIComponent(this.currentAgent.id) + '/git-tree',
          {
            branch: branch,
            require_new: requireNew
          }
        );
        this.applyAgentGitTreeState(this.currentAgent, result && result.current ? result.current : {});
        var store = Alpine.store('app');
        if (store && typeof store.refreshAgents === 'function') {
          await store.refreshAgents({ force: true });
        }
        await this.refreshGitTreeMenu(true);
        this.closeGitTreeMenu();
        InfringToast.success('Switched to branch ' + branch);
      } catch (e) {
        var message = (e && e.message) ? String(e.message) : 'git_tree_switch_failed';
        this.gitTreeMenuError = message;
        InfringToast.error('Git tree switch failed: ' + message);
      } finally {
        this.gitTreeSwitching = false;
      }
    },

    async createAndCheckoutGitBranch() {
      if (!this.currentAgent || !this.currentAgent.id || this.gitTreeSwitching) return;
      var suggested = this.normalizeBranchName('feature/' + String(this.currentAgent.id || '').trim().toLowerCase());
      var input = prompt('Create and checkout new branch:', suggested || 'feature/new-branch');
      if (input == null) return;
      var branch = this.normalizeBranchName(input);
      if (!branch) {
        InfringToast.error('Enter a valid branch name');
        return;
      }
      await this.switchAgentGitTree(branch, { requireNew: true });
    },

    get freshInitCanLaunch() {
      var selectedTemplate = this.freshInitTemplateDef;
      var hasTemplate = !!selectedTemplate;
      if (selectedTemplate && selectedTemplate.is_other) {
        hasTemplate = !!String(this.freshInitOtherPrompt || '').trim() && !this.freshInitAwaitingOtherPrompt;
      }
      return !!(
        this.showFreshArchetypeTiles &&
        !this.freshInitLaunching &&
        !this.freshInitAvatarUploading &&
        hasTemplate
      );
    },

    composerPlaceholder: function(includeCommandHint) {
      if (this.terminalMode) return this.terminalPromptPrefix;
      if (this.recording) return 'Recording... release to send';
      if (this.showFreshArchetypeTiles && this.isFreshInitComposerUnlocked()) {
        return this.freshInitOtherInputPlaceholder();
      }
      var base = this.currentAgent
        ? ('Message ' + (this.currentAgent.name || this.currentAgent.id || 'agent'))
        : 'Message agent';
      return includeCommandHint ? (base + '... (/ for commands)') : (base + '...');
    },

    isFreshInitComposerUnlocked: function() {
      return !!(
        this.showFreshArchetypeTiles &&
        !this.freshInitLaunching &&
        this.freshInitAwaitingOtherPrompt
      );
    },

    isFreshInitComposerLocked: function() {
      return !!(
        this.showFreshArchetypeTiles &&
        !this.freshInitLaunching &&
        !this.freshInitAwaitingOtherPrompt
      );
    },

    inputHistoryMode: function(explicitMode) {
      var mode = String(explicitMode || (this.terminalMode ? 'terminal' : 'chat')).trim().toLowerCase();
      return mode === 'terminal' ? 'terminal' : 'chat';
    },

    inputHistoryLimit: function() {
      var maxEntries = Number(this.inputHistoryMaxEntries || 0);
      if (!Number.isFinite(maxEntries) || maxEntries < 20) maxEntries = 120;
      if (maxEntries > 500) maxEntries = 500;
      return maxEntries;
    },

    normalizeInputHistoryEntry: function(value) {
      return String(value == null ? '' : value).trim();
    },

    normalizeInputHistoryRows: function(rows) {
      var source = Array.isArray(rows) ? rows : [];
      var clean = [];
      for (var i = 0; i < source.length; i += 1) {
        var item = this.normalizeInputHistoryEntry(source[i]);
        if (!item) continue;
        if (clean.length && clean[clean.length - 1] === item) continue;
        clean.push(item);
      }
      var maxEntries = this.inputHistoryLimit();
      if (clean.length > maxEntries) clean = clean.slice(clean.length - maxEntries);
      return clean;
    },

    inputHistoryLegacyAgentKey: function(explicitAgentId) {
      var direct = String(explicitAgentId || '').trim();
      if (direct) return direct;
      var active = this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : '';
      return String(active || '').trim();
    },

    inputHistorySessionScopeKey: function(explicitAgentId) {
      var agentId = this.inputHistoryLegacyAgentKey(explicitAgentId);
      if (!agentId) return '';
      var scopeKey = '';
      if (typeof this.resolveConversationCacheScopeKey === 'function') {
        try {
          scopeKey = String(this.resolveConversationCacheScopeKey(agentId) || '').trim();
        } catch (_) {
          scopeKey = '';
        }
      }
      if (!scopeKey) scopeKey = agentId + '|main';
      var prefix = String(this.inputHistorySessionScopePrefix || 'session:').trim() || 'session:';
      return prefix + scopeKey;
    },

    inputHistoryAgentKey: function(explicitAgentId) {
      var scoped = this.inputHistorySessionScopeKey(explicitAgentId);
      if (scoped) return scoped;
      return this.inputHistoryLegacyAgentKey(explicitAgentId);
    },

    inputHistoryBucketRows: function(cache, agentKey, legacyKey, mode) {
      var buckets = [];
      if (cache && agentKey && cache[agentKey] && typeof cache[agentKey] === 'object') {
        buckets.push(cache[agentKey]);
      }
      if (
        cache &&
        legacyKey &&
        legacyKey !== agentKey &&
        (!buckets.length || !Array.isArray(mode === 'terminal' ? buckets[0].terminal : buckets[0].chat) || !(mode === 'terminal' ? buckets[0].terminal : buckets[0].chat).length) &&
        cache[legacyKey] &&
        typeof cache[legacyKey] === 'object'
      ) {
        buckets.push(cache[legacyKey]);
      }
      for (var i = 0; i < buckets.length; i += 1) {
        var bucket = buckets[i];
        var rows = mode === 'terminal' ? bucket.terminal : bucket.chat;
        if (Array.isArray(rows) && rows.length) return this.normalizeInputHistoryRows(rows);
      }
      return [];
    },

    loadInputHistoryCache: function() {
      var empty = {};
      try {
        var raw = localStorage.getItem(this.inputHistoryCacheKey);
        if (!raw) {
          this._inputHistoryByAgent = empty;
          return;
        }
        var parsed = JSON.parse(raw);
        var next = parsed && typeof parsed === 'object' ? parsed : empty;
        var normalized = {};
        var keys = Object.keys(next);
        for (var i = 0; i < keys.length; i += 1) {
          var key = String(keys[i] || '').trim();
          if (!key) continue;
          var bucket = next[key];
          if (!bucket || typeof bucket !== 'object') continue;
          normalized[key] = {
            chat: this.normalizeInputHistoryRows(bucket.chat),
            terminal: this.normalizeInputHistoryRows(bucket.terminal),
            updated_at: Number(bucket.updated_at || 0) || 0,
          };
        }
        this._inputHistoryByAgent = normalized;
      } catch (_) {
        this._inputHistoryByAgent = empty;
      }
    },

    persistInputHistoryCache: function() {
      try {
        var payload = this._inputHistoryByAgent && typeof this._inputHistoryByAgent === 'object'
          ? this._inputHistoryByAgent
          : {};
        localStorage.setItem(this.inputHistoryCacheKey, JSON.stringify(payload));
      } catch (_) {}
    },
