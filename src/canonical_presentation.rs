use std::collections::{BTreeMap, BTreeSet};

use mech_core::{GenericError, MResult, MechError};
use mech_runtime::CanonicalDocumentRenderer;
use mech_syntax::document::{AstNode, DocumentSyntax};

#[derive(Clone, Debug, Default)]
pub(crate) struct HtmlShimExtraSlots {
    slots: BTreeMap<String, String>,
}

impl HtmlShimExtraSlots {
    #[cfg(any(test, feature = "serve", feature = "formatter"))]
    pub(crate) fn insert(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.slots.insert(name.into(), value.into());
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct HtmlStyleSheets {
    pub palette: String,
    pub source: String,
    pub mechdown: String,
    pub page: String,
    pub repl: String,
}

impl HtmlStyleSheets {
    pub(crate) fn legacy(stylesheet: String) -> Self {
        Self {
            page: stylesheet,
            ..Self::default()
        }
    }

    pub(crate) fn bundle(&self) -> String {
        [
            &self.palette,
            &self.source,
            &self.mechdown,
            &self.page,
            &self.repl,
        ]
        .into_iter()
        .filter(|stylesheet| !stylesheet.is_empty())
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("\n")
    }
}

impl From<String> for HtmlStyleSheets {
    fn from(stylesheet: String) -> Self {
        Self::legacy(stylesheet)
    }
}

impl From<&str> for HtmlStyleSheets {
    fn from(stylesheet: &str) -> Self {
        Self::legacy(stylesheet.to_owned())
    }
}

impl From<&String> for HtmlStyleSheets {
    fn from(stylesheet: &String) -> Self {
        Self::legacy(stylesheet.clone())
    }
}

impl From<&HtmlStyleSheets> for HtmlStyleSheets {
    fn from(stylesheets: &HtmlStyleSheets) -> Self {
        stylesheets.clone()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct HtmlShimRender {
    pub html: String,
    #[cfg(any(test, feature = "serve", feature = "formatter"))]
    pub consumed_slots: BTreeSet<String>,
    #[cfg(any(test, feature = "serve", feature = "formatter"))]
    pub unresolved_mech_slots: BTreeSet<String>,
}

pub(crate) fn render_canonical_html(
    document: &DocumentSyntax,
    styles: HtmlStyleSheets,
    shim: String,
    extra_slots: &HtmlShimExtraSlots,
) -> MResult<HtmlShimRender> {
    render_canonical_html_mode(document, styles, shim, extra_slots, true)
}

pub(crate) fn render_canonical_static_html(
    document: &DocumentSyntax,
    styles: HtmlStyleSheets,
    shim: String,
    extra_slots: &HtmlShimExtraSlots,
) -> MResult<HtmlShimRender> {
    render_canonical_html_mode(document, styles, shim, extra_slots, false)
}

fn render_canonical_html_mode(
    document: &DocumentSyntax,
    styles: HtmlStyleSheets,
    mut shim: String,
    extra_slots: &HtmlShimExtraSlots,
    live: bool,
) -> MResult<HtmlShimRender> {
    if !live {
        shim = normalize_static_document_attributes(&shim);
    }
    let mut presentation = if live {
        CanonicalDocumentRenderer.format_browser_html_slots(document)
    } else {
        CanonicalDocumentRenderer.format_static_html_slots(document)
    }
    .map_err(|error| presentation_error(error.to_string()))?;
    complete_presentation_slots(&mut presentation, &shim);
    let mut source_slots = CanonicalDocumentRenderer
        .format_passive_html_slots(document)
        .map_err(|error| presentation_error(error.to_string()))?;
    complete_presentation_slots(&mut source_slots, &shim);
    let title = document
        .title()
        .and_then(|title| title.syntax().text().ok())
        .unwrap_or_default();
    let title = title.lines().next().unwrap_or_default();
    let repl = r#"<div
  class="console-scroll mech-repl hidden"
  id="mech-output"
  data-mech-repl-mount
  data-mech-repl
  aria-live="polite">
</div>"#;
    let mut slots = BTreeMap::from([
        ("STYLESHEET".to_owned(), styles.bundle()),
        ("PALETTE_STYLESHEET".to_owned(), styles.palette),
        ("MECH_SOURCE_STYLESHEET".to_owned(), styles.source),
        ("MECHDOWN_STYLESHEET".to_owned(), styles.mechdown),
        ("PAGE_STYLESHEET".to_owned(), styles.page),
        ("MECH_REPL_STYLESHEET".to_owned(), styles.repl),
        ("TITLE".to_owned(), escape_html_text(title)),
        ("AUTHOR".to_owned(), String::new()),
        ("DATE".to_owned(), String::new()),
        ("KICKER".to_owned(), String::new()),
        ("SECTION".to_owned(), String::new()),
        ("VERSION".to_owned(), env!("CARGO_PKG_VERSION").to_owned()),
        ("NEXT".to_owned(), String::new()),
        ("PREVIOUS".to_owned(), String::new()),
        ("HERO".to_owned(), String::new()),
        ("SUMMARY".to_owned(), String::new()),
        ("TOC".to_owned(), String::new()),
        ("ABSTRACT".to_owned(), String::new()),
        ("INTRO".to_owned(), String::new()),
        ("CONTENTS".to_owned(), String::new()),
        ("CONTENT".to_owned(), String::new()),
        ("CITED".to_owned(), String::new()),
        ("FOOTNOTES".to_owned(), String::new()),
        ("CODE".to_owned(), String::new()),
        ("REPL".to_owned(), repl.to_owned()),
        ("PRESENTATION".to_owned(), "document".to_owned()),
    ]);
    for (name, value) in presentation {
        if matches!(
            name.as_str(),
            "STYLESHEET"
                | "PALETTE_STYLESHEET"
                | "MECH_SOURCE_STYLESHEET"
                | "MECHDOWN_STYLESHEET"
                | "PAGE_STYLESHEET"
                | "MECH_REPL_STYLESHEET"
                | "TITLE"
                | "VERSION"
                | "CODE"
                | "REPL"
                | "PRESENTATION"
        ) {
            continue;
        }
        slots.insert(name, value);
    }
    slots.extend(extra_slots.slots.clone());
    Ok(render_html_shim(&shim, &slots, Some(&source_slots)))
}

fn normalize_static_document_attributes(shim: &str) -> String {
    let bytes = shim.as_bytes();
    let lower = shim.to_ascii_lowercase();
    let mut cursor = 0;
    let mut edits = Vec::new();
    while let Some(relative) = shim[cursor..].find('<') {
        let start = cursor + relative;
        if shim[start..].starts_with("<!--") {
            let Some(end) = shim[start + 4..].find("-->") else {
                break;
            };
            cursor = start + 4 + end + 3;
            continue;
        }
        cursor = start + 1;
        if !bytes.get(cursor).is_some_and(u8::is_ascii_alphabetic) {
            continue;
        }
        let name_start = cursor;
        while bytes
            .get(cursor)
            .is_some_and(|byte| !byte.is_ascii_whitespace() && !matches!(byte, b'/' | b'>'))
        {
            cursor += 1;
        }
        let tag = &lower[name_start..cursor];
        while cursor < bytes.len() {
            while bytes
                .get(cursor)
                .is_some_and(|byte| byte.is_ascii_whitespace() || *byte == b'/')
            {
                cursor += 1;
            }
            if cursor == bytes.len() {
                break;
            }
            if bytes.get(cursor) == Some(&b'>') {
                cursor += 1;
                break;
            }
            let attribute_start = cursor;
            while bytes.get(cursor).is_some_and(|byte| {
                !byte.is_ascii_whitespace() && !matches!(byte, b'=' | b'/' | b'>')
            }) {
                cursor += 1;
            }
            let attribute_end = cursor;
            if attribute_start == attribute_end {
                cursor += 1;
                continue;
            }
            while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
                cursor += 1;
            }
            let mut value_end = attribute_end;
            if bytes.get(cursor) == Some(&b'=') {
                cursor += 1;
                while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
                    cursor += 1;
                }
                if let Some(quote @ (b'\'' | b'"')) = bytes.get(cursor).copied() {
                    cursor += 1;
                    while bytes.get(cursor).is_some_and(|byte| *byte != quote) {
                        cursor += 1;
                    }
                    if bytes.get(cursor) == Some(&quote) {
                        cursor += 1;
                    }
                } else {
                    while bytes
                        .get(cursor)
                        .is_some_and(|byte| !byte.is_ascii_whitespace() && *byte != b'>')
                    {
                        cursor += 1;
                    }
                }
                value_end = cursor;
            }
            if shim[attribute_start..attribute_end]
                .eq_ignore_ascii_case("data-mech-document-status")
            {
                edits.push((
                    attribute_start,
                    value_end,
                    "data-mech-document-status=\"ready\"",
                ));
            } else if shim[attribute_start..attribute_end]
                .eq_ignore_ascii_case("data-mech-document-controller")
            {
                edits.push((attribute_start, value_end, ""));
            }
        }
        // Attribute-looking examples inside raw text and RCDATA are content.
        if tag == "plaintext" {
            break;
        }
        if matches!(
            tag,
            "script" | "style" | "textarea" | "title" | "xmp" | "iframe" | "noembed" | "noframes"
        ) {
            let closing = format!("</{tag}");
            loop {
                let Some(relative) = lower[cursor..].find(&closing) else {
                    cursor = bytes.len();
                    break;
                };
                cursor += relative;
                let after_name = cursor + closing.len();
                if bytes
                    .get(after_name)
                    .is_none_or(|byte| byte.is_ascii_whitespace() || matches!(byte, b'/' | b'>'))
                {
                    break;
                }
                cursor = after_name;
            }
        }
    }
    let mut output = shim.to_owned();
    for (start, end, value) in edits.into_iter().rev() {
        output.replace_range(start..end, value);
    }
    output
}

fn complete_presentation_slots(presentation: &mut BTreeMap<String, String>, shim: &str) {
    // A custom shell may only expose CONTENT. Preserve unplaced regions there,
    // while shipped shells own their dedicated metadata and intro regions.
    let mut metadata = String::new();
    for name in [
        "AUTHOR", "DATE", "KICKER", "SECTION", "SUMMARY", "HERO", "NEXT", "PREVIOUS",
    ] {
        if !shim.contains(&format!("{{{{{name}}}}}")) {
            if let Some(value) = presentation.get(name).filter(|value| !value.is_empty()) {
                metadata.push_str(&format!(
                    "<div class='mech-title-field'><dt>{}</dt><dd>{value}</dd></div>",
                    name.to_lowercase()
                ));
            }
        }
    }
    if !metadata.is_empty() {
        let intro = presentation.entry("INTRO".to_owned()).or_default();
        *intro = format!("<dl class='mech-title-front-matter'>{metadata}</dl>{intro}");
    }
    for (source, target) in [
        ("ABSTRACT", "INTRO"),
        ("INTRO", "CONTENT"),
        ("FOOTNOTES", "CONTENT"),
        ("CITED", "CONTENT"),
    ] {
        if !shim.contains(&format!("{{{{{source}}}}}")) {
            let value = presentation.get(source).cloned().unwrap_or_default();
            let destination = presentation.entry(target.to_owned()).or_default();
            if source == "ABSTRACT" || source == "INTRO" {
                *destination = format!("{value}{destination}");
            } else {
                destination.push_str(&value);
            }
        }
    }
    presentation.insert("CONTENTS".to_owned(), presentation["CONTENT"].clone());
}

fn render_html_shim(
    shim: &str,
    slots: &BTreeMap<String, String>,
    source_slots: Option<&BTreeMap<String, String>>,
) -> HtmlShimRender {
    let primary_body = ["CONTENT", "CONTENTS"]
        .into_iter()
        .find(|name| shim.contains(&format!("{{{{{name}}}}}")));
    let mut html = String::with_capacity(shim.len());
    let mut consumed_slots = BTreeSet::new();
    let mut unresolved_mech_slots = BTreeSet::new();
    let mut cursor = 0;
    while let Some(relative_open) = shim[cursor..].find("{{") {
        let open = cursor + relative_open;
        html.push_str(&shim[cursor..open]);
        let name_start = open + 2;
        let Some(relative_close) = shim[name_start..].find("}}") else {
            html.push_str(&shim[open..]);
            cursor = shim.len();
            break;
        };
        let name_end = name_start + relative_close;
        let token_end = name_end + 2;
        let name = &shim[name_start..name_end];
        if name.starts_with("VAR:") {
            html.push_str(&shim[open..token_end]);
        } else if let Some(value) = slots.get(name) {
            let is_section = name.strip_prefix("SECTION").is_some_and(|suffix| {
                !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
            });
            let duplicate_body =
                matches!(name, "CONTENT" | "CONTENTS") && primary_body != Some(name);
            let duplicate_region = consumed_slots.contains(name)
                || duplicate_body
                || (is_section && primary_body.is_some());
            let value = if duplicate_region {
                source_slots
                    .and_then(|sources| sources.get(name))
                    .unwrap_or(value)
            } else {
                value
            };
            html.push_str(value);
            consumed_slots.insert(name.to_owned());
        } else {
            if is_mech_slot_name(name) {
                unresolved_mech_slots.insert(name.to_owned());
            }
            html.push_str(&shim[open..token_end]);
        }
        cursor = token_end;
    }
    html.push_str(&shim[cursor..]);
    HtmlShimRender {
        html,
        #[cfg(any(test, feature = "serve", feature = "formatter"))]
        consumed_slots,
        #[cfg(any(test, feature = "serve", feature = "formatter"))]
        unresolved_mech_slots,
    }
}

#[cfg(any(test, feature = "serve", feature = "formatter"))]
pub(crate) fn validate_shipped_shim_render(
    shim_name: &str,
    render: &HtmlShimRender,
) -> MResult<()> {
    if !render.unresolved_mech_slots.is_empty() {
        return Err(presentation_error(format!(
            "shipped HTML shim `{shim_name}` contains unresolved Mech slots: {}",
            render
                .unresolved_mech_slots
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    for slot in ["DOCUMENT_SCRIPT", "TITLE", "CODE", "REPL", "PRESENTATION"] {
        if !render.consumed_slots.contains(slot) {
            return Err(presentation_error(format!(
                "shipped HTML shim `{shim_name}` did not consume required slot `{slot}`"
            )));
        }
    }
    if !render.consumed_slots.contains("CONTENT") && !render.consumed_slots.contains("CONTENTS") {
        return Err(presentation_error(format!(
            "shipped HTML shim `{shim_name}` did not consume CONTENT or CONTENTS"
        )));
    }
    Ok(())
}

fn is_mech_slot_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    matches!(bytes.next(), Some(b'A'..=b'Z'))
        && bytes.all(|byte| matches!(byte, b'A'..=b'Z' | b'0'..=b'9' | b'_'))
}

fn escape_html_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn presentation_error(message: impl Into<String>) -> MechError {
    MechError::new(
        GenericError {
            msg: message.into(),
        },
        None,
    )
    .with_compiler_loc()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_shim_slots_preserve_single_pass_literals_dynamic_bindings_and_layers() {
        let mut slots = BTreeMap::from([
            ("TITLE".into(), "Title".into()),
            ("CONTENT".into(), "{{TITLE}} {{AUTHOR}} {{SECTION1}}".into()),
            ("CUSTOM".into(), "{{TITLE}}".into()),
            ("VAR:answer".into(), "must not replace".into()),
        ]);
        let render = render_html_shim(
            "{{TITLE}}|{{CONTENT}}|{{CUSTOM}}|{{VAR:answer}}|{{UNKNOWN}}",
            &slots,
            None,
        );
        assert_eq!(
            render.html,
            "Title|{{TITLE}} {{AUTHOR}} {{SECTION1}}|{{TITLE}}|{{VAR:answer}}|{{UNKNOWN}}"
        );
        assert_eq!(
            render.consumed_slots,
            BTreeSet::from(["TITLE".into(), "CONTENT".into(), "CUSTOM".into()])
        );
        assert_eq!(
            render.unresolved_mech_slots,
            BTreeSet::from(["UNKNOWN".into()])
        );
        let styles = HtmlStyleSheets {
            palette: "palette".into(),
            source: "source".into(),
            mechdown: "mechdown".into(),
            page: "page".into(),
            repl: "repl".into(),
        };
        assert_eq!(styles.bundle(), "palette\nsource\nmechdown\npage\nrepl");
        slots.insert("STYLESHEET".into(), styles.bundle());
        for (name, value) in [
            ("PALETTE_STYLESHEET", styles.palette),
            ("MECH_SOURCE_STYLESHEET", styles.source),
            ("MECHDOWN_STYLESHEET", styles.mechdown),
            ("PAGE_STYLESHEET", styles.page),
            ("MECH_REPL_STYLESHEET", styles.repl),
        ] {
            slots.insert(name.into(), value.clone());
            assert_eq!(
                render_html_shim(&format!("{{{{{name}}}}}"), &slots, None).html,
                value
            );
        }
        assert_eq!(
            render_html_shim("{{STYLESHEET}}|{{MECH_SOURCE_STYLESHEET}}", &slots, None).html,
            "palette\nsource\nmechdown\npage\nrepl|source"
        );
    }

    #[test]
    fn canonical_shim_overlapping_regions_own_each_live_output_once() {
        let source = mech_runtime::SourceDocument::parse_resolved(
            "bundle:///overlap.mec",
            mech_syntax::document::Revision(0),
            "1. First\n---------\nFirst {11} with [BOOK] and [^note].\n\n2. Second\n----------\nSecond {22}.\n\n[^note]: A footnote.\n\n[BOOK]: A reference.\n",
            mech_syntax::document::ParseConfig::default(),
        )
        .unwrap();
        assert!(source.is_strictly_clean());
        let document = source.document();
        let outputs = mech_runtime::canonical_document_presentation_outputs(&document).unwrap();
        assert_eq!(outputs.len(), 2);
        for shim in [
            "<aside>{{SECTION1}}</aside><main>{{CONTENT}}</main><aside>{{CONTENTS}}{{CONTENT}}{{SECTION1}}</aside>",
            "<main>{{CONTENTS}}</main><aside>{{CONTENTS}}</aside>",
            "<main>{{SECTION1}}{{SECTION2}}</main><aside>{{SECTION1}}</aside>",
            "<aside>{{SECTION1}}</aside><nav>{{TOC}}</nav><main>{{CONTENT}}{{FOOTNOTES}}{{CITED}}</main><aside>{{CONTENT}}{{FOOTNOTES}}{{CITED}}</aside>",
        ] {
            for live in [true, false] {
                let render = if live {
                    render_canonical_html
                } else {
                    render_canonical_static_html
                };
                let html = render(
                    &document,
                    "".into(),
                    shim.into(),
                    &HtmlShimExtraSlots::default(),
                )
                .unwrap()
                .html;
                assert_eq!(
                    html.matches("data-mech-output-address").count(),
                    if live { 2 } else { 0 },
                    "{html}"
                );
                for output in &outputs {
                    assert_eq!(
                        html.matches(&format!("id='{}:0'", output.output_id))
                            .count(),
                        if live { 1 } else { 0 },
                        "{html}"
                    );
                }
                let (main, copies) = html.split_once("</main>").unwrap();
                assert_eq!(
                    main.matches("data-mech-output-address").count(),
                    if live { 2 } else { 0 },
                    "{html}"
                );
                assert!(!copies.contains("data-mech-output-address"), "{html}");
                assert!(html.contains("First"));
                assert!(html.contains("Second"));
                let (before_main, main) = main.split_once("<main>").unwrap();
                assert!(!before_main.contains("id='"), "{html}");
                assert!(!copies.contains("id='"), "{html}");
                for anchor in ["1", "2"] {
                    assert_eq!(html.matches(&format!("id='{anchor}'")).count(), 1, "{html}");
                    assert!(main.contains(&format!("id='{anchor}'")), "{html}");
                }
                if !shim.contains("{{SECTION2}}") {
                    for anchor in ["footnote-note", "reference-BOOK"] {
                        assert_eq!(html.matches(&format!("id='{anchor}'")).count(), 1, "{html}");
                        assert!(main.contains(&format!("id='{anchor}'")), "{html}");
                    }
                }
            }
        }
    }

    #[test]
    fn canonical_shim_empty_title_toc_and_indexed_sections_have_explicit_contracts() {
        for (source, title, headings) in [
            ("Plain prose.\n", "", 0),
            (
                "1. First\n---------\nFirst body.\n\n2. Second\n----------\nSecond body.\n",
                "",
                2,
            ),
        ] {
            let source = mech_runtime::SourceDocument::parse_resolved(
                "bundle:///slots.mec",
                mech_syntax::document::Revision(0),
                source,
                mech_syntax::document::ParseConfig::default(),
            )
            .unwrap();
            assert!(source.is_strictly_clean());
            let result = render_canonical_static_html(
                &source.document(),
                "".into(),
                "{{TITLE}}|{{TOC}}|{{SECTION1}}|{{SECTION2}}".into(),
                &HtmlShimExtraSlots::default(),
            )
            .unwrap();
            assert!(
                result.html.starts_with(&format!("{title}|")),
                "{}",
                result.html
            );
            if headings == 0 {
                assert!(!result.html.contains("mech-toc"));
            } else {
                assert!(result.html.contains("First body."));
                assert!(result.html.contains("Second body."));
                assert!(!result.html.contains("{{SECTION1}}"));
            }
        }
    }

    #[test]
    fn stock_shims_own_one_title_and_one_article() {
        let document = mech_runtime::SourceDocument::parse_resolved(
            "bundle:///title.mec",
            mech_syntax::document::Revision(0),
            std::sync::Arc::<str>::from("A & B\n==============================================================================\nauthor: Ada\nsection: Examples\n==============================================================================\nx := 1\n"),
            mech_syntax::document::ParseConfig::default(),
        ).unwrap();
        assert!(document.is_strictly_clean());
        for shim in [
            include_str!("../include/index.html"),
            include_str!("../include/docs.html"),
            include_str!("../include/blog.html"),
        ] {
            let html = render_canonical_html(
                &document.document(),
                "".into(),
                shim.to_owned(),
                &HtmlShimExtraSlots::default(),
            )
            .unwrap()
            .html;
            assert_eq!(html.matches("<h1").count(), 1, "{html}");
            assert_eq!(html.matches("<article").count(), 1, "{html}");
            assert!(html.contains("A &amp; B"), "{html}");
            assert!(!html.contains("mech-document-header"), "{html}");
            assert!(html.contains("Ada"), "{html}");
            assert!(html.contains("Examples"), "{html}");
        }
    }

    #[test]
    fn front_matter_cannot_replace_host_owned_shim_slots() {
        let document = mech_runtime::SourceDocument::parse_resolved(
            "bundle:///reserved.mec",
            mech_syntax::document::Revision(0),
            "Actual Title\n===============\ntitle: Forged Title\nversion: Forged Version\nstylesheet: Forged Style\n===============\nanswer := 42\n",
            mech_syntax::document::ParseConfig::default(),
        )
        .unwrap();
        let shim = "<title>{{TITLE}}</title><style>{{STYLESHEET}}</style><p>{{VERSION}}</p>";
        let html = render_canonical_html(
            &document.document(),
            "Safe Style".into(),
            shim.to_owned(),
            &HtmlShimExtraSlots::default(),
        )
        .unwrap()
        .html;
        assert!(html.contains("<title>Actual Title</title>"), "{html}");
        assert!(html.contains("<style>Safe Style</style>"), "{html}");
        assert!(html.contains(env!("CARGO_PKG_VERSION")), "{html}");
        assert!(!html.contains("Forged"), "{html}");
    }

    #[test]
    fn static_shims_mark_every_document_status_target_ready() {
        let document = mech_runtime::SourceDocument::parse_resolved(
            "bundle:///static.mec",
            mech_syntax::document::Revision(0),
            "answer := 42\nanswer\n",
            mech_syntax::document::ParseConfig::default(),
        )
        .unwrap();
        for shim in [
            include_str!("../include/index.html"),
            include_str!("../include/docs.html"),
            include_str!("../include/blog.html"),
        ] {
            let html = render_canonical_static_html(
                &document.document(),
                "".into(),
                shim.to_owned(),
                &HtmlShimExtraSlots::default(),
            )
            .unwrap()
            .html;
            assert!(
                !html.contains("data-mech-document-status=\"loading\""),
                "{html}"
            );
            assert_eq!(
                html.matches("data-mech-document-status=\"ready\"").count(),
                2,
                "{html}"
            );
        }
    }

    #[test]
    fn static_shim_status_attributes_support_html_spellings_without_rewriting_literals() {
        let document = mech_runtime::SourceDocument::parse_resolved(
            "bundle:///static.mec",
            mech_syntax::document::Revision(0),
            "Plain prose.\n",
            mech_syntax::document::ParseConfig::default(),
        )
        .unwrap();
        let literals = "<!-- <div data-mech-document-status='loading'> -->\n<script>const sample = \"<div data-mech-document-status='loading'>\";</script>\n<style>/* <div data-mech-document-status='loading'> */</style>\n<textarea><div data-mech-document-status='loading'></textarea>\n<div title=\"data-mech-document-status='loading'\">Literal</div>";
        for attribute in [
            "data-mech-document-status='loading'",
            "data-mech-document-status=\"loading\"",
            "data-mech-document-status=loading",
            "DATA-MECH-DOCUMENT-STATUS = 'loading'",
            "data-mech-document-status\n=\t\"loading\"",
        ] {
            for controller in [
                "data-mech-document-controller",
                "data-mech-document-controller='document'",
                "data-mech-document-controller=\"document\"",
                "data-mech-document-controller = document",
                "DATA-MECH-DOCUMENT-CONTROLLER\n=\t'document'",
            ] {
                let shim = format!(
                    "{literals}<main {controller} data-mech-document-controller-extra='keep' {attribute}>{{{{CONTENT}}}}</main><aside {controller} {attribute}></aside>"
                );
                let html = render_canonical_static_html(
                    &document.document(),
                    "".into(),
                    shim.clone(),
                    &HtmlShimExtraSlots::default(),
                )
                .unwrap()
                .html;
                assert!(html.starts_with(literals), "{html}");
                assert_eq!(
                    html.matches("data-mech-document-status=\"ready\"").count(),
                    2,
                    "{html}"
                );
                let body = &html[literals.len()..];
                assert!(
                    body.contains("data-mech-document-controller-extra='keep'"),
                    "{html}"
                );
                assert!(!body.contains(&format!("{controller} ")), "{html}");
                let live = render_canonical_html(
                    &document.document(),
                    "".into(),
                    shim,
                    &HtmlShimExtraSlots::default(),
                )
                .unwrap()
                .html;
                assert!(
                    live.contains(&format!(
                        "<main {controller} data-mech-document-controller-extra='keep' {attribute}>"
                    )),
                    "{live}"
                );
            }
        }
    }
}
