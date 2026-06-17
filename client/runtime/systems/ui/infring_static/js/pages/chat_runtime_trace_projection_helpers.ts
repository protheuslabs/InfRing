'use strict';

// Shell-only compatibility helpers for rendering Agent Runtime trace projections.
//
// Gateway owns canonical runtime trace grouping and should emit projected rows
// with `children` when a compact parent row is needed. These helpers preserve
// older saved/live rows and keep the Shell renderer simple while the Gateway
// projection finishes absorbing legacy trace shapes.

function chatNormalizeTraceChildren(item, parentKind, fallbackId) {
  var children = Array.isArray(item && item.children) ? item.children : [];
  return children.map(function(child, childIdx) {
    var childRow = child && typeof child === 'object' ? child : {};
    var childText = String(childRow.text || childRow.title || childRow.display_text || childRow.summary || '').replace(/\r\n/g, '\n').trim();
    if (!childText) return null;
    return {
      id: String(childRow.id || childRow.activity_key || childRow.detail_ref || (parentKind + '-' + fallbackId + '-' + childIdx)).slice(0, 180),
      text: childText.split('\n').map(function(part) { return String(part || '').replace(/\s+/g, ' ').trim(); }).filter(Boolean).join('\n'),
      line_kind: String(childRow.line_kind || childRow.activity_kind || childRow.kind || parentKind || '').trim() || 'status',
      state: String(childRow.state || childRow.status || 'done').trim() || 'done',
      status: String(childRow.status || childRow.state || 'done').trim() || 'done',
      shimmer: false
    };
  }).filter(Boolean).slice(0, 40);
}

function chatNormalizeThoughtTraceRows(traceRows, dialogText, normalizeDialogText) {
  var rows = Array.isArray(traceRows) ? traceRows : [];
  var out = [];
  var seen = Object.create(null);
  function addTraceRow(row, fallbackId) {
    var item = row && typeof row === 'object' ? row : {};
    var text = String(item.text || item.title || item.display_text || item.summary || '').replace(/\r\n/g, '\n').trim();
    if (!text) return;
    text.split('\n').forEach(function(part) {
      var line = String(part || '').replace(/\s+/g, ' ').trim();
      if (!line) return;
      var kind = String(item.line_kind || item.activity_kind || item.kind || '').trim() || 'dialog';
      var kindSource = (kind + ' ' + line).toLowerCase();
      if (/file|edit|patch|diff|write|create/.test(kindSource)) kind = 'write';
      else if (/read|search|grep|rg|find|list|scan|inspect|check/.test(kindSource)) kind = 'read';
      else if (/command|exec|shell|bash|tool|function/.test(kindSource)) kind = 'tool';
      var key = (kind + '|' + line).toLowerCase();
      if (seen[key]) return;
      seen[key] = true;
      var projected = {
        id: String(item.id || item.activity_key || (kind + '-' + fallbackId + '-' + out.length)).slice(0, 180),
        text: line,
        line_kind: kind,
        state: String(item.state || item.status || 'done').trim() || 'done',
        shimmer: false
      };
      var children = chatNormalizeTraceChildren(item, kind, fallbackId);
      if (children.length) projected.children = children;
      out.push(projected);
    });
  }
  for (var i = 0; i < rows.length && out.length < 80; i += 1) addTraceRow(rows[i], i);
  var dialog = typeof normalizeDialogText === 'function'
    ? normalizeDialogText(dialogText)
    : String(dialogText || '').replace(/\r\n/g, '\n').trim();
  if (dialog) {
    dialog.split('\n').forEach(function(part, idx) {
      var line = String(part || '').replace(/\s+/g, ' ').trim();
      if (!line || out.length >= 80) return;
      var key = ('dialog|' + line).toLowerCase();
      var looseKey = line.toLowerCase();
      var hasLooseMatch = out.some(function(row) {
        return String(row && row.text || '').replace(/\s+/g, ' ').trim().toLowerCase() === looseKey;
      });
      if (seen[key] || hasLooseMatch) return;
      seen[key] = true;
      out.push({
        id: 'dialog-' + idx + '-' + looseKey.slice(0, 80).replace(/[^a-z0-9]+/g, '-'),
        text: line,
        line_kind: 'dialog',
        state: 'done',
        shimmer: false
      });
    });
  }
  return out.slice(-80);
}

function chatThoughtTraceFileActionVerb(kind) {
  return String(kind || '') === 'write' ? 'Edited' : 'Read';
}

function chatExtractThoughtTraceFileLabel(text, kind) {
  var value = String(text || '').replace(/\r\n/g, '\n').trim();
  if (!value) return '';
  var quoted = value.match(/`([^`\n]+)`/);
  if (quoted && quoted[1]) return quoted[1].trim();
  var colon = value.match(/(?:file|path|changed|edited|wrote|written|created|updated|read|reading)\s*:?\s+(.+)$/i);
  if (colon && colon[1]) return colon[1].trim();
  var stripped = value.replace(/^(?:working on|completed|finished|failed|started|running|ran)\s+/i, '');
  stripped = stripped.replace(/^(?:read|reading|edit|edited|editing|write|wrote|writing|created|creating|updated|updating)\s+(?:file\s+|files\s+)?/i, '');
  stripped = stripped.replace(/^(?:file change|file)\s*:?\s*/i, '');
  stripped = stripped.trim();
  if (!stripped || stripped === value) return value;
  return stripped;
}

function chatCompactThoughtTraceRows(traceRows) {
  var rows = Array.isArray(traceRows) ? traceRows : [];
  var compacted = [];
  var group = null;
  function flush() {
    if (!group) return;
    var count = group.children.length;
    var verb = chatThoughtTraceFileActionVerb(group.line_kind);
    compacted.push({
      id: group.id,
      text: verb + ' ' + count + ' ' + (count === 1 ? 'file' : 'files'),
      line_kind: group.line_kind,
      state: group.state,
      status: group.status,
      shimmer: false,
      children: group.children
    });
    group = null;
  }
  for (var i = 0; i < rows.length; i += 1) {
    var item = rows[i] && typeof rows[i] === 'object' ? rows[i] : {};
    var kind = String(item.line_kind || '').trim();
    if (Array.isArray(item.children) && item.children.length) {
      flush();
      compacted.push(item);
      continue;
    }
    if (kind !== 'read' && kind !== 'write') {
      flush();
      compacted.push(item);
      continue;
    }
    var label = chatExtractThoughtTraceFileLabel(item.text, kind);
    var child = {
      id: String(item.id || (kind + '-file-' + i)).slice(0, 180) + '-detail',
      text: chatThoughtTraceFileActionVerb(kind) + ' ' + label,
      line_kind: kind,
      state: item.state || 'done',
      status: item.status || item.state || 'done',
      shimmer: false
    };
    if (!group || group.line_kind !== kind || group.state !== item.state) {
      flush();
      group = {
        id: 'file-group-' + kind + '-' + String(item.id || i).slice(0, 140),
        line_kind: kind,
        state: item.state || 'done',
        status: item.status || item.state || 'done',
        children: []
      };
    }
    group.children.push(child);
  }
  flush();
  return compacted;
}
