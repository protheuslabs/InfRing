const COMPONENT_TAG = 'infring-chat-thread-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-chat-thread-shell', shadow: 'none' }} />
<script>
  import { onMount, onDestroy } from 'svelte';

  let messages = [];
  let hoveredIdx = -1;
  let renderWindowVersion = 0;
  let inlineThoughtExpanded = {};
  let unsub;
  let unsubRenderWindow;

  function cp() {
    return (typeof window !== 'undefined' && window.InfringChatPage) || null;
  }
  function call(fn) {
    var p = cp();
    if (!p || typeof p[fn] !== 'function') return undefined;
    var args = Array.prototype.slice.call(arguments, 1);
    return p[fn].apply(p, args);
  }
  function callBool(fn) {
    var result = call.apply(null, arguments);
    return !!result;
  }
  function callStr(fn) {
    var result = call.apply(null, arguments);
    return result == null ? '' : String(result);
  }
  function callArr(fn) {
    var result = call.apply(null, arguments);
    return Array.isArray(result) ? result : [];
  }
  function callObj(fn) {
    var result = call.apply(null, arguments);
    return (result && typeof result === 'object') ? result : {};
  }
  function shouldRenderContent(msg, idx, token) {
    void token;
    return callBool('shouldRenderMessageContent', msg, idx, messages);
  }

  onMount(function() {
    var s = typeof window !== 'undefined' && window.InfringChatStore;
    if (s && s.filteredMessages) {
      unsub = s.filteredMessages.subscribe(function(val) {
        messages = Array.isArray(val) ? val : [];
      });
    }
    if (s && s.renderWindowVersion) {
      unsubRenderWindow = s.renderWindowVersion.subscribe(function(val) {
        renderWindowVersion = Number(val || 0);
      });
    }
  });

  onDestroy(function() {
    if (typeof unsub === 'function') unsub();
    if (typeof unsubRenderWindow === 'function') unsubRenderWindow();
  });

  function onMouseEnter(msg, idx) {
    hoveredIdx = idx;
    var p = cp();
    if (p && typeof p.setHoveredMessage === 'function') p.setHoveredMessage(msg, idx);
  }
  function onMouseLeave() {
    hoveredIdx = -1;
    var p = cp();
    if (p && typeof p.clearHoveredMessage === 'function') p.clearHoveredMessage();
  }
  function toggleTool(tool) {
    var p = cp();
    if (p && typeof p.toggleToolExpansion === 'function') {
      p.toggleToolExpansion(tool);
    } else {
      tool.expanded = !tool.expanded;
    }
    messages = messages;
  }
  function inlineThoughtKey(msg, tool, idx) {
    var row = tool && typeof tool === 'object' ? tool : {};
    var messageKey = String((msg && (msg.id || msg.ts || msg._stream_started_at)) || 'message').slice(0, 120);
    var rowKey = String(
      row.id ||
      row.activity_key ||
      row.projection_kind ||
      row.agent_runtime_engine_id ||
      row.name ||
      idx ||
      'thought'
    ).slice(0, 120);
    return messageKey + ':' + rowKey;
  }
  function inlineThoughtIsExpanded(msg, tool, idx) {
    var key = inlineThoughtKey(msg, tool, idx);
    if (key && Object.prototype.hasOwnProperty.call(inlineThoughtExpanded, key)) {
      return !!inlineThoughtExpanded[key];
    }
    return !!(tool && tool.expanded);
  }
  function toggleInlineThought(msg, tool, idx) {
    var key = inlineThoughtKey(msg, tool, idx);
    var next = !inlineThoughtIsExpanded(msg, tool, idx);
    if (tool && typeof tool === 'object') tool.expanded = next;
    if (key) inlineThoughtExpanded = Object.assign({}, inlineThoughtExpanded, { [key]: next });
    messages = messages;
  }
  function inlineThoughtRenderKey(msg, tool, idx) {
    var row = tool && typeof tool === 'object' ? tool : {};
    return String(
      (msg && (msg.id || msg.ts || msg._stream_started_at)) ||
      row.id ||
      row.projection_kind ||
      row.agent_runtime_engine_id ||
      row.name ||
      idx ||
      'thought'
    ).slice(0, 240);
  }
  function onMetaAction(e, msg, idx) {
    var p = cp();
    if (p && typeof p.handleMessageMetaAction === 'function') {
      p.handleMessageMetaAction(e, msg, idx, messages);
    }
  }
  function expandTerminal(msg, idx) {
    var p = cp();
    if (p && typeof p.expandTerminalMessage === 'function') p.expandTerminalMessage(msg, idx, messages);
  }
  function triggerNotice(msg) {
    var p = cp();
    if (p && typeof p.triggerNoticeAction === 'function') p.triggerNoticeAction(msg);
  }
  function expandDisplayed() {
    var p = cp();
    if (p && typeof p.expandDisplayedMessages === 'function') p.expandDisplayedMessages();
  }
  function msgClass(msg, idx) {
    var r = callStr('messageRoleClass', msg);
    r += msg.thinking ? ' thinking' : '';
    r += msg.streaming ? ' streaming' : '';
    r += callBool('isGrouped', idx, messages) ? ' grouped' : '';
    r += callBool('showMessageTail', msg, idx, messages) ? ' has-tail' : '';
    r += !callBool('isLastInSourceRun', idx, messages) ? ' has-next-in-run' : '';
    r += hoveredIdx === idx ? ' hover-linked' : '';
    r += callBool('isMessageMetaCollapsed', msg, idx, messages) ? ' meta-collapsed' : ' meta-expanded';
    r += callBool('isMessageMetaReserveSpace', msg, idx, messages) ? ' meta-reserved' : '';
    return r;
  }
  function bubbleClass(msg) {
    var p = cp();
    var role = String((msg && msg.role) || '').toLowerCase();
    var isAgent = role === 'agent' || role === 'assistant' || role === 'system';
    var r = '';
    if (isAgent && !msg.thinking && !msg.isHtml) r += ' markdown-body';
    if (!msg.thoughtStreaming && callBool('isErrorMessage', msg)) r += ' message-error';
    if (msg.thoughtStreaming) r += ' thinking-live';
    if (msg._finish_bounce) r += ' message-finish-bounce';
    return r;
  }
  function bubbleVisible(msg, idx) {
    if (msg.thinking) return false;
    if (msg.terminal && callBool('terminalMessageCollapsed', msg, idx, messages)) return false;
    var p = cp();
    if (!p) return false;
    return !!(
      (msg.text && msg.text.trim()) ||
      callBool('messageHasTools', msg) ||
      callBool('messageHasSourceChips', msg) ||
      callObj('messageToolTraceSummary', msg).visible ||
      call('messageProgress', msg) ||
      (msg.file_output && msg.file_output.path) ||
      (msg.folder_output && msg.folder_output.path) ||
      (msg.images && msg.images.length)
    );
  }
  function thoughtTools(msg) {
    var projected = callArr('messageThoughtTools', msg);
    if (projected.length) return projected;
    var tools = Array.isArray(msg && msg.tools) ? msg.tools : [];
    return tools.filter(function(tool) {
      return callBool('isThoughtTool', tool);
    });
  }
  function nonThoughtTools(msg) {
    var projected = callArr('messageNonThoughtTools', msg);
    if (projected.length) return projected;
    var tools = Array.isArray(msg && msg.tools) ? msg.tools : [];
    return tools.filter(function(tool) {
      return !callBool('isThoughtTool', tool);
    });
  }
  function showAvatar(msg) {
    var role = String((msg && msg.role) || '').toLowerCase();
    if (role !== 'agent') return false;
    var p = cp();
    return !(p && typeof p.isCurrentAgentArchived === 'function' && p.isCurrentAgentArchived());
  }
  function canExpand() {
    var p = cp();
    return !!(p && p.canExpandDisplayedMessages);
  }
  function renderKey(msg, idx) {
    return callStr('messageRenderKey', msg, idx) || String(idx);
  }
</script>

<div class="chat-thread">
  {#each messages as msg, idx (renderKey(msg, idx))}
    <div class="chat-message-block" id={callStr('messageDomId', msg, idx)} data-msg-idx={idx}>

      {#if msg.is_notice}
        <infring-chat-divider-shell>
          <div class="chat-day-divider chat-event-divider">
            <span class="chat-day-divider-line" aria-hidden="true"></span>
            <span class="chat-day-divider-label">
              <span
                class="chat-event-info-icon"
                style:display={msg.notice_type === 'info' && !callBool('isRenameNotice', msg) ? '' : 'none'}
                aria-hidden="true"
              >{msg.notice_icon || 'i'}</span>
              <span>{msg.notice_type === 'info' ? ('Chat info: ' + String(msg.notice_label || 'Info update')) : (msg.notice_label || 'Model switched')}</span>
              <button
                class="chat-notice-action-btn"
                type="button"
                style:display={callBool('noticeActionVisible', msg) ? '' : 'none'}
                disabled={callBool('noticeActionBusy', msg)}
                on:click|stopPropagation={() => triggerNotice(msg)}
              >{callStr('noticeActionLabel', msg)}</button>
            </span>
            <span class="chat-day-divider-line" aria-hidden="true"></span>
          </div>
        </infring-chat-divider-shell>
      {/if}

      {#if callBool('isNewMessageDay', messages, idx)}
        <infring-chat-divider-shell>
          <div class="chat-day-anchor chat-day-divider" id={callStr('messageDayDomId', msg)} data-day={callStr('messageDayKey', msg)}>
            <span class="chat-day-divider-line" aria-hidden="true"></span>
            <span class="chat-day-divider-label">{callStr('messageDayLabel', msg)}</span>
            <span class="chat-day-divider-line" aria-hidden="true"></span>
          </div>
        </infring-chat-divider-shell>
      {/if}

      <infring-chat-stream-shell
        class={"message " + msgClass(msg, idx)}
        style:display={msg.is_notice ? 'none' : ''}
        data-message-dom-id={callStr('messageDomId', msg, idx)}
        data-origin-kind={callStr('messageOriginKind', msg)}
        role={msg.role || ''}
        grouped={callBool('isGrouped', idx, messages) ? 'true' : null}
        streaming={msg.streaming ? 'true' : null}
        thinking={msg.thinking ? 'true' : null}
        hovered={hoveredIdx === idx ? 'true' : null}
        on:mouseenter={() => onMouseEnter(msg, idx)}
        on:mouseleave={() => onMouseLeave()}
      >
        <div class="message-avatar" style:display={showAvatar(msg) ? '' : 'none'}>
          <span class="agent-mark infring-logo infring-logo--agent-default" aria-hidden="true"><span class="infring-logo-glyph" aria-hidden="true">&infin;</span></span>
        </div>
        <div class="message-body">
          <div class="message-bubble message-bubble-thinking" data-thinking-bubble-id={String(msg.id || '')} style={callStr('thinkingBubbleSizeStyle', msg)} style:display={msg.thinking ? '' : 'none'}>
            <span class="thinking-orb-link" aria-hidden="true">
              <span class="thinking-orb-link-dot thinking-orb-link-dot-1"></span>
              <span class="thinking-orb-link-dot thinking-orb-link-dot-2"></span>
              <span class="thinking-orb-link-dot thinking-orb-link-dot-3"></span>
            </span>
            {#if callArr('thinkingBubbleTraceRows', msg).length}
              <div class="thinking-trace-list">
                {#each callArr('thinkingBubbleTraceRows', msg) as row (row.id)}
                  {#if row.children && row.children.length}
                    <details class="thinking-trace-file-group">
                      <summary class={"thinking-trace-row state-" + String(row.state || 'done') + " line-" + String(row.line_kind || 'status')}>
                        <span class="thinking-trace-dot" aria-hidden="true"></span>
                        <span
                          class={row.shimmer ? 'thinking-inline-subtext thinking-shimmer-text' : 'thinking-inline-subtext'}
                          data-shimmer-text={row.shimmer ? row.text : ''}
                        >{row.text}</span>
                        <span class="thinking-trace-chevron" aria-hidden="true">▸</span>
                      </summary>
                      <div class="thinking-trace-file-children">
                        {#each row.children as child (child.id)}
                          <div class={"thinking-trace-row thinking-trace-child-row state-" + String(child.state || 'done') + " line-" + String(child.line_kind || row.line_kind || 'status')}>
                            <span class="thinking-trace-dot" aria-hidden="true"></span>
                            <span class="thinking-inline-subtext">{child.text}</span>
                          </div>
                        {/each}
                      </div>
                    </details>
                  {:else}
                    <div class={"thinking-trace-row state-" + String(row.state || 'done') + " line-" + String(row.line_kind || 'status')}>
                      <span class="thinking-trace-dot" aria-hidden="true"></span>
                      <span
                        class={row.shimmer ? 'thinking-inline-subtext thinking-shimmer-text' : 'thinking-inline-subtext'}
                        data-shimmer-text={row.shimmer ? row.text : ''}
                      >{row.text}</span>
                    </div>
                  {/if}
                {/each}
              </div>
            {:else}
              <div class="thinking-inline-text"><em class="thinking-shimmer-text" data-shimmer-text={callStr('thinkingBubbleLineText', msg)}>{callStr('thinkingBubbleLineText', msg)}</em></div>
            {/if}
            <div class="typing-dots"><span></span><span></span><span></span></div>
          </div>

          <div
            class={"message-bubble" + bubbleClass(msg)}
            data-message-bubble-content-shell="true"
            style:display={bubbleVisible(msg, idx) || (msg.terminal && callBool('terminalMessageCollapsed', msg, idx, messages)) ? '' : 'none'}
          >
            <div
              class={"message-agent-name " + callStr('messageTitleClass', msg) + (msg.terminal ? ' terminal-actor-label' : '')}
              style:display={callBool('showMessageTitle', msg, idx, messages) ? '' : 'none'}
            >
              <span class="message-agent-name-bracket" aria-hidden="true">[</span><span class="message-agent-name-label">{callStr('messageTitleLabel', msg)}</span><span class="message-agent-name-bracket" aria-hidden="true">]</span>
            </div>

            {#if shouldRenderContent(msg, idx, renderWindowVersion) && thoughtTools(msg).length}
              <div class="message-inline-thoughts">
                {#each thoughtTools(msg) as tool, thoughtIdx (inlineThoughtRenderKey(msg, tool, thoughtIdx))}
                  <section class={"message-inline-thought" + (inlineThoughtIsExpanded(msg, tool, thoughtIdx) ? ' message-inline-thought-expanded' : '')}>
                    <button
                      type="button"
                      class="message-inline-thought-toggle"
                      aria-expanded={inlineThoughtIsExpanded(msg, tool, thoughtIdx) ? 'true' : 'false'}
                      on:click|preventDefault|stopPropagation={() => toggleInlineThought(msg, tool, thoughtIdx)}
                    >
                      <span class="message-inline-thought-label">{callStr('thoughtToolLabel', tool)}</span>
                      <span class="message-inline-thought-chevron" aria-hidden="true">{inlineThoughtIsExpanded(msg, tool, thoughtIdx) ? '▾' : '▸'}</span>
                    </button>
                    {#if inlineThoughtIsExpanded(msg, tool, thoughtIdx)}
                      <div class="message-inline-thought-body">
                        {#each callArr('toolProjectionSections', tool) as section (section.id)}
                          {#if section.trace_rows && section.trace_rows.length}
                            <div class="thinking-trace-list message-inline-thought-trace">
                              {#each section.trace_rows as row (row.id)}
                                {#if row.children && row.children.length}
                                  <details class="thinking-trace-file-group">
                                    <summary class={"thinking-trace-row message-inline-thought-line state-" + String(row.state || 'done') + " line-" + String(row.line_kind || 'status')}>
                                      <span class="thinking-trace-dot" aria-hidden="true"></span>
                                      <span
                                        class={row.shimmer ? 'thinking-inline-subtext thinking-shimmer-text' : 'thinking-inline-subtext'}
                                        data-shimmer-text={row.shimmer ? row.text : ''}
                                      >{row.text}</span>
                                      <span class="thinking-trace-chevron" aria-hidden="true">▸</span>
                                    </summary>
                                    <div class="thinking-trace-file-children">
                                      {#each row.children as child (child.id)}
                                        <div class={"thinking-trace-row thinking-trace-child-row message-inline-thought-line state-" + String(child.state || 'done') + " line-" + String(child.line_kind || row.line_kind || 'status')}>
                                          <span class="thinking-trace-dot" aria-hidden="true"></span>
                                          <span class="thinking-inline-subtext">{child.text}</span>
                                        </div>
                                      {/each}
                                    </div>
                                  </details>
                                {:else}
                                  <div class={"thinking-trace-row message-inline-thought-line state-" + String(row.state || 'done') + " line-" + String(row.line_kind || 'status')}>
                                    <span class="thinking-trace-dot" aria-hidden="true"></span>
                                    <span
                                      class={row.shimmer ? 'thinking-inline-subtext thinking-shimmer-text' : 'thinking-inline-subtext'}
                                      data-shimmer-text={row.shimmer ? row.text : ''}
                                    >{row.text}</span>
                                  </div>
                                {/if}
                              {/each}
                            </div>
                          {:else}
                            <pre class="message-inline-thought-pre">{section.text}</pre>
                          {/if}
                        {/each}
                      </div>
                    {/if}
                  </section>
                {/each}
              </div>
            {/if}

            {#if msg.terminal && callBool('terminalMessageCollapsed', msg, idx, messages)}
              <infring-message-terminal-shell>
                <div
                  class={"terminal-toolbox " + callStr('terminalToolboxSideClass', msg)}
                  role="button"
                  tabindex="0"
                  title="Click to expand full output"
                  on:click={() => expandTerminal(msg, idx)}
                  on:keydown={e => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); expandTerminal(msg, idx); } }}
                >
                  <span class="terminal-toolbox-icon" aria-hidden="true">
                    <svg viewBox="0 0 24 24" focusable="false"><path d="m4 6 6 6-6 6"></path><path d="M12 18h8"></path></svg>
                  </span>
                  <span class="terminal-toolbox-copy">
                    <span class="terminal-toolbox-title">Terminal Output</span>
                    <span class="terminal-toolbox-preview">{callStr('terminalToolboxPreview', msg)}</span>
                  </span>
                </div>
              </infring-message-terminal-shell>
            {/if}

            {#if bubbleVisible(msg, idx) && shouldRenderContent(msg, idx, renderWindowVersion)}
              <infring-chat-bubble-render typing={!!msg._typingVisual ? '1' : '0'} html={callStr('messageBubbleHtml', msg, idx)} plain={String(msg.text || '')}></infring-chat-bubble-render>
            {:else if bubbleVisible(msg, idx)}
              <infring-message-placeholder-shell>
                <div class="message-placeholder-shell message-placeholder-shell-inline" style={callStr('messagePlaceholderStyle', msg, idx, messages)}>
                  {#each callArr('messagePlaceholderLineIndices', msg, idx, messages) as lineIdx}
                    <span class={"message-placeholder-line" + (lineIdx === (call('messagePlaceholderResolvedLineCount', msg, idx, messages) - 1) && call('messagePlaceholderResolvedLineCount', msg, idx, messages) > 1 ? ' message-placeholder-line-short' : '')}></span>
                  {/each}
                </div>
              </infring-message-placeholder-shell>
            {/if}

          <infring-message-context-shell>
            <div class="message-source-chips" style:display={shouldRenderContent(msg, idx, renderWindowVersion) && callBool('messageHasSourceChips', msg) ? '' : 'none'}>
              {#each callArr('messageSourceChips', msg) as chip (chip.id)}
                <a class="message-source-chip" href={chip.url} target="_blank" rel="noopener" title={chip.url}>
                  <span class="message-source-chip-label">{chip.label}</span>
                  <span class="message-source-chip-host" style:display={chip.host ? '' : 'none'}>{chip.host}</span>
                </a>
              {/each}
            </div>
            <div class="message-tool-trace-summary" style:display={shouldRenderContent(msg, idx, renderWindowVersion) && callObj('messageToolTraceSummary', msg).visible ? '' : 'none'}>
              <span class="message-tool-trace-label">{callObj('messageToolTraceSummary', msg).label}</span>
              <span class="message-tool-trace-detail">{callObj('messageToolTraceSummary', msg).detail}</span>
            </div>
          </infring-message-context-shell>

          {#if shouldRenderContent(msg, idx, renderWindowVersion) && call('messageProgress', msg)}
            <infring-message-progress-shell>
              <div class="chat-progress-wrap">
                <div class="chat-progress-meta">
                  <span>{callObj('messageProgress', msg).label}</span>
                  <span>{callObj('messageProgress', msg).percent + '%'}</span>
                </div>
                <div class="chat-progress-track">
                  <span class="chat-progress-fill" style={callStr('progressFillStyle', msg)}></span>
                </div>
              </div>
            </infring-message-progress-shell>
          {/if}

          {#if shouldRenderContent(msg, idx, renderWindowVersion) && msg.file_output && msg.file_output.path}
            <infring-message-artifact-shell>
              <div class="chat-artifact-card chat-file-output">
                <div class="chat-artifact-head">
                  <span class="chat-artifact-title">File Output</span>
                  <span class="chat-artifact-path">{msg.file_output.path}</span>
                </div>
                <pre class="chat-artifact-pre">{msg.file_output.content || ''}</pre>
              </div>
            </infring-message-artifact-shell>
          {/if}

          {#if shouldRenderContent(msg, idx, renderWindowVersion) && msg.folder_output && msg.folder_output.path}
            <infring-message-artifact-shell>
              <div class="chat-artifact-card chat-folder-output">
                <div class="chat-artifact-head">
                  <span class="chat-artifact-title">Folder Output</span>
                  <span class="chat-artifact-path">{msg.folder_output.path}</span>
                </div>
                <pre class="chat-artifact-pre">{msg.folder_output.tree || ''}</pre>
                {#if msg.folder_output.download_url}
                  <a class="chat-folder-download-link" href={msg.folder_output.download_url} target="_blank" rel="noopener">Download archive</a>
                {/if}
              </div>
            </infring-message-artifact-shell>
          {/if}

          {#if shouldRenderContent(msg, idx, renderWindowVersion) && msg.images && msg.images.length}
            <infring-message-media-shell>
              <div style="display:flex;flex-wrap:wrap;gap:8px;margin:8px 0">
                {#each msg.images as img (img.file_id)}
                  <a href={'/api/uploads/' + img.file_id} target="_blank" style="display:block">
                    <img src={'/api/uploads/' + img.file_id} alt={img.filename || 'uploaded image'} style="max-width:320px;max-height:320px;border-radius:8px;border:1px solid var(--border);cursor:pointer" loading="lazy">
                  </a>
                {/each}
              </div>
            </infring-message-media-shell>
          {/if}

          <infring-tool-card-stack-shell>
            {#each (shouldRenderContent(msg, idx, renderWindowVersion) ? nonThoughtTools(msg) : []) as tool (tool.id)}
              <div
                class={"tool-card" + (tool.is_error && !callBool('isBlockedTool', tool) ? ' tool-card-error' : '') + (callBool('isBlockedTool', tool) ? ' tool-card-blocked' : '') + (callBool('isToolSuccessful', tool) ? ' tool-card-success' : '') + (callBool('isThoughtTool', tool) ? ' tool-card-thought' : '')}
                data-tool={tool.name}
              >
                <div class="tool-card-header" on:click={() => toggleTool(tool)}>
                  {#if callBool('isThoughtTool', tool)}
                    <span class="tool-card-thought-brain" aria-hidden="true">
                      <svg viewBox="0 0 24 24" focusable="false">
                        <path d="M9 3c-2.8 0-5 2.2-5 5 0 .5.1 1 .2 1.4A3.8 3.8 0 0 0 3 12.1C3 14.3 4.7 16 6.9 16H9" />
                        <path d="M15 3c2.8 0 5 2.2 5 5 0 .5-.1 1-.2 1.4a3.8 3.8 0 0 1 1.2 2.7c0 2.2-1.7 3.9-3.9 3.9H15" />
                        <path d="M9 3v13M15 3v13" />
                        <path d="M9 7.2h1.1c.6 0 1 .4 1 1v.5c0 .6.4 1 1 1h.8c.6 0 1 .4 1 1V11" />
                        <path d="M9 11.8h1.1c.6 0 1 .4 1 1v.4c0 .6.4 1 1 1h.8c.6 0 1 .4 1 1V16" />
                      </svg>
                    </span>
                  {:else if tool.running}
                    <div class="tool-card-spinner"></div>
                  {:else if callBool('isBlockedTool', tool)}
                    <span class="tool-icon-blocked" aria-hidden="true">
                      <svg viewBox="0 0 24 24" focusable="false"><path d="M12 3 5 6v6c0 5.2 3.6 8.6 7 10 3.4-1.4 7-4.8 7-10V6l-7-3z"></path></svg>
                    </span>
                  {:else if !tool.is_error}
                    <span class="tool-icon-ok">&#10003;</span>
                  {:else}
                    <span class="tool-icon-err">&#10007;</span>
                  {/if}
                  <span class="tool-card-icon" style:display={callBool('isThoughtTool', tool) ? 'none' : ''}>{@html callStr('toolIcon', tool.name)}</span>
                  <span class="tool-card-name">{callBool('isThoughtTool', tool) ? callStr('thoughtToolLabel', tool) : callStr('toolDisplayName', tool)}</span>
                  <span
                    style:display={callBool('isThoughtTool', tool) || !callStr('toolStatusText', tool) ? 'none' : ''}
                    class={"text-xs" + (callBool('isBlockedTool', tool) ? ' tool-status-blocked' : callBool('isToolSuccessful', tool) ? ' tool-status-success' : ' text-dim')}
                    style="margin-left:auto"
                  >{callStr('toolStatusText', tool)}</span>
                  <span class="tool-expand-chevron" style={callBool('isThoughtTool', tool) ? 'margin-left:auto' : ''}>{tool.expanded ? '▾' : '▸'}</span>
                </div>

                {#if tool._imageUrls && tool._imageUrls.length}
                  <div style="padding:8px 12px;display:flex;flex-wrap:wrap;gap:8px">
                    {#each tool._imageUrls || [] as iurl}
                      <a href={iurl} target="_blank" style="display:block">
                        <img src={iurl} alt="Generated image" style="max-width:320px;max-height:320px;border-radius:8px;border:1px solid var(--border);cursor:pointer" loading="lazy">
                      </a>
                    {/each}
                  </div>
                {/if}

                {#if tool._audioFile}
                  <div style="padding:8px 12px">
                    <div class="audio-player">
                      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/><path d="M15.54 8.46a5 5 0 0 1 0 7.07"/><path d="M19.07 4.93a10 10 0 0 1 0 14.14"/></svg>
                      <span class="text-xs">{'Audio: ' + tool._audioFile.split('/').pop()}</span>
                      {#if tool._audioDuration}
                        <span class="text-xs text-dim">{'~' + Math.round((tool._audioDuration || 0) / 1000) + 's'}</span>
                      {/if}
                    </div>
                  </div>
                {/if}

                {#if tool.expanded}
                  <div class="tool-card-body">
                    {#each callArr('toolProjectionSections', tool) as section (section.id)}
                      <div style="margin-bottom:6px">
                        <div class="tool-section-label">{section.label}</div>
                        {#if section.trace_rows && section.trace_rows.length}
                          <div class="thinking-trace-list thinking-trace-list-summary">
                            {#each section.trace_rows as row (row.id)}
                              <div class={"thinking-trace-row state-" + String(row.state || 'done') + " line-" + String(row.line_kind || 'status')}>
                                <span class="thinking-trace-dot" aria-hidden="true"></span>
                                <span
                                  class={row.shimmer ? 'thinking-inline-subtext thinking-shimmer-text' : 'thinking-inline-subtext'}
                                  data-shimmer-text={row.shimmer ? row.text : ''}
                                >{row.text}</span>
                              </div>
                            {/each}
                          </div>
                        {:else}
                          <pre class="tool-pre">{section.text}</pre>
                        {/if}
                      </div>
                    {/each}
                  </div>
                {/if}
              </div>
            {/each}
          </infring-tool-card-stack-shell>

          <infring-message-meta-shell
            state={callStr('messageMetadataShellState', msg, idx, messages)}
            on:message-meta-action={e => onMetaAction(e, msg, idx)}
          ></infring-message-meta-shell>
          </div>
        </div>
      </infring-chat-stream-shell>
    </div>
  {/each}

  {#if canExpand()}
    <div style="display:flex;justify-content:center;padding:10px 0 2px">
      <button class="btn btn-ghost btn-sm" type="button" on:click={() => expandDisplayed()}>Expand</button>
    </div>
  {/if}
</div>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
