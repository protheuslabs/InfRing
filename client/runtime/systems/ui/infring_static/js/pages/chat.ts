// Canonical Shell source-of-truth: assembled runtime chat surface.
// Decomposition debt lives under ./chat.ts.parts/** and must not count as additive production source.
// Infring Chat Page — Agent chat with markdown + streaming
'use strict';


// Infring Chat Page — Agent chat with markdown + streaming
'use strict';

function chatPage() {
  var msgId = 0;
  return {
    currentAgent: null,
    systemThreadId: 'system',
    systemThreadName: 'System',
    systemThreadEmoji: '\u2699\ufe0f',
    systemTerminalSessionId: '',
    messages: [],
    inputText: '',
    chatInputHistory: [],
    terminalInputHistory: [],
    chatInputHistoryCursor: -1,
    terminalInputHistoryCursor: -1,
    chatInputHistoryDraft: '',
    terminalInputHistoryDraft: '',
    inputHistoryMaxEntries: 500,
    inputHistoryCacheKey: 'infring-chat-input-history-v1',
    inputHistorySessionScopePrefix: 'session:',
    _inputHistoryByAgent: {},
    _inputHistoryApplying: false,
    sending: false,
    messageQueue: [],    // Queue for messages sent while streaming
    promptQueueDragId: '',
    _promptQueueSeq: 0,
    thinkingMode: 'off', // 'off' | 'on' | 'stream'
    _wsAgent: null,
    showAttachMenu: false,
    attachPickerSessionActive: false,
    attachPickerRestoreMode: 'send',
    _attachPickerFocusListener: null,
    _attachPickerFocusTimer: 0,
    showSlashMenu: false,
    slashFilter: '',
    slashIdx: 0,
    slashAliasMap: {},
    slashAliasStorageKey: 'infring-chat-slash-aliases-v1',
    agentRuntimeSlashCommandProjection: null,
    agentRuntimeSlashCommandRows: [],
    _agentRuntimeSlashCommandProjectionAt: 0,
    _agentRuntimeSlashCommandProjectionEngineId: '',
    _agentRuntimeSlashCommandProjectionLoading: false,
    attachments: [],
    pasteToMarkdownEnabled: true,
    pasteToMarkdownCharThreshold: 2000,
    pasteToMarkdownLineThreshold: 40,
    dragOver: false,
    contextPressure: 'low',
    contextWindow: 8192,
    contextApproxTokens: 0,
    terminalMode: false,
    terminalCwd: '/workspace',
    terminalShortcutHint: 'Ctrl+\\',
    terminalCursorFocused: false,
    terminalSelectionStart: 0,
    _contextTelemetryTimer: null,
    _telemetryAlertsTimer: null,
    _lastTelemetryAlertDigest: '',
    telemetryNextActions: [],
    _telemetrySnapshot: null,
    _lastContextRequestAt: 0,
    _contextWindowByModel: {},
    _contextModelsFetchedAt: 0,
    _typingTimeout: null,
    // Multi-session state
    sessions: [],
    sessionsOpen: false,
    searchOpen: false,
    searchQuery: '',
    messageDisplayInitialLimit: 10,
    messageDisplayStep: 5,
    messageDisplayCount: 10,
    messageTextRenderWindowRadius: 20,
    _messageDisplayKey: '',
    // Voice recording state
    recording: false,
    _mediaRecorder: null,
    _audioChunks: [],
    recordingTime: 0,
    _recordingTimer: null,
    // Model autocomplete state
    showModelPicker: false,
    modelPickerList: [],
    modelPickerFilter: '',
    modelPickerIdx: 0,
    // Model switcher dropdown
    showModelSwitcher: false,
    modelSwitcherFilter: '',
    modelSwitcherProviderFilter: '',
    modelSwitcherIdx: 0,
    showRuntimeSwitcher: false,
    runtimeEngineRows: [],
    runtimeEngineLoading: false,
    runtimeEngineError: '',
    runtimeEngineCacheTime: 0,
    activeWorkspaceProjection: null,
    activeWorkspaceLoading: false,
    activeWorkspacePicking: false,
    activeWorkspaceError: '',
    activeWorkspaceCacheTime: 0,
    selectedAgentRuntimeEngineId: 'infring_native',
    agentRuntimeEngineStorageKey: 'infring-selected-agent-runtime-engine-v1',
    agentRuntimePermissionPolicyStorageKey: 'infring-agent-runtime-permission-policy-v1',
    agentRuntimePermissionPolicy: null,
    pendingAgentRuntimePermissionRequests: [],
    showGitTreeMenu: false,
    gitTreeMenuLoading: false,
    gitTreeMenuError: '',
    gitTreeMenuItems: [],
    gitTreeSwitching: false,
    modelApiKeyInput: '',
    modelApiKeySaving: false,
    modelApiKeyStatus: '',
    modelDownloadBusy: {},
    modelDownloadProgress: {},
    _modelDownloadProgressTimers: {},
    modelSwitching: false,
    _modelCache: null,
    _modelCacheTime: 0,
    _modelLogoFailIndex: {},
    _modelSourceLogoFailIndex: {},
    _modelSwitcherViewCache: null,
    _chatMapWheelLockInstalled: false,
    sessionLoading: false,
    _sessionLoadSeq: 0,
    _hasMoreMessages: false,
    _messagePageOffset: 0,
    _olderMessagesLoading: false,
    toolPreviewMaxLines: 2,
    toolPreviewMaxChars: 100,
    messageHydration: {},
    messageHydrationReady: false,
    _forcedHydrateById: {},
    _renderWindowRaf: 0,
    showFreshArchetypeTiles: false,
    freshInitTemplateDef: null,
    freshInitTemplateName: '',
    freshInitName: '',
    freshInitEmoji: '',
    freshInitDefaultName: '',
    freshInitDefaultEmoji: '',
    freshInitLaunching: false,
    freshInitRevealMenu: false,
    freshInitStageToken: 0,
    freshInitAvatarUrl: '',
    freshInitAvatarUploading: false,
    freshInitAvatarUploadError: '',
    freshInitEmojiPickerOpen: false,
    freshInitEmojiSearch: '',
    freshInitOtherPrompt: '',
    freshInitAwaitingOtherPrompt: false,
    freshInitPersonalityId: 'none',
    freshInitLifespanId: '1h',
    freshInitAdvancedOpen: false,
    freshInitVibeId: 'none',
    freshInitModelSuggestions: [],
    freshInitModelSelection: '',
    freshInitModelManual: false,
    freshInitModelSuggestLoading: false,
    freshInitPermissionOverrides: {},
    freshInitPermissionCatalog: [
      { category: 'web', name: 'Web', permissions: [
        { key: 'web.search.basic', label: 'Basic web search', default_checked: true },
        { key: 'web.fetch.url', label: 'Fetch URL content', default_checked: false }
      ]},
      { category: 'agent', name: 'Agent', permissions: [
        { key: 'agent.spawn', label: 'Spawn child agents', default_checked: false },
        { key: 'agent.permissions.manage', label: 'Manage permissions', default_checked: false }
      ]},
      { category: 'file', name: 'File', permissions: [
        { key: 'file.read.workspace', label: 'Read workspace files', default_checked: false },
        { key: 'file.write.workspace', label: 'Write workspace files', default_checked: false },
        { key: 'file.delete.workspace', label: 'Delete workspace files', default_checked: false }
      ]},
      { category: 'github', name: 'GitHub', permissions: [
        { key: 'github.issue.create', label: 'Create GitHub issues', default_checked: false }
      ]},
      { category: 'terminal', name: 'Terminal', permissions: [
        { key: 'terminal.exec', label: 'Execute terminal commands', default_checked: false }
      ]},
      { category: 'memory', name: 'Memory', permissions: [
        { key: 'memory.write', label: 'Write durable memory', default_checked: false }
      ]}
    ],
    conversationCache: {},
    conversationCacheKey: 'of-chat-conversation-cache-v1',
    conversationCacheVersionKey: 'of-chat-conversation-cache-version',
    conversationCacheVersion: 'v3-preview-projection-20260515',
    messageLineExpandState: {},
    messageLineExpandStep: 20,
    _persistTimer: null,
    _responseStartedAt: 0,
    typingWordCadenceMs: 1,
    _pointerGridHideTimer: null,
    _pointerTrailMouseHeld: false,
    _pointerTrailHoldHost: null,
    _pointerTrailMouseUpHandler: null,
    _pendingAutoModelSwitchBaseline: '',
    _pendingWsRequest: null,
    _pendingWsRecovering: false,
    _wsConnectSeq: 0,
    _inflightPayload: null,
    _inflightFailoverInProgress: false,
    _sendWatchdogTimer: null,
    _continuitySnapshot: null,
    _hoverClearTimer: 0,
    modelNoticeCache: {},
    modelNoticeCacheKey: 'of-chat-model-notices-v1',
    modelUsageCache: {},
    modelUsageCacheKey: 'of-chat-model-usage-v1',
    showScrollDown: false,
    scrollBottomBufferPx: 84,
    scrollBottomFollowTolerancePx: 32,
    scrollBottomClampSlackPx: 16,
    _stickToBottom: true,
    hoveredMessageDomId: '',
    directHoveredMessageDomId: '',
    selectedMessageDomId: '',
    _systemMessageDedupeIndex: {},
    mapStepIndex: -1,
    suppressMapPreview: false,
    _mapPreviewSuppressTimer: null,
    _scrollSyncFrame: 0,
    _bottomClampTimer: 0,
    _lastMessagesScrollAt: 0,
    _openPinToken: 0,
    _openPinRaf: 0,
    _openPinTimer: 0,
    _lastPointerClientX: 0,
    _lastPointerClientY: 0,
    _lastInactiveNoticeKey: '',
    _agentMissingSince: 0,
    _agentMissingAgentId: '',
    _agentMissingGraceMs: 12000,
    collapsedMessageDays: {},
    showAgentDrawer: false,
    agentDrawerLoading: false,
    agentDrawer: null,
    drawerTab: 'info',
    drawerConfigForm: {},
    drawerConfigSaving: false,
    drawerModelSaving: false,
    drawerIdentitySaving: false,
    drawerSavePending: false,
    drawerEditingModel: false,
    drawerEditingProvider: false,
    drawerEditingFallback: false,
    drawerEditingName: false,
    drawerEditingEmoji: false,
    drawerEmojiPickerOpen: false,
    drawerEmojiSearch: '',
    drawerAvatarUrlPickerOpen: false,
    drawerAvatarUrlDraft: '',
    drawerAvatarUploading: false,
    drawerAvatarUploadError: '',
    drawerNewModelValue: '',
    drawerNewProviderValue: '',
    drawerNewFallbackValue: '',
    drawerEmojiCatalog: [
      { emoji: '🤖', name: 'robot' },
      { emoji: '🧠', name: 'brain' },
      { emoji: '🧑\u200d💻', name: 'developer' },
      { emoji: '🛠️', name: 'tools' },
      { emoji: '🔬', name: 'research' },
      { emoji: '🧪', name: 'experiment' },
      { emoji: '🛰️', name: 'signal' },
      { emoji: '📡', name: 'telemetry' },
      { emoji: '🚀', name: 'launch' },
      { emoji: '🧭', name: 'navigator' },
      { emoji: '🦾', name: 'strong arm' },
      { emoji: '🔐', name: 'security' },
      { emoji: '🛡️', name: 'shield' },
      { emoji: '📈', name: 'growth' },
      { emoji: '📊', name: 'analytics' },
      { emoji: '📝', name: 'writer' },
      { emoji: '🎯', name: 'target' },
      { emoji: '💡', name: 'idea' },
      { emoji: '🌐', name: 'network' },
      { emoji: '🧱', name: 'builder' },
      { emoji: '🧰', name: 'toolbox' },
      { emoji: '🦉', name: 'wise owl' },
      { emoji: '🔥', name: 'fire' }
    ],
    drawerArchetypeOptions: ['Assistant', 'Researcher', 'Coder', 'Writer', 'DevOps', 'Support', 'Analyst', 'Custom'],
    drawerVibeOptions: ['professional', 'friendly', 'technical', 'creative', 'concise', 'mentor'],
    chatArchetypeTemplates: [
      {
        name: 'General Assistant',
        category: 'General',
        description: 'Versatile helper for everyday tasks and recommendations.',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'full',
        archetype: 'assistant',
        system_prompt: 'You are a helpful, friendly assistant. Provide clear, accurate, and concise responses. Ask clarifying questions when needed.'
      },
      {
        name: 'Code Helper',
        category: 'Development',
        description: 'Programming-focused agent for writing, reviewing, and debugging.',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'coding',
        archetype: 'coder',
        system_prompt: 'You are an expert programmer. Help users write clean, efficient code. Explain your reasoning and follow best practices.'
      },
      {
        name: 'Researcher',
        category: 'Research',
        description: 'Analytical agent for complex topics and synthesis.',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'research',
        archetype: 'researcher',
        system_prompt: 'You are a research analyst. Break down complex topics with clear structure and concise findings.'
      },
      {
        name: 'Writer',
        category: 'Writing',
        description: 'Creative drafting, editing, and tone adaptation.',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'full',
        archetype: 'writer',
        system_prompt: 'You are a skilled writer and editor. Help users create polished content and offer constructive improvements.'
      },
      {
        name: 'Data Analyst',
        category: 'Development',
        description: 'Data analysis, SQL/Python queries, and interpretation.',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'coding',
        archetype: 'analyst',
        system_prompt: 'You are a data analysis expert. Help with dataset analysis, SQL/Python queries, and actionable interpretation.'
      },
      {
        name: 'DevOps Engineer',
        category: 'Development',
        description: 'CI/CD, infra, containers, and deployment reliability.',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'automation',
        archetype: 'devops',
        system_prompt: 'You are a DevOps engineer. Help with CI/CD pipelines, Docker, infrastructure, deployment, and reliability.'
      },
      {
        name: 'Customer Support',
        category: 'Business',
        description: 'Empathetic and professional issue resolution.',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'messaging',
        archetype: 'support',
        system_prompt: 'You are a professional support agent. Be empathetic, concise, and solution-oriented.'
      },
      {
        name: 'Tutor',
        category: 'General',
        description: 'Step-by-step explanations adapted to learner level.',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'full',
        archetype: 'assistant',
        system_prompt: 'You are a patient tutor. Explain step-by-step, check understanding, and adapt to learner pace.'
      },
      {
        name: 'API Designer',
        category: 'Development',
        description: 'REST design, schema consistency, and versioning.',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'coding',
        archetype: 'coder',
        system_prompt: 'You are an API design expert. Design clean RESTful APIs with robust schemas, error handling, and versioning.'
      },
      {
        name: 'Meeting Notes',
        category: 'Business',
        description: 'Summaries with decisions and action items.',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'minimal',
        archetype: 'analyst',
        system_prompt: 'You summarize meetings into key decisions, action items, highlights, and follow-up questions.'
      },
      {
        name: 'Other',
        category: 'General',
        description: 'Specify a special purpose.',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'full',
        archetype: 'custom',
        system_prompt: '',
        is_other: true
      }
    ],
    freshInitPersonalityCards: [
      {
        id: 'none',
        name: 'None',
        description: 'Use role defaults.',
        system_suffix: '',
        vibe: '',
      },
      {
        id: 'strategist',
        name: 'Strategist',
        description: 'Plan-first, structured, high-leverage.',
        system_suffix: 'Personality directive: be strategic, structured, and ROI-first. Offer concise plans and tradeoffs before action.',
        vibe: 'strategic',
      },
      {
        id: 'operator',
        name: 'Operator',
        description: 'Execution-focused, decisive, practical.',
        system_suffix: 'Personality directive: prioritize practical execution, clear next steps, and finish-line ownership.',
        vibe: 'operator',
      },
      {
        id: 'teacher',
        name: 'Teacher',
        description: 'Clear explanations, coaching-oriented.',
        system_suffix: 'Personality directive: teach with short, clear explanations and check understanding when useful.',
        vibe: 'mentor',
      },
      {
        id: 'creative',
        name: 'Creative',
        description: 'Idea-rich, exploratory, inventive.',
        system_suffix: 'Personality directive: be inventive and exploratory while staying concrete and useful.',
        vibe: 'creative',
      },
      {
        id: 'skeptic',
        name: 'Skeptic',
        description: 'Challenge assumptions, find weak points.',
        system_suffix: 'Personality directive: challenge assumptions, identify risks early, and pressure-test plans.',
        vibe: 'analytical',
      }
    ],
    freshInitLifespanCards: [
      {
        id: 'permanent',
        name: 'Permanent',
        description: 'No auto-expiry. Stays active until manual archive/revoke.',
        termination_condition: 'manual',
        indefinite: true,
        expiry_seconds: null,
      },
      {
        id: 'task',
        name: 'Until task is finished',
        description: 'Ends only when task is marked complete.',
        termination_condition: 'task_complete',
        indefinite: true,
        expiry_seconds: null,
      },
      {
        id: '1h',
        name: '1 hour',
        description: 'Auto-expires in 1 hour.',
        termination_condition: 'task_or_timeout',
        indefinite: false,
        expiry_seconds: 60 * 60,
      },
      {
        id: '1d',
        name: '1 day',
        description: 'Auto-expires in 1 day.',
        termination_condition: 'task_or_timeout',
        indefinite: false,
        expiry_seconds: 24 * 60 * 60,
      },
      {
        id: '1w',
        name: '1 week',
        description: 'Auto-expires in 1 week.',
        termination_condition: 'task_or_timeout',
        indefinite: false,
        expiry_seconds: 7 * 24 * 60 * 60,
      },
      {
        id: '1m',
        name: '1 month',
        description: 'Auto-expires in 1 month.',
        termination_condition: 'task_or_timeout',
        indefinite: false,
        expiry_seconds: 30 * 24 * 60 * 60,
      }
    ],
    freshInitVibeCards: [
      {
        id: 'none',
        name: 'None',
        description: 'Use role defaults.',
        system_suffix: '',
      },
      {
        id: 'friendly',
        name: 'Friendly',
        description: 'Warm, approachable, and collaborative.',
        system_suffix: 'Vibe directive: keep the tone warm, approachable, and encouraging while staying concise.',
      },

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


    hydrateInputHistoryFromCache: function(explicitMode, explicitAgentId) {
      var mode = this.inputHistoryMode(explicitMode);
      var rows = this.inputHistoryEntries(mode);
      if (!Array.isArray(rows)) return;
      var agentKey = this.inputHistoryAgentKey(explicitAgentId);
      if (!agentKey) return;
      var legacyKey = this.inputHistoryLegacyAgentKey(explicitAgentId);
      var cache = this._inputHistoryByAgent && typeof this._inputHistoryByAgent === 'object'
        ? this._inputHistoryByAgent
        : {};
      var cachedRows = this.inputHistoryBucketRows(cache, agentKey, legacyKey, mode);
      if (!Array.isArray(cachedRows) || !cachedRows.length) return;
      var merged = this.normalizeInputHistoryRows(rows.concat(cachedRows));
      if (mode === 'terminal') this.terminalInputHistory = merged;
      else this.chatInputHistory = merged;
    },

    syncInputHistoryToCache: function(explicitMode, explicitAgentId) {
      var mode = this.inputHistoryMode(explicitMode);
      var rows = this.inputHistoryEntries(mode);
      if (!Array.isArray(rows)) return;
      var agentKey = this.inputHistoryAgentKey(explicitAgentId);
      if (!agentKey) return;
      if (!this._inputHistoryByAgent || typeof this._inputHistoryByAgent !== 'object') {
        this._inputHistoryByAgent = {};
      }
      var bucket = this._inputHistoryByAgent[agentKey] && typeof this._inputHistoryByAgent[agentKey] === 'object'
        ? this._inputHistoryByAgent[agentKey]
        : {};
      var cleanRows = this.normalizeInputHistoryRows(rows);
      if (mode === 'terminal') bucket.terminal = cleanRows;
      else bucket.chat = cleanRows;
      bucket.updated_at = Date.now();
      this._inputHistoryByAgent[agentKey] = bucket;
      this.persistInputHistoryCache();
    },

    inputHistoryEntries: function(explicitMode) {
      var mode = this.inputHistoryMode(explicitMode);
      return mode === 'terminal' ? this.terminalInputHistory : this.chatInputHistory;
    },



    resetInputHistoryNavigation: function(explicitMode) {
      var mode = this.inputHistoryMode(explicitMode);
      if (mode === 'terminal') {
        this.terminalInputHistoryCursor = -1;
        this.terminalInputHistoryDraft = '';
        return;
      }
      this.chatInputHistoryCursor = -1;
      this.chatInputHistoryDraft = '';
    },

    pushInputHistoryEntry: function(explicitMode, rawText) {
      var text = this.normalizeInputHistoryEntry(rawText);
      if (!text) return;
      var mode = this.inputHistoryMode(explicitMode);
      var rows = this.inputHistoryEntries(mode);
      if (!Array.isArray(rows)) return;
      if (rows.length && String(rows[rows.length - 1] || '') === text) {
        this.resetInputHistoryNavigation(mode);
        return;
      }
      var nextRows = this.normalizeInputHistoryRows(rows.concat([text]));
      rows.splice(0, rows.length);
      for (var i = 0; i < nextRows.length; i += 1) rows.push(nextRows[i]);
      this.syncInputHistoryToCache(mode);
      this.resetInputHistoryNavigation(mode);
    },

    navigateInputHistory: function(direction, event) {
      var step = Number(direction || 0);
      if (!Number.isFinite(step) || step === 0) return false;
      var mode = this.inputHistoryMode();
      var rows = this.inputHistoryEntries(mode);
      if (!Array.isArray(rows) || !rows.length) return false;
      var cursor = mode === 'terminal' ? Number(this.terminalInputHistoryCursor || -1) : Number(this.chatInputHistoryCursor || -1);
      if (!Number.isFinite(cursor)) cursor = -1;
      var draft = mode === 'terminal'
        ? String(this.terminalInputHistoryDraft || '')
        : String(this.chatInputHistoryDraft || '');

      var nextText = '';
      if (step < 0) {
        if (cursor < 0) {
          draft = String(this.inputText || '');
          cursor = rows.length - 1;
        } else {
          cursor = Math.max(0, cursor - 1);
        }
        nextText = String(rows[cursor] || '');
      } else {
        if (cursor < 0) {
          return false;
        } else if (cursor >= rows.length - 1) {
          cursor = -1;
          nextText = draft;
        } else {
          cursor += 1;
          nextText = String(rows[cursor] || '');
        }
      }

      if (mode === 'terminal') {
        this.terminalInputHistoryCursor = cursor;
        this.terminalInputHistoryDraft = draft;
      } else {
        this.chatInputHistoryCursor = cursor;
        this.chatInputHistoryDraft = draft;
      }

      this._inputHistoryApplying = true;
      this.inputText = nextText;
      var self = this;
      this.$nextTick(function() {
        var el = document.getElementById('msg-input');
        if (el) {
          var pos = String(self.inputText || '').length;
          if (typeof el.setSelectionRange === 'function') {
            try { el.setSelectionRange(pos, pos); } catch(_) {}
          }
          el.style.height = 'auto';
          el.style.height = Math.min(el.scrollHeight, 150) + 'px';
        }
        if (self.terminalMode) self.updateTerminalCursor({ target: el });
        self._inputHistoryApplying = false;
      });
      if (event && typeof event.preventDefault === 'function') event.preventDefault();
      return true;
    },

    freshInitOtherInputPlaceholder: function() {
      var label = String(
        this.freshInitName ||
        (this.currentAgent && (this.currentAgent.name || this.currentAgent.id)) ||
        'agent'
      ).trim() || 'agent';
      return 'Tell ' + label + ' who they are...';
    },

    toggleFreshInitAdvanced: function() {
      this.freshInitAdvancedOpen = !this.freshInitAdvancedOpen;
    },

    defaultFreshInitPermissionChecked: function(permissionDef) {
      return !!(permissionDef && permissionDef.default_checked);
    },

    isFreshInitPermissionChecked: function(permissionDef) {
      var key = String(permissionDef && permissionDef.key ? permissionDef.key : '').trim();
      if (!key) return false;
      var overrides = this.freshInitPermissionOverrides && typeof this.freshInitPermissionOverrides === 'object'
        ? this.freshInitPermissionOverrides
        : {};
      if (Object.prototype.hasOwnProperty.call(overrides, key)) return !!overrides[key];
      return this.defaultFreshInitPermissionChecked(permissionDef);
    },

    setFreshInitPermissionChecked: function(permissionDef, checked) {
      var key = String(permissionDef && permissionDef.key ? permissionDef.key : '').trim();
      if (!key) return;
      if (!this.freshInitPermissionOverrides || typeof this.freshInitPermissionOverrides !== 'object') {
        this.freshInitPermissionOverrides = {};
      }
      this.freshInitPermissionOverrides[key] = !!checked;
    },

    setFreshInitPermissionCategory: function(categoryId, checked) {
      var category = String(categoryId || '').trim().toLowerCase();
      var rows = Array.isArray(this.freshInitPermissionCatalog) ? this.freshInitPermissionCatalog : [];
      for (var i = 0; i < rows.length; i += 1) {
        var row = rows[i] || {};
        if (String(row.category || '').trim().toLowerCase() !== category) continue;
        var perms = Array.isArray(row.permissions) ? row.permissions : [];
        for (var j = 0; j < perms.length; j += 1) this.setFreshInitPermissionChecked(perms[j], checked);
      }
    },

    resetFreshInitPermissions: function() {
      this.freshInitPermissionOverrides = {};
    },

    resolveFreshInitPermissionManifest: function() {
      var rows = Array.isArray(this.freshInitPermissionCatalog) ? this.freshInitPermissionCatalog : [];
      var grants = {};
      var categories = { agent: 'inherit', web: 'inherit', file: 'inherit', github: 'inherit', terminal: 'inherit', memory: 'inherit' };
      for (var i = 0; i < rows.length; i += 1) {
        var row = rows[i] || {};
        var perms = Array.isArray(row.permissions) ? row.permissions : [];
        for (var j = 0; j < perms.length; j += 1) {
          var permission = perms[j] || {};
          var key = String(permission.key || '').trim();
          if (!key) continue;
          grants[key] = this.isFreshInitPermissionChecked(permission) ? 'allow' : 'inherit';
        }
      }
      grants['web.search.basic'] = 'allow';
      return {
        version: 1,
        trit: { deny: -1, inherit: 0, allow: 1 },
        category_defaults: categories,
        grants: grants
      };
    },

    freshInitRoleKey: function(templateDef) {
      var template = templateDef || this.freshInitTemplateDef || {};
      var raw = String(template.archetype || template.name || '').trim().toLowerCase();
      if (!raw) return 'general';
      if (raw.indexOf('coder') >= 0 || raw.indexOf('devops') >= 0 || raw.indexOf('builder') >= 0 || raw.indexOf('api') >= 0) return 'coding';
      if (raw.indexOf('research') >= 0 || raw.indexOf('analyst') >= 0 || raw.indexOf('tutor') >= 0 || raw.indexOf('teacher') >= 0) return 'reasoning';
      if (raw.indexOf('writer') >= 0 || raw.indexOf('creative') >= 0) return 'creative';
      if (raw.indexOf('support') >= 0 || raw.indexOf('assistant') >= 0) return 'support';
      if (raw.indexOf('custom') >= 0 || raw.indexOf('other') >= 0) return 'general';
      return 'general';
    },

    freshInitModelName: function(model) {
      var row = model || {};
      var display = String(row.display_name || '').trim();
      var id = String(row.id || '').trim();
      if (display) return display;
      if (!id) return 'model';
      if (id.indexOf('/') >= 0) return id.split('/').slice(-1)[0];
      return id;
    },

    normalizeFreshInitModelRef: function(model) {
      var row = model || {};
      var id = String(row.id || '').trim();
      var provider = String(row.provider || '').trim().toLowerCase();
      if (id && id.toLowerCase() === 'auto') return '';
      if (id && id.indexOf('/') >= 0) return id;
      var name = this.freshInitModelName(row);
      if (provider && name) return provider + '/' + name;
      return id || name;
    },

    isFreshInitModelSuggestionSelected: function(model) {
      return this.normalizeFreshInitModelRef(model) === String(this.freshInitModelSelection || '').trim();
    },

    selectFreshInitModelSuggestion: function(model) {
      var ref = this.normalizeFreshInitModelRef(model);
      if (!ref) return;
      this.freshInitModelSelection = ref;
      this.freshInitModelManual = true;
      this.scheduleFreshInitProgressAnchor();
    },

    selectedFreshInitModelSuggestion: function() {
      var selected = String(this.freshInitModelSelection || '').trim();
      var rows = Array.isArray(this.freshInitModelSuggestions) ? this.freshInitModelSuggestions : [];

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
          if (!runtimeModelLabel) {
            runtimeModelLabel = String((runtimeEngineRow && (runtimeEngineRow.display_name || runtimeEngineRow.engine_id)) || runtimeEngineId || 'Agent runtime').trim();
          }
          var compactRuntimeModel = this.truncateModelLabel(runtimeModelLabel);
          if (compactRuntimeModel) return compactRuntimeModel.length > 24 ? compactRuntimeModel.substring(0, 22) + '\u2026' : compactRuntimeModel;
        }
        if (availableModels && !(availableModels.inherit_active_llm_when_unconfigured || availableModels.credential_inheritance_allowed || availableModels.source === 'inherited_infring')) {
          var runtimeLabel = String((runtimeEngineRow && (runtimeEngineRow.display_name || runtimeEngineRow.engine_id)) || runtimeEngineId || 'Agent runtime').trim();
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
    if (!menu || menu.show_in_llm_menu === false) return list;
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

      if (!this.modelDownloadProgress) this.modelDownloadProgress = {};
      var raw = Number(value);
      if (!Number.isFinite(raw)) raw = 0;
      raw = Math.max(0, Math.min(100, Math.round(raw)));
      if (raw <= 0) {
        delete this.modelDownloadProgress[key];
      } else {
        this.modelDownloadProgress[key] = raw;
      }
    },

    clearModelDownloadProgressTimer: function(key) {
      if (!key) return;
      if (!this._modelDownloadProgressTimers) this._modelDownloadProgressTimers = {};
      var timer = this._modelDownloadProgressTimers[key];
      if (timer) {
        clearInterval(timer);
      }
      delete this._modelDownloadProgressTimers[key];
    },

    startModelDownloadProgressTimer: function(key) {
      if (!key) return;
      this.clearModelDownloadProgressTimer(key);
      var self = this;
      var seeded = Number(self.modelDownloadProgress && self.modelDownloadProgress[key] ? self.modelDownloadProgress[key] : 0);
      if (!Number.isFinite(seeded) || seeded <= 0) seeded = 2;
      self.setModelDownloadProgress(key, seeded);
      self._modelDownloadProgressTimers[key] = setInterval(function() {
        var current = Number(self.modelDownloadProgress[key] || 0);
        if (!Number.isFinite(current) || current <= 0) current = 2;
        if (current >= 94) return;
        var bump = current < 30 ? 7 : (current < 60 ? 4 : 2);
        self.setModelDownloadProgress(key, Math.min(94, current + bump));
      }, 520);
    },

    modelPowerIcons: function(model) {
      return 'ϟ'.repeat(this.modelPowerLevel(model));
    },

    modelCostIcons: function(model) {
      return '$'.repeat(this.modelCostLevel(model));
    },

    modelDownloadKey: function(model) {
      var row = model || {};
      var provider = String(row.provider || '').trim().toLowerCase();
      var id = String(row.id || row.display_name || '').trim().toLowerCase();
      return provider + '::' + id;
    },

    isModelDownloadable: function(model) {
      var row = model || {};
      var id = String(row.id || row.model || '').trim();
      var provider = String(row.provider || '').trim().toLowerCase();
      return !!(
        row &&
        (
          row.download_available === true ||
          String(row.local_download_path || '').trim() ||
          (id && provider && provider !== 'auto')
        )
      );
    },

    isModelDownloadBusy: function(model) {
      var key = this.modelDownloadKey(model);
      return !!(key && this.modelDownloadBusy && this.modelDownloadBusy[key] === true);
    },

    downloadModelToLocal: function(model) {
      var self = this;
      var row = model || {};
      if (!self.isModelDownloadable(row)) {
        InfringToast.error('No local download path is available for this model');
        return;
      }
      var key = self.modelDownloadKey(row);
      if (!key) return;
      if (!self.modelDownloadBusy) self.modelDownloadBusy = {};
      if (self.modelDownloadBusy[key]) return;
      self.modelDownloadBusy[key] = true;
      self.setModelDownloadProgress(key, 2);
      self.startModelDownloadProgressTimer(key);
      var modelRef = String(row.id || row.display_name || '').trim();
      var provider = String(row.provider || '').trim();
      InfringAPI.post('/api/shell-socket/models/download', {
        model: modelRef,
        provider: provider
      }).then(function(resp) {
        var method = String((resp && resp.method) || '').trim();
        var localPath = String((resp && resp.download_path) || '').trim();
        self.setModelDownloadProgress(key, 100);
        if (method === 'ollama_pull') {
          InfringToast.success('Model downloaded locally: ' + localPath);
          self.addNoticeEvent({
            notice_label: 'Downloaded ' + (String(row.display_name || row.id || 'model').trim()) + ' locally',
            notice_type: 'info',
            notice_icon: '⬇',
            ts: Date.now(),
          });
        } else {
          InfringToast.success('Local download path prepared: ' + localPath);
          self.addNoticeEvent({
            notice_label: 'Prepared local download path for ' + (String(row.display_name || row.id || 'model').trim()),
            notice_type: 'info',
            notice_icon: '⬇',
            ts: Date.now(),
          });
        }
        self._modelCache = null;
        self._modelCacheTime = 0;
        return InfringAPI.get('/api/shell-socket/models');
      }).then(function(data) {
        var models = self.sanitizeModelCatalogRows((data && data.models) || []);
        self._modelCache = models;
        self._modelCacheTime = Date.now();
        self.modelPickerList = models;
      }).catch(function(e) {
        InfringToast.error('Model download failed: ' + (e && e.message ? e.message : e));
        self.setModelDownloadProgress(key, 0);
      }).finally(function() {
        self.modelDownloadBusy[key] = false;
        self.clearModelDownloadProgressTimer(key);
        if (self.modelDownloadProgress && self.modelDownloadProgress[key] >= 100) {
          setTimeout(function() {
            self.setModelDownloadProgress(key, 0);
          }, 900);
        } else {
          self.setModelDownloadProgress(key, 0);
        }
      });
    },

    pickDefaultAgent(agents) {
      if (!Array.isArray(agents) || !agents.length) return null;
      // Prefer the master/default agent when present; otherwise first running agent.
      var i;
      for (i = 0; i < agents.length; i++) {
        var a = agents[i] || {};
        if (this.isSystemThreadAgent(a)) continue;
        var text = ((a.id || '') + ' ' + (a.name || '') + ' ' + (a.role || '')).toLowerCase();
        if (text.indexOf('master') >= 0 || text.indexOf('default') >= 0 || text.indexOf('primary') >= 0) {
          return a;
        }
      }
      for (i = 0; i < agents.length; i++) {
        var b = agents[i] || {};
        if (this.isSystemThreadAgent(b)) continue;
        if (String(b.state || '').toLowerCase() === 'running') return b;
      }
      for (i = 0; i < agents.length; i++) {
        if (!this.isSystemThreadAgent(agents[i])) return agents[i];
      }
      return null;
    },

    isSystemThreadId(agentId) {
      var target = String(agentId || '').trim().toLowerCase();
      var systemId = String(this.systemThreadId || 'system').trim().toLowerCase();
      if (!target) return false;
      return target === systemId;
    },

    isSystemThreadAgent(agent) {
      if (!agent || typeof agent !== 'object') return false;
      if (agent.is_system_thread === true) return true;
      return this.isSystemThreadId(agent.id);
    },

    isSystemThreadActive() {
      return this.isSystemThreadAgent(this.currentAgent);
    },

    isReservedSystemEmoji(rawEmoji) {
      var normalized = String(rawEmoji || '').replace(/\uFE0F/g, '').trim();
      return normalized === '⚙';
    },

    sanitizeAgentEmojiForDisplay(agentRef, rawEmoji) {
      var emoji = String(rawEmoji || '').trim();
      var isSystem = this.isSystemThreadAgent(agentRef);
      if (isSystem) {
        return String(this.systemThreadEmoji || '\u2699\ufe0f').trim() || '\u2699\ufe0f';
      }
      if (this.isReservedSystemEmoji(emoji)) return '';
      return emoji;
    },

    displayAgentEmoji(agentRef) {
      var agent = agentRef && typeof agentRef === 'object' ? agentRef : this.currentAgent;
      if (!agent) return '';
      var emoji = String((agent.identity && agent.identity.emoji) || '').trim();
      return this.sanitizeAgentEmojiForDisplay(agent, emoji);
    },

    isArchivedAgentRecord(agent) {
      if (!agent || typeof agent !== 'object') return false;
      var store = Alpine.store('app');
      if (store && typeof store.isArchivedLikeAgent === 'function' && store.isArchivedLikeAgent(agent)) return true;
      if (agent.archived === true) return true;
      var state = String(agent.state || '').trim().toLowerCase();
      if (state.indexOf('archived') >= 0 || state.indexOf('inactive') >= 0 || state.indexOf('terminated') >= 0) return true;
      var contract = agent.contract && typeof agent.contract === 'object' ? agent.contract : null;
      var contractStatus = String(contract && contract.status ? contract.status : '').trim().toLowerCase();
      return contractStatus.indexOf('archived') >= 0 || contractStatus.indexOf('inactive') >= 0 || contractStatus.indexOf('terminated') >= 0;
    },

    isCurrentAgentArchived() {
      return this.isArchivedAgentRecord(this.currentAgent);
    },

    makeSystemThreadAgent() {
      var id = String(this.systemThreadId || 'system').trim() || 'system';
      var name = String(this.systemThreadName || 'System').trim() || 'System';
      var emoji = String(this.systemThreadEmoji || '\u2699\ufe0f').trim() || '\u2699\ufe0f';
      return {
        id: id,
        name: name,
        state: 'running',
        role: 'system',
        is_system_thread: true,
        model_provider: 'system',
        model_name: 'terminal',
        runtime_model: 'terminal',
        identity: { emoji: emoji },
        created_at: new Date(0).toISOString(),
        updated_at: new Date().toISOString(),
        auto_terminate_allowed: false,
      };
    },

    resolveAgent(agentOrId) {
      if (!agentOrId) return null;
      var id = typeof agentOrId === 'string' ? agentOrId : agentOrId.id;
      if (!id) return null;
      if (this.isSystemThreadId(id)) return this.makeSystemThreadAgent();
      var store = Alpine.store('app');
      var list = (store && store.agents) || [];
      for (var i = 0; i < list.length; i++) {
        if (list[i] && String(list[i].id) === String(id)) return list[i];
      }
      if (store && store.pendingAgent && String(store.pendingFreshAgentId || '') === String(id)) {
        return store.pendingAgent;
      }
      if (store && store.pendingAgent && String((store.pendingAgent && store.pendingAgent.id) || '') === String(id) && this.isArchivedAgentRecord(store.pendingAgent)) {
        return store.pendingAgent;
      }
      if (typeof agentOrId === 'object' && agentOrId.id && this.isArchivedAgentRecord(agentOrId)) {
        return agentOrId;
      }
      // Only trust stale object references while the store has no live agent list yet.
      if (!list.length && typeof agentOrId === 'object' && agentOrId.id) return agentOrId;
      return null;
    },

    ensureValidCurrentAgent: function(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var store = Alpine.store('app');
      if (this.currentAgent && this.isSystemThreadAgent(this.currentAgent)) {
        this.currentAgent = this.makeSystemThreadAgent();
        return this.currentAgent;
      }
      var rows = store && Array.isArray(store.agents) ? store.agents : [];
      var currentId = this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : '';
      var currentLive = currentId ? this.resolveAgent(currentId) : null;
      if (!currentLive && this.currentAgent && this.isArchivedAgentRecord(this.currentAgent)) {
        return this.currentAgent;
      }
      if (currentLive) {
        if (!this.currentAgent || String(this.currentAgent.id || '') !== String(currentLive.id || '')) {
          this.currentAgent = currentLive;
        } else {
          this.syncCurrentAgentFromStore(currentLive);
        }
        return this.currentAgent;
      }
      var preferred = null;
      if (store && store.activeAgentId) preferred = this.resolveAgent(store.activeAgentId);
      if (!preferred && rows.length) preferred = this.pickDefaultAgent(rows);
      if (preferred) {
        this.selectAgent(preferred);
        return this.resolveAgent(preferred.id || preferred) || preferred;
      }
      if (opts.clear_when_missing) this.currentAgent = null;
      return null;
    },

    refreshAgentRosterAuthoritative: async function() {
      var store = Alpine.store('app');
      var rows = await InfringAPI.get('/api/agents?view=sidebar&authority=runtime');
      var list = (Array.isArray(rows) ? rows : []).filter((row) => {
        if (!row || !row.id) return false;
        return !(this.isArchivedAgentRecord && this.isArchivedAgentRecord(row));
      });
      if (store) {
        store.agents = list;
        store.agentsHydrated = true;
        store.agentsLoading = false;
        store.agentsLastError = '';
        store.agentCount = list.length;
        store._lastAgentsRefreshAt = Date.now();
        if (store.activeAgentId) {
          var stillActive = list.some(function(row) {
            return !!(row && String(row.id || '') === String(store.activeAgentId || ''));
          });
          if (!stillActive && store.pendingAgent && String((store.pendingAgent && store.pendingAgent.id) || '') === String(store.activeAgentId || '') && this.isArchivedAgentRecord(store.pendingAgent)) {
            stillActive = true;
          }
          if (!stillActive && String(store.activeAgentId || '').trim().toLowerCase() === String(this.systemThreadId || 'system').trim().toLowerCase()) {
            stillActive = true;
          }
          if (!stillActive) {
            if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(null);
            else store.activeAgentId = null;
          }
        }
      }
      return list;
    },

    rebindCurrentAgentAuthoritative: async function(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var preferredId = String(opts.preferred_id || '').trim();
      var clearWhenMissing = opts.clear_when_missing !== false;
      var store = Alpine.store('app');
      var rows = [];
      try {
        rows = await this.refreshAgentRosterAuthoritative();
      } catch (_) {
        rows = store && Array.isArray(store.agents) ? store.agents : [];
      }

      var rebound = null;
      if (preferredId) {
        rebound = this.resolveAgent(preferredId);
        if (!rebound && Array.isArray(rows)) {
          var lowerPreferred = preferredId.toLowerCase();
          for (var i = 0; i < rows.length; i++) {
            var row = rows[i];
            if (!row || !row.id) continue;
            if (String(row.id).toLowerCase() === lowerPreferred) {
              rebound = row;
              break;
            }
          }
        }
      }
      if (!rebound && store && store.activeAgentId) rebound = this.resolveAgent(store.activeAgentId);
      if (!rebound && Array.isArray(rows) && rows.length) rebound = this.pickDefaultAgent(rows);

      if (rebound && rebound.id) {
        var resolved = this.resolveAgent(rebound.id) || rebound;
        if (this.isSystemThreadAgent(resolved)) {
          InfringAPI.wsDisconnect();
          this._wsAgent = null;
          this.currentAgent = this.makeSystemThreadAgent();
          this.setStoreActiveAgentId(this.currentAgent.id || null);
          return this.currentAgent;
        }
        this.currentAgent = this.applyAgentGitTreeState(resolved, resolved) || resolved;
        this.setStoreActiveAgentId(this.currentAgent.id || null);
        if (this.currentAgent && this.currentAgent.id) {
          var reboundId = String(this.currentAgent.id);
          if (String(this._wsAgent || '') !== reboundId || !InfringAPI.isWsConnected()) {
            this._wsAgent = null;
            this.connectWs(reboundId);
          }
        }
        return this.currentAgent;
      }

      if (clearWhenMissing) {
        this.currentAgent = null;
        this.setStoreActiveAgentId(null);
      }
      return null;
    },

    applyAgentGitTreeState(targetAgent, sourceState) {
      var target = targetAgent && typeof targetAgent === 'object' ? targetAgent : null;
      var source = sourceState && typeof sourceState === 'object' ? sourceState : null;
      if (!target || !source) return target;
      if (Object.prototype.hasOwnProperty.call(source, 'git_branch')) {
        var branch = source.git_branch ? String(source.git_branch).trim() : '';
        if (branch) target.git_branch = branch;
      }
      if (Object.prototype.hasOwnProperty.call(source, 'git_tree_kind')) {
        target.git_tree_kind = source.git_tree_kind ? String(source.git_tree_kind).trim() : '';
      }
      if (Object.prototype.hasOwnProperty.call(source, 'workspace_dir')) {
        var workspace = source.workspace_dir ? String(source.workspace_dir).trim() : '';
        if (workspace) {
          target.workspace_dir = workspace;
        }
      }
      if (Object.prototype.hasOwnProperty.call(source, 'workspace_rel')) {
        target.workspace_rel = source.workspace_rel ? String(source.workspace_rel).trim() : '';
      }
      if (Object.prototype.hasOwnProperty.call(source, 'git_tree_ready')) {
        target.git_tree_ready = !!source.git_tree_ready;
      }
      if (Object.prototype.hasOwnProperty.call(source, 'git_tree_error')) {
        target.git_tree_error = source.git_tree_error ? String(source.git_tree_error).trim() : '';
      }
      if (Object.prototype.hasOwnProperty.call(source, 'is_master_agent')) {
        target.is_master_agent = !!source.is_master_agent;
      }
      return target;
    },

    syncCurrentAgentFromStore: function(sourceAgent) {
      var source = sourceAgent && typeof sourceAgent === 'object' ? sourceAgent : null;
      if (!source || !this.currentAgent || !this.currentAgent.id) return false;
      if (String(this.currentAgent.id) !== String(source.id)) return false;
      this.applyAgentGitTreeState(this.currentAgent, source);
      var keys = Object.keys(source);
      for (var i = 0; i < keys.length; i++) {
        var key = keys[i];
        if (key === 'id') continue;
        this.currentAgent[key] = source[key];
      }
      return true;
    },

    setStoreActiveAgentId: function(agentId) {
      var store = Alpine.store('app');
      if (!store) return;
      if (typeof store.setActiveAgentId === 'function') {
        store.setActiveAgentId(agentId || null);
        return;
      }
      store.activeAgentId = agentId || null;
      try {
        if (store.activeAgentId) localStorage.setItem('infring-last-active-agent-id', String(store.activeAgentId));
        else localStorage.removeItem('infring-last-active-agent-id');
      } catch {}
    },

    resolveConversationInputMode(agentId) {
      var key = String(agentId || '').trim();
      if (!key) return 'chat';
      if (this.isSystemThreadId(key)) return 'terminal';
      var cached = this.conversationCache && this.conversationCache[key];
      return cached && cached.default_terminal === true ? 'terminal' : 'chat';
    },

    currentConversationInputMode(agentId) {
      if (this.isSystemThreadId(agentId)) return 'terminal';
      return this.terminalMode ? 'terminal' : 'chat';
    },

    applyConversationInputMode(agentId, options) {
      var opts = options && typeof options === 'object' ? options : {};
      var hasForced = Object.prototype.hasOwnProperty.call(opts, 'force_terminal');
      var mode = this.resolveConversationInputMode(agentId);
      if (hasForced) mode = opts.force_terminal === true ? 'terminal' : 'chat';
      if (this.isSystemThreadId(agentId)) mode = 'terminal';
      this.terminalMode = mode === 'terminal';
      this.showSlashMenu = false;
      this.showModelPicker = false;
      this.showModelSwitcher = false;
      this.terminalCursorFocused = false;
      if (!this.terminalMode) this.terminalSelectionStart = 0;
      if (this.terminalMode && !this.terminalCwd) this.terminalCwd = '/workspace';
      return mode;
    },

    sanitizeConversationDraftText(rawText) {
      var text = String(rawText == null ? '' : rawText);
      if (!text) return '';
      if (text.length > 12000) text = text.slice(0, 12000);
      var trimmed = text.trim();
      if (!trimmed) return '';
      if (/^message\s+.+\.\.\.(?:\s+\(\/\s*for commands\))?$/i.test(trimmed)) return '';
      if (/^tell\s+.+\.\.\.$/i.test(trimmed)) return '';
      return text;
    },

    conversationCacheMaxEntries: function() {
      return 20;
    },

    pruneConversationCacheEntries: function() {
      if (!this.conversationCache || typeof this.conversationCache !== 'object') return;
      var keys = Object.keys(this.conversationCache || {});
      var maxEntries = Number(this.conversationCacheMaxEntries ? this.conversationCacheMaxEntries() : 20);
      if (!Number.isFinite(maxEntries) || maxEntries < 1) maxEntries = 20;
      if (keys.length <= maxEntries) return;
      keys.sort((left, right) => {
        var a = this.conversationCache[left] && typeof this.conversationCache[left] === 'object'
          ? Number(this.conversationCache[left].saved_at || 0)
          : 0;
        var b = this.conversationCache[right] && typeof this.conversationCache[right] === 'object'
          ? Number(this.conversationCache[right].saved_at || 0)
          : 0;
        return b - a;
      });
      var next = {};
      for (var i = 0; i < keys.length && i < maxEntries; i += 1) {
        next[keys[i]] = this.conversationCache[keys[i]];
      }
      this.conversationCache = next;
    },

    touchConversationCacheEntry: function(agentId, patch) {
      var key = String(agentId || '').trim();
      if (!key) return null;
      if (!this.conversationCache || typeof this.conversationCache !== 'object') this.conversationCache = {};
      var prior = this.conversationCache[key] && typeof this.conversationCache[key] === 'object'
        ? this.conversationCache[key]
        : {};
      var next = Object.assign({}, prior, patch || {}, { saved_at: Date.now() });
      this.conversationCache[key] = next;
      this.pruneConversationCacheEntries();
      return this.conversationCache[key];
    },

    captureConversationDraft(agentId, explicitMode) {
      var key = String(agentId || '').trim();
      if (!key) return;
      if (!this.conversationCache) this.conversationCache = {};
      var mode = String(explicitMode || this.currentConversationInputMode(key) || 'chat').trim().toLowerCase();
      if (mode !== 'terminal') mode = 'chat';
      var next = this.touchConversationCacheEntry(key) || {};
      var scopeKey = typeof this.resolveConversationCacheScopeKey === 'function'
        ? this.resolveConversationCacheScopeKey(key)
        : key;
      next.session_scope_key = scopeKey;
      var sanitized = this.sanitizeConversationDraftText(this.inputText);
      if (mode === 'terminal') next.draft_terminal = sanitized;
      else next.draft_chat = sanitized;
      this.conversationCache[key] = next;
      this.persistConversationCache();
    },

    restoreConversationDraft(agentId, explicitMode) {
      var key = String(agentId || '').trim();
      if (!key || !this.conversationCache) {
        this.inputText = '';
        return '';
      }
      var cached = this.conversationCache[key];
      if (!cached || typeof cached !== 'object') {
        this.inputText = '';
        return '';
      }
      var scopeKey = typeof this.resolveConversationCacheScopeKey === 'function'
        ? this.resolveConversationCacheScopeKey(key)
        : key;
      var cachedScopeKey = String(cached.session_scope_key || '').trim();
      if (scopeKey && cachedScopeKey && scopeKey !== cachedScopeKey) {
        this.inputText = '';
        return '';
      }
      var mode = String(explicitMode || this.currentConversationInputMode(key) || 'chat').trim().toLowerCase();
      if (mode !== 'terminal') mode = 'chat';
      var raw = mode === 'terminal' ? cached.draft_terminal : cached.draft_chat;
      var nextText = this.sanitizeConversationDraftText(raw);
      this.touchConversationCacheEntry(key);
      this.inputText = nextText;
      var self = this;
      this.$nextTick(function() {
        var el = document.getElementById('msg-input');
        if (!el) return;
        el.style.height = 'auto';
        el.style.height = Math.min(el.scrollHeight, 150) + 'px';
        if (self.terminalMode) self.updateTerminalCursor({ target: el });
      });
      return nextText;
    },

    cacheAgentConversation(agentId) {
      if (!agentId) return;
      if (!this.conversationCache) this.conversationCache = {};
      try {
        var key = String(agentId);
        var scopeKey = typeof this.resolveConversationCacheScopeKey === 'function'
          ? this.resolveConversationCacheScopeKey(agentId)
          : key;
        var currentSessionRow = typeof this.resolveCurrentSessionRow === 'function'
          ? this.resolveCurrentSessionRow(agentId)
          : null;
        var cachedMessages = this.sanitizeConversationForCache(this.messages || []);
        var next = Object.assign(
          {},
          this.touchConversationCacheEntry(key),
          {
          saved_at: Date.now(),
          session_scope_key: scopeKey,
          session_label: typeof this.resolveSessionRowLabel === 'function'
            ? this.resolveSessionRowLabel(currentSessionRow, agentId)
            : '',
          token_count: this.tokenCount || 0,
          default_terminal: this.currentConversationInputMode(agentId) === 'terminal',
          messages: cachedMessages,
          }
        );
        var mode = this.currentConversationInputMode(agentId);
        var draft = this.sanitizeConversationDraftText(this.inputText);
        if (mode === 'terminal') next.draft_terminal = draft;
        else next.draft_chat = draft;
        this.conversationCache[key] = next;
        var appStore = Alpine.store('app');
        if (appStore && typeof appStore.saveAgentChatPreview === 'function') {
          appStore.saveAgentChatPreview(agentId, this.conversationCache[key].messages);
        }
        this.persistConversationCache();
      } catch {}
    },

    cacheCurrentConversation() {
      if (!this.currentAgent || !this.currentAgent.id) return;
      this.cacheAgentConversation(this.currentAgent.id);
    },

    scheduleConversationPersist() {

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

// Layer ownership: client/runtime/systems/ui (dashboard static UX surface only; no runtime authority).
          var hasContextOverlap = contextKeywords.some(function(keyword) {
            return loweredRaw.indexOf(keyword) >= 0;
          });
          if (!hasContextOverlap) continue;
        }
        var key = String(raw || '').toLowerCase();
        if (seen[key]) continue;
        var duplicate = out.some(function(existing) {
          return isNearDuplicate(existing, raw);
        });
        if (duplicate) continue;
        seen[key] = true;
        out.push(raw);
        if (out.length >= 3) break;
      }
      return out;
    },

    derivePromptSuggestionFallback(agent, hint, gateContext) {
      var context = this.buildPromptSuggestionContextSnapshot();
      var typedHistory = (Array.isArray(context.history) ? context.history : [])
        .map(function(entry, index) {
          var role = String((entry && entry.role) || 'agent').trim().toLowerCase();
          if (role === 'assistant') role = 'agent';
          return {
            key: 'fallback:' + String(index),
            kind: role === 'user' || role === 'agent' ? 'message' : 'synthetic',
            role: role,
            text: String((entry && entry.text) || '').trim()
          };
        })
        .filter(function(entry) {
          return entry.kind === 'message' && !!entry.text;
        });
      var corpus = [
        String(hint || ''),
        String(gateContext || ''),
        String(context.lastUser || ''),
        String(context.lastAgent || ''),
        typedHistory.slice(-3).map(function(entry) { return entry.role + ':' + entry.text; }).join(' || ')
      ].join(' || ').toLowerCase();
      var out = [];
      var add = function(value) {
        var text = String(value || '').trim();
        if (text) out.push(text);
      };
      if (/(connect|pair|token|auth|unauthorized|secure context|device identity|gateway|fetch failed)/.test(corpus)) {
        add('Summarize the fastest recovery step for this connection error');
        add('/apikey');
        add('/help');
      }
      if (/(model|provider|fallback|failover|slow|thinking|reasoning)/.test(corpus)) {
        add('/model');
        add('Continue from the last successful step with a safer model');
      }
      if (/(voice|audio|microphone|dictat|record)/.test(corpus) || this.recording) {
        add('Turn the latest voice note into a concise prompt');
        add('Summarize this chat into a one-line handoff note');
      }
      if (/(agent|session|chat|thread|roster|branch)/.test(corpus) || !typedHistory.length) {
        add('/agents');
        add('/new');
      }
      if (!out.length) {
        add('Give me the next best action from this conversation');
        add('/help');
        add('/model');
      }
      return this.normalizePromptSuggestions(
        out,
        String(gateContext || context.signature || '').trim(),
        this.recentUserSuggestionSamples()
      );
    },

    buildPromptSuggestionContextSnapshot() {
      var out = { lastUser: '', lastAgent: '', history: [], signature: '' };
      var history = Array.isArray(this.messages) ? this.messages : [];
      var compact = function(value, maxLen) {
        var cap = Number(maxLen || 240);
        var text = String(value == null ? '' : value).replace(/\s+/g, ' ').trim();
        if (!text) return '';
        if (text.length > cap) return text.substring(0, Math.max(8, cap - 3)) + '...';
        return text;
      };
	      for (var i = history.length - 1; i >= 0; i--) {
	        var row = history[i];
	        if (!row || row.thinking || row.streaming || row.terminal || row.is_notice) continue;
	        var normalizedRole = compact(row.role || '', 16).toLowerCase();
	        if (!normalizedRole) {
	          normalizedRole = row.user ? 'user' : row.assistant ? 'agent' : 'agent';
	        }
	        if (normalizedRole === 'system') continue;
	        var text = compact(row.text, 240);
	        if (!text) continue;
	        if (/^\[runtime-task\]/i.test(text)) continue;
	        if (/task accepted\.\s*report findings in this thread with receipt-backed evidence/i.test(text)) continue;
	        if (/the user wants exactly 3 actionable next user prompts/i.test(text)) continue;
	        if (String(text || '').toLowerCase() === 'heartbeat_ok') continue;
	        if (out.history.length < 7) {
	          out.history.unshift({
	            role: normalizedRole,
	            text: text
	          });
	        }
	        if (!out.lastUser && normalizedRole === 'user') {
	          out.lastUser = text;
	          continue;
	        }
	        if (!out.lastAgent && (normalizedRole === 'agent' || normalizedRole === 'assistant')) {
	          out.lastAgent = text;
	        }
	        if (out.lastUser && out.lastAgent && out.history.length >= 7) break;
	      }
      if (out.history.length > 7) out.history = out.history.slice(-7);
      out.signature = compact(
        out.history
          .map(function(entry) {
            return compact(entry.role || 'agent', 20) + ':' + compact(entry.text || '', 180);
          })
          .join(' || ') ||
          (String(out.lastUser || '') + '|' + String(out.lastAgent || '')),
        1200
      );
      return out;
    },

    // Backward-compat shim for legacy callers during naming migration.
    collectPromptSuggestionContext() {
      return this.buildPromptSuggestionContextSnapshot();
    },

    recentUserSuggestionSamples() {
      var history = Array.isArray(this.messages) ? this.messages : [];
      var out = [];
      for (var i = history.length - 1; i >= 0; i--) {
        var row = history[i];
        if (!row || row.thinking || row.streaming || row.terminal || row.is_notice) continue;
        if (String(row.role || '').toLowerCase() !== 'user') continue;
        var text = String(row.text == null ? '' : row.text).replace(/\s+/g, ' ').trim();
        if (!text) continue;
        out.unshift(text);
        if (out.length >= 7) break;
      }
      return out;
    },

    hasConversationSuggestionSeed() {
      if (this.isSystemThreadAgent && this.isSystemThreadAgent(this.currentAgent)) return false;
      var context = this.buildPromptSuggestionContextSnapshot();
      var count = Array.isArray(context && context.history) ? context.history.length : 0;
      return count >= 7;
    },

    nextPromptQueueId() {
      this._promptQueueSeq = Number(this._promptQueueSeq || 0) + 1;
      return 'pq-' + String(Date.now()) + '-' + String(this._promptQueueSeq);
    },

    get promptQueueItems() {
      var queue = Array.isArray(this.messageQueue) ? this.messageQueue : [];
      var out = [];
      for (var i = 0; i < queue.length; i++) {
        var row = queue[i];
        if (!row || row.terminal) continue;
        if (!row.queue_id) row.queue_id = this.nextPromptQueueId();
        if (!row.queue_kind) row.queue_kind = 'prompt';
        out.push({
          queue_id: String(row.queue_id),
          queue_index: i,
          text: String(row.text || '').trim(),
          files: Array.isArray(row.files) ? row.files : [],
          images: Array.isArray(row.images) ? row.images : [],
          queued_at: Number(row.queued_at || 0) || Date.now()
        });
      }
      return out;
    },

    get hasPromptQueue() {
      return Array.isArray(this.promptQueueItems) && this.promptQueueItems.length > 0;
    },

    queuePromptPreview(item) {
      var text = String(item && item.text ? item.text : '').replace(/\s+/g, ' ').trim();
      if (!text) return '(queued prompt)';
      return text.length > 140 ? text.substring(0, 137) + '...' : text;
    },

    promptQueueSteerActionLabel(item) {
      var engineId = String(this.selectedAgentRuntimeEngineId || 'infring_native').trim() || 'infring_native';
      var usesRuntimeSocket = typeof this.shouldUseAgentRuntimeSocketPath === 'function'
        ? this.shouldUseAgentRuntimeSocketPath(engineId)
        : false;
      if (!this.sending) return 'Send';
      if (!usesRuntimeSocket) return 'Steer';
      var row = typeof this.activeRuntimeEngineRow === 'function' ? this.activeRuntimeEngineRow() : null;
      if (row && row.supports_live_steering === true) return 'Steer now';
      return 'Queue steer';
    },

    removePromptQueueItem(queueId) {
      var id = String(queueId || '').trim();
      if (!id) return;
      var rows = Array.isArray(this.messageQueue) ? this.messageQueue : [];
      var idx = rows.findIndex(function(row) {
        return !!(row && String(row.queue_id || '').trim() === id);
      });
      if (idx < 0) return;
      rows.splice(idx, 1);
      this.messageQueue = rows.slice();
      if (!this.hasPromptQueue && !this.sending && this.currentAgent) {
        this.refreshPromptSuggestions(false, 'queue-cleared');
      }
      this.scheduleConversationPersist();
    },

    movePromptQueueItem(sourceId, targetId) {
      var src = String(sourceId || '').trim();
      var dst = String(targetId || '').trim();
      if (!src || !dst || src === dst) return;
      var rows = Array.isArray(this.messageQueue) ? this.messageQueue.slice() : [];
      var srcIdx = rows.findIndex(function(row) { return !!(row && String(row.queue_id || '').trim() === src); });
      var dstIdx = rows.findIndex(function(row) { return !!(row && String(row.queue_id || '').trim() === dst); });
      if (srcIdx < 0 || dstIdx < 0) return;
      var moving = rows[srcIdx];
      rows.splice(srcIdx, 1);
      if (dstIdx > srcIdx) dstIdx -= 1;
      rows.splice(dstIdx, 0, moving);
      this.messageQueue = rows;
      this.scheduleConversationPersist();
    },

    onPromptQueueDragStart(queueId, event) {
      var id = String(queueId || '').trim();
      if (!id) return;
      this.promptQueueDragId = id;
      if (event && event.dataTransfer) {
        event.dataTransfer.effectAllowed = 'move';
        try { event.dataTransfer.setData('text/plain', id); } catch(_) {}
      }
    },

    onPromptQueueDrop(targetId, event) {
      if (event && typeof event.preventDefault === 'function') event.preventDefault();
      var sourceId = String(this.promptQueueDragId || '').trim();
      if (!sourceId && event && event.dataTransfer) {
        try {
          sourceId = String(event.dataTransfer.getData('text/plain') || '').trim();
        } catch(_) {}
      }
      var destinationId = String(targetId || '').trim();
      if (sourceId && destinationId) {
        this.movePromptQueueItem(sourceId, destinationId);
      }
      this.promptQueueDragId = '';
    },

    onPromptQueueDragEnd() {
      this.promptQueueDragId = '';
    },

    async steerPromptQueueItem(queueId) {
      var id = String(queueId || '').trim();
      if (!id) return;
      var rows = Array.isArray(this.messageQueue) ? this.messageQueue : [];
      var idx = rows.findIndex(function(row) {
        return !!(row && String(row.queue_id || '').trim() === id);
      });
      if (idx < 0) return;
      var item = rows[idx];
      rows.splice(idx, 1);
      this.messageQueue = rows.slice();
      var text = String(item && item.text ? item.text : '').trim();
      var files = Array.isArray(item && item.files) ? item.files : [];
      var images = Array.isArray(item && item.images) ? item.images : [];
      if (!text) return;
      this.messages.push({
        id: ++msgId,
        role: 'system',
        is_notice: true,
        notice_type: 'info',
        notice_label: 'Steer injected into active workflow.',
        text: 'Steer injected into active workflow.',
        meta: '',
        tools: [],
        system_origin: 'prompt_queue:steer',
        ts: Date.now(),
      });
      this.scrollToBottom();
      this.scheduleConversationPersist();

      if (!this.sending) {
        var liveAgent = this.ensureValidCurrentAgent({ clear_when_missing: true });
        if (!liveAgent || !liveAgent.id) return;
        this.appendUserChatMessage(text, images, { deferPersist: true });
        this.scheduleConversationPersist();
        this._sendPayload(text, files, images, {
          agent_id: liveAgent.id,
          steer_injected: true,
          from_queue: true,
          queue_id: id
        });
        return;
      }

      var selectedRuntimeEngineId = String(this.selectedAgentRuntimeEngineId || 'infring_native').trim() || 'infring_native';
      var usesAgentRuntimeSocket = typeof this.shouldUseAgentRuntimeSocketPath === 'function'
        ? this.shouldUseAgentRuntimeSocketPath(selectedRuntimeEngineId)
        : false;
      if (usesAgentRuntimeSocket) {
        var runtimeAgent = this.ensureValidCurrentAgent({ clear_when_missing: true });
        if (!runtimeAgent || !runtimeAgent.id) return;
        try {
          var steerAck = await InfringAPI.post('/api/shell-socket/agent-runtime/steer', {
            engine_id: selectedRuntimeEngineId,
            agent_id: runtimeAgent.id,
            session_id: String((runtimeAgent && (runtimeAgent.session_id || runtimeAgent.id)) || runtimeAgent.id || ''),
            text: text,
            attachments: files,
            mode: 'auto',
            priority: 'steer',
          });
          this.appendUserChatMessage(text, images, { deferPersist: true });
          this.messages.push({
            id: ++msgId,
            role: 'system',
            is_notice: true,
            notice_type: 'info',
            notice_label: steerAck && steerAck.live_injected ? 'Steer injected into active runtime turn.' : 'Steer queued for next runtime turn.',
            text: steerAck && steerAck.live_injected ? 'Steer injected into active runtime turn.' : 'Steer queued for next runtime turn.',
            meta: '',
            tools: [],
            system_origin: 'prompt_queue:agent_runtime_steer',
            ts: Date.now(),
          });
          this.scrollToBottom();
          this.scheduleConversationPersist();
          return;
        } catch(_) {}
      }

      var wsPayload = { type: 'message', content: text, steer: true, priority: 'steer' };
      if (files.length) wsPayload.attachments = files;
      if (InfringAPI.wsSend(wsPayload)) {
        this.appendUserChatMessage(text, images, { deferPersist: true });
        this.scheduleConversationPersist();
        return;
      }

      if (this.currentAgent && this.currentAgent.id) {
        var reboundAgent = this.ensureValidCurrentAgent({ clear_when_missing: true });
        if (!reboundAgent || !reboundAgent.id) return;
        try {
          await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(reboundAgent.id) + '/message', {
            message: text,
            attachments: files,
            steer: true,
            priority: 'steer',
          });
          this.appendUserChatMessage(text, images, { deferPersist: true });
          this.scheduleConversationPersist();
          return;
        } catch(_) {}
      }

      this.messageQueue.unshift({
        queue_id: id,
        queue_kind: 'prompt',
        text: text,
        files: files,
        images: images,
        queued_at: Number(item && item.queued_at ? item.queued_at : Date.now()),
      });
      this.messages.push({
        id: ++msgId,
        role: 'system',
        is_notice: true,
        notice_type: 'error',
        notice_label: 'Steer injection failed; prompt returned to queue.',
        text: 'Steer injection failed; prompt returned to queue.',
        meta: '',
        tools: [],
        system_origin: 'prompt_queue:steer',
        ts: Date.now(),
      });
      this.scrollToBottom();
      this.scheduleConversationPersist();
    },

    loadPromptSuggestionsPreference() {
      var key = String(this.promptSuggestionsStorageKey || '').trim();
      if (!key) return;
      try {
        var raw = localStorage.getItem(key);
        if (raw == null) return;
        var normalized = String(raw).trim().toLowerCase();
        this.promptSuggestionsEnabled = !(
          normalized === '0' ||
          normalized === 'false' ||
          normalized === 'off' ||
          normalized === 'no'
        );
      } catch (_) {}
    },

    persistPromptSuggestionsPreference() {
      var key = String(this.promptSuggestionsStorageKey || '').trim();
      if (!key) return;
      try {
        localStorage.setItem(key, this.promptSuggestionsEnabled ? '1' : '0');
      } catch (_) {}
    },

    setPromptSuggestionsEnabled(enabled) {
      this.promptSuggestionsEnabled = enabled !== false;
      this.persistPromptSuggestionsPreference();
      if (!this.promptSuggestionsEnabled) {
        this.clearPromptSuggestions();
        return;
      }
      this.refreshPromptSuggestions(true, 'toggle-enabled');
    },

    togglePromptSuggestionsEnabled() {
      this.setPromptSuggestionsEnabled(!this.promptSuggestionsEnabled);
    },

    clearPromptSuggestions() {
      this.promptSuggestions = [];
      this.suggestionsLoading = false;
      this._lastSuggestionsAt = 0;
      this._lastSuggestionsAgentId = '';
    },

    async applyPromptSuggestion(suggestion) {
      var text = String(suggestion == null ? '' : suggestion).trim();
      if (!text) return;
      this.inputText = text;
      this.showSlashMenu = false;
      this.showModelPicker = false;
      this.showAttachMenu = false;
      this._pendingPromptSuggestionSend = {
        source: 'prompt_suggestion',
        text_preview: text.slice(0, 240),
        created_at: Date.now()
      };
      try {
        await this.sendMessage();
      } finally {
        this._pendingPromptSuggestionSend = null;
      }
    },

    promptSuggestionNeedsResize(chip) {
      if (!chip) return false;
      var wasExpanded = chip.classList.contains('is-expanded');
      var wasResizing = chip.classList.contains('is-resizing');
      if (wasExpanded) chip.classList.remove('is-expanded');
      if (wasResizing) chip.classList.remove('is-resizing');
      var needs = false;
      try {
        var text = chip.querySelector('.prompt-suggestion-chip-text');
        if (text) needs = (Number(text.scrollWidth || 0) - Number(text.clientWidth || 0)) > 1;
        if (!needs) needs = (Number(chip.scrollWidth || 0) - Number(chip.clientWidth || 0)) > 1;
      } catch(_) {}
      if (wasExpanded) chip.classList.add('is-expanded');
      if (wasResizing) chip.classList.add('is-resizing');
      return !!needs;
    },

    onPromptSuggestionHoverIn(event) {
      if (!event || !event.currentTarget) return;
      var chip = event.currentTarget;
      if (chip._resizeBlurTimer) {
        clearTimeout(chip._resizeBlurTimer);
        chip._resizeBlurTimer = 0;
      }
      if (!this.promptSuggestionNeedsResize(chip)) {
        chip.classList.remove('is-expanded');
        chip.classList.remove('is-resizing');
        return;
      }
      chip.classList.add('is-expanded');
      chip.classList.add('is-resizing');
      chip._resizeBlurTimer = setTimeout(function() {

// FILE_SIZE_EXCEPTION: reason=chat pointer fx split continuity owner=jay expires=2026-06-30
// Layer ownership: client/runtime/systems/ui (dashboard static UX surface only; no runtime authority).
        try {
          chip.classList.remove('is-resizing');
          chip._resizeBlurTimer = 0;
        } catch(_) {}
      }, 65);
    },

    onPromptSuggestionHoverOut(event) {
      if (!event || !event.currentTarget) return;
      var chip = event.currentTarget;
      if (chip._resizeBlurTimer) {
        clearTimeout(chip._resizeBlurTimer);
        chip._resizeBlurTimer = 0;
      }
      chip.classList.remove('is-resizing');
      chip.classList.remove('is-expanded');
    },

    triggerChatResizeBlurPulse(durationMs) {
      this.chatResizeBlurActive = true;
      if (this._chatResizeBlurTimer) {
        clearTimeout(this._chatResizeBlurTimer);
        this._chatResizeBlurTimer = 0;
      }
      var duration = Number(durationMs || 140);
      if (!Number.isFinite(duration) || duration < 60) duration = 140;
      var self = this;
      this._chatResizeBlurTimer = setTimeout(function() {
        self._chatResizeBlurTimer = 0;
        self.chatResizeBlurActive = false;
      }, Math.round(duration));
    },

    teardownChatResizeBlurObserver() {
      if (this._chatResizeObserver && typeof this._chatResizeObserver.disconnect === 'function') {
        try { this._chatResizeObserver.disconnect(); } catch(_) {}
      }
      this._chatResizeObserver = null;
      if (this._chatResizeBlurTimer) {
        clearTimeout(this._chatResizeBlurTimer);
        this._chatResizeBlurTimer = 0;
      }
      this.chatResizeBlurActive = false;
    },

    installChatResizeBlurObserver() {
      this.teardownChatResizeBlurObserver();
      if (typeof ResizeObserver !== 'function') return;
      var host = this.$el || null;
      if (!host || typeof host.getBoundingClientRect !== 'function') return;
      var self = this;
      this._chatResizeLastWidth = Math.round(Number(host.getBoundingClientRect().width || 0));
      this._chatResizeObserver = new ResizeObserver(function(entries) {
        var entry = entries && entries.length ? entries[0] : null;
        if (!entry) return;
        var width = Math.round(Number((entry.contentRect && entry.contentRect.width) || host.getBoundingClientRect().width || 0));
        if (!Number.isFinite(width) || width <= 0) return;
        var previous = Number(self._chatResizeLastWidth || 0);
        self._chatResizeLastWidth = width;
        if (previous <= 0) return;
        if (Math.abs(width - previous) < 2) return;
        self.triggerChatResizeBlurPulse();
      });
      this._chatResizeObserver.observe(host);
    },

    refreshChatInputOverlayMetrics() {
      var host = this.$el || null;
      if (!host || typeof host.querySelector !== 'function' || !host.style) return;
      var inputArea = host.querySelector('.input-area');
      if (!inputArea || inputArea.offsetParent === null) {
        host.style.setProperty('--chat-input-overlay-height', '0px');
        host.style.setProperty('--chat-input-bottom-reserve', '136px');
        return;
      }
      var lane = inputArea.querySelector('.chat-input-lane');
      var areaRect = typeof inputArea.getBoundingClientRect === 'function' ? inputArea.getBoundingClientRect() : null;
      var laneRect = lane && typeof lane.getBoundingClientRect === 'function' ? lane.getBoundingClientRect() : null;
      var measured = Math.max(
        Number(areaRect && areaRect.height ? areaRect.height : 0),
        Number(laneRect && laneRect.height ? laneRect.height : 0)
      );
      if (!Number.isFinite(measured) || measured < 0) measured = 0;
      var overlayHeight = Math.ceil(measured);
      var reserve = overlayHeight > 0 ? (overlayHeight + 20) : 136;
      host.style.setProperty('--chat-input-overlay-height', overlayHeight + 'px');
      host.style.setProperty('--chat-input-bottom-reserve', reserve + 'px');
    },

    teardownChatInputOverlayObserver() {
      if (this._chatInputOverlayObserver && typeof this._chatInputOverlayObserver.disconnect === 'function') {
        try { this._chatInputOverlayObserver.disconnect(); } catch(_) {}
      }
      this._chatInputOverlayObserver = null;
      if (this._chatInputOverlayResizeHandler) {
        try { window.removeEventListener('resize', this._chatInputOverlayResizeHandler); } catch(_) {}
      }
      this._chatInputOverlayResizeHandler = null;
    },

    installChatInputOverlayObserver() {
      this.teardownChatInputOverlayObserver();
      var host = this.$el || null;
      if (!host || typeof host.querySelector !== 'function') return;
      var inputArea = host.querySelector('.input-area');
      this.refreshChatInputOverlayMetrics();
      if (!inputArea) return;
      var self = this;
      if (typeof ResizeObserver === 'function') {
        this._chatInputOverlayObserver = new ResizeObserver(function() {
          self.refreshChatInputOverlayMetrics();
        });
        try { this._chatInputOverlayObserver.observe(inputArea); } catch(_) {}
        var inputLaneEl = inputArea.querySelector('.chat-input-lane');
        if (inputLaneEl) {
          try { this._chatInputOverlayObserver.observe(inputLaneEl); } catch(_) {}
        }
      }
      this._chatInputOverlayResizeHandler = function() {
        self.refreshChatInputOverlayMetrics();
      };
      try { window.addEventListener('resize', this._chatInputOverlayResizeHandler, { passive: true }); } catch(_) {
        window.addEventListener('resize', this._chatInputOverlayResizeHandler);
      }
    },

    async refreshPromptSuggestions(force, hint) {
      var agent = this.currentAgent;
      if (!agent || !agent.id) {
        this.promptSuggestions = [];
        return;
      }
      if (!this.promptSuggestionsEnabled) {
        this.promptSuggestions = [];
        this.suggestionsLoading = false;
        return;
      }
      if (this.terminalMode || this.showFreshArchetypeTiles) {
        this.promptSuggestions = [];
        return;
      }
      if (this.hasPromptQueue) {
        this.promptSuggestions = [];
        this.suggestionsLoading = false;
        return;
      }
      if (!this.hasConversationSuggestionSeed()) {
        this.promptSuggestions = [];
        this.suggestionsLoading = false;
        return;
      }
      var now = Date.now();
      var agentId = String(agent.id);
      var suggestionScopeKey = agentId + '|main';
      if (typeof this.resolveConversationCacheScopeKey === 'function') {
        try {
          var resolvedScopeKey = String(this.resolveConversationCacheScopeKey(agentId) || '').trim();
          if (resolvedScopeKey) suggestionScopeKey = resolvedScopeKey;
        } catch (_) {}
      }
      var recentlyFetched =
        !force &&
        this._lastSuggestionsAgentId === suggestionScopeKey &&
        (now - Number(this._lastSuggestionsAt || 0)) < 12000 &&
        Array.isArray(this.promptSuggestions) &&
        this.promptSuggestions.length > 0;
      if (recentlyFetched) return;

      var seq = Number(this._suggestionFetchSeq || 0) + 1;
      this._suggestionFetchSeq = seq;
      this.suggestionsLoading = true;
	      try {
	        var payload = {};
	        var context = this.collectPromptSuggestionContext();
	        if (context.signature) payload.recent_context = String(context.signature).trim();
          if (suggestionScopeKey) payload.session_scope_key = suggestionScopeKey;
	        var result = await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(agentId) + '/suggestions', payload);
	        if (this._suggestionFetchSeq !== seq) return;
	        var freshContext = this.collectPromptSuggestionContext();
	        var freshHistoryCount = Array.isArray(freshContext.history) ? freshContext.history.length : 0;
	        if (freshHistoryCount < 7) {
	          this.promptSuggestions = [];
	          this._lastSuggestionsAt = Date.now();
	          this._lastSuggestionsAgentId = suggestionScopeKey;
	          return;
	        }
	        var gatingContext = String(context.signature || '');
	        var baseSuggestions = result && result.suggestions ? result.suggestions : [];
	        var suggestions = this.normalizePromptSuggestions(
	          Array.isArray(baseSuggestions) ? baseSuggestions : [],
	          gatingContext,
	          this.recentUserSuggestionSamples()
	        );
        this.promptSuggestions = suggestions;
        this._lastSuggestionsAt = Date.now();
        this._lastSuggestionsAgentId = suggestionScopeKey;
	      } catch (_) {
		        if (this._suggestionFetchSeq === seq) {
		          var fallbackContext = this.collectPromptSuggestionContext();
		          var fallbackHistoryCount = Array.isArray(fallbackContext.history) ? fallbackContext.history.length : 0;
		          if (fallbackHistoryCount < 7) {
		            this.promptSuggestions = [];
		            this._lastSuggestionsAt = Date.now();
		            this._lastSuggestionsAgentId = suggestionScopeKey;
		            return;
		          }
		          this.promptSuggestions = this.derivePromptSuggestionFallback(agent, hint, String(fallbackContext.signature || ''));
          this._lastSuggestionsAt = Date.now();
          this._lastSuggestionsAgentId = suggestionScopeKey;
        }
      } finally {
        if (this._suggestionFetchSeq === seq) this.suggestionsLoading = false;
      }
    },

    resetFreshInitStateForAgent: function(agentRef) {
      var agent = agentRef && typeof agentRef === 'object' ? agentRef : {};
      var seedName = String(agent.name || agent.id || '').trim() || String(agent.id || '').trim();
      var seedEmoji = String((agent.identity && agent.identity.emoji) || '').trim();
      this.showFreshArchetypeTiles = false;
      this.freshInitRevealMenu = false;
      this.freshInitTemplateDef = null;
      this.freshInitTemplateName = '';
      this.freshInitLaunching = false;
      this.freshInitName = '';
      this.freshInitEmoji = '';
      this.freshInitDefaultName = seedName;
      this.freshInitDefaultEmoji = seedEmoji;
      this.freshInitAvatarUrl = String(agent.avatar_url || '').trim();
      this.freshInitAvatarUploading = false;
      this.freshInitAvatarUploadError = '';
      this.freshInitEmojiPickerOpen = false;
      this.freshInitEmojiSearch = '';
      this.freshInitOtherPrompt = '';
      this.freshInitAwaitingOtherPrompt = false;
      this.freshInitPersonalityId = 'none';
      this.freshInitLifespanId = '1h';
      this.freshInitAdvancedOpen = false;
      this.freshInitVibeId = 'none';
      this.freshInitModelSuggestions = [];
      this.freshInitModelSelection = '';
      this.freshInitModelManual = false;
      this.freshInitModelSuggestLoading = false; if (typeof this.resetFreshInitPermissions === 'function') this.resetFreshInitPermissions();
    },

    focusChatComposerFromInit: function(seedText) {
      var self = this;
      var text = seedText == null ? null : String(seedText);
      this.$nextTick(function() {
        var el = document.getElementById('msg-input');
        if (!el) return;
        if (text != null) {
          self.inputText = text;
        }
        el.focus();
        try {
          var cursor = String(self.inputText || '').length;
          el.setSelectionRange(cursor, cursor);
        } catch (_) {}
        el.style.height = 'auto';
        el.style.height = Math.min(el.scrollHeight, 150) + 'px';
      });
    },

    startFreshInitSequence(agent) {
      if (!agent || !agent.id) return;
      var agentId = String(agent.id);
      var token = Number(this.freshInitStageToken || 0) + 1;
      this.freshInitStageToken = token;
      this._freshInitThreadShownFor = agentId;
      this.resetFreshInitStateForAgent(agent);
      this.ensureFailoverModelCache().catch(function() { return []; });
      var agentName = String(agent.name || agent.id || 'agent').trim() || 'agent';
      this.messages = [
        {
          id: ++msgId,
          role: 'agent',
          text: 'Who am I?',
          meta: '',
          tools: [],
          ts: Date.now(),
          thinking: true,
          thinking_status: 'Who am I?',
          agent_id: agentId,
          agent_name: agentName
        }
      ];
      this.recomputeContextEstimate();
      this.cacheAgentConversation(agentId);
      var self = this;
      this.$nextTick(function() {
        self.scrollToBottomImmediate();
        self.stabilizeBottomScroll();
        self.pinToLatestOnOpen(null, { maxFrames: 20 });
      });

      setTimeout(function() {
        if (Number(self.freshInitStageToken || 0) !== token) return;
        if (!self.currentAgent || String(self.currentAgent.id || '') !== agentId) return;
          self.messages = [
            {
              id: ++msgId,
              role: 'agent',
              text: 'Who am I?',
              meta: '',
              tools: [],
              ts: Date.now(),
              thinking: true,
              thinking_status: 'Who am I?',
              agent_id: agentId,
              agent_name: agentName
            }
          ];
        self.recomputeContextEstimate();
        self.cacheAgentConversation(agentId);
        self.$nextTick(function() {
          self.scrollToBottomImmediate();
          self.stabilizeBottomScroll();
          self.pinToLatestOnOpen(null, { maxFrames: 20 });
        });

        setTimeout(function() {
          if (Number(self.freshInitStageToken || 0) !== token) return;
          if (!self.currentAgent || String(self.currentAgent.id || '') !== agentId) return;
          self.freshInitRevealMenu = true;
          self.showFreshArchetypeTiles = true;
          self.$nextTick(function() {
            self.stabilizeBottomScroll();
          });
        }, 900);
      }, 500);
    },

    ensureFreshInitThread(agent) {
      if (!agent || !agent.id) return;
      var agentId = String(agent.id);
      if (this._freshInitThreadShownFor === agentId && Array.isArray(this.messages) && this.messages.length > 0) {
        return;
      }
      this.startFreshInitSequence(agent);
    },

    sessionHasAnyHistory: function(data) {
      if (data && Array.isArray(data.messages) && data.messages.length > 0) return true;
      if (data && data.message_window && Array.isArray(data.message_window.rows) && data.message_window.rows.length > 0) return true;
      if (data && Number(data.message_count || 0) > 0) return true;
      var pools = [];
      if (data && Array.isArray(data.sessions)) pools = pools.concat(data.sessions);
      if (data && data.session && Array.isArray(data.session.sessions)) {
        pools = pools.concat(data.session.sessions);
      }
      for (var i = 0; i < pools.length; i++) {
        var row = pools[i] || {};
        var count = Number(row.message_count);
        if (Number.isFinite(count) && count > 0) return true;
        if (Array.isArray(row.messages) && row.messages.length > 0) return true;
      }
      return false;
    },

    agentHasInitialContract: function(agentRef) {
      var agent = agentRef && typeof agentRef === 'object' ? agentRef : null;
      if (!agent) return false;
      var systemPrompt = String(agent.system_prompt || '').trim();
      if (systemPrompt) return true;
      var contract = agent.contract && typeof agent.contract === 'object' ? agent.contract : null;
      if (!contract) return false;
      var initialPrompt = String(
        contract.initial_prompt ||
        contract.initialPrompt ||
        contract.prompt ||
        ''
      ).trim();
      return !!initialPrompt;
    },

    recoverEmptySessionRender: function(agentId, sessionPayload) {
      var targetId = String(agentId || '').trim();
      if (!targetId) return;
      if (this.isFreshInitInProgressFor(targetId)) return;
      var resolved =
        this.resolveAgent(targetId) ||
        (this.currentAgent && String(this.currentAgent.id || '') === targetId ? this.currentAgent : null);
      if (
        !this.sessionHasAnyHistory(sessionPayload) &&
        resolved &&
        resolved.id &&
        !this.agentHasInitialContract(resolved)
      ) {
        this.ensureFreshInitThread(resolved);
        return;
      }
      this.messages = [{
        id: ++msgId,
        role: 'system',
        is_notice: true,
        notice_type: 'info',
        notice_label: 'This session is empty. Send a message to begin.',
        text: 'This session is empty. Send a message to begin.',
        meta: '',
        tools: [],
        system_origin: 'session:empty',
        ts: Date.now()
      }];
      this.recomputeContextEstimate();
      this.cacheAgentConversation(targetId);
      var self = this;
      this.$nextTick(function() {
        self.scrollToBottomImmediate();
        self.stabilizeBottomScroll();
        self.pinToLatestOnOpen(null, { maxFrames: 20 });
      });
    },

    isFreshInitInProgressFor: function(agentId) {
      var targetId = String(agentId || '').trim();
      if (!targetId) return false;
      var currentId = String(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '').trim();
      if (!currentId || currentId !== targetId) return false;
      var resolved =
        this.resolveAgent(targetId) ||
        (this.currentAgent && String(this.currentAgent.id || '') === targetId ? this.currentAgent : null);
      if (resolved && this.agentHasInitialContract(resolved)) {
        try {
          var appStore = Alpine.store('app');
          var pendingForResolved = String(appStore && appStore.pendingFreshAgentId ? appStore.pendingFreshAgentId : '').trim();
          if (pendingForResolved === targetId) appStore.pendingFreshAgentId = '';
        } catch(_) {}
        return false;
      }
      if (
        this.showFreshArchetypeTiles ||
        this.freshInitRevealMenu ||
        this.freshInitLaunching ||
        this.freshInitAwaitingOtherPrompt ||
        !!this.freshInitTemplateDef
      ) {
        return true;
      }
      var pendingFreshId = '';
      try {
        var store = Alpine.store('app');
        pendingFreshId = String(store && store.pendingFreshAgentId ? store.pendingFreshAgentId : '').trim();
      } catch(_) {}
      return !!pendingFreshId && pendingFreshId === targetId;
    },

    shouldSuppressAgentInactive: function(agentId) {
      var targetId = String(agentId || '').trim();
      if (!targetId) return false;
      if (this.isSystemThreadId && this.isSystemThreadId(targetId)) return true;
      if (this.isFreshInitInProgressFor(targetId)) return true;
      try {
        var store = Alpine.store('app');
        var pendingFreshId = String(store && store.pendingFreshAgentId ? store.pendingFreshAgentId : '').trim();
        var currentId = String(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '').trim();
        if (pendingFreshId && currentId && pendingFreshId === targetId && currentId === targetId) {
          return true;
        }
      } catch(_) {}
      return false;
    },

    pointerFxThemeMode() {
      try {
        var bodyTheme = '';
        var rootTheme = '';
        if (document && document.body && document.body.dataset) {
          bodyTheme = String(document.body.dataset.theme || '').toLowerCase().trim();
        }
        if (document && document.documentElement) {
          rootTheme = String(
            (document.documentElement.dataset && document.documentElement.dataset.theme) ||
            document.documentElement.getAttribute('data-theme') ||
            ''
          ).toLowerCase().trim();
        }
        var resolved = bodyTheme || rootTheme;
        if (!resolved) {
          try {
            resolved = window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches
              ? 'dark'
              : 'light';
          } catch(_) {
            resolved = 'light';
          }
        }
        if (document && document.body && document.body.dataset) {
          if (!bodyTheme || bodyTheme !== resolved) {
            document.body.dataset.theme = resolved;
          }
        }
        return resolved === 'dark' ? 'dark' : 'light';
      } catch(_) {
        return 'light';
      }
    },

    pointerTrailProfile() {
      if (
        typeof window !== 'undefined' &&
        window.__INFRING_POINTER_TRAIL_PROFILE_V1 &&
        typeof window.__INFRING_POINTER_TRAIL_PROFILE_V1 === 'object'
      ) {
        return window.__INFRING_POINTER_TRAIL_PROFILE_V1;
      }
      return {
        spacing: 0.13,
        max_steps: 52,
        head_interval_ms: 28,
        segment_thickness_base: 2.05,
        segment_thickness_gain: 1.85,
        segment_opacity_base: 0.32,
        segment_opacity_gain: 0.45,
        segment_hue_base: -4,
        segment_hue_gain: 8,
        head_particles: [
          { back: 0.0, lateral: 0.0, size: 3.9, opacity: 0.58, hue: 0 },
          { back: 1.55, lateral: 0.64, size: 3.4, opacity: 0.5, hue: 2 },
          { back: 2.45, lateral: -0.58, size: 3.0, opacity: 0.44, hue: -2 },
          { back: 3.15, lateral: 0.0, size: 2.7, opacity: 0.38, hue: 1 }
        ]
      };
    },

    pointerTrailFadeDurationMs(kind, slow) {
      var base = String(kind || '') === 'segment' ? 760 : 860;
      return slow ? (base * 10) : base;
    },

    clearPointerFxCleanupTimer(node) {
      if (!node) return;
      if (node._pointerFxCleanupTimer) {
        try { clearTimeout(node._pointerFxCleanupTimer); } catch(_) {}
        node._pointerFxCleanupTimer = 0;
      }
    },

    schedulePointerFxCleanup(node, kind, slow) {
      if (!node) return;
      this.clearPointerFxCleanupTimer(node);
      var delay = this.pointerTrailFadeDurationMs(kind, !!slow);
      node._pointerFxCleanupTimer = setTimeout(function() {
        try { node.remove(); } catch(_) {}
      }, Math.max(120, delay + 120));
    },

    updatePointerTrailHoldState(container, releaseSlow) {
      var host = this.resolveMessagesScroller(container || this._pointerTrailHoldHost || null) || this.resolveMessagesScroller();
      if (!host) return;
      var layer = this.resolvePointerFxLayer(host) || host;
      var nodes = layer.querySelectorAll('.chat-pointer-trail-dot:not(.chat-pointer-agent), .chat-pointer-trail-segment:not(.chat-pointer-agent)');
      for (var i = 0; i < nodes.length; i++) {
        var node = nodes[i];
        var isSegment = !!(node.classList && node.classList.contains('chat-pointer-trail-segment'));
        var kind = isSegment ? 'segment' : 'dot';
        this.clearPointerFxCleanupTimer(node);
        if (!node.classList) continue;
        if (releaseSlow) {
          node.classList.remove('chat-pointer-held');
          node.classList.remove('chat-pointer-release-slow');
          try { void node.offsetWidth; } catch(_) {}
          node.classList.add('chat-pointer-release-slow');
          this.schedulePointerFxCleanup(node, kind, true);
          continue;
        }
        node.classList.remove('chat-pointer-release-slow');
        node.classList.add('chat-pointer-held');
      }
    },

    ensurePointerTrailReleaseListener() {
      if (this._pointerTrailMouseUpHandler) return;
      var self = this;
      this._pointerTrailMouseUpHandler = function(ev) {
        self.handleMessagesPointerUp(ev || null);
      };
      document.addEventListener('mouseup', this._pointerTrailMouseUpHandler, true);
      document.addEventListener('pointerup', this._pointerTrailMouseUpHandler, true);
      window.addEventListener('blur', this._pointerTrailMouseUpHandler, true);
    },

    removePointerTrailReleaseListener() {
      if (!this._pointerTrailMouseUpHandler) return;
      try { document.removeEventListener('mouseup', this._pointerTrailMouseUpHandler, true); } catch(_) {}
      try { document.removeEventListener('pointerup', this._pointerTrailMouseUpHandler, true); } catch(_) {}
      try { window.removeEventListener('blur', this._pointerTrailMouseUpHandler, true); } catch(_) {}
      this._pointerTrailMouseUpHandler = null;
    },

// Layer ownership: client/runtime/systems/ui (dashboard static UX surface only; no runtime authority).

    spawnPointerTrail(container, x, y, opts) {
      var options = opts || {};
      if (!options.agentTrail && this.shouldSuspendPointerFx && this.shouldSuspendPointerFx()) return;
      var layer = options.agentTrail ? this.resolveAgentFxLayer(container) : this.resolvePointerFxLayer(container);
      if (!layer) return;
      var marker = document.createElement('span');
      marker.className = options.agentTrail ? 'chat-pointer-trail-dot chat-pointer-agent' : 'chat-pointer-trail-dot';
      if (options.agentTrail && marker.dataset) {
        var ownerId = String(
          options.fairyOwnerId ||
          (this.currentFairyOwnerId ? this.currentFairyOwnerId() : '')
        ).trim();
        if (ownerId) marker.dataset.fairyOwner = ownerId;
      }
      marker.style.left = x + 'px';
      marker.style.top = y + 'px';
      if (Number.isFinite(Number(options.size))) marker.style.setProperty('--trail-size', String(Number(options.size)));
      if (Number.isFinite(Number(options.opacity))) marker.style.setProperty('--trail-opacity', String(Number(options.opacity)));
      if (Number.isFinite(Number(options.scale))) marker.style.setProperty('--trail-scale', String(Number(options.scale)));
      if (Number.isFinite(Number(options.hueShift))) marker.style.setProperty('--trail-hue-shift', String(Number(options.hueShift)) + 'deg');
      var holdMouseTrail = !options.agentTrail && !!this._pointerTrailMouseHeld;
      if (holdMouseTrail) marker.classList.add('chat-pointer-held');
      layer.appendChild(marker);
      if (!holdMouseTrail) this.schedulePointerFxCleanup(marker, 'dot', false);
    },

    spawnPointerTrailSegment(container, x0, y0, x1, y1, opts) {
      var options = opts || {};
      if (!options.agentTrail && this.shouldSuspendPointerFx && this.shouldSuspendPointerFx()) return;
      var layer = options.agentTrail ? this.resolveAgentFxLayer(container) : this.resolvePointerFxLayer(container);
      if (!layer) return;
      var dx = Number(x1 || 0) - Number(x0 || 0);
      var dy = Number(y1 || 0) - Number(y0 || 0);
      var dist = Math.sqrt(dx * dx + dy * dy);
      if (!Number.isFinite(dist) || dist < 0.75) return;
      var seg = document.createElement('span');
      seg.className = options.agentTrail ? 'chat-pointer-trail-segment chat-pointer-agent' : 'chat-pointer-trail-segment';
      if (options.agentTrail && seg.dataset) {
        var ownerId = String(
          options.fairyOwnerId ||
          (this.currentFairyOwnerId ? this.currentFairyOwnerId() : '')
        ).trim();
        if (ownerId) seg.dataset.fairyOwner = ownerId;
      }
      var mx = Number(x0 || 0) + (dx * 0.5);
      var my = Number(y0 || 0) + (dy * 0.5);
      var angle = Math.atan2(dy, dx) * (180 / Math.PI);
      seg.style.left = mx + 'px';
      seg.style.top = my + 'px';
      seg.style.width = Math.max(2, dist + 1) + 'px';
      seg.style.transform = 'translate(-50%, -50%) rotate(' + angle + 'deg)';
      if (Number.isFinite(Number(options.thickness))) seg.style.setProperty('--trail-seg-thickness', String(Number(options.thickness)));
      if (Number.isFinite(Number(options.opacity))) seg.style.setProperty('--trail-seg-opacity', String(Number(options.opacity)));
      if (Number.isFinite(Number(options.hueShift))) seg.style.setProperty('--trail-seg-hue-shift', String(Number(options.hueShift)) + 'deg');
      var holdMouseTrail = !options.agentTrail && !!this._pointerTrailMouseHeld;
      if (holdMouseTrail) seg.classList.add('chat-pointer-held');
      layer.appendChild(seg);
      if (!holdMouseTrail) this.schedulePointerFxCleanup(seg, 'segment', false);
    },

    spawnPointerRipple(container, x, y) {
      if (this.shouldSuspendPointerFx && this.shouldSuspendPointerFx()) return;
      var layer = this.resolvePointerFxLayer(container);
      if (!layer) return;
      var ripple = document.createElement('span');
      ripple.className = 'chat-pointer-ripple';
      ripple.style.left = x + 'px';
      ripple.style.top = y + 'px';
      layer.appendChild(ripple);
      setTimeout(function() {
        try { ripple.remove(); } catch(_) {}
      }, 820);
    },
    shouldSuspendPointerFx() {
      return !!this.recording;
    },

    resolvePointerFxLayer(container) {
      if (!container || typeof container.querySelector !== 'function') return null;
      return container.querySelector('.chat-grid-overlay') || container;
    },
    resolveAgentFxLayer(container) {
      if (!container || typeof container.querySelector !== 'function') return null;
      var layer = container.querySelector('.chat-agent-overlay');
      if (layer) return layer;
      layer = document.createElement('div');
      layer.className = 'chat-agent-overlay';
      container.appendChild(layer);
      return layer;
    },

    ensurePointerOrb(container, x, y) {
      if (this.shouldSuspendPointerFx && this.shouldSuspendPointerFx()) return null;
      var layer = this.resolvePointerFxLayer(container);
      if (!layer) return null;
      var orb = this._pointerOrbEl;
      if (!orb || !orb.isConnected || orb.parentNode !== layer) {
        if (orb) {
          try { orb.remove(); } catch(_) {}
        }
        orb = document.createElement('span');
        orb.className = 'chat-pointer-orb';
        layer.appendChild(orb);
        this._pointerOrbEl = orb;
      }
      orb.style.left = x + 'px';
      orb.style.top = y + 'px';
      return orb;
    },

    removePointerOrb() {
      var orb = this._pointerOrbEl;
      if (!orb) return;
      try { orb.remove(); } catch(_) {}
      this._pointerOrbEl = null;
    },

    handleMessagesPointerMove(event) {
      if (!event || !event.currentTarget) return;
      var host = event.currentTarget;
      this.startAgentTrailLoop(host);
      this.syncDirectHoverFromPointer(event);
      if (this.shouldSuspendPointerFx && this.shouldSuspendPointerFx()) {
        this.removePointerOrb();
        return;
      }
      if (this.pointerFxThemeMode() !== 'dark') {
        this.removePointerOrb();
        return;
      }
      var now = Date.now();
      if ((now - Number(this._pointerTrailLastAt || 0)) < 8) return;
      this._pointerTrailLastAt = now;
      var rect = host.getBoundingClientRect();
      // Keep pointer FX in viewport coordinates so the mask remains visible
      // while reading scrolled chat history.
      var x = event.clientX - rect.left;
      var y = event.clientY - rect.top;
      host.style.setProperty('--chat-grid-x', Math.round(x) + 'px');
      host.style.setProperty('--chat-grid-y', Math.round(y) + 'px');
      host.style.setProperty('--chat-grid-active', '1');
      this.ensurePointerOrb(host, x, y);
      if (!this._pointerTrailSeeded) {
        this._pointerTrailLastX = x;
        this._pointerTrailLastY = y;
        this._pointerTrailSeeded = true;
      }
      var dx = x - Number(this._pointerTrailLastX || x);
      var dy = y - Number(this._pointerTrailLastY || y);
      var dist = Math.sqrt(dx * dx + dy * dy);
      var profile = typeof this.pointerTrailProfile === 'function'
        ? this.pointerTrailProfile()
        : null;
      var spacing = Number(profile && profile.spacing);
      if (!Number.isFinite(spacing) || spacing <= 0) spacing = 0.13;
      var maxSteps = Number(profile && profile.max_steps);
      if (!Number.isFinite(maxSteps) || maxSteps < 1) maxSteps = 52;
      var steps = Math.max(1, Math.min(maxSteps, Math.ceil(dist / spacing)));
      for (var i = 1; i <= steps; i++) {
        var t0 = (i - 1) / steps;
        var t1 = i / steps;
        var sx0 = this._pointerTrailLastX + (dx * t0);
        var sy0 = this._pointerTrailLastY + (dy * t0);
        var sx1 = this._pointerTrailLastX + (dx * t1);
        var sy1 = this._pointerTrailLastY + (dy * t1);
        var progress = t1;
        var thickness = Number(profile && profile.segment_thickness_base);
        if (!Number.isFinite(thickness)) thickness = 2.05;
        var thicknessGain = Number(profile && profile.segment_thickness_gain);
        if (!Number.isFinite(thicknessGain)) thicknessGain = 1.85;
        thickness += (progress * thicknessGain);
        var alpha = Number(profile && profile.segment_opacity_base);
        if (!Number.isFinite(alpha)) alpha = 0.32;
        var alphaGain = Number(profile && profile.segment_opacity_gain);
        if (!Number.isFinite(alphaGain)) alphaGain = 0.45;
        alpha += (progress * alphaGain);
        var hueShift = Number(profile && profile.segment_hue_base);
        if (!Number.isFinite(hueShift)) hueShift = -4;
        var hueGain = Number(profile && profile.segment_hue_gain);
        if (!Number.isFinite(hueGain)) hueGain = 8;
        hueShift += (progress * hueGain);
        this.spawnPointerTrailSegment(host, sx0, sy0, sx1, sy1, {

// FILE_SIZE_EXCEPTION: reason=chat pointer fx split continuity owner=jay expires=2026-06-30
          thickness: thickness,
          opacity: alpha,
          hueShift: hueShift,
        });
      }
      var headInterval = Number(profile && profile.head_interval_ms);
      if (!Number.isFinite(headInterval) || headInterval < 1) headInterval = 28;
      var canSpawnHead = (now - Number(this._pointerTrailHeadLastAt || 0)) >= headInterval;
      if (canSpawnHead || dist < 1.5) {
        // Render several smaller head particles instead of one large dot.
        var invDist = dist > 0.0001 ? (1 / dist) : 0;
        var nx = dist > 0.0001 ? (dx * invDist) : 1;
        var ny = dist > 0.0001 ? (dy * invDist) : 0;
        var pxTrail = Array.isArray(profile && profile.head_particles) && profile.head_particles.length
          ? profile.head_particles
          : [
              { back: 0.0, lateral: 0.0, size: 3.9, opacity: 0.58, hue: 0 },
              { back: 1.55, lateral: 0.64, size: 3.4, opacity: 0.5, hue: 2 },
              { back: 2.45, lateral: -0.58, size: 3.0, opacity: 0.44, hue: -2 },
              { back: 3.15, lateral: 0.0, size: 2.7, opacity: 0.38, hue: 1 },
            ];
        for (var j = 0; j < pxTrail.length; j++) {
          var p = pxTrail[j];
          var px = x - (nx * p.back) + (-ny * p.lateral);
          var py = y - (ny * p.back) + (nx * p.lateral);
          this.spawnPointerTrail(host, px, py, {
            size: p.size,
            opacity: p.opacity,
            scale: 1.03,
            hueShift: p.hue,
          });
        }
        this._pointerTrailHeadLastAt = now;
      }
      this._pointerTrailLastX = x;
      this._pointerTrailLastY = y;
    },
    syncGridBackgroundOffset(container) {
      var host = this.resolveMessagesScroller(container || null);
      if (!host) return;
      var scrollX = Number(host.scrollLeft || 0);
      var scrollY = Number(host.scrollTop || 0);
      host.style.setProperty('--chat-grid-scroll-x', String(-Math.round(scrollX)) + 'px');
      host.style.setProperty('--chat-grid-scroll-y', String(-Math.round(scrollY)) + 'px');
    },

    normalizePointerTarget(target) {
      var node = target || null;
      if (!node) return null;
      if (node.nodeType === 3) return node.parentElement || null;
      return node.nodeType === 1 ? node : null;
    },

    isPointerInteractiveTarget(target, host) {
      var node = this.normalizePointerTarget(target);
      while (node && node !== host) {
        if (node.matches && node.matches('button,[role="button"],a[href],summary,details,input,textarea,select,option,label,[data-no-select-gate="true"]')) {
          return true;
        }
        node = node.parentElement;
      }
      return false;
    },

    canStartMessagesTextSelection(target, host) {
      var node = this.normalizePointerTarget(target);
      while (node && node !== host) {
        if (node.matches && node.matches('input,textarea,[contenteditable],[contenteditable=""],[contenteditable="true"],[contenteditable="plaintext-only"]')) {
          return true;
        }
        if (
          node.matches &&
          node.matches(
            '.message-bubble-content, .message-bubble-content *, .chat-artifact-pre, .chat-artifact-pre *, .chat-artifact-path, .chat-artifact-path *, .chat-artifact-title, .chat-artifact-title *'
          )
        ) {
          return true;
        }
        try {
          var style = window.getComputedStyle(node);
          var cursor = String(style && style.cursor ? style.cursor : '').toLowerCase();
          if (cursor.indexOf('text') !== -1) return true;
        } catch(_) {}
        node = node.parentElement;
      }
      return false;
    },

    handleMessagesSelectStart(event) {
      if (!event || !event.currentTarget) return;
      var host = event.currentTarget;
      if (!this.canStartMessagesTextSelection(event.target, host)) {
        event.preventDefault();
      }
    },

    handleMessagesPointerDown(event) {
      if (!event || !event.currentTarget) return;
      var host = event.currentTarget;
      var canSelectText = this.canStartMessagesTextSelection(event.target, host);
      var isInteractive = this.isPointerInteractiveTarget(event.target, host);
      if (!canSelectText && !isInteractive) {
        event.preventDefault();
      }
      if (!canSelectText) {
        this._pointerTrailMouseHeld = true;
        this._pointerTrailHoldHost = host;
        this.updatePointerTrailHoldState(host, false);
        this.ensurePointerTrailReleaseListener();
      }
      if (this.pointerFxThemeMode() !== 'light') return;
      var rect = host.getBoundingClientRect();
      var x = event.clientX - rect.left;
      var y = event.clientY - rect.top;
      this.spawnPointerRipple(host, x, y);
    },

    handleMessagesPointerUp(event) {
      if (!this._pointerTrailMouseHeld) {
        this.removePointerTrailReleaseListener();
        return;
      }
      var host = this.resolveMessagesScroller(this._pointerTrailHoldHost || (event && event.currentTarget ? event.currentTarget : null)) || this.resolveMessagesScroller();
      this._pointerTrailMouseHeld = false;
      this._pointerTrailHoldHost = null;
      this.updatePointerTrailHoldState(host, true);
      this.removePointerTrailReleaseListener();
    },

    clearPointerFx(event) {
      if (!event || !event.currentTarget) return;
      if (this._pointerTrailMouseHeld) return;
      var host = event.currentTarget, layer = this.resolvePointerFxLayer(host);
      if (this._pointerGridHideTimer) {
        clearTimeout(this._pointerGridHideTimer);
        this._pointerGridHideTimer = null;
      }
      var dots = (layer || host).querySelectorAll('.chat-pointer-trail-dot:not(.chat-pointer-agent),.chat-pointer-trail-segment:not(.chat-pointer-agent),.chat-pointer-ripple');
      for (var i = 0; i < dots.length; i++) {
        this.clearPointerFxCleanupTimer(dots[i]);
        try { dots[i].remove(); } catch(_) {}
      }
      this.removePointerOrb();
      this._pointerTrailSeeded = false;
      this._pointerTrailHeadLastAt = 0;
    },
    currentFairyOwnerId() {
      if (!this.currentAgent) return '';
      if (this.isSystemThreadAgent && this.isSystemThreadAgent(this.currentAgent)) return '';
      var value = String(this.currentAgent.id || '').trim();
      return value || '';
    },
    resolveFairyHost(container) {
      var host = this.resolveMessagesScroller(container || null);
      var scope = this.$el || null;
      if (host && scope && scope.contains(host)) return host;
      var scopedRef = this.$refs && this.$refs.messagesEl ? this.$refs.messagesEl : null;
      if (scopedRef && scope && scope.contains(scopedRef) && scopedRef.offsetParent !== null) return scopedRef;
      return null;
    },
    wireAgentTrailOrbBehavior(orb) {
      if (!orb) return null;
      var self = this;
      if (typeof orb.toggleIndex !== 'function') {
        orb.toggleIndex = function(forceTop) {
          return self.toggleAgentTrailOrbIndex(forceTop);
        };
      }
      this.applyAgentTrailOrbIndexState(orb);
      return orb;
    },
    resolveAgentTrailOverlay(orbRef) {
      var orb = orbRef || this._agentTrailOrbEl || null;
      if (!orb) return null;
      var layer = orb.parentElement || null;
      if (!layer || !layer.classList || !layer.classList.contains('chat-agent-overlay')) return null;
      return layer;
    },
    applyAgentTrailOrbIndexState(orbRef) {
      var orb = orbRef || this._agentTrailOrbEl || null;
      if (!orb || !orb.style) return;
      var top = !!this._agentTrailOrbElevated;
      if (top) {
        orb.style.zIndex = '2147483000';
        if (orb.classList) orb.classList.add('fairy-z-top');
      } else {
        orb.style.zIndex = '';
        if (orb.classList) orb.classList.remove('fairy-z-top');
      }
      var layer = this.resolveAgentTrailOverlay(orb);
      if (!layer) return;
      if (top) {
        layer.style.zIndex = '2147482999';
        layer.classList.add('fairy-z-top');
      } else {
        layer.style.zIndex = '';
        layer.classList.remove('fairy-z-top');
      }
    },
    toggleAgentTrailOrbIndex(forceTop) {
      var orb = this._agentTrailOrbEl;
      if (!orb || !orb.style) return false;
      var nextTop = false;
      if (forceTop === true) nextTop = true;
      else if (forceTop === false) nextTop = false;
      else nextTop = !this._agentTrailOrbElevated;
      this._agentTrailOrbElevated = !!nextTop;
      this.applyAgentTrailOrbIndexState(orb);
      return this._agentTrailOrbElevated;
    },
    teleportAgentTrailOrb(orbRef, x, y, toggleIndex, onMidpoint) {
      var orb = this.wireAgentTrailOrbBehavior(orbRef || this._agentTrailOrbEl);
      if (!orb || !orb.style) return;
      var shouldToggleIndex = toggleIndex !== false;
      var targetX = Number(x);
      var targetY = Number(y);
      var pendingX = Number(this._agentTrailTeleportTargetX);
      var pendingY = Number(this._agentTrailTeleportTargetY);
      var pendingToggle = this._agentTrailTeleportToggleIndex !== false;
      var hasPendingTeleport = !!this._agentTrailTeleportTimer;
      var samePendingTeleport = hasPendingTeleport &&
        Number.isFinite(targetX) &&
        Number.isFinite(targetY) &&
        Number.isFinite(pendingX) &&
        Number.isFinite(pendingY) &&
        Math.abs(targetX - pendingX) <= 0.5 &&
        Math.abs(targetY - pendingY) <= 0.5 &&
        pendingToggle === shouldToggleIndex;
      if (samePendingTeleport) return;
      if (hasPendingTeleport) {
        clearTimeout(this._agentTrailTeleportTimer);
        this._agentTrailTeleportTimer = 0;
        orb.style.opacity = '';
        orb.style.transition = '';
      }
      this._agentTrailTeleportTargetX = Number.isFinite(targetX) ? targetX : NaN;
      this._agentTrailTeleportTargetY = Number.isFinite(targetY) ? targetY : NaN;
      this._agentTrailTeleportToggleIndex = shouldToggleIndex;
      var self = this;
      orb.style.transition = 'opacity 95ms ease';
      orb.style.opacity = '0';
      this._agentTrailTeleportTimer = setTimeout(function() {
        self._agentTrailTeleportTimer = 0;
        self._agentTrailTeleportTargetX = NaN;
        self._agentTrailTeleportTargetY = NaN;
        self._agentTrailTeleportToggleIndex = true;
        if (!self._agentTrailOrbEl || self._agentTrailOrbEl !== orb) return;
        if (shouldToggleIndex && typeof orb.toggleIndex === 'function') orb.toggleIndex();
        if (Number.isFinite(targetX)) orb.style.left = targetX + 'px';
        if (Number.isFinite(targetY)) orb.style.top = targetY + 'px';
        if (typeof onMidpoint === 'function') {
          try { onMidpoint(); } catch(_) {}
        }
        requestAnimationFrame(function() {
          if (!self._agentTrailOrbEl || self._agentTrailOrbEl !== orb) return;
          orb.style.opacity = '';
          orb.style.transition = '';
        });
      }, 95);
    },
    setAgentTrailBlinkState(active, orbRef) {
      var orb = this.wireAgentTrailOrbBehavior(orbRef || this._agentTrailOrbEl);
      if (!orb || !orb.classList) return;
      if (active) {
        if (typeof orb.toggleIndex === 'function') orb.toggleIndex(true);
        orb.classList.add('agent-listening');
        return;
      }
      orb.classList.remove('agent-listening');
      if (typeof orb.toggleIndex === 'function') orb.toggleIndex(false);
    },
    ensureAgentTrailOrb(container, x, y) {
      var ownerId = this.currentFairyOwnerId();
      if (!ownerId) {
        this.removeAgentTrailOrb();
        return null;
      }
      var host = this.resolveFairyHost(container || null);
      var layer = this.resolveAgentFxLayer(host || container);
      if (!layer) return null;
      var orb = this._agentTrailOrbEl;
      if (!orb || !orb.isConnected || orb.parentNode !== layer) {
        if (orb) try { orb.remove(); } catch(_) {}
        orb = document.createElement('span');
        orb.className = 'chat-pointer-orb chat-pointer-agent';
        layer.appendChild(orb);
        this._agentTrailOrbEl = orb;
      }
      this.wireAgentTrailOrbBehavior(orb);
      if (ownerId && orb.dataset) {
        orb.dataset.fairyOwner = ownerId;
        this._agentFairyOwnerId = ownerId;
      }
      var currentX = Number(parseFloat(String(orb.style.left || 'NaN')));
      var currentY = Number(parseFloat(String(orb.style.top || 'NaN')));
      if (!Number.isFinite(currentX)) currentX = Number(orb.offsetLeft || NaN);
      if (!Number.isFinite(currentY)) currentY = Number(orb.offsetTop || NaN);
      var dx = Number.isFinite(currentX) ? Math.abs(Number(x) - currentX) : 0;
      var dy = Number.isFinite(currentY) ? Math.abs(Number(y) - currentY) : 0;
      var jumpDistance = Math.sqrt((dx * dx) + (dy * dy));
      if (orb.classList && orb.classList.contains('agent-listening') && jumpDistance >= 72) {
        this.teleportAgentTrailOrb(orb, x, y, !this._agentTrailOrbElevated);
      } else {
        orb.style.left = x + 'px';
        orb.style.top = y + 'px';
      }
      return orb;
    },
    pruneAgentTrailFx(container) {
      var host = this.resolveFairyHost(container || this._agentTrailHost || null);
      if (!host || typeof host.querySelectorAll !== 'function') return;
      var ownerId = this.currentFairyOwnerId();
      if (!ownerId) {
        var staleNodes = host.querySelectorAll('.chat-pointer-agent');
        for (var sn = 0; sn < staleNodes.length; sn++) {
          try { staleNodes[sn].remove(); } catch(_) {}
        }
        this.removeAgentTrailOrb();
        host.style.setProperty('--chat-agent-grid-active', '0');
        return;
      }
      var orbNodes = host.querySelectorAll('.chat-pointer-orb.chat-pointer-agent');
      var keepOrb = this._agentTrailOrbEl && this._agentTrailOrbEl.isConnected
        ? this._agentTrailOrbEl
        : null;
      for (var i = 0; i < orbNodes.length; i++) {
        var node = orbNodes[i];
        var nodeOwner = node && node.dataset && node.dataset.fairyOwner ? String(node.dataset.fairyOwner).trim() : '';
        if (ownerId && nodeOwner && nodeOwner !== ownerId) {
          try { node.remove(); } catch(_) {}
          continue;
        }
        if (ownerId && !nodeOwner && node && node.dataset) node.dataset.fairyOwner = ownerId;
        if (!keepOrb) {
          keepOrb = node;
          this._agentTrailOrbEl = node;
          this.wireAgentTrailOrbBehavior(node);
          continue;
        }
        if (keepOrb === node) continue;
        try { node.remove(); } catch(_) {}
      }
      var trailNodes = host.querySelectorAll('.chat-pointer-trail-dot.chat-pointer-agent, .chat-pointer-trail-segment.chat-pointer-agent');
      var ownedTrailNodes = [];
      for (var ti = 0; ti < trailNodes.length; ti++) {
        var trailNode = trailNodes[ti];
        var trailOwner = trailNode && trailNode.dataset && trailNode.dataset.fairyOwner ? String(trailNode.dataset.fairyOwner).trim() : '';
        if (ownerId && trailOwner && trailOwner !== ownerId) {
          try { trailNode.remove(); } catch(_) {}
          continue;
        }
        if (ownerId && !trailOwner && trailNode && trailNode.dataset) trailNode.dataset.fairyOwner = ownerId;
        ownedTrailNodes.push(trailNode);
      }
      var maxTrailNodes = 220;
      var extra = Number(ownedTrailNodes.length || 0) - maxTrailNodes;
      if (extra > 0) {
        for (var j = 0; j < extra; j++) {
          try { ownedTrailNodes[j].remove(); } catch(_) {}
        }
      }
    },
    removeAgentTrailOrb() {
      var orb = this._agentTrailOrbEl;
      if (this._agentTrailTeleportTimer) {
        clearTimeout(this._agentTrailTeleportTimer);
        this._agentTrailTeleportTimer = 0;
      }
      this._agentTrailTeleportTargetX = NaN;
      this._agentTrailTeleportTargetY = NaN;
      this._agentTrailTeleportToggleIndex = true;
      if (!orb) return;
      var layer = this.resolveAgentTrailOverlay(orb);
      if (layer) {
        layer.style.zIndex = '';
        layer.classList.remove('fairy-z-top');
      }
      try { orb.remove(); } catch(_) {}
      this._agentTrailOrbEl = null;
      this._agentFairyOwnerId = '';
      this._agentTrailOrbElevated = false;
    },
    clearAgentTrailFx(container) {
      var host = this.resolveFairyHost(container || this._agentTrailHost || null);

      if (!host) return;
      var nodes = host.querySelectorAll('.chat-pointer-agent');
      for (var i = 0; i < nodes.length; i++) {
        try { nodes[i].remove(); } catch(_) {}
      }
      this.removeAgentTrailOrb();
      host.style.setProperty('--chat-agent-grid-active', '0');
    },
    dedupeAgentTrailFx(activeContainer) {
      var activeHost = this.resolveFairyHost(activeContainer || this._agentTrailHost || null);
      var scope = this.$el || document;
      var ownerId = this.currentFairyOwnerId();
      if (!ownerId) {
        var stale = scope.querySelectorAll('.chat-pointer-agent, .chat-agent-overlay');
        for (var si = 0; si < stale.length; si++) {
          try { stale[si].remove(); } catch(_) {}
        }
        this._agentTrailOrbEl = null;
        this._agentFairyOwnerId = '';
        this._agentTrailOrbElevated = false;
        var staleHosts = scope.querySelectorAll('#messages');
        for (var shi = 0; shi < staleHosts.length; shi++) {
          try { staleHosts[shi].style.setProperty('--chat-agent-grid-active', '0'); } catch(_) {}
        }
        return;
      }
      var overlays = scope.querySelectorAll('.chat-agent-overlay');
      var keptActiveOverlay = null;
      for (var oi = 0; oi < overlays.length; oi++) {
        var overlay = overlays[oi];
        if (!overlay) continue;
        var owner = overlay.closest ? overlay.closest('#messages') : null;
        if (!activeHost || owner !== activeHost) {
          try { overlay.remove(); } catch(_) {}
          continue;
        }
        if (!keptActiveOverlay) {
          keptActiveOverlay = overlay;
          continue;
        }
        try { overlay.remove(); } catch(_) {}
      }
      var agentNodes = scope.querySelectorAll('.chat-pointer-agent');
      for (var ni = 0; ni < agentNodes.length; ni++) {
        var node = agentNodes[ni];
        if (!activeHost || !activeHost.contains(node)) {
          try { node.remove(); } catch(_) {}
          continue;
        }
        var nodeOwner = node && node.dataset && node.dataset.fairyOwner ? String(node.dataset.fairyOwner).trim() : '';
        if (ownerId && nodeOwner && nodeOwner !== ownerId) {
          try { node.remove(); } catch(_) {}
          continue;
        }
        if (ownerId && !nodeOwner && node && node.dataset) node.dataset.fairyOwner = ownerId;
      }
      if (activeHost && typeof activeHost.querySelectorAll === 'function') {
        var activeOrbs = activeHost.querySelectorAll('.chat-pointer-orb.chat-pointer-agent');
        var keepOrb = null;
        for (var ai = 0; ai < activeOrbs.length; ai++) {
          var orb = activeOrbs[ai];
          var orbOwner = orb && orb.dataset && orb.dataset.fairyOwner ? String(orb.dataset.fairyOwner).trim() : '';
          if (ownerId && orbOwner && orbOwner !== ownerId) {
            try { orb.remove(); } catch(_) {}
            continue;
          }
          if (ownerId && !orbOwner && orb && orb.dataset) orb.dataset.fairyOwner = ownerId;
          if (!keepOrb) {
            keepOrb = orb;
            continue;
          }
          try { orb.remove(); } catch(_) {}
        }
        this._agentTrailOrbEl = keepOrb || null;
        if (this._agentTrailOrbEl) this.wireAgentTrailOrbBehavior(this._agentTrailOrbEl);
        else {
          this._agentTrailOrbElevated = false;
          if (keptActiveOverlay) {
            keptActiveOverlay.style.zIndex = '';
            keptActiveOverlay.classList.remove('fairy-z-top');
          }
        }
      }
      var hosts = scope.querySelectorAll('#messages');
      for (var hi = 0; hi < hosts.length; hi++) {
        var host = hosts[hi];
        if (!host || (activeHost && host === activeHost)) continue;
        host.style.setProperty('--chat-agent-grid-active', '0');
      }
      if (this._agentTrailOrbEl && (!activeHost || !activeHost.contains(this._agentTrailOrbEl))) {
        this._agentTrailOrbEl = null;
        this._agentTrailOrbElevated = false;
        if (activeHost && typeof activeHost.querySelector === 'function') {
          var activeOverlay = activeHost.querySelector('.chat-agent-overlay');
          if (activeOverlay) {
            activeOverlay.style.zIndex = '';
            activeOverlay.classList.remove('fairy-z-top');
          }
        }
      }
    },
    startAgentTrailLoop(container) {
      if (this._agentTrailListening) return;
      if (!this.currentFairyOwnerId()) {
        this.stopAgentTrailLoop(true);
        return;
      }
      var host = this.resolveFairyHost(container || this._agentTrailHost || null);
      if (!host) return;
      var previousHost = this._agentTrailHost || null;
      if (host && previousHost && previousHost !== host) {
        this.clearAgentTrailFx(previousHost);
      }
      if (host) this._agentTrailHost = host;
      this.dedupeAgentTrailFx(this._agentTrailHost);
      if (this._agentTrailRaf) return;
      var self = this;
      var tick = function(ts) {
        self._agentTrailRaf = requestAnimationFrame(tick);
        self.stepAgentTrail(ts || performance.now());
      };
      this._agentTrailLastAt = 0;
      this._agentTrailRaf = requestAnimationFrame(tick);
    },
    stopAgentTrailLoop(clearVisuals) {
      if (this._agentTrailRaf) try { cancelAnimationFrame(this._agentTrailRaf); } catch(_) {}
      this._agentTrailRaf = 0;
      this._agentTrailLastAt = 0;
      this._agentTrailLastDotAt = 0;
      this._agentTrailSeeded = false;
      this._agentTrailState = null;
      if (clearVisuals) this.clearAgentTrailFx(this._agentTrailHost);
      this._agentTrailHost = null;
    },
    stepAgentTrail(now) {
      var host = this.resolveFairyHost(this._agentTrailHost || null);
      if (!host || host.offsetParent === null) {
        this.dedupeAgentTrailFx(null);
        return;
      }
      if (!this.currentFairyOwnerId()) {
        this.clearAgentTrailFx(host);
        return;
      }
      if ((now - Number(this._agentTrailSweepAt || 0)) >= 260) {
        this.dedupeAgentTrailFx(host);
        this._agentTrailSweepAt = now;
      }
      var agentTrailDarkMode = this.pointerFxThemeMode() === 'dark';
      if (!agentTrailDarkMode) {
        var lightModeTrailNodes = host.querySelectorAll('.chat-pointer-trail-dot.chat-pointer-agent, .chat-pointer-trail-segment.chat-pointer-agent');
        for (var ln = 0; ln < lightModeTrailNodes.length; ln++) {
          this.clearPointerFxCleanupTimer(lightModeTrailNodes[ln]);
          try { lightModeTrailNodes[ln].remove(); } catch(_) {}
        }
      }
      this.pruneAgentTrailFx(host);
      this.syncGridBackgroundOffset(host);
      var rect = host.getBoundingClientRect(), w = Number(rect.width || 0), h = Number(rect.height || 0);
      if (!(w > 24 && h > 24)) return;
      var pad = 7;
      var zoneRight = w / 3, zoneTop = h * (2 / 3), shadowBuffer = 42;
      var minX = pad + shadowBuffer, maxX = Math.max(minX + 2, zoneRight - pad);
      var minY = Math.max(pad, zoneTop + 2), maxY = Math.max(minY + 2, h - pad);
      // Thinking anchor has higher priority than init-panel anchor so the
      // fairy always hugs the active thinking bubble during initialization.
      if (this.anchorAgentTrailToThinking(host, rect, now, pad, w, h)) return;
      if (this.anchorAgentTrailToFreshInit(host, rect, now, pad, w, h)) return;
      var s = this._agentTrailState;
      if (!s) { s = { x: (minX + maxX) * 0.5, y: (minY + maxY) * 0.5, vx: 48, vy: -24, dir: Math.random() * Math.PI * 2, target: 0, turnAt: 0 }; s.target = s.dir; s.turnAt = now + 1000; this._agentTrailState = s; }
      var dt = this._agentTrailLastAt > 0 ? Math.min(0.05, Math.max(0.001, (now - this._agentTrailLastAt) / 1000)) : (1 / 60);
      this._agentTrailLastAt = now;
      if (now >= Number(s.turnAt || 0)) { s.target = Math.random() * Math.PI * 2; s.turnAt = now + 1000; }
      var turnDelta = Math.atan2(Math.sin(s.target - s.dir), Math.cos(s.target - s.dir));
      s.dir += turnDelta * Math.min(1, dt * 3.3);
      var ax = Math.cos(s.dir) * 110 + ((Math.random() - 0.5) * 30);
      var ay = Math.sin(s.dir) * 84 + ((Math.random() - 0.5) * 24);
      ay += (((minY + maxY) * 0.5) - s.y) * 1.8;
      var cx = Number(this._lastPointerClientX || 0), cy = Number(this._lastPointerClientY || 0);
      if (cx >= rect.left && cx <= rect.right && cy >= rect.top && cy <= rect.bottom) {
        var px = cx - rect.left, py = cy - rect.top;
        var rx = s.x - px, ry = s.y - py;
        var avoidR = 118, d2 = (rx * rx) + (ry * ry);
        if (d2 > 0.0001 && d2 < (avoidR * avoidR)) {
          var d = Math.sqrt(d2);
          var repel = 1 - (d / avoidR), f = 620 * repel * repel;
          ax += (rx / d) * f;
          ay += (ry / d) * f;
        }
      }
      s.vx = (s.vx + (ax * dt)) * 0.94;
      s.vy = (s.vy + (ay * dt)) * 0.94;
      var speed = Math.sqrt((s.vx * s.vx) + (s.vy * s.vy));
      if (speed > 624) { s.vx = (s.vx / speed) * 624; s.vy = (s.vy / speed) * 624; }
      s.x += s.vx * dt; s.y += s.vy * dt;
      if (s.x < minX || s.x > maxX) { s.vx = (s.x < minX ? Math.abs(s.vx) : -Math.abs(s.vx)) * 0.72; s.x = s.x < minX ? minX : maxX; }
      if (s.y < minY || s.y > maxY) { s.vy = (s.y < minY ? Math.abs(s.vy) : -Math.abs(s.vy)) * 0.72; s.y = s.y < minY ? minY : maxY; }
      var fairyOwnerId = this.currentFairyOwnerId();
      this.ensureAgentTrailOrb(host, s.x, s.y);
      host.style.setProperty('--chat-agent-grid-active', '1'); host.style.setProperty('--chat-agent-grid-x', Math.round(s.x) + 'px'); host.style.setProperty('--chat-agent-grid-y', Math.round(s.y) + 'px');
      if (!agentTrailDarkMode) {
        this._agentTrailSeeded = false;
        this._agentTrailLastDotAt = now;
        if (s) {
          s.trailX = s.x;
          s.trailY = s.y;
        }
        return;
      }
      var shouldSpawnTrail = (now - Number(this._agentTrailLastDotAt || 0)) >= 52;
      if (!this._agentTrailSeeded) {
        this.spawnPointerTrail(host, s.x, s.y, {
          agentTrail: true,
          fairyOwnerId: fairyOwnerId,
          size: 3.4,
          opacity: 0.58,
          scale: 1.02,
        });
        s.trailX = s.x;
        s.trailY = s.y;
        this._agentTrailSeeded = true;
        this._agentTrailLastDotAt = now;
      } else if (shouldSpawnTrail) {
        var fromX = Number(s.trailX);
        var fromY = Number(s.trailY);
        if (!Number.isFinite(fromX) || !Number.isFinite(fromY)) {
          fromX = s.x;
          fromY = s.y;
        }
        this.spawnPointerTrailSegment(host, fromX, fromY, s.x, s.y, {
          agentTrail: true,
          fairyOwnerId: fairyOwnerId,
          thickness: 2.4,
          opacity: 0.52,
        });
        this.spawnPointerTrail(host, s.x, s.y, {
          agentTrail: true,
          fairyOwnerId: fairyOwnerId,
          size: 3.1,
          opacity: 0.56,
          scale: 1.01,
        });
        s.trailX = s.x;
        s.trailY = s.y;
        this._agentTrailLastDotAt = now;
      }
    },
    markAgentMessageComplete(msg) {
      if (!msg || msg.role !== 'agent') return;
      msg._finish_bounce = true;
      setTimeout(function() {
        try { msg._finish_bounce = false; } catch(_) {}
      }, 300);
    },

    fetchModelContextWindows(force) {
      var now = Date.now();
      if (!force && this._contextModelsFetchedAt && (now - this._contextModelsFetchedAt) < 300000) {
        this.setContextWindowFromCurrentAgent();
        return Promise.resolve();
      }
      var self = this;
      return InfringAPI.get('/api/shell-socket/models').then(function(data) {
        self.refreshContextWindowMap(data && data.models ? data.models : []);
        self._contextModelsFetchedAt = Date.now();
        self.setContextWindowFromCurrentAgent();
      }).catch(function() {});
    },

    requestContextTelemetry(force) {
      if (!this.currentAgent || !InfringAPI.isWsConnected()) return false;
      var now = Date.now();
      if (!force && (now - Number(this._lastContextRequestAt || 0)) < 2500) return false;
      this._lastContextRequestAt = now;
      return !!InfringAPI.wsSend({ type: 'command', command: 'context', silent: true });
    },

    normalizeModelUsageKey: function(modelId) {
      return String(modelId || '').trim().toLowerCase();
    },

    loadModelUsageCache: function() {
      try {
        var raw = localStorage.getItem(this.modelUsageCacheKey);
        if (!raw) {
          this.modelUsageCache = {};
          return;
        }
        var parsed = JSON.parse(raw);
        this.modelUsageCache = parsed && typeof parsed === 'object' ? parsed : {};
      } catch {
        this.modelUsageCache = {};
      }
    },

    persistModelUsageCache: function() {
      try {
        localStorage.setItem(this.modelUsageCacheKey, JSON.stringify(this.modelUsageCache || {}));
      } catch {}
    },

    modelUsageTimestamp: function(modelId) {
      var key = this.normalizeModelUsageKey(modelId);
      if (!key || !this.modelUsageCache || typeof this.modelUsageCache !== 'object') return 0;
      var ts = Number(this.modelUsageCache[key] || 0);
      return Number.isFinite(ts) && ts > 0 ? ts : 0;
    },

    // Backward-compat shim for legacy callers during naming migration.
    modelUsageTs: function(modelId) {
      return this.modelUsageTimestamp(modelId);
    },

    recordModelUsageTimestamp: function(modelId, ts) {
      var key = this.normalizeModelUsageKey(modelId);
      if (!key) return;
      if (!this.modelUsageCache || typeof this.modelUsageCache !== 'object') {
        this.modelUsageCache = {};
      }
      var stamp = Number(ts || Date.now());
      this.modelUsageCache[key] = Number.isFinite(stamp) && stamp > 0 ? stamp : Date.now();
      this.persistModelUsageCache();
    },

    // Backward-compat shim for legacy callers during naming migration.
    touchModelUsage: function(modelId, ts) {
      this.recordModelUsageTimestamp(modelId, ts);
    },

    loadModelNoticeCache: function() {
      try {
        var raw = localStorage.getItem(this.modelNoticeCacheKey);
        if (!raw) {
          this.modelNoticeCache = {};
          return;
        }
        var parsed = JSON.parse(raw);
        this.modelNoticeCache = (parsed && typeof parsed === 'object') ? parsed : {};
      } catch {
        this.modelNoticeCache = {};
      }
    },

    persistModelNoticeCache: function() {
      try {
        localStorage.setItem(this.modelNoticeCacheKey, JSON.stringify(this.modelNoticeCache || {}));
      } catch {}
    },

    normalizeNoticeType: function(value, fallbackType) {
      var fallback = String(fallbackType || 'info').toLowerCase();
      if (fallback !== 'model' && fallback !== 'info') fallback = 'info';
      var raw = String(value || '').toLowerCase().trim();
      if (raw === 'model' || raw === 'info') return raw;
      return fallback;
    },

    isModelSwitchNoticeLabel: function(label) {
      var text = String(label || '').trim();
      if (!text) return false;
      return /^Model switched (?:to\b|from\b)/i.test(text);
    },

    rememberModelNotice: function(agentId, label, ts, noticeType, noticeIcon) {
      if (!agentId || !label) return;
      if (!this.modelNoticeCache || typeof this.modelNoticeCache !== 'object') {
        this.modelNoticeCache = {};
      }
      var key = String(agentId);
      if (!Array.isArray(this.modelNoticeCache[key])) this.modelNoticeCache[key] = [];
      var list = this.modelNoticeCache[key];
      var tsNum = Number(ts || Date.now());
      var normalizedType = this.normalizeNoticeType(
        noticeType,
        this.isModelSwitchNoticeLabel(label) ? 'model' : 'info'
      );
      var normalizedIcon = String(noticeIcon || '').trim();
      var exists = list.some(function(entry) {
        return (
          entry &&
          entry.label === label &&
          Number(entry.ts || 0) === tsNum &&
          String(entry.type || '') === normalizedType
        );
      });
      if (!exists) list.push({ label: label, ts: tsNum, type: normalizedType, icon: normalizedIcon });
      if (list.length > 120) this.modelNoticeCache[key] = list.slice(list.length - 120);
      this.persistModelNoticeCache();
    },

    mergeModelNoticesForAgent: function(agentId, rows) {
      var list = Array.isArray(rows) ? rows.slice() : [];
      if (!agentId || !this.modelNoticeCache) return list;
      var notices = this.modelNoticeCache[String(agentId)];
      if (!Array.isArray(notices) || !notices.length) return list;
      var existing = {};
      var self = this;
      list.forEach(function(msg) {
        if (!msg) return;
        var label = msg.notice_label || '';
        if (!label && msg.role === 'system' && typeof msg.text === 'string' && self.isModelSwitchNoticeLabel(msg.text.trim())) {
          label = msg.text.trim();
        }
        if (!label) return;
        var type = self.normalizeNoticeType(
          msg.notice_type,
          self.isModelSwitchNoticeLabel(label) ? 'model' : 'info'
        );
        existing[type + '|' + label + '|' + Number(msg.ts || 0)] = true;
      });
      for (var i = 0; i < notices.length; i++) {
        var n = notices[i] || {};
        var nLabel = String(n.label || '').trim();
        if (!nLabel) continue;
        var nTs = Number(n.ts || 0) || Date.now();
        var nType = this.normalizeNoticeType(
          n.type || n.notice_type,
          this.isModelSwitchNoticeLabel(nLabel) ? 'model' : 'info'
        );
        var nIcon = String(n.icon || n.notice_icon || '').trim();
        var nKey = nType + '|' + nLabel + '|' + nTs;
        if (existing[nKey]) continue;
        list.push({
          id: ++msgId,
          role: 'system',
          text: '',
          meta: '',
          tools: [],
          system_origin: 'notice:' + nType,
          is_notice: true,
          notice_label: nLabel,
          notice_type: nType,
          notice_icon: nIcon,
          ts: nTs
        });
      }
      list.sort(function(a, b) {
        return Number((a && a.ts) || 0) - Number((b && b.ts) || 0);
      });
      return list;
    },

    normalizeMessageRoleForGrouping: function(role) {
      var lower = String(role || '').trim().toLowerCase();
      if (!lower) return 'agent';
      if (lower.indexOf('user') >= 0) return 'user';
      if (lower.indexOf('system') >= 0) return 'system';
      if (lower === 'tool' || lower === 'toolresult' || lower === 'tool_result' || lower === 'toolcall' || lower === 'tool_call') return 'tool';
      if (lower.indexOf('assistant') >= 0 || lower.indexOf('agent') >= 0) return 'agent';
      return lower;
    },

    extractMessageRawText: function(message) {
      var msg = message && typeof message === 'object' ? message : {};
      if (typeof msg.content === 'string') return msg.content;
      if (Array.isArray(msg.content)) {
        var parts = msg.content.map(function(part) {
          return part && part.type === 'text' && typeof part.text === 'string' ? part.text : '';
        }).filter(function(part) { return !!part; });
        if (parts.length) return parts.join('\n');
      }
      if (typeof msg.text === 'string') return msg.text;
      if (typeof msg.message === 'string') return msg.message;
      return '';
    },

    extractMessageThinkingText: function(message) {
      var msg = message && typeof message === 'object' ? message : {};
      if (typeof msg.thinking_text === 'string' && msg.thinking_text.trim()) return msg.thinking_text.trim();
      if (Array.isArray(msg.content)) {
        var parts = msg.content.map(function(part) {
          return part && part.type === 'thinking' && typeof part.thinking === 'string' ? part.thinking.trim() : '';
        }).filter(function(part) { return !!part; });
        if (parts.length) return parts.join('\n');
      }
      var raw = this.extractMessageRawText(msg);
      if (!raw) return '';
      var matches = Array.from(raw.matchAll(/<\s*think(?:ing)?\s*>([\s\S]*?)<\s*\/\s*think(?:ing)?\s*>/gi));
      return matches.map(function(match) { return String((match && match[1]) || '').trim(); }).filter(function(part) { return !!part; }).join('\n');
    },

    extractMessageVisibleText: function(message) {
      var msg = message && typeof message === 'object' ? message : {};
      var raw = typeof msg.text === 'string' && msg.text.trim() ? msg.text : this.extractMessageRawText(msg);
      raw = String(raw || '').replace(/<\s*think(?:ing)?\s*>[\s\S]*?<\s*\/\s*think(?:ing)?\s*>/gi, ' ');
      if (typeof this.stripModelPrefix === 'function') raw = this.stripModelPrefix(raw);
      if (typeof this.sanitizeToolText === 'function') raw = this.sanitizeToolText(raw);
      if (typeof this.stripArtifactDirectivesFromText === 'function') raw = this.stripArtifactDirectivesFromText(raw);
      return raw.replace(/\s+/g, ' ').trim();
    },

    messageMatchesSearchQuery: function(message, query) {
      var normalizedQuery = String(query || '').trim().toLowerCase();
      if (!normalizedQuery) return true;
      var msg = message && typeof message === 'object' ? message : {};
      var parts = [];
      var visible = typeof msg.search_text === 'string' && msg.search_text.trim() ? msg.search_text.trim() : this.extractMessageVisibleText(msg);
      var thinking = typeof msg.thinking_text === 'string' && msg.thinking_text.trim() ? msg.thinking_text.trim() : this.extractMessageThinkingText(msg);
      if (visible) parts.push(visible);
      if (thinking) parts.push(thinking);
      if (msg.notice_label) parts.push(String(msg.notice_label));
      if (Array.isArray(msg.tools)) {
        for (var i = 0; i < msg.tools.length; i += 1) {
          var tool = msg.tools[i] || {};
          if (tool.name) parts.push(String(tool.name));
        }
      }
      return parts.join('\n').toLowerCase().indexOf(normalizedQuery) >= 0;
    },

    normalizeSessionMessages(data) {
      var source = [];
      if (data && Array.isArray(data.messages)) {
        source = data.messages;
      } else if (data && data.message_window && Array.isArray(data.message_window.rows)) {
        source = data.message_window.rows;
      } else if (data && Array.isArray(data.turns)) {
        var turns = data.turns;
        var turnRows = [];
        turns.forEach(function(turn) {
          var ts = turn && turn.ts ? turn.ts : Date.now();
          if (turn && typeof turn.user === 'string' && turn.user.trim()) {
            turnRows.push({ role: 'User', content: turn.user, ts: ts });
          }
          if (turn && typeof turn.assistant === 'string' && turn.assistant.trim()) {
            turnRows.push({ role: 'Agent', content: turn.assistant, ts: ts });
          }
        });
        source = turnRows;
      } else {
        source = [];
      }
      var self = this;
      return source.map(function(m) {
        var roleRaw = String((m && (m.role || m.type)) || '').toLowerCase();
        var isTerminal = roleRaw.indexOf('terminal') >= 0 || !!(m && m.terminal);
        var role = isTerminal ? 'terminal' : self.normalizeMessageRoleForGrouping(roleRaw);
        var textSource = m && (m.content != null ? m.content : (m.text != null ? m.text : m.message));
        if (role === 'user' && m && m.user != null) textSource = m.user;
        if (role === 'agent' && m && m.assistant != null) textSource = m.assistant;
        if (role !== 'user' && !isTerminal && typeof self.assistantTextFromPayload === 'function') {
          var structuredText = self.assistantTextFromPayload(m);
          if (structuredText || Array.isArray(textSource) || (textSource && typeof textSource === 'object')) {
            textSource = structuredText;
          }
        }
        var visibleText = self.extractMessageVisibleText(m);
        if ((!textSource || Array.isArray(textSource) || (textSource && typeof textSource === 'object')) && visibleText) {
          textSource = visibleText;
        }
        var text = typeof textSource === 'string' ? textSource : JSON.stringify(textSource || '');
        text = self.sanitizeToolText(text);
        if (isTerminal) {
          text = String(text || '')
            .replace(/\r\n/g, '\n')
            .replace(/\r/g, '\n')
            .replace(/^\s+|\s+$/g, '');
        }
        if (role === 'agent') text = self.stripModelPrefix(text);
        var derivedSystemOrigin = '';
        if (role === 'user' && /^\s*infring(?:-ops)?\s+/i.test(String(text || ''))) {
          role = 'system';
          derivedSystemOrigin = 'runtime:ops_command';
        }
        if (role === 'user' && /^\s*\[runtime-task\]/i.test(String(text || ''))) {
          role = 'system';
          if (!derivedSystemOrigin) derivedSystemOrigin = 'runtime:task';
        }

        var tools = typeof self.responseToolRowsFromPayload === 'function'
          ? self.responseToolRowsFromPayload(m, 'hist-tool')
          : ((m && Array.isArray(m.tools) ? m.tools : []).map(function(t, idx) {
              return {
                id: (t.name || 'tool') + '-hist-' + idx,
                name: t.name || 'unknown',
                running: false,
                expanded: false,
                status: t.status || (t.is_error ? 'error' : 'done'),
                summary: t.summary || t.label || t.status || '',
                input_ref: t.input_ref || t.detail_ref || '',
                result_ref: t.result_ref || t.detail_ref || '',
                detail_ref: t.detail_ref || t.result_ref || t.input_ref || '',
                is_error: !!t.is_error
              };
            }));
        if (role === 'agent' && !isTerminal) {
          var repairedToolText = '';
          var needsRepair =
            !String(text || '').trim() ||
            (typeof self.textLooksNoFindingsPlaceholder === 'function' && self.textLooksNoFindingsPlaceholder(text)) ||
            (typeof self.textLooksToolAckWithoutFindings === 'function' && self.textLooksToolAckWithoutFindings(text));
          if (needsRepair && typeof self.fallbackAssistantTextFromPayload === 'function') {
            repairedToolText = String(self.fallbackAssistantTextFromPayload(m, tools) || '').trim();
          }
          if (
            repairedToolText &&
            repairedToolText !== String(text || '').trim() &&
            !(typeof self.textLooksNoFindingsPlaceholder === 'function' && self.textLooksNoFindingsPlaceholder(repairedToolText))
          ) {
            text = repairedToolText;
          }
        }
        var messageMetadata = typeof self.assistantTurnMetadataFromPayload === 'function'
          ? self.assistantTurnMetadataFromPayload(m, tools)
          : {};
        var images = (m && Array.isArray(m.images) ? m.images : []).map(function(img) {
          return { file_id: img.file_id, filename: img.filename || 'image' };
        });
        var tsRaw = m && (m.ts || m.timestamp || m.created_at || m.createdAt) ? (m.ts || m.timestamp || m.created_at || m.createdAt) : null;
        var ts = null;
        if (typeof tsRaw === 'number') {
          ts = tsRaw;
        } else if (typeof tsRaw === 'string') {
          var parsedTs = Date.parse(tsRaw);
          ts = Number.isNaN(parsedTs) ? null : parsedTs;
        }
        var meta = typeof (m && m.meta) === 'string' ? m.meta : '';
        if (!meta && m && (m.input_tokens || m.output_tokens)) {
          meta = (m.input_tokens || 0) + ' in / ' + (m.output_tokens || 0) + ' out';
        }
        var isNotice = false;
        var noticeLabel = '';
        var noticeType = '';
        var noticeIcon = '';
        var noticeAction = null;
        if (m && (m.is_notice || m.notice_label || m.notice_type)) {
          var explicitLabel = String(m.notice_label || '').trim();
          var inferredLabel = typeof text === 'string' ? text.trim() : '';
          noticeLabel = explicitLabel || inferredLabel;
          if (noticeLabel) {
            isNotice = true;
            text = '';
            noticeType = self.normalizeNoticeType(
              m.notice_type,
              self.isModelSwitchNoticeLabel(noticeLabel) ? 'model' : 'info'
            );
            noticeIcon = String(m.notice_icon || '').trim();
            noticeAction = self.normalizeNoticeAction(m.notice_action || m.noticeAction || null);
          }
        }
        if (!isNotice && role === 'system' && typeof text === 'string') {
          var compact = text.trim();
          if (self.isModelSwitchNoticeLabel(compact)) {
            isNotice = true;
            noticeLabel = compact;
            text = '';
            noticeType = 'model';
          }
        }
        var systemOrigin = m && m.system_origin ? String(m.system_origin) : derivedSystemOrigin;
        var compactText = typeof text === 'string' ? text.trim() : '';
        if (
          role === 'system' &&
          !isNotice &&
          !systemOrigin &&
          (
            /^\[runtime-task\]/i.test(compactText) ||
            /^task accepted\.\s*report findings in this thread with receipt-backed evidence\.?$/i.test(compactText)
          )
        ) {
          // Legacy synthetic runtime-task chatter (no origin tag) is noise; skip rendering.
          return null;
        }
        if (
          role === 'system' &&
          !isNotice &&
          self.isSystemNotificationGlobalToWorkspace &&
          self.isSystemNotificationGlobalToWorkspace(systemOrigin, compactText) &&
          !(self.isSystemThreadAgent && self.isSystemThreadAgent(self.currentAgent))
        ) {
          // Keep global/system-wide notices out of non-system chats.
          return null;
        }
        var thinkingText = self.extractMessageThinkingText(m);
        return Object.assign({
          id: ++msgId,
          role: role,
          text: text,
          search_text: visibleText || compactText,
          thinking_text: thinkingText,
          meta: meta,
          tools: tools,
          images: images,
          ts: ts,
          is_notice: isNotice,
          notice_label: noticeLabel,
          notice_type: noticeType,
          notice_icon: noticeIcon,
          notice_action: noticeAction,
          terminal: isTerminal,
          terminal_source: m && m.terminal_source ? String(m.terminal_source).toLowerCase() : (isTerminal ? 'user' : ''),
          cwd: m && m.cwd ? String(m.cwd) : '',
          agent_id: m && m.agent_id ? String(m.agent_id) : '',
          agent_name: m && m.agent_name ? String(m.agent_name) : '',
          source_agent_id: m && m.source_agent_id ? String(m.source_agent_id) : '',
          agent_origin: m && m.agent_origin ? String(m.agent_origin) : '',
          system_origin: systemOrigin,
          actor_id: m && m.actor_id ? String(m.actor_id) : '',
          actor: m && m.actor ? String(m.actor) : '',
          render_height_px: Number.isFinite(Number(m && m.render_height_px)) ? Math.max(0, Math.round(Number(m.render_height_px))) : 0,
          render_width_bucket_px: Number.isFinite(Number(m && m.render_width_bucket_px)) ? Math.max(0, Math.round(Number(m.render_width_bucket_px))) : 0,
          render_measured_at: Number.isFinite(Number(m && m.render_measured_at)) ? Math.max(0, Math.round(Number(m.render_measured_at))) : 0
        }, messageMetadata || {});
      }).filter(function(row) { return !!row; });
    },

    isSystemNotificationGlobalToWorkspace: function(systemOrigin, text) {
      var origin = String(systemOrigin || '').trim().toLowerCase();
      var msg = String(text || '').trim().toLowerCase();
      if (!origin && !msg) return false;
      if (
        origin.indexOf('telemetry:') === 0 ||
        origin.indexOf('continuity:') === 0 ||
        origin === 'slash:alerts' ||
        origin === 'slash:next' ||
        origin === 'slash:memory' ||
        origin === 'slash:continuity' ||
        origin === 'slash:opt'
      ) {
        return true;
      }
      if (
        msg.indexOf('memory-backed session context') >= 0 ||
        msg.indexOf('stale memory context') >= 0 ||
        msg.indexOf('continuity cleanup') >= 0 ||
        msg.indexOf('cross-channel continuity') >= 0
      ) {
        return true;
      }
      return false;
    },

    installComposerPlusClickFallback: function() {
      if (typeof document === 'undefined' || this._composerPlusClickFallbackInstalled) return;
      this._composerPlusClickFallbackInstalled = true;
      var self = this;
      document.addEventListener('click', function(event) {
        var target = event && event.target;
        var anchor = target && target.closest ? target.closest('#composer-plus-menu-anchor') : null;
        if (!anchor) return;
        var previous = !!self.showAttachMenu;
        setTimeout(function() {
          if (!!self.showAttachMenu !== previous) return;
          if (typeof self.closeComposerMenus === 'function') self.closeComposerMenus({ attach: true });
          else {
            self.showModelSwitcher = false;
            self.showRuntimeSwitcher = false;
            if (typeof self.closeGitTreeMenu === 'function') self.closeGitTreeMenu();
            else self.showGitTreeMenu = false;
          }
          self.showAttachMenu = !previous;
          if (self.showAttachMenu && typeof self.loadActiveWorkspaceProjection === 'function') {
            Promise.resolve(self.loadActiveWorkspaceProjection({ force: false })).catch(function() {});
          }
        }, 0);
      }, true);
    },

    publishChatPageBridge: function() {
      if (typeof window === 'undefined') return;
      window.InfringChatPage = this;
      try {
        if (document && document.body) {
          document.body.__infringChatPage = this;
          document.body.setAttribute('data-infring-chat-page-bridge', 'ready');
        }
        var wrappers = document && document.querySelectorAll ? document.querySelectorAll('.chat-wrapper') : [];
        for (var i = 0; i < wrappers.length; i += 1) {
          wrappers[i].__infringChatPage = this;
          wrappers[i].setAttribute('data-infring-chat-page-bridge', 'ready');
        }
      } catch (_) {}
    },

    init() {
      var self = this;
      this.publishChatPageBridge();
      this.installComposerPlusClickFallback();

      if (typeof window !== 'undefined') {
        var persistedCache = this.loadConversationCache();
        var runtimeCache = window.__infringChatCache && typeof window.__infringChatCache === 'object'
          ? this.sanitizeConversationCacheForPersistence(window.__infringChatCache)
          : {};

      this.loadModelNoticeCache();
      this.loadModelUsageCache();
      this.loadInputHistoryCache();
      this.loadPromptSuggestionsPreference();

      // Start tip cycle
      this.startTipCycle();

      // Fetch dynamic commands from server
      this.fetchCommands();
      this.loadSlashAliases();
      this.fetchModelContextWindows();
      this.fetchProactiveTelemetryAlerts(false);
      this.refreshCurrentAgentSessionListIfStale = function(reason, maxAgeMs) {
        var agentId = String(self.currentAgent && self.currentAgent.id ? self.currentAgent.id : '').trim();
        if (!agentId) return;
        if (!self._sessionsLastLoadedAtByAgent || typeof self._sessionsLastLoadedAtByAgent !== 'object') {
          self._sessionsLastLoadedAtByAgent = {};
        }
        var normalizedAgentId = typeof self.normalizeSessionAgentId === 'function'
          ? self.normalizeSessionAgentId(agentId)
          : agentId.toLowerCase();
        var lastLoadedAt = Number(self._sessionsLastLoadedAtByAgent[normalizedAgentId] || 0);
        var ttlMs = Number(maxAgeMs || 0);
        if (!Number.isFinite(ttlMs) || ttlMs < 2000) ttlMs = 15000;
        if (lastLoadedAt > 0 && (Date.now() - lastLoadedAt) < ttlMs) return;
        Promise.resolve(self.loadSessions(agentId)).catch(function() { return []; });
      };
      this._chatFocusSessionRefreshHandler = function() {
        if (document && document.visibilityState && document.visibilityState === 'hidden') return;
        self.refreshCurrentAgentSessionListIfStale('focus', 15000);
      };
      this._chatVisibilitySessionRefreshHandler = function() {
        if (!document || document.visibilityState !== 'visible') return;
        self.refreshCurrentAgentSessionListIfStale('visibility', 15000);
      };
      window.addEventListener('focus', this._chatFocusSessionRefreshHandler);
      document.addEventListener('visibilitychange', this._chatVisibilitySessionRefreshHandler);

      // Ctrl+/ keyboard shortcut
      document.addEventListener('keydown', function(e) {
        var key = String(e && e.key ? e.key : '').toLowerCase();
        // Ctrl+T or Ctrl+\ toggles terminal compose mode.
        if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && (key === 't' || key === '\\') && self.currentAgent) {
          e.preventDefault();
          self.toggleTerminalMode();
          return;
        }
        if ((e.ctrlKey || e.metaKey) && e.key === '/') {
          e.preventDefault();
          var input = document.getElementById('msg-input');
          if (input) { input.focus(); self.inputText = '/'; }
        }
        // Ctrl+M for model switcher
        if ((e.ctrlKey || e.metaKey) && e.key === 'm' && self.currentAgent) {
          e.preventDefault();
          self.toggleModelSwitcher();
        }
        // Ctrl+F opens file picker from chat compose.
        if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && key === 'f' && self.currentAgent) {
          e.preventDefault();
          self.beginAttachPickerSession();
          return;
        }
        // Ctrl+G for chat search
        if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && key === 'g' && self.currentAgent) {
          e.preventDefault();
          self.toggleSearch();
        }
      });

      if (this._sendWatchdogTimer) clearInterval(this._sendWatchdogTimer);
      this._sendWatchdogTimer = setInterval(function() {
        if (self.sending) self._reconcileSendingState();
      }, 3000);
      window.addEventListener('beforeunload', function() {
        self.handleMessagesPointerUp(null);
        if (self._sendWatchdogTimer) {
          clearInterval(self._sendWatchdogTimer);
          self._sendWatchdogTimer = null;
        }
        if (self._telemetryAlertsTimer) {
          clearInterval(self._telemetryAlertsTimer);
          self._telemetryAlertsTimer = null;
        }
        if (self._agentTrailListenTimer) {
          clearTimeout(self._agentTrailListenTimer);
          self._agentTrailListenTimer = 0;
        }
        self.teardownChatResizeBlurObserver();
        self.teardownChatInputOverlayObserver();
        self.stopAgentTrailLoop(true);
        if (self._chatFocusSessionRefreshHandler) {
          window.removeEventListener('focus', self._chatFocusSessionRefreshHandler);
          self._chatFocusSessionRefreshHandler = null;
        }
        if (self._chatVisibilitySessionRefreshHandler) {
          document.removeEventListener('visibilitychange', self._chatVisibilitySessionRefreshHandler);
          self._chatVisibilitySessionRefreshHandler = null;
        }
      });

        this.conversationCache = this.sanitizeConversationCacheForPersistence(Object.assign({}, persistedCache, runtimeCache));
        try { delete window.__infringChatCache; } catch (_) { window.__infringChatCache = {}; }
      }
      this.publishChatPageBridge();
      if (typeof this.loadRuntimeEnginesSafely === 'function') {
        this.loadRuntimeEnginesSafely({ force: false }).catch(function() {});
      }
      // Load session + session list when agent changes
      this.$watch('currentAgent', function(agent) {
        if (agent) {
          self.loadSessions(agent.id);
          self.setContextWindowFromCurrentAgent();
          self.requestContextTelemetry(true);
          self.refreshPromptSuggestions(false);
          self.checkForSystemReleaseUpdate(false);
        } else {
          self.clearPromptSuggestions();
        }
        self.$nextTick(function() {
          self.installChatInputOverlayObserver();
          self.refreshChatInputOverlayMetrics();
        });
        var chatStore = window.InfringChatStore;
        if (chatStore && chatStore.currentAgent) chatStore.currentAgent.set(agent || null);
      });

      this.$watch('messages.length', function() {
        self.$nextTick(function() {
          self.scrollToBottom({ force: false });
        });
      });

      this.$watch('messages', function(val) {
        var chatStore = window.InfringChatStore;
        if (!chatStore) return;
        if (typeof chatStore.syncMessages === 'function') chatStore.syncMessages(val, self.allFilteredMessages);
      });

      this.$watch('searchQuery', function() {
        var chatStore = window.InfringChatStore;
        if (chatStore && typeof chatStore.syncMessages === 'function') chatStore.syncMessages(self.messages, self.allFilteredMessages);
      });

      this.$watch('sessionLoading', function(val) {
        var chatStore = window.InfringChatStore;
        if (chatStore && chatStore.sessionLoading) chatStore.sessionLoading.set(!!val);
      });

      this.$watch('sending', function(val) {
        var chatStore = window.InfringChatStore;
        if (chatStore && chatStore.sending) chatStore.sending.set(!!val);
      });

      this.$watch('tokenCount', function(val) {
        var chatStore = window.InfringChatStore;
        if (chatStore && chatStore.tokenCount) chatStore.tokenCount.set(Number(val) || 0);
      });

      this.$watch('inputText', function(val) {
        var chatStore = window.InfringChatStore;
        if (chatStore && chatStore.inputText) chatStore.inputText.set(typeof val === 'string' ? val : '');
      });

      this.$watch('showScrollDown', function(val) {
        var chatStore = window.InfringChatStore;
        if (chatStore && chatStore.showScrollDown) chatStore.showScrollDown.set(!!val);
      });

      this.$watch('_stickToBottom', function(val) {
        var chatStore = window.InfringChatStore;
        if (chatStore && chatStore.stickToBottom) chatStore.stickToBottom.set(!!val);
      });

      this.$watch('mapStepIndex', function(val) {
        var chatStore = window.InfringChatStore;
        if (chatStore && chatStore.mapStepIndex) chatStore.mapStepIndex.set(Number(val) || -1);
        if (chatStore && typeof chatStore.setThreadProjectionCenter === 'function') chatStore.setThreadProjectionCenter(val);
      });

      this.$watch('terminalMode', function() {
        self.$nextTick(function() {
          self.refreshChatInputOverlayMetrics();
        });
      });

      this.$watch('attachments.length', function() {
        self.$nextTick(function() {
          self.refreshChatInputOverlayMetrics();
        });
      });

      // Check for pending agent from Agents page (set before chat mounted)
      var store = Alpine.store('app');
      if (store.pendingAgent) {
        self.selectAgent(store.pendingAgent);
      } else if (store.activeAgentId) {
        self.selectAgent(store.activeAgentId);
      } else {
        var preferred = self.pickDefaultAgent(store.agents || []);
        if (preferred) self.selectAgent(preferred);
      }

      // Watch for future pending agent selections (e.g., user clicks agent while on chat)
      this.$watch('$store.app.pendingAgent', function(agent) {
        if (agent) {
          self.selectAgent(agent);
        }
      });

      // Keep chat selection synced when an explicit active agent is set globally.
      this.$watch('$store.app.activeAgentId', function(agentId) {
        if (!agentId) return;
        if (!self.currentAgent || self.currentAgent.id !== agentId) {
          self.selectAgent(agentId);
        }
      });

      this.$watch('$store.app.chatSidebarVisibleRows', function(rows) {
        var chatStore = window.InfringChatStore;
        if (chatStore && chatStore.sidebarAgents) chatStore.sidebarAgents.set(Array.isArray(rows) ? rows : []);
      });

      this.$watch('$store.app.focusMode', function(val) {
        var chatStore = window.InfringChatStore;
        if (chatStore && chatStore.focusMode) chatStore.focusMode.set(!!val);
      });

      this.$watch('$store.app.connectionState', function(val) {
        var chatStore = window.InfringChatStore;
        if (chatStore && chatStore.connectionState) chatStore.connectionState.set(String(val || ''));
      });

      this.$watch('$store.app.theme', function(val) {
        var chatStore = window.InfringChatStore;
        if (chatStore && chatStore.theme) chatStore.theme.set(String(val || ''));
      });

      // Auto-select the first available agent in chat mode.
      this.$watch('$store.app.agents', function(agents) {
        var store = Alpine.store('app');
        var rows = Array.isArray(agents) ? agents : [];
        var chatStore = window.InfringChatStore;
        if (chatStore && chatStore.agents) chatStore.agents.set(rows);
        self.fetchModelContextWindows();
        if (self.currentAgent && self.isSystemThreadAgent && self.isSystemThreadAgent(self.currentAgent)) {
          self._agentMissingAgentId = '';
          self._agentMissingSince = 0;
          self.currentAgent = self.makeSystemThreadAgent();
          if (!store || !store.activeAgentId || !self.isSystemThreadId || !self.isSystemThreadId(store.activeAgentId)) {
            self.setStoreActiveAgentId(self.systemThreadId || 'system');
          }
          return;
        }
        if (self.currentAgent && self.currentAgent.id) {
          var currentLive = null;
          for (var ai = 0; ai < rows.length; ai++) {
            if (rows[ai] && String(rows[ai].id) === String(self.currentAgent.id)) {
              currentLive = rows[ai];
              break;
            }
          }
          if (!currentLive) {
            if (self.shouldSuppressAgentInactive(self.currentAgent.id)) return;
            var connectionState = String((store && store.connectionState) || '').toLowerCase();
            if (connectionState && connectionState !== 'connected') return;
            var currentId = String(self.currentAgent.id);
            var now = Date.now();
            if (self._agentMissingAgentId !== currentId) {
              self._agentMissingAgentId = currentId;
              self._agentMissingSince = now;
              return;
            }
            var missingForMs = self._agentMissingSince > 0 ? now - self._agentMissingSince : 0;
            var graceMs = Number(self._agentMissingGraceMs || 0);
            if (graceMs > 0 && missingForMs < graceMs) return;
            self._agentMissingAgentId = '';
            self._agentMissingSince = 0;
            self.handleAgentInactive(self.currentAgent.id, 'inactive', { silentNotice: true });
          } else {
            self._agentMissingAgentId = '';
            self._agentMissingSince = 0;
            if (!self.syncCurrentAgentFromStore(currentLive)) {
              self.currentAgent = currentLive;
            }
          }
        }
        if (store.activeAgentId) {
          var resolved = self.resolveAgent(store.activeAgentId);
          if (resolved) {
            if (!self.currentAgent || self.currentAgent.id !== resolved.id) {
              self.selectAgent(resolved);
            } else {
              // Refresh visible metadata without resetting the thread.
              self.syncCurrentAgentFromStore(resolved);
            }
            return;
          }
        }
        if (!self.currentAgent) {
          var preferred = self.pickDefaultAgent(agents || []);
          if (preferred) self.selectAgent(preferred);
        }
      });

      // Watch for slash commands + model autocomplete
      this.$watch('inputText', function(val) {
        if (!self._inputHistoryApplying) {
          self.resetInputHistoryNavigation(self.terminalMode ? 'terminal' : 'chat');
        }
        var hasTyping = String(val == null ? '' : val).length > 0;
        if (self._agentTrailListenTimer) {
          clearTimeout(self._agentTrailListenTimer);
          self._agentTrailListenTimer = 0;
        }
        if (hasTyping) {
          self._agentTrailListening = true;
          self.setAgentTrailBlinkState(true);
          if (self._agentTrailRaf) {
            try { cancelAnimationFrame(self._agentTrailRaf); } catch(_) {}
            self._agentTrailRaf = 0;
          }
          self._agentTrailListenTimer = setTimeout(function() {
            self._agentTrailListenTimer = 0;
            self._agentTrailListening = false;
            self.setAgentTrailBlinkState(false);
            self.startAgentTrailLoop();
          }, 1000);
        } else if (self._agentTrailListening) {
          // Keep the "listening" pulse alive briefly after typing stops so
          // the transition feels intentional instead of abrupt.
          self._agentTrailListenTimer = setTimeout(function() {
            self._agentTrailListenTimer = 0;
            self._agentTrailListening = false;
            self.setAgentTrailBlinkState(false);
            if (!self._agentTrailRaf) self.startAgentTrailLoop();
          }, 1000);
        }
        if (self.terminalMode) {
          self.updateTerminalCursor();
          self.showSlashMenu = false;
          self.showModelPicker = false;
          return;
        }
        var modelMatch = val.match(/^\/model\s+(.*)$/i);
        if (modelMatch) {
          self.showSlashMenu = false;
          self.modelPickerFilter = modelMatch[1].toLowerCase();
          if (!self.modelPickerList.length) {
            InfringAPI.post('/api/shell-socket/models/discover', { input: '__auto__' })
              .catch(function() { return null; })
              .then(function() { return InfringAPI.get('/api/shell-socket/models'); })
              .then(function(data) {
              self.modelPickerList = self.sanitizeModelCatalogRows((data && data.models) || []);
              if (self.availableModelRowsCount(self.modelPickerList) === 0) {
                self.injectNoModelsGuidance('slash_model');
              }
              self.showModelPicker = true;
              self.modelPickerIdx = 0;
            }).catch(function() {});
          } else {
            self.showModelPicker = true;
          }
        } else if (val.startsWith('/')) {
          self.showModelPicker = false;
          self.slashFilter = val.slice(1).toLowerCase();
          self.showSlashMenu = true;
          self.slashIdx = 0;
        } else {
          self.showSlashMenu = false;
          self.showModelPicker = false;
        }
      });

      this.$nextTick(function() {
        self.handleMessagesScroll();
        self.startAgentTrailLoop();
        self.installChatMapWheelLock();
        self.scheduleMessageRenderWindowUpdate();
        self.installChatResizeBlurObserver();
        self.installChatInputOverlayObserver();
        self.refreshChatInputOverlayMetrics();
      });

      InfringAPI.get('/api/status').then(function(status) {
        var suggested = status && (status.workspace_dir || status.root_dir || status.home_dir)
          ? String(status.workspace_dir || status.root_dir || status.home_dir)
          : '';
        if (suggested) self.terminalCwd = suggested;
      }).catch(function() {});

      this.refreshModelCatalogAndGuidance({ discover: true, guidance: true }).catch(function() {});

      if (this._contextTelemetryTimer) clearInterval(this._contextTelemetryTimer);
      this._contextTelemetryTimer = setInterval(function() {
        self.requestContextTelemetry(false);
      }, 8000);
      if (this._telemetryAlertsTimer) clearInterval(this._telemetryAlertsTimer);
      this._telemetryAlertsTimer = setInterval(function() {
        self.fetchProactiveTelemetryAlerts(true);
      }, 15000);

      (function() {
        var chatStore = window.InfringChatStore;
        if (!chatStore) return;
        if (typeof chatStore.syncMessages === 'function') chatStore.syncMessages(self.messages, self.allFilteredMessages);
        if (chatStore.currentAgent) chatStore.currentAgent.set(self.currentAgent || null);
        var appStore = typeof Alpine !== 'undefined' && Alpine.store('app');
        if (chatStore.agents) chatStore.agents.set(Array.isArray(appStore && appStore.agents) ? appStore.agents : []);
        if (chatStore.sidebarAgents) chatStore.sidebarAgents.set(Array.isArray(appStore && appStore.chatSidebarVisibleRows) ? appStore.chatSidebarVisibleRows : []);
        if (chatStore.sessionLoading) chatStore.sessionLoading.set(!!self.sessionLoading);
        if (chatStore.sending) chatStore.sending.set(!!self.sending);
        if (chatStore.tokenCount) chatStore.tokenCount.set(Number(self.tokenCount) || 0);
        if (chatStore.inputText) chatStore.inputText.set(typeof self.inputText === 'string' ? self.inputText : '');
        if (chatStore.showScrollDown) chatStore.showScrollDown.set(!!self.showScrollDown);
        if (chatStore.stickToBottom) chatStore.stickToBottom.set(self._stickToBottom !== false);
        if (chatStore.mapStepIndex) chatStore.mapStepIndex.set(Number(self.mapStepIndex) || -1);
        if (chatStore.wsConnected) {
          chatStore.wsConnected.set(!!(
            (appStore && appStore.wsConnected) ||
            (typeof InfringAPI !== 'undefined' && typeof InfringAPI.isWsConnected === 'function' && InfringAPI.isWsConnected())
          ));
        }
        if (appStore) {
          if (chatStore.focusMode) chatStore.focusMode.set(!!appStore.focusMode);
          if (chatStore.connectionState) chatStore.connectionState.set(String(appStore.connectionState || ''));
          if (chatStore.theme) chatStore.theme.set(String(appStore.theme || ''));
        }
      }());
      window.InfringChatPage = self;
      if (typeof Alpine !== 'undefined' && !window.InfringApp) window.InfringApp = Alpine.store('app');
    },

    toggleTerminalMode() {
      var self = this;
      // infring-chat-page-early-bridge: Svelte Shell islands need the page bridge even if later init work fails.
      if (typeof window !== 'undefined') {
        window.InfringChatPage = self;
        if (typeof Alpine !== 'undefined' && !window.InfringApp) window.InfringApp = Alpine.store('app');
      }
      if (this.isSystemThreadAgent && this.isSystemThreadAgent(this.currentAgent)) {
        this.terminalMode = true;
        if (typeof this.closeComposerMenus === 'function') this.closeComposerMenus();
        else {
          this.showAttachMenu = false;
          this.showModelSwitcher = false;
          if (typeof this.closeGitTreeMenu === 'function') this.closeGitTreeMenu();
          else this.showGitTreeMenu = false;
        }
        this.showSlashMenu = false;
        this.showModelPicker = false;
        this.terminalCursorFocused = false;
        this.$nextTick(function() {
          if (typeof self.closeComposerMenus === 'function') self.closeComposerMenus();
          var input = document.getElementById('msg-input');
          if (input) input.focus();
          self.refreshChatInputOverlayMetrics();
        });
        return;
      }
      if (typeof this.closeComposerMenus === 'function') this.closeComposerMenus();
      else {
        this.showAttachMenu = false;
        this.showModelSwitcher = false;
        if (typeof this.closeGitTreeMenu === 'function') this.closeGitTreeMenu();
        else this.showGitTreeMenu = false;
      }
      this.showSlashMenu = false;
      this.showModelPicker = false;
      this.terminalMode = !this.terminalMode;
      this.resetInputHistoryNavigation('chat');
      this.resetInputHistoryNavigation('terminal');
      this.terminalCursorFocused = false;
      if (!this.terminalMode) this.terminalSelectionStart = 0;
      if (this.terminalMode && !this.terminalCwd) {
        this.terminalCwd = '/workspace';
      }
      if (this.terminalMode && this.currentAgent) {
        this.connectWs(this.currentAgent.id);
      }
      if (this.terminalMode && Array.isArray(this.attachments) && this.attachments.length) {
        for (var i = 0; i < this.attachments.length; i++) {
          if (this.attachments[i] && this.attachments[i].preview) {
            try { URL.revokeObjectURL(this.attachments[i].preview); } catch(_) {}
          }
        }
        this.attachments = [];
      }
      this.$nextTick(function() {
        if (typeof self.closeComposerMenus === 'function') self.closeComposerMenus();
        var input = document.getElementById('msg-input');
        if (input) {
          input.focus();
          if (self.terminalMode) {
            self.setTerminalCursorFocus(true, { target: input });
            self.updateTerminalCursor({ target: input });
          }
        }
        self.scheduleConversationPersist();
        self.refreshChatInputOverlayMetrics();
      });
    },

    setTerminalCursorFocus(active, event) {
      if (!this.terminalMode) {
        this.terminalCursorFocused = false;
        return;
      }
      this.terminalCursorFocused = !!active;
      if (this.terminalCursorFocused) this.updateTerminalCursor(event);
    },

    updateTerminalCursor(event) {
      if (!this.terminalMode) {
        this.terminalSelectionStart = 0;
        return;
      }
      var text = String(this.inputText || '');
      var active = (typeof document !== 'undefined' && document.activeElement && document.activeElement.id === 'msg-input')
        ? document.activeElement
        : null;
      var el = event && event.target ? event.target : (active || document.getElementById('msg-input'));
      var pos = text.length;
      if (el && Number.isFinite(Number(el.selectionStart))) pos = Number(el.selectionStart);
      if (!Number.isFinite(pos) || pos < 0) pos = text.length;
      if (pos > text.length) pos = text.length;
      this.terminalSelectionStart = Math.floor(pos);
    },
    installChatMapWheelLock() {
      var self = this;
      var maps = document.querySelectorAll('.chat-map-scroll');
      for (var i = 0; i < maps.length; i++) {
        var map = maps[i];
        if (!map || map.__ofWheelLock) continue;
        map.__ofWheelLock = true;
        map.addEventListener('wheel', function(ev) {
          var target = ev.currentTarget;
          if (!target) return;
          if (!target.matches(':hover')) return;
          // Keep wheel behavior scoped to chat map so the page does not scroll beneath it.
          var delta = Number(ev.deltaY || 0);
          if (delta !== 0) {
            target.scrollTop += delta;
          }
          ev.preventDefault();
        }, { passive: false });
      }
      var scrollers = document.querySelectorAll('.messages#messages');
      for (var si = 0; si < scrollers.length; si++) {
        var scroller = scrollers[si];
        if (!scroller || scroller.__ofBottomWheelLock) continue;
        scroller.__ofBottomWheelLock = true;
        scroller.addEventListener('wheel', function(ev) {
          self._lastMessagesWheelAt = Date.now();
          if (Number(ev.deltaY || 0) <= 0) return;
          self._stickToBottom = true;
        }, { passive: true });
      }
    },
    anchorAgentTrailToThinking(host, hostRect, now, pad, w, h) {
      if (!host || typeof host.querySelectorAll !== 'function') return false;
      var self = this;
      var pinToLastThinkingAnchor = function() {
        var s = self._agentTrailState || null;
        if (!self.freshInitLaunching || !s || String(s.anchorMode || '') !== 'thinking') return false;
        var x = Number(s.anchorTargetX);
        var y = Number(s.anchorTargetY);
        if (!Number.isFinite(x) || !Number.isFinite(y)) {
          x = Number(s.x);
          y = Number(s.y);
        }
        if (!Number.isFinite(x) || !Number.isFinite(y)) return false;
        x = Math.max(pad + 1, Math.min(w - (pad + 1), x));
        y = Math.max(pad + 1, Math.min(h - (pad + 1), y));
        s.x = x; s.y = y; s.vx = 0; s.vy = 0; s.trailX = x; s.trailY = y; s.anchorLastAt = now;
        self._agentTrailState = s;
        self.ensureAgentTrailOrb(host, x, y);
        self.setAgentTrailBlinkState(true);
        host.style.setProperty('--chat-agent-grid-active', '1');
        host.style.setProperty('--chat-agent-grid-x', Math.round(x) + 'px');
        host.style.setProperty('--chat-agent-grid-y', Math.round(y) + 'px');
        return true;
      };
      var bubbles = host.querySelectorAll('.message.thinking .message-bubble.message-bubble-thinking');
      if (!bubbles || !bubbles.length) {
        if (pinToLastThinkingAnchor()) return true;
        if (!this._agentTrailListening) this.setAgentTrailBlinkState(false);
        return false;
      }
      var rect = hostRect && Number.isFinite(Number(hostRect.width || 0)) ? hostRect : host.getBoundingClientRect();
      var anchor = null;
      for (var i = bubbles.length - 1; i >= 0; i--) {
        var bubble = bubbles[i];
        if (!bubble || bubble.offsetParent === null) continue;
        var bubbleRect = bubble.getBoundingClientRect();
        if (!(Number(bubbleRect.width || 0) > 0 && Number(bubbleRect.height || 0) > 0)) continue;
        if (bubbleRect.bottom < rect.top || bubbleRect.top > rect.bottom || bubbleRect.right < rect.left || bubbleRect.left > rect.right) continue;
        // Pin the autonomous agent orb outside the bottom-left edge of
        // the active thinking dialog while the agent is working.
        // Keep a 1.5rem diagonal offset so the orb stays closer while thinking.
        var remPx = 16;
        try {
          var root = document && document.documentElement
            ? window.getComputedStyle(document.documentElement)
            : null;
          var rootFont = root ? parseFloat(String(root.fontSize || '16')) : 16;
          if (Number.isFinite(rootFont) && rootFont > 0) remPx = rootFont;
        } catch (_) {}
        var orbOffset = remPx * 1.5;
        anchor = { x: (bubbleRect.left - rect.left) - orbOffset, y: (bubbleRect.bottom - rect.top) + orbOffset };
        break;
      }
      if (!anchor) {
        if (pinToLastThinkingAnchor()) return true;
        if (!this._agentTrailListening) this.setAgentTrailBlinkState(false);
        return false;
      }
      var targetX = Math.max(pad + 1, Math.min(w - (pad + 1), Number(anchor.x || 0)));
      var targetY = Math.max(pad + 1, Math.min(h - (pad + 1), Number(anchor.y || 0)));
      var s = this._agentTrailState;
      var enteredThinking = !s || String(s.anchorMode || '') !== 'thinking';
      var x = NaN;
      var y = NaN;
      if (s && Number.isFinite(Number(s.x)) && Number.isFinite(Number(s.y))) {
        x = Number(s.x);
        y = Number(s.y);
      } else if (this._agentTrailOrbEl && this._agentTrailOrbEl.isConnected && this._agentTrailOrbEl.parentNode === host) {
        x = Number(parseFloat(String(this._agentTrailOrbEl.style.left || 'NaN')));
        y = Number(parseFloat(String(this._agentTrailOrbEl.style.top || 'NaN')));
        if (!Number.isFinite(x)) x = Number(this._agentTrailOrbEl.offsetLeft || NaN);
        if (!Number.isFinite(y)) y = Number(this._agentTrailOrbEl.offsetTop || NaN);
      }
      if (!Number.isFinite(x) || !Number.isFinite(y)) {
        x = targetX;
        y = targetY;
      }
      if (!s) {
        s = { x: x, y: y, vx: 0, vy: 0, dir: 0, target: 0, turnAt: now + 1000 };
      }
      var lastAnchorAt = Number(s.anchorLastAt || 0);
      var dt = lastAnchorAt > 0 ? Math.min(0.08, Math.max(0.001, (now - lastAnchorAt) / 1000)) : (1 / 60);
      var dx = targetX - x;
      var dy = targetY - y;
      var dist = Math.sqrt((dx * dx) + (dy * dy));
      if (enteredThinking) dist = 0;
      if (dist > 0.001) {
        // Move in a straight line into the thinking anchor, never teleport.
        var maxStep = 1480 * dt;
        if (dist <= maxStep) {
          x = targetX;
          y = targetY;
        } else {
          x += (dx / dist) * maxStep;
          y += (dy / dist) * maxStep;
        }

      } else {
        x = targetX;
        y = targetY;
      }
      s.x = x;
      s.y = y;
      s.vx = 0;
      s.vy = 0;
      s.trailX = x;
      s.trailY = y;
      s.anchorMode = 'thinking';
      s.anchorTargetX = targetX;
      s.anchorTargetY = targetY;
      s.anchorLastAt = now;
      this._agentTrailState = s;
      this._agentTrailSeeded = true;
      this._agentTrailLastDotAt = now;
      if (enteredThinking && this._agentTrailOrbEl) {
        // Promote + mark listening before reposition so ensureAgentTrailOrb
        // performs the teleport path instead of easing from the last spot.
        this.setAgentTrailBlinkState(true, this._agentTrailOrbEl);
      }
      var orb = this.ensureAgentTrailOrb(host, x, y);
      this.setAgentTrailBlinkState(true, orb);
      host.style.setProperty('--chat-agent-grid-active', '1');
      host.style.setProperty('--chat-agent-grid-x', Math.round(x) + 'px');
      host.style.setProperty('--chat-agent-grid-y', Math.round(y) + 'px');
      this._agentTrailLastAt = now;
      return true;
    },
    anchorAgentTrailToFreshInit(host, hostRect, now, pad, w, h) {
      if (!host || typeof host.querySelector !== 'function') return false;
      if (!this.showFreshArchetypeTiles || !this.freshInitRevealMenu) return false;
      // Never override active thinking positioning during init.
      var activeThinking = host.querySelector('.message.thinking .message-bubble.message-bubble-thinking');
      if (activeThinking && activeThinking.offsetParent !== null) return false;
      var panel = host.querySelector('.chat-init-panel');
      if (!panel || panel.offsetParent === null) return false;
      var rect = hostRect && Number.isFinite(Number(hostRect.width || 0)) ? hostRect : host.getBoundingClientRect();
      var panelRect = panel.getBoundingClientRect();
      if (!(Number(panelRect.width || 0) > 0 && Number(panelRect.height || 0) > 0)) return false;
      if (panelRect.bottom < rect.top || panelRect.top > rect.bottom || panelRect.right < rect.left || panelRect.left > rect.right) return false;
      // During agent initialization, pin the orb to the initial agent chat panel.
      // Keep it 1rem outside the panel's bottom-left corner.
      var anchor = {
        x: (panelRect.left - rect.left) - 16,
        y: (panelRect.bottom - rect.top) + 16,
      };
      var x = Math.max(pad + 1, Math.min(w - (pad + 1), Number(anchor.x || 0)));
      var y = Math.max(pad + 1, Math.min(h - (pad + 1), Number(anchor.y || 0)));
      var orb = this.ensureAgentTrailOrb(host, x, y);
      this.setAgentTrailBlinkState(true, orb);
      host.style.setProperty('--chat-agent-grid-active', '1');
      host.style.setProperty('--chat-agent-grid-x', Math.round(x) + 'px');
      host.style.setProperty('--chat-agent-grid-y', Math.round(y) + 'px');
      this._agentTrailState = { x: x, y: y, vx: 0, vy: 0, dir: 0, target: 0, turnAt: now + 1000 };
      this._agentTrailSeeded = false;
      this._agentTrailLastAt = now;
      return true;
    },

    get filteredModelPicker() {
      var runtimeEngineId = String(this.selectedAgentRuntimeEngineId || 'infring_native').trim() || 'infring_native';
      var rows = typeof this.modelRowsForActiveRuntimeEngine === 'function'
        ? this.modelRowsForActiveRuntimeEngine(this.modelPickerList, runtimeEngineId)
        : this.modelPickerList;
      if (!this.modelPickerFilter) return rows.slice(0, 15);
      var f = this.modelPickerFilter;
      return rows.filter(function(m) {
        return m.id.toLowerCase().indexOf(f) !== -1 || (m.display_name || '').toLowerCase().indexOf(f) !== -1 || m.provider.toLowerCase().indexOf(f) !== -1;
      }).slice(0, 15);
    },
    pickModel(modelId) {
      this.showModelPicker = false;
      this.inputText = '/model ' + modelId;
      this.sendMessage();
    },

    loadModelCatalogSafely: function(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var preferCached = opts.prefer_cached !== false;
      var suppressErrors = opts.suppress_errors === true;
      var self = this;
      return InfringAPI.get('/api/shell-socket/models').then(function(data) {
        var models = self.sanitizeModelCatalogRows((data && data.models) || []);
        self._modelCache = models;
        self._modelCacheTime = Date.now();
        self.modelPickerList = models;
        return models;
      }).catch(function(error) {
        var fallback = preferCached ? self.sanitizeModelCatalogRows(self._modelCache || []) : [];
        if (fallback.length) {
          self._modelCache = fallback;
          self.modelPickerList = fallback;
          return fallback;
        }
        if (typeof self.loadProviderModelCatalogSafely === 'function') {
          return self.loadProviderModelCatalogSafely({
            merge_existing: true
          }).then(function(providerModels) {
            if (providerModels.length) return providerModels;
            if (suppressErrors) return [];
            throw error;
          });
        }
        if (suppressErrors) return [];
        throw error;
      });
    },

    describeModelDiscoveryResult: function(resp, catalogRows) {
      var provider = String((resp && resp.provider) || '').trim();
      var inputKind = String((resp && resp.input_kind) || '').trim().toLowerCase();
      var discoveredCount = Number((resp && resp.model_count) || ((resp && resp.models && resp.models.length) || 0));
      if (!Number.isFinite(discoveredCount) || discoveredCount < 0) discoveredCount = 0;
      var availableRows = Array.isArray(catalogRows) ? catalogRows : [];
      var availableCount = this.availableModelRowsCount ? this.availableModelRowsCount(availableRows) : availableRows.length;
      var prefix = '';
      if (inputKind === 'local_path') {
        prefix = provider
          ? ('Indexed local path for `' + provider + '`')
          : 'Indexed local path';
      } else {
        prefix = provider
          ? ('Added provider `' + provider + '`')
          : 'Saved model discovery input';
      }
      prefix += ' (' + discoveredCount + ' discovered';
      if (availableCount > 0) {
        prefix += ', ' + availableCount + ' available now';
      }
      prefix += ').';
      return prefix;
    },

    toggleModelSwitcher() {
      if (this.showModelSwitcher) { this.showModelSwitcher = false; return; }
      var self = this;
      var now = Date.now();
      if (typeof this.closeComposerMenus === 'function') this.closeComposerMenus({ model: true });
      else {
        this.showAttachMenu = false;
        this.closeGitTreeMenu();
      }
      this.modelApiKeyStatus = '';
      var cached = self.sanitizeModelCatalogRows(self._modelCache || []);
      if (cached.length) {
        self._modelCache = cached;
        self.modelPickerList = cached;
      }
      this.modelSwitcherFilter = '';
      this.modelSwitcherProviderFilter = '';
      this.modelSwitcherIdx = 0;
      this.showModelSwitcher = true;
      if (typeof self.loadRuntimeEnginesSafely === 'function') {
        self.loadRuntimeEnginesSafely({ force: true }).then(function() {
          self._modelSwitcherViewCache = null;
        }).catch(function() {
          self._modelSwitcherViewCache = null;
        });
      }
      this.$nextTick(function() {
        var el = document.getElementById('model-switcher-search');
        if (el) el.focus();
      });

      if (!cached.length && typeof self.loadProviderModelCatalogSafely === 'function') {
        self.loadProviderModelCatalogSafely({
          merge_existing: true
        }).catch(function() { return []; });
      }

      var cacheFresh = Array.isArray(this._modelCache) && (now - this._modelCacheTime) < 300000;
      var cachedAvailable = self.availableModelRowsCount ? self.availableModelRowsCount(cached) : 0;
      var shouldRefresh = !cacheFresh || cached.length < 8 || cachedAvailable < 4;
      if (!shouldRefresh) return;
      self.refreshModelCatalogAndGuidance({ discover: true, guidance: true }).catch(function(e) {
        return self.loadModelCatalogSafely({
          prefer_cached: true,
          suppress_errors: true
        }).then(function(models) {
          if (!models.length && (!self.modelPickerList || !self.modelPickerList.length)) {
            var active = self.resolveActiveSwitcherModel([]);
            self.modelPickerList = active ? [active] : [];
          }
          self.modelApiKeyStatus = models.length
            ? 'Unable to refresh model list (showing cached entries)'
            : 'Unable to refresh model list right now';
          InfringToast.error('Failed to refresh models: ' + e.message);
        });
      });
    },

    fallbackRuntimeEngineRows: function() {
      return [
        { engine_id: 'infring_native', display_name: 'InfRing Native', status: 'available', selectable: true, engine_kind: 'native_orchestration', supports_next_turn_steering: true, steering_mode: 'next_turn' },
        { engine_id: 'codex_cli', display_name: 'Codex', status: 'not_downloaded', selectable: false, download_available: true, display_when_missing: 'download_icon', supports_next_turn_steering: true, steering_mode: 'next_turn' },
        { engine_id: 'claude_code', display_name: 'Claude Code', status: 'not_downloaded', selectable: false, download_available: true, display_when_missing: 'download_icon', supports_next_turn_steering: true, steering_mode: 'next_turn' },
        { engine_id: 'openhands', display_name: 'OpenHands', status: 'not_downloaded', selectable: false, download_available: true, display_when_missing: 'download_icon', supports_next_turn_steering: true, steering_mode: 'next_turn' }
      ];
    },

    normalizeRuntimeEngineRows: function(payload) {
      var rows = payload && Array.isArray(payload.engines) ? payload.engines : [];
      var out = [];
      var canonicalDisplayNames = {
        infring_native: 'InfRing Native',
        codex_cli: 'Codex',
        claude_code: 'Claude Code',
        grok_code: 'Grok Code',
        openclaw: 'OpenClaw',
        hermes_agent: 'Hermes Agent'
      };
      for (var i = 0; i < rows.length; i += 1) {
        var row = rows[i] && typeof rows[i] === 'object' ? rows[i] : {};
        var id = String(row.engine_id || '').trim();
        if (!id) continue;
        var displayName = String(row.display_name || id).trim() || id;
        out.push({
          engine_id: id,
          canonical_engine_id: id,
          display_name: canonicalDisplayNames[id] || displayName,
          display_label: canonicalDisplayNames[id] || displayName,
          engine_kind: String(row.engine_kind || '').trim(),
          transport_kind: String(row.transport_kind || '').trim(),
          status: String(row.status || 'unknown').trim(),
          selectable: row.selectable !== false && ['not_downloaded', 'not_configured', 'planned_adapter'].indexOf(String(row.status || '').trim()) < 0,
          capabilities: Array.isArray(row.capabilities) ? row.capabilities : [],
          supports_live_steering: row.supports_live_steering === true,
          supports_next_turn_steering: row.supports_next_turn_steering === true,
          steering_mode: String(row.steering_mode || '').trim(),
          steering_transport: String(row.steering_transport || '').trim(),
          download_available: row.download_available === true,
          install_action_available: row.install_action_available === true || row.command_line_install_available === true,
          command_line_install_available: row.command_line_install_available === true,
          install_permission_state: String(row.install_permission_state || '').trim(),
          provider_readiness: String(row.provider_readiness || '').trim(),
          error_code: String(row.error_code || '').trim(),
          reason: String(row.reason || '').trim(),
          download_action_ref: String(row.download_action_ref || '').trim(),
          preferred_install_method: String(row.preferred_install_method || '').trim(),
          command_line_hint: String(row.command_line_hint || '').trim(),
          browser_fallback_url: String(row.browser_fallback_url || '').trim(),
          display_when_missing: String(row.display_when_missing || '').trim(),
          version_preview: String(row.version_preview || '').trim(),
          available_models: row.available_models && typeof row.available_models === 'object' ? row.available_models : null,
          model_menu: row.model_menu && typeof row.model_menu === 'object' ? row.model_menu : null
        });
      }
      return out.length ? out : this.fallbackRuntimeEngineRows();
    },

    loadRuntimeEnginesSafely: function(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var self = this;
      if (typeof this.ensureAgentRuntimePendingApprovalSyncLoop === 'function') this.ensureAgentRuntimePendingApprovalSyncLoop();
      if (!opts.force && Array.isArray(this.runtimeEngineRows) && this.runtimeEngineRows.length && (Date.now() - Number(this.runtimeEngineCacheTime || 0)) < 120000) {
        return Promise.resolve(this.runtimeEngineRows);
      }
      this.runtimeEngineLoading = true;
      this.runtimeEngineError = '';
      return InfringAPI.get('/api/shell-socket/agent-runtime/engines').then(function(payload) {
        var rows = self.normalizeRuntimeEngineRows(payload);
        self.runtimeEngineRows = rows;
        self.runtimeEngineCacheTime = Date.now();
        var restoredLocalSelection = typeof self.restoreAgentRuntimeEngineSelection === 'function' ? self.restoreAgentRuntimeEngineSelection() : false;
        var serverSelected = String(payload && (payload.active_engine_id || payload.selected_default_engine_id) || '').trim();
        if (restoredLocalSelection) {
          var restoredEngineId = String(self.selectedAgentRuntimeEngineId || 'infring_native').trim() || 'infring_native';
          if (restoredEngineId && restoredEngineId !== serverSelected) {
            InfringAPI.post('/api/shell-socket/agent-runtime/selection', { engine_id: restoredEngineId }).catch(function() {});
          }
        } else if (serverSelected) {
          self.selectedAgentRuntimeEngineId = serverSelected;
        }
        self._modelSwitcherViewCache = null;
        if (self.showModelSwitcher && typeof self.sanitizeModelCatalogRows === 'function') {
          var refreshedModels = self.sanitizeModelCatalogRows(self._modelCache || self.modelPickerList || []);
          self._modelCache = refreshedModels.slice();
          self.modelPickerList = refreshedModels.slice();
        }
        return rows;
      }).catch(function(e) {
        self.runtimeEngineError = e && e.message ? String(e.message) : 'runtime_engines_unavailable';
        self.runtimeEngineRows = self.runtimeEngineRows && self.runtimeEngineRows.length ? self.runtimeEngineRows : self.fallbackRuntimeEngineRows();
        if (typeof self.restoreAgentRuntimeEngineSelection === 'function') self.restoreAgentRuntimeEngineSelection();
        return self.runtimeEngineRows;
      }).finally(function() {
        self.runtimeEngineLoading = false;
      });
    },

    toggleRuntimeSwitcher: function() {
      if (this.showRuntimeSwitcher) { this.showRuntimeSwitcher = false; return; }
      if (typeof this.closeComposerMenus === 'function') this.closeComposerMenus({ runtime: true });
      else {
        this.showAttachMenu = false;
        this.showModelSwitcher = false;
        this.closeGitTreeMenu();
      }
      this.showRuntimeSwitcher = true;
      this.loadRuntimeEnginesSafely({ force: true }).catch(function() { return []; });
    },

    activeRuntimeEngineRow: function() {
      if (typeof this.restoreAgentRuntimeEngineSelection === 'function') this.restoreAgentRuntimeEngineSelection();
      var id = String(this.selectedAgentRuntimeEngineId || 'infring_native').trim();
      var rows = Array.isArray(this.runtimeEngineRows) && this.runtimeEngineRows.length ? this.runtimeEngineRows : this.fallbackRuntimeEngineRows();
      for (var i = 0; i < rows.length; i += 1) {
        if (String(rows[i] && rows[i].engine_id || '') === id) return rows[i];
      }
      return rows[0] || null;
    },

    restoreAgentRuntimeEngineSelection: function() {
      if (this._agentRuntimeEngineSelectionRestored === true) return this._agentRuntimeEngineSelectionSource === 'local_storage';
      this._agentRuntimeEngineSelectionRestored = true;
      var key = String(this.agentRuntimeEngineStorageKey || 'infring-selected-agent-runtime-engine-v1');
      var saved = '';
      try {
        if (typeof window !== 'undefined' && window.localStorage) saved = String(window.localStorage.getItem(key) || '').trim();
      } catch (_e) {}
      if (!saved) return false;
      this.selectedAgentRuntimeEngineId = saved;
      this._agentRuntimeEngineSelectionSource = 'local_storage';
      return true;
    },

    persistAgentRuntimeEngineSelection: function(engineId) {
      var id = String(engineId || 'infring_native').trim() || 'infring_native';
      this.selectedAgentRuntimeEngineId = id;
      try { window.localStorage.setItem(this.agentRuntimeEngineStorageKey, id); } catch (_e) {}
      InfringAPI.post('/api/shell-socket/agent-runtime/selection', { engine_id: id }).catch(function() {});
      return id;
    },

    runtimeEngineDisplayNameForId: function(engineId, fallbackRow) {
      var id = String(engineId || '').trim() || 'infring_native';
      if (fallbackRow && String(fallbackRow.engine_id || '').trim() === id) {
        return String(fallbackRow.display_name || id).trim() || id;
      }
      var rows = Array.isArray(this.runtimeEngineRows) && this.runtimeEngineRows.length ? this.runtimeEngineRows : this.fallbackRuntimeEngineRows();
      for (var i = 0; i < rows.length; i += 1) {
        if (String(rows[i] && rows[i].engine_id || '').trim() === id) {
          return String(rows[i].display_name || id).trim() || id;
        }
      }
      return id === 'infring_native' ? 'InfRing Native' : id;
    },

    addRuntimeEngineSwitchNotice: function(previousEngineId, nextEngineId, nextRow) {
      var previousId = String(previousEngineId || 'infring_native').trim() || 'infring_native';
      var nextId = String(nextEngineId || 'infring_native').trim() || 'infring_native';
      if (previousId === nextId) return;
      if (typeof this.addNoticeEvent !== 'function') return;
      var fromName = this.runtimeEngineDisplayNameForId(previousId, null);
      var toName = this.runtimeEngineDisplayNameForId(nextId, nextRow);
      this.addNoticeEvent({
        notice_label: 'Changed active engine from ' + fromName + ' to ' + toName,
        notice_type: 'info',
        ts: Date.now()
      });
    },

    runtimeEngineMenuLabel: function() {
      var row = this.activeRuntimeEngineRow();
      return String((row && row.display_name) || 'InfRing Native').trim();
    },

    activeWorkspaceMenuLabel: function() {
      var row = this.activeWorkspaceProjection && typeof this.activeWorkspaceProjection === 'object' ? this.activeWorkspaceProjection : null;
      var label = String(row && (row.display_label || row.basename) || '').trim();
      if (label) return label.length > 22 ? label.substring(0, 19) + '...' : label;
      var cwd = String(this.terminalPromptPath || this.terminalCwd || '/workspace').trim();
      var parts = cwd.split(/[\\/]+/).filter(Boolean);
      var base = parts.length ? parts[parts.length - 1] : cwd;
      return base ? '.../' + base : '/workspace';
    },

    activeWorkspacePath: function() {
      var row = this.activeWorkspaceProjection && typeof this.activeWorkspaceProjection === 'object' ? this.activeWorkspaceProjection : null;
      return String(row && (row.workspace_dir || row.active_workspace) || this.terminalPromptPath || this.terminalCwd || '/workspace').trim();
    },

    activeWorkspaceTurnContextProjection: function() {
      var row = this.activeWorkspaceProjection && typeof this.activeWorkspaceProjection === 'object' ? this.activeWorkspaceProjection : null;
      if (!row) return null;
      return {
        workspace_dir: String(row.workspace_dir || row.active_workspace || '').trim(),
        active_workspace: String(row.active_workspace || row.workspace_dir || '').trim(),
        display_label: String(row.display_label || '').trim(),
        git_root: String(row.git_root || '').trim(),
        git_root_label: String(row.git_root_label || '').trim(),
        permission_boundary: row.permission_boundary || null
      };
    },

    applyActiveWorkspaceProjection: function(payload) {
      var row = payload && typeof payload === 'object' ? payload : null;
      if (!row) return null;
      this.activeWorkspaceProjection = row;
      this.activeWorkspaceCacheTime = Date.now();
      this.activeWorkspaceError = '';
      if (row.workspace_dir || row.active_workspace) {
        this.terminalCwd = String(row.workspace_dir || row.active_workspace);
      }
      return row;
    },

    loadActiveWorkspaceProjection: function(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var self = this;
      if (!opts.force && this.activeWorkspaceProjection && (Date.now() - Number(this.activeWorkspaceCacheTime || 0)) < 120000) {
        return Promise.resolve(this.activeWorkspaceProjection);
      }
      this.activeWorkspaceLoading = true;
      this.activeWorkspaceError = '';
      return InfringAPI.get('/api/shell-socket/agent-runtime/workspace').then(function(payload) {
        return self.applyActiveWorkspaceProjection(payload);
      }).catch(function(e) {
        self.activeWorkspaceError = e && e.message ? String(e.message) : 'workspace_unavailable';
        return self.activeWorkspaceProjection;
      }).finally(function() {
        self.activeWorkspaceLoading = false;
      });
    },

    chooseActiveWorkspace: function() {
      var self = this;
      this.activeWorkspacePicking = true;
      this.activeWorkspaceError = '';
      if (typeof this.closeComposerMenus === 'function') this.closeComposerMenus();
      InfringToast.info('Choose InfRing working directory...');
      return InfringAPI.post('/api/shell-socket/agent-runtime/workspace/pick', {
        scope: 'global_default'
      }).then(function(payload) {
        if (!payload || payload.ok === false) {
          var reason = payload && (payload.reason || payload.error) ? String(payload.reason || payload.error) : 'workspace_picker_cancelled';
          self.activeWorkspaceError = reason;
          if (payload && payload.error !== 'workspace_picker_cancelled') InfringToast.error('Workspace picker: ' + reason);
          return payload;
        }
        self.applyActiveWorkspaceProjection(payload);
        InfringToast.success('Working directory: ' + self.activeWorkspaceMenuLabel());
        return payload;
      }).catch(function(e) {
        var reason = e && e.message ? String(e.message) : 'workspace_picker_failed';
        self.activeWorkspaceError = reason;
        InfringToast.error('Workspace picker: ' + reason);
        return null;
      }).finally(function() {
        self.activeWorkspacePicking = false;
      });
    },

    isRuntimeEngineActive: function(row) {
      return String(row && row.engine_id || '') === String(this.selectedAgentRuntimeEngineId || 'infring_native');
    },

    runtimeEngineStatusLabel: function(row) {
      var status = String(row && row.status || '').trim();
      if (status === 'available') return 'Available';
      if (status === 'adapter_ready') return 'Adapter ready';
      if (status === 'not_downloaded') return 'Not installed';
      if (status === 'not_connected') return 'Not connected';
      if (status === 'planned_adapter') return 'Planned';
      return status || 'Unknown';
    },

    runtimeEngineMeta: function(row) {
      var parts = [];
      var kind = String(row && row.engine_kind || '').replace(/_/g, ' ').trim();
      var status = this.runtimeEngineStatusLabel(row);
      var steering = String(row && row.steering_mode || '').trim();
      var reason = String(row && row.reason || '').trim();
      if (kind) parts.push(kind);
      if (status) parts.push(status);
      if (reason) parts.push(reason);
      if (steering === 'live') parts.push('live steer');
      else if (steering === 'next_turn') parts.push('next-turn steer');
      return parts.join(' · ');
    },

    isRuntimeEngineSelectionBlocked: function(row) {
      if (!row || row.selectable !== false) return false;
      var meta = '';
      try { meta = this.runtimeEngineMeta(row) || ''; } catch (_e) { meta = ''; }
      var statusText = [
        row.status || '',
        row.connection_status || '',
        row.availability || '',
        row.provider_readiness || '',
        row.reason || '',
        meta
      ].join(' ').toLowerCase();
      if (
        statusText.indexOf('provider_blocked') >= 0 ||
        statusText.indexOf('runtime_requirement_missing') >= 0 ||
        statusText.indexOf('auth_required') >= 0 ||
        statusText.indexOf('unavailable') >= 0 ||
        statusText.indexOf('blocked') >= 0 ||
        statusText.indexOf('not_downloaded') >= 0 ||
        statusText.indexOf('not connected') >= 0 ||
        statusText.indexOf('not_connected') >= 0 ||
        statusText.indexOf('planned') >= 0 ||
        statusText.indexOf('timeout') >= 0
      ) return true;
      if (
        row.available === true ||
        row.connected === true ||
        row.installed === true ||
        row.discovered === true ||
        statusText.indexOf('available') >= 0 ||
        statusText.indexOf('adapter_ready') >= 0 ||
        statusText.indexOf('connected') >= 0
      ) return false;
      return true;
    },

    runtimeEngineActionIcon: function(row) {
      if (this.isRuntimeEngineActive(row)) return '✓';
      var status = String(row && row.status || '').trim();
      if (row && row.download_available && ['available', 'adapter_ready'].indexOf(status) < 0) return '⇩';
      if (row && String(row.status || '') === 'not_connected') return '!';
      return '';
    },

    selectRuntimeEngine: function(row) {
      var engineId = String(row && row.engine_id || '').trim();
      if (!engineId) return;
      if (this.isRuntimeEngineSelectionBlocked(row)) {
        if (row.install_action_available || row.command_line_install_available || row.preferred_install_method === 'command_line') {
          this.installRuntimeEngine(row);
          return;
        }
        this.openRuntimeEngineInstall(row);
        return;
      }
      var previousEngineId = String(this.selectedAgentRuntimeEngineId || 'infring_native').trim() || 'infring_native';
      this.persistAgentRuntimeEngineSelection(engineId);
      this.showRuntimeSwitcher = false;
      this.addRuntimeEngineSwitchNotice(previousEngineId, engineId, row);
      InfringToast.success('Agent runtime: ' + String((row && row.display_name) || engineId));
    },

    installRuntimeEngine: function(row) {
      var engineId = String(row && row.engine_id || '').trim();
      if (!engineId) return Promise.resolve(null);
      var previousEngineId = String(this.selectedAgentRuntimeEngineId || 'infring_native').trim() || 'infring_native';
      var self = this;
      var label = String((row && row.display_name) || engineId);
      this.showRuntimeSwitcher = false;
      InfringToast.info('Installing agent runtime: ' + label);
      return InfringAPI.post('/api/shell-socket/agent-runtime/engines/' + encodeURIComponent(engineId) + '/install', {
        engine_id: engineId
      }).then(function(payload) {
        var status = String(payload && payload.status || '').trim();
        if (payload && payload.ok && (status === 'installed_available' || status === 'already_available')) {
          self.persistAgentRuntimeEngineSelection(engineId);
          self.addRuntimeEngineSwitchNotice(previousEngineId, engineId, row);
          InfringToast.success('Agent runtime ready: ' + label);
          return self.loadRuntimeEnginesSafely({ force: true });
        }
        if (status === 'permission_required') {
          InfringToast.error('Install permission is required for ' + label);
          return payload;
        }
        if (payload && payload.command_line_hint) InfringToast.info(String(payload.command_line_hint));
        if (payload && payload.browser_fallback_url && status !== 'command_line_installer_unavailable_for_platform') {
          try { window.open(String(payload.browser_fallback_url), '_blank', 'noopener,noreferrer'); } catch (_e) {}
        } else {
          InfringToast.error('Runtime install did not complete: ' + (status || 'unknown'));
        }
        return payload;
      }).catch(function(e) {
        InfringToast.error(e && e.message ? String(e.message) : 'runtime install failed');
        return null;
      });
    },

    openRuntimeEngineInstall: function(row) {
      var hint = String(row && row.command_line_hint || '').trim();
      var preferred = String(row && row.preferred_install_method || '').trim();
      var url = String(row && row.browser_fallback_url || '').trim();
      if (hint) {
        InfringToast.info(hint);
        if (preferred === 'command_line') return;
      }
      if (url && preferred !== 'command_line') {
        try { window.open(url, '_blank', 'noopener,noreferrer'); } catch (_e) {}
      } else if (!hint) {
        InfringToast.info('Runtime install path is not configured yet.');
      }
    },

    discoverModelsFromApiKey: function() {
      var self = this;
      var entry = String(this.modelApiKeyInput || '').trim();
      if (!entry) {
        InfringToast.error('Enter an API key or local model path first');
        return;
      }
      this.modelApiKeySaving = true;
      this.modelApiKeyStatus = 'Detecting...';
      InfringAPI.post('/api/shell-socket/models/discover', {
        input: entry,
        api_key: entry
      }).then(function(resp) {
        var provider = String((resp && resp.provider) || '').trim();
        var inputKind = String((resp && resp.input_kind) || '').trim().toLowerCase();
        var count = Number((resp && resp.model_count) || ((resp && resp.models && resp.models.length) || 0));
        self.modelApiKeyInput = '';
        if (inputKind === 'local_path') {
          self.modelApiKeyStatus = provider
            ? ('Indexed local path to ' + provider + ' (' + count + ' models)')
            : ('Indexed local path (' + count + ' models)');
        } else {
          self.modelApiKeyStatus = provider ? ('Added ' + provider + ' (' + count + ' models)') : 'API key saved';
        }
        self._modelCache = null;
        self._modelCacheTime = 0;
        return self.loadModelCatalogSafely({
          prefer_cached: false,
          suppress_errors: false
        }).then(function(models) {
          self.modelApiKeyStatus = self.describeModelDiscoveryResult(resp, models);
          return models;
        });
      }).then(function(models) {
        if (self.availableModelRowsCount(models) === 0) {
          self.injectNoModelsGuidance('discover_key');
        }
      }).catch(function(e) {
        self.modelApiKeyStatus = '';
        InfringToast.error('Model discovery failed: ' + (e && e.message ? e.message : e));
      }).finally(function() {
        self.modelApiKeySaving = false;
      });
    },

    resolveModelContextWindowForSwitch: function(targetModelRef) {
      var modelId = '';
      var explicitWindow = 0;
      if (targetModelRef && typeof targetModelRef === 'object') {
        modelId = String(
          targetModelRef.id || targetModelRef.model || targetModelRef.model_name || ''
        ).trim();
        explicitWindow = Number(
          targetModelRef.context_window || targetModelRef.context_window_tokens || 0
        );
      } else {
        modelId = String(targetModelRef || '').trim();
      }
      if (Number.isFinite(explicitWindow) && explicitWindow > 0) {
        return Math.round(explicitWindow);
      }
      var map = this._contextWindowByModel || {};
      var candidates = [];
      if (modelId) {

        candidates.push(modelId);
        if (modelId.indexOf('/') >= 0) {
          candidates.push(modelId.split('/').slice(-1)[0]);
        }
      }
      var bestFromMap = 0;
      var bestInferred = 0;
      var needsFloor = false;
      for (var i = 0; i < candidates.length; i++) {
        var candidate = String(candidates[i] || '').trim();
        if (!candidate) continue;
        if (typeof this.contextWindowNeedsFloor === 'function' && this.contextWindowNeedsFloor(candidate)) {
          needsFloor = true;
        }
        var fromMap = Number(map[candidate] || 0);
        if (Number.isFinite(fromMap) && fromMap > bestFromMap) {
          bestFromMap = Math.round(fromMap);
        }
        var inferred = this.inferContextWindowFromModelId(
          candidate.indexOf('/') >= 0 ? candidate.split('/').slice(-1)[0] : candidate
        );
        if (Number.isFinite(inferred) && inferred > bestInferred) {
          bestInferred = Math.round(inferred);
        }
      }
      if (needsFloor && bestInferred > 0) {
        return Math.max(bestFromMap, bestInferred);
      }
      if (bestFromMap > 0) return bestFromMap;
      if (bestInferred > 0) return bestInferred;
      return 0;
    },

    ensureContextBudgetForModelSwitch: function(agentId, targetModelRef, options) {
      var self = this;
      var opts = options && typeof options === 'object' ? options : {};
      var id = String(agentId || '').trim();
      if (!id) return Promise.resolve({ compacted: false });
      var targetWindow = self.resolveModelContextWindowForSwitch(targetModelRef);
      var usedTokens = Number(self.contextApproxTokens || 0);
      if (
        !Number.isFinite(targetWindow) ||
        targetWindow <= 0 ||
        !Number.isFinite(usedTokens) ||
        usedTokens <= targetWindow
      ) {
        return Promise.resolve({
          compacted: false,
          target_context_window: targetWindow,
          before_tokens: Math.max(0, Math.round(usedTokens || 0)),
          after_tokens: Math.max(0, Math.round(usedTokens || 0))
        });
      }
      var targetRatio = Number(opts.target_ratio);
      if (!Number.isFinite(targetRatio) || targetRatio <= 0 || targetRatio >= 1) {
        targetRatio = 0.8;
      }
      var targetTokens = Math.max(1, Math.floor(targetWindow * targetRatio));
      InfringToast.info('Switching to a model with smaller context may degrade performance.');
      return InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(id) + '/compact-session', {
        target_context_window: targetWindow,
        target_ratio: targetRatio,
        min_recent_messages: 12,
        max_messages: 200
      }).then(function(resp) {
        var beforeTokens = usedTokens;
        var afterTokens = Math.min(usedTokens, targetTokens);
        if (Number.isFinite(afterTokens) && afterTokens >= 0) {
          self.contextApproxTokens = Math.max(0, Math.round(afterTokens));
        }
        if (Number.isFinite(targetWindow) && targetWindow > 0) {
          self.contextWindow = Math.round(targetWindow);
        }
        self.refreshContextPressure();
        self.addNoticeEvent({
          notice_label:
            'Context compacted from ' +
            self.formatTokenK(beforeTokens) +
            ' to ' +
            self.formatTokenK(afterTokens) +
            ' tokens (target ' +
            self.formatTokenK(targetTokens) +
            ')',
          notice_type: 'info',
          ts: Date.now()
        });
        return {
          compacted: !!(resp && resp.accepted !== false),
          receipt_ref: resp && resp.receipt_ref,
          before_tokens: beforeTokens,
          after_tokens: afterTokens
        };
      });
    },

    switchAgentModelWithGuards: function(targetModelRef, options) {
      var self = this;
      var opts = options && typeof options === 'object' ? options : {};
      var reboundAgent = self.ensureValidCurrentAgent({ clear_when_missing: true });
      var agentId = String(opts.agent_id || (reboundAgent && reboundAgent.id) || '').trim();
      if (!agentId) return Promise.reject(new Error('No agent selected'));
      var requestedModel = '';
      if (targetModelRef && typeof targetModelRef === 'object') {
        requestedModel = String(
          targetModelRef.id || targetModelRef.model || targetModelRef.model_name || ''
        ).trim();
      } else {
        requestedModel = String(targetModelRef || '').trim();
      }
      if (!requestedModel) return Promise.reject(new Error('Model is required'));
      var previousModel = String(
        opts.previous_model != null
          ? opts.previous_model
          : ((self.currentAgent && (self.currentAgent.runtime_model || self.currentAgent.model_name)) || '')
      ).trim();
      var previousProvider = String(
        opts.previous_provider != null
          ? opts.previous_provider
          : ((self.currentAgent && self.currentAgent.model_provider) || '')
      ).trim();
      return self
        .ensureContextBudgetForModelSwitch(agentId, targetModelRef, opts)
        .catch(function(error) {
          InfringToast.error(
            'Context compaction failed before model switch: ' +
              (error && error.message ? error.message : error)
          );
          return null;
        })
        .then(function() {
          return InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(agentId) + '/model', {
            model: requestedModel
          });
        })
        .catch(function(error) {
          var message = String(error && error.message ? error.message : error || '');
          var lower = message.toLowerCase();
          var allowRetry = !opts._rebind_retry && (lower.indexOf('agent_not_found') >= 0 || lower.indexOf('agent not found') >= 0);
          if (!allowRetry) throw error;
          return self.rebindCurrentAgentAuthoritative({
            preferred_id: agentId,
            clear_when_missing: true
          }).then(function(rebound) {
            var reboundId = String(rebound && rebound.id ? rebound.id : '').trim();
            if (!reboundId || reboundId === agentId) throw error;
            self.addNoticeEvent({
              notice_label: 'Active agent reference expired. Rebound to ' + String(rebound.name || rebound.id || reboundId),
              notice_type: 'warn',
              ts: Date.now()
            });
            var retryOptions = {};
            var keys = Object.keys(opts);
            for (var k = 0; k < keys.length; k++) retryOptions[keys[k]] = opts[keys[k]];
            retryOptions.agent_id = reboundId;
            retryOptions._rebind_retry = true;
            return self.switchAgentModelWithGuards(targetModelRef, retryOptions);
          });
        })
        .then(function(resp) {
          if (self.currentAgent && String(self.currentAgent.id || '') === agentId) {
            self.currentAgent.model_name = (resp && resp.model) || requestedModel;
            self.currentAgent.model_provider =
              (resp && resp.provider) || self.currentAgent.model_provider || '';
            self.currentAgent.runtime_model =
              (resp && resp.runtime_model) ||
              self.currentAgent.runtime_model ||
              self.currentAgent.model_name;
            var resolvedContextWindow = Number(
              resp && resp.context_window != null ? resp.context_window : 0
            );
            if (Number.isFinite(resolvedContextWindow) && resolvedContextWindow > 0) {
              self.currentAgent.context_window = Math.round(resolvedContextWindow);
              self.contextWindow = Math.round(resolvedContextWindow);
              self.refreshContextPressure();
            }
            self.recordModelUsageTimestamp(requestedModel || '');
            self.recordModelUsageTimestamp(self.currentAgent.model_name || '');
            self.recordModelUsageTimestamp(self.currentAgent.runtime_model || '');
            if (self.currentAgent.model_provider && self.currentAgent.model_name) {
              self.recordModelUsageTimestamp(
                self.currentAgent.model_provider + '/' + self.currentAgent.model_name
              );
            }
            if (self.currentAgent.model_provider && self.currentAgent.runtime_model) {
              self.recordModelUsageTimestamp(
                self.currentAgent.model_provider + '/' + self.currentAgent.runtime_model
              );
            }
            self.addModelSwitchNotice(
              previousModel,
              previousProvider,
              self.currentAgent.model_name,
              self.currentAgent.model_provider
            );
          }
          return resp || {};
        });
    },

    switchModel(model) {
      var activeAgent = this.ensureValidCurrentAgent({ clear_when_missing: true });
      if (!activeAgent) return;
      if (model && model.available === false) {
        InfringToast.error('This model is not ready yet. Configure its provider/API key first.');
        return;
      }
      if (model.id === this.currentAgent.model_name) {
        this.recordModelUsageTimestamp(model.id || '');
        this.showModelSwitcher = false;
        return;
      }
      var self = this;
      this.modelSwitching = true;
      self.switchAgentModelWithGuards(model, {
        agent_id: activeAgent.id
      }).then(function() {
        InfringToast.success('Switched to ' + (model.display_name || model.id));
        self.showModelSwitcher = false;
      }).catch(function(e) {
        InfringToast.error('Switch failed: ' + e.message);
      }).finally(function() {
        self.modelSwitching = false;
      });
    },

    ensureFailoverModelCache: function() {
      var now = Date.now();
      if (this._modelCache && (now - Number(this._modelCacheTime || 0)) < 180000) {
        return Promise.resolve(this._modelCache);
      }
      var self = this;
      return InfringAPI.get('/api/shell-socket/models')
        .then(function(data) {
          var models = self.sanitizeModelCatalogRows(Array.isArray(data && data.models) ? data.models : []);
          var available = models.filter(function(m) { return !!(m && m.available); });
          self._modelCache = models;
          self._modelCacheTime = Date.now();
          self.modelPickerList = models;
          return available;
        })
        .catch(function() {
          return Array.isArray(self._modelCache) ? self._modelCache : [];
        });
    },

    normalizeFailoverCandidateId: function(entry) {
      if (!entry) return '';
      if (typeof entry === 'string') return String(entry || '').trim();
      if (typeof entry !== 'object') return '';
      var model = String(entry.id || entry.model || entry.model_name || '').trim();
      var provider = String(entry.provider || entry.model_provider || '').trim();
      if (!model) return '';
      if (provider && model.indexOf('/') < 0) return provider + '/' + model;
      return model;
    },

    collectModelIdVariants: function(values) {
      var set = {};
      var add = function(value) {
        var raw = String(value || '').trim();
        if (!raw) return;
        var lower = raw.toLowerCase();
        set[lower] = true;
        if (raw.indexOf('/') >= 0) {
          var tail = String(raw.split('/').slice(-1)[0] || '').toLowerCase();
          if (tail) set[tail] = true;
        }
      };
      if (Array.isArray(values)) {
        for (var i = 0; i < values.length; i++) add(values[i]);
      } else {
        add(values);
      }
      return set;
    },

    // Backward-compat shim for legacy callers during naming migration.
    modelIdVariantSet: function(values) {
      return this.collectModelIdVariants(values);
    },

    extractRecoverableBackendFailure: function(text) {
      var raw = String(text || '').trim();
      if (!raw) return null;
      var lower = raw.toLowerCase();
      if (
        lower === 'i lost the final response handoff for this turn. context is still intact, and i can continue from exactly where this left off.' ||
        lower.indexOf('completed tool steps:') === 0
      ) {
        return null;
      }
      var markers = [
        "couldn't reach a chat model backend",
        'could not reach a chat model backend',
        'hosted_model_provider_sync_failed',
        'provider-sync',
        'switch-provider',
        'lane_timeout_1500ms',
        'start ollama',
        'configure app-plane',
        'model backend unavailable',
        'no chat model backend',
        'app_plane_chat_ui',
        'did not receive a final answer',
        'lost the final response handoff'
      ];
      var matched = false;
      for (var i = 0; i < markers.length; i++) {
        if (lower.indexOf(markers[i]) >= 0) {
          matched = true;
          break;
        }
      }
      if (!matched) return null;
      var summary = raw.replace(/\s+/g, ' ').trim();
      if (summary.length > 220) summary = summary.slice(0, 217) + '...';
      return { raw: raw, summary: summary };
    },

    attemptAutomaticFailoverRecovery: async function(source, rawFailure, options) {
      var payload = this._inflightPayload;
      if (payload && typeof payload === 'object') {
        payload.failover_skipped = true;
        payload.failover_skip_reason = 'automatic_model_failover_disabled';
        payload.failover_source = String(source || 'runtime');
      }
      void rawFailure;
      void options;
      return false;
    },

    // Fetch dynamic slash commands from server
    fetchCommands: function() {
      var self = this;
      InfringAPI.get('/api/commands').then(function(data) {
        if (data.commands && data.commands.length) {
          // Build a set of known cmds to avoid duplicates
          var existing = {};
          self.slashCommands.forEach(function(c) { existing[c.cmd] = true; });
          data.commands.forEach(function(c) {
            if (!existing[c.cmd]) {
              self.slashCommands.push({ cmd: c.cmd, desc: c.desc || '', source: c.source || 'server' });
              existing[c.cmd] = true;
            }
          });
        }
      }).catch(function() { /* silent — use hardcoded list */ });
    },

    normalizeAgentRuntimeSlashCommandRow: function(row, group) {
      if (!row || typeof row !== 'object') return null;
      var displayCommand = String(row.display_command || row.command || '').trim();
      if (!displayCommand) return null;
      var title = String(row.title || row.intent_id || displayCommand).trim();
      var description = String(row.description || '').trim();
      var operationalState = String(row.operational_state || 'stubbed_or_unwired').trim();
      var operationalLabel = String(row.operational_label || operationalState).trim();
      return {
        row_kind: 'command',
        cmd: displayCommand,
        desc: description || title,
        title: title,
        source: 'agent_runtime_command_catalog',
        command_id: String(row.command_id || ''),
        intent_id: String(row.intent_id || ''),
        engine_id: String(row.engine_id || ''),
        group_id: String(row.group_id || (group && group.group_id) || ''),
        group_title: String((group && group.title) || ''),
        native_command: String(row.native_command || ''),
        native_command_kind: String(row.native_command_kind || ''),
        execution_kind: String(row.execution_kind || ''),
        safety_class: String(row.safety_class || ''),
        operational_state: operationalState,
        operational_label: operationalLabel,
        operational_detail: String(row.operational_detail || ''),
        connected: row.connected !== false,
        fully_operational: row.fully_operational === true,
        selectable: true,
        action_route: String(row.action_route || '/api/shell-socket/agent-runtime/commands/execute'),
        default_passthrough_allowed: false,
        chat_memory_eligible: false
      };
    },

    normalizeAgentRuntimeSlashCommandProjection: function(payload) {
      var groups = Array.isArray(payload && payload.groups) ? payload.groups : [];
      var rows = [];
      for (var i = 0; i < groups.length; i += 1) {
        var group = groups[i] && typeof groups[i] === 'object' ? groups[i] : {};
        var commands = Array.isArray(group.commands) ? group.commands : [];
        for (var j = 0; j < commands.length; j += 1) {
          var normalized = this.normalizeAgentRuntimeSlashCommandRow(commands[j], group);
          if (normalized) rows.push(normalized);
        }
      }
      return rows;
    },

    fetchAgentRuntimeSlashCommands: function(force) {
      var self = this;
      var engineId = String(this.selectedAgentRuntimeEngineId || 'infring_native').trim() || 'infring_native';
      var now = Date.now();
      if (
        force !== true &&
        this.agentRuntimeSlashCommandProjection &&
        this._agentRuntimeSlashCommandProjectionEngineId === engineId &&
        now - Number(this._agentRuntimeSlashCommandProjectionAt || 0) < 10000
      ) {
        return Promise.resolve(this.agentRuntimeSlashCommandProjection);
      }
      if (this._agentRuntimeSlashCommandProjectionLoading && force !== true) {
        return Promise.resolve(this.agentRuntimeSlashCommandProjection);
      }
      this._agentRuntimeSlashCommandProjectionLoading = true;
      return InfringAPI.get('/api/shell-socket/agent-runtime/commands?engine_id=' + encodeURIComponent(engineId)).then(function(payload) {
        self.agentRuntimeSlashCommandProjection = payload || null;
        self.agentRuntimeSlashCommandRows = self.normalizeAgentRuntimeSlashCommandProjection(payload);
        self._agentRuntimeSlashCommandProjectionAt = Date.now();
        self._agentRuntimeSlashCommandProjectionEngineId = engineId;
        self._agentRuntimeSlashCommandProjectionLoading = false;
        return payload;
      }).catch(function() {
        self._agentRuntimeSlashCommandProjectionLoading = false;
        return null;
      });
    },

    executeAgentRuntimeSlashCommand: async function(row, cmdArgs) {
      var command = row && typeof row === 'object' ? row : {};
      var engineId = String(command.engine_id || this.selectedAgentRuntimeEngineId || 'infring_native').trim() || 'infring_native';
      var payload = {
        engine_id: engineId,
        intent_id: String(command.intent_id || ''),
        display_command: String(command.cmd || command.display_command || ''),
        args: String(cmdArgs || ''),
        client_surface: 'dashboard',
        source: 'slash_command_menu'
      };
      try {
        var res = await InfringAPI.post('/api/shell-socket/agent-runtime/commands/execute', payload);
        var text = String(res && (res.display_text || res.message || res.status || '') || '').trim();
        if (text) {
          this.messages.push({
            id: ++msgId,
            role: 'system',
            is_notice: true,
            notice_type: res && res.status === 'manual_action_required' ? 'warning' : 'info',
            notice_label: String(command.title || command.cmd || 'Runtime command'),
            text: text,
            meta: '',
            tools: [],
            system_origin: 'slash:agent-runtime-command',
            ts: Date.now()
          });
          this.scrollToBottom();
          this.scheduleConversationPersist();
        }
        return res;
      } catch (error) {
        var message = String(error && error.message ? error.message : error || 'runtime_command_failed').trim();
        if (typeof this.emitCommandFailureNotice === 'function') {
          this.emitCommandFailureNotice(command.cmd || payload.display_command, message, ['/help']);
        }
        return null;
      }
    },

    // Keep thinking indicators alive while work is still in-flight.
    // Only hard-timeout once no pending activity remains or the request is
    // genuinely stale far beyond expected runtime.
    _resetTypingTimeout: function() {
      var self = this;
      if (self._typingTimeout) clearTimeout(self._typingTimeout);
      self._typingTimeout = setTimeout(function() {
        var hasPending = typeof self.hasLivePendingResponse === 'function'
          ? self.hasLivePendingResponse()
          : false;
        var hardStale = typeof self.pendingResponseExceededHardTimeout === 'function'
          ? self.pendingResponseExceededHardTimeout()
          : false;
        if (hasPending && !hardStale) {
          self._resetTypingTimeout();
          return;
        }
        // Transport timeout: do not fabricate assistant content.
        self._clearStreamingTypewriters();
        typeof self.clearTransientThinkingRows === 'function' ? self.clearTransientThinkingRows({ force: true }) : (self.messages = self.messages.filter(function(m) { return !m.thinking && !m.streaming; }));
        // Do not inject transport-authored text into the chat transcript.
        self.sending = false;
        self._responseStartedAt = 0;
        self.tokenCount = 0;
        self._inflightPayload = null;
        self._clearPendingWsRequest();
        self.setAgentLiveActivity(self.currentAgent && self.currentAgent.id ? self.currentAgent.id : '', 'idle', { optimistic: true, source: 'shell_display_hint' });
        self.scheduleConversationPersist();
      }, 120000);
    },

    hasLivePendingResponse: function() {
      var rows = Array.isArray(this.messages) ? this.messages : [];
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i];
        if (!row) continue;
        if (row.thinking || row.streaming || (row.terminal && row.thinking)) {
          return true;
        }
      }
      return !!(this._pendingWsRequest && this._pendingWsRequest.agent_id);
    },

    pendingResponseExceededHardTimeout: function() {
      var now = Date.now();
      var startedAt = Number(this._responseStartedAt || 0);
      if ((!Number.isFinite(startedAt) || startedAt <= 0) && this._pendingWsRequest) {
        startedAt = Number(this._pendingWsRequest.started_at || 0);
      }
      if (!Number.isFinite(startedAt) || startedAt <= 0) {
        var rows = Array.isArray(this.messages) ? this.messages : [];
        for (var i = rows.length - 1; i >= 0; i--) {
          var row = rows[i];
          if (!row) continue;
          if (!(row.thinking || row.streaming || (row.terminal && row.thinking))) continue;
          var rowStartedAt = Number(row._stream_started_at || row._stream_updated_at || row.ts || 0);
          if (Number.isFinite(rowStartedAt) && rowStartedAt > 0) {
            startedAt = rowStartedAt;
            break;
          }
        }
      }
      if (!Number.isFinite(startedAt) || startedAt <= 0) return false;
      return Math.max(0, now - startedAt) >= 900000;
    },

    _clearTypingTimeout: function() {
      if (this._typingTimeout) {
        clearTimeout(this._typingTimeout);
        this._typingTimeout = null;
      }
    },

    _resolveLiveMessageRef: function(message) {
      if (!message || typeof message !== 'object') return null;
      var msgId = message.id;
      if (!Array.isArray(this.messages) || !this.messages.length) return message;
      if (msgId == null) return message;
      for (var i = this.messages.length - 1; i >= 0; i--) {
        var row = this.messages[i];
        if (!row || typeof row !== 'object') continue;
        if (String(row.id) === String(msgId)) return row;
      }
      return message;
    },

    _clearMessageTypewriter: function(message, options) {
      var liveMessage = this._resolveLiveMessageRef(message);
      if (!liveMessage || typeof liveMessage !== 'object') return;
      var opts = options && typeof options === 'object' ? options : {};
      var preserveTypingVisual = opts.preserveTypingVisual === true;
      var preservePartialText = opts.preservePartialText === true;
      var clearFinalText = opts.clearFinalText !== false;
      if (liveMessage._typewriterTimer) {
        clearTimeout(liveMessage._typewriterTimer);
        liveMessage._typewriterTimer = null;
      }
      if (message && message !== liveMessage && message._typewriterTimer) {
        clearTimeout(message._typewriterTimer);
        message._typewriterTimer = null;
      }
      liveMessage._typewriterRunning = false;
      if (message && message !== liveMessage) message._typewriterRunning = false;
      if (!preserveTypingVisual) {
        if (
          !preservePartialText &&
          liveMessage._typingVisual &&
          typeof liveMessage._typewriterFinalText === 'string'
        ) {
          liveMessage.text = String(liveMessage._typewriterFinalText || '');
        }
        liveMessage._typingVisual = false;
        if (message && message !== liveMessage) message._typingVisual = false;
      }
      if (clearFinalText && !preserveTypingVisual) {
        liveMessage._typewriterFinalText = '';
        if (message && message !== liveMessage) message._typewriterFinalText = '';
      }
      if (!preserveTypingVisual) {
        liveMessage._typingVisualHtml = '';
        liveMessage._typingVisualHtmlStable = '';
        liveMessage._typingVisualHtmlActive = '';
        liveMessage._typingVisualHtmlActiveStable = '';
        if (message && message !== liveMessage) message._typingVisualHtml = '';
        if (message && message !== liveMessage) message._typingVisualHtmlStable = '';
        if (message && message !== liveMessage) message._typingVisualHtmlActive = '';
        if (message && message !== liveMessage) message._typingVisualHtmlActiveStable = '';
      }
    },

    _clearStreamingTypewriters: function() {
      var rows = Array.isArray(this.messages) ? this.messages : [];
      for (var i = 0; i < rows.length; i++) {
        this._clearMessageTypewriter(rows[i], {
          preserveTypingVisual: false,
          preservePartialText: false,
        });
      }
    },

    _hasActiveTypewriterVisual: function() {
      var rows = Array.isArray(this.messages) ? this.messages : [];
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i];
        if (!row || typeof row !== 'object') continue;
        if (row._typingVisual || row._typewriterRunning || row._typewriterTimer) return true;
      }
      return false;
    },

    _queueStreamTypingRender: function(message, nextText) {
      if (!message || typeof message !== 'object') return;
      var targetText = String(nextText || '');
      message._streamTargetText = targetText;
      message._typewriterFinalText = '';
      if (message._typewriterRunning) return;
      var self = this;
      message._typewriterRunning = true;

      var step = function() {
        if (!message || !message.streaming) {
          self._clearMessageTypewriter(message);
          return;
        }
        var target = String(message._streamTargetText || '');
        var current = String(message.text || '');
        if (target === current) {
          self._clearMessageTypewriter(message);
          return;
        }
        // If sanitization trims or rewrites content, snap to the newest safe value.
        if (target.length < current.length || target.indexOf(current) !== 0) {
          message.text = target;
          self._clearMessageTypewriter(message);
          self.scrollToBottom();
          return;
        }
        var remaining = target.length - current.length;
        var take = Math.max(1, Math.min(8, Math.ceil(remaining / 4)));
        message.text = target.slice(0, current.length + take);
        self.scrollToBottom();
        if (message.text.length < target.length) {
          message._typewriterTimer = setTimeout(step, 1);
          return;
        }
        self._clearMessageTypewriter(message);
      };

      step();
    },

    _resolveTypingDelayForToken: function(baseDelay, emittedToken, fullText, emittedIndex) {
      var base = Number(baseDelay || 1);
      if (!Number.isFinite(base) || base < 0) base = 1;
      var token = String(emittedToken || '');
      if (!/[.!?]/.test(token)) return base;
      var source = String(fullText || '');
      var idx = Number(emittedIndex || 0);
      if (!Number.isFinite(idx) || idx < 0) idx = 0;
      var next = source.charAt(idx + 1);
      if (!next || /\s|["')\]]/.test(next)) {
        return base * 2;
      }
      return base;
    },

    // Backward-compat shim for legacy callers during naming migration.
    _typingDelayForToken: function(baseDelay, emittedToken, fullText, emittedIndex) {
      return this._resolveTypingDelayForToken(baseDelay, emittedToken, fullText, emittedIndex);
    },

    _resolveTypingWordCadenceMs: function(fallbackDelayMs) {
      var cadenceMs = Number(this.typingWordCadenceMs);
      if (!Number.isFinite(cadenceMs) || cadenceMs <= 0) cadenceMs = Number(fallbackDelayMs);
      if (!Number.isFinite(cadenceMs) || cadenceMs <= 0) cadenceMs = 1;
      cadenceMs = Math.max(1, Math.min(2000, cadenceMs));
      return cadenceMs;
    },

    _escapeTypingVisualTokenHtml: function(token) {
      var raw = String(token == null ? '' : token);
      var escaped = '';
      if (typeof this.escapeHtml === 'function') escaped = this.escapeHtml(raw);
      else escaped = raw
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#39;');
      escaped = escaped.replace(/\t/g, '    ');
      escaped = escaped.replace(/\n/g, '<br>');
      return escaped;
    },

    _queueFinalWordTypingRender: function(message, finalText, wordDelayMs) {
      var baseMessage = this._resolveLiveMessageRef(message);
      if (!baseMessage || typeof baseMessage !== 'object') return;
      var targetText = String(finalText || '');
      this._clearMessageTypewriter(baseMessage, {
        preserveTypingVisual: false,
        preservePartialText: false,
      });
      baseMessage._typingVisual = false;
      if (!targetText.trim()) {
        baseMessage._typewriterFinalText = '';
        baseMessage._typingVisualHtml = '';
        baseMessage._typingVisualHtmlStable = '';
        baseMessage._typingVisualHtmlActive = '';
        baseMessage._typingVisualHtmlActiveStable = '';
        baseMessage.text = targetText;
        if (typeof this.scheduleConversationPersist === 'function') this.scheduleConversationPersist();
        return;
      }
      var segments = [];
      var segmentPattern = /\S+\s*/g;
      var segmentMatch;
      var typingRenderText = typeof normalizeChatMarkdownListBreaks === 'function'
        ? normalizeChatMarkdownListBreaks(targetText)
        : targetText;
      while ((segmentMatch = segmentPattern.exec(typingRenderText)) !== null) {
        segments.push({
          text: String(segmentMatch[0] || ''),
          index: Number(segmentMatch.index || 0)
        });
      }
      var leadingWhitespaceMatch = typingRenderText.match(/^\s+/);
      if (leadingWhitespaceMatch && segments.length) {
        var leadingWhitespace = String(leadingWhitespaceMatch[0] || '');
        segments[0].text = leadingWhitespace + String(segments[0].text || '');
        segments[0].index = 0;
      }
      if (!Array.isArray(segments) || !segments.length) {
        baseMessage._typewriterFinalText = '';
        baseMessage._typingVisualHtml = '';
        baseMessage._typingVisualHtmlStable = '';
        baseMessage._typingVisualHtmlActive = '';
        baseMessage._typingVisualHtmlActiveStable = '';
        baseMessage.text = targetText;
        baseMessage._typingVisual = false;
        if (typeof this.scheduleConversationPersist === 'function') this.scheduleConversationPersist();
        return;
      }
      baseMessage._typewriterFinalText = targetText;
      baseMessage.text = '';
      baseMessage._typingVisualHtml = '';
      baseMessage._typingVisualHtmlStable = '';
      baseMessage._typingVisualHtmlActive = '';
      baseMessage._typingVisualHtmlActiveStable = '';
      baseMessage._typingVisual = true;
      baseMessage._typewriterRunning = true;
      var self = this;
      var index = 0;
      var markdownState = { bold: false, italic: false };
      var cadenceMs = typeof this._resolveTypingWordCadenceMs === 'function'
        ? this._resolveTypingWordCadenceMs(wordDelayMs)
        : 1;
      var maxTokensPerTick = 24;
      var nextTickAt = Date.now();
      var keepPinnedToBottom = function() {
        try {
          if (typeof self.scrollToBottomImmediate === 'function') {
            self.scrollToBottomImmediate({ force: false });
          } else {
            self.scrollToBottom();
          }
        } catch (_) {}
      };
      var step = function() {
        var liveMessage = self._resolveLiveMessageRef(baseMessage);
        if (!liveMessage) {
          self._clearMessageTypewriter(baseMessage);
          return;
        }
        if (!liveMessage._typewriterRunning) {
          self._clearMessageTypewriter(liveMessage, {
            preserveTypingVisual: false,
            preservePartialText: false,
          });
          if (typeof self.scheduleConversationPersist === 'function') self.scheduleConversationPersist();
          return;
        }
        if (index >= segments.length) {
          liveMessage.text = targetText;
          liveMessage._typingVisual = false;
          liveMessage._typingVisualHtmlStable = '';
          liveMessage._typingVisualHtmlActive = '';
          liveMessage._typingVisualHtmlActiveStable = '';
          liveMessage._typingVisualHtml = '';
          if (baseMessage !== liveMessage) {
            baseMessage._typingVisual = false;
            baseMessage._typingVisualHtmlStable = '';
            baseMessage._typingVisualHtmlActive = '';
            baseMessage._typingVisualHtmlActiveStable = '';
            baseMessage._typingVisualHtml = '';
          }
          liveMessage._typewriterRunning = false;
          liveMessage._typewriterTimer = null;
          if (baseMessage !== liveMessage) {
            baseMessage._typewriterRunning = false;
            baseMessage._typewriterTimer = null;
          }
          if (typeof self.scheduleConversationPersist === 'function') self.scheduleConversationPersist();
          return;
        }
        var now = Date.now();
        if (now < nextTickAt) {
          var waitMs = Math.max(1, Math.min(2000, Math.round(nextTickAt - now)));
          var waitTimer = setTimeout(step, waitMs);
          liveMessage._typewriterTimer = waitTimer;
          if (baseMessage !== liveMessage) baseMessage._typewriterTimer = waitTimer;
          return;
        }
        var emitted = 0;
        var stableHtml = String(liveMessage._typingVisualHtmlStable || '') + String(liveMessage._typingVisualHtmlActiveStable || '');
        var activeHtml = '';
        var activeStable = '';
        var hasFutureMarkdownDelimiter = function(sourceText, delimiter, absoluteIndex) {
          var source = String(sourceText || '');
          var marker = String(delimiter || '');
          if (!marker) return false;
          var startAt = Math.max(0, Number(absoluteIndex || 0) + marker.length);
          return source.indexOf(marker, startAt) >= 0;
        };
        while (index < segments.length && emitted < maxTokensPerTick) {
          now = Date.now();
          if (now < nextTickAt) break;
          cadenceMs = typeof self._resolveTypingWordCadenceMs === 'function'
            ? self._resolveTypingWordCadenceMs(wordDelayMs)
            : cadenceMs;
          var segment = segments[index] || { text: '', index: 0 };
          var token = String(segment.text || '');
          index += 1;
          emitted += 1;
          liveMessage.text = String(liveMessage.text || '') + token;
          var tokenEndIndex = Number(segment.index || 0) + Math.max(0, token.length - 1);
          var nextDelay = typeof self._resolveTypingDelayForToken === 'function'
            ? self._resolveTypingDelayForToken(cadenceMs, token, typingRenderText, tokenEndIndex)
            : cadenceMs;
          if (!Number.isFinite(nextDelay) || nextDelay <= 0) nextDelay = cadenceMs;
          nextDelay = Math.max(1, Math.min(2000, nextDelay));
          nextTickAt += nextDelay;
          var tokenHtmlStable = '';
          var tokenHtmlActive = '';
          var tokenState = { bold: !!markdownState.bold, italic: !!markdownState.italic };
          var appendChunk = function(chunk, isActiveChunk) {
            if (!chunk) return;
            var chunkHtml = self._escapeTypingVisualTokenHtml(chunk);
            if (tokenState.bold) chunkHtml = '<strong>' + chunkHtml + '</strong>';
            if (tokenState.italic) chunkHtml = '<em>' + chunkHtml + '</em>';
            if (isActiveChunk) {
              tokenHtmlActive +=
                '<span class="typing-word-active" style="--typing-word-fade-ms:' +
                '1000ms">' +
                chunkHtml +
                '</span>';
              tokenHtmlStable += chunkHtml;
              return;
            }
            tokenHtmlStable += chunkHtml;
            tokenHtmlActive += chunkHtml;
          };
          var cursor = 0;
          while (cursor < token.length) {
            var absoluteCursor = Number(segment.index || 0) + cursor;
            if (token.charAt(cursor) === '\\' && (cursor + 1) < token.length && token.charAt(cursor + 1) === '*') {
              appendChunk('*', true);
              cursor += 2;
              continue;
            }
            if ((cursor + 1) < token.length && token.charAt(cursor) === '*' && token.charAt(cursor + 1) === '*') {
              if (tokenState.bold || hasFutureMarkdownDelimiter(typingRenderText, '**', absoluteCursor)) {
                tokenState.bold = !tokenState.bold;
              } else {
                appendChunk('**', true);
              }
              cursor += 2;
              continue;
            }
            if (token.charAt(cursor) === '*') {
              if (tokenState.italic || hasFutureMarkdownDelimiter(typingRenderText, '*', absoluteCursor)) {
                tokenState.italic = !tokenState.italic;
              } else {
                appendChunk('*', true);
              }
              cursor += 1;
              continue;
            }
            var start = cursor;
            while (cursor < token.length) {
              if (token.charAt(cursor) === '\\' && (cursor + 1) < token.length && token.charAt(cursor + 1) === '*') break;
              if ((cursor + 1) < token.length && token.charAt(cursor) === '*' && token.charAt(cursor + 1) === '*') break;
              if (token.charAt(cursor) === '*') break;
              cursor += 1;
            }
            var chunk = token.slice(start, cursor);
            if (!chunk) continue;
            if (!/\S/.test(chunk)) {
              appendChunk(chunk, false);
              continue;
            }
            var leadMatch = chunk.match(/^\s+/);
            var trailMatch = chunk.match(/\s+$/);
            var lead = leadMatch ? String(leadMatch[0] || '') : '';
            var trail = trailMatch ? String(trailMatch[0] || '') : '';
            var coreStart = lead.length;
            var coreEnd = chunk.length - trail.length;
            if (coreEnd < coreStart) {
              coreEnd = coreStart;
              trail = '';
            }
            var core = chunk.slice(coreStart, coreEnd);
            if (lead) appendChunk(lead, false);
            if (core) appendChunk(core, true);
            if (trail) appendChunk(trail, false);
          }
          markdownState.bold = !!tokenState.bold;
          markdownState.italic = !!tokenState.italic;
          if (activeStable) stableHtml += activeStable;
          activeStable = tokenHtmlStable;
          activeHtml = tokenHtmlActive;
        }
        liveMessage._typingVisualHtmlStable = stableHtml;
        liveMessage._typingVisualHtmlActive = activeHtml;
        liveMessage._typingVisualHtmlActiveStable = activeStable;
        liveMessage._typingVisualHtml = stableHtml + activeHtml;
        if (baseMessage !== liveMessage) {
          baseMessage._typingVisualHtmlStable = liveMessage._typingVisualHtmlStable;
          baseMessage._typingVisualHtmlActive = liveMessage._typingVisualHtmlActive;
          baseMessage._typingVisualHtmlActiveStable = liveMessage._typingVisualHtmlActiveStable;
          baseMessage._typingVisualHtml = liveMessage._typingVisualHtml;
        }
        if (emitted > 0) keepPinnedToBottom();
        if (index < segments.length) {
          var timerDelay = Math.max(1, Math.min(2000, Math.round(nextTickAt - Date.now())));
          var timerId = setTimeout(step, timerDelay);
          liveMessage._typewriterTimer = timerId;
          if (baseMessage !== liveMessage) baseMessage._typewriterTimer = timerId;
          return;
        }
        liveMessage.text = targetText;
        liveMessage._typingVisual = false;
        liveMessage._typingVisualHtmlStable = '';
        liveMessage._typingVisualHtmlActive = '';
        liveMessage._typingVisualHtmlActiveStable = '';
        liveMessage._typingVisualHtml = '';
        if (baseMessage !== liveMessage) {
          baseMessage._typingVisual = false;
          baseMessage._typingVisualHtmlStable = '';
          baseMessage._typingVisualHtmlActive = '';
          baseMessage._typingVisualHtmlActiveStable = '';
          baseMessage._typingVisualHtml = '';
        }
        liveMessage._typewriterRunning = false;
        liveMessage._typewriterTimer = null;
        if (baseMessage !== liveMessage) {
          baseMessage._typewriterRunning = false;
          baseMessage._typewriterTimer = null;
        }
        keepPinnedToBottom();
        if (typeof self.scheduleConversationPersist === 'function') self.scheduleConversationPersist();
      };
      step();
    },
    _reconcileSendingState: function() {
      if (!this.sending) return false;
      var pending = this._pendingWsRequest && this._pendingWsRequest.agent_id ? this._pendingWsRequest : null;
      var hasPendingWs = !!pending;
      var inflight = this._inflightPayload && typeof this._inflightPayload === 'object'
        ? this._inflightPayload
        : null;
      var pendingStatusText = pending && String(pending.status_text || '').trim()
        ? String(pending.status_text || '').trim()
        : '';
      var rows = Array.isArray(this.messages) ? this.messages : [];
      var hasVisiblePending = false;
      var now = Date.now();
      var currentAgentId = String(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '').trim();
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i];
        if (!row) continue;
        if (row.thinking || row.streaming || (row.terminal && row.thinking)) {
          if (
            (!String(row.thinking_status || '').trim() ||
              (
                typeof this.isThinkingPlaceholderText === 'function' &&
                this.isThinkingPlaceholderText(row.thinking_status)
              )) &&
            (!pending || !pending.agent_id || !String(row.agent_id || '').trim() || String(row.agent_id || '').trim() === String(pending.agent_id || '').trim())
          ) {
            row.thinking_status = pendingStatusText;
          }
          hasVisiblePending = true;
        }
      }
      if (!hasVisiblePending && hasPendingWs && typeof this.ensureLiveThinkingRow === 'function') {
        var keepRow = this.ensureLiveThinkingRow({
          agent_id: String(pending.agent_id || ''),
          agent_name: this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '',
          status_text: pendingStatusText
        });
        if (keepRow) {
          keepRow.thinking = true;
          keepRow.streaming = true;
          if (!Number.isFinite(Number(keepRow._stream_started_at))) keepRow._stream_started_at = now;
          keepRow._stream_updated_at = now;
          if (!String(keepRow.text || '').trim()) keepRow.text = '';
          if (
            !String(keepRow.thinking_status || '').trim() ||
            (
              typeof this.isThinkingPlaceholderText === 'function' &&
              this.isThinkingPlaceholderText(keepRow.thinking_status)
            )
          ) {
            keepRow.thinking_status = pendingStatusText;
          }
          hasVisiblePending = true;
        }
      }
      if (pending) {
        var pendingAgentId = String(pending.agent_id || '');
        var pendingAgeMs = Math.max(0, now - Number(pending.started_at || now));
        if (currentAgentId && pendingAgentId && pendingAgentId !== currentAgentId) {
          hasPendingWs = false;
          if (!this._pendingWsRecovering) {
            this._recoverPendingWsRequest('cross_agent_pending');
          }
        } else if (pendingAgeMs >= 12000) {
          if (!this._pendingWsRecovering) {
            this._recoverPendingWsRequest('stale_pending');
          }
          if (pendingAgeMs >= 900000) {
            this._clearPendingWsRequest();
            hasPendingWs = false;
          }
        }
      }
      if (!hasVisiblePending && !hasPendingWs && inflight) {
        var inflightAgentId = String(inflight.agent_id || currentAgentId || '').trim();
        var inflightStartedAt = Number(
          this._responseStartedAt ||
          (pending && pending.started_at) ||
          inflight.created_at ||
          0
        );
        var hasRecentInflightReply = false;
        if (
          Number.isFinite(inflightStartedAt) &&
          inflightStartedAt > 0 &&
          typeof this._recentAgentReplyObserved === 'function'
        ) {
          hasRecentInflightReply = this._recentAgentReplyObserved(rows, inflightStartedAt);
        }
        if (inflightAgentId && !hasRecentInflightReply) {
          this._setPendingWsRequest(
            inflightAgentId,
            String(inflight.final_text || ''),
            {
              started_at: Number.isFinite(inflightStartedAt) && inflightStartedAt > 0
                ? inflightStartedAt
                : Date.now(),
              status_text: pendingStatusText
            }
          );
          pending = this._pendingWsRequest;
          hasPendingWs = !!pending;
          if (!Number.isFinite(Number(this._responseStartedAt)) || Number(this._responseStartedAt) <= 0) {
            this._responseStartedAt = Number(
              (pending && pending.started_at) ||
              inflight.created_at ||
              Date.now()
            );
          }
          if (typeof this.ensureLiveThinkingRow === 'function') {
            var restoredPending = this.ensureLiveThinkingRow({
              agent_id: inflightAgentId,
              agent_name: this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '',
              status_text: pendingStatusText
            });
            if (restoredPending) {
              restoredPending.thinking = true;
              restoredPending.streaming = true;
              restoredPending._stream_updated_at = now;
              if (!Number.isFinite(Number(restoredPending._stream_started_at))) {
                restoredPending._stream_started_at = now;
              }
              hasVisiblePending = true;
            }
          }
        }
      }
      if (hasVisiblePending || hasPendingWs) {
        var keepBusyAgentId = '';
        if (pending && pending.agent_id) keepBusyAgentId = String(pending.agent_id || '').trim();
        if (!keepBusyAgentId) keepBusyAgentId = String(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '').trim();
        if (keepBusyAgentId) this.setAgentLiveActivity(keepBusyAgentId, 'working', { optimistic: true, source: 'shell_display_hint' });
      }
      if (hasVisiblePending || hasPendingWs) return false;
      this.sending = false;
      this._responseStartedAt = 0;
      this.tokenCount = 0;
      this._clearTypingTimeout();
      this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '', 'idle', { optimistic: true, source: 'shell_display_hint' });
      return true;
    },
    _setPendingWsRequest: function(agentId, messageText, options) {
      var id = String(agentId || '').trim();
      if (!id) return;
      var opts = options && typeof options === 'object' ? options : {};
      var startedAt = Number(opts.started_at || 0);
      if (!Number.isFinite(startedAt) || startedAt <= 0) startedAt = Date.now();
      var statusText = String(opts.status_text || '').trim();
      this._pendingWsRequest = {
        agent_id: id,
        message_text: String(messageText || '').trim(),
        status_text: statusText,
        started_at: startedAt,
      };
      this._pendingWsRecovering = false;
    },

    _setPendingWsStatusText: function(agentId, statusText) {
      if (!this._pendingWsRequest) return;
      var pendingAgentId = String(this._pendingWsRequest.agent_id || '').trim();
      var targetAgentId = String(agentId || '').trim();
      if (targetAgentId && pendingAgentId && pendingAgentId !== targetAgentId) return;
      var nextStatus = String(statusText || '').trim();
      if (!nextStatus) return;
      if (typeof this.normalizeThinkingStatusCandidate === 'function') {
        var normalized = this.normalizeThinkingStatusCandidate(nextStatus);
        if (normalized) nextStatus = normalized;
      }
      if (!nextStatus) return;
      this._pendingWsRequest.status_text = nextStatus;
    },

    _clearPendingWsRequest: function(agentId) {
      if (!this._pendingWsRequest) return;
      if (!agentId) {
        this._pendingWsRequest = null;
        this._pendingWsRecovering = false;
        return;
      }
      var current = String(this._pendingWsRequest.agent_id || '').trim();
      if (current && current === String(agentId)) {
        this._pendingWsRequest = null;
        this._pendingWsRecovering = false;
      }
    },

    _markAgentPreviewUnread: function(agentId, unread) {
      var id = String(agentId || '').trim();
      if (!id) return;
      try {
        var store = Alpine.store('app');
        if (!store) return;
        if (typeof store.markAgentPreviewUnread === 'function') {
          store.markAgentPreviewUnread(id, unread !== false);
        } else if (store.agentChatPreviews && store.agentChatPreviews[id]) {
          store.agentChatPreviews[id].unread_response = unread !== false;
        }
      } catch(_) {}
    },

    _pendingRequestReplyObserved: function(normalizedMessages, pendingRequest, startedAt) {
      var rows = Array.isArray(normalizedMessages) ? normalizedMessages : [];
      if (!rows.length) return false;

      var pendingText = this.sanitizeToolText(String(
        pendingRequest && pendingRequest.message_text ? pendingRequest.message_text : ''
      )).trim();
      var started = Number(startedAt || (pendingRequest && pendingRequest.started_at) || 0);
      var skewToleranceMs = 15000;
      var lastMatchingUserIndex = -1;

      if (pendingText) {
        for (var i = 0; i < rows.length; i++) {
          var userMsg = rows[i] || {};
          var userRole = String(userMsg.role || '').toLowerCase();
          if (userRole !== 'user') continue;
          var userText = this.sanitizeToolText(String(userMsg.text || '')).trim();
          if (userText && userText === pendingText) {
            lastMatchingUserIndex = i;
          }
        }
      }

      for (var j = 0; j < rows.length; j++) {
        var msg = rows[j] || {};
        var role = String(msg.role || '').toLowerCase();
        var text = String(msg.text || '').trim();
        var hasToolPayload = Array.isArray(msg.tools) && msg.tools.length > 0;
        var isAgentRole = role === 'agent' || role === 'assistant';
        if (!isAgentRole || (!text && !hasToolPayload)) continue;

        var ts = Number(msg.ts || 0);
        if (started > 0 && ts > 0 && (ts + skewToleranceMs) >= started) {
          return true;
        }
        if (lastMatchingUserIndex >= 0 && j > lastMatchingUserIndex) {
          return true;
        }
      }
      return false;
    },

    _recentAgentReplyObserved: function(rows, startedAt) {
      var list = Array.isArray(rows) ? rows : [];
      if (!list.length) return false;
      var started = Number(startedAt || 0);
      var skewToleranceMs = 20000;
      for (var i = list.length - 1; i >= 0; i -= 1) {
        var msg = list[i] || {};
        if (msg.thinking || msg.streaming) continue;
        var role = String(msg.role || '').toLowerCase();
        if (role !== 'agent' && role !== 'assistant') continue;
        var text = String(msg.text || '').trim();
        var hasToolPayload = Array.isArray(msg.tools) && msg.tools.length > 0;
        if (!text && !hasToolPayload) continue;
        if (msg._auto_fallback) continue;
        if (text && /^thinking\.\.\.$/i.test(text)) continue;
        var ts = Number(msg.ts || 0);
        if (started > 0 && ts > 0 && (ts + skewToleranceMs) < started) continue;
        return true;
      }
      return false;
    },

    _recoverPendingWsRequest: async function(reason) {
      if (this._pendingWsRecovering) return;
      var pending = this._pendingWsRequest;
      if (!pending || !pending.agent_id) return;
      this._pendingWsRecovering = true;
      var recoverySeq = Number(this._pendingWsRecoverySeq || 0) + 1;
      this._pendingWsRecoverySeq = recoverySeq;
      var agentId = String(pending.agent_id);
      var startedAt = Number(pending.started_at || Date.now());
      var recoverStartedAt = Date.now();
      var maxRecoverMs = 15000;
      var resolved = false;
      var self = this;
      var recoveryStillCurrent = function() {
        if (Number(self._pendingWsRecoverySeq || 0) !== recoverySeq) return false;
        if (!self._pendingWsRequest || String(self._pendingWsRequest.agent_id || '') !== agentId) return false;
        return true;
      };
      for (var attempt = 0; attempt < 30; attempt++) {
        if (!recoveryStillCurrent()) {
          break;
        }
        if ((Date.now() - recoverStartedAt) > maxRecoverMs) {
          break;
        }
        try {
          var sessionData = await InfringAPI.get('/api/agents/' + encodeURIComponent(agentId) + '/session');
          if (!recoveryStillCurrent()) break;
          var normalized = this.normalizeSessionMessages(sessionData);
          var hasFreshAgentReply =
            this._pendingRequestReplyObserved(normalized, pending, startedAt) ||
            this._recentAgentReplyObserved(normalized, startedAt);
          if (!hasFreshAgentReply) {
            await new Promise(function(resolve) { setTimeout(resolve, 650); });
            continue;
          }
          if (!this.conversationCache) this.conversationCache = {};
          var normalizedPreview = this.sanitizeConversationForCache(normalized || []);
          this.conversationCache[String(agentId)] = {
            saved_at: Date.now(),
            token_count: Number(this.contextApproxTokens || 0),
            messages: normalizedPreview,
          };
          try {
            var appStore = Alpine.store('app');
            if (appStore && typeof appStore.saveAgentChatPreview === 'function') {
              appStore.saveAgentChatPreview(agentId, normalizedPreview);
            }
          } catch(_) {}
          var isActive = !!(this.currentAgent && String(this.currentAgent.id || '') === agentId);
          if (isActive) {
            this.messages = this.mergeModelNoticesForAgent(agentId, JSON.parse(JSON.stringify(normalized || [])));
            this.scrollToBottom();
          } else {
            this._markAgentPreviewUnread(agentId, true);
          }
          this.persistConversationCache();
          resolved = true;
          break;
        } catch(_) {
          if (!recoveryStillCurrent()) break;
          await new Promise(function(resolve) { setTimeout(resolve, 500); });
        }
      }

      if (!recoveryStillCurrent()) {
        this._pendingWsRecovering = false;
        return;
      }
      var stillActiveAgent = !!(this.currentAgent && String(this.currentAgent.id || '') === agentId);
      if (!resolved && stillActiveAgent) {
        var localRows = Array.isArray(this.messages) ? this.messages : [];
        if (this._pendingRequestReplyObserved(localRows, pending, startedAt)) {
          resolved = true;
        }
        if (!resolved && this._recentAgentReplyObserved(localRows, startedAt)) {
          resolved = true;
        }
        if (!resolved && this._recentAgentReplyObserved(localRows, Math.max(0, startedAt - 120000))) {
          resolved = true;
        }
      }
      var pendingAgeMs = Math.max(0, Date.now() - Number(startedAt || Date.now()));
      if (!resolved && stillActiveAgent && pendingAgeMs < 900000) {
        this._pendingWsRecovering = false;
        return;
      }
      if (!resolved && stillActiveAgent) {
        typeof this.clearTransientThinkingRows === 'function' ? this.clearTransientThinkingRows({ force: true }) : (this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; }));
        // Do not inject transport-authored text into the chat transcript.
      }
      if (!resolved && !stillActiveAgent) {
        this._pendingWsRecovering = false;
        return;
      }
      this.setAgentLiveActivity(agentId, 'idle', { optimistic: true, source: 'shell_display_hint' });
      if (stillActiveAgent) {
        this.sending = false;
        this._responseStartedAt = 0;
        this.tokenCount = 0;
        var self = this;
        this.$nextTick(function() {
          var el = document.getElementById('msg-input'); if (el) el.focus();
          self._processQueue();
        });
      }
      this._clearPendingWsRequest(agentId);
      this._pendingWsRecovering = false;
    },

    async executeSlashCommand(cmd, cmdArgs, commandRow) {
      this.showSlashMenu = false;

// Layer ownership: client/runtime/systems/ui (dashboard static UX surface only; no runtime authority).
      this.inputText = '';
      var self = this;
      cmdArgs = cmdArgs || '';
      var selectedSlashRow = commandRow && typeof commandRow === 'object'
        ? commandRow
        : (typeof this.findSlashCommandDefinition === 'function' ? this.findSlashCommandDefinition(cmd) : null);
      if (
        selectedSlashRow &&
        selectedSlashRow.source === 'agent_runtime_command_catalog' &&
        typeof this.executeAgentRuntimeSlashCommand === 'function'
      ) {
        return this.executeAgentRuntimeSlashCommand(selectedSlashRow, cmdArgs);
      }
      switch (cmd) {
        case '/help':
          self.messages.push({
            id: ++msgId,
            role: 'system',
            is_notice: true,
            notice_type: 'info',
            text: (function(rows) {
              var commands = Array.isArray(rows) ? rows : [];
              var groups = { navigation: [], session: [], tooling: [], other: [] };
              commands.forEach(function(row) {
                var name = String(row && row.cmd ? row.cmd : '').trim();
                if (!name) return;
                var summary = '`' + name + '` — ' + String(row && row.desc ? row.desc : '').trim();
                if (/^\/(agents|new|model|apikey|status)$/i.test(name)) groups.navigation.push(summary);
                else if (/^\/(compact|stop|usage|think|context|queue)$/i.test(name)) groups.session.push(summary);
                else if (/^\/(alerts|next|memory|continuity|aliases|alias|opt|file|folder)$/i.test(name)) groups.tooling.push(summary);
                else groups.other.push(summary);
              });
              var voiceLine = (navigator && navigator.mediaDevices && typeof navigator.mediaDevices.getUserMedia === 'function')
                ? '- Voice note capture is available from the composer mic.'
                : '- Voice note capture is unavailable in this browser.';
              var sections = ['**Slash Help**'];
              if (groups.navigation.length) sections.push('**Navigation**\n' + groups.navigation.slice(0, 5).join('\n'));
              if (groups.session.length) sections.push('**Session Controls**\n' + groups.session.slice(0, 6).join('\n'));
              if (groups.tooling.length) sections.push('**Tooling & Recovery**\n' + groups.tooling.slice(0, 8).join('\n'));
              if (groups.other.length) sections.push('**More**\n' + groups.other.slice(0, 6).join('\n'));
              sections.push('**Voice**\n' + voiceLine);
              return sections.join('\n\n');
            })(self.slashCommands),
            meta: '',
            tools: [],
            system_origin: 'slash:help',
            notice_label: 'Slash Help'
          });
          self.scrollToBottom();
          break;
        case '/agents':
          location.hash = 'agents';
          break;
        case '/new':
          if (self.currentAgent) {
            InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(self.currentAgent.id) + '/fresh-session', {}).then(function() {
              self.messages = [];
              InfringToast.success('Session reset');
            }).catch(function(e) { InfringToast.error('Reset failed: ' + e.message); });
          }
          break;
        case '/compact':
          if (self.currentAgent) {
            self.messages.push({ id: ++msgId, role: 'system', is_notice: true, notice_type: 'info', notice_label: 'Compacting session...', text: 'Compacting session...', meta: '', tools: [], system_origin: 'slash:compact' });
            InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(self.currentAgent.id) + '/compact-session', {}).then(function(res) {
              var accepted = !res || res.accepted !== false;
              var label = accepted ? 'Compaction accepted' : 'Compaction rejected';
              self.messages.push({ id: ++msgId, role: 'system', is_notice: true, notice_type: accepted ? 'info' : 'warn', notice_label: label, text: label, meta: '', tools: [], system_origin: 'slash:compact' });
              self.scrollToBottom();
            }).catch(function(e) { InfringToast.error('Compaction failed: ' + e.message); });
          }
          break;
        case '/stop':
          self.stopAgent();
          break;
        case '/usage':
          if (self.currentAgent) {
            var approxTokens = self.messages.reduce(function(sum, m) { return sum + Math.round((m.text || '').length / 4); }, 0);
            self.messages.push({ id: ++msgId, role: 'system', is_notice: true, notice_type: 'info', notice_label: 'Session Usage', text: '**Session Usage**\n- Messages: ' + self.messages.length + '\n- Approx tokens: ~' + approxTokens, meta: '', tools: [], system_origin: 'slash:usage' });
            self.scrollToBottom();
          }
          break;
        case '/think':
          if (cmdArgs === 'on') {
            self.thinkingMode = 'on';
          } else if (cmdArgs === 'off') {
            self.thinkingMode = 'off';
          } else if (cmdArgs === 'stream') {
            self.thinkingMode = 'stream';
          } else {
            // Cycle: off -> on -> stream -> off
            if (self.thinkingMode === 'off') self.thinkingMode = 'on';
            else if (self.thinkingMode === 'on') self.thinkingMode = 'stream';
            else self.thinkingMode = 'off';
          }
          var modeLabel = self.thinkingMode === 'stream' ? 'enabled (streaming reasoning)' : (self.thinkingMode === 'on' ? 'enabled' : 'disabled');
          self.messages.push({ id: ++msgId, role: 'system', is_notice: true, notice_type: 'info', notice_label: 'Extended thinking ' + modeLabel, text: 'Extended thinking **' + modeLabel + '**. ' +
            (self.thinkingMode === 'stream' ? 'Reasoning tokens will appear in a collapsible panel.' :
             self.thinkingMode === 'on' ? 'The agent will show its reasoning when supported by the model.' :
             'Normal response mode.'), meta: '', tools: [], system_origin: 'slash:think' });
          self.scrollToBottom();
          break;

        case '/context':
          // Visual-only update for context ring; no chat message noise.
          if (self.currentAgent && InfringAPI.isWsConnected()) {
            InfringAPI.wsSend({ type: 'command', command: 'context', args: '', silent: true });
          } else {
            self.recomputeContextEstimate();
            self.setContextWindowFromCurrentAgent();
          }
          break;
        case '/verbose':
          if (self.currentAgent && InfringAPI.isWsConnected()) {
            InfringAPI.wsSend({ type: 'command', command: 'verbose', args: cmdArgs });
          } else {
            self.pushSystemMessage({ id: ++msgId, role: 'system', is_notice: true, notice_type: 'info', notice_label: 'Not connected. Connect to an agent first.', text: 'Not connected. Connect to an agent first.', meta: '', tools: [], system_origin: 'slash:verbose' });
          }
          break;
        case '/queue':
          if (self.currentAgent && InfringAPI.isWsConnected()) {
            InfringAPI.wsSend({ type: 'command', command: 'queue', args: '' });
          } else {
            self.pushSystemMessage({ id: ++msgId, role: 'system', is_notice: true, notice_type: 'info', notice_label: 'Not connected.', text: 'Not connected.', meta: '', tools: [], system_origin: 'slash:queue' });
          }
          break;
        case '/status':
          InfringAPI.get('/api/shell-socket/runtime-status').then(function(s) {
            var statusLabel = String((s && (s.label || s.state)) || 'unknown').trim();
            var degradedReason = String((s && s.degraded_reason) || '').trim();
            var ageSeconds = s && s.age_seconds != null ? Number(s.age_seconds) : NaN;
            var lines = ['**Runtime Status**', '- State: ' + (statusLabel || 'unknown')];
            if (degradedReason) lines.push('- Degraded: ' + degradedReason);
            if (Number.isFinite(ageSeconds)) lines.push('- Age: ' + Math.max(0, Math.round(ageSeconds)) + 's');
            self.pushSystemMessage({ id: ++msgId, role: 'system', is_notice: true, notice_type: 'info', notice_label: 'Runtime Status', text: lines.join('\n'), meta: '', tools: [], system_origin: 'slash:status' });
          }).catch(function() {});
          break;
        case '/alerts':
          await self.runSlashAlerts();
          break;
        case '/next':
          await self.runSlashNextActions();
          break;
        case '/memory':
          await self.runSlashMemoryHygiene();
          break;
        case '/continuity':
          await self.runSlashContinuity();
          break;
        case '/aliases':
          self.executeSlashAliases();
          break;
        case '/alias':
          self.executeSlashAliasCommand(cmdArgs);
          break;
        case '/opt':
          await self.runSlashOptimizeWorkers();
          break;
        case '/model':
          if (self.currentAgent) {
            if (cmdArgs) {
              var resolvedSlashModel = typeof self.resolveModelCatalogOption === 'function'
                ? self.resolveModelCatalogOption(
                  cmdArgs,
                  String((self.currentAgent && self.currentAgent.model_provider) || '').trim(),
                  typeof self.modelCatalogRows === 'function' ? self.modelCatalogRows() : []
                )
                : null;
              self.switchAgentModelWithGuards(resolvedSlashModel || { id: cmdArgs }, {
                agent_id: self.currentAgent.id
              }).catch(function(e) {
                InfringToast.error('Model switch failed: ' + e.message);
              });
            } else {
              var catalogRows = typeof self.modelCatalogRows === 'function' ? self.modelCatalogRows() : [];
              var selectedModelRef = typeof self.normalizeQualifiedModelRef === 'function'
                ? self.normalizeQualifiedModelRef(
                  String((self.currentAgent && (self.currentAgent.model_name || self.currentAgent.runtime_model)) || ''),
                  String((self.currentAgent && self.currentAgent.model_provider) || '').trim(),
                  catalogRows
                )
                : String((self.currentAgent && (self.currentAgent.model_name || self.currentAgent.runtime_model)) || '').trim();
              var runtimeModelRef = typeof self.normalizeQualifiedModelRef === 'function'
                ? self.normalizeQualifiedModelRef(
                  String((self.currentAgent && (self.currentAgent.runtime_model || self.currentAgent.model_name)) || ''),
                  String((self.currentAgent && self.currentAgent.model_provider) || '').trim(),
                  catalogRows
                )
                : String((self.currentAgent && (self.currentAgent.runtime_model || self.currentAgent.model_name)) || '').trim();
              var selectedDisplay = typeof self.formatQualifiedModelDisplay === 'function'
                ? self.formatQualifiedModelDisplay(selectedModelRef)
                : selectedModelRef;
              var runtimeDisplay = typeof self.formatQualifiedModelDisplay === 'function'
                ? self.formatQualifiedModelDisplay(runtimeModelRef)
                : runtimeModelRef;
              var availableCount = Array.isArray(catalogRows)
                ? catalogRows.filter(function(row) { return row && row.available !== false; }).length
                : 0;
              self.pushSystemMessage({
                id: ++msgId,
                role: 'system',
                is_notice: true,
                notice_type: 'info',
                notice_label: 'Current Model',
                text: '**Current Model**\n' +
                  '- Provider: `' + (self.currentAgent.model_provider || '?') + '`\n' +
                  '- Selected: `' + (selectedDisplay || selectedModelRef || '?') + '`\n' +
                  '- Runtime: `' + (runtimeDisplay || runtimeModelRef || '?') + '`\n' +
                  '- Available catalog models: ' + availableCount + '\n' +
                  '- Usage: `/model <provider/model>` or `/model <model>`',
                meta: '',
                tools: [],
                system_origin: 'slash:model'
              });
            }
          } else {
            self.pushSystemMessage({ id: ++msgId, role: 'system', is_notice: true, notice_type: 'info', notice_label: 'No agent selected.', text: 'No agent selected.', meta: '', tools: [], system_origin: 'slash:model' });
          }
          break;
        case '/apikey':
          await self.runSlashApiKeyDiscovery(cmdArgs);
          break;
        case '/file':
          if (!self.currentAgent) {
            self.pushSystemMessage({ id: ++msgId, role: 'system', is_notice: true, notice_type: 'info', notice_label: 'No agent selected.', text: 'No agent selected.', meta: '', tools: [], system_origin: 'slash:file' });
            break;
          }
          if (!cmdArgs || !String(cmdArgs).trim()) {
            self.pushSystemMessage({ id: ++msgId, role: 'system', is_notice: true, notice_type: 'info', notice_label: 'Usage: /file <path>', text: 'Usage: `/file <path>`', meta: '', tools: [], system_origin: 'slash:file' });
            break;
          }
          try {
            var fileRes = await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(self.currentAgent.id) + '/artifacts/file/read', {
              path: String(cmdArgs || '').trim()
            });
            var fileMeta = fileRes && fileRes.file ? fileRes.file : null;
            if (!fileMeta || !fileMeta.ok) {
              self.pushSystemMessage({ id: ++msgId, role: 'system', is_notice: true, notice_type: 'error', notice_label: 'Error: failed to read file output.', text: 'Error: failed to read file output.', meta: '', tools: [], system_origin: 'slash:file', ts: Date.now() });
            } else {
              var bytes = Number(fileMeta.bytes || 0);
              var fileMetaText = (bytes > 0 ? (bytes + ' bytes') : '');
              if (fileMeta.truncated) {
                var maxBytes = Number(fileMeta.max_bytes || 0);
                fileMetaText += (fileMetaText ? ' | ' : '') + 'truncated to ' + (maxBytes > 0 ? maxBytes : 'limit') + ' bytes';
              }
              self.messages.push({
                id: ++msgId, role: 'agent', text: '', meta: fileMetaText, tools: [], ts: Date.now(),
                file_output: { path: String(fileMeta.path || cmdArgs || ''), content: String(fileMeta.content || ''), truncated: !!fileMeta.truncated, bytes: bytes }
              });
            }
            self.scrollToBottom();
          } catch (e) {
            self.pushSystemMessage({ id: ++msgId, role: 'system', is_notice: true, notice_type: 'error', notice_label: 'File read failed', text: 'Error: ' + (e && e.message ? e.message : 'file read failed'), meta: '', tools: [], system_origin: 'slash:file', ts: Date.now() });
          }
          break;
        case '/folder':
          if (!self.currentAgent) {
            self.pushSystemMessage({ id: ++msgId, role: 'system', is_notice: true, notice_type: 'info', notice_label: 'No agent selected.', text: 'No agent selected.', meta: '', tools: [], system_origin: 'slash:folder' });
            break;
          }
          if (!cmdArgs || !String(cmdArgs).trim()) {
            self.pushSystemMessage({ id: ++msgId, role: 'system', is_notice: true, notice_type: 'info', notice_label: 'Usage: /folder <path>', text: 'Usage: `/folder <path>`', meta: '', tools: [], system_origin: 'slash:folder' });
            break;
          }
          try {
            var folderRes = await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(self.currentAgent.id) + '/artifacts/folder/export', {
              path: String(cmdArgs || '').trim()
            });
            var folderMeta = folderRes && folderRes.folder ? folderRes.folder : null;
            var archiveMeta = folderRes && folderRes.archive ? folderRes.archive : null;
            if (!folderMeta || !folderMeta.ok) {
              self.pushSystemMessage({ id: ++msgId, role: 'system', is_notice: true, notice_type: 'error', notice_label: 'Error: failed to export folder output.', text: 'Error: failed to export folder output.', meta: '', tools: [], system_origin: 'slash:folder', ts: Date.now() });
            } else {
              var entryCount = Number(folderMeta.entries || 0);
              var folderMetaText = (entryCount > 0 ? (entryCount + ' entries') : '');
              if (folderMeta.truncated) folderMetaText += (folderMetaText ? ' | ' : '') + 'tree truncated';
              if (archiveMeta && archiveMeta.file_name) folderMetaText += (folderMetaText ? ' | ' : '') + archiveMeta.file_name;
              self.messages.push({
                id: ++msgId, role: 'agent', text: '', meta: folderMetaText, tools: [], ts: Date.now(),
                folder_output: {
                  path: String(folderMeta.path || cmdArgs || ''), tree: String(folderMeta.tree || ''), entries: entryCount, truncated: !!folderMeta.truncated,
                  download_url: archiveMeta && archiveMeta.download_url ? String(archiveMeta.download_url) : '', archive_name: archiveMeta && archiveMeta.file_name ? String(archiveMeta.file_name) : '',
                  archive_bytes: Number(archiveMeta && archiveMeta.bytes ? archiveMeta.bytes : 0)
                }
              });
            }
            self.scrollToBottom();
          } catch (e2) {
            self.pushSystemMessage({ id: ++msgId, role: 'system', is_notice: true, notice_type: 'error', notice_label: 'Folder export failed', text: 'Error: ' + (e2 && e2.message ? e2.message : 'folder export failed'), meta: '', tools: [], system_origin: 'slash:folder', ts: Date.now() });
          }
          break;
        case '/clear':
          self.messages = [];
          break;
        case '/exit':
          InfringAPI.wsDisconnect();
          self._wsAgent = null;
          self.currentAgent = null;
          self.setStoreActiveAgentId(null);
          self.messages = [];
          window.dispatchEvent(new Event('close-chat'));
          break;
        case '/budget':
          self.pushSystemMessage({ id: ++msgId, role: 'system', is_notice: true, notice_type: 'info', notice_label: 'Budget status is not exposed through a Shell Socket projection yet.', text: 'Budget status is not exposed through a Shell Socket projection yet.', meta: '', tools: [], system_origin: 'slash:budget' });
          break;
        case '/peers':
          self.pushSystemMessage({ id: ++msgId, role: 'system', is_notice: true, notice_type: 'info', notice_label: 'Peer status is not exposed through a Shell Socket projection yet.', text: 'Peer status is not exposed through a Shell Socket projection yet.', meta: '', tools: [], system_origin: 'slash:peers' });
          break;
        case '/a2a':
          self.pushSystemMessage({ id: ++msgId, role: 'system', is_notice: true, notice_type: 'info', notice_label: 'A2A agent discovery is not exposed through a Shell Socket projection yet.', text: 'A2A agent discovery is not exposed through a Shell Socket projection yet.', meta: '', tools: [], system_origin: 'slash:a2a' });
          break;
        case '/memprobe':
          // Heap diagnostic: snapshots the chat page's memory footprint and
          // emits a structured report to chat + console. Run twice with an
          // idle gap (e.g., /memprobe, wait 30s, /memprobe again) to compute
          // a leak rate. The console output is the canonical record;
          // pushSystemMessage gives the operator a visible ack.
          self.runSlashMemprobe(cmdArgs);
          break;
      }
      this.scheduleConversationPersist();
    },

    runSlashMemprobe: function(cmdArgs) {
      var report = this.collectMemprobeReport(cmdArgs);
      // Pretty console block first (canonical record, full detail).
      try {
        var label = '[memprobe ' + report.captured_at_iso + ']';
        if (typeof console !== 'undefined' && console.group) {
          console.group(label);
          console.table(report.heap);
          console.table(report.dom_counts);
          console.table(report.custom_element_counts);
          console.table(report.suspected_accumulators);
          console.log('full_report:', report);
          if (report.delta) console.log('delta_vs_previous:', report.delta);
          console.groupEnd();
        } else {
          console.log(label, report);
        }
      } catch (_) {
        // Console errors should never break the chat.
      }
      // Compact chat ack with the headline numbers; the heavy detail stays in console.
      var heap = report.heap || {};
      var msgIdLocal = ++msgId;
      var heapMb = (Number(heap.used_js_heap_mb) || 0).toFixed(1);
      var heapTotalMb = (Number(heap.total_js_heap_mb) || 0).toFixed(1);
      var domNodes = (report.dom_counts && report.dom_counts.total_nodes) || 0;
      var bubbles = (report.custom_element_counts && report.custom_element_counts['infring-chat-bubble-render']) || 0;
      var placeholders = (report.custom_element_counts && report.custom_element_counts['infring-message-placeholder-shell']) || 0;
      var messageCount = Array.isArray(this.messages) ? this.messages.length : 0;
      var lines = [
        '**memprobe ' + report.capture_index + '**',
        '- heap_used: ' + heapMb + ' MB / total: ' + heapTotalMb + ' MB' + (heap.heap_unsupported ? ' (performance.memory unavailable)' : ''),
        '- dom_nodes: ' + domNodes,
        '- chat_bubble_render instances: ' + bubbles,
        '- message_placeholder_shell instances: ' + placeholders,
        '- messages: ' + messageCount,
      ];
      if (report.delta) {
        var d = report.delta;
        lines.push('- delta vs prev: heap ' + (d.used_js_heap_mb >= 0 ? '+' : '') + d.used_js_heap_mb.toFixed(1) + ' MB, dom ' + (d.total_nodes >= 0 ? '+' : '') + d.total_nodes + ' nodes, bubbles ' + (d.bubble_count >= 0 ? '+' : '') + d.bubble_count + ', elapsed ' + d.elapsed_ms + ' ms');
      } else {
        lines.push('- (run /memprobe again after 30s to see delta)');
      }
      lines.push('- full report logged to DevTools console');
      this.pushSystemMessage({
        id: msgIdLocal,
        role: 'system',
        is_notice: true,
        notice_type: 'info',
        notice_label: 'memprobe ' + report.capture_index,
        text: lines.join('\n'),
        meta: '',
        tools: [],
        system_origin: 'slash:memprobe',
      });
    },

    collectMemprobeReport: function(cmdArgs) {
      var now = Date.now();
      var capturedAt = new Date(now);
      var prev = this._lastMemprobeReport && typeof this._lastMemprobeReport === 'object'
        ? this._lastMemprobeReport
        : null;
      // Heap (Chromium-only; absent in Firefox/Safari).
      var perfMem = null;
      try {
        if (typeof performance !== 'undefined' && performance && performance.memory) {
          perfMem = performance.memory;
        }
      } catch (_) { perfMem = null; }
      var bytesToMb = function(n) { return Math.round((Number(n) || 0) / 1024 / 1024 * 100) / 100; };
      var heap = perfMem
        ? {
            heap_unsupported: false,
            used_js_heap_mb: bytesToMb(perfMem.usedJSHeapSize),
            total_js_heap_mb: bytesToMb(perfMem.totalJSHeapSize),
            jsheap_size_limit_mb: bytesToMb(perfMem.jsHeapSizeLimit),
            used_js_heap_bytes: Number(perfMem.usedJSHeapSize) || 0,
            total_js_heap_bytes: Number(perfMem.totalJSHeapSize) || 0,
          }
        : { heap_unsupported: true };
      // DOM counts.
      var domCounts = { total_nodes: 0, scripts: 0, styles: 0, divs: 0 };
      try {
        domCounts.total_nodes = document.querySelectorAll('*').length;
        domCounts.scripts = document.querySelectorAll('script').length;
        domCounts.styles = document.querySelectorAll('style,link[rel="stylesheet"]').length;
        domCounts.divs = document.querySelectorAll('div').length;
      } catch (_) {}
      // Custom element instance counts. The ones that follow are the per-message
      // Svelte islands plus the bubble itself; if any of these grow without
      // bound while messages stays roughly flat, the unmount path is leaking.
      var customElementTags = [
        'infring-chat-bubble-render',
        'infring-message-placeholder-shell',
        'infring-message-context-shell',
        'infring-message-meta-shell',
        'infring-message-artifact-shell',
        'infring-message-media-shell',
        'infring-message-progress-shell',
        'infring-message-terminal-shell',
        'infring-chat-divider-shell',
        'infring-chat-thread-shell',
        'infring-chat-stream-shell',
        'infring-messages-surface-shell',
        'infring-chat-map-shell',
      ];
      var customElementCounts = {};
      try {
        for (var i = 0; i < customElementTags.length; i++) {
          var tag = customElementTags[i];
          customElementCounts[tag] = document.querySelectorAll(tag).length;
        }
      } catch (_) {}
      // Suspected accumulators: sizes of fields known to potentially retain payload data.
      var jsonByteSize = function(value) {
        try {
          if (value == null) return 0;
          return JSON.stringify(value).length;
        } catch (_) { return -1; }
      };
      var storageByteSize = function(storage) {
        var total = 0;
        try {
          if (!storage) return 0;
          for (var si = 0; si < storage.length; si += 1) {
            var key = storage.key(si);
            if (!key) continue;
            total += String(key).length + String(storage.getItem(key) || '').length;
          }
        } catch (_) { return -1; }
        return total;
      };
      var messages = Array.isArray(this.messages) ? this.messages : [];
      var totalMessageTextBytes = 0;
      var totalMessageStreamBufferBytes = 0;
      var maxMessageStreamBufferBytes = 0;
      try {
        for (var m = 0; m < messages.length; m++) {
          var msg = messages[m];
          if (!msg || typeof msg !== 'object') continue;
          totalMessageTextBytes += String(msg.text || '').length;
          var streamBuf = String(msg._streamRawText || '').length
            + String(msg._cleanText || '').length
            + String(msg._thoughtText || '').length
            + String(msg._typewriterFinalText || '').length
            + String(msg._typingVisualHtml || '').length;
          totalMessageStreamBufferBytes += streamBuf;
          if (streamBuf > maxMessageStreamBufferBytes) maxMessageStreamBufferBytes = streamBuf;
        }
      } catch (_) {}
      var suspectedAccumulators = {
        message_count: messages.length,
        message_text_total_bytes: totalMessageTextBytes,
        message_text_total_kb: Math.round(totalMessageTextBytes / 1024),
        message_stream_buffer_total_bytes: totalMessageStreamBufferBytes,
        message_stream_buffer_total_kb: Math.round(totalMessageStreamBufferBytes / 1024),
        message_stream_buffer_max_bytes: maxMessageStreamBufferBytes,
        telemetry_snapshot_bytes: jsonByteSize(this._telemetrySnapshot),
        continuity_snapshot_bytes: jsonByteSize(this._continuitySnapshot),
        message_hydration_keys: this.messageHydration && typeof this.messageHydration === 'object'
          ? Object.keys(this.messageHydration).length
          : 0,
        forced_hydrate_keys: this._forcedHydrateById && typeof this._forcedHydrateById === 'object'
          ? Object.keys(this._forcedHydrateById).length
          : 0,
        message_line_expand_state_keys: this.messageLineExpandState && typeof this.messageLineExpandState === 'object'
          ? Object.keys(this.messageLineExpandState).length
          : 0,
        sessions_last_loaded_keys: this._sessionsLastLoadedAtByAgent && typeof this._sessionsLastLoadedAtByAgent === 'object'
          ? Object.keys(this._sessionsLastLoadedAtByAgent).length
          : 0,
      };
      var localStorageBytes = storageByteSize(typeof localStorage !== 'undefined' ? localStorage : null);
      var sessionStorageBytes = storageByteSize(typeof sessionStorage !== 'undefined' ? sessionStorage : null);
      var storageBytes = {
        local_storage_bytes: localStorageBytes,
        session_storage_bytes: sessionStorageBytes,
        total_storage_bytes: Math.max(0, localStorageBytes) + Math.max(0, sessionStorageBytes),
      };
      var captureIndex = Number((this._memprobeCaptureCount || 0) + 1) || 1;
      this._memprobeCaptureCount = captureIndex;
      var report = {
        type: 'chat_memprobe_report',
        capture_index: captureIndex,
        captured_at_ms: now,
        captured_at_iso: capturedAt.toISOString(),
        args: String(cmdArgs == null ? '' : cmdArgs),
        heap: heap,
        dom_counts: domCounts,
        custom_element_counts: customElementCounts,
        storage_bytes: storageBytes,
        suspected_accumulators: suspectedAccumulators,
        page_visible: typeof document !== 'undefined' && document && document.visibilityState ? document.visibilityState : 'unknown',
      };
      // Delta vs previous capture (if any).
      if (prev && prev.captured_at_ms) {
        var prevHeapMb = Number((prev.heap && prev.heap.used_js_heap_mb) || 0);
        var nextHeapMb = Number(heap.used_js_heap_mb || 0);
        var prevNodes = Number((prev.dom_counts && prev.dom_counts.total_nodes) || 0);
        var nextNodes = Number(domCounts.total_nodes || 0);
        var prevBubbles = Number((prev.custom_element_counts && prev.custom_element_counts['infring-chat-bubble-render']) || 0);
        var nextBubbles = Number(customElementCounts['infring-chat-bubble-render'] || 0);
        var prevStorage = Number((prev.storage_bytes && prev.storage_bytes.total_storage_bytes) || 0);
        var nextStorage = Number(storageBytes.total_storage_bytes || 0);
        report.delta = {
          elapsed_ms: now - Number(prev.captured_at_ms),
          used_js_heap_mb: Math.round((nextHeapMb - prevHeapMb) * 100) / 100,
          total_nodes: nextNodes - prevNodes,
          bubble_count: nextBubbles - prevBubbles,
          total_storage_bytes: nextStorage - prevStorage,
          message_count: messages.length - Number(prev.suspected_accumulators && prev.suspected_accumulators.message_count || 0),
        };
      }
      this._lastMemprobeReport = report;
      // Also expose the latest report via window for ad-hoc devtools access.
      try { if (typeof window !== 'undefined') window.__infringMemprobe = report; } catch (_) {}
      return report;
    },
    maybeDiscardPendingFreshAgent: function(nextAgentId) {
      var store = Alpine.store('app');
      if (!store) return;
      var pendingId = String(store.pendingFreshAgentId || '').trim();
      if (!pendingId) return;
      var targetId = String(nextAgentId || '').trim();
      if (!targetId || targetId === pendingId) return;
      store.pendingFreshAgentId = null;
      store.pendingAgent = null;
      InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(pendingId) + '/archive', { reason: 'discard_pending_fresh_agent' }).catch(function() {});
      if (typeof store.refreshAgents === 'function') {
        setTimeout(function() { store.refreshAgents({ force: true }).catch(function() {}); }, 0);
      }
    },
    selectAgent(agent) {
      var resolved = this.resolveAgent(agent);
      if (!resolved) return;
      var selectingSystemThread = this.isSystemThreadAgent(resolved);
      this.closeGitTreeMenu();
      var currentAgentId = this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : '';
      var nextAgentId = String((resolved && resolved.id) || '');
      this.maybeDiscardPendingFreshAgent(nextAgentId);
      if (currentAgentId !== nextAgentId) {
        var activeSearch = String(this.searchQuery || '').trim();
        if (activeSearch) {
          this.searchQuery = '';
          this.searchOpen = false;
        }
      }
      this._markAgentPreviewUnread(resolved.id, false);
      var store = Alpine.store('app');
      var pendingFreshId = store && store.pendingFreshAgentId ? String(store.pendingFreshAgentId) : '';
      var forceFreshSession = pendingFreshId && String(resolved.id) === pendingFreshId;
      this.clearHoveredMessageHard();
      if (this.currentAgent && this.currentAgent.id && this.currentAgent.id !== resolved.id) {
        var switchingFrom = String(this.currentAgent.id || '');
        if (
          this.sending &&
          this._pendingWsRequest &&
          String(this._pendingWsRequest.agent_id || '') === switchingFrom
        ) {
          this._clearTypingTimeout();
          this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; });
          this.sending = false;
          this._responseStartedAt = 0;
          this.setAgentLiveActivity(switchingFrom, 'working');
          this._recoverPendingWsRequest('agent_switch');
        }
        if (typeof this.captureConversationDraft === 'function') {
          this.captureConversationDraft(this.currentAgent.id);
        }
        this.cacheAgentConversation(this.currentAgent.id);
      }
      if (this.currentAgent && this.currentAgent.id === resolved.id) {
        if (selectingSystemThread) {
          this.activateSystemThread({ preserve_if_empty: true });
          return;
        }
        this.currentAgent = this.applyAgentGitTreeState(resolved, resolved) || resolved;
        this.recordModelUsageTimestamp(resolved.model_name || resolved.runtime_model || '');
        if (forceFreshSession) {
          this.applyConversationInputMode(resolved.id, { force_terminal: false });
          this.messages = [];
          this.inputText = '';
          this.contextApproxTokens = 0;
          this.refreshContextPressure();
          this.resetFreshInitStateForAgent(resolved);
          if (this.conversationCache) {
            delete this.conversationCache[String(resolved.id)];
            this.persistConversationCache();
          }
          InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(resolved.id) + '/fresh-session', {}).catch(function() {});
          this.connectWs(resolved.id);
          this.loadSessions(resolved.id);
          this.requestContextTelemetry(true);
          this.clearPromptSuggestions();
          this.startFreshInitSequence(resolved);
          if (typeof this.restoreConversationDraft === 'function') {
            this.restoreConversationDraft(resolved.id, 'chat');
          }
          var selfFreshCurrent = this;
          this.$nextTick(function() {
            selfFreshCurrent.scrollToBottomImmediate();
            selfFreshCurrent.stabilizeBottomScroll();
            selfFreshCurrent.pinToLatestOnOpen(null, { maxFrames: 20 });
            selfFreshCurrent.installChatMapWheelLock();
            selfFreshCurrent.scheduleMessageRenderWindowUpdate();
          });
        } else {
          this.loadSession(resolved.id, false);
        }
        if (!(this.isSystemThreadAgent && this.isSystemThreadAgent(resolved))) {
          this.refreshModelCatalogAndGuidance({ discover: true, guidance: true }).catch(function() {});
        }
        return;
      }
      if (selectingSystemThread) {
        this.activateSystemThread({ preserve_if_empty: false });
        return;
      }
      this.currentAgent = this.applyAgentGitTreeState(resolved, resolved) || resolved;
      if (store) this.setStoreActiveAgentId(resolved.id || null);
      this.recordModelUsageTimestamp(resolved.model_name || resolved.runtime_model || '');
      // Reset context meter on agent switch to avoid stale carry-over from prior threads.
      this.contextApproxTokens = 0;
      this.contextPressure = 'low';
      this.setContextWindowFromCurrentAgent();
      if (forceFreshSession) this.applyConversationInputMode(resolved.id, { force_terminal: false });
      else this.applyConversationInputMode(resolved.id);
      if (forceFreshSession && this.conversationCache) {
        delete this.conversationCache[String(resolved.id)];
        this.persistConversationCache();
        InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(resolved.id) + '/fresh-session', {}).catch(function() {});
      }
      var restored = forceFreshSession ? false : this.restoreAgentConversation(resolved.id);
      if (!restored) {
        this.messages = [];
        this.inputText = '';
        this.contextApproxTokens = 0;
        this.refreshContextPressure();
      }
      if (typeof this.restoreConversationDraft === 'function') {
        this.restoreConversationDraft(resolved.id);
      }
      this.showFreshArchetypeTiles = false;
      this.freshInitRevealMenu = false;
      if (forceFreshSession) {
        this.resetFreshInitStateForAgent(resolved);
        this.clearPromptSuggestions();
        this.startFreshInitSequence(resolved);
      } else {
        this.freshInitStageToken = Number(this.freshInitStageToken || 0) + 1;
        this._freshInitThreadShownFor = '';
      }
      this._reconcileSendingState();
      this.connectWs(resolved.id);
      if (!forceFreshSession) {
        this.loadSession(resolved.id, false);
      }
      this.loadSessions(resolved.id);
      this.requestContextTelemetry(true);
      this.refreshModelCatalogAndGuidance({ discover: true, guidance: true }).catch(function() {});
      if (!forceFreshSession) {
        this.refreshPromptSuggestions(false);
      }
      if (this.showAgentDrawer) {
        this.openAgentDrawer();
      }
      // Focus input after agent selection
      var self = this;
      this.$nextTick(function() {
        var el = document.getElementById('msg-input');
        if (el) el.focus();
        self.scrollToBottomImmediate();
        self.stabilizeBottomScroll();
        self.pinToLatestOnOpen(null, { maxFrames: 20 });
        self.installChatMapWheelLock();
        self.scheduleMessageRenderWindowUpdate();
      });
    },
    isMessageVirtualizationActive(list) {
      var rows = Array.isArray(list) ? list : this.messages;
      return Array.isArray(rows) && rows.length > 80;
    },
    messageRenderMetrics(msg) {
      if (!msg || typeof msg !== 'object') return null;
      var metrics = msg._renderMetrics;
      if (!metrics || typeof metrics !== 'object') {
        metrics = {};
        msg._renderMetrics = metrics;
      }
      return metrics;
    },
    resolveMessageByDomId(domId) {
      var target = String(domId || '').trim();
      if (!target) return null;
      var rows = Array.isArray(this.messages) ? this.messages : [];
      for (var i = 0; i < rows.length; i++) {
        if (this.messageDomId(rows[i], i) === target) return rows[i];
      }
      return null;
    },
    trackRenderedMessageMetrics(blockEl) {
      if (!blockEl || typeof blockEl.querySelector !== 'function') return;
      var metricRoot = blockEl.classList && blockEl.classList.contains('chat-message-block') ? blockEl : ((typeof blockEl.closest === 'function' && blockEl.closest('.chat-message-block')) || blockEl), bubble = metricRoot.querySelector('.message:not(.message-placeholder) .message-bubble:not(.message-placeholder-bubble)');
      if (!bubble) return;
      var msg = this.resolveMessageByDomId(String(metricRoot.id || blockEl.id || '').trim());
      if (!msg) return;
      var styles = window.getComputedStyle(bubble);
      var paddingTop = parseFloat(styles.paddingTop || '0');
      var paddingBottom = parseFloat(styles.paddingBottom || '0');
      var lineHeightRaw = parseFloat(styles.lineHeight || '0');
      var fontSizeRaw = parseFloat(styles.fontSize || '14');
      var lineHeight = Number.isFinite(lineHeightRaw) && lineHeightRaw > 0
        ? lineHeightRaw
        : Math.max(20, Math.round(fontSizeRaw * 1.6));
      var bubbleHeight = Math.max(0, Math.round(bubble.getBoundingClientRect().height));
      var bubbleWidth = Math.max(0, Math.round(bubble.getBoundingClientRect().width));
      var contentHeight = Math.max(0, bubbleHeight - Math.round(paddingTop + paddingBottom));
      var lineCount = Math.max(1, Math.ceil(contentHeight / Math.max(lineHeight, 1)));
      var metrics = this.messageRenderMetrics(msg);
      if (!metrics) return;
      metrics.lineCount = lineCount;
      metrics.lineHeight = Math.max(18, Math.round(lineHeight));
      metrics.bubbleHeight = Math.max(Math.round(lineHeight + paddingTop + paddingBottom), bubbleHeight);
      metrics.bubbleWidth = bubbleWidth;
      metrics.updatedAt = Date.now();
    },
    shouldRenderMessage(msg, idx, list) { void msg; void idx; void list; return true; },
    // Gate the heavyweight bubble content on the render window. When this returns
    // false, Alpine's x-if branch in index_body.html.parts unmounts the
    // <infring-chat-bubble-render> element (markdown + code blocks + media) and
    // mounts the lightweight <infring-message-placeholder-shell> instead, which
    // is a sized stack of <span class="message-placeholder-line"> elements
    // dimensioned from msg._renderMetrics so scroll position is preserved.
    //
    // Previously this was hardcoded to `return true`, which meant the heavy
    // bubble never unmounted; the .message-text-skeletonized CSS class was used
    // as a visual fallback (transparent text + repeating-linear-gradient gray
    // lines) but every DOM node was still rendered, so markdown parsing, code
    // tokenization, and layout cost all stayed in the hot path. Flipping this
    // gate to delegate to isMessageTextInRenderWindow turns the existing
    // placeholder infrastructure into a real DOM-level virtualization.
    shouldRenderMessageContent(msg, idx, list) {
      var rows = Array.isArray(list) ? list : this.messages;
      // Virtualization only kicks in once the chat passes the threshold
      // (currently > 80 messages, see isMessageVirtualizationActive). Below
      // that, render everything to keep the small-chat path simple.
      if (typeof this.isMessageVirtualizationActive === 'function'
        && !this.isMessageVirtualizationActive(rows)) return true;
      // Always keep streaming / thinking / typing-visual / thought-streaming
      // messages fully rendered. The user is actively watching them and any
      // visual flicker from unmount/remount destroys the live-text experience.
      if (msg && (msg.streaming || msg.thinking || msg._typingVisual || msg.thoughtStreaming)) return true;
      var domId = typeof this.messageDomId === 'function'
        ? this.messageDomId(msg, idx)
        : null;
      if (domId) {
        var forced = this._forcedHydrateById && typeof this._forcedHydrateById === 'object'
          ? this._forcedHydrateById
          : null;
        if (forced && Number(forced[domId] || 0) > Date.now()) return true;
        if (this.messageHydrationReady && this.messageHydration && typeof this.messageHydration === 'object') {
          return !!this.messageHydration[domId];
        }
      }
      // Fall through to the existing render-window logic (±messageTextRenderWindowRadius
      // around the active scroll position, default 20). Active position is
      // updated on every scroll by syncMapSelectionToScroll which sets
      // mapStepIndex + selectedMessageDomId from the viewport center, so this
      // gate follows the user's current focus.
      if (typeof this.isMessageTextInRenderWindow === 'function') {
        return !!this.isMessageTextInRenderWindow(msg, idx, rows);
      }
      // Conservative fallback: if the gate plumbing is missing, render.
      return true;
    },
    isMessageTextInRenderWindow(msg, idx, list) {
      var rows = Array.isArray(list) ? list : this.messages, active = Number(this.mapStepIndex), selected = String(this.selectedMessageDomId || this.hoveredMessageDomId || this.directHoveredMessageDomId || '').trim(), windowRows = Number(this.messageTextRenderWindowRadius || 20);
      if (!this.isMessageVirtualizationActive(rows)) return true;
      var domId = typeof this.messageDomId === 'function' ? this.messageDomId(msg, idx) : '';
      if (selected && domId && selected === domId) return true;
      if (!Number.isFinite(active) || active < 0 || active >= rows.length) active = Math.max(0, rows.length - 1);
      return Math.abs(Number(idx || 0) - active) <= (Number.isFinite(windowRows) && windowRows > 0 ? windowRows : 20) || !!(msg && (msg.streaming || msg.thinking || msg._typingVisual));
    },
    messageEstimatedLineCount(msg) {
      var metrics = this.messageRenderMetrics(msg);
      if (metrics && Number.isFinite(Number(metrics.lineCount)) && Number(metrics.lineCount) > 0) {
        return Math.max(1, Math.round(Number(metrics.lineCount)));
      }
      if (!msg || typeof msg !== 'object') return 1;
      var preview = '';
      if (typeof this.messageVisiblePreviewText === 'function') {
        preview = String(this.messageVisiblePreviewText(msg) || '');
      }
      if (!preview && typeof msg.text === 'string') preview = String(msg.text || '');
      var logicalLines = preview ? preview.split(/\r?\n/) : [''];
      var charsPerLine = msg.terminal ? 72 : (String(msg.role || '').toLowerCase() === 'user' ? 46 : 54);
      var lineCount = 0;
      for (var i = 0; i < logicalLines.length; i++) {
        var segment = String(logicalLines[i] || '');
        lineCount += Math.max(1, Math.ceil(segment.length / Math.max(charsPerLine, 1)));
      }
      if (Array.isArray(msg.tools) && msg.tools.length) lineCount += Math.max(2, msg.tools.length * 2);
      if (msg.file_output && msg.file_output.path) lineCount += 4;
      if (msg.folder_output && msg.folder_output.path) lineCount += 5;
      if (Array.isArray(msg.images) && msg.images.length) lineCount += Math.max(2, msg.images.length * 2);
      if (typeof this.messageProgress === 'function' && this.messageProgress(msg)) lineCount += 2;
      if (typeof this.messageToolTraceSummary === 'function' && this.messageToolTraceSummary(msg).visible) lineCount += 1;
      return Math.max(1, Math.min(48, lineCount));
    },
    messagePlaceholderResolvedLineCount(msg, idx, list) {
      void idx;
      void list;
      return this.messageEstimatedLineCount(msg);
    },
    messagePlaceholderResolvedLineHeight(msg, idx, list) {
      void idx;
      void list;
      var metrics = this.messageRenderMetrics(msg);
      if (metrics && Number.isFinite(Number(metrics.lineHeight)) && Number(metrics.lineHeight) > 0) {
        return Math.max(18, Math.round(Number(metrics.lineHeight)));
      }
      return msg && msg.terminal ? 20 : 24;
    },
    messagePlaceholderStyle(msg, idx, list) {
      var lineCount = this.messagePlaceholderResolvedLineCount(msg, idx, list);
      var lineHeight = this.messagePlaceholderResolvedLineHeight(msg, idx, list);
      var metrics = this.messageRenderMetrics(msg);
      var bubbleHeight = metrics && Number.isFinite(Number(metrics.bubbleHeight)) && Number(metrics.bubbleHeight) > 0
        ? Math.round(Number(metrics.bubbleHeight))
        : Math.round((lineCount * lineHeight) + (msg && msg.terminal ? 20 : 28));
      var trackedWidth = metrics && Number.isFinite(Number(metrics.bubbleWidth)) ? Math.round(Number(metrics.bubbleWidth)) : 0;
      var widthValue = 'var(--message-bubble-readable-width)';
      if (msg && msg.terminal) {
        widthValue = trackedWidth > 0 ? (trackedWidth + 'px') : 'min(84ch, 90%)';
      } else if (lineCount > 1 && trackedWidth > 0) {
        widthValue = Math.max(180, trackedWidth) + 'px';
      }
      return '--message-placeholder-line-count:' + String(lineCount) + ';' +
        '--message-placeholder-line-height:' + String(lineHeight) + 'px;' +
        '--message-placeholder-bubble-height:' + String(bubbleHeight) + 'px;' +
        '--message-placeholder-width:' + widthValue + ';';
    },
    messagePlaceholderLineIndices(msg, idx, list) {
      var count = this.messagePlaceholderResolvedLineCount(msg, idx, list);
      var indices = [];
      for (var i = 0; i < count; i++) indices.push(i);
      return indices;
    },
    forceMessageRender(msg, idx, ttlMs) {
      if (!msg) return;
      if (!this._forcedHydrateById || typeof this._forcedHydrateById !== 'object') this._forcedHydrateById = {};
      var domId = this.messageDomId(msg, idx);
      if (!domId) return;
      var ttl = Number(ttlMs || 0);
      if (!Number.isFinite(ttl) || ttl < 250) ttl = 2500;
      this._forcedHydrateById[domId] = Date.now() + ttl;
      this.scheduleMessageRenderWindowUpdate();
    },
    scheduleMessageRenderWindowUpdate(container) {
      var root = container && typeof container.querySelectorAll === 'function' ? container : null;
      if (this._renderWindowRaf) window.cancelAnimationFrame(this._renderWindowRaf);
      var self = this;
      this._renderWindowRaf = window.requestAnimationFrame(function() {
        self._renderWindowRaf = 0;
        self.updateMessageRenderWindow(root);
      });
    },
    updateMessageRenderWindow(container) {
      var root = container && typeof container.querySelectorAll === 'function'
        ? container
        : (this.$refs && this.$refs.messagesEl ? this.$refs.messagesEl : document.getElementById('messages'));
      if (!root) {
        this.messageHydration = {};
        this.messageHydrationReady = false;
        return;
      }
      var blocks = Array.prototype.slice.call(root.querySelectorAll('.chat-message-block[id]')); if (!blocks.length) blocks = Array.prototype.slice.call(root.querySelectorAll('.chat-message-block .message[id]'));
      if (!blocks.length) {
        this.messageHydration = {};
        this.messageHydrationReady = false;
        return;
      }
      for (var i = 0; i < blocks.length; i++) this.trackRenderedMessageMetrics(blocks[i]);
      if (!this.isMessageVirtualizationActive(blocks)) {
        this.messageHydration = {};
        this.messageHydrationReady = false;
        return;
      }
      var scrollTop = Number(root.scrollTop || 0);
      var viewportHeight = Number(root.clientHeight || 0);
      var bufferPx = Math.max(viewportHeight, 320);
      var firstVisible = -1;
      var lastVisible = -1;
      for (var j = 0; j < blocks.length; j++) {
        var block = blocks[j];
        var top = Number(block.offsetTop || 0);
        var height = Number(block.offsetHeight || 0);
        var bottom = top + Math.max(height, 1);
        if (bottom >= (scrollTop - bufferPx) && top <= (scrollTop + viewportHeight + bufferPx)) {
          if (firstVisible < 0) firstVisible = j;
          lastVisible = j;
        }
      }
      if (firstVisible < 0 || lastVisible < 0) {
        firstVisible = Math.max(0, blocks.length - 20);
        lastVisible = blocks.length - 1;
      }
      var extraRows = 10;
      var start = Math.max(0, firstVisible - extraRows);
      var end = Math.min(blocks.length - 1, lastVisible + extraRows);
      var nextHydration = {};
      for (var k = start; k <= end; k++) {
        nextHydration[blocks[k].id] = true;
      }
      if (blocks.length > 0) {
        nextHydration[blocks[0].id] = true;
        nextHydration[blocks[blocks.length - 1].id] = true;
      }
      if (this.selectedMessageDomId) nextHydration[String(this.selectedMessageDomId)] = true;
      if (this.hoveredMessageDomId) nextHydration[String(this.hoveredMessageDomId)] = true;
      if (this.directHoveredMessageDomId) nextHydration[String(this.directHoveredMessageDomId)] = true;
      var retainedForced = {};
      var now = Date.now();
      var forced = this._forcedHydrateById && typeof this._forcedHydrateById === 'object' ? this._forcedHydrateById : {};
      Object.keys(forced).forEach(function(domId) {
        var expiresAt = Number(forced[domId] || 0);
        if (!Number.isFinite(expiresAt) || expiresAt <= now) return;
        retainedForced[domId] = expiresAt;
        nextHydration[domId] = true;
      });
      this._forcedHydrateById = retainedForced;
      this.messageHydration = nextHydration;
      this.messageHydrationReady = true;
      var chatStore = window.InfringChatStore;
      if (chatStore && typeof chatStore.bumpRenderWindowVersion === 'function') chatStore.bumpRenderWindowVersion();
    },

    runSlashApiKeyDiscovery: async function(cmdArgs) {
      if (!cmdArgs || !String(cmdArgs).trim()) {
        this.messages.push({
          id: ++msgId,
          role: 'system',
          is_notice: true,
          notice_type: 'info',
          notice_label: 'Usage: /apikey <api-key-or-local-model-path>',
          text: 'Usage: `/apikey <api-key-or-local-model-path>`',
          meta: '',
          tools: [],
          system_origin: 'slash:apikey'
        });
        this.scrollToBottom();
        return;
      }
      try {
        var discoveryInput = String(cmdArgs || '').trim();
        var discovery = await InfringAPI.post('/api/shell-socket/models/discover', {
          input: discoveryInput,
          api_key: discoveryInput
        });
        var catalogRows = typeof this.loadModelCatalogSafely === 'function'
          ? await this.loadModelCatalogSafely({
            prefer_cached: true,
            suppress_errors: true
          })
          : this.sanitizeModelCatalogRows(this._modelCache || []);
        if (this.availableModelRowsCount(catalogRows) === 0) {
          this.injectNoModelsGuidance('apikey_discover');
        }
        var statusLine = typeof this.describeModelDiscoveryResult === 'function'
          ? this.describeModelDiscoveryResult(discovery, catalogRows)
          : 'Model discovery updated.';
        var providerName = String((discovery && discovery.provider) || '').trim();
        var inputKind = String((discovery && discovery.input_kind) || '').trim().toLowerCase();
        var guidanceLine = inputKind === 'local_path'
          ? 'Local model path indexed and ready for `/model`.'
          : (providerName
            ? ('Provider `' + providerName + '` is now available in the model switcher.')
            : 'Refresh the model switcher to use the new entries.');
        this.messages.push({
          id: ++msgId,
          role: 'system',
          is_notice: true,
          notice_type: 'info',
          notice_label: statusLine,
          text: statusLine + '\n' + guidanceLine,
          meta: '',
          tools: [],
          system_origin: 'slash:apikey'
        });
        this.scrollToBottom();
        this.scheduleConversationPersist();
      } catch (eApikey) {
        this.messages.push({
          id: ++msgId,
          role: 'system',
          is_notice: true,
          notice_type: 'error',
          notice_label: 'API key/model path discovery failed',
          text: 'API key/model path discovery failed: ' + (eApikey && eApikey.message ? eApikey.message : eApikey),
          meta: '',
          tools: [],
          system_origin: 'slash:apikey'
        });
        this.scrollToBottom();
      }
    },
    exportCurrentChatMarkdown: function() {
      var assistantName = String(
        (this.currentAgent && (this.currentAgent.name || this.currentAgent.id)) || 'infring'
      ).trim() || 'infring';
      return exportChatMarkdown(this.messages, assistantName);
    },

    isFreshInitTemplateSelected(templateDef) {
      if (!templateDef) return false;
      var key = String(templateDef.name || '').trim();
      return !!key && key === String(this.freshInitTemplateName || '').trim();
    },

    freshInitTemplateDescription: function(templateDef) {
      if (!templateDef) return '';
      if (templateDef.is_other) {
        var typed = String(this.freshInitOtherPrompt || '').trim();
        if (typed) return this.truncateFreshInitSummary(typed, 86);
      }
      return String(templateDef.description || '').trim();
    },

    truncateFreshInitSummary: function(text, limit) {
      var clean = String(text || '').replace(/\s+/g, ' ').trim();
      if (!clean) return '';
      var max = Number(limit || 0);
      if (!Number.isFinite(max) || max < 12) max = 80;
      if (clean.length <= max) return clean;
      return clean.slice(0, Math.max(8, max - 1)).trimEnd() + '…';
    },

    filteredFreshInitEmojiCatalog: function() {
      var source = Array.isArray(this.drawerEmojiCatalog) ? this.drawerEmojiCatalog : [];
      var query = String(this.freshInitEmojiSearch || '').trim().toLowerCase();
      var self = this;
      var rows = source.filter(function(row) {
        var emoji = String((row && row.emoji) || '');
        if (self.isReservedSystemEmoji && self.isReservedSystemEmoji(emoji)) return false;
        return true;
      });
      if (!query) return rows.slice(0, 24);
      return rows.filter(function(row) {
        var emoji = String((row && row.emoji) || '');
        var name = String((row && row.name) || '').toLowerCase();
        return emoji.indexOf(query) >= 0 || name.indexOf(query) >= 0;
      }).slice(0, 24);
    },

    toggleFreshInitEmojiPicker: function() {
      this.freshInitEmojiPickerOpen = !this.freshInitEmojiPickerOpen;
      if (!this.freshInitEmojiPickerOpen) {
        this.freshInitEmojiSearch = '';
      }
    },

    selectFreshInitEmoji: function(choice) {
      var emoji = String(choice && choice.emoji ? choice.emoji : choice || '').trim();
      if (!emoji) return;
      var sanitized = this.sanitizeAgentEmojiForDisplay
        ? this.sanitizeAgentEmojiForDisplay(this.currentAgent, emoji)
        : emoji;
      if (!sanitized) {
        InfringToast.info('The gear icon is reserved for the System thread.');
        return;
      }
      this.freshInitEmoji = sanitized;
      this.freshInitAvatarUrl = '';
      this.freshInitEmojiPickerOpen = false;
      this.freshInitEmojiSearch = '';
    },

    openFreshInitAvatarPicker: function() {
      if (this.$refs && this.$refs.freshInitAvatarInput) {
        this.$refs.freshInitAvatarInput.click();
      }
    },

    uploadFreshInitAvatar: async function(fileList) {
      if (!this.currentAgent || !this.currentAgent.id) return;
      var files = Array.isArray(fileList) ? fileList : Array.from(fileList || []);
      if (!files.length) return;
      var file = files[0];
      if (!file) return;
      var mime = String(file.type || '').toLowerCase();
      if (mime && mime.indexOf('image/') !== 0) {
        InfringToast.error('Avatar must be an image file.');
        return;
      }
      this.freshInitAvatarUploading = true;
      this.freshInitAvatarUploadError = '';
      try {
        var headers = {
          'Content-Type': file.type || 'application/octet-stream',
          'X-Filename': file.name || 'avatar'
        };
        var token = (typeof InfringAPI !== 'undefined' && typeof InfringAPI.getToken === 'function')
          ? String(InfringAPI.getToken() || '')
          : '';
        if (token) headers.Authorization = 'Bearer ' + token;
        var response = await fetch('/api/agents/' + encodeURIComponent(this.currentAgent.id) + '/avatar', {
          method: 'POST',
          headers: headers,
          body: file
        });
        var payload = null;
        try {
          payload = await response.json();
        } catch (_) {
          payload = null;
        }
        if (!response.ok || !payload || !payload.ok || !payload.avatar_url) {
          throw new Error(String(payload && payload.error ? payload.error : 'avatar_upload_failed'));
        }
        this.freshInitAvatarUrl = String(payload.avatar_url || '').trim();
        this.freshInitEmojiPickerOpen = false;
        this.freshInitEmojiSearch = '';
      } catch (error) {
        var message = (error && error.message) ? String(error.message) : 'avatar_upload_failed';
        this.freshInitAvatarUploadError = message;
        InfringToast.error('Failed to upload avatar: ' + message);
      } finally {
        this.freshInitAvatarUploading = false;
      }
    },

    clearFreshInitAvatar: function() {
      this.freshInitAvatarUrl = '';
      this.freshInitAvatarUploadError = '';
    },

    isFreshInitPersonalitySelected: function(card) {
      if (!card) return false;
      return String(card.id || '') === String(this.freshInitPersonalityId || '');
    },

    selectFreshInitPersonality: function(card) {
      var id = String(card && card.id ? card.id : 'none').trim() || 'none';
      this.freshInitPersonalityId = id;
      if (typeof this.scheduleFreshInitProgressAnchor === 'function') this.scheduleFreshInitProgressAnchor();
    },

    selectedFreshInitPersonality: function() {
      var cards = Array.isArray(this.freshInitPersonalityCards) ? this.freshInitPersonalityCards : [];
      var selectedId = String(this.freshInitPersonalityId || 'none');
      for (var i = 0; i < cards.length; i += 1) {
        if (String(cards[i] && cards[i].id ? cards[i].id : '') === selectedId) return cards[i];
      }
      return cards.length ? cards[0] : null;
    },

    isFreshInitLifespanSelected: function(card) {
      if (!card) return false;
      return String(card.id || '') === String(this.freshInitLifespanId || '');
    },

    selectFreshInitLifespan: function(card) {
      var id = String(card && card.id ? card.id : '1h').trim() || '1h';
      this.freshInitLifespanId = id;
      if (typeof this.scheduleFreshInitProgressAnchor === 'function') this.scheduleFreshInitProgressAnchor();
    },

    selectedFreshInitLifespan: function() {
      var cards = Array.isArray(this.freshInitLifespanCards) ? this.freshInitLifespanCards : [];
      var selectedId = String(this.freshInitLifespanId || '1h');
      var fallback = null;
      for (var i = 0; i < cards.length; i += 1) {
        var cardId = String(cards[i] && cards[i].id ? cards[i].id : '');
        if (cardId === '1h') fallback = cards[i];
        if (cardId === selectedId) return cards[i];
      }
      return fallback || (cards.length ? cards[0] : null);
    },

    async applyChatArchetypeTemplate(templateDef) {
      if (!templateDef) return;
      this.freshInitTemplateDef = templateDef;
      this.freshInitTemplateName = String(templateDef.name || '').trim();
      this.freshInitModelManual = false;
      this.freshInitModelSelection = '';
      this.refreshFreshInitModelSuggestions(templateDef);
      if (templateDef.is_other) {
        this.freshInitAwaitingOtherPrompt = true;
        this.focusChatComposerFromInit(String(this.freshInitOtherPrompt || '').trim());
      } else {
        this.freshInitAwaitingOtherPrompt = false;
      }
      if (typeof this.scheduleFreshInitProgressAnchor === 'function') this.scheduleFreshInitProgressAnchor();
    },

    captureFreshInitOtherPrompt: function() {
      if (!this.showFreshArchetypeTiles || !this.freshInitAwaitingOtherPrompt) return false;
      if (Array.isArray(this.attachments) && this.attachments.length > 0) {
        InfringToast.info('Init prompt does not support file attachments.');
        return false;
      }
      var text = String(this.inputText || '').trim();
      if (!text) {
        InfringToast.info('Describe the special purpose first.');
        this.focusChatComposerFromInit('');
        return false;
      }
      this.freshInitOtherPrompt = text;
      this.freshInitAwaitingOtherPrompt = false;
      this.inputText = '';
      var ta = document.getElementById('msg-input');
      if (ta) ta.style.height = '';
      if (typeof this.scheduleFreshInitProgressAnchor === 'function') this.scheduleFreshInitProgressAnchor('lifespan');
      return true;
    },

    resolveFreshInitSystemPrompt: function(templateDef, agentName, personalityCard, vibeCard) {
      if (!templateDef) return '';
      var basePrompt = '';
      if (templateDef.is_other) {
        var purpose = String(this.freshInitOtherPrompt || '').trim();
        basePrompt = [
          'You are ' + String(agentName || 'the assistant') + '.',
          'Special purpose: ' + purpose,
          'Act as a focused specialist for this purpose. Stay concise, practical, and reliable.',
        ].join('\n');
      } else {
        basePrompt = String(templateDef.system_prompt || '').trim();
      }
      var personalitySuffix = String(personalityCard && personalityCard.system_suffix ? personalityCard.system_suffix : '').trim();
      var vibeSuffix = String(vibeCard && vibeCard.system_suffix ? vibeCard.system_suffix : '').trim();
      var suffixes = [];
      if (personalitySuffix) suffixes.push(personalitySuffix);
      if (vibeSuffix) suffixes.push(vibeSuffix);
      if (suffixes.length) {
        return (basePrompt ? (basePrompt + '\n\n') : '') + suffixes.join('\n');
      }
      return basePrompt;
    },

    resolveFreshInitRole: function(templateDef) {
      var currentRole = String((this.currentAgent && this.currentAgent.role) || '').trim().toLowerCase();
      if (!templateDef) return currentRole || 'analyst';
      var hint = String(
        templateDef.role || templateDef.archetype || templateDef.profile || templateDef.name || ''
      ).trim().toLowerCase();
      if (!hint) return currentRole || 'analyst';
      if (hint.indexOf('teacher') >= 0 || hint.indexOf('tutor') >= 0 || hint.indexOf('mentor') >= 0 || hint.indexOf('coach') >= 0 || hint.indexOf('instructor') >= 0) {
        return 'tutor';
      }
      if (hint.indexOf('code') >= 0 || hint.indexOf('coder') >= 0 || hint.indexOf('engineer') >= 0 || hint.indexOf('developer') >= 0 || hint.indexOf('devops') >= 0 || hint.indexOf('api') >= 0 || hint.indexOf('build') >= 0) {
        return 'engineer';
      }
      if (hint.indexOf('research') >= 0 || hint.indexOf('investig') >= 0) {
        return 'researcher';
      }
      if (hint.indexOf('analyst') >= 0 || hint.indexOf('analysis') >= 0 || hint.indexOf('data') >= 0 || hint.indexOf('meeting') >= 0) {
        return 'analyst';
      }
      if (hint.indexOf('writer') >= 0 || hint.indexOf('editor') >= 0 || hint.indexOf('content') >= 0) {
        return 'writer';
      }
      if (hint.indexOf('design') >= 0 || hint.indexOf('ui') >= 0 || hint.indexOf('ux') >= 0) {
        return 'designer';
      }
      if (hint.indexOf('support') >= 0) {
        return 'support';
      }
      return currentRole || 'analyst';
    },

    resolveFreshInitContractPayload: function(agentName) {
      var selected = this.selectedFreshInitLifespan();
      var mission = 'Initialize and run as ' + String(agentName || 'agent') + '.';
      if (!selected) {
        return {
          mission: mission,
          termination_condition: 'task_or_timeout',
          expiry_seconds: 60 * 60,
          indefinite: false,
          auto_terminate_allowed: true,
          idle_terminate_allowed: true,
        };
      }
      var terminationCondition = String(selected.termination_condition || 'task_or_timeout');
      var expirySeconds = selected.expiry_seconds == null ? null : Number(selected.expiry_seconds);
      var indefinite = selected.indefinite === true;
      var supportsTimeout = terminationCondition === 'timeout' || terminationCondition === 'task_or_timeout';
      return {
        mission: mission,
        termination_condition: terminationCondition,
        expiry_seconds: expirySeconds,
        indefinite: indefinite,
        auto_terminate_allowed: !indefinite && supportsTimeout && expirySeconds != null,
        idle_terminate_allowed: !indefinite && supportsTimeout && expirySeconds != null,
      };
    },

    async launchFreshAgentInitialization() {
      if (!this.currentAgent || !this.currentAgent.id) return;
      if (this.freshInitLaunching) return;
      if (!this.freshInitTemplateDef) {
        InfringToast.info('Select an archetype first.');
        return;
      }
      var agentId = this.currentAgent.id;
      var templateDef = this.freshInitTemplateDef;
      var provider = String(templateDef.provider || '').trim();
      var model = String(templateDef.model || '').trim();
      var selectedModel = this.selectedFreshInitModelSuggestion();
      var selectedModelRef = this.normalizeFreshInitModelRef(selectedModel);
      if (!selectedModelRef) {
        await this.refreshFreshInitModelSuggestions(templateDef);
        selectedModel = this.selectedFreshInitModelSuggestion();
        selectedModelRef = this.normalizeFreshInitModelRef(selectedModel);
      }
      var resolvedModelRef = selectedModelRef;
      if (!resolvedModelRef && provider && model) resolvedModelRef = provider.toLowerCase() + '/' + model;
      var requestedName = String(this.freshInitName || '').trim();
      var requestedEmoji = String(this.freshInitEmoji || '').trim();
      var launchName = requestedName || 'agent';
      if (templateDef.is_other && !String(this.freshInitOtherPrompt || '').trim()) {
        InfringToast.info('Describe the special purpose for Other before launch.');
        this.freshInitAwaitingOtherPrompt = true;
        this.focusChatComposerFromInit('');
        return;
      }
      var selectedPersonality = this.selectedFreshInitPersonality();
      var selectedVibe = this.selectedFreshInitVibe();
      var resolvedSystemPrompt = this.resolveFreshInitSystemPrompt(templateDef, launchName, selectedPersonality, selectedVibe);
      var resolvedContract = this.resolveFreshInitContractPayload(launchName);
      var resolvedPermissions = this.resolveFreshInitPermissionManifest ? this.resolveFreshInitPermissionManifest() : null;
      if (resolvedPermissions && typeof resolvedPermissions === 'object') resolvedContract.permissions_manifest = resolvedPermissions;
      this.freshInitLaunching = true;
      this.freshInitRevealMenu = false;
      this.freshInitEmojiPickerOpen = false;
      try {
        if (resolvedModelRef) {
          await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(agentId) + '/model', {
            model: resolvedModelRef
          });
        }
        var sanitizedRequestedEmoji = this.sanitizeAgentEmojiForDisplay
          ? this.sanitizeAgentEmojiForDisplay(this.currentAgent, requestedEmoji || '')
          : (requestedEmoji || '');
        var identityPayload = {};
        if (String(sanitizedRequestedEmoji || '').trim()) {
          identityPayload.emoji = String(sanitizedRequestedEmoji || '').trim();
        }
        var vibeValue = String(selectedVibe && selectedVibe.id ? selectedVibe.id : '').trim();
        if (vibeValue && vibeValue !== 'none') identityPayload.vibe = vibeValue;
        var configPayload = {
          role: this.resolveFreshInitRole(templateDef),
          identity: identityPayload,
          system_prompt: resolvedSystemPrompt,
          archetype: String(templateDef.archetype || '').trim(),
          profile: String(templateDef.profile || '').trim(),
          contract: resolvedContract,
          termination_condition: resolvedContract.termination_condition,
          expiry_seconds: resolvedContract.expiry_seconds,
          indefinite: resolvedContract.indefinite === true,
        };
        if (requestedName) {
          configPayload.name = requestedName;
        }
        if (!Object.keys(identityPayload).length) {
          delete configPayload.identity;
        }
        if (this.freshInitAvatarUrl) {
          configPayload.avatar_url = String(this.freshInitAvatarUrl || '').trim();
        }
        await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(agentId) + '/config', {
          ...configPayload
        });
        var appliedAgentName = requestedName || String(this.currentAgent.name || this.currentAgent.id || agentId).trim() || 'agent';
        this.addNoticeEvent({
          notice_label: 'Initialized ' + appliedAgentName + ' as ' + String(templateDef.name || 'template'),
          notice_type: 'info',
          ts: Date.now()
        });
        try {
          var store = Alpine.store('app');
          if (store) {
            store.pendingFreshAgentId = null;
            store.pendingAgent = null;
            if (typeof store.refreshAgents === 'function') {
              await store.refreshAgents();
            }
          }
        } catch(_) {}
        await this.syncDrawerAgentAfterChange();
        await this.loadSession(agentId, false);
        this.requestContextTelemetry(true);
        this.freshInitStageToken = Number(this.freshInitStageToken || 0) + 1;
        this.showFreshArchetypeTiles = false;
        this.freshInitTemplateDef = null;
        this.freshInitTemplateName = '';
        this.freshInitLaunching = false; if (typeof this.resetFreshInitPermissions === 'function') this.resetFreshInitPermissions();
        var launchedRole = String((templateDef && (templateDef.name || templateDef.profile || templateDef.archetype)) || 'agent').trim() || 'agent';
        InfringToast.success('Launched ' + String(appliedAgentName || 'agent') + ' as ' + launchedRole);
      } catch (e) {
        this.freshInitLaunching = false;
        this.freshInitRevealMenu = true;
        InfringToast.error('Failed to initialize agent: ' + e.message);
      }
    },

    extractTerminalCommandsFromHistoryText: function(rawText) {
      var text = String(rawText || '');
      if (!text.trim()) return [];
      var lines = text.split('\n');
      var out = [];
      for (var i = 0; i < lines.length; i++) {
        var line = String(lines[i] || '').trim();
        if (!line) continue;
        var marker = line.indexOf(' % ');
        if (marker <= 0) continue;
        var cmd = line.slice(marker + 3).trim();
        if (cmd) out.push(cmd);
      }
      return out;
    },

    normalizeSessionKeyToken: function(value, fallback) {
      var raw = String(value == null ? '' : value).trim().toLowerCase();
      raw = raw.replace(/[^a-z0-9:_-]+/g, '-').replace(/^-+|-+$/g, '');
      if (raw) return raw;
      var fallbackValue = String(fallback == null ? '' : fallback).trim().toLowerCase();
      return fallbackValue || 'main';
    },

    normalizeSessionAgentId: function(value) {
      var raw = String(value == null ? '' : value).trim().toLowerCase();
      raw = raw.replace(/[^a-z0-9_-]+/g, '-').replace(/^-+|-+$/g, '');
      return raw || 'main';
    },

    parseAgentSessionKey: function(sessionKey) {
      var raw = String(sessionKey == null ? '' : sessionKey).trim().toLowerCase();
      if (!raw) return null;
      var parts = raw.split(':').filter(Boolean);
      if (parts.length < 3 || parts[0] !== 'agent') return null;
      var agentId = this.normalizeSessionAgentId(parts[1]);
      var rest = parts.slice(2).join(':');
      if (!rest) return null;
      return {
        agentId: agentId,
        rest: this.normalizeSessionKeyToken(rest, 'main')
      };
    },

    resolveSessionAgentIdFromKey: function(sessionKey, fallbackAgentId) {
      var parsed = this.parseAgentSessionKey(sessionKey);
      if (parsed && parsed.agentId) return parsed.agentId;
      return this.normalizeSessionAgentId(fallbackAgentId);
    },

    isSubagentSessionKey: function(sessionKey) {
      var raw = String(sessionKey == null ? '' : sessionKey).trim().toLowerCase();
      if (!raw) return false;
      if (raw.indexOf('subagent:') === 0) return true;
      var parsed = this.parseAgentSessionKey(raw);
      return !!(parsed && parsed.rest.indexOf('subagent:') === 0);
    },

    resolveSessionRowScopeToken: function(row) {
      var rawKey = String(
        (row && (row.session_key || row.key || row.session_id || row.id || row.main_key)) || ''
      ).trim();
      var parsed = this.parseAgentSessionKey(rawKey);
      if (parsed && parsed.rest) return parsed.rest;
      return this.normalizeSessionKeyToken(rawKey, 'main');
    },

    resolveSessionRowLabel: function(row, fallbackAgentId) {
      var explicitLabel = String((row && (row.label || row.name || row.session_label)) || '').trim();
      if (explicitLabel) return explicitLabel;
      var rawKey = String((row && (row.session_key || row.key || row.session_id || row.id)) || '').trim();
      var parsed = this.parseAgentSessionKey(rawKey);
      var scopeToken = parsed && parsed.rest ? parsed.rest : this.resolveSessionRowScopeToken(row);
      if (scopeToken === 'main') return 'Main';
      if (scopeToken.indexOf('subagent:') === 0) {
        var subagentTail = scopeToken.slice('subagent:'.length).replace(/[:_-]+/g, ' ').trim();
        return subagentTail ? ('Subagent ' + subagentTail) : 'Subagent';
      }
      var normalized = String(scopeToken || '').replace(/[:_-]+/g, ' ').trim();
      if (normalized) return normalized;
      return this.normalizeSessionAgentId(fallbackAgentId);
    },

    normalizeSessionsList: function(rows, fallbackAgentId) {
      var source = Array.isArray(rows) ? rows : [];
      var out = [];
      var seen = {};
      for (var i = 0; i < source.length; i++) {
        var row = source[i];
        if (!row || typeof row !== 'object') continue;
        var rawKey = String((row.session_key || row.key || row.session_id || row.id) || '').trim();
        var agentId = this.resolveSessionAgentIdFromKey(rawKey, row.agent_id || row.agentId || fallbackAgentId);
        var scopeToken = this.resolveSessionRowScopeToken(row);
        var scopeKey = this.normalizeSessionAgentId(agentId) + '|' + scopeToken;
        if (seen[scopeKey]) continue;
        seen[scopeKey] = true;
        out.push(Object.assign({}, row, {
          _agent_id: this.normalizeSessionAgentId(agentId),
          _scope_token: scopeToken,
          _scope_key: scopeKey,
          _label: this.resolveSessionRowLabel(row, agentId),
          _is_subagent: this.isSubagentSessionKey(rawKey),
        }));
      }
      return out;
    },

    resolveCurrentSessionRow: function(agentId) {
      var normalizedAgentId = this.normalizeSessionAgentId(agentId);
      var rows = this.normalizeSessionsList(this.sessions || [], normalizedAgentId);
      var fallback = null;
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i] || {};
        if (!fallback && row._agent_id === normalizedAgentId) fallback = row;
        if (row._agent_id === normalizedAgentId && row.active === true) return row;
      }
      if (fallback) return fallback;
      for (var j = 0; j < rows.length; j++) {
        if (rows[j] && rows[j].active === true) return rows[j];
      }
      return rows.length ? rows[0] : null;
    },

    resolveConversationCacheScopeKey: function(agentId, explicitSessionRow) {
      var normalizedAgentId = this.normalizeSessionAgentId(agentId);
      var row = explicitSessionRow && typeof explicitSessionRow === 'object'
        ? explicitSessionRow
        : this.resolveCurrentSessionRow(normalizedAgentId);
      var scopeToken = row && row._scope_token
        ? row._scope_token
        : this.resolveSessionRowScopeToken(row || {});
      return normalizedAgentId + '|' + this.normalizeSessionKeyToken(scopeToken, 'main');
    },

    applySessionsPayloadSnapshot: function(agentId, payload) {
      var normalizedAgentId = this.normalizeSessionAgentId(agentId);
      var rows = [];
      if (payload && payload.session && Array.isArray(payload.session.sessions)) {
        rows = payload.session.sessions;
      } else if (payload && Array.isArray(payload.sessions)) {
        rows = payload.sessions;
      }
      var normalizedRows = this.normalizeSessionsList(rows, normalizedAgentId);
      if (!normalizedRows.length) return;
      this.sessions = normalizedRows;
      if (!this._sessionsLastLoadedAtByAgent || typeof this._sessionsLastLoadedAtByAgent !== 'object') {
        this._sessionsLastLoadedAtByAgent = {};
      }
      this._sessionsLastLoadedAtByAgent[normalizedAgentId] = Date.now();
    },

    rebuildInputHistoryFromSessionPayload: function(data) {
      var payload = data && typeof data === 'object' ? data : {};
      var fallbackAgentId = String(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '').trim();
      this.applySessionsPayloadSnapshot(fallbackAgentId, payload);
      var state = payload && payload.session && typeof payload.session === 'object' ? payload.session : {};
      var sessions = this.normalizeSessionsList(Array.isArray(state.sessions) ? state.sessions : [], fallbackAgentId);
      var sourceRows = [];
      var seenSessionScopes = {};
      for (var i = 0; i < sessions.length; i++) {
        var session = sessions[i] || {};
        var scopeKey = String(session._scope_key || '').trim();
        if (scopeKey && seenSessionScopes[scopeKey]) continue;
        if (scopeKey) seenSessionScopes[scopeKey] = true;
        var messages = Array.isArray(session.messages) ? session.messages : [];
        for (var j = 0; j < messages.length; j++) sourceRows.push(messages[j]);
      }
      if (Array.isArray(payload.messages)) {
        for (var m = 0; m < payload.messages.length; m++) sourceRows.push(payload.messages[m]);
      }
      if (payload && payload.message_window && Array.isArray(payload.message_window.rows)) {
        for (var mw = 0; mw < payload.message_window.rows.length; mw++) sourceRows.push(payload.message_window.rows[mw]);
      }
      if (!sourceRows.length) {
        this.chatInputHistory = [];
        this.terminalInputHistory = [];
        this.hydrateInputHistoryFromCache('chat');
        this.hydrateInputHistoryFromCache('terminal');
        this.resetInputHistoryNavigation('chat');
        this.resetInputHistoryNavigation('terminal');
        return;
      }

      var normalized = this.normalizeSessionMessages({ messages: sourceRows });
      var maxEntries = Number(this.inputHistoryMaxEntries || 0);
      if (!Number.isFinite(maxEntries) || maxEntries < 20) maxEntries = 120;
      var chatRows = [];
      var terminalRows = [];
      for (var k = 0; k < normalized.length; k++) {
        var row = normalized[k] || {};
        var role = String(row.role || '').toLowerCase();
        var text = String(row.text || '').trim();
        if (!text) continue;
        if (role === 'user') {
          chatRows.push(text);
          continue;
        }
        var isTerminal = !!row.terminal || role === 'terminal';
        if (!isTerminal) continue;
        var source = String(row.terminal_source || '').toLowerCase();
        if (source && source !== 'user') continue;
        var commands = this.extractTerminalCommandsFromHistoryText(text);
        for (var c = 0; c < commands.length; c++) {
          var command = String(commands[c] || '').trim();
          if (command) terminalRows.push(command);
        }
      }
      if (!chatRows.length && !terminalRows.length) {
        this.chatInputHistory = [];
        this.terminalInputHistory = [];
        this.hydrateInputHistoryFromCache('chat');
        this.hydrateInputHistoryFromCache('terminal');
        this.resetInputHistoryNavigation('chat');
        this.resetInputHistoryNavigation('terminal');
        return;
      }
      chatRows = chatRows.filter(function(text, idx, arr) { return idx === 0 || text !== arr[idx - 1]; });
      terminalRows = terminalRows.filter(function(text, idx, arr) { return idx === 0 || text !== arr[idx - 1]; });
      if (chatRows.length > maxEntries) chatRows = chatRows.slice(chatRows.length - maxEntries);
      if (terminalRows.length > maxEntries) terminalRows = terminalRows.slice(terminalRows.length - maxEntries);


      this.chatInputHistory = chatRows;
      this.terminalInputHistory = terminalRows;
      this.hydrateInputHistoryFromCache('chat', fallbackAgentId);
      this.hydrateInputHistoryFromCache('terminal', fallbackAgentId);
      this.syncInputHistoryToCache('chat', fallbackAgentId);
      this.syncInputHistoryToCache('terminal', fallbackAgentId);
      this.resetInputHistoryNavigation('chat');
      this.resetInputHistoryNavigation('terminal');
    },

    async loadSession(agentId, keepCurrent) {
      var self = this;
      var loadSeq = ++this._sessionLoadSeq;
      this.sessionLoading = true;
      var targetAgentId = String(agentId || '');
      var loadStillCurrent = function() {
        if (self._sessionLoadSeq !== loadSeq) return false;
        if (!self.currentAgent || !self.currentAgent.id) return true;
        return String(self.currentAgent.id || '') === targetAgentId;
      };
      try {
        var preserveFreshInit = self.isFreshInitInProgressFor(agentId);
        var data = await InfringAPI.get('/api/agents/' + agentId + '/session?limit=80');
        if (!loadStillCurrent()) return;
        self.rebuildInputHistoryFromSessionPayload(data);
        if (self.currentAgent && String(self.currentAgent.id || '') === String(agentId || '')) {
          self.applyAgentGitTreeState(self.currentAgent, data || {});
        }
        var normalized = self.mergeModelNoticesForAgent(agentId, self.normalizeSessionMessages(data));
        var shouldApplyAuthoritativeMessages = true;
        var pendingRequest = self._pendingWsRequest && self._pendingWsRequest.agent_id
          ? self._pendingWsRequest
          : null;
        if (pendingRequest && String(pendingRequest.agent_id || '') === String(agentId || '')) {
          var pendingStartedAt = Number(pendingRequest.started_at || 0);
          var observedPendingReply = false;
          if (typeof self._pendingRequestReplyObserved === 'function') {
            observedPendingReply = self._pendingRequestReplyObserved(normalized, pendingRequest, pendingStartedAt);
          }
          if (!observedPendingReply && typeof self._recentAgentReplyObserved === 'function') {
            observedPendingReply = self._recentAgentReplyObserved(normalized, pendingStartedAt);
          }
          if (!observedPendingReply) {
            // Keep optimistic local rows (user prompt + live thinking) visible
            // until authoritative session state catches up for this pending turn.
            shouldApplyAuthoritativeMessages = false;
          }
        }
        if (!loadStillCurrent()) return;
        if (normalized.length) {
          if (!preserveFreshInit) {
            self.freshInitStageToken = Number(self.freshInitStageToken || 0) + 1;
            self.freshInitRevealMenu = false;
            self.showFreshArchetypeTiles = false;
          }
          if (shouldApplyAuthoritativeMessages) {
            // Always prefer server-authoritative session state over potentially stale cache.
            self.messages = normalized;
            self._hasMoreMessages = !!(data && data.has_more);
            self._messagePageOffset = normalized.length;
            self.clearHoveredMessageHard();
            self.recomputeContextEstimate();
            self.cacheAgentConversation(agentId);
            self.$nextTick(function() {
              self.scrollToBottomImmediate();
              self.stabilizeBottomScroll();
              self.pinToLatestOnOpen(null, { maxFrames: 20 });
            });
          } else {
            self.recomputeContextEstimate();
            self.cacheAgentConversation(agentId);
            self._reconcileSendingState();
            self.$nextTick(function() {
              self.scrollToBottom();
              self.stabilizeBottomScroll();
            });
          }
        } else if (!keepCurrent) {
          if (!preserveFreshInit) {
            self.freshInitStageToken = Number(self.freshInitStageToken || 0) + 1;
            self.freshInitRevealMenu = false;
            self.showFreshArchetypeTiles = false;
            self.messages = [];
            self.clearHoveredMessageHard();
            self.recomputeContextEstimate();
            self.recoverEmptySessionRender(agentId, data || null);
          }

        }
      } catch(e) {
        if (!loadStillCurrent()) return;
        var loadErrorText = String(e && e.message ? e.message : 'session_load_failed').trim();
        var lowerLoadError = loadErrorText.toLowerCase();
        if (
          lowerLoadError.indexOf('agent_not_found') >= 0 ||
          lowerLoadError.indexOf('agent not found') >= 0
        ) {
          try {
            var reboundAgent = typeof self.rebindCurrentAgentAuthoritative === 'function'
              ? await self.rebindCurrentAgentAuthoritative({ preferred_id: agentId, clear_when_missing: true })
              : null;
            var reboundId = reboundAgent && reboundAgent.id ? String(reboundAgent.id || '').trim() : '';
            if (reboundId && reboundId !== String(agentId || '').trim() && self._sessionLoadSeq === loadSeq) {
              await self.loadSession(reboundId, keepCurrent);
              return;
            }
          } catch(_) {}
        }
        var restoredFromCache = false;
        try {
          restoredFromCache = self.restoreAgentConversation(agentId);
        } catch(_) {
          restoredFromCache = false;
        }
        if (!restoredFromCache && !keepCurrent && (!Array.isArray(self.messages) || !self.messages.length)) {
          var errText = loadErrorText;
          self.messages = [{
            id: ++msgId,
            role: 'system',
            text: 'Unable to load this agent session right now (' + errText + ').',
            meta: '',
            tools: [],
            system_origin: 'session:load_error',
            is_notice: true,
            notice_label: 'Unable to load this agent session right now (' + errText + ').',
            notice_type: 'error',
            ts: Date.now()
          }];
        }
      }
      finally {
        if (self._sessionLoadSeq === loadSeq) {
          await new Promise(function(resolve) {
            self.$nextTick(function() {
              self.scrollToBottomImmediate();
              self.stabilizeBottomScroll();
              self.pinToLatestOnOpen(null, { maxFrames: 22 });
              self.scheduleMessageRenderWindowUpdate();
              resolve();
            });
          });
          await self.waitForSessionRender(agentId, loadSeq);
          if (self._sessionLoadSeq === loadSeq) {
            self.enforceLatestViewportDeterminism();
            self.pinToLatestOnOpen(null, { maxFrames: 24 });
            self.sessionLoading = false;
          }
          self._reconcileSendingState();
          if (!self.showFreshArchetypeTiles) {
            self.refreshPromptSuggestions(false);
          }
        }
      }
    },

    async loadOlderMessages() {
      var self = this;
      if (!self._hasMoreMessages || self._olderMessagesLoading) return;
      var agentId = self.currentAgent && self.currentAgent.id;
      if (!agentId) return;
      self._olderMessagesLoading = true;
      try {
        var offset = Number(self._messagePageOffset || 0);
        var data = await InfringAPI.get('/api/agents/' + agentId + '/session?limit=80&offset=' + offset);
        if (!data || !data.ok) return;
        var older = self.normalizeSessionMessages(data);
        if (!older.length) {
          self._hasMoreMessages = false;
          return;
        }
        self._hasMoreMessages = !!(data.has_more);
        self._messagePageOffset = offset + older.length;
        var el = self.resolveMessagesScroller(null);
        var prevScrollHeight = el ? el.scrollHeight : 0;
        self.messages = older.concat(Array.isArray(self.messages) ? self.messages : []);
        self.$nextTick(function() {
          if (el) el.scrollTop += (el.scrollHeight - prevScrollHeight);
        });
      } catch(_) {
      } finally {
        self._olderMessagesLoading = false;
      }
    },

    waitForAnimationFrame() {
      return new Promise(function(resolve) {
        if (typeof requestAnimationFrame === 'function') {
          requestAnimationFrame(function() { resolve(); });
        } else {
          setTimeout(resolve, 16);
        }
      });
    },

    async waitForSessionRender(agentId, loadSeq) {
      var self = this;
      var expectedAgent = String(agentId || '');
      var hasSessionMessages = Array.isArray(this.messages) && this.messages.length > 0;
      var minFrames = hasSessionMessages ? 2 : 1;
      var maxFrames = hasSessionMessages ? 42 : 6;
      var messagesEl = null;
      // Let Alpine commit template updates before probing for rendered blocks.
      await this.waitForAnimationFrame();
      await this.waitForAnimationFrame();
      for (var frame = 0; frame < maxFrames; frame++) {
        if (self._sessionLoadSeq !== loadSeq) return;
        if (!self.currentAgent || String(self.currentAgent.id || '') !== expectedAgent) return;
        if (!messagesEl) messagesEl = self.resolveMessagesScroller();
        if (!messagesEl) {
          await self.waitForAnimationFrame();
          continue;
        }
        self.scheduleMessageRenderWindowUpdate(messagesEl);
        if (!hasSessionMessages) {
          if (frame >= minFrames) return;
          await self.waitForAnimationFrame();
          continue;
        }

        var blockCount = messagesEl.querySelectorAll('.chat-message-block').length;
        var renderedCount = messagesEl.querySelectorAll('.chat-message-block .message, .chat-message-block .message-placeholder-shell, .chat-day-anchor, .chat-day-divider').length;
        if (blockCount > 0 && renderedCount > 0 && frame >= minFrames) {
          return;
        }
        await self.waitForAnimationFrame();
      }
    },

    enforceLatestViewportDeterminism() {
      var el = this.resolveMessagesScroller();
      if (!el) return false;
      if (!Array.isArray(this.messages) || this.messages.length < 1) return false;
      var blocks = el.querySelectorAll('.chat-message-block[data-msg-idx], .chat-message-block');
      if (!blocks || !blocks.length) {
        this.scrollToBottomImmediate({ force: true });
        return true;
      }
      var viewportTop = Number(el.scrollTop || 0);
      var viewportBottom = viewportTop + Math.max(0, Number(el.clientHeight || 0));
      var lastBottom = 0;
      for (var i = 0; i < blocks.length; i++) {
        var block = blocks[i];
        if (!block || block.offsetParent === null) continue;
        var bottom = Number(block.offsetTop || 0) + Math.max(0, Number(block.offsetHeight || 0));
        if (bottom > lastBottom) lastBottom = bottom;
      }
      if (!(lastBottom > 0)) return false;
      if (viewportTop > (lastBottom + 24) || viewportBottom < 24) {
        this.scrollToBottomImmediate({ force: true, tolerancePx: 999999 });
        this.stabilizeBottomScroll();
        return true;
      }
      return false;
    },

    ensureLiveThinkingRow: function(data) {
      var incomingStatus = String(
        data && (data.thinking_status || data.status_text) ? (data.thinking_status || data.status_text) : ''
      ).trim();
      if (incomingStatus && typeof this.normalizeThinkingStatusCandidate === 'function') {
        incomingStatus = this.normalizeThinkingStatusCandidate(incomingStatus);
      }
      var row = this.messages.length ? this.messages[this.messages.length - 1] : null;
      if (row && (row.thinking || row.streaming)) {
        row.thinking = true;
        row.streaming = true;
        if (!Number.isFinite(Number(row._stream_started_at))) row._stream_started_at = Date.now();
        row._stream_updated_at = Date.now();
        if (
          incomingStatus &&
          (
            !String(row.thinking_status || '').trim() ||
            (
              typeof this.isThinkingPlaceholderText === 'function' &&
              this.isThinkingPlaceholderText(row.thinking_status)
            )
          )
        ) {
          row.thinking_status = incomingStatus;
        }
        var activeStore = window.InfringChatStore;
        if (activeStore && typeof activeStore.syncMessages === 'function') activeStore.syncMessages(this.messages, this.allFilteredMessages);
        return row;
      }
      row = {
        id: ++msgId,
        role: 'agent',
        text: '',
        meta: '',
        thinking: true,
        streaming: true,
        thinking_status: incomingStatus,
        tools: [],
        _stream_started_at: Date.now(),
        _stream_updated_at: Date.now(),
        ts: Date.now(),
        agent_id: data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : ''),
        agent_name: data && data.agent_name ? String(data.agent_name) : (this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '')
      };
      this.messages.push(row);
      var chatStore = window.InfringChatStore;
      if (chatStore && typeof chatStore.syncMessages === 'function') chatStore.syncMessages(this.messages, this.allFilteredMessages);
      return row;
    },

    // Multi-session: load session list for current agent
    async loadSessions(agentId) {
      try {
        var data = await InfringAPI.get('/api/shell-socket/agents/' + encodeURIComponent(agentId) + '/sessions');
        var normalizedAgentId = typeof this.normalizeSessionAgentId === 'function'
          ? this.normalizeSessionAgentId(agentId)
          : String(agentId || '').trim().toLowerCase();
        var rows = data && Array.isArray(data.sessions) ? data.sessions : [];
        if (typeof this.normalizeSessionsList === 'function') {
          rows = this.normalizeSessionsList(rows, normalizedAgentId);
        }
        this.sessions = rows;
        if (!this._sessionsLastLoadedAtByAgent || typeof this._sessionsLastLoadedAtByAgent !== 'object') {
          this._sessionsLastLoadedAtByAgent = {};
        }
        this._sessionsLastLoadedAtByAgent[normalizedAgentId] = Date.now();
      } catch(e) { this.sessions = []; }
    },

    // Multi-session: create a new session
    async createSession() {
      if (!this.currentAgent) return;
      this.cacheCurrentConversation();
      var label = prompt('Session name (optional):');
      if (label === null) return; // cancelled
      try {
        await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(this.currentAgent.id) + '/sessions', {
          label: label.trim() || undefined
        });
        await this.loadSessions(this.currentAgent.id);
        await this.loadSession(this.currentAgent.id);
        if (typeof InfringToast !== 'undefined') InfringToast.success('New session created');
      } catch(e) {
        if (typeof InfringToast !== 'undefined') InfringToast.error('Failed to create session');
      }
    },

    // Multi-session: switch to an existing session
    async switchSession(sessionId) {
      if (!this.currentAgent) return;
      this.cacheCurrentConversation();
      try {
        await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(this.currentAgent.id) + '/sessions/' + encodeURIComponent(sessionId) + '/switch', {});
        await this.loadSession(this.currentAgent.id);
        await this.loadSessions(this.currentAgent.id);
        // Reconnect WebSocket for new session
        this._wsAgent = null;
        this.connectChatWebSocket(this.currentAgent.id);
      } catch(e) {
        if (typeof InfringToast !== 'undefined') InfringToast.error('Failed to switch session');
      }
    },

    connectChatWebSocket(agentId) {
      var targetAgentId = String(agentId || '').trim();
      if (!targetAgentId) return;
      if (this._wsAgent === targetAgentId && InfringAPI.isWsConnected()) return;
      var connectSeq = Number(this._wsConnectSeq || 0) + 1;
      this._wsConnectSeq = connectSeq;
      this._wsAgent = targetAgentId;
      var self = this;
      var reconnectPending = false;
      var reconnectSyncInFlight = false;
      var isLiveConnection = function(eventAgentId) {
        if (Number(self._wsConnectSeq || 0) !== connectSeq) return false;
        if (String(self._wsAgent || '').trim() !== targetAgentId) return false;
        var eventId = String(eventAgentId || '').trim();
        return !eventId || eventId === targetAgentId;
      };
      var ensurePendingThinkingRow = function(statusText) {
        var nextStatus = String(statusText || '').trim();
        if (typeof self.isThinkingPlaceholderText === 'function' && self.isThinkingPlaceholderText(nextStatus)) {
          nextStatus = '';
        }
        var pendingRow = null;
        var rows = Array.isArray(self.messages) ? self.messages : [];
        for (var i = rows.length - 1; i >= 0; i--) {
          var row = rows[i];
          if (!row) continue;
          if (row.thinking || row.streaming) {
            pendingRow = row;
            break;
          }
          if (String(row.role || '').toLowerCase() === 'agent') break;
        }
        if (!pendingRow) {
          pendingRow = {
            id: ++msgId,
            role: 'agent',
            text: '',
            meta: '',
            thinking: true,
            streaming: true,
            thinking_status: nextStatus,
            tools: [],
            agent_id: targetAgentId,
            agent_name: self.currentAgent && self.currentAgent.name ? String(self.currentAgent.name) : '',
            ts: Date.now(),
          };
          self.messages.push(pendingRow);
        } else {
          pendingRow.thinking = true;
          pendingRow.streaming = true;
          if (!String(pendingRow.text || '').trim()) pendingRow.text = '';
          if (nextStatus && pendingRow.thinking_status !== nextStatus) pendingRow.thinking_status = nextStatus;
          pendingRow._stream_updated_at = Date.now();
        }
        var chatStore = window.InfringChatStore;
        if (chatStore && typeof chatStore.syncMessages === 'function') chatStore.syncMessages(self.messages, self.allFilteredMessages);
      };
      var syncPendingAfterReconnect = function(reason) {
        if (reconnectSyncInFlight) return;
        var pending = self._pendingWsRequest;
        if (!pending || String(pending.agent_id || '').trim() !== targetAgentId) return;
        reconnectSyncInFlight = true;
        ensurePendingThinkingRow('Reconnected. Syncing response...');
        self.setAgentLiveActivity(targetAgentId, 'working', { optimistic: true, source: 'shell_display_hint' });
        Promise.resolve()
          .then(function() {
            return self.loadSessions(targetAgentId);
          })
          .catch(function() { return null; })
          .then(function() {
            var isActive = !!(self.currentAgent && String(self.currentAgent.id || '').trim() === targetAgentId);
            if (!isActive) return null;
            return self.loadSession(targetAgentId, true).catch(function() { return null; });
          })
          .then(function() {
            return self._recoverPendingWsRequest(reason || 'ws_reopen');
          })
          .catch(function() { return null; })
          .finally(function() {
            reconnectSyncInFlight = false;
          });
      };

      InfringAPI.wsConnect(targetAgentId, {
        onOpen: function() {
          if (!isLiveConnection('')) return;
          Alpine.store('app').wsConnected = true;
          var cs = window.InfringChatStore; if (cs && cs.wsConnected) cs.wsConnected.set(true);
          self.requestContextTelemetry(true);
          if (reconnectPending) {
            reconnectPending = false;
            syncPendingAfterReconnect('ws_reopen');
          } else if (!self.sending) {
            self.$nextTick(function() { self._processQueue(); });
          }
        },
        onMessage: function(data) {
          var dataAgentId = data && data.agent_id ? data.agent_id : '';
          if (!isLiveConnection(dataAgentId)) return;
          self.handleChatWebSocketMessage(data);
        },
        onReconnect: function() {
          if (!isLiveConnection('')) return;
          Alpine.store('app').wsConnected = false;
          var cs = window.InfringChatStore; if (cs && cs.wsConnected) cs.wsConnected.set(false);
          reconnectPending = true;
          var pending = self._pendingWsRequest;
          if (pending && pending.agent_id) {
            ensurePendingThinkingRow('Connection interrupted. Reconnecting...');
            self.setAgentLiveActivity(pending.agent_id, 'working', { optimistic: true, source: 'shell_display_hint' });
          }
        },
        onClose: function() {
          if (!isLiveConnection('')) return;
          Alpine.store('app').wsConnected = false;
          var cs = window.InfringChatStore; if (cs && cs.wsConnected) cs.wsConnected.set(false);
          self._wsAgent = null;
          var pending = self._pendingWsRequest;
          if (self.sending && pending && pending.agent_id) {
            reconnectPending = true;
            self._clearTypingTimeout();
            ensurePendingThinkingRow('Connection interrupted. Reconnecting...');
            self.setAgentLiveActivity(pending.agent_id, 'working', { optimistic: true, source: 'shell_display_hint' });
            self._recoverPendingWsRequest('ws_close');
            self.scrollToBottom();
          }
          if (self.currentAgent && self.currentAgent.id) {
            Alpine.store('app').refreshAgents().then(function() {
              var stillLive = self.resolveAgent(self.currentAgent.id);
              if (!stillLive && !self.shouldSuppressAgentInactive(self.currentAgent.id)) {
                self.handleAgentInactive(self.currentAgent.id, 'inactive');
              }
            }).catch(function() {});
          }
        },
        onError: function() {
          if (!isLiveConnection('')) return;
          Alpine.store('app').wsConnected = false;
          var cs = window.InfringChatStore; if (cs && cs.wsConnected) cs.wsConnected.set(false);
          self._wsAgent = null;
          var pending = self._pendingWsRequest;
          if (self.sending && pending && pending.agent_id) {
            reconnectPending = true;
            self._clearTypingTimeout();
            ensurePendingThinkingRow('Connection interrupted. Reconnecting...');
            self.setAgentLiveActivity(pending.agent_id, 'working', { optimistic: true, source: 'shell_display_hint' });
            self._recoverPendingWsRequest('ws_error');
            self.scrollToBottom();
          }
        }
      });
    },

    // Backward-compat shim for legacy callers during naming migration.
    connectWs(agentId) {
      this.connectChatWebSocket(agentId);
    },

    formatInactiveReason: function(reason) {
      var raw = String(reason || '').trim();
      if (!raw) return 'inactive';
      raw = raw.replace(/^agent_contract_/, '');
      raw = raw.replace(/^rogue_/, '');
      raw = raw.replace(/_/g, ' ').trim();
      return raw || 'inactive';
    },

    handleAgentInactive: function(agentId, reason, options) {
      var opts = options || {};
      var targetId = String(agentId || (this.currentAgent && this.currentAgent.id) || '').trim();
      if (
        (targetId && this.isSystemThreadId && this.isSystemThreadId(targetId)) ||
        (!targetId && this.isSystemThreadActive && this.isSystemThreadActive())
      ) {
        if (!this.currentAgent || !this.isSystemThreadAgent || !this.isSystemThreadAgent(this.currentAgent)) {
          this.activateSystemThread({ preserve_if_empty: true });
        } else {
          this.currentAgent = this.makeSystemThreadAgent();
          this.setStoreActiveAgentId(this.currentAgent.id || null);
        }
        return;
      }
      if (!opts.force && this.shouldSuppressAgentInactive(targetId)) {
        return;
      }
      var reasonLabel = this.formatInactiveReason(reason || 'inactive');
      var noticeKey = targetId + '|' + reasonLabel;
      var self = this;

      this._clearTypingTimeout();
      this._clearPendingWsRequest(targetId);
      typeof this.clearTransientThinkingRows === 'function' ? this.clearTransientThinkingRows({ force: true }) : (this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; }));
      this.sending = false;
      this._responseStartedAt = 0;
      this.tokenCount = 0;
      this._inflightPayload = null;
      this.setAgentLiveActivity(targetId || (this.currentAgent && this.currentAgent.id ? this.currentAgent.id : ''), 'idle', { optimistic: true, source: 'shell_display_hint' });

      if (!opts.silentNotice && noticeKey !== this._lastInactiveNoticeKey) {
        var noticeText = opts.noticeText || '';
        if (!noticeText) {
          noticeText = targetId
            ? ('Agent ' + targetId + ' is now inactive (' + reasonLabel + ').')
            : ('Agent is now inactive (' + reasonLabel + ').');
        }
        this.messages.push({ id: ++msgId, role: 'system', text: noticeText, meta: '', tools: [], system_origin: 'agent:inactive', is_notice: true, notice_label: noticeText, notice_type: 'info', ts: Date.now() });
        this._lastInactiveNoticeKey = noticeKey;
      }

      if (targetId && this._wsAgent && String(this._wsAgent) === targetId) {
        InfringAPI.wsDisconnect();
        this._wsAgent = null;
      }

      if (this.currentAgent && this.currentAgent.id && (!targetId || String(this.currentAgent.id) === targetId)) {
        this.currentAgent = null;
        this.setStoreActiveAgentId(null);
        this.showAgentDrawer = false;
      }

      this.scrollToBottom();
      this.$nextTick(function() { self._processQueue(); });

      try { Alpine.store('app').refreshAgents(); } catch(_) {}
    },

    setAgentLiveActivity(agentId, state, options) {
      var id = String(agentId || '').trim();
      if (!id) return;
      var opts = options && typeof options === 'object' ? options : { optimistic: true, source: 'shell_display_hint' };
      try {
        var store = Alpine.store('app');
        if (store && typeof store.setAgentLiveActivity === 'function') {
          store.setAgentLiveActivity(id, state, opts);
        }
      } catch(_) {}
    },

    handleStopResponse: function(agentId, payload) {
      var result = payload && typeof payload === 'object' ? payload : {};
      var reasonRaw = String(result.reason || result.error || '').trim();
      var reason = reasonRaw || (result.contract_terminated ? 'contract_terminated' : '');
      var state = String(result.state || '').trim().toLowerCase();
      var reasonLower = reason.toLowerCase();
      var isInactive =
        !!result.archived ||
        !!result.contract_terminated ||
        state === 'inactive' ||
        state === 'archived' ||
        state === 'terminated' ||
        String(result.type || '').toLowerCase() === 'agent_archived' ||
        reasonLower.indexOf('inactive') >= 0 ||
        reasonLower.indexOf('terminated') >= 0;

      if (isInactive) {
        this.handleAgentInactive(
          agentId,
          reason || (result.contract_terminated ? 'contract_terminated' : 'inactive'),
          result.message ? { noticeText: String(result.message) } : {}
        );
        return;
      }

      this.setAgentLiveActivity(agentId || (this.currentAgent && this.currentAgent.id ? this.currentAgent.id : ''), 'idle', { optimistic: true, source: 'shell_display_hint' });
      this._clearTypingTimeout();
      typeof this.clearTransientThinkingRows === 'function' ? this.clearTransientThinkingRows({ force: true }) : (this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; }));
      this.messages.push({ id: ++msgId, role: 'system', text: result.message || 'Run cancelled', meta: '', tools: [], system_origin: 'agent:stop', is_notice: true, notice_label: result.message || 'Run cancelled', notice_type: 'info', ts: Date.now() });
      this.sending = false;
      this._responseStartedAt = 0;
      this.tokenCount = 0;
      this.scrollToBottom();
      var self = this;
      this.$nextTick(function() { self._processQueue(); });
      try { Alpine.store('app').refreshAgents(); } catch(_) {}
    },

    // Preferred naming for websocket event entrypoint.
    handleChatWebSocketMessage(data) {
      this.handleWsMessage(data);
    },

    // Backward-compat websocket event entrypoint.
    handleWsMessage(data) {
      var eventAgentId = String(data && data.agent_id ? data.agent_id : '').trim();
      var activeWsAgentId = String(this._wsAgent || '').trim();
      if (eventAgentId && activeWsAgentId && eventAgentId !== activeWsAgentId) {
        return;
      }
      switch (data.type) {
        case 'connected':
          var connectedAgentId = String(data && data.agent_id ? data.agent_id : '').trim();
          if (connectedAgentId) {
            var selectedAgentId = String(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '').trim();
            if (activeWsAgentId && connectedAgentId !== activeWsAgentId) break;
            if (selectedAgentId && connectedAgentId !== selectedAgentId) break;
            var connectedLive = this.resolveAgent(connectedAgentId);
            if (connectedLive) {
              this.currentAgent = this.applyAgentGitTreeState(connectedLive, connectedLive) || connectedLive;
              this.setStoreActiveAgentId(connectedAgentId);
            } else {
              var selfConnected = this;
              Promise.resolve()
                .then(function() {
                  return selfConnected.rebindCurrentAgentAuthoritative({
                    preferred_id: connectedAgentId,
                    clear_when_missing: false,
                  });
                })
                .catch(function() {});
            }
          }
          break;

        case 'context_state':
          this.applyContextTelemetry(data);
          break;

        // Legacy thinking event (backward compat)
        case 'thinking':
          if (!this.messages.length || !this.messages[this.messages.length - 1].thinking) {
            this.ensureLiveThinkingRow(data);
            this.scrollToBottom();
            this._resetTypingTimeout();
          }
          break;

        // New typing lifecycle
        case 'typing':
          if (typeof this.shouldReloadHistoryForFinalEventPayload === 'function' && this.shouldReloadHistoryForFinalEventPayload(data)) {
            var finalAgentId = String((data && data.agent_id) || (this.currentAgent && this.currentAgent.id) || '').trim();
            var canReloadFinalSnapshot =
              !!finalAgentId &&
              !this.sending &&
              !(typeof this.hasLivePendingResponse === 'function' && this.hasLivePendingResponse()) &&
              !(typeof this._hasActiveTypewriterVisual === 'function' && this._hasActiveTypewriterVisual());
            if (canReloadFinalSnapshot) {
              var selfFinal = this;
              Promise.resolve()
                .then(function() { return selfFinal.loadSessions(finalAgentId); })
                .catch(function() { return []; })
                .then(function() { return selfFinal.loadSession(finalAgentId, true).catch(function() { return null; }); });
            }
          }
          if (data.state === 'start') {
            this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'typing', { optimistic: true, source: 'shell_display_hint' });
            if (!this.messages.length || !this.messages[this.messages.length - 1].thinking) {
              this.ensureLiveThinkingRow(data);
              this.scrollToBottom();
            }
            this._resetTypingTimeout();
          } else if (data.state === 'tool') {
            this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'working', { optimistic: true, source: 'shell_display_hint' });
            var typingMsg = this.messages.length ? this.messages[this.messages.length - 1] : null;
            if (typingMsg && (typingMsg.thinking || typingMsg.streaming)) {
              typingMsg.text = '';
              if (typeof this.isThinkingPlaceholderText === 'function' && this.isThinkingPlaceholderText(typingMsg.thinking_status)) {
                typingMsg.thinking_status = '';
              }
            }
            this._resetTypingTimeout();
          } else if (data.state === 'stop') {
            var stillPending = (this.sending === true)
              || (typeof this.hasLivePendingResponse === 'function' && this.hasLivePendingResponse());
            if (stillPending) {
              if (typeof this.ensureLiveThinkingRow === 'function') {
                var pendingMsg = this.ensureLiveThinkingRow(data);
                if (pendingMsg) {
                  pendingMsg.thinking = true;
                  pendingMsg.streaming = true;
                  pendingMsg._stream_updated_at = Date.now();
                  if (!Number.isFinite(Number(pendingMsg._stream_started_at))) {
                    pendingMsg._stream_started_at = Date.now();
                  }
                }
              }
              this._resetTypingTimeout();
            } else this._clearTypingTimeout();
          }
          break;

        case 'phase':
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'working', { optimistic: true, source: 'shell_display_hint' });
          // Show tool/phase progress so the user sees the agent is working
          var phaseMsg = this.ensureLiveThinkingRow(data);
          if (phaseMsg && (phaseMsg.thinking || phaseMsg.streaming)) {
            var phaseThoughtText = String(data && data.detail ? data.detail : '').trim();
            var phasePercent = Number(
              data && data.progress_percent != null

                ? data.progress_percent
                : (data && data.percent != null ? data.percent : NaN)
            );
	            if (Number.isFinite(phasePercent)) {
	              phaseMsg.progress = {
	                percent: Math.max(0, Math.min(100, Math.round(phasePercent))),
	                label: data && data.phase ? ('Progress · ' + String(data.phase)) : 'Progress'
	              };
	            }
	            phaseMsg._stream_updated_at = Date.now();
	            if (!Number.isFinite(Number(phaseMsg._stream_started_at))) {
	              phaseMsg._stream_started_at = Date.now();
	            }
	            var phaseStatusCandidate = String((data && (data.thinking_status || data.status_text || data.workflow_stage || data.stage)) || phaseDetailText || '').trim();
            var phaseKey = String(data && data.phase ? data.phase : '').trim().toLowerCase();
            if (!phaseStatusCandidate && phaseKey) {
              phaseStatusCandidate = phaseKey.replace(/[_-]+/g, ' ').trim();
            }
            if (typeof this.normalizeThinkingStatusCandidate === 'function') {
              phaseStatusCandidate = this.normalizeThinkingStatusCandidate(phaseStatusCandidate);
            }
            if (typeof this.isThinkingPlaceholderText === 'function' && this.isThinkingPlaceholderText(phaseStatusCandidate)) {
              phaseStatusCandidate = '';
            }
            var phaseCurrentStatus = String(phaseMsg.thinking_status || '').trim();
            var phaseCanReplaceStatus = !!phaseStatusCandidate && (
              !phaseCurrentStatus ||
              (typeof this.isThinkingPlaceholderText === 'function' && this.isThinkingPlaceholderText(phaseCurrentStatus))
            );
            var phaseFingerprint = phaseKey + '|' + phaseDetailText + '|' + phaseStatusCandidate + '|' + (Number.isFinite(phasePercent) ? String(Math.round(phasePercent)) : '');
            if (phaseMsg._phase_update_fingerprint === phaseFingerprint) {
              phaseMsg._stream_updated_at = Date.now();
              this._resetTypingTimeout();
              this.scrollToBottom();
              break;
            }
            phaseMsg._phase_update_fingerprint = phaseFingerprint;
            // Skip phases that have no user-meaningful display text — "streaming"
            // and "done" are lifecycle signals, not status to show in the chat bubble.
            if (phaseKey === 'streaming' || phaseKey === 'done') {
              break;
            }
            if (phaseStatusCandidate && typeof this._setPendingWsStatusText === 'function') {
              this._setPendingWsStatusText(data && data.agent_id ? String(data.agent_id) : '', phaseStatusCandidate);
            }
            // Context warning: show prominently as a separate system message
            if (phaseKey === 'context_warning') {
              var cwDetail = data.detail || 'Context limit reached.';
              this.messages.push({ id: ++msgId, role: 'system', is_notice: true, notice_type: 'warning', notice_label: String(cwDetail || 'Context window warning'), text: cwDetail, meta: '', tools: [], system_origin: 'context:warning' });
              if (phaseMsg.thinking_status !== 'Context window warning') phaseMsg.thinking_status = 'Context window warning';
            } else if (
              phaseKey === 'thinking' ||
              phaseKey === 'reasoning' ||
              phaseKey === 'analysis' ||
              phaseKey === 'planning' ||
              phaseKey === 'plan'
            ) {
              var thoughtChunk = String(data.detail || '').trim();
              if (thoughtChunk && typeof this.normalizeThinkingStatusCandidate === 'function') {
                thoughtChunk = this.normalizeThinkingStatusCandidate(thoughtChunk);
              }
              if (thoughtChunk) {
                var chunkChanged = phaseMsg._thought_latest_chunk !== thoughtChunk;
                phaseMsg._thought_latest_chunk = thoughtChunk;
                if (chunkChanged) {
                  phaseMsg._thoughtText = this.appendThoughtChunk(phaseMsg._thoughtText, thoughtChunk);
                  phaseMsg._reasoning = phaseMsg._thoughtText;
                  if (typeof this.appendNativeWorkflowActivityToThinkingRow === 'function') {
                    this.appendNativeWorkflowActivityToThinkingRow(phaseMsg, {
                      activity_kind: 'decision_dialog',
                      provider_event_type: 'native.workflow.' + (phaseKey || 'reasoning'),
                      status: 'running',
                      display_text: thoughtChunk,
                      text: thoughtChunk,
                      item_id: 'native-workflow-dialog-' + (phaseKey || 'reasoning')
                    });
                  }
                  phaseMsg.isHtml = true;
                  phaseMsg.thoughtStreaming = true;
                  phaseMsg.text = this.renderLiveThoughtHtml(phaseMsg._thoughtText, phaseMsg);
                }
                if (typeof this._setPendingWsStatusText === 'function') {
                  this._setPendingWsStatusText(data && data.agent_id ? String(data.agent_id) : '', phaseStatusCandidate || thoughtChunk);
                }
                if (phaseCanReplaceStatus) {
                  if (phaseMsg.thinking_status !== phaseStatusCandidate) phaseMsg.thinking_status = phaseStatusCandidate;
                }
              }
            } else if (phaseMsg.thinking) {
              if (phaseStatusCandidate && typeof this.appendNativeWorkflowActivityToThinkingRow === 'function') {
                this.appendNativeWorkflowActivityToThinkingRow(phaseMsg, {
                  activity_kind: 'runtime_activity',
                  provider_event_type: 'native.workflow.phase',
                  status: 'running',
                  display_text: phaseStatusCandidate,
                  text: phaseStatusCandidate,
                  item_id: 'native-workflow-phase-' + (phaseKey || 'status')
                });
              }
              if (phaseStatusCandidate && phaseMsg.thinking_status !== phaseStatusCandidate) phaseMsg.thinking_status = phaseStatusCandidate;
            }
            if (phaseStatusCandidate && phaseMsg.status_text !== phaseStatusCandidate) phaseMsg.status_text = phaseStatusCandidate;
            if (phaseCanReplaceStatus) {
              if (phaseMsg.thinking_status !== phaseStatusCandidate) phaseMsg.thinking_status = phaseStatusCandidate;
            }
	          }
	          this._resetTypingTimeout();
	          this.scrollToBottom();
	          break;

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

              text: errorText,
              meta: '',
              tools: [],
              system_origin: 'runtime:error',
              ts: Date.now(),
              dedupe_window_ms: 12000
            });
            self2._inflightPayload = null;
            self2.scrollToBottom();
            self2.$nextTick(function() {
              var el = document.getElementById('msg-input'); if (el) el.focus();
              self2._processQueue();
            });
            self2.refreshPromptSuggestions(true, 'post-error');
          });
          break;

        case 'agent_archived':
          this.setAgentLiveActivity(
            data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? this.currentAgent.id : ''),
            'idle'
          );
          this._clearPendingWsRequest(data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? this.currentAgent.id : ''));
          this.handleAgentInactive(
            data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? this.currentAgent.id : ''),
            data && data.reason ? String(data.reason) : 'archived'
          );
          break;

        case 'agents_updated':
          if (data.agents) {
            var nextAgents = (Array.isArray(data.agents) ? data.agents : []).filter((row) => {
              if (!row || !row.id) return false;
              return !(this.isArchivedAgentRecord && this.isArchivedAgentRecord(row));
            });
            Alpine.store('app').agents = nextAgents;
            Alpine.store('app').agentCount = nextAgents.length;
          }
          break;

        case 'command_result':
          if (typeof this.appendChatSideResultNotice === 'function' && this.appendChatSideResultNotice(data)) {
            break;
          }
          this.applyContextTelemetry(data);
          var isContextTelemetryResult = Object.prototype.hasOwnProperty.call(data || {}, 'context_tokens') ||
            Object.prototype.hasOwnProperty.call(data || {}, 'context_window') ||
            Object.prototype.hasOwnProperty.call(data || {}, 'context_ratio') ||
            Object.prototype.hasOwnProperty.call(data || {}, 'context_pressure');
          if (!data.silent && !isContextTelemetryResult) {
            this.messages.push({ id: ++msgId, role: 'system', is_notice: true, notice_type: 'info', notice_label: data.message || 'Command executed.', text: data.message || 'Command executed.', meta: '', tools: [], system_origin: 'command:result' });
            this.scrollToBottom();
          }
          break;

        case 'terminal_output':
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
          this._clearTypingTimeout();
          this.messages = this.messages.filter(function(m) { return !(m && m.terminal && m.thinking); });
          var stdout = typeof data.stdout === 'string' ? data.stdout : '';
          var stderr = typeof data.stderr === 'string' ? data.stderr : '';
          var cleanStdout = stdout.replace(/\r\n/g, '\n').replace(/\r/g, '\n').replace(/^(?:[ \t]*\n)+|(?:\n[ \t]*)+$/g, '');
          var cleanStderr = stderr.replace(/\r\n/g, '\n').replace(/\r/g, '\n').replace(/^(?:[ \t]*\n)+|(?:\n[ \t]*)+$/g, '');
          var termText = '';
          if (cleanStdout.trim()) termText += cleanStdout;
          if (cleanStderr.trim()) termText += (termText ? '\n' : '') + cleanStderr;
          if (!termText.trim()) termText = '(no output)';
          var termMeta = 'exit ' + (Number.isFinite(Number(data.exit_code)) ? String(Number(data.exit_code)) : '1');
          var termDuration = this.formatResponseDuration(Number(data.duration_ms || 0));
          if (termDuration) termMeta += ' | ' + termDuration;
          var termCwd = this.terminalPromptPath;
          if (data.cwd) {
            termCwd = String(data.cwd);
            this.terminalCwd = termCwd;
            termMeta += ' | ' + termCwd;
          }
          var invokedBy = data && data.terminal_source ? String(data.terminal_source).trim().toLowerCase() : '';
          if (invokedBy === 'assistant') invokedBy = 'agent';
          if (invokedBy !== 'user' && invokedBy !== 'agent') invokedBy = '';
          var invokedCommand = String(
            (data && (data.command || data.requested_command || data.executed_command)) || ''
          ).trim();
          if (invokedBy === 'agent' && invokedCommand) {
            this._appendTerminalMessage({
              role: 'terminal',
              text: this._terminalPromptLine(termCwd, invokedCommand),
              meta: termCwd,
              tools: [],
              ts: Date.now(),
              terminal_source: 'agent',
              cwd: termCwd,
              agent_id: data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : ''),
              agent_name: data && data.agent_name ? String(data.agent_name) : (this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '')
            });
          }
          this._appendTerminalMessage({
            role: 'terminal',
            text: termText,
            meta: termMeta,
            tools: [],
            ts: Date.now(),
            terminal_source: 'system',
            cwd: termCwd,
            agent_id: data && data.agent_id ? String(data.agent_id) : '',
            agent_name: data && data.agent_name ? String(data.agent_name) : ''
          });
          this.sending = false;
          this._responseStartedAt = 0;
          this.scrollToBottom();
          this.$nextTick(() => this._processQueue());
          this.refreshPromptSuggestions(true, 'post-terminal');
          break;

        case 'terminal_error':
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
          this._clearTypingTimeout();
          this.messages = this.messages.filter(function(m) { return !(m && m.terminal && m.thinking); });
          var terminalErrorSource = data && data.terminal_source ? String(data.terminal_source).trim().toLowerCase() : '';
          if (terminalErrorSource === 'assistant') terminalErrorSource = 'agent';
          if (terminalErrorSource !== 'user' && terminalErrorSource !== 'agent') terminalErrorSource = '';
          var terminalErrorCommand = String(
            (data && (data.command || data.requested_command || data.executed_command)) || ''
          ).trim();
          var terminalErrorCwd = data && data.cwd ? String(data.cwd) : this.terminalPromptPath;
          if (terminalErrorSource === 'agent' && terminalErrorCommand) {
            this._appendTerminalMessage({
              role: 'terminal',
              text: this._terminalPromptLine(terminalErrorCwd, terminalErrorCommand),
              meta: terminalErrorCwd,
              tools: [],
              ts: Date.now(),
              terminal_source: 'agent',
              cwd: terminalErrorCwd,
              agent_id: data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : ''),
              agent_name: data && data.agent_name ? String(data.agent_name) : (this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '')
            });
          }
          this._appendTerminalMessage({
            role: 'terminal',
            text: 'Terminal error: ' + (data && data.message ? data.message : 'command failed'),
            meta: '',
            tools: [],
            ts: Date.now(),
            terminal_source: 'system',
            cwd: terminalErrorCwd
          });
          this.sending = false;
          this._responseStartedAt = 0;
          this.scrollToBottom();
          this.$nextTick(() => this._processQueue());
          break;

        case 'canvas':
          // Agent presented an interactive canvas — render it in an iframe sandbox
          var canvasHtml = '<div class="canvas-panel" style="border:1px solid var(--border);border-radius:8px;margin:8px 0;overflow:hidden;">';
          canvasHtml += '<div style="padding:6px 12px;background:var(--surface);border-bottom:1px solid var(--border);font-size:0.85em;display:flex;justify-content:space-between;align-items:center;">';
          canvasHtml += '<span>' + (data.title || 'Canvas') + '</span>';
          canvasHtml += '<span style="opacity:0.5;font-size:0.8em;">' + (data.canvas_id || '').substring(0, 8) + '</span></div>';
          canvasHtml += '<iframe sandbox="allow-scripts" srcdoc="' + (data.html || '').replace(/"/g, '&quot;') + '" ';
          canvasHtml += 'style="width:100%;min-height:300px;border:none;background:#fff;" loading="lazy"></iframe></div>';
          this.messages.push({ id: ++msgId, role: 'agent', text: canvasHtml, meta: 'canvas', isHtml: true, tools: [] });
          this.scrollToBottom();
          break;

        case 'pong': break;
      }
      var activeStore = window.InfringChatStore;
      if (activeStore && typeof activeStore.syncMessages === 'function' && data && data.type !== 'connected' && data.type !== 'context_state' && data.type !== 'pong') activeStore.syncMessages(this.messages, this.allFilteredMessages);
      this.scheduleConversationPersist();
    },

    truncateToolSummaryText: function(value, maxLen) {
      var text = String(value || '').replace(/\s+/g, ' ').trim();
      var limit = Number(maxLen || 0) || 160;
      if (!text) return '';
      if (text.length <= limit) return text;
      return text.slice(0, Math.max(0, limit - 1)).trim() + '…';
    },

    toolSummaryModelLabel: function(summary) {
      var row = summary && typeof summary === 'object' ? summary : {};
      var provider = String(row.provider || row.model_provider || '').trim();
      var model = String(row.model || row.runtime_model || row.model_name || row.selected_model || '').trim();
      if (!provider && !model) return '';
      if (!provider) return model;
      if (!model) return provider;
      if (model.toLowerCase().indexOf((provider + '/').toLowerCase()) === 0) return model;
      return provider + '/' + model;
    },

    toolSummaryFallbackLines: function(summary) {
      var row = summary && typeof summary === 'object' ? summary : {};
      var lines = [];
      var seen = {};
      var addLine = function(text) {
        var next = String(text || '').trim();
        if (!next) return;
        if (seen[next]) return;
        seen[next] = true;
        lines.push(next);
      };
      var attemptSummaries = Array.isArray(row.fallback_attempt_summaries) ? row.fallback_attempt_summaries : [];
      for (var i = 0; i < attemptSummaries.length && lines.length < 3; i += 1) {
        addLine('Fallback: ' + this.truncateToolSummaryText(attemptSummaries[i], 140));
      }
      var attempts = Array.isArray(row.fallback_attempts) ? row.fallback_attempts : [];
      for (var j = 0; j < attempts.length && lines.length < 3; j += 1) {
        var attempt = attempts[j] && typeof attempts[j] === 'object' ? attempts[j] : {};
        var provider = String(attempt.provider || '').trim();
        var model = String(attempt.model || '').trim();
        var reason = String(attempt.reason || attempt.code || attempt.error || '').trim().replace(/_/g, ' ');
        var label = provider && model ? (provider + '/' + model) : (model || provider);
        if (!label) continue;
        addLine('Fallback: ' + label + (reason ? ' (' + this.truncateToolSummaryText(reason, 64) + ')' : ''));
      }
      return lines;
    },

    toolSummaryOutputPreview: function(summary, fallbackText) {
      var row = summary && typeof summary === 'object' ? summary : {};
      var preview = String(row.output_preview || row.preview || row.result_preview || '').trim();
      if (!preview) preview = String(fallbackText || '').trim();
      if (!preview || preview === '(no output)') return '';
      return this.truncateToolSummaryText(preview, 180);
    },

    // Format timestamp for display
    formatClockTime: function(ts) {
      if (!ts) return '';
      var d = new Date(ts);
      if (Number.isNaN(d.getTime())) return '';
      var h = d.getHours();
      var m = d.getMinutes();
      var ampm = h >= 12 ? 'PM' : 'AM';
      h = h % 12 || 12;
      return h + ':' + (m < 10 ? '0' : '') + m + ' ' + ampm;
    },

    // Backward-compat shim for legacy callers during naming migration.
    formatTime: function(ts) {
      return this.formatClockTime(ts);
    },

    isSameDay: function(a, b) {
      if (!a || !b) return false;
      return (
        a.getFullYear() === b.getFullYear() &&
        a.getMonth() === b.getMonth() &&
        a.getDate() === b.getDate()
      );
    },

    // UI-safe timestamp formatter for templates
    messageTimestampLabel: function(msg) {
      if (!msg || !msg.ts) return '';
      var ts = new Date(msg.ts);
      if (Number.isNaN(ts.getTime())) return '';
      var now = new Date();
      if (this.isSameDay(ts, now)) return this.formatClockTime(ts);
      var yesterday = new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1);
      if (this.isSameDay(ts, yesterday)) {
        return 'Yesterday at ' + this.formatClockTime(ts);
      }
      var dateText = ts.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
      return dateText + ' at ' + this.formatClockTime(ts);
    },

    // Backward-compat shim for legacy callers during naming migration.
    messageTs: function(msg) {
      return this.messageTimestampLabel(msg);
    },

    parseProgressFromText: function(text) {
      var value = String(text || '');
      if (!value) return null;
      var explicit = value.match(/\[\[\s*progress\s*:\s*([0-9]{1,3})(?:\s*\/\s*([0-9]{1,3}))?\s*\]\]/i);
      if (explicit) {
        var part = Number(explicit[1] || 0);
        var total = Number(explicit[2] || 100);
        if (Number.isFinite(part) && Number.isFinite(total) && total > 0) {
          var pct = Math.max(0, Math.min(100, Math.round((part / total) * 100)));
          return { percent: pct, label: 'Progress ' + pct + '%' };
        }
      }
      var percent = value.match(/\bprogress(?:\s+is)?\s*[:=-]?\s*([0-9]{1,3})\s*%/i);
      if (percent) {
        var p = Number(percent[1] || 0);
        if (Number.isFinite(p)) {
          var clamped = Math.max(0, Math.min(100, Math.round(p)));
          return { percent: clamped, label: 'Progress ' + clamped + '%' };
        }
      }
      return null;
    },

    messageProgress: function(msg) {
      if (!msg || msg.terminal || msg.is_notice) return null;
      var key = String(msg.id || '') + '|' + String(msg.text || '').length + '|' + String(msg.meta || '').length;
      if (!this._progressCache || typeof this._progressCache !== 'object') this._progressCache = {};
      var keys = Object.keys(this._progressCache);
      if (keys.length > 4096) {
        this._progressCache = {};
      }
      if (Object.prototype.hasOwnProperty.call(this._progressCache, key)) return this._progressCache[key];

      var progress = null;
      if (msg.progress && typeof msg.progress === 'object') {
        var pct = Number(msg.progress.percent);
        if (Number.isFinite(pct)) {
          progress = {
            percent: Math.max(0, Math.min(100, Math.round(pct))),
            label: String(msg.progress.label || ('Progress ' + Math.round(pct) + '%')).trim()
          };
        }
      }
      if (!progress) progress = this.parseProgressFromText(msg.text || '');
      this._progressCache[key] = progress;
      return progress;
    },

    progressFillStyle: function(msg) {
      var progress = this.messageProgress(msg);
      if (!progress) return 'width:0%';
      return 'width:' + progress.percent + '%';
    },

    messageDomId: function(msg, idx) {
      var suffix = (msg && msg.id != null) ? String(msg.id) : String(idx || 0);
      return 'chat-msg-' + suffix;
    },

    messageRenderKey: function(msg, idx) {
      var idPart = (msg && msg.id != null) ? String(msg.id) : '';
      var tsPart = (msg && msg.ts != null) ? String(msg.ts) : '';
      var rolePart = String((msg && msg.role) || '');
      var noticePart = msg && msg.is_notice ? 'notice' : 'message';
      if (msg && (msg.thinking || msg.streaming || msg._typingVisual || msg._typewriterRunning)) {
        return noticePart + '|' + idPart + '|' + tsPart + '|' + rolePart + '|' + String(idx || 0) + '|live';
      }
      var textLen = (msg && typeof msg.text === 'string') ? msg.text.length : 0;
      return noticePart + '|' + idPart + '|' + tsPart + '|' + rolePart + '|' + String(idx || 0) + '|' + String(textLen);
    },

    messageRoleClass: function(msg) {
      if (msg && msg.terminal) {
        var source = this.terminalMessageSource(msg);
        if (source === 'user') return 'terminal terminal-user';
        if (source === 'agent') return 'terminal terminal-agent';
        return 'terminal terminal-system';
      }
      if (!msg || !msg.role) return 'agent';
      return String(msg.role);
    },

    terminalMessageSource: function(msg) {
      if (!msg || !msg.terminal) return 'agent';
      var source = String(msg.terminal_source || '').trim().toLowerCase();
      if (source === 'user' || source === 'agent' || source === 'system') return source;
      if (source === 'assistant') return 'agent';
      return 'system';
    },

    terminalToolboxSideClass: function(msg) {
      return this.terminalMessageSource(msg) === 'user' ? 'terminal-toolbox-right' : 'terminal-toolbox-left';
    },

    expandTerminalMessage: function(msg, idx, rows) {
      if (!msg || !msg.terminal || msg.thinking) return;
      if (msg._terminal_expanded === true) return;
      if (!this.terminalMessageCollapsed(msg, idx, rows)) return;
      msg._terminal_expanded = true;
      this.scheduleConversationPersist();
      this.$nextTick(() => {
        this.scheduleMessageRenderWindowUpdate();
        this.stabilizeBottomScroll();
      });
    },

    terminalMessageCollapsed: function(msg, idx, rows) {
      if (!msg || !msg.terminal || msg.thinking) return false;
      if (msg._terminal_compact !== true) return false;
      if (msg._terminal_expanded === true) return false;
      var list = Array.isArray(rows) ? rows : this.messages;
      if (!Array.isArray(list) || idx < 0 || idx >= list.length) return false;
      for (var i = idx + 1; i < list.length; i++) {
        var row = list[i];
        if (!row || row.is_notice || row.terminal || row.thinking) continue;
        var hasText = typeof row.text === 'string' && row.text.trim().length > 0;
        var hasTools = Array.isArray(row.tools) && row.tools.length > 0;
        var hasArtifact = !!(row.file_output || row.folder_output);
        if (hasText || hasTools || hasArtifact) return true;
      }
      return false;
    },

    terminalToolboxPreview: function(msg) {
      if (!msg || !msg.terminal) return '';
      var text = String(msg.text || '').trim();
      if (!text) return 'Command completed';
      var first = text.split('\n')[0] || '';
      var compact = first.replace(/\s+/g, ' ').trim();
      if (!compact) return 'Command completed';
      if (compact.length > 108) return compact.slice(0, 105) + '...';
      return compact;
    },


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

    messageGroupRole: function(msg) {
      if (!msg) return '';
      if (msg.terminal) return 'terminal';
      return String(msg.role || '');
    },

    messageOriginKind: function(msg) {
      if (!msg || typeof msg !== 'object') return 'other';
      if (msg.terminal) {
        var terminalSource = typeof this.terminalMessageSource === 'function'
          ? this.terminalMessageSource(msg)
          : String(msg.terminal_source || '').trim().toLowerCase();
        if (terminalSource === 'user') return 'human';
        if (terminalSource === 'agent' || terminalSource === 'assistant') return 'agent';
        return 'system';
      }
      var role = String(msg.role || '').trim().toLowerCase();
      if (!role) return 'other';
      if (role === 'assistant') role = 'agent';
      if (role === 'user' || role === 'human') return 'human';
      if (role === 'agent') return 'agent';
      if (role === 'system') return 'system';
      return 'other';
    },

    messageIsAgentOrigin: function(msg) {
      return this.messageOriginKind(msg) === 'agent';
    },

    messageIsHumanOrigin: function(msg) {
      return this.messageOriginKind(msg) === 'human';
    },

    messageStatReadNumberPath: function(source, path) {
      if (!source || typeof source !== 'object') return 0;
      var keyPath = String(path || '').trim();
      if (!keyPath) return 0;
      var target = source;
      var parts = keyPath.split('.');
      for (var i = 0; i < parts.length; i += 1) {
        var key = String(parts[i] || '').trim();
        if (!key || !target || typeof target !== 'object' || !Object.prototype.hasOwnProperty.call(target, key)) return 0;
        target = target[key];
      }
      var numeric = Number(typeof target === 'string' ? target.replace(/,/g, '').trim() : target);
      if (!Number.isFinite(numeric) || numeric <= 0) return 0;
      return numeric;
    },

    messageStatReadNumberFromPaths: function(msg, paths) {
      var row = msg && typeof msg === 'object' ? msg : {};
      var probes = Array.isArray(paths) ? paths : [];
      for (var i = 0; i < probes.length; i += 1) {
        var numeric = this.messageStatReadNumberPath(row, probes[i]);
        if (numeric > 0) return numeric;
      }
      return 0;
    },

    messageStatDurationFromMeta: function(msg) {
      var meta = String(msg && msg.meta || '').trim();
      if (!meta) return 0;
      var minuteMatch = meta.match(/(?:^|\|)\s*([0-9]{1,3})\s*m\s*([0-9]{1,2})\s*s\s*(?:\||$)/i);
      if (minuteMatch) {
        var min = Number(minuteMatch[1] || 0);
        var sec = Number(minuteMatch[2] || 0);
        if (Number.isFinite(min) && Number.isFinite(sec) && (min > 0 || sec > 0)) return (min * 60000) + (sec * 1000);
      }
      var secondMatch = meta.match(/(?:^|\|)\s*([0-9]+(?:\.[0-9]+)?)\s*s\s*(?:\||$)/i);
      if (secondMatch) {
        var seconds = Number(secondMatch[1] || 0);
        if (Number.isFinite(seconds) && seconds > 0) return Math.round(seconds * 1000);
      }
      var milliMatch = meta.match(/(?:^|\|)\s*([0-9]+(?:\.[0-9]+)?)\s*ms\s*(?:\||$)/i);
      if (milliMatch) {
        var millis = Number(milliMatch[1] || 0);
        if (Number.isFinite(millis) && millis > 0) return Math.round(millis);
      }
      return 0;
    },

    messageStatResponseTimeMs: function(msg) {
      if (!msg || typeof msg !== 'object') return 0;
      var fromPayload = this.messageStatReadNumberFromPaths(msg, [
        'duration_ms',
        'elapsed_ms',
        'response_ms',
        'response_time_ms',
        'responseTimeMs',
        'latency_ms',
        'latencyMs',
        'turn_transaction.duration_ms',
        'turn_transaction.elapsed_ms',
        'turn_transaction.response_ms',
        'turn_transaction.response_time_ms',
        'turn_transaction.responseTimeMs',
        'turn_transaction.metrics.duration_ms',
        'response_finalization.duration_ms',
        'response_finalization.elapsed_ms',
        'response_finalization.response_ms',
        'response_finalization.response_time_ms',
        'response_workflow.duration_ms'
      ]);
      if (fromPayload > 0) return fromPayload;
      return this.messageStatDurationFromMeta(msg);
    },

    messageStatResponseTimeText: function(msg) {
      var service = typeof this.messageMetadataService === 'function' ? this.messageMetadataService() : null;
      var durationMs = this.messageStatResponseTimeMs(msg);
      if (service && typeof service.responseTimeText === 'function') {
        return service.responseTimeText(msg, durationMs, typeof this.formatResponseDuration === 'function' ? this.formatResponseDuration.bind(this) : null);
      }
      if (!msg || msg.thinking || msg.is_notice || !durationMs || durationMs <= 0) return '';
      return Math.round(durationMs) + 'ms';
    },

    messageStatTokensFromMeta: function(msg) {
      var meta = String(msg && msg.meta || '').trim();
      if (!meta) return 0;
      var tokenMatch = meta.match(/([0-9][0-9,]*)\s*in\s*\/\s*([0-9][0-9,]*)\s*out/i);
      if (!tokenMatch) return 0;
      var inTokens = Number(String(tokenMatch[1] || '0').replace(/,/g, ''));
      var outTokens = Number(String(tokenMatch[2] || '0').replace(/,/g, ''));
      if (!Number.isFinite(inTokens) || inTokens < 0) inTokens = 0;
      if (!Number.isFinite(outTokens) || outTokens < 0) outTokens = 0;
      return inTokens + outTokens;
    },

    messageStatBurnTotalTokens: function(msg) {
      if (!msg || typeof msg !== 'object') return 0;
      var total = this.messageStatReadNumberFromPaths(msg, [
        'total_tokens',
        'usage.total_tokens',
        'token_usage.total_tokens',
        'turn_transaction.total_tokens',
        'turn_transaction.usage.total_tokens',
        'turn_transaction.token_usage.total_tokens',
        'response_finalization.total_tokens',
        'response_finalization.usage.total_tokens',
        'response_workflow.total_tokens'
      ]);
      if (total > 0) return total;
      var inTokens = this.messageStatReadNumberFromPaths(msg, [
        'input_tokens',
        'usage.input_tokens',
        'token_usage.input_tokens',
        'turn_transaction.input_tokens',
        'turn_transaction.usage.input_tokens',
        'turn_transaction.token_usage.input_tokens',
        'response_finalization.input_tokens',
        'response_finalization.usage.input_tokens',
        'response_workflow.input_tokens'
      ]);
      var outTokens = this.messageStatReadNumberFromPaths(msg, [
        'output_tokens',
        'usage.output_tokens',
        'token_usage.output_tokens',
        'turn_transaction.output_tokens',
        'turn_transaction.usage.output_tokens',
        'turn_transaction.token_usage.output_tokens',
        'response_finalization.output_tokens',
        'response_finalization.usage.output_tokens',
        'response_workflow.output_tokens'
      ]);
      var combined = inTokens + outTokens;
      if (combined > 0) return combined;
      return this.messageStatTokensFromMeta(msg);
    },

    messageStatBurnLabelText: function(msg) {
      var total = this.messageStatBurnTotalTokens(msg);
      var service = typeof this.messageMetadataService === 'function' ? this.messageMetadataService() : null;
      if (service && typeof service.burnLabelText === 'function') {
        return service.burnLabelText(msg, total, typeof this.formatTokenK === 'function' ? this.formatTokenK.bind(this) : null);
      }
      if (!msg || msg.thinking || msg.is_notice || !Number.isFinite(total) || total <= 0) return '';
      return total < 1000 ? String(Math.round(total)) : ((Math.round((total / 1000) * 10) / 10).toFixed(1).replace(/\.0$/, '') + 'k');
    },

    shouldReloadHistoryForFinalEventPayload: function(payload) {
      return !!(
        payload &&
        typeof payload === 'object' &&
        String(payload.state || '').trim().toLowerCase() === 'final'
      );
    },

    parseChatSideResult: function(payload) {
      if (!payload || typeof payload !== 'object') return null;
      var candidate = payload;
      if (candidate.kind !== 'btw') return null;
      var runId = String(candidate.runId || '').trim();
      var sessionKey = String(candidate.sessionKey || '').trim();
      var question = String(candidate.question || '').trim();
      var text = String(candidate.text || '').trim();
      if (!(runId && sessionKey && question && text)) return null;
      return {
        kind: 'btw',
        runId: runId,
        sessionKey: sessionKey,
        question: question,
        text: text,
        isError: candidate.isError === true,
        ts:
          typeof candidate.ts === 'number' && Number.isFinite(candidate.ts)
            ? candidate.ts
            : Date.now()
      };
    },

    appendChatSideResultNotice: function(payload) {
      var parsed = this.parseChatSideResult(payload);
      if (!parsed) return false;
      this.messages.push({
        id: ++msgId,
        role: 'system',
        text: parsed.text,
        meta: '',
        tools: [],
        system_origin: parsed.isError ? 'runtime:btw:error' : 'runtime:btw',
        notice_label: 'Background note: ' + parsed.question,
        notice_type: parsed.isError ? 'warn' : 'info',
        run_id: parsed.runId,
        session_key: parsed.sessionKey,
        ts: parsed.ts
      });
      this.scrollToBottom();
      return true;
    },

    isStackBoundaryNoticeMessage: function(msg) {
      if (!msg || msg.terminal) return false;
      if (msg.is_notice) return true;
      if (msg.notice_label || msg.notice_type || msg.notice_action) return true;
      var role = String(msg.role || '').trim().toLowerCase();
      if (role !== 'system') return false;
      var text = String(msg.text || '').trim();
      if (!text) return false;
      if (this.isModelSwitchNoticeLabel(text)) return true;
      if (/^changed name from\s+/i.test(text)) return true;
      if (/^initialized\s+.+\s+as\s+/i.test(text)) return true;
      return false;
    },

    messageSourceKey: function(msg) {
      if (!msg || msg.is_notice) return '';
      if (this.isStackBoundaryNoticeMessage(msg)) {
        var noticeLabel = String(msg.notice_label || msg.text || '').trim().toLowerCase();
        var noticeTs = Number(msg.ts || 0) || 0;
        return 'notice:' + noticeLabel + ':' + noticeTs;
      }
      if (msg.terminal) {
        var terminalSource = this.terminalMessageSource(msg);
        if (terminalSource === 'user') return 'terminal:user';
        if (terminalSource === 'system') return 'terminal:system';
        var terminalAgentId = String((msg && msg.agent_id) || (this.currentAgent && this.currentAgent.id) || '').trim();
        return terminalAgentId ? ('terminal:agent:' + terminalAgentId.toLowerCase()) : 'terminal:agent';
      }
      var role = String(msg.role || '').trim().toLowerCase();
      if (!role) return '';
      if (role === 'user') return 'user';
      if (role === 'system') {
        // System rows should stack as one source-run when consecutive, regardless
        // of internal origin tags (inject:test, runtime:error, slash:status, etc).
        // This keeps UI grouping consistent for user-facing system narration.
        return 'system';
      }
      if (role === 'agent') {
        var agentOrigin = String(
          (msg && msg.agent_origin) ||
          (msg && msg.source_agent_id) ||
          (msg && msg.agent_id) ||
          (msg && msg.actor_id) ||
          (msg && msg.actor) ||
          (msg && msg.agent_name) ||
          ''
        ).trim();
        if (!agentOrigin && this.currentAgent && this.currentAgent.id) {
          agentOrigin = String(this.currentAgent.id || '').trim();
        }
        return agentOrigin ? ('agent:' + agentOrigin.toLowerCase()) : 'agent';
      }
      var genericOrigin = String(
        (msg && msg.agent_id) ||
        (msg && msg.actor_id) ||
        (msg && msg.actor) ||
        ''
      ).trim();
      return genericOrigin ? (role + ':' + genericOrigin.toLowerCase()) : role;
    },

    isFirstInSourceRun: function(idx, rows) {
      var list = Array.isArray(rows) ? rows : this.messages;
      if (!Array.isArray(list) || idx < 0 || idx >= list.length) return false;
      var curr = list[idx];
      if (!curr || curr.is_notice) return false;
      var currKey = this.messageSourceKey(curr);
      if (!currKey) return false;
      if (idx === 0) return true;
      var prev = list[idx - 1];
      if (!prev || this.isStackBoundaryNoticeMessage(prev)) return true;
      var prevKey = this.messageSourceKey(prev);
      return prevKey !== currKey;
    },

    isLastInSourceRun: function(idx, rows) {
      var list = Array.isArray(rows) ? rows : this.messages;
      if (!Array.isArray(list) || idx < 0 || idx >= list.length) return false;
      var curr = list[idx];
      if (!curr || curr.is_notice) return false;
      var currKey = this.messageSourceKey(curr);
      if (!currKey) return false;
      if (idx >= list.length - 1) return true;
      var next = list[idx + 1];
      if (!next || this.isStackBoundaryNoticeMessage(next)) return true;
      var nextKey = this.messageSourceKey(next);
      return nextKey !== currKey;
    },

    messagePreview: function(msg) {
      if (!msg) return '';
      if (msg.is_notice && msg.notice_label) {
        return String(msg.notice_label);
      }
      var raw = '';
      if (typeof msg.text === 'string' && msg.text.trim()) {
        raw = msg.text;
      } else if (Array.isArray(msg.tools) && msg.tools.length) {
        raw = 'Tool calls: ' + msg.tools.map(function(tool) {
          return tool && tool.name ? tool.name : 'tool';
        }).join(', ');
      } else {
        raw = '[' + (msg.role || 'message') + ']';
      }
      var compact = raw.replace(/\s+/g, ' ').trim();
      if (compact.length > 140) return compact.slice(0, 137) + '...';
      return compact;
    },

    messageMapPreview: function(msg) {
      if (this.messageMapMarkerType(msg) === 'tool') {
        return this.messageToolPreview(msg);
      }
      return this.messagePreview(msg);
    },

    appendAgentTerminalTranscript: function(rows) {
      if (!Array.isArray(rows) || !rows.length || typeof this._appendTerminalMessage !== 'function') return false;
      var appended = false;
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i] || {};
        var cwd = row.cwd ? String(row.cwd) : this.terminalPromptPath;
        var command = row.command ? String(row.command).trim() : '';
        var output = row.output ? String(row.output).trim() : '';
        if (command) {
          this._appendTerminalMessage({ role: 'terminal', text: this._terminalPromptLine(cwd, command), meta: cwd, tools: [], ts: Date.now(), terminal_source: 'agent', cwd: cwd });
          appended = true;
        }
        if (output) {
          this._appendTerminalMessage({ role: 'terminal', text: output, meta: row.is_error ? 'command failed' : 'command output', tools: [], ts: Date.now(), terminal_source: 'system', cwd: cwd, _terminal_compact: output.length > 500 });
          appended = true;
        }
      }
      return appended;
    },

    isThinkingPlaceholderText: function(input) {
      var value = String(input || '').replace(/<[^>]*>/g, ' ').replace(/\*+/g, '').replace(/\s+/g, ' ').trim().toLowerCase();
      if (!value) return true;
      if (/^(thinking|processing|working|preparing response|reasoning through context)(\.\.\.|…)?$/.test(value)) return true;
      if (/^waiting for (workflow completion|runtime response)(\.\.\.|…)?$/.test(value)) return true;
      if (/^reconnected\. syncing response(\.\.\.|…)?$/.test(value)) return true;
      if (/^(using|calling)\b.+(\.\.\.|…)?$/.test(value)) return true;
      var stripped = value.replace(/[.,!?;:…-]+/g, ' ').replace(/\s+/g, ' ').trim();
      if (stripped) {
        var words = stripped.split(' ').filter(function(part) { return !!part; });
        var placeholderLexicon = {
          thinking: true,
          processing: true,
          working: true,
          preparing: true,
          response: true,
          reasoning: true,
          through: true,
          context: true,
          waiting: true,
          workflow: true,
          completion: true,
          runtime: true,
          reconnected: true,
          syncing: true
        };
        if (words.length > 0 && words.length <= 24) {
          var allPlaceholder = words.every(function(word) {
            return !!placeholderLexicon[word];
          });
          if (allPlaceholder) return true;
        }
      }
      return false;
    },

    normalizeThinkingStatusCandidate: function(rawStatus) {
      var value = String(rawStatus || '').replace(/\r/g, '\n').trim();
      if (!value) return '';
      var lines = value
        .split('\n')
        .map(function(line) { return String(line || '').replace(/\s+/g, ' ').trim(); })
        .filter(function(line) { return !!line; });
      if (!lines.length) return '';
      for (var i = 0; i < lines.length; i++) {
        var line = String(lines[i] || '').trim();
        if (!line) continue;
        if (this.isThinkingPlaceholderText(line)) continue;
        line = line.replace(/\[(?:end|done|start)\]/ig, '').replace(/\s+/g, ' ').trim();
        if (!line) continue;
        var lowered = line.toLowerCase();
        if (/^(active|idle|running)$/.test(lowered)) continue;
        if (/^phase[:\s]/.test(lowered)) {
          line = line.replace(/^phase[:\s]*/i, '').trim();
          lowered = line.toLowerCase();
        }
        if (/web[_\s-]?search|searching (the )?(web|internet)|duckduckgo|serp/.test(lowered)) {
          line = 'Searching internet';
        } else if (/web[_\s-]?fetch|reading web|browse|browsing/.test(lowered)) {
          line = 'Reading web pages';
        } else if (/read(_|\s)?file|file read|reading files?/.test(lowered)) {
          line = 'Scanning files';
        } else if (/folder|directory|filesystem scan|scan folders?/.test(lowered)) {
          line = 'Scanning folders';
        } else if (/terminal|shell|command execution|run command/.test(lowered)) {
          line = 'Running terminal command';
        } else if (/spawn_subagents|spawn_swarm|subagents?|swarm|parallel workers?/.test(lowered)) {
          line = 'Summoning agents';
        } else if (/memory.*query|semantic memory|vector search/.test(lowered)) {
          line = 'Searching memory';
        } else if (/context warning|context limit|context window/.test(lowered)) {
          line = 'Context window warning';
        }
        line = String(line || '').replace(/\s+/g, ' ').trim();
        if (!line || this.isThinkingPlaceholderText(line)) continue;
        if (line.length > 220) line = line.slice(0, 217) + '...';
        return line;
      }
      return '';
    },

    messageToolPreview: function(msg) {
      if (!msg || !Array.isArray(msg.tools) || !msg.tools.length) {
        return this.messagePreview(msg);
      }
      var self = this;
      var compactToolText = function(value, maxLen) {
        if (value == null) return '';
        var raw = '';
        if (typeof value === 'string') {
          raw = value;
        } else {
          try {
            raw = JSON.stringify(value);
          } catch (e) {
            raw = String(value);
          }
        }
        var compact = raw.replace(/\s+/g, ' ').trim();
        if (!compact) return '';
        if (compact.length > maxLen) return compact.slice(0, maxLen - 3) + '...';
        return compact;
      };

      var parts = msg.tools.map(function(tool) {
        if (!tool) return '';
        var name = self.toolDisplayName(tool);
        var status = self.toolStatusText(tool);
        var summary = status ? (name + ' [' + status + ']') : name;
        var inputPreview = compactToolText(tool.input_preview || '', 96);
        var resultPreview = compactToolText(self.toolProjectionPreviewText(tool), 120);
        var detail = '';
        if (inputPreview && resultPreview) {
          detail = inputPreview + ' -> ' + resultPreview;
        } else {
          detail = inputPreview || resultPreview;
        }
        if (detail) summary += ': ' + detail;
        return summary;
      }).filter(function(part) { return !!part; });

      if (!parts.length) return 'Tool call';
      var preview = parts.join(' | ');
      if (preview.length > 220) return preview.slice(0, 217) + '...';
      return preview;
    },

    isLongMessagePreview: function(msg) {
      var compact = this.messageVisiblePreviewText(msg);
      if (!compact) return false;
      return compact.length >= 220 || compact.indexOf('\n\n') >= 0;
    },

    messageVisiblePreviewText: function(msg) {
      if (!msg) return '';
      var text = typeof this.extractMessageVisibleText === 'function' ? this.extractMessageVisibleText(msg) : '';
      if (!text && typeof msg.thinking_text === 'string') text = String(msg.thinking_text || '');
      if (!text && Array.isArray(msg.tools) && msg.tools.length) text = this.messageToolSummary(msg);
      if (!text && msg.notice_label) text = String(msg.notice_label || '');
      return String(text || '').trim();
    },

    isSelectedMessage: function(msg, idx) {
      if (!this.selectedMessageDomId) return false;
      return this.selectedMessageDomId === this.messageDomId(msg, idx);
    },

    truncateActorLabel: function(label, maxChars) {
      var text = String(label || '').replace(/\s+/g, ' ').trim();
      if (!text) return '';
      var limitRaw = Number(maxChars || 0);
      var limit = Number.isFinite(limitRaw) && limitRaw > 0 ? Math.max(8, Math.floor(limitRaw)) : 24;
      if (text.length <= limit) return text;
      return text.slice(0, limit - 1) + '\u2026';
    },

    normalizeSystemMessageText: function(rawText) {
      var raw = String(rawText || '');
      if (!raw.trim()) return '';
      var lowered = raw.toLowerCase();
      var errorLike = /^\s*error:/i.test(raw) || lowered.indexOf('request_read_failed') >= 0;
      if (!errorLike) return raw.trim();

      var lines = raw.split(/\r?\n/);
      var deduped = [];
      var previousKey = '';
      for (var i = 0; i < lines.length; i++) {
        var line = String(lines[i] || '').replace(/\s+/g, ' ').trim();
        if (!line) continue;
        var key = line.toLowerCase();
        if (key === previousKey) continue;
        deduped.push(line);
        previousKey = key;
      }

      if (lowered.indexOf('request_read_failed') >= 0 && deduped.length > 1) {
        var unique = [];
        var seen = {};
        for (var j = 0; j < deduped.length; j++) {
          var value = String(deduped[j] || '');
          var valueKey = value.toLowerCase();
          if (seen[valueKey]) continue;
          seen[valueKey] = true;
          unique.push(value);
        }
        deduped = unique;
      }

      return deduped.join('\n').trim();
    },

    messageAgentLabel: function(msg) {
      var name = '';
      if (msg && msg.agent_name) name = String(msg.agent_name || '');
      if (!name && msg && msg.agent_id) {
        var resolved = this.resolveAgent(msg.agent_id);
        if (resolved && resolved.name) name = String(resolved.name || '');
      }
      if (!name && this.currentAgent && this.currentAgent.name) {
        name = String(this.currentAgent.name || '');
      }
      var shortName = this.truncateActorLabel(name, 28);
      return shortName || 'Agent';
    },

    messageActorLabel: function(msg) {
      if (!msg) return 'Message';
      if (msg.is_notice) {
        if (this.normalizeNoticeType(msg.notice_type, 'model') === 'info') return '\u24d8 Info';
        return 'Model';
      }
      if (msg.terminal) {
        var terminalSource = this.terminalMessageSource(msg);
        if (terminalSource === 'user') return 'You';
        if (terminalSource === 'system') return 'System';
        return this.messageAgentLabel(msg);
      }
      if (Array.isArray(msg.tools) && msg.tools.length && (!msg.text || !String(msg.text).trim())) {
        return 'Tool';
      }
      if (msg.role === 'user') return 'You';
      if (msg.role === 'system') return 'System';
      if (msg.role === 'agent') {
        var name = '';
        if (msg && msg.agent_name) name = String(msg.agent_name || '');
        if (!name && msg && msg.agent_id) {
          var resolved = this.resolveAgent(msg.agent_id);
          if (resolved && resolved.name) name = String(resolved.name || '');
        }
        if (!name && this.currentAgent && this.currentAgent.name) {
          name = String(this.currentAgent.name || '');
        }
        var shortName = this.truncateActorLabel(name, 24);
        if (shortName) return shortName;
      }
      return 'Agent';
    },

    messageTitleLabel: function(msg) {
      if (!msg) return '';
      var role = String(msg.role || '').toLowerCase();
      if (role === 'user') return 'me';
      return this.messageActorLabel(msg);
    },

    messageTitleClass: function(msg) {
      var role = String((msg && msg.role) || '').toLowerCase();
      if (role === 'user') return 'message-agent-name-user-ghost';
      return '';
    },

    isMessageMetaReserveSpace: function(msg, idx, rows) {
      if (!msg || msg.is_notice || msg.thinking) return false;
      var list = Array.isArray(rows) ? rows : this.messages;
      if (!Array.isArray(list)) return false;
      if (this.isDirectHoveredMessage(msg, idx)) return false;
      return this.isLastInSourceRun(idx, list);
    },

    isRenameNotice: function(msg) {
      if (!msg || !msg.is_notice) return false;
      return /^changed name from /i.test(String(msg.notice_label || '').trim());
    },

    messageMapToolOutcome: function(msg) {
      if (!msg || !Array.isArray(msg.tools) || !msg.tools.length) return '';
      var hasError = false;
      var hasWarning = false;
      for (var i = 0; i < msg.tools.length; i++) {
        var tool = msg.tools[i] || {};
        if (tool.running || this.isBlockedTool(tool)) {
          hasWarning = true;
          continue;
        }
        if (tool.is_error) {
          hasError = true;
        }
      }
      if (hasError) return 'error';
      if (hasWarning) return 'warning';
      return 'success';
    },

    messageMapMarkerType: function(msg) {
      if (!msg) return '';
      if (msg.is_notice) {
        return this.normalizeNoticeType(msg.notice_type, 'model') === 'info' ? 'info' : 'model';
      }
      if (msg.terminal) return 'terminal';
      if (Array.isArray(msg.tools) && msg.tools.length) return 'tool';
      return '';
    },

    messageMapShowMarker: function(msg) {
      return this.messageMapMarkerType(msg) !== '';
    },

    messageMapMarkerTitle: function(msg) {
      var type = this.messageMapMarkerType(msg);
      if (type === 'model') {
        return msg && msg.notice_label ? String(msg.notice_label) : 'Model switched';
      }
      if (type === 'info') {
        return msg && msg.notice_label ? String(msg.notice_label) : 'Info';
      }
      if (type === 'tool') {
        var outcome = this.messageMapToolOutcome(msg) || 'success';
        if (outcome === 'error') return 'Tool call error';
        if (outcome === 'warning') return 'Tool call warning';
        return 'Tool call success';
      }
      if (type === 'terminal') {
        return 'Terminal activity';
      }
      return '';
    },

    messageDayKey: function(msg) {
      if (!msg || !msg.ts) return '';
      var d = new Date(msg.ts);
      if (Number.isNaN(d.getTime())) return '';
      var y = d.getFullYear();
      var m = String(d.getMonth() + 1).padStart(2, '0');
      var day = String(d.getDate()).padStart(2, '0');
      return y + '-' + m + '-' + day;
    },

    messageDayLabel: function(msg) {
      if (!msg || !msg.ts) return 'Unknown day';
      var d = new Date(msg.ts);
      if (Number.isNaN(d.getTime())) return 'Unknown day';
      return d.toLocaleDateString(undefined, { weekday: 'long', month: 'short', day: 'numeric', year: 'numeric' });
    },

    messageDayDomId: function(msg) {
      var key = this.messageDayKey(msg);
      return key ? ('chat-day-' + key) : '';
    },

    messageDayCollapseKey: function(msg) {
      var dayKey = this.messageDayKey(msg);
      if (!dayKey) return '';
      var agentKey = String((this.currentAgent && this.currentAgent.id) || (msg && msg.agent_id) || 'global').trim();
      return (agentKey || 'global') + '::' + dayKey;
    },

    isMessageDayCollapsed: function(msg) {
      var key = this.messageDayCollapseKey(msg);
      if (!key) return false;
      return !!(this.collapsedMessageDays && this.collapsedMessageDays[key]);
    },

    toggleMessageDayCollapse: function(msg) {
      var key = this.messageDayCollapseKey(msg);
      if (!key) return;
      if (!this.collapsedMessageDays) this.collapsedMessageDays = {};
      this.collapsedMessageDays[key] = !this.collapsedMessageDays[key];
    },

    isNewMessageDay: function(list, idx) {
      if (!Array.isArray(list) || idx < 0 || idx >= list.length) return false;
      if (idx === 0) return true;
      var curr = this.messageDayKey(list[idx]);
      var prev = this.messageDayKey(list[idx - 1]);
      if (!curr) return false;
      return curr !== prev;
    },

    jumpToMessage: function(msg, idx) {
      var id = this.messageDomId(msg, idx);
      this.forceMessageRender(msg, idx, 9000);
      this.selectedMessageDomId = id;
      this.hoveredMessageDomId = id;
      this.mapStepIndex = idx;
      var chatStore = window.InfringChatStore;
      if (chatStore && typeof chatStore.setThreadProjectionCenter === 'function') {
        chatStore.setThreadProjectionCenter(idx);
      }
      this.centerChatMapOnMessage(id);
      var self = this;
      var attempts = 0;
      var scrollToTarget = function() {
        var target = document.getElementById(id);
        if (!target) {
          attempts += 1;
          if (attempts <= 4) {
            setTimeout(scrollToTarget, 28);
          }
          return;
        }
        target.scrollIntoView({ behavior: 'smooth', block: 'center' });
        if (typeof self.scheduleMessageRenderWindowUpdate === 'function') {
          self.scheduleMessageRenderWindowUpdate();
        }
      };
      scrollToTarget();
    },

    jumpToMessageDay: function(msg) {
      var key = this.messageDayKey(msg);
      if (!key) return;
      var target = document.querySelector('.chat-day-anchor[data-day="' + key + '"]');
      if (!target) return;
      target.scrollIntoView({ behavior: 'smooth', block: 'start' });
    },

    addNoticeEvent: function(notice) {
      if (!notice || typeof notice !== 'object') return;
      var label = String(notice.notice_label || notice.label || '').trim();
      if (!label) return;
      var type = this.normalizeNoticeType(
        notice.notice_type || notice.type,
        this.isModelSwitchNoticeLabel(label) ? 'model' : 'info'
      );
      var icon = String(notice.notice_icon || notice.icon || '').trim();
      if (type === 'info' && /^changed name from /i.test(label)) {
        icon = '';
      }
      var tsRaw = Number(notice.ts || 0);
      var ts = Number.isFinite(tsRaw) && tsRaw > 0 ? tsRaw : Date.now();
      var action = this.normalizeNoticeAction(notice.notice_action || notice.noticeAction || null);
      this.messages.push({
        id: ++msgId,
        role: 'system',
        text: '',
        meta: '',
        tools: [],
        system_origin: 'notice:' + type,
        is_notice: true,
        notice_label: label,
        notice_type: type,
        notice_icon: icon,
        notice_action: action,
        ts: ts
      });
      if (this.currentAgent && this.currentAgent.id) {
        this.rememberModelNotice(this.currentAgent.id, label, ts, type, icon);
      }
      this.scrollToBottom();
      this.scheduleConversationPersist();
    },

    agentMessageSignature: function(message) {
      if (!message || typeof message !== 'object') return '';
      var text = this.messageVisiblePreviewText(message).replace(/\s+/g, ' ').trim().toLowerCase();
      var tools = Array.isArray(message.tools) ? message.tools : [];
      var toolParts = [];
      for (var i = 0; i < tools.length && i < 8; i += 1) {
        var tool = tools[i] || {};
        var name = String(tool.name || '').trim().toLowerCase();
        var result = String(this.toolProjectionPreviewText(tool) || '').replace(/\s+/g, ' ').trim().toLowerCase();
        if (result.length > 180) result = result.slice(0, 180);
        var state = tool && tool.is_error ? 'error' : (tool && tool.running ? 'running' : 'ok');
        if (name || result) toolParts.push(name + ':' + state + ':' + result);
      }
      return (text || '') + '||' + toolParts.join('||');
    },

    assistantTurnStartTimestamp: function(message) {
      if (!message || typeof message !== 'object') return 0;
      var turn = message.turn_transaction && typeof message.turn_transaction === 'object'
        ? message.turn_transaction
        : null;
      var raw = Number(
        message._turn_started_at ||
        (turn && (turn.started_at || turn.request_started_at || turn.created_at || turn.ts)) ||
        0
      );
      if (!Number.isFinite(raw) || raw <= 0) return 0;
      return raw;
    },

    findRecentDuplicateAgentMessage: function(candidate, dedupeWindowMs) {
      if (!candidate || typeof candidate !== 'object') return null;
      var rows = Array.isArray(this.messages) ? this.messages : [];
      if (!rows.length) return null;
      var signature = this.agentMessageSignature(candidate);
      if (!signature) return null;
      var candidateTurnStart = this.assistantTurnStartTimestamp(candidate);
      var nowTs = Number(candidate.ts || Date.now());
      var maxAge = Number(dedupeWindowMs || 70000);
      if (!Number.isFinite(maxAge) || maxAge < 5000) maxAge = 70000;
      var checked = 0;
      for (var i = rows.length - 1; i >= 0; i -= 1) {
        var row = rows[i];
        if (!row || row.thinking || row.streaming) continue;
        var role = String(row.role || '').toLowerCase();
        if (role !== 'agent' && role !== 'assistant') continue;
        checked += 1;
        var rowTs = Number(row.ts || 0);
        var ageMs = rowTs > 0 ? Math.abs(nowTs - rowTs) : 0;
        if (ageMs > maxAge && checked > 3) break;
        var rowSignature = this.agentMessageSignature(row);
        if (rowSignature === signature && (!rowTs || ageMs <= maxAge)) return row;
        if (candidateTurnStart > 0) {
          var rowTurnStart = this.assistantTurnStartTimestamp(row);
          if (!(rowTurnStart > 0 && Math.abs(rowTurnStart - candidateTurnStart) <= 1200)) {
            continue;
          }
        }
        if (rowSignature === signature) return row;
        if (checked >= 16) break;
      }
      return null;
    },

    pushAgentMessageDeduped: function(message, options) {
      var payload = message && typeof message === 'object' ? message : null;
      if (!payload) return null;
      var opts = options && typeof options === 'object' ? options : {};
      var dedupeWindowMs = Number(opts.dedupe_window_ms || opts.dedupeWindowMs || 70000);
      var duplicate = this.findRecentDuplicateAgentMessage(payload, dedupeWindowMs);
      if (!duplicate) {
        return this.appendActiveChatMessage(payload);
      }
      var mergeToolCards = function(existingTools, incomingTools) {
        var base = Array.isArray(existingTools) ? existingTools.slice() : [];
        var incoming = Array.isArray(incomingTools) ? incomingTools : [];
        if (!incoming.length) return base;
        var keyFor = function(tool) {
          if (!tool || typeof tool !== 'object') return '';
          var id = String(tool.id || '').trim();
          if (id) return 'id:' + id;
          var name = String(tool.name || '').trim().toLowerCase();
          var input = String(tool.input_preview || '').trim();
          return 'sig:' + name + '::' + input;
        };
        var index = Object.create(null);
        for (var i = 0; i < base.length; i++) {
          var baseKey = keyFor(base[i]);
          if (!baseKey) continue;
          index[baseKey] = i;
        }
        for (var j = 0; j < incoming.length; j++) {
          var next = incoming[j];
          if (!next || typeof next !== 'object') continue;
          var nextKey = keyFor(next);
          var pos = (nextKey && Object.prototype.hasOwnProperty.call(index, nextKey))
            ? Number(index[nextKey])
            : -1;
          if (pos < 0 || pos >= base.length) {
            base.push(next);
            if (nextKey) index[nextKey] = base.length - 1;
            continue;
          }
          var prior = base[pos];
          if (!prior || typeof prior !== 'object') {
            base[pos] = next;
            continue;
          }
          if (!String(prior.result_preview || '').trim() && String(next.result_preview || '').trim()) prior.result_preview = next.result_preview;
          if (!String(prior.input_preview || '').trim() && String(next.input_preview || '').trim()) prior.input_preview = next.input_preview;
          if (!String(prior.id || '').trim() && String(next.id || '').trim()) prior.id = next.id;
          if (next.is_error) prior.is_error = true;
          if (prior.running && next.running === false) prior.running = false;
        }
        return base;
      };
      if (duplicate._auto_fallback && !payload._auto_fallback) {
        duplicate.text = payload.text;
        duplicate.tools = Array.isArray(payload.tools) ? payload.tools : [];
        duplicate._auto_fallback = false;
      } else if ((!String(duplicate.text || '').trim()) && String(payload.text || '').trim()) {
        duplicate.text = payload.text;
      }
      if (Array.isArray(payload.tools) && payload.tools.length) {
        duplicate.tools = mergeToolCards(duplicate.tools, payload.tools);
      }
      if (payload.response_finalization && typeof payload.response_finalization === 'object') {
        duplicate.response_finalization = payload.response_finalization;
      }
      if (payload.turn_transaction && typeof payload.turn_transaction === 'object') {
        duplicate.turn_transaction = payload.turn_transaction;
      }
      if (Array.isArray(payload.terminal_transcript) && payload.terminal_transcript.length) {
        duplicate.terminal_transcript = payload.terminal_transcript;
      }
      if (payload.attention_queue && typeof payload.attention_queue === 'object') {
        duplicate.attention_queue = payload.attention_queue;
      }
      if (String(payload.tool_failure_summary || '').trim()) {
        duplicate.tool_failure_summary = String(payload.tool_failure_summary || '').trim();
      }
      if (!String(duplicate.text || '').trim() && typeof this.fallbackAssistantTextFromPayload === 'function') {
        var repairedDuplicateText = String(this.fallbackAssistantTextFromPayload(duplicate, duplicate.tools || []) || '').trim();
        if (repairedDuplicateText) duplicate.text = repairedDuplicateText;
      }
      var nextMeta = String(payload.meta || '').trim();
      if (nextMeta) {
        var priorMeta = String(duplicate.meta || '').trim();
        duplicate.meta = priorMeta ? priorMeta : nextMeta;
      }
      duplicate.ts = Number(payload.ts || Date.now());
      duplicate.agent_id = payload.agent_id || duplicate.agent_id;
      duplicate.agent_name = payload.agent_name || duplicate.agent_name;
      if (typeof this.syncActiveChatMessages === 'function') this.syncActiveChatMessages();
      this.scheduleConversationPersist();
      return duplicate;

    },

    normalizeNoticeAction: function(action) {
      if (!action || typeof action !== 'object') return null;
      var kind = String(action.kind || action.type || '').trim().toLowerCase();
      if (!kind) return null;
      var label = String(action.label || '').trim();
      if (kind === 'system_update') {
        return {
          kind: kind,
          label: label || 'Update',
          latest_version: String(action.latest_version || '').trim(),
          current_version: String(action.current_version || '').trim(),
          busy: !!action.busy
        };
      }
      if (kind === 'model_discover') {
        return {
          kind: kind,
          label: label || 'Discover models',
          reason: String(action.reason || '').trim(),
          starter_model: String(action.starter_model || '').trim(),
          starter_provider: String(action.starter_provider || 'ollama').trim(),
          busy: !!action.busy
        };
      }
      if (kind === 'open_url') {
        var url = String(action.url || '').trim();
        if (!url) return null;
        return {
          kind: kind,
          label: label || 'Open link',
          url: url,
          busy: !!action.busy
        };
      }
      return null;
    },

    noticeActionVisible: function(msg) {
      return !!this.normalizeNoticeAction(msg && msg.notice_action ? msg.notice_action : null);
    },

    noticeActionLabel: function(msg) {
      var action = this.normalizeNoticeAction(msg && msg.notice_action ? msg.notice_action : null);
      return action ? String(action.label || 'Update') : '';
    },

    noticeActionBusy: function(msg) {
      return !!(msg && msg.notice_action && msg.notice_action.busy === true);
    },

    isTrustedExternalActionUrl: function(value) {
      var raw = String(value || '').trim();
      if (!raw) return false;
      try {
        var target = new URL(raw, window.location.href);
        var host = String(target.hostname || '').trim().toLowerCase();
        var sameHost = false;
        try {
          var local = new URL(window.location.href);
          sameHost = String(target.host || '').trim().toLowerCase() === String(local.host || '').trim().toLowerCase();
        } catch (_) {}
        if (sameHost) return true;
        return (
          host === 'localhost' ||
          host === '127.0.0.1' ||
          host === '::1' ||
          host === '[::1]' ||
          host.indexOf('127.') === 0
        );
      } catch (_) {
        return false;
      }
    },

    openNoticeActionUrl: function(url) {
      var target = String(url || '').trim();
      if (!target) return false;
      if (typeof window === 'undefined' || typeof window.open !== 'function') return false;
      if (this.isTrustedExternalActionUrl(target)) {
        window.open(target, '_blank', 'noopener,noreferrer');
        return true;
      }
      InfringToast.confirm(
        'Open External Link',
        'Open this external URL?\n' + target,
        function() {
          try {
            window.open(target, '_blank', 'noopener,noreferrer');
          } catch (_) {}
        }
      );
      return true;
    },

    async triggerNoticeAction(msg) {
      var action = this.normalizeNoticeAction(msg && msg.notice_action ? msg.notice_action : null);
      if (!action) return;
      if (this.systemUpdateBusy || this.noticeActionBusy(msg)) return;
      if (msg && msg.notice_action) msg.notice_action.busy = true;
      this.systemUpdateBusy = true;
      this.scheduleConversationPersist();
      try {
        if (action.kind === 'system_update') {
          var payload = {};
          if (action.latest_version) payload.latest_version = action.latest_version;
          if (action.current_version) payload.current_version = action.current_version;
          var result = await InfringAPI.post('/api/system/update', payload);
          this.addNoticeEvent({
            notice_label: String(result && result.message ? result.message : 'System update started.'),
            notice_type: 'info',
            notice_icon: '\u21bb'
          });
          if (msg) msg.notice_action = null;
        } else if (action.kind === 'model_discover') {
          var models = await this.refreshModelCatalogAndGuidance({ discover: true, guidance: true });
          var available = this.availableModelRowsCount(models);
          if (available > 0) {
            this.addNoticeEvent({
              notice_label: 'Model discovery ready: ' + available + ' runnable model' + (available === 1 ? '' : 's') + ' detected.',
              notice_type: 'info',
              notice_icon: '\u2713'
            });
            if (msg) msg.notice_action = null;
          } else {
            var starterProvider = String(action.starter_provider || 'ollama').trim();
            var starterModel = String(action.starter_model || '').trim();
            if (starterProvider && starterModel) {
              await InfringAPI.post('/api/shell-socket/models/download', {
                provider: starterProvider,
                model: starterProvider + '/' + starterModel
              }).catch(function() { return null; });
              models = await this.refreshModelCatalogAndGuidance({ discover: true, guidance: true });
              available = this.availableModelRowsCount(models);
            }
            if (available > 0) {
              this.addNoticeEvent({
                notice_label: 'Starter model ready. You can chat now.',
                notice_type: 'info',
                notice_icon: '\u2713'
              });
              if (msg) msg.notice_action = null;
            } else {
              if (msg && msg.notice_action) {
                msg.notice_action = {
                  kind: 'open_url',
                  label: 'Install Ollama',
                  url: 'https://ollama.com/download'
                };
              }
              this.addNoticeEvent({
                notice_label: 'Still no runnable models detected. Install Ollama, then retry discovery.',
                notice_type: 'warn',
                notice_icon: '\u26a0'
              });
            }
          }
        } else if (action.kind === 'open_url') {
          var opened = this.openNoticeActionUrl(action.url);
          if (opened && msg) msg.notice_action = null;
        }
      } catch (e) {
        var reason = e && e.message ? String(e.message) : 'unknown_error';
        if (action.kind === 'system_update') {
          InfringToast.error('Failed to start system update: ' + reason);
        } else if (action.kind === 'model_discover') {
          InfringToast.error('Model recovery failed: ' + reason);
        } else if (action.kind === 'open_url') {
          InfringToast.error('Failed to open link: ' + reason);
        }
        if (msg && msg.notice_action) msg.notice_action.busy = false;
      } finally {
        this.systemUpdateBusy = false;
        this.scheduleConversationPersist();
      }
    },

    async checkForSystemReleaseUpdate(force) {
      if (this._releaseCheckInFlight) return;
      if (!this.currentAgent || !this.currentAgent.id) return;
      this._releaseCheckInFlight = true;
      try {
        var result = await InfringAPI.get('/api/system/release-check' + (force ? '?force=1' : ''));
        if (!result || result.ok === false || !result.update_available) return;
        var latest = String(result.latest_version || '').trim();
        var current = String(result.current_version || '').trim();
        if (!latest) return;
        var noticeKey = latest + '|' + current;
        if (noticeKey && this._releaseUpdateNoticeKey === noticeKey) return;
        var label = 'Update available: ' + latest + (current ? ' (current ' + current + ')' : '');
        var existing = Array.isArray(this.messages) && this.messages.some(function(row) {
          return !!(row && row.is_notice && String(row.notice_label || '').trim() === label);
        });
        if (existing) {
          this._releaseUpdateNoticeKey = noticeKey;
          return;
        }
        this._releaseUpdateNoticeKey = noticeKey;
        this.addNoticeEvent({
          notice_label: label,
          notice_type: 'info',
          notice_icon: '\u21e7',
          notice_action: {
            kind: 'system_update',
            label: 'Update',
            latest_version: latest,
            current_version: current
          }
        });
      } catch (_) {
      } finally {
        this._releaseCheckInFlight = false;
      }
    },

    ensureActiveChatMessagesArray: function() {
      if (!Array.isArray(this.messages)) this.messages = [];
      return this.messages;
    },

    syncActiveChatMessages: function() {
      var activeStore = window.InfringChatStore;
      if (activeStore && typeof activeStore.syncMessages === 'function') {
        activeStore.syncMessages(this.messages, this.allFilteredMessages);
      }
      return this.messages;
    },

    replaceActiveChatMessages: function(rows) {
      this.messages = Array.isArray(rows) ? rows : [];
      this.syncActiveChatMessages();
      return this.messages;
    },

    mutateActiveChatMessages: function(mutator) {
      var rows = this.ensureActiveChatMessagesArray();
      var nextRows = typeof mutator === 'function' ? mutator(rows) : rows;
      if (Array.isArray(nextRows) && nextRows !== rows) this.messages = nextRows;
      this.syncActiveChatMessages();
      return this.messages;
    },

    appendActiveChatMessage: function(message) {
      this.ensureActiveChatMessagesArray().push(message);
      this.syncActiveChatMessages();
      return message;
    },

    pushSystemMessage: function(entry) {
      var payload = entry && typeof entry === 'object' ? entry : { text: entry };
      var rawText = String(payload && payload.text ? payload.text : '');
      var text = this.normalizeSystemMessageText
        ? this.normalizeSystemMessageText(rawText)
        : rawText.trim();
      if (!text) return null;
      var canonicalText = text.replace(/\s+/g, ' ').trim().toLowerCase();
      if (/^error:\s*/i.test(canonicalText) && canonicalText.indexOf('operation was aborted') >= 0) return null;
      if (payload.allow_chat_injection !== true) {
        if (!Array.isArray(this.systemTelemetry)) this.systemTelemetry = [];
        this.systemTelemetry.push({ text: text, origin: payload.system_origin || payload.systemOrigin || '', ts: Date.now() });
        return null;
      }

      var origin = String(payload.system_origin || payload.systemOrigin || '').trim();
      var tsRaw = Number(payload.ts || 0);
      var ts = Number.isFinite(tsRaw) && tsRaw > 0 ? tsRaw : Date.now();
      var dedupeWindowMs = Number(payload.dedupe_window_ms || payload.dedupeWindowMs || 8000);
      if (!Number.isFinite(dedupeWindowMs) || dedupeWindowMs < 0) dedupeWindowMs = 8000;
      if (dedupeWindowMs > 60000) dedupeWindowMs = 60000;
      var canDedupe = payload.dedupe !== false;
      var systemThreadId = String(this.systemThreadId || 'system').trim() || 'system';
      var activeId = String((this.currentAgent && this.currentAgent.id) || '').trim();
      var targetId = activeId || systemThreadId;
      var isGlobalNotice = !!(
        this.isSystemNotificationGlobalToWorkspace &&
        this.isSystemNotificationGlobalToWorkspace(origin, text)
      );
      var routeToSystem =
        payload.route_to_system === true ||
        (payload.route_to_system !== false && isGlobalNotice);
      if (routeToSystem) targetId = systemThreadId;
      var activeThread = !!activeId && activeId === targetId;
      if (!this._systemMessageDedupeIndex || typeof this._systemMessageDedupeIndex !== 'object') this._systemMessageDedupeIndex = {};

      var targetRows = null;
      var targetCache = null;
      if (activeThread) {
        targetRows = this.ensureActiveChatMessagesArray();
      } else {
        if (!this.conversationCache || typeof this.conversationCache !== 'object') this.conversationCache = {};
        targetCache = this.conversationCache[targetId];
        if (!targetCache || typeof targetCache !== 'object' || !Array.isArray(targetCache.messages)) {
          targetCache = { saved_at: Date.now(), token_count: 0, messages: [] };
          this.conversationCache[targetId] = targetCache;
        }
        targetRows = targetCache.messages;
      }

      if (!Array.isArray(targetRows)) return null;
      var dedupeKey = targetId + '|' + (origin || '_') + '|' + canonicalText;
      if (canDedupe) {
        for (var idx = targetRows.length - 1, scanned = 0; idx >= 0 && scanned < 24; idx -= 1) {
          var row = targetRows[idx];
          if (!row || row.thinking || row.streaming) continue;
          if (String(row.role || '').toLowerCase() !== 'system' || row.is_notice) continue;
          scanned += 1;
          var rowText = String(row.text || '').replace(/\s+/g, ' ').trim().toLowerCase();
          if (rowText !== canonicalText) continue;
          var rowTs = Number(row.ts || 0);
          if (Number.isFinite(rowTs) && Math.abs(ts - rowTs) > dedupeWindowMs) continue;
          var rowOrigin = String(row.system_origin || '').trim();
          if (rowOrigin && origin && rowOrigin !== origin && !/^error:/i.test(canonicalText)) continue;
          var repeatCount = Number(row._repeat_count || 1);
          if (!Number.isFinite(repeatCount) || repeatCount < 1) repeatCount = 1;
          repeatCount += 1;
          row._repeat_count = repeatCount;
          var priorMeta = String(row.meta || '').trim().replace(/\s*\|\s*repeated x\d+\s*$/i, '').trim();
          row.meta = (priorMeta ? (priorMeta + ' | ') : '') + 'repeated x' + repeatCount;
          row.ts = ts;
          this._systemMessageDedupeIndex[dedupeKey] = { id: row.id, ts: ts };
          if (activeThread) {
            this.syncActiveChatMessages();
            this.scheduleConversationPersist();
          } else this.persistConversationCache();
          return row;
        }
      }

      var message = {
        id: ++msgId,
        role: 'system',
        text: text,
        meta: String(payload.meta || ''),
        tools: Array.isArray(payload.tools) ? payload.tools : [],
        system_origin: origin,
        ts: ts
      };
      if (activeThread) this.appendActiveChatMessage(message);
      else targetRows.push(message);
      if (canDedupe && canonicalText) this._systemMessageDedupeIndex[dedupeKey] = { id: message.id, ts: ts };
      var store = Alpine.store('app');
      if (store && typeof store.saveAgentChatPreview === 'function') {
        store.saveAgentChatPreview(targetId, targetRows);
      }
      if (activeThread) {
        if (payload.auto_scroll !== false) this.scrollToBottom();
        this.scheduleConversationPersist();
      } else {
        if (targetCache) {
          targetCache.saved_at = Date.now();
          targetCache.token_count = 0;
        }
        this.persistConversationCache();
      }
      return message;
    },
    addModelSwitchNotice: function(previousModelName, previousProviderName, modelName, providerName) {
      var legacyCall = arguments.length < 3;
      var previousModel = '';
      var model = '';
      if (legacyCall) {
        model = String(previousModelName || '').trim();
      } else {
        previousModel = String(previousModelName || '').trim();
        model = String(modelName || '').trim();
      }
      if (!model) return;
      if (!previousModel && this.currentAgent) {
        previousModel = String(this.currentAgent.runtime_model || this.currentAgent.model_name || '').trim();
      }
      if (!previousModel) previousModel = 'unknown';
      var label = 'Model switched from ' + previousModel + ' to ' + model;
      this.touchModelUsage(model);
      this.addNoticeEvent({ notice_label: label, notice_type: 'model', ts: Date.now() });
    },

    addAgentRenameNotice: function(previousName, nextName) {
      var fromName = String(previousName || '').trim();
      var toName = String(nextName || '').trim();
      if (!toName || fromName === toName) return;
      if (!fromName) fromName = 'Unnamed agent';
      this.addNoticeEvent({
        notice_label: 'changed name from ' + fromName + ' to ' + toName,
        notice_type: 'info',
        ts: Date.now()
      });
    },

    formatResponseDuration: function(ms) {
      var num = Number(ms || 0);
      if (!Number.isFinite(num) || num <= 0) return '';
      if (num < 1000) return Math.round(num) + 'ms';
      if (num < 60000) {
        return (num < 10000 ? (num / 1000).toFixed(1) : Math.round(num / 1000)) + 's';
      }
      var min = Math.floor(num / 60000);
      var sec = Math.round((num % 60000) / 1000);
      return min + 'm ' + sec + 's';
    },
    stepMessageMap: function(list, dir) {
      if (!Array.isArray(list) || !list.length) return;
      this.suppressMapPreview = true;
      if (typeof this.hideDashboardPopupBySource === 'function') {
        this.hideDashboardPopupBySource(this.chatMapPopupSource());
      }
      if (this._mapPreviewSuppressTimer) clearTimeout(this._mapPreviewSuppressTimer);
      var visibleIndexes = [];
      var fallbackIndexes = [];
      var searchQuery = String(this.searchQuery || '').trim();
      for (var i = 0; i < list.length; i++) {
        if (this.isMessageDayCollapsed(list[i])) continue;
        fallbackIndexes.push(i);
        if (!searchQuery || !this.messageMatchesSearchQuery || this.messageMatchesSearchQuery(list[i], searchQuery)) visibleIndexes.push(i);

      }
      if (!visibleIndexes.length) visibleIndexes = fallbackIndexes;
      if (!visibleIndexes.length) return;

      var activePos = -1;
      var anchorDomId = String(this.selectedMessageDomId || '');
      if (anchorDomId) {
        for (var p = 0; p < visibleIndexes.length; p++) {
          var vi = visibleIndexes[p];
          if (this.messageDomId(list[vi], vi) === anchorDomId) {
            activePos = p;
            break;
          }
        }
      }
      if (activePos < 0) {
        for (var p2 = 0; p2 < visibleIndexes.length; p2++) {
          if (visibleIndexes[p2] === this.mapStepIndex) {
            activePos = p2;
            break;
          }
        }
      }

      if (activePos < 0) {
        activePos = dir > 0 ? 0 : (visibleIndexes.length - 1);
      } else {
        activePos = activePos + (dir > 0 ? 1 : -1);
        if (activePos < 0) activePos = 0;
        if (activePos > visibleIndexes.length - 1) activePos = visibleIndexes.length - 1;
      }

      var next = visibleIndexes[activePos];
      var msg = list[next];
      if (!msg) return;
      this.setHoveredMessage(msg, next);
      this.jumpToMessage(msg, next);
      this.centerChatMapOnMessage(this.messageDomId(msg, next));
      var self = this;
      this._mapPreviewSuppressTimer = setTimeout(function() {
        self.suppressMapPreview = false;
      }, 220);
    },

    chatMapPopupSource: function() {
      return 'chat-map';
    },

    messageMapPopupTitle: function(msg) {
      if (!msg) return 'Message';
      return this.messageActorLabel(msg);
    },

    messageMapPopupBody: function(msg) {
      if (!msg) return '';
      var preview = typeof this.messageVisiblePreviewText === 'function' ? this.messageVisiblePreviewText(msg) : '';
      if (!preview && typeof this.messageMapPreview === 'function') preview = this.messageMapPreview(msg);
      return String(preview || '').trim();
    },

    showMapItemPopup: function(msg, idx, ev) {
      if (!msg) return;
      var domId = this.messageDomId(msg, idx);
      this.forceMessageRender(msg, idx, 9000);
      this.suppressMapPreview = false;
      this.selectedMessageDomId = domId;
      this.mapStepIndex = idx;
      this.setHoveredMessage(msg, idx);
      if (typeof this.showDashboardPopup !== 'function') return;
      this.showDashboardPopup('chat-map-item:' + domId, this.messageMapPopupTitle(msg), ev, {
        source: this.chatMapPopupSource(),
        side: 'left',
        body: this.messageMapPopupBody(msg),
        meta_origin: 'Chat map',
        meta_time: String(this.messageTimestampLabel(msg) || '').trim()
      });
    },

    hideMapItemPopup: function() {
      if (typeof this.hideDashboardPopupBySource === 'function') {
        this.hideDashboardPopupBySource(this.chatMapPopupSource());
      }
      this.clearHoveredMessage();
    },

    showMapDayPopup: function(msg, ev) {
      if (!msg) return;
      this.suppressMapPreview = false;
      if (typeof this.showDashboardPopup !== 'function') return;
      this.showDashboardPopup('chat-map-day:' + this.messageDayKey(msg), this.messageDayLabel(msg), ev, {
        source: this.chatMapPopupSource(),
        side: 'left',
        body: this.isMessageDayCollapsed(msg)
          ? 'Expand this day in the chat map'
          : 'Collapse this day in the chat map',
        meta_origin: 'Chat map'
      });
    },

    hideMapDayPopup: function() {
      if (typeof this.hideDashboardPopupBySource === 'function') {
        this.hideDashboardPopupBySource(this.chatMapPopupSource());
      }
    },

    setHoveredMessage: function(msg, idx) {
      if (this._hoverClearTimer) {
        clearTimeout(this._hoverClearTimer);
        this._hoverClearTimer = 0;
      }
      if (!msg && msg !== 0) {
        this.hoveredMessageDomId = this.selectedMessageDomId || '';
        this.directHoveredMessageDomId = '';
        return;
      }
      var domId = this.messageDomId(msg, idx);
      this.hoveredMessageDomId = domId;
      this.directHoveredMessageDomId = domId;
    },

    clearHoveredMessage: function() {
      if (this._hoverClearTimer) clearTimeout(this._hoverClearTimer);
      var self = this;
      this._hoverClearTimer = setTimeout(function() {
        self._hoverClearTimer = 0;
        self.hoveredMessageDomId = self.selectedMessageDomId || '';
        self.directHoveredMessageDomId = '';
      }, 42);
    },

    clearHoveredMessageHard: function() {
      if (this._hoverClearTimer) {
        clearTimeout(this._hoverClearTimer);
        this._hoverClearTimer = 0;
      }
      if (typeof this.hideDashboardPopupBySource === 'function') {
        this.hideDashboardPopupBySource(this.chatMapPopupSource());
      }
      this.hoveredMessageDomId = '';
      this.directHoveredMessageDomId = '';
      this.selectedMessageDomId = '';
    },

    isHoveredMessage: function(msg, idx) {
      if (!this.hoveredMessageDomId) return false;
      return this.hoveredMessageDomId === this.messageDomId(msg, idx);
    },

    isDirectHoveredMessage: function(msg, idx) {
      if (!this.directHoveredMessageDomId) return false;
      return this.directHoveredMessageDomId === this.messageDomId(msg, idx);
    },

    centerChatMapOnMessage: function(domId, options) {
      if (!domId) return;
      var immediate = !!(options && options.immediate);
      var map = null;
      var maps = document.querySelectorAll('.chat-map-scroll');
      for (var i = 0; i < maps.length; i++) {
        var candidate = maps[i];
        if (candidate && candidate.offsetParent !== null) {
          map = candidate;
          break;
        }
      }
      if (!map) return;
      var host = map.closest('.chat-map') || map;
      var item = host.querySelector('.chat-map-item[data-msg-dom-id="' + domId + '"]');
      if (!item) return;
      var topGuard = 28;
      var bottomGuard = 28;
      var viewport = Math.max(20, map.clientHeight - topGuard - bottomGuard);
      var desired = item.offsetTop + (item.offsetHeight / 2) - (viewport / 2) - topGuard;
      var max = Math.max(0, map.scrollHeight - map.clientHeight);
      var nextTop = Math.max(0, Math.min(max, desired));
      var diff = Math.abs(map.scrollTop - nextTop);
      if (diff < 3) return;
      map.scrollTo({ top: nextTop, behavior: (immediate || this.suppressMapPreview) ? 'auto' : 'smooth' });
    },

    filteredDrawerEmojiCatalog: function() {
      var source = Array.isArray(this.drawerEmojiCatalog) ? this.drawerEmojiCatalog : [];
      var query = String(this.drawerEmojiSearch || '').trim().toLowerCase();
      var self = this;
      var allowSystemReserved = this.isSystemThreadAgent && this.isSystemThreadAgent(this.currentAgent);
      var rows = source.filter(function(row) {
        var emoji = String((row && row.emoji) || '');
        if (!allowSystemReserved && self.isReservedSystemEmoji && self.isReservedSystemEmoji(emoji)) return false;
        return true;
      });
      if (!query) return rows.slice(0, 24);
      return rows.filter(function(row) {
        var emoji = String((row && row.emoji) || '');
        var name = String((row && row.name) || '').toLowerCase();
        return emoji.indexOf(query) >= 0 || name.indexOf(query) >= 0;
      }).slice(0, 24);
    },

    defaultFreshEmojiForAgent: function(agentRef) {
      void agentRef;
      return '∞';
    },

    suggestedFreshIdentityForAgent: function(agentRef, templateDef) {
      var agent = agentRef && typeof agentRef === 'object' ? agentRef : {};
      var id = String(agent.id || agentRef || '').trim();
      var name = String(agent.name || '').trim();
      var emoji = String((agent.identity && agent.identity.emoji) || '').trim();
      if (!emoji) {
        emoji = this.defaultFreshEmojiForAgent(id || name || 'agent');
      }
      if (templateDef && templateDef.category) {
        var category = String(templateDef.category).toLowerCase();
        if (category.indexOf('development') >= 0) emoji = '🧑\u200d💻';
        else if (category.indexOf('research') >= 0) emoji = '🔬';
        else if (category.indexOf('operations') >= 0 || category.indexOf('ops') >= 0) emoji = '🛠️';
        else if (category.indexOf('writing') >= 0) emoji = '📝';
      }
      emoji = this.sanitizeAgentEmojiForDisplay ? this.sanitizeAgentEmojiForDisplay(agent, emoji) : emoji;
      if (!emoji) emoji = '∞';
      return {
        name: name || String(id || '').trim(),
        emoji: String(emoji || '∞').trim() || '∞',
      };
    },

    toggleDrawerEmojiPicker: function() {
      this.drawerEmojiPickerOpen = !this.drawerEmojiPickerOpen;
      if (!this.drawerEmojiPickerOpen) {
        this.drawerEmojiSearch = '';
      } else {
        this.drawerAvatarUrlPickerOpen = false;
        this.drawerEditingEmoji = true;
      }
    },

    toggleDrawerAvatarUrlPicker: function() {
      this.drawerAvatarUrlPickerOpen = !this.drawerAvatarUrlPickerOpen;
      if (this.drawerAvatarUrlPickerOpen) {
        this.drawerEmojiPickerOpen = false;
        this.drawerAvatarUploadError = '';
        this.drawerAvatarUrlDraft = String(
          (this.drawerConfigForm && this.drawerConfigForm.avatar_url) ||
          (this.agentDrawer && this.agentDrawer.avatar_url) ||
          ''
        ).trim();
      } else {
        this.drawerAvatarUrlDraft = '';
      }
    },

    applyDrawerAvatarUrl: async function() {
      if (!this.agentDrawer || !this.agentDrawer.id) return;
      var draft = String(this.drawerAvatarUrlDraft || '').trim();
      if (!draft) {
        this.drawerAvatarUploadError = 'avatar_url_required';
        InfringToast.error('Avatar URL is required.');
        return;
      }
      var parsed = null;
      try {
        parsed = new URL(draft);
      } catch (_) {
        parsed = null;
      }
      if (!parsed || (parsed.protocol !== 'http:' && parsed.protocol !== 'https:')) {
        this.drawerAvatarUploadError = 'avatar_url_invalid';
        InfringToast.error('Avatar URL must start with http:// or https://');
        return;
      }
      if (!this.drawerConfigForm || typeof this.drawerConfigForm !== 'object') {
        this.drawerConfigForm = {};
      }
      var normalized = String(parsed.toString()).trim();
      this.drawerConfigForm.avatar_url = normalized;
      if (this.agentDrawer && typeof this.agentDrawer === 'object') {
        this.agentDrawer.avatar_url = normalized;
      }
      this.drawerAvatarUploadError = '';
      this.drawerEmojiPickerOpen = false;
      this.drawerAvatarUrlPickerOpen = false;
      this.drawerAvatarUrlDraft = '';
      this.drawerEditingEmoji = false;
      await this.saveDrawerIdentity('avatar');
    },

    selectDrawerEmoji: function(choice) {
      var emoji = String(choice && choice.emoji ? choice.emoji : choice || '').trim();
      if (!emoji) return;
      var sanitized = this.sanitizeAgentEmojiForDisplay
        ? this.sanitizeAgentEmojiForDisplay(this.agentDrawer || this.currentAgent, emoji)
        : emoji;
      if (!sanitized) {
        InfringToast.info('The gear icon is reserved for the System thread.');
        return;
      }
      if (!this.drawerConfigForm || typeof this.drawerConfigForm !== 'object') {
        this.drawerConfigForm = {};
      }
      this.drawerConfigForm.emoji = sanitized;
      // Choosing emoji explicitly switches away from image avatar mode.
      this.drawerConfigForm.avatar_url = '';
      if (this.agentDrawer && typeof this.agentDrawer === 'object') {
        this.agentDrawer.avatar_url = '';
      }
      this.drawerEmojiPickerOpen = false;
      this.drawerEmojiSearch = '';
      this.drawerEditingEmoji = false;
    },

    openDrawerAvatarPicker: function() {
      this.drawerAvatarUrlPickerOpen = false;
      this.drawerAvatarUrlDraft = '';
      if (this.$refs && this.$refs.drawerAvatarInput) {
        this.$refs.drawerAvatarInput.click();
      }
    },

    uploadDrawerAvatar: async function(fileList) {
      if (!this.agentDrawer || !this.agentDrawer.id) return;
      var files = Array.isArray(fileList) ? fileList : Array.from(fileList || []);
      if (!files.length) return;
      var file = files[0];
      if (!file) return;
      var mime = String(file.type || '').toLowerCase();
      if (mime && mime.indexOf('image/') !== 0) {
        InfringToast.error('Avatar must be an image file.');
        return;
      }
      this.drawerAvatarUploading = true;
      this.drawerAvatarUploadError = '';
      try {
        var headers = {
          'Content-Type': file.type || 'application/octet-stream',
          'X-Filename': file.name || 'avatar'
        };
        var token = (typeof InfringAPI !== 'undefined' && typeof InfringAPI.getToken === 'function')
          ? String(InfringAPI.getToken() || '')
          : '';
        if (token) headers.Authorization = 'Bearer ' + token;
        var response = await fetch('/api/agents/' + encodeURIComponent(this.agentDrawer.id) + '/avatar', {
          method: 'POST',
          headers: headers,
          body: file
        });
        var payload = null;
        try {
          payload = await response.json();
        } catch (_) {
          payload = null;
        }
        if (!response.ok || !payload || !payload.ok || !payload.avatar_url) {
          var reason = payload && payload.error ? payload.error : 'avatar_upload_failed';
          throw new Error(String(reason));
        }
        if (!this.drawerConfigForm || typeof this.drawerConfigForm !== 'object') {
          this.drawerConfigForm = {};
        }
        this.drawerConfigForm.avatar_url = String(payload.avatar_url || '').trim();
        this.agentDrawer.avatar_url = String(payload.avatar_url || '').trim();
        this.drawerEditingEmoji = false;
        this.drawerEmojiPickerOpen = false;
        this.drawerAvatarUrlPickerOpen = false;
        this.drawerAvatarUrlDraft = '';
        InfringToast.success('Avatar uploaded');
        await this.saveDrawerIdentity('avatar');
      } catch (error) {
        var message = (error && error.message) ? String(error.message) : 'avatar_upload_failed';
        this.drawerAvatarUploadError = message;
        InfringToast.error('Failed to upload avatar: ' + message);
      } finally {
        this.drawerAvatarUploading = false;
      }
    },

    async openAgentDrawer() {
      if (!this.currentAgent || !this.currentAgent.id) return;
      if (this.isSystemThreadAgent && this.isSystemThreadAgent(this.currentAgent)) return;
      if (this.isCurrentAgentArchived && this.isCurrentAgentArchived()) return;
      this.showAgentDrawer = true;
      this.agentDrawerLoading = true;
      this.drawerTab = 'info';
      this.drawerEditingModel = false;
      this.drawerEditingProvider = false;
      this.drawerEditingFallback = false;
      this.drawerEditingName = false;
      this.drawerEditingEmoji = false;
      this.drawerEmojiPickerOpen = false;
      this.drawerEmojiSearch = '';
      this.drawerAvatarUrlPickerOpen = false;
      this.drawerAvatarUrlDraft = '';
      this.drawerAvatarUploading = false;
      this.drawerAvatarUploadError = '';
      this.drawerIdentitySaving = false;
      this.drawerSavePending = false;
      this.drawerNewModelValue = '';
      this.drawerNewProviderValue = '';
      this.drawerNewFallbackValue = '';
      var base = this.resolveAgent(this.currentAgent) || this.currentAgent;
      this.agentDrawer = Object.assign({}, base, {
        _fallbacks: Array.isArray(base && base._fallbacks) ? base._fallbacks : []
      });
      this.drawerConfigForm = {
        name: this.agentDrawer.name || '',
        system_prompt: this.agentDrawer.system_prompt || '',
        emoji: this.sanitizeAgentEmojiForDisplay
          ? this.sanitizeAgentEmojiForDisplay(this.agentDrawer, (this.agentDrawer.identity && this.agentDrawer.identity.emoji) || '')
          : ((this.agentDrawer.identity && this.agentDrawer.identity.emoji) || ''),
        avatar_url: this.agentDrawer.avatar_url || '',
        color: (this.agentDrawer.identity && this.agentDrawer.identity.color) || '#2563EB',
        archetype: (this.agentDrawer.identity && this.agentDrawer.identity.archetype) || '',
        vibe: (this.agentDrawer.identity && this.agentDrawer.identity.vibe) || '',
      };
      try {
        var full = await InfringAPI.get('/api/agents/' + this.currentAgent.id);
        this.agentDrawer = Object.assign({}, base, full || {}, {
          _fallbacks: Array.isArray(full && full.fallback_models) ? full.fallback_models : []
        });
        this.drawerConfigForm = {
          name: this.agentDrawer.name || '',
          system_prompt: this.agentDrawer.system_prompt || '',
          emoji: this.sanitizeAgentEmojiForDisplay
            ? this.sanitizeAgentEmojiForDisplay(this.agentDrawer, (this.agentDrawer.identity && this.agentDrawer.identity.emoji) || '')
            : ((this.agentDrawer.identity && this.agentDrawer.identity.emoji) || ''),
          avatar_url: this.agentDrawer.avatar_url || '',
          color: (this.agentDrawer.identity && this.agentDrawer.identity.color) || '#2563EB',
          archetype: (this.agentDrawer.identity && this.agentDrawer.identity.archetype) || '',
          vibe: (this.agentDrawer.identity && this.agentDrawer.identity.vibe) || '',
        };
      } catch(e) {
        // Keep best-effort drawer data from current agent/store.
      } finally {
        this.agentDrawerLoading = false;
      }
    },

    closeAgentDrawer() {
      this.showAgentDrawer = false;
      this.drawerEditingName = false;
      this.drawerEditingEmoji = false;
      this.drawerEmojiPickerOpen = false;
      this.drawerEmojiSearch = '';
      this.drawerAvatarUrlPickerOpen = false;
      this.drawerAvatarUrlDraft = '';
      this.drawerAvatarUploadError = '';
    },

    toggleAgentDrawer() {
      if (this.isCurrentAgentArchived && this.isCurrentAgentArchived()) return;
      if (this.showAgentDrawer) {
        this.closeAgentDrawer();
        return;
      }
      this.openAgentDrawer();
    },

    async reviveCurrentArchivedAgent() {
      var agent = this.currentAgent && typeof this.currentAgent === 'object' ? this.currentAgent : null;
      if (!agent || !agent.id) return;
      if (!(this.isArchivedAgentRecord && this.isArchivedAgentRecord(agent))) return;
      var agentId = String(agent.id || '').trim();
      if (!agentId) return;
      try {
        await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(agentId) + '/revive', {
          role: String(agent.role || 'analyst')
        });
        this.currentAgent = Object.assign({}, agent, {
          archived: false,
          state: 'running'
        });
        var store = Alpine.store('app');
        if (store) {
          if (Array.isArray(store.agents)) {
            store.agents = store.agents.map(function(row) {
              if (!row || String((row && row.id) || '') !== agentId) return row;
              return Object.assign({}, row, { archived: false, state: 'running' });
            });
          }
          if (store.pendingAgent && String((store.pendingAgent && store.pendingAgent.id) || '') === agentId) {
            store.pendingAgent = null;
            store.pendingFreshAgentId = null;
          }
          if (Array.isArray(store.archivedAgentIds)) {
            store.archivedAgentIds = store.archivedAgentIds.filter(function(id) {
              return String(id || '') !== agentId;
            });
            if (typeof store.persistArchivedAgentIds === 'function') {
              store.persistArchivedAgentIds();
            } else {
              try {
                localStorage.setItem('infring-archived-agent-ids', JSON.stringify(store.archivedAgentIds));
              } catch(_) {}
            }
          }
          if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(agentId);
          else store.activeAgentId = agentId;
          if (typeof store.refreshAgents === 'function') {

            await store.refreshAgents({ force: true });
          }
        }
        var resolved = this.resolveAgent(agentId);
        if (resolved) {
          this.currentAgent = Object.assign({}, resolved, {
            archived: false,
            state: String(resolved.state || 'running')
          });
        } else if (this.currentAgent && String((this.currentAgent && this.currentAgent.id) || '') === agentId) {
          this.currentAgent = Object.assign({}, this.currentAgent, { archived: false, state: 'running' });
        }
        this.showAgentDrawer = false;
        this.showFreshArchetypeTiles = false;
        await this.loadSessions(agentId);
        await this.loadSession(agentId, false);
        this.requestContextTelemetry(true);
        InfringToast.success('Revived ' + (resolved && (resolved.name || resolved.id) ? (resolved.name || resolved.id) : agentId));
      } catch (e) {
        InfringToast.error('Failed to revive archived agent: ' + (e && e.message ? e.message : 'unknown_error'));
      }
    },

    async syncDrawerAgentAfterChange() {
      if (!this.agentDrawer || !this.agentDrawer.id) return;
      try {
        await Alpine.store('app').refreshAgents();
      } catch {}
      var refreshed = this.resolveAgent(this.agentDrawer.id);
      if (refreshed) {
        this.currentAgent = refreshed;
      }
      await this.openAgentDrawer();
    },

    normalizeDrawerPermissionValue(raw) {
      if (typeof raw === 'number' && Number.isFinite(raw)) return raw < 0 ? -1 : (raw > 0 ? 1 : 0);
      if (typeof raw === 'boolean') return raw ? 1 : -1;
      var lowered = String(raw == null ? '' : raw).trim().toLowerCase();
      if (!lowered) return 0;
      if (lowered === 'allow' || lowered === 'true' || lowered === '1' || lowered === '+1') return 1;
      if (lowered === 'deny' || lowered === 'false' || lowered === '-1') return -1;
      if (lowered === 'inherit' || lowered === '0') return 0;
      return 0;
    },

    normalizeDrawerPermissionKey(raw) {
      var key = String(raw == null ? '' : raw).trim().toLowerCase();
      if (!key) return '';
      key = key.replace(/\s+/g, '.');
      key = key.replace(/[^a-z0-9._:-]/g, '');
      key = key.replace(/\.{2,}/g, '.');
      key = key.replace(/^\.+|\.+$/g, '');
      if (key.length > 128) key = key.slice(0, 128);
      if (key.indexOf('.') <= 0) return '';
      return key;
    },

    resolveDrawerPermissionsManifest() {
      var row = this.agentDrawer && typeof this.agentDrawer === 'object' ? this.agentDrawer : {};
      var contract = row.contract && typeof row.contract === 'object' ? row.contract : {};
      var source = (contract.permissions_manifest && typeof contract.permissions_manifest === 'object')
        ? contract.permissions_manifest
        : ((row.permissions_manifest && typeof row.permissions_manifest === 'object') ? row.permissions_manifest : {});
      var catalog = Array.isArray(this.freshInitPermissionCatalog) ? this.freshInitPermissionCatalog : [];
      var defaultsSource = (source.category_defaults && typeof source.category_defaults === 'object')
        ? source.category_defaults
        : ((source.categories && typeof source.categories === 'object') ? source.categories : {});
      var grantsSource = (source.grants && typeof source.grants === 'object') ? source.grants : {};
      var out = {
        version: 1,
        trit: { deny: -1, inherit: 0, allow: 1 },
        category_defaults: {},
        grants: {}
      };
      var grantCount = 0;
      var maxGrantCount = 2048;
      for (var ci = 0; ci < catalog.length; ci += 1) {
        var category = String((catalog[ci] && catalog[ci].category) || '').trim().toLowerCase();
        if (!category) continue;
        out.category_defaults[category] = this.normalizeDrawerPermissionValue(defaultsSource[category]);
      }
      Object.keys(grantsSource || {}).forEach(function(key) {
        if (grantCount >= maxGrantCount) return;
        var permissionKey = this.normalizeDrawerPermissionKey(key);
        if (!permissionKey || Object.prototype.hasOwnProperty.call(out.grants, permissionKey)) return;
        out.grants[permissionKey] = this.normalizeDrawerPermissionValue(grantsSource[key]);
        grantCount += 1;
      }, this);
      Object.keys(source || {}).forEach(function(key) {
        if (grantCount >= maxGrantCount) return;
        var permissionKey = this.normalizeDrawerPermissionKey(key);
        if (!permissionKey || Object.prototype.hasOwnProperty.call(out.grants, permissionKey)) return;
        out.grants[permissionKey] = this.normalizeDrawerPermissionValue(source[key]);
        grantCount += 1;
      }, this);
      for (var i = 0; i < catalog.length; i += 1) {
        if (grantCount >= maxGrantCount) break;
        var section = catalog[i] || {};
        var permissions = Array.isArray(section.permissions) ? section.permissions : [];
        for (var j = 0; j < permissions.length; j += 1) {
          if (grantCount >= maxGrantCount) break;
          var key = this.normalizeDrawerPermissionKey((permissions[j] && permissions[j].key) || '');
          if (!key || Object.prototype.hasOwnProperty.call(out.grants, key)) continue;
          out.grants[key] = 0;
          grantCount += 1;
        }
      }
      var webSearchKey = this.normalizeDrawerPermissionKey('web.search.basic');
      out.grants[webSearchKey] = out.grants[webSearchKey] < 0 ? 0 : 1;
      return out;
    },

    ensureDrawerPermissionsManifest() {
      var manifest = this.resolveDrawerPermissionsManifest();
      if (!this.agentDrawer || typeof this.agentDrawer !== 'object') this.agentDrawer = {};
      if (!this.agentDrawer.contract || typeof this.agentDrawer.contract !== 'object') this.agentDrawer.contract = {};
      this.agentDrawer.permissions_manifest = manifest;
      this.agentDrawer.contract.permissions_manifest = manifest;
      return manifest;
    },

    drawerPermissionLabelForKey(permissionKey) {
      var key = this.normalizeDrawerPermissionKey(permissionKey);
      if (!key) return '';
      var rows = Array.isArray(this.freshInitPermissionCatalog) ? this.freshInitPermissionCatalog : [];
      for (var i = 0; i < rows.length; i += 1) {
        var perms = Array.isArray(rows[i] && rows[i].permissions) ? rows[i].permissions : [];
        for (var j = 0; j < perms.length; j += 1) {
          if (this.normalizeDrawerPermissionKey((perms[j] && perms[j].key) || '') === key) {
            return String((perms[j] && perms[j].label) || key).trim() || key;
          }
        }
      }
      return key;
    },

    drawerPermissionRows() {
      var manifest = this.resolveDrawerPermissionsManifest();
      var grants = (manifest && manifest.grants && typeof manifest.grants === 'object') ? manifest.grants : {};
      var out = [];
      var byCategory = {};
      var catalog = Array.isArray(this.freshInitPermissionCatalog) ? this.freshInitPermissionCatalog : [];
      for (var i = 0; i < catalog.length; i += 1) {
        var categoryId = String((catalog[i] && catalog[i].category) || '').trim().toLowerCase();
        if (!categoryId || byCategory[categoryId]) continue;
        var name = String((catalog[i] && catalog[i].name) || categoryId).trim() || categoryId;
        byCategory[categoryId] = { category: categoryId, name: name, permissions: [] };
        out.push(byCategory[categoryId]);
      }
      var keys = Object.keys(grants || {}).sort(function(left, right) {
        return String(left || '').localeCompare(String(right || ''));
      });
      var seenByCategory = {};
      for (var k = 0; k < keys.length; k += 1) {
        var key = this.normalizeDrawerPermissionKey(keys[k]);
        if (!key) continue;
        var category = key.split('.')[0] || 'other';
        if (!byCategory[category]) {
          byCategory[category] = {
            category: category,
            name: category.charAt(0).toUpperCase() + category.slice(1),
            permissions: []
          };
          out.push(byCategory[category]);
        }
        if (!seenByCategory[category]) seenByCategory[category] = {};
        if (seenByCategory[category][key]) continue;
        seenByCategory[category][key] = true;
        byCategory[category].permissions.push({ key: key, label: this.drawerPermissionLabelForKey(key) });
      }
      return out.filter(function(section) {
        return section && Array.isArray(section.permissions) && section.permissions.length > 0;
      });
    },

    drawerPermissionState(permissionKey) {
      var key = this.normalizeDrawerPermissionKey(permissionKey);
      if (!key) return 0;
      var manifest = this.resolveDrawerPermissionsManifest();
      var grants = manifest && manifest.grants && typeof manifest.grants === 'object' ? manifest.grants : {};
      return this.normalizeDrawerPermissionValue(grants[key]);
    },

    drawerPermissionStateLabel(rawValue) {
      var value = this.normalizeDrawerPermissionValue(rawValue);
      if (value > 0) return 'Allowed';
      if (value < 0) return 'No access';
      return 'Inherited';
    },

    drawerPermissionStateClass(rawValue) {
      var value = this.normalizeDrawerPermissionValue(rawValue);
      if (value > 0) return 'perm-state-allow';
      if (value < 0) return 'perm-state-deny';
      return 'perm-state-inherit';
    },

    drawerPermissionDescriptionForKey(permissionKey) {
      var key = String(permissionKey || '').trim();
      if (!key) return '';
      var tokens = key.split('.').map(function(part) { return String(part || '').trim().toLowerCase(); }).filter(Boolean);
      if (tokens.length < 2) return 'Scope key: ' + key;
      var verb = tokens[1];
      var subjectTokens = tokens.slice(2).map(function(part) {
        return part.replace(/_/g, ' ');
      });
      var subject = subjectTokens.join(' ').trim() || 'this scope';
      if (verb === 'read') return 'Read access to ' + subject + '.';
      if (verb === 'write') return 'Write access to ' + subject + '.';
      if (verb === 'delete') return 'Delete access to ' + subject + '.';
      if (verb === 'search') return 'Search access for ' + subject + '.';
      if (verb === 'fetch') return 'Fetch access for ' + subject + '.';
      if (verb === 'create') return 'Create access for ' + subject + '.';
      if (verb === 'exec') return 'Execution access for ' + subject + '.';
      if (verb === 'spawn') return 'Can spawn child agents.';
      if (verb === 'manage') return 'Can manage ' + subject + '.';
      return 'Scope key: ' + key;
    },

    drawerPermissionCategoryState(section) {
      var perms = Array.isArray(section && section.permissions) ? section.permissions : [];
      var allow = 0;
      var inherit = 0;
      var deny = 0;
      for (var i = 0; i < perms.length; i += 1) {
        var value = this.drawerPermissionState(perms[i] && perms[i].key);
        if (value > 0) allow += 1;
        else if (value < 0) deny += 1;
        else inherit += 1;
      }
      return {
        allow: allow,
        inherit: inherit,
        deny: deny,
        total: perms.length
      };
    },

    setDrawerPermissionState(permissionKey, nextValue) {
      var key = this.normalizeDrawerPermissionKey(permissionKey);
      if (!key) return;
      var manifest = this.ensureDrawerPermissionsManifest();
      manifest.grants[key] = this.normalizeDrawerPermissionValue(nextValue);
      this.agentDrawer.permissions_manifest = manifest;
      this.agentDrawer.contract.permissions_manifest = manifest;
    },

    setDrawerPermissionCategoryState(categoryId, nextValue) {
      var category = String(categoryId || '').trim().toLowerCase();
      if (!category) return;
      var manifest = this.ensureDrawerPermissionsManifest();
      var rows = this.drawerPermissionRows();
      for (var i = 0; i < rows.length; i += 1) {
        if (String((rows[i] && rows[i].category) || '').trim().toLowerCase() !== category) continue;
        var perms = Array.isArray(rows[i].permissions) ? rows[i].permissions : [];
        for (var j = 0; j < perms.length; j += 1) {
          var key = this.normalizeDrawerPermissionKey((perms[j] && perms[j].key) || '');
          if (!key) continue;
          manifest.grants[key] = this.normalizeDrawerPermissionValue(nextValue);
        }
      }
      this.agentDrawer.permissions_manifest = manifest;
      this.agentDrawer.contract.permissions_manifest = manifest;
    },

    drawerPermissionChecked(permissionKey) {
      return this.drawerPermissionState(permissionKey) >= 0;
    },

    setDrawerPermissionChecked(permissionKey, checked) {
      var key = this.normalizeDrawerPermissionKey(permissionKey);
      if (!key) return;
      var current = this.drawerPermissionState(key);
      this.setDrawerPermissionState(key, checked ? (current < 0 ? 0 : current) : -1);
    },

    async setDrawerMode(mode) {
      if (!this.agentDrawer || !this.agentDrawer.id) return;
      try {
        await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(this.agentDrawer.id) + '/mode', { mode: mode });
        InfringToast.success('Mode set to ' + mode);
        await this.syncDrawerAgentAfterChange();
      } catch(e) {
        InfringToast.error('Failed to set mode: ' + e.message);
      }
    },

    async saveDrawerAll() {
      if (!this.agentDrawer || !this.agentDrawer.id || this.drawerSavePending) return;
      var agentId = this.agentDrawer.id;
      var previousName = String((this.agentDrawer && this.agentDrawer.name) || (this.currentAgent && this.currentAgent.name) || '').trim();
      var requestedName = String((this.drawerConfigForm && this.drawerConfigForm.name) || '').trim();
      var previousFallbacks = Array.isArray(this.agentDrawer._fallbacks) ? this.agentDrawer._fallbacks.slice() : [];
      var appendedFallback = false;
      this.drawerSavePending = true;
      this.drawerConfigSaving = true;
      this.drawerModelSaving = true;
      this.drawerIdentitySaving = true;
      try {
        var configPayload = Object.assign({}, this.drawerConfigForm || {});
        configPayload.permissions_manifest = this.resolveDrawerPermissionsManifest();
        if (this.drawerEditingFallback && String(this.drawerNewFallbackValue || '').trim()) {
          var fallbackParts = String(this.drawerNewFallbackValue || '').trim().split('/');
          var fallbackProvider = fallbackParts.length > 1 ? fallbackParts[0] : this.agentDrawer.model_provider;
          var fallbackModel = fallbackParts.length > 1 ? fallbackParts.slice(1).join('/') : fallbackParts[0];

    isBlockedTool: function(tool) {
      if (!tool) return false;
      if (tool.blocked === true) return true;
      var txt = String(this.toolRawResultTextIfLoaded(tool) || this.toolProjectionPreviewText(tool) || '').toLowerCase();
      if (String(tool.status || '').toLowerCase() === 'blocked') return true;
      if (!tool.is_error) return false;
      return (
        txt.indexOf('blocked') >= 0 ||
        txt.indexOf('policy') >= 0 ||
        txt.indexOf('denied') >= 0 ||
        txt.indexOf('not allowed') >= 0 ||
        txt.indexOf('forbidden') >= 0 ||
        txt.indexOf('approval') >= 0 ||
        txt.indexOf('permission') >= 0 ||
        txt.indexOf('fail-closed') >= 0
      );
    },

    isToolSuccessful: function(tool) {
      if (!tool) return false;
      if (tool.running) return false;
      if (this.isBlockedTool(tool)) return false;
      if (tool.is_error) return false;
      return true;
    },

  isThoughtTool: function(tool) {
    var row = tool && typeof tool === 'object' ? tool : null;
    if (!row) return false;
    var name = String(row.name || '').toLowerCase();
    return !!(
      name === 'thought_process' ||
      row.agent_decision_dialog ||
      row.agent_runtime_decision_dialog ||
      row.agent_runtime_activity_trace ||
      row.projection_kind === 'decision_dialog'
    );
  },

    toolNameKey: function(tool) {
      if (!tool) return '';
      return String(tool.name || '')
        .trim()
        .toLowerCase()
        .replace(/[\s-]+/g, '_');
    },

    toolInputPayload: function(tool) {
      if (!tool || typeof tool !== 'object') return null;
      var raw = String(tool._detail_loaded ? (tool.input || tool.input_preview || '') : (tool.input_preview || '')).trim();
      if (!raw) return null;
      if (raw.indexOf('<function=') >= 0 && raw.indexOf('{') >= 0) {
        raw = raw.slice(raw.indexOf('{')).trim();
      }
      if (!(raw.charAt(0) === '{' || raw.charAt(0) === '[')) return null;
      try {
        var parsed = JSON.parse(raw);
        return parsed && typeof parsed === 'object' ? parsed : null;
      } catch (_) {
        return null;
      }
    },

    toolPayloadCount: function(payload, keys) {
      if (!payload || typeof payload !== 'object') return 0;
      var list = Array.isArray(keys) ? keys : [];
      for (var i = 0; i < list.length; i++) {
        var key = list[i];
        if (!Object.prototype.hasOwnProperty.call(payload, key)) continue;
        var value = payload[key];
        if (Array.isArray(value)) return value.length;
        if (typeof value === 'number' && Number.isFinite(value)) return Math.max(0, Math.round(value));
        if (typeof value === 'string' && value.trim()) return 1;
      }
      return 0;
    },

    prettifyToolLabel: function(value) {
      var raw = String(value || '').trim();
      if (!raw) return 'tool';
      var normalized = raw
        .replace(/[_-]+/g, ' ')
        .replace(/\s+/g, ' ')
        .trim();
      if (!normalized) return 'tool';
      return normalized
        .split(' ')
        .map(function(token) {
          return token ? token.charAt(0).toUpperCase() + token.slice(1) : token;
        })
        .join(' ');
    },

    toolActionName: function(tool) {
      var payload = this.toolInputPayload(tool);
      if (!payload || typeof payload !== 'object') return '';
      return String(
        payload.action ||
        payload.method ||
        payload.operation ||
        payload.op ||
        ''
      ).trim();
    },

    toolDisplayName: function(tool) {
      if (!tool) return 'tool';
      if (this.isThoughtTool(tool)) return 'thought';
      var key = this.toolNameKey(tool);
      var actionName = this.toolActionName(tool);
      switch (key) {
        case 'web_search':
        case 'search_web':
        case 'search':
        case 'web_query':
        case 'batch_query':
          return 'Web search';
        case 'web_fetch':
        case 'browse':
        case 'web_conduit_fetch':
          return 'Web fetch';
        case 'file_read':
        case 'read_file':
        case 'file':
          return 'File read';
        case 'folder_export':
        case 'list_folder':
        case 'folder_tree':
        case 'folder':
          return 'Folder export';
        case 'terminal_exec':
        case 'run_terminal':
        case 'terminal':
        case 'shell_exec':
          return 'Terminal command';
        case 'spawn_subagents':
        case 'spawn_swarm':
        case 'agent_spawn':
        case 'sessions_spawn':
          return 'Swarm spawn';
        case 'memory_semantic_query':
          return 'Memory query';
        case 'cron_schedule':
          return 'Schedule task';
        case 'cron_run':
          return 'Run scheduled task';
        case 'cron_list':
          return 'List schedules';
        case 'session_rollback_last_turn':
          return 'Undo last turn';
        case 'slack':
          return actionName ? ('Slack ' + this.prettifyToolLabel(actionName)) : 'Slack';
        case 'gmail':
          return actionName ? ('Gmail ' + this.prettifyToolLabel(actionName)) : 'Gmail';
        case 'github':
          return actionName ? ('GitHub ' + this.prettifyToolLabel(actionName)) : 'GitHub';
        case 'notion':
          return actionName ? ('Notion ' + this.prettifyToolLabel(actionName)) : 'Notion';
        default:
          return this.prettifyToolLabel(String(tool.name || 'tool'));
      }
    },

    toolThinkingActionLabel: function(tool) {
      if (!tool) return '';
      if (this.isThoughtTool(tool)) return 'Thinking';
      var key = this.toolNameKey(tool);
      var payload = this.toolInputPayload(tool);
      switch (key) {
        case 'web_search':
        case 'search_web':
        case 'search':
        case 'web_query':
          return 'Searching internet';
        case 'web_fetch':
        case 'browse':
        case 'web_conduit_fetch':
          return 'Reading web pages';
        case 'file_read':
        case 'read_file':
        case 'file':
          var fileCount = this.toolPayloadCount(payload, ['paths', 'files', 'file_paths', 'targets', 'path', 'file']);
          if (fileCount > 1) return 'Scanning ' + fileCount + ' files';
          if (fileCount === 1) return 'Scanning 1 file';
          return 'Scanning files';
        case 'folder_export':
        case 'list_folder':
        case 'folder_tree':
        case 'folder':
          var folderCount = this.toolPayloadCount(payload, ['folders', 'paths', 'targets', 'path', 'folder']);
          if (folderCount > 1) return 'Scanning ' + folderCount + ' folders';
          if (folderCount === 1) return 'Scanning 1 folder';
          return 'Scanning folders';
        case 'terminal_exec':
        case 'run_terminal':
        case 'terminal':
        case 'shell_exec':
          return 'Running terminal command';
        case 'spawn_subagents':
        case 'spawn_swarm':
        case 'agent_spawn':
        case 'sessions_spawn':
          var spawnCount = this.toolPayloadCount(payload, ['count', 'agent_count', 'num_agents', 'agents']);
          if (spawnCount > 0) return 'Summoning ' + spawnCount + ' agents';
          return 'Summoning agents';
        case 'memory_semantic_query':
          return 'Searching memory';
        case 'cron_schedule':
          return 'Scheduling follow-up work';
        case 'cron_run':
          return 'Running scheduled work';
        case 'cron_list':
          return 'Checking schedules';
        case 'session_rollback_last_turn':
          return 'Rewinding the last turn';
        default:
          return 'Running ' + this.toolDisplayName(tool);
      }
    },

    ensureStreamingToolCard: function(msg, toolName, toolInput, options) {
      if (!msg || typeof msg !== 'object') return null;
      if (!Array.isArray(msg.tools)) msg.tools = [];
      var name = String(toolName || '').trim();
      if (!name) name = 'tool';
      var opts = options && typeof options === 'object' ? options : {};
      var identity = typeof this.toolAttemptIdentity === 'function'
        ? this.toolAttemptIdentity({ name: name, attempt_id: opts.attempt_id || '', attempt_sequence: opts.attempt_sequence || (msg.tools.length + 1), tool_attempt_receipt: opts.tool_attempt_receipt || null }, msg.tools.length, 'stream-tool')
        : { id: name + '-' + Date.now(), attempt_id: '', attempt_sequence: (msg.tools.length + 1), identity_key: name.toLowerCase() };
      var markRunning = opts.running !== false;
      var allowCreate = opts.no_create !== true;
      for (var i = msg.tools.length - 1; i >= 0; i--) {
        var card = msg.tools[i];
        if (!card) continue;
        var matchesIdentity = String(card.identity_key || '').trim() && String(card.identity_key || '').trim() === String(identity.identity_key || '').trim();
        if (!matchesIdentity && String(card.name || '') !== name) continue;
        if (markRunning && card.running) {
          if (typeof toolInput === 'string') card.input_preview = toolInput;
          if (identity.id) card.id = identity.id;
          if (identity.attempt_id) card.attempt_id = identity.attempt_id; if (identity.attempt_sequence) card.attempt_sequence = identity.attempt_sequence; if (identity.identity_key) card.identity_key = identity.identity_key;
          return card;
        }
        if (!markRunning && card.running) {
          if (typeof toolInput === 'string') card.input_preview = toolInput;
          if (identity.id) card.id = identity.id;
          if (identity.attempt_id) card.attempt_id = identity.attempt_id; if (identity.attempt_sequence) card.attempt_sequence = identity.attempt_sequence; if (identity.identity_key) card.identity_key = identity.identity_key;
          card.running = false;
          return card;
        }
      }
      if (!allowCreate) return null;
      var created = { id: identity.id, name: name, running: markRunning, expanded: false, input_preview: typeof toolInput === 'string' ? toolInput : '', result_preview: '', is_error: false, attempt_id: identity.attempt_id, attempt_sequence: identity.attempt_sequence, identity_key: identity.identity_key };
      msg.tools.push(created);
      return created;
    },

    currentToolDialogLabel: function(msg) {
      if (!msg || !Array.isArray(msg.tools) || !msg.tools.length) return '';
      for (var i = msg.tools.length - 1; i >= 0; i--) {
        var tool = msg.tools[i];
        if (!tool || this.isThoughtTool(tool) || !tool.running) continue;
        if (tool.agent_runtime_live_activity || tool.agent_activity_event) {
          var liveLabel = String(tool.display_text || tool.summary || tool.result_preview || tool.output_preview || tool.name || '').trim();
          if (liveLabel) return liveLabel.split('\n')[0].replace(/\s+/g, ' ').slice(0, 220);
        }
        return this.toolThinkingActionLabel(tool);
      }
      return '';
    },

    hasRunningActionableTools: function(msg) {
      if (!msg || !Array.isArray(msg.tools) || !msg.tools.length) return false;
      return msg.tools.some(function(tool) { return !!(tool && !this.isThoughtTool(tool) && tool.running); }, this);
    },

    clearTransientThinkingRows: function(options) {
      var opts = options && typeof options === 'object' ? options : {}, force = opts.force === true;
      var preserveRunningTools = !force && opts.preserve_running_tools !== false;
      var pendingAgentId = !force && opts.preserve_pending_ws !== false && this._pendingWsRequest && this._pendingWsRequest.agent_id ? String(this._pendingWsRequest.agent_id || '').trim() : '';
      var rows = Array.isArray(this.messages) ? this.messages : []; if (!rows.length) return 0;
      var kept = [], now = Date.now(), keptPending = false;
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i];
        if (!row || (!row.thinking && !row.streaming)) { kept.push(row); continue; }
        var rowAgentId = String(row.agent_id || '').trim();
        var keep = (preserveRunningTools && this.hasRunningActionableTools(row)) || (!!pendingAgentId && (!rowAgentId || rowAgentId === pendingAgentId));
        if (!keep) continue;
        if (pendingAgentId && (!rowAgentId || rowAgentId === pendingAgentId)) keptPending = true;
        row.thinking = true; row.streaming = true; row._stream_updated_at = now;
        if (!Number.isFinite(Number(row._stream_started_at))) row._stream_started_at = now;
        if (!String(row.thinking_status || '').trim()) {
          var label = typeof this.currentToolDialogLabel === 'function' ? String(this.currentToolDialogLabel(row) || '').trim() : '';
          if (label) row.thinking_status = label;
        }
        kept.push(row);
      }
      if (typeof this.replaceActiveChatMessages === 'function') this.replaceActiveChatMessages(kept);
      else this.messages = kept;
      if (!force && pendingAgentId && !keptPending && typeof this.ensureLiveThinkingRow === 'function') {
        var restored = this.ensureLiveThinkingRow({ agent_id: pendingAgentId, agent_name: this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '' });
        if (restored) {
          restored.thinking = true; restored.streaming = true; restored._stream_updated_at = now;
          if (!Number.isFinite(Number(restored._stream_started_at))) restored._stream_started_at = now;
        }
      }
      return Math.max(0, rows.length - this.messages.length);
    },

    thoughtToolDurationSeconds: function(tool) {
      if (!tool || typeof tool !== 'object') return 0;
      var ms = Number(tool.duration_ms || tool.durationMs || tool.elapsed_ms || 0);
      if (!Number.isFinite(ms) || ms < 0) ms = 0;
      var seconds = Math.round(ms / 1000);
      if (ms > 0 && seconds < 1) seconds = 1;
      return Math.max(0, seconds);
    },

    thoughtToolLabel: function(tool) {
      if (tool && (tool.agent_decision_dialog || tool.agent_runtime_decision_dialog || tool.agent_runtime_activity_trace)) {
        var seconds = this.thoughtToolDurationSeconds(tool);
        var hours = Math.floor(seconds / 3600);
        var minutes = Math.floor((seconds % 3600) / 60);
        var remaining = seconds % 60;
        var parts = [];
        if (hours > 0) parts.push(hours + 'h');
        if (hours > 0 || minutes > 0) parts.push(minutes + 'm');
        parts.push(remaining + 's');
        return 'Worked for ' + parts.join(' ');
      }
      return 'Thought for ' + this.thoughtToolDurationSeconds(tool) + ' seconds';
    },

    toolRawResultTextIfLoaded: function(tool) {
      var row = tool && typeof tool === 'object' ? tool : {};
      if (!row._detail_loaded || row.result == null) return '';
      if (typeof row.result === 'string') return row.result;
      try {
        return JSON.stringify(row.result);
      } catch (_) {
        return '';
      }
    },

    toolProjectionPreviewText: function(tool) {
      var row = tool && typeof tool === 'object' ? tool : {};
      var raw = this.toolRawResultTextIfLoaded(row);
      if (raw) return raw;
      return String(row.summary || row.result_preview || row.output_preview || row.display_text || row.status || '').trim();
    },

    toolStatusText: function(tool) {
      if (!tool) return '';
      if (tool.running) return 'running...';
      if (tool._detail_loading) return 'loading detail...';
      if (tool.agent_runtime_activity_trace) return '';
      if (this.isThoughtTool(tool)) return 'thought';
      if (this.isBlockedTool(tool)) return 'blocked';
      if (tool.is_error) return 'error';
      var loadedResult = this.toolRawResultTextIfLoaded(tool);
      if (loadedResult) {
        return loadedResult.length > 500 ? Math.round(loadedResult.length / 1024) + 'KB' : 'done';
      }
      return 'done';
    },

    // Mark chat-rendered error messages for styling
    isErrorMessage: function(msg) {
      if (!msg || !msg.text) return false;
      if (String(msg.role || '').toLowerCase() !== 'system') return false;
      var t = String(msg.text).trim().toLowerCase();
      return t.startsWith('error:');
    },

    messageHasTools: function(msg) {
      return !!(msg && Array.isArray(msg.tools) && msg.tools.length);
    },

    allToolsCollapsed: function(msg) {
      if (!this.messageHasTools(msg)) return true;
      return !msg.tools.some(function(tool) {
        return !!(tool && tool.expanded);
      });
    },

    toggleMessageTools: function(msg) {
      if (!this.messageHasTools(msg)) return;
      var expand = this.allToolsCollapsed(msg);
      msg.tools.forEach(function(tool) {
        if (tool) tool.expanded = expand;
      });
      this.scheduleConversationPersist();
    },

    formatToolOutputForClipboard: function(text) {
      var raw = String(text == null ? '' : text);
      var trimmed = raw.trim();
      if (!trimmed) return '';
      if (trimmed.charAt(0) === '{' || trimmed.charAt(0) === '[') {
        try {
          return '```json\n' + JSON.stringify(JSON.parse(trimmed), null, 2) + '\n```';
        } catch (_) {}
      }
      return raw;
    },

    truncateToolOutputPreview: function(text) {
      var raw = String(text == null ? '' : text).trim();
      if (!raw) return '';
      var allLines = raw.split('\n');
      var maxLines = Number(this.toolPreviewMaxLines || 0);
      if (!Number.isFinite(maxLines) || maxLines < 1) maxLines = 2;
      var maxChars = Number(this.toolPreviewMaxChars || 0);
      if (!Number.isFinite(maxChars) || maxChars < 24) maxChars = 100;
      var preview = allLines.slice(0, maxLines).join('\n');
      if (preview.length > maxChars) return preview.slice(0, maxChars).trimEnd() + '…';
      return allLines.length > maxLines ? preview.trimEnd() + '…' : preview;
    },

    toolProjectionSections: function(tool) {
      var row = tool && typeof tool === 'object' ? tool : {};
      var sections = [];
      if (row._detail_loading) {
        sections.push({ id: 'loading', label: 'Detail', text: 'Loading detail…' });
        return sections;
      }
      if (row._detail_error) {
        sections.push({ id: 'error', label: 'Detail', text: String(row._detail_error || 'Unable to load detail right now.') });
        return sections;
      }
      if (row.agent_decision_dialog || row.agent_runtime_decision_dialog || row.agent_runtime_activity_trace) {
        var dialog = String(
          row.agent_decision_dialog_text ||
          row.input ||
          row.input_preview ||
          row.result ||
          row.result_preview ||
          row.summary ||
          row.display_text ||
          ''
        ).trim();
        if (dialog) {
          sections.push({
            id: 'agent-decision-dialog',
            label: 'Agent decision dialog',
            text: this.formatToolOutputForClipboard(dialog) || dialog
          });
        }
        return sections;
      }
      var summary = String(row.summary || row.display_text || '').trim();
      if (summary) sections.push({ id: 'summary', label: 'Summary', text: summary });
      var input = String(row._detail_loaded ? (row.input || row.input_preview || '') : (row.input_preview || '')).trim();
      if (input) sections.push({ id: 'input', label: 'Input', text: this.formatToolOutputForClipboard(input) || input });
      var result = String(this.toolProjectionPreviewText(row) || '').trim();
      if (result) sections.push({ id: 'result', label: 'Result', text: this.formatToolOutputForClipboard(result) || result });
      if (!sections.length && this.shellDetailRefForTool(row)) {
        sections.push({ id: 'detail-ref', label: 'Detail', text: 'Open the tool to load detail from the gateway.' });
      }
      return sections;
    },

    messageCopyMarkdown: function(msg) {
      var row = msg && typeof msg === 'object' ? msg : {};
      var parts = [];
      var label = typeof this.messageActorLabel === 'function'
        ? String(this.messageActorLabel(row) || '').trim()
        : String(row.role || 'Message').trim();
      var stamp = typeof this.messageTimestampLabel === 'function' ? String(this.messageTimestampLabel(row) || '').trim() : '';
      if (label) parts.push('**' + label + '**');
      if (stamp) parts.push('_' + stamp + '_');

      var text = '';
      if (typeof this.extractMessageVisibleText === 'function') {
        text = String(this.extractMessageVisibleText(row) || '').trim();
      }
      if (!text && typeof this.messageVisiblePreviewText === 'function') {
        text = String(this.messageVisiblePreviewText(row) || '').trim();
      }
      if (!text) text = String(row.text || '').trim();
      if (text) parts.push(text);

      if (row.notice_label) {
        var notice = String(row.notice_label || '').trim();
        if (notice) parts.push('Notice: ' + notice);
      }

      var toolLines = [];
      var tools = Array.isArray(row.tools) ? row.tools : [];
      for (var i = 0; i < tools.length; i += 1) {
        var tool = tools[i] || {};
        var toolName = this.toolDisplayName(tool);
        var status = String(tool.status || '').trim();
        var rendered = this.formatToolOutputForClipboard(this.toolProjectionPreviewText(tool) || '');
        var preview = rendered ? this.truncateToolOutputPreview(rendered) : '';
        var line = '- ' + toolName;
        if (status) line += ' (' + status + ')';
        if (preview) line += ': ' + preview;
        toolLines.push(line);
      }
      if (toolLines.length) {
        parts.push('');
        parts.push('Tools:');
        for (var j = 0; j < toolLines.length; j += 1) parts.push(toolLines[j]);
      }

      if (row.file_output && row.file_output.path) parts.push('', 'File: `' + String(row.file_output.path).trim() + '`');
      if (row.folder_output && row.folder_output.path) parts.push('', 'Folder: `' + String(row.folder_output.path).trim() + '`');

      return parts.filter(function(part, idx, arr) {
        if (part !== '') return true;
        return idx > 0 && arr[idx - 1] !== '';
      }).join('\n').trim();
    },

    // Copy message text to clipboard as markdown
    copyMessage: function(msg) {
      if (!msg || msg._copying) return;
      var text = this.messageCopyMarkdown(msg);
      if (!text || !navigator.clipboard || typeof navigator.clipboard.writeText !== 'function') {
        InfringToast.error('Copy failed.');
        return;
      }
      msg._copying = true;
      navigator.clipboard.writeText(text).then(function() {
        msg._copying = false;
        msg._copied = true;
        setTimeout(function() { msg._copied = false; }, 1500);
      }).catch(function() {
        msg._copying = false;
        InfringToast.error('Copy failed.');
      });
    },

    prefersReducedMotion: function() {
      if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return false;
      try {
        return !!window.matchMedia('(prefers-reduced-motion: reduce)').matches;
      } catch (_) {
        return false;
      }
    },

    captureComposerSendMorph: function(textInput) {
      if (this.prefersReducedMotion() || this.terminalMode || this.showFreshArchetypeTiles) return null;
      if (typeof document === 'undefined') return null;
      var shell = document.querySelector('.input-row .composer-shell');
      var input = document.getElementById('msg-input');
      if (!shell || !input) return null;
      var text = String(textInput == null ? '' : textInput).trim();
      if (!text) return null;
      var rect = input.getBoundingClientRect();
      if (!(rect.width > 80 && rect.height > 24)) return null;
      var ghost = document.createElement('div');
      ghost.className = 'composer-send-morph-ghost';
      ghost.textContent = text.length > 260 ? (text.slice(0, 257) + '...') : text;
      ghost.style.left = rect.left + 'px';
      ghost.style.top = rect.top + 'px';
      ghost.style.width = rect.width + 'px';
      ghost.style.minHeight = rect.height + 'px';
      document.body.appendChild(ghost);
      shell.classList.add('composer-shell-send-morph');
      return { shell: shell, ghost: ghost };
    },

    clearComposerSendMorph: function(snapshot) {
      if (!snapshot || typeof snapshot !== 'object') return;
      if (snapshot.shell && snapshot.shell.classList) snapshot.shell.classList.remove('composer-shell-send-morph');
      if (snapshot.ghost && snapshot.ghost.parentNode) snapshot.ghost.parentNode.removeChild(snapshot.ghost);
    },

    playComposerSendMorphToMessage: function(snapshot, messageId) {
      if (!snapshot || !snapshot.ghost) return;
      if (this.prefersReducedMotion()) {
        snapshot.ghost.style.opacity = '0.56';
        setTimeout(this.clearComposerSendMorph.bind(this, snapshot), 240);
        return;
      }
      var row = document.getElementById('chat-msg-' + String(messageId || '').trim());
      var bubble = row ? row.querySelector('.message-bubble') : null;
      if (!bubble) {
        this.clearComposerSendMorph(snapshot);
        return;
      }
      var rect = bubble.getBoundingClientRect();
      if (!(rect.width > 24 && rect.height > 20)) {
        this.clearComposerSendMorph(snapshot);
        return;
      }
      var ghost = snapshot.ghost;
      var self = this;
      ghost.classList.add('in-flight');
      var finish = function() { self.clearComposerSendMorph(snapshot); };
      ghost.addEventListener('transitionend', finish, { once: true });
      requestAnimationFrame(function() {
        ghost.style.left = rect.left + 'px';
        ghost.style.top = rect.top + 'px';
        ghost.style.width = rect.width + 'px';
        ghost.style.minHeight = rect.height + 'px';
        ghost.style.opacity = '0.2';
      });
      setTimeout(finish, 760);
    },

    appendUserChatMessage: function(finalText, msgImages, options) {
      var opts = options && typeof options === 'object' ? options : {};
      var text = String(finalText == null ? '' : finalText);
      var images = Array.isArray(msgImages) ? msgImages : [];
      if (!String(text || '').trim() && !images.length) return;
      var msg = {
        id: ++msgId,
        role: 'user',
        text: text,
        meta: '',
        tools: [],
        images: images,
        ts: Number.isFinite(Number(opts.ts)) ? Number(opts.ts) : Date.now()
      };
      this.messages.push(msg);
      this._stickToBottom = true;
      this.scrollToBottom({ force: true, stabilize: true });
      localStorage.setItem('of-first-msg', 'true');
      this.promptSuggestions = [];
      if (!opts.deferPersist) this.scheduleConversationPersist();
      return msg;
    },

    // Process queued messages after current response completes
    _processQueue: function() {
      if (!this.messageQueue.length || this.sending || this._inflightFailoverInProgress) return;
      var next = this.messageQueue.shift();
      if (next && next.terminal) {
        this._sendTerminalPayload(next.command);
        return;
      }
      var nextText = String(next && next.text ? next.text : '');
      var nextFiles = Array.isArray(next && next.files) ? next.files : [];
      var nextImages = Array.isArray(next && next.images) ? next.images : [];
      if (!nextText.trim() && !nextFiles.length) {
        var self = this;
        this.$nextTick(function() { self._processQueue(); });
        return;
      }
      this.appendUserChatMessage(nextText, nextImages, { deferPersist: true });
      this.scheduleConversationPersist();
      this._sendPayload(nextText, nextFiles, nextImages, {
        from_queue: true,
        queue_id: next && next.queue_id ? String(next.queue_id) : '',
        agent_runtime_engine_id: String((next && next.agent_runtime_engine_id) || this.selectedAgentRuntimeEngineId || 'infring_native')
      });
    },

    _terminalPromptLine: function(cwd, command) {
      var path = String(cwd || this.terminalPromptPath || '/workspace');
      var cmd = String(command || '').trim();
      if (!cmd) return path + ' %';
      return path + ' % ' + cmd;
    },

    _appendTerminalMessage: function(entry) {
      var payload = entry || {};
      var text = String(payload.text || '').replace(/\r\n/g, '\n').replace(/\r/g, '\n').replace(/^\s+|\s+$/g, '');
      var now = Date.now();
      var ts = Number.isFinite(Number(payload.ts)) ? Number(payload.ts) : now;
      var role = payload.role ? String(payload.role) : 'terminal';
      var terminalSource = payload.terminal_source ? String(payload.terminal_source).toLowerCase() : '';
      if (terminalSource !== 'user' && terminalSource !== 'agent' && terminalSource !== 'system') {
        terminalSource = role === 'user' ? 'user' : 'system';
      }
      var cwd = payload.cwd ? String(payload.cwd) : this.terminalPromptPath;
      var meta = payload.meta == null ? '' : String(payload.meta);
      var tools = Array.isArray(payload.tools) ? payload.tools : [];
      var shouldAppendToLast = payload.append_to_last === true;
      var agentId = payload.agent_id ? String(payload.agent_id) : '';
      var agentName = payload.agent_name ? String(payload.agent_name) : '';
      if (terminalSource === 'agent') {
        if (!agentId && this.currentAgent && this.currentAgent.id) agentId = String(this.currentAgent.id);
        if (!agentName && this.currentAgent && this.currentAgent.name) agentName = String(this.currentAgent.name);
      }

      var rows = this.ensureActiveChatMessagesArray();
      var last = rows.length ? rows[rows.length - 1] : null;
      if (shouldAppendToLast && last && !last.thinking && last.terminal) {
        if (text) {
          if (last.text && !/\n$/.test(last.text)) last.text += '\n';
          last.text += text.replace(/^[\r\n]+/, '');
        }
        if (meta) last.meta = meta;
        if (cwd) {
          last.cwd = cwd;
          this.terminalCwd = cwd;
        }
        if (terminalSource) last.terminal_source = terminalSource;
        if (agentId) last.agent_id = agentId;
        if (agentName) last.agent_name = agentName;
        last.ts = ts;
        if (!Array.isArray(last.tools)) last.tools = [];
        if (tools.length) last.tools = last.tools.concat(tools);
        if (typeof this.syncActiveChatMessages === 'function') this.syncActiveChatMessages();
        return last;
      }

      var msg = {
        id: ++msgId,
        role: role,
        text: text,
        meta: meta,
        tools: tools,
        ts: ts,
        terminal: true,
        terminal_source: terminalSource || 'system',
        cwd: cwd
      };
      if (agentId) msg.agent_id = agentId;
      if (agentName) msg.agent_name = agentName;
      this.appendActiveChatMessage(msg);
      if (cwd) this.terminalCwd = cwd;
      return msg;
    },

          if (!Array.isArray(this.agentDrawer._fallbacks)) this.agentDrawer._fallbacks = [];
          this.agentDrawer._fallbacks.push({ provider: fallbackProvider, model: fallbackModel });
          appendedFallback = true;
          configPayload.fallback_models = this.dedupeFallbackModelList(this.agentDrawer._fallbacks, {
            primary_id: (this.agentDrawer && (this.agentDrawer.runtime_model || this.agentDrawer.model_name)) || '',
            primary_provider: (this.agentDrawer && this.agentDrawer.model_provider) || '',
          });
          this.agentDrawer._fallbacks = configPayload.fallback_models.slice();
        } else if (Array.isArray(this.agentDrawer._fallbacks)) {
          configPayload.fallback_models = this.dedupeFallbackModelList(this.agentDrawer._fallbacks, {
            primary_id: (this.agentDrawer && (this.agentDrawer.runtime_model || this.agentDrawer.model_name)) || '',
            primary_provider: (this.agentDrawer && this.agentDrawer.model_provider) || '',
          });
          this.agentDrawer._fallbacks = configPayload.fallback_models.slice();
        }

        var configResponse = await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(agentId) + '/config', configPayload);
        if (configResponse && configResponse.rename_notice) {
          this.addNoticeEvent(configResponse.rename_notice);
        } else if (requestedName && requestedName !== previousName) {
          this.addAgentRenameNotice(previousName, requestedName);
        }

        if (this.drawerEditingProvider && String(this.drawerNewProviderValue || '').trim()) {
          var previousProviderName = String((this.agentDrawer && this.agentDrawer.model_provider) || (this.currentAgent && this.currentAgent.model_provider) || '').trim();
          var previousModelName = String((this.agentDrawer && (this.agentDrawer.runtime_model || this.agentDrawer.model_name)) || (this.currentAgent && (this.currentAgent.runtime_model || this.currentAgent.model_name)) || '').trim();
          var resolvedProviderModel = this.resolveProviderScopedModelCatalogOption(
            this.drawerNewProviderValue,
            (this.agentDrawer && (this.agentDrawer.runtime_model || this.agentDrawer.model_name)) || '',
            this.modelCatalogRows()
          );
          await this.switchAgentModelWithGuards(
            resolvedProviderModel || { id: String(this.drawerNewProviderValue || '').trim() + '/' + (this.agentDrawer.model_name || '') },
            {
            agent_id: agentId,
            previous_model: previousModelName,
            previous_provider: previousProviderName
            }
          );
        } else if (this.drawerEditingModel && String(this.drawerNewModelValue || '').trim()) {
          var previousModelNameForModelEdit = String((this.agentDrawer && (this.agentDrawer.runtime_model || this.agentDrawer.model_name)) || (this.currentAgent && (this.currentAgent.runtime_model || this.currentAgent.model_name)) || '').trim();
          var previousProviderForModelEdit = String((this.agentDrawer && this.agentDrawer.model_provider) || (this.currentAgent && this.currentAgent.model_provider) || '').trim();
          var resolvedDrawerModel = this.resolveModelCatalogOption(
            this.drawerNewModelValue,
            previousProviderForModelEdit,
            this.modelCatalogRows()
          );
          await this.switchAgentModelWithGuards(
            resolvedDrawerModel || { id: String(this.drawerNewModelValue || '').trim() },
            {
              agent_id: agentId,
              previous_model: previousModelNameForModelEdit,
              previous_provider: previousProviderForModelEdit
            }
          );
        }

        this.drawerEditingName = false;
        this.drawerEditingEmoji = false;
        this.drawerEditingModel = false;
        this.drawerEditingProvider = false;
        this.drawerEditingFallback = false;
        this.drawerNewModelValue = '';
        this.drawerNewProviderValue = '';
        this.drawerNewFallbackValue = '';
        InfringToast.success('Agent settings saved');
        await this.syncDrawerAgentAfterChange();
        this.closeAgentDrawer();
      } catch (e) {
        if (appendedFallback) {
          this.agentDrawer._fallbacks = previousFallbacks;
        }
        InfringToast.error('Failed to save agent settings: ' + e.message);
      } finally {
        this.drawerSavePending = false;
        this.drawerConfigSaving = false;
        this.drawerModelSaving = false;
        this.drawerIdentitySaving = false;
      }
    },

    async saveDrawerConfig() {
      if (!this.agentDrawer || !this.agentDrawer.id) return;
      var previousName = String((this.agentDrawer && this.agentDrawer.name) || (this.currentAgent && this.currentAgent.name) || '').trim();
      var requestedName = String((this.drawerConfigForm && this.drawerConfigForm.name) || '').trim();
      this.drawerConfigSaving = true;
      try {
        var response = await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(this.agentDrawer.id) + '/config', this.drawerConfigForm || {});
        if (response && response.rename_notice) {
          this.addNoticeEvent(response.rename_notice);
        } else if (requestedName && requestedName !== previousName) {
          this.addAgentRenameNotice(previousName, requestedName);
        }
        InfringToast.success('Config updated');
        await this.syncDrawerAgentAfterChange();
        this.closeAgentDrawer();
      } catch(e) {
        InfringToast.error('Failed to save config: ' + e.message);
      }
      this.drawerConfigSaving = false;
    },

    async saveDrawerIdentity(part) {
      if (!this.agentDrawer || !this.agentDrawer.id) return;
      var payload = {};
      var previousName = String((this.agentDrawer && this.agentDrawer.name) || (this.currentAgent && this.currentAgent.name) || '').trim();
      if (part === 'name') {
        payload.name = String((this.drawerConfigForm && this.drawerConfigForm.name) || '').trim();
      } else if (part === 'emoji') {
        payload.emoji = String((this.drawerConfigForm && this.drawerConfigForm.emoji) || '').trim();
        if (this.sanitizeAgentEmojiForDisplay) {
          payload.emoji = this.sanitizeAgentEmojiForDisplay(this.agentDrawer || this.currentAgent, payload.emoji);
        }
        if (!payload.emoji) {
          InfringToast.info('The gear icon is reserved for the System thread.');
          this.drawerIdentitySaving = false;
          return;
        }
        payload.avatar_url = '';
        if (this.drawerConfigForm && typeof this.drawerConfigForm === 'object') {
          this.drawerConfigForm.avatar_url = '';
        }
        if (this.agentDrawer && typeof this.agentDrawer === 'object') {
          this.agentDrawer.avatar_url = '';
        }
      } else if (part === 'avatar') {
        payload.avatar_url = String((this.drawerConfigForm && this.drawerConfigForm.avatar_url) || '').trim();
      } else {
        return;
      }
      this.drawerIdentitySaving = true;
      try {
        var response = await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(this.agentDrawer.id) + '/config', payload);
        if (response && response.rename_notice) {
          this.addNoticeEvent(response.rename_notice);
        } else if (part === 'name' && payload.name && payload.name !== previousName) {
          this.addAgentRenameNotice(previousName, payload.name);
        }
        if (part === 'name') this.drawerEditingName = false;
        if (part === 'emoji') this.drawerEditingEmoji = false;
        if (part === 'avatar') {
          this.drawerAvatarUploadError = '';
          this.drawerAvatarUrlPickerOpen = false;
          this.drawerAvatarUrlDraft = '';
        }
        InfringToast.success(
          part === 'name'
            ? 'Name updated'
            : (part === 'emoji' ? 'Emoji updated' : 'Avatar updated')
        );
        await this.syncDrawerAgentAfterChange();
        this.closeAgentDrawer();
      } catch(e) {
        InfringToast.error('Failed to save ' + part + ': ' + e.message);
      }
      this.drawerIdentitySaving = false;
    },

    async changeDrawerModel() {
      if (!this.agentDrawer || !this.agentDrawer.id || !String(this.drawerNewModelValue || '').trim()) return;
      this.drawerModelSaving = true;
      try {
        var previousModel = String((this.agentDrawer && (this.agentDrawer.runtime_model || this.agentDrawer.model_name)) || (this.currentAgent && (this.currentAgent.runtime_model || this.currentAgent.model_name)) || '').trim();
        var previousProvider = String((this.agentDrawer && this.agentDrawer.model_provider) || (this.currentAgent && this.currentAgent.model_provider) || '').trim();
        var resolvedDrawerModel = this.resolveModelCatalogOption(
          this.drawerNewModelValue,
          previousProvider,
          this.modelCatalogRows()
        );
        var resp = await this.switchAgentModelWithGuards(
          resolvedDrawerModel || { id: String(this.drawerNewModelValue || '').trim() },
          {
            agent_id: this.agentDrawer.id,
            previous_model: previousModel,
            previous_provider: previousProvider
          }
        );
        var providerInfo = (resp && resp.provider) ? ' (provider: ' + resp.provider + ')' : '';
        InfringToast.success('Model changed' + providerInfo + ' (memory reset)');
        this.drawerEditingModel = false;
        this.drawerNewModelValue = '';
        await this.syncDrawerAgentAfterChange();
      } catch(e) {
        InfringToast.error('Failed to change model: ' + e.message);
      }
      this.drawerModelSaving = false;
    },

    async changeDrawerProvider() {
      if (!this.agentDrawer || !this.agentDrawer.id || !String(this.drawerNewProviderValue || '').trim()) return;
      this.drawerModelSaving = true;
      try {
        var previousProvider = String((this.agentDrawer && this.agentDrawer.model_provider) || (this.currentAgent && this.currentAgent.model_provider) || '').trim();
        var previousModel = String((this.agentDrawer && (this.agentDrawer.runtime_model || this.agentDrawer.model_name)) || (this.currentAgent && (this.currentAgent.runtime_model || this.currentAgent.model_name)) || '').trim();
        var resolvedProviderModel = this.resolveProviderScopedModelCatalogOption(
          this.drawerNewProviderValue,
          previousModel || (this.agentDrawer && this.agentDrawer.model_name) || '',
          this.modelCatalogRows()
        );
        var resp = await this.switchAgentModelWithGuards(
          resolvedProviderModel || { id: String(this.drawerNewProviderValue || '').trim() + '/' + (this.agentDrawer.model_name || '') },
          {
            agent_id: this.agentDrawer.id,
            previous_model: previousModel,
            previous_provider: previousProvider
          }
        );
        InfringToast.success('Provider changed to ' + (resp && resp.provider ? resp.provider : String(this.drawerNewProviderValue || '').trim()));
        this.drawerEditingProvider = false;
        this.drawerNewProviderValue = '';
        await this.syncDrawerAgentAfterChange();
      } catch(e) {
        InfringToast.error('Failed to change provider: ' + e.message);
      }
      this.drawerModelSaving = false;
    },

    async addDrawerFallback() {
      if (!this.agentDrawer || !this.agentDrawer.id || !String(this.drawerNewFallbackValue || '').trim()) return;
      var parts = String(this.drawerNewFallbackValue || '').trim().split('/');
      var provider = parts.length > 1 ? parts[0] : this.agentDrawer.model_provider;
      var model = parts.length > 1 ? parts.slice(1).join('/') : parts[0];
      if (!this.agentDrawer._fallbacks) this.agentDrawer._fallbacks = [];
      var previousFallbacks = this.agentDrawer._fallbacks.slice();
      var nextFallbacks = this.dedupeFallbackModelList(
        this.agentDrawer._fallbacks.concat([{ provider: provider, model: model }]),
        {
          primary_id: (this.agentDrawer && (this.agentDrawer.runtime_model || this.agentDrawer.model_name)) || '',
          primary_provider: (this.agentDrawer && this.agentDrawer.model_provider) || '',
        }
      );
      if (nextFallbacks.length === this.agentDrawer._fallbacks.length) {
        InfringToast.info('Fallback already exists or matches the primary model');
        return;
      }
      this.agentDrawer._fallbacks = nextFallbacks;
      try {
        await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(this.agentDrawer.id) + '/config', {
          fallback_models: this.agentDrawer._fallbacks
        });
        var latestFallback = this.agentDrawer._fallbacks[this.agentDrawer._fallbacks.length - 1] || {};
        InfringToast.success('Fallback added: ' + String((latestFallback.provider || provider) || '').trim() + '/' + String((latestFallback.model || model) || '').trim());
        this.drawerEditingFallback = false;
        this.drawerNewFallbackValue = '';
      } catch(e) {
        InfringToast.error('Failed to save fallbacks: ' + e.message);
        this.agentDrawer._fallbacks = previousFallbacks;
      }
    },

    async removeDrawerFallback(idx) {
      if (!this.agentDrawer || !this.agentDrawer.id || !Array.isArray(this.agentDrawer._fallbacks)) return;
      var removed = this.agentDrawer._fallbacks.splice(idx, 1);
      try {
        await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(this.agentDrawer.id) + '/config', {
          fallback_models: this.agentDrawer._fallbacks
        });
        InfringToast.success('Fallback removed');
      } catch(e) {
        InfringToast.error('Failed to save fallbacks: ' + e.message);
        if (removed && removed.length) this.agentDrawer._fallbacks.splice(idx, 0, removed[0]);
      }
    },

    runSlashAlerts: async function() {
      try {
        var alertsPayload = await this.fetchProactiveTelemetryAlerts(false);
        var alertsRows = Array.isArray(alertsPayload && alertsPayload.alerts) ? alertsPayload.alerts : [];
        if (!alertsRows.length) {
          this.pushSystemMessage({
            id: ++msgId,
            role: 'system',
            is_notice: true,
            notice_type: 'info',
            notice_label: 'No proactive telemetry alerts right now.',
            text: 'No proactive telemetry alerts right now.',
            meta: '',
            tools: [],
            system_origin: 'slash:alerts',
            ts: Date.now()
          });
        } else {
          var alertText = alertsRows.map(function(row) {
            var sev = String((row && row.severity) || 'info').toUpperCase();
            var msg = String((row && row.message) || '').trim();
            var cmd = String((row && row.recommended_command) || '').trim();
            return '- [' + sev + '] ' + msg + (cmd ? ('\n  ↳ `' + cmd + '`') : '');
          }).join('\n');
          this.pushSystemMessage({
            id: ++msgId,
            role: 'system',
            is_notice: true,
            notice_type: 'info',
            notice_label: 'Telemetry Alerts',
            text: '**Telemetry Alerts**\n' + alertText,
            meta: '',
            tools: [],
            system_origin: 'slash:alerts',
            ts: Date.now()
          });
        }
      } catch (error) {
        this.emitCommandFailureNotice('/alerts', error, ['/status', '/continuity']);
      }
    },

    runSlashNextActions: async function() {
      try {
        var alertsPayload = await this.fetchProactiveTelemetryAlerts(false);
        var rows = Array.isArray(alertsPayload && alertsPayload.next_actions)
          ? alertsPayload.next_actions
          : [];
        if (!rows.length) {
          this.pushSystemMessage({
            id: ++msgId,
            role: 'system',
            is_notice: true,
            notice_type: 'info',
            notice_label: 'No predicted next actions right now.',
            text: 'No predicted next actions right now.',
            meta: '',
            tools: [],
            system_origin: 'slash:next',
            ts: Date.now()
          });
          return;
        }
        var rendered = rows.slice(0, 6).map(function(row) {
          var cmd = String((row && row.command) || '').trim();
          var reason = String((row && row.reason) || '').trim();
          var priority = String((row && row.priority) || 'low').toUpperCase();
          return '- [' + priority + '] `' + cmd + '`' + (reason ? ('\n  ↳ ' + reason) : '');
        }).join('\n');
        this.pushSystemMessage({
          id: ++msgId,
          role: 'system',
          is_notice: true,
          notice_type: 'info',
          notice_label: 'Predicted Next Actions',
          text: '**Predicted Next Actions**\n' + rendered,
          meta: '',
          tools: [],
          system_origin: 'slash:next',
          ts: Date.now()
        });
      } catch (error) {
        this.emitCommandFailureNotice('/next', error, ['/alerts', '/status']);
      }
    },

    runSlashMemoryHygiene: async function() {
      try {
        var alertsPayload = await this.fetchProactiveTelemetryAlerts(false);
        var hygiene = alertsPayload && alertsPayload.memory_hygiene ? alertsPayload.memory_hygiene : {};
        var stale48 = Number(hygiene.stale_contexts_48h || 0);
        var stale7d = Number(hygiene.stale_contexts_7d || 0);
        var bytes = Number(hygiene.snapshot_history_bytes || 0);
        var overCap = !!hygiene.snapshot_history_over_soft_cap;
        var recs = Array.isArray(hygiene.recommendations) ? hygiene.recommendations : [];
        var recText = recs.slice(0, 4).map(function(row) {
          var cmd = String((row && row.command) || '').trim();
          var reason = String((row && row.reason) || '').trim();
          return '- `' + cmd + '`' + (reason ? ('\n  ↳ ' + reason) : '');
        }).join('\n');
        this.pushSystemMessage({
          id: ++msgId,
          role: 'system',
          is_notice: true,
          notice_type: 'info',
          notice_label: 'Memory Hygiene',
          text:
            '**Memory Hygiene**\n' +
            '- Stale contexts (48h+): ' + stale48 + '\n' +
            '- Stale contexts (7d+): ' + stale7d + '\n' +
            '- Snapshot history bytes: ' + bytes + '\n' +
            '- Over soft cap: ' + (overCap ? 'yes' : 'no') +
            (recText ? ('\n\nRecommended actions:\n' + recText) : ''),
          meta: '',
          tools: [],
          system_origin: 'slash:memory',
          ts: Date.now()
        });
      } catch (error) {
        this.emitCommandFailureNotice('/memory', error, ['/continuity', '/alerts']);
      }
    },

    runSlashContinuity: async function() {
      try {
        var continuity = await InfringAPI.get('/api/continuity/pending');
        var taskPending = Number((((continuity || {}).tasks || {}).pending) || 0);
        var staleSessions = Number(((((continuity || {}).sessions) || {}).stale_48h_count) || 0);
        var channelAttention = Number(((((continuity || {}).channels) || {}).attention_needed_count) || 0);
        var continuityRows = [];
        var self = this;
        var formatContinuitySessionIdentity = function(row) {
          var agentId = String((row && (row.agent_id || row.agentId)) || '').trim();
          if (typeof self.resolveSessionRowLabel !== 'function') {
            return agentId || '?';
          }
          var label = self.resolveSessionRowLabel({
            label: row && (row.session_label || row.label),
            key: row && (row.session_key || row.key),
            session_key: row && row.session_key,
            session_id: row && (row.session_id || row.id),
            id: row && row.id,
            agent_id: agentId,
          }, agentId);
          if (!agentId) return label || '?';
          if (!label || label.toLowerCase() === 'main') return agentId;
          return agentId + ' / ' + label;
        };
        continuityRows.push('**Cross-Channel Continuity**');
        continuityRows.push('- Pending tasks: ' + taskPending);
        continuityRows.push('- Stale sessions (48h+): ' + staleSessions);
        continuityRows.push('- Channel attention needed: ' + channelAttention);
        var activeAgentRows = ((((continuity || {}).active_agents) || {}).rows) || [];
        if (Array.isArray(activeAgentRows) && activeAgentRows.length) {
          continuityRows.push('');
          continuityRows.push('Active agent markers:');
          var markers = activeAgentRows.slice(0, 4).map(function(row) {
            var id = formatContinuitySessionIdentity(row);
            var objective = String((row && row.objective) || '').trim();
            if (objective.length > 70) objective = objective.slice(0, 67) + '...';
            var completion = Number((row && row.completion_percent) || 0);
            return '- `' + id + '` — ' + objective + ' (' + completion + '%)';
          });
          continuityRows = continuityRows.concat(markers);
        }
        var stale = (((continuity || {}).sessions) || {}).stale_48h || [];
        if (Array.isArray(stale) && stale.length) {
          var stalePreview = stale.slice(0, 3).map(function(row) {
            return '- `' + formatContinuitySessionIdentity(row) + '` — ' + Number((row && row.age_hours) || 0).toFixed(1) + 'h';
          });
          continuityRows.push('');
          continuityRows.push('Stale session previews:');
          continuityRows = continuityRows.concat(stalePreview);
        }
        this.pushSystemMessage({
          id: ++msgId,
          role: 'system',
          is_notice: true,
          notice_type: 'info',
          notice_label: 'Cross-Channel Continuity',
          text: continuityRows.join('\n'),
          meta: '',
          tools: [],
          system_origin: 'slash:continuity',
          ts: Date.now()
        });
      } catch (error) {
        this.emitCommandFailureNotice('/continuity', error, ['/status', '/alerts']);
      }
    },

    executeSlashAliases: function() {
      this.loadSlashAliases();
      this.messages.push({
        id: ++msgId,
        role: 'system',
        is_notice: true,
        notice_type: 'info',
        notice_label: 'Slash Aliases',
        text: '**Slash Aliases**\n' + (this.formatSlashAliasRows() || '_No aliases configured_'),
        meta: '',
        tools: [],
        system_origin: 'slash:aliases',
        ts: Date.now()
      });
      this.scrollToBottom();
    },

    // Backward-compat shim for legacy callers during naming migration.
    runSlashAliases: function() {
      this.executeSlashAliases();
    },

    executeSlashAliasCommand: function(cmdArgs) {
      var aliasTokens = String(cmdArgs || '').trim().split(/\s+/).filter(Boolean);
      if (aliasTokens.length < 2) {
        this.messages.push({
          id: ++msgId,
          role: 'system',
          is_notice: true,
          notice_type: 'info',
          notice_label: 'Usage: /alias /shortcut /target [extra args]',
          text: 'Usage: `/alias /shortcut /target [extra args]`',
          meta: '',
          tools: [],
          system_origin: 'slash:alias',
          ts: Date.now()
        });
        this.scrollToBottom();
        return;
      }
      var aliasKey = String(aliasTokens[0] || '').trim().toLowerCase();
      var aliasTarget = String(aliasTokens.slice(1).join(' ') || '').trim().toLowerCase();
      if (!aliasKey.startsWith('/') || !aliasTarget.startsWith('/')) {
        this.messages.push({
          id: ++msgId,
          role: 'system',
          is_notice: true,
          notice_type: 'error',
          notice_label: 'Alias and target must both start with /.',
          text: 'Alias and target must both start with `/`.',
          meta: '',
          tools: [],
          system_origin: 'slash:alias',
          ts: Date.now()
        });
        this.scrollToBottom();
        return;
      }
      this.loadSlashAliases();
      this.slashAliasMap[aliasKey] = aliasTarget;
      this.saveSlashAliases();
      this.messages.push({
        id: ++msgId,
        role: 'system',
        is_notice: true,
        notice_type: 'info',
        notice_label: 'Saved alias ' + aliasKey,
        text: 'Saved alias `' + aliasKey + '` → `' + aliasTarget + '`',
        meta: '',
        tools: [],
        system_origin: 'slash:alias',
        ts: Date.now()
      });
      this.scrollToBottom();
    },

    // Backward-compat shim for legacy callers during naming migration.
    runSlashAliasCommand: function(cmdArgs) {
      this.executeSlashAliasCommand(cmdArgs);
    },

    runSlashOptimizeWorkers: async function() {
      try {
        var optimization = await InfringAPI.get('/api/continuity/pending');
        var pending = Number((((optimization || {}).tasks || {}).pending) || 0);
        var activeWorkers = Number((((optimization || {}).workers || {}).active_workers) || 0);
        var recommendation = pending > 0
          ? 'Queue has pending tasks. Keep workers in service mode:\n`infring task worker --service=1 --wait-ms=125 --idle-hibernate-ms=15000`'
          : 'Queue is empty. Workers can hibernate safely:\n`infring task worker --service=1 --idle-hibernate-ms=15000`';
        this.pushSystemMessage({
          id: ++msgId,
          role: 'system',
          is_notice: true,
          notice_type: 'info',
          notice_label: 'Worker Optimization',
          text: '**Worker Optimization**\n- Pending tasks: ' + pending + '\n- Active workers: ' + activeWorkers + '\n\n' + recommendation,
          meta: '',
          tools: [],
          system_origin: 'slash:opt',
          ts: Date.now()
        });
      } catch (error) {
        this.emitCommandFailureNotice('/opt', error, ['/status', '/continuity']);
      }
    },

    pushSystemMessage: function(entry) {
      var payload = entry && typeof entry === 'object' ? entry : { text: entry };
      var rawText = String(payload && payload.text ? payload.text : '');
      var text = this.normalizeSystemMessageText
        ? this.normalizeSystemMessageText(rawText)
        : rawText.trim();
      if (!text) return null;
      var canonicalText = text.replace(/\s+/g, ' ').trim().toLowerCase();
      if (/^error:\s*/i.test(canonicalText) && canonicalText.indexOf('operation was aborted') >= 0) return null;
      if (payload.allow_chat_injection !== true) {
        if (!Array.isArray(this.systemTelemetry)) this.systemTelemetry = [];
        this.systemTelemetry.push({ text: text, origin: payload.system_origin || payload.systemOrigin || '', ts: Date.now() });
        return null;
      }

      var origin = String(payload.system_origin || payload.systemOrigin || '').trim();
      var tsRaw = Number(payload.ts || 0);
      var ts = Number.isFinite(tsRaw) && tsRaw > 0 ? tsRaw : Date.now();
      var dedupeWindowMs = Number(payload.dedupe_window_ms || payload.dedupeWindowMs || 8000);
      if (!Number.isFinite(dedupeWindowMs) || dedupeWindowMs < 0) dedupeWindowMs = 8000;
      if (dedupeWindowMs > 60000) dedupeWindowMs = 60000;
      var canDedupe = payload.dedupe !== false;
      var systemThreadId = String(this.systemThreadId || 'system').trim() || 'system';
      var activeId = String((this.currentAgent && this.currentAgent.id) || '').trim();
      var targetId = activeId || systemThreadId;
      var isGlobalNotice = !!(
        this.isSystemNotificationGlobalToWorkspace &&
        this.isSystemNotificationGlobalToWorkspace(origin, text)
      );
      var routeToSystem =
        payload.route_to_system === true ||
        (payload.route_to_system !== false && isGlobalNotice);
      if (routeToSystem) targetId = systemThreadId;
      var activeThread = !!activeId && activeId === targetId;
      if (!this._systemMessageDedupeIndex || typeof this._systemMessageDedupeIndex !== 'object') this._systemMessageDedupeIndex = {};

      var targetRows = null;
      var targetCache = null;
      if (activeThread) {
        if (!Array.isArray(this.messages)) this.messages = [];
        targetRows = this.messages;
      } else {
        if (!this.conversationCache || typeof this.conversationCache !== 'object') this.conversationCache = {};
        targetCache = this.conversationCache[targetId];
        if (!targetCache || typeof targetCache !== 'object' || !Array.isArray(targetCache.messages)) {
          targetCache = { saved_at: Date.now(), token_count: 0, messages: [] };
          this.conversationCache[targetId] = targetCache;
        }
        targetRows = targetCache.messages;
      }

      if (!Array.isArray(targetRows)) return null;
      var dedupeKey = targetId + '|' + (origin || '_') + '|' + canonicalText;
      if (canDedupe) {
        for (var idx = targetRows.length - 1, scanned = 0; idx >= 0 && scanned < 24; idx -= 1) {
          var row = targetRows[idx];
          if (!row || row.thinking || row.streaming) continue;
          if (String(row.role || '').toLowerCase() !== 'system' || row.is_notice) continue;
          scanned += 1;
          var rowText = String(row.text || '').replace(/\s+/g, ' ').trim().toLowerCase();
          if (rowText !== canonicalText) continue;
          var rowTs = Number(row.ts || 0);
          if (Number.isFinite(rowTs) && Math.abs(ts - rowTs) > dedupeWindowMs) continue;
          var rowOrigin = String(row.system_origin || '').trim();
          if (rowOrigin && origin && rowOrigin !== origin && !/^error:/i.test(canonicalText)) continue;
          var repeatCount = Number(row._repeat_count || 1);
          if (!Number.isFinite(repeatCount) || repeatCount < 1) repeatCount = 1;
          repeatCount += 1;
          row._repeat_count = repeatCount;
          var priorMeta = String(row.meta || '').trim().replace(/\s*\|\s*repeated x\d+\s*$/i, '').trim();
          row.meta = (priorMeta ? (priorMeta + ' | ') : '') + 'repeated x' + repeatCount;
          row.ts = ts;
          this._systemMessageDedupeIndex[dedupeKey] = { id: row.id, ts: ts };
          if (activeThread) this.scheduleConversationPersist();
          else this.persistConversationCache();
          return row;
        }
      }

      var message = {
        id: ++msgId,
        role: 'system',
        text: text,
        meta: String(payload.meta || ''),
        tools: Array.isArray(payload.tools) ? payload.tools : [],
        system_origin: origin,
        ts: ts
      };
      targetRows.push(message);
      if (canDedupe && canonicalText) this._systemMessageDedupeIndex[dedupeKey] = { id: message.id, ts: ts };

      var store = Alpine.store('app');
      if (store && typeof store.saveAgentChatPreview === 'function') {
        store.saveAgentChatPreview(targetId, targetRows);
      }
      if (activeThread) {
        if (payload.auto_scroll !== false) this.scrollToBottom();
        this.scheduleConversationPersist();
      } else {
        if (targetCache) {
          targetCache.saved_at = Date.now();
          targetCache.token_count = 0;
        }
        this.persistConversationCache();
      }
      return message;
    },

    activateSystemThread: function(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var priorAgentId = String((this.currentAgent && this.currentAgent.id) || '').trim();
      if (priorAgentId && !this.isSystemThreadId(priorAgentId) && typeof this.captureConversationDraft === 'function') {
        this.captureConversationDraft(priorAgentId);
      }
      this.currentAgent = this.makeSystemThreadAgent();
      this.setStoreActiveAgentId(this.currentAgent.id || null);
      this._clearTypingTimeout();
      this._clearPendingWsRequest(this.currentAgent.id || '');
      this.sending = false;
      this._responseStartedAt = 0;
      this.tokenCount = 0;
      this.messageQueue = Array.isArray(this.messageQueue)
        ? this.messageQueue.filter(function(row) { return !row || !row.terminal; })
        : [];
      InfringAPI.wsDisconnect();
      this._wsAgent = null;
      this.sessions = [];
      this.showFreshArchetypeTiles = false;
      this.freshInitRevealMenu = false;
      this.terminalMode = true;
      var restored = this.restoreAgentConversation(this.currentAgent.id);
      if (!restored && opts.preserve_if_empty !== true) {
        this.messages = [];
      }
      if (typeof this.restoreConversationDraft === 'function') {
        this.restoreConversationDraft(this.currentAgent.id, 'terminal');
      }
      this.recomputeContextEstimate();
      this.refreshContextPressure();
      this.clearPromptSuggestions();
      this.$nextTick(() => {
        var input = document.getElementById('msg-input');
        if (input) input.focus();
        this.scrollToBottomImmediate();
        this.stabilizeBottomScroll();
        this.pinToLatestOnOpen(null, { maxFrames: 20 });
        this.scheduleMessageRenderWindowUpdate();
      });
    },

    defaultSlashAliases: function() {
      return {
        '/status': '/status',
        '/opt': '/continuity',
        '/q': '/queue',
        '/ctx': '/context',
        '/mods': '/model',
        '/mem': '/compact'
      };
    },

    normalizeSlashCommandName: function(value) {
      var name = String(value || '').trim().toLowerCase();
      if (!name) return '';
      return name.startsWith('/') ? name : ('/' + name);
    },

    findSlashCommandDefinition: function(value) {
      var target = this.normalizeSlashCommandName(value);
      if (!target) return null;
      var projectedRows = Array.isArray(this.filteredSlashCommands) ? this.filteredSlashCommands : [];
      for (var p = 0; p < projectedRows.length; p += 1) {
        var projected = projectedRows[p] && typeof projectedRows[p] === 'object' ? projectedRows[p] : null;
        if (!projected || projected.row_kind === 'heading' || projected.selectable === false) continue;
        if (this.normalizeSlashCommandName(projected.cmd) === target) return projected;
      }
      var rows = Array.isArray(this.slashCommands) ? this.slashCommands : [];
      for (var i = 0; i < rows.length; i += 1) {
        var row = rows[i] && typeof rows[i] === 'object' ? rows[i] : null;
        if (!row) continue;
        if (this.normalizeSlashCommandName(row.cmd) === target) return row;
      }
      return null;
    },

    formatSlashCommandUsage: function(value) {
      var target = this.normalizeSlashCommandName(value);
      if (!target) return '';
      var def = this.findSlashCommandDefinition(target);
      var desc = String(def && def.desc ? def.desc : '').trim();
      return desc ? ('`' + target + '` — ' + desc) : ('`' + target + '`');
    },

    loadSlashAliases: function() {
      var defaults = this.defaultSlashAliases();
      var persisted = {};
      try {
        var raw = localStorage.getItem(this.slashAliasStorageKey || '');
        if (raw) {
          var parsed = JSON.parse(raw);
          if (parsed && typeof parsed === 'object') persisted = parsed;
        }
      } catch(_) {}
      var merged = {};
      Object.keys(defaults).forEach(function(key) {
        var target = String(defaults[key] || '').trim().toLowerCase();
        var alias = String(key || '').trim().toLowerCase();
        if (!alias.startsWith('/') || !target.startsWith('/')) return;
        merged[alias] = target;
      });
      Object.keys(persisted).forEach(function(key) {
        var alias = String(key || '').trim().toLowerCase();
        var target = String(persisted[key] || '').trim().toLowerCase();
        if (!alias.startsWith('/') || !target.startsWith('/')) return;
        merged[alias] = target;
      });
      this.slashAliasMap = merged;
      return merged;
    },

    saveSlashAliases: function() {
      try {
        localStorage.setItem(
          this.slashAliasStorageKey || '',
          JSON.stringify(this.slashAliasMap || {})
        );
      } catch(_) {}
    },

    resolveSlashAlias: function(inputCmd, cmdArgs) {
      var cmd = this.normalizeSlashCommandName(inputCmd);
      var args = String(cmdArgs || '').trim();
      var aliases = this.slashAliasMap || {};
      var visited = {};
      var expandedCmd = cmd;
      var expandedArgs = args;
      var rendered = cmd + (args ? (' ' + args) : '');
      for (var depth = 0; depth < 5; depth += 1) {
        var target = String(aliases[expandedCmd] || '').trim();
        if (!target) break;
        if (visited[expandedCmd]) break;
        visited[expandedCmd] = true;
        rendered = target + (expandedArgs ? (' ' + expandedArgs) : '');
        var targetParts = target.split(/\s+/).filter(Boolean);
        if (!targetParts.length) break;
        expandedCmd = this.normalizeSlashCommandName(targetParts[0]);
        var trailing = targetParts.slice(1).join(' ').trim();
        if (trailing) {
          expandedArgs = trailing + (expandedArgs ? (' ' + expandedArgs) : '');
        }
      }
      return { cmd: expandedCmd, args: expandedArgs.trim(), expanded: rendered };
    },

    formatSlashAliasRows: function() {
      var self = this;
      var aliases = this.slashAliasMap || {};
      var rows = Object.keys(aliases)
        .sort()
        .map(function(alias) {
          var target = String(aliases[alias] || '').trim();
          var targetCommand = self.normalizeSlashCommandName(target.split(/\s+/)[0] || '');
          var usage = self.formatSlashCommandUsage(targetCommand);
          return '- `' + alias + '` → `' + target + '`' + (usage ? ('\n  ↳ ' + usage) : '');
        });
      return rows.join('\n');
    },

    fetchProactiveTelemetryAlerts: function(notify) {
      var self = this;
      return InfringAPI.get('/api/telemetry/alerts').then(function(payload) {
        var rows = Array.isArray(payload && payload.alerts) ? payload.alerts : [];
        var nextActions = Array.isArray(payload && payload.next_actions) ? payload.next_actions : [];
        var digest = rows.map(function(row) {
          return String((row && row.id) || '') + ':' + String((row && row.message) || '');
        }).join('|');
        self._telemetrySnapshot = payload && typeof payload === 'object' ? payload : null;
        self._continuitySnapshot = payload && payload.continuity ? payload.continuity : null;
        self.telemetryNextActions = nextActions.slice(0, 6);
        if (notify && digest && digest !== String(self._lastTelemetryAlertDigest || '')) {
          var rendered = rows.map(function(row) {
            var severity = String((row && row.severity) || 'info').toUpperCase();
            var message = String((row && row.message) || '').trim();
            var command = String((row && row.recommended_command) || '').trim();
            return '- [' + severity + '] ' + message + (command ? ('\n  ↳ `' + command + '`') : '');
          }).join('\n');
          var nextRendered = nextActions.slice(0, 3).map(function(row) {
            var cmd = String((row && row.command) || '').trim();
            var reason = String((row && row.reason) || '').trim();
            return '- `' + cmd + '`' + (reason ? ('\n  ↳ ' + reason) : '');
          }).join('\n');
          if (rendered) {
            self.pushSystemMessage({
              text: '**Telemetry Alerts**\n' + rendered + (nextRendered ? ('\n\n**Suggested Next Actions**\n' + nextRendered) : ''),
              system_origin: 'telemetry:alerts',
              ts: Date.now(),
              auto_scroll: false
            });
          }
        }
        self._lastTelemetryAlertDigest = digest;
        return payload;
      }).catch(function() {
        self._telemetrySnapshot = null;
        self.telemetryNextActions = [];
        return { ok: false, alerts: [] };
      });
    },

    staleMemoryWarningText: function() {
      return '';
    },

    thinkingTraceRows: function(msg) {
      var rows = [];
      if (!msg || !msg.thinking) return rows;
      var tools = Array.isArray(msg.tools) ? msg.tools : [];
      for (var i = 0; i < tools.length; i++) {
        var tool = tools[i];
        if (!tool || this.isThoughtTool(tool)) continue;
        var state = tool.running ? 'running' : (this.isBlockedTool(tool) ? 'blocked' : (tool.is_error ? 'error' : 'done'));
        rows.push({
          id: String(tool.id || ('trace-tool-' + i)),
          label: this.toolDisplayName(tool),
          state: state,
          state_label: state === 'done' ? 'complete' : state
        });
      }
      if (!rows.length) {
        var status = String(
          typeof this.thinkingStatusText === 'function'
            ? this.thinkingStatusText(msg)
            : (msg.thinking_status || '')
        ).trim();
        if (status) {
          rows.push({
            id: 'trace-status',
            label: status,
            state: 'running',
            state_label: 'active'
          });
        }
      }
      return rows.slice(-4);
    },

    emitCommandFailureNotice: function(command, error, fallbackCommands) {
      var cmd = String(command || '').trim() || '/status';
      var message = String(error && error.message ? error.message : error || 'command_failed').trim();
      if (message.length > 220) message = message.slice(0, 217) + '...';
      var fallbacks = Array.isArray(fallbackCommands) ? fallbackCommands : [];
      var fallbackText = fallbacks
        .map(function(row) { return '`' + String(row || '').trim() + '`'; })
        .filter(Boolean)
        .join(' · ');
      this.messages.push({
        id: ++msgId,
        role: 'system',
        is_notice: true,
        notice_type: 'error',
        notice_label: 'Command ' + cmd + ' failed',
        text:
          'Command `' + cmd + '` failed: ' + message +
          (fallbackText ? ('\nTry recovery: ' + fallbackText) : ''),
        meta: '',
        tools: [],
        system_origin: 'slash:error',
        ts: Date.now()
      });
      this.scrollToBottom();
      this.scheduleConversationPersist();
    },

    get filteredSlashCommands() {
      if (this.showSlashMenu && typeof this.fetchAgentRuntimeSlashCommands === 'function') {
        this.fetchAgentRuntimeSlashCommands(false);
      }
      var base = Array.isArray(this.slashCommands) ? this.slashCommands.slice() : [];
      var aliases = this.slashAliasMap || {};
      Object.keys(aliases).forEach(function(alias) {
        if (!base.some(function(c) { return c && c.cmd === alias; })) {
          base.push({
            cmd: alias,
            desc: 'Alias → ' + String(aliases[alias] || ''),
            source: 'alias'
          });
        }
      });
      var runtimeRows = Array.isArray(this.agentRuntimeSlashCommandRows) ? this.agentRuntimeSlashCommandRows : [];
      var gatewayByCmd = {};
      runtimeRows.forEach(function(row) {
        if (!row || row.group_id !== 'infring_native_commands') return;
        var cmd = String(row.cmd || '').trim().toLowerCase();
        if (cmd) gatewayByCmd[cmd] = row;
      });
      var localOperational = function(cmd) {
        var target = String(cmd || '').trim().toLowerCase();
        if (/^\/(model|apikey|file|folder|stop|usage|think|context|verbose|queue|status|alerts|next|memory|continuity|aliases|alias|opt|clear|help)$/i.test(target)) {
          return {
            operational_state: 'connected',
            operational_label: 'Operational',
            operational_detail: 'Handled by the current InfRing client command path.',
            connected: true,
            fully_operational: true
          };
        }
        return {
          operational_state: 'intent_route_only',
          operational_label: 'Legacy route',
          operational_detail: 'Cataloged as an InfRing slash command; full operational proof is not yet attached.',
          connected: true,
          fully_operational: false
        };
      };
      var commandRow = function(row) {
        var cmd = String(row && row.cmd || '').trim();
        var gateway = gatewayByCmd[cmd.toLowerCase()] || null;
        var meta = gateway || localOperational(cmd);
        return {
          row_kind: 'command',
          cmd: cmd,
          desc: String(row && row.desc || gateway && gateway.desc || '').trim(),
          title: String(row && row.title || gateway && gateway.title || cmd).trim(),
          source: String(row && row.source || 'infring_native_slash'),
          command_id: String(gateway && gateway.command_id || ('infring-local:' + cmd.replace(/^\//, ''))),
          intent_id: String(gateway && gateway.intent_id || ''),
          engine_id: String(gateway && gateway.engine_id || 'infring'),
          group_id: 'infring_native_commands',
          group_title: 'InfRing native / commands',
          execution_kind: String(gateway && gateway.execution_kind || 'client_command_handler'),
          safety_class: String(gateway && gateway.safety_class || 'control'),
          operational_state: String(meta.operational_state || 'stubbed_or_unwired'),
          operational_label: String(meta.operational_label || meta.operational_state || 'Unknown'),
          operational_detail: String(meta.operational_detail || ''),
          connected: meta.connected !== false,
          fully_operational: meta.fully_operational === true,
          selectable: true,
          action_route: String(gateway && gateway.action_route || '')
        };
      };
      var sections = [];
      var f = String(this.slashFilter || '').trim().toLowerCase();
      var matches = function(row) {
        if (!f) return true;
        return String(row.cmd || '').toLowerCase().indexOf(f) !== -1 ||
          String(row.desc || '').toLowerCase().indexOf(f) !== -1 ||
          String(row.title || '').toLowerCase().indexOf(f) !== -1 ||
          String(row.operational_label || '').toLowerCase().indexOf(f) !== -1;
      };
      var addSection = function(label, rows, source) {
        var filtered = rows.filter(matches);
        if (!filtered.length) return;
        sections.push({
          row_kind: 'heading',
          cmd: '__heading_' + source + '_' + sections.length,
          label: label,
          desc: '',
          source: source,
          selectable: false
        });
        Array.prototype.push.apply(sections, filtered);
      };
      var activeRuntimeRows = runtimeRows.filter(function(row) {
        return row && row.group_id === 'runtime_native_commands';
      });
      addSection(
        activeRuntimeRows.length && activeRuntimeRows[0].group_title ? activeRuntimeRows[0].group_title : 'Runtime / commands',
        activeRuntimeRows,
        'agent_runtime_command_catalog'
      );
      var localRows = base.map(commandRow).filter(function(row) { return !!row.cmd; });
      runtimeRows.forEach(function(row) {
        if (!row || row.group_id !== 'infring_native_commands') return;
        var cmd = String(row.cmd || '').trim().toLowerCase();
        if (!cmd) return;
        if (!localRows.some(function(local) { return String(local.cmd || '').trim().toLowerCase() === cmd; })) {
          localRows.push(row);
        }
      });
      addSection('InfRing native / commands', localRows, 'infring_native_slash');
      return sections;
    },

    toolAttemptIdentity: function(tool, idx, prefix) {
      var row = tool && typeof tool === 'object' ? tool : {};
      var receipt = row.tool_attempt_receipt && typeof row.tool_attempt_receipt === 'object'
        ? row.tool_attempt_receipt
        : {};
      var toolName = String(row.name || row.tool || receipt.tool_name || 'tool').trim() || 'tool';
      var attemptId = String(row.attempt_id || row.tool_attempt_id || receipt.attempt_id || '').trim();
      var attemptSequence = Number(row.attempt_sequence || row.tool_attempt_sequence || idx + 1);
      if (!Number.isFinite(attemptSequence) || attemptSequence < 1) attemptSequence = idx + 1;
      var fallbackId = String(row.id || ((prefix || 'tool') + '-' + toolName + '-' + attemptSequence)).trim();
      return {
        id: attemptId || fallbackId,
        attempt_id: attemptId,
        attempt_sequence: attemptSequence,
        identity_key: attemptId || (toolName.toLowerCase() + '#' + attemptSequence)
      };
    },
    stringifyStructuredToolValue: function(value, maxLen) {
      var limit = Number(maxLen || 16000);
      if (!Number.isFinite(limit) || limit < 1) limit = 16000;
      if (typeof value === 'string') return String(value).slice(0, limit);
      if (value == null) return '';
      try {
        return JSON.stringify(value).slice(0, limit);
      } catch (_) {
        return String(value).slice(0, limit);
      }
    },
    normalizeToolContentType: function(value) {
      return typeof value === 'string' ? String(value).toLowerCase() : '';
    },
    isToolCallContentType: function(value) {
      var type = this.normalizeToolContentType(value);
      return type === 'toolcall' || type === 'tool_call' || type === 'tooluse' || type === 'tool_use';
    },
    isToolResultContentType: function(value) {
      var type = this.normalizeToolContentType(value);
      return type === 'toolresult' || type === 'tool_result' || type === 'tool_result_error';
    },
    resolveToolBlockArgs: function(block) {
      if (!block || typeof block !== 'object') return '';
      return block.args != null ? block.args : (block.arguments != null ? block.arguments : (block.input != null ? block.input : ''));
    },
    resolveToolUseId: function(block) {
      if (!block || typeof block !== 'object') return '';
      var id = '';
      if (typeof block.id === 'string' && block.id.trim()) id = block.id;
      else if (typeof block.tool_use_id === 'string' && block.tool_use_id.trim()) id = block.tool_use_id;
      else if (typeof block.toolUseId === 'string' && block.toolUseId.trim()) id = block.toolUseId;
      return String(id || '').trim();
    },
    structuredContentBlocksFromPayload: function(payload) {
      var data = payload && typeof payload === 'object' ? payload : {};
      var out = [];
      var pushBlocks = function(value) {
        if (!Array.isArray(value)) return;
        for (var i = 0; i < value.length; i++) out.push(value[i]);
      };
      pushBlocks(data.content);
      pushBlocks(data.response);
      if (data.message && typeof data.message === 'object') pushBlocks(data.message.content);
      return out;
    },
    responseWorkflowFromPayload: function(payload) {
      var data = payload && typeof payload === 'object' ? payload : {};
      return data.response_workflow && typeof data.response_workflow === 'object'
        ? data.response_workflow
        : null;
    },
    workflowResponseTextFromPayload: function(payload) {
      var workflow = this.responseWorkflowFromPayload(payload);
      if (!workflow) return '';
      var status = String(workflow && workflow.final_llm_response && workflow.final_llm_response.status || '').trim().toLowerCase();
      var response = typeof workflow.response === 'string' ? String(workflow.response || '').trim() : '';
      if (status !== 'synthesized' || !response) return '';
      if (this.textLooksNoFindingsPlaceholder(response) || this.textLooksToolAckWithoutFindings(response)) return '';
      return response;
    },
    assistantTextFromPayload: function(payload) {
      var data = payload && typeof payload === 'object' ? payload : {};
      var workflowText = this.workflowResponseTextFromPayload(data);
      if (workflowText) return workflowText;
      if (typeof data.response === 'string') return String(data.response || '');
      if (typeof data.content === 'string') return String(data.content || '');
      var blocks = this.structuredContentBlocksFromPayload(data);
      if (!blocks.length) return '';
      var parts = [];
      for (var i = 0; i < blocks.length; i++) {
        var entry = blocks[i];
        if (typeof entry === 'string') {
          if (entry.trim()) parts.push(entry);
          continue;
        }
        if (!entry || typeof entry !== 'object') continue;
        if (this.isToolCallContentType(entry.type) || this.isToolResultContentType(entry.type)) continue;
        var text = typeof entry.text === 'string'
          ? entry.text
          : (typeof entry.content === 'string' ? entry.content : '');
        if (String(text || '').trim()) parts.push(String(text));
      }
      return parts.join('\n\n').trim();
    },
    normalizeResponseToolCard: function(tool, idx, prefix) {
      var row = tool && typeof tool === 'object' ? tool : {};
      var identity = this.toolAttemptIdentity(row, idx, prefix || 'tool');
      return {
        id: identity.id,
        name: row.name || row.tool || 'tool',
        running: false,
        expanded: false,
        input_preview: this.stringifyStructuredToolValue(row.input_preview || row.arguments_preview || row.args_preview || row.input || row.arguments || row.args || '', 2000),
        result_preview: this.stringifyStructuredToolValue(row.result_preview || row.output_preview || row.summary || row.result || row.output || '', 2000),
        is_error: !!(row.is_error || row.error || row.blocked),
        blocked: row.blocked === true || String(row.status || '').toLowerCase() === 'blocked',
        status: String(row.status || '').trim().toLowerCase(),
        attempt_id: identity.attempt_id,
        attempt_sequence: identity.attempt_sequence,
        identity_key: identity.identity_key,
        tool_attempt_receipt: row.tool_attempt_receipt || null
      };
    },
    toolCardFromAttemptReceipt: function(rawAttempt, idx, prefix) {
      var envelope = rawAttempt && typeof rawAttempt === 'object' ? rawAttempt : {};
      var attempt = envelope.attempt && typeof envelope.attempt === 'object' ? envelope.attempt : envelope;
      var toolName = String(attempt.tool_name || attempt.tool || 'tool').trim() || 'tool';
      var rawStatus = String(attempt.status || attempt.outcome || '').trim().toLowerCase();
      var blocked = rawStatus === 'blocked' || rawStatus === 'policy_denied';
      var isError = !blocked && !!rawStatus && rawStatus !== 'ok';
      var normalizedArgs = envelope.normalized_result && envelope.normalized_result.normalized_args
        ? envelope.normalized_result.normalized_args
        : null;
      var input = '';
      try {
        if (normalizedArgs && typeof normalizedArgs === 'object') input = JSON.stringify(normalizedArgs);
      } catch (_) {}
      var reason = String(envelope.error || attempt.reason || rawStatus || '').trim();
      var backend = String(attempt.backend || '').trim().replace(/_/g, ' ');
      var result = reason;
      if (!result && backend) result = 'Attempted via ' + backend;
      if (!result && rawStatus === 'ok') result = 'Attempt succeeded';
      if (!result) result = 'Attempt recorded';
      var identity = this.toolAttemptIdentity({
        name: toolName,
        attempt_id: attempt.attempt_id || '',
        attempt_sequence: idx + 1,
        tool_attempt_receipt: attempt
      }, idx, prefix || 'attempt');
      return {
        id: identity.id,
        name: toolName,
        running: false,
        expanded: false,
        input_preview: input,
        result_preview: result,
        is_error: isError,
        blocked: blocked,
        status: blocked ? 'blocked' : (rawStatus || (isError ? 'error' : 'ok')),
        attempt_id: identity.attempt_id,
        attempt_sequence: identity.attempt_sequence,
        identity_key: identity.identity_key,
        reason_code: String(attempt.reason_code || '').trim(),
        backend: String(attempt.backend || '').trim(),
        tool_attempt_receipt: attempt
      };
    },
    structuredContentToolRows: function(payload, prefix) {
      var blocks = this.structuredContentBlocksFromPayload(payload);
      if (!blocks.length) return [];
      var rows = [];
      var byKey = {};
      var ensureRow = function(seed, idx) {
        var identity = this.toolAttemptIdentity(seed, idx, prefix || 'content');
        var key = identity.identity_key;
        var current = byKey[key];
        if (!current) {
          current = {
            id: identity.id,
            name: String(seed.name || seed.tool || 'tool').trim() || 'tool',
            running: false,
            expanded: false,
            input_preview: '',
            result_preview: '',
            is_error: false,
            blocked: false,
            status: '',
            attempt_id: identity.attempt_id,
            attempt_sequence: identity.attempt_sequence,
            identity_key: identity.identity_key,
            tool_attempt_receipt: null
          };
          byKey[key] = current;
          rows.push(current);
        }
        return current;
      }.bind(this);
      for (var i = 0; i < blocks.length; i++) {
        var block = blocks[i];
        if (!block || typeof block !== 'object') continue;
        if (this.isToolCallContentType(block.type)) {
          var callName = String(block.name || block.tool || 'tool').trim() || 'tool';
          var callRow = ensureRow({
            name: callName,
            attempt_id: this.resolveToolUseId(block),
            attempt_sequence: rows.length + 1
          }, rows.length);
          if (!callRow.input_preview) callRow.input_preview = this.stringifyStructuredToolValue(this.resolveToolBlockArgs(block), 2000);
          continue;
        }
        if (!this.isToolResultContentType(block.type)) continue;
        var resultName = String(block.name || block.tool || 'tool').trim() || 'tool';
        var resultRow = ensureRow({
          name: resultName,
          attempt_id: this.resolveToolUseId(block),
          attempt_sequence: rows.length + 1
        }, rows.length);
        var resultText = this.stringifyStructuredToolValue(
          block.result != null ? block.result : (
            block.output != null ? block.output : (
              block.content != null ? block.content : (
                block.text != null ? block.text : (
                  block.error != null ? block.error : ''
                )
              )
            )
          ),
          24000
        );
        if (!resultRow.result_preview && resultText) resultRow.result_preview = resultText;
        var rawStatus = String(block.status || '').trim().toLowerCase();
        var blocked = block.blocked === true || rawStatus === 'blocked' || rawStatus === 'policy_denied';
        var isError = block.is_error === true || this.normalizeToolContentType(block.type) === 'tool_result_error' || (!!rawStatus && rawStatus !== 'ok' && !blocked);
        if (blocked) resultRow.blocked = true;
        if (isError) resultRow.is_error = true;
        if (rawStatus) resultRow.status = rawStatus;
      }
      return rows.slice(0, 16);
    },
    mergeToolCardSets: function(baseRows, incomingRows) {
      var merged = Array.isArray(baseRows) ? baseRows.slice() : [];
      var incoming = Array.isArray(incomingRows) ? incomingRows : [];
      var claimedBaseIndexes = {};
      for (var i = 0; i < incoming.length; i++) {
        var candidate = incoming[i];
        if (!candidate) continue;
        var matched = false;
        for (var j = 0; j < merged.length; j++) {
          var current = merged[j];
          if (!current) continue;
          var sameAttempt = !!candidate.attempt_id && String(current.attempt_id || '').trim() === String(candidate.attempt_id || '').trim();
          var sameUnnamedTool = !candidate.attempt_id && String(current.name || '').toLowerCase() === String(candidate.name || '').toLowerCase();
          var adoptUnnamedBase = !sameAttempt && !current.attempt_id && !claimedBaseIndexes[j] && String(current.name || '').toLowerCase() === String(candidate.name || '').toLowerCase();
          if (!sameAttempt && !sameUnnamedTool && !adoptUnnamedBase) continue;
          if (!current.input_preview && candidate.input_preview) current.input_preview = candidate.input_preview;
          if ((!current.result_preview || !String(current.result_preview).trim()) && candidate.result_preview) current.result_preview = candidate.result_preview;
          if (candidate.blocked) current.blocked = true;
          if (candidate.status) current.status = candidate.status;
          if (candidate.is_error) current.is_error = true;
          if (candidate.id) current.id = candidate.id;
          if (candidate.attempt_id) current.attempt_id = candidate.attempt_id;
          if (candidate.attempt_sequence) current.attempt_sequence = candidate.attempt_sequence;
          if (candidate.identity_key) current.identity_key = candidate.identity_key;
          if (!current.tool_attempt_receipt && candidate.tool_attempt_receipt) current.tool_attempt_receipt = candidate.tool_attempt_receipt;
          claimedBaseIndexes[j] = true;
          matched = true;
          break;
        }
        if (!matched) merged.push(candidate);
      }
      return merged.slice(0, 16);
    },
    parseStructuredToolInput: function(tool) {
      var row = tool && typeof tool === 'object' ? tool : {};
      var input = row._detail_loaded ? row.input : row.input_preview;
      if (input && typeof input === 'object' && !Array.isArray(input)) return input;
      var raw = typeof input === 'string' ? String(input).trim() : '';
      if (!raw || raw.charAt(0) !== '{') return {};
      try {
        var parsed = JSON.parse(raw);
        return parsed && typeof parsed === 'object' && !Array.isArray(parsed) ? parsed : {};
      } catch (_) {
        return {};
      }
    },
    toolMetaCandidates: function(tool) {
      var input = this.parseStructuredToolInput(tool);
      var out = [];
      var action = String(input.action || input.method || input.operation || input.op || '').trim();
      if (action) out.push(this.prettifyToolLabel(action));
      var query = String(input.query || input.q || '').trim();
      if (query) out.push('"' + query + '"');
      var url = String(input.url || input.link || '').trim();
      if (url) out.push(url);
      var filePath = String(input.path || input.file || '').trim();
      if (filePath) {
        if (/^\/Users\/[^/]+(\/|$)/.test(filePath)) {
          filePath = filePath.replace(/^\/Users\/[^/]+(\/|$)/, '~$1');
        } else if (/^\/home\/[^/]+(\/|$)/.test(filePath)) {
          filePath = filePath.replace(/^\/home\/[^/]+(\/|$)/, '~$1');
        } else if (/^C:\\Users\\[^\\]+(\\|$)/i.test(filePath)) {
          filePath = filePath.replace(/^C:\\Users\\[^\\]+(\\|$)/i, '~$1');
        }
        out.push(filePath);
      }
      return out.slice(0, 3);
    },
    formatToolAggregateMeta: function(tool) {
      var label = String(tool && tool.name ? tool.name : 'tool').replace(/_/g, ' ').trim() || 'tool';
      var metas = this.toolMetaCandidates(tool);
      if (!metas.length) return label;
      return label + ': ' + metas.join('; ');
    },
    backfillToolRowsFromCompletion: function(rows, payload) {
      var merged = Array.isArray(rows) ? rows.map(function(row) {
        return row && typeof row === 'object' ? Object.assign({}, row) : row;
      }) : [];
      var data = payload && typeof payload === 'object' ? payload : {};
      var completion =
        data.response_finalization &&
        data.response_finalization.tool_completion &&
        typeof data.response_finalization.tool_completion === 'object'
          ? data.response_finalization.tool_completion
          : null;
      var steps = Array.isArray(completion && completion.live_tool_steps)
        ? completion.live_tool_steps
        : [];
      if (!steps.length) return merged.slice(0, 16);
      if (!merged.length) {
        for (var si = 0; si < steps.length && merged.length < 16; si++) {
          var stepSeed = steps[si] && typeof steps[si] === 'object' ? steps[si] : {};
          var stepName = String(stepSeed.tool || stepSeed.name || 'tool').trim() || 'tool';
          var stepStatus = String(stepSeed.status || '').trim();
          if (!stepName && !stepStatus) continue;
          merged.push(this.normalizeResponseToolCard({
            id: 'completion-step-' + (si + 1) + '-' + stepName,
            name: stepName,
            result_preview: stepStatus ? ('Missing tool_result block; last known status: ' + stepStatus) : '',
            is_error: !!stepSeed.is_error,
            status: stepStatus ? stepStatus.toLowerCase() : ''
          }, si, 'completion'));
        }
      }
      for (var i = 0; i < merged.length; i++) {
        var row = merged[i] && typeof merged[i] === 'object' ? merged[i] : null;
        if (!row) continue;
        var rowName = String(row.name || '').trim().toLowerCase();
        var step = null;
        var byIndex = steps[i] && typeof steps[i] === 'object' ? steps[i] : null;
        if (byIndex && String(byIndex.tool || byIndex.name || '').trim().toLowerCase() === rowName && String(byIndex.status || '').trim()) {
          step = byIndex;
        } else {
          for (var si = 0; si < steps.length; si++) {
            var candidate = steps[si] && typeof steps[si] === 'object' ? steps[si] : null;
            if (!candidate) continue;
            if (String(candidate.tool || candidate.name || '').trim().toLowerCase() !== rowName) continue;
            if (!String(candidate.status || '').trim()) continue;
            step = candidate;
            break;
          }
        }
        if (!step) continue;
        var statusText = String(step.status || '').trim();
        if (!row.status && statusText) row.status = statusText.toLowerCase();
        if ((!row.result_preview || !String(row.result_preview).trim()) && statusText) {
          row.result_preview = 'Missing tool_result block; last known status: ' + statusText;
        }
        if (step.is_error === true && !row.blocked) row.is_error = true;
      }
      return merged.slice(0, 16);
    },
    responseToolRowsFromPayload: function(payload, prefix) {
      var data = payload && typeof payload === 'object' ? payload : {};
      var base = this.mergeToolCardSets(
        Array.isArray(data.tools)
          ? data.tools.map(function(row, idx) { return this.normalizeResponseToolCard(row, idx, prefix || 'tool'); }, this)
          : [],
        this.structuredContentToolRows(data, prefix || 'content')
      );
      var completion =
        data.response_finalization &&
        data.response_finalization.tool_completion &&
        typeof data.response_finalization.tool_completion === 'object'
          ? data.response_finalization.tool_completion
          : null;
      var attempts = Array.isArray(completion && completion.tool_attempts)
        ? completion.tool_attempts
        : [];
      if (!attempts.length) return this.backfillToolRowsFromCompletion(base, data).slice(0, 16);
      var merged = base.slice();
      for (var i = 0; i < attempts.length; i++) {
        var attemptCard = this.toolCardFromAttemptReceipt(attempts[i], i, prefix || 'attempt');
        merged = this.mergeToolCardSets(merged, [attemptCard]);
      }
      return this.backfillToolRowsFromCompletion(merged, data).slice(0, 16);
    },
    responseFinalizationFromPayload: function(payload) {
      var data = payload && typeof payload === 'object' ? payload : {};
      return data.response_finalization && typeof data.response_finalization === 'object'
        ? data.response_finalization
        : null;
    },
    readableToolFailureSummary: function(payload, tools) {
      var rows = Array.isArray(tools) ? tools.filter(function(tool) {
        return !!(tool && String(tool.name || '').toLowerCase() !== 'thought_process');
      }) : [];
      if (!rows.length) return '';
      var blocked = rows.find(function(tool) {
        return !!(tool && !tool.running && this.isBlockedTool(tool));
      }, this);
      if (blocked) {
        var blockedName = this.toolDisplayName(blocked);
        var blockedDetail = this.toolResultSummarySnippet(blocked) || String(blocked.status || '').trim() || 'blocked by policy';
        return 'The ' + (blockedName || 'tool') + ' step was blocked before I could finish the answer: ' + blockedDetail;
      }
      var failed = rows.find(function(tool) {
        return !!(tool && !tool.running && tool.is_error);
      });
      if (failed) {
        var failedName = this.toolDisplayName(failed);
        var failedDetail = this.toolResultSummarySnippet(failed) || String(failed.status || '').trim() || 'step failed';
        return 'The ' + (failedName || 'tool') + ' step failed before I could finish the answer: ' + failedDetail;
      }
      var actionableWeb = rows.find(function(tool) {
        if (!tool || tool.running || !this.isWebLikeToolName(tool.name || '')) return false;
        return (
          this.textMentionsContextGuard(this.toolProjectionPreviewText(tool) || '') ||
          this.textLooksNoFindingsPlaceholder(this.toolProjectionPreviewText(tool) || '') ||
          this.textLooksToolAckWithoutFindings(this.toolProjectionPreviewText(tool) || '')
        );
      }, this);
      if (actionableWeb) {
        return this.lowSignalWebToolSummary(actionableWeb);
      }
      return '';
    },
    fallbackAssistantTextFromPayload: function(payload, tools) {
      var data = payload && typeof payload === 'object' ? payload : {};
      var workflowText = this.workflowResponseTextFromPayload(data);
      if (workflowText) return workflowText;
      return '';
    },
    assistantTurnMetadataFromPayload: function(payload, tools) {
      var data = payload && typeof payload === 'object' ? payload : {};
      var out = {};
      if (data.response_workflow && typeof data.response_workflow === 'object') {
        out.response_workflow = data.response_workflow;
      }
      var finalization = this.responseFinalizationFromPayload(data);
      if (finalization) out.response_finalization = finalization;
      if (data.turn_transaction && typeof data.turn_transaction === 'object') {
        out.turn_transaction = data.turn_transaction;
      }
      if (Array.isArray(data.terminal_transcript) && data.terminal_transcript.length) {
        out.terminal_transcript = data.terminal_transcript.slice(0, 48);
      }
      if (data.attention_queue && typeof data.attention_queue === 'object') {
        out.attention_queue = data.attention_queue;
      }
      var failureSummary = this.readableToolFailureSummary(data, tools);
      if (failureSummary) out.tool_failure_summary = failureSummary;
      return out;
    },


    async ensureSystemTerminalSession() {
      var existing = String(this.systemTerminalSessionId || '').trim();
      if (existing) return existing;
      var preferredId = String(this.systemThreadId || 'system').trim() || 'system';
      this.systemTerminalSessionId = preferredId;
      return preferredId;
    },

    async _sendSystemTerminalPayload(command) {
      var cmd = String(command || '').trim();
      if (!cmd) return;
      this.sending = true;
      this.setAgentLiveActivity(this.systemThreadId || 'system', 'working');
      this._responseStartedAt = Date.now();
      this._appendTerminalMessage({
        role: 'terminal',
        text: this._terminalPromptLine(this.terminalPromptPath, cmd),
        meta: this.terminalPromptPath,
        tools: [],
        ts: Date.now(),
        terminal_source: 'user',
        cwd: this.terminalPromptPath
      });
      this.scrollToBottom();
      this.scheduleConversationPersist();
      try {
        var sessionId = await this.ensureSystemTerminalSession();
        var ack = await InfringAPI.post('/api/shell-socket/terminal/commands', {
          agent_id: sessionId,
          command: cmd,
          cwd: this.terminalPromptPath
        });
        if (!ack || ack.rejected) throw new Error(String((ack && ack.reason_code) || 'terminal_command_rejected'));
        this.sending = false;
        this._responseStartedAt = 0;
        this.setAgentLiveActivity(this.systemThreadId || 'system', 'idle', { optimistic: true, source: 'shell_socket_terminal_ack' });
      } catch (error) {
        this.sending = false;
        this._responseStartedAt = 0;
        InfringToast.error(error && error.message ? error.message : 'command failed');
      }
    },


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
      var sendTrigger = this._pendingPromptSuggestionSend && typeof this._pendingPromptSuggestionSend === 'object'
        ? this._pendingPromptSuggestionSend
        : null;
      var sendTriggerSource = sendTrigger && sendTrigger.source
        ? String(sendTrigger.source).slice(0, 80)
        : 'composer';
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
        if (typeof this.fetchAgentRuntimeSlashCommands === 'function') {
          await this.fetchAgentRuntimeSlashCommands(false);
        }
        var matched = typeof this.findSlashCommandDefinition === 'function'
          ? this.findSlashCommandDefinition(routedCmd)
          : this.slashCommands.find(function(c) { return c.cmd === routedCmd; });
        if (matched) {
          this.executeSlashCommand(matched.cmd, routedArgs, matched);
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
          agent_runtime_engine_id: selectedRuntimeEngineId,
          trigger_source: sendTriggerSource
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
      if (usesExternalRuntime) {
        if (typeof this.syncActiveChatMessages === 'function') this.syncActiveChatMessages();
        if (typeof this.scheduleMessageRenderWindowUpdate === 'function') this.scheduleMessageRenderWindowUpdate();
      }
      this._sendPayload(finalText, uploadedFiles, msgImages, {
        agent_id: activeAgent.id,
        agent_runtime_engine_id: selectedRuntimeEngineId,
        trigger_source: sendTriggerSource
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
      var alreadyPending = pending.some(function(item) { return String(item && item.approval_id || '') === id; });
      pending = pending.filter(function(item) { return String(item && item.approval_id || '') !== id; });
      pending.unshift(row);
      this.pendingAgentRuntimePermissionRequests = pending.slice(0, 5);
      if (!alreadyPending) InfringToast.info('Permission requested: ' + this.permissionRequestPreview(row));
    },

    syncAgentRuntimePendingApprovals: function(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var now = Date.now();
      if (!opts.force && this._agentRuntimePendingApprovalSyncAt && (now - this._agentRuntimePendingApprovalSyncAt) < 4000) {
        return Promise.resolve(this.pendingAgentRuntimePermissionRequests || []);
      }
      this._agentRuntimePendingApprovalSyncAt = now;
      var self = this;
      return InfringAPI.get('/api/shell-socket/approvals/pending').then(function(payload) {
        var rows = payload && Array.isArray(payload.pending_requests) ? payload.pending_requests : [];
        rows.slice(0, 5).forEach(function(row) {
          if (row && row.approval_id) self.enqueueAgentRuntimePermissionRequest(row);
        });
        return rows;
      }).catch(function() {
        return self.pendingAgentRuntimePermissionRequests || [];
      });
    },

    ensureAgentRuntimePendingApprovalSyncLoop: function() {
      if (this._agentRuntimePendingApprovalSyncLoop) return;
      var self = this;
      this._agentRuntimePendingApprovalSyncLoop = window.setInterval(function() {
        if (typeof self.syncAgentRuntimePendingApprovals === 'function') {
          self.syncAgentRuntimePendingApprovals({ force: false }).catch(function() {});
        }
      }, 5000);
      this.syncAgentRuntimePendingApprovals({ force: true }).catch(function() {});
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
        var liveRow = typeof self.findAgentRuntimeLiveThinkingRow === 'function'
          ? self.findAgentRuntimeLiveThinkingRow(request.engine_id, request.live_message_id)
          : null;
        var liveEngineId = String(request.engine_id || self.selectedAgentRuntimeEngineId || '').trim();
        if (choice === 'deny') {
          var denyText = 'Permission denied: ' + self.permissionRequestPreview(request);
          if (liveRow && typeof self.appendAgentRuntimeActivityToThinkingRow === 'function') {
            self.appendAgentRuntimeActivityToThinkingRow({
              activity_kind: 'permission_event',
              provider_event_type: 'permission.denied',
              status: 'blocked',
              display_text: denyText,
              text: denyText,
              engine_id: liveEngineId,
              item_id: approvalId
            }, liveEngineId);
            var denyEvents = Array.isArray(liveRow.agent_runtime_live_events) ? liveRow.agent_runtime_live_events.slice(-80) : [];
            var denyDurationMs = Math.max(0, Date.now() - Number(liveRow._stream_started_at || liveRow.ts || Date.now()));
            var denyTool = self.agentRuntimeActivityEventsToDecisionTool(denyEvents, liveEngineId, denyDurationMs, denyText);
            self.finalizeAgentRuntimeThinkingRow(liveRow, {
              text: denyText,
              meta: 'runtime ' + (liveEngineId || 'agent_runtime') + ' | permission denied',
              tools: denyTool ? [denyTool] : [],
              agent_activity_events: denyEvents,
              agent_activity_event_count: denyEvents.length,
              pending_permission_request: request,
              projection_kind: 'permission_request'
            });
          }
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
          var approveText = 'Permission approved: ' + self.permissionRequestPreview(request);
          if (liveRow && typeof self.appendAgentRuntimeActivityToThinkingRow === 'function') {
            self.appendAgentRuntimeActivityToThinkingRow({
              activity_kind: 'permission_event',
              provider_event_type: 'permission.approved',
              status: 'completed',
              display_text: approveText,
              text: approveText,
              engine_id: liveEngineId,
              item_id: approvalId
            }, liveEngineId);
          }
          if (payload && payload.execution_result && payload.execution_result.ok) {
            if (liveRow && typeof self.appendAgentRuntimeActivityToThinkingRow === 'function') {
              var executedText = String(payload.execution_result.display_text || payload.execution_result.path || 'Approved action completed.').trim();
              if (payload.execution_result.path && executedText === String(payload.execution_result.path).trim()) executedText = 'Created ' + executedText + '.';
              self.appendAgentRuntimeActivityToThinkingRow({
                activity_kind: 'tool_call_event',
                provider_event_type: 'permission.effect.executed',
                status: 'completed',
                display_text: executedText,
                text: executedText,
                engine_id: liveEngineId,
                item_id: approvalId + ':effect'
              }, liveEngineId);
              var execEvents = Array.isArray(liveRow.agent_runtime_live_events) ? liveRow.agent_runtime_live_events.slice(-80) : [];
              var execDurationMs = Math.max(0, Date.now() - Number(liveRow._stream_started_at || liveRow.ts || Date.now()));
              var execTool = self.agentRuntimeActivityEventsToDecisionTool(execEvents, liveEngineId, execDurationMs);
              self.finalizeAgentRuntimeThinkingRow(liveRow, {
                text: executedText,
                meta: 'runtime ' + (liveEngineId || 'agent_runtime') + ' | approval executed',
                tools: execTool ? [execTool] : [],
                agent_activity_events: execEvents,
                agent_activity_event_count: execEvents.length,
                result_ref: String(payload.execution_result.result_ref || ''),
                receipt_ref: String(payload.execution_result.receipt_ref || ''),
                pending_permission_request: request,
                projection_kind: 'permission_request'
              });
              return payload;
            }
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
                decision_receipt_ref: String(payload && payload.decision_receipt_ref || '').trim(),
                live_message_id: String(request.live_message_id || '').trim()
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
      if (value === 'decision_dialog') return 'Decision dialog';
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
        var lineKind = this.agentRuntimeActivityLineKind(event);
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
          activity_line_kind: lineKind,
          activity_text: text || providerType,
          activity_status: String(event.status || 'completed').trim() || 'completed',
          agent_activity_event: true,
          agent_runtime_live_activity: true,
          agent_runtime_dialog_line: lineKind === 'dialog',
          agent_runtime_activity_dialog_text: text || providerType,
          agent_runtime_activity_latest: false,
          notice_type: kind,
          running: false,
          ts: Date.now()
        });
      }
      return out;
    },

    agentRuntimeActivityLineKind: function(event) {
      var row = event && typeof event === 'object' ? event : {};
      var kind = String(row.activity_kind || row.kind || row.type || '').toLowerCase();
      var providerType = String(row.provider_event_type || row.event_type || '').toLowerCase();
      var joined = kind + ' ' + providerType;
      if (
        joined.indexOf('decision') >= 0 ||
        joined.indexOf('reasoning') >= 0 ||
        joined.indexOf('thought') >= 0 ||
        joined.indexOf('plan') >= 0 ||
        joined.indexOf('assistant_delta') >= 0
      ) {
        return 'dialog';
      }
      if (
        joined.indexOf('command') >= 0 ||
        joined.indexOf('exec') >= 0 ||
        joined.indexOf('shell') >= 0 ||
        joined.indexOf('file') >= 0 ||
        joined.indexOf('patch') >= 0 ||
        joined.indexOf('write') >= 0 ||
        joined.indexOf('tool') >= 0 ||
        joined.indexOf('search') >= 0 ||
        joined.indexOf('permission') >= 0 ||
        joined.indexOf('approval') >= 0 ||
        joined.indexOf('error') >= 0
      ) {
        return 'tool';
      }
      return 'status';
    },

    agentRuntimeActivityEventToTraceLine: function(event, engineId) {
      var row = event && typeof event === 'object' ? event : {};
      var text = String(row.display_text || row.text || row.summary || '').replace(/\r\n/g, '\n').trim();
      var providerType = String(row.provider_event_type || row.event_type || '').trim();
      if (!text && providerType) text = this.agentRuntimeProviderEventLabel(providerType);
      if (!text) return null;
      var status = String(row.status || '').trim();
      var kind = this.agentRuntimeActivityLineKind(row);
      return {
        id: String(row.item_id || row.sequence_no || (kind + '-' + Date.now() + '-' + Math.random())).slice(0, 180),
        text: text.split('\n').map(function(part) {
          return String(part || '').replace(/\s+/g, ' ').trim();
        }).filter(Boolean).join('\n').slice(0, 1400),
        line_kind: kind,
        state: status === 'failed' || status === 'error'
          ? 'error'
          : status === 'paused_pending_approval'
            ? 'blocked'
            : status === 'running' || status === 'started' || status === 'activity'
              ? 'running'
              : 'done',
        status: status,
        engine_id: String(row.engine_id || engineId || '').trim(),
        ts: Date.now()
      };
    },

    appendActivityTraceLineToMessage: function(target, traceLine) {
      if (!target || !traceLine || !traceLine.text) return false;
      var traceRows = Array.isArray(target.agent_runtime_live_trace_rows) ? target.agent_runtime_live_trace_rows.slice(-47) : [];
      var lastTrace = traceRows.length ? traceRows[traceRows.length - 1] : null;
      var nextText = String(traceLine.text || '').trim();
      var nextKind = String(traceLine.line_kind || 'status');
      if (
        lastTrace &&
        String(lastTrace.line_kind || '') === nextKind &&
        nextKind === 'dialog' &&
        (
          nextText.indexOf(String(lastTrace.text || '').trim()) === 0 ||
          String(lastTrace.text || '').trim().indexOf(nextText) === 0
        )
      ) {
        if (nextText.length >= String(lastTrace.text || '').length) lastTrace.text = nextText;
        lastTrace.state = traceLine.state || lastTrace.state;
        lastTrace.status = traceLine.status || lastTrace.status;
        lastTrace.ts = traceLine.ts;
      } else if (
        lastTrace &&
        String(lastTrace.text || '').trim() === nextText &&
        String(lastTrace.line_kind || '') === nextKind
      ) {
        lastTrace.state = traceLine.state || lastTrace.state;
        lastTrace.status = traceLine.status || lastTrace.status;
        lastTrace.ts = traceLine.ts;
      } else {
        traceRows.push(traceLine);
      }
      target.agent_runtime_live_trace_rows = traceRows.slice(-48);
      return true;
    },

    appendNativeWorkflowActivityToThinkingRow: function(target, event) {
      if (!target || typeof target !== 'object') return false;
      var row = event && typeof event === 'object' ? event : {};
      var engineId = String(row.engine_id || target.agent_runtime_engine_id || 'infring_native').trim() || 'infring_native';
      var normalized = Object.assign({}, row, {
        type: 'agent_activity_event',
        source: String(row.source || 'infring_native_workflow_projection'),
        engine_id: engineId,
        sequence_no: Number(row.sequence_no || 0) || ((Array.isArray(target.agent_runtime_live_events) ? target.agent_runtime_live_events.length : 0) + 1)
      });
      var traceLine = this.agentRuntimeActivityEventToTraceLine(normalized, engineId);
      if (traceLine) this.appendActivityTraceLineToMessage(target, traceLine);
      var liveEvents = Array.isArray(target.agent_runtime_live_events) ? target.agent_runtime_live_events.slice(-79) : [];
      liveEvents.push(normalized);
      target.agent_runtime_live_events = liveEvents;
      var liveDialog = this.agentRuntimeActivityEventsToDecisionDialog(liveEvents);
      if (liveDialog) target.agent_runtime_decision_dialog_text = liveDialog;
      target.agent_runtime_engine_id = engineId;
      target._stream_updated_at = Date.now();
      return true;
    },

    nativeWorkflowDecisionToolFromMessage: function(msg, durationMs, extraDialog) {
      var row = msg && typeof msg === 'object' ? msg : {};
      var events = Array.isArray(row.agent_runtime_live_events) ? row.agent_runtime_live_events.slice(-80) : [];
      var extra = String(extraDialog || row._thoughtText || row._reasoning || '').trim();
      if (!events.length && !extra) return null;
      if (typeof this.agentRuntimeActivityEventsToDecisionTool === 'function') {
        return this.agentRuntimeActivityEventsToDecisionTool(events, row.agent_runtime_engine_id || 'infring_native', durationMs, extra);
      }
      return extra && typeof this.makeThoughtToolCard === 'function' ? this.makeThoughtToolCard(extra, durationMs) : null;
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
      var traceLine = this.agentRuntimeActivityEventToTraceLine(event, engineId);
      if (!tools.length && !traceLine) return;
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
      if (traceLine && traceLine.text) this.appendActivityTraceLineToMessage(target, traceLine);
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
      if (tools.length) target.tools = target.tools.concat(tools).slice(-40);
      var latestTool = tools[tools.length - 1] || {};
      var latestText = String(
        (traceLine && traceLine.line_kind !== 'dialog' && traceLine.text) ||
        latestTool.display_text ||
        latestTool.summary ||
        latestTool.name ||
        ''
      ).trim();
      target.text = latestText || 'Working through runtime activity...';
      target.thinking_status = latestText || 'Working through runtime activity...';
      target.meta = 'Agent runtime: ' + String(engineId || 'runtime') + ' | streaming activity';
      target._stream_updated_at = Date.now();
      if (typeof this.syncActiveChatMessages === 'function') this.syncActiveChatMessages();
      if (typeof this.scheduleMessageRenderWindowUpdate === 'function') this.scheduleMessageRenderWindowUpdate();
      this.scrollToBottom();
    },

    findAgentRuntimeLiveThinkingRow: function(engineId, messageId) {
      var rows = Array.isArray(this.messages) ? this.messages : [];
      var targetId = String(messageId || '').trim();
      var runtimeId = String(engineId || '').trim();
      for (var i = rows.length - 1; i >= 0; i -= 1) {
        var row = rows[i];
        if (!row || !row.thinking) continue;
        if (targetId && String(row.id || '') === targetId) return row;
        if (!targetId && (!runtimeId || String(row.agent_runtime_engine_id || '') === runtimeId)) return row;
      }
      return null;
    },

    finalizeAgentRuntimeThinkingRow: function(row, payload) {
      var target = row && typeof row === 'object' ? row : null;
      var data = payload && typeof payload === 'object' ? payload : {};
      if (!target) return false;
      target.thinking = false;
      target.streaming = false;
      target.text = String(data.text || '').trim();
      target.meta = String(data.meta || '').trim();
      target.tools = Array.isArray(data.tools) ? data.tools : [];
      target.agent_activity_events = Array.isArray(data.agent_activity_events) ? data.agent_activity_events : [];
      target.agent_activity_event_count = Number(data.agent_activity_event_count || 0) || target.agent_activity_events.length;
      target.result_ref = String(data.result_ref || '').slice(0, 240);
      target.receipt_ref = String(data.receipt_ref || '').slice(0, 240);
      target.isHtml = false;
      target._typingVisual = false;
      target.pending_permission_request = data.pending_permission_request || null;
      if (data.projection_kind) target.projection_kind = data.projection_kind;
      target.projection_schema_version = data.projection_schema_version || 1;
      target._stream_updated_at = Date.now();
      this.markAgentMessageComplete(target);
      if (typeof this.syncActiveChatMessages === 'function') this.syncActiveChatMessages();
      if (typeof this.scheduleMessageRenderWindowUpdate === 'function') this.scheduleMessageRenderWindowUpdate();
      this.scheduleConversationPersist();
      return true;
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
      var opts = resumeOptions && typeof resumeOptions === 'object' ? resumeOptions : {};
      var approvalResume = (opts.approval_id || opts.resume_token || opts.approval_decision)
        ? opts
        : null;
      var thinkingMessage = this.findAgentRuntimeLiveThinkingRow
        ? this.findAgentRuntimeLiveThinkingRow(engineId, approvalResume && approvalResume.live_message_id)
        : null;
      if (thinkingMessage) {
        thinkingMessage.thinking = true;
        thinkingMessage.streaming = true;
        thinkingMessage.meta = 'Agent runtime: ' + engineId + ' | resuming';
        thinkingMessage.agent_runtime_engine_id = engineId;
        thinkingMessage.agent_id = targetAgentId;
        thinkingMessage.agent_name = this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '';
        thinkingMessage._stream_updated_at = Date.now();
        if (!Number.isFinite(Number(thinkingMessage._stream_started_at))) thinkingMessage._stream_started_at = startedAt;
        if (typeof this.appendAgentRuntimeActivityToThinkingRow === 'function') {
          this.appendAgentRuntimeActivityToThinkingRow({
            activity_kind: 'permission_event',
            provider_event_type: 'permission.resume',
            status: 'running',
            display_text: 'Resuming agent turn after approval.',
            text: 'Resuming agent turn after approval.',
            engine_id: engineId,
            item_id: approvalResume && approvalResume.approval_id || 'approval-resume'
          }, engineId);
        }
      } else {
        thinkingMessage = {
          id: ++msgId,
          role: 'agent',
          text: '',
          meta: 'Agent runtime: ' + engineId,
          thinking: true,
          streaming: true,
          tools: [],
          ts: Date.now(),
          _stream_started_at: startedAt,
          _stream_updated_at: startedAt,
          agent_id: targetAgentId,
          agent_name: this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '',
          agent_runtime_engine_id: engineId
        };
        this.messages.push(thinkingMessage);
      }
      this.scrollToBottom();
      this.scheduleConversationPersist();

      try {
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
          input_source: String(opts.trigger_source || 'composer').slice(0, 80),
          turn_trigger_source: String(opts.trigger_source || 'composer').slice(0, 80),
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
        var projectedPendingPermissionRequest = res && res.pending_permission_request
          ? res.pending_permission_request
          : (res && res.permission_request ? res.permission_request : null);
        if (projectedPendingPermissionRequest) {
          var pendingPermissionRequest = projectedPendingPermissionRequest;
          pendingPermissionRequest.projection_kind = 'permission_request';
          pendingPermissionRequest.projection_schema_version = 1;
          var pendingRuntimeDurationMs = Math.max(0, Date.now() - startedAt);
          var pendingRuntimeDuration = this.formatResponseDuration(pendingRuntimeDurationMs);
          var pendingResponseActivityEvents = Array.isArray(res && res.agent_activity_events) ? res.agent_activity_events : [];
          var pendingFinalActivityEvents = pendingResponseActivityEvents.length ? pendingResponseActivityEvents : streamedActivityEvents;
          var pendingLiveEvents = Array.isArray(thinkingMessage.agent_runtime_live_events) ? thinkingMessage.agent_runtime_live_events.slice(-80) : [];
          if (pendingLiveEvents.length) pendingFinalActivityEvents = pendingLiveEvents;
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
            live_message_id: String(thinkingMessage && thinkingMessage.id || ''),
            paused_reason: 'waiting_for_permission_decision'
          };
          pendingPermissionRequest.live_message_id = String(thinkingMessage && thinkingMessage.id || '');
          pendingPermissionRequest.status = 'paused_pending_approval';
          this.enqueueAgentRuntimePermissionRequest(pendingPermissionRequest);
          if (typeof this.appendAgentRuntimeActivityToThinkingRow === 'function') {
            this.appendAgentRuntimeActivityToThinkingRow({
              activity_kind: 'permission_request',
              provider_event_type: 'permission.requested',
              status: 'paused_pending_approval',
              display_text: 'Waiting for approval: ' + pendingToolId,
              text: 'Waiting for approval: ' + pendingToolId,
              engine_id: engineId,
              item_id: String(pendingPermissionRequest.approval_id || '')
            }, engineId);
          }
          thinkingMessage.text = 'Waiting for approval: ' + pendingToolId;
          thinkingMessage.thinking_status = 'Waiting for approval: ' + pendingToolId;
          thinkingMessage.meta = pendingMeta;
          thinkingMessage.tools = pendingDecisionTool ? [pendingDecisionTool] : (Array.isArray(thinkingMessage.tools) ? thinkingMessage.tools : []);
          thinkingMessage.agent_activity_events = pendingFinalActivityEvents.slice(-80);
          thinkingMessage.agent_activity_event_count = Number(res && res.activity_event_count) || pendingFinalActivityEvents.length;
          thinkingMessage.result_ref = String(res && res.result_ref || '').slice(0, 240);
          thinkingMessage.receipt_ref = String(res && res.receipt_ref || '').slice(0, 240);
          thinkingMessage.agent_id = targetAgentId;
          thinkingMessage.agent_name = this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '';
          thinkingMessage.agent_runtime_engine_id = engineId;
          thinkingMessage.pending_permission_request = pendingPermissionRequest;
          thinkingMessage.approval_pause_active = true;
          thinkingMessage.projection_kind = 'permission_request';
          thinkingMessage.projection_schema_version = 1;
          thinkingMessage.thinking = true;
          thinkingMessage.streaming = true;
          thinkingMessage._stream_updated_at = Date.now();
          this._clearPendingWsRequest(targetAgentId);
          this._inflightPayload = null;
          this.sending = false;
          this._responseStartedAt = 0;
          this.tokenCount = 0;
          this._clearTypingTimeout();
          this.setAgentLiveActivity(targetAgentId, 'idle', { optimistic: true, source: 'agent_runtime_permission_wait' });
          this.scheduleConversationPersist();
          if (typeof this.syncActiveChatMessages === 'function') this.syncActiveChatMessages();
          if (typeof this.scheduleMessageRenderWindowUpdate === 'function') this.scheduleMessageRenderWindowUpdate();
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
        var accumulatedLiveEvents = Array.isArray(thinkingMessage.agent_runtime_live_events) ? thinkingMessage.agent_runtime_live_events.slice(-80) : [];
        if (accumulatedLiveEvents.length) finalActivityEvents = accumulatedLiveEvents;
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
          var hadPermissionPause = !!(res && (res.pending_permission_request || res.permission_request));
          if (!hadPermissionPause) InfringToast.info('Agent runtime returned no display text.');
          var emptyText = 'Agent runtime returned no display text.';
          if (typeof this.appendAgentRuntimeActivityToThinkingRow === 'function') {
            this.appendAgentRuntimeActivityToThinkingRow({
              activity_kind: 'error',
              provider_event_type: 'turn.no_display_text',
              status: 'failed',
              display_text: emptyText,
              text: emptyText,
              engine_id: engineId,
              item_id: 'turn-no-display-text'
            }, engineId);
          }
          var emptyEvents = Array.isArray(thinkingMessage.agent_runtime_live_events) ? thinkingMessage.agent_runtime_live_events.slice(-80) : finalActivityEvents;
          var emptyTool = this.agentRuntimeActivityEventsToDecisionTool(emptyEvents, engineId, runtimeDurationMs, emptyText);
          this.finalizeAgentRuntimeThinkingRow(thinkingMessage, {
            text: emptyText,
            meta: runtimeMeta + ' | no display text',
            tools: emptyTool ? [emptyTool] : runtimeActivityTools,
            agent_activity_events: emptyEvents,
            agent_activity_event_count: emptyEvents.length,
            result_ref: String(res && res.result_ref || '').slice(0, 240),
            receipt_ref: String(res && res.receipt_ref || '').slice(0, 240)
          });
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
        if (!this.finalizeAgentRuntimeThinkingRow(thinkingMessage, {
          text: runtimeText,
          meta: runtimeMeta,
          tools: runtimeActivityTools,
          agent_activity_events: finalActivityEvents.slice(-80),
          agent_activity_event_count: Number(res && res.activity_event_count) || runtimeActivityTools.length,
          result_ref: String(res && res.result_ref || '').slice(0, 240),
          receipt_ref: String(res && res.receipt_ref || '').slice(0, 240)
        })) {
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
        }
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
          trigger_source: String(opts.trigger_source || 'composer').slice(0, 80),
          failover_attempted: !!opts.retry_from_failover,
          created_at: Date.now()
        };
      } else {
        this._inflightPayload.final_text = String(finalText || '');
        this._inflightPayload.uploaded_files = safeFiles;
        this._inflightPayload.msg_images = safeImages;
        this._inflightPayload.agent_runtime_engine_id = runtimeEngineId;
        this._inflightPayload.trigger_source = String(opts.trigger_source || this._inflightPayload.trigger_source || 'composer').slice(0, 80);
        this._inflightPayload.retry_started_at = Date.now();
      }
      this._pendingAutoModelSwitchBaseline = '';
      var useRuntimeSocketPath = typeof this.shouldUseAgentRuntimeSocketPath === 'function'
        ? this.shouldUseAgentRuntimeSocketPath(runtimeEngineId)
        : (typeof this.isExternalAgentRuntimeEngineSelected === 'function' && this.isExternalAgentRuntimeEngineSelected(runtimeEngineId));
      if (useRuntimeSocketPath) {
        await this._sendAgentRuntimeSocketPayload(targetAgentId, finalText, safeFiles, safeImages, runtimeEngineId, {
          trigger_source: String(opts.trigger_source || 'composer').slice(0, 80)
        });
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

// Layer ownership: client/runtime/systems/ui (dashboard static UX surface only; no runtime authority).
          this._inflightPayload = null;
          this.refreshPromptSuggestions(true, 'post-http-abort');
          this.scheduleConversationPersist();
          return;
        }
        if (
          !opts.retry_from_agent_rebind &&
          (lowerHttpError.indexOf('agent_not_found') >= 0 || lowerHttpError.indexOf('agent not found') >= 0)
        ) {
          var reboundAgent = this.ensureValidCurrentAgent({ clear_when_missing: true });
          if (!reboundAgent || String(reboundAgent.id || '') === String(targetAgentId || '')) {
            reboundAgent = await this.rebindCurrentAgentAuthoritative({
              preferred_id: targetAgentId,
              clear_when_missing: true
            });
          }
          var reboundAgentId = reboundAgent && reboundAgent.id ? String(reboundAgent.id) : '';
          if (reboundAgentId && reboundAgentId !== targetAgentId) {
            this.addNoticeEvent({
              notice_label:
                'Active agent reference expired. Switched to ' +
                String(reboundAgent.name || reboundAgent.id || reboundAgentId) +
                ' and retried.',
              notice_type: 'warn',
              ts: Date.now(),
            });
            await this._sendPayload(finalText, uploadedFiles, msgImages, {
              agent_id: reboundAgentId,
              retry_from_agent_rebind: true,
            });
            return;
          }
        }
        var noModelsError =
          lowerHttpError.indexOf('no_models_available') >= 0 ||
          lowerHttpError.indexOf('no models available') >= 0;
        if (noModelsError) {
          this.injectNoModelsGuidance('send_error');
          this._inflightPayload = null;
          this.scheduleConversationPersist();
          return;
        }
        handedOffToRecovery = await this.attemptAutomaticFailoverRecovery(
          'http_error',
          rawHttpError,
          { remove_last_agent_failure: false }
        );
        if (!handedOffToRecovery) {
          var rawSendErrorText = String(rawHttpError || (e && e.message) || '').replace(/\s+/g, ' ').trim();
          var lowerSendErrorText = rawSendErrorText.toLowerCase();
          var isTransientDisconnectError =
            lowerSendErrorText === 'fetch failed' ||
            lowerSendErrorText === 'failed to fetch' ||
            lowerSendErrorText === 'connect failed' ||
            lowerSendErrorText.indexOf('gateway connect failed') >= 0;
          var normalizedSendErrorText = (function(message) {
            var raw = String(message || '').replace(/\s+/g, ' ').trim();
            var lower = raw.toLowerCase();
            if (!raw || lower === 'unknown error') return 'Connection failed before the runtime returned a usable response. Try again after the gateway is reachable.';
            if (lower.indexOf('pairing required') >= 0) return 'Gateway pairing is required. Open Settings, pair this dashboard with the gateway, then try again.';
            if (
              lower.indexOf('device identity required') >= 0 ||
              lower.indexOf('secure context') >= 0 ||
              lower.indexOf('https/localhost') >= 0
            ) return 'This action requires HTTPS or localhost. Reopen the dashboard from a trusted origin, then try again.';
            if (
              lower.indexOf('unauthorized') >= 0 ||
              lower.indexOf('token mismatch') >= 0 ||
              lower.indexOf('token missing') >= 0 ||
              lower.indexOf('auth failed') >= 0 ||
              lower.indexOf('authentication') >= 0
            ) return 'Gateway authentication failed. Verify the API token or password in Settings, then retry.';
            if (
              lower === 'fetch failed' ||
              lower === 'failed to fetch' ||
              lower === 'connect failed' ||
              lower.indexOf('gateway connect failed') >= 0
            ) return 'Gateway connect failed. Check runtime availability, pairing, and auth settings, then retry.';
            return 'Connection error: ' + raw;
          })(rawHttpError || (e && e.message) || '');
          if (!isTransientDisconnectError) {
            this.pushSystemMessage({
              text: normalizedSendErrorText,
              meta: '',
              tools: [],
              system_origin: 'http:error',
              ts: Date.now(),
              dedupe_window_ms: 12000
            });
          }
          this._inflightPayload = null;
        } else {
          return;

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

        }
        streamedTools = streamedTools.concat(Array.isArray(row.tools) ? row.tools : []);
      }
      return {
        text: streamedText,
        tools: streamedTools,
        thought: String(streamedThought || '').trim()
      };
    },
    extractThinkingLeak: function(text) {
      if (!text) return { thought: '', content: '' };
      var raw = String(text).replace(/\r\n?/g, '\n');
      var trimmed = raw.replace(/^\s+/, '');
      if (!trimmed) return { thought: '', content: '' };
      var thinkingPrefix = /^(thinking(?:\s+out\s+loud)?(?:\.\.\.|:)?|analysis(?:\.\.\.|:)?|reasoning(?:\.\.\.|:)?)/i;
      var explicitPrefix = thinkingPrefix.test(trimmed);
      if (!explicitPrefix && !this.looksLikeThoughtLeak(trimmed)) return { thought: '', content: raw };
      var splitAt = this.findThinkingBoundary(trimmed);
      if (splitAt < 0) return { thought: trimmed.trim(), content: '' };
      return {
        thought: trimmed.slice(0, splitAt).trim(),
        content: trimmed.slice(splitAt).trim()
      };
    },

    looksLikeThoughtLeak: function(text) {
      var value = String(text || '').replace(/\s+/g, ' ').trim();
      if (!value) return false;
      if (value.length < 80) return false;
      var lead = /^(alright|okay|ok|hmm|let me|i need to|to answer this|first[, ]|i should|i will|i'm going to)\b/i;
      if (!lead.test(value)) return false;
      var markers = [
        /\b(user'?s request|the user asked|address the user|step by step)\b/i,
        /\blet me think\b/i,
        /\bi need to\b/i,
        /\bfirst[, ]/i,
        /\bcheck\b/i,
        /\bconsider\b/i
      ];
      var hits = 0;
      for (var i = 0; i < markers.length; i++) {
        if (markers[i].test(value)) hits += 1;
      }
      return hits >= 2;
    },
    findThinkingBoundary: function(text) {
      if (!text) return -1;
      var boundaries = [];
      var markers = [
        /\n\s*final answer\s*:/i,
        /\n\s*answer\s*:/i,
        /\n\s*response\s*:/i,
        /\n\s*output\s*:/i,
        /\n\s*```/i,
        /\n\s*\n(?=\s*[\{\[])/,
      ];
      markers.forEach(function(rx) {
        var match = text.match(rx);
        if (match && Number.isFinite(match.index)) {
          boundaries.push(match.index + 1);
        }
      });
      if (!boundaries.length) return -1;
      boundaries.sort(function(a, b) { return a - b; });
      return boundaries[0];
    },

    makeThoughtToolCard: function(thoughtText, durationMs) {
      var ms = Number(durationMs || 0);
      if (!Number.isFinite(ms) || ms < 0) ms = 0;
      return {
        id: 'thought-' + Date.now() + '-' + Math.floor(Math.random() * 10000),
        name: 'thought_process',
        running: false,
        expanded: false,
        input: String(thoughtText || '').trim(),
        result: '',
        is_error: false,
        duration_ms: ms
      };
    },

    appendThoughtChunk: function(base, chunk) {
      var prior = String(base || '').trim();
      var next = String(chunk || '').trim();
      if (!next) return prior;
      if (!prior) return next;
      if (prior.slice(-next.length) === next) return prior;
      var merged = prior + '\n' + next;
      if (merged.length > 8000) {
        merged = merged.slice(merged.length - 8000);
      }
      return merged;
    },
    latestCompleteSentence: function(inputText) {
      var raw = String(inputText || '')
        .replace(/<[^>]*>/g, ' ')
        .replace(/^\*+|\*+$/g, '')
        .replace(/\r/g, '')
        .trim();
      if (!raw) return '';
      var value = raw.replace(/[ \t]+/g, ' ').trim();
      if (!value) return '';
      var sentenceMatches = value.match(/[^.!?…。！？;:]+[.!?…。！？;:]+(?:["')\]]+)?/g);
      if (sentenceMatches && sentenceMatches.length) {
        var latest = String(sentenceMatches[sentenceMatches.length - 1] || '').trim();
        return latest || '';
      }
      var lines = raw.split('\n').map(function(line) {
        return String(line || '').replace(/\s+/g, ' ').trim();
      }).filter(function(line) { return !!line; });
      if (lines.length < 2) return '';
      var finalLine = String(lines[lines.length - 1] || '').trim();
      if (/[.!?…]$/.test(finalLine)) return finalLine;
      return String(lines[lines.length - 2] || '').trim();
    },
    thoughtSentenceFrames: function(inputText) {
      var value = String(inputText || '')
        .replace(/<[^>]*>/g, ' ')
        .replace(/\r/g, '')
        .trim();
      if (!value) return [];
      var matches = value.match(/[^.!?…。！？;:]+[.!?…。！？;:]+(?:["')\]]+)?/g) || [];
      return matches
        .map(function(part) { return String(part || '').replace(/\s+/g, ' ').trim(); })
        .filter(function(part) { return !!part; });
    },
    nextThoughtSentenceFrame: function(msg, thoughtText) {
      var frames = this.thoughtSentenceFrames(thoughtText);
      if (!frames.length) return '';
      if (!msg || typeof msg !== 'object') {
        return frames[frames.length - 1];
      }
      var nextIndex = Number(msg._thought_frame_index || 0);
      if (!Number.isFinite(nextIndex) || nextIndex < 0) nextIndex = 0;
      var seenCount = Number(msg._thought_frame_seen_count || 0);
      if (!Number.isFinite(seenCount) || seenCount < 0) seenCount = 0;
      // Advance the shown thought line only when an additional complete sentence
      // appears (punctuation-delimited), not on every text delta token.
      if (seenCount <= 0) {
        nextIndex = 0;
      } else if (frames.length > seenCount) {
        nextIndex = Math.min(nextIndex + (frames.length - seenCount), Math.max(0, frames.length - 1));
      } else {
        nextIndex = Math.max(0, Math.min(nextIndex, frames.length - 1));
      }
      msg._thought_frame_seen_count = frames.length;
      msg._thought_frame_index = nextIndex;
      msg._thought_frame_signature = frames.length + '|' + frames[frames.length - 1];
      var frame = String(frames[Math.max(0, Math.min(frames.length - 1, nextIndex))] || '').trim();
      if (frame) msg._thought_last_complete_sentence = frame;
      return frame;
    },
    renderLiveThoughtHtml: function(thoughtText, msg) {
      var text = this.nextThoughtSentenceFrame(msg, thoughtText) || this.latestCompleteSentence(thoughtText) || '';
      return '<span class="thinking-live-inline"><em>' + escapeHtml(text) + '</em></span>';
    },
    textLooksNoFindingsPlaceholder: function(text) {
      var lower = String(text || '').replace(/\s+/g, ' ').trim().toLowerCase();
      if (!lower) return false;
      return (
        lower.indexOf("don't have usable tool findings from this turn yet") >= 0 ||
        lower.indexOf("dont have usable tool findings from this turn yet") >= 0 ||
        lower.indexOf('no usable findings yet') >= 0 ||
        lower.indexOf("couldn't extract usable findings") >= 0 ||
        lower.indexOf('could not extract usable findings') >= 0 ||
        lower.indexOf("couldn't produce source-backed findings in this turn") >= 0 ||
        lower.indexOf('search returned no useful information') >= 0
      );
    },
    textLooksToolAckWithoutFindings: function(text) {
      var lower = String(text || '').replace(/\s+/g, ' ').trim().toLowerCase();
      if (!lower) return false;
      return (
        lower.indexOf('completed tool steps:') === 0 ||
        lower.indexOf('completed the tool call, but no synthesized response was available yet') >= 0 ||
        lower.indexOf('returned no usable findings yet') >= 0
      );
    },
    textMentionsContextGuard: function(text) {
      var lower = String(text || '').replace(/\s+/g, ' ').trim().toLowerCase();
      if (!lower) return false;
      return (
        lower.indexOf('context overflow: estimated context size exceeds safe threshold during tool loop') >= 0 ||
        lower.indexOf('more characters truncated') >= 0 ||
        lower.indexOf('middle content omitted') >= 0 ||
        lower.indexOf('safe context budget') >= 0
      );
    },
    isWebLikeToolName: function(toolName) {
      var lower = String(toolName || '').trim().toLowerCase();
      return (
        lower === 'web_search' ||
        lower === 'web_fetch' ||
        lower === 'batch_query' ||
        lower === 'search_web' ||
        lower === 'web_query' ||
        lower === 'browse'
      );
    },
    stripContextGuardMarkers: function(text) {
      var value = String(text || '');
      if (!value) return '';
      return value
        .replace(/\[\.\.\.\s+\d+\s+more characters truncated\]/gi, ' ')
        .replace(/context overflow:\s*estimated context size exceeds safe threshold during tool loop\.?/gi, ' ')
        .replace(/middle content omitted/gi, ' ')
        .replace(/\s+/g, ' ')
        .trim();
    },
    toolResultSummarySnippet: function(tool) {
      var text = this.stripContextGuardMarkers(String(this.toolProjectionPreviewText(tool) || ''));
      if (!text) return '';
      if (this.textLooksNoFindingsPlaceholder(text) || this.textLooksToolAckWithoutFindings(text)) return '';
      var sentence = this.latestCompleteSentence(text) || text;
      var out = String(sentence || '').replace(/\s+/g, ' ').trim();
      if (out.length > 160) out = out.slice(0, 157) + '...';
      return out;
    },
    lowSignalWebToolSummary: function(tool) {
      var toolName = String(tool && tool.name ? tool.name : 'web tool').replace(/_/g, ' ').trim();
      var aggregate = typeof this.formatToolAggregateMeta === 'function'
        ? String(this.formatToolAggregateMeta(tool || {}) || '').trim()
        : toolName;
      var suffix = aggregate && aggregate !== toolName ? ' (' + aggregate.replace(/^.*?:\s*/, '') + ')' : '';
      if (this.textMentionsContextGuard(this.toolProjectionPreviewText(tool) || '')) {
        return 'The ' + (toolName || 'web tool') + ' step' + suffix + ' returned more output than fit safely in context. Retry with a narrower query, one specific source URL, or ask me to continue from the partial result.';
      }
      return 'The ' + (toolName || 'web tool') + ' step' + suffix + ' ran, but only low-signal web output came back. Retry with a narrower query, one specific source URL, or ask me to continue from the recorded tool result.';
    },
    responseHasAuthoritativeToolCompletion: function(payload, tools) {
      var rows = Array.isArray(tools) ? tools : [];
      var finalization = payload && payload.response_finalization && typeof payload.response_finalization === 'object'
        ? payload.response_finalization
        : null;
      var completion = finalization && finalization.tool_completion && typeof finalization.tool_completion === 'object'
        ? finalization.tool_completion
        : null;
      var attempts = Array.isArray(completion && completion.tool_attempts) ? completion.tool_attempts : [];
      if (attempts.length) return true;
      if (finalization && finalization.findings_available === true) return true;
      return rows.some(function(tool) {
        if (!tool || tool.running) return false;
        if (tool.blocked || tool.is_error) return true;
        return !!String(this.toolProjectionPreviewText(tool) || tool.status || '').trim();
      }, this);
    },
    completedToolOnlySummary: function(tools) {
      var _ = tools;
      return '';
    },

    defaultAssistantFallback: function(thoughtText, tools) {
      var _ = [thoughtText, tools];
      return '';
    },

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
      if (this._pendingWsRequest && this._pendingWsRequest.agent_id) return 'Waiting for runtime response...';
      return 'Analyzing next step...';
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
      var out = [];
      var seen = Object.create(null);
      for (var i = 0; i < sourceRows.length && out.length < 48; i += 1) {
        var row = sourceRows[i] && typeof sourceRows[i] === 'object' ? sourceRows[i] : {};
        var text = String(row.text || row.display_text || row.summary || '').replace(/\r\n/g, '\n').trim();
        if (!text) continue;
        var lineKind = String(row.line_kind || row.kind || '').trim() || 'status';
        var state = String(row.state || row.status || '').trim() || 'done';
        var key = (lineKind + '|' + text).toLowerCase();
        if (seen[key]) continue;
        seen[key] = true;
        out.push({
          id: String(row.id || (lineKind + '-' + i)).slice(0, 180),
          text: text,
          line_kind: lineKind,
          state: state,
          shimmer: false
        });
      }
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

    messageCanReportIssueFromMeta: function(msg) {
      var service = typeof this.messageMetadataService === 'function' ? this.messageMetadataService() : null;
      if (service && typeof service.canReportIssue === 'function') {
        return service.canReportIssue(msg, this.currentAgent);
      }
      if (!this.currentAgent || !this.currentAgent.id) return false;
      if (typeof this.messageIsAgentOrigin === 'function' && !this.messageIsAgentOrigin(msg)) return false;
      if (!msg || msg.thinking || msg.terminal || msg.is_notice) return false;
      return !!((msg.text && String(msg.text).trim()) || (msg.tools && msg.tools.length) || msg.meta || msg.ts);
    },

    reportIssueFromMeta: async function(msg, idx) {
      if (!this.messageCanReportIssueFromMeta(msg)) return;
      try {
        var result = await InfringAPI.post('/api/shell-socket/issues', {
          agent_id: this.currentAgent.id,
          message_id: String(msg && msg.id || ''),
          message_index: idx
        });
        if (!result || result.ok === false) {
          throw new Error(String((result && (result.error || result.message)) || 'eval_report_failed'));
        }
        if (typeof InfringToast !== 'undefined') InfringToast.success('Eval review queued.');
      } catch (e) {
        if (typeof InfringToast !== 'undefined') InfringToast.error('Failed to queue eval review: ' + String(e && e.message ? e.message : 'unknown error'));
      }
    },

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

      }
      lastTop = top;
      lastHeight = height;
      lastClient = client;
    } else {
      stable = 0;
    }
    frame += 1;
    if (stable >= stableFramesNeeded || frame >= maxFrames) {
      cancelPinToLatestOnOpenJob(page);
      if (typeof page.scrollToBottomImmediate === 'function') page.scrollToBottomImmediate();
      return;
    }
    schedule();
  };
  schedule();
}

function resolveBottomFollowTolerancePx(page, overridePx) {
  var raw = Number(overridePx);
  if (!Number.isFinite(raw) || raw < 1) raw = Number(page && page.scrollBottomFollowTolerancePx);
  if (!Number.isFinite(raw) || raw < 1) raw = 32;
  if (raw > 160) raw = 160;
  return raw;
}

function extractChatMarkdownText(message) {
  var row = message && typeof message === 'object' ? message : {};
  var text = String(row.text || '').trim();
  if (!text && row.file_output && row.file_output.content) {
    text = String(row.file_output.content || '').trim();
  }
  if (!text && row.folder_output && row.folder_output.tree) {
    text = String(row.folder_output.tree || '').trim();
  }
  return text;
}

function buildChatMarkdown(messages, assistantName) {
  var rows = Array.isArray(messages) ? messages : [];
  if (!rows.length) return '';
  var assistantLabel = String(assistantName || 'Assistant').trim() || 'Assistant';
  var lines = ['# Chat with ' + assistantLabel, ''];
  for (var i = 0; i < rows.length; i++) {
    var row = rows[i] && typeof rows[i] === 'object' ? rows[i] : {};
    var role = String(row.role || '').toLowerCase();
    var label = role === 'user'
      ? 'You'
      : (role === 'agent'
        ? assistantLabel
        : (role === 'system' ? 'System' : 'Tool'));
    var content = extractChatMarkdownText(row);
    if (!content) continue;
    var ts = Number(row.ts || row.timestamp || 0);
    var tsLabel = Number.isFinite(ts) && ts > 0 ? (' (' + new Date(ts).toISOString() + ')') : '';
    lines.push('## ' + label + tsLabel, '', content, '');
  }
  return lines.join('\n').trim();
}

function exportChatMarkdown(messages, assistantName) {
  var markdown = buildChatMarkdown(messages, assistantName);
  if (!markdown) return false;
  var blob = new Blob([markdown + '\n'], { type: 'text/markdown' });
  var url = URL.createObjectURL(blob);
  var anchor = document.createElement('a');
  var label = String(assistantName || 'chat').trim().replace(/[^A-Za-z0-9._-]+/g, '-').replace(/^-+|-+$/g, '') || 'chat';
  anchor.href = url;
  anchor.download = 'chat-' + label + '-' + Date.now() + '.md';
  anchor.click();
  URL.revokeObjectURL(url);
  return true;
}

function resolveDistanceFromLatestMessageBottom(page, el) {
  var host = el || (page && typeof page.resolveMessagesScroller === 'function' ? page.resolveMessagesScroller() : null);
  if (!host) return Number.POSITIVE_INFINITY;
  var targetTop = resolveLatestMessageScrollTop(page, host);
  var top = Math.max(0, Number(host.scrollTop || 0));
  return Math.max(0, targetTop - top);
}

function syncLatestMessageBottomState(page, el, tolerancePx) {
  if (!page || typeof page !== 'object') return;
  var host = el || (typeof page.resolveMessagesScroller === 'function' ? page.resolveMessagesScroller() : null);
  if (!host) return;
  var hiddenBottom = resolveDistanceFromLatestMessageBottom(page, host);
  page._stickToBottom = hiddenBottom <= resolveBottomFollowTolerancePx(page, tolerancePx);
  page.showScrollDown = hiddenBottom > 120;
}

function isNearLatestMessageBottom(page, el, tolerancePx) {
  var host = el || (page && typeof page.resolveMessagesScroller === 'function' ? page.resolveMessagesScroller() : null);
  if (!host) return false;
  return resolveDistanceFromLatestMessageBottom(page, host) <= resolveBottomFollowTolerancePx(page, tolerancePx);
}

function clampScrollToLatestMessageBottom(page, el) {
  var host = el || (page && typeof page.resolveMessagesScroller === 'function' ? page.resolveMessagesScroller() : null);
  if (!host) return 0;
  var targetTop = resolveLatestMessageScrollTop(page, host);
  if ((page && page.showFreshArchetypeTiles) || !host.querySelector('.chat-message-block[data-msg-idx], .chat-message-block')) return targetTop;
  var top = Number(host.scrollTop || 0), clientHeight = Math.max(0, Number(host.clientHeight || 0));
  var maxTop = Math.max(0, Number(host.scrollHeight || 0) - clientHeight);
  var hardCapTop = Math.min(maxTop, targetTop);
  var slack = Number(page && page.scrollBottomClampSlackPx);
  if (!Number.isFinite(slack) || slack < 0) slack = 16;
  if (top > (hardCapTop + slack)) {
    var wheelAt = Number(page && page._lastMessagesWheelAt || 0), recentWheel = wheelAt > 0 && ((Date.now() - wheelAt) < 120);
    if (!recentWheel) setTimeout(function() { host.scrollTop = Math.min(Number(host.scrollTop || 0), resolveLatestMessageScrollTop(page, host)); }, 24);
  }
  return hardCapTop;
}
function scheduleBottomHardCapClamp(page, el, targetTop, delayMs) {
  if (!page || typeof page !== 'object') return;
  if (page._bottomClampTimer) clearTimeout(page._bottomClampTimer);
  var host = el || (typeof page.resolveMessagesScroller === 'function' ? page.resolveMessagesScroller() : null);
  if (!host) return;
  var hardCapTop = Number(targetTop), delay = Number(delayMs);
  if (!Number.isFinite(hardCapTop)) hardCapTop = resolveLatestMessageScrollTop(page, host);
  if (!Number.isFinite(delay) || delay < 24) delay = 120;
  page._bottomClampTimer = setTimeout(function() {
    page._bottomClampTimer = 0;
    var now = Date.now(), recentAt = Math.max(Number(page._lastMessagesWheelAt || 0), Number(page._lastMessagesScrollAt || 0));
    if (recentAt > 0 && (now - recentAt) < 96) return scheduleBottomHardCapClamp(page, host, hardCapTop, 72);
    clampScrollToLatestMessageBottom(page, host);
    if (typeof page.syncGridBackgroundOffset === 'function') page.syncGridBackgroundOffset(host);
    syncLatestMessageBottomState(page, host);
  }, delay);
}
function resolveLatestMessageScrollTop(page, el) {
  var host = el || (page && typeof page.resolveMessagesScroller === 'function' ? page.resolveMessagesScroller() : null);
  if (!host) return 0;
  var clientHeight = Math.max(0, Number(host.clientHeight || 0));
  var maxTop = Math.max(0, Number(host.scrollHeight || 0) - clientHeight);
  void page;
  return maxTop;
}
