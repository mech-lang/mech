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
use crate::{Environment, InterpreterExecution, MResult, MechFunction, OperationId};
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
        Some(sbscrpt) =>
        {
            #[cfg(feature = "subscript")]
            for s in sbscrpt {
                let s_result = subscript_ref(&s, &sink, &source, env, p)?;
                return Ok(s_result);
            }
        }
        #[cfg(feature = "assign")]
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
            let mut fxn_input = vec![sink.clone()];
            match &subs[..] {
                #[cfg(feature = "subscript_formula")]
                [Subscript::Formula(_)] => {
                    fxn_input.push(source.clone());
                    let ixes = subscript_formula_ix(&subs[0], env, p)?;
                    let shape = ixes.shape();
                    fxn_input.push(ixes);
                    match shape[..] {
                        #[cfg(feature = "matrix")]
                        [1, 1] => {
                            plan.borrow_mut()
                                .push(catalog_assignment_function(p, "assign", &fxn_input)?);
                        }
                        #[cfg(all(
                            feature = "matrix",
                            feature = "subscript_range",
                            feature = "assign"
                        ))]
                        [1, _] => {
                            plan.borrow_mut()
                                .push(catalog_assignment_function(p, "assign", &fxn_input)?);
                        }
                        #[cfg(all(
                            feature = "matrix",
                            feature = "subscript_range",
                            feature = "assign"
                        ))]
                        [_, 1] => {
                            plan.borrow_mut()
                                .push(catalog_assignment_function(p, "assign", &fxn_input)?);
                        }
                        _ => todo!(),
                    }
                }
                #[cfg(all(feature = "matrix", feature = "subscript_range"))]
                [Subscript::Range(_)] => {
                    fxn_input.push(source.clone());
                    let ixes = subscript_range(&subs[0], env, p)?;
                    fxn_input.push(ixes);
                    plan.borrow_mut()
                        .push(catalog_assignment_function(p, "assign", &fxn_input)?);
                }
                #[cfg(all(feature = "matrix", feature = "subscript_range"))]
                [Subscript::All] => {
                    fxn_input.push(source.clone());
                    fxn_input.push(LegacyValue::IndexAll);
                    plan.borrow_mut()
                        .push(catalog_assignment_function(p, "assign", &fxn_input)?);
                }
                [Subscript::All, Subscript::All] => todo!(),
                #[cfg(feature = "subscript_formula")]
                [Subscript::Formula(_), Subscript::Formula(_)] => {
                    fxn_input.push(source.clone());
                    let result1 = subscript_formula_ix(&subs[0], env, p)?;
                    let result2 = subscript_formula_ix(&subs[1], env, p)?;
                    let shape1 = result1.shape();
                    let shape2 = result2.shape();
                    fxn_input.push(result1);
                    fxn_input.push(result2);
                    match ((shape1[0], shape1[1]), (shape2[0], shape2[1])) {
                        #[cfg(feature = "matrix")]
                        ((1, 1), (1, 1)) => {
                            plan.borrow_mut()
                                .push(catalog_assignment_function(p, "assign", &fxn_input)?);
                        }
                        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
                        ((1, 1), (_, 1)) => {
                            plan.borrow_mut()
                                .push(catalog_assignment_function(p, "assign", &fxn_input)?);
                        }
                        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
                        ((_, 1), (1, 1)) => {
                            plan.borrow_mut()
                                .push(catalog_assignment_function(p, "assign", &fxn_input)?);
                        }
                        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
                        ((_, 1), (_, 1)) => {
                            plan.borrow_mut()
                                .push(catalog_assignment_function(p, "assign", &fxn_input)?);
                        }
                        _ => unreachable!(),
                    }
                }
                #[cfg(all(feature = "matrix", feature = "subscript_range"))]
                [Subscript::Range(_), Subscript::Range(_)] => {
                    fxn_input.push(source.clone());
                    let result = subscript_range(&subs[0], env, p)?;
                    fxn_input.push(result);
                    let result = subscript_range(&subs[1], env, p)?;
                    fxn_input.push(result);
                    plan.borrow_mut()
                        .push(catalog_assignment_function(p, "assign", &fxn_input)?);
                }
                #[cfg(all(feature = "matrix", feature = "subscript_formula"))]
                [Subscript::All, Subscript::Formula(_)] => {
                    fxn_input.push(source.clone());
                    fxn_input.push(LegacyValue::IndexAll);
                    let ix = subscript_formula_ix(&subs[1], env, p)?;
                    let shape = ix.shape();
                    fxn_input.push(ix);
                    match shape[..] {
                        #[cfg(feature = "matrix")]
                        [1, 1] => {
                            plan.borrow_mut()
                                .push(catalog_assignment_function(p, "assign", &fxn_input)?);
                        }
                        #[cfg(feature = "matrix")]
                        [1, _] => {
                            plan.borrow_mut()
                                .push(catalog_assignment_function(p, "assign", &fxn_input)?);
                        }
                        #[cfg(feature = "matrix")]
                        [_, 1] => {
                            plan.borrow_mut()
                                .push(catalog_assignment_function(p, "assign", &fxn_input)?);
                        }
                        _ => todo!(),
                    }
                }
                #[cfg(feature = "subscript_formula")]
                [Subscript::Formula(_), Subscript::All] => {
                    fxn_input.push(source.clone());
                    let ix = subscript_formula_ix(&subs[0], env, p)?;
                    let shape = ix.shape();
                    fxn_input.push(ix);
                    fxn_input.push(LegacyValue::IndexAll);
                    match shape[..] {
                        #[cfg(feature = "matrix")]
                        [1, 1] => {
                            plan.borrow_mut()
                                .push(catalog_assignment_function(p, "assign", &fxn_input)?);
                        }
                        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
                        [1, _] => {
                            plan.borrow_mut()
                                .push(catalog_assignment_function(p, "assign", &fxn_input)?);
                        }
                        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
                        [_, 1] => {
                            plan.borrow_mut()
                                .push(catalog_assignment_function(p, "assign", &fxn_input)?);
                        }
                        _ => todo!(),
                    }
                }
                #[cfg(all(feature = "subscript_formula", feature = "subscript_range"))]
                [Subscript::Range(_), Subscript::Formula(_)] => {
                    fxn_input.push(source.clone());
                    let result = subscript_range(&subs[0], env, p)?;
                    fxn_input.push(result);
                    let result = subscript_formula_ix(&subs[1], env, p)?;
                    let shape = result.shape();
                    fxn_input.push(result);
                    match &shape[..] {
                        #[cfg(feature = "matrix")]
                        [1, 1] => {
                            plan.borrow_mut()
                                .push(catalog_assignment_function(p, "assign", &fxn_input)?);
                        }
                        #[cfg(feature = "matrix")]
                        [1, _] => {
                            plan.borrow_mut()
                                .push(catalog_assignment_function(p, "assign", &fxn_input)?);
                        }
                        #[cfg(feature = "matrix")]
                        [_, 1] => {
                            plan.borrow_mut()
                                .push(catalog_assignment_function(p, "assign", &fxn_input)?);
                        }
                        _ => todo!(),
                    }
                }
                #[cfg(all(feature = "subscript_formula", feature = "subscript_range"))]
                [Subscript::Formula(_), Subscript::Range(_)] => {
                    fxn_input.push(source.clone());
                    let result = subscript_formula_ix(&subs[0], env, p)?;
                    let shape = result.shape();
                    fxn_input.push(result);
                    let result = subscript_range(&subs[1], env, p)?;
                    fxn_input.push(result);
                    match &shape[..] {
                        #[cfg(feature = "matrix")]
                        [1, 1] => {
                            plan.borrow_mut()
                                .push(catalog_assignment_function(p, "assign", &fxn_input)?);
                        }
                        #[cfg(feature = "matrix")]
                        [1, _] => {
                            plan.borrow_mut()
                                .push(catalog_assignment_function(p, "assign", &fxn_input)?);
                        }
                        #[cfg(feature = "matrix")]
                        [_, 1] => {
                            plan.borrow_mut()
                                .push(catalog_assignment_function(p, "assign", &fxn_input)?);
                        }
                        _ => todo!(),
                    }
                }
                #[cfg(all(feature = "matrix", feature = "subscript_range"))]
                [Subscript::All, Subscript::Range(_)] => {
                    fxn_input.push(source.clone());
                    fxn_input.push(LegacyValue::IndexAll);
                    let result = subscript_range(&subs[1], env, p)?;
                    fxn_input.push(result);
                    plan.borrow_mut()
                        .push(catalog_assignment_function(p, "assign", &fxn_input)?);
                }
                #[cfg(all(feature = "matrix", feature = "subscript_range"))]
                [Subscript::Range(_), Subscript::All] => {
                    fxn_input.push(source.clone());
                    let result = subscript_range(&subs[0], env, p)?;
                    fxn_input.push(result);
                    fxn_input.push(LegacyValue::IndexAll);
                    plan.borrow_mut()
                        .push(catalog_assignment_function(p, "assign", &fxn_input)?);
                }
                _ => unreachable!(),
            };
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
