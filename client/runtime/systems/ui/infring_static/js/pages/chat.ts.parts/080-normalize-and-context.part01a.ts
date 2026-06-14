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
      if (typeof this.normalizeMessageThoughtTools === 'function') {
        try { this.normalizeMessageThoughtTools(msg, { commit: true }); } catch (_) {}
      }
      msg._finish_bounce = true;
      setTimeout(function() {
        try { msg._finish_bounce = false; } catch(_) {}
      }, 300);
    },
