use std::collections::{BTreeMap, BTreeSet};

use mech_core::{GenericError, MResult, MechError};
use mech_runtime::CanonicalDocumentRenderer;
use mech_syntax::document::{AstNode, DocumentSyntax};

#[derive(Clone, Debug, Default)]
pub(crate) struct HtmlShimExtraSlots {
    slots: BTreeMap<String, String>,
}

impl HtmlShimExtraSlots {
    #[cfg_attr(not(any(feature = "serve", feature = "formatter")), allow(dead_code))]
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
    #[cfg_attr(not(any(feature = "serve", feature = "formatter")), allow(dead_code))]
    pub consumed_slots: BTreeSet<String>,
    #[cfg_attr(not(any(feature = "serve", feature = "formatter")), allow(dead_code))]
    pub unresolved_mech_slots: BTreeSet<String>,
}

pub(crate) fn render_canonical_html(
    document: &DocumentSyntax,
    styles: HtmlStyleSheets,
    shim: String,
    extra_slots: &HtmlShimExtraSlots,
) -> MResult<HtmlShimRender> {
    let content = CanonicalDocumentRenderer
        .render_html(document, &[])
        .map_err(|error| presentation_error(error.to_string()))?;
    let title = document
        .title()
        .and_then(|title| title.syntax().text().ok())
        .unwrap_or_default();
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
        ("TITLE".to_owned(), escape_html_text(&title)),
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
        ("CONTENTS".to_owned(), content.clone()),
        ("CONTENT".to_owned(), content),
        ("CITED".to_owned(), String::new()),
        ("FOOTNOTES".to_owned(), String::new()),
        ("CODE".to_owned(), String::new()),
        ("REPL".to_owned(), repl.to_owned()),
        ("PRESENTATION".to_owned(), "document".to_owned()),
    ]);
    slots.extend(extra_slots.slots.clone());
    Ok(render_html_shim(&shim, &slots))
}

fn render_html_shim(shim: &str, slots: &BTreeMap<String, String>) -> HtmlShimRender {
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
        consumed_slots,
        unresolved_mech_slots,
    }
}

#[cfg_attr(not(any(feature = "serve", feature = "formatter")), allow(dead_code))]
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
