//! Reuse exact shape identities after the current typed view is admitted.
//! The original selected witness still validates schema, owner and dimensions.
//! This changes private SSA words only; all four exported view words retain
//! their original values, including empty row/column vectors and cold reloads.
use crate::native_artifact::codegen::{
    common::ssa_words::Words,
    control::plan::CodePlan,
    options::{OptimizationOptions, OptimizationPass},
};
use crate::native_artifact::program::prepare::readonly_numeric::Proof;
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::ir::{InstBuilder, Value, types};
use cranelift_frontend::FunctionBuilder;
use mech_core::{DimensionExpr, SchemaBody};
use mech_engine::native_artifact::NativeIterationAction as Action;

fn axes(b: &mut FunctionBuilder<'_>, length: Value, schema: &SchemaBody) -> Option<(Value, Value)> {
    let SchemaBody::Matrix { dimensions, .. } = schema else {
        return None;
    };
    if dimensions.len() != 2 {
        return None;
    }
    let row = matches!(dimensions[0], DimensionExpr::Constant(1));
    let column = matches!(dimensions[1], DimensionExpr::Constant(1));
    if !row && !column {
        return None;
    }
    let one = b.ins().iconst(types::I64, 1);
    Some(if row { (one, length) } else { (length, one) })
}

pub(super) fn canonicalize(
    options: OptimizationOptions,
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    code: CodePlan<'_>,
    proof: &Proof,
    shadow: usize,
    array: usize,
) -> Result<bool, NativeCompileError> {
    if !options.enabled(OptimizationPass::InvariantVectorGeometry) {
        return Ok(false);
    }
    let Action::LibraryCall { template, .. } = code.blocks()[proof.arrays[array].witness].action()
    else {
        return Err(backend("numeric geometry lost its selected view witness"));
    };
    let schema = template
        .inputs()
        .first()
        .ok_or_else(|| backend("numeric geometry witness lacks input"))?
        .body();
    let view = shadow + proof.view(array);
    let length = words.read(b, view + 1);
    let Some((rows, columns)) = axes(b, length, schema) else {
        return Ok(false);
    };
    words.write(b, view + 2, rows);
    words.write(b, view + 3, columns);
    Ok(true)
}
#[cfg(test)]
mod tests;
