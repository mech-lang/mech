//! Presentation addresses shared by the legacy document adapter and formatter.
//! This is presentation metadata only; it does not compile or execute syntax.

use crate::{
    Comment, Expression, FencedMechCode, MDList, MechCode, ModuleImport, Paragraph,
    ParagraphElement, Program, Section, SectionElement, SourceLocation, Statement, Title,
    TitleField, hash_str, inline_document_output_id,
};
#[cfg(feature = "no_std")]
use alloc::{string::String, vec::Vec};

/// One namespace-aware address sequence for every slot of a document render.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DocumentPresentationAddresses {
    inline_counts: Vec<(u64, u64)>,
    fence_counts: Vec<((u64, u64), u64)>,
    inline_occurrences: Vec<((u64, u64), u64)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentPresentationOutputKind {
    Inline,
    Fence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocumentPresentationOutputIdentity {
    pub output_id: u64,
    pub semantic_id: u64,
    pub kind: DocumentPresentationOutputKind,
    pub source_range: Option<(SourceLocation, SourceLocation)>,
}

impl DocumentPresentationAddresses {
    pub fn inline_expression(&mut self, namespace: u64, expression: &Expression) -> u64 {
        self.inline(namespace);
        let base = inline_document_output_id(namespace, expression, 0);
        let count = match self
            .inline_occurrences
            .iter_mut()
            .find(|(key, _)| *key == (namespace, base))
        {
            Some((_, count)) => count,
            None => {
                self.inline_occurrences.push(((namespace, base), 0));
                &mut self.inline_occurrences.last_mut().unwrap().1
            }
        };
        let id = inline_document_output_id(namespace, expression, *count);
        *count += 1;
        id
    }

    pub fn inline(&mut self, namespace: u64) -> u64 {
        let count = match self
            .inline_counts
            .iter_mut()
            .find(|(owner, _)| *owner == namespace)
        {
            Some((_, count)) => count,
            None => {
                self.inline_counts.push((namespace, 0));
                &mut self.inline_counts.last_mut().unwrap().1
            }
        };
        let id = hash_str(&format!("inline-eval:{namespace}:{count}"));
        *count += 1;
        id
    }

    pub fn set_inline_offset(&mut self, namespace: u64, offset: u64) {
        match self
            .inline_counts
            .iter_mut()
            .find(|(owner, _)| *owner == namespace)
        {
            Some((_, count)) => *count = offset,
            None => self.inline_counts.push((namespace, offset)),
        }
    }

    pub fn inline_count(&self, namespace: u64) -> u64 {
        self.inline_counts
            .iter()
            .find(|(owner, _)| *owner == namespace)
            .map_or(0, |(_, count)| *count)
    }

    /// Preserve the first historical fence address and distinguish repetitions.
    /// The producer and HTML renderer must advance this same occurrence rule.
    pub fn fence(&mut self, block: &FencedMechCode, namespace: u64) -> Option<u64> {
        let base = fenced_document_output_id(block)?;
        let count = match self
            .fence_counts
            .iter_mut()
            .find(|(key, _)| *key == (namespace, base))
        {
            Some((_, count)) => count,
            None => {
                self.fence_counts.push(((namespace, base), 0));
                &mut self.fence_counts.last_mut().unwrap().1
            }
        };
        let id = if *count == 0 {
            base
        } else {
            fenced_document_output_occurrence_id(block, *count)?
        };
        *count += 1;
        Some(id)
    }
}

/// Field order survives the historical grouped Title representation through
/// token locations. Default-location hand-built trees keep their field order.
#[derive(Clone, Copy)]
pub enum TitlePresentationField<'a> {
    Paragraph(&'static str, &'a Paragraph),
    Hero(&'a SectionElement),
    Import(&'a ModuleImport, Option<&'a Comment>),
}

pub fn title_presentation_fields(title: &Title) -> Vec<TitlePresentationField<'_>> {
    let mut fields = Vec::new();
    if !title.fields.is_empty() {
        for field in &title.fields {
            fields.push(match field {
                TitleField::Author(p) => TitlePresentationField::Paragraph("author", p),
                TitleField::Date(p) => TitlePresentationField::Paragraph("date", p),
                TitleField::Kicker(p) => TitlePresentationField::Paragraph("kicker", p),
                TitleField::Section(p) => TitlePresentationField::Paragraph("section", p),
                TitleField::Summary(p) => TitlePresentationField::Paragraph("summary", p),
                TitleField::Next(p) => TitlePresentationField::Paragraph("next", p),
                TitleField::Previous(p) => TitlePresentationField::Paragraph("previous", p),
                TitleField::Hero(p) => TitlePresentationField::Hero(p),
            });
        }
    } else {
        for (name, paragraph) in [("author", &title.author), ("date", &title.date)] {
            if let Some(paragraph) = paragraph {
                fields.push(TitlePresentationField::Paragraph(name, paragraph));
            }
        }
        if let Some(hero) = &title.hero {
            fields.push(TitlePresentationField::Hero(hero));
        }
        for (name, paragraph) in [
            ("kicker", &title.kicker),
            ("section", &title.section),
            ("summary", &title.summary),
            ("next", &title.next),
            ("previous", &title.previous),
        ] {
            if let Some(paragraph) = paragraph {
                fields.push(TitlePresentationField::Paragraph(name, paragraph));
            }
        }
    }
    for (import, comment) in &title.imports {
        fields.push(TitlePresentationField::Import(import, comment.as_ref()));
    }
    fields.sort_by_key(|field| {
        let tokens = match field {
            TitlePresentationField::Paragraph(_, paragraph) => paragraph.tokens(),
            TitlePresentationField::Hero(hero) => hero.tokens(),
            TitlePresentationField::Import(import, _) => import.tokens(),
        };
        tokens
            .iter()
            .map(|token| (token.src_range.start.row, token.src_range.start.col))
            .filter(|(row, _)| *row != 0)
            .min()
            .unwrap_or((0, 0))
    });
    fields
}

impl DocumentPresentationAddresses {
    pub fn collect_title_outputs(
        &mut self,
        title: &Title,
        outputs: &mut Vec<DocumentPresentationOutputIdentity>,
    ) {
        for field in title_presentation_fields(title) {
            match field {
                TitlePresentationField::Paragraph(_, paragraph) => {
                    collect_paragraph_output_ids(paragraph, self, outputs)
                }
                TitlePresentationField::Hero(hero) => {
                    collect_section_output_ids(hero, self, outputs)
                }
                // Import comments are authoring metadata, with no rendered slot.
                TitlePresentationField::Import(_, _) => {}
            }
        }
    }

    pub fn collect_section_outputs(
        &mut self,
        section: &Section,
        outputs: &mut Vec<DocumentPresentationOutputIdentity>,
    ) {
        if let Some(subtitle) = &section.subtitle {
            collect_paragraph_output_ids(&subtitle.text, self, outputs);
        }
        for element in &section.elements {
            collect_section_output_ids(element, self, outputs);
        }
    }
}

pub fn root_document_presentation_identities(
    program: &Program,
) -> (Vec<DocumentPresentationOutputIdentity>, u64) {
    let mut outputs = Vec::new();
    let mut addresses = DocumentPresentationAddresses::default();
    if let Some(title) = &program.title {
        addresses.collect_title_outputs(title, &mut outputs);
    }
    for section in &program.body.sections {
        addresses.collect_section_outputs(section, &mut outputs);
    }
    (outputs, addresses.inline_count(0))
}

fn collect_section_output_ids(
    element: &SectionElement,
    addresses: &mut DocumentPresentationAddresses,
    output_ids: &mut Vec<DocumentPresentationOutputIdentity>,
) {
    match element {
        SectionElement::Float((element, _)) | SectionElement::Prompt(element) => {
            collect_section_output_ids(element, addresses, output_ids);
        }
        SectionElement::MechCode(code) => {
            collect_code_comments(code, addresses, output_ids);
        }
        SectionElement::FencedMechCode(block) => {
            collect_fenced_output_ids(block, addresses, output_ids);
        }
        SectionElement::Comment(comment) => {
            collect_comment_output_ids(comment, addresses, output_ids);
        }
        SectionElement::Abstract(paragraphs)
        | SectionElement::QuoteBlock(paragraphs)
        | SectionElement::InfoBlock(paragraphs)
        | SectionElement::SuccessBlock(paragraphs)
        | SectionElement::IdeaBlock(paragraphs)
        | SectionElement::WarningBlock(paragraphs)
        | SectionElement::ErrorBlock(paragraphs)
        | SectionElement::QuestionBlock(paragraphs)
        | SectionElement::Footnote((_, paragraphs)) => {
            for paragraph in paragraphs {
                collect_paragraph_output_ids(paragraph, addresses, output_ids);
            }
        }
        SectionElement::Citation(citation) => {
            collect_paragraph_output_ids(&citation.text, addresses, output_ids);
        }
        SectionElement::Paragraph(paragraph) => {
            collect_paragraph_output_ids(paragraph, addresses, output_ids);
        }
        SectionElement::Subtitle(subtitle) => {
            collect_paragraph_output_ids(&subtitle.text, addresses, output_ids);
        }
        SectionElement::Image(image) => {
            if let Some(caption) = &image.caption {
                collect_paragraph_output_ids(caption, addresses, output_ids);
            }
        }
        SectionElement::List(list) => {
            collect_list_output_ids(list, addresses, output_ids);
        }
        SectionElement::Table(table) => {
            for cell in &table.header {
                collect_paragraph_output_ids(cell, addresses, output_ids);
            }
            for row in &table.rows {
                for cell in row {
                    collect_paragraph_output_ids(cell, addresses, output_ids);
                }
            }
        }
        SectionElement::FigureTable(table) => {
            for row in &table.rows {
                for figure in row {
                    collect_paragraph_output_ids(&figure.caption, addresses, output_ids);
                }
            }
        }
        _ => {}
    }
}

fn collect_fenced_output_ids(
    block: &FencedMechCode,
    addresses: &mut DocumentPresentationAddresses,
    output_ids: &mut Vec<DocumentPresentationOutputIdentity>,
) {
    if block.config.disabled || block.config.namespace != 0 {
        return;
    }
    // A fence publishes its block result; comment expressions are presentation-only.
    if block.config.hidden || !block.config.output {
        return;
    }
    if let Some(output_id) = addresses.fence(block, 0) {
        output_ids.push(DocumentPresentationOutputIdentity {
            output_id,
            semantic_id: fenced_document_output_id(block).unwrap(),
            kind: DocumentPresentationOutputKind::Fence,
            source_range: None,
        });
    }
}

fn collect_code_comments(
    code: &[(MechCode, Option<Comment>)],
    addresses: &mut DocumentPresentationAddresses,
    output_ids: &mut Vec<DocumentPresentationOutputIdentity>,
) {
    for (code, trailing_comment) in code {
        if let MechCode::Comment(comment) = code {
            collect_comment_output_ids(comment, addresses, output_ids);
        }
        if let Some(comment) = trailing_comment {
            collect_comment_output_ids(comment, addresses, output_ids);
        }
    }
}

fn collect_comment_output_ids(
    comment: &Comment,
    addresses: &mut DocumentPresentationAddresses,
    output_ids: &mut Vec<DocumentPresentationOutputIdentity>,
) {
    collect_paragraph_output_ids(&comment.paragraph, addresses, output_ids);
}

fn collect_paragraph_output_ids(
    paragraph: &Paragraph,
    addresses: &mut DocumentPresentationAddresses,
    output_ids: &mut Vec<DocumentPresentationOutputIdentity>,
) {
    for element in &paragraph.elements {
        collect_paragraph_element_output_ids(element, addresses, output_ids);
    }
}

fn collect_paragraph_element_output_ids(
    element: &ParagraphElement,
    addresses: &mut DocumentPresentationAddresses,
    output_ids: &mut Vec<DocumentPresentationOutputIdentity>,
) {
    match element {
        ParagraphElement::EvalInlineMechCode(expression) => {
            let tokens = expression.tokens();
            output_ids.push(DocumentPresentationOutputIdentity {
                output_id: addresses.inline_expression(0, expression),
                semantic_id: inline_document_output_id(0, expression, 0),
                kind: DocumentPresentationOutputKind::Inline,
                source_range: tokens
                    .first()
                    .zip(tokens.last())
                    .map(|(first, last)| (first.src_range.start, last.src_range.end)),
            });
        }
        ParagraphElement::Emphasis(element)
        | ParagraphElement::Highlight(element)
        | ParagraphElement::Strikethrough(element)
        | ParagraphElement::Strong(element)
        | ParagraphElement::Underline(element) => {
            collect_paragraph_element_output_ids(element, addresses, output_ids);
        }
        ParagraphElement::Hyperlink((paragraph, _)) => {
            collect_paragraph_output_ids(paragraph, addresses, output_ids);
        }
        _ => {}
    }
}

fn collect_list_output_ids(
    list: &MDList,
    addresses: &mut DocumentPresentationAddresses,
    output_ids: &mut Vec<DocumentPresentationOutputIdentity>,
) {
    match list {
        MDList::Unordered(items) => {
            for ((_, paragraph), nested) in items {
                collect_paragraph_output_ids(paragraph, addresses, output_ids);
                if let Some(nested) = nested {
                    collect_list_output_ids(nested, addresses, output_ids);
                }
            }
        }
        MDList::Ordered(list) => {
            for ((_, paragraph), nested) in &list.items {
                collect_paragraph_output_ids(paragraph, addresses, output_ids);
                if let Some(nested) = nested {
                    collect_list_output_ids(nested, addresses, output_ids);
                }
            }
        }
        MDList::Check(items) => {
            for ((_, paragraph), nested) in items {
                collect_paragraph_output_ids(paragraph, addresses, output_ids);
                if let Some(nested) = nested {
                    collect_list_output_ids(nested, addresses, output_ids);
                }
            }
        }
    }
}

pub fn root_document_presentation_addresses(program: &Program) -> (Vec<u64>, u64) {
    let (identities, count) = root_document_presentation_identities(program);
    (
        identities
            .into_iter()
            .map(|identity| identity.output_id)
            .collect(),
        count,
    )
}

pub fn fenced_document_output_id(block: &FencedMechCode) -> Option<u64> {
    if !block
        .code
        .iter()
        .any(|(code, _)| code_produces_fence_result(code))
    {
        return None;
    }
    let mut identity = String::from("mech/fenced-document-output/v2");
    for (code, _) in &block.code {
        identity.push_str(match code {
            MechCode::Comment(_) => "/comment",
            MechCode::ActivationScope(_) => "/activation",
            MechCode::Expression(_) => "/expression",
            MechCode::FsmImplementation(_) => "/fsm-implementation",
            MechCode::FsmSpecification(_) => "/fsm-specification",
            MechCode::FunctionDefine(_) => "/function",
            MechCode::Import(_) => "/import",
            MechCode::Statement(_) => "/statement",
            MechCode::Error(_, _) => "/error",
        });
        for token in code.tokens() {
            identity.push('/');
            identity.push_str(&format!("{:?}:{}", token.kind, token.to_string()));
        }
    }
    Some(hash_str(&identity))
}
pub fn fenced_document_output_occurrence_id(
    block: &FencedMechCode,
    occurrence: u64,
) -> Option<u64> {
    let base = fenced_document_output_id(block)?;
    if occurrence == 0 {
        Some(base)
    } else {
        Some(hash_str(&format!(
            "mech/fenced-document-output/{base}/{occurrence}"
        )))
    }
}

fn code_produces_fence_result(code: &MechCode) -> bool {
    match code {
        MechCode::ActivationScope(_) | MechCode::Expression(_) => true,
        MechCode::Statement(statement) => matches!(
            statement,
            Statement::OpAssign(_)
                | Statement::VariableAssign(_)
                | Statement::VariableDefine(_)
                | Statement::ContextSend(_)
                | Statement::TupleDestructure(_)
        ),
        _ => false,
    }
}
