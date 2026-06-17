const COMPONENT_TAG = 'infring-sidebar-agent-list-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-sidebar-agent-list-shell', shadow: 'none' }} />
<script>
  import { onMount, onDestroy } from 'svelte';

  let sidebarAgents = [];
  let uiTick = 0;
  let localConfirmArchiveId = '';
  let unsubs = [];
  let tickTimer = 0;

  function appStoreService() {
    var services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
    return services && services.appStore ? services.appStore : null;
  }
  function app() {
    try {
      var service = appStoreService();
      if (service && typeof service.root === 'function') {
        var root = service.root();
        if (root) return root;
      }
      if (service && typeof service.current === 'function') {
        var current = service.current();
        if (current) return current;
      }
      return null;
    } catch (_e) {
      return null;
    }
  }
  function call(fn) {
    var service = appStoreService();
    if (service && typeof service.method === 'function') {
      var method = service.method(fn);
      if (method) {
        var methodArgs = Array.prototype.slice.call(arguments, 1);
        try { return method.apply(null, methodArgs); } catch (_e) { return undefined; }
      }
    }
    var s = app();
    if (!s || typeof s[fn] !== 'function') return undefined;
    var args = Array.prototype.slice.call(arguments, 1);
    try { return s[fn].apply(s, args); } catch (_e) { return undefined; }
  }
  function chatStore() {
    return typeof window !== 'undefined' ? window.InfringChatStore : null;
  }
  function setSidebarAgents(rows) {
    sidebarAgents = Array.isArray(rows) ? rows : [];
  }
  function appSidebarRows() {
    var s = app() || {};
    if (Array.isArray(s.chatSidebarVisibleRows)) return s.chatSidebarVisibleRows;
    if (Array.isArray(s.chatSidebarRows)) return s.chatSidebarRows;
    return [];
  }
  function refreshSidebarAgents() {
    var rows = appSidebarRows();
    setSidebarAgents(rows);
    var store = chatStore();
    if (store && store.sidebarAgents && typeof store.sidebarAgents.set === 'function') {
      var current = typeof store.sidebarAgents.get === 'function' ? store.sidebarAgents.get() : null;
      if (current !== rows) store.sidebarAgents.set(rows);
    }
  }
  function bump() {
    refreshSidebarAgents();
    uiTick += 1;
  }
  function agentId(agent) {
    return String((agent && agent.id) || '');
  }
  function agentKey(agent) {
    return 'nav-agent-' + agentId(agent) + '-' + ((agent && agent._sidebar_search_result) ? 'search' : 'live');
  }
  function isSystem(agent) {
    return !!(agent && (agent.is_system_thread === true || String(agent.id || '').toLowerCase() === 'system'));
  }
  function preview(agent) {
    return call('chatSidebarPreview', agent) || {};
  }
  function previewText(agent) {
    var rowPreview = preview(agent);
    return String(rowPreview.text || (isSystem(agent) ? '' : 'No messages yet'));
  }
  function previewTime(agent) {
    return String(call('formatChatSidebarTime', preview(agent).ts) || '');
  }
  function statusState(agent) {
    return String(call('agentStatusState', agent) || (agent && agent.sidebar_status_state) || 'unknown').trim().toLowerCase() || 'unknown';
  }
  function statusLabel(agent) {
    return String(call('agentStatusLabel', agent) || '').trim();
  }
  function displayEmoji(agent) {
    return String(call('sidebarDisplayEmoji', agent) || '');
  }
  function canReorder() {
    return !!call('chatSidebarCanReorderTopology');
  }
  function confirmArchiveId() {
    var s = app();
    return String(localConfirmArchiveId || (s && s.confirmArchiveAgentId) || '');
  }
  function isActive(agent) {
    var s = app();
    return !!(s && s.page === 'chat' && s.activeAgentId === (agent && agent.id));
  }
  function rowClass(agent) {
    var s = app() || {};
    var id = agentId(agent);
    var classes = [];
    if (isActive(agent)) classes.push('active');
    if (agent && agent.sidebar_archived) classes.push('nav-agent-row-archived');
    if (agent && agent.revive_recommended) classes.push('nav-agent-row-timeout');
    if (call('shouldShowExpiryCountdown', agent) || call('shouldShowInfinityLifespan', agent)) classes.push('nav-agent-row-with-countdown');
    if (canReorder()) classes.push('nav-agent-row-draggable');
    if (canReorder() && String(s.chatSidebarDragAgentId || '') === id) classes.push('dragging');
    if (canReorder() && String(s.chatSidebarDropTargetId || '') === id) classes.push('drag-target');
    if (canReorder() && String(s.chatSidebarDropTargetId || '') === id && !s.chatSidebarDropAfter) classes.push('drop-before');
    if (canReorder() && String(s.chatSidebarDropTargetId || '') === id && !!s.chatSidebarDropAfter) classes.push('drop-after');
    return classes.join(' ');
  }
  function avatarWrapClass(agent) {
    var classes = [];
    if (agent && agent.sidebar_archived) classes.push('nav-agent-avatar-archived-mask');
    if (agent && agent.revive_recommended) classes.push('nav-agent-avatar-timeout-mask');
    if (call('expiryCountdownCritical', agent)) classes.push('nav-agent-avatar-expiring-critical');
    return classes.join(' ');
  }
  function previewClass(agent) {
    var rowPreview = preview(agent);
    var classes = [];
    if (rowPreview.unread_response) classes.push('preview-unread');
    if (agent && agent.revive_recommended) classes.push('nav-agent-preview-timeout');
    return classes.join(' ');
  }
  function isCollapsed() {
    var s = app();
    return !!(s && s.sidebarCollapsed);
  }
  function selectAgent(agent) {
    call('selectAgentChatFromSidebar', agent);
    bump();
  }
  function startTopologyDrag(agent, event) {
    call('startChatSidebarTopologyDrag', agent, event);
    bump();
  }
  function overTopologyDrag(agent, event) {
    call('handleChatSidebarTopologyDragOver', agent, event);
    bump();
  }
  function dropTopology(agent, event) {
    call('handleChatSidebarTopologyDrop', agent, event);
    bump();
  }
  function endTopologyDrag() {
    call('endChatSidebarTopologyDrag');
    bump();
  }
  function showCollapsedAgent(agent, event) {
    if (isCollapsed()) call('showCollapsedSidebarAgentPopup', agent, event);
  }
  function normalizedPreviewText(agent) {
    var rowPreview = preview(agent);
    var text = String(rowPreview.text || '').trim();
    var normalized = call('normalizeSidebarPopupText', text);
    if (typeof normalized === 'string') text = normalized;
    if (!text && !isSystem(agent)) text = String(previewText(agent) || '').trim();
    normalized = call('normalizeSidebarPopupText', text);
    return typeof normalized === 'string' ? normalized : text;
  }
  function popupTitle(agent) {
    return String((agent && (agent.name || agent.id)) || '').trim();
  }
  function popupBody(agent) {
    return normalizedPreviewText(agent) || previewText(agent);
  }
  function showAgentPreview(agent, event) {
    if (isCollapsed()) {
      showCollapsedAgent(agent, event);
      return;
    }
    var id = agentId(agent);
    var title = String((agent && (agent.name || agent.id)) || '').trim();
    var rowPreview = preview(agent);
    var body = normalizedPreviewText(agent);
    if (!id || !title || isSystem(agent) || !body) {
      call('hideDashboardPopupBySource', 'sidebar');
      return;
    }
    call('showDashboardPopup', 'sidebar-agent:' + id, title, event, {
      source: 'sidebar',
      side: 'right',
      body: body,
      meta_origin: call('sidebarPopupMetaOrigin', rowPreview, 'Agent') || 'Agent',
      meta_time: previewTime(agent),
      unread: !!rowPreview.unread_response
    });
  }
  function hideSidebarPopup() {
    call('hideDashboardPopupBySource', 'sidebar');
  }
  function showStatus(agent, event) {
    var label = statusLabel(agent);
    var id = agentId(agent);
    if (label) {
      call('showDashboardPopup', 'sidebar-status:' + id, 'Agent status', event, {
        source: 'sidebar',
        side: 'right',
        body: label,
        meta_origin: 'Sidebar',
        meta_time: previewTime(agent)
      });
    } else {
      call('hideDashboardPopup', 'sidebar-status:' + id);
    }
  }
  function hideStatus(agent) {
    call('hideDashboardPopup', 'sidebar-status:' + agentId(agent));
  }
  function showTool(agent, event) {
    var rowPreview = preview(agent);
    var label = String((rowPreview && rowPreview.tool_label) || '').trim();
    var id = agentId(agent);
    if (label) {
      call('showDashboardPopup', 'sidebar-tool:' + id, 'Tool activity', event, {
        source: 'sidebar',
        side: 'right',
        body: label,
        meta_origin: 'Sidebar',
        meta_time: previewTime(agent),
        unread: !!rowPreview.unread_response
      });
    } else {
      call('hideDashboardPopup', 'sidebar-tool:' + id);
    }
  }
  function hideTool(agent) {
    call('hideDashboardPopup', 'sidebar-tool:' + agentId(agent));
  }
  function canArchive(agent) {
    return !call('isSidebarArchivedAgent', agent) && !isSystem(agent);
  }
  function requestArchive(agent, event) {
    if (event) event.stopPropagation();
    localConfirmArchiveId = agentId(agent);
    var s = app();
    if (s) s.confirmArchiveAgentId = localConfirmArchiveId;
    bump();
  }
  function confirmArchive(agent, event) {
    if (event) event.stopPropagation();
    localConfirmArchiveId = '';
    call('archiveAgentFromSidebar', agent);
    bump();
  }
  function cancelConfirmArchive(agent) {
    call('hideDashboardPopup', 'sidebar-utility:confirm-archive');
    var id = agentId(agent);
    if (confirmArchiveId() === id) {
      localConfirmArchiveId = '';
      var s = app();
      if (s) s.confirmArchiveAgentId = '';
      bump();
    }
  }
  function showArchivePopup(event) {
    call('showDashboardPopup', 'sidebar-utility:archive-chat', 'Archive chat', event, {
      source: 'sidebar',
      side: 'right',
      body: 'Archive this agent conversation',
      meta_origin: 'Sidebar'
    });
  }
  function showConfirmArchivePopup(event) {
    call('showDashboardPopup', 'sidebar-utility:confirm-archive', 'Confirm archive', event, {
      source: 'sidebar',
      side: 'right',
      body: 'Archive this agent conversation now',
      meta_origin: 'Sidebar'
    });
  }

  onMount(function() {
    refreshSidebarAgents();
    var s = chatStore();
    if (s && s.sidebarAgents) {
      unsubs.push(s.sidebarAgents.subscribe(function(rows) {
        setSidebarAgents(rows);
      }));
    }
    if (s && s.currentAgent) unsubs.push(s.currentAgent.subscribe(bump));
    if (s && s.agents) unsubs.push(s.agents.subscribe(bump));
    tickTimer = window.setInterval(bump, 1000);
  });

  onDestroy(function() {
    for (var i = 0; i < unsubs.length; i++) {
      if (typeof unsubs[i] === 'function') unsubs[i]();
    }
    if (tickTimer) window.clearInterval(tickTimer);
  });
</script>

{#each sidebarAgents as agent (agentKey(agent))}
  <a
    class={"nav-item nav-sub-item nav-agent-row " + rowClass(agent)}
    data-agent-id={agentId(agent)}
    data-dashboard-popup-title={popupTitle(agent)}
    data-dashboard-popup-body={popupBody(agent)}
    data-dashboard-popup-id={'sidebar-agent-' + agentId(agent)}
    on:click={() => selectAgent(agent)}
    aria-current={isActive(agent) ? 'page' : undefined}
    draggable={canReorder()}
    on:dragstart={(event) => startTopologyDrag(agent, event)}
    on:dragover={(event) => overTopologyDrag(agent, event)}
    on:drop={(event) => dropTopology(agent, event)}
    on:dragend={endTopologyDrag}
    on:mouseenter={(event) => showAgentPreview(agent, event)}
    on:mousemove={(event) => showAgentPreview(agent, event)}
    on:focus={(event) => showAgentPreview(agent, event)}
    on:mouseleave={hideSidebarPopup}
    on:blur={hideSidebarPopup}
  >
    <span class="nav-icon nav-agent-icon">
      <span class={"nav-agent-avatar-wrap " + avatarWrapClass(agent)}>
        {#if call('isAgentLiveBusy', agent)}
          <span class="agent-activity-spinner" aria-hidden="true"></span>
        {/if}
        <span class="nav-agent-avatar">
          {#if agent && agent.avatar_url}
            <img src={agent.avatar_url || ''} alt={(agent.name || agent.id || 'agent') + ' avatar'} loading="lazy" />
          {:else if displayEmoji(agent)}
            <span class="nav-agent-avatar-emoji">{displayEmoji(agent)}</span>
          {:else}
            <span class="nav-agent-avatar-fallback infring-logo infring-logo--agent-default" aria-hidden="true"><span class="infring-logo-glyph" aria-hidden="true">&infin;</span></span>
          {/if}
        </span>
        {#if call('shouldShowExpiryCountdown', agent)}
          <span class:critical={!!call('expiryCountdownCritical', agent)} class="agent-expiry-countdown">{call('expiryCountdownLabel', agent)}</span>
        {/if}
        {#if call('shouldShowInfinityLifespan', agent)}
          <span class="agent-expiry-countdown infinite">&infin;</span>
        {/if}
      </span>
      <span class={"agent-status-dot nav-agent-status status-" + statusState(agent)} on:mouseenter={(event) => showStatus(agent, event)} on:mousemove={(event) => showStatus(agent, event)} on:mouseleave={() => hideStatus(agent)} aria-hidden="true"></span>
    </span>
    <span class="nav-agent-main">
      <span class="nav-agent-top">
        <span class="nav-agent-name truncate">{(agent && (agent.name || agent.id)) || ''}</span>
        <span class="nav-agent-meta">
          {#if preview(agent).has_tools}
            <span class={"nav-agent-tool-pill state-" + (preview(agent).tool_state || 'warning')} on:mouseenter={(event) => showTool(agent, event)} on:mousemove={(event) => showTool(agent, event)} on:mouseleave={() => hideTool(agent)}>
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/></svg>
            </span>
          {/if}
          <span class="nav-agent-time">{previewTime(agent)}</span>
        </span>
      </span>
      <span class="nav-agent-bottom">
        <span class={"nav-agent-preview " + previewClass(agent)}>{previewText(agent)}</span>
        {#if canArchive(agent) && confirmArchiveId() !== agentId(agent)}
          <button class="nav-agent-archive" type="button" on:click={(event) => requestArchive(agent, event)} on:mouseenter={showArchivePopup} on:mousemove={showArchivePopup} on:mouseleave={() => call('hideDashboardPopup', 'sidebar-utility:archive-chat')}>
            <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="4" width="18" height="5" rx="1"/><path d="M5 9h14v10a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2z"/><path d="M10 13h4"/></svg>
          </button>
        {/if}
        {#if canArchive(agent) && confirmArchiveId() === agentId(agent)}
          <button class="nav-agent-archive nav-agent-archive-confirm" type="button" on:click={(event) => confirmArchive(agent, event)} on:mouseleave={() => cancelConfirmArchive(agent)} on:mouseenter={showConfirmArchivePopup} on:mousemove={showConfirmArchivePopup}>
            Confirm
          </button>
        {/if}
      </span>
    </span>
  </a>
{/each}
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
