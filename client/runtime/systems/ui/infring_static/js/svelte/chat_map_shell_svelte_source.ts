const COMPONENT_TAG = 'infring-chat-map-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-chat-map-shell', shadow: 'none' }} />
<script>
  import { onMount, onDestroy } from 'svelte';

  export let dragbarSurface = 'chat-map';
  export let wall = '';
  export let dragging = false;
  export let parentOwnedMechanics = true;

  let mapRows = [];
  let activeIndex = -1;
  let hoveredIndex = -1;
  let unsubs = [];

  function appStoreService() {
    var services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
    return services && services.appStore ? services.appStore : null;
  }
  function rootPage() {
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
    } catch (_) {}
    return null;
  }
  function cp() {
    return (typeof window !== 'undefined' && window.InfringChatPage) || rootPage();
  }
  function store() {
    return (typeof window !== 'undefined' && window.InfringChatStore) || null;
  }
  function call(fn) {
    var args = Array.prototype.slice.call(arguments, 1);
    var service = appStoreService();
    if (service && typeof service.method === 'function') {
      var method = service.method(fn);
      if (method) {
        try { return method.apply(null, args); } catch (_) {}
      }
    }
    var page = cp();
    if (!page || typeof page[fn] !== 'function') return undefined;
    return page[fn].apply(page, args);
  }
  function getMessage(row) {
    var page = cp();
    var idx = Number(row && row.index);
    if (!page || !Array.isArray(page.messages) || !Number.isFinite(idx)) return null;
    return page.messages[idx] || null;
  }
  function refreshMapRows() {
    var s = store();
    var page = cp();
    if (s && typeof s.refreshMapRows === 'function' && page) s.refreshMapRows(page.messages || []);
  }
  function rowClass(row) {
    var role = String(row && row.role ? row.role : 'agent');
    var classes = ['role-' + role];
    if (row && row.longMessage) classes.push('long-message');
    if (Number(row && row.index) === Number(activeIndex)) classes.push('selected-linked');
    if (Number(row && row.index) === Number(hoveredIndex)) classes.push('hover-linked');
    return classes.filter(Boolean).join(' ');
  }
  function startDrag(event) {
    call('startChatMapPointerDrag', event);
  }
  function hidePopup() {
    call('hideDashboardPopupBySource', 'chat-map');
  }
  function step(dir) {
    var page = cp();
    call('stepMessageMap', page && Array.isArray(page.messages) ? page.messages : [], dir);
  }
  function showItem(row, event) {
    var msg = getMessage(row);
    if (!msg) return;
    hoveredIndex = Number(row.index);
    call('showMapItemPopup', msg, Number(row.index), event);
  }
  function hideItem() {
    hoveredIndex = -1;
    call('hideMapItemPopup');
  }
  function jump(row) {
    var msg = getMessage(row);
    if (!msg) return;
    call('jumpToMessage', msg, Number(row.index));
  }
  function toggleDay(row) {
    var msg = getMessage(row);
    if (!msg) return;
    call('toggleMessageDayCollapse', msg);
    refreshMapRows();
  }
  function showDay(row, event) {
    var msg = getMessage(row);
    if (!msg) return;
    call('showMapDayPopup', msg, event);
  }
  function hideDay() {
    call('hideMapDayPopup');
  }

  onMount(function() {
    var s = store();
    if (s && s.mapRows) {
      unsubs.push(s.mapRows.subscribe(function(rows) {
        mapRows = Array.isArray(rows) ? rows : [];
      }));
    }
    if (s && s.mapStepIndex) {
      unsubs.push(s.mapStepIndex.subscribe(function(value) {
        var next = Number(value);
        activeIndex = Number.isFinite(next) ? next : -1;
      }));
    }
  });

  onDestroy(function() {
    for (var i = 0; i < unsubs.length; i++) {
      if (typeof unsubs[i] === 'function') unsubs[i]();
    }
  });
</script>

<div
  class="chat-map-surface drag-bar overlay-shared-surface"
  data-dragbar-surface={dragbarSurface}
  class:is-container-dragging={!!dragging}
  on:pointerdown|capture={startDrag}
  on:mousedown|capture={startDrag}
>
  <button class="chat-map-jump chat-map-jump-up" type="button" on:click={() => step(-1)} title="Previous message" aria-label="Previous message">
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="m18 15-6-6-6 6"></path></svg>
  </button>

  <div class="chat-map-scroll" on:scroll|passive={hidePopup}>
    <div class="chat-map-spacer" aria-hidden="true"></div>
    {#each mapRows as row (row.key)}
      <div class="chat-map-entry">
        {#if row.newDay}
          <button class="chat-map-day" type="button" on:click={() => toggleDay(row)} on:mouseenter={(event) => showDay(row, event)} on:mousemove={(event) => showDay(row, event)} on:focus={(event) => showDay(row, event)} on:mouseleave={hideDay} on:blur={hideDay} aria-label={'Messages for ' + row.dayLabel} data-dashboard-popup-title={'Messages for ' + row.dayLabel} data-dashboard-popup-body={row.dayCollapsed ? 'Expand this day in the chat map' : 'Collapse this day in the chat map'}>
            <span class="chat-map-day-chevron" aria-hidden="true">{row.dayCollapsed ? '▸' : '▾'}</span>
            <span class="chat-map-day-icon" aria-hidden="true">&#9728;</span>
          </button>
        {/if}

        {#if !row.dayCollapsed}
          <button class={"chat-map-item " + rowClass(row)} data-msg-dom-id={row.domId} on:mouseenter={(event) => showItem(row, event)} on:mousemove={(event) => showItem(row, event)} on:focus={(event) => showItem(row, event)} on:mouseleave={hideItem} on:blur={hideItem} on:click={() => jump(row)} aria-label={'Jump to message ' + (Number(row.index) + 1)} data-dashboard-popup-title={'Jump to message ' + (Number(row.index) + 1)} data-dashboard-popup-body={row.markerTitle || ''}>
            {#if !row.markerType}
              <span class="chat-map-item-main">
                <span class="chat-map-bar"></span>
              </span>
            {/if}
            {#if row.markerType === 'model'}
              <span class="chat-map-tool-row">
                <span class="chat-map-marker chat-map-marker-model" title={row.markerTitle} aria-hidden="true">
                  <svg viewBox="0 0 24 24"><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/></svg>
                </span>
              </span>
            {/if}
            {#if row.markerType === 'info'}
              <span class="chat-map-tool-row">
                <span class="chat-map-marker chat-map-marker-info" title={row.markerTitle} aria-hidden="true">
                  <span class="chat-event-info-icon chat-map-info-icon">{row.noticeIcon || 'i'}</span>
                </span>
              </span>
            {/if}
            {#if row.markerType === 'tool'}
              <span class="chat-map-tool-row">
                <span class={"chat-map-marker chat-map-marker-tool state-" + (row.toolOutcome || 'success')} title={row.markerTitle} aria-hidden="true">
                  <svg viewBox="0 0 24 24"><path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/></svg>
                </span>
              </span>
            {/if}
            {#if row.markerType === 'terminal'}
              <span class="chat-map-tool-row">
                <span class="chat-map-marker chat-map-marker-terminal" title={row.markerTitle} aria-hidden="true">
                  <svg viewBox="0 0 24 24"><path d="M4 17 10 11 4 5"/><path d="M12 19h8"/></svg>
                </span>
              </span>
            {/if}
          </button>
        {/if}
      </div>
    {/each}
    <div class="chat-map-spacer" aria-hidden="true"></div>
  </div>

  <button class="chat-map-jump chat-map-jump-down" type="button" on:click={() => step(1)} title="Next message" aria-label="Next message">
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"></path></svg>
  </button>
</div>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
