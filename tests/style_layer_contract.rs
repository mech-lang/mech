use std::fs;
use std::path::PathBuf;

fn include(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("include")
        .join(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

#[test]
fn shipped_shims_expose_the_four_independent_style_layers_in_order() {
    for shim in ["index.html", "blog.html", "docs.html"] {
        let html = include(shim);
        let mut previous = 0;
        for layer in ["source", "mechdown", "page", "repl"] {
            let marker = format!("data-mech-style-layer=\"{layer}\"");
            assert_eq!(
                html.matches(&marker).count(),
                1,
                "{shim} must own one {layer} layer"
            );
            let position = html.find(&marker).unwrap();
            assert!(position >= previous, "{shim} put {layer} out of order");
            previous = position;
        }
        assert!(html.contains("data-mech-repl-host"));
        assert!(html.contains("data-mechdown"));
    }
}

#[test]
fn source_layer_keeps_the_structured_syntax_highlighting_contract() {
    let css = include("mech-source.css");
    assert!(css.contains("[data-mech-source]"));
    for token in [
        ".mech-code-block",
        ".mech-var-name",
        ".mech-number",
        ".mech-kind-annotation",
        ".mech-function-name",
        ".mech-matrix",
        ".mech-table",
    ] {
        assert!(css.contains(token), "source layer lost {token}");
    }
    for foreign in ["body {", ".console-pane", ".site-header", "[data-mechdown]"] {
        assert!(!css.contains(foreign), "source layer leaked {foreign}");
    }
}

#[test]
fn page_variants_do_not_own_source_or_repl_components() {
    for stylesheet in ["style.css", "blog.css", "docs.css"] {
        let css = include(stylesheet);
        assert!(
            css.contains("--mech-page-style-signal: 1px"),
            "{stylesheet} lost the observable page-style lifecycle signal",
        );
        for component in [
            ".mech-code-block",
            ".mech-number",
            ".mech-repl",
            ".console-pane",
            ".repl-input",
            ".resize-handle",
            ".edge-handle",
            ".document-console-toggle",
            ".mech-backmatter-heading",
            "[data-mechdown]",
            "--mech-repl-",
            "--console-width",
        ] {
            assert!(
                !css.contains(component),
                "{stylesheet} still owns component selector {component}",
            );
        }
    }
}

#[test]
fn mechdown_layer_owns_the_portable_editorial_hierarchy() {
    let css = include("mechdown.css");
    for contract in [
        "counter-reset: mechdown-section",
        ".mechdown-section.mechdown-titled-section {\n  counter-increment: mechdown-section",
        ".mechdown-section > h2::before",
        "content: \"section \" counter(mechdown-section, decimal)",
        ".mechdown-section > h3::before",
        "counter(mechdown-subsection, decimal)",
        ".mech-abstract {",
        "border: 1px solid var(--mechdown-accent)",
        "--mechdown-inline-code: var(--var-name-color, hsl(290, 45%, 90%))",
        "color: var(--mechdown-inline-code)",
    ] {
        assert!(
            css.contains(contract),
            "Mechdown lost editorial style {contract}"
        );
    }
}

#[test]
fn document_controller_keeps_panel_discovery_inside_its_selected_root() {
    let controller = include("document.js");
    assert!(controller.contains(
        "return state.root\n    ? state.root.querySelector(selector)\n    : document.querySelector(selector);"
    ));
    assert!(controller.contains("controllerQuery(ERROR_PANEL_SELECTOR)"));
    assert!(controller.contains("controllerQuery(OUTPUT_PANEL_SELECTOR)"));
}

#[test]
fn document_controller_initializes_repl_controls_before_starting_wasm() {
    let controller = include("document.js");
    let main = controller
        .split_once("async function main() {")
        .expect("document controller must expose main")
        .1;
    let attach = main.find("attachConsole();").unwrap();
    let layout = main.find("initializeLayout();").unwrap();
    let wasm_import = main.find("await import(wasmModule)").unwrap();

    assert!(
        attach < wasm_import,
        "console controls must survive startup errors"
    );
    assert!(
        layout < wasm_import,
        "layout controls must survive startup errors"
    );

    let fatal = controller
        .split_once("function showFatalError(error) {")
        .expect("document controller must expose fatal error handling")
        .1
        .split_once("\n}")
        .unwrap()
        .0;
    assert!(fatal.contains("activateConsolePanel(\"errors\")"));
    assert!(fatal.contains("state.replBusy = false;"));
}

#[test]
fn document_controller_keeps_toc_and_error_activity_state_continuous() {
    let controller = include("document.js");
    assert!(controller.contains("function documentPageScrollOwner(requestedOwner = null)"));
    assert!(controller.contains("requestedOwner === \"content-shell\" && contentShell"));
    assert!(controller.contains("contentShell.scrollHeight > contentShell.clientHeight + 1"));
    assert!(
        controller.contains("saved?.owner === \"content-shell\" ? \"content-shell\" : \"window\"")
    );
    assert!(controller.contains("owner: owner === window ? \"window\" : \"content-shell\""));
    assert!(controller.contains("owner === window ? window.scrollY : owner.scrollTop"));
    assert!(controller.contains("document.querySelector(\".content-shell\")?.addEventListener("));
    assert!(controller.contains("scrollContainer?.addEventListener(\"scroll\", schedule"));
    assert!(controller.contains("activeLink.classList.add(\"active\", \"active-path\")"));
    assert!(controller.contains("activeItem?.classList.add(\"expanded\")"));
    assert!(controller.contains("maximumScroll > 1 && metrics.top >= maximumScroll - 2"));
    assert!(controller.contains("const sectionActivationLine = metrics.viewportTop + 20"));
    assert!(controller.contains("const subsectionActivationLine = metrics.viewportTop +"));
    assert!(controller.contains("activationLine: subsectionActivationLine"));
    assert!(controller.contains("className = \"mech-toc-toggle\""));
    assert!(controller.contains("layout.classList.toggle(\"is-toc-open\", open)"));
    assert!(controller.contains("new MutationObserver(updateConsoleErrorBadge)"));
    assert!(controller.contains("mech-console-error-count"));

    let page = include("style.css");
    assert!(page.contains(".toc li.expanded > .toc-sub"));
    assert!(page.contains("border-left: 1px dotted var(--toc-accent-soft)"));
    assert!(page.contains(".article-layout.is-toc-open > .main-content"));
    assert!(page.contains("scrollbar-color: rgb(128 128 128 / 70%) transparent"));

    let repl = include("mech-repl.css");
    assert!(repl.contains(".mech-console-error-count"));
    assert!(repl.contains("border-radius: 999px"));
}

#[test]
fn shipped_documents_load_and_activate_math_and_diagram_renderers() {
    for shim in ["index.html", "blog.html", "docs.html"] {
        let html = include(shim);
        assert!(html.contains("katex@0.16.22/dist/katex.min.css"), "{shim}");
        assert!(html.contains("katex@0.16.22/dist/katex.min.js"), "{shim}");
        assert!(html.contains("mermaid/dist/mermaid.min.js"), "{shim}");
    }

    let controller = include("document.js");
    assert!(controller.contains("[data-mech-equation]:not([data-mech-rendered])"));
    assert!(controller.contains("element.getAttribute(\"equation\") ?? element.textContent"));
    assert!(controller.contains(".mermaid:not([data-processed])"));
    assert!(controller.contains("window.mermaid.run({ nodes })"));
}

#[test]
fn mechdown_and_repl_layers_are_standalone_components() {
    let mechdown = include("mechdown.css");
    assert!(mechdown.contains("[data-mechdown] h1"));
    assert!(mechdown.contains("[data-mechdown] .mechdown-table"));
    assert!(mechdown.contains("[data-mechdown] .mech-backmatter-heading"));
    assert!(mechdown.contains("[data-mechdown] .mech-backmatter-heading::before"));
    assert!(!mechdown.contains("\nbody {"));
    assert!(!mechdown.contains(".console-pane"));

    let repl = include("mech-repl.css");
    for selector in [
        "[data-mech-repl-host]",
        "[data-mech-repl]",
        "[data-mech-console-pane]",
        ".mech-repl-transcript",
        "[data-mech-repl-popup]",
    ] {
        assert!(repl.contains(selector), "REPL layer lost {selector}");
    }
    for leaked_selector in [
        "\n.console-pane {",
        "\n.console-tab {",
        "\n.resize-handle,",
        "\n.repl-input {",
        "\n.mech-inline-popup {",
    ] {
        assert!(
            !repl.contains(leaked_selector),
            "REPL layer leaked unowned selector {leaked_selector}",
        );
    }
    assert!(repl.contains("box-sizing: border-box;"));
    assert!(repl.contains("border: 0;"));
    assert!(repl.contains("text-overflow: ellipsis;"));
    assert!(!repl.contains(".site-header"));
    assert!(!repl.contains("[data-mechdown]"));
}
