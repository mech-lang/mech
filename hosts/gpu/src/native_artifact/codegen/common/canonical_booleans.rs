//! Producer-derived zero/one facts for direct literal Boolean branches.
//! Types alone, current sample bits and previous invocations grant no fact.
use super::super::control::plan::CodePlan;
use super::boolean_branches::{boolean_constant, literal};
use crate::native_artifact::codegen::options::{OptimizationOptions, OptimizationPass};
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeMatchAction as Scalar, NativeMatchValue as Value,
    NativeProgramFacts, NativeProgramProducerRole as Role, NativeScalarKind as Kind,
    NativeScalarOp as Op,
};

pub(super) fn primitive(op: Op, operands: impl FnOnce() -> bool) -> bool {
    match op {
        Op::BoolAnd | Op::BoolOr | Op::BoolXor | Op::BoolNot => operands(),
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
    }
}

fn canonical(
    code: CodePlan<'_>,
    facts: &NativeProgramFacts<'_>,
    value: Value,
    depth: usize,
    fuel: &mut usize,
) -> bool {
    if depth == 0 || *fuel == 0 || code.kind(value) != Some(Kind::Bool) {
        return false;
    }
    *fuel -= 1;
    if boolean_constant(code, value).is_some() {
        return true;
    }
    let Ok(definitions) = facts.definition_sites(value.get()) else {
        return false;
    };
    // All producers must be known scalar definitions. Raw inputs, library
    // results, structural projections and carrier entry/latch state decline.
    // A cycle exhausts the bounded proof and keeps the original literal test.
    !definitions.is_empty()
        && definitions.iter().all(|definition| {
            if definition.role() != Role::Scalar {
                return false;
            }
            let Some(site) = definition.site() else {
                return false;
            };
            let Some(block) = code.blocks().get(site as usize) else {
                return false;
            };
            match block.action() {
                Action::Scalar(Scalar::Copy { source, output }) if *output == value => {
                    canonical(code, facts, *source, depth - 1, fuel)
                }
                Action::Scalar(Scalar::Primitive {
                    op,
                    operands,
                    output,
                    ..
                }) if *output == value => primitive(*op, || {
                    operands
                        .iter()
                        .all(|&operand| canonical(code, facts, operand, depth - 1, fuel))
                }),
                _ => false,
            }
        })
}

pub(in crate::native_artifact) fn direct_literal(
    code: CodePlan<'_>,
    options: OptimizationOptions,
    left: Value,
    right: Value,
) -> Option<(Value, bool)> {
    if !options.enabled(OptimizationPass::CanonicalBooleanBranches) {
        return None;
    }
    let CodePlan::Program(prepared) = code else {
        return None;
    };
    let (condition, expected) = literal(code, left, right)?;
    canonical(code, &prepared.plan().facts(), condition, 32, &mut 256)
        .then_some((condition, expected))
}
