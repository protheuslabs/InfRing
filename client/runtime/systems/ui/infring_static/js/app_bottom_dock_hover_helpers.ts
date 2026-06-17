// Bottom-dock hover/preview helpers keep pointer projection logic out of the app-store shell.
function infringSetBottomDockHover(page, id, ev) {
  if (String(page.bottomDockDragId || '').trim()) return;
  if (page.bottomDockContainerDragActive || page._bottomDockContainerPointerActive) return;
  var key = String(id || '').trim();
  page.bottomDockHoverId = key;
  if (ev) {
    var evX = Number(ev.clientX || 0);
    var evY = Number(ev.clientY || 0);
    if (Number.isFinite(evX) && evX > 0) page.bottomDockPointerX = evX;
    if (Number.isFinite(evY) && evY > 0) page.bottomDockPointerY = evY;
  }
  if (page._bottomDockPreviewHideTimer) {
    try { clearTimeout(page._bottomDockPreviewHideTimer); } catch(_) {}
    page._bottomDockPreviewHideTimer = 0;
  }
  if (!Number.isFinite(page.bottomDockPointerX) || page.bottomDockPointerX <= 0) {
    try {
      var slot = document.querySelector('.bottom-dock [data-dock-id="' + key + '"], .bottom-dock .dock-tile-slot[data-dock-slot-id="' + key + '"]');
      if (slot && typeof slot.getBoundingClientRect === 'function') {
        var slotRect = slot.getBoundingClientRect();
        page.bottomDockPointerX = Number(slotRect.left || 0) + (Number(slotRect.width || 0) / 2);
        page.bottomDockPointerY = Number(slotRect.top || 0) + (Number(slotRect.height || 0) / 2);
      }
    } catch(_) {}
  }
  page.refreshBottomDockHoverWeights();
  page.syncBottomDockPreview();
  page.scheduleBottomDockPreviewReflow();
}

function infringClearBottomDockHover(page, id) {
  if (id) return;
  page.bottomDockHoverId = '';
  if (!page.bottomDockHoverId) {
    page.bottomDockHoverWeightById = {};
    page.bottomDockPointerX = 0;
    page.bottomDockPointerY = 0;
    page.cancelBottomDockPreviewReflow();
    if (page._bottomDockPreviewHideTimer) {
      try { clearTimeout(page._bottomDockPreviewHideTimer); } catch(_) {}
    }
    page._bottomDockPreviewHideTimer = window.setTimeout(function() {
      page._bottomDockPreviewHideTimer = 0;
      if (!String(page.bottomDockHoverId || '').trim()) {
        page.bottomDockPreviewVisible = false;
        page.bottomDockPreviewText = '';
        page.bottomDockPreviewMorphFromText = '';
        page.bottomDockPreviewLabelMorphing = false;
        page.bottomDockPreviewWidth = 0;
      }
    }, 40);
    return;
  }
  page.syncBottomDockPreview();
}

function infringReadBottomDockSlotCenters() {
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
}

function infringBottomDockWeightForDistance(distancePx) {
  var d = Math.abs(Number(distancePx || 0));
  if (!Number.isFinite(d)) return 0;
  var sigma = 52;
  var exponent = -((d * d) / (2 * sigma * sigma));
  var weight = Math.exp(exponent);
  if (!Number.isFinite(weight) || weight < 0.008) return 0;
  if (weight > 1) return 1;
  return weight;
}

function infringRefreshBottomDockHoverWeights(page) {
  var side = page.bottomDockActiveSide();
  var vertical = page.bottomDockIsVerticalSide(side);
  var primaryPointer = vertical
    ? Number(page.bottomDockPointerY || 0)
    : Number(page.bottomDockPointerX || 0);
  if (!Number.isFinite(primaryPointer) || primaryPointer <= 0) {
    page.bottomDockHoverWeightById = {};
    return;
  }
  var centers = page.readBottomDockSlotCenters();
  if (!centers.length) {
    page.bottomDockHoverWeightById = {};
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
    weights[item.id] = page.bottomDockWeightForDistance(dist);
  }
  page.bottomDockHoverWeightById = weights;
  if (nearestId) page.bottomDockHoverId = nearestId;
}

function infringUpdateBottomDockPointer(page, ev) {
  if (!ev) return;
  if (String(page.bottomDockDragId || '').trim()) return;
  if (page.bottomDockContainerDragActive || page._bottomDockContainerPointerActive) return;
  var x = Number(ev.clientX || 0);
  var y = Number(ev.clientY || 0);
  if (!Number.isFinite(x) || x <= 0) return;
  page.bottomDockPointerX = x;
  if (Number.isFinite(y) && y > 0) page.bottomDockPointerY = y;
  page.refreshBottomDockHoverWeights();
  page.syncBottomDockPreview();
}

function infringReviveBottomDockHoverFromPoint(page, clientX, clientY) {
  if (String(page.bottomDockDragId || '').trim()) return;
  if (page.bottomDockContainerDragActive || page._bottomDockContainerPointerActive) return;
  var x = Number(clientX || 0);
  var y = Number(clientY || 0);
  if (!Number.isFinite(x) || !Number.isFinite(y) || x <= 0 || y <= 0) return;
  var root = document.querySelector('.bottom-dock');
  if (!root || typeof root.getBoundingClientRect !== 'function') return;
  var rect = root.getBoundingClientRect();
  var withinX = x >= (Number(rect.left || 0) - 16) && x <= (Number(rect.right || 0) + 16);
  var withinY = y >= (Number(rect.top || 0) - 18) && y <= (Number(rect.bottom || 0) + 18);
  if (!withinX || !withinY) return;
  page.bottomDockPointerX = x;
  page.bottomDockPointerY = y;
  page.refreshBottomDockHoverWeights();
  page.syncBottomDockPreview();
  page.scheduleBottomDockPreviewReflow();
}

function infringScheduleBottomDockPreviewReflow(page) {
  page.cancelBottomDockPreviewReflow();
  page._bottomDockPreviewReflowFrames = 10;
  var step = function() {
    if (!String(page.bottomDockHoverId || '').trim()) {
      page._bottomDockPreviewReflowRaf = 0;
      page._bottomDockPreviewReflowFrames = 0;
      return;
    }
    page.syncBottomDockPreview();
    page._bottomDockPreviewReflowFrames = Math.max(0, Number(page._bottomDockPreviewReflowFrames || 0) - 1);
    if (page._bottomDockPreviewReflowFrames <= 0) {
      page._bottomDockPreviewReflowRaf = 0;
      return;
    }
    page._bottomDockPreviewReflowRaf = requestAnimationFrame(step);
  };
  page._bottomDockPreviewReflowRaf = requestAnimationFrame(step);
}

function infringCancelBottomDockPreviewReflow(page) {
  if (page._bottomDockPreviewReflowRaf && typeof cancelAnimationFrame === 'function') {
    try { cancelAnimationFrame(page._bottomDockPreviewReflowRaf); } catch(_) {}
  }
  page._bottomDockPreviewReflowRaf = 0;
  page._bottomDockPreviewReflowFrames = 0;
}

function infringResetBottomDockPreview(page, hoverKey) {
  page.bottomDockPreviewVisible = false;
  page.bottomDockPreviewText = '';
  page.bottomDockPreviewMorphFromText = '';
  page.bottomDockPreviewHoverKey = hoverKey || '';
  page.bottomDockPreviewLabelMorphing = false;
  page.bottomDockPreviewWidth = 0;
  page.bottomDockPreviewLabelFxReady = true;
}

function infringSyncBottomDockPreview(page) {
  var key = String(page.bottomDockHoverId || '').trim();
  if (!key) {
    infringResetBottomDockPreview(page, '');
    return;
  }
  var text = page.bottomDockTileData(key, 'tooltip', '');
  var label = String(text || '').trim();
  if (!label) {
    infringResetBottomDockPreview(page, '');
    return;
  }
  var root = document.querySelector('.bottom-dock');
  var slot = document.querySelector('.bottom-dock [data-dock-id="' + key + '"], .bottom-dock .dock-tile-slot[data-dock-slot-id="' + key + '"]');
  if (!root || !slot) {
    infringResetBottomDockPreview(page, '');
    return;
  }
  var centerX = 0;
  var centerY = 0;
  var anchorY = 0;
  var anchorX = 0;
  var wallSide = page.bottomDockWallSide();
  var openSide = page.bottomDockOpenSide();
  var vertical = page.bottomDockIsVerticalSide(wallSide);
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
  var pointerX = Number(page.bottomDockPointerX || 0);
  var pointerY = Number(page.bottomDockPointerY || 0);
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
  page.bottomDockPreviewX = vertical ? anchorX : centerX;
  page.bottomDockPreviewY = vertical ? centerY : anchorY;
  page.bottomDockPreviewHoverKey = key;
  page.bottomDockPreviewVisible = true;
  page.bottomDockPreviewText = label;
  page.bottomDockPreviewMorphFromText = '';
  page.bottomDockPreviewLabelMorphing = false;
  page.bottomDockPreviewWidth = 0;
  page.bottomDockPreviewLabelFxReady = true;
}
