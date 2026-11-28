//! Pair writes already admitted by the exclusive NoGrowth append contract.
//!
//! The caller supplies its complete, validated set of current append writes
//! on a little-endian target. Every selected store stays within its admitted,
//! exclusively owned output span. This is not a general memory-motion pass.
use cranelift_codegen::{
    cursor::{Cursor, FuncCursor},
    ir::{
        Endianness, Function, Inst, InstBuilder, InstructionData as Data, MemFlags, Opcode, Value,
        types,
    },
};
use std::collections::BTreeSet;

#[derive(Clone, Copy)]
struct Store {
    inst: Inst,
    value: Value,
    address: Value,
    flags: MemFlags,
    offset: i32,
}

fn store(function: &Function, inst: Inst, admitted: &BTreeSet<Inst>) -> Option<Store> {
    if !admitted.contains(&inst) {
        return None;
    }
    let Data::Store {
        opcode: Opcode::Store,
        args: [value, address],
        flags,
        offset,
    } = function.dfg.insts[inst]
    else {
        return None;
    };
    // Individual eight-byte alignment does not establish sixteen-byte
    // alignment. Retain all flags verbatim; never strengthen that claim.
    (function.dfg.value_type(value) == types::I64
        && !flags.aligned()
        && flags.endianness(Endianness::Little) == Endianness::Little)
        .then_some(Store {
            inst,
            value,
            address: function.dfg.resolve_aliases(address),
            flags,
            offset: offset.into(),
        })
}

pub(super) fn fold(function: &mut Function, stores: &[Inst]) -> usize {
    if function.dfg.values_labels.is_some() || !function.debug_tags.is_empty() {
        return 0;
    }
    let admitted = stores.iter().copied().collect::<BTreeSet<_>>();
    let mut pairs = Vec::new();
    for block in function.layout.blocks() {
        let mut previous: Option<(Store, bool)> = None;
        for inst in function.layout.block_insts(block) {
            let next = store(function, inst, &admitted);
            if let (Some((first, _)), Some(second)) = (previous, next)
                && first.address == second.address
                && first.flags == second.flags
                && first.offset % 8 == 0
                && (-512..=504).contains(&first.offset)
                && first.offset.checked_add(8) == Some(second.offset)
                && function.srcloc(first.inst) == function.srcloc(second.inst)
            {
                pairs.push((first, second));
                previous = None;
            } else if let Some(second) = next {
                previous = Some((second, false));
            } else if let Some((first, false)) = previous
                && matches!(function.dfg.insts[inst], Data::Unary {
                    opcode: Opcode::Ireduce, arg,
                } if function.dfg.resolve_aliases(arg) == function.dfg.resolve_aliases(first.value))
            {
                // Previous-value forwarding inserts this pure projection
                // immediately after a write. Delay that write past only this
                // one nontrapping instruction, leaving the projection intact.
                previous = Some((first, true));
            } else {
                previous = None;
            }
        }
    }
    for &(first, second) in &pairs {
        let source = function.srcloc(first.inst);
        let mut cursor = FuncCursor::new(function).at_inst(second.inst);
        cursor.set_srcloc(source);
        let pair = cursor.ins().iconcat(first.value, second.value);
        cursor
            .ins()
            .store(first.flags, pair, first.address, first.offset);
        cursor.func.layout.remove_inst(first.inst);
        cursor.func.layout.remove_inst(second.inst);
    }
    pairs.len()
}

#[cfg(all(test, feature = "jit"))]
#[path = "append_store_pairs_tests.rs"]
mod tests;
