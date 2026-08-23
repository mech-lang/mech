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
  historyDraft: "",
  console: null,
  replInputAction: null,
  replStepLimit: null,
  replQuiet: false,
  replBusy: false,
  replTerminated: false,
  activeHostRequest: null,
  hostRequestSequence: 0,
  activeCooperativeOperation: null,
  cooperativeOperationSequence: 0,
  documentationFragmentSequence: 0,
  programDisplays: new Map(),
  directedProgramOutput: false,
  inlineInspectors: new Map(),
  inlineInspectorSequence: 0,
  reflectiveElementIdentities: new WeakMap(),
  reflectiveElementIdentitySequence: 0,
  replHostOffsetObserver: null,
  replPageStyleProbe: null,
  replStyleObserver: null,
  consolePointerSession: null,
  persistedLayout: undefined,
  pagePositionSaveTimer: null,
  pendingPagePosition: null,
  pagePositionRestore: null,
  consoleSizeObserver: null,
  errorBadgeObserver: null,
  tocUpdateFrame: null,
  tocEventCleanup: null,
  tocLinkHandlers: new Map(),
  mermaidInitialized: false,
  outputFullscreenController: null,
  computeBridge: null,
  computeBridgeRefresh: null,
  computeBridgeGeneration: null,
  computeBridgeBuildId: 0,
  computeBridgeLifecycle: "absent",
  computeAdapter: undefined,
};

const ERROR_PANEL_SELECTOR =
  "#mech-document-errors, [data-mech-document-errors], [data-mech-errors-panel]";
const OUTPUT_PANEL_SELECTOR =
  "#mech-document-output, [data-mech-document-output], [data-mech-output-panel]";
const DOCUMENT_LAYOUT_STORAGE_VERSION = 1;
const KIND_ANNOTATION_MAX_CHARACTERS = 96;

function setComputeBridgeLifecycle(lifecycle) {
  state.computeBridgeLifecycle = lifecycle;
  document.documentElement.dataset.mechComputeLifecycle = lifecycle;
}

function documentLayoutStorageKey() {
  return `mech:document-layout:v${DOCUMENT_LAYOUT_STORAGE_VERSION}:${location.origin}${location.pathname}${location.search}`;
}

function persistedDocumentLayout() {
  if (state.persistedLayout !== undefined) {
    return state.persistedLayout;
  }
  try {
    const stored = localStorage.getItem(documentLayoutStorageKey());
    const parsed = stored ? JSON.parse(stored) : null;
    state.persistedLayout = parsed && typeof parsed === "object" ? parsed : {};
  } catch (_error) {
    state.persistedLayout = {};
  }
  return state.persistedLayout;
}

function updatePersistedDocumentLayout(patch) {
  const next = { ...persistedDocumentLayout(), ...patch };
  state.persistedLayout = next;
  try {
    localStorage.setItem(documentLayoutStorageKey(), JSON.stringify(next));
  } catch (_error) {
    // Storage can be unavailable in embedded or privacy-restricted contexts;
    // the live document remains fully functional without persistence.
  }
}

function saveConsoleOpeningSize(axis, size) {
  if (!["width", "height"].includes(axis) || !Number.isFinite(size) || size <= 0) {
    return;
  }
  updatePersistedDocumentLayout({
    console: { axis, size: Math.round(size) },
  });
}

function documentPageScrollOwner() {
  const contentShell = document.querySelector(".content-shell");
  if (contentShell) {
    const style = getComputedStyle(contentShell);
    const scrollable = ["auto", "scroll", "overlay"].includes(style.overflowY) &&
      contentShell.scrollHeight > contentShell.clientHeight + 1;
    if (scrollable) {
      return contentShell;
    }
  }
  return window;
}

function documentPageContentOrigin(contentShell) {
  let x = 0;
  let y = 0;
  let element = contentShell;
  while (element) {
    x += element.offsetLeft;
    y += element.offsetTop;
    element = element.offsetParent;
  }
  return { x, y };
}

function documentPagePositionForOwner(position, owner) {
  const sourceOwner = position.owner === "content-shell" ? "content-shell" : "window";
  const targetOwner = owner === window ? "window" : "content-shell";
  const x = Number(position.x) || 0;
  const y = Number(position.y) || 0;
  const contentShell = document.querySelector(".content-shell");
  if (position.coordinateSpace === "content-shell") {
    if (!contentShell) {
      return null;
    }
    const { x: originX, y: originY } = documentPageContentOrigin(contentShell);
    if (targetOwner === "window") {
      return {
        x: Math.max(0, originX + x),
        y: Math.max(0, originY + y),
      };
    }
    return {
      x: Math.max(0, x),
      y: Math.max(0, y),
    };
  }
  if (position.coordinateSpace === "window") {
    if (targetOwner === "window") {
      return { x: Math.max(0, x), y: Math.max(0, y) };
    }
    if (!contentShell) {
      return null;
    }
    const { x: originX, y: originY } = documentPageContentOrigin(contentShell);
    return {
      x: Math.max(0, x - originX),
      y: Math.max(0, y - originY),
    };
  }

  // Legacy positions used coordinates local to their recorded owner. Keep
  // that behavior defined for existing v1 entries, including ownerless
  // entries (which were window coordinates), while all newly saved entries
  // use the layout-independent content-shell coordinate space above.
  if (sourceOwner === targetOwner) {
    return { x: Math.max(0, x), y: Math.max(0, y) };
  }
  if (!contentShell) {
    return sourceOwner === "content-shell"
      ? null
      : { x: Math.max(0, x), y: Math.max(0, y) };
  }
  const { x: originX, y: originY } = documentPageContentOrigin(contentShell);
  if (sourceOwner === "content-shell") {
    return {
      x: Math.max(0, originX + x),
      y: Math.max(0, originY + y),
    };
  }
  return {
    x: Math.max(0, x - originX),
    y: Math.max(0, y - originY),
  };
}

function currentPagePosition() {
  const owner = documentPageScrollOwner();
  const contentShell = document.querySelector(".content-shell");
  if (!contentShell) {
    return {
      owner: "window",
      coordinateSpace: "window",
      x: Math.max(0, Math.round(window.scrollX)),
      y: Math.max(0, Math.round(window.scrollY)),
    };
  }
  const origin = documentPageContentOrigin(contentShell);
  const ownerX = owner === window ? window.scrollX : owner.scrollLeft;
  const ownerY = owner === window ? window.scrollY : owner.scrollTop;
  return {
    owner: owner === window ? "window" : "content-shell",
    coordinateSpace: "content-shell",
    x: Math.round(owner === window ? ownerX - origin.x : ownerX),
    y: Math.round(owner === window ? ownerY - origin.y : ownerY),
  };
}

function savePagePosition(position = currentPagePosition()) {
  if (state.pagePositionRestore) {
    return;
  }
  if (state.pagePositionSaveTimer !== null) {
    clearTimeout(state.pagePositionSaveTimer);
    state.pagePositionSaveTimer = null;
  }
  state.pendingPagePosition = null;
  updatePersistedDocumentLayout({
    page: position,
  });
}

function schedulePagePositionSave() {
  if (state.pagePositionRestore) {
    return;
  }
  state.pendingPagePosition = currentPagePosition();
  if (state.pagePositionSaveTimer !== null) {
    return;
  }
  state.pagePositionSaveTimer = setTimeout(() => {
    state.pagePositionSaveTimer = null;
    const pending = state.pendingPagePosition;
    if (pending) {
      savePagePosition(pending);
    }
  }, 120);
}

function flushPagePositionSave() {
  const pending = state.pendingPagePosition;
  if (pending) {
    savePagePosition(pending);
  }
}

function applyPersistedConsoleOpeningSize() {
  const pane = documentConsolePane();
  const saved = persistedDocumentLayout().console;
  if (!pane || !state.root || !saved || !["width", "height"].includes(saved.axis)) {
    return;
  }
  const requested = Number(saved.size);
  if (!Number.isFinite(requested) || requested <= 0) {
    return;
  }
  const horizontal = saved.axis === "width";
  const minimum = horizontal ? Math.min(370, window.innerWidth) : 160;
  const maximum = horizontal
    ? Math.max(minimum, Math.floor(state.root.getBoundingClientRect().width * 0.8))
    : 900;
  const size = Math.max(minimum, Math.min(maximum, requested));
  state.root.style.setProperty("--mech-console-size", `${size}px`);
  pane.style[saved.axis] = `${size}px`;
}

function restoreConsoleOpeningSize() {
  applyPersistedConsoleOpeningSize();
  const refresh = () => {
    if (!documentConsolePane()?.classList.contains("is-fullscreen")) {
      applyPersistedConsoleOpeningSize();
    }
  };
  window.addEventListener("resize", refresh);
  window.visualViewport?.addEventListener("resize", refresh);
  if (typeof ResizeObserver === "function" && state.root) {
    state.consoleSizeObserver?.disconnect();
    state.consoleSizeObserver = new ResizeObserver(refresh);
    state.consoleSizeObserver.observe(state.root);
  }
}

function finishPagePositionRestore({ preserveSaved = true } = {}) {
  const restore = state.pagePositionRestore;
  if (!restore) {
    return;
  }
  clearTimeout(restore.timer);
  restore.observer?.disconnect();
  restore.mutationObserver?.disconnect();
  for (const [target, type, listener] of restore.cancellations) {
    target.removeEventListener(type, listener);
  }
  state.pagePositionRestore = null;
  delete document.documentElement.dataset.mechPagePositionRestore;
  if (!preserveSaved) {
    savePagePosition();
  }
}

function scrollToImmediately(owner, x, y) {
  const styleOwner = owner === window ? document.documentElement : owner;
  const scrollBehavior = styleOwner.style.scrollBehavior;
  styleOwner.style.scrollBehavior = "auto";
  owner.scrollTo(x, y);
  styleOwner.style.scrollBehavior = scrollBehavior;
}

function restorePagePosition() {
  const saved = persistedDocumentLayout().page;
  // Ownerless entries predate scroll-owner persistence and always recorded
  // window coordinates. Entries without coordinateSpace retain owner-local
  // v1 semantics; current entries are canonical content-shell coordinates.
  const sourceOwner = saved?.owner === "content-shell" ? "content-shell" : "window";
  const coordinateSpace = saved?.coordinateSpace === "content-shell"
    ? "content-shell"
    : saved?.coordinateSpace === "window" ? "window" : "owner";
  const x = Number(saved?.x);
  const y = Number(saved?.y);
  if (!Number.isFinite(x) || !Number.isFinite(y)) {
    return;
  }
  finishPagePositionRestore();
  const restore = {
    x,
    y,
    owner: sourceOwner,
    coordinateSpace,
    deadline: performance.now() + 15_000,
    timer: null,
    observer: null,
    mutationObserver: null,
    cancellations: [],
    mapping: null,
    stableSince: null,
  };
  state.pagePositionRestore = restore;
  document.documentElement.dataset.mechPagePositionRestore = "pending";
  const cancel = () => finishPagePositionRestore({ preserveSaved: false });
  const activeOwner = documentPageScrollOwner();
  for (const [target, type] of [
    [window, "wheel"],
    [window, "touchstart"],
    [window, "pointerdown"],
    [window, "keydown"],
  ]) {
    target.addEventListener(type, cancel, { passive: true });
    restore.cancellations.push([target, type, cancel]);
  }
  if (activeOwner !== window) {
    for (const type of ["wheel", "touchstart", "pointerdown", "keydown"]) {
      activeOwner.addEventListener(type, cancel, { passive: true });
      restore.cancellations.push([activeOwner, type, cancel]);
    }
  }
  const attempt = () => {
    if (state.pagePositionRestore !== restore) {
      return;
    }
    const owner = documentPageScrollOwner();
    const target = documentPagePositionForOwner(restore, owner);
    if (!target) {
      restore.mapping = null;
      restore.stableSince = null;
      document.documentElement.dataset.mechPagePositionRestore = "waiting-anchor";
      if (performance.now() >= restore.deadline) {
        finishPagePositionRestore({ preserveSaved: false });
        return;
      }
      clearTimeout(restore.timer);
      restore.timer = setTimeout(attempt, 120);
      return;
    }
    const mapping = `${owner === window ? "window" : "content-shell"}:${target.x}:${target.y}`;
    if (restore.mapping !== mapping) {
      restore.mapping = mapping;
      restore.stableSince = null;
    }
    scrollToImmediately(owner, target.x, target.y);
    const currentX = owner === window ? window.scrollX : owner.scrollLeft;
    const currentY = owner === window ? window.scrollY : owner.scrollTop;
    const reached = Math.abs(currentX - target.x) <= 1 &&
      Math.abs(currentY - target.y) <= 1;
    if (reached) {
      const now = performance.now();
      // A raw window coordinate is only provisional while the canonical
      // content anchor is absent. Keep the restoration lifecycle alive for
      // the full retry window so a late shell can become the owner and
      // receive the translated coordinate instead of inheriting scrollTop 0.
      if (owner === window && !document.querySelector(".content-shell")) {
        restore.stableSince = null;
        document.documentElement.dataset.mechPagePositionRestore = "waiting-owner";
        if (now >= restore.deadline) {
          finishPagePositionRestore();
          return;
        }
        clearTimeout(restore.timer);
        restore.timer = setTimeout(attempt, 120);
        return;
      }
      restore.stableSince ??= now;
      document.documentElement.dataset.mechPagePositionRestore = "settling";
      if (now - restore.stableSince >= 600 || now >= restore.deadline) {
        finishPagePositionRestore();
        return;
      }
      clearTimeout(restore.timer);
      restore.timer = setTimeout(attempt, 120);
      return;
    }
    restore.stableSince = null;
    document.documentElement.dataset.mechPagePositionRestore = "restoring";
    if (performance.now() >= restore.deadline) {
      finishPagePositionRestore({ preserveSaved: false });
      return;
    }
    clearTimeout(restore.timer);
    restore.timer = setTimeout(attempt, 120);
  };
  if (typeof ResizeObserver === "function") {
    restore.observer = new ResizeObserver(attempt);
    restore.observer.observe(document.documentElement);
    if (document.body) {
      restore.observer.observe(document.body);
    }
    const contentShell = document.querySelector(".content-shell");
    if (contentShell) {
      restore.observer.observe(contentShell);
    }
  }
  if (typeof MutationObserver === "function") {
    restore.mutationObserver = new MutationObserver(attempt);
    restore.mutationObserver.observe(document.documentElement, {
      childList: true,
      subtree: true,
    });
  }
  for (const image of document.images) {
    if (!image.complete) {
      image.addEventListener("load", attempt, { once: true });
      image.addEventListener("error", attempt, { once: true });
    }
  }
  window.addEventListener("load", attempt, { once: true });
  requestAnimationFrame(() => requestAnimationFrame(attempt));
}

function initializeDocumentLayoutPersistence() {
  if ("scrollRestoration" in history) {
    history.scrollRestoration = "manual";
  }
  restoreConsoleOpeningSize();
  window.addEventListener("scroll", schedulePagePositionSave, { passive: true });
  document.querySelector(".content-shell")?.addEventListener(
    "scroll",
    schedulePagePositionSave,
    { passive: true },
  );
  window.addEventListener("pagehide", flushPagePositionSave);
}

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

function requestedReplValueElementLimit() {
  const values = [
    state.controllerElement?.dataset.mechReplMaxElements,
    state.root?.dataset.mechReplMaxElements,
    document.documentElement?.dataset.mechReplMaxElements,
  ];
  for (const value of values) {
    const parsed = Number.parseInt(value || "", 10);
    if (Number.isSafeInteger(parsed) && parsed > 0) {
      return parsed;
    }
  }
  return 500;
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

function dispatch(name, detail = undefined) {
  window.dispatchEvent(new CustomEvent(name, { detail }));
}

function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}

function isRecoverableResidentTurnError(error) {
  return error instanceof Error && error.mechRecoverableResidentTurn === true;
}

function controllerQuery(selector) {
  return state.root
    ? state.root.querySelector(selector)
    : document.querySelector(selector);
}

function errorPanel() {
  return controllerQuery(ERROR_PANEL_SELECTOR);
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
  return controllerQuery(OUTPUT_PANEL_SELECTOR);
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

function selectedConsolePanel(pane = documentConsolePane()) {
  if (!pane) {
    return "";
  }
  return pane.dataset.mechConsoleActivePanel ||
    pane.querySelector("[data-mech-console-tab][aria-selected='true'], .console-tab[aria-selected='true']")
      ?.dataset.mechConsoleTab ||
    pane.querySelector(".console-tab[aria-selected='true']")?.dataset.tab ||
    "";
}

function consoleTabBaseLabel(tab) {
  if (!("mechConsoleBaseLabel" in tab.dataset)) {
    const text = [...tab.childNodes]
      .filter(node => node.nodeType === Node.TEXT_NODE)
      .map(node => node.textContent || "")
      .join(" ")
      .trim();
    tab.dataset.mechConsoleBaseLabel = tab.getAttribute("aria-label") || text ||
      tab.dataset.mechConsoleTab || tab.dataset.tab || "Console section";
  }
  return tab.dataset.mechConsoleBaseLabel;
}

function updateConsoleTabAccessibleLabel(tab) {
  const label = [consoleTabBaseLabel(tab)];
  const errors = Number(tab.dataset.mechConsoleErrorCount || "0");
  if (errors > 0) {
    label.push(`${errors} ${errors === 1 ? "error" : "errors"}`);
  }
  if (tab.dataset.mechConsoleUnread === "true") {
    label.push("new activity");
  }
  tab.setAttribute("aria-label", label.join(", "));
}

function updateConsoleErrorBadge() {
  const panel = errorPanel();
  const count = panel?.querySelectorAll(
    ".mech-console-error:not(.mech-repl-diagnostic), " +
    ".mech-repl-diagnostic[data-mech-diagnostic-severity='error'], " +
    ".mech-repl-diagnostic[data-mech-diagnostic-severity='fatal']",
  ).length || 0;
  const pane = documentConsolePane();
  for (const tab of pane?.querySelectorAll(
    "[data-mech-console-tab='errors'], [data-tab='errors']",
  ) || []) {
    let badge = tab.querySelector(".mech-console-error-count");
    if (!badge) {
      badge = document.createElement("span");
      badge.className = "mech-console-error-count";
      badge.setAttribute("aria-hidden", "true");
      tab.append(badge);
    }
    badge.textContent = String(count);
    badge.hidden = count === 0;
    tab.dataset.mechConsoleErrorCount = String(count);
    updateConsoleTabAccessibleLabel(tab);
  }
}

function initializeConsoleErrorBadge() {
  state.errorBadgeObserver?.disconnect();
  state.errorBadgeObserver = null;
  updateConsoleErrorBadge();
  const panel = errorPanel();
  if (panel && typeof MutationObserver === "function") {
    state.errorBadgeObserver = new MutationObserver(updateConsoleErrorBadge);
    state.errorBadgeObserver.observe(panel, { childList: true, subtree: true });
  }
}

function markConsolePanelActivity(name) {
  const pane = documentConsolePane();
  if (!pane || pane.classList.contains("is-fullscreen") || selectedConsolePanel(pane) === name) {
    return;
  }
  const tab = [...pane.querySelectorAll(
    ".console-tab, [data-mech-console-tab], [data-tab]",
  )].find(candidate =>
    (candidate.dataset.mechConsoleTab || candidate.dataset.tab) === name
  );
  if (tab) {
    tab.dataset.mechConsoleUnread = "true";
    updateConsoleTabAccessibleLabel(tab);
  }
}

function appendDiagnostic(diagnostic) {
  const interaction = diagnostic.owner === "interaction";
  const target = interaction ? transcript() : errorRegion("program-diagnostics");
  if (!target) {
    return;
  }
  const row = document.createElement("article");
  const severity = diagnostic.severity || "error";
  row.className = `mech-repl-entry mech-repl-diagnostic mech-repl-${severity}`;
  row.dataset.mechDiagnosticId = diagnostic.id || "";
  row.dataset.mechDiagnosticSeverity = severity;
  if (severity === "error" || severity === "fatal") {
    row.classList.add("mech-repl-error");
    row.setAttribute("role", "alert");
  }
  if (!interaction) {
    row.classList.add("mech-console-error", "mech-program-diagnostic");
  }
  const heading = document.createElement("header");
  heading.textContent = [severity, diagnostic.code]
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
  if (interaction) {
    appendToTranscript(row);
  } else {
    target.append(row);
    markConsolePanelActivity("errors");
  }
}

function cancelFrame() {
  if (state.animationFrame !== null) {
    cancelAnimationFrame(state.animationFrame);
    state.animationFrame = null;
  }
}

function invalidateCooperativeOwnership() {
  const activeHostRequest = state.activeHostRequest;
  state.cooperativeOperationSequence += 1;
  state.activeCooperativeOperation = null;
  state.hostRequestSequence += 1;
  state.activeHostRequest = null;
  activeHostRequest?.controller.abort();
  state.replBusy = false;
  syncConsoleInputState();
}

function stopRuntime() {
  dismissInlineInspectors({ restoreFocus: false });
  state.running = false;
  setComputeBridgeLifecycle("stopped");
  state.computeBridgeBuildId += 1;
  cancelFrame();
  state.computeBridge?.retire();
  state.computeBridge = null;
  invalidateCooperativeOwnership();
  if (!state.document) {
    return;
  }
  state.replTerminated = true;
  state.replBusy = false;
  syncConsoleInputState();
  try {
    state.document.stop();
  } catch (error) {
    console.error(error);
  }
}

function showFatalError(error) {
  stopRuntime();
  state.replTerminated = true;
  state.replBusy = false;
  syncConsoleInputState();
  const message = appendError(error);
  if (documentConsolePane()) {
    if (isOutputPresentation()) {
      setDocumentPresentationView("workspace", { focus: false });
    }
    setConsoleOpen(true);
    activateConsolePanel("errors");
  }
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
  const address = element.dataset.mechOutputAddress || element.id;
  const separator = address.lastIndexOf(":");
  if (separator <= 0 || separator === address.length - 1) {
    throw new Error(`invalid Mech output address \`${address}\``);
  }
  return {
    outputId: BigInt(address.slice(0, separator)),
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

function setKindAnnotation(element, kind) {
  const complete = String(kind || "");
  const elided = complete.length > KIND_ANNOTATION_MAX_CHARACTERS;
  element.textContent = elided
    ? `${complete.slice(0, KIND_ANNOTATION_MAX_CHARACTERS - 1)}…`
    : complete;
  if (elided) {
    element.title = complete;
    element.dataset.mechKindElided = "true";
  } else {
    element.removeAttribute("title");
    delete element.dataset.mechKindElided;
  }
}

function createOutputEntry(address, rendered) {
  const row = document.createElement("article");
  row.className = "mech-document-output-entry";
  if (address.outputId !== undefined) {
    row.dataset.mechOutputId = address.outputId.toString();
  }
  if (address.selectionToken) {
    row.dataset.mechSelectionToken = address.selectionToken;
  }
  row.dataset.mechRenderedKind = rendered.kind;

  const heading = document.createElement("header");
  heading.className = "mech-document-output-heading";
  if (rendered.name) {
    const name = document.createElement("span");
    name.className = "mech-document-output-name";
    name.textContent = rendered.name;
    heading.append(name);
  }
  const kind = document.createElement("span");
  kind.className = "mech-output-kind";
  setKindAnnotation(kind, rendered.kind);
  heading.append(kind);
  const body = document.createElement("div");
  body.className = "mech-document-output-html mech-output-value";
  body.dataset.mechSource = "";
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
  if (state.directedProgramOutput) {
    return;
  }
  for (const entry of entries) {
    const element = createOutputEntry(entry.address, entry.rendered);
    panel.append(element);
    bindOutputClick(element, entry.address);
  }
}

function restoreImplicitProgramOutput() {
  const programOutput = state.document?.renderedProgramOutput?.() || null;
  refreshOutputPanel(programOutput
    ? [{ address: { selectionToken: programOutput.selectionToken }, rendered: programOutput }]
    : []);
}

function formattedMechInline(value) {
  const source = String(value ?? "_");
  const fragment = document.createDocumentFragment();
  const append = (text, className = "") => {
    const node = className ? document.createElement("span") : document.createTextNode(text);
    if (className) {
      node.className = className;
      node.textContent = text;
    }
    fragment.append(node);
  };
  let cursor = 0;
  while (cursor < source.length) {
    const rest = source.slice(cursor);
    if (rest[0] === '"') {
      let end = 1;
      let escaped = false;
      while (end < rest.length) {
        const character = rest[end];
        end += 1;
        if (!escaped && character === '"') {
          break;
        }
        if (!escaped && character === "\\") {
          escaped = true;
        } else {
          escaped = false;
        }
      }
      append(rest.slice(0, end), "mech-string");
      cursor += end;
      continue;
    }
    if (rest[0] === "<") {
      const close = rest.indexOf(">");
      if (close >= 0) {
        append(rest.slice(0, close + 1), "mech-kind-annotation");
        cursor += close + 1;
        continue;
      }
    }
    const atom = rest.match(/^:[A-Za-z_][A-Za-z0-9_./-]*/)?.[0];
    if (atom) {
      append(atom, "mech-atom");
      cursor += atom.length;
      continue;
    }
    const number = rest.match(/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?/)?.[0];
    if (number) {
      append(number, "mech-number");
      cursor += number.length;
      continue;
    }
    const boolean = rest.match(/^(?:true|false|none)(?![A-Za-z0-9_])/)?.[0];
    if (boolean) {
      append(boolean, "mech-boolean");
      cursor += boolean.length;
      continue;
    }
    const plain = rest.match(/^[^"<:+\-.\d]+/)?.[0] || rest[0];
    append(plain);
    cursor += plain.length;
  }
  return fragment;
}

function formattedValueElement(kind, value) {
  const rendered = document.createElement("span");
  rendered.className = "mech-repl-formatted-value";
  rendered.dataset.mechSource = "";
  if (kind) {
    const kindElement = document.createElement("span");
    kindElement.className = "mech-repl-result-kind mech-kind-annotation";
    setKindAnnotation(kindElement, kind);
    rendered.append(kindElement);
  }
  const valueElement = document.createElement("span");
  valueElement.className = "mech-repl-result-value";
  valueElement.append(formattedMechInline(value));
  rendered.append(valueElement);
  return rendered;
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
    appendLinkedText(body, data.text || "");
    return body;
  }
  if (content?.kind === "value") {
    body.append(formattedValueElement(data.kind || "", data.text || data.inline_text || "_"));
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
      const selectionToken = data.row_selection_tokens?.[rowIndex] || null;
      if (selectionToken) {
        row.dataset.mechSelectionToken = selectionToken;
      }
      if (mutedRows.has(rowIndex)) {
        row.classList.add("mech-repl-row-muted");
      }
      for (const [columnIndex, value] of values.entries()) {
        const cell = document.createElement("td");
        const column = String((data.columns || [])[columnIndex] || "").toLowerCase();
        if (column === "value") {
          cell.dataset.mechSource = "";
          cell.append(formattedMechInline(value));
        } else if (column === "type" || column === "kind") {
          cell.className = "mech-repl-result-kind";
          setKindAnnotation(cell, value);
        } else {
          cell.textContent = value;
        }
        row.append(cell);
      }
      tableBody.append(row);
    }
    table.append(head, tableBody);
    body.append(table);
    return body;
  }
  if (content?.kind === "matrix") {
    body.dataset.mechSource = "";
    body.append("[");
    const width = Math.max(1, data.columns || 0);
    for (const [index, cell] of (data.cells || []).entries()) {
      if (index > 0) {
        body.append(index % width === 0 ? "; " : " ");
      }
      body.append(formattedMechInline(cell));
    }
    body.append("]");
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
        for (const strip of scene.line_strips || []) {
          const element = document.createElementNS(namespace, "polyline");
          element.dataset.mechSceneId = strip.id;
          const points = [...(strip.positions || [])];
          if (strip.closed && points.length > 0) {
            points.push(points[0]);
          }
          for (const [name, value] of [
            ["points", points.map(point => `${point[0]},${point[1]}`).join(" ")],
            ["fill", "none"], ["stroke", strip.stroke],
            ["stroke-width", strip.stroke_width], ["stroke-linecap", strip.line_cap],
            ["stroke-linejoin", strip.line_join], ["opacity", strip.opacity],
          ]) {
            element.setAttribute(name, String(value));
          }
          svg.append(element);
        }
        for (const text of scene.texts || []) {
          const element = document.createElementNS(namespace, "text");
          element.dataset.mechSceneId = text.id;
          for (const [name, value] of [
            ["x", text.x], ["y", text.y], ["fill", text.fill],
            ["font-size", text.font_size], ["font-family", text.font_family],
            ["font-weight", text.font_weight], ["text-anchor", text.text_anchor],
            ["opacity", text.opacity],
          ]) {
            element.setAttribute(name, String(value));
          }
          element.textContent = text.value;
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

function appendLinkedText(target, text) {
  const source = String(text ?? "");
  const pattern = /https?:\/\/[^\s<>]+/g;
  let cursor = 0;
  for (const match of source.matchAll(pattern)) {
    target.append(source.slice(cursor, match.index));
    const link = document.createElement("a");
    link.href = match[0];
    link.textContent = match[0];
    link.rel = "noopener noreferrer";
    target.append(link);
    cursor = match.index + match[0].length;
  }
  target.append(source.slice(cursor));
}

function appendProgramOutput(output) {
  if (output.stream !== "stderr" && output.source) {
    state.directedProgramOutput = true;
    outputRegion("document")?.replaceChildren();
  }
  if (output.operation === "clear" && !output.display_id) {
    for (const entry of state.programDisplays.values()) {
      entry.element.remove();
    }
    state.programDisplays.clear();
    outputRegion("repl")?.replaceChildren();
    errorRegion("program")?.replaceChildren();
    state.directedProgramOutput = false;
    restoreImplicitProgramOutput();
    return;
  }
  const stderr = output.stream === "stderr";
  const target = stderr ? errorRegion("program") : outputRegion("repl");
  const scroller = target?.closest(".console-scroll") || target;
  const keepAtBottom = scroller
    ? scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight <= 24
    : false;
  const displayId = typeof output.display_id === "string"
    ? output.display_id
    : output.display_id?.[0] || null;
  let entry = displayId ? state.programDisplays.get(displayId) || null : null;
  if (output.operation === "remove" && displayId) {
    entry?.element.remove();
    state.programDisplays.delete(displayId);
    if (entry?.directed && ![...state.programDisplays.values()].some(value => value.directed)) {
      state.directedProgramOutput = false;
      restoreImplicitProgramOutput();
    }
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
      entry = {
        element: row,
        currentStream: output.stream,
        currentRegion: target,
        directed: Boolean(output.source),
      };
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
    entry.directed ||= Boolean(output.source);
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
    if (keepAtBottom) {
      scroller.scrollTop = scroller.scrollHeight;
    }
    markConsolePanelActivity(stderr ? "errors" : "output");
    return;
  }
  row.replaceChildren(outputContentElement(output.content));
  if (keepAtBottom) {
    scroller.scrollTop = scroller.scrollHeight;
  }
  markConsolePanelActivity(stderr ? "errors" : "output");
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

function dismissInlineInspectors(options = {}) {
  const inspectors = [...state.inlineInspectors.values()]
    .sort((left, right) => left.order - right.order);
  for (const [index, inspector] of inspectors.entries()) {
    inspector.dismiss({
      ...options,
      restoreFocus: Boolean(options.restoreFocus) && index === inspectors.length - 1,
    });
  }
}

function reflectiveElementIdentity(element, prefix) {
  let identity = state.reflectiveElementIdentities.get(element);
  if (!identity) {
    state.reflectiveElementIdentitySequence += 1;
    identity = `${prefix}-element:${state.reflectiveElementIdentitySequence}`;
    state.reflectiveElementIdentities.set(element, identity);
  }
  return identity;
}

function showInlineInspector(identity, title, rendered, anchor, error = null) {
  const existing = state.inlineInspectors.get(identity);
  if (existing) {
    existing.update(title, rendered, anchor, error);
    return;
  }
  const popup = document.createElement("aside");
  popup.className = "mech-inline-popup";
  popup.dataset.mechReplPopup = "true";
  popup.dataset.mechReplPopupIdentity = identity;
  popup.setAttribute("role", "dialog");
  popup.setAttribute("aria-label", `${title || "ans"} value inspector`);
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
  const content = document.createElement("div");
  content.className = "mech-inline-popup__content";
  const kind = document.createElement("div");
  kind.className = "mech-output-kind";
  setKindAnnotation(kind, error === null ? rendered?.kind || "" : "Error");
  const value = document.createElement("div");
  value.className = "mech-output-value";
  if (error === null) {
    value.dataset.mechSource = "";
    value.innerHTML = rendered?.blockHtml || rendered?.inlineHtml || "";
  } else {
    value.textContent = errorMessage(error);
    popup.classList.add("mech-inline-popup--error");
  }
  content.append(kind, value);
  header.append(heading, close);
  popup.append(header, content);
  document.body.append(popup);

  let currentAnchor = anchor;
  let inspector = null;
  let dragging = false;
  let dragOffsetX = 0;
  let dragOffsetY = 0;
  const viewportPadding = 12;
  const viewport = () => {
    const visual = window.visualViewport;
    const left = visual?.offsetLeft || 0;
    const top = visual?.offsetTop || 0;
    return {
      left,
      top,
      right: left + (visual?.width || window.innerWidth),
      bottom: top + (visual?.height || window.innerHeight),
    };
  };
  const clampPosition = (left, top) => ({
    left: Math.max(
      viewport().left + viewportPadding,
      Math.min(
        left,
        Math.max(
          viewport().left + viewportPadding,
          viewport().right - popup.offsetWidth - viewportPadding,
        ),
      ),
    ),
    top: Math.max(
      viewport().top + viewportPadding,
      Math.min(
        top,
        Math.max(
          viewport().top + viewportPadding,
          viewport().bottom - popup.offsetHeight - viewportPadding,
        ),
      ),
    ),
  });
  const applyPosition = (left, top) => {
    const position = clampPosition(left, top);
    popup.style.left = `${position.left}px`;
    popup.style.top = `${position.top}px`;
  };
  const reclamp = () => {
    if (!popup.isConnected) {
      return;
    }
    const rect = popup.getBoundingClientRect();
    applyPosition(rect.left, rect.top);
  };
  const move = event => {
    if (!dragging || !popup.isConnected) {
      return;
    }
    applyPosition(
      event.clientX - dragOffsetX,
      event.clientY - dragOffsetY,
    );
  };
  const stopDragging = () => {
    dragging = false;
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", stopDragging);
    window.removeEventListener("pointercancel", stopDragging);
  };
  const closeOnEscape = event => {
    const topmost = [...state.inlineInspectors.values()]
      .sort((left, right) => left.order - right.order)
      .at(-1);
    if (event.key === "Escape" && topmost === inspector) {
      // Every inspector owns a document-level listener. Consume this Escape
      // before dismissing so the next inspector cannot become topmost and
      // handle the same key event later in the listener queue.
      event.preventDefault();
      event.stopImmediatePropagation();
      dismiss();
    }
  };
  const dismiss = ({ restoreFocus = true } = {}) => {
    stopDragging();
    document.removeEventListener("keydown", closeOnEscape);
    window.removeEventListener("resize", reclamp);
    window.removeEventListener("orientationchange", reclamp);
    window.visualViewport?.removeEventListener("resize", reclamp);
    window.visualViewport?.removeEventListener("scroll", reclamp);
    popup.remove();
    if (state.inlineInspectors.get(identity) === inspector) {
      state.inlineInspectors.delete(identity);
    }
    if (restoreFocus && currentAnchor?.isConnected) {
      currentAnchor.focus({ preventScroll: true });
    }
  };
  const raise = () => {
    state.inlineInspectorSequence += 1;
    inspector.order = state.inlineInspectorSequence;
    popup.style.zIndex = String(12000 + inspector.order);
  };
  const update = (nextTitle, nextRendered, nextAnchor, nextError = null) => {
    currentAnchor = nextAnchor;
    heading.textContent = nextTitle || "ans";
    popup.setAttribute("aria-label", `${nextTitle || "ans"} value inspector`);
    setKindAnnotation(
      kind,
      nextError === null ? nextRendered?.kind || "" : "Error",
    );
    popup.classList.toggle("mech-inline-popup--error", nextError !== null);
    if (nextError === null) {
      value.dataset.mechSource = "";
      value.innerHTML = nextRendered?.blockHtml || nextRendered?.inlineHtml || "";
    } else {
      delete value.dataset.mechSource;
      value.textContent = errorMessage(nextError);
    }
    raise();
    reclamp();
    close.focus({ preventScroll: true });
  };
  inspector = { popup, dismiss, reclamp, raise, update, order: 0 };
  state.inlineInspectors.set(identity, inspector);
  raise();
  close.addEventListener("click", dismiss);
  popup.addEventListener("pointerdown", raise);
  document.addEventListener("keydown", closeOnEscape);
  window.addEventListener("resize", reclamp);
  window.addEventListener("orientationchange", reclamp);
  window.visualViewport?.addEventListener("resize", reclamp);
  window.visualViewport?.addEventListener("scroll", reclamp);
  header.addEventListener("pointerdown", event => {
    if (event.button !== 0 || event.target.closest("button")) {
      return;
    }
    const rect = popup.getBoundingClientRect();
    dragging = true;
    dragOffsetX = event.clientX - rect.left;
    dragOffsetY = event.clientY - rect.top;
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", stopDragging, { once: true });
    window.addEventListener("pointercancel", stopDragging, { once: true });
    event.preventDefault();
  });

  const anchorRect = anchor?.getBoundingClientRect();
  const popupRect = popup.getBoundingClientRect();
  let left = anchorRect ? anchorRect.right + viewportPadding : viewport().left + 24;
  if (anchorRect && left + popupRect.width > viewport().right - viewportPadding) {
    left = anchorRect.left - popupRect.width - viewportPadding;
  }
  applyPosition(left, anchorRect?.top ?? viewport().top + 96);
  close.focus({ preventScroll: true });
}

function replResponseHasDiagnostics(response) {
  return (response?.events || []).some(
    envelope => envelope.event?.channel === "diagnostic",
  );
}

function consumeSelection(selection, title, anchor, open, fallbackIdentity) {
  if (open) {
    consumeReplResponse(selection?.response);
    activateConsolePanel("console");
    state.console?.input?.focus();
  } else if (selection?.rendered) {
    showInlineInspector(
      selection.identity || fallbackIdentity,
      title,
      selection.rendered,
      anchor,
    );
  } else if (replResponseHasDiagnostics(selection?.response)) {
    consumeReplResponse(selection.response);
  }
}

function handleSelectionFailure(error, title, anchor, open, identity) {
  appendConsoleError(error);
  if (!open) {
    showInlineInspector(identity, title, null, anchor, error);
  }
}

function reflectiveSelectionAllowed() {
  return !state.replBusy && !state.replTerminated;
}

function bindSymbolClick(element, name, selectionToken = null) {
  if (!name || element.dataset.mechReplBound === "true") {
    return;
  }
  element.dataset.mechReplBound = "true";
  element.dataset.mechVarName = name;
  element.classList.add("mech-clickable");
  element.tabIndex = 0;
  element.setAttribute("role", "button");
  const fallbackIdentity = reflectiveElementIdentity(element, "symbol");
  const select = (event) => {
    event.preventDefault();
    event.stopPropagation();
    if (
      element.dataset.mechValueAvailable === "false" ||
      element.dataset.mechValueInteractive === "false" ||
      !reflectiveSelectionAllowed()
    ) {
      return;
    }
    try {
      const open = consoleIsOpen();
      consumeSelection(
        selectionToken
          ? state.repl.selectRetained(selectionToken, !open)
          : state.repl.selectSymbol(name, !open),
        name,
        element,
        open,
        fallbackIdentity,
      );
    } catch (error) {
      handleSelectionFailure(error, name, element, consoleIsOpen(), fallbackIdentity);
    }
  };
  element.addEventListener("click", select);
  element.addEventListener("keydown", event => {
    if (event.key === "Enter" || event.key === " ") {
      select(event);
    }
  });
}

function setReflectiveValueAvailability(element, available, interactive = true) {
  element.dataset.mechValueAvailable = String(available);
  element.dataset.mechValueInteractive = String(interactive);
  const enabled = available && interactive;
  element.classList.toggle("mech-clickable", enabled);
  if (enabled) {
    element.tabIndex = 0;
    element.setAttribute("role", "button");
    element.removeAttribute("aria-disabled");
  } else {
    element.removeAttribute("tabindex");
    element.removeAttribute("role");
    element.setAttribute("aria-disabled", "true");
  }
}

function bindOutputClick(element, address) {
  if (element.dataset.mechReplBound === "true") {
    return;
  }
  element.dataset.mechReplBound = "true";
  element.classList.add("mech-clickable");
  element.tabIndex = 0;
  element.setAttribute("role", "button");
  const fallbackIdentity = reflectiveElementIdentity(element, "output");
  const select = (event) => {
    event.preventDefault();
    event.stopPropagation();
    if (!reflectiveSelectionAllowed()) {
      return;
    }
    try {
      const open = consoleIsOpen();
      const selection = address.selectionToken
        ? state.repl.selectRetained(address.selectionToken, !open)
        : state.repl.selectOutput(address.outputId, !open);
      consumeSelection(
        selection,
        selection?.rendered?.name || "ans",
        element,
        open,
        fallbackIdentity,
      );
    } catch (error) {
      handleSelectionFailure(error, "ans", element, consoleIsOpen(), fallbackIdentity);
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
  output.dataset.mechSource = "";
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
    if (
      placeholder.dataset.mechConstraint === "true" ||
      placeholder.dataset.mechValueAvailable === "false"
    ) {
      continue;
    }
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
  for (const output of state.root?.querySelectorAll(".mech-block-output[id]") || []) {
    try {
      const address = outputAddress(output);
      const rendered = state.document.renderedOutput(address.outputId);
      if (rendered !== null) {
        output.dataset.mechSource = "";
        output.innerHTML = rendered.blockHtml;
        bindOutputClick(output, address);
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
      const rendered = state.document.renderedDocumentValue(
        placeholder.dataset.mechVarName,
      );
      if (rendered !== null) {
        placeholder.dataset.mechSource = "";
        placeholder.innerHTML = rendered.inlineHtml;
        placeholder.dataset.mechConstraint = String(rendered.interactive === false);
        setReflectiveValueAvailability(
          placeholder,
          true,
          rendered.interactive !== false,
        );
      } else {
        delete placeholder.dataset.mechSource;
        delete placeholder.dataset.mechConstraint;
        placeholder.textContent = "—";
        setReflectiveValueAvailability(placeholder, false, false);
      }
    } catch (error) {
      appendError(error);
    }
  }
  const programOutput = state.document.renderedProgramOutput?.() || null;
  refreshOutputPanel(programOutput
    ? [{ address: { selectionToken: programOutput.selectionToken }, rendered: programOutput }]
    : []);
  bindReflectiveValues();
  dispatch("mech:document-rendered");
}

// A read-only reflection seam for browser conformance harnesses and embedders.
// It observes an ordinary retained resident symbol; it does not allocate a
// compute presentation buffer or route scalar compute through JavaScript.
globalThis.__MECH_RENDERED_DOCUMENT_VALUE__ = (name) =>
  state.document?.renderedDocumentValue?.(String(name)) || null;

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
  const decoded = document.createElement("span");
  decoded.innerHTML = rendered.inlineHtml || "_";
  row.append(formattedValueElement(rendered.kind, decoded.textContent));
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
}

function clearReplDiagnostics() {
  for (const diagnostic of transcript()?.querySelectorAll(".mech-repl-diagnostic") || []) {
    diagnostic.remove();
  }
  errorPanel()?.querySelector('[data-mech-error-region="repl"]')?.remove();
  errorPanel()?.querySelector('[data-mech-error-region="program-diagnostics"]')?.remove();
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
  if (pending) {
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
  formatConsoleSource(code, source);
  row.append(prompt, code);
  appendToTranscript(row);
  return row;
}

function formatConsoleSource(target, source) {
  const formatted = state.repl?.formatSource?.(source) || null;
  if (!formatted) {
    target.textContent = source;
    return false;
  }
  target.dataset.mechSource = "";
  target.innerHTML = formatted;
  return true;
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
        if (content.kind === "fragments") {
          rendered.querySelector(".mech-repl-output-text")?.classList.add("mech-repl-logo");
        }
      }
      if (repl.payload?.kind === "symbol_inspection") {
        const table = rendered.querySelector("table");
        table?.classList.add("mech-repl-symbols");
        for (const symbolRow of table?.tBodies[0]?.rows || []) {
          const nameCell = symbolRow.cells[0];
          const name = nameCell?.textContent.trim();
          if (nameCell && name) {
            bindSymbolClick(
              nameCell,
              name,
              symbolRow.dataset.mechSelectionToken || null,
            );
          }
        }
      } else if (repl.payload?.kind === "integrity_constraint_inspection") {
        rendered.querySelector("table")?.classList.add("mech-repl-constraints");
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

function nextDocumentationFragmentNamespace() {
  state.documentationFragmentSequence += 1;
  return `mech-repl-documentation-${state.documentationFragmentSequence}`;
}

function namespaceDocumentationFragment(fragment, namespace) {
  const idMap = new Map();
  for (const element of fragment.querySelectorAll("[id]")) {
    const localId = element.id;
    const namespacedId = `${namespace}--${localId}`;
    if (
      element.matches(".mech-inline-mech-code, .mech-block-output") &&
      !element.dataset.mechOutputAddress
    ) {
      element.dataset.mechOutputAddress = localId;
    }
    idMap.set(localId, namespacedId);
    element.id = namespacedId;
  }

  const singleIdReferences = new Set(["for", "form", "list", "section"]);
  const idReferenceLists = new Set([
    "aria-activedescendant",
    "aria-controls",
    "aria-describedby",
    "aria-details",
    "aria-errormessage",
    "aria-flowto",
    "aria-labelledby",
    "aria-owns",
    "headers",
  ]);
  for (const element of fragment.querySelectorAll("*")) {
    for (const attribute of [...element.attributes]) {
      const name = attribute.name.toLowerCase();
      let value = attribute.value;
      if ((name === "href" || name === "xlink:href") && value.startsWith("#")) {
        const target = idMap.get(value.slice(1));
        if (target) {
          element.setAttribute(attribute.name, `#${target}`);
        }
        continue;
      }
      if (singleIdReferences.has(name)) {
        const target = idMap.get(value);
        if (target) {
          element.setAttribute(attribute.name, target);
        }
        continue;
      }
      if (idReferenceLists.has(name)) {
        value = value
          .split(/\s+/)
          .map(id => idMap.get(id) || id)
          .join(" ");
      }
      value = value.replace(/url\(#([^)]+)\)/g, (match, id) => {
        const target = idMap.get(id);
        return target ? `url(#${target})` : match;
      });
      if (value !== attribute.value) {
        element.setAttribute(attribute.name, value);
      }
    }
  }
}

async function fulfillReplHostRequest(requestId, request, signal) {
  if (request?.kind !== "documentation") {
    throw new Error(`unsupported browser REPL host request: ${request?.kind || "unknown"}`);
  }
  const topic = request.data?.topic?.trim() || "";
  if (!topic) {
    consumeReplResponse(state.document.replDocumentationIndex(requestId));
    return;
  }
  const parts = topic.split("/").filter(Boolean);
  if (parts.length !== 2 || parts.some(part => !/^[A-Za-z0-9._-]+$/.test(part))) {
    throw new Error("Usage: :docs <machine>/<document>");
  }
  const [machine, documentName] = parts;
  const url =
    `https://raw.githubusercontent.com/mech-machines/${encodeURIComponent(machine)}` +
    `/main/docs/${encodeURIComponent(documentName)}.mec`;
  const fetched = await fetch(url, { signal });
  if (!fetched.ok) {
    throw new Error(
      `failed to load documentation \`${topic}\`: ${fetched.status} ${fetched.statusText}`,
    );
  }
  const loaded = state.document.replLoadDocumentation(
    requestId,
    topic,
    await fetched.text(),
  );
  consumeReplResponse(loaded.response);
  const panel = outputRegion("repl");
  if (panel && loaded.accepted && loaded.html) {
    const row = document.createElement("article");
    row.className = "mech-repl-output-entry mech-repl-documentation";
    row.dataset.mechdown = "";
    row.dataset.mechDocumentationTopic = topic;
    row.innerHTML = loaded.html;
    namespaceDocumentationFragment(row, nextDocumentationFragmentNamespace());
    panel.append(row);
    activateConsolePanel("output");
    panel.scrollTop = panel.scrollHeight;
  }
  renderValues();
}

async function consumeCooperativeResponse(response) {
  const operation = ++state.cooperativeOperationSequence;
  state.activeCooperativeOperation = operation;
  consumeReplResponse(response);
  state.console.pendingSubmission = null;
  const stepRequestId = response?.pending ? response?.stepRequestId : null;
  const ownsHostRequest = Boolean(response?.hostRequest);
  const hostRequest = ownsHostRequest
    ? {
        controller: new AbortController(),
        id: response.hostRequestId,
        sequence: ++state.hostRequestSequence,
      }
    : null;
  if (hostRequest) {
    state.activeHostRequest = hostRequest;
  } else if (!response?.hostPending) {
    state.activeHostRequest = null;
  }
  state.replTerminated = Boolean(response?.terminated);
  state.replBusy = Boolean(response?.pending || response?.hostPending || response?.hostRequest);
  syncConsoleInputState();
  try {
    if (response?.pending && !stepRequestId) {
      throw new Error("pending cooperative step response omitted its request id");
    }
    if (response?.hostRequest) {
      await fulfillReplHostRequest(
        hostRequest.id,
        response.hostRequest,
        hostRequest.controller.signal,
      );
    }
    while (response?.pending) {
      await nextBrowserTurn();
      if (state.activeCooperativeOperation !== operation) {
        return;
      }
      response = state.repl.continueStep(128, stepRequestId);
      if (response?.pending && response?.stepRequestId !== stepRequestId) {
        throw new Error("cooperative step response changed request ownership");
      }
      consumeReplResponse(response);
      state.replTerminated = Boolean(response?.terminated);
    }
  } catch (error) {
    if (!hostRequest?.controller.signal.aborted) {
      throw error;
    }
  } finally {
    const releasesActiveOperation = state.activeCooperativeOperation === operation;
    const releasesActiveRequest =
      !hostRequest || state.activeHostRequest?.sequence === hostRequest.sequence;
    if (releasesActiveOperation && releasesActiveRequest) {
      state.activeCooperativeOperation = null;
      if (ownsHostRequest) {
        state.activeHostRequest = null;
      }
      state.replBusy = false;
      syncConsoleInputState();
      if (ownsHostRequest) {
        const finished = state.repl.finishHostRequest(hostRequest.id);
        consumeReplResponse(finished);
        state.replTerminated ||= Boolean(finished?.terminated);
      }
      if (state.replTerminated) {
        stopRuntime();
        setDocumentStatus("stopped");
        dispatch("mech:document-stopped");
      } else {
        renderValues();
      }
    }
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
  if (state.replBusy || state.replTerminated) {
    return;
  }
  if (state.console?.input !== input || !row.classList.contains("mech-repl-active-prompt")) {
    return;
  }
  const source = value.trim();
  if (!source) {
    return;
  }
  state.history.push(source);
  state.historyIndex = state.history.length;
  state.historyDraft = "";
  const code = document.createElement("span");
  code.className = "repl-code";
  formatConsoleSource(code, value);
  input.replaceWith(code);
  row.classList.remove("mech-repl-active-prompt");
  row.classList.add("mech-repl-source");
  state.console.pendingSubmission = { source, row };
  appendActivePrompt();
  try {
    const result = runConsoleCommand(source);
    if (result && typeof result.catch === "function") {
      result.catch(error => {
        state.console.pendingSubmission = null;
        appendConsoleError(error);
      });
    }
  } catch (error) {
    state.console.pendingSubmission = null;
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

function caretIsOnFirstLine(input) {
  const start = input.selectionStart ?? input.value.length;
  const end = input.selectionEnd ?? start;
  if (start !== end) {
    return false;
  }
  return !input.value.slice(0, start).includes("\n");
}

function recallConsoleHistory(input, direction) {
  if (!state.history.length) {
    return false;
  }
  if (!caretIsOnFirstLine(input)) {
    return false;
  }
  if (direction < 0 && state.historyIndex === state.history.length) {
    state.historyDraft = input.value;
  }
  const nextIndex = Math.max(
    0,
    Math.min(state.history.length, state.historyIndex + direction),
  );
  if (nextIndex === state.historyIndex) {
    return false;
  }
  state.historyIndex = nextIndex;
  input.value = nextIndex === state.history.length
    ? state.historyDraft
    : state.history[nextIndex];
  input.setSelectionRange(0, 0);
  input.dispatchEvent(new Event("input", { bubbles: true }));
  return true;
}

function resizeConsoleInput(input) {
  input.style.height = "0";
  input.style.height = `${input.scrollHeight}px`;
}

function scrollConsoleTranscript(key) {
  const target = transcript();
  if (!target) {
    return false;
  }
  const page = Math.max(1, Math.floor(target.clientHeight * 0.85));
  switch (key) {
    case "PageUp":
      target.scrollTop -= page;
      return true;
    case "PageDown":
      target.scrollTop += page;
      return true;
    case "Home":
      target.scrollTop = 0;
      return true;
    case "End":
      target.scrollTop = target.scrollHeight;
      return true;
    default:
      return false;
  }
}

function appendActivePrompt() {
  const target = transcript();
  if (!target || !state.console) {
    return;
  }
  const activeRows = [...target.querySelectorAll(":scope > .mech-repl-active-prompt")];
  const existing = activeRows.pop() || null;
  for (const duplicate of activeRows) {
    duplicate.remove();
  }
  const existingInput = existing?.querySelector(".repl-input") || null;
  if (existingInput) {
    state.console.inputRow = existing;
    state.console.input = existingInput;
    syncConsoleInputState();
    existingInput.focus();
    return;
  }
  existing?.remove();
  const inputRow = document.createElement("div");
  inputRow.className = "repl-line mech-repl-input-row mech-repl-active-prompt";
  const prompt = document.createElement("span");
  prompt.className = "repl-prompt";
  prompt.textContent = ">:";
  const input = document.createElement("textarea");
  input.className = "repl-input";
  input.dataset.mechInteractiveEvaluation = "resident";
  input.setAttribute("aria-label", "Mech resident REPL input");
  input.addEventListener("input", () => resizeConsoleInput(input));
  input.addEventListener("keydown", (event) => {
    if (event.isComposing) {
      return;
    }
    if (state.replBusy && event.ctrlKey && event.key.toLowerCase() === "c") {
      event.preventDefault();
      invalidateCooperativeOwnership();
      try {
        const response = state.repl.interrupt();
        consumeReplResponse(response);
        state.replTerminated = Boolean(response?.terminated);
        state.replBusy = Boolean(response?.pending || response?.hostPending);
        syncConsoleInputState();
      } catch (error) {
        appendConsoleError(error);
      }
      return;
    }
    if (
      !event.ctrlKey && !event.altKey && !event.shiftKey && !event.metaKey &&
      scrollConsoleTranscript(event.key)
    ) {
      event.preventDefault();
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
    if (event.key === "ArrowUp" && recallConsoleHistory(input, -1)) {
      event.preventDefault();
      return;
    }
    if (event.key === "ArrowDown" && recallConsoleHistory(input, 1)) {
      event.preventDefault();
    }
  });
  inputRow.append(prompt, input);
  target.append(inputRow);
  state.console.inputRow = inputRow;
  state.console.input = input;
  syncConsoleInputState();
  resizeConsoleInput(input);
  target.scrollTop = target.scrollHeight;
  input.focus();
}

function syncConsoleInputState() {
  const input = state.console?.input;
  if (!input) {
    return;
  }
  input.disabled = state.replTerminated;
  input.readOnly = state.replBusy && !state.replTerminated;
  input.setAttribute("aria-busy", String(state.replBusy));
  for (const target of statusTargets()) {
    if (state.activeHostRequest) {
      target.dataset.mechHostRequestId = String(state.activeHostRequest.id);
    } else {
      delete target.dataset.mechHostRequestId;
    }
  }
  setConsoleStatus(
    state.replTerminated ? "terminated" : state.replBusy ? "busy" : "ready",
  );
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
  const hint = document.createElement("div");
  hint.className = "mech-repl-hint";
  hint.textContent = "Enter submits · Ctrl+Enter adds a line · :help prints help";
  transcriptElement.append(hint);
  mount.append(transcriptElement);
  state.console = {
    mount,
    transcript: transcriptElement,
    input: null,
    inputRow: null,
    pendingSubmission: null,
  };
  mount.addEventListener("click", event => {
    if (
      event.target.closest("button, a, input, textarea, select, [contenteditable='true']") ||
      !window.getSelection()?.isCollapsed
    ) {
      return;
    }
    state.console?.input?.focus({ preventScroll: true });
  });
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
    ...root.querySelectorAll(":scope > [data-mech-console-resizer]"),
    ...root.querySelectorAll(":scope > #resizer, :scope > #edgeHandle"),
    ...(pane ? pane.querySelectorAll(":scope > [data-mech-console-resizer]") : []),
  ])];
}

function ensureConsoleWorkspaceResizers(pane = documentConsolePane()) {
  const panels = pane?.querySelector(":scope > .console-panels");
  if (!panels) {
    return [];
  }
  const specifications = [
    ["column", "vertical", "Resize Console and Output panels", 55],
    ["row", "horizontal", "Resize Output and Errors panels", 60],
  ];
  for (const [name, orientation, label, initialValue] of specifications) {
    if (panels.querySelector(`[data-mech-console-workspace-resizer="${name}"]`)) {
      continue;
    }
    const resizer = document.createElement("div");
    resizer.dataset.mechConsoleWorkspaceResizer = name;
    resizer.className = `console-workspace-resizer console-workspace-resizer-${name}`;
    resizer.setAttribute("role", "separator");
    resizer.setAttribute("aria-orientation", orientation);
    resizer.setAttribute("aria-label", label);
    resizer.setAttribute("aria-valuemin", "0");
    resizer.setAttribute("aria-valuemax", "100");
    resizer.setAttribute("aria-valuenow", String(initialValue));
    resizer.tabIndex = 0;
    panels.append(resizer);
  }
  return [...panels.querySelectorAll("[data-mech-console-workspace-resizer]")];
}

function documentConsoleToggles() {
  if (!state.root) {
    return [];
  }
  return [...new Set(state.root.querySelectorAll(
    ":scope > [data-mech-console-toggle], :scope > #toggle-repl",
  ))];
}

function documentConsoleFullscreenControls() {
  const root = state.root;
  const pane = documentConsolePane();
  if (!root) {
    return [];
  }
  return pane
    ? [...new Set(pane.querySelectorAll(
        "[data-mech-console-fullscreen], #consoleFullscreenToggle",
      ))]
    : [];
}

function documentOutputFullscreenControls() {
  const root = state.root;
  const pane = documentConsolePane();
  if (!root) {
    return [];
  }
  return pane
    ? [...new Set(pane.querySelectorAll(
        "[data-mech-output-fullscreen], #outputFullscreenToggle",
      ))]
    : [];
}

function ensureOutputFullscreenControl(pane = documentConsolePane()) {
  const existing = documentOutputFullscreenControls();
  if (existing.length || !pane) {
    return existing;
  }
  const topbar = pane.querySelector(":scope > .console-topbar");
  if (!topbar) {
    return [];
  }
  const control = document.createElement("button");
  control.className = "output-fullscreen-toggle";
  control.type = "button";
  control.dataset.mechOutputFullscreen = "";
  control.setAttribute("aria-pressed", "false");
  control.setAttribute("aria-label", "Enter fullscreen output");
  const icon = document.createElement("span");
  icon.setAttribute("aria-hidden", "true");
  icon.textContent = "⛶";
  control.append(icon);
  topbar.insertBefore(control, documentConsoleFullscreenControls()[0] || null);
  return [control];
}

function normalizeReplComponentContract() {
  if (!state.root) {
    return;
  }
  state.root.dataset.mechReplHost = "";
  const pane = documentConsolePane();
  if (pane) {
    pane.dataset.mechConsolePane = "";
    for (const tab of pane.querySelectorAll(
      "[data-mech-console-tab], .console-tab[data-tab]",
    )) {
      tab.dataset.mechConsoleTab ||= tab.dataset.tab || "";
    }
    const tablist = pane.querySelector(":scope > .console-topbar .console-tabs");
    if (tablist) {
      for (const name of ["output", "console", "errors"]) {
        const tab = [...tablist.querySelectorAll(
          "[data-mech-console-tab], .console-tab[data-tab]",
        )].find(candidate =>
          (candidate.dataset.mechConsoleTab || candidate.dataset.tab) === name
        );
        if (tab) {
          tablist.append(tab);
        }
      }
    }
    const legacyPanelNames = {
      "console-panel": "console",
      "output-panel": "output",
      "errors-panel": "errors",
    };
    for (const panel of pane.querySelectorAll(
      "[data-mech-console-panel], .console-panel[data-panel], #console-panel, #output-panel, #errors-panel",
    )) {
      panel.dataset.mechConsolePanel ||=
        panel.dataset.panel || legacyPanelNames[panel.id] || "";
      const name = panel.dataset.mechConsolePanel;
      panel.dataset.mechConsoleLabel ||= name
        ? `${name.charAt(0).toUpperCase()}${name.slice(1)}`
        : "Panel";
    }
    ensureConsoleWorkspaceResizers(pane);
  }
  for (const resizer of documentConsoleResizers()) {
    resizer.dataset.mechConsoleResizer = "";
    if (resizer.id === "edgeHandle" || resizer.classList.contains("edge-handle")) {
      resizer.dataset.mechConsoleEdgeHandle = "";
    }
  }
  for (const toggle of documentConsoleToggles()) {
    toggle.dataset.mechConsoleToggle = "";
  }
  for (const control of documentConsoleFullscreenControls()) {
    control.dataset.mechConsoleFullscreen = "";
  }
  for (const control of ensureOutputFullscreenControl(pane)) {
    control.dataset.mechOutputFullscreen = "";
  }
  const mount = state.root.querySelector("#mech-output");
  if (mount) {
    mount.dataset.mechRepl = "";
  }
  const output = outputPanel();
  if (output) {
    output.dataset.mechOutputPanel = "";
  }
  const errors = errorPanel();
  if (errors) {
    errors.dataset.mechErrorsPanel = "";
  }
}

function setConsoleOpen(open) {
  if (open) {
    dismissInlineInspectors({ restoreFocus: false });
  }
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
  pane.dataset.mechConsoleActivePanel = name;
  const workspace = pane.classList.contains("is-fullscreen");
  for (const candidate of pane.querySelectorAll(
    ".console-panel, [data-mech-console-panel], [data-panel]",
  )) {
    const selected = candidate === panel;
    candidate.hidden = workspace ? false : !selected;
    candidate.classList.toggle("active", selected);
    candidate.classList.toggle("is-active", selected);
  }
  for (const tab of pane.querySelectorAll(
    ".console-tab, [data-mech-console-tab], [data-tab]",
  )) {
    const selected = (tab.dataset.mechConsoleTab || tab.dataset.tab) === name;
    tab.classList.toggle("active", selected);
    tab.setAttribute("aria-selected", String(selected));
    if (selected) {
      clearConsoleTabUnread(tab);
    }
  }
}

function clearConsoleTabUnread(tab) {
  delete tab.dataset.mechConsoleUnread;
  updateConsoleTabAccessibleLabel(tab);
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
  activateConsolePanel(selectedConsolePanel(pane) || "console", pane);
}

function initializeConsoleToggle() {
  for (const toggle of documentConsoleToggles()) {
    toggle.addEventListener("click", () => {
      const isOpen = state.root?.dataset.mechConsoleOpen !== "false";
      setConsoleOpen(!isOpen);
    });
  }
}

function isOutputPresentation() {
  return state.root?.dataset.mechPresentation === "output";
}

function outputFullscreenActive() {
  return state.root?.dataset.mechOutputFullscreenActive === "true";
}

function setOutputFullscreenVisualState(active) {
  if (!state.root) {
    return;
  }
  state.root.dataset.mechOutputFullscreenActive = String(active);
  document.body.classList.toggle("output-fullscreen", active);
  if (active) {
    setConsoleOpen(true);
    activateConsolePanel("output");
  }
  for (const control of documentOutputFullscreenControls()) {
    control.setAttribute("aria-pressed", String(active));
    control.setAttribute(
      "aria-label",
      active ? "Exit fullscreen output" : "Enter fullscreen output",
    );
  }
  dispatch("mech:output-fullscreen", { active });
}

function setDocumentPresentationView(view) {
  if (!isOutputPresentation() || !state.root) {
    return;
  }
  const next = view === "workspace" ? "workspace" : "output";
  state.root.dataset.mechPresentationView = next;
  setConsoleOpen(true);
  activateConsolePanel("output");
  setOutputFullscreenVisualState(next === "output");
  dispatch("mech:presentation-view", { presentation: "output", view: next });
}

function initializeDocumentPresentation() {
  if (isOutputPresentation()) {
    setDocumentPresentationView("output");
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
    event.stopPropagation();
    if (outputFullscreenActive()) {
      if (state.outputFullscreenController) {
        void state.outputFullscreenController.exit({ revealWorkspace: true });
      } else {
        setOutputFullscreenVisualState(false);
        if (isOutputPresentation()) {
          setDocumentPresentationView("workspace");
        }
        setConsoleOpen(true);
        activateConsolePanel("output");
      }
      return;
    }
    const isOpen = state.root?.dataset.mechConsoleOpen !== "false";
    setConsoleOpen(!isOpen);
    if (isOpen) {
      return;
    }
    activateConsolePanel("console");
    requestAnimationFrame(() => state.console?.input?.focus());
  });
}

function beginConsolePointerSession(event, handle, axis, onMove, onFinish) {
  if (event.pointerType === "mouse" && event.button !== 0) {
    return;
  }
  event.preventDefault();
  state.consolePointerSession?.cancel();
  const pointerId = event.pointerId;
  let finished = false;
  let moved = false;
  const startX = event.clientX;
  const startY = event.clientY;
  const session = {
    cancel: () => finish(event, true),
  };
  const move = (moveEvent) => {
    if (moveEvent.pointerId !== pointerId) {
      return;
    }
    moved ||= Math.abs(moveEvent.clientX - startX) > 1 ||
      Math.abs(moveEvent.clientY - startY) > 1;
    onMove(moveEvent);
  };
  const finish = (finishEvent, cancelled = false) => {
    if (finished || finishEvent.pointerId !== pointerId) {
      return;
    }
    finished = true;
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", complete);
    window.removeEventListener("pointercancel", cancel);
    handle.removeEventListener("lostpointercapture", cancel);
    if (handle.hasPointerCapture?.(pointerId)) {
      handle.releasePointerCapture(pointerId);
    }
    if (state.consolePointerSession === session) {
      state.consolePointerSession = null;
    }
    document.body.classList.remove("is-resizing");
    delete document.body.dataset.mechResizeAxis;
    onFinish({ cancelled, moved });
  };
  const complete = finishEvent => finish(finishEvent, false);
  const cancel = finishEvent => finish(finishEvent, true);
  state.consolePointerSession = session;
  document.body.classList.add("is-resizing");
  document.body.dataset.mechResizeAxis = axis;
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", complete);
  window.addEventListener("pointercancel", cancel);
  handle.addEventListener("lostpointercapture", cancel);
  try {
    handle.setPointerCapture?.(pointerId);
  } catch (_error) {
    // Synthetic browser tests and older engines can reject pointer capture;
    // the window-owned session still provides complete cleanup.
  }
}

function initializeResizeHandles() {
  const pane = documentConsolePane();
  if (!pane || !state.root) {
    return;
  }
  for (const handle of documentConsoleResizers()) {
    handle.addEventListener("pointerdown", (event) => {
      const rect = pane.getBoundingClientRect();
      const horizontal = handle.dataset.mechConsoleResizeAxis === "width" ||
        pane.dataset.mechConsoleResizeAxis === "width" ||
        handle.id === "resizer" ||
        handle.id === "edgeHandle";
      const start = horizontal ? event.clientX : event.clientY;
      const initial = horizontal ? rect.width : rect.height;
      let ordinarySize = null;
      beginConsolePointerSession(event, handle, horizontal ? "width" : "height", (moveEvent) => {
        const delta = (horizontal ? moveEvent.clientX : moveEvent.clientY) - start;
        const requested = initial + (horizontal ? -delta : delta);
        const minimum = horizontal ? Math.min(370, window.innerWidth) : 160;
        const maximum = horizontal
          ? Math.max(minimum, Math.floor(state.root.getBoundingClientRect().width * 0.8))
          : 900;
        const overdrag = 48;
        if (horizontal && requested < minimum - overdrag) {
          ordinarySize = null;
          delete pane.dataset.mechFullscreenFallback;
          delete pane.dataset.mechFullscreenMode;
          const controls = documentConsoleFullscreenControls();
          if (controls.length) {
            for (const toggle of controls) {
              setFullscreenState(pane, toggle, false, null);
            }
          } else {
            pane.classList.remove("is-fullscreen");
            document.body.classList.remove("console-fullscreen");
            delete state.root.dataset.mechConsoleFullscreen;
          }
          setConsoleOpen(false);
          return;
        }
        setConsoleOpen(true);
        if (horizontal && requested > maximum + overdrag) {
          ordinarySize = null;
          pane.dataset.mechFullscreenFallback = "true";
          pane.dataset.mechFullscreenMode = "drag";
          pane.classList.add("is-fullscreen");
          for (const toggle of documentConsoleFullscreenControls()) {
            setFullscreenState(pane, toggle, true, "drag");
          }
          return;
        }
        if (
          horizontal &&
          pane.dataset.mechFullscreenFallback === "true" &&
          pane.dataset.mechFullscreenMode === "drag"
        ) {
          delete pane.dataset.mechFullscreenFallback;
          delete pane.dataset.mechFullscreenMode;
          pane.classList.remove("is-fullscreen");
          for (const toggle of documentConsoleFullscreenControls()) {
            setFullscreenState(pane, toggle, false, null);
          }
        }
        const size = Math.max(minimum, Math.min(maximum, requested));
        state.root.style.setProperty("--mech-console-size", `${size}px`);
        pane.style[horizontal ? "width" : "height"] = `${size}px`;
        ordinarySize = size;
      }, ({ cancelled, moved }) => {
        if (
          !cancelled && moved && ordinarySize !== null &&
          !pane.classList.contains("is-fullscreen")
        ) {
          saveConsoleOpeningSize(horizontal ? "width" : "height", ordinarySize);
        }
        if (handle.id === "edgeHandle" && !cancelled && !moved) {
          const isOpen = state.root?.dataset.mechConsoleOpen !== "false";
          setConsoleOpen(!isOpen);
        }
      });
    });
  }
}

function workspaceSizeMetrics(pane, resizer) {
  const panels = pane.querySelector(":scope > .console-panels");
  if (!panels) {
    return null;
  }
  const rect = panels.getBoundingClientRect();
  const axis = resizer.dataset.mechConsoleWorkspaceResizer;
  const total = axis === "column" ? rect.width : rect.height;
  if (total <= 0) {
    return null;
  }
  const columnMinimum = window.matchMedia("(max-width: 900px)").matches ? 180 : 240;
  const minimum = Math.min(
    axis === "column" ? columnMinimum : 120,
    Math.max(0, total / 2 - 4),
  );
  const maximum = Math.max(minimum, total - minimum - 8);
  return { axis, total, minimum, maximum };
}

function updateWorkspaceResizerAria(resizer, metrics, size) {
  const percentage = value => Math.round((value / metrics.total) * 100);
  resizer.setAttribute("aria-valuemin", String(percentage(metrics.minimum)));
  resizer.setAttribute("aria-valuemax", String(percentage(metrics.maximum)));
  resizer.setAttribute("aria-valuenow", String(percentage(size)));
}

function setWorkspaceSize(pane, resizer, requested) {
  const metrics = workspaceSizeMetrics(pane, resizer);
  if (!metrics) {
    return;
  }
  const { axis, total, minimum, maximum } = metrics;
  const size = Math.max(minimum, Math.min(maximum, requested));
  pane.style.setProperty(
    axis === "column" ? "--mech-console-workspace-left" : "--mech-console-workspace-top",
    `${(size / total) * 100}%`,
  );
  updateWorkspaceResizerAria(resizer, metrics, size);
}

function refreshWorkspaceResizers(pane) {
  if (!pane.classList.contains("is-fullscreen")) {
    return;
  }
  for (const resizer of ensureConsoleWorkspaceResizers(pane)) {
    const metrics = workspaceSizeMetrics(pane, resizer);
    if (!metrics) {
      continue;
    }
    const size = metrics.axis === "column"
      ? panelFor("console", pane)?.getBoundingClientRect().width || 0
      : panelFor("output", pane)?.getBoundingClientRect().height || 0;
    updateWorkspaceResizerAria(resizer, metrics, size);
  }
}

function initializeWorkspaceResizers() {
  const pane = documentConsolePane();
  if (!pane) {
    return;
  }
  for (const resizer of ensureConsoleWorkspaceResizers(pane)) {
    const axis = resizer.dataset.mechConsoleWorkspaceResizer;
    resizer.addEventListener("pointerdown", event => {
      if (!pane.classList.contains("is-fullscreen")) {
        return;
      }
      const panels = pane.querySelector(":scope > .console-panels");
      const rect = panels?.getBoundingClientRect();
      if (!rect) {
        return;
      }
      beginConsolePointerSession(event, resizer, axis, moveEvent => {
        setWorkspaceSize(
          pane,
          resizer,
          axis === "column"
            ? moveEvent.clientX - rect.left
            : moveEvent.clientY - rect.top,
        );
      }, () => {});
    });
    resizer.addEventListener("keydown", event => {
      if (!pane.classList.contains("is-fullscreen")) {
        return;
      }
      const backwards = axis === "column" ? event.key === "ArrowLeft" : event.key === "ArrowUp";
      const forwards = axis === "column" ? event.key === "ArrowRight" : event.key === "ArrowDown";
      if (!backwards && !forwards) {
        return;
      }
      event.preventDefault();
      const current = axis === "column"
        ? panelFor("console", pane)?.getBoundingClientRect().width || 0
        : panelFor("output", pane)?.getBoundingClientRect().height || 0;
      setWorkspaceSize(pane, resizer, current + (forwards ? 24 : -24));
    });
  }
  const refresh = () => refreshWorkspaceResizers(pane);
  window.addEventListener("resize", refresh);
  window.visualViewport?.addEventListener("resize", refresh);
}

function setFullscreenState(pane, toggle, active, mode = null) {
  const buttonFullscreen = active && mode === "button";
  pane.classList.toggle("is-fullscreen", active);
  document.body.classList.toggle("console-fullscreen", active);
  if (active && mode) {
    pane.dataset.mechFullscreenMode = mode;
    state.root?.setAttribute("data-mech-console-fullscreen", mode);
  } else if (!active) {
    delete pane.dataset.mechFullscreenMode;
    delete state.root?.dataset.mechConsoleFullscreen;
  }
  toggle.setAttribute("aria-pressed", String(buttonFullscreen));
  toggle.setAttribute(
    "aria-label",
    buttonFullscreen ? "Minimize console workspace" : "Enter fullscreen workspace",
  );
  activateConsolePanel(selectedConsolePanel(pane) || "console", pane);
  if (active) {
    for (const tab of pane.querySelectorAll(
      ".console-tab, [data-mech-console-tab], [data-tab]",
    )) {
      clearConsoleTabUnread(tab);
    }
    refreshWorkspaceResizers(pane);
  }
}

function initializeFullscreen() {
  const pane = documentConsolePane();
  const [toggle] = documentConsoleFullscreenControls();
  if (!pane || !toggle) {
    return;
  }
  let buttonFullscreenState = "idle";

  const synchronize = () => {
    const nativeFullscreen =
      document.fullscreenElement === pane && !outputFullscreenActive();
    if (nativeFullscreen) {
      buttonFullscreenState = "native";
      delete pane.dataset.mechFullscreenFallback;
    } else if (buttonFullscreenState === "native") {
      // Escape and browser-chrome exits are authoritative. Once an established
      // native session ends, the next button press must start a fresh entry.
      buttonFullscreenState = "idle";
      delete pane.dataset.mechFullscreenFallback;
      delete pane.dataset.mechFullscreenMode;
    }
    const fallbackFullscreen =
      ["requesting", "fallback"].includes(buttonFullscreenState) &&
      pane.dataset.mechFullscreenFallback === "true";
    const active = nativeFullscreen || fallbackFullscreen;
    const mode = nativeFullscreen
      ? "button"
      : fallbackFullscreen
        ? pane.dataset.mechFullscreenMode || "button"
        : null;
    setFullscreenState(pane, toggle, active, mode);
  };

  document.addEventListener("fullscreenchange", synchronize);
  synchronize();
  toggle.addEventListener("click", async () => {
    if (
      buttonFullscreenState !== "idle" ||
      pane.dataset.mechFullscreenMode === "button"
    ) {
      buttonFullscreenState = "idle";
      delete pane.dataset.mechFullscreenFallback;
      delete pane.dataset.mechFullscreenMode;
      synchronize();
      if (document.fullscreenElement === pane) {
        try {
          await document.exitFullscreen();
        } catch (error) {
          appendError(error);
        }
        synchronize();
      }
      return;
    }

    buttonFullscreenState = "requesting";
    pane.dataset.mechFullscreenFallback = "true";
    pane.dataset.mechFullscreenMode = "button";
    synchronize();
    if (pane.requestFullscreen) {
      try {
        await pane.requestFullscreen();
        if (buttonFullscreenState === "idle") {
          if (document.fullscreenElement === pane) {
            await document.exitFullscreen();
          }
          return;
        }
        if (document.fullscreenElement === pane) {
          buttonFullscreenState = "native";
          delete pane.dataset.mechFullscreenFallback;
        } else {
          buttonFullscreenState = "fallback";
        }
      } catch (error) {
        if (buttonFullscreenState !== "idle") {
          buttonFullscreenState = "fallback";
          pane.dataset.mechFullscreenFallback = "true";
          appendError(error);
        }
      }
    }
    synchronize();
  });
}

function initializeOutputFullscreen() {
  const pane = documentConsolePane();
  const controls = documentOutputFullscreenControls();
  if (!pane || !controls.length) {
    return;
  }
  let buttonState = "idle";

  const revealWorkspace = () => {
    setOutputFullscreenVisualState(false);
    if (isOutputPresentation()) {
      setDocumentPresentationView("workspace");
    } else {
      setConsoleOpen(true);
      activateConsolePanel("output");
    }
  };

  const synchronize = () => {
    const nativeFullscreen =
      document.fullscreenElement === pane && outputFullscreenActive();
    if (nativeFullscreen) {
      buttonState = "native";
    } else if (buttonState === "native") {
      buttonState = "idle";
      revealWorkspace();
    }
    setOutputFullscreenVisualState(outputFullscreenActive());
  };

  const exit = async ({ revealWorkspace: shouldReveal = true } = {}) => {
    buttonState = "idle";
    if (document.fullscreenElement === pane) {
      try {
        await document.exitFullscreen();
      } catch (error) {
        appendError(error);
      }
    }
    if (shouldReveal) {
      revealWorkspace();
    } else {
      setOutputFullscreenVisualState(false);
    }
  };

  const enter = async () => {
    buttonState = "requesting";
    if (isOutputPresentation()) {
      state.root.dataset.mechPresentationView = "output";
    }
    setOutputFullscreenVisualState(true);
    if (pane.requestFullscreen) {
      try {
        await pane.requestFullscreen();
        buttonState = document.fullscreenElement === pane ? "native" : "fallback";
      } catch (error) {
        buttonState = "fallback";
        appendError(error);
      }
    } else {
      buttonState = "fallback";
    }
    synchronize();
  };

  state.outputFullscreenController = { enter, exit };
  document.addEventListener("fullscreenchange", synchronize);
  for (const control of controls) {
    control.addEventListener("click", () => {
      if (outputFullscreenActive()) {
        void exit({ revealWorkspace: true });
      } else {
        void enter();
      }
    });
  }
  synchronize();
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
  state.tocEventCleanup?.();
  state.tocEventCleanup = null;
  if (state.tocUpdateFrame !== null) {
    cancelAnimationFrame(state.tocUpdateFrame);
    state.tocUpdateFrame = null;
  }
  for (const [link, handler] of state.tocLinkHandlers) {
    link.removeEventListener("click", handler);
  }
  state.tocLinkHandlers.clear();
  const tocControls = new Map();
  const controlCleanups = [];
  for (const layout of document.querySelectorAll(".article-layout, .docs-layout")) {
    const toc = layout.querySelector(".mech-toc, [data-mech-toc], .toc");
    const empty = !toc || !toc.querySelector("a[href^='#']");
    layout.classList.toggle("has-empty-toc", empty);
    layout.classList.toggle("is-toc-open", false);
    if (toc) {
      toc.hidden = empty;
    }
    let toggle = layout.querySelector(":scope > .mech-toc-toggle");
    if (!toggle && toc) {
      toggle = document.createElement("button");
      toggle.type = "button";
      toggle.className = "mech-toc-toggle";
      toggle.textContent = "Contents";
      layout.insertBefore(toggle, toc);
    }
    if (!toggle) {
      continue;
    }
    toggle.hidden = empty;
    toggle.setAttribute("aria-expanded", "false");
    if (empty || !toc) {
      toggle.removeAttribute("aria-controls");
      continue;
    }
    if (!toc.id) {
      toc.id = `${layout.id || "mech-document"}-contents`;
    }
    toggle.setAttribute("aria-controls", toc.id);
    toggle.setAttribute("aria-label", "Show document contents");
    const setOpen = (open) => {
      layout.classList.toggle("is-toc-open", open);
      toggle.setAttribute("aria-expanded", String(open));
      toggle.setAttribute(
        "aria-label",
        open ? "Close document contents" : "Show document contents",
      );
    };
    const activate = () => setOpen(!layout.classList.contains("is-toc-open"));
    toggle.addEventListener("click", activate);
    controlCleanups.push(() => toggle.removeEventListener("click", activate));
    tocControls.set(toc, { layout, toggle, setOpen });
  }
  const links = [...document.querySelectorAll(".mech-toc a[href^='#'], [data-mech-toc] a[href^='#']")];
  if (!links.length) {
    state.tocEventCleanup = () => {
      for (const cleanup of controlCleanups) cleanup();
    };
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
  if (!sections.length) {
    return;
  }
  const toc = links[0]?.closest(".mech-toc, [data-mech-toc], .toc") || null;
  const tocList = toc?.querySelector(":scope > ul") || links[0]?.closest("ul") || null;
  const topItems = [...(tocList?.children || [])]
    .filter(item => item instanceof HTMLLIElement);
  const topSections = topItems
    .map((item) => {
      const link = item.querySelector(":scope > a[href^='#']");
      return sections.find(section => section.link === link) || null;
    })
    .filter(Boolean);
  const primarySections = topSections.length ? topSections : sections;
  for (const { link, target } of sections) {
    const handler = (event) => {
      event.preventDefault();
      const control = tocControls.get(
        link.closest(".mech-toc, [data-mech-toc], .toc"),
      );
      if (control?.layout.classList.contains("is-toc-open")) {
        control.setOpen(false);
        requestAnimationFrame(() => {
          target.scrollIntoView({ behavior: "smooth", block: "start" });
        });
      } else {
        target.scrollIntoView({ behavior: "smooth", block: "start" });
      }
    };
    state.tocLinkHandlers.set(link, handler);
    link.addEventListener("click", handler);
  }
  const keepVisible = (link) => {
    if (!toc || toc.scrollHeight <= toc.clientHeight + 1) {
      return;
    }
    const tocRect = toc.getBoundingClientRect();
    const linkRect = link.getBoundingClientRect();
    if (linkRect.top < tocRect.top) {
      toc.scrollTop -= tocRect.top - linkRect.top;
    } else if (linkRect.bottom > tocRect.bottom) {
      toc.scrollTop += linkRect.bottom - tocRect.bottom;
    }
  };
  const select = (activeSection, options) => {
    const { activationLine, includeNested, nearBottom, viewportBottom } = options;
    for (const { link } of sections) {
      link.classList.remove("active", "active-path");
      link.removeAttribute("aria-current");
    }
    for (const item of toc?.querySelectorAll("li") || []) {
      item.classList.remove("active-path", "expanded");
    }
    const activeLink = activeSection.link;
    const activeItem = activeLink.closest("li");
    activeLink.classList.add("active", "active-path");
    activeItem?.classList.add("expanded");

    let currentLink = activeLink;
    if (includeNested && activeItem) {
      const nested = sections.filter(section =>
        section.link !== activeLink && activeItem.contains(section.link)
      );
      let activeNested = null;
      for (const section of nested) {
        const top = section.target.getBoundingClientRect().top;
        if (nearBottom ? top <= viewportBottom - 40 : top <= activationLine) {
          activeNested = section;
        }
      }
      if (nearBottom && !activeNested && nested.length) {
        activeNested = nested[nested.length - 1];
      }
      if (activeNested) {
        currentLink = activeNested.link;
        currentLink.classList.add("active", "active-path");
        let item = currentLink.closest("li");
        while (item && item !== activeItem) {
          item.classList.add("expanded");
          const parentItem = item.parentElement?.closest("li") || null;
          parentItem?.querySelector(":scope > a")?.classList.add("active-path");
          item = parentItem;
        }
      }
    }
    currentLink.setAttribute("aria-current", "location");
    keepVisible(currentLink);
  };
  const scrollContainer = primarySections[0]?.target.closest(".content-shell") || null;
  const scrollMetrics = () => {
    const containerStyle = scrollContainer ? getComputedStyle(scrollContainer) : null;
    const contained = Boolean(
      scrollContainer &&
      /auto|scroll|overlay/.test(containerStyle?.overflowY || "") &&
      scrollContainer.scrollHeight > scrollContainer.clientHeight + 1
    );
    if (contained) {
      return {
        top: scrollContainer.scrollTop,
        height: scrollContainer.clientHeight,
        scrollHeight: scrollContainer.scrollHeight,
        viewportTop: scrollContainer.getBoundingClientRect().top,
      };
    }
    const scrolling = document.scrollingElement || document.documentElement;
    return {
      top: window.scrollY,
      height: window.innerHeight,
      scrollHeight: scrolling.scrollHeight,
      viewportTop: 0,
    };
  };
  const update = () => {
    state.tocUpdateFrame = null;
    const metrics = scrollMetrics();
    const maximumScroll = Math.max(0, metrics.scrollHeight - metrics.height);
    const nearBottom = maximumScroll > 1 && metrics.top >= maximumScroll - 2;
    const sectionActivationLine = metrics.viewportTop + 20;
    const subsectionActivationLine = metrics.viewportTop +
      Math.min(metrics.height * 0.35, 280);
    let active = primarySections[0];
    if (nearBottom) {
      active = primarySections[primarySections.length - 1];
    } else if (metrics.top > 1) {
      for (const section of primarySections) {
        if (section.target.getBoundingClientRect().top > sectionActivationLine) {
          break;
        }
        active = section;
      }
    }
    select(active, {
      activationLine: subsectionActivationLine,
      includeNested: metrics.top > 1,
      nearBottom,
      viewportBottom: metrics.viewportTop + metrics.height,
    });
  };
  const schedule = () => {
    if (state.tocUpdateFrame === null) {
      state.tocUpdateFrame = requestAnimationFrame(update);
    }
  };
  const reconcileCompactState = () => {
    for (const { layout, toggle, setOpen } of tocControls.values()) {
      if (
        layout.classList.contains("is-toc-open") &&
        getComputedStyle(toggle).display === "none"
      ) {
        setOpen(false);
      }
    }
    schedule();
  };
  const closeOnEscape = (event) => {
    if (event.key !== "Escape") {
      return;
    }
    for (const { layout, toggle, setOpen } of tocControls.values()) {
      if (layout.classList.contains("is-toc-open")) {
        event.preventDefault();
        setOpen(false);
        toggle.focus({ preventScroll: true });
        break;
      }
    }
  };
  const resizeObserver = typeof ResizeObserver === "function"
    ? new ResizeObserver(reconcileCompactState)
    : null;
  for (const { layout } of tocControls.values()) {
    resizeObserver?.observe(layout);
  }
  window.addEventListener("scroll", schedule, { passive: true });
  scrollContainer?.addEventListener("scroll", schedule, { passive: true });
  window.addEventListener("resize", reconcileCompactState);
  document.addEventListener("keydown", closeOnEscape);
  state.tocEventCleanup = () => {
    resizeObserver?.disconnect();
    for (const cleanup of controlCleanups) cleanup();
    window.removeEventListener("scroll", schedule);
    scrollContainer?.removeEventListener("scroll", schedule);
    window.removeEventListener("resize", reconcileCompactState);
    document.removeEventListener("keydown", closeOnEscape);
  };
  update();
}

function initializeOptionalRenderers() {
  if (window.katex && typeof window.katex.render === "function") {
    const equationSelector = [
      "[data-mech-equation]:not([data-mech-rendered])",
      "[data-katex]:not([data-mech-rendered])",
      ".mech-inline-equation:not([data-mech-rendered])",
      ".mech-equation:not([data-mech-rendered])",
      ".math-inline:not([data-mech-rendered])",
      ".math-display:not([data-mech-rendered])",
    ].join(", ");
    for (const element of document.querySelectorAll(equationSelector)) {
      try {
        const source = element.getAttribute("equation") ?? element.textContent;
        window.katex.render(source, element, {
          displayMode:
            element.classList.contains("mech-equation") ||
            element.classList.contains("math-display"),
          throwOnError: false,
        });
        element.dataset.mechRendered = "katex";
      } catch (error) {
        appendError(error);
      }
    }
  }
  if (
    window.mermaid &&
    typeof window.mermaid.run === "function" &&
    document.querySelector(".mermaid:not([data-processed])")
  ) {
    try {
      if (!state.mermaidInitialized && typeof window.mermaid.initialize === "function") {
        window.mermaid.initialize({ startOnLoad: false, theme: "dark" });
        state.mermaidInitialized = true;
      }
      const nodes = document.querySelectorAll(".mermaid:not([data-processed])");
      Promise.resolve(window.mermaid.run({ nodes }))
        .catch(appendError);
    } catch (error) {
      appendError(error);
    }
  }
}

function syncReplHostOffset() {
  const header = document.querySelector(".site-header, #header");
  const position = header ? getComputedStyle(header).position : "";
  const overlaysViewport = position === "fixed" || position === "sticky";
  const offset = overlaysViewport
    ? Math.max(0, header.getBoundingClientRect().height)
    : 0;
  for (const host of document.querySelectorAll("[data-mech-repl-host]")) {
    host.style.setProperty("--mech-repl-top-offset", `${offset}px`);
  }
}

function initializePageStyleProbe() {
  state.replPageStyleProbe?.remove();
  const probe = document.createElement("span");
  probe.dataset.mechPageStyleProbe = "";
  probe.setAttribute("aria-hidden", "true");
  probe.style.cssText = [
    "position: fixed",
    "visibility: hidden",
    "pointer-events: none",
    "overflow: hidden",
    "width: var(--mech-page-style-signal, 0px)",
    "height: 0",
  ].join(";");
  (state.root || document.body).append(probe);
  state.replPageStyleProbe = probe;
  return probe;
}

function initializeLayout() {
  window.addEventListener("mech:output", event => {
    if (event instanceof CustomEvent && event.detail) {
      appendProgramOutput(event.detail);
    }
  });
  const pageStyleProbe = initializePageStyleProbe();
  syncReplHostOffset();
  window.addEventListener("resize", syncReplHostOffset);
  window.addEventListener("scroll", syncReplHostOffset, { passive: true });
  window.visualViewport?.addEventListener("scroll", syncReplHostOffset, {
    passive: true,
  });
  window.addEventListener("mech:styles-changed", syncReplHostOffset);
  const header = document.querySelector(".site-header, #header");
  if (typeof ResizeObserver === "function") {
    state.replHostOffsetObserver?.disconnect();
    state.replHostOffsetObserver = new ResizeObserver(syncReplHostOffset);
    for (const target of [header, pageStyleProbe].filter(Boolean)) {
      state.replHostOffsetObserver.observe(target);
    }
  }
  const styleRoot = document.head || document.documentElement;
  if (styleRoot && typeof MutationObserver === "function") {
    state.replStyleObserver?.disconnect();
    state.replStyleObserver = new MutationObserver(syncReplHostOffset);
    state.replStyleObserver.observe(styleRoot, {
      attributes: true,
      attributeFilter: ["disabled", "href", "media", "rel"],
      characterData: true,
      childList: true,
      subtree: true,
    });
  }
  initializeDocumentLayoutPersistence();
  initializeConsoleState();
  initializeConsoleTabs();
  initializeDocumentPresentation();
  initializeConsoleErrorBadge();
  initializeConsoleToggle();
  initializeConsoleKeyboardToggle();
  initializeResizeHandles();
  initializeWorkspaceResizers();
  initializeFullscreen();
  initializeOutputFullscreen();
  initializeBreadcrumb();
  window.addEventListener("mech:document-layout-refresh", initializeToc);
  initializeToc();
  initializeOptionalRenderers();
  window.addEventListener("load", initializeOptionalRenderers, { once: true });
}

function servedComputeHostConfig() {
  const authority = window.__MECH_HOST_CONFIG;
  const hosts = authority?.hosts || authority?.payload?.hosts || [];
  return hosts.find(host => host?.provider === "compute") || null;
}

async function probeServedComputeAdapter() {
  const computeHost = servedComputeHostConfig();
  if (!computeHost) {
    return;
  }
  const requestedBackend = computeHost.settings?.backend || "auto";
  document.documentElement.dataset.mechComputeAdapterStatus = "requesting";
  try {
    state.computeAdapter = requestedBackend === "cpu-scalar" || !navigator.gpu
      ? null
      : await navigator.gpu.requestAdapter({ powerPreference: "high-performance" });
  } catch (error) {
    document.documentElement.dataset.mechComputeAdapterStatus = "failed";
    if (requestedBackend !== "auto") {
      throw error;
    }
    state.computeAdapter = null;
  }
  // Runtime construction is synchronous. Publish the result of the actual
  // adapter probe so `auto` selects the same backend the bridge can execute,
  // including browsers that expose navigator.gpu but return no adapter.
  window.__MECH_GPU_AVAILABLE = Boolean(state.computeAdapter);
  document.documentElement.dataset.mechComputeAdapterStatus = state.computeAdapter
    ? "ready"
    : "unavailable";
}

class DocumentComputeBridge {
  static async create(controller) {
    if (typeof controller.computeManifest !== "function") {
      return null;
    }
    const manifest = controller.computeManifest();
    if (!manifest) {
      return null;
    }
    const backend = controller.computeBackend();
    if (backend !== "wgpu") {
      // Resident scalar compute completes inside the runtime. It has no
      // browser transport, command queue, or full-output presentation bridge.
      document.documentElement.dataset.mechComputeBackend = backend;
      document.documentElement.dataset.mechComputeInstances = String(
        manifest.dispatchElements,
      );
      return null;
    }
    const adapter = state.computeAdapter !== undefined
      ? state.computeAdapter
      : navigator.gpu
        ? await navigator.gpu.requestAdapter({ powerPreference: "high-performance" })
        : null;
    if (!adapter) {
      throw new Error("the document selected WebGPU but no compatible adapter is available");
    }
    document.documentElement.dataset.mechComputeDeviceStatus = "requesting";
    let resource;
    try {
      resource = await globalThis.MechBrowserComputeDevice.create(manifest, adapter, []);
    } catch (error) {
      document.documentElement.dataset.mechComputeDeviceStatus = "failed";
      throw error;
    }
    document.documentElement.dataset.mechComputeDeviceStatus = "ready";
    return new DocumentComputeBridge(controller, manifest, backend, resource);
  }

  constructor(controller, manifest, backend, resource) {
    this.controller = controller;
    this.manifest = manifest;
    this.backend = backend;
    this.resource = resource;
    this.device = resource?.device || null;
    this.pipeline = resource?.pipeline || null;
    this.activeBuffer = 0;
    this.blocked = false;
    this.failure = null;
    this.dispatches = 0;
    this.retired = false;
    this.generation = typeof controller.computeGeneration === "function"
      ? controller.computeGeneration()
      : "0";
    this.lifecycle = new globalThis.MechComputeSubmissionLifecycle(this.generation);
    this.device?.lost.then((info) => {
      if (this.isCurrent()) {
        const failure = new Error(`GPU device lost: ${info.message || info.reason || "unknown reason"}`);
        failure.mechDeviceLost = true;
        this.failure = this.lifecycle.markFailed(failure);
      }
    });
    document.documentElement.dataset.mechComputeBackend = this.backend;
    document.documentElement.dataset.mechComputeInstances = String(
      this.manifest.dispatchElements,
    );
  }

  isCurrent() {
    return !this.retired && (
      typeof this.controller.computeGeneration !== "function" ||
      this.controller.computeGeneration() === this.generation
    );
  }

  retire() {
    this.retired = true;
    this.resource?.dispose();
  }

  publishCompletion(outputs) {
    this.dispatches += 1;
    document.documentElement.dataset.mechComputeDispatches = String(this.dispatches);
    window.dispatchEvent(new CustomEvent("mech:compute-complete", {
      detail: {
        backend: this.backend,
        completedTurns: this.dispatches,
        outputs: (outputs || []).map(output => ({
          name: output.name,
          values: output.values instanceof Float32Array
            ? output.values
            : Float32Array.from(output.values || []),
        })),
      },
    }));
  }

  submit(command) {
    if (!command?.dispatch) {
      return;
    }
    if (!this.isCurrent()) {
      throw new Error("a retired document compute bridge received a dispatch");
    }
    if (this.blocked) {
      throw new Error("a checked document compute dispatch is already in flight");
    }
    if (
      command.acknowledgementRequired !== true ||
      typeof command.dispatchToken !== "string" ||
      !/^[1-9][0-9]*:[1-9][0-9]*$/.test(command.dispatchToken)
    ) {
      throw new Error("the document compute command has no valid completion identity");
    }
    this.resource.setRequestedOutputs(command.requestedOutputs || []);
    const { outputIndex } = this.resource.submit(command, this.activeBuffer);
    // queue.submit() has succeeded at this point. Record that fact before any
    // promise continuation can observe device loss; this generation may no
    // longer be rebuilt on scalar compute automatically.
    this.lifecycle.markSubmitted(command.dispatchToken);
    this.blocked = true;
    this.finish(command.dispatchToken, outputIndex);
  }

  async finish(dispatchToken, outputIndex) {
    try {
      const { outputs, integrity } = await this.resource.finish();
      if (integrity) {
        if (this.isCurrent()) {
          this.controller.rejectIntegrityComputeCommand(
            dispatchToken,
            integrity.constraint,
            integrity.instance,
          );
          // Integrity rejection is a matching terminal completion: the
          // candidate state was not published, but the host is reusable for
          // the next independent turn.
          this.lifecycle.markAccepted(dispatchToken);
        }
        return;
      }
      if (this.isCurrent()) {
        if (outputs.length) {
          this.controller.completeComputeCommand(dispatchToken, outputs);
        } else {
          this.controller.acknowledgeComputeCommand(dispatchToken);
        }
        this.lifecycle.markAccepted(dispatchToken);
        this.activeBuffer = outputIndex;
        this.publishCompletion(outputs);
      }
    } catch (error) {
      try {
        if (!this.isCurrent()) {
          return;
        }
        this.controller.rejectComputeCommand(
          dispatchToken,
          error instanceof Error ? error.message : String(error),
        );
      } catch (rejectionError) {
        this.failure = rejectionError;
      }
      this.failure = this.lifecycle.markFailed(this.failure || error);
    } finally {
      this.blocked = false;
    }
  }
}

function refreshDocumentComputeBridge() {
  const bridge = state.computeBridge;
  const generation = typeof state.document?.computeGeneration === "function"
    ? state.document.computeGeneration()
    : "0";
  if (
    state.computeBridgeRefresh ||
    (state.computeBridgeGeneration === generation && (!bridge || bridge.isCurrent()))
  ) {
    return Boolean(state.computeBridgeRefresh);
  }
  bridge?.retire();
  state.computeBridge = null;
  const buildId = ++state.computeBridgeBuildId;
  const controller = state.document;
  setComputeBridgeLifecycle("building");
  const refresh = createDocumentComputeBridgeWithFallback(controller)
    .then(next => {
      const currentGeneration = typeof controller?.computeGeneration === "function"
        ? controller.computeGeneration()
        : "0";
      if (
        buildId !== state.computeBridgeBuildId ||
        controller !== state.document ||
        state.computeBridgeLifecycle === "stopped" ||
        next?.generation !== currentGeneration
      ) {
        next?.retire();
        return;
      }
      state.computeBridge = next;
      state.computeBridgeGeneration = typeof controller?.computeGeneration === "function"
        ? controller.computeGeneration()
        : generation;
      setComputeBridgeLifecycle("ready");
    })
    .catch(error => {
      if (buildId === state.computeBridgeBuildId) {
        setComputeBridgeLifecycle("fatal");
        showFatalError(error);
      }
    })
    .finally(() => {
      if (state.computeBridgeRefresh === refresh) state.computeBridgeRefresh = null;
      if (state.running && buildId === state.computeBridgeBuildId) {
        state.animationFrame = requestAnimationFrame(frame);
      }
    });
  state.computeBridgeRefresh = refresh;
  return true;
}

function frame() {
  if (!state.running || !state.document) {
    return;
  }
  try {
    if (refreshDocumentComputeBridge()) {
      return;
    }
    if (state.computeBridge?.failure) {
      const failure = state.computeBridge.failure;
      const requestedBackend = servedComputeHostConfig()?.settings?.backend || "auto";
      if (
        failure.mechDeviceLost &&
        requestedBackend === "auto" &&
        state.computeBridge.lifecycle.canAutoFallback()
      ) {
        state.computeBridge.failure = null;
        state.computeBridge.retire();
        state.computeBridge = null;
        state.computeAdapter = null;
        window.__MECH_GPU_AVAILABLE = false;
        state.document.fallbackComputeToCpu();
        refreshDocumentComputeBridge();
        return;
      }
      throw failure;
    }
    if (state.computeBridge?.blocked) {
      // The resident compute host is in its single InFlight phase. Leave
      // timer and pointer packets queued (or coalesced by their configured
      // policy) until the matching completion has been accepted.
      state.animationFrame = requestAnimationFrame(frame);
      return;
    }
    const result = state.document.frame(8);
    state.computeBridge?.submit(result.computeCommand);
    if (result.events?.length) {
      consumeReplResponse(result);
    }
    if (result.processed > 0) {
      renderValues();
    }
    state.animationFrame = requestAnimationFrame(frame);
  } catch (error) {
    if (isRecoverableResidentTurnError(error)) {
      appendError(error);
      state.animationFrame = requestAnimationFrame(frame);
      return;
    }
    showFatalError(error);
  }
}

function constructDocumentController(WasmDocument, documentSources) {
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
    return documentSources.version === 2
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
  }
  if (documentSources) {
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
    return documentSources.version === 2
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
  }
  return WasmDocument.fromEncoded(state.initialEncoded);
}

async function createDocumentComputeBridgeWithFallback(controller = state.document) {
  try {
    return await DocumentComputeBridge.create(controller);
  } catch (error) {
    const requestedBackend = servedComputeHostConfig()?.settings?.backend || "auto";
    if (requestedBackend !== "auto" || controller.computeBackend() !== "wgpu") {
      throw error;
    }
    state.computeAdapter = null;
    window.__MECH_GPU_AVAILABLE = false;
    if (typeof controller.fallbackComputeToCpu !== "function") {
      throw new Error(
        "automatic WebGPU fallback requires a browser runtime that can rebuild the accepted generation",
        { cause: error },
      );
    }
    setComputeBridgeLifecycle("falling-back");
    controller.fallbackComputeToCpu();
    const bridge = await DocumentComputeBridge.create(controller);
    if (bridge?.backend === "wgpu") {
      throw error;
    }
    return bridge;
  }
}

async function main() {
  state.root = documentRoot();
  if (!state.root) {
    throw new Error("the document controller requires a .mech-root element");
  }
  normalizeReplComponentContract();
  setDocumentStatus("loading");
  state.replBusy = true;
  attachConsole();
  initializeLayout();
  syncConsoleInputState();
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
    await probeServedComputeAdapter();
  }
  state.document = constructDocumentController(WasmDocument, documentSources);
  if (typeof state.document.replInvoke !== "function") {
    throw new Error(
      "the browser WASM build does not include the document-backed resident REPL host",
    );
  }
  setComputeBridgeLifecycle("building");
  const initialBuildId = ++state.computeBridgeBuildId;
  state.computeBridge = await createDocumentComputeBridgeWithFallback(state.document);
  if (initialBuildId !== state.computeBridgeBuildId) {
    state.computeBridge?.retire();
    return;
  }
  setComputeBridgeLifecycle("ready");
  state.computeBridgeGeneration = typeof state.document.computeGeneration === "function"
    ? state.document.computeGeneration()
    : "0";
  state.repl = {
    invoke: source => state.document.replInvoke(source),
    continueStep: (count, requestId) => state.document.replContinueStep(count, requestId),
    interrupt: () => state.document.replInterrupt(),
    setQuiet: quiet => state.document.replSetQuiet(quiet),
    setValueElementLimit: maxElements =>
      state.document.replSetValueElementLimit(maxElements),
    formatSource: source => state.document.replFormatSource(source),
    finishHostRequest: requestId => state.document.replFinishHostRequest(requestId),
    selectSymbol: (name, renderPopup) =>
      state.document.replSelectSymbol(name, renderPopup),
    selectRetained: (selectionToken, renderPopup) =>
      state.document.replSelectRetained(selectionToken, renderPopup),
    selectOutput: (outputId, renderPopup) =>
      state.document.replSelectOutput(outputId, renderPopup),
  };
  state.replQuiet = requestedReplQuiet();
  consumeReplResponse(state.repl.setQuiet(state.replQuiet));
  consumeReplResponse(state.repl.setValueElementLimit(requestedReplValueElementLimit()));
  state.replBusy = false;
  syncConsoleInputState();
  prepareVarPlaceholders();
  renderValues();
  state.document.start();
  state.running = true;
  setDocumentStatus("ready");
  restorePagePosition();
  dispatch("mech:document-ready");
  state.animationFrame = requestAnimationFrame(frame);
}

window.addEventListener("beforeunload", () => {
  flushPagePositionSave();
  stopRuntime();
  if (document.documentElement.dataset.mechDocumentStatus !== "error") {
    setDocumentStatus("stopped");
  }
});

main().catch(showFatalError);
