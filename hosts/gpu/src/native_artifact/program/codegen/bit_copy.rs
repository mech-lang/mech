//! Exact-bit transfers between already admitted numeric banks. The caller owns
//! span validation, initialized storage, lifetime and complete source funding.
//! Partial overlap keeps the original left-to-right scalar-unit semantics.
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::ir::{InstBuilder, MemFlags, Type, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;

fn transfer(b: &mut FunctionBuilder<'_>, source: Value, output: Value, at: Value, ty: Type) {
    let from = b.ins().iadd(source, at);
    let to = b.ins().iadd(output, at);
    let value = b.ins().load(ty, MemFlags::new(), from, 0);
    b.ins().store(MemFlags::new(), value, to, 0);
}
fn scalar_loop(b: &mut FunctionBuilder<'_>, source: Value, output: Value, bytes: u64, ty: Type) {
    let body = b.create_block();
    let done = b.create_block();
    b.append_block_param(body, types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().jump(body, &[zero.into()]);
    b.switch_to_block(body);
    let at = b.block_params(body)[0];
    transfer(b, source, output, at, ty);
    let next = b.ins().iadd_imm(at, i64::from(ty.bytes()));
    let more = b
        .ins()
        .icmp_imm(IntCC::UnsignedLessThan, next, bytes as i64);
    b.ins().brif(more, body, &[next.into()], done, &[]);
    b.switch_to_block(done);
}
fn vector_copy(b: &mut FunctionBuilder<'_>, source: Value, output: Value, bytes: u64, ty: Type) {
    let bulk = bytes / 64 * 64;
    if bulk != 0 {
        let body = b.create_block();
        let done = b.create_block();
        b.append_block_param(body, types::I64);
        let zero = b.ins().iconst(types::I64, 0);
        b.ins().jump(body, &[zero.into()]);
        b.switch_to_block(body);
        let at = b.block_params(body)[0];
        for offset in [0, 16, 32, 48] {
            let index = b.ins().iadd_imm(at, offset);
            transfer(b, source, output, index, types::I8X16);
        }
        let next = b.ins().iadd_imm(at, 64);
        let more = b.ins().icmp_imm(IntCC::UnsignedLessThan, next, bulk as i64);
        b.ins().brif(more, body, &[next.into()], done, &[]);
        b.switch_to_block(done);
    }
    let mut at = bulk;
    while bytes - at >= 16 {
        let index = b.ins().iconst(types::I64, at as i64);
        transfer(b, source, output, index, types::I8X16);
        at += 16;
    }
    while at < bytes {
        let index = b.ins().iconst(types::I64, at as i64);
        transfer(b, source, output, index, ty);
        at += u64::from(ty.bytes());
    }
}
pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    source: Value,
    output: Value,
    bytes: u64,
    width: u8,
) -> Result<(), NativeCompileError> {
    let ty = match width {
        1 => types::I8,
        2 => types::I16,
        4 => types::I32,
        8 => types::I64,
        _ => return Err(backend("native bit copy lost its scalar width")),
    };
    if bytes > isize::MAX as u64 || bytes % u64::from(width) != 0 {
        return Err(backend("native bit copy lost its finite unit geometry"));
    }
    if bytes == 0 {
        return Ok(());
    }
    if bytes < 16 {
        vector_copy(b, source, output, bytes, ty);
        return Ok(());
    }
    // Wrapping pointer distances conservatively retain scalar order whenever
    // the two valid, non-wrapping admitted spans overlap, including equality.
    let forward = b.ins().isub(output, source);
    let reverse = b.ins().isub(source, output);
    let forward = b
        .ins()
        .icmp_imm(IntCC::UnsignedLessThan, forward, bytes as i64);
    let reverse = b
        .ins()
        .icmp_imm(IntCC::UnsignedLessThan, reverse, bytes as i64);
    let overlap = b.ins().bor(forward, reverse);
    let scalar = b.create_block();
    let vector = b.create_block();
    let done = b.create_block();
    b.ins().brif(overlap, scalar, &[], vector, &[]);
    b.switch_to_block(scalar);
    scalar_loop(b, source, output, bytes, ty);
    b.ins().jump(done, &[]);
    b.switch_to_block(vector);
    vector_copy(b, source, output, bytes, ty);
    b.ins().jump(done, &[]);
    b.switch_to_block(done);
    Ok(())
}
#[cfg(test)]
#[path = "bit_copy_tests.rs"]
mod tests;
