use mech_core::{
    BlockConfig, Comment, FencedMechCode, MDList, MechCode, Paragraph, ParagraphElement, Program,
    SectionAnnotation, SectionElement, Statement, Title, TitleField, hash_str,
    inline_document_output_id,
};

/// One stable browser presentation address and its content-derived identity.
/// The semantic identity intentionally omits positional occurrence so a
/// retained browser document can match unchanged duplicate outputs across an
/// accepted source edit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootDocumentOutputIdentity {
    pub output_id: u64,
    pub semantic_id: u64,
}

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
    root_document_output_identities(program)
        .into_iter()
        .map(|identity| identity.output_id)
        .collect()
}

/// Returns root-document presentation identities in canonical publication
/// order, retaining both the public occurrence address and its semantic base.
pub fn root_document_output_identities(program: &Program) -> Vec<RootDocumentOutputIdentity> {
    let mut output_ids = Vec::new();
    let mut inline_count = 0_u64;
    let mut inline_occurrences = Vec::new();
    let mut fence_occurrences = Vec::new();
    if let Some(title) = &program.title {
        collect_title_output_ids(
            title,
            &mut inline_count,
            &mut inline_occurrences,
            &mut fence_occurrences,
            &mut output_ids,
        );
    }
    for section in &program.body.sections {
        for element in &section.elements {
            collect_section_output_ids(
                element,
                &mut inline_count,
                &mut inline_occurrences,
                &mut fence_occurrences,
                &mut output_ids,
            );
        }
        if section
            .annotations
            .iter()
            .any(|annotation| annotation.name.as_ref() == PROGRAM_OUTPUT_PUBLICATION_ANNOTATION)
        {
            let output_id = root_document_program_output_id();
            push_unique(&mut output_ids, output_id, output_id);
        }
    }
    output_ids
}

/// Counts root-presentation inline evaluations using the same traversal as
/// [`root_document_output_ids`]. Browser hosts use this to append separately
/// formatted document fragments without restarting their address namespace.
pub fn root_document_inline_eval_count(program: &Program) -> u64 {
    let mut output_ids = Vec::new();
    let mut inline_count = 0_u64;
    let mut inline_occurrences = Vec::new();
    let mut fence_occurrences = Vec::new();
    if let Some(title) = &program.title {
        collect_title_output_ids(
            title,
            &mut inline_count,
            &mut inline_occurrences,
            &mut fence_occurrences,
            &mut output_ids,
        );
    }
    for section in &program.body.sections {
        for element in &section.elements {
            collect_section_output_ids(
                element,
                &mut inline_count,
                &mut inline_occurrences,
                &mut fence_occurrences,
                &mut output_ids,
            );
        }
    }
    inline_count
}

fn collect_title_output_ids(
    title: &Title,
    inline_count: &mut u64,
    inline_occurrences: &mut Vec<(u64, u64)>,
    fence_occurrences: &mut Vec<(u64, u64)>,
    output_ids: &mut Vec<RootDocumentOutputIdentity>,
) {
    if !title.fields.is_empty() {
        for field in &title.fields {
            match field {
                TitleField::Hero(hero) => collect_section_output_ids(
                    hero,
                    inline_count,
                    inline_occurrences,
                    fence_occurrences,
                    output_ids,
                ),
                TitleField::Author(paragraph)
                | TitleField::Date(paragraph)
                | TitleField::Kicker(paragraph)
                | TitleField::Section(paragraph)
                | TitleField::Summary(paragraph)
                | TitleField::Next(paragraph)
                | TitleField::Previous(paragraph) => collect_paragraph_output_ids(
                    paragraph,
                    inline_count,
                    inline_occurrences,
                    output_ids,
                ),
            }
        }
        return;
    }
    for paragraph in [&title.author, &title.date].into_iter().flatten() {
        collect_paragraph_output_ids(paragraph, inline_count, inline_occurrences, output_ids);
    }
    if let Some(hero) = &title.hero {
        collect_section_output_ids(
            hero,
            inline_count,
            inline_occurrences,
            fence_occurrences,
            output_ids,
        );
    }
    for paragraph in [
        &title.kicker,
        &title.section,
        &title.summary,
        &title.next,
        &title.previous,
    ]
    .into_iter()
    .flatten()
    {
        collect_paragraph_output_ids(paragraph, inline_count, inline_occurrences, output_ids);
    }
}

fn collect_section_output_ids(
    element: &SectionElement,
    inline_count: &mut u64,
    inline_occurrences: &mut Vec<(u64, u64)>,
    fence_occurrences: &mut Vec<(u64, u64)>,
    output_ids: &mut Vec<RootDocumentOutputIdentity>,
) {
    match element {
        SectionElement::Float((element, _)) | SectionElement::Prompt(element) => {
            collect_section_output_ids(
                element,
                inline_count,
                inline_occurrences,
                fence_occurrences,
                output_ids,
            );
        }
        SectionElement::MechCode(code) => {
            collect_code_comments(code, inline_count, inline_occurrences, output_ids);
        }
        SectionElement::FencedMechCode(block) => {
            collect_fenced_output_ids(
                block,
                inline_count,
                inline_occurrences,
                fence_occurrences,
                output_ids,
            );
        }
        SectionElement::Comment(comment) => {
            collect_comment_output_ids(comment, inline_count, inline_occurrences, output_ids);
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
                collect_paragraph_output_ids(
                    paragraph,
                    inline_count,
                    inline_occurrences,
                    output_ids,
                );
            }
        }
        SectionElement::Citation(citation) => {
            collect_paragraph_output_ids(
                &citation.text,
                inline_count,
                inline_occurrences,
                output_ids,
            );
        }
        SectionElement::Paragraph(paragraph) => {
            collect_paragraph_output_ids(paragraph, inline_count, inline_occurrences, output_ids);
        }
        SectionElement::Subtitle(subtitle) => {
            collect_paragraph_output_ids(
                &subtitle.text,
                inline_count,
                inline_occurrences,
                output_ids,
            );
        }
        SectionElement::Image(image) => {
            if let Some(caption) = &image.caption {
                collect_paragraph_output_ids(caption, inline_count, inline_occurrences, output_ids);
            }
        }
        SectionElement::List(list) => {
            collect_list_output_ids(list, inline_count, inline_occurrences, output_ids);
        }
        SectionElement::Table(table) => {
            for cell in &table.header {
                collect_paragraph_output_ids(cell, inline_count, inline_occurrences, output_ids);
            }
            for row in &table.rows {
                for cell in row {
                    collect_paragraph_output_ids(
                        cell,
                        inline_count,
                        inline_occurrences,
                        output_ids,
                    );
                }
            }
        }
        SectionElement::FigureTable(table) => {
            for row in &table.rows {
                for figure in row {
                    collect_paragraph_output_ids(
                        &figure.caption,
                        inline_count,
                        inline_occurrences,
                        output_ids,
                    );
                }
            }
        }
        _ => {}
    }
}

fn collect_fenced_output_ids(
    block: &FencedMechCode,
    inline_count: &mut u64,
    inline_occurrences: &mut Vec<(u64, u64)>,
    fence_occurrences: &mut Vec<(u64, u64)>,
    output_ids: &mut Vec<RootDocumentOutputIdentity>,
) {
    if block.config.disabled || block.config.hidden || block.config.namespace != 0 {
        return;
    }
    collect_code_comments(&block.code, inline_count, inline_occurrences, output_ids);
    // The capture executes beside the source value so it snapshots the right
    // `ans`, but its public ordinal belongs to the original document boundary.
    // The boundary annotation below publishes it after all source-visible
    // inline and fenced outputs in that section.
    if block.config.namespace_str == PROGRAM_OUTPUT_CAPTURE_NAMESPACE {
        return;
    }
    if !block.config.output {
        return;
    }
    if let Some(base_id) = fenced_document_output_id(block) {
        let occurrence = match fence_occurrences
            .iter_mut()
            .find(|(candidate, _)| *candidate == base_id)
        {
            Some((_, count)) => {
                let occurrence = *count;
                *count = count.saturating_add(1);
                occurrence
            }
            None => {
                fence_occurrences.push((base_id, 1));
                0
            }
        };
        output_ids.push(RootDocumentOutputIdentity {
            output_id: fenced_document_output_occurrence_id(block, occurrence).unwrap(),
            semantic_id: base_id,
        });
    }
}

pub(crate) fn fenced_document_output_id(block: &FencedMechCode) -> Option<u64> {
    if block.config.namespace_str == PROGRAM_OUTPUT_CAPTURE_NAMESPACE {
        return Some(root_document_program_output_id());
    }
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

pub(crate) fn fenced_document_output_occurrence_id(
    block: &FencedMechCode,
    occurrence: u64,
) -> Option<u64> {
    let base = fenced_document_output_id(block)?;
    if occurrence == 0 || block.config.namespace_str == PROGRAM_OUTPUT_CAPTURE_NAMESPACE {
        Some(base)
    } else {
        Some(hash_str(&format!(
            "mech/fenced-document-output/{base}/{occurrence}"
        )))
    }
}

fn section_contains_program_value(elements: &[SectionElement]) -> bool {
    elements.iter().any(element_contains_program_value)
}

/// Whether a root document contains an ordinary value eligible for the
/// implicit program-result presentation.
pub fn root_document_has_program_value(program: &Program) -> bool {
    program
        .body
        .sections
        .iter()
        .any(|section| section_contains_program_value(&section.elements))
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
        SectionElement::Float((element, _)) | SectionElement::Prompt(element) => {
            element_contains_program_value(element)
        }
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

fn collect_code_comments(
    code: &[(MechCode, Option<Comment>)],
    inline_count: &mut u64,
    inline_occurrences: &mut Vec<(u64, u64)>,
    output_ids: &mut Vec<RootDocumentOutputIdentity>,
) {
    for (code, trailing_comment) in code {
        if let MechCode::Comment(comment) = code {
            collect_comment_output_ids(comment, inline_count, inline_occurrences, output_ids);
        }
        if let Some(comment) = trailing_comment {
            collect_comment_output_ids(comment, inline_count, inline_occurrences, output_ids);
        }
    }
}

fn collect_comment_output_ids(
    comment: &Comment,
    inline_count: &mut u64,
    inline_occurrences: &mut Vec<(u64, u64)>,
    output_ids: &mut Vec<RootDocumentOutputIdentity>,
) {
    collect_paragraph_output_ids(
        &comment.paragraph,
        inline_count,
        inline_occurrences,
        output_ids,
    );
}

fn collect_paragraph_output_ids(
    paragraph: &Paragraph,
    inline_count: &mut u64,
    inline_occurrences: &mut Vec<(u64, u64)>,
    output_ids: &mut Vec<RootDocumentOutputIdentity>,
) {
    for element in &paragraph.elements {
        collect_paragraph_element_output_ids(element, inline_count, inline_occurrences, output_ids);
    }
}

fn collect_paragraph_element_output_ids(
    element: &ParagraphElement,
    inline_count: &mut u64,
    inline_occurrences: &mut Vec<(u64, u64)>,
    output_ids: &mut Vec<RootDocumentOutputIdentity>,
) {
    match element {
        ParagraphElement::EvalInlineMechCode(expression) => {
            let base = inline_document_output_id(0, expression, 0);
            let occurrence = match inline_occurrences
                .iter_mut()
                .find(|(candidate, _)| *candidate == base)
            {
                Some((_, count)) => {
                    let occurrence = *count;
                    *count = count.saturating_add(1);
                    occurrence
                }
                None => {
                    inline_occurrences.push((base, 1));
                    0
                }
            };
            *inline_count = inline_count.saturating_add(1);
            push_unique(
                output_ids,
                inline_document_output_id(0, expression, occurrence),
                base,
            );
        }
        ParagraphElement::Emphasis(element)
        | ParagraphElement::Highlight(element)
        | ParagraphElement::Strikethrough(element)
        | ParagraphElement::Strong(element)
        | ParagraphElement::Underline(element) => collect_paragraph_element_output_ids(
            element,
            inline_count,
            inline_occurrences,
            output_ids,
        ),
        ParagraphElement::Hyperlink((paragraph, _)) => {
            collect_paragraph_output_ids(paragraph, inline_count, inline_occurrences, output_ids)
        }
        _ => {}
    }
}

fn collect_list_output_ids(
    list: &MDList,
    inline_count: &mut u64,
    inline_occurrences: &mut Vec<(u64, u64)>,
    output_ids: &mut Vec<RootDocumentOutputIdentity>,
) {
    match list {
        MDList::Unordered(items) => {
            for ((_, paragraph), nested) in items {
                collect_paragraph_output_ids(
                    paragraph,
                    inline_count,
                    inline_occurrences,
                    output_ids,
                );
                if let Some(nested) = nested {
                    collect_list_output_ids(nested, inline_count, inline_occurrences, output_ids);
                }
            }
        }
        MDList::Ordered(list) => {
            for ((_, paragraph), nested) in &list.items {
                collect_paragraph_output_ids(
                    paragraph,
                    inline_count,
                    inline_occurrences,
                    output_ids,
                );
                if let Some(nested) = nested {
                    collect_list_output_ids(nested, inline_count, inline_occurrences, output_ids);
                }
            }
        }
        MDList::Check(items) => {
            for ((_, paragraph), nested) in items {
                collect_paragraph_output_ids(
                    paragraph,
                    inline_count,
                    inline_occurrences,
                    output_ids,
                );
                if let Some(nested) = nested {
                    collect_list_output_ids(nested, inline_count, inline_occurrences, output_ids);
                }
            }
        }
    }
}

fn push_unique(output_ids: &mut Vec<RootDocumentOutputIdentity>, output_id: u64, semantic_id: u64) {
    if !output_ids
        .iter()
        .any(|identity| identity.output_id == output_id)
    {
        output_ids.push(RootDocumentOutputIdentity {
            output_id,
            semantic_id,
        });
    }
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

    #[test]
    fn hidden_fences_have_no_presentation_addresses() {
        let tree = mech_syntax::parse("```mech:hidden\n42 -- Result {1 + 1}\n```\n").unwrap();
        assert!(root_document_output_ids(&tree).is_empty());
    }

    #[test]
    fn repeated_fences_receive_distinct_presentation_addresses() {
        let tree = mech_syntax::parse("```mech\n42\n```\n\n```mech\n42\n```\n").unwrap();
        let output_ids = root_document_output_ids(&tree);
        assert_eq!(output_ids.len(), 2);
        assert_ne!(output_ids[0], output_ids[1]);
    }

    #[test]
    fn title_front_matter_uses_the_root_inline_identity_namespace() {
        let tree = mech_syntax::parse(
            "Document\n========\nauthor: Ada {40 + 2}\nhero: ![Result {41 + 1}](hero.svg)\n========\n",
        )
        .unwrap();
        let output_ids = root_document_output_ids(&tree);
        assert_eq!(output_ids.len(), 2);
        assert_eq!(root_document_inline_eval_count(&tree), 2);
    }

    #[test]
    fn title_front_matter_preserves_authored_output_order_and_duplicates() {
        let tree = mech_syntax::parse(
            "Document\n========\ndate: {40 + 2}\nauthor: {41 + 1}\nauthor: {42 + 0}\n========\n",
        )
        .unwrap();
        let output_ids = root_document_output_ids(&tree);
        let expected = [
            "Document\n========\ndate: {40 + 2}\n========\n",
            "Document\n========\nauthor: {41 + 1}\n========\n",
            "Document\n========\nauthor: {42 + 0}\n========\n",
        ]
        .map(|source| {
            let field = mech_syntax::parse(source).unwrap();
            root_document_output_ids(&field)[0]
        });

        assert_eq!(output_ids, expected);
        assert_eq!(root_document_inline_eval_count(&tree), 3);
    }

    #[test]
    fn prompt_wrapped_fences_remain_program_values() {
        let tree = mech_syntax::parse(">: ```mech\n42\n```\n").unwrap();
        assert!(root_document_has_program_value(&tree));
    }

    #[test]
    fn declaration_only_fences_have_no_presentation_address() {
        let tree = mech_syntax::parse("```mech\n#Identity(x) => x\n```\n").unwrap();
        assert!(root_document_output_ids(&tree).is_empty());
    }

    #[test]
    fn fences_with_the_same_final_expression_keep_semantic_identities() {
        let original =
            mech_syntax::parse("```mech\nx := 1\nx\n```\n\n```mech\nx := 2\nx\n```\n").unwrap();
        let inserted = mech_syntax::parse(
            "```mech\nx := 0\nx\n```\n\n```mech\nx := 1\nx\n```\n\n```mech\nx := 2\nx\n```\n",
        )
        .unwrap();
        let original_ids = root_document_output_ids(&original);
        let inserted_ids = root_document_output_ids(&inserted);
        assert_eq!(original_ids.len(), 2);
        assert_eq!(inserted_ids.len(), 3);
        assert!(original_ids.iter().all(|id| inserted_ids.contains(id)));
    }
}
