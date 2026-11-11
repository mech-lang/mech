//! Fold repeated Boolean tests only along an unchanged, sole-predecessor edge.
//! This retains every source visit/action and all unknown-value error paths.
use super::super::control::plan::CodePlan;
use crate::native_artifact::codegen::options::{OptimizationOptions, OptimizationPass};
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeIterationTerminator as Term,
    NativeMatchAction as Scalar, NativeMatchTerminator as Branch, NativeMatchValue as Value,
    NativeScalarKind as Kind, NativeScalarOp as Op,
};

fn boolean_constant(code: CodePlan<'_>, value: Value) -> Option<bool> {
    if code.kind(value) != Some(Kind::Bool) {
        return None;
    }
    (0..code.constant_count()).find_map(|i| {
        let (at, word) = code.constant(i).ok()?;
        (at == value).then_some(word?).and_then(|word| match word {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        })
    })
}

// Do not infer a valid Boolean merely from its declared type. A native
// comparison always emits zero/one, even for arbitrary raw operand bits.
// Library results, raw inputs, carriers and ambiguous definitions stay checked.
fn native_boolean(
    code: CodePlan<'_>,
    facts: &mech_engine::native_artifact::NativeProgramFacts<'_>,
    value: Value,
    depth: usize,
) -> bool {
    if depth == 0 || code.kind(value) != Some(Kind::Bool) {
        return false;
    }
    if facts
        .definition_sites(value.get())
        .map_or(true, |sites| sites.len() != 1)
    {
        return false;
    }
    if boolean_constant(code, value).is_some() {
        return true;
    }
    let mut definitions = code
        .blocks()
        .iter()
        .filter_map(|block| match block.action() {
            Action::Scalar(
                action @ (Scalar::Primitive { output, .. } | Scalar::Copy { output, .. }),
            ) if *output == value => Some(action),
            _ => None,
        });
    let Some(definition) = definitions.next() else {
        return false;
    };
    if definitions.next().is_some() {
        return false;
    }
    match definition {
        Scalar::Copy { source, .. } => native_boolean(code, facts, *source, depth - 1),
        Scalar::Primitive { op, operands, .. } => match op {
            Op::BoolAnd | Op::BoolOr | Op::BoolXor | Op::BoolNot => operands
                .iter()
                .all(|&operand| native_boolean(code, facts, operand, depth - 1)),
            Op::U8Eq
            | Op::U8Ne
            | Op::U8Lt
            | Op::U8Le
            | Op::U8Gt
            | Op::U8Ge
            | Op::U16Eq
            | Op::U16Ne
            | Op::U16Lt
            | Op::U16Le
            | Op::U16Gt
            | Op::U16Ge
            | Op::U32Eq
            | Op::U32Ne
            | Op::U32Lt
            | Op::U32Le
            | Op::U32Gt
            | Op::U32Ge
            | Op::U64Eq
            | Op::U64Ne
            | Op::U64Lt
            | Op::U64Le
            | Op::U64Gt
            | Op::U64Ge
            | Op::I64Eq
            | Op::I64Ne
            | Op::I64Lt
            | Op::I64Le
            | Op::I64Gt
            | Op::I64Ge
            | Op::F64Eq
            | Op::F64Ne
            | Op::F64Lt
            | Op::F64Le
            | Op::F64Gt
            | Op::F64Ge
            | Op::BoolEq
            | Op::BoolNe => true,
            _ => false,
        },
        _ => false,
    }
}

fn literal(code: CodePlan<'_>, left: Value, right: Value) -> Option<(Value, bool)> {
    boolean_constant(code, right)
        .map(|constant| (left, constant))
        .or_else(|| boolean_constant(code, left).map(|constant| (right, constant)))
}

pub(in crate::native_artifact) fn target(
    code: CodePlan<'_>,
    site: usize,
    options: OptimizationOptions,
) -> Option<u32> {
    if !options.enabled(OptimizationPass::BooleanBranches) {
        return None;
    }
    let CodePlan::Program(prepared) = code else {
        return None;
    };
    let facts = prepared.plan().facts();
    let block = code.blocks().get(site)?;
    if !matches!(block.action(), Action::Scalar(Scalar::None)) {
        return None;
    }
    let (condition, wanted, yes, no) = match block.terminator() {
        Term::Scalar(Branch::LiteralBranch {
            left,
            right,
            yes,
            no,
        }) => {
            let (condition, wanted) = literal(code, left, right)?;
            (condition, wanted, yes, no)
        }
        Term::Scalar(Branch::Branch { condition, yes, no }) => (condition, true, yes, no),
        _ => return None,
    };
    if !native_boolean(code, &facts, condition, 32) {
        return None;
    }
    let mut current = site as u32;
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..32 {
        if !seen.insert(current) {
            return None;
        }
        let mut predecessors = code.blocks().iter().enumerate().filter(|(_, block)| {
            mech_engine::native_artifact::NativeProgramFacts::control_successors(block.terminator())
                .contains(&Some(current))
        });
        let (at, prior) = predecessors.next()?;
        if predecessors.next().is_some() {
            return None;
        }
        match prior.terminator() {
            Term::Scalar(Branch::LiteralBranch {
                left,
                right,
                yes: py,
                no: pn,
            }) if py != pn => {
                let (tested, constant) = literal(code, left, right)?;
                if tested != condition {
                    return None;
                }
                let known = if current == py { constant } else { !constant };
                return Some(if known == wanted { yes } else { no });
            }
            Term::Scalar(Branch::Branch {
                condition: tested,
                yes: py,
                no: pn,
            }) if py != pn && tested == condition => {
                return Some(if (current == py) == wanted { yes } else { no });
            }
            Term::Scalar(Branch::Jump(_))
                if matches!(prior.action(), Action::Scalar(Scalar::None)) =>
            {
                current = at as u32;
            }
            _ => return None,
        }
    }
    None
}
