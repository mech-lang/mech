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
fn shipped_shims_expose_the_five_independent_style_layers_in_order() {
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
        for layer in ["palette", "source", "mechdown", "page", "repl"] {
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
    assert!(!docs.contains("data-mech-compact-toc=\"hidden\""));
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
        "--mechdown-inline-code: var(--mech-syntax-variable, var(--var-name-color, #eddaf1))",
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
    assert!(
        controller
            .contains("addRuntimeEventListener(\n    document.querySelector(\".content-shell\"),")
    );
    assert!(controller.contains("addRuntimeEventListener(scrollContainer, \"scroll\", schedule"));
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

#[test]
fn canonical_palette_is_a_single_token_layer_without_gradients() {
    let palette = include("palette.css");
    for token in [
        "--mech-canvas: #0d1117",
        "--mech-text-reading: #fff6e5",
        "--mech-link: #f7ce6e",
        "--mech-syntax-function: #bbddc2",
        "--mech-syntax-kind: #f09fca",
        "--mech-syntax-atom: #d7c0a4",
        "--mech-syntax-machine: #d8a5e4",
        "--mech-syntax-context: #acc9d2",
        "--mech-syntax-invariant: var(--mech-warning)",
    ] {
        assert!(palette.contains(token), "palette lost {token}");
    }

    let page = include("style.css");
    for canonical_definition in ["--mech-canvas:", "--mech-link:", "--mech-syntax-function:"] {
        assert!(
            !page.contains(canonical_definition),
            "page shell duplicated palette definition {canonical_definition}"
        );
    }

    for stylesheet in [
        "palette.css",
        "palette-lookbook.css",
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
            "{stylesheet} introduced a gradient"
        );
    }
}

#[test]
fn established_blog_design_remains_in_the_shared_layers() {
    let page = include("style.css");
    for selector in [
        ".hero {",
        ".hero-panel {",
        ".hero h1 {",
        ".mech-meta {",
        ".post-pagination{",
        ".hero-avatar {",
    ] {
        assert!(page.contains(selector), "shared blog shell lost {selector}");
    }

    let mechdown = include("mechdown.css");
    for contract in [
        "counter-reset: mechdown-section",
        "counter-increment: mechdown-section",
        "content: \"section \" counter(mechdown-section, decimal)",
        "counter(mechdown-subsection, decimal)",
        "padding: 0 10px 30px",
    ] {
        assert!(
            mechdown.contains(contract),
            "section design lost {contract}"
        );
    }

    let blog = include("blog.css");
    for addition in [
        ".mech-hero-img figure img",
        "p:first-of-type::first-letter",
        ".mech-block-quote::before",
        ".mech-equation::after",
        ".backmatter-cited .backmatter-body",
    ] {
        assert!(blog.contains(addition), "blog addition lost {addition}");
    }
}

#[test]
fn docs_and_app_shims_keep_their_distinct_structure() {
    let docs = include("docs.css");
    for contract in [
        ".docs-layout {",
        "grid-template-columns: var(--toc-width) minmax(0, 1fr)",
        ".docs-layout:is(.hide-toc, .no-sections, .has-empty-toc)",
        ".mechdown-section > :is(h2, h3, h4, h5, h6)::before",
        "font-size: clamp(2rem, 3vw, 2.75rem)",
        "@media (max-width: 900px)",
        "@container (max-width: 900px)",
    ] {
        assert!(docs.contains(contract), "docs shim lost {contract}");
    }

    let app = include("project.html");
    assert!(app.contains("data-mech-project=\"/\""));
    for document_surface in [
        "data-mech-style-layer",
        "data-mechdown",
        "data-mech-repl-host",
        "katex",
        "mermaid",
    ] {
        assert!(
            !app.contains(document_surface),
            "raw app shim loaded {document_surface}"
        );
    }
}

#[test]
fn mechdown_keeps_the_existing_icons_and_extended_formatter_surfaces() {
    let css = include("mechdown.css");
    for block in ["info", "question", "success", "warning", "error", "idea"] {
        assert!(
            css.contains(&format!(".mech-{block}-block::before")),
            "Mechdown lost the {block} icon"
        );
        assert!(css.contains(&format!("--mechdown-{block}")));
    }
    for selector in [
        ".mech-figure-table-cell",
        ".mech-figure-panel",
        ".mech-figure-caption-ref",
        ".mech-citation-link-icon",
        ".mech-reference",
        ".mech-mika-section",
        "a.mech-hyperlink .mech-inline-code",
        ".mech-diagram :is(.edgePath path, .flowchart-link)",
    ] {
        assert!(css.contains(selector), "Mechdown lost {selector}");
    }
    assert!(css.contains("mask: url(\"data:image/svg+xml;base64,"));
    assert!(!css.contains("rotate(45deg)"), "callouts became diamonds");
}

#[test]
fn source_palette_follows_construct_roles_and_context_parts() {
    let css = include("mech-source.css");
    for contract in [
        "--mech-source-function: var(--mech-syntax-function, #bbddc2)",
        "--mech-source-match: var(--mech-syntax-match, #d7c0a4)",
        "--mech-source-atom: var(--mech-syntax-atom, #d7c0a4)",
        "--mech-source-machine: var(--mech-syntax-machine, #d8a5e4)",
        "--mech-source-context: var(--mech-syntax-context, #acc9d2)",
        ".mech-match-guard-separator",
        ".mech-pattern-array-open",
        ".mech-state-variable-separator",
        ".mech-fsm-start-op",
        ".mech-atom-sigil",
        ".mech-context-provider",
        ".mech-context-path",
        ".mech-match-guard",
        ".mech-pattern-array-op",
        ".mech-pattern-separator",
        ".mech-matrix-size-separator",
        ".mech-tuple-destructure .mech-tuple-vars",
        "Guard rails are one sand-yellow signal",
    ] {
        assert!(css.contains(contract), "source palette lost {contract}");
    }

    let formatter = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/syntax/src/formatter.rs"),
    )
    .unwrap();
    for markup in [
        "mech-atom-sigil",
        "mech-enum-variant-sigil",
        "mech-match-guard-separator",
        "mech-pattern-array-open",
        "mech-state-variable-separator",
        "mech-fsm-start-op",
        "mech-context-provider",
        "mech-context-capability",
        "mech-match-guard",
        "mech-pattern-array-op",
        "mech-pattern-separator",
        "mech-matrix-size-separator",
    ] {
        assert!(formatter.contains(markup), "formatter lost {markup}");
    }
}

#[test]
fn blog_and_docs_do_not_collapse_into_one_presentation() {
    let blog = include("blog.css");
    for editorial in [
        "counter-reset: mechdown-section equation",
        ".mech-block-quote::before",
        "content: \"Eq. \" counter(equation)",
        "column-count: 2",
    ] {
        assert!(blog.contains(editorial), "blog lost {editorial}");
    }

    let docs = include("docs.css");
    for quiet_contract in [
        "data-mech-shim=\"docs\"",
        ".mechdown-section > :is(h2, h3, h4, h5, h6)::before",
        "font-size: clamp(2rem, 3vw, 2.75rem)",
        "color: var(--text-secondary)",
    ] {
        assert!(docs.contains(quiet_contract), "docs lost {quiet_contract}");
    }
    for blog_only in [
        "content: \"Eq. \"",
        "first-of-type::first-letter",
        ".mech-block-quote::before",
        "column-count: 2",
    ] {
        assert!(!docs.contains(blog_only), "docs inherited {blog_only}");
    }

    let repl = include("mech-repl.css");
    assert!(repl.contains(
        "grid-template-columns: minmax(0, 1fr) 8px var(--mech-console-size, var(--mech-repl-size))"
    ));
}

#[test]
fn palette_lookbook_is_static_and_uses_the_canonical_tokens() {
    let html = include("palette.html");
    assert!(html.contains("href=\"./palette.css\""));
    assert!(html.contains("href=\"./palette-lookbook.css\""));
    assert!(html.contains("@clock"));
    assert!(html.contains("match-tuple.mec"));
    assert!(html.contains("bubble-sort.mec"));
}
