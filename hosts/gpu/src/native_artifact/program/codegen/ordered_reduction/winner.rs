//! Reconstruct the earliest winning index only after a strict cost improvement.
use super::emit::minimum_splat;
use cranelift_codegen::ir::{InstBuilder, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;

fn indices(
    b: &mut FunctionBuilder<'_>,
    costs: Value,
    minima: Value,
    indices: Value,
    missing: Value,
) -> Value {
    let equal = b.ins().icmp(IntCC::Equal, costs, minima);
    let missing = b.ins().splat(types::I32X4, missing);
    let selected = b.ins().bitselect(equal, indices, missing);
    minimum_splat(b, selected)
}

pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    costs: Value,
    positions: Value,
    best: Value,
    missing: Value,
    defer: bool,
) -> (Value, Value) {
    // Every lane was seeded with the incoming best. Only a strict improvement
    // records an index. Equal costs therefore retain the missing sentinel;
    // among improved lanes the earliest index can occur in any vector lane.
    let minima = minimum_splat(b, costs);
    if !defer {
        // Preserve the existing IR construction order when the option is off.
        let winners = indices(b, costs, minima, positions, missing);
        let minimum = b.ins().extractlane(minima, 0);
        let winner = b.ins().extractlane(winners, 0);
        return (minimum, winner);
    }
    let minimum = b.ins().extractlane(minima, 0);
    let improved = b.ins().icmp(IntCC::UnsignedLessThan, minimum, best);
    let select = b.create_block();
    let done = b.create_block();
    b.append_block_param(done, types::I32);
    b.ins().brif(improved, select, &[], done, &[missing.into()]);
    b.switch_to_block(select);
    let winners = indices(b, costs, minima, positions, missing);
    let winner = b.ins().extractlane(winners, 0);
    b.ins().jump(done, &[winner.into()]);
    b.switch_to_block(done);
    (minimum, b.block_params(done)[0])
}
