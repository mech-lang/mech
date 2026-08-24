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
    for (shim, identity) in [
        ("index.html", "document"),
        ("blog.html", "blog"),
        ("docs.html", "docs"),
    ] {
        let html = include(shim);
        assert!(
            html.contains(&format!("data-mech-shim=\"{identity}\"")),
            "{shim} lost its shim identity"
        );
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
fn shipped_shims_keep_distinct_presentation_contracts() {
    let blog = include("blog.html");
    let hero = blog.find("class=\"hero\"").unwrap();
    let intro = blog.find("class=\"article-intro\"").unwrap();
    let layout = blog.find("data-mech-toc-mode=\"after-intro\"").unwrap();
    let toc = blog.find("{{TOC}}").unwrap();
    assert!(hero < intro && intro < layout && layout < toc);
    for contract in [
        "{{KICKER}}",
        "{{AUTHOR}}",
        "{{DATE}}",
        "{{SUMMARY}}",
        "{{HERO}}",
    ] {
        assert!(blog.contains(contract), "blog shim lost {contract}");
    }

    let docs = include("docs.html");
    let layout = docs.find("data-mech-toc-mode=\"persistent\"").unwrap();
    let toc = docs.find("{{TOC}}").unwrap();
    let header = docs.find("class=\"docs-header\"").unwrap();
    assert!(layout < toc && toc < header);
    assert!(docs.contains("data-mech-compact-toc=\"hidden\""));
    assert!(docs.contains("data-mech-console-mode=\"docked\""));
    for contract in [
        "{{SECTION}}",
        "{{TITLE}}",
        "{{VERSION}}",
        "{{ABSTRACT}}",
        "{{INTRO}}",
    ] {
        assert!(docs.contains(contract), "docs shim lost {contract}");
    }

    let app = include("project.html");
    assert!(app.contains("data-mech-shim=\"app\""));
    assert!(app.contains("data-mech-app-host"));
    assert!(app.contains("data-mech-project=\"/\""));
    for document_surface in [
        "data-mech-style-layer",
        "data-mechdown",
        "data-mech-source",
        "data-mech-repl-host",
        "katex",
        "mermaid",
    ] {
        assert!(
            !app.contains(document_surface),
            "raw app shim loaded document surface {document_surface}"
        );
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
fn page_shell_and_variants_do_not_own_source_or_repl_components() {
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
            "--mech-repl-",
            "--console-width",
        ] {
            assert!(
                !css.contains(component),
                "{stylesheet} still owns component selector {component}",
            );
        }
    }

    let shared_page = include("style.css");
    for portable_component in [".mech-backmatter-heading", "[data-mechdown]"] {
        assert!(
            !shared_page.contains(portable_component),
            "shared page shell still owns {portable_component}"
        );
    }
}

#[test]
fn mechdown_layer_owns_portable_content_without_blog_numbering() {
    let css = include("mechdown.css");
    for contract in [
        ".mechdown-section {",
        "padding: 0 0 30px",
        ".mech-abstract {",
        "border: 1px solid var(--mechdown-accent)",
        "--mechdown-inline-code: var(--mech-syntax-variable, var(--var-name-color, #eddaf1))",
        "color: var(--mechdown-inline-code)",
    ] {
        assert!(
            css.contains(contract),
            "Mechdown lost editorial style {contract}"
        );
    }
    for blog_only in [
        "counter(mechdown-section",
        "counter(mechdown-subsection",
        "content: \"section \"",
        "content: \"Eq. \"",
        ".mech-block-quote::before",
    ] {
        assert!(
            !css.contains(blog_only),
            "portable Mechdown retained blog-only decoration {blog_only}"
        );
    }
}

#[test]
fn canonical_palette_is_shared_by_every_style_layer_without_gradients() {
    let page = include("style.css");
    for token in [
        "--mech-canvas: #0d1117",
        "--mech-brand: #f4bd3e",
        "--mech-selection-bg: rgb(244 189 62 / 24%)",
        "--mech-syntax-function: #bbddc2",
        "--mech-syntax-match: #d7c0a4",
        "--mech-syntax-machine: #d8a5e4",
        "--mech-syntax-context: #acc9d2",
    ] {
        assert!(page.contains(token), "page palette lost {token}");
    }

    for stylesheet in [
        "style.css",
        "blog.css",
        "docs.css",
        "mech-source.css",
        "mechdown.css",
        "mech-repl.css",
    ] {
        let css = include(stylesheet);
        assert!(
            !css.to_ascii_lowercase().contains("gradient("),
            "{stylesheet} introduced a gradient",
        );
    }

    assert!(include("mech-source.css").contains("var(--mech-syntax-function, #bbddc2)"));
    assert!(include("mechdown.css").contains("var(--mech-brand, var(--accent-primary"));
    assert!(include("mech-repl.css").contains("var(--mech-syntax-kind"));
}

#[test]
fn mechdown_preserves_icons_and_portable_formatter_details() {
    let css = include("mechdown.css");
    for block in ["info", "question", "success", "warning", "error", "idea"] {
        assert!(
            css.contains(&format!(".mech-{block}-block::before")),
            "Mechdown lost the {block} icon",
        );
        assert!(
            css.contains(&format!("--mechdown-{block}")),
            "Mechdown lost the {block} role",
        );
    }
    for restored in [
        ".mech-figure-table-cell",
        ".mech-figure-panel",
        ".mech-figure-grid-image",
        ".mech-figure-caption-ref",
        ".mech-citation-link-icon",
        ".mech-reference",
        ".mech-mika-section",
        "a.mech-hyperlink .mech-inline-code",
        ".mech-diagram :is(.edgePath path, .flowchart-link)",
    ] {
        assert!(css.contains(restored), "Mechdown lost {restored}");
    }
    assert!(css.contains("mask: url(\"data:image/svg+xml;base64,"));
    assert!(
        !css.contains("rotate(45deg)"),
        "callouts must not become diamonds"
    );

    let page = include("blog.css");
    for restored in [
        ".mech-hero-img figure img",
        ".article-intro > .mech-intro > p:first-of-type::first-letter",
        ".backmatter-cited .backmatter-body",
        ".mech-block-quote::before",
        ".mech-equation::after",
    ] {
        assert!(page.contains(restored), "blog variant lost {restored}");
    }
}

#[test]
fn blog_and_docs_variants_do_not_collapse_into_one_visual_system() {
    let blog = include("blog.css");
    for editorial in [
        "html[data-mech-shim=\"blog\"] .hero",
        "counter-reset: mechdown-section equation",
        "content: \"section \" counter(mechdown-section, decimal)",
        ".mech-block-quote::before",
        "content: \"Eq. \" counter(equation)",
        "column-count: 2",
    ] {
        assert!(
            blog.contains(editorial),
            "blog lost editorial contract {editorial}"
        );
    }

    let docs = include("docs.css");
    for workspace in [
        "html[data-mech-shim=\"docs\"] .docs-layout",
        "grid-template-columns: var(--toc-width) minmax(0, 1fr)",
        ".docs-layout > .toc {\n  display: block;",
        ".docs-layout > .mech-toc-toggle",
        "data-mech-shim=\"docs\"",
        "@media (max-width: 900px)",
        "@container (max-width: 720px)",
        "display: none !important",
    ] {
        assert!(
            docs.contains(workspace),
            "docs lost workspace contract {workspace}"
        );
    }
    for editorial in [
        "counter(mechdown-section",
        "content: \"section \"",
        "content: \"Eq. \"",
        "first-of-type::first-letter",
        ".mech-block-quote::before",
        "column-count: 2",
    ] {
        assert!(
            !docs.contains(editorial),
            "docs inherited blog decoration {editorial}"
        );
    }

    let repl = include("mech-repl.css");
    assert!(repl.contains(
        "grid-template-columns: minmax(0, 1fr) 8px var(--mech-console-size, var(--mech-repl-size))"
    ));
}

#[test]
fn source_palette_follows_construct_roles_instead_of_literal_types() {
    let css = include("mech-source.css");
    for contract in [
        "--mech-source-function: var(--mech-syntax-function, #bbddc2)",
        "--mech-source-match: var(--mech-syntax-match, #d7c0a4)",
        "--mech-source-atom: var(--mech-syntax-atom, #d7c0a4)",
        "--mech-source-machine: var(--mech-syntax-machine, #d8a5e4)",
        "--mech-source-context: var(--mech-syntax-context, #acc9d2)",
        ".mech-match-guard-separator",
        ".mech-match-guard",
        ".mech-pattern-array-open",
        ".mech-pattern-array-op",
        ".mech-pattern-separator",
        ".mech-state-variable-separator",
        ".mech-fsm-start-op",
        ".mech-matrix-size-separator",
        ".mech-tuple-destructure .mech-tuple-vars",
        ".mech-atom-sigil",
        ".mech-context-provider",
        ".mech-context-path",
        "Guard rails are one sand-yellow signal",
    ] {
        assert!(css.contains(contract), "source palette lost {contract}");
    }

    let formatter = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("syntax")
            .join("src")
            .join("formatter.rs"),
    )
    .unwrap();
    for markup in [
        "mech-atom-sigil",
        "mech-enum-variant-sigil",
        "mech-match-guard-separator",
        "mech-match-guard",
        "mech-pattern-array-open",
        "mech-pattern-array-op",
        "mech-pattern-separator",
        "mech-state-variable-separator",
        "mech-fsm-start-op",
        "mech-matrix-size-separator",
        "mech-context-provider",
        "mech-context-capability",
    ] {
        assert!(formatter.contains(markup), "formatter lost {markup}");
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
    assert!(controller.contains("function documentPageScrollOwner()"));
    assert!(controller.contains("contentShell.scrollHeight > contentShell.clientHeight + 1"));
    assert!(controller.contains("function documentPageContentOrigin(contentShell)"));
    assert!(controller.contains("function documentPagePositionForOwner(position, owner)"));
    assert!(controller.contains("x += element.offsetLeft"));
    assert!(controller.contains("y += element.offsetTop"));
    assert!(controller.contains("position.coordinateSpace === \"content-shell\""));
    assert!(controller.contains("position.coordinateSpace === \"window\""));
    assert!(controller.contains("coordinateSpace: \"content-shell\""));
    assert!(controller.contains("coordinateSpace: \"window\""));
    assert!(controller.contains("ownerX - origin.x"));
    assert!(controller.contains("ownerY - origin.y"));
    assert!(controller.contains("y: Math.max(0, originY + y)"));
    assert!(controller.contains("y: Math.max(0, y - originY)"));
    assert!(
        controller.contains("saved?.owner === \"content-shell\" ? \"content-shell\" : \"window\"")
    );
    assert!(controller.contains("owner: owner === window ? \"window\" : \"content-shell\""));
    assert!(controller.contains("coordinateSpace,"));
    assert!(controller.contains("const target = documentPagePositionForOwner(restore, owner)"));
    assert!(controller.contains("if (!target) {"));
    assert!(controller.contains("restore.mapping !== mapping"));
    assert!(controller.contains("restore.stableSince ??= now"));
    assert!(controller.contains("now - restore.stableSince >= 600"));
    assert!(controller.contains("new MutationObserver(attempt)"));
    assert!(controller.contains("dataset.mechPagePositionRestore = \"waiting-anchor\""));
    assert!(controller.contains("dataset.mechPagePositionRestore = \"waiting-owner\""));
    assert!(controller.contains("dataset.mechPagePositionRestore = \"settling\""));
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
    assert!(controller.contains("layout.dataset.mechCompactToc === \"hidden\""));
    assert!(controller.contains("toggle?.remove()"));
    assert!(controller.contains("layout.classList.toggle(\"is-toc-open\", open)"));
    assert!(controller.contains("new MutationObserver(updateConsoleErrorBadge)"));
    assert!(controller.contains("mech-console-error-count"));

    let page = include("style.css");
    assert!(page.contains(".toc li.expanded > .toc-sub"));
    assert!(page.contains("border-left: 1px dotted var(--toc-accent-soft)"));
    assert!(page.contains(".article-layout.is-toc-open > .main-content"));
    assert!(page.contains("scrollbar-color: var(--mech-scrollbar) transparent"));

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
