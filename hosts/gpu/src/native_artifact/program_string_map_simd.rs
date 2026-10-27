//! Shared prepared ASCII mapping, in bounded 16-byte batches on AArch64.
//! The same entry guard supplies the actual initialized byte table and source/
//! output owners. Source and output may coincide: the full batch is loaded
//! before any store. A missing entry leaves the batch to the scalar fallback.
use super::*;
use cranelift_codegen::ir::Value;

pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    source: Value,
    length: Value,
    table: Value,
    output: Value,
    kind: MappingKind,
) -> Value {
    let init = b.create_block();
    let scan = b.create_block();
    let write = b.create_block();
    let tail = b.create_block();
    b.append_block_param(scan, types::I64);
    b.append_block_param(tail, types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    let fits = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, length, 16);
    b.ins().brif(fits, init, &[], tail, &[zero.into()]);
    b.switch_to_block(init);
    let last = b.ins().iadd_imm(length, -16);
    let tables = Tables::new(b, table, kind);
    b.ins().jump(scan, &[zero.into()]);
    b.switch_to_block(scan);
    let offset = b.block_params(scan)[0];
    let address = b.ins().iadd(source, offset);
    let keys = b.ins().load(types::I8X16, MemFlags::new(), address, 0);
    let (mapped, missing) = tables.map(b, keys);
    b.ins().brif(missing, tail, &[offset.into()], write, &[]);
    b.switch_to_block(write);
    let address = b.ins().iadd(output, offset);
    b.ins().store(MemFlags::new(), mapped, address, 0);
    let next = b.ins().iadd_imm(offset, 16);
    let more = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, next, last);
    b.ins()
        .brif(more, scan, &[next.into()], tail, &[next.into()]);
    b.switch_to_block(tail);
    b.block_params(tail)[0]
}
