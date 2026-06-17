'use strict';

var InfringSharedShellServices = (function(existing) {
  var services = existing && typeof existing === 'object' ? existing : {};
  var agentMessageRoles = { agent: true, assistant: true };

  function trimString(value) {
    return String(value == null ? '' : value).trim();
  }

  function normalizeRole(value) {
    var role = trimString(value).toLowerCase();
    return role === 'assistant' ? 'agent' : role;
  }

  function messageRole(row) {
    if (!row || typeof row !== 'object') return '';
    if (row.terminal) {
      var terminalSource = normalizeRole(row.terminal_source);
      if (terminalSource === 'user' || terminalSource === 'human') return 'human';
      if (terminalSource === 'agent') return 'agent';
      return 'system';
    }
    var role = normalizeRole(row.role);
    if (role === 'user') return 'human';
    return role;
  }

  function messageIsAgentAuthored(row) {
    var role = messageRole(row);
    return !!agentMessageRoles[role];
  }

  function messageHasVisibleContent(row) {
    if (!row || typeof row !== 'object') return false;
    if (trimString(row.text)) return true;
    if (Array.isArray(row.tools) && row.tools.length > 0) return true;
    if (row.meta) return true;
    if (row.ts) return true;
    return false;
  }

  function agentHasSession(agent) {
    return !!(agent && typeof agent === 'object' && trimString(agent.id));
  }

  function canRequestEvalIssueReport(row, agent) {
    if (!agentHasSession(agent)) return false;
    if (!messageIsAgentAuthored(row)) return false;
    if (!row || row.thinking || row.terminal || row.is_notice) return false;
    return messageHasVisibleContent(row);
  }

  function numericOr(value, fallback) {
    var numeric = Number(value);
    return Number.isFinite(numeric) ? numeric : fallback;
  }

  function normalizeRect(rect, fallbackWidth, fallbackHeight) {
    var source = rect && typeof rect === 'object' ? rect : {};
    var left = numericOr(source.left, 0);
    var top = numericOr(source.top, 0);
    var width = numericOr(source.width, fallbackWidth || 0);
    var height = numericOr(source.height, fallbackHeight || 0);
    var right = numericOr(source.right, left + width);
    var bottom = numericOr(source.bottom, top + height);
    return {
      left: left,
      right: right,
      top: top,
      bottom: bottom,
      width: Math.max(0, right - left || width),
      height: Math.max(0, bottom - top || height)
    };
  }

  function resolveOverlayPlacement(originRect, viewportRect, options) {
    var viewport = normalizeRect(viewportRect, 0, 0);
    var origin = normalizeRect(originRect, 0, 0);
    var opts = options && typeof options === 'object' ? options : {};
    var horizontalCenter = origin.left + origin.width / 2;
    var verticalCenter = origin.top + origin.height / 2;
    var viewportHorizontalCenter = viewport.left + viewport.width / 2;
    var viewportVerticalCenter = viewport.top + viewport.height / 2;
    var horizontal = horizontalCenter <= viewportHorizontalCenter ? 'right' : 'left';
    var vertical = verticalCenter <= viewportVerticalCenter ? 'below' : 'above';
    if (opts.preferHorizontal === 'left' || opts.preferHorizontal === 'right') horizontal = opts.preferHorizontal;
    if (opts.preferVertical === 'above' || opts.preferVertical === 'below') vertical = opts.preferVertical;
    return {
      horizontal: horizontal,
      vertical: vertical,
      anchorX: horizontal === 'right' ? origin.left : origin.right,
      anchorY: vertical === 'below' ? origin.bottom : origin.top,
      className: 'shell-overlay-placement shell-overlay-x-' + horizontal + ' shell-overlay-y-' + vertical
    };
  }

  function viewportRect() {
    if (typeof window === 'undefined') {
      return { left: 0, top: 0, right: 1, bottom: 1, width: 1, height: 1 };
    }
    var width = numericOr(window.innerWidth, 1);
    var height = numericOr(window.innerHeight, 1);
    return {
      left: 0,
      top: 0,
      right: Math.max(1, width),
      bottom: Math.max(1, height),
      width: Math.max(1, width),
      height: Math.max(1, height)
    };
  }

  function normalizePopupSide(sideValue, fallbackSide) {
    var fallback = trimString(fallbackSide || 'bottom').toLowerCase();
    if (fallback !== 'top' && fallback !== 'left' && fallback !== 'right') fallback = 'bottom';
    var side = trimString(sideValue || fallback).toLowerCase();
    if (side !== 'top' && side !== 'left' && side !== 'right') side = 'bottom';
    return side;
  }

  function oppositePopupSide(sideValue) {
    var side = normalizePopupSide(sideValue, 'bottom');
    if (side === 'top') return 'bottom';
    if (side === 'left') return 'right';
    if (side === 'right') return 'left';
    return 'top';
  }

  function popupWallAffinity(rect, explicitViewportRect) {
    if (!rect) return null;
    var viewport = normalizeRect(explicitViewportRect || viewportRect(), 1, 1);
    var origin = normalizeRect(rect, 1, 1);
    var width = Math.max(1, Math.abs(origin.right - origin.left));
    var height = Math.max(1, Math.abs(origin.bottom - origin.top));
    var distanceToLeft = Math.max(0, origin.left - viewport.left);
    var distanceToRight = Math.max(0, viewport.right - origin.right);
    var distanceToTop = Math.max(0, origin.top - viewport.top);
    var distanceToBottom = Math.max(0, viewport.bottom - origin.bottom);
    var proximityScore = function(distance) {
      var normalized = numericOr(distance, 0);
      if (normalized < 0) normalized = 0;
      return 1 / (1 + normalized);
    };
    return {
      scores: {
        top: width * proximityScore(distanceToTop),
        bottom: width * proximityScore(distanceToBottom),
        left: height * proximityScore(distanceToLeft),
        right: height * proximityScore(distanceToRight)
      },
      distances: {
        top: distanceToTop,
        bottom: distanceToBottom,
        left: distanceToLeft,
        right: distanceToRight
      }
    };
  }

  function popupSideAwayFromNearestWall(rect, fallbackSide, explicitViewportRect) {
    var fallback = normalizePopupSide('', fallbackSide);
    var affinity = popupWallAffinity(rect, explicitViewportRect);
    if (!affinity || !affinity.scores || !affinity.distances) return fallback;
    var scores = affinity.scores;
    var distances = affinity.distances;
    var walls = ['top', 'bottom', 'left', 'right'];
    var fallbackWall = oppositePopupSide(fallback);
    var winner = walls[0];
    var winnerScore = numericOr(scores[winner], 0);
    var epsilon = 0.000001;
    for (var i = 1; i < walls.length; i += 1) {
      var wall = walls[i];
      var score = numericOr(scores[wall], 0);
      if (score > winnerScore + epsilon) {
        winner = wall;
        winnerScore = score;
        continue;
      }
      if (Math.abs(score - winnerScore) <= epsilon) {
        if (wall === fallbackWall && winner !== fallbackWall) {
          winner = wall;
          winnerScore = score;
          continue;
        }
        if (numericOr(distances[wall], 0) < numericOr(distances[winner], 0)) {
          winner = wall;
          winnerScore = score;
        }
      }
    }
    return oppositePopupSide(winner);
  }

  function popupHorizontalAwayFromNearestWall(rect, fallbackSide, explicitViewportRect) {
    var fallback = trimString(fallbackSide || 'right').toLowerCase();
    if (fallback !== 'left') fallback = 'right';
    var affinity = popupWallAffinity(rect, explicitViewportRect);
    if (!affinity || !affinity.distances) return fallback;
    var distances = affinity.distances;
    var nearest = numericOr(distances.left, 0) <= numericOr(distances.right, 0) ? 'left' : 'right';
    return nearest === 'left' ? 'right' : 'left';
  }

  function popupVerticalAwayFromNearestWall(rect, fallbackSide, explicitViewportRect) {
    var fallback = trimString(fallbackSide || 'bottom').toLowerCase();
    if (fallback !== 'top') fallback = 'bottom';
    var affinity = popupWallAffinity(rect, explicitViewportRect);
    if (!affinity || !affinity.distances) return fallback;
    var distances = affinity.distances;
    var nearest = numericOr(distances.top, 0) <= numericOr(distances.bottom, 0) ? 'top' : 'bottom';
    return nearest === 'top' ? 'bottom' : 'top';
  }

  function popupAxisAwareSideAway(rect, fallbackSide, explicitViewportRect) {
    var fallback = normalizePopupSide('', fallbackSide || 'bottom');
    if (fallback === 'left' || fallback === 'right') {
      return popupHorizontalAwayFromNearestWall(rect, fallback, explicitViewportRect);
    }
    return popupVerticalAwayFromNearestWall(rect, fallback, explicitViewportRect);
  }

  function popupAnchorPoint(rect, sideOverride, explicitViewportRect) {
    var preferredSide = normalizePopupSide(sideOverride, 'bottom');
    if (!rect) {
      return { left: 0, top: 0, side: preferredSide, inline_away: 'right', block_away: 'bottom' };
    }
    var origin = normalizeRect(rect, 1, 1);
    var side = popupAxisAwareSideAway(origin, preferredSide, explicitViewportRect);
    var inlineAway = popupHorizontalAwayFromNearestWall(origin, 'right', explicitViewportRect);
    var blockAway = popupVerticalAwayFromNearestWall(origin, 'bottom', explicitViewportRect);
    var left = Math.round(origin.left);
    var top = Math.round(origin.bottom);
    if (side === 'top') {
      left = inlineAway === 'left' ? Math.round(origin.right) : Math.round(origin.left);
      top = Math.round(origin.top);
    } else if (side === 'bottom') {
      left = inlineAway === 'left' ? Math.round(origin.right) : Math.round(origin.left);
      top = Math.round(origin.bottom);
    } else if (side === 'left') {
      left = Math.round(origin.left);
      top = blockAway === 'top' ? Math.round(origin.bottom) : Math.round(origin.top);
    } else if (side === 'right') {
      left = Math.round(origin.right);
      top = blockAway === 'top' ? Math.round(origin.bottom) : Math.round(origin.top);
    }
    return {
      left: left,
      top: top,
      side: side,
      inline_away: inlineAway === 'left' ? 'left' : 'right',
      block_away: blockAway === 'top' ? 'top' : 'bottom'
    };
  }

  function popupDropdownClass(rect, fallbackSide, layoutKey, explicitViewportRect) {
    trimString(layoutKey);
    var fallback = normalizePopupSide('', fallbackSide || 'bottom');
    var side = rect ? popupAxisAwareSideAway(rect, fallback, explicitViewportRect) : fallback;
    var inlineAway = rect ? popupHorizontalAwayFromNearestWall(rect, 'right', explicitViewportRect) : 'right';
    var blockAway = rect ? popupVerticalAwayFromNearestWall(rect, 'bottom', explicitViewportRect) : 'bottom';
    return {
      'taskbar-anchored-dropdown': true,
      'is-side-top': side === 'top',
      'is-side-bottom': side === 'bottom',
      'is-side-left': side === 'left',
      'is-side-right': side === 'right',
      'is-inline-away-left': inlineAway === 'left',
      'is-inline-away-right': inlineAway === 'right',
      'is-block-away-top': blockAway === 'top',
      'is-block-away-bottom': blockAway === 'bottom'
    };
  }

  function emptyPopupState() {
    return {
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
  }

  function popupOrigin(overrides) {
    return Object.assign({
      source: '',
      active: false,
      ready: false,
      side: 'top',
      inline_away: 'right',
      block_away: 'bottom',
      left: 0,
      top: 0,
      compact: false,
      title: '',
      body: '',
      meta_origin: '',
      meta_time: '',
      unread: false
    }, overrides || {});
  }

  function openPopupState(id, label, config, anchor) {
    var source = config && typeof config === 'object' ? config : {};
    var point = anchor && typeof anchor === 'object' ? anchor : popupAnchorPoint(null, source.side);
    return {
      id: trimString(id),
      active: true,
      source: trimString(source.source),
      title: trimString(label),
      body: trimString(source.body),
      meta_origin: '',
      meta_time: '',
      unread: !!source.unread,
      left: Math.round(numericOr(point.left, 0)),
      top: Math.round(numericOr(point.top, 0)),
      side: normalizePopupSide(point.side, 'bottom'),
      inline_away: point.inline_away === 'left' ? 'left' : 'right',
      block_away: point.block_away === 'top' ? 'top' : 'bottom',
      compact: false
    };
  }

  function closePopupState(current, rawId) {
    var popupId = trimString(rawId);
    var currentId = trimString(current && current.id);
    if (popupId && currentId && popupId !== currentId) return current || emptyPopupState();
    return emptyPopupState();
  }

  function popupStateOrigin(popup) {
    var source = popup && typeof popup === 'object' ? popup : {};
    var title = trimString(source.title);
    var body = trimString(source.body);
    var left = Math.round(numericOr(source.left, 0));
    var top = Math.round(numericOr(source.top, 0));
    var side = normalizePopupSide(source.side, 'bottom');
    var inlineAway = trimString(source.inline_away || 'right').toLowerCase();
    var blockAway = trimString(source.block_away || 'bottom').toLowerCase();
    if (inlineAway !== 'left' && inlineAway !== 'right') inlineAway = 'center';
    if (blockAway !== 'top' && blockAway !== 'bottom') blockAway = 'center';
    if (!source.active || !title) return popupOrigin();
    return popupOrigin({
      source: trimString(source.source || 'ui'),
      active: true,
      ready: left > 0 && top > 0,
      side: side,
      inline_away: inlineAway,
      block_away: blockAway,
      left: left,
      top: top,
      compact: false,
      title: title,
      body: body,
      meta_origin: '',
      meta_time: '',
      unread: !!source.unread
    });
  }

  function popupOverlayClass(popup, glassKind) {
    var source = popup && typeof popup === 'object' ? popup : popupOrigin();
    var glassClass = glassSurfaceClass(glassKind || 'fogged-glass');
    var classes = {
      'is-visible': !!(source.active && source.ready && source.title),
      'is-side-top': source.side === 'top',
      'is-side-bottom': source.side === 'bottom',
      'is-side-left': source.side === 'left',
      'is-side-right': source.side === 'right',
      'is-inline-away-left': source.inline_away === 'left',
      'is-inline-away-right': source.inline_away === 'right',
      'is-inline-away-center': source.inline_away !== 'left' && source.inline_away !== 'right',
      'is-block-away-top': source.block_away === 'top',
      'is-block-away-bottom': source.block_away === 'bottom',
      'is-block-away-center': source.block_away !== 'top' && source.block_away !== 'bottom',
      'is-unread': !!source.unread
    };
    classes[glassClass] = true;
    return classes;
  }

  function popupOverlayStyle(popup) {
    var source = popup && typeof popup === 'object' ? popup : popupOrigin();
    if (!source.active || !source.ready) return 'left:-9999px;top:-9999px;';
    return 'left:' + Math.round(numericOr(source.left, 0)) + 'px;top:' + Math.round(numericOr(source.top, 0)) + 'px;';
  }

  function glassSurfaceClass(kind) {
    var value = trimString(kind).toLowerCase().replace(/_/g, '-');
    if (value === 'fogged' || value === 'fogged-glass') return 'fogged-glass';
    if (value === 'warped' || value === 'warped-glass' || value === 'magnified-glass') return 'warped-glass';
    if (value === 'simple' || value === 'simple-glass') return 'simple-glass';
    return 'simple-glass';
  }

  services.message = Object.assign({}, services.message || {}, {
    role: messageRole,
    isAgentAuthored: messageIsAgentAuthored,
    hasVisibleContent: messageHasVisibleContent,
    canRequestEvalIssueReport: canRequestEvalIssueReport
  });
  services.overlay = Object.assign({}, services.overlay || {}, {
    resolvePlacement: resolveOverlayPlacement
  });
  services.glass = Object.assign({}, services.glass || {}, {
    surfaceClass: glassSurfaceClass
  });
  services.popup = Object.assign({}, services.popup || {}, {
    normalizeSide: normalizePopupSide,
    oppositeSide: oppositePopupSide,
    wallAffinity: popupWallAffinity,
    sideAwayFromNearestWall: popupSideAwayFromNearestWall,
    horizontalAwayFromNearestWall: popupHorizontalAwayFromNearestWall,
    verticalAwayFromNearestWall: popupVerticalAwayFromNearestWall,
    axisAwareSideAway: popupAxisAwareSideAway,
    anchorPoint: popupAnchorPoint,
    dropdownClass: popupDropdownClass,
    emptyState: emptyPopupState,
    origin: popupOrigin,
    openState: openPopupState,
    closeState: closePopupState,
    stateOrigin: popupStateOrigin,
    overlayClass: popupOverlayClass,
    overlayStyle: popupOverlayStyle
  });

  return services;
})(typeof InfringSharedShellServices === 'object' ? InfringSharedShellServices : null);

if (typeof window !== 'undefined') {
  window.InfringSharedShellServices = InfringSharedShellServices;
}
