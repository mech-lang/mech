//! Native immutable-read index attached to canonical Map entries. One concrete
//! owner holds both entries and an open-addressed table of entry ordinals.
//! Map words: entries, count, entry capacity, slot pointer, slot mask, owner.
use super::{arrays::Arrays, layout::Type, maps, strings};
use crate::native_artifact::program::codegen::utf8_histogram::emit_hash;
use crate::native_artifact::{
    NativeCompileError,
    compiler::{backend, guard},
};
use cranelift_codegen::ir::{
    Block, Endianness, InstBuilder, MemFlags, Value, condcodes::IntCC, types,
};
use cranelift_frontend::FunctionBuilder;

fn address(b: &mut FunctionBuilder<'_>, base: Value, index: Value, stride: usize) -> Value {
    let offset = b.ins().imul_imm(index, stride as i64);
    b.ins().iadd(base, offset)
}
fn multiply(b: &mut FunctionBuilder<'_>, value: Value, factor: usize, error: Block) -> Value {
    let bound = b.ins().icmp_imm(
        IntCC::UnsignedLessThanOrEqual,
        value,
        (isize::MAX as usize / factor) as i64,
    );
    guard(b, bound, error, 1);
    b.ins().imul_imm(value, factor as i64)
}
pub(super) fn allocate(
    arrays: &Arrays,
    b: &mut FunctionBuilder<'_>,
    capacity: Value,
    entry: &Type,
    error: Block,
) -> Result<Vec<Value>, NativeCompileError> {
    let stride = entry
        .packed_bytes
        .ok_or_else(|| backend("native Map entry stride absent"))?;
    if stride % 8 != 0 {
        return Err(backend("native Map entry stride is not word aligned"));
    }
    let needed = multiply(b, capacity, 2, error);
    let head = b.create_block();
    b.append_block_param(head, types::I64);
    let grow = b.create_block();
    let done = b.create_block();
    b.append_block_param(done, types::I64);
    let four = b.ins().iconst(types::I64, 4);
    b.ins().jump(head, &[four.into()]);
    b.switch_to_block(head);
    let slots = b.block_params(head)[0];
    let enough = b
        .ins()
        .icmp(IntCC::UnsignedGreaterThanOrEqual, slots, needed);
    b.ins().brif(enough, done, &[slots.into()], grow, &[]);
    b.switch_to_block(grow);
    let slots = maps::add(b, slots, slots, error);
    b.ins().jump(head, &[slots.into()]);
    b.switch_to_block(done);
    let slots = b.block_params(done)[0];
    let mask = b.ins().iadd_imm(slots, -1);
    let entry_words = multiply(b, capacity, stride / 8, error);
    let total_words = maps::add(b, entry_words, slots, error);
    let word = Type::derive(&mech_core::SchemaBody::UnsignedInteger(
        mech_core::IntegerWidth::W64,
    ))?;
    let mut view = arrays.allocate(b, total_words, &word, true, error)?;
    let bytes = b.ins().imul_imm(entry_words, 8);
    let index = b.ins().iadd(view[0], bytes);
    view[2] = capacity;
    view[3] = index;
    view[4] = mask;
    Ok(view)
}
pub(super) fn writable(
    arrays: &Arrays,
    b: &mut FunctionBuilder<'_>,
    source: &[Value],
    consume: bool,
    capacity: Value,
    entry: &Type,
    error: Block,
) -> Result<Vec<Value>, NativeCompileError> {
    let done = b.create_block();
    for _ in 0..6 {
        b.append_block_param(done, types::I64);
    }
    let copy = b.create_block();
    if consume {
        let inspect = b.create_block();
        b.ins().brif(source[5], inspect, &[], copy, &[]);
        b.switch_to_block(inspect);
        let references = b.ins().load(types::I64, MemFlags::new(), source[5], 0);
        let unique = b.ins().icmp_imm(IntCC::Equal, references, 1);
        let fits = b
            .ins()
            .icmp(IntCC::UnsignedLessThanOrEqual, capacity, source[2]);
        let indexed = b.ins().icmp_imm(IntCC::NotEqual, source[3], 0);
        let reuse = b.ins().band(unique, fits);
        let reuse = b.ins().band(reuse, indexed);
        b.ins().brif(
            reuse,
            done,
            &source.iter().copied().map(Into::into).collect::<Vec<_>>(),
            copy,
            &[],
        );
    } else {
        b.ins().jump(copy, &[]);
    }
    b.switch_to_block(copy);
    let mut view = allocate(arrays, b, capacity, entry, error)?;
    let bytes = multiply(b, source[1], entry.packed_bytes.unwrap(), error);
    arrays.copy(b, view[0], source[0], bytes);
    view[1] = source[1];
    b.ins().jump(
        done,
        &view.iter().copied().map(Into::into).collect::<Vec<_>>(),
    );
    b.switch_to_block(done);
    Ok(b.block_params(done).to_vec())
}
/// Rebuild after canonical insertion/sorting; only initialized keys are read.
pub(super) fn rebuild(b: &mut FunctionBuilder<'_>, view: &[Value], entry: &Type, error: Block) {
    let zero = b.ins().iconst(types::I64, 0);
    let clear = b.create_block();
    b.append_block_param(clear, types::I64);
    let clear_one = b.create_block();
    let cleared = b.create_block();
    b.ins().jump(clear, &[zero.into()]);
    b.switch_to_block(clear);
    let at = b.block_params(clear)[0];
    let more = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, at, view[4]);
    b.ins().brif(more, clear_one, &[], cleared, &[]);
    b.switch_to_block(clear_one);
    let slot = address(b, view[3], at, 8);
    b.ins().store(MemFlags::new(), zero, slot, 0);
    let next = b.ins().iadd_imm(at, 1);
    b.ins().jump(clear, &[next.into()]);
    b.switch_to_block(cleared);
    let head = b.create_block();
    b.append_block_param(head, types::I64);
    let body = b.create_block();
    let done = b.create_block();
    b.ins().jump(head, &[zero.into()]);
    b.switch_to_block(head);
    let ordinal = b.block_params(head)[0];
    let more = b.ins().icmp(IntCC::UnsignedLessThan, ordinal, view[1]);
    b.ins().brif(more, body, &[], done, &[]);
    b.switch_to_block(body);
    let key = address(b, view[0], ordinal, entry.packed_bytes.unwrap());
    let pointer = b.ins().load(types::I64, MemFlags::new(), key, 0);
    let length = b.ins().load(types::I64, MemFlags::new(), key, 8);
    let hash = emit_hash(b, pointer, length);
    let first = b.ins().band(hash, view[4]);
    let probe = b.create_block();
    b.append_block_param(probe, types::I64);
    b.append_block_param(probe, types::I64);
    let advance = b.create_block();
    let insert = b.create_block();
    b.ins().jump(probe, &[first.into(), zero.into()]);
    b.switch_to_block(probe);
    let at = b.block_params(probe)[0];
    let tried = b.block_params(probe)[1];
    let bounded = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, tried, view[4]);
    guard(b, bounded, error, 0x19503);
    let slot = address(b, view[3], at, 8);
    let previous = b.ins().load(types::I64, MemFlags::new(), slot, 0);
    b.ins().brif(previous, advance, &[], insert, &[]);
    b.switch_to_block(advance);
    let at = b.ins().iadd_imm(at, 1);
    let at = b.ins().band(at, view[4]);
    let tried = b.ins().iadd_imm(tried, 1);
    b.ins().jump(probe, &[at.into(), tried.into()]);
    b.switch_to_block(insert);
    let marker = b.ins().iadd_imm(ordinal, 1);
    b.ins().store(MemFlags::new(), marker, slot, 0);
    b.ins().jump(head, &[marker.into()]);
    b.switch_to_block(done);
}
pub(super) fn find(
    b: &mut FunctionBuilder<'_>,
    view: &[Value],
    key: &[Value],
    stride: usize,
    endian: Endianness,
    error: Block,
) -> (Value, Value) {
    let zero = b.ins().iconst(types::I64, 0);
    let one = b.ins().iconst(types::I64, 1);
    let indexed = b.create_block();
    let ordinary = b.create_block();
    let done = b.create_block();
    b.append_block_param(done, types::I64);
    b.append_block_param(done, types::I64);
    b.ins().brif(view[3], indexed, &[], ordinary, &[]);
    b.switch_to_block(ordinary);
    let (at, found) = maps::find(b, view[0], view[1], key, stride, endian);
    b.ins().jump(done, &[at.into(), found.into()]);
    b.switch_to_block(indexed);
    let hash = emit_hash(b, key[0], key[1]);
    let first = b.ins().band(hash, view[4]);
    let probe = b.create_block();
    b.append_block_param(probe, types::I64);
    b.append_block_param(probe, types::I64);
    let compare = b.create_block();
    let hit = b.create_block();
    let advance = b.create_block();
    b.ins().jump(probe, &[first.into(), zero.into()]);
    b.switch_to_block(probe);
    let at = b.block_params(probe)[0];
    let tried = b.block_params(probe)[1];
    let bounded = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, tried, view[4]);
    guard(b, bounded, error, 0x19503);
    let slot = address(b, view[3], at, 8);
    let marker = b.ins().load(types::I64, MemFlags::new(), slot, 0);
    b.ins()
        .brif(marker, compare, &[], done, &[zero.into(), zero.into()]);
    b.switch_to_block(compare);
    let ordinal = b.ins().iadd_imm(marker, -1);
    let bounded = b.ins().icmp(IntCC::UnsignedLessThan, ordinal, view[1]);
    guard(b, bounded, error, 0x19503);
    let target = address(b, view[0], ordinal, stride);
    let pointer = b.ins().load(types::I64, MemFlags::new(), target, 0);
    let length = b.ins().load(types::I64, MemFlags::new(), target, 8);
    let same = strings::equal(b, key[0], key[1], pointer, length);
    b.ins().brif(same, hit, &[], advance, &[]);
    b.switch_to_block(hit);
    b.ins().jump(done, &[ordinal.into(), one.into()]);
    b.switch_to_block(advance);
    let at = b.ins().iadd_imm(at, 1);
    let at = b.ins().band(at, view[4]);
    let tried = b.ins().iadd_imm(tried, 1);
    b.ins().jump(probe, &[at.into(), tried.into()]);
    b.switch_to_block(done);
    (b.block_params(done)[0], b.block_params(done)[1])
}
