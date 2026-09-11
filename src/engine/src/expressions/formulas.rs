#[cfg(any(
    feature = "string_concat",
    feature = "math_add",
    feature = "math_sub",
    feature = "math_mul",
    feature = "math_div",
    feature = "math_mod",
    feature = "math_pow",
    feature = "matrix_matmul",
    feature = "matrix_solve",
    feature = "matrix_dot",
    feature = "compare_eq",
    feature = "compare_seq",
    feature = "compare_neq",
    feature = "compare_sneq",
    feature = "compare_lte",
    feature = "compare_gte",
    feature = "compare_lt",
    feature = "compare_gt",
    feature = "logic_and",
    feature = "logic_or",
    feature = "logic_not",
    feature = "logic_xor",
    feature = "table",
    feature = "set_union",
    feature = "set_intersection",
    feature = "set_difference",
    feature = "set_symmetric_difference",
    feature = "set_subset",
    feature = "set_superset",
    feature = "set_proper_subset",
    feature = "set_proper_superset",
    feature = "set_element_of",
    feature = "set_not_element_of"
))]
use super::environment::expression_solves_deferred;
#[cfg(any(
    feature = "string_concat",
    feature = "math_add",
    feature = "math_sub",
    feature = "math_mul",
    feature = "math_div",
    feature = "math_mod",
    feature = "math_pow",
    feature = "matrix_matmul",
    feature = "matrix_solve",
    feature = "matrix_dot",
    feature = "compare_eq",
    feature = "compare_seq",
    feature = "compare_neq",
    feature = "compare_sneq",
    feature = "compare_lte",
    feature = "compare_gte",
    feature = "compare_lt",
    feature = "compare_gt",
    feature = "logic_and",
    feature = "logic_or",
    feature = "logic_not",
    feature = "logic_xor",
    feature = "table",
    feature = "set_union",
    feature = "set_intersection",
    feature = "set_difference",
    feature = "set_symmetric_difference",
    feature = "set_subset",
    feature = "set_superset",
    feature = "set_proper_subset",
    feature = "set_proper_superset",
    feature = "set_element_of",
    feature = "set_not_element_of"
))]
use super::registration::register_expression_function_batch;
#[cfg(any(
    feature = "math_neg",
    feature = "logic_not",
    feature = "matrix_transpose"
))]
use super::registration::register_initialized_expression_function;
use super::{Environment, UnhandledFormulaOperatorError, expression_cell};
#[cfg(feature = "subscript_formula")]
use super::{
    current_string_access_expression_live, mark_current_string_access_expression_live,
    mark_string_access_value_live, string_access_input_is_live,
};
#[cfg(any(feature = "string_concat", feature = "math_add", feature = "math_sub"))]
use crate::AddSubOp;
#[cfg(any(
    feature = "compare_eq",
    feature = "compare_seq",
    feature = "compare_neq",
    feature = "compare_sneq",
    feature = "compare_lte",
    feature = "compare_gte",
    feature = "compare_lt",
    feature = "compare_gt"
))]
use crate::ComparisonOp;
#[cfg(any(
    feature = "logic_and",
    feature = "logic_or",
    feature = "logic_not",
    feature = "logic_xor"
))]
use crate::LogicOp;
#[cfg(any(feature = "math_mul", feature = "math_div", feature = "math_mod"))]
use crate::MulDivOp;
#[cfg(feature = "math_pow")]
use crate::PowerOp;
#[cfg(any(
    feature = "set_union",
    feature = "set_intersection",
    feature = "set_difference",
    feature = "set_symmetric_difference",
    feature = "set_subset",
    feature = "set_superset",
    feature = "set_proper_subset",
    feature = "set_proper_superset",
    feature = "set_element_of",
    feature = "set_not_element_of"
))]
use crate::SetOp;
#[cfg(feature = "table")]
use crate::TableOp;
#[cfg(any(
    feature = "matrix_matmul",
    feature = "matrix_solve",
    feature = "matrix_dot"
))]
use crate::VecOp;
use crate::{Factor, InterpreterExecution, MResult, MechError, Term, ValueCell};
#[cfg(any(
    feature = "math_neg",
    feature = "matrix_transpose",
    feature = "string_concat",
    feature = "math_add",
    feature = "math_sub",
    feature = "math_mul",
    feature = "math_div",
    feature = "math_mod",
    feature = "math_pow",
    feature = "matrix_matmul",
    feature = "matrix_solve",
    feature = "matrix_dot",
    feature = "compare_eq",
    feature = "compare_seq",
    feature = "compare_neq",
    feature = "compare_sneq",
    feature = "compare_lte",
    feature = "compare_gte",
    feature = "compare_lt",
    feature = "compare_gt",
    feature = "logic_and",
    feature = "logic_or",
    feature = "logic_not",
    feature = "logic_xor",
    feature = "table",
    feature = "set_union",
    feature = "set_intersection",
    feature = "set_difference",
    feature = "set_symmetric_difference",
    feature = "set_subset",
    feature = "set_superset",
    feature = "set_proper_subset",
    feature = "set_proper_superset",
    feature = "set_element_of",
    feature = "set_not_element_of"
))]
use crate::{FormulaOperator, OperationId};
#[cfg(any(
    feature = "math_neg",
    feature = "matrix_transpose",
    feature = "string_concat",
    feature = "math_add",
    feature = "math_sub",
    feature = "math_mul",
    feature = "math_div",
    feature = "math_mod",
    feature = "math_pow",
    feature = "matrix_matmul",
    feature = "matrix_solve",
    feature = "matrix_dot",
    feature = "compare_eq",
    feature = "compare_seq",
    feature = "compare_neq",
    feature = "compare_sneq",
    feature = "compare_lte",
    feature = "compare_gte",
    feature = "compare_lt",
    feature = "compare_gt",
    feature = "logic_and",
    feature = "logic_or",
    feature = "logic_not",
    feature = "logic_xor",
    feature = "table",
    feature = "set_union",
    feature = "set_intersection",
    feature = "set_difference",
    feature = "set_symmetric_difference",
    feature = "set_subset",
    feature = "set_superset",
    feature = "set_proper_subset",
    feature = "set_proper_superset",
    feature = "set_element_of",
    feature = "set_not_element_of"
))]
fn specialize_formula_operation(
    p: &InterpreterExecution<'_>,
    canonical_name: &str,
    arguments: &[ValueCell],
) -> MResult<crate::SpecializedFunction> {
    let invocation =
        crate::SpecializationInvocation::from_cells(arguments.to_vec().into_boxed_slice());
    p.specialize_visible_invocation_named(
        OperationId::from_name(canonical_name),
        Some(canonical_name),
        &invocation,
    )
}

#[cfg(any(feature = "string_concat", feature = "math_add"))]
fn specialize_add_operation(
    p: &InterpreterExecution<'_>,
    lhs: &ValueCell,
    rhs: &ValueCell,
) -> MResult<crate::SpecializedFunction> {
    #[cfg(all(feature = "string_concat", feature = "math_add"))]
    {
        let invocation = crate::SpecializationInvocation::from_cells(
            vec![lhs.clone(), rhs.clone()].into_boxed_slice(),
        );
        let string_name = "string/concat";
        if p.operation_semantically_accepts(
            OperationId::from_name(string_name),
            string_name,
            &invocation,
        )? {
            return specialize_formula_operation(p, string_name, &[lhs.clone(), rhs.clone()]);
        }
        return specialize_formula_operation(p, "math/add", &[lhs.clone(), rhs.clone()]);
    }
    #[cfg(all(feature = "string_concat", not(feature = "math_add")))]
    {
        return specialize_formula_operation(p, "string/concat", &[lhs.clone(), rhs.clone()]);
    }
    #[cfg(all(feature = "math_add", not(feature = "string_concat")))]
    {
        return specialize_formula_operation(p, "math/add", &[lhs.clone(), rhs.clone()]);
    }
}

pub fn factor(
    fctr: &Factor,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    match fctr {
        Factor::Term(trm) => {
            let result = term(trm, env, p)?;
            Ok(result)
        }
        Factor::Parenthetical(paren) => factor(&*paren, env, p),
        Factor::Expression(expr) => expression_cell(expr, env, p),
        #[cfg(feature = "math_neg")]
        Factor::Negate(neg) => {
            let value = factor(neg, env, p)?;
            #[cfg(feature = "subscript_formula")]
            let value_is_live =
                current_string_access_expression_live(p) || string_access_input_is_live(&value, p);
            let arguments = vec![value];
            let function = specialize_formula_operation(p, "math/neg", &arguments)
                .map_err(|error| error.with_tokens(fctr.tokens()))?;
            let plan = p.plan();
            let out = register_initialized_expression_function(&plan, function)?;
            #[cfg(feature = "subscript_formula")]
            if value_is_live {
                mark_current_string_access_expression_live(p);
                mark_string_access_value_live(p, &out);
            }
            Ok(out)
        }
        #[cfg(feature = "logic_not")]
        Factor::Not(neg) => {
            let value = factor(neg, env, p)?;
            #[cfg(feature = "subscript_formula")]
            let value_is_live =
                current_string_access_expression_live(p) || string_access_input_is_live(&value, p);
            let arguments = vec![value];
            let function = specialize_formula_operation(p, "logic/not", &arguments)
                .map_err(|error| error.with_tokens(fctr.tokens()))?;
            let plan = p.plan();
            let out = register_initialized_expression_function(&plan, function)?;
            #[cfg(feature = "subscript_formula")]
            if value_is_live {
                mark_current_string_access_expression_live(p);
                mark_string_access_value_live(p, &out);
            }
            Ok(out)
        }
        #[cfg(feature = "matrix_transpose")]
        Factor::Transpose(fctr) => {
            let value = factor(fctr, env, p)?;
            #[cfg(feature = "subscript_formula")]
            let value_is_live =
                current_string_access_expression_live(p) || string_access_input_is_live(&value, p);
            let arguments = vec![value];
            let function = specialize_formula_operation(p, "matrix/transpose", &arguments)
                .map_err(|error| error.with_tokens(fctr.tokens()))?;
            let plan = p.plan();
            let out = register_initialized_expression_function(&plan, function)?;
            #[cfg(feature = "subscript_formula")]
            if value_is_live {
                mark_current_string_access_expression_live(p);
                mark_string_access_value_live(p, &out);
            }
            Ok(out)
        }
        #[cfg(not(all(
            feature = "math_neg",
            feature = "logic_not",
            feature = "matrix_transpose"
        )))]
        _ => todo!(),
    }
}

#[cfg(any(
    feature = "string_concat",
    feature = "math_add",
    feature = "math_sub",
    feature = "math_mul",
    feature = "math_div",
    feature = "math_mod",
    feature = "math_pow",
    feature = "matrix_matmul",
    feature = "matrix_solve",
    feature = "matrix_dot",
    feature = "compare_eq",
    feature = "compare_seq",
    feature = "compare_neq",
    feature = "compare_sneq",
    feature = "compare_lte",
    feature = "compare_gte",
    feature = "compare_lt",
    feature = "compare_gt",
    feature = "logic_and",
    feature = "logic_or",
    feature = "logic_not",
    feature = "logic_xor",
    feature = "table",
    feature = "set_union",
    feature = "set_intersection",
    feature = "set_difference",
    feature = "set_symmetric_difference",
    feature = "set_subset",
    feature = "set_superset",
    feature = "set_proper_subset",
    feature = "set_proper_superset",
    feature = "set_element_of",
    feature = "set_not_element_of"
))]
pub fn term(
    trm: &Term,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let plan = p.plan();
    let mut lhs = factor(&trm.lhs, env, p)?;
    let mut term_plan: Vec<crate::SpecializedFunction> = Vec::new();
    for (op, rhs) in &trm.rhs {
        let rhs = factor(&rhs, env, p)?;
        #[cfg(feature = "subscript_formula")]
        let new_fxn_is_live = current_string_access_expression_live(p)
            || string_access_input_is_live(&lhs, p)
            || string_access_input_is_live(&rhs, p);
        let new_fxn = (|| -> MResult<crate::SpecializedFunction> {
            Ok(match op {
                // Math
                #[cfg(any(feature = "string_concat", feature = "math_add"))]
                FormulaOperator::AddSub(AddSubOp::Add) => specialize_add_operation(p, &lhs, &rhs)?,
                #[cfg(feature = "math_sub")]
                FormulaOperator::AddSub(AddSubOp::Sub) => {
                    specialize_formula_operation(p, "math/sub", &[lhs, rhs])?
                }
                #[cfg(feature = "math_mul")]
                FormulaOperator::MulDiv(MulDivOp::Mul) => {
                    specialize_formula_operation(p, "math/mul", &[lhs, rhs])?
                }
                #[cfg(feature = "math_div")]
                FormulaOperator::MulDiv(MulDivOp::Div) => {
                    specialize_formula_operation(p, "math/div", &[lhs, rhs])?
                }
                #[cfg(feature = "math_mod")]
                FormulaOperator::MulDiv(MulDivOp::Mod) => {
                    specialize_formula_operation(p, "math/mod", &[lhs, rhs])?
                }
                #[cfg(feature = "math_pow")]
                FormulaOperator::Power(PowerOp::Pow) => {
                    specialize_formula_operation(p, "math/pow", &[lhs, rhs])?
                }

                // Matrix
                #[cfg(feature = "matrix_matmul")]
                FormulaOperator::Vec(VecOp::MatMul) => {
                    specialize_formula_operation(p, "matrix/matmul", &[lhs, rhs])?
                }
                #[cfg(feature = "matrix_solve")]
                FormulaOperator::Vec(VecOp::Solve) => {
                    specialize_formula_operation(p, "matrix/solve", &[lhs, rhs])?
                }
                #[cfg(feature = "matrix_dot")]
                FormulaOperator::Vec(VecOp::Dot) => {
                    specialize_formula_operation(p, "matrix/dot", &[lhs, rhs])?
                }

                // Compare
                #[cfg(feature = "compare_eq")]
                FormulaOperator::Comparison(ComparisonOp::Equal) => {
                    specialize_formula_operation(p, "compare/eq", &[lhs, rhs])?
                }
                #[cfg(feature = "compare_seq")]
                FormulaOperator::Comparison(ComparisonOp::StrictEqual) => {
                    specialize_formula_operation(p, "compare/seq", &[lhs, rhs])?
                }
                #[cfg(feature = "compare_neq")]
                FormulaOperator::Comparison(ComparisonOp::NotEqual) => {
                    specialize_formula_operation(p, "compare/neq", &[lhs, rhs])?
                }
                #[cfg(feature = "compare_sneq")]
                FormulaOperator::Comparison(ComparisonOp::StrictNotEqual) => {
                    specialize_formula_operation(p, "compare/sneq", &[lhs, rhs])?
                }
                #[cfg(feature = "compare_lte")]
                FormulaOperator::Comparison(ComparisonOp::LessThanEqual) => {
                    specialize_formula_operation(p, "compare/lte", &[lhs, rhs])?
                }
                #[cfg(feature = "compare_gte")]
                FormulaOperator::Comparison(ComparisonOp::GreaterThanEqual) => {
                    specialize_formula_operation(p, "compare/gte", &[lhs, rhs])?
                }
                #[cfg(feature = "compare_lt")]
                FormulaOperator::Comparison(ComparisonOp::LessThan) => {
                    specialize_formula_operation(p, "compare/lt", &[lhs, rhs])?
                }
                #[cfg(feature = "compare_gt")]
                FormulaOperator::Comparison(ComparisonOp::GreaterThan) => {
                    specialize_formula_operation(p, "compare/gt", &[lhs, rhs])?
                }

                // Logic
                #[cfg(feature = "logic_and")]
                FormulaOperator::Logic(LogicOp::And) => {
                    specialize_formula_operation(p, "logic/and", &[lhs, rhs])?
                }
                #[cfg(feature = "logic_or")]
                FormulaOperator::Logic(LogicOp::Or) => {
                    specialize_formula_operation(p, "logic/or", &[lhs, rhs])?
                }
                #[cfg(feature = "logic_not")]
                FormulaOperator::Logic(LogicOp::Not) => {
                    specialize_formula_operation(p, "logic/not", &[lhs, rhs])?
                }
                #[cfg(feature = "logic_xor")]
                FormulaOperator::Logic(LogicOp::Xor) => {
                    specialize_formula_operation(p, "logic/xor", &[lhs, rhs])?
                }

                // Table
                #[cfg(feature = "table")]
                FormulaOperator::Table(TableOp::InnerJoin) => {
                    specialize_formula_operation(p, "table/join", &[lhs, rhs])?
                }
                #[cfg(feature = "table")]
                FormulaOperator::Table(TableOp::LeftOuterJoin) => {
                    specialize_formula_operation(p, "table/left-outer-join", &[lhs, rhs])?
                }
                #[cfg(feature = "table")]
                FormulaOperator::Table(TableOp::RightOuterJoin) => {
                    specialize_formula_operation(p, "table/right-outer-join", &[lhs, rhs])?
                }
                #[cfg(feature = "table")]
                FormulaOperator::Table(TableOp::FullOuterJoin) => {
                    specialize_formula_operation(p, "table/full-outer-join", &[lhs, rhs])?
                }
                #[cfg(feature = "table")]
                FormulaOperator::Table(TableOp::LeftSemiJoin) => {
                    specialize_formula_operation(p, "table/left-semi-join", &[lhs, rhs])?
                }
                #[cfg(feature = "table")]
                FormulaOperator::Table(TableOp::LeftAntiJoin) => {
                    specialize_formula_operation(p, "table/left-anti-join", &[lhs, rhs])?
                }

                // Set
                #[cfg(feature = "set_union")]
                FormulaOperator::Set(SetOp::Union) => {
                    specialize_formula_operation(p, "set/union", &[lhs, rhs])?
                }
                #[cfg(feature = "set_intersection")]
                FormulaOperator::Set(SetOp::Intersection) => {
                    specialize_formula_operation(p, "set/intersection", &[lhs, rhs])?
                }
                #[cfg(feature = "set_difference")]
                FormulaOperator::Set(SetOp::Difference) => {
                    specialize_formula_operation(p, "set/difference", &[lhs, rhs])?
                }
                #[cfg(feature = "set_symmetric_difference")]
                FormulaOperator::Set(SetOp::SymmetricDifference) => {
                    specialize_formula_operation(p, "set/symmetric-difference", &[lhs, rhs])?
                }
                #[cfg(feature = "set_subset")]
                FormulaOperator::Set(SetOp::Subset) => {
                    specialize_formula_operation(p, "set/subset", &[lhs, rhs])?
                }
                #[cfg(feature = "set_superset")]
                FormulaOperator::Set(SetOp::Superset) => {
                    specialize_formula_operation(p, "set/superset", &[lhs, rhs])?
                }
                #[cfg(feature = "set_proper_subset")]
                FormulaOperator::Set(SetOp::ProperSubset) => {
                    specialize_formula_operation(p, "set/proper_subset", &[lhs, rhs])?
                }
                #[cfg(feature = "set_proper_superset")]
                FormulaOperator::Set(SetOp::ProperSuperset) => {
                    specialize_formula_operation(p, "set/proper-superset", &[lhs, rhs])?
                }
                #[cfg(feature = "set_element_of")]
                FormulaOperator::Set(SetOp::ElementOf) => {
                    specialize_formula_operation(p, "set/element-of", &[lhs, rhs])?
                }
                #[cfg(feature = "set_not_element_of")]
                FormulaOperator::Set(SetOp::NotElementOf) => {
                    specialize_formula_operation(p, "set/not-element-of", &[lhs, rhs])?
                }
                x => {
                    return Err(MechError::new(
                        UnhandledFormulaOperatorError {
                            operator: x.clone(),
                        },
                        None,
                    )
                    .with_compiler_loc()
                    .with_tokens(trm.tokens()));
                }
            })
        })()
        .map_err(|error| error.with_tokens(trm.tokens()))?;
        if !expression_solves_deferred(p) {
            if let Err(mut error) = new_fxn.instance().solve_result() {
                if let Some(operation) = new_fxn
                    .instance()
                    .implementation()
                    .semantic_operation_name()
                {
                    error.message = Some(format!(
                        "semantic operation `{operation}` failed: {}",
                        error.display_message(),
                    ));
                }
                return Err(error.with_tokens(trm.tokens()));
            }
        }
        let res = new_fxn.output().clone();
        #[cfg(feature = "subscript_formula")]
        if new_fxn_is_live {
            mark_current_string_access_expression_live(p);
            mark_string_access_value_live(p, &res);
        }
        term_plan.push(new_fxn);
        lhs = res;
    }
    register_expression_function_batch(&plan, term_plan)?;
    Ok(lhs)
}

#[cfg(not(any(
    feature = "string_concat",
    feature = "math_add",
    feature = "math_sub",
    feature = "math_mul",
    feature = "math_div",
    feature = "math_mod",
    feature = "math_pow",
    feature = "matrix_matmul",
    feature = "matrix_solve",
    feature = "matrix_dot",
    feature = "compare_eq",
    feature = "compare_seq",
    feature = "compare_neq",
    feature = "compare_sneq",
    feature = "compare_lte",
    feature = "compare_gte",
    feature = "compare_lt",
    feature = "compare_gt",
    feature = "logic_and",
    feature = "logic_or",
    feature = "logic_not",
    feature = "logic_xor",
    feature = "table",
    feature = "set_union",
    feature = "set_intersection",
    feature = "set_difference",
    feature = "set_symmetric_difference",
    feature = "set_subset",
    feature = "set_superset",
    feature = "set_proper_subset",
    feature = "set_proper_superset",
    feature = "set_element_of",
    feature = "set_not_element_of"
)))]
pub fn term(
    trm: &Term,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let lhs = factor(&trm.lhs, env, p)?;
    match trm.rhs.first() {
        Some((operator, _)) => Err(MechError::new(
            UnhandledFormulaOperatorError {
                operator: operator.clone(),
            },
            None,
        )
        .with_compiler_loc()
        .with_tokens(trm.tokens())),
        None => Ok(lhs),
    }
}
