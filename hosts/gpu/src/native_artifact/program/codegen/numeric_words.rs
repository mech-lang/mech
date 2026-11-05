//! Shared native word/field and unsigned matrix emission for numeric regions.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::codegen::common::{aggregates, ssa_words::Words};
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::compiler::backend;
use crate::native_artifact::program::prepare::readonly_numeric::{FrameWords, Operation};
use cranelift_codegen::ir::condcodes::IntCC;
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
            Some(Operation::AppendLeBytes {
                incoming, count, ..
            }) => {
                slots.extend([*incoming as usize, *count as usize]);
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
/// Original read/write ordering, shared with the current-traversal analysis.
pub(super) struct FrameWordMoves {
    pub assignments: Vec<(usize, usize)>,
    pub parallel: bool,
}

/// Exact scalar lane assignments; managed fields retain their checked recipes.
pub(super) fn frame_word_moves(
    shadow: usize,
    code: CodePlan<'_>,
    site: usize,
    recipes: &[FrameWords],
) -> Result<FrameWordMoves, NativeCompileError> {
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
    let assignments = match operation {
        Frame::Pack { output } => {
            let Action::LibraryCall { operands, .. } = code.blocks()[site].action() else {
                return Err(backend("numeric pack lost operands"));
            };
            fields(output.get())?
                .iter()
                .zip(operands)
                .filter_map(|(field, operand)| {
                    field.map(|field| (shadow + field, operand.get() as usize))
                })
                .collect()
        }
        Frame::Copy { source, output } => fields(source.get())?
            .iter()
            .zip(fields(output.get())?)
            .filter_map(|(from, to)| from.zip(*to))
            .map(|(from, to)| (shadow + to, shadow + from))
            .collect(),
        Frame::Project {
            source,
            ordinal,
            output,
        } => fields(source.get())?[*ordinal]
            .map(|field| (output.get() as usize, shadow + field))
            .into_iter()
            .collect(),
    };
    Ok(FrameWordMoves {
        assignments,
        parallel: matches!(operation, Frame::Copy { .. }),
    })
}

/// Preserve the original Pack/Project and overlapping Copy emission order.
pub(super) fn frame_words(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    shadow: usize,
    code: CodePlan<'_>,
    site: usize,
    recipes: &[FrameWords],
) -> Result<(), NativeCompileError> {
    let moves = frame_word_moves(shadow, code, site, recipes)?;
    if moves.parallel {
        // Copy must read every old scalar before writing any next-state lane.
        let copied = moves
            .assignments
            .into_iter()
            .map(|(to, from)| (to, words.read(b, from)))
            .collect::<Vec<_>>();
        for (to, value) in copied {
            words.write(b, to, value);
        }
    } else {
        for (to, from) in moves.assignments {
            let value = words.read(b, from);
            words.write(b, to, value);
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
        // Generic schemas can still carry a current row or column vector.
        // The caller has checked this admitted view's bounds, so either unit
        // dimension makes the physical ordinal identical to source order.
        let one_row = b.ins().icmp_imm(IntCC::Equal, rows, 1);
        let one_column = b.ins().icmp_imm(IntCC::Equal, columns, 1);
        let contiguous = b.ins().bor(one_row, one_column);
        let general = b.create_block();
        let done = b.create_block();
        b.append_block_param(done, types::I64);
        b.ins()
            .brif(contiguous, done, &[ordinal.into()], general, &[]);
        b.switch_to_block(general);
        let column = b.ins().udiv(ordinal, rows);
        // Bounds on the admitted view prove rows > 0. Reuse the quotient:
        // column * rows <= ordinal, so this internal remainder cannot overflow.
        // Separate urem/udiv lower to duplicate machine divisions on AArch64.
        let consumed = b.ins().imul(column, rows);
        let row = b.ins().isub(ordinal, consumed);
        let row = b.ins().imul(row, columns);
        let physical = b.ins().iadd(row, column);
        b.ins().jump(done, &[physical.into()]);
        b.switch_to_block(done);
        b.block_params(done)[0]
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
