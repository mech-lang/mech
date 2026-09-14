//! Canonical document rendering from retained syntax and completed scope results.

use std::collections::{HashMap, HashSet};

use mech_engine::{CanonicalSourceProgram, SourceDocumentOutputKind};
use mech_syntax::document::{
    AstNode, CodeBlockSyntax, CodeFenceScope, DocumentScopeId, DocumentSyntax,
    EvalInlineMechCodeSyntax, InlineMechCodeSyntax, MechCodeSyntax, MikaSectionSyntax,
    ParagraphSyntax, SectionElementSyntax, SectionSyntax, SyntaxElement, SyntaxKind, SyntaxNode,
    TextRange, UlSubtitleSyntax,
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
    pub owner: DocumentScopeId,
    pub scope: CanonicalRenderScope,
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
        if identity.document != owner.document {
            return Err(CanonicalDocumentRenderError {
                message: "scope program and presentation owner belong to different documents"
                    .to_owned(),
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
    pub fn render_html(
        &self,
        document: &DocumentSyntax,
        results: &[CanonicalScopeResults],
    ) -> Result<String, CanonicalDocumentRenderError> {
        let lookup = ResultLookup::new(document, results)?;
        let mut output = String::from("<article class='mech-document'>");
        if let Some(title) = document.title() {
            output.push_str("<header class='mech-document-title'><pre>");
            output.push_str(&escape_html(&node_text(title.syntax())?));
            output.push_str("</pre></header>");
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
        output.push_str("</article>");
        Ok(output)
    }

    pub fn render_text(
        &self,
        document: &DocumentSyntax,
        results: &[CanonicalScopeResults],
    ) -> Result<String, CanonicalDocumentRenderError> {
        let lookup = ResultLookup::new(document, results)?;
        let mut output = String::new();
        if let Some(title) = document.title() {
            output.push_str(&node_text(title.syntax())?);
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

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ResultKey {
    owner: DocumentScopeId,
    scope: CanonicalRenderScope,
    kind: SourceDocumentOutputKind,
    range: Option<TextRange>,
}

struct ResultLookup<'a> {
    values: HashMap<ResultKey, &'a RuntimeValueSnapshot>,
}

impl<'a> ResultLookup<'a> {
    fn new(
        document: &DocumentSyntax,
        results: &'a [CanonicalScopeResults],
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
        Ok(Self { values })
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
    } else if matches!(value.kind(), SyntaxKind::Citation | SyntaxKind::Footnote) {
        render_note_definition_html(value, owner, lookup, output)?;
    } else if matches!(
        value.kind(),
        SyntaxKind::Figures | SyntaxKind::FiguresRow | SyntaxKind::FigureItem | SyntaxKind::Float
    ) {
        render_figure_container_html(value, owner, lookup, output)?;
    } else if let Some(code) = MechCodeSyntax::cast(value.clone()) {
        output.push_str("<pre class='mech-code'><code>");
        output.push_str(&escape_html(&node_text(code.syntax())?));
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
            .filter(|section| valid_section_number(section))
            .ok_or_else(|| range_error(node.range()))?;
        (section, 2usize)
    } else {
        let (section, _) = first_line
            .strip_prefix('(')
            .and_then(|line| line.split_once(')'))
            .filter(|(section, _)| valid_section_number(section))
            .ok_or_else(|| range_error(node.range()))?;
        let depth = section.split('.').count();
        (section, if depth < 3 { 3 } else { depth + 1 }.min(6))
    };
    let paragraph = node
        .children()
        .find(|child| child.kind() == SyntaxKind::ParagraphNewline)
        .ok_or_else(|| range_error(node.range()))?;
    output.push_str(&format!("<h{level} class='mech-subtitle' id='section-"));
    output.push_str(&escape_attribute(section));
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
        let child = mika.scope_id();
        if let Some(body) = mika.body() {
            render_section_text(&body, child, lookup, output)?;
        }
        let required = mika
            .body()
            .and_then(|body| visible_root_program_range(body.syntax()));
        append_program_text(child, &CanonicalRenderScope::Root, lookup, output, required)?;
    } else if MechCodeSyntax::cast(value.clone()).is_some()
        || UlSubtitleSyntax::cast(value.clone()).is_some()
    {
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
    let (tag, class, sigil) = match node.kind() {
        SyntaxKind::AbstractEl => ("aside", "mech-abstract", SyntaxKind::AbstractSigil),
        SyntaxKind::QuoteBlock => ("blockquote", "mech-quote-block", SyntaxKind::QuoteSigil),
        SyntaxKind::InfoBlock => ("aside", "mech-info-block", SyntaxKind::InfoSigil),
        SyntaxKind::SuccessBlock => ("aside", "mech-success-block", SyntaxKind::SuccessSigil),
        SyntaxKind::IdeaBlock => ("aside", "mech-idea-block", SyntaxKind::IdeaSigil),
        SyntaxKind::WarningBlock => ("aside", "mech-warning-block", SyntaxKind::WarningSigil),
        SyntaxKind::ErrorBlock => ("aside", "mech-error-block", SyntaxKind::ErrorSigil),
        SyntaxKind::QuestionBlock => ("aside", "mech-question-block", SyntaxKind::QuestionSigil),
        SyntaxKind::Prompt => ("div", "mech-prompt", SyntaxKind::PromptSigil),
        _ => return Err(range_error(node.range())),
    };
    output.push_str(&format!("<{tag} class='{class}'>"));
    render_inline_children_html(node, owner, lookup, output, &[sigil])?;
    output.push_str(&format!("</{tag}>"));
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
            output.push_str(&format!("<{tag} class='{class}'>"));
            for child in node.children() {
                render_list_html(&child, owner, lookup, output)?;
            }
            output.push_str(&format!("</{tag}>"));
        }
        SyntaxKind::OrderedListItem | SyntaxKind::UnorderedListItem | SyntaxKind::CheckListItem => {
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
            for child in node.children() {
                render_list_html(&child, owner, lookup, output)?;
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
            if let Some(header) = header {
                output.push_str("<thead><tr>");
                render_table_cells(&header, "th", owner, lookup, output)?;
                output.push_str("</tr></thead>");
            }
            output.push_str("<tbody>");
            for row in node
                .children()
                .filter(|child| child.kind() == SyntaxKind::MechdownTableRow)
            {
                output.push_str("<tr>");
                render_table_cells(&row, "td", owner, lookup, output)?;
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
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    for cell in row.children().filter(|child| {
        matches!(
            child.kind(),
            SyntaxKind::InlineParagraph | SyntaxKind::EmptyParagraph
        )
    }) {
        output.push_str(&format!("<{tag}>"));
        render_inline_html(&cell, owner, lookup, output)?;
        output.push_str(&format!("</{tag}>"));
    }
    Ok(())
}

fn render_figure_container_html(
    node: &SyntaxNode,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    let class = match node.kind() {
        SyntaxKind::Figures => "mech-figures",
        SyntaxKind::FiguresRow => "mech-figures-row",
        SyntaxKind::FigureItem => "mech-figure-item",
        SyntaxKind::Float => "mech-float",
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

fn render_note_definition_html(
    node: &SyntaxNode,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    let prefix = if node.kind() == SyntaxKind::Footnote {
        "footnote"
    } else {
        "reference"
    };
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
    output.push_str("<aside class='mech-");
    output.push_str(prefix);
    output.push_str("' id='");
    output.push_str(&escape_attribute(&format!("{prefix}-{label}")));
    output.push_str("'>");
    if let Some(paragraph) = find::<ParagraphSyntax>(node) {
        render_paragraph_html(&paragraph, owner, lookup, output)?;
    }
    output.push_str("</aside>");
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
    let mut replacements = Vec::new();
    collect_inline_replacements(node, &mut replacements);
    replacements.sort_by_key(|node| node.range().start);
    let mut cursor = node.range().start;
    for replacement in replacements {
        let range = replacement.range();
        push_source(node, TextRange::new(cursor, range.start), output, false)?;
        if EvalInlineMechCodeSyntax::cast(replacement.clone()).is_some() {
            let value = lookup
                .get(
                    owner,
                    &CanonicalRenderScope::Root,
                    SourceDocumentOutputKind::Inline,
                    Some(range),
                )
                .ok_or_else(|| CanonicalDocumentRenderError {
                    message: "evaluated inline expression has no completed scope result".to_owned(),
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
            let range = node.range();
            let value = lookup
                .get(
                    owner,
                    &CanonicalRenderScope::Root,
                    SourceDocumentOutputKind::Inline,
                    Some(range),
                )
                .ok_or_else(|| CanonicalDocumentRenderError {
                    message: "evaluated inline expression has no completed scope result".to_owned(),
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
        SyntaxKind::RawHyperlink => render_raw_hyperlink_html(node, output),
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
        SyntaxKind::Reference => {
            render_reference_html(node, "mech-reference", "reference", "[", "]", output)
        }
        SyntaxKind::FootnoteReference => render_reference_html(
            node,
            "mech-footnote-reference",
            "footnote",
            "[^",
            "]",
            output,
        ),
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
    output.push_str("<a class='mech-hyperlink' href='");
    output.push_str(&escape_attribute(&href));
    output.push_str("'>");
    render_inline_html(&label, owner, lookup, output)?;
    output.push_str("</a>");
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

fn render_raw_hyperlink_html(
    node: &SyntaxNode,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    let href = node_text(node)?;
    validate_hyperlink(&href, node.range())?;
    output.push_str("<a class='mech-hyperlink' href='");
    output.push_str(&escape_attribute(&href));
    output.push_str("'>");
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

    output.push_str("<figure class='mech-figure'><img class='mech-image' src='");
    output.push_str(&escape_attribute(&source));
    output.push_str("' alt='");
    output.push_str(&escape_attribute(&alt));
    output.push_str("' />");
    if let Some(caption) = caption {
        output.push_str("<figcaption class='mech-figure-caption'>");
        render_inline_html(&caption, owner, lookup, output)?;
        output.push_str("</figcaption>");
    }
    output.push_str("</figure>");
    Ok(())
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
    if info.hidden {
        return Ok(());
    }
    let presentation = fence
        .presentation()
        .ok_or_else(|| CanonicalDocumentRenderError {
            message: "code fence has no valid presentation options".to_owned(),
            range: Some(fence.syntax().range()),
        })?;
    output.push_str("<figure class='mech-code-block'");
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
    output.push_str("><pre><code>");
    output.push_str(&escape_html(&fence_body(fence)?));
    output.push_str("</code></pre>");
    let scope = render_scope(&info.scope);
    if presentation.show_output
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

fn render_fence_text(
    fence: &CodeBlockSyntax,
    owner: DocumentScopeId,
    lookup: &ResultLookup<'_>,
    output: &mut String,
) -> Result<(), CanonicalDocumentRenderError> {
    let info = fence.info().ok_or_else(|| CanonicalDocumentRenderError {
        message: "code fence is missing canonical info".to_owned(),
        range: Some(fence.syntax().range()),
    })?;
    if info.hidden {
        return Ok(());
    }
    output.push_str(&fence_body(fence)?);
    let presentation = fence
        .presentation()
        .ok_or_else(|| CanonicalDocumentRenderError {
            message: "code fence has no valid presentation options".to_owned(),
            range: Some(fence.syntax().range()),
        })?;
    if presentation.show_output
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
            SyntaxKind::InlineMechCode | SyntaxKind::MikaSection
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

fn append_program_html(
    owner: DocumentScopeId,
    scope: &CanonicalRenderScope,
    lookup: &ResultLookup<'_>,
    output: &mut String,
    required: Option<TextRange>,
) -> Result<(), CanonicalDocumentRenderError> {
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

fn range_error(range: TextRange) -> CanonicalDocumentRenderError {
    CanonicalDocumentRenderError {
        message: "canonical document source range is unavailable".to_owned(),
        range: Some(range),
    }
}

fn find<T: AstNode>(node: &SyntaxNode) -> Option<T> {
    T::cast(node.clone()).or_else(|| node.children().find_map(|child| find(&child)))
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
