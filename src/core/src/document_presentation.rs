//! Presentation addresses shared by the legacy document adapter and formatter.
//! This is presentation metadata only; it does not compile or execute syntax.

use crate::{
    Comment, FencedMechCode, MDList, MechCode, ModuleImport, Paragraph, ParagraphElement, Program,
    Section, SectionElement, Title, hash_str,
};
#[cfg(feature = "no_std")]
use alloc::vec::Vec;

/// One namespace-aware address sequence for every slot of a document render.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DocumentPresentationAddresses {
    inline_counts: Vec<(u64, u64)>,
    fence_counts: Vec<((u64, u64), u64)>,
}

impl DocumentPresentationAddresses {
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
        let (last, _) = block.code.last()?;
        let base = hash_str(&format!("{last:?}"));
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
            hash_str(&format!("fence-output:{namespace}:{base}:{count}"))
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
    pub fn collect_title_outputs(&mut self, title: &Title, outputs: &mut Vec<u64>) {
        for field in title_presentation_fields(title) {
            match field {
                TitlePresentationField::Paragraph(_, paragraph) => {
                    collect_paragraph_output_ids(paragraph, self, outputs)
                }
                TitlePresentationField::Hero(hero) => {
                    collect_section_output_ids(hero, self, outputs)
                }
                TitlePresentationField::Import(_, Some(comment)) => {
                    collect_comment_output_ids(comment, self, outputs)
                }
                TitlePresentationField::Import(_, None) => {}
            }
        }
    }

    pub fn collect_section_outputs(&mut self, section: &Section, outputs: &mut Vec<u64>) {
        if let Some(subtitle) = &section.subtitle {
            collect_paragraph_output_ids(&subtitle.text, self, outputs);
        }
        for element in &section.elements {
            collect_section_output_ids(element, self, outputs);
        }
    }
}

pub fn root_document_presentation_addresses(program: &Program) -> (Vec<u64>, u64) {
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
    output_ids: &mut Vec<u64>,
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
    output_ids: &mut Vec<u64>,
) {
    if block.config.disabled || block.config.namespace != 0 {
        return;
    }
    collect_code_comments(&block.code, addresses, output_ids);
    if block.config.hidden || !block.config.output {
        return;
    }
    if let Some(output_id) = addresses.fence(block, 0) {
        output_ids.push(output_id);
    }
}

fn collect_code_comments(
    code: &[(MechCode, Option<Comment>)],
    addresses: &mut DocumentPresentationAddresses,
    output_ids: &mut Vec<u64>,
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
    output_ids: &mut Vec<u64>,
) {
    collect_paragraph_output_ids(&comment.paragraph, addresses, output_ids);
}

fn collect_paragraph_output_ids(
    paragraph: &Paragraph,
    addresses: &mut DocumentPresentationAddresses,
    output_ids: &mut Vec<u64>,
) {
    for element in &paragraph.elements {
        collect_paragraph_element_output_ids(element, addresses, output_ids);
    }
}

fn collect_paragraph_element_output_ids(
    element: &ParagraphElement,
    addresses: &mut DocumentPresentationAddresses,
    output_ids: &mut Vec<u64>,
) {
    match element {
        ParagraphElement::EvalInlineMechCode(_) => {
            output_ids.push(addresses.inline(0));
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
    output_ids: &mut Vec<u64>,
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
