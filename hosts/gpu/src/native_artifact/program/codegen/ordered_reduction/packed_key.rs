//! A bounded unsigned cost/ordinal key. Zero ordinal preserves the old choice.
use super::emit::minimum_word;
use cranelift_codegen::ir::{
    ConstantData, Endianness, InstBuilder, MemFlags, Value, condcodes::IntCC, types,
};
use cranelift_frontend::FunctionBuilder;

pub(super) fn fits(
    b: &mut FunctionBuilder<'_>,
    best: Value,
    stop: Value,
    count_admitted: bool,
) -> Value {
    let best = b.ins().icmp_imm(IntCC::UnsignedLessThan, best, 65536);
    if count_admitted {
        best
    } else {
        let count = b.ins().icmp_imm(IntCC::UnsignedLessThan, stop, 65536);
        b.ins().band(best, count)
    }
}

pub(super) fn initial(b: &mut FunctionBuilder<'_>, best: Value) -> Value {
    let key = b.ins().ishl_imm(best, 16);
    b.ins().splat(types::I32X4, key)
}

pub(super) fn halfwords(b: &mut FunctionBuilder<'_>, values: [u16; 8]) -> Value {
    let bytes = values
        .into_iter()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    let data = b.func.dfg.constants.insert(ConstantData::from(bytes));
    b.ins().vconst(types::I16X8, data)
}

pub(super) fn key(b: &mut FunctionBuilder<'_>, costs: Value, positions: Value) -> Value {
    // best <= U16::MAX, so a larger candidate cannot win. Supplying a zero
    // high half permits one UQXTN; ZIP1 consumes only the four low halfwords.
    let zero = b.ins().iconst(types::I32, 0);
    let zero = b.ins().splat(types::I32X4, zero);
    let costs = b.ins().uunarrow(costs, zero);
    let flags = MemFlags::new().with_endianness(Endianness::Little);
    let positions = b.ins().bitcast(types::I8X16, flags, positions);
    let costs = b.ins().bitcast(types::I8X16, flags, costs);
    let mask = b.func.dfg.immediates.push(ConstantData::from(vec![
        0, 1, 16, 17, 2, 3, 18, 19, 4, 5, 20, 21, 6, 7, 22, 23,
    ]));
    let packed = b.ins().shuffle(positions, costs, mask);
    b.ins().bitcast(types::I32X4, flags, packed)
}

pub(super) fn finish(b: &mut FunctionBuilder<'_>, keys: Value) -> (Value, Value) {
    let key = minimum_word(b, keys);
    let cost = b.ins().ushr_imm(key, 16);
    let biased = b.ins().band_imm(key, 65535);
    let index = b.ins().iadd_imm(biased, -1);
    (cost, index)
}

/// Widen before removing the ordinal bias. A full-width item offset can then
/// fold into this subtraction, without a wrapping I32 operation in between.
/// Zero ordinal becomes I64 -1; the item's unchanged sentinel test discards
/// the pure total extension of that index and preserves the complete old item.
pub(super) fn finish_wide_item(b: &mut FunctionBuilder<'_>, keys: Value) -> (Value, Value) {
    let key = minimum_word(b, keys);
    let cost = b.ins().ushr_imm(key, 16);
    let biased = b.ins().band_imm(key, 65535);
    let biased = b.ins().uextend(types::I64, biased);
    let index = b.ins().iadd_imm(biased, -1);
    (cost, index)
}
