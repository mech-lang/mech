use super::super::environment::expression_solves_deferred;
#[cfg(feature = "subscript_formula")]
use super::string::{
    current_string_access_expression_live, string_access_argument_is_live,
    string_access_index_argument, string_access_source_argument,
};
use super::{
    Environment, catalog_access_function, subscript_formula, subscript_formula_ix, subscript_range,
};
use crate::{InterpreterExecution, LegacyValue, MResult, Subscript, ValueKind};
#[cfg(feature = "subscript_formula")]
use crate::{StringAccessCompileMode, set_next_string_access_compile_mode};

pub(super) fn access(
    sbscrpt: &Subscript,
    val: &LegacyValue,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<LegacyValue> {
    let plan = p.plan();
    match sbscrpt {
        #[cfg(feature = "subscript_slice")]
        Subscript::Bracket(subs) => {
            let string_source_is_live = matches!(val.deref_kind(), ValueKind::String)
                && string_access_argument_is_live(val, p);
            let mut fxn_input = if matches!(val.deref_kind(), ValueKind::String) {
                vec![string_access_source_argument(val, p)]
            } else {
                vec![val.clone()]
            };
            match &subs[..] {
                #[cfg(feature = "subscript_formula")]
                [Subscript::Formula(ix)] => {
                    let raw_index = subscript_formula(&subs[0], env, p)?;
                    let index_arg = if matches!(val.deref_kind(), ValueKind::String) {
                        string_access_index_argument(raw_index, &subs[0], env, p)?
                    } else {
                        super::subscript_formula_index(&raw_index, p)?
                    };
                    if matches!(val.deref_kind(), ValueKind::String)
                        && matches!(fxn_input.first(), Some(LegacyValue::String(_)))
                        && matches!(&index_arg, LegacyValue::Index(_))
                    {
                        let mode = if current_string_access_expression_live(p)
                            || string_source_is_live
                            || string_access_argument_is_live(&index_arg, p)
                        {
                            StringAccessCompileMode::LiveDirect
                        } else {
                            StringAccessCompileMode::Constant
                        };
                        set_next_string_access_compile_mode(mode);
                    }
                    let shape = index_arg.shape();
                    fxn_input.push(index_arg);
                    match shape[..] {
                        [1, 1] => {
                            plan.borrow_mut().push(catalog_access_function(
                                p,
                                "access/scalar",
                                &fxn_input,
                            )?);
                        }
                        #[cfg(feature = "subscript_range")]
                        [1, n] => {
                            plan.borrow_mut().push(catalog_access_function(
                                p,
                                "access/range",
                                &fxn_input,
                            )?);
                        }
                        #[cfg(feature = "subscript_range")]
                        [n, 1] => {
                            plan.borrow_mut().push(catalog_access_function(
                                p,
                                "access/range",
                                &fxn_input,
                            )?);
                        }
                        _ => todo!(),
                    }
                }
                #[cfg(feature = "subscript_range")]
                [Subscript::Range(ix)] => {
                    let result = subscript_range(&subs[0], env, p)?;
                    fxn_input.push(result);
                    plan.borrow_mut()
                        .push(catalog_access_function(p, "access/range", &fxn_input)?);
                }
                [Subscript::All] => {
                    fxn_input.push(LegacyValue::IndexAll);
                    #[cfg(feature = "matrix")]
                    plan.borrow_mut().push(catalog_access_function(
                        p,
                        "access/scalar",
                        &fxn_input,
                    )?);
                }
                [Subscript::All, Subscript::All] => todo!(),
                #[cfg(feature = "subscript_formula")]
                [Subscript::Formula(ix1), Subscript::Formula(ix2)] => {
                    let result = subscript_formula_ix(&subs[0], env, p)?;
                    let shape1 = result.shape();
                    fxn_input.push(result);
                    let result = subscript_formula_ix(&subs[1], env, p)?;
                    let shape2 = result.shape();
                    fxn_input.push(result);
                    match ((shape1[0], shape1[1]), (shape2[0], shape2[1])) {
                        #[cfg(feature = "matrix")]
                        ((1, 1), (1, 1)) => {
                            plan.borrow_mut().push(catalog_access_function(
                                p,
                                "access/scalar",
                                &fxn_input,
                            )?);
                        }
                        #[cfg(feature = "matrix")]
                        ((1, 1), (m, 1)) => {
                            plan.borrow_mut().push(catalog_access_function(
                                p,
                                "access/range",
                                &fxn_input,
                            )?);
                        }
                        #[cfg(feature = "matrix")]
                        ((n, 1), (1, 1)) => {
                            plan.borrow_mut().push(catalog_access_function(
                                p,
                                "access/range",
                                &fxn_input,
                            )?);
                        }
                        #[cfg(feature = "matrix")]
                        ((n, 1), (m, 1)) => {
                            plan.borrow_mut().push(catalog_access_function(
                                p,
                                "access/range",
                                &fxn_input,
                            )?);
                        }
                        _ => unreachable!(),
                    }
                }
                #[cfg(feature = "subscript_range")]
                [Subscript::Range(ix1), Subscript::Range(ix2)] => {
                    let result = subscript_range(&subs[0], env, p)?;
                    fxn_input.push(result);
                    let result = subscript_range(&subs[1], env, p)?;
                    fxn_input.push(result);
                    #[cfg(feature = "matrix")]
                    plan.borrow_mut()
                        .push(catalog_access_function(p, "access/range", &fxn_input)?);
                }
                #[cfg(all(feature = "subscript_range", feature = "subscript_formula"))]
                [Subscript::All, Subscript::Formula(ix2)] => {
                    fxn_input.push(LegacyValue::IndexAll);
                    let result = subscript_formula_ix(&subs[1], env, p)?;
                    let shape = result.shape();
                    fxn_input.push(result);
                    match &shape[..] {
                        #[cfg(feature = "matrix")]
                        [1, 1] => {
                            plan.borrow_mut().push(catalog_access_function(
                                p,
                                "access/scalar",
                                &fxn_input,
                            )?);
                        }
                        #[cfg(feature = "matrix")]
                        [1, n] => {
                            plan.borrow_mut().push(catalog_access_function(
                                p,
                                "access/range",
                                &fxn_input,
                            )?);
                        }
                        #[cfg(feature = "matrix")]
                        [n, 1] => {
                            plan.borrow_mut().push(catalog_access_function(
                                p,
                                "access/range",
                                &fxn_input,
                            )?);
                        }
                        _ => todo!(),
                    }
                }
                #[cfg(all(feature = "subscript_range", feature = "subscript_formula"))]
                [Subscript::Formula(ix1), Subscript::All] => {
                    let result = subscript_formula_ix(&subs[0], env, p)?;
                    let shape = result.shape();
                    fxn_input.push(result);
                    fxn_input.push(LegacyValue::IndexAll);
                    match &shape[..] {
                        #[cfg(feature = "matrix")]
                        [1, 1] => {
                            plan.borrow_mut().push(catalog_access_function(
                                p,
                                "access/scalar",
                                &fxn_input,
                            )?);
                        }
                        #[cfg(feature = "matrix")]
                        [1, n] => {
                            plan.borrow_mut().push(catalog_access_function(
                                p,
                                "access/range",
                                &fxn_input,
                            )?);
                        }
                        #[cfg(feature = "matrix")]
                        [n, 1] => {
                            plan.borrow_mut().push(catalog_access_function(
                                p,
                                "access/range",
                                &fxn_input,
                            )?);
                        }
                        _ => todo!(),
                    }
                }
                #[cfg(all(feature = "subscript_range", feature = "subscript_formula"))]
                [Subscript::Range(ix1), Subscript::Formula(ix2)] => {
                    let result = subscript_range(&subs[0], env, p)?;
                    fxn_input.push(result);
                    let result = subscript_formula_ix(&subs[1], env, p)?;
                    let shape = result.shape();
                    fxn_input.push(result);
                    match &shape[..] {
                        #[cfg(feature = "matrix")]
                        [1, 1] => {
                            plan.borrow_mut().push(catalog_access_function(
                                p,
                                "access/range",
                                &fxn_input,
                            )?);
                        }
                        #[cfg(feature = "matrix")]
                        [1, n] => {
                            plan.borrow_mut().push(catalog_access_function(
                                p,
                                "access/range",
                                &fxn_input,
                            )?);
                        }
                        #[cfg(feature = "matrix")]
                        [n, 1] => {
                            plan.borrow_mut().push(catalog_access_function(
                                p,
                                "access/range",
                                &fxn_input,
                            )?);
                        }
                        _ => todo!(),
                    }
                }
                #[cfg(all(feature = "subscript_range", feature = "subscript_formula"))]
                [Subscript::Formula(ix1), Subscript::Range(ix2)] => {
                    let result = subscript_formula_ix(&subs[0], env, p)?;
                    let shape = result.shape();
                    fxn_input.push(result);
                    let result = subscript_range(&subs[1], env, p)?;
                    fxn_input.push(result);
                    match &shape[..] {
                        #[cfg(feature = "matrix")]
                        [1, 1] => {
                            plan.borrow_mut().push(catalog_access_function(
                                p,
                                "access/range",
                                &fxn_input,
                            )?);
                        }
                        #[cfg(feature = "matrix")]
                        [1, n] => {
                            plan.borrow_mut().push(catalog_access_function(
                                p,
                                "access/range",
                                &fxn_input,
                            )?);
                        }
                        #[cfg(feature = "matrix")]
                        [n, 1] => {
                            plan.borrow_mut().push(catalog_access_function(
                                p,
                                "access/range",
                                &fxn_input,
                            )?);
                        }
                        _ => todo!(),
                    }
                }
                #[cfg(feature = "subscript_range")]
                [Subscript::All, Subscript::Range(ix2)] => {
                    fxn_input.push(LegacyValue::IndexAll);
                    let result = subscript_range(&subs[1], env, p)?;
                    fxn_input.push(result);
                    #[cfg(feature = "matrix")]
                    plan.borrow_mut()
                        .push(catalog_access_function(p, "access/range", &fxn_input)?);
                }
                #[cfg(feature = "subscript_range")]
                [Subscript::Range(ix1), Subscript::All] => {
                    let result = subscript_range(&subs[0], env, p)?;
                    fxn_input.push(result);
                    fxn_input.push(LegacyValue::IndexAll);
                    #[cfg(feature = "matrix")]
                    plan.borrow_mut()
                        .push(catalog_access_function(p, "access/range", &fxn_input)?);
                }
                _ => unreachable!(),
            };
            let plan_brrw = plan.borrow();
            let mut new_fxn = &plan_brrw.last().unwrap();
            if !expression_solves_deferred(p) {
                new_fxn.solve_result()?;
            }
            let res = new_fxn.out();
            return Ok(res);
        }
        _ => unreachable!(),
    }
}
