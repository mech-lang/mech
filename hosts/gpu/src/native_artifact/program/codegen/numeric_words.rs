//! Shared native word/field and unsigned matrix emission for numeric regions.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::codegen::common::{aggregates, ssa_words::Words};
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::compiler::backend;
use crate::native_artifact::program::prepare::readonly_numeric::{FrameWords, Operation};
use cranelift_codegen::ir::{InstBuilder, Value, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::{
    NativeBufferFrameOperation as Frame, NativeIterationAction as Action,
    NativeIterationTerminator as Term, NativeMatchAction as Scalar,
    NativeMatchTerminator as Branch,
};
use std::collections::BTreeSet;
pub(super) fn source_slots(
    code: CodePlan<'_>,
    sites: impl IntoIterator<Item = usize>,
    actions: &[(usize, Operation)],
    scratch: impl IntoIterator<Item = usize>,
) -> Result<BTreeSet<usize>, NativeCompileError> {
    let mut slots = BTreeSet::from_iter(scratch);
    for site in sites {
        match code.blocks()[site].action() {
            Action::Scalar(Scalar::Primitive {
                operands, output, ..
            }) => {
                slots.extend(operands.iter().map(|v| v.get() as usize));
                slots.insert(output.get() as usize);
            }
            Action::Scalar(Scalar::Copy { source, output }) if code.kind(*output).is_some() => {
                slots.extend([source.get() as usize, output.get() as usize]);
            }
            Action::Structural { output, .. } | Action::LibraryCall { output, .. }
                if code.kind(*output).is_some() =>
            {
                slots.insert(output.get() as usize);
            }
            _ => {}
        }
        match actions
            .iter()
            .find(|(s, _)| *s == site)
            .map(|(_, action)| action)
        {
            Some(Operation::Read { index, .. }) => {
                slots.insert(*index as usize);
            }
            Some(Operation::Write {
                index, incoming, ..
            }) => {
                slots.extend([*index as usize, *incoming as usize]);
            }
            Some(Operation::Append { incoming, .. }) => {
                slots.insert(*incoming as usize);
            }
            _ => {}
        }
        if let CodePlan::Program(program) = code {
            if let Some(action) = program.metadata().scalar_aggregates.action(site) {
                aggregates::aggregate_words(
                    &program.metadata().scalar_aggregates,
                    action,
                    &mut slots,
                )
                .ok_or_else(|| backend("numeric aggregate lost exact word lanes"))?;
            }
            if let Some(frame) = program.metadata().buffer_frames.action(site) {
                match frame.operation() {
                    Frame::Pack { .. } => {
                        if let Action::LibraryCall { operands, .. } = code.blocks()[site].action() {
                            slots.extend(
                                operands
                                    .iter()
                                    .filter(|v| code.kind(**v).is_some())
                                    .map(|v| v.get() as usize),
                            );
                        }
                    }
                    Frame::Project { output, .. } if code.kind(*output).is_some() => {
                        slots.insert(output.get() as usize);
                    }
                    _ => {}
                }
            }
        }
        match code.blocks()[site].terminator() {
            Term::Scalar(Branch::Branch { condition, .. }) => {
                slots.insert(condition.get() as usize);
            }
            Term::Scalar(Branch::LiteralBranch { left, right, .. }) => {
                slots.extend([left.get() as usize, right.get() as usize]);
            }
            _ => {}
        }
    }
    Ok(slots)
}
/// Emit the same scalar frame words for a leaf or an acyclic parent piece.
/// Managed fields remain in the owning checked frame/array recipes.
pub(super) fn frame_words(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    shadow: usize,
    code: CodePlan<'_>,
    site: usize,
    recipes: &[FrameWords],
) -> Result<(), NativeCompileError> {
    let CodePlan::Program(program) = code else {
        return Err(backend("numeric frame lacks managed plan"));
    };
    let fields = |value| {
        recipes
            .iter()
            .find(|f| f.value == value)
            .map(|f| f.fields.as_slice())
            .ok_or_else(|| backend("numeric frame lost exact scalar fields"))
    };
    let operation = program
        .metadata()
        .buffer_frames
        .action(site)
        .ok_or_else(|| backend("numeric frame lost fields"))?
        .operation();
    match operation {
        Frame::Pack { output } => {
            let Action::LibraryCall { operands, .. } = code.blocks()[site].action() else {
                return Err(backend("numeric pack lost operands"));
            };
            for (field, operand) in fields(output.get())?.iter().zip(operands) {
                if let Some(field) = field {
                    let value = words.read(b, operand.get() as usize);
                    words.write(b, shadow + field, value);
                }
            }
        }
        Frame::Copy { source, output } => {
            // Read every old scalar before writing any next-state lane.
            let copied = fields(source.get())?
                .iter()
                .zip(fields(output.get())?)
                .filter_map(|(from, to)| from.zip(*to))
                .map(|(from, to)| (to, words.read(b, shadow + from)))
                .collect::<Vec<_>>();
            for (to, value) in copied {
                words.write(b, shadow + to, value);
            }
        }
        Frame::Project {
            source,
            ordinal,
            output,
        } => {
            if let Some(field) = fields(source.get())?[*ordinal] {
                let value = words.read(b, shadow + field);
                words.write(b, output.get() as usize, value);
            }
        }
    }
    Ok(())
}

/// Canonical linear selection is column-major over row-major physical storage.
/// Call only after the admitted unsigned view and one-based bounds check.
pub(super) fn linear_address(
    b: &mut FunctionBuilder<'_>,
    ordinal: Value,
    rows: Value,
    columns: Value,
    width: u8,
    schema: &mech_core::SchemaBody,
    base: impl FnOnce(&mut FunctionBuilder<'_>) -> Value,
) -> Result<(Value, cranelift_codegen::ir::Type), NativeCompileError> {
    // The same entry witness that lends this view has already checked the
    // current owner's exact schema and dimensions. A unit dimension makes
    // column-major source order identical to row-major backing order. Derive
    // this immutable fact from that witness, never from a previous view.
    let vector = matches!(schema, mech_core::SchemaBody::Matrix { dimensions, .. }
        if dimensions.len() == 2 && dimensions.iter().any(|dimension|
            matches!(dimension, mech_core::DimensionExpr::Constant(1))));
    let physical = if vector {
        ordinal
    } else {
        let row = b.ins().urem(ordinal, rows);
        let column = b.ins().udiv(ordinal, rows);
        let row = b.ins().imul(row, columns);
        b.ins().iadd(row, column)
    };
    let at = b.ins().imul_imm(physical, i64::from(width));
    let base = base(b);
    let address = b.ins().iadd(base, at);
    let ty = match width {
        1 => types::I8,
        2 => types::I16,
        4 => types::I32,
        8 => types::I64,
        _ => return Err(backend("numeric load width differs")),
    };
    Ok((address, ty))
}

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "numeric_address_tests.rs"]
mod tests;
