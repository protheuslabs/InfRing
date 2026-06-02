const COMPONENT_TAG = 'infring-chat-input-footer-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-chat-input-footer-shell', shadow: 'none' }} />
<script>
  import { onMount, onDestroy, tick } from 'svelte';

  const ACCEPT = 'image/*,.txt,.pdf,.md,.json,.csv,.mp3,.wav,.ogg,.webm,.m4a,.flac';
  let inputText = '';
  let focused = false;
  let fileInput;
  let textarea;
  let shellHost;
  let timer = 0;
  let focusTimer = 0;
  let focusListener = null;
  let state = {
    currentAgent: null,
    archived: false,
    terminalMode: false,
    terminalCursorFocused: false,
    terminalCursorStyle: '',
    terminalShortcutHint: 'Ctrl+\\\\',
    sending: false,
    recording: false,
    locked: false,
    systemThread: false,
    showScrollDown: false,
    showFreshArchetypeTiles: false,
    freshInitAwaitingOtherPrompt: false,
    attachments: [],
    attachMode: 'send',
    menuOpen: false,
    showGitTreeMenu: false,
    showModelSwitcher: false,
    showRuntimeSwitcher: false,
    branchLabel: '',
    menuModelLabel: '',
    runtimeEngineLabel: 'InfRing Native',
    modelDisplayName: '',
    contextLabel: '',
    contextTooltip: '',
    contextStyle: '',
    promptSuggestionsEnabled: false,
    promptQueueItems: [],
    promptSuggestions: [],
    slashOpen: false,
    slashRows: [],
    slashIdx: 0,
    modelPickerOpen: false,
    modelPickerRows: [],
    modelPickerIdx: 0,
    gitTreeRows: [],
    gitTreeLoading: false,
    gitTreeError: '',
    gitTreeSwitching: false,
    modelRows: [],
    modelSwitching: false,
    runtimeEngineRows: [],
    runtimeEngineLoading: false,
    runtimeEngineError: '',
    modelSwitcherFilter: '',
    modelSwitcherProviderFilter: '',
    switcherProviders: [],
    currentTip: '',
    tokenCount: 0
  };

  function cp() { return (typeof window !== 'undefined' && window.InfringChatPage) || null; }
  function call(name, ...args) {
    const p = cp();
    if (!p || typeof p[name] !== 'function') return undefined;
    try { return p[name](...args); } catch (_e) { return undefined; }
  }
  function pageValue(name, fallback) {
    const p = cp();
    return p && name in p ? p[name] : fallback;
  }
  function bool(name) { return !!pageValue(name, false); }
  function list(name) {
    const value = pageValue(name, []);
    return Array.isArray(value) ? value : [];
  }
  function refresh() {
    const p = cp();
    if (!p) return;
    const nextText = typeof p.inputText === 'string' ? p.inputText : '';
    if (!focused || nextText === '' || nextText !== inputText) inputText = nextText;
    const terminalMode = !!p.terminalMode;
    const archived = !!(p.currentAgent && typeof p.isCurrentAgentArchived === 'function' && p.isCurrentAgentArchived());
    let modelRows = list('renderedSwitcherModels');
    if (!modelRows.length && p.showModelSwitcher && typeof p.fallbackModelCatalogRows === 'function') {
      modelRows = p.fallbackModelCatalogRows();
      p._modelCache = modelRows;
      p._modelCacheTime = Date.now();
      p.modelPickerList = modelRows;
    }
    state = {
      currentAgent: p.currentAgent || null,
      archived,
      terminalMode,
      terminalCursorFocused: !!p.terminalCursorFocused,
      terminalCursorStyle: String(p.terminalCursorStyle || ''),
      terminalShortcutHint: String(p.terminalShortcutHint || 'Ctrl+\\\\'),
      sending: !!p.sending,
      recording: !!p.recording,
      locked: typeof p.isFreshInitComposerLocked === 'function' ? !!p.isFreshInitComposerLocked() : false,
      systemThread: typeof p.isSystemThreadActive === 'function' ? !!p.isSystemThreadActive() : false,
      showScrollDown: !!p.showScrollDown,
      showFreshArchetypeTiles: !!p.showFreshArchetypeTiles,
      freshInitAwaitingOtherPrompt: !!p.freshInitAwaitingOtherPrompt,
      attachments: list('attachments'),
      attachMode: typeof p.currentInputToggleMode === 'function' ? String(p.currentInputToggleMode() || 'send') : (!!p.recording ? 'voice' : 'send'),
      menuOpen: !!p.showAttachMenu,
      showGitTreeMenu: !!p.showGitTreeMenu,
      showModelSwitcher: !!p.showModelSwitcher,
      showRuntimeSwitcher: !!p.showRuntimeSwitcher,
      branchLabel: String(p.activeGitBranchMenuLabel || ''),
      menuModelLabel: String(p.menuModelLabel || ''),
      runtimeEngineLabel: typeof p.runtimeEngineMenuLabel === 'function' ? String(p.runtimeEngineMenuLabel() || 'InfRing Native') : 'InfRing Native',
      modelDisplayName: String(p.modelDisplayName || ''),
      contextLabel: String(p.contextRingCompactLabel || ''),
      contextTooltip: String(p.contextRingTooltip || ''),
      contextStyle: String(p.contextRingProgressStyle || ''),
      promptSuggestionsEnabled: !!p.promptSuggestionsEnabled,
      promptQueueItems: list('promptQueueItems'),
      promptSuggestions: list('promptSuggestions'),
      slashOpen: !terminalMode && !!p.showSlashMenu,
      slashRows: list('filteredSlashCommands'),
      slashIdx: Number(p.slashIdx || 0),
      modelPickerOpen: !terminalMode && !!p.showModelPicker,
      modelPickerRows: list('filteredModelPicker'),
      modelPickerIdx: Number(p.modelPickerIdx || 0),
      gitTreeRows: list('gitTreeMenuItems'),
      gitTreeLoading: !!p.gitTreeMenuLoading,
      gitTreeError: String(p.gitTreeMenuError || ''),
      gitTreeSwitching: !!p.gitTreeSwitching,
      modelRows,
      modelSwitching: !!p.modelSwitching,
      runtimeEngineRows: list('runtimeEngineRows'),
      runtimeEngineLoading: !!p.runtimeEngineLoading,
      runtimeEngineError: String(p.runtimeEngineError || ''),
      modelSwitcherFilter: String(p.modelSwitcherFilter || ''),
      modelSwitcherProviderFilter: String(p.modelSwitcherProviderFilter || ''),
      switcherProviders: list('switcherProviders'),
      currentTip: String(p.currentTip || ''),
      tokenCount: Number(p.tokenCount || 0)
    };
  }
  function syncInput(value) {
    const p = cp();
    inputText = String(value == null ? '' : value);
    if (p) p.inputText = inputText;
    if (state.terminalMode) call('updateTerminalCursor', { target: textarea });
    call('refreshChatInputOverlayMetrics');
    refresh();
  }
  function resizeInput() {
    if (!textarea) return;
    textarea.style.height = 'auto';
    textarea.style.height = Math.min(textarea.scrollHeight, 150) + 'px';
  }
  async function afterAction() {
    await tick();
    resizeInput();
    refresh();
  }
  function setMenu(open) {
    const p = cp();
    if (!p) return;
    if (open && typeof p.closeComposerMenus === 'function') p.closeComposerMenus({ attach: true });
    if (!open && typeof p.closeComposerMenus === 'function') p.closeComposerMenus();
    p.showAttachMenu = !!open;
    if (!open) {
      p.showModelSwitcher = false;
      p.showRuntimeSwitcher = false;
      if (typeof p.closeGitTreeMenu === 'function') p.closeGitTreeMenu();
      else p.showGitTreeMenu = false;
    }
    refresh();
  }
  function toggleMenu(event) {
    if (event) event.stopPropagation();
    setMenu(!state.menuOpen);
  }
  function outsideClick(event) {
    if (!shellHost || shellHost.contains(event.target)) return;
    setMenu(false);
  }
  function beginAttachPicker() {
    const p = cp();
    if (!p || state.systemThread || !fileInput) return;
    if (p.terminalMode && typeof p.toggleTerminalMode === 'function') p.toggleTerminalMode();
    p.attachPickerRestoreMode = p.recording ? 'voice' : 'send';
    p.attachPickerSessionActive = true;
    p.showAttachMenu = false;
    if (focusListener) window.removeEventListener('focus', focusListener);
    focusListener = function() {
      if (focusTimer) clearTimeout(focusTimer);
      focusTimer = setTimeout(function() {
        focusTimer = 0;
        const page = cp();
        if (page) {
          page.attachPickerSessionActive = false;
          if (typeof page.endAttachPickerSession === 'function') page.endAttachPickerSession();
        }
        refresh();
      }, 180);
    };
    window.addEventListener('focus', focusListener, { once: true });
    try { fileInput.click(); } catch (_e) { if (typeof p.endAttachPickerSession === 'function') p.endAttachPickerSession(); }
    refresh();
  }
  function filesChanged(event) {
    const p = cp();
    const input = event && event.target;
    if (p && input && input.files && input.files.length && typeof p.addFiles === 'function') p.addFiles(input.files);
    if (input) input.value = '';
    if (p && typeof p.endAttachPickerSession === 'function') p.endAttachPickerSession();
    else if (p) p.attachPickerSessionActive = false;
    refresh();
  }
  function removeAttachment(index) { call('removeAttachment', index); refresh(); }
  function runSend() { call('sendMessage'); afterAction(); }
  function runStop() { call('stopAgent'); refresh(); }
  function toggleTerminal() { if (!state.systemThread) call('toggleTerminalMode'); afterAction(); }
  function toggleSuggestions() { call('togglePromptSuggestionsEnabled'); refresh(); }
  function toggleVoice() { state.recording ? call('stopRecording') : call('startRecording'); refresh(); }
  function toggleGit() { call('toggleGitTreeMenu'); refresh(); }
  function toggleModel() { call('toggleModelSwitcher'); refresh(); }
  function toggleRuntime() { call('toggleRuntimeSwitcher'); refresh(); }
  function selectGit(branch) { call('switchAgentGitTree', branch); refresh(); }
  function createGitBranch() { call('createAndCheckoutGitBranch'); refresh(); }
  function switchModel(row) { call('switchModel', row); refresh(); }
  function selectRuntime(row) { call('selectRuntimeEngine', row); refresh(); }
  function clean(value) { return String(value == null ? '' : value).trim(); }
  function firstNonEmpty() {
    for (let i = 0; i < arguments.length; i += 1) {
      const value = clean(arguments[i]);
      if (value) return value;
    }
    return '';
  }
  function safeClass(value) {
    return clean(value).toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '') || 'balanced';
  }
  function modelName(row) { return String(call('modelSwitcherItemName', row) || row.display_name || row.id || 'model'); }
  function modelLogo(row) { return clean(call('modelLogoUrl', row)); }
  function modelLogoTitle(row) { return firstNonEmpty(call('modelLogoTooltip', row), 'Model family'); }
  function modelSourceLogo(row) { return clean(call('modelSourceLogoUrl', row)); }
  function modelSourceLogoTitle(row) { return firstNonEmpty(call('modelSourceLogoTooltip', row), 'Model source'); }
  function modelProvider(row) { return firstNonEmpty(row && row.provider, row && row.model_provider, 'provider'); }
  function modelDeployment(row) { return firstNonEmpty(call('modelDeploymentLabel', row), row && row.deployment_kind, row && row.deployment); }
  function modelContext(row) { return clean(call('modelContextWindowLabel', row)); }
  function modelParams(row) { return clean(call('modelParamLabel', row)); }
  function modelSpecialty(row) { return clean(call('modelSpecialtyLabel', row)); }
  function modelTier(row) {
    const raw = clean(row && (row.tier || row.quality_tier || row.speed_tier || row.specialty)).toLowerCase();
    if (raw.indexOf('frontier') >= 0 || raw.indexOf('reason') >= 0) return 'frontier';
    if (raw.indexOf('smart') >= 0 || raw.indexOf('coding') >= 0 || raw.indexOf('vision') >= 0) return 'smart';
    if (raw.indexOf('fast') >= 0 || raw.indexOf('speed') >= 0) return 'fast';
    if (raw.indexOf('local') >= 0) return 'local';
    return safeClass(raw || 'balanced');
  }
  function modelPower(row) { return clean(call('modelPowerIcons', row)); }
  function modelCost(row) { return clean(call('modelCostIcons', row)); }
  function modelMetaParts(row) {
    return [modelProvider(row), modelDeployment(row), modelContext(row), modelParams(row)].filter(function(value) { return !!clean(value); });
  }
  function modelMeta(row) { return modelMetaParts(row).join(' · '); }
  function runtimeName(row) { return String(row && (row.display_name || row.engine_id) || 'runtime'); }
  function runtimeMeta(row) { return String(call('runtimeEngineMeta', row) || row.status || ''); }
  function runtimeActive(row) { return !!call('isRuntimeEngineActive', row); }
  function runtimeActionIcon(row) { return String(call('runtimeEngineActionIcon', row) || ''); }
  function applySuggestion(value) { call('applyPromptSuggestion', value); afterAction(); }
  function queuePreview(row) { return String(call('queuePromptPreview', row) || row.text || 'Queued prompt'); }
  function setQueueText(row) { syncInput(row && row.text); afterAction(); }
  function keyForAttachment(att, index) {
    const file = att && att.file;
    return String(file && file.name || 'attachment') + '-' + String(file && file.size || index);
  }
  function handleKeydown(event) {
    const p = cp();
    if (!p) return;
    if (event.key === 'Enter' && !event.shiftKey && !event.isComposing && event.keyCode !== 229) {
      event.preventDefault();
      if (!state.terminalMode && p.showModelPicker && state.modelPickerRows.length) call('pickModel', state.modelPickerRows[state.modelPickerIdx] && state.modelPickerRows[state.modelPickerIdx].id);
      else if (!state.terminalMode && p.showSlashMenu && state.slashRows.length) call('executeSlashCommand', state.slashRows[state.slashIdx] && state.slashRows[state.slashIdx].cmd);
      else runSend();
      return;
    }
    if (event.key === 'Escape') {
      p.showSlashMenu = false;
      p.showModelPicker = false;
      refresh();
      return;
    }
    if (event.key === 'ArrowUp') {
      event.preventDefault();
      if (!state.terminalMode && p.showModelPicker) p.modelPickerIdx = Math.max(0, Number(p.modelPickerIdx || 0) - 1);
      else if (!state.terminalMode && p.showSlashMenu) p.slashIdx = Math.max(0, Number(p.slashIdx || 0) - 1);
      else call('navigateInputHistory', -1, event);
      refresh();
    }
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      if (!state.terminalMode && p.showModelPicker) p.modelPickerIdx = Math.min(state.modelPickerRows.length - 1, Number(p.modelPickerIdx || 0) + 1);
      else if (!state.terminalMode && p.showSlashMenu) p.slashIdx = Math.min(state.slashRows.length - 1, Number(p.slashIdx || 0) + 1);
      else call('navigateInputHistory', 1, event);
      refresh();
    }
  }
  function sendDisabled() {
    if (state.showFreshArchetypeTiles) return !state.freshInitAwaitingOtherPrompt || !inputText.trim();
    return !inputText.trim() && !state.attachments.length;
  }
  function footerText() {
    if (state.terminalMode) return 'terminal mode (' + state.terminalShortcutHint + ')';
    if (state.tokenCount > 0) return '~' + state.tokenCount + ' tokens';
    if (state.attachments.length) return state.attachments.length + ' file(s)';
    return '';
  }
  function placeholder() {
    return String(call('composerPlaceholder', true) || (state.terminalMode ? '/workspace' : 'Message...'));
  }
  onMount(function() {
    refresh();
    timer = setInterval(refresh, 120);
    document.addEventListener('click', outsideClick, true);
  });
  onDestroy(function() {
    if (timer) clearInterval(timer);
    if (focusTimer) clearTimeout(focusTimer);
    if (focusListener) window.removeEventListener('focus', focusListener);
    document.removeEventListener('click', outsideClick, true);
  });
</script>

{#if state.currentAgent && !state.archived}
<div class="input-area" style="position:relative" class:terminal-mode={state.terminalMode}>
  <infring-composer-lane-shell>
  <div class="chat-input-lane" bind:this={shellHost}>
    {#if state.showScrollDown}
      <button class="chat-scroll-down" on:click={() => call('scrollToBottom', { buttonAnimated: true, force: true })} title="Scroll to latest" aria-label="Scroll to latest"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"></path></svg></button>
    {/if}
    {#if !state.terminalMode && state.attachments.length}
      <div style="display:flex;gap:8px;flex-wrap:wrap;padding:0 0 8px 0">
        {#each state.attachments as att, aidx (keyForAttachment(att, aidx))}
          <div style="position:relative;border:1px solid var(--border);border-radius:6px;padding:4px;display:flex;align-items:center;gap:6px;background:var(--surface2);max-width:180px">
            {#if att && att.preview}<img src={att.preview} style="width:32px;height:32px;object-fit:cover;border-radius:4px" alt="attachment preview">{:else}<span style="font-size:18px;width:32px;text-align:center">📄</span>{/if}
            <span class="text-xs truncate" style="max-width:100px">{att && att.file && att.file.name ? att.file.name : 'attachment'}</span>
            {#if att && att.uploading}<span class="spinner" style="width:12px;height:12px;border-width:2px"></span>{/if}
            <button on:click={() => removeAttachment(aidx)} style="position:absolute;top:-6px;right:-6px;width:18px;height:18px;border-radius:50%;background:var(--danger);color:#fff;border:none;cursor:pointer;font-size:11px;display:flex;align-items:center;justify-content:center;line-height:1" aria-label="Remove attachment">&times;</button>
          </div>
        {/each}
      </div>
    {/if}
    {#if state.slashOpen && state.slashRows.length}
      <infring-slash-command-menu-shell><div class="slash-menu">{#each state.slashRows as cmd, idx (cmd.cmd)}<div class:slash-active={idx === state.slashIdx} class="slash-menu-item" on:click={() => call('executeSlashCommand', cmd.cmd)} on:mouseenter={() => { const p = cp(); if (p) p.slashIdx = idx; refresh(); }}><span class="font-bold" style="font-size:13px">{cmd.cmd}</span><span class="text-xs text-dim">{cmd.desc}</span></div>{/each}</div></infring-slash-command-menu-shell>
    {/if}
    {#if state.modelPickerOpen && state.modelPickerRows.length}
      <infring-model-picker-menu-shell><div class="slash-menu" style="max-height:280px;overflow-y:auto"><div class="text-xs text-dim" style="padding:4px 10px;border-bottom:1px solid var(--border)">Available models - pick one or keep typing</div>{#each state.modelPickerRows as m, idx (m.id)}<div class:slash-active={idx === state.modelPickerIdx} class="slash-menu-item" on:click={() => call('pickModel', m.id)} on:mouseenter={() => { const p = cp(); if (p) p.modelPickerIdx = idx; refresh(); }}><span class="font-bold" style="font-size:12px;font-family:var(--font-mono)">{m.id}</span><span class="text-xs text-dim">{modelMeta(m)}</span></div>{/each}</div></infring-model-picker-menu-shell>
    {/if}
    <div class="composer-stack">
      {#if !state.terminalMode && state.promptQueueItems.length}
        <infring-prompt-queue-shell><div class="prompt-queue-row"><div class="prompt-queue-list">{#each state.promptQueueItems as item (item.queue_id)}<div class="prompt-queue-item" draggable="true" on:dragstart={(e) => call('onPromptQueueDragStart', item.queue_id, e)} on:dragover|preventDefault on:drop={(e) => call('onPromptQueueDrop', item.queue_id, e)} on:dragend={() => call('onPromptQueueDragEnd')}><span class="prompt-queue-drag" title="Drag to reorder">⋮⋮</span><button class="prompt-queue-text" type="button" on:click={() => setQueueText(item)} title={item.text}>{queuePreview(item)}</button><button class="prompt-queue-steer" type="button" on:click={() => call('steerPromptQueueItem', item.queue_id)}>Steer</button><button class="prompt-queue-remove" type="button" on:click={() => call('removePromptQueueItem', item.queue_id)} aria-label="Remove queued prompt">&times;</button></div>{/each}</div></div></infring-prompt-queue-shell>
      {:else if !state.terminalMode && state.promptSuggestionsEnabled && state.promptSuggestions.length}
        <infring-prompt-suggestions-shell><div class="prompt-suggestions-row">{#each state.promptSuggestions as suggestion, sidx (suggestion + '-' + sidx)}<button class="prompt-suggestion-chip prompt-suggestion-chip-rise" type="button" on:click={() => applySuggestion(suggestion)} on:mouseenter={(e) => call('onPromptSuggestionHoverIn', e)} on:mouseleave={(e) => call('onPromptSuggestionHoverOut', e)} title={suggestion} style={'--prompt-suggestion-entry-delay:' + (sidx * 16) + 'ms'}><span class="prompt-suggestion-chip-text">{suggestion}</span></button>{/each}</div></infring-prompt-suggestions-shell>
      {/if}
      <div class="input-row">
        <input bind:this={fileInput} type="file" multiple accept={ACCEPT} on:change={filesChanged} style="display:none">
        <div class:composer-shell-disabled={state.locked} class:system-thread-active={state.systemThread} class="composer-shell">
          <div class="composer-main-row">
            <div class="composer-display-pill" aria-label="Message input controls">
              {#if !state.systemThread}
              <div class="composer-menu-pill composer-shared-input-pill">
                <div class="composer-plus-wrap composer-icon-left">
                  <button id="composer-plus-menu-anchor" class="composer-icon-btn composer-hamburger-btn" on:click={toggleMenu} title="Add files and more (Ctrl+F)" aria-label="Add files and more" aria-expanded={state.menuOpen ? 'true' : 'false'}><svg class="composer-hamburger-icon" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><line x1="4" y1="7" x2="20" y2="7"/><line x1="4" y1="12" x2="20" y2="12"/><line x1="4" y1="17" x2="20" y2="17"/></svg></button>
                  {#if state.menuOpen || state.showModelSwitcher || state.showGitTreeMenu || state.showRuntimeSwitcher}
                  <infring-taskbar-menu-shell class="composer-plus-menu dashboard-dropdown-surface" shellprimitive="taskbar-dock" wrapperrole="taskbar-menu" parentownedmechanics="true" anchorid="composer-plus-menu-anchor" fallbackside="top" layoutkey="composer-plus-menu">
                    {#if state.menuOpen && !state.terminalMode}<div class="composer-plus-menu-item composer-plus-menu-context-row"><span class="context-ring-inline-label">{state.contextLabel}</span><div class="context-ring context-ring-toggle dashboard-preview-trigger dashboard-preview-wrap" data-tooltip={state.contextTooltip} tabindex="0"><svg viewBox="0 0 36 36" aria-hidden="true"><circle class="context-ring-track" cx="18" cy="18" r="14" pathLength="100"></circle><circle class="context-ring-progress" cx="18" cy="18" r="14" pathLength="100" style={state.contextStyle}></circle></svg></div></div>{/if}
                    {#if state.menuOpen && !state.terminalMode}<button class="composer-plus-menu-item composer-plus-menu-item-toggle composer-plus-menu-item-suggestions composer-plus-menu-entry" on:click={toggleSuggestions} title="Toggle chat suggestions"><span class="composer-plus-toggle-label"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18h6"/><path d="M10 22h4"/><path d="M12 2a7 7 0 0 0-4 12.75c.63.45 1 1.16 1 1.94V18h6v-1.31c0-.78.37-1.49 1-1.94A7 7 0 0 0 12 2z"/></svg><span>Chat suggestions</span></span><span class:active={state.promptSuggestionsEnabled} class="composer-plus-vtoggle" aria-hidden="true"><span class="composer-plus-vtoggle-knob"></span></span></button>{/if}
                    {#if state.menuOpen}<button class="composer-plus-menu-item composer-plus-menu-item-toggle composer-plus-menu-item-terminal composer-plus-menu-entry" on:click={toggleTerminal} disabled={state.systemThread} title={state.systemThread ? 'System thread is terminal-only' : 'Switch compose mode'}><span class="composer-plus-toggle-label">{#if state.terminalMode}<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg>{:else}<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 17 10 11 4 5"/><path d="M12 19h8"/></svg>{/if}<span>{state.systemThread ? 'Terminal locked' : (state.terminalMode ? 'Chat mode' : 'Terminal mode')}</span></span><span class="composer-plus-hotkey" aria-hidden="true">Ctrl+T / Ctrl+\</span><span class:active={state.terminalMode} class="composer-plus-vtoggle" aria-hidden="true"><span class="composer-plus-vtoggle-knob"></span></span></button>{/if}
                    {#if !state.terminalMode}
                      <div class="input-box-column input-box-column-selectors composer-plus-inline-controls">
                        {#if state.branchLabel}
                          <div class="input-box-selector-row">
                            <button id="composer-git-tree-menu-anchor" type="button" class="input-box-selector-activator composer-plus-menu-entry" title={'Active branch: ' + state.branchLabel} aria-expanded={state.showGitTreeMenu ? 'true' : 'false'} on:click={toggleGit}><span class:active={state.showGitTreeMenu} class="composer-icon-btn composer-git-btn input-box-selector-trigger" aria-hidden="true"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round"><circle cx="6" cy="6" r="3"></circle><circle cx="18" cy="18" r="3"></circle><path d="M6 9v6a6 6 0 0 0 6 6h3"></path><path d="M18 15V9"></path></svg></span><span class="model-inline-label input-box-selector-label">Change git tree</span><span class="composer-plus-state-pill">{state.branchLabel}</span></button>
                            {#if state.showGitTreeMenu}
                              <infring-taskbar-menu-shell class="chat-branch-menu dashboard-dropdown-surface" shellprimitive="taskbar-dock" wrapperrole="taskbar-menu" parentownedmechanics="true" anchorid="composer-git-tree-menu-anchor" fallbackside="top" layoutkey="composer-git-tree-menu">
                                <div class="chat-branch-menu-head">Switch Git Tree</div>
                                {#if state.gitTreeLoading}<div class="chat-branch-menu-status">Loading trees...</div>{/if}
                                {#if !state.gitTreeLoading && state.gitTreeError}<div class="chat-branch-menu-status chat-branch-menu-error">{state.gitTreeError}</div>{/if}
                                {#if !state.gitTreeLoading && !state.gitTreeError}
                                  <div class="chat-branch-menu-list">
                                    {#each state.gitTreeRows as row (row.branch)}
                                      <button type="button" class:active={row.current} class="chat-branch-menu-item" disabled={state.gitTreeSwitching || row.current} on:click={() => selectGit(row.branch)}><span class="chat-branch-menu-item-name">{row.branch}</span><span class="chat-branch-menu-item-meta">{row.main ? 'main' : (row.in_use_by_agents > 0 ? row.in_use_by_agents + ' agents' : 'branch')}</span></button>
                                    {/each}
                                  </div>
                                {/if}
                                <button type="button" class="chat-branch-menu-create" disabled={state.gitTreeSwitching} on:click={createGitBranch}>Create and checkout new branch</button>
                              </infring-taskbar-menu-shell>
                            {/if}
                          </div>
                        {/if}
                        <div class="input-box-selector-row">
                          <button id="composer-model-menu-anchor" type="button" class="input-box-selector-activator composer-plus-menu-entry" aria-expanded={state.showModelSwitcher ? 'true' : 'false'} on:click={toggleModel} title={'Active model: ' + state.modelDisplayName}><span class:active={state.showModelSwitcher} class="composer-icon-btn composer-model-btn input-box-selector-trigger" aria-hidden="true"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/></svg></span><span class="model-inline-label input-box-selector-label">Active LLM</span><span class="composer-plus-state-pill composer-plus-state-pill-model">{state.menuModelLabel}</span></button>
                          {#if state.showModelSwitcher}
                            <infring-taskbar-menu-shell class="model-switcher-dropdown model-switcher-dropdown-inline dashboard-dropdown-surface" shellprimitive="taskbar-dock" wrapperrole="taskbar-menu" parentownedmechanics="true" anchorid="composer-model-menu-anchor" fallbackside="top" layoutkey="composer-model-switcher">
                              <div class="model-switcher-search"><input id="model-switcher-search" type="text" value={state.modelSwitcherFilter} placeholder="Search models..." on:input={(e) => { const p = cp(); if (p) p.modelSwitcherFilter = e.target.value; refresh(); }}><select class="model-switcher-provider-select" value={state.modelSwitcherProviderFilter} on:change={(e) => { const p = cp(); if (p) p.modelSwitcherProviderFilter = e.target.value; refresh(); }}><option value="">All</option>{#each state.switcherProviders as pn (pn)}<option value={pn}>{pn}</option>{/each}</select></div>
                              {#if state.modelSwitching}
                                <div style="display:flex;align-items:center;justify-content:center;padding:12px;gap:8px"><div class="tool-card-spinner"></div><span class="text-xs text-dim">Switching...</span></div>
                              {:else}
                                <div class="model-switcher-list">
                                  {#if !state.modelRows.length}<div class="chat-branch-menu-status">No models match this filter.</div>{/if}
                                  {#each state.modelRows as m (m.id)}
                                    <button type="button" class:active={call('isSwitcherModelActive', m)} class="model-switcher-item" on:click={() => switchModel(m)}>{#if modelLogo(m)}<span class="model-switcher-logo-slot" title={modelLogoTitle(m)}><img class="model-switcher-logo-image" src={modelLogo(m)} alt="" loading="lazy" on:load={(e) => call('onModelLogoLoad', e)} on:error={(e) => call('onModelLogoError', 'model', m, e)}></span>{:else if modelSourceLogo(m)}<span class="model-switcher-logo-slot" title={modelSourceLogoTitle(m)}><img class="model-switcher-logo-image model-switcher-logo-source" src={modelSourceLogo(m)} alt="" loading="lazy" on:load={(e) => call('onModelLogoLoad', e)} on:error={(e) => call('onModelLogoError', 'source', m, e)}></span>{/if}<span class="model-switcher-main"><span class="model-switcher-title-row"><span class="model-switcher-item-name">{modelName(m)}</span>{#if modelSpecialty(m)}<span class={'model-switcher-tier tier-' + modelTier(m)}>{modelSpecialty(m)}</span>{/if}</span><span class="model-switcher-item-meta">{#each modelMetaParts(m) as part, midx (part + '-' + midx)}{#if midx > 0}<span class="model-meta-sep">·</span>{/if}<span class="model-meta-stat">{part}</span>{/each}</span></span><span class="model-switcher-item-tools">{#if modelPower(m)}<span class="model-meta-stat model-meta-power" title="Power">{modelPower(m)}</span>{/if}{#if modelCost(m)}<span class="model-meta-stat model-meta-cost" title="Cost">{modelCost(m)}</span>{/if}</span></button>
                                  {/each}
                                </div>
                              {/if}
                            </infring-taskbar-menu-shell>
                          {/if}
                        </div>
                        <div class="input-box-selector-row">
                          <button id="composer-runtime-menu-anchor" type="button" class="input-box-selector-activator composer-plus-menu-entry" aria-expanded={state.showRuntimeSwitcher ? 'true' : 'false'} on:click={toggleRuntime} title={'Agent runtime: ' + state.runtimeEngineLabel}><span class:active={state.showRuntimeSwitcher} class="composer-icon-btn composer-model-btn input-box-selector-trigger" aria-hidden="true"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round"><path d="M4 7h16"/><path d="M7 7v10a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2V7"/><path d="M9 11h6"/><path d="M9 15h6"/></svg></span><span class="model-inline-label input-box-selector-label">Agent runtime</span><span class="composer-plus-state-pill composer-plus-state-pill-model">{state.runtimeEngineLabel}</span></button>
                          {#if state.showRuntimeSwitcher}
                            <infring-taskbar-menu-shell class="model-switcher-dropdown model-switcher-dropdown-inline dashboard-dropdown-surface" shellprimitive="taskbar-dock" wrapperrole="taskbar-menu" parentownedmechanics="true" anchorid="composer-runtime-menu-anchor" fallbackside="top" layoutkey="composer-runtime-switcher">
                              <div class="chat-branch-menu-head">Agent Runtime</div>
                              {#if state.runtimeEngineLoading}
                                <div style="display:flex;align-items:center;justify-content:center;padding:12px;gap:8px"><div class="tool-card-spinner"></div><span class="text-xs text-dim">Sensing runtimes...</span></div>
                              {/if}
                              {#if state.runtimeEngineError}
                                <div class="chat-branch-menu-status chat-branch-menu-error">{state.runtimeEngineError}</div>
                              {/if}
                              <div class="model-switcher-list">
                                {#if !state.runtimeEngineRows.length}<div class="chat-branch-menu-status">No runtime sockets are registered.</div>{/if}
                                {#each state.runtimeEngineRows as r (r.engine_id)}
                                  <button type="button" class:active={runtimeActive(r)} class="model-switcher-item" on:click={() => selectRuntime(r)} title={runtimeMeta(r)}><span class="model-switcher-main"><span class="model-switcher-title-row"><span class="model-switcher-item-name">{runtimeName(r)}</span></span><span class="model-switcher-item-meta">{runtimeMeta(r)}</span></span>{#if runtimeActionIcon(r)}<span class="model-switcher-item-tools"><span class="model-meta-stat">{runtimeActionIcon(r)}</span></span>{/if}</button>
                                {/each}
                              </div>
                            </infring-taskbar-menu-shell>
                          {/if}
                        </div>
                      </div>
                    {/if}
                  </infring-taskbar-menu-shell>
                  {/if}
                </div>
              </div>
              {/if}
              <div class="composer-input-pill composer-shared-input-pill">{#if state.terminalMode && state.terminalCursorFocused}<span class="terminal-block-cursor" style={state.terminalCursorStyle} aria-hidden="true">█</span>{/if}<textarea bind:this={textarea} id="msg-input" rows="1" value={inputText} disabled={state.locked} placeholder={placeholder()} class:streaming-active={state.sending} class:terminal-textarea={state.terminalMode} class:composer-input-disabled={state.locked} on:focus={(e) => { focused = true; if (state.terminalMode) call('setTerminalCursorFocus', true, e); }} on:blur={(e) => { focused = false; if (state.terminalMode) call('setTerminalCursorFocus', false, e); }} on:click={(e) => { if (state.terminalMode) call('updateTerminalCursor', e); }} on:keyup={(e) => { if (state.terminalMode) call('updateTerminalCursor', e); }} on:select={(e) => { if (state.terminalMode) call('updateTerminalCursor', e); }} on:keydown={handleKeydown} on:input={(e) => { syncInput(e.target.value); resizeInput(); }}></textarea></div>
              <div class="composer-controls-pill composer-shared-input-pill"><div class="composer-actions-right">{#if state.terminalMode}<button class="btn-send btn-send-terminal" on:click={runSend} disabled={state.showFreshArchetypeTiles ? (!state.freshInitAwaitingOtherPrompt || !inputText.trim()) : !inputText.trim()} title="Run command"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="19" x2="12" y2="5"></line><polyline points="5 12 12 5 19 12"></polyline></svg></button>{:else}<div class="toggle-pill toggle-pill--triple input-toggle-wrapper" data-mode={state.attachMode} role="group" aria-label="Voice and send controls"><button type="button" class="composer-send-voice-opt composer-send-voice-opt-attach" on:click={beginAttachPicker} title="Add files" aria-label="Add files"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m21.44 11.05-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.48"/></svg></button><button type="button" class:active={state.recording} class:btn-recording={state.recording} class="composer-send-voice-opt composer-send-voice-opt-voice" on:click={toggleVoice} title="Toggle voice recording" aria-label="Toggle voice recording">{#if !state.recording}<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/><line x1="12" x2="12" y1="19" y2="22"/></svg>{:else}<span class="recording-dot"></span>{/if}</button>{#if !state.sending}<button type="button" class:active={!state.recording} class="composer-send-voice-opt composer-send-voice-opt-send" on:click={runSend} disabled={sendDisabled()} title="Send" aria-label="Send message">{#if state.recording || state.attachMode === 'attach'}<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg>{:else}<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="19" x2="12" y2="5"></line><polyline points="5 12 12 5 19 12"></polyline></svg>{/if}</button>{:else}<button type="button" class="composer-send-voice-opt composer-send-voice-opt-stop active" on:click={runStop} title="Stop generating" aria-label="Stop generating"><svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><rect x="6" y="6" width="12" height="12" rx="2"/></svg></button>{/if}</div>{/if}</div></div>
            </div>
          </div>
        </div>
      </div>
      <infring-system-thread-placeholder-shell>{#if state.systemThread}<div class="system-thread-placeholder-row" aria-hidden="true"></div>{/if}</infring-system-thread-placeholder-shell>
      <div class="input-footer"><div class="flex items-center gap-2"><span class="text-xs text-dim">{footerText()}</span></div><div class="input-footer-right">{#if state.currentTip && !state.sending && !state.terminalMode}<div class="tip-bar"><span class="text-xs">{state.currentTip}</span><button class="tip-bar-dismiss" on:click={() => call('dismissTips')} title="Dismiss">&times;</button></div>{/if}</div></div>
    </div>
  </div>
  </infring-composer-lane-shell>
</div>
{:else if state.currentAgent && state.archived}
<infring-chat-archived-banner-shell><div class="chat-archived-banner chat-archived-banner-bottom-center" role="status" aria-live="polite"><span class="text-xs" style="margin-right:10px">Archived thread is read-only. Revive to send messages, run commands, or edit configuration.</span><button type="button" class="btn btn-primary btn-sm" on:click={() => call('reviveCurrentArchivedAgent')}>Revive Agent</button></div></infring-chat-archived-banner-shell>
{/if}
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
