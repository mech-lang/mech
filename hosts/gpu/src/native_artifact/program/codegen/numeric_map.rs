//! A checked scalar DAG inside a native immutable-source collection loop.
use super::hooks::Hooks;
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::compiler::{backend, offset, primitive};
use crate::native_artifact::program::prepare::numeric_maps::Region;
use crate::native_artifact::program::protocol::GlobalHelper;
use crate::native_artifact::program::runtime::NativeNumericMapFrame as Frame;
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeMatchAction as Scalar, NativeProgramArtifactPlan as Plan,
};
use std::{collections::BTreeMap, mem::offset_of};

pub(in crate::native_artifact) fn emit_entry(
    b: &mut FunctionBuilder<'_>,
    locals: Value,
    site: usize,
    hooks: &Hooks,
    region: &Region,
    plan: &Plan,
    counter_slot: usize,
    exit: Block,
    error: Block,
) -> Result<(), NativeCompileError> {
    let zero = b.ins().iconst(types::I64, 0);
    hooks.call(b, site, GlobalHelper::NumericMapPrepare, [zero; 3], error);
    let frame = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
    let selected = b.create_block();
    let ordinary = b.create_block();
    b.ins().brif(frame, selected, &[], ordinary, &[]);
    b.switch_to_block(selected);
    // External scalar captures are immutable and already current at this
    // header. Snapshot each native word once, never inside the element loop.
    let mut captures = BTreeMap::new();
    for &site in &region.steps {
        let Action::Scalar(action) = plan.blocks()[site as usize].action() else {
            return Err(backend("numeric map step is not scalar"));
        };
        let operands = match action {
            Scalar::Copy { source, .. } => std::slice::from_ref(source),
            Scalar::Primitive { operands, .. } => &operands[..],
            _ => return Err(backend("numeric map step kind")),
        };
        for &value in operands {
            if region.captures.contains(&value.get()) && !captures.contains_key(&value.get()) {
                captures.insert(
                    value.get(),
                    crate::native_artifact::control_compiler::load(b, locals, value)?,
                );
            }
        }
    }
    if captures.len() != region.captures.len() {
        return Err(backend("numeric map capture inventory differs"));
    }
    let complete = b.create_block();
    let miss = b.create_block();
    let malformed = b.create_block();
    let settle = b.create_block();
    b.append_block_param(miss, types::I64);
    b.append_block_param(settle, types::I64);
    emit(b, frame, region, plan, &captures, complete, miss, malformed)?;
    b.switch_to_block(complete);
    b.ins().jump(settle, &[zero.into()]);
    b.switch_to_block(miss);
    let declined = b.ins().iconst(types::I64, 1);
    b.ins().jump(settle, &[declined.into()]);
    b.switch_to_block(malformed);
    let invalid = b.ins().iconst(types::I64, 2);
    b.ins().jump(settle, &[invalid.into()]);
    b.switch_to_block(settle);
    let status = b.block_params(settle)[0];
    hooks.call(
        b,
        site,
        GlobalHelper::NumericMapFinish,
        [frame, status, zero],
        error,
    );
    let ready = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
    let done = b.create_block();
    b.ins().brif(ready, done, &[], ordinary, &[]);
    b.switch_to_block(done);
    b.ins()
        .store(MemFlags::new(), zero, locals, offset(counter_slot)?);
    b.ins().jump(exit, &[]);
    b.switch_to_block(ordinary);
    Ok(())
}
fn get(b: &mut FunctionBuilder<'_>, frame: Value, at: usize) -> Value {
    b.ins().load(types::I64, MemFlags::new(), frame, at as i32)
}
fn require(b: &mut FunctionBuilder<'_>, good: Value, bad: Block) {
    let next = b.create_block();
    b.ins().brif(good, next, &[], bad, &[]);
    b.switch_to_block(next);
}
fn integer(width: u8) -> cranelift_codegen::ir::Type {
    match width {
        1 => types::I8,
        2 => types::I16,
        4 => types::I32,
        8 => types::I64,
        _ => unreachable!(),
    }
}
/// All dynamic loop work is prepaid by the loan. Arithmetic uses the common
/// checked native emitter; no source visit, helper or per-operation dispatch
/// occurs between the exact prepare and finish boundaries.
fn emit(
    b: &mut FunctionBuilder<'_>,
    frame: Value,
    region: &Region,
    plan: &Plan,
    captures: &BTreeMap<u32, Value>,
    complete: Block,
    miss: Block,
    malformed: Block,
) -> Result<(), NativeCompileError> {
    let count = get(b, frame, offset_of!(Frame, count));
    let input = get(b, frame, offset_of!(Frame, input));
    let output = get(b, frame, offset_of!(Frame, output));
    for (field, expected) in [
        (offset_of!(Frame, input_width), region.input_width),
        (offset_of!(Frame, output_width), region.output_width),
    ] {
        let width = get(b, frame, field);
        let good = b.ins().icmp_imm(IntCC::Equal, width, i64::from(expected));
        require(b, good, malformed);
    }
    let epoch = get(b, frame, offset_of!(Frame, epoch));
    let issued = get(b, frame, offset_of!(Frame, issued));
    let next = get(b, frame, offset_of!(Frame, next));
    let good = b.ins().icmp(IntCC::Equal, epoch, issued);
    require(b, good, malformed);
    let good = b.ins().icmp_imm(IntCC::NotEqual, epoch, 0);
    require(b, good, malformed);
    let good = b.ins().icmp_imm(IntCC::Equal, next, 0);
    require(b, good, malformed);
    for (width, pointer, field) in [
        (region.input_width, input, offset_of!(Frame, input_bytes)),
        (
            region.output_width,
            output,
            offset_of!(Frame, capacity_bytes),
        ),
    ] {
        let good = b.ins().icmp_imm(
            IntCC::UnsignedLessThanOrEqual,
            count,
            (isize::MAX as u64 / u64::from(width)) as i64,
        );
        require(b, good, malformed);
        let bytes = b.ins().imul_imm(count, i64::from(width));
        let bound = get(b, frame, field);
        let good = b.ins().icmp(IntCC::Equal, bytes, bound);
        require(b, good, malformed);
        let empty = b.ins().icmp_imm(IntCC::Equal, count, 0);
        let nonnull = b.ins().icmp_imm(IntCC::NotEqual, pointer, 0);
        let good = b.ins().bor(empty, nonnull);
        require(b, good, malformed);
        let (_, overflow) = b.ins().uadd_overflow(pointer, bytes);
        let good = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
        require(b, good, malformed);
    }
    let cursor = b.declare_var(types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    b.def_var(cursor, zero);
    let head = b.create_block();
    let body = b.create_block();
    let done = b.create_block();
    b.ins().jump(head, &[]);
    b.switch_to_block(head);
    let at = b.use_var(cursor);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, at, count);
    b.ins().brif(more, body, &[], done, &[]);
    b.switch_to_block(body);
    let at = b.use_var(cursor);
    let displacement = b.ins().imul_imm(at, i64::from(region.input_width));
    let address = b.ins().iadd(input, displacement);
    let raw = b
        .ins()
        .load(integer(region.input_width), MemFlags::new(), address, 0);
    let word = if region.input_width == 8 {
        raw
    } else {
        b.ins().uextend(types::I64, raw)
    };
    let mut words = captures.clone();
    words.insert(region.element, word);
    for &site in &region.steps {
        let Action::Scalar(action) = plan.blocks()[site as usize].action() else {
            return Err(backend("numeric map lost scalar step"));
        };
        let (output, value) = match action {
            Scalar::Copy { source, output } => (
                *output,
                *words
                    .get(&source.get())
                    .ok_or_else(|| backend("numeric map missing copy word"))?,
            ),
            Scalar::Primitive {
                op,
                operands,
                output,
            } => {
                let values = operands
                    .iter()
                    .map(|v| {
                        words
                            .get(&v.get())
                            .copied()
                            .ok_or_else(|| backend("numeric map missing operand"))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                (*output, primitive(b, *op, &values, miss))
            }
            _ => return Err(backend("numeric map step changed")),
        };
        words.insert(output.get(), value);
    }
    let result = *words
        .get(&region.result)
        .ok_or_else(|| backend("numeric map missing result"))?;
    let value = if region.output_width == 8 {
        result
    } else {
        b.ins().ireduce(integer(region.output_width), result)
    };
    let displacement = b.ins().imul_imm(at, i64::from(region.output_width));
    let address = b.ins().iadd(output, displacement);
    b.ins().store(MemFlags::new(), value, address, 0);
    let next = b.ins().iadd_imm(at, 1);
    b.def_var(cursor, next);
    b.ins().jump(head, &[]);
    b.switch_to_block(done);
    b.ins().store(
        MemFlags::new(),
        count,
        frame,
        offset_of!(Frame, next) as i32,
    );
    b.ins().jump(complete, &[]);
    Ok(())
}
