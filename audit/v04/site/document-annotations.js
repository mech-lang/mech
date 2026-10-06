const svgNamespace = 'http://www.w3.org/2000/svg';

function textNodes(element) {
  const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT);
  const nodes = [];
  let offset = 0;
  while (walker.nextNode()) {
    const node = walker.currentNode;
    nodes.push({node, start:offset, end:offset + node.length});
    offset += node.length;
  }
  return nodes;
}

// Match displayed text within canonical source regions. This projection never
// reparses Mech or estimates positions from the width of source characters.
function displayedRange(content, region, diagnostic, source, comments) {
  for (const code of content.querySelectorAll('pre code')) {
    const text = code.textContent;
    if (!text) continue;
    let at = source.indexOf(text, region.start);
    while (at >= region.start && at + text.length <= region.end) {
      if (diagnostic.start >= at && diagnostic.end <= at + text.length) {
        return {code,start:diagnostic.start - at,end:diagnostic.end - at};
      }
      at = source.indexOf(text, at + 1);
    }
    // Canonical comment ranges separate source code from rendered comment
    // markup. Match unchanged code segments in source order across that markup.
    const pieces = [];
    let piece;
    for (const row of textNodes(code)) {
      if (row.node.parentElement.closest('.mech-comment')) {piece=null;continue;}
      if (piece && piece.end === row.start) {piece.text += row.node.data;piece.end=row.end;}
      else {piece={start:row.start,end:row.end,text:row.node.data};pieces.push(piece);}
    }
    let cursor = region.start;
    for (const piece of pieces) {
      let at = source.indexOf(piece.text,cursor);
      while (at >= 0 && at + piece.text.length <= region.end) {
        const comment = comments.find(range => range.start < at + piece.text.length && range.end > at);
        if (!comment) break;
        at = source.indexOf(piece.text,comment.end);
      }
      if (at < cursor || at + piece.text.length > region.end) continue;
      if (diagnostic.start >= at && diagnostic.end <= at + piece.text.length) {
        return {code,start:piece.start + diagnostic.start-at,end:piece.start + diagnostic.end-at};
      }
      cursor = at + piece.text.length;
    }
    // Rich comments can change displayed prose. Match the complete affected
    // code line to its source line, retaining its occurrence within the region.
    const lineStart = source.lastIndexOf('\n', Math.max(0, diagnostic.start - 1)) + 1;
    const nextLine = source.indexOf('\n', diagnostic.start);
    const lineEnd = nextLine < 0 ? source.length : nextLine;
    const sourceLine = source.slice(lineStart, lineEnd).replace(/\r$/, '');
    if (!sourceLine || diagnostic.end > lineEnd || lineStart < region.start) continue;
    const sourceLines = source.slice(region.start,lineStart).split('\n');
    const occurrence = sourceLines.filter(line => line.replace(/\r$/, '') === sourceLine).length;
    const nodes = textNodes(code);
    let renderedOffset = 0, matched = 0;
    for (const line of text.split('\n')) {
      const inComment = nodes.some(({node,start,end}) => start < renderedOffset + line.length && end > renderedOffset && node.parentElement.closest('.mech-comment'));
      if (line.replace(/\r$/, '') === sourceLine && !inComment && matched++ === occurrence) {
        return {code,start:renderedOffset + diagnostic.start - lineStart,end:renderedOffset + diagnostic.end - lineStart};
      }
      renderedOffset += line.length + 1;
    }
  }
  return null;
}

function markRanges(code, entries) {
  const originalNodes = textNodes(code);
  for (const {node,start,end} of originalNodes) {
    const spans = entries.filter(entry => entry.range.start < end && entry.range.end > start);
    const points = entries.filter(entry => entry.range.start === entry.range.end && entry.range.start >= start && (entry.range.start < end || end === code.textContent.length && entry.range.start === end));
    if (!spans.length && !points.length) continue;
    const boundaries = new Set([start,end]);
    for (const entry of spans) {boundaries.add(Math.max(start,entry.range.start));boundaries.add(Math.min(end,entry.range.end));}
    for (const entry of points) boundaries.add(entry.range.start);
    const cuts = [...boundaries].sort((a,b) => a-b);
    const fragment = document.createDocumentFragment();
    for (let i = 0; i < cuts.length; i++) {
      const from = cuts[i], to = cuts[i+1];
      for (const entry of points.filter(entry => entry.range.start === from)) {
        const point = document.createElement('span');point.className = 'preview-source-point';point.setAttribute('aria-hidden','true');
        point.dataset.error = entry.number;entry.marks.push(point);fragment.append(point);
      }
      if (to === undefined || from === to) continue;
      const overlapping = spans.filter(entry => entry.range.start < to && entry.range.end > from);
      const text = document.createTextNode(node.data.slice(from-start,to-start));
      if (overlapping.length) {
        const mark = document.createElement('mark');mark.className = 'preview-source-error';
        mark.dataset.error = overlapping.map(entry => entry.number).join(' ');
        mark.setAttribute('aria-describedby', overlapping.map(entry => entry.note.id).join(' '));
        mark.title = overlapping.map(entry => entry.diagnostic.message).join('\n');
        mark.append(text);fragment.append(mark);
        overlapping.forEach(entry => entry.marks.push(mark));
      } else fragment.append(text);
    }
    node.replaceWith(fragment);
  }
}

function callout(entry, locate) {
  const {diagnostic,number} = entry;
  const note = document.createElement('button');note.type = 'button';note.className = `preview-callout preview-callout-${diagnostic.severity}`;
  note.id = `preview-error-${number}`;
  const badge = document.createElement('span');badge.className = 'preview-callout-number';badge.textContent = number;badge.setAttribute('aria-hidden','true');
  const location = document.createElement('span');location.className = 'preview-callout-location';
  location.textContent = diagnostic.range ? `Line ${diagnostic.line}, column ${diagnostic.column}` : diagnostic.phase || 'Program diagnostic';
  const message = document.createElement('span');message.className = 'preview-callout-message';message.textContent = diagnostic.message;
  note.append(badge,location,message);
  note.setAttribute('aria-label', `${diagnostic.severity === 'warning' ? 'Warning' : 'Error'} ${number}. ${location.textContent}. ${diagnostic.message}. Locate in source.`);
  if (diagnostic.range) note.onclick = () => locate(diagnostic);
  else note.disabled = true;
  const active = enabled => entry.marks.forEach(mark => mark.classList.toggle('preview-source-active',enabled));
  note.onmouseenter = () => active(true);note.onmouseleave = () => active(false);
  note.onfocus = () => active(true);note.onblur = () => active(false);
  return note;
}

function connectorPath(svg, points, entry) {
  const path = document.createElementNS(svgNamespace,'path');
  path.setAttribute('d',points);path.setAttribute('class',`preview-connector preview-connector-${entry.diagnostic.severity}`);
  path.setAttribute('marker-end',`url(#preview-arrow-${entry.number})`);
  const marker = document.createElementNS(svgNamespace,'marker');
  marker.id = `preview-arrow-${entry.number}`;marker.setAttribute('viewBox','0 0 6 6');marker.setAttribute('refX','5');marker.setAttribute('refY','3');marker.setAttribute('markerWidth','6');marker.setAttribute('markerHeight','6');marker.setAttribute('orient','auto');
  const head = document.createElementNS(svgNamespace,'path');head.setAttribute('d','M 0 0 L 6 3 L 0 6');head.setAttribute('class','preview-arrowhead');marker.append(head);
  svg.append(marker,path);
}

export function annotateDocumentPreview(preview, diagnostics, offsets, source, locate, tree) {
  const comments = tree.filter(row => !row.token && row.kind === 'Comment').map(row => ({start:offsets[row.range[0]],end:offsets[row.range[1]]}));
  const regions = [...preview.querySelectorAll('[data-mech-start][data-mech-end]')].map(element => ({element,start:offsets[Number(element.dataset.mechStart)],end:offsets[Number(element.dataset.mechEnd)]})).filter(region => Number.isInteger(region.start) && Number.isInteger(region.end) && region.end >= region.start);
  const groups = new Map();
  diagnostics.forEach((diagnostic,index) => {
    const region = diagnostic.range ? regions.filter(region => diagnostic.start >= region.start && diagnostic.end <= region.end).sort((a,b) => (a.end-a.start)-(b.end-b.start))[0] : null;
    const key = region?.element || preview;
    if (!groups.has(key)) groups.set(key,{region,entries:[],original:region?.element.innerHTML});
    groups.get(key).entries.push({diagnostic,number:index+1,marks:[]});
  });
  const cleanups = [];
  for (const {region,entries,original} of groups.values()) {
    const host = document.createElement('div');host.className = 'preview-annotations';
    const content = document.createElement('div');content.className = 'preview-annotation-content';
    const notes = document.createElement('aside');notes.className = 'preview-annotation-notes';notes.setAttribute('aria-label','Source error annotations');
    if (region) {
      content.append(...region.element.childNodes);
      region.element.append(host);host.append(content,notes);
    } else {host.classList.add('preview-annotations-unmapped');host.append(notes);preview.prepend(host);}
    const byCode = new Map();
    for (const entry of entries) {
      entry.note = callout(entry,locate);notes.append(entry.note);
      entry.range = region && displayedRange(content,region,entry.diagnostic,source,comments);
      if (entry.range) {
        const code = entry.range.code;
        if (!byCode.has(code)) byCode.set(code,[]);
        byCode.get(code).push(entry);
      }
    }
    for (const [code,mapped] of byCode) markRanges(code,mapped);
    const svg = document.createElementNS(svgNamespace,'svg');svg.classList.add('preview-connectors');svg.setAttribute('aria-hidden','true');host.append(svg);
    let frame;
    function draw() {
      svg.replaceChildren();
      const box = host.getBoundingClientRect();
      if (!box.width || !box.height || !region) return;
      svg.setAttribute('viewBox',`0 0 ${box.width} ${box.height}`);
      const contentBox = content.getBoundingClientRect();
      const stacked = notes.getBoundingClientRect().top >= contentBox.bottom - 1;
      for (const entry of entries) {
        const mark = entry.marks[0];
        const markBox = mark?.getClientRects()[0];
        const targetBox = markBox || content.querySelector('pre')?.getBoundingClientRect() || contentBox;
        const noteBox = entry.note.getBoundingClientRect();
        let targetX = markBox ? (targetBox.left + targetBox.right)/2 : targetBox.right - 8;
        let targetY = markBox ? targetBox.bottom + 3 : targetBox.top + 16;
        const visible = mark?.closest('pre')?.getBoundingClientRect() || contentBox;
        targetX = Math.max(visible.left + 3,Math.min(targetX,visible.right - 3));
        targetY = Math.max(visible.top,Math.min(targetY,visible.bottom + 3));
        targetX -= box.left;targetY -= box.top;
        if (stacked) {
          const fromX = noteBox.left - box.left, fromY = noteBox.top - box.top + 22;
          const leftRail = fromX - 8 - (entry.number % 3)*4;
          const rightRail = contentBox.right - box.left - 7 - (entry.number % 3)*4;
          const railY = contentBox.bottom - box.top + 10 + (entry.number % 3)*3;
          connectorPath(svg,`M ${fromX} ${fromY} H ${leftRail} V ${railY} H ${rightRail} V ${targetY} H ${targetX}`,entry);
        } else {
          const fromX = noteBox.left - box.left, fromY = noteBox.top - box.top + 22;
          const railX = contentBox.right - box.left + 9 + (entry.number % 3)*4;
          connectorPath(svg,`M ${fromX} ${fromY} H ${railX} V ${targetY} H ${targetX}`,entry);
        }
      }
    }
    const redraw = () => {cancelAnimationFrame(frame);frame=requestAnimationFrame(draw);};
    const observer = new ResizeObserver(redraw);observer.observe(host);observer.observe(content);observer.observe(notes);
    host.addEventListener('scroll',redraw,true);window.addEventListener('resize',redraw);
    content.addEventListener('click',event => {
      const number = Number(event.target.closest?.('[data-error]')?.dataset.error.split(' ')[0]);
      const entry = entries.find(entry => entry.number === number);
      if (entry) locate(entry.diagnostic);
    });
    redraw();
    cleanups.push(() => {
      cancelAnimationFrame(frame);observer.disconnect();window.removeEventListener('resize',redraw);host.removeEventListener('scroll',redraw,true);
      if (region?.element.isConnected && region.element.contains(host)) region.element.innerHTML = original;
      else host.remove();
    });
  }
  return () => cleanups.reverse().forEach(cleanup => cleanup());
}
