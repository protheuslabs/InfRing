      if (remainingMs <= 0) return this.isAgentPendingTermination(agent) ? '0m' : '';
      var totalMin = Math.max(1, Math.ceil(remainingMs / 60000));
      var monthMin = 30 * 24 * 60;
      if (totalMin >= monthMin) {
        return Math.max(1, Math.ceil(totalMin / monthMin)) + 'm';
      }
      if (totalMin >= 1440) {
        return Math.max(1, Math.ceil(totalMin / 1440)) + 'd';
      }
      if (totalMin >= 60) {
        return Math.max(1, Math.ceil(totalMin / 60)) + 'h';
      }
      return totalMin + 'm';
    },

    expiryCountdownCritical(agent) {
      if (agent && agent.revive_recommended === true) return false;
      if (this.isAgentPendingTermination(agent)) return true;
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null) return false;
      var totalMs = this.agentContractTotalMs(agent);
      if (!Number.isFinite(totalMs) || totalMs <= 0) return false;
      var thresholdMs = Math.min(3600000, Math.max(1, Math.floor(totalMs * 0.2)));
      return remainingMs > 0 && remainingMs <= thresholdMs;
    },

    agentContractTotalMs(agent) {
      if (!agent || typeof agent !== 'object') return null;
      var durationMs = Number(agent.contract_total_ms);
      if (Number.isFinite(durationMs) && durationMs > 0) return Math.floor(durationMs);
      return null;
    },

    agentHeartStates(agent) {
      var totalHearts = 5;
      var hearts = [true, true, true, true, true];
      if (!agent || typeof agent !== 'object') return hearts;
      if (agent.is_system_thread) return hearts;
      if (agent.revive_recommended === true) return [false, false, false, false, false];
      if (!this.agentAutoTerminateEnabled(agent) || !this.agentContractHasFiniteExpiry(agent)) return [true];
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null) return [true];
      if (remainingMs <= 0 && this.isAgentPendingTermination(agent)) return [false, false, false, false, false];
      var totalMs = this.agentContractTotalMs(agent);
      if (!Number.isFinite(totalMs) || totalMs <= 0) return [true];
      var ratio = Math.max(0, Math.min(1, remainingMs / totalMs));
      var filled = Math.ceil(ratio * totalHearts);
      if (remainingMs <= 0 && this.isAgentPendingTermination(agent)) filled = 0;
      if (filled < 0) filled = 0;
      if (filled > totalHearts) filled = totalHearts;
      for (var i = 0; i < totalHearts; i++) {
        hearts[i] = i < filled;
      }
      return hearts;
    },

    agentHeartShowsInfinity(agent) {
      if (!agent || typeof agent !== 'object') return false;
      if (agent.is_system_thread) return false;
      if (agent.revive_recommended === true) return false;
      return !this.agentAutoTerminateEnabled(agent) || !this.agentContractHasFiniteExpiry(agent);
    },

    agentHeartMeterLabel(agent) {
      if (!agent || typeof agent !== 'object' || agent.is_system_thread) return '';
      if (agent.revive_recommended === true) return 'Time limit: timed out';
      if (!this.agentAutoTerminateEnabled(agent) || !this.agentContractHasFiniteExpiry(agent)) {
        return 'Time limit: unlimited';
      }
      var label = this.expiryCountdownLabel(agent);
      if (label) return 'Time remaining: ' + label;
      return 'Time limit active';
    },

    showDashboardPopup(id, label, ev, overrides) {
      var popupId = String(id || '').trim();
      var title = String(label || '').trim();
      if (!popupId || !title) {
        if (typeof this.hideDashboardPopup === 'function') this.hideDashboardPopup();
        return;
      }
      var eventType = String((ev && ev.type) || '').toLowerCase();
      if (
        eventType === 'mouseleave' ||
        eventType === 'pointerleave' ||
        eventType === 'blur' ||
        eventType === 'focusout'
      ) {
        if (typeof this.hideDashboardPopup === 'function') this.hideDashboardPopup(popupId);
        return;
      }
      var config = overrides && typeof overrides === 'object' ? overrides : {};
      var anchor = typeof this.dashboardPopupAnchorPoint === 'function'
        ? this.dashboardPopupAnchorPoint(ev, config.side)
        : { left: 0, top: 0, side: String(config.side || 'bottom'), inline_away: 'right', block_away: 'bottom' };
      if (
        (!anchor || !Number.isFinite(Number(anchor.left)) || !Number.isFinite(Number(anchor.top)) || Number(anchor.left) <= 0 || Number(anchor.top) <= 0) &&
        ev
      ) {
        var fallbackNode = ev.currentTarget || ev.target;
        if (fallbackNode && typeof fallbackNode.getBoundingClientRect === 'function') {
          var fallbackRect = fallbackNode.getBoundingClientRect();
          var fallbackSide = String(config.side || (anchor && anchor.side) || 'bottom').toLowerCase();
          if (fallbackSide !== 'top' && fallbackSide !== 'left' && fallbackSide !== 'right') fallbackSide = 'bottom';
          anchor = {
            left: Math.round(fallbackSide === 'right' ? Number(fallbackRect.right || 0) : Number(fallbackRect.left || 0)),
            top: Math.round(fallbackSide === 'bottom' ? Number(fallbackRect.bottom || 0) : Number(fallbackRect.top || 0)),
            side: fallbackSide,
            inline_away: 'right',
            block_away: 'bottom'
          };
        }
      }
      var service = typeof this.dashboardPopupService === 'function' ? this.dashboardPopupService() : null;
      this.dashboardPopup = service && typeof service.openState === 'function'
        ? service.openState(popupId, title, config, anchor)
        : {
          id: popupId,
          active: true,
          source: String(config.source || '').trim(),
          title: title,
          body: String(config.body || '').trim(),
          meta_origin: '',
          meta_time: '',
          unread: !!config.unread,
          left: anchor.left,
          top: anchor.top,
          side: anchor.side,
          inline_away: anchor.inline_away === 'left' ? 'left' : 'right',
          block_away: anchor.block_away === 'top' ? 'top' : 'bottom',
          compact: false
        };
    },

    hideDashboardPopup(rawId) {
      var service = typeof this.dashboardPopupService === 'function' ? this.dashboardPopupService() : null;
      if (service && typeof service.closeState === 'function') {
        this.dashboardPopup = service.closeState(this.dashboardPopup, rawId);
        return;
      }
      var popupId = String(rawId || '').trim();
      var currentId = String((this.dashboardPopup && this.dashboardPopup.id) || '').trim();
      if (popupId && currentId && popupId !== currentId) return;
      if (typeof this.clearDashboardPopupState === 'function') {
        this.clearDashboardPopupState();
        return;
      }
      this.dashboardPopup = {
        id: '',
        active: false,
        source: '',
        title: '',
        body: '',
        meta_origin: '',
        meta_time: '',
        unread: false,
        left: 0,
        top: 0,
        side: 'bottom',
        inline_away: 'right',
        block_away: 'bottom',
        compact: false
      };
    },

    hideDashboardPopupBySource(source) {
      var popupSource = String(source || '').trim();
      if (!popupSource) return;
      var popup = this.dashboardPopup || {};
      if (String(popup.source || '').trim() !== popupSource) return;
      this.hideDashboardPopup(String(popup.id || '').trim());
    },

    closeTaskbarHeroMenu() {
      this.taskbarHeroMenuOpen = false;
    },

    async waitForTaskbarHeroGatewayHealth(options) {
      var opts = (options && typeof options === 'object') ? options : {};
      var timeoutMs = Number(opts.timeoutMs);
      if (!Number.isFinite(timeoutMs) || timeoutMs < 1000) timeoutMs = 24000;
      var intervalMs = Number(opts.intervalMs);
      if (!Number.isFinite(intervalMs) || intervalMs < 200) intervalMs = 750;
      var requestTimeoutMs = Number(opts.requestTimeoutMs);
      if (!Number.isFinite(requestTimeoutMs) || requestTimeoutMs < 250) requestTimeoutMs = 1600;
      var deadline = Date.now() + timeoutMs;
      var lastError = '';
      var setTimer = (typeof window !== 'undefined' && typeof window.setTimeout === 'function')
        ? window.setTimeout.bind(window)
        : setTimeout;
      var clearTimer = (typeof window !== 'undefined' && typeof window.clearTimeout === 'function')
        ? window.clearTimeout.bind(window)
        : clearTimeout;
      while (Date.now() <= deadline) {
        var controller = null;
        var timer = 0;
        try {
          if (typeof AbortController !== 'undefined') controller = new AbortController();
        } catch (_) {
          controller = null;
        }
        try {
          if (controller) {
            timer = setTimer(function() {
              try {
                controller.abort();
              } catch (_) {}
            }, requestTimeoutMs);
          }
          var response = await fetch('/healthz?taskbar_restart_probe=' + encodeURIComponent(String(Date.now())), {
            method: 'GET',
            cache: 'no-store',
            signal: controller ? controller.signal : undefined
          });
          if (response && response.ok) {
            var parsed = {};
            try {
              parsed = await response.json();
            } catch (_) {
              parsed = {};
            }
            if (!parsed || parsed.ok !== false) {
              return { ok: true, type: 'gateway_health_recovered', payload: parsed };
            }
          }
          lastError = 'health_http_' + (response && response.status ? response.status : 'unknown');
        } catch (error) {
          lastError = String(error && error.message ? error.message : error || 'health_probe_failed').slice(0, 180);
        } finally {
          if (timer) {
            try {
              clearTimer(timer);
            } catch (_) {}
          }
        }
        if (Date.now() > deadline) break;
        await new Promise(function(resolve) {
          setTimer(resolve, intervalMs);
        });
      }
      return {
        ok: false,
        type: 'gateway_health_reconnect_timeout',
        error: lastError || 'gateway_health_unavailable'
      };
    },

    closeTaskbarTextMenu() {
      this.taskbarTextMenuOpen = '';
    },

    taskbarTextMenuIsOpen(menuName) {
      var key = String(menuName || '').trim().toLowerCase();
      if (!key) return false;
      return String(this.taskbarTextMenuOpen || '').trim().toLowerCase() === key;
    },

    toggleTaskbarTextMenu(menuName) {
      var key = String(menuName || '').trim().toLowerCase();
      if (!key) {
        this.closeTaskbarTextMenu();
        return;
      }
      this.closeTaskbarHeroMenu();
      this.taskbarTextMenuOpen = this.taskbarTextMenuIsOpen(key) ? '' : key;
    },

    handleTaskbarHelpManual() {
      this.closeTaskbarTextMenu();
      this.openPopupWindow('manual');
    },
    handleTaskbarHelpReportIssue() {
      this.closeTaskbarTextMenu();
      this.openPopupWindow('report');
    },
    async submitReportIssueDraft() {
      var draft = String(this.reportIssueDraft || '').trim();
      if (!draft) {
        InfringToast.error('Please add issue details before submitting.');
        return;
      }
      var entry = {
        id: 'issue-' + String(Date.now()),
        ts: Date.now(),
        text: draft,
        page: String(this.page || '').trim(),
        agent_id: String((this.currentAgent && this.currentAgent.id) || '').trim()
      };
      try {
        var raw = localStorage.getItem('infring-issue-report-drafts');
        var list = raw ? JSON.parse(raw) : [];
        if (!Array.isArray(list)) list = [];
        list.unshift(entry);
        localStorage.setItem('infring-issue-report-drafts', JSON.stringify(list.slice(0, 25)));
      } catch(_) {}
      var title = ((draft.split(/\r?\n/).find(function(line) { return String(line || '').trim(); }) || draft).replace(/\s+/g, ' ').trim().slice(0, 120) || 'Dashboard issue report');
      var issueBody = '## User Report\n\n' + draft + '\n\n## Runtime Context\n- page: ' + (entry.page || 'unknown') + '\n- agent_id: ' + (entry.agent_id || 'none') + '\n- reported_at: ' + new Date(entry.ts || Date.now()).toISOString() + '\n- client_version: ' + String(this.version || 'unknown');
      try {
        var result = await InfringAPI.post('/api/dashboard/action', {
          action: 'dashboard.github.issue.create',
          payload: { title: title, body: issueBody, source: 'dashboard_report_popup' }
        });
        var actionResult = result && typeof result === 'object' ? (result.lane || result.payload || result) : {};
        if ((result && result.ok === false) || (actionResult && actionResult.ok === false)) {
          throw new Error(String((actionResult && (actionResult.error || actionResult.message)) || (result && (result.error || result.message)) || 'issue_submit_failed'));
        }
        var issueUrl = String((actionResult && (actionResult.html_url || actionResult.issue_url)) || '').trim();
        this.reportIssueDraft = ''; this.closePopupWindow('report');
        InfringToast.success(issueUrl ? ('Issue submitted: ' + issueUrl) : 'Issue submitted.');
      } catch (e) {
        InfringToast.error('Issue submit failed (saved locally): ' + String(e && e.message ? e.message : 'unknown error'));
      }
    },
    manualDocumentMarkdown() {
      // Canonical source: docs/workspace/manuals/infring_manual_help_tab.md
      var encoded = 'IyBJbmZyaW5nIE1hbnVhbAoKX09wZXJhdG9yLWZhY2luZyBndWlkZSBmb3IgdGhlIEhlbHAgdGFiXwoKIyMgVGFibGUgb2YgQ29udGVudHMKLSBbV2hhdCBJbmZyaW5nIElzXSgjd2hhdC1pbmZyaW5nLWlzKQotIFtJbnN0YWxsICsgU3RhcnRdKCNpbnN0YWxsLS1zdGFydCkKLSBbQ0xJIEd1aWRlXSgjY2xpLWd1aWRlKQotIFtVSSBHdWlkZV0oI3VpLWd1aWRlKQotIFtUb29scyArIEV2aWRlbmNlXSgjdG9vbHMtLWV2aWRlbmNlKQotIFtNZW1vcnkgKyBTZXNzaW9uc10oI21lbW9yeS0tc2Vzc2lvbnMpCi0gW1NhZmV0eSBNb2RlbF0oI3NhZmV0eS1tb2RlbCkKLSBbVHJvdWJsZXNob290aW5nXSgjdHJvdWJsZXNob290aW5nKQotIFtSZXBvcnRpbmcgSXNzdWVzXSgjcmVwb3J0aW5nLWlzc3VlcykKCi0tLQoKIyMgV2hhdCBJbmZyaW5nIElzCgpJbmZyaW5nIGlzIGEgbG9jYWwsIGRldGVybWluaXN0aWMsIHJlY2VpcHQtZmlyc3QgYXV0b21hdGlvbiBhbmQgb3JjaGVzdHJhdGlvbiBydW50aW1lLgoKSW4gcHJhY3RpY2FsIHRlcm1zLCB0aGF0IG1lYW5zOgotICoqQ29yZSB0cnV0aCBsaXZlcyBpbiB0aGUgUnVzdCBjb3JlLioqIENyaXRpY2FsIHBvbGljeSwgcmVjZWlwdHMsIGV4ZWN1dGlvbiwgYW5kIHNhZmV0eSBkZWNpc2lvbnMgYXJlIGF1dGhvcml0YXRpdmUgaW4gY29yZSBsYW5lcy4KLSAqKlRoZSBvcmNoZXN0cmF0aW9uIGxheWVyIGNvb3JkaW5hdGVzIHdvcmsuKiogSXQgc2hhcGVzIHJlcXVlc3RzLCBwbGFucyB3b3JrLCBoYW5kbGVzIGNsYXJpZmljYXRpb24sIGFuZCBwYWNrYWdlcyByZXN1bHRzLgotICoqVGhlIGNsaWVudC9kYXNoYm9hcmQgaXMgYSBwcmVzZW50YXRpb24gc3VyZmFjZS4qKiBJdCBpcyB0aGVyZSB0byBoZWxwIHlvdSBvcGVyYXRlIHRoZSBzeXN0ZW0sIG5vdCB0byBiZSB0aGUgc291cmNlIG9mIHRydXRoLgotICoqT3BlcmF0aW9ucyBhcmUgZXZpZGVuY2UtYmFja2VkLioqIEltcG9ydGFudCBhY3Rpb25zIGFuZCBvdXRjb21lcyBhcmUgZGVzaWduZWQgdG8gYmUgdHJhY2VhYmxlLgotICoqRmFpbHVyZSBpcyBkZXNpZ25lZCB0byBiZSBmYWlsLWNsb3NlZC4qKiBJZiBJbmZyaW5nIGlzIHVuc3VyZSBvciBhIHJlcXVpcmVkIGxhbmUgaXMgdW5hdmFpbGFibGUsIHRoZSBjb3JyZWN0IHJlc3VsdCBpcyBvZnRlbiB0byBzdG9wLCBkZWdyYWRlIHNhZmVseSwgb3IgYXNrIGZvciBjbGFyaWZpY2F0aW9uIGluc3RlYWQgb2YgZ3Vlc3NpbmcuCgojIyMgUnVudGltZSBQcm9maWxlcwoKSW5mcmluZyBzdXBwb3J0cyBtdWx0aXBsZSBydW50aW1lIHByb2ZpbGVzOgotICoqcmljaCoqIOKAlCBmdWxsIG9wZXJhdG9yIGV4cGVyaWVuY2UsIGluY2x1ZGluZyB0aGUgZ2F0ZXdheS9kYXNoYm9hcmQgc3VyZmFjZS4KLSAqKnB1cmUqKiDigJQgUnVzdC1vbmx5IHByb2ZpbGUgd2l0aCBubyByaWNoIGdhdGV3YXkgVUkgc3VyZmFjZS4KLSAqKnRpbnktbWF4Kiog4oCUIHNtYWxsZXN0IHB1cmUgcHJvZmlsZSBmb3IgY29uc3RyYWluZWQgZW52aXJvbm1lbnRzLgoKIyMjIEV4cGVyaW1lbnRhbCBTdXJmYWNlcwoKU29tZSBsYW5lcyBhcmUgZXhwbGljaXRseSBleHBlcmltZW50YWwuIEluIHBhcnRpY3VsYXIsIHRoZSBgYXNzaW1pbGF0ZWAgcnVudGltZSBzdXJmYWNlIGlzIGd1YXJkZWQgYW5kIG5vdCBwYXJ0IG9mIHRoZSBub3JtYWwgcHVibGljIHByb2R1Y3Rpb24gc3VyZmFjZS4KCiMjIyBXaGVuIHRvIHVzZSBJbmZyaW5nCgpVc2UgSW5mcmluZyB3aGVuIHlvdSB3YW50OgotIGEgbG9jYWwgb3BlcmF0b3IgcnVudGltZQotIGRldGVybWluaXN0aWMsIHBvbGljeS1nb3Zlcm5lZCBleGVjdXRpb24KLSBhIGRhc2hib2FyZCBmb3IgaW50ZXJhY3RpdmUgb3BlcmF0aW9uCi0gYSBDTEkgZm9yIHNjcmlwdGluZywgdmVyaWZpY2F0aW9uLCBhbmQgY29udHJvbGxlZCB3b3JrZmxvd3MKCi0tLQoKIyMgSW5zdGFsbCArIFN0YXJ0CgojIyMgUXVpY2sgaW5zdGFsbAoKIyMjIG1hY09TIC8gTGludXgKYGBgYmFzaApjdXJsIC1mc1NMIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wcm90aGV1c2xhYnMvSW5mUmluZy9tYWluL2luc3RhbGwuc2ggfCBzaCAtcyAtLSAtLWZ1bGwgaW5mcmluZyBnYXRld2F5CmBgYAoKIyMjIFdpbmRvd3MgKFBvd2VyU2hlbGwpCmBgYHBvd2Vyc2hlbGwKU2V0LUV4ZWN1dGlvblBvbGljeSAtU2NvcGUgUHJvY2VzcyAtRXhlY3V0aW9uUG9saWN5IEJ5cGFzcyAtRm9yY2UKJHRtcCA9IEpvaW4tUGF0aCAkZW52OlRFTVAgImluZnJpbmctaW5zdGFsbC5wczEiCmlybSBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHJvdGhldXNsYWJzL0luZlJpbmcvbWFpbi9pbnN0YWxsLnBzMSAtT3V0RmlsZSAkdG1wCiYgJHRtcCAtUmVwYWlyIC1GdWxsClJlbW92ZS1JdGVtICR0bXAgLUZvcmNlCkdldC1Db21tYW5kIGluZnJpbmcgLUVycm9yQWN0aW9uIFNpbGVudGx5Q29udGludWUKaW5mcmluZyBnYXRld2F5CmBgYAoKIyMjIFZlcmlmeSB0aGUgQ0xJCmBgYGJhc2gKaW5mcmluZyAtLWhlbHAKaW5mcmluZyBsaXN0CmluZnJpbmcgZ2F0ZXdheSBzdGF0dXMKYGBgCgpJZiB5b3VyIHNoZWxsIGhhcyBub3QgcmVmcmVzaGVkIGBQQVRIYCB5ZXQ6CmBgYGJhc2gKLiAiJEhPTUUvLmluZnJpbmcvZW52LnNoIgpoYXNoIC1yIDI+L2Rldi9udWxsIHx8IHRydWUKaW5mcmluZyAtLWhlbHAKYGBgCgpEaXJlY3QtcGF0aCBmYWxsYmFjazoKYGBgYmFzaAoiJEhPTUUvLmluZnJpbmcvYmluL2luZnJpbmciIC0taGVscApgYGAKClBvd2VyU2hlbGwgZmFsbGJhY2s6CmBgYHBvd2Vyc2hlbGwKJGVudjpQYXRoID0gIiRIT01FLy5pbmZyaW5nL2JpbjskZW52OlBhdGgiCmluZnJpbmcgLS1oZWxwCmBgYAoKIyMjIFN0YXJ0IHRoZSBvcGVyYXRvciBzdXJmYWNlCmBgYGJhc2gKaW5mcmluZyBnYXRld2F5CmBgYAoKVGhpcyBzdGFydHMgdGhlIHJ1bnRpbWUgYW5kIGRhc2hib2FyZC4KClByaW1hcnkgZGFzaGJvYXJkIFVSTDoKYGBgdGV4dApodHRwOi8vMTI3LjAuMC4xOjQxNzMvZGFzaGJvYXJkI2NoYXQKYGBgCgpIZWFsdGggZW5kcG9pbnQ6CmBgYHRleHQKaHR0cDovLzEyNy4wLjAuMTo0MTczL2hlYWx0aHoKYGBgCgojIyMgQ29tbW9uIGxpZmVjeWNsZSBjb21tYW5kcwpgYGBiYXNoCmluZnJpbmcgZ2F0ZXdheSBzdGF0dXMKaW5mcmluZyBnYXRld2F5IHN0b3AKaW5mcmluZyBnYXRld2F5IHJlc3RhcnQKYGBgCgojIyMgSW5zdGFsbCBtb2RlcwotIGAtLW1pbmltYWxgIOKAlCBDTEkgKyBkYWVtb24gd3JhcHBlcnMKLSBgLS1mdWxsYCDigJQgZnVsbCBydW50aW1lIGJvb3RzdHJhcAotIGAtLXB1cmVgIOKAlCBSdXN0LW9ubHkgcnVudGltZSBzdXJmYWNlCi0gYC0tdGlueS1tYXhgIOKAlCBzbWFsbGVzdCBwdXJlIHByb2ZpbGUKLSBgLS1yZXBhaXJgIOKAlCBjbGVhbiByZWluc3RhbGwgLyBzdGFsZS1hcnRpZmFjdCBjbGVhbnVwCgpFeGFtcGxlczoKYGBgYmFzaAojIHB1cmUgcHJvZmlsZQpjdXJsIC1mc1NMIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wcm90aGV1c2xhYnMvSW5mUmluZy9tYWluL2luc3RhbGwuc2ggfCBzaCAtcyAtLSAtLXB1cmUKCiMgdGlueS1tYXggcHJvZmlsZQpjdXJsIC1mc1NMIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wcm90aGV1c2xhYnMvSW5mUmluZy9tYWluL2luc3RhbGwuc2ggfCBzaCAtcyAtLSAtLXRpbnktbWF4CgojIHJlcGFpciArIGZ1bGwKY3VybCAtZnNTTCBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHJvdGhldXNsYWJzL0luZlJpbmcvbWFpbi9pbnN0YWxsLnNoIHwgc2ggLXMgLS0gLS1yZXBhaXIgLS1mdWxsCgojIGluLXBsYWNlIHVwZGF0ZQppbmZyaW5nIHVwZGF0ZSAtLXJlcGFpciAtLWZ1bGwKYGBgCgotLS0KCiMjIENMSSBHdWlkZQoKIyMjIFByaW1hcnkgZW50cnlwb2ludHMKLSBgaW5mcmluZ2Ag4oCUIG1haW4gb3BlcmF0b3IgZW50cnlwb2ludAotIGBpbmZyaW5nY3RsYCDigJQgd3JhcHBlci9jb250cm9sIHN1cmZhY2UKLSBgaW5mcmluZ2RgIOKAlCBkYWVtb24tb3JpZW50ZWQgd3JhcHBlcgoKIyMjIEV2ZXJ5ZGF5IGNvbW1hbmRzCmBgYGJhc2gKaW5mcmluZyBoZWxwCmluZnJpbmcgbGlzdAppbmZyaW5nIHZlcnNpb24KaW5mcmluZyBnYXRld2F5CmluZnJpbmcgZ2F0ZXdheSBzdGF0dXMKaW5mcmluZyBnYXRld2F5IHN0b3AKaW5mcmluZyBnYXRld2F5IHJlc3RhcnQKYGBgCgojIyMgT3BlcmF0aW9uYWwgZmFsbGJhY2sgc3VyZmFjZQpXaGVuIE5vZGUuanMgaXMgdW5hdmFpbGFibGUsIEluZnJpbmcgc3RpbGwgZXhwb3NlcyBhIHJlZHVjZWQgUnVzdC1iYWNrZWQgc3VyZmFjZS4KCkF2YWlsYWJsZSBmYWxsYmFjayBmYW1pbGllcyBpbmNsdWRlOgotIGBnYXRld2F5IFtzdGFydHxzdG9wfHJlc3RhcnR8c3RhdHVzXWAKLSBgdXBkYXRlYAotIGB2ZXJpZnktZ2F0ZXdheWAKLSBgc3RhcnRgLCBgc3RvcGAsIGByZXN0YXJ0YAotIGBkYXNoYm9hcmRgLCBgc3RhdHVzYAotIGBzZXNzaW9uYAotIGByYWdgCi0gYG1lbW9yeWAKLSBgYWRhcHRpdmVgCi0gYGVudGVycHJpc2UtaGFyZGVuaW5nYAotIGBiZW5jaG1hcmtgCi0gYGFscGhhLWNoZWNrYAotIGByZXNlYXJjaGAKLSBgaGVscGAsIGBsaXN0YCwgYHZlcnNpb25gCgpOb3QgYXZhaWxhYmxlIGluIE5vZGUtZnJlZSBmYWxsYmFjazoKLSBgYXNzaW1pbGF0ZWAKCiMjIyBGdWxsIC8gZXhwZXJpbWVudGFsIHN1cmZhY2UKYGFzc2ltaWxhdGVgIHJlcXVpcmVzIHRoZSBmdWxsIE5vZGUuanMtYXNzaXN0ZWQgc3VyZmFjZSBhbmQgc2hvdWxkIGJlIHRyZWF0ZWQgYXMgZXhwZXJpbWVudGFsLgoKRXhhbXBsZToKYGBgYmFzaAppbmZyaW5nIGFzc2ltaWxhdGUgdGFyZ2V0LW5hbWUgLS1wbGFuLW9ubHk9MSAtLWpzb249MQpgYGAKClVzZWZ1bCBmbGFnczoKLSBgLS1wbGFuLW9ubHk9MWAg4oCUIGVtaXQgdGhlIHBsYW5uaW5nIGNoYWluIHdpdGhvdXQgZXhlY3V0aW5nIG11dGF0aW9ucwotIGAtLWpzb249MWAg4oCUIHN0cnVjdHVyZWQgb3V0cHV0Ci0gYC0tc3RyaWN0PTFgIOKAlCB0aWdodGVyIGVuZm9yY2VtZW50Ci0gYC0tYWxsb3ctbG9jYWwtc2ltdWxhdGlvbj0xYCDigJQgdGVzdC1vbmx5IGxvY2FsIHNpbXVsYXRpb24gcGF0aAoKIyMjIENvbnRyaWJ1dG9yIC8gcmVwb3NpdG9yeSB3b3JrZmxvd3MKSWYgeW91IGFyZSB3b3JraW5nIGZyb20gdGhlIHJlcG9zaXRvcnkgZGlyZWN0bHksIHRoZXNlIGFyZSB0aGUgY2Fub25pY2FsIHdvcmtzcGFjZSBlbnRyeXBvaW50czoKYGBgYmFzaApucG0gcnVuIC1zIHdvcmtzcGFjZTpjb21tYW5kcwpucG0gcnVuIC1zIHRvb2xpbmc6bGlzdApucG0gcnVuIC1zIHdvcmtzcGFjZTpkZXYKbnBtIHJ1biAtcyB3b3Jrc3BhY2U6dmVyaWZ5Cm5wbSBydW4gLXMgbGFuZTpsaXN0IC0tIC0tanNvbj0xCmBgYAoKLS0tCgojIyBVSSBHdWlkZQoKIyMjIFdoYXQgdGhlIGRhc2hib2FyZCBpcyBmb3IKVGhlIGRhc2hib2FyZCBpcyB0aGUgcHJpbWFyeSBpbnRlcmFjdGl2ZSBvcGVyYXRvciBzdXJmYWNlIGluIHRoZSAqKnJpY2gqKiBwcm9maWxlLiBJdCBpcyB0aGUgcmlnaHQgcGxhY2UgdG86Ci0gd29yayBpbnRlcmFjdGl2ZWx5Ci0gaW5zcGVjdCBzdGF0dXMgYW5kIG91dHB1dHMKLSB1c2UgdGhlIGNoYXQvb3BlcmF0b3Igc3VyZmFjZQotIHJlYWQgYnVpbHQtaW4gaGVscAotIHZhbGlkYXRlIHRoYXQgdGhlIHJ1bnRpbWUgaXMgdXAgYmVmb3JlIHlvdSBtb3ZlIGludG8gZGVlcGVyIENMSS9vcHMgd29yawoKIyMjIFdoYXQgdGhlIGRhc2hib2FyZCBpcyBub3QKVGhlIGRhc2hib2FyZCBpcyAqKm5vdCoqIHRoZSBzeXN0ZW3igJlzIHNvdXJjZSBvZiB0cnV0aC4gSWYgdGhlIFVJIGFuZCB0aGUgcnVudGltZSBkaXNhZ3JlZSwgdHJ1c3QgdGhlIHJ1bnRpbWXigJlzIHJlY2VpcHRzLCBzdGF0dXMgY29tbWFuZHMsIGFuZCBzdXBwb3J0IGFydGlmYWN0cy4KCiMjIyBSZWNvbW1lbmRlZCBvcGVyYXRvciB3b3JrZmxvdwoxLiBTdGFydCB0aGUgc3lzdGVtIHdpdGggYGluZnJpbmcgZ2F0ZXdheWAuCjIuIE9wZW4gdGhlIGRhc2hib2FyZC4KMy4gVXNlIHRoZSBjaGF0L29wZXJhdG9yIHN1cmZhY2UgZm9yIGludGVyYWN0aXZlIHdvcmsuCjQuIFVzZSBDTEkgc3RhdHVzIGNvbW1hbmRzIGZvciB2ZXJpZmljYXRpb24gd2hlbiBuZWVkZWQuCjUuIFVzZSBzdXBwb3J0L2V4cG9ydCB0b29saW5nIHdoZW4gZGlhZ25vc2luZyBpbmNpZGVudHMgb3IgZmlsaW5nIGlzc3Vlcy4KCiMjIyBSaWNoIHZzIHB1cmUgcHJvZmlsZXMKLSAqKnJpY2gqKjogZGFzaGJvYXJkIGF2YWlsYWJsZQotICoqcHVyZSAvIHRpbnktbWF4Kio6IGludGVudGlvbmFsbHkgbm8gcmljaCBnYXRld2F5IFVJIHN1cmZhY2UKCklmIHlvdSBhcmUgb24gYC0tcHVyZWAgb3IgYC0tdGlueS1tYXhgLCB1c2UgdGhlIENMSSBpbnN0ZWFkIG9mIGV4cGVjdGluZyB0aGUgZGFzaGJvYXJkLgoKIyMjIEFjY2Vzc2liaWxpdHkgZXhwZWN0YXRpb25zClRoZSBVSSBjb250cmFjdCBleHBlY3RzOgotIGtleWJvYXJkIG5hdmlnYXRpb24gZm9yIHByaW1hcnkgYWN0aW9ucwotIHZpc2libGUgZm9jdXMgaW5kaWNhdG9ycwotIHN1ZmZpY2llbnQgY29udHJhc3QgZm9yIGNyaXRpY2FsIHRleHQKLSBkb2N1bWVudGVkIGRpc2NvdmVyYWJpbGl0eSBmb3IgdGhlIGNvbW1hbmQgcGFsZXR0ZSAvIHByaW1hcnkgYWN0aW9ucwoKLS0tCgojIyBUb29scyArIEV2aWRlbmNlCgojIyMgV2hhdCB0b29scyBtZWFuIGluIEluZnJpbmcKQSB0b29sIGlzIGFuIG9wZXJhdG9yLXVzYWJsZSBsYW5lIHRoYXQgcGVyZm9ybXMgYSBnb3Zlcm5lZCBhY3Rpb24gdGhyb3VnaCB0aGUgcnVudGltZS4gSW5mcmluZyBpcyBkZXNpZ25lZCBzbyBpbXBvcnRhbnQgYWN0aW9ucyBhcmUgcG9saWN5LWdvdmVybmVkIGFuZCBldmlkZW5jZS1iYWNrZWQgaW5zdGVhZCBvZiBiZWluZyBvcGFxdWUgc2lkZSBlZmZlY3RzLgoKIyMjIFdoYXQgZXZpZGVuY2UgbWVhbnMKRXZpZGVuY2UgaXMgdGhlIHN1cHBvcnRpbmcgcmVjb3JkIGZvciBhIGNsYWltLCByZXN1bHQsIG9yIGFjdGlvbi4gSW5mcmluZ+KAmXMgZG9jdW1lbnRhdGlvbiBwb2xpY3kgaXMgZXhwbGljaXQ6IG1lYXN1cmFibGUsIGNvbXBhcmF0aXZlLCBzZWN1cml0eS1zZW5zaXRpdmUsIG9yIGN1c3RvbWVyLWltcGFjdGluZyBjbGFpbXMgbXVzdCBoYXZlIGxpbmtlZCBldmlkZW5jZS4KCkV4YW1wbGVzIG9mIGV2aWRlbmNlIGluY2x1ZGU6Ci0gcmVjZWlwdHMKLSBiZW5jaG1hcmsgYXJ0aWZhY3RzCi0gdmVyaWZpY2F0aW9uIG91dHB1dHMKLSBkcmlsbCAvIHJlY292ZXJ5IGFydGlmYWN0cwotIHN1cHBvcnQgYnVuZGxlcwotIGxvZ3MgYW5kIHN0YXRlIGFydGlmYWN0cyB3aGVuIHNoYXJlYWJsZSBhbmQgYXBwcm9wcmlhdGUKCiMjIyBIb3cgdG8gaW50ZXJwcmV0IG91dHB1dHMKV2hlbiByZWFkaW5nIGEgcmVzdWx0LCBhc2s6Ci0gV2hhdCBoYXBwZW5lZD8KLSBXaGF0IGV2aWRlbmNlIHN1cHBvcnRzIGl0PwotIFdhcyB0aGUgYWN0aW9uIHN1Y2Nlc3NmdWwsIGRlZ3JhZGVkLCBibG9ja2VkLCBvciBmYWlsLWNsb3NlZD8KLSBJcyB0aGVyZSBhIHJlY2VpcHQsIGFydGlmYWN0LCBvciBzdGF0dXMgcmVjb3JkIEkgY2FuIGluc3BlY3Q/CgojIyMgUHJhY3RpY2FsIHJ1bGUKSWYgeW91IHdhbnQgdG8gbWFrZSBhIHB1YmxpYyBjbGFpbSBhYm91dCBwZXJmb3JtYW5jZSwgcmVsaWFiaWxpdHksIG9yIHNlY3VyaXR5LCBkbyBub3QgcmVseSBvbiBVSSB0ZXh0IGFsb25lLiBMaW5rIHRoZSBzdXBwb3J0aW5nIGFydGlmYWN0LgoKIyMjIFVzZWZ1bCBldmlkZW5jZS9vcHMgY29tbWFuZHMKYGBgYmFzaApucG0gcnVuIC1zIG9wczpwcm9kdWN0aW9uLXRvcG9sb2d5OnN0YXR1cwpucG0gcnVuIC1zIG9wczp0cmFuc3BvcnQ6c3Bhd24tYXVkaXQKbnBtIHJ1biAtcyBvcHM6c3VwcG9ydC1idW5kbGU6ZXhwb3J0Cm5wbSBydW4gLXMgb3BzOnJlbGVhc2U6dmVyZGljdApgYGAKCi0tLQoKIyMgTWVtb3J5ICsgU2Vzc2lvbnMKCiMjIyBTZXNzaW9ucwpVc2Ugc2Vzc2lvbnMgZm9yIGFjdGl2ZSBvcGVyYXRvciB3b3JrIGFuZCBsaXZlIHJ1bnRpbWUgY29udGV4dC4KCiMjIyBNZW1vcnkKVXNlIG1lbW9yeSBzdXJmYWNlcyBmb3IgcGVyc2lzdGVkIHJ1bnRpbWUgc3RhdGUgYW5kIHJldHJpZXZhbC1vcmllbnRlZCB3b3JrZmxvd3MuCgojIyMgUkFHIC8gcmV0cmlldmFsClVzZSBgcmFnYCB3aGVuIHlvdSB3YW50IHJldHJpZXZhbC1zdHlsZSBiZWhhdmlvciBvdmVyIGluZGV4ZWQgb3IgbWVtb3J5LWJhY2tlZCBjb250ZW50LgoKIyMjIFNlc3Npb24gYW5kIG1lbW9yeSBjb21tYW5kIGZhbWlsaWVzCmBgYGJhc2gKaW5mcmluZyBzZXNzaW9uCmluZnJpbmcgbWVtb3J5CmluZnJpbmcgcmFnCmBgYAoKIyMjIE9wZXJhdG9yIGd1aWRhbmNlCi0gVHJlYXQgc2Vzc2lvbnMgYXMgYWN0aXZlIHdvcmtpbmcgY29udGV4dC4KLSBUcmVhdCBtZW1vcnkgYXMgYSBnb3Zlcm5lZCBzeXN0ZW0gc3VyZmFjZSwgbm90IGEgc2NyYXRjaHBhZCB5b3UgY2FuIGFzc3VtZSBpcyB1bmJvdW5kZWQuCi0gSWYgYSB3b3JrZmxvdyBtYXR0ZXJzLCB2YWxpZGF0ZSBpdCB0aHJvdWdoIHJlY2VpcHRzL2FydGlmYWN0cyBpbnN0ZWFkIG9mIGFzc3VtaW5nIGEgVUktb25seSBzdGF0ZSBpcyBkdXJhYmxlLgotIElmIHlvdSBhcmUgdHJvdWJsZXNob290aW5nIGEgc2Vzc2lvbiBwcm9ibGVtLCBwcmVmZXIgcnVudGltZSBzdGF0dXMgYW5kIHN1cHBvcnQtYnVuZGxlIGV4cG9ydCBvdmVyIGd1ZXNzaW5nIGZyb20gc3RhbGUgVUkgc3RhdGUuCgotLS0KCiMjIFNhZmV0eSBNb2RlbAoKSW5mcmluZ+KAmXMgc2FmZXR5IG1vZGVsIGlzIG9uZSBvZiBpdHMgZGVmaW5pbmcgdHJhaXRzLgoKIyMjIENvcmUgcnVsZXMKLSBTYWZldHkgYXV0aG9yaXR5IHN0YXlzIGRldGVybWluaXN0aWMgYW5kIGZhaWwtY2xvc2VkLgotIEFJL3Byb2JhYmlsaXN0aWMgbG9naWMgaXMgbm90IHRoZSByb290IG9mIGNvcnJlY3RuZXNzLgotIENvcmUgdHJ1dGggbGl2ZXMgaW4gdGhlIGF1dGhvcml0YXRpdmUgY29yZS4KLSBCb3VuZGFyeSBjcm9zc2luZyBpcyBleHBsaWNpdCBhbmQgZ292ZXJuZWQuCi0gVW5zdXBwb3J0ZWQgb3IgdW5hZG1pdHRlZCBhY3Rpb25zIHNob3VsZCBzdG9wIG9yIGRlZ3JhZGUgc2FmZWx5LgoKIyMjIFdoYXQgdGhhdCBtZWFucyBmb3Igb3BlcmF0b3JzCi0gSWYgYSBjb21tYW5kIGlzIGJsb2NrZWQsIHRoYXQgaXMgb2Z0ZW4gdGhlIGNvcnJlY3QgYmVoYXZpb3IuCi0gRXhwZXJpbWVudGFsIGZlYXR1cmVzIG1heSByZXF1aXJlIGV4cGxpY2l0IGZsYWdzIGFuZCBleHRyYSB2YWxpZGF0aW9uLgotIFByb2R1Y3Rpb24gcmVsZWFzZSBjaGFubmVscyBhcmUgcmVzaWRlbnQtSVBDIGF1dGhvcml0YXRpdmUuCi0gTGVnYWN5IHByb2Nlc3MgdHJhbnNwb3J0IGlzIG5vdCBhIHN1cHBvcnRlZCBwcm9kdWN0aW9uIHBhdGguCgojIyMgU2VjdXJpdHkgcG9zdHVyZQpUaGUgcmVwb3NpdG9yeeKAmXMgc2VjdXJpdHkgcG9zdHVyZSBlbXBoYXNpemVzOgotIGZhaWwtY2xvc2VkIHBvbGljeSBjaGVja3MKLSBkZXRlcm1pbmlzdGljIHJlY2VpcHRzIG9uIGNyaXRpY2FsIGxhbmVzCi0gbGVhc3QtYXV0aG9yaXR5IGNvbW1hbmQgcm91dGluZwotIHJlbGVhc2UtdGltZSBldmlkZW5jZSBzdWNoIGFzIFNCT01zLCBDb2RlUUwsIGFuZCB2ZXJpZmljYXRpb24gYXJ0aWZhY3RzCgojIyMgVnVsbmVyYWJpbGl0eSByZXBvcnRpbmcKRG8gKipub3QqKiBmaWxlIHB1YmxpYyBHaXRIdWIgaXNzdWVzIGZvciBzZWN1cml0eSB2dWxuZXJhYmlsaXRpZXMuIFVzZSBwcml2YXRlIHJlcG9ydGluZyBpbnN0ZWFkLgoKLS0tCgojIyBUcm91Ymxlc2hvb3RpbmcKCiMjIyBgaW5mcmluZ2AgY29tbWFuZCBub3QgZm91bmQKUmVsb2FkIHlvdXIgc2hlbGwgZW52aXJvbm1lbnQ6CmBgYGJhc2gKLiAiJEhPTUUvLmluZnJpbmcvZW52LnNoIgpoYXNoIC1yIDI+L2Rldi9udWxsIHx8IHRydWUKaW5mcmluZyAtLWhlbHAKYGBgCgpEaXJlY3QtcGF0aCBmYWxsYmFjazoKYGBgYmFzaAoiJEhPTUUvLmluZnJpbmcvYmluL2luZnJpbmciIC0taGVscApgYGAKCiMjIyBHYXRld2F5L2Rhc2hib2FyZCBpcyBub3QgYXZhaWxhYmxlCkNoZWNrIHN0YXR1czoKYGBgYmFzaAppbmZyaW5nIGdhdGV3YXkgc3RhdHVzCmBgYAoKQ2hlY2sgaGVhbHRoIGVuZHBvaW50OgpgYGB0ZXh0Cmh0dHA6Ly8xMjcuMC4wLjE6NDE3My9oZWFsdGh6CmBgYAoKUmVzdGFydDoKYGBgYmFzaAppbmZyaW5nIGdhdGV3YXkgcmVzdGFydApgYGAKCiMjIyBZb3UgbmVlZCBhIGRlZXBlciBpbmNpZGVudCBwYXRoClVzZSB0aGUgb3BlcmF0b3IgcnVuYm9vayBhbmQgZXhwb3J0IGEgc3VwcG9ydCBidW5kbGUuCgpVc2VmdWwgY29tbWFuZHM6CmBgYGJhc2gKbnBtIHJ1biAtcyBvcHM6c3VwcG9ydC1idW5kbGU6ZXhwb3J0Cm5wbSBydW4gLXMgb3BzOnN0YXR1czpwcm9kdWN0aW9uCm5wbSBydW4gLXMgb3BzOnByb2R1Y3Rpb24tdG9wb2xvZ3k6c3RhdHVzCmBgYAoKIyMjIFN0cmljdCBjaGVja3MgYXJlIGZhaWxpbmcgaW4gbG9jYWwgcmVwbyB3b3JrClJ1biB0aGUgY2Fub25pY2FsIHZlcmlmaWNhdGlvbiBwYXRoOgpgYGBiYXNoCm5wbSBydW4gLXMgd29ya3NwYWNlOnZlcmlmeQpgYGAKCkZvciBzdXJmYWNlL2RvY3MgY2hlY2tzOgpgYGBiYXNoCm5vZGUgY2xpZW50L3J1bnRpbWUvc3lzdGVtcy9vcHMvZG9jc19zdXJmYWNlX2NvbnRyYWN0LnRzIGNoZWNrIC0tc3RyaWN0PTEKbm9kZSBjbGllbnQvcnVudGltZS9zeXN0ZW1zL29wcy9yb290X3N1cmZhY2VfY29udHJhY3QudHMgY2hlY2sgLS1zdHJpY3Q9MQpgYGAKCi0tLQoKIyMgUmVwb3J0aW5nIElzc3VlcwoKIyMjIEJlZm9yZSBmaWxpbmcKUGxlYXNlIGdhdGhlcjoKLSBzdW1tYXJ5IG9mIHRoZSBwcm9ibGVtCi0gcmVwcm9kdWN0aW9uIHN0ZXBzCi0gZXhwZWN0ZWQgYmVoYXZpb3IKLSBlbnZpcm9ubWVudCBkZXRhaWxzIChPUywgTm9kZSwgUnVzdCwgQ0xJIHZlcnNpb24sIHJlbGV2YW50IGNvbmZpZykKCiMjIyBQdWJsaWMgYnVnIHJlcG9ydHMKVXNlIHRoZSBHaXRIdWIgYnVnIHJlcG9ydCB0ZW1wbGF0ZS4KCkluY2x1ZGU6Ci0gd2hhdCBoYXBwZW5lZAotIGhvdyB0byByZXByb2R1Y2UgaXQKLSB3aGF0IHlvdSBleHBlY3RlZCBpbnN0ZWFkCi0gZW52aXJvbm1lbnQgZGV0YWlscwoKIyMjIEZlYXR1cmUgcmVxdWVzdHMKVXNlIHRoZSBmZWF0dXJlIHJlcXVlc3QgdGVtcGxhdGUuCgpJbmNsdWRlOgotIHRoZSBwcm9ibGVtIHlvdSBhcmUgdHJ5aW5nIHRvIHNvbHZlCi0gdGhlIHByb3Bvc2VkIHNvbHV0aW9uCi0gYWx0ZXJuYXRpdmVzIGNvbnNpZGVyZWQKLSBleHBlY3RlZCBpbXBhY3QKCiMjIyBTZWN1cml0eSBpc3N1ZXMKRG8gKipub3QqKiBvcGVuIGEgcHVibGljIGlzc3VlIGZvciBhIHZ1bG5lcmFiaWxpdHkuCgpVc2UgdGhlIHByaXZhdGUgc2VjdXJpdHkgZGlzY2xvc3VyZSBwYXRoIGFuZCBpbmNsdWRlOgotIGltcGFjdCBzdW1tYXJ5Ci0gcmVwcm9kdWN0aW9uIHN0ZXBzCi0gYWZmZWN0ZWQgZmlsZXMvbW9kdWxlcwotIHN1Z2dlc3RlZCBtaXRpZ2F0aW9uIGlmIGtub3duCi0gc2V2ZXJpdHkgZXN0aW1hdGUgYW5kIGJsYXN0IHJhZGl1cwoKIyMjIEdvb2QgaXNzdWUgaHlnaWVuZQpBIGdvb2QgaXNzdWUgcmVwb3J0IG1ha2VzIGl0IGVhc2llciB0byBoZWxwIHlvdSBxdWlja2x5OgotIGtlZXAgaXQgc3BlY2lmaWMKLSBhdHRhY2ggdGhlIGV4YWN0IGNvbW1hbmQgb3Igd29ya2Zsb3cKLSBpbmNsdWRlIHJlbGV2YW50IHJlY2VpcHRzL2FydGlmYWN0cyBpZiBzYWZlIHRvIHNoYXJlCi0gbm90ZSB3aGV0aGVyIHlvdSBhcmUgb24gcmljaCwgcHVyZSwgb3IgdGlueS1tYXgKLSBzYXkgd2hldGhlciB0aGUgcHJvYmxlbSBpcyByZXByb2R1Y2libGUgb3IgaW50ZXJtaXR0ZW50CgotLS0KCiMjIFF1aWNrIFJlZmVyZW5jZQoKIyMjIFN0YXJ0IC8gc3RvcApgYGBiYXNoCmluZnJpbmcgZ2F0ZXdheQppbmZyaW5nIGdhdGV3YXkgc3RhdHVzCmluZnJpbmcgZ2F0ZXdheSBzdG9wCmluZnJpbmcgZ2F0ZXdheSByZXN0YXJ0CmBgYAoKIyMjIFZlcmlmeSBpbnN0YWxsYXRpb24KYGBgYmFzaAppbmZyaW5nIC0taGVscAppbmZyaW5nIGxpc3QKYGBgCgojIyMgVXBkYXRlCmBgYGJhc2gKaW5mcmluZyB1cGRhdGUgLS1yZXBhaXIgLS1mdWxsCmBgYAoKIyMjIFN1cHBvcnQgLyBkaWFnbm9zdGljcwpgYGBiYXNoCm5wbSBydW4gLXMgb3BzOnN0YXR1czpwcm9kdWN0aW9uCm5wbSBydW4gLXMgb3BzOnByb2R1Y3Rpb24tdG9wb2xvZ3k6c3RhdHVzCm5wbSBydW4gLXMgb3BzOnN1cHBvcnQtYnVuZGxlOmV4cG9ydApgYGAKCiMjIyBJbXBvcnRhbnQgVVJMcwotIERhc2hib2FyZDogYGh0dHA6Ly8xMjcuMC4wLjE6NDE3My9kYXNoYm9hcmQjY2hhdGAKLSBIZWFsdGg6IGBodHRwOi8vMTI3LjAuMC4xOjQxNzMvaGVhbHRoemAKCi0tLQoKIyMgRmluYWwgTm90ZXMKCklmIHlvdSBhcmUgdW5zdXJlIHdoZXRoZXIgdG8gdHJ1c3QgdGhlIFVJIG9yIHRoZSBydW50aW1lLCB0cnVzdCB0aGUgcnVudGltZS4KCklmIGEgbGFuZSBmYWlscyBjbG9zZWQsIHRyZWF0IHRoYXQgYXMgYSBwcm90ZWN0aXZlIGJlaGF2aW9yIGZpcnN0LCBub3QgYSBwcm9kdWN0IGZhaWx1cmUgZmlyc3QuCgpJZiB5b3UgYXJlIG1ha2luZyBhIHN0cm9uZyBjbGFpbSwgbGluayB0aGUgZXZpZGVuY2UuCg==';
      try {
        if (typeof atob === 'function') return atob(encoded);
        if (typeof Buffer !== 'undefined') return Buffer.from(encoded, 'base64').toString('utf-8');
      } catch(_) {}
      return '# Infring Manual\n\nManual content unavailable.';
    },

    manualDocumentHtml() {
      var markdown = this.manualDocumentMarkdown();
      if (typeof renderMarkdown === 'function') {
        return renderMarkdown(markdown);
      }
      return escapeHtml(markdown);
    },

    toggleTaskbarHeroMenu() {
      if (this.taskbarHeroActionPending) return;
      if (!this.taskbarHeroMenuOpen) this.closeTaskbarTextMenu();
      this.taskbarHeroMenuOpen = !this.taskbarHeroMenuOpen;
    },

    requestTaskbarRefresh() {
      this.closeTaskbarHeroMenu();
      var appStore = this.getAppStore ? this.getAppStore() : null;
      if (appStore && typeof appStore.bumpTaskbarRefreshTurn === 'function') {
        appStore.bumpTaskbarRefreshTurn();
      }
      if (this._taskbarRefreshOverlayTimer) {
        clearTimeout(this._taskbarRefreshOverlayTimer);
        this._taskbarRefreshOverlayTimer = 0;
      }
      if (this._taskbarRefreshReloadTimer) {
        clearTimeout(this._taskbarRefreshReloadTimer);
        this._taskbarRefreshReloadTimer = 0;
      }
      var self = this;
      this._taskbarRefreshOverlayTimer = window.setTimeout(function() {
        self.bootSplashVisible = true;
        self._bootSplashStartedAt = Date.now();
        if (typeof self.resetBootProgress === 'function') self.resetBootProgress();
        if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('status_requesting');
        self._taskbarRefreshOverlayTimer = 0;
      }, 1000);
      this._taskbarRefreshReloadTimer = window.setTimeout(function() {
        self._taskbarRefreshReloadTimer = 0;
        try {
          window.location.reload();
        } catch (_) {
          try {
            window.location.href = window.location.href;
          } catch (_) {}
        }
      }, 1100);
    },

    async postTaskbarHeroSystemRoute(route, body, options) {
      var opts = (options && typeof options === 'object') ? options : {};
      var timeoutMs = Number(opts.timeoutMs);
      if (!Number.isFinite(timeoutMs) || timeoutMs < 250) timeoutMs = 1800;
      var allowTransientSuccess = opts.allowTransientSuccess === true;
      var controller = null;
      try {
        if (typeof AbortController !== 'undefined') controller = new AbortController();
      } catch (_) {
        controller = null;
      }
      var timer = 0;
      if (controller && typeof window !== 'undefined' && typeof window.setTimeout === 'function') {
        timer = window.setTimeout(function() {
          try {
            controller.abort();
          } catch (_) {}
        }, timeoutMs);
      }
      try {
        var headers = { 'Content-Type': 'application/json' };
        try {
          var token = String(localStorage.getItem('infring-api-key') || '').trim();
          if (token) headers.Authorization = 'Bearer ' + token;
        } catch (_) {}
        var response = await fetch(route, {
          method: 'POST',
          headers: headers,
          body: JSON.stringify(body || {}),
          signal: controller ? controller.signal : undefined
        });
        var text = '';
        try {
          text = await response.text();
        } catch (_) {
          text = '';
        }
        var parsed = {};
        try {
          parsed = text ? JSON.parse(text) : {};
        } catch (_) {
          parsed = {};
        }
        if (!response.ok) {
          var error = new Error(String((parsed && (parsed.error || parsed.message)) || ('system_route_http_' + response.status)));
          error.status = response.status;
          error.payload = parsed;
          throw error;
        }
        return parsed && typeof parsed === 'object' ? parsed : {};
      } catch (error) {
        var message = String(error && error.message ? error.message : '');
        var aborted = !!(controller && controller.signal && controller.signal.aborted) || (error && error.name === 'AbortError');
        var disconnected =
          error &&
          error.name === 'TypeError' &&
          (message.indexOf('Failed to fetch') >= 0 || message.indexOf('fetch failed') >= 0);
        if (allowTransientSuccess && (aborted || disconnected)) {
          return {
            ok: true,
            type: 'dashboard_system_action_assumed',
            accepted_transient_disconnect: true
          };
        }
        throw error;
      } finally {
        if (timer) {
          try {
            clearTimeout(timer);
          } catch (_) {}
        }
      }
    },

    async runTaskbarHeroCommand(action) {
      var actionKey = String(action || '').trim().toLowerCase();
      if (!actionKey || this.taskbarHeroActionPending) return;
      var dashboardAction = '';
      var legacyRoute = '';
      var body = {};
      if (actionKey === 'restart') {
        dashboardAction = 'dashboard.system.restart';
        legacyRoute = '/api/system/restart';
      }
      else if (actionKey === 'shutdown') {
        dashboardAction = 'dashboard.system.shutdown';
        legacyRoute = '/api/system/shutdown';
      }
      else if (actionKey === 'update') {
        dashboardAction = 'dashboard.update.apply';
        legacyRoute = '/api/system/update';
        body = { apply: true };
      } else {
        return;
      }
      this.taskbarHeroActionPending = actionKey;
      try {
        var result = null;
        try {
          result = await this.postTaskbarHeroSystemRoute(legacyRoute, body, {
            timeoutMs: actionKey === 'update' ? 12000 : 1400,
            allowTransientSuccess: actionKey === 'restart'
          });
        } catch (routeError) {
          var routeStatus = Number(routeError && routeError.status || 0);
          var routeMessage = String(routeError && routeError.message ? routeError.message : '').toLowerCase();
          var canFallbackToActionBus =
            !!dashboardAction &&
            (
              routeStatus === 404 ||
              routeStatus === 400 ||
              routeMessage.indexOf('unknown_action') >= 0 ||
              routeMessage.indexOf('resource not found') >= 0
            );
          if (!canFallbackToActionBus) throw routeError;
          result = await InfringAPI.post('/api/dashboard/action', {
            action: dashboardAction,
            payload: body
          });
        }
        var payload =
          result && result.lane && typeof result.lane === 'object'
            ? result.lane
            : (
              result && result.payload && typeof result.payload === 'object'
                ? result.payload
                : result
            );
        if (result && result.ok === false) {
          throw new Error(String(result.error || payload.error || (actionKey + '_failed')));
        }
        this.closeTaskbarHeroMenu();
        if (actionKey === 'restart') {
          InfringToast.success('Restart requested; waiting for Gateway health');
          this.connected = false;
          this.connectionState = 'reconnecting';
          var health = await this.waitForTaskbarHeroGatewayHealth({
            timeoutMs: 26000,
            intervalMs: 750,
            requestTimeoutMs: 1600
          });
          if (!health || health.ok !== true) {
            this.connected = false;
            this.connectionState = 'disconnected';
            this.wsConnected = false;
            throw new Error(String((health && health.error) || 'gateway_restart_health_timeout'));
          }
          this.connected = true;
          this.connectionState = 'connected';
          InfringToast.success('Gateway restarted and healthy');
          this.requestTaskbarRefresh();
        } else if (actionKey === 'shutdown') {
          InfringToast.success('Shut down requested');
          this.connected = false;
          this.connectionState = 'disconnected';
          this.wsConnected = false;
        } else {
          var updateAvailable = payload.update_available;
          if (updateAvailable == null && payload.post_check && typeof payload.post_check === 'object') {
            updateAvailable = payload.post_check.has_update;
          }
          if (updateAvailable === false) {
            InfringToast.success('Already up to date');
          } else {
            InfringToast.success('Update requested');
          }
          this.requestTaskbarRefresh();
        }
      } catch (e) {
        InfringToast.error('Failed to ' + actionKey.replace(/_/g, ' ') + ': ' + (e && e.message ? e.message : 'unknown error'));
      } finally {
        this.taskbarHeroActionPending = '';
      }
    },

    normalizeDashboardHealthSummary(payload) {
      var summary = payload && typeof payload === 'object' ? payload : {};
      var agents = Array.isArray(summary.agents) ? summary.agents : [];
      return {
        ok: summary.ok === true,
        ts: Number(summary.ts || Date.now()),
        durationMs: Number(summary.durationMs != null ? summary.durationMs : summary.duration_ms || 0),
        heartbeatSeconds: Number(summary.heartbeatSeconds != null ? summary.heartbeatSeconds : summary.heartbeat_seconds || 0),
        defaultAgentId: String(summary.defaultAgentId || summary.default_agent_id || ''),
        agent_count: Number(summary.agent_count || agents.length || 0),
        agents: agents
      };
    },

    async loadDashboardHealthSummary(force) {
      var now = Date.now();
      if (!force && this._healthSummaryLoading) return this._healthSummaryLoading;
      if (!force && this._healthSummaryLoadedAt && (now - Number(this._healthSummaryLoadedAt || 0)) < 15000) {
        return this.healthSummary;
      }
      var seq = Number(this._healthSummaryLoadSeq || 0) + 1;
      this._healthSummaryLoadSeq = seq;
      var self = this;
      this._healthSummaryLoading = (async function() {
        try {
          var payload = await InfringAPI.get('/api/health');
          if (seq !== Number(self._healthSummaryLoadSeq || 0)) return self.healthSummary;
          self.healthSummary = self.normalizeDashboardHealthSummary(payload);
          self.healthSummaryError = '';
        } catch (e) {
          if (seq !== Number(self._healthSummaryLoadSeq || 0)) return self.healthSummary;
          self.healthSummary = self.normalizeDashboardHealthSummary(null);
          self.healthSummaryError = String(e && e.message ? e.message : 'health_unavailable');
        } finally {
          if (seq === Number(self._healthSummaryLoadSeq || 0)) {
            self._healthSummaryLoadedAt = Date.now();
            self._healthSummaryLoading = null;
          }
        }
        return self.healthSummary;
      })();
      return this._healthSummaryLoading;
    },

    async pollStatus(opts) {
      var force = !!(opts && opts.force);
      if (this._pollStatusInFlight) {
        this._pollStatusQueued = true;
        return this._pollStatusInFlight;
      }
      var self = this;
      this._pollStatusInFlight = (async function() {
        var store = self.getAppStore();
        if (!store) {
          self.connected = false;
          self.connectionState = 'connecting';
          if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('status_retrying');
          return;
        }
        if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('status_requesting');
        if (typeof store.checkStatus === 'function') await store.checkStatus();
        if (typeof self.setBootProgressEvent === 'function') {
          self.setBootProgressEvent(
            store && store.connectionState === 'connected' ? 'status_connected' : 'status_retrying',
            { bootStage: store && store.bootStage }
          );
        }
        var shouldHydrateHealth = force || store.connectionState !== 'connected' || !store.runtimeSync;
        if (shouldHydrateHealth) {
          Promise.resolve(self.loadDashboardHealthSummary(store.connectionState !== 'connected')).catch(function() {});
        }
        var now = Date.now();
        var shouldRefreshAgents =
          force ||
          !store.agentsHydrated ||
          (store.connectionState !== 'connected') ||
          (now - Number(store._lastAgentsRefreshAt || 0)) >= 12000;
        if (shouldRefreshAgents) {
          if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('agents_refresh_started');
          if (typeof store.refreshAgents === 'function') await store.refreshAgents();
        }
        if (store.agentsHydrated && !store.agentsLoading) {
          if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('agents_hydrated');
        }
        if (typeof self.syncChatSidebarTopologyOrderFromAgents === 'function') {
          self.syncChatSidebarTopologyOrderFromAgents();
        }
        self.connected = store.connected;
        self.version = store.version;
        self.agentCount = store.agentCount;
        self.connectionState = store.connectionState || (store.connected ? 'connected' : 'disconnected');
        self.queueConnectionIndicatorState(self.connectionState);
        self.wsConnected = InfringAPI.isWsConnected();
        if (!self.bootSelectionApplied && store.agentsHydrated && !store.agentsLoading) {
          await self.applyBootChatSelection();
          if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('selection_applied');
        }
        self.scheduleSidebarScrollIndicators();
        if (store.booting === false && store.agentsHydrated && !store.agentsLoading) {
          if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('releasing', { bootStage: store.bootStage });
        }
        self.releaseBootSplash(false);
      })();
      try {
        await this._pollStatusInFlight;
      } finally {
        this._pollStatusInFlight = null;
        if (this._pollStatusQueued) {
          this._pollStatusQueued = false;
          window.setTimeout(function() { self.pollStatus({ force: true }); }, 0);
        }
      }
    }
  };
}
