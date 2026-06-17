    setBottomDockHover(id, ev) {
      if (String(this.bottomDockDragId || '').trim()) return;
      if (this.bottomDockContainerDragActive || this._bottomDockContainerPointerActive) return;
      var key = String(id || '').trim();
      this.bottomDockHoverId = key;
      if (ev) {
        var evX = Number(ev.clientX || 0);
        var evY = Number(ev.clientY || 0);
        if (Number.isFinite(evX) && evX > 0) this.bottomDockPointerX = evX;
        if (Number.isFinite(evY) && evY > 0) this.bottomDockPointerY = evY;
      }
      if (this._bottomDockPreviewHideTimer) {
        try { clearTimeout(this._bottomDockPreviewHideTimer); } catch(_) {}
        this._bottomDockPreviewHideTimer = 0;
      }
      if (!Number.isFinite(this.bottomDockPointerX) || this.bottomDockPointerX <= 0) {
        try {
          var slot = document.querySelector('.bottom-dock [data-dock-id="' + key + '"], .bottom-dock .dock-tile-slot[data-dock-slot-id="' + key + '"]');
          if (slot && typeof slot.getBoundingClientRect === 'function') {
            var slotRect = slot.getBoundingClientRect();
            this.bottomDockPointerX = Number(slotRect.left || 0) + (Number(slotRect.width || 0) / 2);
            this.bottomDockPointerY = Number(slotRect.top || 0) + (Number(slotRect.height || 0) / 2);
          }
        } catch(_) {}
      }
      this.refreshBottomDockHoverWeights();
      this.syncBottomDockPreview();
      this.scheduleBottomDockPreviewReflow();
    },

    clearBottomDockHover(id) {
      if (id) return;
      this.bottomDockHoverId = '';
      if (!this.bottomDockHoverId) {
        this.bottomDockHoverWeightById = {};
        this.bottomDockPointerX = 0;
        this.bottomDockPointerY = 0;
        this.cancelBottomDockPreviewReflow();
        var self = this;
        if (this._bottomDockPreviewHideTimer) {
          try { clearTimeout(this._bottomDockPreviewHideTimer); } catch(_) {}
        }
        this._bottomDockPreviewHideTimer = window.setTimeout(function() {
          self._bottomDockPreviewHideTimer = 0;
          if (!String(self.bottomDockHoverId || '').trim()) {
            self.bottomDockPreviewVisible = false;
            self.bottomDockPreviewText = '';
            self.bottomDockPreviewMorphFromText = '';
            self.bottomDockPreviewLabelMorphing = false;
            self.bottomDockPreviewWidth = 0;
          }
        }, 40);
        return;
      }
      this.syncBottomDockPreview();
    },

    readBottomDockSlotCenters() {
      var out = [];
      if (typeof document === 'undefined') return out;
      var root = document.querySelector('.bottom-dock');
      if (!root || typeof root.querySelectorAll !== 'function') return out;
      var nodes = root.querySelectorAll('[data-dock-id], .dock-tile-slot[data-dock-slot-id]');
      for (var i = 0; i < nodes.length; i += 1) {
        var node = nodes[i];
        if (!node || typeof node.getAttribute !== 'function' || typeof node.getBoundingClientRect !== 'function') continue;
        var id = String(node.getAttribute('data-dock-id') || node.getAttribute('data-dock-slot-id') || '').trim();
        if (!id) continue;
        var rect = node.getBoundingClientRect();
        var centerX = Number(rect.left || 0) + (Number(rect.width || 0) / 2);
        var centerY = Number(rect.top || 0) + (Number(rect.height || 0) / 2);
        if (!Number.isFinite(centerX) || !Number.isFinite(centerY)) continue;
        out.push({ id: id, centerX: centerX, centerY: centerY });
      }
      return out;
    },

    bottomDockWeightForDistance(distancePx) {
      var d = Math.abs(Number(distancePx || 0));
      if (!Number.isFinite(d)) return 0;
      var sigma = 52;
      var exponent = -((d * d) / (2 * sigma * sigma));
      var weight = Math.exp(exponent);
      if (!Number.isFinite(weight) || weight < 0.008) return 0;
      if (weight > 1) return 1;
      return weight;
    },

    refreshBottomDockHoverWeights() {
      var side = this.bottomDockActiveSide();
      var vertical = this.bottomDockIsVerticalSide(side);
      var primaryPointer = vertical
        ? Number(this.bottomDockPointerY || 0)
        : Number(this.bottomDockPointerX || 0);
      if (!Number.isFinite(primaryPointer) || primaryPointer <= 0) {
        this.bottomDockHoverWeightById = {};
        return;
      }
      var centers = this.readBottomDockSlotCenters();
      if (!centers.length) {
        this.bottomDockHoverWeightById = {};
        return;
      }
      var nearestId = '';
      var nearestDistance = Number.POSITIVE_INFINITY;
      var weights = {};
      for (var i = 0; i < centers.length; i += 1) {
        var item = centers[i];
        if (!item || !item.id) continue;
        var anchor = vertical ? Number(item.centerY || 0) : Number(item.centerX || 0);
        var dist = Math.abs(primaryPointer - anchor);
        if (!Number.isFinite(dist)) continue;
        if (dist < nearestDistance) {
          nearestDistance = dist;
          nearestId = item.id;
        }
        weights[item.id] = this.bottomDockWeightForDistance(dist);
      }
      this.bottomDockHoverWeightById = weights;
      if (nearestId) this.bottomDockHoverId = nearestId;
    },

    updateBottomDockPointer(ev) {
      if (!ev) return;
      if (String(this.bottomDockDragId || '').trim()) return;
      if (this.bottomDockContainerDragActive || this._bottomDockContainerPointerActive) return;
      var x = Number(ev.clientX || 0);
      var y = Number(ev.clientY || 0);
      if (!Number.isFinite(x) || x <= 0) return;
      this.bottomDockPointerX = x;
      if (Number.isFinite(y) && y > 0) this.bottomDockPointerY = y;
      this.refreshBottomDockHoverWeights();
      this.syncBottomDockPreview();
    },

    reviveBottomDockHoverFromPoint(clientX, clientY) {
      if (String(this.bottomDockDragId || '').trim()) return;
      if (this.bottomDockContainerDragActive || this._bottomDockContainerPointerActive) return;
      var x = Number(clientX || 0);
      var y = Number(clientY || 0);
      if (!Number.isFinite(x) || !Number.isFinite(y) || x <= 0 || y <= 0) return;
      var root = document.querySelector('.bottom-dock');
      if (!root || typeof root.getBoundingClientRect !== 'function') return;
      var rect = root.getBoundingClientRect();
      var withinX = x >= (Number(rect.left || 0) - 16) && x <= (Number(rect.right || 0) + 16);
      var withinY = y >= (Number(rect.top || 0) - 18) && y <= (Number(rect.bottom || 0) + 18);
      if (!withinX || !withinY) return;
      this.bottomDockPointerX = x;
      this.bottomDockPointerY = y;
      this.refreshBottomDockHoverWeights();
      this.syncBottomDockPreview();
      this.scheduleBottomDockPreviewReflow();
    },

    scheduleBottomDockPreviewReflow() {
      this.cancelBottomDockPreviewReflow();
      var self = this;
      this._bottomDockPreviewReflowFrames = 10;
      var step = function() {
        if (!String(self.bottomDockHoverId || '').trim()) {
          self._bottomDockPreviewReflowRaf = 0;
          self._bottomDockPreviewReflowFrames = 0;
          return;
        }
        self.syncBottomDockPreview();
        self._bottomDockPreviewReflowFrames = Math.max(0, Number(self._bottomDockPreviewReflowFrames || 0) - 1);
        if (self._bottomDockPreviewReflowFrames <= 0) {
          self._bottomDockPreviewReflowRaf = 0;
          return;
        }
        self._bottomDockPreviewReflowRaf = requestAnimationFrame(step);
      };
      this._bottomDockPreviewReflowRaf = requestAnimationFrame(step);
    },

    cancelBottomDockPreviewReflow() {
      if (this._bottomDockPreviewReflowRaf && typeof cancelAnimationFrame === 'function') {
        try { cancelAnimationFrame(this._bottomDockPreviewReflowRaf); } catch(_) {}
      }
      this._bottomDockPreviewReflowRaf = 0;
      this._bottomDockPreviewReflowFrames = 0;
    },

    syncBottomDockPreview() {
      var key = String(this.bottomDockHoverId || '').trim();
      if (!key) {
        this.bottomDockPreviewVisible = false;
        this.bottomDockPreviewText = '';
        this.bottomDockPreviewMorphFromText = '';
        this.bottomDockPreviewHoverKey = '';
        this.bottomDockPreviewLabelMorphing = false;
        this.bottomDockPreviewWidth = 0;
        this.bottomDockPreviewLabelFxReady = true;
        return;
      }
      var text = this.bottomDockTileData(key, 'tooltip', '');
      var label = String(text || '').trim();
      if (!label) {
        this.bottomDockPreviewVisible = false;
        this.bottomDockPreviewText = '';
        this.bottomDockPreviewMorphFromText = '';
        this.bottomDockPreviewHoverKey = '';
        this.bottomDockPreviewLabelMorphing = false;
        this.bottomDockPreviewWidth = 0;
        this.bottomDockPreviewLabelFxReady = true;
        return;
      }
      var root = document.querySelector('.bottom-dock');
      var slot = document.querySelector('.bottom-dock [data-dock-id="' + key + '"], .bottom-dock .dock-tile-slot[data-dock-slot-id="' + key + '"]');
      if (!root || !slot) {
        this.bottomDockPreviewVisible = false;
        this.bottomDockPreviewText = '';
        this.bottomDockPreviewMorphFromText = '';
        this.bottomDockPreviewHoverKey = '';
        this.bottomDockPreviewLabelMorphing = false;
        this.bottomDockPreviewWidth = 0;
        this.bottomDockPreviewLabelFxReady = true;
        return;
      }
      var centerX = 0;
      var centerY = 0;
      var anchorY = 0;
      var anchorX = 0;
      var wallSide = this.bottomDockWallSide();
      var openSide = this.bottomDockOpenSide();
      var vertical = this.bottomDockIsVerticalSide(wallSide);
      var dockRect = (typeof root.getBoundingClientRect === 'function')
        ? root.getBoundingClientRect()
        : null;
      if (typeof slot.getBoundingClientRect === 'function' && dockRect) {
        var slotRect = slot.getBoundingClientRect();
        centerX = Number(slotRect.left || 0) + (Number(slotRect.width || 0) / 2);
        centerY = Number(slotRect.top || 0) + (Number(slotRect.height || 0) / 2);
        if (openSide === 'top') {
          anchorY = Number(dockRect.top || 0) - 8;
        } else if (openSide === 'bottom') {
          anchorY = Number(dockRect.bottom || 0) + 8;
        } else if (openSide === 'left') {
          anchorX = Number(dockRect.left || 0) - 8;
        } else {
          anchorX = Number(dockRect.right || 0) + 8;
        }
      } else if (slot.offsetParent === root) {
        var rootRect = root.getBoundingClientRect();
        centerX = Number(rootRect.left || 0) + Number(slot.offsetLeft || 0) + (Number(slot.offsetWidth || 0) / 2);
        centerY = Number(rootRect.top || 0) + Number(slot.offsetTop || 0) + (Number(slot.offsetHeight || 0) / 2);
        if (openSide === 'top') {
          anchorY = Number(rootRect.top || 0) - 8;
        } else if (openSide === 'bottom') {
          anchorY = Number(rootRect.bottom || 0) + 8;
        } else if (openSide === 'left') {
          anchorX = Number(rootRect.left || 0) - 8;
        } else {
          anchorX = Number(rootRect.right || 0) + 8;
        }
      }
      var pointerX = Number(this.bottomDockPointerX || 0);
      var pointerY = Number(this.bottomDockPointerY || 0);
      if (!vertical && Number.isFinite(pointerX) && pointerX > 0) {
        if (dockRect) {
          var minX = Number(dockRect.left || 0);
          var maxX = Number(dockRect.right || 0);
          if (Number.isFinite(minX) && Number.isFinite(maxX) && maxX > minX) {
            pointerX = Math.max(minX, Math.min(maxX, pointerX));
          }
        }
        centerX = pointerX;
      }
      if (vertical && Number.isFinite(pointerY) && pointerY > 0) {
        if (dockRect) {
          var minY = Number(dockRect.top || 0);
          var maxY = Number(dockRect.bottom || 0);
          if (Number.isFinite(minY) && Number.isFinite(maxY) && maxY > minY) {
            pointerY = Math.max(minY, Math.min(maxY, pointerY));
          }
        }
        centerY = pointerY;
      }
      if (!Number.isFinite(centerX)) centerX = 0;
      if (!Number.isFinite(centerY)) centerY = 0;
      if (!Number.isFinite(anchorX)) anchorX = 0;
      if (!Number.isFinite(anchorY)) anchorY = 0;
      this.bottomDockPreviewX = vertical ? anchorX : centerX;
      this.bottomDockPreviewY = vertical ? centerY : anchorY;
      this.bottomDockPreviewHoverKey = key;
      this.bottomDockPreviewVisible = true;
      this.bottomDockPreviewText = label;
      this.bottomDockPreviewMorphFromText = '';
      this.bottomDockPreviewLabelMorphing = false;
      this.bottomDockPreviewWidth = 0;
      this.bottomDockPreviewLabelFxReady = true;
    },
