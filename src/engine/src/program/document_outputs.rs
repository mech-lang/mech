use mech_core::{
    BlockConfig, FencedMechCode, MechCode, Program, SectionAnnotation, SectionElement, Statement,
    hash_str,
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
    let mut ids = Vec::new();
    let mut addresses = mech_core::document_presentation::DocumentPresentationAddresses::default();
    if let Some(title) = &program.title {
        addresses.collect_title_outputs(title, &mut ids);
    }
    for section in &program.body.sections {
        addresses.collect_section_outputs(section, &mut ids);
        if section
            .annotations
            .iter()
            .any(|annotation| annotation.name.as_ref() == PROGRAM_OUTPUT_PUBLICATION_ANNOTATION)
        {
            let id = root_document_program_output_id();
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    ids
}

/// Root inline count uses exactly the same complete presentation traversal.
pub fn root_document_inline_eval_count(program: &Program) -> u64 {
    mech_core::document_presentation::root_document_presentation_addresses(program).1
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

pub(crate) fn code_is_program_value(code: &MechCode) -> bool {
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
                    | Statement::ContextSend(_)
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
    if !element_contains_program_value(element) {
        return None;
    }
    // Non-value declarations and comments execute without replacing `ans`, so
    // retaining the original element avoids inventing split-fence outputs and
    // preserves the formatter's source-visible publication order.
    Some(vec![
        element.clone(),
        SectionElement::FencedMechCode(capture.clone()),
    ])
}

#[cfg(all(test, feature = "source"))]
mod tests {
    use super::*;

    #[test]
    fn effect_only_context_sends_are_not_program_values() {
        let tree = mech_syntax::parse("@view/replace <- scene").unwrap();
        let code = tree
            .body
            .sections
            .iter()
            .flat_map(|section| section.elements.iter())
            .find_map(|element| match element {
                SectionElement::MechCode(code) => code.first().map(|(code, _)| code),
                _ => None,
            })
            .expect("the context send must parse as Mech code");

        assert!(matches!(
            code,
            MechCode::Statement(Statement::ContextSend(_))
        ));
        assert!(!code_is_program_value(code));
    }
}
