use mech_core::{
    Comment, FencedMechCode, MechCode, Paragraph, ParagraphElement, Program, SectionElement,
    hash_str,
};

/// Returns the stable browser addresses for root-document values that the
/// compiler planner actually evaluates and the HTML formatter exposes.
///
/// The order is the publication contract: `ProgramCompiler` publishes these
/// values first and `WasmDocument` maps each stable source address to the same
/// compact artifact-output ordinal. Keep this traversal aligned with
/// `mechdown::section_element` and the formatter's root presentation namespace.
pub fn root_document_output_ids(program: &Program) -> Vec<u64> {
    let mut output_ids = Vec::new();
    let mut inline_index = 0_u64;
    for section in &program.body.sections {
        for element in &section.elements {
            collect_section_output_ids(element, &mut inline_index, &mut output_ids);
        }
    }
    output_ids
}

/// Counts root-presentation inline evaluations using the same traversal as
/// [`root_document_output_ids`]. Browser hosts use this to append separately
/// formatted document fragments without restarting their address namespace.
pub fn root_document_inline_eval_count(program: &Program) -> u64 {
    let mut output_ids = Vec::new();
    let mut inline_index = 0_u64;
    for section in &program.body.sections {
        for element in &section.elements {
            collect_section_output_ids(element, &mut inline_index, &mut output_ids);
        }
    }
    inline_index
}

fn collect_section_output_ids(
    element: &SectionElement,
    inline_index: &mut u64,
    output_ids: &mut Vec<u64>,
) {
    match element {
        SectionElement::Float((element, _)) => {
            collect_section_output_ids(element, inline_index, output_ids);
        }
        SectionElement::MechCode(code) => {
            collect_code_comments(code, inline_index, output_ids);
        }
        SectionElement::FencedMechCode(block) => {
            collect_fenced_output_ids(block, inline_index, output_ids);
        }
        SectionElement::Comment(comment) => {
            collect_comment_output_ids(comment, inline_index, output_ids);
        }
        SectionElement::Paragraph(paragraph) => {
            collect_paragraph_output_ids(paragraph, inline_index, output_ids);
        }
        SectionElement::Table(table) => {
            for row in &table.rows {
                for cell in row {
                    collect_paragraph_output_ids(cell, inline_index, output_ids);
                }
            }
        }
        SectionElement::FigureTable(table) => {
            for row in &table.rows {
                for figure in row {
                    collect_paragraph_output_ids(&figure.caption, inline_index, output_ids);
                }
            }
        }
        _ => {}
    }
}

fn collect_fenced_output_ids(
    block: &FencedMechCode,
    inline_index: &mut u64,
    output_ids: &mut Vec<u64>,
) {
    if block.config.disabled || block.config.namespace != 0 {
        return;
    }
    collect_code_comments(&block.code, inline_index, output_ids);
    if !block.config.output {
        return;
    }
    if let Some((last_code, _)) = block.code.last() {
        push_unique(output_ids, hash_str(&format!("{last_code:?}")));
    }
}

fn collect_code_comments(
    code: &[(MechCode, Option<Comment>)],
    inline_index: &mut u64,
    output_ids: &mut Vec<u64>,
) {
    for (code, trailing_comment) in code {
        if let MechCode::Comment(comment) = code {
            collect_comment_output_ids(comment, inline_index, output_ids);
        }
        if let Some(comment) = trailing_comment {
            collect_comment_output_ids(comment, inline_index, output_ids);
        }
    }
}

fn collect_comment_output_ids(
    comment: &Comment,
    inline_index: &mut u64,
    output_ids: &mut Vec<u64>,
) {
    collect_paragraph_output_ids(&comment.paragraph, inline_index, output_ids);
}

fn collect_paragraph_output_ids(
    paragraph: &Paragraph,
    inline_index: &mut u64,
    output_ids: &mut Vec<u64>,
) {
    for element in &paragraph.elements {
        if matches!(element, ParagraphElement::EvalInlineMechCode(_)) {
            let output_id = hash_str(&format!("inline-eval:0:{inline_index}"));
            *inline_index += 1;
            push_unique(output_ids, output_id);
        }
    }
}

fn push_unique(output_ids: &mut Vec<u64>, output_id: u64) {
    if !output_ids.contains(&output_id) {
        output_ids.push(output_id);
    }
}
