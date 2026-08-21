use mech_core::{
    BlockConfig, Comment, FencedMechCode, MechCode, Paragraph, ParagraphElement, Program,
    SectionAnnotation, SectionElement, Statement, hash_str,
};

/// Runtime-only namespace used by the browser document adapter to capture the
/// last ordinary source result before interactive console overlays begin.
const PROGRAM_OUTPUT_CAPTURE_NAMESPACE: &str = "\0mech/document-program-output/capture";

/// Runtime-only section annotation that publishes the captured value at the
/// original document boundary. Console overlays are appended after this
/// boundary, so their outputs can never renumber or replace it.
pub(crate) const PROGRAM_OUTPUT_PUBLICATION_ANNOTATION: &str =
    "\0mech/document-program-output/publish";

/// Stable semantic address for the document's implicit program result.
pub fn root_document_program_output_id() -> u64 {
    hash_str("mech/document-program-output/v1")
}

/// Mark a parsed `ans` fence as the runtime-only program-result capture.
///
/// The caller supplies parsed syntax so the engine does not depend on the
/// source parser. This fence is never present in the retained source tree or
/// HTML; it exists only in the candidate tree compiled for a document REPL.
pub fn configure_root_document_program_output_capture(block: &mut FencedMechCode) {
    block.config = BlockConfig {
        namespace_str: PROGRAM_OUTPUT_CAPTURE_NAMESPACE.to_string(),
        namespace: 0,
        disabled: false,
        hidden: true,
        output: false,
    };
}

/// Insert a runtime-only capture immediately after the last ordinary Mech
/// value in this document and publish it at the end of the document boundary.
/// Integrity constraints and declarations are deliberately not display
/// candidates. The returned marker is stable while later REPL sections are
/// appended to the same resident program.
pub fn insert_root_document_program_output_capture(
    program: &mut Program,
    mut capture: FencedMechCode,
) -> bool {
    configure_root_document_program_output_capture(&mut capture);
    let Some(section_index) = program
        .body
        .sections
        .iter()
        .rposition(|section| section_contains_program_value(&section.elements))
    else {
        return false;
    };
    if !insert_capture_after_last_program_value(
        &mut program.body.sections[section_index].elements,
        &capture,
    ) {
        return false;
    }
    let Some(boundary) = program.body.sections.last_mut() else {
        return false;
    };
    if !boundary
        .annotations
        .iter()
        .any(|annotation| annotation.name.as_ref() == PROGRAM_OUTPUT_PUBLICATION_ANNOTATION)
    {
        boundary.annotations.push(SectionAnnotation {
            name: PROGRAM_OUTPUT_PUBLICATION_ANNOTATION.into(),
            arguments: Box::new([]),
        });
    }
    true
}

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
        if section
            .annotations
            .iter()
            .any(|annotation| annotation.name.as_ref() == PROGRAM_OUTPUT_PUBLICATION_ANNOTATION)
        {
            push_unique(&mut output_ids, root_document_program_output_id());
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
    if let Some(output_id) = fenced_document_output_id(block) {
        push_unique(output_ids, output_id);
    }
}

pub(crate) fn fenced_document_output_id(block: &FencedMechCode) -> Option<u64> {
    if block.config.namespace_str == PROGRAM_OUTPUT_CAPTURE_NAMESPACE {
        return Some(root_document_program_output_id());
    }
    block
        .code
        .last()
        .map(|(last_code, _)| hash_str(&format!("{last_code:?}")))
}

fn section_contains_program_value(elements: &[SectionElement]) -> bool {
    elements.iter().any(element_contains_program_value)
}

fn element_contains_program_value(element: &SectionElement) -> bool {
    match element {
        SectionElement::MechCode(code) => code.iter().any(|(code, _)| code_is_program_value(code)),
        SectionElement::FencedMechCode(block) => {
            !block.config.disabled
                && block
                    .code
                    .iter()
                    .any(|(code, _)| code_is_program_value(code))
        }
        SectionElement::Float((element, _)) => element_contains_program_value(element),
        _ => false,
    }
}

fn code_is_program_value(code: &MechCode) -> bool {
    match code {
        MechCode::Expression(_) => true,
        MechCode::Statement(statement) => {
            #[cfg(feature = "invariant_define")]
            if matches!(statement, Statement::InvariantDefine(_)) {
                return false;
            }
            !matches!(
                statement,
                Statement::ImportDeclaration(_)
                    | Statement::ExportDeclaration(_)
                    | Statement::ContextDeclaration(_)
                    | Statement::EnumDefine(_)
                    | Statement::FsmDeclare(_)
                    | Statement::KindDefine(_)
                    | Statement::SplitTable
                    | Statement::FlattenTable
            )
        }
        MechCode::Comment(_)
        | MechCode::ActivationScope(_)
        | MechCode::FsmImplementation(_)
        | MechCode::FsmSpecification(_)
        | MechCode::FunctionDefine(_)
        | MechCode::Import(_)
        | MechCode::Error(_, _) => false,
    }
}

fn insert_capture_after_last_program_value(
    elements: &mut Vec<SectionElement>,
    capture: &FencedMechCode,
) -> bool {
    for element_index in (0..elements.len()).rev() {
        let Some(replacement) =
            split_element_at_last_program_value(&elements[element_index], capture)
        else {
            continue;
        };
        elements.splice(element_index..=element_index, replacement);
        return true;
    }
    false
}

fn split_element_at_last_program_value(
    element: &SectionElement,
    capture: &FencedMechCode,
) -> Option<Vec<SectionElement>> {
    match element {
        SectionElement::MechCode(code) => {
            let target = code
                .iter()
                .rposition(|(code, _)| code_is_program_value(code))?;
            let mut replacement = vec![SectionElement::MechCode(code[..=target].to_vec())];
            replacement.push(SectionElement::FencedMechCode(capture.clone()));
            if target + 1 < code.len() {
                replacement.push(SectionElement::MechCode(code[target + 1..].to_vec()));
            }
            Some(replacement)
        }
        SectionElement::FencedMechCode(block) if !block.config.disabled => {
            let target = block
                .code
                .iter()
                .rposition(|(code, _)| code_is_program_value(code))?;
            let mut left = block.clone();
            left.code = block.code[..=target].to_vec();
            let mut replacement = vec![SectionElement::FencedMechCode(left)];
            replacement.push(SectionElement::FencedMechCode(capture.clone()));
            if target + 1 < block.code.len() {
                let mut right = block.clone();
                right.code = block.code[target + 1..].to_vec();
                right.imports.clear();
                right.exports.clear();
                replacement.push(SectionElement::FencedMechCode(right));
            }
            Some(replacement)
        }
        SectionElement::Float((inner, direction)) => {
            let split = split_element_at_last_program_value(inner, capture)?;
            Some(
                split
                    .into_iter()
                    .map(|element| {
                        if matches!(
                            &element,
                            SectionElement::FencedMechCode(block)
                                if block.config.namespace_str == PROGRAM_OUTPUT_CAPTURE_NAMESPACE
                        ) {
                            element
                        } else {
                            SectionElement::Float((Box::new(element), direction.clone()))
                        }
                    })
                    .collect(),
            )
        }
        _ => None,
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
