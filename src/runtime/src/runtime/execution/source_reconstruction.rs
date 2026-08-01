use crate::resolver::SourceScope;
use crate::runtime::RuntimeInvalidOperationError;
use mech_core::{MResult, MechError, MechSourceCode};

pub(super) fn module_source_for_scope(
    source: &MechSourceCode,
    scope: &SourceScope,
) -> MResult<MechSourceCode> {
    match scope {
        SourceScope::Program => Ok(source.clone()),
        SourceScope::Interpreter(interpreter) => {
            let MechSourceCode::String(source_text) = source else {
                return Err(MechError::new(
                    RuntimeInvalidOperationError {
                        operation: "run_module_scope",
                        reason: "interpreter scope execution requires string source".to_string(),
                    },
                    None,
                ));
            };

            let tree = mech_syntax::parser::parse(source_text.trim())?;

            if let Some(source) = source_from_parsed_fenced_blocks(&tree, interpreter.namespace)? {
                return Ok(MechSourceCode::String(source));
            }

            Err(MechError::new(
                RuntimeInvalidOperationError {
                    operation: "run_module_scope",
                    reason: format!(
                        "interpreter scope `{}` not found",
                        interpreter.namespace_str
                    ),
                },
                None,
            ))
        }
    }
}

pub(super) fn source_from_parsed_fenced_blocks(
    tree: &mech_core::Program,
    namespace: u64,
) -> MResult<Option<String>> {
    let mut blocks = Vec::new();

    for section in &tree.body.sections {
        for element in &section.elements {
            if let mech_core::SectionElement::FencedMechCode(fenced) = element {
                if fenced.config.namespace == namespace {
                    let block = source_from_parsed_fenced_code(fenced)?;
                    blocks.push(block.trim_end().to_string());
                }
            }
        }
    }

    if blocks.is_empty() {
        Ok(None)
    } else {
        Ok(Some(blocks.join("\n")))
    }
}

pub(super) fn source_from_parsed_fenced_code(
    fenced: &mech_core::FencedMechCode,
) -> MResult<String> {
    source_from_tokens(std::slice::from_ref(&fenced.source))
}

pub(super) fn source_from_tokens(tokens: &[mech_core::Token]) -> MResult<String> {
    if tokens.is_empty() {
        return Ok(String::new());
    }

    if tokens
        .iter()
        .any(|token| token.src_range.start.row == 0 || token.src_range.start.col == 0)
    {
        return Ok(tokens
            .iter()
            .map(|token| token.to_string())
            .collect::<Vec<_>>()
            .join(" "));
    }

    let mut source = String::new();
    let mut row = tokens[0].src_range.start.row;
    let mut col = tokens[0].src_range.start.col;

    for token in tokens {
        let start = &token.src_range.start;
        while row < start.row {
            source.push('\n');
            row += 1;
            col = 1;
        }
        while col < start.col {
            source.push(' ');
            col += 1;
        }

        let token_text = token.to_string();
        for ch in token_text.chars() {
            source.push(ch);
            if ch == '\n' {
                row += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
    }

    Ok(source)
}
