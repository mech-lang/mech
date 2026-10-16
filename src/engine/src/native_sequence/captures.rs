//! Immutable lexical captures for typed native source functions.
use super::*;

const GLOBAL_PREFIX: &str = "\0global:";
pub(super) fn global_key(name: &str) -> String {
    format!("{GLOBAL_PREFIX}{name}")
}
pub(super) fn lexical_globals(vars: &BTreeMap<String, usize>) -> BTreeMap<String, usize> {
    let mut output = BTreeMap::new();
    for (key, &id) in vars {
        if key.starts_with(GLOBAL_PREFIX) {
            output.insert(key.clone(), id);
        }
    }
    output
}

pub(super) fn free_globals(
    source: &SourceFunction,
    functions: &BTreeMap<String, SourceFunction>,
    globals: &BTreeMap<String, SyntaxNode>,
) -> Result<BTreeSet<String>, String> {
    fn scan(
        source: &SourceFunction,
        functions: &BTreeMap<String, SourceFunction>,
        globals: &BTreeMap<String, SyntaxNode>,
        active: &mut BTreeSet<String>,
        out: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        fn expression(
            node: &SyntaxNode,
            locals: &BTreeSet<String>,
            functions: &BTreeMap<String, SourceFunction>,
            globals: &BTreeMap<String, SyntaxNode>,
            active: &mut BTreeSet<String>,
            out: &mut BTreeSet<String>,
        ) -> Result<(), String> {
            if node.kind() == SyntaxKind::Variable {
                let name = text(node)?.trim().to_string();
                if !locals.contains(&name) {
                    if globals.contains_key(&name) {
                        out.insert(name.clone());
                    }
                    if let Some(callee) = functions.get(&name) {
                        if active.insert(name.clone()) {
                            scan(callee, functions, globals, active, out)?;
                            active.remove(&name);
                        }
                    }
                }
            }
            if let Some(call) = FunctionCallSyntax::cast(node.clone()) {
                let name = text(call.function().ok_or("capture callee")?.syntax())?;
                if let Some(callee) = functions.get(&name) {
                    if active.insert(name.clone()) {
                        scan(callee, functions, globals, active, out)?;
                        active.remove(&name);
                    }
                }
            }
            for child in node.children() {
                expression(&child, locals, functions, globals, active, out)?;
            }
            Ok(())
        }
        let mut locals: BTreeSet<_> = source.inputs.iter().map(|(n, _)| n.clone()).collect();
        for statement in source
            .body
            .children()
            .filter(|n| n.kind() == SyntaxKind::Statement)
        {
            let definition = statement
                .children()
                .find_map(VariableDefineSyntax::cast)
                .ok_or("capture function requires definitions")?;
            expression(
                definition
                    .value()
                    .ok_or("capture definition value")?
                    .syntax(),
                &locals,
                functions,
                globals,
                active,
                out,
            )?;
            locals.insert(text(
                definition.name().ok_or("capture local name")?.syntax(),
            )?);
        }
        Ok(())
    }
    let mut out = BTreeSet::new();
    scan(source, functions, globals, &mut BTreeSet::new(), &mut out)?;
    Ok(out)
}

/// Infer captured global kinds without evaluating their definitions or repeating IO.
pub(super) fn infer_kind(
    node: SyntaxNode,
    globals: &BTreeMap<String, SyntaxNode>,
    functions: &BTreeMap<String, SourceFunction>,
    catalog: Catalog,
    bindings: &ports::SourceBindings,
    active: &mut BTreeSet<String>,
) -> Result<SequenceKind, String> {
    match node.kind() {
        SyntaxKind::Number | SyntaxKind::StringLiteral => return Ok(literal(node)?.kind()),
        SyntaxKind::Variable => {
            let variable = mech_syntax::document::VariableSyntax::cast(node.clone())
                .ok_or("invalid captured variable")?;
            if bindings.read(&variable)?.is_some() {
                return Ok(SequenceKind::Text);
            }
            let name = text(&node)?.trim().to_string();
            let definition = globals
                .get(&name)
                .ok_or_else(|| format!("undefined captured global {name}"))?;
            if !active.insert(name.clone()) {
                return Err("cyclic captured global".into());
            }
            let result = infer_kind(
                definition.clone(),
                globals,
                functions,
                catalog,
                bindings,
                active,
            );
            active.remove(&name);
            return result;
        }
        SyntaxKind::FunctionCall => {
            let call = FunctionCallSyntax::cast(node).ok_or("capture call")?;
            let name = text(call.function().ok_or("capture callee")?.syntax())?;
            let name = bindings.resolve_name(&name).to_string();
            if let Some(function) = functions.get(&name) {
                return Ok(function.output.1);
            }
            let signature = catalog
                .iter()
                .find(|s| s.name == name)
                .ok_or_else(|| format!("unsupported captured operation {name}"))?;
            let arguments = call.arguments().ok_or("capture arguments")?.arguments();
            let input = |index: usize,
                         active: &mut BTreeSet<String>|
             -> Result<SequenceKind, String> {
                let Some(AnyCallArgumentSyntax::Positional(argument)) = arguments.get(index) else {
                    return Err("capture positional argument".into());
                };
                infer_kind(
                    argument.value().ok_or("capture value")?.syntax().clone(),
                    globals,
                    functions,
                    catalog,
                    bindings,
                    active,
                )
            };
            return match signature.output {
                OutputKind::Fixed(kind) => Ok(kind),
                OutputKind::Input(i) => input(i, active),
                OutputKind::Numeric => Ok(if input(0, active)? == SequenceKind::Integer {
                    SequenceKind::Float
                } else {
                    SequenceKind::Numbers
                }),
                OutputKind::Formatted => Ok(
                    if matches!(
                        input(0, active)?,
                        SequenceKind::Float | SequenceKind::Integer
                    ) {
                        SequenceKind::Text
                    } else {
                        SequenceKind::Texts
                    },
                ),
            };
        }
        SyntaxKind::MultiplicativeExpression | SyntaxKind::AdditiveExpression => {
            let children: Vec<_> = node.children().collect();
            if children.len() >= 3 {
                let mut kind = infer_kind(
                    children[0].clone(),
                    globals,
                    functions,
                    catalog,
                    bindings,
                    active,
                )?;
                for pair in children[1..].chunks(2) {
                    let other = infer_kind(
                        pair[1].clone(),
                        globals,
                        functions,
                        catalog,
                        bindings,
                        active,
                    )?;
                    kind = if kind == SequenceKind::Text
                        && other == SequenceKind::Text
                        && text(&pair[0])?.trim() == "+"
                    {
                        SequenceKind::Text
                    } else if kind == SequenceKind::Integer && other == SequenceKind::Integer {
                        SequenceKind::Integer
                    } else if kind == SequenceKind::Float && other == SequenceKind::Float {
                        SequenceKind::Float
                    } else if matches!(kind, SequenceKind::Float | SequenceKind::Numbers)
                        && matches!(other, SequenceKind::Float | SequenceKind::Numbers)
                    {
                        SequenceKind::Numbers
                    } else {
                        return Err("incompatible captured numeric kinds".into());
                    };
                }
                return Ok(kind);
            }
        }
        _ => {}
    }
    let children: Vec<_> = node.children().collect();
    if children.len() != 1 {
        return Err(format!("unsupported capture syntax {:?}", node.kind()));
    }
    infer_kind(
        children[0].clone(),
        globals,
        functions,
        catalog,
        bindings,
        active,
    )
}
