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
        for component in [
            ".mech-code-block",
            ".mech-number",
            ".mech-repl",
            ".console-pane",
            ".repl-input",
            ".resize-handle",
            ".edge-handle",
            ".document-console-toggle",
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
fn mechdown_and_repl_layers_are_standalone_components() {
    let mechdown = include("mechdown.css");
    assert!(mechdown.contains("[data-mechdown] h1"));
    assert!(mechdown.contains("[data-mechdown] .mechdown-table"));
    assert!(!mechdown.contains("\nbody {"));
    assert!(!mechdown.contains(".console-pane"));

    let repl = include("mech-repl.css");
    for selector in [
        "[data-mech-repl-host]",
        "[data-mech-repl]",
        ".console-pane",
        ".mech-repl-transcript",
        ".mech-inline-popup",
    ] {
        assert!(repl.contains(selector), "REPL layer lost {selector}");
    }
    assert!(repl.contains("border: 0;"));
    assert!(repl.contains("text-overflow: ellipsis;"));
    assert!(!repl.contains(".site-header"));
    assert!(!repl.contains("[data-mechdown]"));
}
