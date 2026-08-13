const controllerElement = document.querySelector(
  "script[data-mech-document-controller]",
);

const state = {
  controllerElement,
  document: null,
  initialEncoded: "",
  root: null,
  running: false,
  animationFrame: null,
  history: [],
  historyIndex: 0,
  console: null,
};

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

function outputPanel() {
  return state.root?.querySelector(
    "#mech-document-output, [data-mech-document-output], [data-mech-output-panel]",
  ) || document.querySelector(
    "#mech-document-output, [data-mech-document-output], [data-mech-output-panel]",
  );
}

function appendError(error) {
  const message = errorMessage(error);
  const panel = errorPanel();
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
    interpreterId: BigInt(element.id.slice(separator + 1)),
  };
}

function resolveNamedInterpreter(name) {
  const identifier = state.document.interpreterIdByName(name);
  if (identifier === null || identifier === undefined) {
    throw new Error(`named interpreter \`${name}\` was not found`);
  }
  return identifier.toString();
}

function prepareVarPlaceholders() {
  const root = state.root;
  if (!root) {
    return;
  }
  const pattern = /\{\{VAR:([^@}\s]+)(?:@([^}\s]+))?\}\}/g;
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
      try {
        placeholder.dataset.mechInterpreterId = match[2]
          ? resolveNamedInterpreter(match[2])
          : "0";
      } catch (error) {
        placeholder.dataset.mechInterpreterError = errorMessage(error);
        placeholder.textContent = "[unresolved Mech variable]";
        appendError(error);
      }
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
  row.dataset.mechInterpreterId = address.interpreterId.toString();
  row.dataset.mechOutputId = address.outputId.toString();
  row.dataset.mechRenderedKind = rendered.kind;

  const heading = document.createElement("header");
  heading.className = "mech-document-output-heading";
  heading.textContent =
    `interpreter ${address.interpreterId} · output ${address.outputId} · ${rendered.kind}`;
  const body = document.createElement("div");
  body.className = "mech-document-output-html";
  body.innerHTML = rendered.blockHtml;
  row.append(heading, body);
  return row;
}

function refreshOutputPanel(entries) {
  const panel = outputPanel();
  if (!panel) {
    return;
  }
  panel.replaceChildren();
  for (const entry of entries) {
    panel.append(createOutputEntry(entry.address, entry.rendered));
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
      const rendered = state.document.renderedOutput(
        address.interpreterId,
        address.outputId,
      );
      if (rendered !== null) {
        output.innerHTML = rendered.blockHtml;
        outputEntries.push({ address, rendered });
      }
    } catch (error) {
      appendError(error);
    }
  }
  for (const output of state.root?.querySelectorAll(".mech-inline-mech-code[id]") || []) {
    try {
      const address = outputAddress(output);
      const rendered = state.document.renderedOutput(
        address.interpreterId,
        address.outputId,
      );
      if (rendered !== null) {
        output.innerHTML = rendered.inlineHtml;
      }
    } catch (error) {
      appendError(error);
    }
  }
  for (const placeholder of state.root?.querySelectorAll(".mech-var-placeholder") || []) {
    if (placeholder.dataset.mechInterpreterError) {
      continue;
    }
    try {
      const rendered = state.document.renderedSymbol(
        BigInt(placeholder.dataset.mechInterpreterId || "0"),
        placeholder.dataset.mechVarName,
      );
      if (rendered !== null) {
        placeholder.innerHTML = rendered.inlineHtml;
      }
    } catch (error) {
      appendError(error);
    }
  }
  refreshOutputPanel(outputEntries);
  dispatch("mech:document-rendered");
}

function transcript() {
  return state.console?.transcript || null;
}

function appendTranscriptRow(className, text) {
  const target = transcript();
  if (!target) {
    return null;
  }
  const row = document.createElement("div");
  row.className = `mech-repl-entry ${className}`;
  row.textContent = text;
  target.append(row);
  target.scrollTop = target.scrollHeight;
  return row;
}

function appendRenderedResult(rendered) {
  const target = transcript();
  if (!target) {
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
  target.append(row);
  target.scrollTop = target.scrollHeight;
}

function appendConsoleError(error) {
  appendTranscriptRow("mech-repl-error", errorMessage(error));
  appendError(error);
}

function supportsInteractiveEvaluation() {
  return typeof state.document?.evaluate === "function";
}

function appendHelp() {
  const target = transcript();
  if (!target) {
    return;
  }
  const table = document.createElement("table");
  table.className = "mech-repl-help";
  const body = document.createElement("tbody");
  for (const [command, description] of [
    [":help", "Show browser console commands."],
    [":clc", "Clear the console transcript."],
    [":clear", "Restore the original document program."],
    [":whos [names...]", "Render root symbols."],
    [":step [count]", "Advance the document program."],
  ]) {
    const row = document.createElement("tr");
    const name = document.createElement("th");
    name.scope = "row";
    name.textContent = command;
    const detail = document.createElement("td");
    detail.textContent = description;
    row.append(name, detail);
    body.append(row);
  }
  table.append(body);
  target.append(table);
  target.scrollTop = target.scrollHeight;
}

function appendSymbolRows(rows) {
  const target = transcript();
  if (!target) {
    return;
  }
  const table = document.createElement("table");
  table.className = "mech-repl-symbols";
  const body = document.createElement("tbody");
  for (const rendered of rows) {
    const row = document.createElement("tr");
    row.dataset.mechSymbolName = rendered.name;
    row.dataset.mechResultKind = rendered.kind;
    const name = document.createElement("th");
    name.scope = "row";
    name.textContent = rendered.name;
    const kind = document.createElement("td");
    kind.textContent = rendered.kind;
    const value = document.createElement("td");
    value.innerHTML = rendered.inlineHtml;
    row.append(name, kind, value);
    body.append(row);
  }
  table.append(body);
  target.append(table);
  target.scrollTop = target.scrollHeight;
}

function clearTranscript() {
  transcript()?.replaceChildren();
}

function parseStepCount(argumentsList) {
  if (argumentsList.length > 1) {
    throw new Error(":step accepts at most one count");
  }
  const raw = argumentsList[0] || "1";
  if (!/^\d+$/.test(raw)) {
    throw new Error(":step count must be a positive integer");
  }
  const count = BigInt(raw);
  if (count === 0n) {
    throw new Error(":step count must be greater than zero");
  }
  return count;
}

async function runConsoleCommand(source) {
  const input = source.trim();
  if (!input) {
    return;
  }
  if (!input.startsWith(":")) {
    if (!supportsInteractiveEvaluation()) {
      throw new Error(
        "interactive source evaluation is unavailable in standard resident documents; use :help for document commands",
      );
    }
    const rendered = state.document.evaluate(source);
    appendRenderedResult(rendered);
    renderValues();
    return;
  }

  const [command, ...argumentsList] = input.split(/\s+/);
  switch (command.toLowerCase()) {
    case ":help":
      appendHelp();
      return;
    case ":clc":
      clearTranscript();
      return;
    case ":clear":
      state.document.reset(state.initialEncoded);
      prepareVarPlaceholders();
      renderValues();
      appendTranscriptRow("mech-repl-info", "Document reset.");
      return;
    case ":whos": {
      const names = argumentsList.length === 0 ? null : argumentsList;
      appendSymbolRows(state.document.renderedSymbols(names));
      return;
    }
    case ":step": {
      const count = parseStepCount(argumentsList);
      state.document.step(count);
      appendTranscriptRow("mech-repl-info", `Advanced ${count} step${count === 1n ? "" : "s"}.`);
      renderValues();
      return;
    }
    default:
      throw new Error(`unsupported browser command \`${command}\``);
  }
}

function submitConsoleInput(value) {
  const source = value.trim();
  if (!source) {
    return;
  }
  appendTranscriptRow("mech-repl-source", source);
  state.history.push(source);
  state.historyIndex = state.history.length;
  try {
    const result = runConsoleCommand(source);
    if (result && typeof result.catch === "function") {
      result.catch(appendConsoleError);
    }
  } catch (error) {
    appendConsoleError(error);
  }
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
  const inputRow = document.createElement("div");
  inputRow.className = "mech-repl-input-row";
  const interactiveEvaluation = supportsInteractiveEvaluation();
  const prompt = document.createElement("span");
  prompt.className = "repl-prompt";
  prompt.textContent = interactiveEvaluation ? ">:" : ":";
  const input = document.createElement("textarea");
  input.className = "repl-input";
  input.dataset.mechInteractiveEvaluation = interactiveEvaluation ? "available" : "unavailable";
  input.setAttribute(
    "aria-label",
    interactiveEvaluation ? "Mech developer REPL input" : "Mech document command input",
  );
  if (!interactiveEvaluation) {
    input.placeholder = "Document commands only (:help)";
  }
  input.addEventListener("keydown", (event) => {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      const source = input.value;
      input.value = "";
      submitConsoleInput(source);
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
  mount.append(transcriptElement, inputRow);
  state.console = { mount, transcript: transcriptElement, input };
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
        // The document console is anchored to the right edge, so moving its
        // left resize handle left must make the pane wider.
        // Some shipped shells start with a responsive console wider than the
        // old fixed 900px ceiling. Keep enough space for the document while
        // never turning a widening drag into a forced shrink.
        const maximum = horizontal
          ? Math.max(900, window.innerWidth - 240, initial)
          : 900;
        const size = Math.max(
          160,
          Math.min(maximum, initial + (horizontal ? -delta : delta)),
        );
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
  initializeConsoleState();
  initializeConsoleTabs();
  initializeConsoleToggle();
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
  const { default: initializeWasm, WasmDocument } = await import(wasmModule);
  await initializeWasm();
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
