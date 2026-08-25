use super::{
    ComprehensionGeneratorError, Environment, ReactiveComprehensionStructureUnsupported, expression,
};
#[cfg(feature = "matrix_comprehensions")]
use crate::MatrixComprehension;
#[cfg(any(feature = "rational", feature = "complex"))]
use crate::ToValue;
#[cfg(feature = "matrix_comprehensions")]
pub use crate::intrinsics::constructors::ValueMatrixComprehension;
#[cfg(feature = "set_comprehensions")]
pub use crate::intrinsics::constructors::ValueSetComprehension;
use crate::patterns::PatternBindingSink;
use crate::{
    ComprehensionQualifier, ExternalInteraction, FunctionSpecializer, Interpreter,
    InterpreterExecution, LegacyValue, MResult, MechError, MechFunction, ReactiveNodeKind, Ref,
    execute_catalog_operation, hash_str, val_ref_reactive_cell_ids,
};
#[cfg(feature = "set_comprehensions")]
use crate::{MechSet, SetComprehension};
#[cfg(feature = "matrix_comprehensions")]
use mech_core::matrix::Matrix;
use std::collections::{HashMap, HashSet};

#[cfg(any(feature = "set_comprehensions", feature = "matrix_comprehensions"))]
fn value_depends_on_reactive_turn(value: &LegacyValue, p: &InterpreterExecution<'_>) -> bool {
    let mut turn_cells = {
        let state = p.state.borrow();
        let symbols = state.symbol_table.borrow();
        symbols
            .mutable_variables
            .values()
            .flat_map(val_ref_reactive_cell_ids)
            .collect::<HashSet<_>>()
    };
    let plan = p.plan();
    let plan = plan.borrow();
    for node in &plan.nodes {
        let external = node
            .function
            .semantic_operation_contract()
            .is_some_and(|contract| contract.interaction != ExternalInteraction::Pure);
        let depends_on_turn = node.kind == ReactiveNodeKind::Register
            || external
            || node
                .inputs
                .iter()
                .any(|input| turn_cells.contains(&input.cell));
        if depends_on_turn {
            turn_cells.extend(node.outputs.iter().copied());
        }
    }
    value
        .reactive_cell_ids()
        .iter()
        .any(|cell| turn_cells.contains(cell))
}

#[cfg(any(feature = "set_comprehensions", feature = "matrix_comprehensions"))]
fn reject_reactive_structure(
    value: &LegacyValue,
    qualifier: &'static str,
    p: &InterpreterExecution<'_>,
) -> MResult<()> {
    if value_depends_on_reactive_turn(value, p) {
        return Err(MechError::new(
            ReactiveComprehensionStructureUnsupported { qualifier },
            None,
        )
        .with_compiler_loc());
    }
    Ok(())
}

#[cfg(any(feature = "set_comprehensions", feature = "matrix_comprehensions"))]
fn comprehension_environments(
    qualifiers: &[ComprehensionQualifier],
    comprehension_id: u64,
    p: &InterpreterExecution<'_>,
) -> MResult<(Vec<Environment>, Interpreter)> {
    let mut envs: Vec<Environment> = vec![HashMap::new()];
    let mut new_p: Interpreter = (**p).clone();
    new_p.id = comprehension_id;
    // A comprehension has its own lexical environment, not its own reactive
    // lifetime. Keep the enclosing plan shared so operations compiled inside
    // generators, filters, lets, and the result expression remain live and
    // precede the final comprehension node in the same dependency graph.
    for qual in qualifiers {
        envs = match qual {
            ComprehensionQualifier::Generator((pttrn, expr)) => {
                let compiled = crate::patterns::compile_pattern(pttrn, None, &new_p)?;
                let mut new_envs = Vec::new();
                for env in &envs {
                    let collection = p.with_interpreter(&new_p, |execution| {
                        expression(expr, Some(env), execution)
                    })?;
                    reject_reactive_structure(&collection, "generator", p)?;
                    for elmnt in comprehension_generator_values(&collection)? {
                        let mut new_env = env.clone();
                        let pattern_match = p.with_interpreter(&new_p, |execution| {
                            crate::patterns::match_compiled_pattern_with_environment_constraints(
                                &compiled, &elmnt, &new_env, execution,
                            )
                        })?;
                        if pattern_match.matched {
                            crate::patterns::EnvironmentBindingSink::new(&mut new_env)
                                .commit(&pattern_match)?;
                            new_envs.push(new_env);
                        }
                    }
                }
                new_envs
            }
            ComprehensionQualifier::Filter(expr) => {
                let mut filtered = Vec::new();
                for env in envs {
                    let result = p.with_interpreter(&new_p, |execution| {
                        expression(expr, Some(&env), execution)
                    })?;
                    reject_reactive_structure(&result, "filter", p)?;
                    if matches!(result, LegacyValue::Bool(value) if *value.borrow()) {
                        filtered.push(env);
                    }
                }
                filtered
            }
            ComprehensionQualifier::Let(var_def) => envs
                .into_iter()
                .map(|mut env| -> MResult<_> {
                    let val = p.with_interpreter(&new_p, |execution| {
                        expression(&var_def.expression, Some(&env), execution)
                    })?;
                    env.insert(var_def.var.name.hash(), val);
                    Ok(env)
                })
                .collect::<MResult<Vec<_>>>()?,
        };
    }
    Ok((envs, new_p))
}

#[cfg(any(feature = "set_comprehensions", feature = "matrix_comprehensions"))]
fn comprehension_generator_values(collection: &LegacyValue) -> MResult<Vec<LegacyValue>> {
    match collection {
        #[cfg(feature = "set")]
        LegacyValue::Set(mset) => Ok(mset.borrow().set.iter().cloned().collect()),
        #[cfg(feature = "matrix")]
        LegacyValue::MatrixIndex(matrix) => Ok(matrix
            .as_vec()
            .into_iter()
            .map(|value| LegacyValue::Index(Ref::new(value)))
            .collect()),
        #[cfg(all(feature = "matrix", feature = "bool"))]
        LegacyValue::MatrixBool(matrix) => {
            Ok(matrix.as_vec().into_iter().map(LegacyValue::from).collect())
        }
        #[cfg(all(feature = "matrix", feature = "u8"))]
        LegacyValue::MatrixU8(matrix) => {
            Ok(matrix.as_vec().into_iter().map(LegacyValue::from).collect())
        }
        #[cfg(all(feature = "matrix", feature = "u16"))]
        LegacyValue::MatrixU16(matrix) => {
            Ok(matrix.as_vec().into_iter().map(LegacyValue::from).collect())
        }
        #[cfg(all(feature = "matrix", feature = "u32"))]
        LegacyValue::MatrixU32(matrix) => {
            Ok(matrix.as_vec().into_iter().map(LegacyValue::from).collect())
        }
        #[cfg(all(feature = "matrix", feature = "u64"))]
        LegacyValue::MatrixU64(matrix) => {
            Ok(matrix.as_vec().into_iter().map(LegacyValue::from).collect())
        }
        #[cfg(all(feature = "matrix", feature = "u128"))]
        LegacyValue::MatrixU128(matrix) => {
            Ok(matrix.as_vec().into_iter().map(LegacyValue::from).collect())
        }
        #[cfg(all(feature = "matrix", feature = "i8"))]
        LegacyValue::MatrixI8(matrix) => {
            Ok(matrix.as_vec().into_iter().map(LegacyValue::from).collect())
        }
        #[cfg(all(feature = "matrix", feature = "i16"))]
        LegacyValue::MatrixI16(matrix) => {
            Ok(matrix.as_vec().into_iter().map(LegacyValue::from).collect())
        }
        #[cfg(all(feature = "matrix", feature = "i32"))]
        LegacyValue::MatrixI32(matrix) => {
            Ok(matrix.as_vec().into_iter().map(LegacyValue::from).collect())
        }
        #[cfg(all(feature = "matrix", feature = "i64"))]
        LegacyValue::MatrixI64(matrix) => {
            Ok(matrix.as_vec().into_iter().map(LegacyValue::from).collect())
        }
        #[cfg(all(feature = "matrix", feature = "i128"))]
        LegacyValue::MatrixI128(matrix) => {
            Ok(matrix.as_vec().into_iter().map(LegacyValue::from).collect())
        }
        #[cfg(all(feature = "matrix", feature = "f32"))]
        LegacyValue::MatrixF32(matrix) => {
            Ok(matrix.as_vec().into_iter().map(LegacyValue::from).collect())
        }
        #[cfg(all(feature = "matrix", feature = "f64"))]
        LegacyValue::MatrixF64(matrix) => {
            Ok(matrix.as_vec().into_iter().map(LegacyValue::from).collect())
        }
        #[cfg(all(feature = "matrix", feature = "string"))]
        LegacyValue::MatrixString(matrix) => {
            Ok(matrix.as_vec().into_iter().map(LegacyValue::from).collect())
        }
        #[cfg(all(feature = "matrix", feature = "rational"))]
        LegacyValue::MatrixR64(matrix) => Ok(matrix
            .as_vec()
            .into_iter()
            .map(|value| value.to_value())
            .collect()),
        #[cfg(all(feature = "matrix", feature = "complex"))]
        LegacyValue::MatrixC64(matrix) => Ok(matrix
            .as_vec()
            .into_iter()
            .map(|value| value.to_value())
            .collect()),
        #[cfg(feature = "matrix")]
        LegacyValue::MatrixValue(matrix) => Ok(matrix.as_vec()),
        LegacyValue::MutableReference(reference) => {
            comprehension_generator_values(&reference.borrow())
        }
        x => Err(
            MechError::new(ComprehensionGeneratorError { found: x.kind() }, None)
                .with_compiler_loc(),
        ),
    }
}

#[cfg(feature = "set_comprehensions")]
pub struct SetComprehensionDefine {}
#[cfg(all(feature = "set_comprehensions", feature = "functions"))]
impl FunctionSpecializer for SetComprehensionDefine {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        Ok(Box::new(ValueSetComprehension {
            arguments: arguments.to_vec(),
            out: Ref::new(MechSet::from_vec(arguments.to_vec())),
        }))
    }
}
#[cfg(feature = "matrix_comprehensions")]
pub struct MatrixComprehensionDefine {}
#[cfg(all(feature = "matrix_comprehensions", feature = "functions"))]
impl FunctionSpecializer for MatrixComprehensionDefine {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        let out = if arguments.is_empty() {
            LegacyValue::MatrixValue(Matrix::from_vec(vec![], 0, 0))
        } else {
            let fxn = crate::intrinsics::horzcat::impl_horzcat_fxn(arguments)?;
            fxn.solve_result()?;
            fxn.out()
        };
        Ok(Box::new(ValueMatrixComprehension {
            arguments: arguments.to_vec(),
            out: Ref::new(out),
        }))
    }
}
#[cfg(feature = "set_comprehensions")]
pub fn set_comprehension(
    set_comp: &SetComprehension,
    p: &InterpreterExecution<'_>,
) -> MResult<LegacyValue> {
    let comprehension_id = hash_str(&format!("{:?}", set_comp));
    let (envs, new_p) = comprehension_environments(&set_comp.qualifiers, comprehension_id, p)?;
    let mut values = Vec::new();
    for env in envs {
        let val = p.with_interpreter(&new_p, |execution| {
            expression(&set_comp.expression, Some(&env), execution)
        })?;
        values.push(val);
    }
    let plan = p.plan();
    execute_catalog_operation(p, &plan, "set/comprehension", values)
}

#[cfg(feature = "matrix_comprehensions")]
pub fn matrix_comprehension(
    matrix_comp: &MatrixComprehension,
    p: &InterpreterExecution<'_>,
) -> MResult<LegacyValue> {
    let comprehension_id = hash_str(&format!("{:?}", matrix_comp));
    let (envs, new_p) = comprehension_environments(&matrix_comp.qualifiers, comprehension_id, p)?;
    let mut values = Vec::new();
    for env in envs {
        values.push(p.with_interpreter(&new_p, |execution| {
            expression(&matrix_comp.expression, Some(&env), execution)
        })?);
    }
    let plan = p.plan();
    execute_catalog_operation(p, &plan, "matrix/comprehension", values)
}
