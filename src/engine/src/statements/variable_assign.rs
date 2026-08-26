#[cfg(feature = "variable_assign")]
use super::{NotMutableError, UndefinedVariableError};
use crate::LegacyValue;
#[cfg(all(feature = "subscript", feature = "assign"))]
use crate::Subscript;
#[cfg(all(feature = "subscript", feature = "assign", feature = "tuple"))]
use crate::real;
#[cfg(all(feature = "subscript", feature = "assign", feature = "subscript_range"))]
use crate::subscript_range;
#[cfg(any(
    feature = "variable_assign",
    all(feature = "subscript", feature = "assign")
))]
use crate::{Environment, InterpreterExecution, MResult};
#[cfg(all(feature = "subscript", feature = "assign"))]
use crate::{MatrixSelector, MechFunction, OperationId};
#[cfg(feature = "variable_assign")]
use crate::{
    MechError, VariableAssign, execute_catalog_operation_with_registration_arguments, expression,
};
#[cfg(all(
    feature = "subscript",
    feature = "assign",
    feature = "subscript_formula"
))]
use crate::{subscript_formula, subscript_formula_ix};

#[cfg(any(
    feature = "variable_assign",
    feature = "math_add_assign",
    feature = "math_sub_assign",
    feature = "math_div_assign",
    feature = "math_mul_assign"
))]
pub(super) fn assignment_registration_operand(value: &LegacyValue) -> LegacyValue {
    match value {
        LegacyValue::MutableReference(reference) => {
            let inner = reference.borrow();
            assignment_registration_operand(&inner)
        }
        _ => value.clone(),
    }
}

#[cfg(all(feature = "subscript", feature = "assign"))]
fn catalog_assignment_function(
    p: &InterpreterExecution<'_>,
    canonical_name: &str,
    arguments: &[LegacyValue],
) -> MResult<Box<dyn MechFunction>> {
    p.specialize_visible_operation_named(
        OperationId::from_name(canonical_name),
        Some(canonical_name),
        arguments,
    )
}

#[cfg(all(feature = "subscript", feature = "assign"))]
fn source_assignment_function(
    p: &InterpreterExecution<'_>,
    canonical_name: &str,
    arguments: &[LegacyValue],
    selectors: &[MatrixSelector],
) -> MResult<Box<dyn MechFunction>> {
    crate::intrinsics::assign::specialize_matrix_assignment(
        p,
        canonical_name,
        arguments[0].clone(),
        arguments[1].clone(),
        selectors,
    )
}

#[cfg(feature = "variable_assign")]
pub fn variable_assign(
    var_assgn: &VariableAssign,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<LegacyValue> {
    let slc = &var_assgn.target;
    if slc.context.is_some() {
        return super::context_assign(var_assgn, env, p);
    }
    let source = expression(&var_assgn.expression, env, p)?;
    let id = slc.name.hash();
    let sink = {
        let symbols = p.symbols();
        let symbols_brrw = symbols.borrow();
        match symbols_brrw.get_mutable(id) {
            Some(val) => val.borrow().clone(),
            None => {
                if !symbols_brrw.contains(id) {
                    return Err(MechError::new(
            UndefinedVariableError { id, name: slc.name.to_string() },
            Some("(!)> Variables are defined with the `:=` operator. *e.g.*: {{x := 123}}".to_string()),
          ).with_compiler_loc().with_tokens(slc.name.tokens()));
                } else {
                    return Err(MechError::new(
            NotMutableError { id },
            Some("(!)> Mutable variables are defined with the `~` operator. *e.g.*: {{~x := 123}}".to_string()),
          ).with_compiler_loc().with_tokens(slc.name.tokens()));
                }
            }
        }
    };
    match &slc.subscript {
        #[cfg(feature = "subscript")]
        Some(sbscrpt) => {
            for s in sbscrpt {
                let s_result = subscript_ref(&s, &sink, &source, env, p)?;
                return Ok(s_result);
            }
        }
        #[cfg(not(feature = "subscript"))]
        Some(_) => todo!("variable assignment subscripts require subscript support"),
        None => {
            let plan = p.plan();
            let registration_source = assignment_registration_operand(&source);
            return execute_catalog_operation_with_registration_arguments(
                p,
                &plan,
                "assign",
                vec![sink, source],
                vec![registration_source],
            );
        }
    }
    #[cfg(feature = "subscript")]
    unreachable!(); // subscript should have thrown an error if we can't access an element
}

#[cfg(all(feature = "subscript", feature = "assign"))]
pub fn subscript_ref(
    sbscrpt: &Subscript,
    sink: &LegacyValue,
    source: &LegacyValue,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<LegacyValue> {
    let plan = p.plan();
    match sbscrpt {
        Subscript::Dot(x) => {
            let key = x.hash();
            let fxn_input: Vec<LegacyValue> =
                vec![sink.clone(), source.clone(), LegacyValue::Id(key)];
            let new_fxn = catalog_assignment_function(p, "assign/column", &fxn_input)?;
            new_fxn.solve_result()?;
            let res = new_fxn.out();
            plan.borrow_mut().push(new_fxn);
            return Ok(res);
        }
        #[cfg(feature = "tuple")]
        Subscript::DotInt(x) => {
            let ix = real(x, p)?.as_index()?;
            let fxn_input: Vec<LegacyValue> = vec![sink.clone(), source.clone(), ix.clone()];
            let new_fxn = catalog_assignment_function(p, "assign", &fxn_input)?;
            new_fxn.solve_result()?;
            let res = new_fxn.out();
            plan.borrow_mut().push(new_fxn);
            return Ok(res);
        }
        Subscript::Swizzle(_) => {
            unreachable!()
        }
        Subscript::Bracket(subs) => {
            let fxn_input = vec![sink.clone(), source.clone()];
            let mut selectors = Vec::with_capacity(subs.len());
            match &subs[..] {
                #[cfg(feature = "subscript_formula")]
                [Subscript::Formula(_)] => {
                    let ixes = subscript_formula_ix(&subs[0], env, p)?;
                    let shape = ixes.shape();
                    selectors.push(MatrixSelector::Value(ixes.clone()));
                    match shape[..] {
                        #[cfg(feature = "matrix")]
                        [1, 1] => {}
                        #[cfg(all(
                            feature = "matrix",
                            feature = "subscript_range",
                            feature = "assign"
                        ))]
                        [1, _] => {}
                        #[cfg(all(
                            feature = "matrix",
                            feature = "subscript_range",
                            feature = "assign"
                        ))]
                        [_, 1] => {}
                        _ => todo!(),
                    }
                }
                #[cfg(all(feature = "matrix", feature = "subscript_range"))]
                [Subscript::Range(_)] => {
                    let ixes = subscript_range(&subs[0], env, p)?;
                    selectors.push(MatrixSelector::Value(ixes));
                }
                #[cfg(all(feature = "matrix", feature = "subscript_range"))]
                [Subscript::All] => {
                    selectors.push(MatrixSelector::All);
                }
                [Subscript::All, Subscript::All] => todo!(),
                #[cfg(feature = "subscript_formula")]
                [Subscript::Formula(_), Subscript::Formula(_)] => {
                    let result1 = subscript_formula_ix(&subs[0], env, p)?;
                    let result2 = subscript_formula_ix(&subs[1], env, p)?;
                    let shape1 = result1.shape();
                    let shape2 = result2.shape();
                    selectors.push(MatrixSelector::Value(result1));
                    selectors.push(MatrixSelector::Value(result2));
                    match ((shape1[0], shape1[1]), (shape2[0], shape2[1])) {
                        #[cfg(feature = "matrix")]
                        ((1, 1), (1, 1)) => {}
                        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
                        ((1, 1), (_, 1)) => {}
                        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
                        ((_, 1), (1, 1)) => {}
                        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
                        ((_, 1), (_, 1)) => {}
                        _ => unreachable!(),
                    }
                }
                #[cfg(all(feature = "matrix", feature = "subscript_range"))]
                [Subscript::Range(_), Subscript::Range(_)] => {
                    let result = subscript_range(&subs[0], env, p)?;
                    selectors.push(MatrixSelector::Value(result));
                    let result = subscript_range(&subs[1], env, p)?;
                    selectors.push(MatrixSelector::Value(result));
                }
                #[cfg(all(feature = "matrix", feature = "subscript_formula"))]
                [Subscript::All, Subscript::Formula(_)] => {
                    selectors.push(MatrixSelector::All);
                    let ix = subscript_formula_ix(&subs[1], env, p)?;
                    let shape = ix.shape();
                    selectors.push(MatrixSelector::Value(ix));
                    match shape[..] {
                        [1, 1] => {}
                        [1, _] => {}
                        [_, 1] => {}
                        _ => todo!(),
                    }
                }
                #[cfg(feature = "subscript_formula")]
                [Subscript::Formula(_), Subscript::All] => {
                    let ix = subscript_formula_ix(&subs[0], env, p)?;
                    let shape = ix.shape();
                    selectors.push(MatrixSelector::Value(ix));
                    selectors.push(MatrixSelector::All);
                    match shape[..] {
                        #[cfg(feature = "matrix")]
                        [1, 1] => {}
                        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
                        [1, _] => {}
                        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
                        [_, 1] => {}
                        _ => todo!(),
                    }
                }
                #[cfg(all(feature = "subscript_formula", feature = "subscript_range"))]
                [Subscript::Range(_), Subscript::Formula(_)] => {
                    let result = subscript_range(&subs[0], env, p)?;
                    selectors.push(MatrixSelector::Value(result));
                    let result = subscript_formula_ix(&subs[1], env, p)?;
                    let shape = result.shape();
                    selectors.push(MatrixSelector::Value(result));
                    match &shape[..] {
                        #[cfg(feature = "matrix")]
                        [1, 1] => {}
                        #[cfg(feature = "matrix")]
                        [1, _] => {}
                        #[cfg(feature = "matrix")]
                        [_, 1] => {}
                        _ => todo!(),
                    }
                }
                #[cfg(all(feature = "subscript_formula", feature = "subscript_range"))]
                [Subscript::Formula(_), Subscript::Range(_)] => {
                    let result = subscript_formula_ix(&subs[0], env, p)?;
                    let shape = result.shape();
                    selectors.push(MatrixSelector::Value(result));
                    let result = subscript_range(&subs[1], env, p)?;
                    selectors.push(MatrixSelector::Value(result));
                    match &shape[..] {
                        #[cfg(feature = "matrix")]
                        [1, 1] => {}
                        #[cfg(feature = "matrix")]
                        [1, _] => {}
                        #[cfg(feature = "matrix")]
                        [_, 1] => {}
                        _ => todo!(),
                    }
                }
                #[cfg(all(feature = "matrix", feature = "subscript_range"))]
                [Subscript::All, Subscript::Range(_)] => {
                    selectors.push(MatrixSelector::All);
                    let result = subscript_range(&subs[1], env, p)?;
                    selectors.push(MatrixSelector::Value(result));
                }
                #[cfg(all(feature = "matrix", feature = "subscript_range"))]
                [Subscript::Range(_), Subscript::All] => {
                    let result = subscript_range(&subs[0], env, p)?;
                    selectors.push(MatrixSelector::Value(result));
                    selectors.push(MatrixSelector::All);
                }
                _ => unreachable!(),
            };
            plan.borrow_mut().push(source_assignment_function(
                p, "assign", &fxn_input, &selectors,
            )?);
            let plan_brrw = plan.borrow();
            let new_fxn = &plan_brrw.last().unwrap();
            new_fxn.solve_result()?;
            let res = new_fxn.out();
            return Ok(res);
        }
        Subscript::Brace(subs) => {
            let mut fxn_input = vec![sink.clone()];
            match &subs[..] {
                #[cfg(feature = "subscript_formula")]
                [Subscript::Formula(_)] => {
                    fxn_input.push(source.clone());
                    let ixes = subscript_formula(&subs[0], env, p)?;
                    let shape = ixes.shape();
                    fxn_input.push(ixes);
                    match shape[..] {
                        #[cfg(feature = "map")]
                        [1, 1] => {
                            plan.borrow_mut()
                                .push(catalog_assignment_function(p, "assign", &fxn_input)?);
                        }
                        _ => todo!(),
                    }
                }
                #[cfg(all(feature = "matrix", feature = "subscript_range"))]
                [Subscript::Range(_)] => {
                    todo!();
                }
                #[cfg(all(feature = "matrix", feature = "subscript_range"))]
                [Subscript::All] => {
                    todo!();
                }
                _ => unreachable!(),
            };
            let plan_brrw = plan.borrow();
            let new_fxn = &plan_brrw.last().unwrap();
            new_fxn.solve_result()?;
            let res = new_fxn.out();
            return Ok(res);
        }
        _ => unreachable!(),
    }
}
