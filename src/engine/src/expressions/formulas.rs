use super::environment::expression_solves_deferred;
use super::registration::{
    register_expression_function_batch, register_initialized_expression_function,
};
use super::{Environment, UnhandledFormulaOperatorError, expression};
#[cfg(feature = "subscript_formula")]
use super::{
    current_string_access_expression_live, mark_current_string_access_expression_live,
    mark_string_access_value_live, string_access_input_is_live,
};
#[cfg(all(
    feature = "kind_annotation",
    any(feature = "set_element_of", feature = "set_not_element_of"),
    feature = "enum",
    feature = "atom"
))]
use crate::ProgramState;
#[cfg(all(
    feature = "kind_annotation",
    any(feature = "set_element_of", feature = "set_not_element_of")
))]
use crate::Ref;
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
use crate::{
    AddSubOp, ComparisonOp, Factor, FormulaOperator, InterpreterExecution, LegacyValue, LogicOp,
    MResult, MechError, MechFunction, MulDivOp, OperationId, PowerOp, TableOp, Term, VecOp,
};
#[cfg(all(
    feature = "kind_annotation",
    any(feature = "set_element_of", feature = "set_not_element_of")
))]
use crate::{ValueKind, detach_value};

fn specialize_formula_operation(
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

pub fn factor(
    fctr: &Factor,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<LegacyValue> {
    match fctr {
        Factor::Term(trm) => {
            let result = term(trm, env, p)?;
            Ok(result)
        }
        Factor::Parenthetical(paren) => factor(&*paren, env, p),
        Factor::Expression(expr) => expression(expr, env, p),
        #[cfg(feature = "math_neg")]
        Factor::Negate(neg) => {
            let value = factor(neg, env, p)?;
            #[cfg(feature = "subscript_formula")]
            let value_is_live =
                current_string_access_expression_live(p) || string_access_input_is_live(&value, p);
            let arguments = vec![value];
            let function = specialize_formula_operation(p, "math/neg", &arguments)?;
            let plan = p.plan();
            let out = register_initialized_expression_function(&plan, function, &arguments)?;
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
            let function = specialize_formula_operation(p, "logic/not", &arguments)?;
            let plan = p.plan();
            let out = register_initialized_expression_function(&plan, function, &arguments)?;
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
            let function = specialize_formula_operation(p, "matrix/transpose", &arguments)?;
            let plan = p.plan();
            let out = register_initialized_expression_function(&plan, function, &arguments)?;
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

pub fn term(
    trm: &Term,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<LegacyValue> {
    let plan = p.plan();
    let mut lhs = factor(&trm.lhs, env, p)?;
    let mut term_plan: Vec<(Box<dyn MechFunction>, Vec<LegacyValue>)> = Vec::new();
    for (op, rhs) in &trm.rhs {
        let rhs = factor(&rhs, env, p)?;
        let dependency_arguments = vec![lhs.clone(), rhs.clone()];
        #[cfg(feature = "subscript_formula")]
        let new_fxn_is_live = current_string_access_expression_live(p)
            || string_access_input_is_live(&lhs, p)
            || string_access_input_is_live(&rhs, p);
        let new_fxn: Box<dyn MechFunction> = match op {
            // Math
            #[cfg(feature = "string_concat")]
            FormulaOperator::AddSub(AddSubOp::Add) if lhs.is_string() || rhs.is_string() => {
                specialize_formula_operation(p, "string/concat", &[lhs, rhs])?
            }
            #[cfg(feature = "math_add")]
            FormulaOperator::AddSub(AddSubOp::Add) => {
                specialize_formula_operation(p, "math/add", &[lhs, rhs])?
            }
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
                #[cfg(feature = "kind_annotation")]
                if let LegacyValue::Kind(kind) = &rhs {
                    lhs = LegacyValue::Bool(Ref::new(value_in_kind(&lhs, kind, p)));
                    continue;
                }
                specialize_formula_operation(p, "set/element-of", &[lhs, rhs])?
            }
            #[cfg(feature = "set_not_element_of")]
            FormulaOperator::Set(SetOp::NotElementOf) => {
                #[cfg(feature = "kind_annotation")]
                if let LegacyValue::Kind(kind) = &rhs {
                    lhs = LegacyValue::Bool(Ref::new(!value_in_kind(&lhs, kind, p)));
                    continue;
                }
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
        };
        if !expression_solves_deferred(p) {
            new_fxn.solve_result()?;
        }
        let res = new_fxn.out();
        #[cfg(feature = "subscript_formula")]
        if new_fxn_is_live {
            mark_current_string_access_expression_live(p);
            mark_string_access_value_live(p, &res);
        }
        term_plan.push((new_fxn, dependency_arguments));
        lhs = res;
    }
    register_expression_function_batch(&plan, term_plan)?;
    Ok(lhs)
}

#[cfg(all(
    feature = "kind_annotation",
    any(feature = "set_element_of", feature = "set_not_element_of"),
    feature = "enum",
    feature = "atom"
))]
fn enum_value_matches_kind(value: &LegacyValue, enum_id: u64, state: &ProgramState) -> bool {
    let enum_def = match state.enums.get(&enum_id) {
        Some(enm) => enm,
        None => return false,
    };
    let names_brrw = enum_def.names.borrow();
    let atom_matches_variant = |variant_id: u64, atom_id: u64, atom_name: &str| {
        if variant_id == atom_id {
            return true;
        }
        let variant_name = match names_brrw.get(&variant_id) {
            Some(name) => name.as_str(),
            None => return false,
        };
        let short_variant = variant_name.rsplit('/').next().unwrap_or(variant_name);
        let short_atom = atom_name.rsplit('/').next().unwrap_or(atom_name);
        short_variant == short_atom
    };
    match value {
        LegacyValue::Enum(enum_value) => {
            let enum_value_brrw = enum_value.borrow();
            if enum_value_brrw.id != enum_id {
                return false;
            }
            if enum_value_brrw.variants.len() != 1 {
                return false;
            }
            let (variant_id, payload) = &enum_value_brrw.variants[0];
            let (_, declared_payload_kind) = match enum_def
                .variants
                .iter()
                .find(|(known_variant, _)| *known_variant == *variant_id)
            {
                Some(entry) => entry,
                None => return false,
            };
            match (payload, declared_payload_kind) {
                (None, None) => true,
                (Some(payload_value), Some(LegacyValue::Kind(expected_kind))) => {
                    match expected_kind {
                        ValueKind::Enum(inner_enum_id, _) => {
                            enum_value_matches_kind(payload_value, *inner_enum_id, state)
                        }
                        _ => {
                            payload_value.kind() == expected_kind.clone()
                                || payload_value.convert_to(expected_kind).is_some()
                        }
                    }
                }
                _ => false,
            }
        }
        LegacyValue::Atom(atom) => {
            let atom_brrw = atom.borrow();
            let variant_id = atom_brrw.id();
            let atom_name = atom_brrw.name();
            enum_def
                .variants
                .iter()
                .any(|(known_variant, payload_kind)| {
                    atom_matches_variant(*known_variant, variant_id, &atom_name)
                        && payload_kind.is_none()
                })
        }
        #[cfg(feature = "tuple")]
        LegacyValue::Tuple(tuple_val) => {
            let tuple_brrw = tuple_val.borrow();
            if tuple_brrw.elements.len() != 2 {
                return false;
            }
            let (tag, tag_name) = match tuple_brrw.elements[0].as_ref() {
                LegacyValue::Atom(atom) => {
                    let atom_brrw = atom.borrow();
                    (atom_brrw.id(), atom_brrw.name())
                }
                _ => return false,
            };
            let payload = tuple_brrw.elements[1].as_ref();
            let (_, declared_payload_kind) = match enum_def
                .variants
                .iter()
                .find(|(known_variant, _)| atom_matches_variant(*known_variant, tag, &tag_name))
            {
                Some(entry) => entry,
                None => return false,
            };
            match declared_payload_kind {
                Some(LegacyValue::Kind(expected_kind)) => match expected_kind {
                    ValueKind::Enum(inner_enum_id, _) => {
                        enum_value_matches_kind(payload, *inner_enum_id, state)
                    }
                    _ => {
                        payload.kind() == expected_kind.clone()
                            || payload.convert_to(expected_kind).is_some()
                    }
                },
                _ => false,
            }
        }
        _ => false,
    }
}

#[cfg(all(
    feature = "kind_annotation",
    any(feature = "set_element_of", feature = "set_not_element_of")
))]
fn value_in_kind(value: &LegacyValue, kind: &ValueKind, p: &InterpreterExecution<'_>) -> bool {
    let detached = detach_value(value);
    #[cfg(all(feature = "enum", feature = "atom"))]
    if let ValueKind::Enum(enum_id, _) = kind {
        let state_brrw = p.state.borrow();
        return enum_value_matches_kind(&detached, *enum_id, &state_brrw);
    }
    detached.convert_to(kind).is_some()
}
