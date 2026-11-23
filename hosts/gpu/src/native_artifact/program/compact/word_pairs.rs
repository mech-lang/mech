//! Combine adjacent trusted packet words on targets with an integer pair access.
use cranelift_codegen::{
    cursor::{Cursor, FuncCursor},
    ir::{
        Endianness, Function, Inst, InstBuilder, InstructionData as Data, MemFlags, Opcode,
        StackSlot, Value, types,
    },
    isa::TargetIsa,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum AccessKind {
    Load,
    Store,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Address {
    Pointer(Value),
    Stack(StackSlot),
}

#[derive(Clone, Copy)]
struct Access {
    inst: Inst,
    kind: AccessKind,
    address: Address,
    word: Value,
    offset: i32,
}

fn access(
    function: &Function,
    inst: Inst,
    result_packet: Option<(Value, usize)>,
    stack_words: bool,
) -> Option<Access> {
    let dfg = &function.dfg;
    let (kind, address, word, flags, offset) = match dfg.insts[inst] {
        Data::Load {
            opcode: Opcode::Load,
            arg,
            flags,
            offset,
        } => (
            AccessKind::Load,
            Address::Pointer(dfg.resolve_aliases(arg)),
            dfg.first_result(inst),
            Some(flags),
            offset,
        ),
        Data::Store {
            opcode: Opcode::Store,
            args: [word, address],
            flags,
            offset,
        } => (
            AccessKind::Store,
            Address::Pointer(dfg.resolve_aliases(address)),
            word,
            Some(flags),
            offset,
        ),
        Data::StackLoad {
            opcode: Opcode::StackLoad,
            stack_slot,
            offset,
        } if stack_words => (
            AccessKind::Load,
            Address::Stack(stack_slot),
            dfg.first_result(inst),
            None,
            offset,
        ),
        Data::StackStore {
            opcode: Opcode::StackStore,
            stack_slot,
            arg,
            offset,
        } if stack_words => (
            AccessKind::Store,
            Address::Stack(stack_slot),
            arg,
            None,
            offset,
        ),
        _ => return None,
    };
    let offset = i32::from(offset);
    let nontrapping = match address {
        Address::Pointer(pointer) => {
            let packet_store = kind == AccessKind::Store
                && flags == Some(MemFlags::new())
                && result_packet.is_some_and(|(packet, words)| {
                    pointer == packet
                        && offset % 8 == 0
                        && usize::try_from(offset).is_ok_and(|offset| offset / 8 < words)
                });
            flags == Some(MemFlags::trusted())
                || flags == Some(MemFlags::new().with_notrap())
                || packet_store
        }
        Address::Stack(slot) => {
            let slot = &function.sized_stack_slots[slot];
            slot.align_shift >= 3
                && offset % 8 == 0
                && u32::try_from(offset).is_ok_and(|offset| offset + 8 <= slot.size)
        }
    };
    (nontrapping && dfg.value_type(word) == types::I64).then_some(Access {
        inst,
        kind,
        address,
        word,
        offset,
    })
}

fn pair(function: &Function, first: Access, next: Access) -> Option<(Access, Access)> {
    if first.kind != next.kind
        || first.address != next.address
        || function.srcloc(first.inst) != function.srcloc(next.inst)
    {
        return None;
    }
    let (lo, hi) = if first.offset < next.offset {
        (first, next)
    } else {
        (next, first)
    };
    // AArch64's signed scaled pair displacement encodes -64..=63 words.
    ((-512..=504).contains(&lo.offset) && lo.offset % 8 == 0 && hi.offset == lo.offset + 8)
        .then_some((lo, hi))
}

pub(super) fn available(isa: &dyn TargetIsa) -> bool {
    isa.name() == "aarch64" && isa.endianness() == Endianness::Little
}

pub(super) fn fold(
    function: &mut Function,
    result_packet: Option<(Value, usize)>,
    stack_words: bool,
) -> usize {
    if function.dfg.values_labels.is_some() || !function.debug_tags.is_empty() {
        return 0;
    }
    let mut edits = Vec::new();
    for block in function.layout.blocks() {
        let mut previous = None;
        for inst in function.layout.block_insts(block) {
            let next = access(function, inst, result_packet, stack_words);
            if let (Some(first), Some(second)) = (previous, next)
                && let Some((lo, hi)) = pair(function, first, second)
            {
                edits.push((first.inst, second.inst, lo, hi));
                previous = None;
            } else {
                previous = next;
            }
        }
    }
    for &(first, second, lo, hi) in &edits {
        let source = function.srcloc(first);
        let mut cursor = FuncCursor::new(function).at_inst(first);
        cursor.set_srcloc(source);
        // Claim only non-trapping for the combined IR memory access. Each
        // word can be individually aligned without the combined access being so.
        let flags = MemFlags::new().with_notrap();
        let address = match lo.address {
            Address::Pointer(pointer) => pointer,
            Address::Stack(slot) => cursor.ins().stack_addr(types::I64, slot, 0),
        };
        match lo.kind {
            AccessKind::Load => {
                let pair = cursor.ins().load(types::I128, flags, address, lo.offset);
                let (lower, upper) = cursor.ins().isplit(pair);
                for (access, replacement) in [(lo, lower), (hi, upper)] {
                    let dfg = &mut cursor.func.dfg;
                    dfg.clear_results(access.inst);
                    dfg.change_to_alias(access.word, replacement);
                }
            }
            AccessKind::Store => {
                let pair = cursor.ins().iconcat(lo.word, hi.word);
                cursor.ins().store(flags, pair, address, lo.offset);
            }
        }
        cursor.func.layout.remove_inst(first);
        cursor.func.layout.remove_inst(second);
    }
    edits.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::{AbiParam, StackSlotData, StackSlotKind};
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
    use cranelift_jit::{JITBuilder, JITModule};
    use cranelift_module::{Linkage, Module, default_libcall_names};

    #[test]
    fn pairs_trusted_words_and_retains_checked_accesses() {
        let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
        let mut context = module.make_context();
        context.func.signature.params = vec![AbiParam::new(types::I64); 2];
        let mut builder = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let [input, output] = *b.block_params(entry) else {
            unreachable!()
        };
        let trusted = MemFlags::trusted();
        let checked = MemFlags::new();
        let hi = b.ins().load(types::I64, trusted, input, 8);
        let lo = b.ins().load(types::I64, trusted, input, 0);
        let local =
            b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 16, 3));
        b.ins().stack_store(lo, local, 0);
        b.ins().stack_store(hi, local, 8);
        let hi = b.ins().stack_load(types::I64, local, 8);
        let lo = b.ins().stack_load(types::I64, local, 0);
        b.ins().store(checked, lo, output, 8);
        b.ins().store(checked, hi, output, 0);
        let extra = b.ins().load(types::I64, checked, input, 16);
        let tail = b.ins().load(types::I64, checked, input, 24);
        let sum = b.ins().iadd(extra, tail);
        b.ins().store(checked, sum, output, 16);
        b.ins().return_(&[]);
        b.seal_all_blocks();
        b.finalize();
        assert_eq!(fold(&mut context.func, Some((output, 2)), true), 4);
        context.verify(module.isa()).unwrap();
        let id = module
            .declare_function(
                "trusted_word_pairs",
                Linkage::Export,
                &context.func.signature,
            )
            .unwrap();
        module.define_function(id, &mut context).unwrap();
        module.finalize_definitions().unwrap();
        let function: extern "C" fn(*const u64, *mut u64) =
            unsafe { std::mem::transmute(module.get_finalized_function(id)) };
        let input = [10, 50, 11, 23];
        let mut output = [0; 3];
        function(input.as_ptr(), output.as_mut_ptr());
        assert_eq!(output, [50, 10, 34]);
    }
}
