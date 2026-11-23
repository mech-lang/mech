//! Generated source operations over a genuine private fixed-key loan. Only
//! cold checked failures and finite fuel refill call the bound owner.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::compiler::{backend, guard, offset};
use crate::native_artifact::program::codegen::hooks::Hooks;
use crate::native_artifact::program::protocol::OwnerOperation;
use crate::native_artifact::program::runtime::maps::fixed_keys::Frame;
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeStringMatrixKeyCell as Cell,
};

fn load(b: &mut FunctionBuilder<'_>, base: Value, bytes: usize) -> Value {
    b.ins()
        .load(types::I64, MemFlags::new(), base, bytes as i32)
}
macro_rules! field {
    ($b:expr, $frame:expr, $name:ident) => {
        load($b, $frame, core::mem::offset_of!(Frame, $name))
    };
}

struct Operation<'a> {
    hooks: &'a Hooks,
    function: Value,
    context: Value,
    site: usize,
    index: Value,
    bytes: Value,
    snapshot_nodes: Value,
    error: Block,
}
impl Operation<'_> {
    fn require(&self, b: &mut FunctionBuilder<'_>, allowed: Value) {
        let denied = b.create_block();
        let next = b.create_block();
        b.set_cold_block(denied);
        b.ins().brif(allowed, next, &[], denied, &[]);
        b.switch_to_block(denied);
        // Preserve the actual atomic observations. Resampling here could hide
        // the first denied source operation when another worker retires data.
        self.hooks.bound_call(
            b,
            self.function,
            self.context,
            self.site,
            OwnerOperation::FixedKeyFailure,
            [self.index, self.bytes, self.snapshot_nodes],
            self.error,
        );
        let zero = b.ins().iconst(types::I8, 0);
        guard(b, zero, self.error, crate::native_artifact::abi::STATUS_ABI);
        b.ins().jump(next, &[]);
        b.switch_to_block(next);
    }
    fn capacity(&self, b: &mut FunctionBuilder<'_>, base: Value, extra: Value, limit: Value) {
        let (total, overflow) = b.ins().uadd_overflow(base, extra);
        let fits = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, total, limit);
        let valid = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
        let fits = b.ins().band(fits, valid);
        self.require(b, fits);
    }
}

pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    locals: Value,
    frame: Value,
    function: Value,
    context: Value,
    site: usize,
    producer: bool,
    action: &Action,
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    let Action::LibraryCall {
        operands, output, ..
    } = action
    else {
        return Err(backend("fixed-key operation lacks selected source call"));
    };
    if operands.len() != if producer { 2 } else { 3 } {
        return Err(backend("fixed-key source arity changed"));
    }
    let index = b.ins().load(
        types::I64,
        MemFlags::new(),
        locals,
        offset(operands[1].get() as usize)?,
    );
    let used_pointer = field!(b, frame, used_bytes);
    let nodes_pointer = field!(b, frame, snapshot_nodes);
    // Cranelift atomic_load is sequentially consistent. Both addresses come
    // from the retained genuine Core account, never a Rust struct offset.
    let bytes = b
        .ins()
        .atomic_load(types::I64, MemFlags::new(), used_pointer);
    let snapshots = b
        .ins()
        .atomic_load(types::I64, MemFlags::new(), nodes_pointer);
    let operation = Operation {
        hooks,
        function,
        context,
        site,
        index,
        bytes,
        snapshot_nodes: snapshots,
        error,
    };
    let fixed = field!(b, frame, fixed_nodes);
    let (nodes, overflow) = b.ins().uadd_overflow(snapshots, fixed);
    let valid = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
    operation.require(b, valid);
    let temporary_limit = field!(b, frame, temporary_limit);
    let node_limit = field!(b, frame, node_limit);
    let count = field!(b, frame, count);
    let zero = b.ins().iconst(types::I64, 0);
    if producer {
        // The original producer checks current coexistence before its ordinal.
        operation.capacity(b, bytes, zero, temporary_limit);
        operation.capacity(b, nodes, zero, node_limit);
        let portable = b.ins().icmp_imm(
            IntCC::UnsignedLessThanOrEqual,
            index,
            mech_core::PORTABLE_SELECTOR_INDEX_MAX as i64,
        );
        operation.require(b, portable);
        let ordinal = b.ins().iadd_imm(index, -1);
        let in_bounds = b.ins().icmp(IntCC::UnsignedLessThan, ordinal, count);
        operation.require(b, in_bounds);
        let cells = field!(b, frame, cells);
        let displacement = b.ins().imul_imm(ordinal, size_of::<Cell>() as i64);
        let cell = b.ins().iadd(cells, displacement);
        // The owning loan has already checked every immutable per-call work,
        // output and clone cap. Current coexistence is checked at this call.
        let extra_bytes = load(
            b,
            cell,
            core::mem::offset_of!(Cell, logical_temporary_bytes),
        );
        let extra_nodes = load(b, cell, core::mem::offset_of!(Cell, logical_extra_nodes));
        operation.capacity(b, bytes, extra_bytes, temporary_limit);
        operation.capacity(b, nodes, extra_nodes, node_limit);
        b.ins().store(
            MemFlags::new(),
            ordinal,
            locals,
            offset(output.get() as usize)?,
        );
    } else {
        // This private ordinal is used once by the complete adjacent consumer.
        // It never escapes as a source Value or a forged Engine capability.
        let in_bounds = b.ins().icmp(IntCC::UnsignedLessThan, index, count);
        guard(b, in_bounds, error, crate::native_artifact::abi::STATUS_ABI);
        let token = b.ins().load(
            types::I64,
            MemFlags::new(),
            locals,
            offset(operands[0].get() as usize)?,
        );
        let expected = field!(b, frame, token);
        let current = b.ins().icmp(IntCC::Equal, token, expected);
        guard(b, current, error, crate::native_artifact::abi::STATUS_ABI);
        let map_nodes = field!(b, frame, map_nodes);
        operation.capacity(b, nodes, map_nodes, node_limit);
        operation.capacity(b, bytes, zero, temporary_limit);
        let updates = field!(b, frame, updates);
        let one = b.ins().iconst(types::I64, 1);
        let (next, overflow) = b.ins().uadd_overflow(updates, one);
        let valid = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
        guard(b, valid, error, crate::native_artifact::abi::STATUS_ABI);
        let values = field!(b, frame, values);
        let displacement = b.ins().ishl_imm(index, 3);
        let address = b.ins().iadd(values, displacement);
        let value = b.ins().load(
            types::I64,
            MemFlags::new(),
            locals,
            offset(operands[2].get() as usize)?,
        );
        b.ins().store(MemFlags::new(), value, address, 0);
        b.ins().store(
            MemFlags::new(),
            next,
            frame,
            core::mem::offset_of!(Frame, updates) as i32,
        );
        b.ins().store(
            MemFlags::new(),
            token,
            locals,
            offset(output.get() as usize)?,
        );
    }
    Ok(())
}
