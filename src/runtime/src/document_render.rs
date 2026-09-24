//! Canonical document rendering from retained syntax and completed scope results.

use std::collections::{BTreeMap, HashMap, HashSet};

use mech_engine::{CanonicalSourceProgram, SourceDocumentOutputKind};
use mech_syntax::document::{
    AstNode, CodeBlockSyntax, CodeFenceScope, DocumentScopeId, DocumentSyntax,
    EvalInlineMechCodeSyntax, IdentifierSyntax, InlineMechCodeSyntax, MechCodeSyntax,
    MikaSectionSyntax, NodeFlags, OptionMapSyntax, ParagraphSyntax, SectionElementSyntax,
    SectionSyntax, SyntaxElement, SyntaxKind, SyntaxNode, TextRange, TitleSyntax, UlSubtitleSyntax,
};

use crate::RuntimeValueSnapshot;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum CanonicalRenderScope {
    Root,
    Named(String),
}

#[derive(Clone)]
struct CanonicalRenderedOutput {
    kind: SourceDocumentOutputKind,
    range: TextRange,
    value: RuntimeValueSnapshot,
    visible: bool,
}

#[derive(Clone)]
pub struct CanonicalScopeResults {
    owner: DocumentScopeId,
    scope: CanonicalRenderScope,
    revision: mech_syntax::document::Revision,
    outputs: Vec<CanonicalRenderedOutput>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalDocumentRenderError {
    pub message: String,
    pub range: Option<TextRange>,
}

impl core::fmt::Display for CanonicalDocumentRenderError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.message.fmt(formatter)
    }
}

impl std::error::Error for CanonicalDocumentRenderError {}

impl CanonicalScopeResults {
    pub fn owner(&self) -> DocumentScopeId {
        self.owner
    }

    pub fn scope(&self) -> &CanonicalRenderScope {
        &self.scope
    }

    /// Associate completed artifact outputs with their retained presentation owners.
    pub fn from_values(
        owner: DocumentScopeId,
        scope: CanonicalRenderScope,
        program: &CanonicalSourceProgram,
        values: &[RuntimeValueSnapshot],
    ) -> Result<Self, CanonicalDocumentRenderError> {
        let identity =
            program
                .source_map()
                .outputs
                .first()
                .ok_or_else(|| CanonicalDocumentRenderError {
                    message: "canonical scope program has no retained source identity".to_owned(),
                    range: None,
                })?;
        if program.document_owner() != Some(owner) || identity.document != owner.document {
            return Err(CanonicalDocumentRenderError {
                message: "scope program does not match its retained presentation owner".to_owned(),
                range: Some(identity.range),
            });
        }
        let mut outputs = Vec::new();
        for binding in program.document_outputs() {
            let output = binding.output as usize;
            let value =
                values
                    .get(output)
                    .cloned()
                    .ok_or_else(|| CanonicalDocumentRenderError {
                        message: format!("missing completed value for document output {output}"),
                        range: program
                            .source_map()
                            .outputs
                            .get(output)
                            .map(|anchor| anchor.range),
                    })?;
            let range = program
                .source_map()
                .outputs
                .get(output)
                .ok_or_else(|| CanonicalDocumentRenderError {
                    message: format!("document output {output} has no source anchor"),
                    range: None,
                })?
                .range;
            let anchor = program.source_map().outputs[output];
            if anchor.document != identity.document || anchor.revision != identity.revision {
                return Err(CanonicalDocumentRenderError {
                    message: "canonical scope outputs span more than one source revision"
                        .to_owned(),
                    range: Some(anchor.range),
                });
            }
            outputs.push(CanonicalRenderedOutput {
                kind: binding.kind,
                range,
                value,
                visible: binding.visible,
            });
        }
        Ok(Self {
            owner,
            scope,
            revision: identity.revision,
            outputs,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct CanonicalDocumentRenderer;

impl CanonicalDocumentRenderer {
    /// Render one complete root-only REPL submission from canonical syntax.
    /// Document prose, fences and Mika-local scopes are deliberately rejected:
    /// the browser document renderer owns those presentation forms.
    pub fn render_repl_source_html(
        &self,
        document: &DocumentSyntax,
    ) -> Result<Option<String>, CanonicalDocumentRenderError> {
        if document.syntax().flags().intersects(
            NodeFlags::ERROR
                | NodeFlags::MISSING
                | NodeFlags::CONTAINS_ERROR
                | NodeFlags::CONTAINS_MISSING,
        ) {
            return Err(CanonicalDocumentRenderError {
                message: "cannot render an invalid canonical REPL source".to_owned(),
                range: Some(document.syntax().range()),
            });
        }
        if !document.contains_executable_source() {
            return Ok(None);
        }

        let mut code = Vec::new();
        let mut pending = vec![document.syntax().clone()];
        while let Some(node) = pending.pop() {
            if let Some(mech_code) = MechCodeSyntax::cast(node.clone()) {
                code.extend(
                    mech_code
                        .items()
                        .into_iter()
                        .filter_map(|item| item.value()),
                );
                continue;
            }
            match node.kind() {
                SyntaxKind::Document
                | SyntaxKind::Body
                | SyntaxKind::Section
                | SyntaxKind::SectionElement => {
                    pending.extend(node.children());
                }
                SyntaxKind::Comment | SyntaxKind::BlankLine => {}
                // Only root code and trivia belong to a REPL submission. This
                // excludes every heading and other document presentation owner.
                _ => return Ok(None),
            }
        }
        if code.is_empty() {
            return Ok(None);
        }
        code.sort_by_key(|node| node.range().start);

        let source = document.syntax().source();
        let mut cursor = source.full_range().start;
        let mut output = String::from("<div class='mech-code-block'><pre><code>");
        for item in code {
            let range = item.range();
            let gap = source
                .text(TextRange::new(cursor, range.start))
                .map_err(|_| range_error(range))?;
            render_repl_gap_html(&gap, &mut output);
            output.push_str("<span class='");
            output.push_str(repl_item_class(&item));
            output.push_str("'>");
            for token in item.tokens() {
                let text = token.text().map_err(|_| range_error(token.range()))?;
                if token.kind() == SyntaxKind::Semicolon {
                    output.push_str("<span class='mech-code-terminal'>");
                    output.push_str(&escape_html(&text));
                    output.push_str("</span>");
                } else {
                    output.push_str(&escape_html(&text));
                }
            }
            output.push_str("</span>");
            cursor = range.end;
        }
        let tail = source
            .text(TextRange::new(cursor, source.full_range().end))
            .map_err(|_| range_error(source.full_range()))?;
        render_repl_gap_html(&tail, &mut output);
        output.push_str("</code></pre></div>");
        Ok(Some(output))
    }

    /// Format canonical syntax without execution or completed output values.
    pub fn format_html(
        &self,
        document: &DocumentSyntax,
    ) -> Result<String, CanonicalDocumentRenderError> {
        self.render_html_mode(document, &[], RenderMode::Source, true, &[])
    }

    /// Format section content for a host shim that owns the article and title.
    pub fn format_html_body(
        &self,
        document: &DocumentSyntax,
    ) -> Result<String, CanonicalDocumentRenderError> {
        self.render_html_mode(document, &[], RenderMode::Source, false, &[])
    }

    /// Format a browser fragment with addresses for values in the resident
    /// interactive document. The host fills these slots after publication.
    pub fn format_html_body_live(
        &self,
        document: &DocumentSyntax,
        output_addresses: &[(TextRange, u64)],
    ) -> Result<String, CanonicalDocumentRenderError> {
        self.render_html_mode(document, &[], RenderMode::Live, false, output_addresses)
    }

    /// Format a live browser document before execution, preserving canonical output addresses.
    pub fn format_browser_html(
        &self,
        document: &DocumentSyntax,
    ) -> Result<String, CanonicalDocumentRenderError> {
        self.render_html_mode(document, &[], RenderMode::Browser, true, &[])
    }

    /// Render the documented host-shim regions from the same retained syntax and
    /// inline/document rendering authority used by complete documents.
    pub fn format_browser_html_slots(
        &self,
        document: &DocumentSyntax,
    ) -> Result<BTreeMap<String, String>, CanonicalDocumentRenderError> {
        self.format_html_slots_mode(document, RenderMode::Browser)
    }

    /// Format a served source page without executable browser mounts.
    pub fn format_static_html_slots(
        &self,
        document: &DocumentSyntax,
    ) -> Result<BTreeMap<String, String>, CanonicalDocumentRenderError> {
        self.format_html_slots_mode(document, RenderMode::Source)
    }

    fn format_html_slots_mode(
        &self,
        document: &DocumentSyntax,
        mode: RenderMode,
    ) -> Result<BTreeMap<String, String>, CanonicalDocumentRenderError> {
        let lookup = ResultLookup::new(document, &[], mode)?;
        let owner = document.scope_id();
        let mut slots = BTreeMap::new();
        if let Some(front) = document.title().and_then(|title| title.front_matter()) {
            let mut key = None;
            for child in front.syntax().children() {
                if child.kind() == SyntaxKind::Identifier {
                    key = Some(node_text(&child)?.to_uppercase());
                } else if matches!(
                    child.kind(),
                    SyntaxKind::InlineParagraph | SyntaxKind::Img | SyntaxKind::Figures
                ) {
                    let name = key.take().ok_or_else(|| range_error(child.range()))?;
                    let mut html = String::new();
                    if child.kind() == SyntaxKind::InlineParagraph {
                        render_inline_html(&child, owner, &lookup, &mut html)?;
                    } else {
                        render_document_node_html(&child, owner, &lookup, &mut html)?;
                    }
                    slots.insert(name, html);
                }
            }
        }
        let mut intro = String::new();
        let mut abstract_html = String::new();
        let mut footnotes = String::new();
        let mut content = String::new();
        let mut toc = String::new();
        let mut numbered = 0usize;
        if let Some(body) = document.body() {
            for section in body.sections() {
                let mut section_html = String::new();
                for child in section.syntax().children() {
                    let value = SectionElementSyntax::cast(child.clone())
                        .and_then(|element| element.value())
                        .unwrap_or(child);
                    if value.kind() == SyntaxKind::UlSubtitle {
                        if numbered > 0 {
                            content.push_str(&section_html);
                            slots
                                .entry(format!("SECTION{numbered}"))
                                .or_insert_with(String::new)
                                .push_str(&section_html);
                            section_html.clear();
                        }
                        numbered += 1;
                    }
                    if matches!(value.kind(), SyntaxKind::UlSubtitle | SyntaxKind::Subtitle) {
                        let (number, _) = subtitle_coordinates(&value)?;
                        let paragraph = value
                            .children()
                            .find(|child| child.kind() == SyntaxKind::ParagraphNewline)
                            .ok_or_else(|| range_error(value.range()))?;
                        toc.push_str("<li><a href='#section-");
                        toc.push_str(&escape_attribute(&number));
                        toc.push_str("'>");
                        render_inline_children_html(
                            &paragraph,
                            owner,
                            &lookup,
                            &mut toc,
                            &[SyntaxKind::Newline, SyntaxKind::CarriageReturn],
                        )?;
                        toc.push_str("</a></li>");
                    }
                    let target = match value.kind() {
                        SyntaxKind::AbstractEl => &mut abstract_html,
                        SyntaxKind::Footnote => &mut footnotes,
                        _ if numbered == 0 => &mut intro,
                        _ => &mut section_html,
                    };
                    render_document_node_html(&value, owner, &lookup, target)?;
                }
                if numbered > 0 {
                    content.push_str(&section_html);
                    slots
                        .entry(format!("SECTION{numbered}"))
                        .or_insert_with(String::new)
                        .push_str(&section_html);
                }
            }
        }
        // The ordinary final result belongs after the document's source body.
        let result_region = if numbered == 0 {
            &mut intro
        } else {
            &mut content
        };
        append_program_html(
            owner,
            &CanonicalRenderScope::Root,
            &lookup,
            result_region,
            visible_root_program_range(document.syntax()),
        )?;
        let mut cited = String::new();
        append_citations_html(&lookup, &mut cited)?;
        if !toc.is_empty() {
            toc = format!(
                "<nav class='mech-toc' aria-label='Table of contents'><ol>{toc}</ol></nav>"
            );
        }
        slots.extend([
            ("INTRO".to_owned(), intro),
            ("ABSTRACT".to_owned(), abstract_html),
            ("FOOTNOTES".to_owned(), footnotes),
            ("CITED".to_owned(), cited),
            ("TOC".to_owned(), toc),
            ("CONTENTS".to_owned(), content.clone()),
            ("CONTENT".to_owned(), content),
        ]);
        Ok(slots)
    }

    pub fn render_html(
        &self,
        document: &DocumentSyntax,
        results: &[CanonicalScopeResults],
    ) -> Result<String, CanonicalDocumentRenderError> {
        self.render_html_mode(document, results, RenderMode::Completed, true, &[])
    }

    fn render_html_mode(
        &self,
        document: &DocumentSyntax,
        results: &[CanonicalScopeResults],
        mode: RenderMode,
        document_frame: bool,
        output_addresses: &[(TextRange, u64)],
    ) -> Result<String, CanonicalDocumentRenderError> {
        let mut lookup = ResultLookup::new(document, results, mode)?;
        lookup.output_addresses = output_addresses.iter().copied().collect();
        let mut output = String::new();
        if document_frame {
            output.push_str("<article class='mech-document'>");
        }
        if let Some(title) = document.title() {
            if document_frame {
                render_title_html(&title, document.scope_id(), &lookup, &mut output)?;
            } else {
                render_title_front_matter_html(&title, document.scope_id(), &lookup, &mut output)?;
            }
        }
        if let Some(body) = document.body() {
            for section in body.sections() {
                render_section_html(&section, document.scope_id(), &lookup, &mut output)?;
            }
        }
        append_program_html(
            document.scope_id(),
            &CanonicalRenderScope::Root,
            &lookup,
            &mut output,
            visible_root_program_range(document.syntax()),
        )?;
        append_citations_html(&lookup, &mut output)?;
        if document_frame {
            output.push_str("</article>");
        }
        Ok(output)
    }

    /// Preserve executable source and its syntax boundaries without evaluating it.
    pub fn format_text(
        &self,
        document: &DocumentSyntax,
    ) -> Result<String, CanonicalDocumentRenderError> {
        self.render_text_mode(document, &[], RenderMode::Source)
    }

    /// Pretty-print executable canonical items while retaining document prose,
    /// comments, strings, and source directives in their original regions.
    pub fn format_pretty_text(
        &self,
        document: &DocumentSyntax,
    ) -> Result<String, CanonicalDocumentRenderError> {
        let source = document
            .syntax()
            .source()
            .text(document.syntax().range())
            .map_err(|_| range_error(document.syntax().range()))?;
        let mut code_nodes = Vec::new();
        collect_nodes(document.syntax(), SyntaxKind::MechCode, &mut code_nodes);
        let mut edits = Vec::new();
        for code in code_nodes {
            let Some(code) = MechCodeSyntax::cast(code) else {
                continue;
            };
            for item in code.items() {
                let Some(value) = item.value() else {
                    continue;
                };
                if matches!(value.kind(), SyntaxKind::Comment | SyntaxKind::BlankLine) {
                    continue;
                }
                let formatted = format_canonical_item(&value)?;
                if formatted != node_text(&value)? {
                    edits.push((
                        value.range().start.0 as usize,
                        value.range().end.0 as usize,
                        formatted,
                    ));
                }
            }
        }
        edits.sort_by_key(|(start, end, _)| (*start, usize::MAX - *end));
        let mut formatted = String::with_capacity(source.len());
        let mut cursor = 0;
        for (start, end, replacement) in edits {
            if start < cursor {
                continue;
            }
            formatted.push_str(
                source
                    .get(cursor..start)
                    .ok_or_else(|| range_error(document.syntax().range()))?,
            );
            formatted.push_str(&replacement);
            cursor = end;
        }
        formatted.push_str(
            source
                .get(cursor..)
                .ok_or_else(|| range_error(document.syntax().range()))?,
        );
        Ok(formatted)
    }

    pub fn render_text(
        &self,
        document: &DocumentSyntax,
        results: &[CanonicalScopeResults],
    ) -> Result<String, CanonicalDocumentRenderError> {
        self.render_text_mode(document, results, RenderMode::Completed)
    }

    fn render_text_mode(
        &self,
        document: &DocumentSyntax,
        results: &[CanonicalScopeResults],
        mode: RenderMode,
    ) -> Result<String, CanonicalDocumentRenderError> {
        let lookup = ResultLookup::new(document, results, mode)?;
        let mut output = String::new();
        if let Some(title) = document.title() {
            render_inline_text(title.syntax(), document.scope_id(), &lookup, &mut output)?;
        }
        if let Some(body) = document.body() {
            for section in body.sections() {
                render_section_text(&section, document.scope_id(), &lookup, &mut output)?;
            }
        }
        append_program_text(
            document.scope_id(),
            &CanonicalRenderScope::Root,
            &lookup,
            &mut output,
            visible_root_program_range(document.syntax()),
        )?;
        Ok(output)
    }
}

fn repl_item_class(item: &SyntaxNode) -> &'static str {
    let mut kinds = vec![item.kind()];
    let mut pending = item.children().collect::<Vec<_>>();
    while let Some(node) = pending.pop() {
        kinds.push(node.kind());
        pending.extend(node.children());
    }
    let contains = |kind| kinds.contains(&kind);
    if contains(SyntaxKind::VariableDefine) {
        return "mech-variable-define";
    }
    if contains(SyntaxKind::VariableAssign) {
        return "mech-variable-assign";
    }
    if contains(SyntaxKind::OpAssign) {
        return "mech-op-assign";
    }
    if contains(SyntaxKind::FunctionDefine) {
        return "mech-function-define";
    }
    if kinds.iter().any(|kind| {
        matches!(
            kind,
            SyntaxKind::FsmImplementation
                | SyntaxKind::FsmSpecification
                | SyntaxKind::FsmStatementTransition
                | SyntaxKind::FsmBlockTransition
        )
    }) {
        return "mech-fsm";
    }
    match item.kind() {
        SyntaxKind::Comment => "mech-comment",
        SyntaxKind::ImportDeclaration | SyntaxKind::ModuleImport => "mech-import",
        SyntaxKind::ExportDeclaration => "mech-export",
        SyntaxKind::ContextDeclaration | SyntaxKind::ContextSend => "mech-context",
        SyntaxKind::Statement => "mech-statement",
        _ => "mech-expression",
    }
}

fn render_repl_gap_html(gap: &str, output: &mut String) {
    let mut remaining = gap;
    while !remaining.is_empty() {
        if let Some(comment) = remaining.strip_prefix("--") {
            let end = comment.find('\n').map_or(remaining.len(), |end| end + 2);
            output.push_str("<span class='mech-comment'>");
            output.push_str(&escape_html(&remaining[..end]));
            output.push_str("</span>");
            remaining = &remaining[end..];
        } else if let Some(after) = remaining.strip_prefix(';') {
            output.push_str("<span class='mech-code-terminal'>;</span>");
            remaining = after;
        } else {
            let next = remaining
                .char_indices()
                .skip(1)
                .find_map(|(index, _)| {
                    (remaining[index..].starts_with("--") || remaining[index..].starts_with(';'))
                        .then_some(index)
                })
                .unwrap_or(remaining.len());
            output.push_str(&escape_html(&remaining[..next]));
            remaining = &remaining[next..];
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ResultKey {
    owner: DocumentScopeId,
    scope: CanonicalRenderScope,
    kind: SourceDocumentOutputKind,
    range: Option<TextRange>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RenderMode {
    Source,
    Live,
    Browser,
    Completed,
}

struct ResultLookup<'a> {
    mode: RenderMode,
    root_owner: DocumentScopeId,
    values: HashMap<ResultKey, &'a RuntimeValueSnapshot>,
    output_addresses: HashMap<TextRange, u64>,
    citation_numbers: HashMap<String, usize>,
    citations: Vec<(DocumentScopeId, SyntaxNode)>,
    footnote_numbers: HashMap<String, usize>,
    coordinates: HashMap<TextRange, RetainedCoordinates>,
}

impl<'a> ResultLookup<'a> {
    fn new(
        document: &DocumentSyntax,
        results: &'a [CanonicalScopeResults],
        mode: RenderMode,
    ) -> Result<Self, CanonicalDocumentRenderError> {
        let mut owners = document
            .mika_scopes()
            .into_iter()
            .map(|scope| scope.section.scope_id())
            .collect::<HashSet<_>>();
        owners.insert(document.scope_id());
        let retained_coordinates = retained_coordinates(document);
        let mut values = HashMap::new();
        for result in results {
            if !owners.contains(&result.owner)
                || result.revision != document.syntax().source().revision()
            {
                return Err(CanonicalDocumentRenderError {
                    message: "scope results belong to a different canonical document revision"
                        .to_owned(),
                    range: None,
                });
            }
            for output in &result.outputs {
                let coordinates = retained_coordinates.get(&output.range);
                if coordinates.map(|coordinates| coordinates.owner) != Some(result.owner) {
                    return Err(CanonicalDocumentRenderError {
                        message: "scope results do not match their retained presentation owner"
                            .to_owned(),
                        range: Some(output.range),
                    });
                }
                if coordinates.and_then(|coordinates| coordinates.scope.as_ref())
                    != Some(&result.scope)
                {
                    return Err(CanonicalDocumentRenderError {
                        message:
                            "scope results do not match their retained document execution scope"
                                .to_owned(),
                        range: Some(output.range),
                    });
                }
                if !output.visible {
                    continue;
                }
                let key = ResultKey {
                    owner: result.owner,
                    scope: result.scope.clone(),
                    kind: output.kind,
                    range: (output.kind != SourceDocumentOutputKind::Program)
                        .then_some(output.range),
                };
                if values.insert(key, &output.value).is_some() {
                    return Err(CanonicalDocumentRenderError {
                        message: "duplicate results target the same document presentation slot"
                            .to_owned(),
                        range: Some(output.range),
                    });
                }
            }
        }
        let mut citation_nodes = Vec::new();
        collect_nodes(document.syntax(), SyntaxKind::Citation, &mut citation_nodes);
        citation_nodes.sort_by_key(|citation| citation.range().start);
        let mut citations = citation_nodes
            .into_iter()
            .map(|citation| {
                let owner = retained_coordinates
                    .get(&citation.range())
                    .map(|coordinates| coordinates.owner)
                    .ok_or_else(|| range_error(citation.range()))?;
                Ok((owner, citation))
            })
            .collect::<Result<Vec<_>, CanonicalDocumentRenderError>>()?;
        let mut defined_citations = HashSet::new();
        for (_, citation) in &citations {
            let label = definition_label(citation)?;
            if !defined_citations.insert(label.clone()) {
                return Err(CanonicalDocumentRenderError {
                    message: format!("duplicate citation definition {label:?}"),
                    range: Some(citation.range()),
                });
            }
        }
        let mut citation_occurrences = citations
            .iter()
            .map(|(_, citation)| citation.clone())
            .collect::<Vec<_>>();
        collect_nodes(
            document.syntax(),
            SyntaxKind::Reference,
            &mut citation_occurrences,
        );
        citation_occurrences.sort_by_key(|node| node.range().start);
        let mut citation_numbers = HashMap::new();
        for occurrence in citation_occurrences {
            let label = if occurrence.kind() == SyntaxKind::Citation {
                definition_label(&occurrence)?
            } else {
                reference_label(&occurrence, "[", "]")?
            };
            if !citation_numbers.contains_key(&label) {
                citation_numbers.insert(label, citation_numbers.len() + 1);
            }
        }
        citations.sort_by_key(|(_, citation)| {
            definition_label(citation)
                .ok()
                .and_then(|label| citation_numbers.get(&label).copied())
                .unwrap_or(usize::MAX)
        });
        let mut footnotes = Vec::new();
        collect_nodes(document.syntax(), SyntaxKind::Footnote, &mut footnotes);
        footnotes.sort_by_key(|footnote| footnote.range().start);
        let mut defined_footnotes = HashSet::new();
        for footnote in &footnotes {
            let label = definition_label(&footnote)?;
            if !defined_footnotes.insert(label.clone()) {
                return Err(CanonicalDocumentRenderError {
                    message: format!("duplicate footnote definition {label:?}"),
                    range: Some(footnote.range()),
                });
            }
        }
        let mut footnote_occurrences = footnotes;
        collect_nodes(
            document.syntax(),
            SyntaxKind::FootnoteReference,
            &mut footnote_occurrences,
        );
        footnote_occurrences.sort_by_key(|node| node.range().start);
        let mut footnote_numbers = HashMap::new();
        for occurrence in footnote_occurrences {
            let label = if occurrence.kind() == SyntaxKind::Footnote {
                definition_label(&occurrence)?
            } else {
                reference_label(&occurrence, "[^", "]")?
            };
            if !footnote_numbers.contains_key(&label) {
                footnote_numbers.insert(label, footnote_numbers.len() + 1);
            }
        }
        Ok(Self {
            mode,
            root_owner: document.scope_id(),
            values,
            output_addresses: HashMap::new(),
            citation_numbers,
            citations,
            footnote_numbers,
            coordinates: retained_coordinates,
        })
    }

    fn inline_value(
        &self,
        owner: DocumentScopeId,
        range: TextRange,
    ) -> Option<&RuntimeValueSnapshot> {
        let coordinates = self.coordinates.get(&range)?;
        if coordinates.owner != owner {
            return None;
        }
        self.get(
            owner,
            coordinates.scope.as_ref()?,
            SourceDocumentOutputKind::Inline,
            Some(range),
        )
    }

    fn get(
        &self,
        owner: DocumentScopeId,
        scope: &CanonicalRenderScope,
        kind: SourceDocumentOutputKind,
        range: Option<TextRange>,
    ) -> Option<&RuntimeValueSnapshot> {
        self.values
            .get(&ResultKey {
                owner,
                scope: scope.clone(),
                kind,
                range,
            })
            .copied()
    }
}

#[derive(Clone)]
struct RetainedCoordinates {
    owner: DocumentScopeId,
    scope: Option<CanonicalRenderScope>,
}

fn retained_coordinates(document: &DocumentSyntax) -> HashMap<TextRange, RetainedCoordinates> {
    let mut coordinates = HashMap::new();
    let mut pending = vec![(
        document.syntax().clone(),
        RetainedCoordinates {
            owner: document.scope_id(),
            scope: Some(CanonicalRenderScope::Root),
        },
    )];
    while let Some((node, inherited)) = pending.pop() {
        let mut retained = inherited;
        if let Some(section) = MikaSectionSyntax::cast(node.clone()) {
            retained = RetainedCoordinates {
                owner: section.scope_id(),
                scope: Some(CanonicalRenderScope::Root),
            };
        }
        if let Some(fence) = CodeBlockSyntax::cast(node.clone()) {
            retained.scope = fence.info().and_then(|info| render_scope(&info.scope));
        }
        coordinates.insert(node.range(), retained.clone());
        for child in node.children() {
            pending.push((child, retained.clone()));
        }
    }
    coordinates
}

fn render_title_html(
    title: &TitleSyntax,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    let title_end = title
        .syntax()
        .tokens()
        .into_iter()
        .find(|token| {
            matches!(
                token.kind(),
                SyntaxKind::Newline | SyntaxKind::CarriageReturn
            )
        })
        .map(|token| token.range().start)
        .ok_or_else(|| range_error(title.syntax().range()))?;
    let title_text = title
        .syntax()
        .source()
        .text(TextRange::new(title.syntax().range().start, title_end))
        .map_err(|_| range_error(title.syntax().range()))?;
    output.push_str("<header class='mech-document-header'><h1 class='mech-document-title'>");
    output.push_str(&escape_html(title_text.trim()));
    output.push_str("</h1>");
    render_title_front_matter_html(title, owner, lookup, output)?;
    output.push_str("</header>");
    Ok(())
}

fn render_title_front_matter_html(
    title: &TitleSyntax,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    if let Some(front_matter) = title.front_matter() {
        output.push_str("<dl class='mech-title-front-matter'>");
        let mut pending_key = None::<String>;
        for child in front_matter.syntax().children() {
            if let Some(key) = IdentifierSyntax::cast(child.clone()) {
                pending_key = Some(node_text(key.syntax())?);
            } else if matches!(
                child.kind(),
                SyntaxKind::InlineParagraph | SyntaxKind::Img | SyntaxKind::Figures
            ) {
                let key = pending_key
                    .take()
                    .ok_or_else(|| range_error(child.range()))?;
                output.push_str("<div class='mech-title-field'><dt>");
                output.push_str(&escape_html(&key));
                output.push_str("</dt><dd>");
                if child.kind() == SyntaxKind::InlineParagraph {
                    render_inline_html(&child, owner, lookup, output)?;
                } else {
                    render_document_node_html(&child, owner, lookup, output)?;
                }
                output.push_str("</dd></div>");
            }
        }
        if pending_key.is_some() {
            return Err(range_error(front_matter.syntax().range()));
        }
        output.push_str("</dl>");
    }
    Ok(())
}

fn render_section_html(
    section: &SectionSyntax,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    output.push_str("<section class='mech-section'>");
    for child in section.syntax().children() {
        let value = SectionElementSyntax::cast(child.clone())
            .and_then(|element| element.value())
            .unwrap_or(child);
        render_document_node_html(&value, owner, lookup, output)?;
    }
    output.push_str("</section>");
    Ok(())
}

fn render_document_node_html(
    value: &SyntaxNode,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    if let Some(paragraph) = ParagraphSyntax::cast(value.clone()) {
        output.push_str("<p>");
        render_paragraph_html(&paragraph, owner, lookup, output)?;
        output.push_str("</p>");
    } else if let Some(fence) = CodeBlockSyntax::cast(value.clone()) {
        render_fence_html(&fence, owner, lookup, output)?;
    } else if let Some(mika) = find::<MikaSectionSyntax>(value) {
        let child = mika.scope_id();
        output.push_str("<section class='mech-mika'>");
        if let Some(body) = mika.body() {
            render_section_html(&body, child, lookup, output)?;
        }
        let required = mika
            .body()
            .and_then(|body| visible_root_program_range(body.syntax()));
        append_program_html(child, &CanonicalRenderScope::Root, lookup, output, required)?;
        output.push_str("</section>");
    } else if value.kind() == SyntaxKind::Img {
        render_image_html(value, owner, lookup, output)?;
    } else if matches!(
        value.kind(),
        SyntaxKind::AbstractEl
            | SyntaxKind::QuoteBlock
            | SyntaxKind::InfoBlock
            | SyntaxKind::SuccessBlock
            | SyntaxKind::IdeaBlock
            | SyntaxKind::WarningBlock
            | SyntaxKind::ErrorBlock
            | SyntaxKind::QuestionBlock
            | SyntaxKind::Prompt
    ) {
        render_callout_html(value, owner, lookup, output)?;
    } else if value.kind() == SyntaxKind::ThematicBreak {
        output.push_str("<hr class='mech-thematic-break' />");
    } else if value.kind() == SyntaxKind::Equation {
        render_equation_html(value, output)?;
    } else if value.kind() == SyntaxKind::MechdownList {
        render_list_html(value, owner, lookup, output)?;
    } else if value.kind() == SyntaxKind::MechdownTable {
        render_table_html(value, owner, lookup, output)?;
    } else if value.kind() == SyntaxKind::Citation {
        // Citation definitions are emitted together in document backmatter.
    } else if value.kind() == SyntaxKind::Footnote {
        render_note_definition_html(value, owner, lookup, output)?;
    } else if matches!(
        value.kind(),
        SyntaxKind::Figures | SyntaxKind::FiguresRow | SyntaxKind::FigureItem | SyntaxKind::Float
    ) {
        render_figure_container_html(value, owner, lookup, output)?;
    } else if let Some(code) = MechCodeSyntax::cast(value.clone()) {
        output.push_str("<pre class='mech-code'><code>");
        render_code_comments(code.syntax(), owner, lookup, output, true)?;
        output.push_str("</code></pre>");
    } else if UlSubtitleSyntax::cast(value.clone()).is_some()
        || value.kind() == SyntaxKind::Subtitle
    {
        render_subtitle_html(value, owner, lookup, output)?;
    } else if value.kind() != mech_syntax::document::SyntaxKind::BlankLine {
        output.push_str("<div class='mech-document-node' data-mech-kind='");
        output.push_str(&format!("{:?}", value.kind()));
        output.push_str("'>");
        render_inline(value, owner, lookup, output, true)?;
        output.push_str("</div>");
    }
    Ok(())
}

fn render_subtitle_html(
    node: &SyntaxNode,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    let (section, level) = subtitle_coordinates(node)?;
    let paragraph = node
        .children()
        .find(|child| child.kind() == SyntaxKind::ParagraphNewline)
        .ok_or_else(|| range_error(node.range()))?;
    output.push_str(&format!("<h{level} class='mech-subtitle' id='section-"));
    output.push_str(&escape_attribute(&section));
    output.push_str("'>");
    render_inline_children_html(
        &paragraph,
        owner,
        lookup,
        output,
        &[SyntaxKind::Newline, SyntaxKind::CarriageReturn],
    )?;
    output.push_str(&format!("</h{level}>"));
    Ok(())
}

fn subtitle_coordinates(
    node: &SyntaxNode,
) -> Result<(String, usize), CanonicalDocumentRenderError> {
    let source = node_text(node)?;
    let first_line = source
        .lines()
        .next()
        .ok_or_else(|| range_error(node.range()))?
        .trim();
    let (section, level) = if node.kind() == SyntaxKind::UlSubtitle {
        let section = first_line
            .split_once('.')
            .map(|(section, _)| section)
            .filter(|section| valid_section_number(section));
        // Underlined headings may be named/annotated without an authored
        // section number. Use their retained anchor for a unique navigation
        // address instead of rejecting otherwise valid document syntax.
        return Ok((
            section
                .map(str::to_owned)
                .unwrap_or_else(|| format!("at-{}", node.range().start.0)),
            2,
        ));
    } else {
        let (section, _) = first_line
            .strip_prefix('(')
            .and_then(|line| line.split_once(')'))
            .filter(|(section, _)| valid_section_number(section))
            .ok_or_else(|| range_error(node.range()))?;
        let depth = section.split('.').count();
        (section, if depth < 3 { 3 } else { depth + 1 }.min(6))
    };
    Ok((section.to_owned(), level))
}

fn valid_section_number(value: &str) -> bool {
    !value.is_empty()
        && value.split('.').all(|part| {
            !part.is_empty() && part.chars().all(|character| character.is_alphanumeric())
        })
}

fn render_section_text(
    section: &SectionSyntax,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    for child in section.syntax().children() {
        let value = SectionElementSyntax::cast(child.clone())
            .and_then(|element| element.value())
            .unwrap_or(child);
        render_document_node_text(&value, owner, lookup, output)?;
    }
    Ok(())
}

fn render_document_node_text(
    value: &SyntaxNode,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    if let Some(paragraph) = ParagraphSyntax::cast(value.clone()) {
        render_paragraph_text(&paragraph, owner, lookup, output)?;
    } else if let Some(fence) = CodeBlockSyntax::cast(value.clone()) {
        render_fence_text(&fence, owner, lookup, output)?;
    } else if let Some(mika) = find::<MikaSectionSyntax>(value) {
        if lookup.mode != RenderMode::Completed {
            output.push_str(&node_text(value)?);
            return Ok(());
        }
        let child = mika.scope_id();
        if let Some(body) = mika.body() {
            render_section_text(&body, child, lookup, output)?;
        }
        let required = mika
            .body()
            .and_then(|body| visible_root_program_range(body.syntax()));
        append_program_text(child, &CanonicalRenderScope::Root, lookup, output, required)?;
    } else if let Some(code) = MechCodeSyntax::cast(value.clone()) {
        render_code_comments(code.syntax(), owner, lookup, output, false)?;
    } else if UlSubtitleSyntax::cast(value.clone()).is_some() {
        output.push_str(&node_text(value)?);
    } else if value.kind() == mech_syntax::document::SyntaxKind::BlankLine {
        output.push_str(&node_text(value)?);
    } else {
        render_inline(value, owner, lookup, output, false)?;
    }
    Ok(())
}

fn render_callout_html(
    node: &SyntaxNode,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    let (tag, class) = match node.kind() {
        SyntaxKind::AbstractEl => ("aside", "mech-abstract"),
        SyntaxKind::QuoteBlock => ("blockquote", "mech-quote-block"),
        SyntaxKind::InfoBlock => ("aside", "mech-info-block"),
        SyntaxKind::SuccessBlock => ("aside", "mech-success-block"),
        SyntaxKind::IdeaBlock => ("aside", "mech-idea-block"),
        SyntaxKind::WarningBlock => ("aside", "mech-warning-block"),
        SyntaxKind::ErrorBlock => ("aside", "mech-error-block"),
        SyntaxKind::QuestionBlock => ("aside", "mech-question-block"),
        SyntaxKind::Prompt => ("div", "mech-prompt"),
        _ => return Err(range_error(node.range())),
    };
    output.push_str(&format!("<{tag} class='{class}'>"));
    for child in node.children() {
        if child.kind() == SyntaxKind::ParagraphNewline {
            render_retained_paragraph_html(&child, owner, lookup, output)?;
        } else if child.kind() == SyntaxKind::SectionElement {
            let value = SectionElementSyntax::cast(child.clone())
                .and_then(|element| element.value())
                .ok_or_else(|| range_error(child.range()))?;
            render_document_node_html(&value, owner, lookup, output)?;
        } else {
            render_inline_html(&child, owner, lookup, output)?;
        }
    }
    output.push_str(&format!("</{tag}>"));
    Ok(())
}

fn render_retained_paragraph_html(
    node: &SyntaxNode,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    let paragraph = ParagraphSyntax::cast(node.clone())
        .or_else(|| find::<ParagraphSyntax>(node))
        .ok_or_else(|| range_error(node.range()))?;
    output.push_str("<p>");
    render_paragraph_html(&paragraph, owner, lookup, output)?;
    output.push_str("</p>");
    Ok(())
}

fn render_equation_html(
    node: &SyntaxNode,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    output.push_str("<div class='mech-equation'>");
    for element in node.children_with_tokens() {
        if let SyntaxElement::Token(token) = element
            && token.kind() != SyntaxKind::EquationSigil
        {
            output.push_str(&escape_html(
                &token.text().map_err(|_| range_error(token.range()))?,
            ));
        }
    }
    output.push_str("</div>");
    Ok(())
}

fn render_list_html(
    node: &SyntaxNode,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    match node.kind() {
        SyntaxKind::MechdownList | SyntaxKind::Sublist => {
            for child in node.children() {
                render_list_html(&child, owner, lookup, output)?;
            }
        }
        SyntaxKind::OrderedList | SyntaxKind::UnorderedList | SyntaxKind::CheckList => {
            let (tag, class) = if node.kind() == SyntaxKind::OrderedList {
                ("ol", "mech-ordered-list")
            } else if node.kind() == SyntaxKind::CheckList {
                ("ul", "mech-check-list")
            } else {
                ("ul", "mech-unordered-list")
            };
            output.push_str(&format!("<{tag} class='{class}'"));
            if node.kind() == SyntaxKind::OrderedList
                && let Some(start) = node.children().find_map(|child| {
                    (child.kind() == SyntaxKind::OrderedListItem)
                        .then(|| ordered_list_marker(&child))
                        .flatten()
                })
                && start != 1
            {
                output.push_str(&format!(" start='{start}'"));
            }
            output.push('>');
            for child in node.children() {
                render_list_html(&child, owner, lookup, output)?;
            }
            output.push_str(&format!("</{tag}>"));
        }
        SyntaxKind::OrderedListItem => {
            output.push_str("<li");
            if let Some(value) = ordered_list_marker(node) {
                output.push_str(&format!(" value='{value}'"));
            }
            output.push('>');
            for child in node.children() {
                render_list_html(&child, owner, lookup, output)?;
            }
            output.push_str("</li>");
        }
        SyntaxKind::UnorderedListItem | SyntaxKind::CheckListItem => {
            output.push_str("<li>");
            for child in node.children() {
                render_list_html(&child, owner, lookup, output)?;
            }
            output.push_str("</li>");
        }
        SyntaxKind::CheckedItem | SyntaxKind::UncheckedItem => {
            output.push_str("<input class='mech-check-item' type='checkbox' disabled");
            if node.kind() == SyntaxKind::CheckedItem {
                output.push_str(" checked");
            }
            output.push_str(" />");
            let mut paragraph_index = 0usize;
            for child in node.children() {
                if child.kind() == SyntaxKind::ParagraphNewline {
                    let paragraph = find::<ParagraphSyntax>(&child)
                        .ok_or_else(|| range_error(child.range()))?;
                    if paragraph_index == 0 {
                        output.push_str("<span class='mech-list-item-label'>");
                    } else {
                        output.push_str("<p class='mech-list-item-continuation'>");
                    }
                    render_paragraph_html(&paragraph, owner, lookup, output)?;
                    if paragraph_index == 0 {
                        output.push_str("</span>");
                    } else {
                        output.push_str("</p>");
                    }
                    paragraph_index += 1;
                } else {
                    render_list_html(&child, owner, lookup, output)?;
                }
            }
        }
        SyntaxKind::ParagraphNewline | SyntaxKind::Paragraph | SyntaxKind::InlineParagraph => {
            render_inline_html(node, owner, lookup, output)?;
        }
        _ => {
            for child in node.children() {
                render_list_html(&child, owner, lookup, output)?;
            }
        }
    }
    Ok(())
}

fn ordered_list_marker(node: &SyntaxNode) -> Option<u64> {
    node_text(node)
        .ok()?
        .lines()
        .next()?
        .split_once('.')?
        .0
        .trim()
        .parse()
        .ok()
}

fn render_table_html(
    node: &SyntaxNode,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    match node.kind() {
        SyntaxKind::MechdownTable => {
            for child in node.children() {
                render_table_html(&child, owner, lookup, output)?;
            }
        }
        SyntaxKind::MechdownTableWithHeader | SyntaxKind::MechdownTableNoHeader => {
            output.push_str("<table class='mech-table'>");
            let header = node
                .children()
                .find(|child| child.kind() == SyntaxKind::MechdownTableHeader);
            let alignments = header.as_ref().map(table_alignments).unwrap_or_default();
            if let Some(header) = header {
                output.push_str("<thead><tr>");
                render_table_cells(&header, "th", &alignments, owner, lookup, output)?;
                output.push_str("</tr></thead>");
            }
            output.push_str("<tbody>");
            for row in node
                .children()
                .filter(|child| child.kind() == SyntaxKind::MechdownTableRow)
            {
                output.push_str("<tr>");
                render_table_cells(&row, "td", &alignments, owner, lookup, output)?;
                output.push_str("</tr>");
            }
            output.push_str("</tbody></table>");
        }
        _ => return Err(range_error(node.range())),
    }
    Ok(())
}

fn render_table_cells(
    row: &SyntaxNode,
    tag: &str,
    alignments: &[&str],
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    for (index, cell) in row
        .children()
        .filter(|child| {
            matches!(
                child.kind(),
                SyntaxKind::InlineParagraph | SyntaxKind::EmptyParagraph
            )
        })
        .enumerate()
    {
        let alignment = alignments.get(index).copied().unwrap_or("left");
        output.push_str(&format!(
            "<{tag} class='mech-table-cell mech-align-{alignment}'>"
        ));
        render_inline_html(&cell, owner, lookup, output)?;
        output.push_str(&format!("</{tag}>"));
    }
    Ok(())
}

fn table_alignments(header: &SyntaxNode) -> Vec<&'static str> {
    header
        .children()
        .filter(|child| child.kind() == SyntaxKind::AlignmentSeparator)
        .map(|alignment| {
            if find::<mech_syntax::document::CenterAlignmentSyntax>(&alignment).is_some() {
                "center"
            } else if find::<mech_syntax::document::RightAlignmentSyntax>(&alignment).is_some() {
                "right"
            } else {
                "left"
            }
        })
        .collect()
}

fn render_figure_container_html(
    node: &SyntaxNode,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    if node.kind() == SyntaxKind::Figures {
        return render_figures_html(node, owner, lookup, output);
    }
    let class = match node.kind() {
        SyntaxKind::FiguresRow => "mech-figures-row",
        SyntaxKind::FigureItem => "mech-figure-item",
        SyntaxKind::Float => {
            let sigil = node
                .children()
                .find(|child| child.kind() == SyntaxKind::FloatSigil)
                .ok_or_else(|| range_error(node.range()))?;
            if node_text(&sigil)?.starts_with("<<") {
                "mech-float mech-float-left"
            } else {
                "mech-float mech-float-right"
            }
        }
        _ => return Err(range_error(node.range())),
    };
    output.push_str(&format!("<div class='{class}'>"));
    for child in node.children() {
        if child.kind() == SyntaxKind::FloatSigil {
            continue;
        }
        match child.kind() {
            SyntaxKind::Figures
            | SyntaxKind::FiguresRow
            | SyntaxKind::FigureItem
            | SyntaxKind::Float => {
                render_figure_container_html(&child, owner, lookup, output)?;
            }
            SyntaxKind::SectionElement => {
                if let Some(value) =
                    SectionElementSyntax::cast(child.clone()).and_then(|element| element.value())
                {
                    render_document_node_html(&value, owner, lookup, output)?;
                }
            }
            _ => render_inline_html(&child, owner, lookup, output)?,
        }
    }
    output.push_str("</div>");
    Ok(())
}

fn render_figures_html(
    node: &SyntaxNode,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    let mut panels = Vec::<(String, RetainedImage)>::new();
    let mut panel_index = 0usize;
    output.push_str("<figure class='mech-figure-table'><div class='mech-figure-grid'>");
    for row in node
        .children()
        .filter(|child| child.kind() == SyntaxKind::FiguresRow)
    {
        output.push_str("<div class='mech-figures-row'>");
        for item in row
            .children()
            .filter(|child| child.kind() == SyntaxKind::FigureItem)
        {
            let image_node = item
                .children()
                .find(|child| child.kind() == SyntaxKind::Img)
                .ok_or_else(|| range_error(item.range()))?;
            let image = retained_image(&image_node)?;
            let label = panel_label(panel_index);
            panel_index += 1;
            output.push_str("<figure class='mech-subfigure' data-panel='");
            output.push_str(&escape_attribute(&label));
            output.push_str("'>");
            render_image_tag_html(&image, output);
            output.push_str(
                "<figcaption class='mech-subfigure-caption'><span class='mech-subfigure-label'>(",
            );
            output.push_str(&escape_html(&label));
            output.push_str(")</span> ");
            if let Some(caption) = &image.caption {
                render_inline_html(caption, owner, lookup, output)?;
            }
            output.push_str("</figcaption></figure>");
            panels.push((label, image));
        }
        output.push_str("</div>");
    }
    output.push_str("</div><figcaption class='mech-figure-table-caption'>");
    for (index, (label, image)) in panels.iter().enumerate() {
        if index > 0 {
            output.push(' ');
        }
        output
            .push_str("<span class='mech-subfigure-summary'><span class='mech-subfigure-label'>(");
        output.push_str(&escape_html(label));
        output.push_str(")</span> ");
        if let Some(caption) = &image.caption {
            render_inline_html(caption, owner, lookup, output)?;
        }
        output.push_str("</span>");
    }
    output.push_str("</figcaption></figure>");
    Ok(())
}

fn panel_label(index: usize) -> String {
    if index < 26 {
        char::from(b'a' + index as u8).to_string()
    } else {
        (index + 1).to_string()
    }
}

fn render_note_definition_html(
    node: &SyntaxNode,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    let prefix = "footnote";
    let label = definition_label(node)?;
    output.push_str("<aside class='mech-footnote' id='");
    output.push_str(&escape_attribute(&format!("{prefix}-{label}")));
    output.push_str("'><span class='mech-footnote-id'>");
    if let Some(number) = lookup.footnote_numbers.get(&label) {
        output.push_str(&number.to_string());
    } else {
        output.push_str(&escape_html(&label));
    }
    output.push_str(":</span>");
    for child in node.children() {
        if ParagraphSyntax::cast(child.clone()).is_some()
            || child.kind() == SyntaxKind::ParagraphNewline
        {
            render_retained_paragraph_html(&child, owner, lookup, output)?;
        }
    }
    output.push_str("</aside>");
    Ok(())
}

fn definition_label(node: &SyntaxNode) -> Result<String, CanonicalDocumentRenderError> {
    let tokens = node.tokens();
    let start = tokens
        .iter()
        .find(|token| {
            matches!(
                token.kind(),
                SyntaxKind::LeftBracket | SyntaxKind::FootnotePrefix
            )
        })
        .map(|token| token.range().end)
        .ok_or_else(|| range_error(node.range()))?;
    let end = tokens
        .iter()
        .find(|token| token.kind() == SyntaxKind::RightBracket && token.range().start >= start)
        .map(|token| token.range().start)
        .ok_or_else(|| range_error(node.range()))?;
    let label = node
        .source()
        .text(TextRange::new(start, end))
        .map_err(|_| range_error(node.range()))?;
    Ok(label)
}

fn append_citations_html(
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    if lookup.citations.is_empty() {
        return Ok(());
    }
    output.push_str(
        "<section class='mech-works-cited'><h3 class='mech-backmatter-heading'>Works Cited</h3>",
    );
    for (owner, citation) in &lookup.citations {
        let label = definition_label(citation)?;
        let number = lookup
            .citation_numbers
            .get(&label)
            .copied()
            .ok_or_else(|| range_error(citation.range()))?;
        output.push_str("<div class='mech-citation' id='reference-");
        output.push_str(&escape_attribute(&label));
        output.push_str("'><span class='mech-citation-id'>[");
        output.push_str(&number.to_string());
        output.push_str("]:</span><div class='mech-citation-body'>");
        for child in citation.children() {
            if let Some(paragraph) =
                ParagraphSyntax::cast(child.clone()).or_else(|| find::<ParagraphSyntax>(&child))
            {
                render_citation_paragraph_html(&paragraph, *owner, lookup, output)?;
            }
        }
        output.push_str("</div></div>");
    }
    output.push_str("</section>");
    Ok(())
}

fn render_paragraph_html(
    paragraph: &ParagraphSyntax,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    render_inline(paragraph.syntax(), owner, lookup, output, true)
}

fn render_paragraph_text(
    paragraph: &ParagraphSyntax,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    render_inline(paragraph.syntax(), owner, lookup, output, false)
}

fn render_inline(
    node: &SyntaxNode,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
    html: bool,
) -> Result<(), CanonicalDocumentRenderError> {
    if html {
        return render_inline_html(node, owner, lookup, output);
    }
    render_inline_text(node, owner, lookup, output)
}

fn render_inline_text(
    node: &SyntaxNode,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    if lookup.mode != RenderMode::Completed {
        return push_source(node, node.range(), output, false);
    }
    let mut replacements = Vec::new();
    collect_inline_replacements(node, &mut replacements);
    replacements.sort_by_key(|node| node.range().start);
    let mut cursor = node.range().start;
    for replacement in replacements {
        let range = replacement.range();
        push_source(node, TextRange::new(cursor, range.start), output, false)?;
        if EvalInlineMechCodeSyntax::cast(replacement.clone()).is_some() {
            let value =
                lookup
                    .inline_value(owner, range)
                    .ok_or_else(|| CanonicalDocumentRenderError {
                        message: "evaluated inline expression has no completed scope result"
                            .to_owned(),
                        range: Some(range),
                    })?;
            output.push_str(&value.format_canonical_inline());
        } else {
            let source = node.source().text(range).map_err(|_| range_error(range))?;
            let inner = source
                .strip_prefix("{{")
                .and_then(|source| source.strip_suffix("}}"))
                .unwrap_or(&source);
            output.push_str(inner);
        }
        cursor = range.end;
    }
    push_source(
        node,
        TextRange::new(cursor, node.range().end),
        output,
        false,
    )
}

fn render_inline_html(
    node: &SyntaxNode,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    match node.kind() {
        SyntaxKind::EvalInlineMechCode => {
            if lookup.mode == RenderMode::Live {
                let range = node.range();
                if let Some(address) = lookup.output_addresses.get(&range) {
                    output.push_str(&format!(
                        "<code id='{address}:0' class='mech-inline-mech-code' data-mech-source>{}</code>",
                        escape_html(&node_text(node)?)
                    ));
                } else {
                    output.push_str("<code class='mech-inline'>");
                    output.push_str(&escape_html(&node_text(node)?));
                    output.push_str("</code>");
                }
                return Ok(());
            }
            if lookup.mode == RenderMode::Browser {
                if lookup
                    .coordinates
                    .get(&node.range())
                    .is_some_and(|coordinates| {
                        coordinates.owner == lookup.root_owner
                            && coordinates.scope.as_ref() == Some(&CanonicalRenderScope::Root)
                    })
                {
                    append_browser_mount(
                        output,
                        "span",
                        "mech-inline-mech-code",
                        SourceDocumentOutputKind::Inline,
                        node.range(),
                    );
                } else {
                    output.push_str("<code class='mech-inline'>");
                    output.push_str(&escape_html(&node_text(node)?));
                    output.push_str("</code>");
                }
                return Ok(());
            }
            if lookup.mode == RenderMode::Source {
                output.push_str("<code class='mech-inline'>");
                output.push_str(&escape_html(&node_text(node)?));
                output.push_str("</code>");
                return Ok(());
            }
            let range = node.range();
            let value =
                lookup
                    .inline_value(owner, range)
                    .ok_or_else(|| CanonicalDocumentRenderError {
                        message: "evaluated inline expression has no completed scope result"
                            .to_owned(),
                        range: Some(range),
                    })?;
            output.push_str(&value.format_html());
            Ok(())
        }
        SyntaxKind::InlineMechCode => {
            let source = node_text(node)?;
            let inner = source
                .strip_prefix("{{")
                .and_then(|source| source.strip_suffix("}}"))
                .unwrap_or(&source);
            output.push_str("<code class='mech-inline'>");
            output.push_str(&escape_html(inner));
            output.push_str("</code>");
            Ok(())
        }
        SyntaxKind::Strong => render_inline_wrapper(
            node,
            owner,
            lookup,
            output,
            "strong",
            "mech-strong",
            &[SyntaxKind::StrongSigil],
        ),
        SyntaxKind::Emphasis => render_inline_wrapper(
            node,
            owner,
            lookup,
            output,
            "em",
            "mech-emphasis",
            &[SyntaxKind::EmphasisSigil],
        ),
        SyntaxKind::Underline => render_inline_wrapper(
            node,
            owner,
            lookup,
            output,
            "u",
            "mech-underline",
            &[SyntaxKind::UnderlineSigil, SyntaxKind::Underscore],
        ),
        SyntaxKind::Strikethrough => render_inline_wrapper(
            node,
            owner,
            lookup,
            output,
            "del",
            "mech-strikethrough",
            &[SyntaxKind::StrikeSigil, SyntaxKind::Tilde],
        ),
        SyntaxKind::Highlight => render_inline_wrapper(
            node,
            owner,
            lookup,
            output,
            "mark",
            "mech-highlight",
            &[SyntaxKind::HighlightSigil],
        ),
        SyntaxKind::Hyperlink => render_hyperlink_html(node, owner, lookup, output),
        SyntaxKind::RawHyperlink => render_raw_hyperlink_html(node, output, false),
        SyntaxKind::InlineCode => render_inline_code_html(node, output),
        SyntaxKind::InlineEquation => render_delimited_inline_html(
            node,
            owner,
            lookup,
            output,
            "span",
            "mech-inline-equation",
            &[SyntaxKind::EquationSigil],
        ),
        SyntaxKind::Reference => render_citation_reference_html(node, lookup, output),
        SyntaxKind::FootnoteReference => render_footnote_reference_html(node, lookup, output),
        SyntaxKind::SectionReference => render_reference_html(
            node,
            "mech-section-reference-link",
            "section",
            "§",
            "",
            output,
        ),
        SyntaxKind::Img => render_image_html(node, owner, lookup, output),
        _ => render_inline_children_html(node, owner, lookup, output, &[]),
    }
}

fn render_inline_wrapper(
    node: &SyntaxNode,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
    tag: &str,
    class: &str,
    delimiters: &[SyntaxKind],
) -> Result<(), CanonicalDocumentRenderError> {
    output.push_str(&format!("<{tag} class='{class}'>"));
    render_inline_children_html(node, owner, lookup, output, delimiters)?;
    output.push_str(&format!("</{tag}>"));
    Ok(())
}

fn render_inline_children_html(
    node: &SyntaxNode,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
    skipped_tokens: &[SyntaxKind],
) -> Result<(), CanonicalDocumentRenderError> {
    for element in node.children_with_tokens() {
        match element {
            SyntaxElement::Node(child) => render_inline_html(&child, owner, lookup, output)?,
            SyntaxElement::Token(token) if skipped_tokens.contains(&token.kind()) => {}
            SyntaxElement::Token(token) => {
                output.push_str(&escape_html(
                    &token.text().map_err(|_| range_error(token.range()))?,
                ));
            }
        }
    }
    Ok(())
}

fn render_hyperlink_html(
    node: &SyntaxNode,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    render_hyperlink_html_with_attributes(node, owner, lookup, output, false)
}

fn render_hyperlink_html_with_attributes(
    node: &SyntaxNode,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
    citation_external: bool,
) -> Result<(), CanonicalDocumentRenderError> {
    let label = node
        .children()
        .find(|child| child.kind() == SyntaxKind::InlineParagraph)
        .ok_or_else(|| range_error(node.range()))?;
    let tokens = node.tokens();
    let start = tokens
        .iter()
        .find(|token| {
            token.kind() == SyntaxKind::LeftParen && token.range().start >= label.range().end
        })
        .map(|token| token.range().end)
        .ok_or_else(|| range_error(node.range()))?;
    let end = tokens
        .iter()
        .rev()
        .find(|token| token.kind() == SyntaxKind::RightParen && token.range().start >= start)
        .map(|token| token.range().start)
        .ok_or_else(|| range_error(node.range()))?;
    let href = node
        .source()
        .text(TextRange::new(start, end))
        .map_err(|_| range_error(node.range()))?;
    validate_hyperlink(&href, node.range())?;
    output.push_str("<a class='mech-hyperlink");
    if citation_external {
        output.push_str(" mech-citation-external-link");
    }
    output.push_str("' href='");
    output.push_str(&escape_attribute(&href));
    output.push('\'');
    if citation_external {
        output.push_str(" target='_blank' rel='noopener noreferrer'");
    }
    output.push('>');
    render_inline_html(&label, owner, lookup, output)?;
    output.push_str("</a>");
    Ok(())
}

fn render_citation_paragraph_html(
    paragraph: &ParagraphSyntax,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    output.push_str("<p>");
    render_citation_inline_children(paragraph.syntax(), owner, lookup, output, &[])?;
    output.push_str("</p>");
    Ok(())
}

fn render_citation_inline_html(
    node: &SyntaxNode,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    let wrapper = match node.kind() {
        SyntaxKind::Strong => Some(("strong", "mech-strong", &[SyntaxKind::StrongSigil][..])),
        SyntaxKind::Emphasis => Some(("em", "mech-emphasis", &[SyntaxKind::EmphasisSigil][..])),
        SyntaxKind::Underline => Some((
            "u",
            "mech-underline",
            &[SyntaxKind::UnderlineSigil, SyntaxKind::Underscore][..],
        )),
        SyntaxKind::Strikethrough => Some((
            "del",
            "mech-strikethrough",
            &[SyntaxKind::StrikeSigil, SyntaxKind::Tilde][..],
        )),
        SyntaxKind::Highlight => {
            Some(("mark", "mech-highlight", &[SyntaxKind::HighlightSigil][..]))
        }
        _ => None,
    };
    if let Some((tag, class, delimiters)) = wrapper {
        output.push_str(&format!("<{tag} class='{class}'>"));
        render_citation_inline_children(node, owner, lookup, output, delimiters)?;
        output.push_str(&format!("</{tag}>"));
        return Ok(());
    }
    match node.kind() {
        SyntaxKind::Hyperlink => {
            render_hyperlink_html_with_attributes(node, owner, lookup, output, true)
        }
        SyntaxKind::RawHyperlink => render_raw_hyperlink_html(node, output, true),
        SyntaxKind::Paragraph | SyntaxKind::ParagraphElement | SyntaxKind::InlineParagraph => {
            render_citation_inline_children(node, owner, lookup, output, &[])
        }
        _ => render_inline_html(node, owner, lookup, output),
    }
}

fn render_citation_inline_children(
    node: &SyntaxNode,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
    skipped_tokens: &[SyntaxKind],
) -> Result<(), CanonicalDocumentRenderError> {
    for element in node.children_with_tokens() {
        match element {
            SyntaxElement::Node(child) => {
                render_citation_inline_html(&child, owner, lookup, output)?;
            }
            SyntaxElement::Token(token) if skipped_tokens.contains(&token.kind()) => {}
            SyntaxElement::Token(token) => output.push_str(&escape_html(
                &token.text().map_err(|_| range_error(token.range()))?,
            )),
        }
    }
    Ok(())
}

fn render_delimited_inline_html(
    node: &SyntaxNode,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
    tag: &str,
    class: &str,
    delimiters: &[SyntaxKind],
) -> Result<(), CanonicalDocumentRenderError> {
    output.push_str(&format!("<{tag} class='{class}'>"));
    render_inline_children_html(node, owner, lookup, output, delimiters)?;
    output.push_str(&format!("</{tag}>"));
    Ok(())
}

fn render_reference_html(
    node: &SyntaxNode,
    class: &str,
    target_prefix: &str,
    source_prefix: &str,
    source_suffix: &str,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    let source = node_text(node)?;
    let label = source
        .strip_prefix(source_prefix)
        .and_then(|source| source.strip_suffix(source_suffix))
        .unwrap_or(&source);
    let target = format!("{target_prefix}-{label}");
    output.push_str("<a class='");
    output.push_str(class);
    output.push_str("' href='#");
    output.push_str(&escape_attribute(&target));
    output.push_str("'>");
    output.push_str(&escape_html(&source));
    output.push_str("</a>");
    Ok(())
}

fn render_citation_reference_html(
    node: &SyntaxNode,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    let label = reference_label(node, "[", "]")?;
    let number = lookup.citation_numbers.get(&label);
    output
        .push_str("<span class='mech-reference'>[<a class='mech-reference-link' href='#reference-");
    output.push_str(&escape_attribute(&label));
    output.push_str("'>");
    if let Some(number) = number {
        output.push_str(&number.to_string());
    } else {
        output.push_str(&escape_html(&label));
    }
    output.push_str("</a>]</span>");
    Ok(())
}

fn render_footnote_reference_html(
    node: &SyntaxNode,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    let source = node_text(node)?;
    let label = reference_label(node, "[^", "]")?;
    output.push_str("<a class='mech-footnote-reference' href='#footnote-");
    output.push_str(&escape_attribute(&label));
    output.push_str("'>");
    if let Some(number) = lookup.footnote_numbers.get(&label) {
        output.push_str(&number.to_string());
    } else {
        output.push_str(&escape_html(&source));
    }
    output.push_str("</a>");
    Ok(())
}

fn reference_label(
    node: &SyntaxNode,
    prefix: &str,
    suffix: &str,
) -> Result<String, CanonicalDocumentRenderError> {
    let source = node_text(node)?;
    Ok(source
        .strip_prefix(prefix)
        .and_then(|source| source.strip_suffix(suffix))
        .unwrap_or(&source)
        .to_owned())
}

fn render_raw_hyperlink_html(
    node: &SyntaxNode,
    output: &mut String,
    citation_external: bool,
) -> Result<(), CanonicalDocumentRenderError> {
    let href = node_text(node)?;
    validate_hyperlink(&href, node.range())?;
    output.push_str("<a class='mech-hyperlink");
    if citation_external {
        output.push_str(" mech-citation-external-link");
    }
    output.push_str("' href='");
    output.push_str(&escape_attribute(&href));
    output.push('\'');
    if citation_external {
        output.push_str(" target='_blank' rel='noopener noreferrer'");
    }
    output.push('>');
    output.push_str(&escape_html(&href));
    output.push_str("</a>");
    Ok(())
}

fn render_inline_code_html(
    node: &SyntaxNode,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    let source = node_text(node)?;
    let content = source
        .strip_prefix('`')
        .and_then(|source| source.strip_suffix('`'))
        .unwrap_or(&source)
        .trim();
    output.push_str("<code class='mech-inline-code'>");
    output.push_str(&escape_html(content));
    output.push_str("</code>");
    Ok(())
}

fn render_image_html(
    node: &SyntaxNode,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    let image = retained_image(node)?;
    output.push_str("<figure class='mech-figure'>");
    render_image_tag_html(&image, output);
    if let Some(caption) = image.caption {
        output.push_str("<figcaption class='mech-figure-caption'>");
        render_inline_html(&caption, owner, lookup, output)?;
        output.push_str("</figcaption>");
    }
    output.push_str("</figure>");
    Ok(())
}

struct RetainedImage {
    source: String,
    alt: String,
    caption: Option<SyntaxNode>,
    classes: Vec<String>,
    styles: Vec<(String, String)>,
}

fn retained_image(node: &SyntaxNode) -> Result<RetainedImage, CanonicalDocumentRenderError> {
    let caption = node
        .children()
        .find(|child| child.kind() == SyntaxKind::InlineParagraph);
    let caption_end = caption
        .as_ref()
        .map(|caption| caption.range().end)
        .unwrap_or(node.range().start);
    let tokens = node.tokens();
    let start = tokens
        .iter()
        .find(|token| token.kind() == SyntaxKind::LeftParen && token.range().start >= caption_end)
        .map(|token| token.range().end)
        .ok_or_else(|| range_error(node.range()))?;
    let end = tokens
        .iter()
        .find(|token| token.kind() == SyntaxKind::RightParen && token.range().start >= start)
        .map(|token| token.range().start)
        .ok_or_else(|| range_error(node.range()))?;
    let source = node
        .source()
        .text(TextRange::new(start, end))
        .map_err(|_| range_error(node.range()))?;
    validate_image_source(&source, node.range())?;
    let alt = caption
        .as_ref()
        .map(node_text)
        .transpose()?
        .unwrap_or_default();
    let mut classes = Vec::new();
    let mut styles = Vec::new();
    if let Some(options) = node.children().find_map(OptionMapSyntax::cast) {
        for mapping in options.mappings() {
            let Some(key) = mapping.key().and_then(|key| node_text(key.syntax()).ok()) else {
                continue;
            };
            let Some(value) = mapping.value().and_then(|value| value.decoded_text()) else {
                continue;
            };
            match key.trim().to_ascii_lowercase().as_str() {
                "width" | "height" if safe_css_length(&value) => {
                    styles.push((key.trim().to_ascii_lowercase(), value.trim().to_owned()));
                }
                "align" | "alignment" => match value.trim().to_ascii_lowercase().as_str() {
                    direction @ ("left" | "center" | "right") => {
                        classes.push(format!("mech-image-align-{direction}"));
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
    Ok(RetainedImage {
        source,
        alt,
        caption,
        classes,
        styles,
    })
}

fn render_image_tag_html(image: &RetainedImage, output: &mut String) {
    output.push_str("<img class='mech-image");
    for class in &image.classes {
        output.push(' ');
        output.push_str(class);
    }
    output.push_str("' src='");
    output.push_str(&escape_attribute(&image.source));
    output.push_str("' alt='");
    output.push_str(&escape_attribute(&image.alt));
    if !image.styles.is_empty() {
        output.push_str("' style='");
        for (index, (key, value)) in image.styles.iter().enumerate() {
            if index > 0 {
                output.push_str("; ");
            }
            output.push_str(key);
            output.push_str(": ");
            output.push_str(&escape_attribute(value));
        }
    }
    output.push_str("' />");
}

fn safe_css_length(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    if value == "auto" {
        return true;
    }
    let number_end = value
        .char_indices()
        .take_while(|(_, character)| character.is_ascii_digit() || *character == '.')
        .map(|(index, character)| index + character.len_utf8())
        .last()
        .unwrap_or(0);
    number_end > 0
        && value[..number_end].parse::<f64>().is_ok()
        && matches!(
            &value[number_end..],
            "" | "px" | "%" | "em" | "rem" | "vw" | "vh" | "vmin" | "vmax"
        )
}

fn validate_hyperlink(href: &str, range: TextRange) -> Result<(), CanonicalDocumentRenderError> {
    let trimmed = href.trim();
    let scheme_end = trimmed.find(':').filter(|colon| {
        !trimmed[..*colon]
            .bytes()
            .any(|byte| matches!(byte, b'/' | b'?' | b'#'))
    });
    if let Some(colon) = scheme_end {
        let scheme = &trimmed[..colon];
        if !["http", "https", "mailto", "tel"]
            .iter()
            .any(|allowed| scheme.eq_ignore_ascii_case(allowed))
        {
            return Err(CanonicalDocumentRenderError {
                message: format!("unsafe hyperlink scheme {scheme:?}"),
                range: Some(range),
            });
        }
    }
    Ok(())
}

fn validate_image_source(
    source: &str,
    range: TextRange,
) -> Result<(), CanonicalDocumentRenderError> {
    let trimmed = source.trim();
    let scheme_end = trimmed.find(':').filter(|colon| {
        !trimmed[..*colon]
            .bytes()
            .any(|byte| matches!(byte, b'/' | b'?' | b'#'))
    });
    if let Some(colon) = scheme_end {
        let scheme = &trimmed[..colon];
        if !["http", "https"]
            .iter()
            .any(|allowed| scheme.eq_ignore_ascii_case(allowed))
        {
            return Err(CanonicalDocumentRenderError {
                message: format!("unsafe image source scheme {scheme:?}"),
                range: Some(range),
            });
        }
    }
    Ok(())
}

fn collect_inline_replacements(node: &SyntaxNode, output: &mut Vec<SyntaxNode>) {
    for child in node.children() {
        if EvalInlineMechCodeSyntax::cast(child.clone()).is_some()
            || InlineMechCodeSyntax::cast(child.clone()).is_some()
        {
            output.push(child);
        } else {
            collect_inline_replacements(&child, output);
        }
    }
}

fn render_code_comments(
    code: &SyntaxNode,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
    html: bool,
) -> Result<(), CanonicalDocumentRenderError> {
    if !html && lookup.mode == RenderMode::Source {
        return push_source(code, code.range(), output, false);
    }
    let mut comments = Vec::new();
    let mut pending = vec![code.clone()];
    while let Some(node) = pending.pop() {
        if node.kind() == SyntaxKind::Comment {
            comments.push(node);
        } else {
            pending.extend(node.children());
        }
    }
    comments.sort_by_key(|comment| comment.range().start);
    let mut cursor = code.range().start;
    for comment in comments {
        push_source(
            code,
            TextRange::new(cursor, comment.range().start),
            output,
            html,
        )?;
        if html {
            output.push_str("<span class='mech-comment'>");
        }
        render_inline(&comment, owner, lookup, output, html)?;
        if html {
            output.push_str("</span>");
        }
        cursor = comment.range().end;
    }
    push_source(code, TextRange::new(cursor, code.range().end), output, html)
}

fn render_fence_html(
    fence: &CodeBlockSyntax,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    let info = fence.info().ok_or_else(|| CanonicalDocumentRenderError {
        message: "code fence is missing canonical info".to_owned(),
        range: Some(fence.syntax().range()),
    })?;
    if info.hidden && lookup.mode == RenderMode::Completed {
        return Ok(());
    }
    let presentation = fence
        .presentation()
        .ok_or_else(|| CanonicalDocumentRenderError {
            message: "code fence has no valid presentation options".to_owned(),
            range: Some(fence.syntax().range()),
        })?;
    output.push_str("<figure class='mech-code-block");
    if info.hidden {
        output.push_str(" hidden");
    }
    output.push('\'');
    if !presentation.styles.is_empty() {
        let styles = presentation
            .styles
            .iter()
            .map(|(key, value)| format!("{key}:{value}"))
            .collect::<Vec<_>>()
            .join(";");
        output.push_str(" data-mech-styles='");
        output.push_str(&escape_attribute(&styles));
        output.push('\'');
    }
    output.push_str("><pre><code");
    if matches!(info.scope, CodeFenceScope::Inert)
        && let Some(language) = fence_language(fence)
    {
        output.push_str(" data-language='");
        output.push_str(&escape_attribute(&language));
        output.push('\'');
    }
    output.push('>');
    if let Some(code) = fence
        .mech_code()
        .filter(|_| render_scope(&info.scope).is_some())
    {
        render_code_comments(code.syntax(), owner, lookup, output, true)?;
    } else {
        output.push_str(&escape_html(&fence_body(fence)?));
    }
    output.push_str("</code></pre>");
    let scope = render_scope(&info.scope);
    if lookup.mode == RenderMode::Live
        && !info.hidden
        && presentation.show_output
        && scope.is_some()
    {
        let range = fence.syntax().range();
        if let Some(address) = lookup.output_addresses.get(&range) {
            output.push_str(&format!(
                "<div class='mech-block-output' id='{address}:0'></div>"
            ));
        }
    }
    if lookup.mode == RenderMode::Browser
        && presentation.show_output
        && !info.hidden
        && owner == lookup.root_owner
        && scope.as_ref() == Some(&CanonicalRenderScope::Root)
    {
        append_browser_mount(
            output,
            "figcaption",
            "mech-block-output",
            SourceDocumentOutputKind::Fence,
            fence.syntax().range(),
        );
    }
    if lookup.mode == RenderMode::Completed
        && presentation.show_output
        && fence
            .mech_code()
            .is_some_and(|code| scope_has_compiled_value(code.syntax()))
        && let Some(scope) = scope
    {
        let value = lookup
            .get(
                owner,
                &scope,
                SourceDocumentOutputKind::Fence,
                Some(fence.syntax().range()),
            )
            .ok_or_else(|| CanonicalDocumentRenderError {
                message: "visible executable fence has no completed scope result".to_owned(),
                range: Some(fence.syntax().range()),
            })?;
        output.push_str("<figcaption class='mech-output'>");
        output.push_str(&value.format_html());
        output.push_str("</figcaption>");
    }
    output.push_str("</figure>");
    Ok(())
}

fn fence_language(fence: &CodeBlockSyntax) -> Option<String> {
    let info = fence.syntax().source().text(fence.info_range()?).ok()?;
    let language = info.split_once('{').map_or(info.as_str(), |(info, _)| info);
    language
        .split_whitespace()
        .next()
        .filter(|language| !language.is_empty())
        .map(str::to_owned)
}

fn render_fence_text(
    fence: &CodeBlockSyntax,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    if lookup.mode != RenderMode::Completed {
        output.push_str(&node_text(fence.syntax())?);
        return Ok(());
    }
    let info = fence.info().ok_or_else(|| CanonicalDocumentRenderError {
        message: "code fence is missing canonical info".to_owned(),
        range: Some(fence.syntax().range()),
    })?;
    if info.hidden {
        return Ok(());
    }
    if let Some(code) = fence
        .mech_code()
        .filter(|_| render_scope(&info.scope).is_some())
    {
        render_code_comments(code.syntax(), owner, lookup, output, false)?;
    } else {
        output.push_str(&fence_body(fence)?);
    }
    let presentation = fence
        .presentation()
        .ok_or_else(|| CanonicalDocumentRenderError {
            message: "code fence has no valid presentation options".to_owned(),
            range: Some(fence.syntax().range()),
        })?;
    if lookup.mode == RenderMode::Completed
        && presentation.show_output
        && let Some(scope) = render_scope(&info.scope)
    {
        let value = lookup
            .get(
                owner,
                &scope,
                SourceDocumentOutputKind::Fence,
                Some(fence.syntax().range()),
            )
            .ok_or_else(|| CanonicalDocumentRenderError {
                message: "visible executable fence has no completed scope result".to_owned(),
                range: Some(fence.syntax().range()),
            })?;
        output.push_str("\n=> ");
        output.push_str(&value.format_canonical_inline());
        output.push('\n');
    }
    Ok(())
}

fn render_scope(scope: &CodeFenceScope) -> Option<CanonicalRenderScope> {
    match scope {
        CodeFenceScope::Root => Some(CanonicalRenderScope::Root),
        CodeFenceScope::Named(name) => Some(CanonicalRenderScope::Named(name.clone())),
        CodeFenceScope::Disabled | CodeFenceScope::Inert => None,
    }
}

fn visible_root_program_range(root: &SyntaxNode) -> Option<TextRange> {
    fn retain_latest(latest: &mut Option<(TextRange, bool)>, range: TextRange, visible: bool) {
        if latest
            .as_ref()
            .is_none_or(|(current, _)| current.start < range.start)
        {
            *latest = Some((range, visible));
        }
    }

    fn collect(node: &SyntaxNode, latest: &mut Option<(TextRange, bool)>) {
        if matches!(
            node.kind(),
            SyntaxKind::InlineMechCode | SyntaxKind::MikaSection | SyntaxKind::Comment
        ) {
            return;
        }
        if EvalInlineMechCodeSyntax::cast(node.clone()).is_some() {
            retain_latest(latest, node.range(), false);
            return;
        }
        if let Some(fence) = CodeBlockSyntax::cast(node.clone()) {
            if !matches!(
                fence.info().map(|info| info.scope),
                Some(CodeFenceScope::Root)
            ) {
                return;
            }
            let mut fence_latest = None;
            if let Some(code) = fence.mech_code() {
                collect(code.syntax(), &mut fence_latest);
            }
            if let Some((range, _)) = fence_latest {
                retain_latest(latest, range, false);
            }
            return;
        }
        if matches!(
            node.kind(),
            SyntaxKind::VariableDefine
                | SyntaxKind::Expression
                | SyntaxKind::OpAssign
                | SyntaxKind::VariableAssign
        ) {
            retain_latest(latest, node.range(), true);
            return;
        }
        if matches!(
            node.kind(),
            SyntaxKind::ContextDeclaration
                | SyntaxKind::ExportDeclaration
                | SyntaxKind::ImportDeclaration
                | SyntaxKind::ModuleImport
        ) {
            return;
        }
        for child in node.children() {
            collect(&child, latest);
        }
    }

    let mut latest = None;
    collect(root, &mut latest);
    latest.and_then(|(range, visible)| visible.then_some(range))
}

/// Browser transport identity derived from the canonical output's retained anchor.
/// Program results keep their document-boundary address across REPL appends.
pub fn canonical_document_output_id(kind: SourceDocumentOutputKind, range: TextRange) -> u64 {
    if kind == SourceDocumentOutputKind::Program {
        return mech_core::hash_str("mech/document-program-output/v1");
    }
    mech_core::hash_str(&format!(
        "mech/canonical-document-output/v1:{kind:?}:{}:{}",
        range.start.0, range.end.0
    ))
}

/// Stable browser addresses for source-visible canonical presentation outputs.
///
/// This is the syntax-only half of the browser publication contract. It lets
/// formatting products describe their rendered mounts without enabling the
/// semantic compiler. The compiler independently publishes the same inline and
/// fence owners in source order.
pub fn canonical_document_presentation_output_ids(
    document: &DocumentSyntax,
) -> Result<Vec<u64>, CanonicalDocumentRenderError> {
    let mut outputs = Vec::new();
    collect_root_presentation_outputs(document.syntax(), &mut outputs)?;
    outputs.sort_by_key(|(range, _)| range.start);
    Ok(outputs.into_iter().map(|(_, output)| output).collect())
}

fn collect_root_presentation_outputs(
    node: &SyntaxNode,
    outputs: &mut Vec<(TextRange, u64)>,
) -> Result<(), CanonicalDocumentRenderError> {
    if matches!(
        node.kind(),
        SyntaxKind::InlineMechCode | SyntaxKind::MikaSection
    ) {
        return Ok(());
    }
    if EvalInlineMechCodeSyntax::cast(node.clone()).is_some() {
        let range = node.range();
        outputs.push((
            range,
            canonical_document_output_id(SourceDocumentOutputKind::Inline, range),
        ));
        return Ok(());
    }
    if let Some(fence) = CodeBlockSyntax::cast(node.clone()) {
        let Some(info) = fence.info() else {
            return Err(range_error(fence.syntax().range()));
        };
        if !matches!(info.scope, CodeFenceScope::Root) {
            return Ok(());
        }
        let Some(code) = fence.mech_code() else {
            return Err(range_error(fence.syntax().range()));
        };
        collect_root_presentation_outputs(code.syntax(), outputs)?;
        let presentation = fence
            .presentation()
            .ok_or_else(|| range_error(fence.syntax().range()))?;
        if presentation.show_output && scope_has_compiled_value(code.syntax()) {
            let range = fence.syntax().range();
            outputs.push((
                range,
                canonical_document_output_id(SourceDocumentOutputKind::Fence, range),
            ));
        }
        return Ok(());
    }
    for child in node.children() {
        collect_root_presentation_outputs(&child, outputs)?;
    }
    Ok(())
}

fn scope_has_compiled_value(node: &SyntaxNode) -> bool {
    if matches!(
        node.kind(),
        SyntaxKind::InlineMechCode | SyntaxKind::MikaSection
    ) {
        return false;
    }
    if EvalInlineMechCodeSyntax::cast(node.clone()).is_some()
        || matches!(
            node.kind(),
            SyntaxKind::ActivationScope
                | SyntaxKind::ContextSend
                | SyntaxKind::VariableDefine
                | SyntaxKind::TupleDestructure
                | SyntaxKind::Expression
                | SyntaxKind::OpAssign
                | SyntaxKind::VariableAssign
        )
    {
        return true;
    }
    if matches!(
        node.kind(),
        SyntaxKind::FunctionDefine
            | SyntaxKind::InvariantDefine
            | SyntaxKind::KindDefine
            | SyntaxKind::EnumDefine
            | SyntaxKind::FsmSpecification
            | SyntaxKind::FsmImplementation
            | SyntaxKind::ContextDeclaration
            | SyntaxKind::ImportDeclaration
            | SyntaxKind::ExportDeclaration
            | SyntaxKind::ModuleImport
    ) {
        return false;
    }
    if let Some(fence) = CodeBlockSyntax::cast(node.clone()) {
        return matches!(
            fence.info().map(|info| info.scope),
            Some(CodeFenceScope::Root)
        ) && fence
            .mech_code()
            .is_some_and(|code| scope_has_compiled_value(code.syntax()));
    }
    node.children()
        .any(|child| scope_has_compiled_value(&child))
}

/// Whether the canonical root scope can produce a runtime value.
pub fn canonical_document_has_root_program(document: &DocumentSyntax) -> bool {
    scope_has_compiled_value(document.syntax())
}

fn append_browser_mount(
    output: &mut String,
    tag: &str,
    class: &str,
    kind: SourceDocumentOutputKind,
    range: TextRange,
) {
    let id = canonical_document_output_id(kind, range);
    output.push_str(&format!(
        "<{tag} class='{class}' id='{id}:0' data-mech-output-address='{id}:0'></{tag}>"
    ));
}

fn append_program_html(
    owner: DocumentScopeId,
    scope: &CanonicalRenderScope,
    lookup: &ResultLookup<'_>,
    output: &mut String,
    required: Option<TextRange>,
) -> Result<(), CanonicalDocumentRenderError> {
    if lookup.mode == RenderMode::Browser {
        if owner == lookup.root_owner
            && *scope == CanonicalRenderScope::Root
            && let Some(range) = required
        {
            append_browser_mount(
                output,
                "output",
                "mech-block-output mech-program-output",
                SourceDocumentOutputKind::Program,
                range,
            );
        }
        return Ok(());
    }
    if lookup.mode != RenderMode::Completed {
        return Ok(());
    }
    let value = lookup.get(owner, scope, SourceDocumentOutputKind::Program, None);
    if let Some(range) = required
        && value.is_none()
    {
        return Err(CanonicalDocumentRenderError {
            message: "visible root program has no completed scope result".to_owned(),
            range: Some(range),
        });
    }
    if let Some(value) = value {
        output.push_str("<output class='mech-program-output'>");
        output.push_str(&value.format_html());
        output.push_str("</output>");
    }
    Ok(())
}

fn append_program_text(
    owner: DocumentScopeId,
    scope: &CanonicalRenderScope,
    lookup: &ResultLookup<'_>,
    output: &mut String,
    required: Option<TextRange>,
) -> Result<(), CanonicalDocumentRenderError> {
    if lookup.mode != RenderMode::Completed {
        return Ok(());
    }
    let value = lookup.get(owner, scope, SourceDocumentOutputKind::Program, None);
    if let Some(range) = required
        && value.is_none()
    {
        return Err(CanonicalDocumentRenderError {
            message: "visible root program has no completed scope result".to_owned(),
            range: Some(range),
        });
    }
    if let Some(value) = value {
        output.push_str("\n=> ");
        output.push_str(&value.format_canonical_inline());
        output.push('\n');
    }
    Ok(())
}

fn fence_body(fence: &CodeBlockSyntax) -> Result<String, CanonicalDocumentRenderError> {
    if let Some(code) = fence.mech_code() {
        return node_text(code.syntax());
    }
    let delimiters = fence.delimiters();
    let start = fence
        .syntax()
        .tokens()
        .into_iter()
        .find(|token| token.kind() == mech_syntax::document::SyntaxKind::Newline)
        .map(|token| token.range().end)
        .unwrap_or(fence.syntax().range().start);
    let end = delimiters
        .last()
        .map(|token| token.range().start)
        .unwrap_or(fence.syntax().range().end);
    fence
        .syntax()
        .source()
        .text(TextRange::new(start, end))
        .map_err(|_| range_error(fence.syntax().range()))
}

fn push_source(
    node: &SyntaxNode,
    range: TextRange,
    output: &mut String,
    html: bool,
) -> Result<(), CanonicalDocumentRenderError> {
    let source = node.source().text(range).map_err(|_| range_error(range))?;
    if html {
        output.push_str(&escape_html(&source));
    } else {
        output.push_str(&source);
    }
    Ok(())
}

fn node_text(node: &SyntaxNode) -> Result<String, CanonicalDocumentRenderError> {
    node.text().map_err(|_| range_error(node.range()))
}

fn format_canonical_item(node: &SyntaxNode) -> Result<String, CanonicalDocumentRenderError> {
    let mut protected = Vec::new();
    for kind in [
        SyntaxKind::Comment,
        SyntaxKind::StringLiteral,
        SyntaxKind::Utf8String,
        SyntaxKind::RawString,
    ] {
        collect_nodes(node, kind, &mut protected);
    }
    let protected = protected
        .into_iter()
        .map(|node| node.range())
        .collect::<Vec<_>>();
    let mut output = String::new();
    let mut gap = String::new();
    let mut previous = None;
    for token in node.tokens() {
        let kind = token.kind();
        let text = token.text().map_err(|_| range_error(token.range()))?;
        let literal = protected
            .iter()
            .any(|range| token.range().start >= range.start && token.range().end <= range.end);
        if !literal
            && matches!(
                kind,
                SyntaxKind::Whitespace
                    | SyntaxKind::Tab
                    | SyntaxKind::Newline
                    | SyntaxKind::CarriageReturn
            )
        {
            gap.push_str(&text);
            continue;
        }
        if gap.contains(['\r', '\n']) {
            output.push_str(&gap);
        } else if previous.is_some() {
            if !gap.is_empty()
                || matches!(
                    kind,
                    SyntaxKind::DefineOperatorToken | SyntaxKind::AssignOperator
                )
                || matches!(
                    previous,
                    Some(SyntaxKind::DefineOperatorToken | SyntaxKind::AssignOperator)
                )
            {
                output.push(' ');
            }
        } else {
            output.push_str(&gap);
        }
        gap.clear();
        output.push_str(&text);
        previous = Some(kind);
    }
    output.push_str(&gap);
    Ok(output)
}

fn range_error(range: TextRange) -> CanonicalDocumentRenderError {
    CanonicalDocumentRenderError {
        message: "canonical document source range is unavailable".to_owned(),
        range: Some(range),
    }
}

fn find<T: AstNode>(node: &SyntaxNode) -> Option<T> {
    T::cast(node.clone()).or_else(|| node.children().find_map(|child| find(&child)))
}

fn collect_nodes(node: &SyntaxNode, kind: SyntaxKind, output: &mut Vec<SyntaxNode>) {
    if node.kind() == kind {
        output.push(node.clone());
    }
    for child in node.children() {
        collect_nodes(&child, kind, output);
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attribute(value: &str) -> String {
    escape_html(value)
        .replace('\'', "&#39;")
        .replace('"', "&quot;")
}
