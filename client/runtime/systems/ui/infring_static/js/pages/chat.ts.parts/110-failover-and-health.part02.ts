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
      if (typeof this.publishSlashCommandFeedback === 'function') {
        this.publishSlashCommandFeedback(selectedSlashRow || { cmd: cmd }, {
          status: 'accepted',
          notice_type: 'info',
          text: String(selectedSlashRow && (selectedSlashRow.desc || selectedSlashRow.title || selectedSlashRow.operational_label) || 'Command accepted.').trim()
        });
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
