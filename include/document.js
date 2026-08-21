const controllerElement = document.querySelector(
  "script[data-mech-document-controller]",
);

const state = {
  controllerElement,
  document: null,
  initialEncoded: "",
  root: null,
  repl: null,
  running: false,
  animationFrame: null,
  history: [],
  historyIndex: 0,
  console: null,
  replInputAction: null,
  replStepLimit: null,
  replQuiet: false,
  replBusy: false,
  replTerminated: false,
  programDisplays: new Map(),
};

function truthySetting(value) {
  return typeof value === "string" &&
    !["", "0", "false", "no", "off"].includes(value.trim().toLowerCase());
}

function requestedReplQuiet() {
  return [
    state.controllerElement?.dataset.mechReplQuiet,
    state.root?.dataset.mechReplQuiet,
    document.documentElement?.dataset.mechReplQuiet,
  ].some(truthySetting);
}

function documentRoot() {
  return document.querySelector(".mech-root, .mech-document");
}

function statusTargets() {
  return [...new Set([document.documentElement, state.root].filter(Boolean))];
}

function setDocumentStatus(status, error) {
  for (const target of statusTargets()) {
    target.dataset.mechDocumentStatus = status;
    if (error) {
      target.dataset.mechDocumentError = error;
    } else {
      delete target.dataset.mechDocumentError;
    }
  }
}

function setConsoleStatus(status) {
  for (const target of statusTargets()) {
    target.dataset.mechConsoleStatus = status;
  }
}

function dispatch(name) {
  window.dispatchEvent(new CustomEvent(name));
}

function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}

function errorPanel() {
  return state.root?.querySelector(
    "#mech-document-errors, [data-mech-document-errors], [data-mech-errors-panel]",
  ) || document.querySelector(
    "#mech-document-errors, [data-mech-document-errors], [data-mech-errors-panel]",
  );
}

function errorRegion(kind) {
  const panel = errorPanel();
  if (!panel) {
    return null;
  }
  let region = panel.querySelector(`[data-mech-error-region="${kind}"]`);
  if (!region) {
    region = document.createElement("div");
    region.dataset.mechErrorRegion = kind;
    region.className = `mech-error-region mech-error-region-${kind}`;
    panel.append(region);
  }
  return region;
}

function outputPanel() {
  return state.root?.querySelector(
    "#mech-document-output, [data-mech-document-output], [data-mech-output-panel]",
  ) || document.querySelector(
    "#mech-document-output, [data-mech-document-output], [data-mech-output-panel]",
  );
}

function outputRegion(kind) {
  const panel = outputPanel();
  if (!panel) {
    return null;
  }
  let region = panel.querySelector(`[data-mech-output-region="${kind}"]`);
  if (!region) {
    region = document.createElement("div");
    region.dataset.mechOutputRegion = kind;
    region.className = `mech-output-region mech-output-region-${kind}`;
    panel.append(region);
  }
  return region;
}

function appendError(error, owner = "document") {
  const message = errorMessage(error);
  const panel = errorRegion(owner) || errorPanel();
  if (panel) {
    const row = document.createElement("div");
    row.className = "mech-console-error";
    row.setAttribute("role", "alert");
    row.textContent = message;
    panel.append(row);
    activateConsolePanel("errors");
    return message;
  }

  let alert = state.root?.querySelector("#mech-document-error") || null;
  if (!alert) {
    alert = document.createElement("pre");
    alert.id = "mech-document-error";
    alert.setAttribute("role", "alert");
    (state.root || document.body).prepend(alert);
  }
  alert.textContent = `Mech document failed to run: ${message}`;
  return message;
}

function appendDiagnostic(diagnostic) {
  const panel = errorRegion("repl");
  if (!panel) {
    return;
  }
  const row = document.createElement("article");
  row.className = "mech-console-error mech-repl-diagnostic";
  row.dataset.mechDiagnosticId = diagnostic.id || "";
  row.dataset.mechDiagnosticSeverity = diagnostic.severity || "error";
  const heading = document.createElement("header");
  heading.textContent = [diagnostic.severity, diagnostic.code]
    .filter(Boolean)
    .join(" ");
  const message = document.createElement("div");
  message.textContent = diagnostic.message || "Unknown REPL diagnostic";
  row.append(heading, message);
  for (const note of diagnostic.notes || []) {
    const detail = document.createElement("div");
    detail.className = "mech-repl-diagnostic-note";
    detail.textContent = `note: ${note.message}`;
    row.append(detail);
  }
  panel.append(row);
  activateConsolePanel("errors");
}

function cancelFrame() {
  if (state.animationFrame !== null) {
    cancelAnimationFrame(state.animationFrame);
    state.animationFrame = null;
  }
}

function stopRuntime() {
  state.running = false;
  cancelFrame();
  if (!state.document) {
    return;
  }
  try {
    state.document.stop();
  } catch (error) {
    console.error(error);
  }
}

function showFatalError(error) {
  stopRuntime();
  const message = appendError(error);
  setDocumentStatus("error", message);
  console.error(error);
}

function embeddedDocumentCode() {
  const selector =
    "script[type='application/x-mech-code'][data-mech-document-code], " +
    "[data-mech-document-code]";
  const element = state.root?.querySelector(selector) || document.querySelector(selector);
  return element?.textContent?.trim() || "";
}

async function loadEncodedDocument() {
  const sourceUrlKey =
    state.root?.dataset.mechSourceUrlKey ||
    document.documentElement?.dataset.mechSourceUrlKey ||
    "";
  let fetchFailure = null;

  if (sourceUrlKey.trim()) {
    try {
      const response = await fetch(`/code/${sourceUrlKey}`);
      if (response.ok) {
        return (await response.text()).trim();
      }
      fetchFailure = new Error(
        `failed to fetch compiled Mech document: ${response.status} ${response.statusText}`,
      );
    } catch (error) {
      fetchFailure = error;
    }
  }

  const embedded = embeddedDocumentCode();
  if (embedded) {
    return embedded;
  }
  if (fetchFailure) {
    throw fetchFailure;
  }
  throw new Error("the document has no embedded encoded Mech payload");
}

async function loadDocumentSourceMap() {
  const sourceUrlKey =
    state.root?.dataset.mechSourceUrlKey ||
    document.documentElement?.dataset.mechSourceUrlKey ||
    "";
  if (!sourceUrlKey.trim()) {
    return null;
  }

  const response = await fetch("/_mech/project-sources.json");
  if (response.status === 404) {
    return null;
  }
  if (!response.ok) {
    throw new Error(
      `failed to fetch served project source manifest: ${response.status} ${response.statusText}`,
    );
  }

  let manifest;
  try {
    manifest = await response.json();
  } catch {
    throw new Error("invalid served project source manifest");
  }
  if (
    (manifest?.version !== 1 && manifest?.version !== 2) ||
    !Array.isArray(manifest.sources) ||
    (manifest.version === 2 &&
      (!Array.isArray(manifest.roots) ||
        !Array.isArray(manifest.resolutions))) ||
    manifest.sources.some(
      source =>
        typeof source?.specifier !== "string" ||
        typeof source?.url !== "string" ||
        !source.url.startsWith("source/"),
    )
  ) {
    throw new Error("invalid served project source manifest");
  }

  const root = manifest.sources.find(
    source => source.url === `source/${sourceUrlKey}`,
  );
  if (!root) {
    throw new Error(
      `served document source \`${sourceUrlKey}\` is missing from the project source manifest`,
    );
  }

  const sourceSpecifiers = new Set(
    manifest.sources.map(source => source.specifier),
  );
  if (
    manifest.version === 2 &&
    manifest.roots.some(
      specifier =>
        typeof specifier !== "string" ||
        !specifier.trim() ||
        !sourceSpecifiers.has(specifier),
    )
  ) {
    throw new Error("invalid served project root source identity");
  }
  const resolutions = [];
  const resolutionTargets = new Map();
  for (const resolution of manifest.version === 2 ? manifest.resolutions : []) {
    if (
      typeof resolution?.referrer !== "string" ||
      !resolution.referrer.trim() ||
      typeof resolution?.specifier !== "string" ||
      !resolution.specifier.trim() ||
      typeof resolution?.target !== "string" ||
      !resolution.target.trim() ||
      !sourceSpecifiers.has(resolution.referrer) ||
      !sourceSpecifiers.has(resolution.target)
    ) {
      throw new Error("invalid served project source resolution");
    }
    const key = JSON.stringify([resolution.referrer, resolution.specifier]);
    const existing = resolutionTargets.get(key);
    if (existing !== undefined && existing !== resolution.target) {
      throw new Error("conflicting served project source resolution");
    }
    if (existing === undefined) {
      resolutionTargets.set(key, resolution.target);
      resolutions.push({
        referrer: resolution.referrer,
        specifier: resolution.specifier,
        target: resolution.target,
      });
    }
  }

  const hasServedAuthority = Object.prototype.hasOwnProperty.call(
    window,
    "__MECH_HOST_CONFIG",
  );
  const [config, sourceEntries] = await Promise.all([
    hasServedAuthority
      ? (async () => {
          const configResponse = await fetch("/mech.mcfg");
          if (configResponse.status === 404) {
            return null;
          }
          if (!configResponse.ok) {
            throw new Error(
              `failed to fetch served project configuration: ${configResponse.status} ${configResponse.statusText}`,
            );
          }
          return configResponse.text();
        })()
      : Promise.resolve(null),
    Promise.all(
      manifest.sources.map(async source => {
        const sourceResponse = await fetch(`/${source.url}`);
        if (!sourceResponse.ok) {
          throw new Error(
            `failed to fetch served project source \`${source.specifier}\`: ` +
              `${sourceResponse.status} ${sourceResponse.statusText}`,
          );
        }
        return [source.specifier, await sourceResponse.text()];
      }),
    ),
  ]);

  return {
    version: manifest.version,
    config,
    rootSpecifier: root.specifier,
    sources: Object.fromEntries(sourceEntries),
    resolutions,
  };
}

function loadEmbeddedDocumentSourceBundle() {
  const selector = "script[type='application/x-mech-source-bundle'][data-mech-document-sources]";
  const element =
    state.controllerElement?.parentElement?.querySelector(selector) ||
    document.querySelector(selector);
  const encoded = element?.textContent?.trim();
  if (!encoded) {
    return null;
  }

  let bundle;
  try {
    const bytes = Uint8Array.from(atob(encoded), byte => byte.charCodeAt(0));
    bundle = JSON.parse(new TextDecoder().decode(bytes));
  } catch {
    throw new Error("invalid embedded Mech document source bundle");
  }

  if (
    (bundle?.version !== 1 && bundle?.version !== 2) ||
    typeof bundle.rootSpecifier !== "string" ||
    !bundle.rootSpecifier.trim() ||
    !Array.isArray(bundle.sources) ||
    (bundle.version === 2 && !Array.isArray(bundle.resolutions))
  ) {
    throw new Error("invalid embedded Mech document source bundle");
  }

  const sources = {};
  for (const source of bundle.sources) {
    if (
      typeof source?.specifier !== "string" ||
      !source.specifier.trim() ||
      typeof source?.source !== "string" ||
      Object.prototype.hasOwnProperty.call(sources, source.specifier)
    ) {
      throw new Error("invalid embedded Mech document source bundle");
    }
    sources[source.specifier] = source.source;
  }
  if (!Object.prototype.hasOwnProperty.call(sources, bundle.rootSpecifier)) {
    throw new Error("embedded Mech document root is missing from its source bundle");
  }

  const resolutions = [];
  const resolutionTargets = new Map();
  for (const resolution of bundle.version === 2 ? bundle.resolutions : []) {
    if (
      typeof resolution?.referrer !== "string" ||
      !resolution.referrer.trim() ||
      typeof resolution?.specifier !== "string" ||
      !resolution.specifier.trim() ||
      typeof resolution?.target !== "string" ||
      !resolution.target.trim() ||
      !Object.prototype.hasOwnProperty.call(sources, resolution.referrer) ||
      !Object.prototype.hasOwnProperty.call(sources, resolution.target)
    ) {
      throw new Error("invalid embedded Mech document source resolution");
    }
    const key = JSON.stringify([resolution.referrer, resolution.specifier]);
    const existing = resolutionTargets.get(key);
    if (existing !== undefined && existing !== resolution.target) {
      throw new Error("conflicting embedded Mech document source resolution");
    }
    if (existing === resolution.target) {
      continue;
    }
    resolutionTargets.set(key, resolution.target);
    resolutions.push({
      referrer: resolution.referrer,
      specifier: resolution.specifier,
      target: resolution.target,
    });
  }

  return {
    config: null,
    version: bundle.version,
    rootSpecifier: bundle.rootSpecifier,
    sources,
    resolutions,
  };
}

function outputAddress(element) {
  const separator = element.id.lastIndexOf(":");
  if (separator <= 0 || separator === element.id.length - 1) {
    throw new Error(`invalid Mech output address \`${element.id}\``);
  }
  return {
    outputId: BigInt(element.id.slice(0, separator)),
  };
}

function prepareVarPlaceholders() {
  const root = state.root;
  if (!root) {
    return;
  }
  const pattern = /\{\{VAR:([^@}\s]+)\}\}/g;
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  const candidates = [];
  while (walker.nextNode()) {
    const node = walker.currentNode;
    if (
      node.nodeValue.includes("{{VAR:") &&
      !node.parentElement?.closest("script, style, code, pre, textarea")
    ) {
      candidates.push(node);
    }
  }

  for (const node of candidates) {
    const text = node.nodeValue;
    const fragment = document.createDocumentFragment();
    let cursor = 0;
    let match;
    while ((match = pattern.exec(text)) !== null) {
      fragment.append(text.slice(cursor, match.index));
      const placeholder = document.createElement("span");
      placeholder.className = "mech-var-placeholder";
      placeholder.dataset.mechVarName = match[1];
      fragment.append(placeholder);
      cursor = pattern.lastIndex;
    }
    fragment.append(text.slice(cursor));
    node.replaceWith(fragment);
  }
}

function createOutputEntry(address, rendered) {
  const row = document.createElement("article");
  row.className = "mech-document-output-entry";
  row.dataset.mechOutputId = address.outputId.toString();
  row.dataset.mechRenderedKind = rendered.kind;

  const heading = document.createElement("header");
  heading.className = "mech-document-output-heading";
  const name = document.createElement("span");
  name.className = "mech-document-output-name";
  name.textContent = rendered.name || "output";
  const kind = document.createElement("span");
  kind.className = "mech-output-kind";
  kind.textContent = rendered.kind;
  heading.append(name, kind);
  const body = document.createElement("div");
  body.className = "mech-document-output-html mech-output-value";
  body.innerHTML = rendered.blockHtml;
  row.append(heading, body);
  return row;
}

function refreshOutputPanel(entries) {
  const panel = outputRegion("document");
  if (!panel) {
    return;
  }
  panel.replaceChildren();
  for (const entry of entries) {
    panel.append(createOutputEntry(entry.address, entry.rendered));
  }
}

function outputContentElement(content) {
  const body = document.createElement("div");
  body.className = `mech-repl-output-content mech-repl-output-${content?.kind || "unknown"}`;
  const data = content?.data || {};
  if (content?.kind === "fragments") {
    for (const fragment of Array.isArray(data) ? data : []) {
      body.append(outputContentElement(fragment));
    }
    return body;
  }
  if (content?.kind === "text") {
    body.textContent = data.text || "";
    return body;
  }
  if (content?.kind === "value") {
    body.textContent = data.text || data.inline_text || "_";
    return body;
  }
  if (content?.kind === "table") {
    const table = document.createElement("table");
    table.className = "mech-repl-output-table";
    const head = document.createElement("thead");
    const headRow = document.createElement("tr");
    for (const column of data.columns || []) {
      const cell = document.createElement("th");
      cell.textContent = column;
      headRow.append(cell);
    }
    head.append(headRow);
    const tableBody = document.createElement("tbody");
    const mutedRows = new Set(data.muted_rows || []);
    for (const [rowIndex, values] of (data.rows || []).entries()) {
      const row = document.createElement("tr");
      if (mutedRows.has(rowIndex)) {
        row.classList.add("mech-repl-row-muted");
      }
      for (const value of values) {
        const cell = document.createElement("td");
        cell.textContent = value;
        row.append(cell);
      }
      tableBody.append(row);
    }
    table.append(head, tableBody);
    body.append(table);
    return body;
  }
  if (content?.kind === "matrix") {
    const rows = [];
    const width = Math.max(1, data.columns || 0);
    for (let index = 0; index < (data.cells || []).length; index += width) {
      rows.push(data.cells.slice(index, index + width).join(" "));
    }
    body.textContent = `[${rows.join("; ")}]`;
    return body;
  }
  const representations = data.representations?.representations || data.representations || [];
  if (content?.kind === "scene") {
    const sceneRepresentation = representations.find(
      entry => entry.media_type === "application/vnd.mech.scene+json",
    );
    const encoded = sceneRepresentation?.data?.value;
    if (typeof encoded === "string") {
      try {
        const scene = JSON.parse(encoded);
        const namespace = "http://www.w3.org/2000/svg";
        const svg = document.createElementNS(namespace, "svg");
        svg.classList.add("mech-repl-scene");
        svg.dataset.mechRichScene = "true";
        svg.setAttribute("viewBox", `0 0 ${scene.width} ${scene.height}`);
        svg.setAttribute("width", String(scene.width));
        svg.setAttribute("height", String(scene.height));
        svg.setAttribute("role", "img");
        const background = document.createElementNS(namespace, "rect");
        background.setAttribute("x", "0");
        background.setAttribute("y", "0");
        background.setAttribute("width", String(scene.width));
        background.setAttribute("height", String(scene.height));
        background.setAttribute("fill", scene.background || "transparent");
        svg.append(background);
        for (const circle of scene.circles || []) {
          const element = document.createElementNS(namespace, "circle");
          element.dataset.mechSceneId = circle.id;
          for (const [name, value] of [
            ["cx", circle.x], ["cy", circle.y], ["r", circle.radius],
            ["fill", circle.fill], ["stroke", circle.stroke],
            ["stroke-width", circle.stroke_width], ["opacity", circle.opacity],
          ]) {
            element.setAttribute(name, String(value));
          }
          svg.append(element);
        }
        for (const line of scene.lines || []) {
          const element = document.createElementNS(namespace, "line");
          element.dataset.mechSceneId = line.id;
          for (const [name, value] of [
            ["x1", line.x1], ["y1", line.y1], ["x2", line.x2], ["y2", line.y2],
            ["stroke", line.stroke], ["stroke-width", line.stroke_width],
            ["stroke-linecap", line.line_cap], ["opacity", line.opacity],
          ]) {
            element.setAttribute(name, String(value));
          }
          element.setAttribute(
            "transform",
            `rotate(${line.rotation} ${line.origin_x} ${line.origin_y})`,
          );
          svg.append(element);
        }
        body.append(svg);
        return body;
      } catch (error) {
        console.error("failed to render Mech scene output", error);
      }
    }
  }
  const fallback = representations.find(entry => entry.media_type === "text/plain");
  body.textContent = fallback?.data?.value || data.alt_text || `${content?.kind || "rich"} output`;
  return body;
}

function appendProgramOutput(output) {
  if (output.operation === "clear" && !output.display_id) {
    for (const entry of state.programDisplays.values()) {
      entry.element.remove();
    }
    state.programDisplays.clear();
    outputRegion("repl")?.replaceChildren();
    errorRegion("program")?.replaceChildren();
    return;
  }
  const stderr = output.stream === "stderr";
  const target = stderr ? errorRegion("program") : outputRegion("repl");
  const displayId = typeof output.display_id === "string"
    ? output.display_id
    : output.display_id?.[0] || null;
  let entry = displayId ? state.programDisplays.get(displayId) || null : null;
  if (output.operation === "remove" && displayId) {
    entry?.element.remove();
    state.programDisplays.delete(displayId);
    return;
  }
  if (!target) {
    entry?.element.remove();
    if (displayId) {
      state.programDisplays.delete(displayId);
    }
    return;
  }
  let row = entry?.element || null;
  if (!row) {
    row = document.createElement("article");
    if (displayId) {
      row.dataset.mechDisplayId = displayId;
      entry = { element: row, currentStream: output.stream, currentRegion: target };
      state.programDisplays.set(displayId, entry);
    }
  }
  row.className = stderr
    ? "mech-repl-output-entry mech-console-error mech-program-stderr"
    : "mech-repl-output-entry";
  row.dataset.mechDisplayOperation = output.operation;
  if (output.operation === "update") {
    row.dataset.mechDisplayUpdates = String(
      Number(row.dataset.mechDisplayUpdates || "0") + 1,
    );
  }
  if (row.parentElement !== target) {
    target.append(row);
  }
  if (entry) {
    entry.currentStream = output.stream;
    entry.currentRegion = target;
  }
  if (output.operation === "append" && row.firstElementChild) {
    const next = outputContentElement(output.content);
    const existing = row.firstElementChild;
    if (
      output.content?.kind === "text" &&
      existing?.classList.contains("mech-repl-output-text")
    ) {
      existing.textContent += output.content.data?.text || "";
    } else {
      row.append(next);
    }
    target.scrollTop = target.scrollHeight;
    activateConsolePanel(stderr ? "errors" : "output");
    return;
  }
  row.replaceChildren(outputContentElement(output.content));
  target.scrollTop = target.scrollHeight;
  activateConsolePanel(stderr ? "errors" : "output");
}

function focusProgramDisplay(payload) {
  const displayId = typeof payload?.display_id === "string"
    ? payload.display_id
    : payload?.display_id?.[0] || null;
  let entry = displayId ? state.programDisplays.get(displayId) || null : null;
  const stream = entry?.currentStream || payload?.stream || "stdout";
  if (!entry && displayId && payload?.content) {
    appendProgramOutput({
      stream,
      display_id: displayId,
      operation: "replace",
      content: payload.content,
    });
    entry = state.programDisplays.get(displayId) || null;
  }
  activateConsolePanel((entry?.currentStream || stream) === "stderr" ? "errors" : "output");
  entry?.element.scrollIntoView({ block: "nearest" });
}

function consoleIsOpen() {
  return state.root?.dataset.mechConsoleOpen !== "false";
}

function showInlinePopup(title, rendered) {
  document.querySelector(".mech-inline-popup[data-mech-repl-popup]")?.remove();
  const popup = document.createElement("aside");
  popup.className = "mech-inline-popup";
  popup.dataset.mechReplPopup = "true";
  const header = document.createElement("header");
  header.className = "mech-inline-popup__header";
  const heading = document.createElement("strong");
  heading.className = "mech-inline-popup__title";
  heading.textContent = title || "ans";
  const close = document.createElement("button");
  close.className = "mech-inline-popup__close";
  close.type = "button";
  close.setAttribute("aria-label", "Close value inspector");
  close.textContent = "×";
  close.addEventListener("click", () => popup.remove());
  const content = document.createElement("div");
  content.className = "mech-inline-popup__content";
  const kind = document.createElement("div");
  kind.className = "mech-output-kind";
  kind.textContent = rendered?.kind || "";
  const value = document.createElement("div");
  value.className = "mech-output-value";
  value.innerHTML = rendered?.blockHtml || rendered?.inlineHtml || "";
  content.append(kind, value);
  header.append(heading, close);
  popup.append(header, content);
  document.body.append(popup);
}

function consumeSelection(response, title, rendered) {
  consumeReplResponse(response);
  renderValues();
  if (consoleIsOpen()) {
    activateConsolePanel("console");
    state.console?.input?.focus();
  } else {
    showInlinePopup(title, rendered);
  }
}

function bindSymbolClick(element, name) {
  if (!name || element.dataset.mechReplBound === "true") {
    return;
  }
  element.dataset.mechReplBound = "true";
  element.dataset.mechVarName = name;
  element.classList.add("mech-clickable");
  element.tabIndex = 0;
  element.setAttribute("role", "button");
  const select = (event) => {
    event.preventDefault();
    event.stopPropagation();
    try {
      const rendered = state.document.renderedSymbol(name);
      consumeSelection(state.repl.selectSymbol(name), name, rendered);
    } catch (error) {
      appendConsoleError(error);
    }
  };
  element.addEventListener("click", select);
  element.addEventListener("keydown", event => {
    if (event.key === "Enter" || event.key === " ") {
      select(event);
    }
  });
}

function bindOutputClick(element, address) {
  if (element.dataset.mechReplBound === "true") {
    return;
  }
  element.dataset.mechReplBound = "true";
  element.classList.add("mech-clickable");
  element.tabIndex = 0;
  element.setAttribute("role", "button");
  const select = (event) => {
    event.preventDefault();
    event.stopPropagation();
    try {
      const rendered = state.document.renderedOutput(address.outputId);
      if (!rendered) {
        throw new Error("document output is not resident");
      }
      consumeSelection(
        state.repl.selectOutput(address.outputId),
        rendered.name || "ans",
        rendered,
      );
    } catch (error) {
      appendConsoleError(error);
    }
  };
  element.addEventListener("click", select);
  element.addEventListener("keydown", event => {
    if (event.key === "Enter" || event.key === " ") {
      select(event);
    }
  });
}

function renderInlineValue(output, address, rendered) {
  const probe = document.createElement("span");
  probe.innerHTML = rendered.inlineHtml;
  const plain = probe.textContent || "";
  output.replaceChildren();
  if (plain.length > 80) {
    const preview = document.createElement("span");
    preview.className = "mech-inline-preview";
    preview.textContent = `${plain.slice(0, 77)}…`;
    const expand = document.createElement("span");
    expand.className = "mech-inline-expand";
    expand.setAttribute("aria-label", "Inspect complete value as ans");
    expand.textContent = " ›";
    output.append(preview, expand);
  } else {
    output.innerHTML = rendered.inlineHtml;
  }
  bindOutputClick(output, address);
}

function bindReflectiveValues() {
  for (const placeholder of state.root?.querySelectorAll(".mech-var-placeholder") || []) {
    bindSymbolClick(placeholder, placeholder.dataset.mechVarName);
  }
  for (const variable of state.root?.querySelectorAll(".mech-var-name") || []) {
    if (variable.closest("#mech-console, .console-pane")) {
      continue;
    }
    bindSymbolClick(variable, variable.dataset.mechVarName || variable.textContent.trim());
  }
  for (const entry of outputRegion("document")?.querySelectorAll("[data-mech-output-id]") || []) {
    try {
      const outputId = BigInt(entry.dataset.mechOutputId);
      const rendered = state.document.renderedOutput(outputId);
      if (rendered) {
        bindOutputClick(entry, { outputId });
      }
    } catch (error) {
      appendError(error);
    }
  }
}

function renderValues() {
  if (!state.document) {
    return;
  }
  const outputEntries = [];
  for (const output of state.root?.querySelectorAll(".mech-block-output[id]") || []) {
    try {
      const address = outputAddress(output);
      const rendered = state.document.renderedOutput(address.outputId);
      if (rendered !== null) {
        output.innerHTML = rendered.blockHtml;
        bindOutputClick(output, address);
        outputEntries.push({ address, rendered });
      }
    } catch (error) {
      appendError(error);
    }
  }
  for (const output of state.root?.querySelectorAll(".mech-inline-mech-code[id]") || []) {
    try {
      const address = outputAddress(output);
      const rendered = state.document.renderedOutput(address.outputId);
      if (rendered !== null) {
        renderInlineValue(output, address, rendered);
      }
    } catch (error) {
      appendError(error);
    }
  }
  for (const placeholder of state.root?.querySelectorAll(".mech-var-placeholder") || []) {
    try {
      const rendered = state.document.renderedSymbol(placeholder.dataset.mechVarName);
      if (rendered !== null) {
        placeholder.innerHTML = rendered.inlineHtml;
      }
    } catch (error) {
      appendError(error);
    }
  }
  refreshOutputPanel(outputEntries);
  bindReflectiveValues();
  dispatch("mech:document-rendered");
}

function transcript() {
  return state.console?.transcript || null;
}

function appendToTranscript(row) {
  const target = transcript();
  if (!target) {
    return;
  }
  const activePrompt = state.console?.inputRow || null;
  target.insertBefore(row, activePrompt?.parentElement === target ? activePrompt : null);
  target.scrollTop = target.scrollHeight;
}

function appendRenderedResult(rendered) {
  const target = transcript();
  if (!target || !rendered) {
    return;
  }
  const row = document.createElement("div");
  row.className = "mech-repl-entry mech-repl-result";
  row.dataset.mechResultKind = rendered.kind;
  const kind = document.createElement("span");
  kind.className = "mech-repl-result-kind";
  kind.textContent = rendered.kind;
  const value = document.createElement("span");
  value.className = "mech-repl-result-value";
  value.innerHTML = rendered.inlineHtml;
  row.append(kind, value);
  appendToTranscript(row);
}

function appendTranscriptRow(className, text) {
  const target = transcript();
  if (!target) {
    return null;
  }
  const row = document.createElement("div");
  row.className = `mech-repl-entry ${className}`;
  row.textContent = text;
  appendToTranscript(row);
  return row;
}

function appendConsoleError(error) {
  appendTranscriptRow("mech-repl-error", errorMessage(error));
  appendError(error, "repl");
}

function clearReplDiagnostics() {
  errorRegion("repl")?.replaceChildren();
}

function clearTranscript() {
  const target = transcript();
  if (!target) {
    return;
  }
  const activePrompt = state.console?.inputRow || null;
  target.replaceChildren(...(activePrompt ? [activePrompt] : []));
}

function appendSourceEcho(source) {
  const pending = state.console?.pendingSubmission;
  if (pending && pending.source.trimEnd() === source.trimEnd()) {
    state.console.pendingSubmission = null;
    return pending.row;
  }
  const row = document.createElement("div");
  row.className = "repl-line mech-repl-entry mech-repl-source";
  const prompt = document.createElement("span");
  prompt.className = "repl-prompt";
  prompt.textContent = ">:";
  const code = document.createElement("span");
  code.className = "repl-code";
  code.textContent = source;
  row.append(prompt, code);
  appendToTranscript(row);
  return row;
}

function consumeReplResponse(response) {
  for (const envelope of response?.events || []) {
    const event = envelope.event || {};
    if (event.channel === "output") {
      appendProgramOutput(event.event);
      continue;
    }
    if (event.channel === "diagnostic") {
      appendDiagnostic(event.event);
      continue;
    }
    if (event.channel !== "repl") {
      continue;
    }
    const repl = event.event || {};
    if (repl.kind === "source_echo") {
      appendSourceEcho(repl.payload?.source || "");
      continue;
    }
    if (repl.kind === "clear") {
      if (repl.payload === "interaction") {
        clearTranscript();
      }
      if (repl.payload === "diagnostics") {
        clearReplDiagnostics();
      }
      continue;
    }
    if (repl.kind === "focus_display") {
      focusProgramDisplay(repl.payload);
      continue;
    }
    if (repl.kind !== "response") {
      continue;
    }
    const content = repl.payload?.content;
    if (content) {
      const row = document.createElement("div");
      row.className = `mech-repl-entry mech-repl-response mech-repl-${repl.payload?.status || "neutral"}`;
      if (repl.payload?.kind === "value_inspection") {
        row.classList.add("mech-repl-result");
      } else if (content.kind === "text") {
        row.classList.add("mech-repl-info");
      }
      if (repl.payload?.title) {
        const heading = document.createElement("strong");
        heading.className = "mech-repl-response-title";
        heading.textContent = repl.payload.title;
        row.append(heading);
      }
      const rendered = outputContentElement(content);
      if (repl.payload?.kind === "help") {
        rendered.querySelector("table")?.classList.add("mech-repl-help");
      }
      if (repl.payload?.kind === "symbol_inspection") {
        rendered.querySelector("table")?.classList.add("mech-repl-symbols");
      }
      row.append(rendered);
      appendToTranscript(row);
    }
  }
  appendRenderedResult(response?.result);
}

function nextBrowserTurn() {
  return new Promise(resolve => requestAnimationFrame(() => resolve()));
}

async function fulfillReplHostRequest(request) {
  if (request?.kind !== "documentation") {
    throw new Error(`unsupported browser REPL host request: ${request?.kind || "unknown"}`);
  }
  const topic = request.data?.topic?.trim() || "";
  const parts = topic.split("/").filter(Boolean);
  if (parts.length !== 2 || parts.some(part => !/^[A-Za-z0-9._-]+$/.test(part))) {
    throw new Error("Usage: :docs <machine>/<document>");
  }
  const [machine, documentName] = parts;
  const url =
    `https://raw.githubusercontent.com/mech-machines/${encodeURIComponent(machine)}` +
    `/main/docs/${encodeURIComponent(documentName)}.mec`;
  const fetched = await fetch(url);
  if (!fetched.ok) {
    throw new Error(
      `failed to load documentation \`${topic}\`: ${fetched.status} ${fetched.statusText}`,
    );
  }
  const loaded = state.document.replLoadDocumentation(topic, await fetched.text());
  consumeReplResponse(loaded.response);
  const panel = outputRegion("repl");
  if (panel) {
    const row = document.createElement("article");
    row.className = "mech-repl-output-entry mech-repl-documentation";
    row.dataset.mechDocumentationTopic = topic;
    row.innerHTML = loaded.html;
    panel.append(row);
    activateConsolePanel("output");
    panel.scrollTop = panel.scrollHeight;
  }
  renderValues();
}

async function consumeCooperativeResponse(response) {
  consumeReplResponse(response);
  if (response?.hostRequest) {
    await fulfillReplHostRequest(response.hostRequest);
  }
  state.replTerminated = Boolean(response?.terminated);
  if (state.replTerminated && state.console?.input) {
    state.console.input.disabled = true;
    setConsoleStatus("terminated");
  }
  state.replBusy = Boolean(response?.pending);
  if (state.console?.input) {
    state.console.input.disabled = state.replBusy || state.replTerminated;
  }
  try {
    while (response?.pending) {
      await nextBrowserTurn();
      response = state.repl.continueStep(128);
      consumeReplResponse(response);
      state.replTerminated = Boolean(response?.terminated);
    }
  } finally {
    state.replBusy = false;
    if (state.console?.input && !state.replTerminated) {
      state.console.input.disabled = false;
    }
    renderValues();
  }
}

function runConsoleCommand(source) {
  const input = source.trim();
  if (!input) {
    return;
  }
  return consumeCooperativeResponse(state.repl.invoke(source));
}

function submitConsoleInput(value, row, input) {
  const source = value.trim();
  if (!source) {
    return;
  }
  state.history.push(source);
  state.historyIndex = state.history.length;
  const code = document.createElement("span");
  code.className = "repl-code";
  code.textContent = value;
  input.replaceWith(code);
  row.classList.remove("mech-repl-active-prompt");
  row.classList.add("mech-repl-source");
  state.console.pendingSubmission = { source, row };
  appendActivePrompt();
  try {
    const result = runConsoleCommand(source);
    if (result && typeof result.catch === "function") {
      result.catch(appendConsoleError);
    }
  } catch (error) {
    appendConsoleError(error);
  }
}

function resolveReplInput(event) {
  if (state.replInputAction) {
    return state.replInputAction(
      event.key,
      event.ctrlKey,
      event.altKey,
      event.shiftKey,
      event.metaKey,
    );
  }
  if (event.key !== "Enter") {
    return null;
  }
  return event.ctrlKey ? "insert_line_break" : "submit";
}

function insertLineBreak(input) {
  const start = input.selectionStart ?? input.value.length;
  const end = input.selectionEnd ?? start;
  input.setRangeText("\n", start, end, "end");
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

function appendActivePrompt() {
  const target = transcript();
  if (!target || !state.console) {
    return;
  }
  const inputRow = document.createElement("div");
  inputRow.className = "repl-line mech-repl-input-row mech-repl-active-prompt";
  const prompt = document.createElement("span");
  prompt.className = "repl-prompt";
  prompt.textContent = ">:";
  const input = document.createElement("textarea");
  input.className = "repl-input";
  input.dataset.mechInteractiveEvaluation = "resident";
  input.setAttribute("aria-label", "Mech resident REPL input");
  input.placeholder = "Enter submits · Ctrl+Enter adds a line";
  input.addEventListener("keydown", (event) => {
    if (event.isComposing) {
      return;
    }
    if (state.replBusy && event.ctrlKey && event.key.toLowerCase() === "c") {
      event.preventDefault();
      consumeReplResponse(state.repl.interrupt());
      return;
    }
    const action = resolveReplInput(event);
    if (action === "insert_line_break") {
      event.preventDefault();
      insertLineBreak(input);
      return;
    }
    if (action === "submit") {
      event.preventDefault();
      submitConsoleInput(input.value, inputRow, input);
      return;
    }
    if (event.key === "ArrowUp" && state.history.length) {
      event.preventDefault();
      state.historyIndex = Math.max(0, state.historyIndex - 1);
      input.value = state.history[state.historyIndex];
      return;
    }
    if (event.key === "ArrowDown" && state.history.length) {
      event.preventDefault();
      state.historyIndex = Math.min(state.history.length, state.historyIndex + 1);
      input.value = state.history[state.historyIndex] || "";
    }
  });
  inputRow.append(prompt, input);
  target.append(inputRow);
  state.console.inputRow = inputRow;
  state.console.input = input;
  target.scrollTop = target.scrollHeight;
  input.focus();
}

function attachConsole() {
  const mount = state.root?.querySelector("#mech-output");
  if (!mount || state.console) {
    return;
  }
  mount.replaceChildren();
  mount.classList.remove("hidden");
  const transcriptElement = document.createElement("div");
  transcriptElement.className = "mech-repl-transcript";
  transcriptElement.setAttribute("aria-live", "polite");
  mount.append(transcriptElement);
  state.console = {
    mount,
    transcript: transcriptElement,
    input: null,
    inputRow: null,
    pendingSubmission: null,
  };
  appendActivePrompt();
  setConsoleStatus("ready");
  dispatch("mech:console-ready");
}

function documentConsolePane() {
  if (!state.root) {
    return null;
  }
  return state.root.querySelector("[data-mech-console-pane], #mech-console") ||
    state.root.querySelector(".console-pane");
}

function documentConsoleResizers() {
  const root = state.root;
  const pane = documentConsolePane();
  if (!root) {
    return [];
  }
  return [...new Set([
    ...root.querySelectorAll("[data-mech-console-resizer]"),
    ...root.querySelectorAll("#resizer, #edgeHandle"),
    ...(pane ? pane.querySelectorAll(".resize-handle") : []),
  ])];
}

function documentConsoleToggles() {
  if (!state.root) {
    return [];
  }
  return [...new Set(state.root.querySelectorAll(
    "[data-mech-console-toggle], #toggle-repl",
  ))];
}

function documentConsoleFullscreenControls() {
  const root = state.root;
  const pane = documentConsolePane();
  if (!root) {
    return [];
  }
  return [...new Set([
    ...root.querySelectorAll("[data-mech-console-fullscreen], #consoleFullscreenToggle"),
    ...(pane ? pane.querySelectorAll("[data-mech-console-fullscreen]") : []),
  ])];
}

function setConsoleOpen(open) {
  const pane = documentConsolePane();
  if (pane) {
    pane.hidden = !open;
    pane.classList.toggle("hidden", !open);
    pane.classList.toggle("is-collapsed", !open);
  }
  state.root?.setAttribute("data-mech-console-open", String(open));
  for (const toggle of documentConsoleToggles()) {
    toggle.setAttribute("aria-expanded", String(open));
  }
}

function initializeConsoleState() {
  if (!state.root) {
    return;
  }
  const requested = state.root.dataset.mechConsoleOpen;
  if (requested !== undefined) {
    setConsoleOpen(requested !== "false");
    return;
  }
  const pane = documentConsolePane();
  const visible = pane && getComputedStyle(pane).display !== "none";
  setConsoleOpen(Boolean(visible));
}

function panelFor(name, pane = documentConsolePane()) {
  if (!pane) {
    return null;
  }
  const known = {
    console: "#mech-output",
    output: "#mech-document-output",
    errors: "#mech-document-errors",
  };
  const target = pane.querySelector(
    `[data-mech-console-panel="${name}"], [data-panel="${name}"], ${known[name] || "[data-mech-console-panel]"}`,
  );
  return target?.closest(".console-panel, [data-mech-console-panel], [data-panel]") || target;
}

function activateConsolePanel(name, pane = documentConsolePane()) {
  const panel = panelFor(name, pane);
  if (!pane || !panel) {
    return;
  }
  for (const candidate of pane.querySelectorAll(
    ".console-panel, [data-mech-console-panel], [data-panel]",
  )) {
    const selected = candidate === panel;
    candidate.hidden = !selected;
    candidate.classList.toggle("active", selected);
    candidate.classList.toggle("is-active", selected);
  }
  for (const tab of pane.querySelectorAll(
    ".console-tab, [data-mech-console-tab], [data-tab]",
  )) {
    const selected = (tab.dataset.mechConsoleTab || tab.dataset.tab) === name;
    tab.classList.toggle("active", selected);
    tab.setAttribute("aria-selected", String(selected));
  }
}

function initializeConsoleTabs() {
  const pane = documentConsolePane();
  if (!pane) {
    return;
  }
  for (const tab of pane.querySelectorAll(
    ".console-tab, [data-mech-console-tab], [data-tab]",
  )) {
    tab.addEventListener("click", () => {
      const name = tab.dataset.mechConsoleTab || tab.dataset.tab;
      if (name) {
        activateConsolePanel(name, pane);
      }
    });
  }
}

function initializeConsoleToggle() {
  for (const toggle of documentConsoleToggles()) {
    toggle.addEventListener("click", () => {
      const isOpen = state.root?.dataset.mechConsoleOpen !== "false";
      setConsoleOpen(!isOpen);
    });
  }
}

function initializeConsoleKeyboardToggle() {
  document.addEventListener("keydown", event => {
    if (
      event.isComposing ||
      event.key !== "`" ||
      event.ctrlKey ||
      event.altKey ||
      event.shiftKey ||
      event.metaKey
    ) {
      return;
    }
    event.preventDefault();
    const isOpen = state.root?.dataset.mechConsoleOpen !== "false";
    setConsoleOpen(!isOpen);
    if (isOpen) {
      return;
    }
    activateConsolePanel("console");
    requestAnimationFrame(() => state.console?.input?.focus());
  });
}

function initializeResizeHandles() {
  const pane = documentConsolePane();
  if (!pane || !state.root) {
    return;
  }
  for (const handle of documentConsoleResizers()) {
    handle.addEventListener("pointerdown", (event) => {
      event.preventDefault();
      let moved = false;
      const rect = pane.getBoundingClientRect();
      const horizontal = handle.dataset.mechConsoleResizeAxis === "width" ||
        pane.dataset.mechConsoleResizeAxis === "width" ||
        handle.id === "resizer" ||
        handle.id === "edgeHandle";
      const start = horizontal ? event.clientX : event.clientY;
      const initial = horizontal ? rect.width : rect.height;
      const move = (moveEvent) => {
        moved = true;
        const delta = (horizontal ? moveEvent.clientX : moveEvent.clientY) - start;
        const requested = initial + (horizontal ? -delta : delta);
        const minimum = horizontal ? Math.min(370, window.innerWidth) : 160;
        const maximum = horizontal
          ? Math.max(minimum, Math.floor(state.root.getBoundingClientRect().width * 0.8))
          : 900;
        const overdrag = 48;
        if (horizontal && requested < minimum - overdrag) {
          delete pane.dataset.mechFullscreenFallback;
          pane.classList.remove("is-fullscreen");
          setConsoleOpen(false);
          return;
        }
        setConsoleOpen(true);
        if (horizontal && requested > maximum + overdrag) {
          pane.dataset.mechFullscreenFallback = "true";
          pane.classList.add("is-fullscreen");
          for (const toggle of documentConsoleFullscreenControls()) {
            setFullscreenState(pane, toggle, true);
          }
          return;
        }
        if (horizontal && pane.dataset.mechFullscreenFallback === "true") {
          delete pane.dataset.mechFullscreenFallback;
          pane.classList.remove("is-fullscreen");
          for (const toggle of documentConsoleFullscreenControls()) {
            setFullscreenState(pane, toggle, false);
          }
        }
        const size = Math.max(minimum, Math.min(maximum, requested));
        state.root.style.setProperty("--mech-console-size", `${size}px`);
        pane.style[horizontal ? "width" : "height"] = `${size}px`;
      };
      const finish = () => {
        window.removeEventListener("pointermove", move);
        window.removeEventListener("pointerup", finish);
        if (handle.id === "edgeHandle" && !moved) {
          const isOpen = state.root?.dataset.mechConsoleOpen !== "false";
          setConsoleOpen(!isOpen);
        }
      };
      window.addEventListener("pointermove", move);
      window.addEventListener("pointerup", finish, { once: true });
    });
  }
}

function setFullscreenState(pane, toggle, active) {
  pane.classList.toggle("is-fullscreen", active);
  toggle.setAttribute("aria-pressed", String(active));
  toggle.setAttribute(
    "aria-label",
    active ? "Exit fullscreen" : "Enter fullscreen",
  );
}

function initializeFullscreen() {
  const pane = documentConsolePane();
  const [toggle] = documentConsoleFullscreenControls();
  if (!pane || !toggle) {
    return;
  }

  const synchronize = () => {
    const nativeFullscreen = document.fullscreenElement === pane;
    const fallbackFullscreen = pane.dataset.mechFullscreenFallback === "true";
    setFullscreenState(pane, toggle, nativeFullscreen || fallbackFullscreen);
  };

  document.addEventListener("fullscreenchange", synchronize);
  synchronize();
  toggle.addEventListener("click", async () => {
    if (document.fullscreenElement === pane) {
      try {
        await document.exitFullscreen();
      } catch (error) {
        appendError(error);
      }
      synchronize();
      return;
    }

    if (pane.dataset.mechFullscreenFallback === "true") {
      delete pane.dataset.mechFullscreenFallback;
      synchronize();
      return;
    }

    if (pane.requestFullscreen) {
      try {
        await pane.requestFullscreen();
      } catch (error) {
        pane.dataset.mechFullscreenFallback = "true";
        appendError(error);
      }
    } else {
      pane.dataset.mechFullscreenFallback = "true";
    }
    synchronize();
  });
}

function initializeBreadcrumb() {
  const breadcrumb = document.getElementById("breadcrumb");
  if (!breadcrumb || breadcrumb.textContent.trim()) {
    return;
  }
  const leaf = document.title || location.pathname.split("/").filter(Boolean).pop() || "Document";
  breadcrumb.textContent = leaf;
}

function initializeToc() {
  const links = [...document.querySelectorAll(".mech-toc a[href^='#'], [data-mech-toc] a[href^='#']")];
  if (!links.length) {
    return;
  }
  const sections = links
    .map((link) => ({
      link,
      // Heading IDs are generated from Mech source and may begin with a
      // digit. That is a valid HTML ID, but not a valid unescaped CSS
      // selector, so resolve the hash as an ID rather than a selector.
      target: document.getElementById((link.getAttribute("href") || "").slice(1)),
    }))
    .filter(({ target }) => target);
  for (const { link, target } of sections) {
    link.addEventListener("click", (event) => {
      event.preventDefault();
      target.scrollIntoView({ behavior: "smooth", block: "start" });
    });
  }
  if (!("IntersectionObserver" in window)) {
    return;
  }
  const observer = new IntersectionObserver((entries) => {
    const visible = entries.find((entry) => entry.isIntersecting);
    if (!visible) {
      return;
    }
    for (const { link, target } of sections) {
      link.classList.toggle("active", target === visible.target);
    }
  }, { rootMargin: "-20% 0px -70% 0px" });
  for (const { target } of sections) {
    observer.observe(target);
  }
}

function initializeOptionalRenderers() {
  if (window.katex && typeof window.katex.render === "function") {
    for (const element of document.querySelectorAll("[data-katex], .math-inline, .math-display")) {
      try {
        window.katex.render(element.textContent, element, {
          displayMode: element.classList.contains("math-display"),
          throwOnError: false,
        });
      } catch (error) {
        appendError(error);
      }
    }
  }
  if (
    window.mermaid &&
    typeof window.mermaid.run === "function" &&
    document.querySelector(".mermaid")
  ) {
    try {
      Promise.resolve(window.mermaid.run({ nodes: document.querySelectorAll(".mermaid") }))
        .catch(appendError);
    } catch (error) {
      appendError(error);
    }
  }
}

function initializeLayout() {
  window.addEventListener("mech:output", event => {
    if (event instanceof CustomEvent && event.detail) {
      appendProgramOutput(event.detail);
    }
  });
  initializeConsoleState();
  initializeConsoleTabs();
  initializeConsoleToggle();
  initializeConsoleKeyboardToggle();
  initializeResizeHandles();
  initializeFullscreen();
  initializeBreadcrumb();
  initializeToc();
  initializeOptionalRenderers();
  window.addEventListener("load", initializeOptionalRenderers, { once: true });
}

function frame() {
  if (!state.running || !state.document) {
    return;
  }
  try {
    const result = state.document.frame(8);
    if (result.events?.length) {
      consumeReplResponse(result);
    }
    if (result.processed > 0) {
      renderValues();
    }
    state.animationFrame = requestAnimationFrame(frame);
  } catch (error) {
    showFatalError(error);
  }
}

async function main() {
  state.root = documentRoot();
  if (!state.root) {
    throw new Error("the document controller requires a .mech-root element");
  }
  setDocumentStatus("loading");
  const embeddedDocumentSources = loadEmbeddedDocumentSourceBundle();
  const wasmModule =
    state.controllerElement?.dataset.mechWasmModule || "/_mech/pkg/mech_wasm.js";
  const wasmBindings = await import(wasmModule);
  const { default: initializeWasm, WasmDocument } = wasmBindings;
  await initializeWasm();
  state.replInputAction = typeof wasmBindings.replInputAction === "function"
    ? wasmBindings.replInputAction
    : null;
  state.replStepLimit = typeof wasmBindings.replStepLimit === "function"
    ? wasmBindings.replStepLimit()
    : null;
  state.initialEncoded = await loadEncodedDocument();
  const documentSources =
    embeddedDocumentSources || await loadDocumentSourceMap();
  if (documentSources?.config) {
    if (
      !Object.prototype.hasOwnProperty.call(window, "__MECH_HOST_CONFIG") ||
      (documentSources.version === 2
        ? typeof WasmDocument.fromServedEncodedWithBundle !== "function"
        : typeof WasmDocument.fromServedEncoded !== "function")
    ) {
      throw new Error(
        "configured source documents require a browser WASM build with served project authority",
      );
    }
    state.document = documentSources.version === 2
      ? WasmDocument.fromServedEncodedWithBundle(
          state.initialEncoded,
          documentSources.rootSpecifier,
          documentSources.config,
          documentSources.sources,
          documentSources.resolutions,
        )
      : WasmDocument.fromServedEncoded(
          state.initialEncoded,
          documentSources.rootSpecifier,
          documentSources.config,
          documentSources.sources,
        );
  } else if (documentSources) {
    if (
      documentSources.version === 2 &&
      typeof WasmDocument.fromEncodedWithBundle !== "function"
    ) {
      throw new Error(
        "source document bundles require a browser WASM build with explicit source resolution",
      );
    }
    if (
      documentSources.version !== 2 &&
      typeof WasmDocument.fromEncodedWithSources !== "function"
    ) {
      throw new Error(
        "source documents with imports require a browser WASM build with document source resolution",
      );
    }
    state.document = documentSources.version === 2
      ? WasmDocument.fromEncodedWithBundle(
          state.initialEncoded,
          documentSources.rootSpecifier,
          documentSources.sources,
          documentSources.resolutions,
        )
      : WasmDocument.fromEncodedWithSources(
          state.initialEncoded,
          documentSources.rootSpecifier,
          documentSources.sources,
        );
  } else {
    state.document = WasmDocument.fromEncoded(state.initialEncoded);
  }
  if (typeof state.document.replInvoke !== "function") {
    throw new Error(
      "the browser WASM build does not include the document-backed resident REPL host",
    );
  }
  state.repl = {
    invoke: source => state.document.replInvoke(source),
    continueStep: count => state.document.replContinueStep(count),
    interrupt: () => state.document.replInterrupt(),
    setQuiet: quiet => state.document.replSetQuiet(quiet),
    selectSymbol: name => state.document.replSelectSymbol(name),
    selectOutput: outputId => state.document.replSelectOutput(outputId),
  };
  state.replQuiet = requestedReplQuiet();
  consumeReplResponse(state.repl.setQuiet(state.replQuiet));
  attachConsole();
  initializeLayout();
  prepareVarPlaceholders();
  renderValues();
  state.document.start();
  state.running = true;
  setDocumentStatus("ready");
  dispatch("mech:document-ready");
  state.animationFrame = requestAnimationFrame(frame);
}

window.addEventListener("beforeunload", () => {
  stopRuntime();
  if (document.documentElement.dataset.mechDocumentStatus !== "error") {
    setDocumentStatus("stopped");
  }
});

main().catch(showFatalError);
