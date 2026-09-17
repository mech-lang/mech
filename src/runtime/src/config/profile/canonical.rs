//! Canonical typed input to the existing restricted configuration compiler.
use mech_core::MResult;
use mech_syntax::document::*;

use super::{
    ConfigExpr, ConfigFunction, ConfigItem, ConfigLet, ConfigProfileOptions,
    ConfigProfileViolation, ConfigProgram,
};

fn missing<T>(value: Option<T>) -> MResult<T> {
    value.ok_or_else(|| ConfigProfileViolation::error("incomplete canonical configuration syntax"))
}
fn text(node: &SyntaxNode) -> MResult<String> {
    node.source()
        .text(node.range())
        .map(|text| text.to_string())
        .map_err(|_| ConfigProfileViolation::error("invalid configuration source range"))
}
fn child<N: AstNode>(node: &SyntaxNode) -> Option<N> {
    node.children().find_map(N::cast)
}
fn descendants(node: &SyntaxNode, kind: SyntaxKind) -> Vec<SyntaxNode> {
    let mut found = Vec::new();
    let mut pending = vec![node.clone()];
    while let Some(node) = pending.pop() {
        if node.kind() == kind {
            found.push(node);
        } else {
            pending.extend(node.children().collect::<Vec<_>>().into_iter().rev());
        }
    }
    found
}

pub(super) fn compile(
    document: &DocumentSyntax,
    options: &ConfigProfileOptions,
) -> MResult<ConfigProgram> {
    let mut items = Vec::new();
    let mut pending = vec![document.syntax().clone()];
    while let Some(node) = pending.pop() {
        match node.kind() {
            SyntaxKind::MikaSection
            | SyntaxKind::Paragraph
            | SyntaxKind::InlineMechCode
            | SyntaxKind::EvalInlineMechCode => continue,
            SyntaxKind::CodeBlock => {
                let fence = missing(CodeBlockSyntax::cast(node))?;
                if let Some(CodeFenceInfo {
                    scope: CodeFenceScope::Named(name),
                    ..
                }) = fence.info()
                    && options.executable_namespaces.contains(&name)
                {
                    if let Some(code) = fence.mech_code() {
                        compile_items(&code, &mut items)?;
                    }
                }
            }
            SyntaxKind::MechCode => {
                compile_items(&missing(MechCodeSyntax::cast(node))?, &mut items)?
            }
            _ => pending.extend(node.children().collect::<Vec<_>>().into_iter().rev()),
        }
    }
    Ok(ConfigProgram { items })
}

fn compile_items(code: &MechCodeSyntax, items: &mut Vec<ConfigItem>) -> MResult<()> {
    for item in code.items() {
        let node = missing(item.value())?;
        match node.kind() {
            SyntaxKind::Comment => {}
            SyntaxKind::Expression => items.push(ConfigItem::Expr(expression(&missing(
                ExpressionSyntax::cast(node),
            )?)?)),
            SyntaxKind::FunctionDefine => items.push(ConfigItem::Function(function(&node)?)),
            SyntaxKind::Statement => items.push(statement(&missing(node.children().next())?)?),
            SyntaxKind::ActivationScope => {
                return Err(ConfigProfileViolation::error(
                    "ActivationScopeBytecodeUnsupported",
                ));
            }
            SyntaxKind::FsmSpecification | SyntaxKind::FsmImplementation => {
                return Err(ConfigProfileViolation::error(
                    "state machines are not allowed in Mech config",
                ));
            }
            SyntaxKind::ModuleImport => {
                return Err(ConfigProfileViolation::error(
                    "module imports are not allowed in Mech config",
                ));
            }
            _ => {
                return Err(ConfigProfileViolation::error(format!(
                    "{:?} is not allowed in Mech config",
                    node.kind()
                )));
            }
        }
    }
    Ok(())
}

fn statement(node: &SyntaxNode) -> MResult<ConfigItem> {
    if let Some(definition) = VariableDefineSyntax::cast(node.clone()) {
        if definition.mutability_marker().is_some() {
            return Err(ConfigProfileViolation::error(
                "mutable bindings are not allowed in Mech config",
            ));
        }
        let variable = missing(definition.variable())?;
        let VariableStemSyntax::Identifier(name) = missing(variable.stem())? else {
            return Err(ConfigProfileViolation::error(
                "addressed resources are not allowed in Mech config",
            ));
        };
        return Ok(ConfigItem::Let(ConfigLet {
            name: text(name.syntax())?,
            expr: expression(&missing(definition.value())?)?,
        }));
    }
    let message = match node.kind() {
        SyntaxKind::ImportDeclaration => "imports are not allowed in Mech config",
        SyntaxKind::ExportDeclaration => "exports are not allowed in Mech config",
        SyntaxKind::ContextDeclaration => "context declarations are not allowed in Mech config",
        SyntaxKind::FsmDeclare => "state machines are not allowed in Mech config",
        SyntaxKind::OpAssign => "op assignment is not allowed in Mech config",
        SyntaxKind::VariableAssign => "assignment is not allowed in Mech config",
        SyntaxKind::ContextSend => "context sends are not allowed in Mech config",
        SyntaxKind::InvariantDefine => "invariants are not allowed in Mech config",
        SyntaxKind::TupleDestructure => "tuple destructuring is not allowed in Mech config",
        SyntaxKind::KindDefine => "kind definitions are not allowed in Mech config v1",
        SyntaxKind::EnumDefine => "enum definitions are not allowed in Mech config v1",
        _ => "this statement is not allowed in Mech config v1",
    };
    Err(ConfigProfileViolation::error(message))
}

fn function(node: &SyntaxNode) -> MResult<ConfigFunction> {
    let arms = descendants(node, SyntaxKind::FunctionMatchArm);
    if !descendants(node, SyntaxKind::Statement).is_empty() || arms.len() != 1 {
        return Err(ConfigProfileViolation::error(
            "pattern-dispatched config helper functions are not supported in config v1",
        ));
    }
    if let Some(pattern) = child::<PatternSyntax>(&arms[0])
        && !matches!(pattern.value(), Some(PatternValueSyntax::Wildcard(_)))
    {
        return Err(ConfigProfileViolation::error(
            "pattern-dispatched config helper functions are not supported in config v1",
        ));
    }
    Ok(ConfigFunction {
        name: text(missing(child::<IdentifierSyntax>(node))?.syntax())?,
        params: descendants(node, SyntaxKind::FunctionArg)
            .iter()
            .map(|arg| text(missing(child::<IdentifierSyntax>(arg))?.syntax()))
            .collect::<MResult<_>>()?,
        body: expression(&missing(child::<ExpressionSyntax>(&arms[0]))?)?,
    })
}

fn expression(expr: &ExpressionSyntax) -> MResult<ConfigExpr> {
    if !expr.match_arms().is_empty() {
        return Err(ConfigProfileViolation::error(
            "match expressions are not supported in Mech config v1",
        ));
    }
    body(&missing(expr.body())?)
}
fn body(value: &ExpressionBodySyntax) -> MResult<ConfigExpr> {
    match value {
        ExpressionBodySyntax::Formula(value) => formula(value),
        ExpressionBodySyntax::Range(_) => Err(ConfigProfileViolation::error(
            "range expressions are not supported in Mech config v1",
        )),
        ExpressionBodySyntax::FsmPipe(_) => Err(ConfigProfileViolation::error(
            "state machines are not allowed in Mech config",
        )),
        _ => Err(ConfigProfileViolation::error(
            "comprehensions are not allowed in Mech config",
        )),
    }
}
fn formula(value: &FormulaSyntax) -> MResult<ConfigExpr> {
    match value {
        FormulaSyntax::Factor(value) => factor(value),
        FormulaSyntax::Additive(chain) => {
            let operands = chain.operands();
            let operators = chain.operators();
            if operands.len() != operators.len() + 1 {
                return missing(None);
            }
            let mut value = formula(&operands[0])?;
            for (operator, rhs) in operators.iter().zip(&operands[1..]) {
                let operation =
                    child::<OperatorSyntax>(operator.syntax()).and_then(|op| op.semantic());
                let rhs = Box::new(formula(rhs)?);
                value = match operation {
                    Some(CanonicalOperator::Add) => ConfigExpr::Add(Box::new(value), rhs),
                    Some(CanonicalOperator::Subtract) => ConfigExpr::Sub(Box::new(value), rhs),
                    _ => return missing(None),
                };
            }
            Ok(value)
        }
        _ => Err(ConfigProfileViolation::error(
            "only add and subtract formulas are supported in Mech config v1",
        )),
    }
}
fn factor(value: &FactorSyntax) -> MResult<ConfigExpr> {
    if value.transpose().is_some() {
        return Err(ConfigProfileViolation::error(
            "transpose expressions are not supported in Mech config v1",
        ));
    }
    match missing(value.value())? {
        FactorValueSyntax::Parenthetical(value) => body(&missing(value.expression())?),
        FactorValueSyntax::Negate(value) => Ok(ConfigExpr::Negate(Box::new(factor(&missing(
            value.operand(),
        )?)?))),
        FactorValueSyntax::Not(value) => Ok(ConfigExpr::Not(Box::new(factor(&missing(
            value.operand(),
        )?)?))),
        FactorValueSyntax::Literal(value) => literal(&value),
        FactorValueSyntax::Structure(value) => structure(&value),
        FactorValueSyntax::Variable(value) => {
            let VariableStemSyntax::Identifier(name) = missing(value.stem())? else {
                return Err(ConfigProfileViolation::error(
                    "addressed resources are not allowed in Mech config",
                ));
            };
            let name = text(name.syntax())?;
            Ok(if name == "null" {
                ConfigExpr::Null
            } else {
                ConfigExpr::Var(name)
            })
        }
        FactorValueSyntax::Call(call) => {
            let mut args = Vec::new();
            for arg in missing(call.arguments())?.arguments() {
                let AnyCallArgumentSyntax::Positional(arg) = arg else {
                    return Err(ConfigProfileViolation::error(
                        "named function-call arguments are not supported in Mech config v1",
                    ));
                };
                args.push(expression(&missing(arg.value())?)?);
            }
            Ok(ConfigExpr::Call {
                name: text(missing(call.function())?.syntax())?,
                args,
            })
        }
        FactorValueSyntax::Slice(_) => Err(ConfigProfileViolation::error(
            "slice expressions are not supported in Mech config v1",
        )),
        FactorValueSyntax::MatrixComprehension(_) => Err(ConfigProfileViolation::error(
            "comprehensions are not allowed in Mech config",
        )),
    }
}
fn literal(value: &LiteralSyntax) -> MResult<ConfigExpr> {
    if value.true_token().is_some() {
        return Ok(ConfigExpr::Bool(true));
    }
    if value.false_token().is_some() {
        return Ok(ConfigExpr::Bool(false));
    }
    match missing(value.value())? {
        LiteralValueSyntax::Empty(_) => Ok(ConfigExpr::Null),
        LiteralValueSyntax::Atom(value) => {
            Ok(ConfigExpr::Atom(text(missing(value.name())?.syntax())?))
        }
        LiteralValueSyntax::String(value) => Ok(ConfigExpr::String(missing(value.decoded_text())?)),
        LiteralValueSyntax::Number(value) => number(&value),
        LiteralValueSyntax::KindAnnotation(_) => Err(ConfigProfileViolation::error(
            "kind literals are not supported in Mech config values",
        )),
    }
}
fn number(value: &NumberSyntax) -> MResult<ConfigExpr> {
    if value.complex().is_some() {
        return Err(ConfigProfileViolation::error(
            "complex numbers are not supported in Mech config",
        ));
    }
    let real = missing(value.real())?;
    let value = missing(real.value())?;
    let mut raw = numeric_spelling(&value)?;
    // The canonical tree retains numeric separator spelling. The restricted
    // value parser consumes its numeric value, independently of that spelling.
    raw.retain(|character| character != '_');
    if real.is_negated() {
        raw.insert(0, '-');
    }
    if matches!(
        value.kind(),
        SyntaxKind::FloatLiteral | SyntaxKind::ScientificLiteral
    ) {
        raw.parse::<f64>()
            .map(ConfigExpr::Float)
            .map_err(|_| ConfigProfileViolation::error(format!("invalid config float `{raw}`")))
    } else {
        raw.trim_start_matches("0d")
            .parse::<i64>()
            .map(ConfigExpr::Integer)
            .map_err(|_| ConfigProfileViolation::error(format!("invalid config integer `{raw}`")))
    }
}
// Read numeric payloads through their typed boundaries: a scientific exponent
// can have a type suffix, and its grammar admits both '+' and '-' before it.
// Those are syntax components, not part of Rust's decimal conversion spelling.
fn numeric_spelling(node: &SyntaxNode) -> MResult<String> {
    if let Some(integer) = IntegerLiteralSyntax::cast(node.clone()) {
        let digits = integer
            .typed()
            .and_then(|value| value.digits())
            .or_else(|| integer.untyped().and_then(|value| value.digits()));
        return text(missing(digits)?.syntax());
    }
    if let Some(scientific) = ScientificLiteralSyntax::cast(node.clone()) {
        let base = numeric_spelling(&missing(scientific.base())?)?;
        let exponent = numeric_spelling(&missing(scientific.exponent())?)?;
        let negative = node.children_with_tokens().iter().any(|element| {
            matches!(element, SyntaxElement::Token(token) if token.kind() == SyntaxKind::Dash)
        });
        return Ok(format!(
            "{base}e{}{exponent}",
            if negative { "-" } else { "" }
        ));
    }
    text(node)
}

fn structure(value: &StructureSyntax) -> MResult<ConfigExpr> {
    match missing(value.value())? {
        StructureValueSyntax::EmptyMap(_) => Ok(ConfigExpr::Map(Vec::new())),
        StructureValueSyntax::EmptySet(_) => Ok(ConfigExpr::List(Vec::new())),
        StructureValueSyntax::Record(record) => record
            .bindings()
            .iter()
            .map(|binding| {
                Ok((
                    text(missing(binding.name())?.syntax())?,
                    expression(&missing(binding.value())?)?,
                ))
            })
            .collect::<MResult<_>>()
            .map(ConfigExpr::Map),
        StructureValueSyntax::Map(map) => map
            .entries()
            .iter()
            .map(|entry| {
                let key = match expression(&missing(entry.key())?)? {
                    ConfigExpr::String(key) | ConfigExpr::Atom(key) => key,
                    ConfigExpr::Integer(key) => key.to_string(),
                    _ => {
                        return Err(ConfigProfileViolation::error(
                            "config map keys must be literal strings, atoms, or integers",
                        ));
                    }
                };
                Ok((key, expression(&missing(entry.value())?)?))
            })
            .collect::<MResult<_>>()
            .map(ConfigExpr::Map),
        StructureValueSyntax::Set(set) => set
            .items()
            .iter()
            .map(expression)
            .collect::<MResult<_>>()
            .map(ConfigExpr::List),
        StructureValueSyntax::Tuple(tuple) => tuple
            .items()
            .iter()
            .map(expression)
            .collect::<MResult<_>>()
            .map(ConfigExpr::List),
        StructureValueSyntax::Matrix(matrix) => matrix
            .rows()
            .iter()
            .flat_map(|row| row.columns())
            .map(|column| expression(&missing(column.value())?))
            .collect::<MResult<_>>()
            .map(ConfigExpr::List),
        _ => Err(ConfigProfileViolation::error(
            "this structure is not allowed in Mech config v1",
        )),
    }
}
