//! Static bundle presentation uses the admitted retained document.
use mech_core::{GenericError, MResult, MechError};
use mech_runtime::CanonicalDocumentRenderer;
use mech_syntax::document::{AstNode, DocumentSyntax};
use std::collections::BTreeMap;

pub(crate) fn render_canonical_html(
    document: &DocumentSyntax,
    stylesheet: &str,
    shim: &str,
) -> MResult<String> {
    let content = CanonicalDocumentRenderer
        .format_html(document)
        .map_err(|error| presentation_error(error.to_string()))?;
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
    let slots = BTreeMap::from([
        ("STYLESHEET".to_owned(), stylesheet.to_owned()),
        ("PALETTE_STYLESHEET".to_owned(), String::new()),
        ("MECH_SOURCE_STYLESHEET".to_owned(), String::new()),
        ("MECHDOWN_STYLESHEET".to_owned(), String::new()),
        ("PAGE_STYLESHEET".to_owned(), stylesheet.to_owned()),
        ("MECH_REPL_STYLESHEET".to_owned(), String::new()),
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
        ("CONTENTS".to_owned(), content.clone()),
        ("CONTENT".to_owned(), content),
        ("CITED".to_owned(), String::new()),
        ("FOOTNOTES".to_owned(), String::new()),
        ("CODE".to_owned(), String::new()),
        ("REPL".to_owned(), repl.to_owned()),
        ("PRESENTATION".to_owned(), "document".to_owned()),
    ]);
    Ok(render_html_shim(shim, &slots))
}

fn render_html_shim(shim: &str, slots: &BTreeMap<String, String>) -> String {
    let mut html = String::with_capacity(shim.len());
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
        } else {
            html.push_str(&shim[open..token_end]);
        }
        cursor = token_end;
    }
    html.push_str(&shim[cursor..]);
    html
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
