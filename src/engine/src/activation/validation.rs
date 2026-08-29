use crate::{
    ActivationArm, ActivationArmBody, ActivationScope, CompiledPattern, ComprehensionQualifier,
    DimensionExpr, Expression, Factor, FunctionDefinition, FunctionResolver, GuardFunctionSafety,
    Interpreter, MResult, MechCode, MechError, MechErrorKind, Pattern, RangeExpression,
    ReactiveCellId, ResolvedNamedFunction, SchemaBody, Slice, Statement, Structure, Subscript,
    Token, ValueCell, compile_pattern,
};
use std::collections::HashSet;

use super::{
    ActivationPatternArmsNonExhaustive, ActivationPatternCapture,
    ActivationPatternCaptureKindUnsupported, ActivationPatternContextEffectUnsupported,
    ActivationPatternDefinitionUnsupported, ActivationPatternGuardMustBePure,
    ActivationPatternTriggerInvariant, ActivationPatternWildcardMustBeLast,
    arms::{PreflightActivationArm, PreflightPatternedActivation},
    create_capture_slot_for_schema,
    registers::validate_patterned_register_write,
};

fn pattern_is_irrefutable(pattern: &CompiledPattern, trigger_schema: &SchemaBody) -> bool {
    fn check(
        pattern: &CompiledPattern,
        trigger_schema: &SchemaBody,
        bindings: &mut HashSet<usize>,
    ) -> bool {
        match pattern {
            CompiledPattern::Wildcard => true,
            CompiledPattern::Binding { binding_index, .. } => bindings.insert(*binding_index),
            CompiledPattern::ExpressionValue { .. }
            | CompiledPattern::EnumVariant { .. }
            | CompiledPattern::AtomTuple { .. } => false,
            CompiledPattern::Tuple { elements } => {
                let SchemaBody::Tuple(schemas) = trigger_schema else {
                    return false;
                };
                elements.len() == schemas.len()
                    && elements
                        .iter()
                        .zip(schemas)
                        .all(|(element, schema)| check(element, schema, bindings))
            }
            CompiledPattern::Array {
                prefix,
                spread,
                suffix,
            } => {
                let SchemaBody::Matrix {
                    element,
                    dimensions,
                } = trigger_schema
                else {
                    return false;
                };
                let minimum_len = prefix.len() + suffix.len();
                let known_len = dimensions
                    .iter()
                    .map(|dimension| match dimension {
                        DimensionExpr::Constant(value) => usize::try_from(*value).ok(),
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>()
                    .map(|dimensions| dimensions.into_iter().product::<usize>());
                match (known_len, spread) {
                    (Some(len), None) if len != minimum_len => return false,
                    (Some(len), Some(_)) if len < minimum_len => return false,
                    (None, None) => return false,
                    (None, Some(_)) if minimum_len != 0 => return false,
                    _ => {}
                }
                if !prefix
                    .iter()
                    .chain(suffix)
                    .all(|pattern| check(pattern, element, bindings))
                {
                    return false;
                }
                spread
                    .as_ref()
                    .and_then(|spread| spread.binding.as_deref())
                    .map_or(true, |binding| {
                        let middle_schema = SchemaBody::Matrix {
                            element: element.clone(),
                            dimensions: known_len
                                .map_or_else(
                                    || vec![DimensionExpr::Hole, DimensionExpr::Hole],
                                    |len| {
                                        vec![
                                            DimensionExpr::Constant(1),
                                            DimensionExpr::Constant((len - minimum_len) as u64),
                                        ]
                                    },
                                )
                                .into_boxed_slice(),
                        };
                        check(binding, &middle_schema, bindings)
                    })
            }
        }
    }

    check(pattern, trigger_schema, &mut HashSet::new())
}

pub(super) fn preflight_patterned_activation(
    scope: &ActivationScope,
    arms: &[ActivationArm],
    trigger: &ValueCell,
    trigger_cells: &[ReactiveCellId],
    i: &Interpreter,
) -> MResult<PreflightPatternedActivation> {
    arms.last().ok_or_else(|| {
        MechError::new(ActivationPatternArmsNonExhaustive, None).with_tokens(scope.tokens())
    })?;
    let trigger_id = match &scope.trigger {
        Expression::Var(var) => var.name.hash(),
        _ => {
            return Err(MechError::new(ActivationPatternTriggerInvariant, None)
                .with_tokens(scope.trigger.tokens()));
        }
    };
    for arm in arms {
        if let Some(guard) = &arm.guard {
            validate_patterned_guard_expression(guard, i)?;
        }
        validate_patterned_arm_body(&arm.body, trigger_id, trigger_cells, i)?;
    }
    if trigger_cells != [trigger.reactive_cell_id()] {
        return Err(
            MechError::new(ActivationPatternTriggerInvariant, None).with_tokens(scope.tokens())
        );
    }
    let trigger_schema = trigger.closed_schema_body()?;
    let mut compiled = Vec::new();
    for a in arms {
        let pattern = compile_pattern(&a.pattern, Some(&trigger_schema), i)?;
        let captures = pattern
            .binding_specs()
            .into_iter()
            .map(|binding| {
                let schema = binding.schema.ok_or_else(|| {
                    MechError::new(ActivationPatternCaptureKindUnsupported, None)
                        .with_tokens(a.pattern.tokens())
                })?;
                let proposed = create_capture_slot_for_schema(&schema)
                    .map_err(|error| error.with_tokens(a.pattern.tokens()))?;
                let committed = create_capture_slot_for_schema(&schema)
                    .map_err(|error| error.with_tokens(a.pattern.tokens()))?;
                Ok(ActivationPatternCapture {
                    id: binding.id,
                    name: binding.name,
                    schema,
                    proposed,
                    committed,
                })
            })
            .collect::<MResult<Vec<_>>>()?;
        compiled.push(PreflightActivationArm { pattern, captures });
    }
    let last = arms.last().unwrap();
    if last.guard.is_some()
        || !pattern_is_irrefutable(&compiled.last().unwrap().pattern, &trigger_schema)
    {
        return Err(
            MechError::new(ActivationPatternArmsNonExhaustive, None).with_tokens(scope.tokens())
        );
    }
    if arms[..arms.len() - 1]
        .iter()
        .any(|arm| arm.guard.is_none() && matches!(arm.pattern, Pattern::Wildcard))
    {
        return Err(
            MechError::new(ActivationPatternWildcardMustBeLast, None).with_tokens(scope.tokens())
        );
    }
    Ok(PreflightPatternedActivation { arms: compiled })
}

fn validation_error(kind: impl MechErrorKind + 'static, tokens: Vec<Token>) -> MResult<()> {
    Err(MechError::new(kind, None).with_tokens(tokens))
}

pub(super) fn validate_patterned_arm_body(
    body: &ActivationArmBody,
    trigger_id: u64,
    trigger_cells: &[ReactiveCellId],
    interpreter: &Interpreter,
) -> MResult<()> {
    match body {
        ActivationArmBody::Block(body) => {
            for (code, _) in body {
                validate_patterned_code(code, trigger_id, trigger_cells, interpreter)?;
            }
            Ok(())
        }
        ActivationArmBody::Expression(expression) => validate_patterned_expression(expression),
    }
}
fn validate_patterned_code(
    code: &MechCode,
    trigger_id: u64,
    trigger_cells: &[ReactiveCellId],
    interpreter: &Interpreter,
) -> MResult<()> {
    match code {
        MechCode::Comment(_) => Ok(()),
        MechCode::Expression(expression) => validate_patterned_expression(expression),
        MechCode::Statement(statement) => {
            validate_patterned_statement(statement, trigger_id, trigger_cells, interpreter)
        }
        MechCode::ActivationScope(_)
        | MechCode::FunctionDefine(_)
        | MechCode::FsmSpecification(_)
        | MechCode::FsmImplementation(_)
        | MechCode::Import(_)
        | MechCode::Error(_, _) => {
            validation_error(ActivationPatternDefinitionUnsupported, code.tokens())
        }
    }
}
fn validate_patterned_statement(
    statement: &Statement,
    trigger_id: u64,
    trigger_cells: &[ReactiveCellId],
    interpreter: &Interpreter,
) -> MResult<()> {
    match statement {
        Statement::VariableDefine(definition)
            if !definition.mutable && definition.var.context.is_none() =>
        {
            validate_patterned_expression(&definition.expression)
        }
        Statement::VariableDefine(definition) if definition.var.context.is_some() => {
            validation_error(
                ActivationPatternContextEffectUnsupported,
                statement.tokens(),
            )
        }
        Statement::VariableDefine(_) => {
            validation_error(ActivationPatternDefinitionUnsupported, statement.tokens())
        }
        Statement::VariableAssign(assignment) => validate_patterned_register_write(
            &assignment.target,
            &assignment.expression,
            trigger_id,
            trigger_cells,
            interpreter,
            statement.tokens(),
        ),
        Statement::OpAssign(assignment) => validate_patterned_register_write(
            &assignment.target,
            &assignment.expression,
            trigger_id,
            trigger_cells,
            interpreter,
            statement.tokens(),
        ),
        Statement::ContextSend(_) => validation_error(
            ActivationPatternContextEffectUnsupported,
            statement.tokens(),
        ),
        _ => validation_error(ActivationPatternDefinitionUnsupported, statement.tokens()),
    }
}
pub(super) fn validate_patterned_expression(expression: &Expression) -> MResult<()> {
    match expression {
        Expression::Literal(_) | Expression::Var(_) => Ok(()),
        Expression::Slice(slice) => validate_patterned_slice(slice),
        Expression::Formula(factor) => validate_patterned_factor(factor),
        Expression::FunctionCall(call) => {
            for (_, expression) in &call.args {
                validate_patterned_expression(expression)?;
            }
            Ok(())
        }
        Expression::Match(matched) => {
            validate_patterned_expression(&matched.source)?;
            for arm in &matched.arms {
                validate_patterned_pattern(&arm.pattern)?;
                if let Some(guard) = &arm.guard {
                    validate_patterned_expression(guard)?;
                }
                validate_patterned_expression(&arm.expression)?;
            }
            Ok(())
        }
        Expression::Range(range) => validate_patterned_range(range),
        Expression::Structure(structure) => validate_patterned_structure(structure),
        Expression::SetComprehension(comprehension) => {
            validate_patterned_expression(&comprehension.expression)?;
            for qualifier in &comprehension.qualifiers {
                validate_patterned_qualifier(qualifier)?;
            }
            Ok(())
        }
        Expression::MatrixComprehension(comprehension) => {
            validate_patterned_expression(&comprehension.expression)?;
            for qualifier in &comprehension.qualifiers {
                validate_patterned_qualifier(qualifier)?;
            }
            Ok(())
        }
        Expression::FsmPipe(_) => {
            validation_error(ActivationPatternDefinitionUnsupported, expression.tokens())
        }
    }
}

pub(super) fn validate_patterned_guard_expression(
    expression: &Expression,
    interpreter: &Interpreter,
) -> MResult<()> {
    validate_patterned_expression(expression)?;
    if guard_expression_is_not_static_pure(expression, interpreter, &mut HashSet::new()) {
        validation_error(ActivationPatternGuardMustBePure, expression.tokens())
    } else {
        Ok(())
    }
}

fn guard_expression_is_not_static_pure(
    expression: &Expression,
    interpreter: &Interpreter,
    visiting_functions: &mut HashSet<u64>,
) -> bool {
    match expression {
        Expression::Literal(_) | Expression::Var(_) => false,
        Expression::Slice(slice) => slice.subscript.iter().any(|subscript| {
            guard_subscript_is_not_static_pure(subscript, interpreter, visiting_functions)
        }),
        Expression::Formula(factor) => {
            guard_factor_is_not_static_pure(factor, interpreter, visiting_functions)
        }
        Expression::FunctionCall(call) => {
            if call.args.iter().any(|(_, expression)| {
                guard_expression_is_not_static_pure(expression, interpreter, visiting_functions)
            }) {
                return true;
            }
            let function_id = call.name.hash();
            let function_name = call.name.to_string();
            let resolved = {
                let state = interpreter.state.borrow();
                FunctionResolver::new(
                    interpreter.function_catalog(),
                    &state.function_environment,
                    &state.function_extensions,
                    &state.user_functions,
                )
                .resolve_named(&function_name)
                .map(|resolved| match resolved {
                    ResolvedNamedFunction::User(definition) => {
                        Ok::<FunctionDefinition, GuardFunctionSafety>(definition.clone())
                    }
                    ResolvedNamedFunction::Catalog(entry) => Err(entry.specializer.guard_safety()),
                    ResolvedNamedFunction::Extension(entry) => {
                        Err(entry.specializer.guard_safety())
                    }
                })
            };

            let user_function = match resolved {
                Ok(Ok(user_function)) => user_function,
                Ok(Err(GuardFunctionSafety::PureStatic)) => return false,
                Ok(Err(GuardFunctionSafety::Unsupported)) => return true,
                Err(error) if error.kind_name() == "MissingFunction" => return false,
                Err(_) => return true,
            };

            if !visiting_functions.insert(function_id) {
                return true;
            }
            let eager = match user_function.code.match_arms.as_slice() {
                [arm] if matches!(arm.pattern, Pattern::Wildcard) => {
                    guard_expression_is_not_static_pure(
                        &arm.expression,
                        interpreter,
                        visiting_functions,
                    )
                }
                _ => true,
            };
            visiting_functions.remove(&function_id);
            eager
        }
        Expression::Match(_)
        | Expression::SetComprehension(_)
        | Expression::MatrixComprehension(_)
        | Expression::FsmPipe(_) => true,
        Expression::Range(range) => {
            guard_range_is_not_static_pure(range, interpreter, visiting_functions)
        }
        Expression::Structure(structure) => {
            guard_structure_is_not_static_pure(structure, interpreter, visiting_functions)
        }
    }
}

fn guard_factor_is_not_static_pure(
    factor: &Factor,
    interpreter: &Interpreter,
    visiting_functions: &mut HashSet<u64>,
) -> bool {
    match factor {
        Factor::Expression(expression) => {
            guard_expression_is_not_static_pure(expression, interpreter, visiting_functions)
        }
        Factor::Negate(factor)
        | Factor::Not(factor)
        | Factor::Parenthetical(factor)
        | Factor::Transpose(factor) => {
            guard_factor_is_not_static_pure(factor, interpreter, visiting_functions)
        }
        Factor::Term(term) => {
            guard_factor_is_not_static_pure(&term.lhs, interpreter, visiting_functions)
                || term.rhs.iter().any(|(_, factor)| {
                    guard_factor_is_not_static_pure(factor, interpreter, visiting_functions)
                })
        }
    }
}

fn guard_range_is_not_static_pure(
    range: &RangeExpression,
    interpreter: &Interpreter,
    visiting_functions: &mut HashSet<u64>,
) -> bool {
    guard_factor_is_not_static_pure(&range.start, interpreter, visiting_functions)
        || range.increment.as_ref().map_or(false, |(_, increment)| {
            guard_factor_is_not_static_pure(increment, interpreter, visiting_functions)
        })
        || guard_factor_is_not_static_pure(&range.terminal, interpreter, visiting_functions)
}

fn guard_subscript_is_not_static_pure(
    subscript: &Subscript,
    interpreter: &Interpreter,
    visiting_functions: &mut HashSet<u64>,
) -> bool {
    match subscript {
        Subscript::Brace(subscripts) | Subscript::Bracket(subscripts) => {
            subscripts.iter().any(|subscript| {
                guard_subscript_is_not_static_pure(subscript, interpreter, visiting_functions)
            })
        }
        Subscript::Formula(factor) => {
            guard_factor_is_not_static_pure(factor, interpreter, visiting_functions)
        }
        Subscript::Range(range) => {
            guard_range_is_not_static_pure(range, interpreter, visiting_functions)
        }
        Subscript::All | Subscript::Dot(_) | Subscript::DotInt(_) | Subscript::Swizzle(_) => false,
    }
}

fn guard_structure_is_not_static_pure(
    structure: &Structure,
    interpreter: &Interpreter,
    visiting_functions: &mut HashSet<u64>,
) -> bool {
    match structure {
        Structure::Empty => false,
        Structure::Map(map) => map.elements.iter().any(|mapping| {
            guard_expression_is_not_static_pure(&mapping.key, interpreter, visiting_functions)
                || guard_expression_is_not_static_pure(
                    &mapping.value,
                    interpreter,
                    visiting_functions,
                )
        }),
        Structure::Matrix(matrix) => matrix.rows.iter().any(|row| {
            row.columns.iter().any(|column| {
                guard_expression_is_not_static_pure(
                    &column.element,
                    interpreter,
                    visiting_functions,
                )
            })
        }),
        Structure::Record(record) => record.bindings.iter().any(|binding| {
            guard_expression_is_not_static_pure(&binding.value, interpreter, visiting_functions)
        }),
        Structure::Set(set) => set.elements.iter().any(|expression| {
            guard_expression_is_not_static_pure(expression, interpreter, visiting_functions)
        }),
        Structure::Table(table) => table.rows.iter().any(|row| {
            row.columns.iter().any(|column| {
                guard_expression_is_not_static_pure(
                    &column.element,
                    interpreter,
                    visiting_functions,
                )
            })
        }),
        Structure::Tuple(tuple) => tuple.elements.iter().any(|expression| {
            guard_expression_is_not_static_pure(expression, interpreter, visiting_functions)
        }),
        Structure::TupleStruct(tuple) => {
            guard_expression_is_not_static_pure(&tuple.value, interpreter, visiting_functions)
        }
    }
}
fn validate_patterned_pattern(pattern: &Pattern) -> MResult<()> {
    match pattern {
        Pattern::Expression(expression) => validate_patterned_expression(expression),
        Pattern::Tuple(tuple) => {
            for pattern in &tuple.0 {
                validate_patterned_pattern(pattern)?;
            }
            Ok(())
        }
        Pattern::TupleStruct(tuple) => {
            for pattern in &tuple.patterns {
                validate_patterned_pattern(pattern)?;
            }
            Ok(())
        }
        Pattern::Array(array) => {
            for pattern in array.prefix.iter().chain(&array.suffix) {
                validate_patterned_pattern(pattern)?;
            }
            if let Some(spread) = &array.spread {
                if let Some(binding) = &spread.binding {
                    validate_patterned_pattern(binding)?;
                }
            }
            Ok(())
        }
        Pattern::Wildcard => Ok(()),
    }
}
fn validate_patterned_factor(factor: &Factor) -> MResult<()> {
    match factor {
        Factor::Expression(expression) => validate_patterned_expression(expression),
        Factor::Negate(factor)
        | Factor::Not(factor)
        | Factor::Parenthetical(factor)
        | Factor::Transpose(factor) => validate_patterned_factor(factor),
        Factor::Term(term) => {
            validate_patterned_factor(&term.lhs)?;
            for (_, factor) in &term.rhs {
                validate_patterned_factor(factor)?;
            }
            Ok(())
        }
    }
}
fn validate_patterned_range(range: &RangeExpression) -> MResult<()> {
    validate_patterned_factor(&range.start)?;
    if let Some((_, increment)) = &range.increment {
        validate_patterned_factor(increment)?;
    }
    validate_patterned_factor(&range.terminal)
}
fn validate_patterned_slice(slice: &Slice) -> MResult<()> {
    for subscript in &slice.subscript {
        validate_patterned_subscript(subscript)?;
    }
    Ok(())
}
fn validate_patterned_subscript(subscript: &Subscript) -> MResult<()> {
    match subscript {
        Subscript::Brace(subscripts) | Subscript::Bracket(subscripts) => {
            for subscript in subscripts {
                validate_patterned_subscript(subscript)?;
            }
            Ok(())
        }
        Subscript::Formula(factor) => validate_patterned_factor(factor),
        Subscript::Range(range) => validate_patterned_range(range),
        Subscript::All | Subscript::Dot(_) | Subscript::DotInt(_) | Subscript::Swizzle(_) => Ok(()),
    }
}
fn validate_patterned_structure(structure: &Structure) -> MResult<()> {
    match structure {
        Structure::Empty => Ok(()),
        Structure::Map(map) => {
            for mapping in &map.elements {
                validate_patterned_expression(&mapping.key)?;
                validate_patterned_expression(&mapping.value)?;
            }
            Ok(())
        }
        Structure::Matrix(matrix) => {
            for row in &matrix.rows {
                for column in &row.columns {
                    validate_patterned_expression(&column.element)?;
                }
            }
            Ok(())
        }
        Structure::Record(record) => {
            for binding in &record.bindings {
                validate_patterned_expression(&binding.value)?;
            }
            Ok(())
        }
        Structure::Set(set) => {
            for expression in &set.elements {
                validate_patterned_expression(expression)?;
            }
            Ok(())
        }
        Structure::Table(table) => {
            for row in &table.rows {
                for column in &row.columns {
                    validate_patterned_expression(&column.element)?;
                }
            }
            Ok(())
        }
        Structure::Tuple(tuple) => {
            for expression in &tuple.elements {
                validate_patterned_expression(expression)?;
            }
            Ok(())
        }
        Structure::TupleStruct(tuple) => validate_patterned_expression(&tuple.value),
    }
}
fn validate_patterned_qualifier(qualifier: &ComprehensionQualifier) -> MResult<()> {
    match qualifier {
        ComprehensionQualifier::Generator((pattern, expression)) => {
            validate_patterned_pattern(pattern)?;
            validate_patterned_expression(expression)
        }
        ComprehensionQualifier::Filter(expression) => validate_patterned_expression(expression),
        ComprehensionQualifier::Let(definition) if definition.mutable => {
            validation_error(ActivationPatternDefinitionUnsupported, definition.tokens())
        }
        ComprehensionQualifier::Let(definition) if definition.var.context.is_some() => {
            validation_error(
                ActivationPatternContextEffectUnsupported,
                definition.tokens(),
            )
        }
        ComprehensionQualifier::Let(definition) => {
            validate_patterned_expression(&definition.expression)
        }
    }
}
