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
