// Presentation is independent of the optional executable document controller.
(() => {
  const state = { optionalRendererLoads: new Map(), mermaidInitialized: false };
  let reportError = error => console.error(error);
  const appendError = error => reportError(error);
  function loadOptionalRenderer(name) {
    if (state.optionalRendererLoads.has(name)) return;
    const resource = document.querySelector(`script[data-mech-renderer="${name}"]`);
    const source = resource?.dataset.mechRendererSrc;
    if (!source) return;
    const ready = new Promise(resolve => {
      const script = document.createElement("script");
      script.async = true;
      script.src = source;
      if (resource.crossOrigin) script.crossOrigin = resource.crossOrigin;
      script.addEventListener("load", () => resolve(true), { once: true });
      script.addEventListener("error", () => resolve(false), { once: true });
      document.head.append(script);
    });
    state.optionalRendererLoads.set(name, ready);
    ready.then(loaded => {
      if (loaded) initializeOptionalRenderers();
    });
  }

  function initializeOptionalRenderers() {
    const equationSelector = [
      "[data-mech-equation]:not([data-mech-rendered])",
      "[data-katex]:not([data-mech-rendered])",
      ".mech-inline-equation:not([data-mech-rendered])",
      ".mech-equation:not([data-mech-rendered])",
      ".math-inline:not([data-mech-rendered])",
      ".math-display:not([data-mech-rendered])",
    ].join(", ");
    const equations = document.querySelectorAll(equationSelector);
    if (equations.length) {
      const stylesheet = document.querySelector('link[data-mech-renderer="katex"]');
      if (stylesheet && !stylesheet.href && stylesheet.dataset.mechRendererHref) {
        stylesheet.href = stylesheet.dataset.mechRendererHref;
      }
      if (!window.katex || typeof window.katex.render !== "function") {
        loadOptionalRenderer("katex");
      } else {
        for (const element of equations) {
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
    }
    if (document.querySelector(".mermaid:not([data-processed])") &&
        (!window.mermaid || typeof window.mermaid.run !== "function")) {
      loadOptionalRenderer("mermaid");
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

  window.MechPresentation = Object.freeze({
    refresh(onError) {
      if (onError) reportError = onError;
      initializeOptionalRenderers();
    },
  });
  window.MechPresentation.refresh();
})();
