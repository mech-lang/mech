//! Complete uniform unsigned construction over an already admitted private bank.
use super::hooks::Hooks;
use crate::native_artifact::compiler::{guard, offset};
use crate::native_artifact::program::protocol::GlobalHelper;
use crate::native_artifact::program::runtime::NativeUniformFillFrame as Frame;
use crate::native_artifact::{NativeCompileError, abi::STATUS_ABI};
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::{NativeScalarKind, NativeUniformWordCollection};
use std::mem::offset_of;

pub(in crate::native_artifact) fn emit_entry(
    b: &mut FunctionBuilder<'_>,
    locals: Value,
    site: usize,
    hooks: &Hooks,
    region: &NativeUniformWordCollection,
    counter_slot: usize,
    exit: Block,
    error: Block,
) -> Result<(), NativeCompileError> {
    let end = crate::native_artifact::control_compiler::load(b, locals, region.limit())?;
    let start = crate::native_artifact::control_compiler::load(b, locals, region.start())?;
    // The selected range preparation already checked ordered, representable
    // endpoints and its exact cardinality. No range error moves past it.
    let difference = b.ins().isub(end, start);
    let initial = if region.inclusive() {
        b.ins().iadd_imm(difference, 1)
    } else {
        difference
    };
    let count = if region.filtered() { end } else { initial };
    let remaining = b
        .ins()
        .load(types::I64, MemFlags::new(), locals, offset(counter_slot)?);
    let fresh = b.ins().icmp(IntCC::Equal, remaining, initial);
    let attempt = b.create_block();
    let ordinary = b.create_block();
    b.ins().brif(fresh, attempt, &[], ordinary, &[]);
    b.switch_to_block(attempt);
    let word = crate::native_artifact::control_compiler::load(b, locals, region.word())?;
    let zero = b.ins().iconst(types::I64, 0);
    hooks.call(
        b,
        site,
        GlobalHelper::UniformFillPrepare,
        [word, count, zero],
        error,
    );
    let frame = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
    let selected = b.create_block();
    b.ins().brif(frame, selected, &[], ordinary, &[]);
    b.switch_to_block(selected);
    let complete = b.create_block();
    let malformed = b.create_block();
    let settle = b.create_block();
    b.append_block_param(settle, types::I64);
    let width = match region.kind() {
        NativeScalarKind::U8 => 1,
        NativeScalarKind::U16 => 2,
        NativeScalarKind::U32 => 4,
        NativeScalarKind::U64 => 8,
        _ => {
            return Err(crate::native_artifact::compiler::backend(
                "native fill unsigned kind",
            ));
        }
    };
    emit(b, frame, width, complete, malformed, true);
    b.switch_to_block(complete);
    b.ins().jump(settle, &[zero.into()]);
    b.switch_to_block(malformed);
    let bad = b.ins().iconst(types::I64, 1);
    b.ins().jump(settle, &[bad.into()]);
    b.switch_to_block(settle);
    let status = b.block_params(settle)[0];
    hooks.call(
        b,
        site,
        GlobalHelper::UniformFillFinish,
        [frame, status, zero],
        error,
    );
    let ready = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
    let good = b.ins().icmp_imm(IntCC::Equal, ready, 1);
    guard(b, good, error, STATUS_ABI);
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
/// No Call, IndirectCall or per-element callback. The full store allowance was
/// charged before entry. Every dynamic extent and emitted access is bounded by
/// the exact capacity, with a 16-byte SIMD loop and a scalar remainder.
pub(in crate::native_artifact) fn emit(
    b: &mut FunctionBuilder<'_>,
    frame: Value,
    width: u64,
    complete: Block,
    malformed: Block,
    vectorized: bool,
) {
    assert!(matches!(width, 1 | 2 | 4 | 8));
    let count = get(b, frame, offset_of!(Frame, count));
    let capacity = get(b, frame, offset_of!(Frame, capacity_bytes));
    let output = get(b, frame, offset_of!(Frame, output));
    let word = get(b, frame, offset_of!(Frame, word));
    let stored_width = get(b, frame, offset_of!(Frame, width));
    let epoch = get(b, frame, offset_of!(Frame, epoch));
    let issued = get(b, frame, offset_of!(Frame, issued));
    let next = get(b, frame, offset_of!(Frame, next));
    let good = b.ins().icmp_imm(IntCC::Equal, stored_width, width as i64);
    require(b, good, malformed);
    let good = b.ins().icmp(IntCC::Equal, epoch, issued);
    require(b, good, malformed);
    let good = b.ins().icmp_imm(IntCC::NotEqual, epoch, 0);
    require(b, good, malformed);
    let good = b.ins().icmp_imm(IntCC::Equal, next, 0);
    require(b, good, malformed);
    let good = b.ins().icmp_imm(
        IntCC::UnsignedLessThanOrEqual,
        count,
        (isize::MAX as u64 / width) as i64,
    );
    require(b, good, malformed);
    let bytes = b.ins().imul_imm(count, width as i64);
    let good = b.ins().icmp(IntCC::Equal, bytes, capacity);
    require(b, good, malformed);
    let empty = b.ins().icmp_imm(IntCC::Equal, count, 0);
    let pointer = b.ins().icmp_imm(IntCC::NotEqual, output, 0);
    let good = b.ins().bor(empty, pointer);
    require(b, good, malformed);
    let (_, overflow) = b.ins().uadd_overflow(output, bytes);
    let good = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
    require(b, good, malformed);
    if width < 8 {
        let maximum = (1u64 << (width * 8)) - 1;
        let good = b
            .ins()
            .icmp_imm(IntCC::UnsignedLessThanOrEqual, word, maximum as i64);
        require(b, good, malformed);
    }
    let scalar_type = match width {
        1 => types::I8,
        2 => types::I16,
        4 => types::I32,
        8 => types::I64,
        _ => unreachable!(),
    };
    let scalar = if width == 8 {
        word
    } else {
        b.ins().ireduce(scalar_type, word)
    };
    let cursor = b.declare_var(types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    b.def_var(cursor, zero);
    let tail_head = b.create_block();
    let tail_body = b.create_block();
    let done = b.create_block();
    if vectorized {
        let vector_type = match width {
            1 => types::I8X16,
            2 => types::I16X8,
            4 => types::I32X4,
            8 => types::I64X2,
            _ => unreachable!(),
        };
        let vector = b.ins().splat(vector_type, scalar);
        let head = b.create_block();
        let body = b.create_block();
        b.ins().jump(head, &[]);
        b.switch_to_block(head);
        let at = b.use_var(cursor);
        let remain = b.ins().isub(count, at);
        let full = b
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remain, 16 / width as i64);
        b.ins().brif(full, body, &[], tail_head, &[]);
        b.switch_to_block(body);
        let at = b.use_var(cursor);
        let offset = b.ins().imul_imm(at, width as i64);
        let address = b.ins().iadd(output, offset);
        b.ins().store(MemFlags::new(), vector, address, 0);
        let next = b.ins().iadd_imm(at, 16 / width as i64);
        b.def_var(cursor, next);
        b.ins().jump(head, &[]);
    } else {
        b.ins().jump(tail_head, &[]);
    }
    b.switch_to_block(tail_head);
    let at = b.use_var(cursor);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, at, count);
    b.ins().brif(more, tail_body, &[], done, &[]);
    b.switch_to_block(tail_body);
    let at = b.use_var(cursor);
    let offset = b.ins().imul_imm(at, width as i64);
    let address = b.ins().iadd(output, offset);
    b.ins().store(MemFlags::new(), scalar, address, 0);
    let next = b.ins().iadd_imm(at, 1);
    b.def_var(cursor, next);
    b.ins().jump(tail_head, &[]);
    b.switch_to_block(done);
    b.ins().store(
        MemFlags::new(),
        count,
        frame,
        offset_of!(Frame, next) as i32,
    );
    b.ins().jump(complete, &[]);
}

#[cfg(all(test, feature = "canonical-native-program", feature = "jit"))]
#[path = "constant_fill_tests.rs"]
mod tests;
