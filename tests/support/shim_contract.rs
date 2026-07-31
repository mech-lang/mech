//! Shared end-to-end assertions for the supported HTML shim contract.
//!
//! Keep this expectation list in one place: `mech format` and `mech serve`
//! intentionally render the same original-shim slot contract.

pub fn assert_complete_slot_contract(html: &str, source_key: &str) {
    for wrapper in [
        "slot-title",
        "slot-author",
        "slot-date",
        "slot-kicker",
        "slot-section",
        "slot-version",
        "slot-next",
        "slot-previous",
        "slot-hero",
        "slot-summary",
        "slot-toc",
        "slot-abstract",
        "slot-intro",
        "slot-content",
        "slot-contents",
        "slot-cited",
        "slot-footnotes",
        "slot-section-1",
        "slot-section-2",
        "slot-repl",
        "slot-code",
        "slot-source-key",
        "slot-var-root",
    ] {
        assert!(html.contains(wrapper), "missing slot wrapper `{wrapper}`");
    }

    for expected in [
        "Shim Contract Fixture",
        "Ada Lovelace",
        "July 30, 2026",
        "Announcement",
        "Compatibility",
        "Fixture hero",
        "First fixture section",
        "Second fixture section",
        "mech-works-cited",
        "mech-footnotes",
        "mech-inline-mech-code",
        "mech-output",
        "data-mech-document-controller",
        "WasmDocument",
        "{{VAR:answer}}",
    ] {
        assert!(
            html.contains(expected),
            "missing rendered value `{expected}`"
        );
    }

    assert!(html.contains("<img"), "hero slot did not render an image");
    assert!(
        html.contains("href=\"#"),
        "TOC did not render document anchors"
    );
    assert!(
        html.contains("{{TITLE}}"),
        "literal placeholder-looking document content was rewritten"
    );
    assert!(
        !html.contains("Shim contract: {{TITLE}}"),
        "the title slot in the original shim was left unresolved"
    );
    assert!(
        !html.contains("id=\"slot-title\">{{TITLE}}</div>"),
        "the title wrapper in the original shim was left unresolved"
    );
    assert!(
        html.contains(&format!("data-value=\"{source_key}\"")),
        "source URL key did not match `{source_key}`"
    );

    for slot in [
        "STYLESHEET",
        "AUTHOR",
        "DATE",
        "KICKER",
        "SECTION",
        "VERSION",
        "NEXT",
        "PREVIOUS",
        "HERO",
        "SUMMARY",
        "TOC",
        "ABSTRACT",
        "INTRO",
        "CONTENT",
        "CONTENTS",
        "CITED",
        "FOOTNOTES",
        "SECTION1",
        "SECTION2",
        "REPL",
        "CODE",
        "DOCUMENT_SCRIPT",
        "DOCUMENT_SOURCES",
        "WASM_MODULE_URL",
        "SOURCE_URL_KEY",
    ] {
        assert!(
            !html.contains(&format!("{{{{{slot}}}}}")),
            "supported static slot `{slot}` was left unresolved"
        );
    }
}

pub fn assert_rich_shell(html: &str, selectors: &[&str]) {
    for selector in selectors {
        assert!(
            html.contains(selector),
            "rich document shell is missing `{selector}`"
        );
    }
    assert!(html.contains("data-mech-document-controller"));
    assert!(html.contains("WasmDocument"));
    assert!(html.contains("mech-output"));
    assert!(!html.contains("under construction"));
}
