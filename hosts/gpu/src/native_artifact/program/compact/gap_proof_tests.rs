use super::prove;

use cranelift_codegen::ir::{
    AbiParam, Function, InstBuilder, MemFlags, Value, condcodes::IntCC, types,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

fn indexed(b: &mut FunctionBuilder<'_>, data: Value, at: Value) -> Value {
    let bytes = b.ins().ishl_imm(at, 2);
    b.ins().iadd(data, bytes)
}

fn table(variant: u8) -> Function {
    let mut f = Function::new();
    f.signature.params.extend([AbiParam::new(types::I64); 3]);
    f.signature.returns.extend([AbiParam::new(types::I64); 2]);
    let mut context = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut f, &mut context);
    let entry = b.create_block();
    let gaps = b.create_block();
    let sort = b.create_block();
    let read = b.create_block();
    let insert = b.create_block();
    let compare = b.create_block();
    let shift = b.create_block();
    let place = b.create_block();
    let next_gap = b.create_block();
    let advance = b.create_block();
    let done = b.create_block();
    b.append_block_params_for_function_params(entry);
    for block in [gaps, sort, insert] {
        b.append_block_param(block, types::I64);
    }
    b.switch_to_block(entry);
    let packet = b.block_params(entry)[1];
    let out = b.block_params(entry)[2];
    let flags = MemFlags::trusted();
    let push = b.ins().load(types::I64, flags, packet, 0);
    let packed = b.ins().load(types::I64, flags, packet, 8);
    let base = b.ins().load(
        types::I64,
        flags,
        packet,
        if variant == 1 { 24 } else { 16 },
    );
    let zero = b.ins().iconst(types::I64, 0);
    let one = b.ins().iconst(types::I64, 1);
    b.ins().jump(gaps, &[packed.into()]);
    b.switch_to_block(gaps);
    let all = b.block_params(gaps)[0];
    let gap = b.ins().band_imm(all, 255);
    let exists = b.ins().icmp(IntCC::NotEqual, gap, zero);
    let fits = b.ins().icmp(IntCC::UnsignedLessThan, gap, push);
    let exists_wide = b.ins().uextend(types::I64, exists);
    let fits = b.ins().uextend(types::I64, fits);
    let ready = b.ins().band(exists_wide, fits);
    b.ins().brif(ready, sort, &[gap.into()], next_gap, &[]);
    b.switch_to_block(sort);
    let from = b.block_params(sort)[0];
    let more = b.ins().icmp(IntCC::UnsignedLessThan, from, push);
    b.ins().brif(more, read, &[], next_gap, &[]);
    b.switch_to_block(read);
    let key_at = indexed(&mut b, base, from);
    let key = b.ins().uload32(flags, key_at, 0);
    b.ins().jump(insert, &[from.into()]);
    b.switch_to_block(insert);
    let at = b.block_params(insert)[0];
    let can_move = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, at, gap);
    b.ins().brif(can_move, compare, &[], place, &[]);
    b.switch_to_block(compare);
    let previous = b.ins().isub(at, gap);
    let at_previous = indexed(&mut b, base, previous);
    let prior = b.ins().uload32(flags, at_previous, 0);
    let ordering = if variant == 2 {
        IntCC::UnsignedLessThan
    } else {
        IntCC::UnsignedGreaterThan
    };
    let moving = b.ins().icmp(ordering, prior, key);
    b.ins().brif(moving, shift, &[], place, &[]);
    b.switch_to_block(shift);
    let here = indexed(&mut b, base, at);
    let narrowed = b.ins().ireduce(types::I32, prior);
    if variant == 3 {
        b.ins().store(flags, narrowed, at_previous, 0);
    }
    b.ins().store(flags, narrowed, here, 0);
    b.ins().jump(insert, &[previous.into()]);
    b.switch_to_block(place);
    let here = indexed(&mut b, base, at);
    let narrowed = b
        .ins()
        .ireduce(types::I32, if variant == 4 { at } else { key });
    b.ins().store(flags, narrowed, here, 0);
    let next = b.ins().iadd(from, one);
    b.ins().jump(sort, &[next.into()]);
    b.switch_to_block(next_gap);
    b.ins().brif(exists, advance, &[], done, &[]);
    b.switch_to_block(advance);
    let rest = b.ins().ushr_imm(all, if variant == 5 { 16 } else { 8 });
    b.ins().jump(gaps, &[rest.into()]);
    b.switch_to_block(done);
    b.ins().store(flags, base, out, 8);
    b.ins()
        .return_(&[if variant == 6 { one } else { zero }, zero]);
    b.seal_all_blocks();
    b.finalize();
    f
}

#[test]
fn proves_the_whole_recurrence_and_rejects_different_observable_semantics() {
    let f = table(0);
    let witness = prove(&f).expect("exact gapped insertion");
    assert!(f.layout.inst_block(witness.entry).is_some());
    assert_eq!(
        f.dfg
            .block_params(witness.original.block(&f.dfg.value_lists))
            .len(),
        1
    );
    assert!(f.dfg.block_params(witness.success).is_empty());
    assert_ne!(witness.data, witness.length);
    assert_ne!(witness.gaps, witness.length);
    for variant in 1..=6 {
        assert!(prove(&table(variant)).is_none(), "variant {variant}");
    }
}

#[test]
fn nonzero_pack_requires_both_cursor_updates_and_preserves_faults() {
    use super::nonzero_pack::prove;
    use cranelift_codegen::cursor::{Cursor, FuncCursor};

    let mut f = Function::new();
    f.signature.params.extend([AbiParam::new(types::I64); 3]);
    f.signature.returns.extend([AbiParam::new(types::I64); 2]);
    let mut context = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut f, &mut context);
    let [entry, head, bounds, read, empty, space, store, done, error] =
        core::array::from_fn(|_| b.create_block());
    b.append_block_params_for_function_params(entry);
    let index = b.append_block_param(head, types::I64);
    let used = b.append_block_param(head, types::I64);
    b.switch_to_block(entry);
    let packet = b.block_params(entry)[1];
    let mem = MemFlags::new();
    let mut field = |offset| b.ins().load(types::I64, mem, packet, offset);
    let [start, limit, first, used0] = [0, 8, 16, 24].map(&mut field);
    let [input, input_len, output, output_len] = [32, 40, 64, 72].map(field);
    let zero = b.ins().iconst(types::I64, 0);
    let one = b.ins().iconst(types::I64, 1);
    b.ins().jump(head, &[first.into(), used0.into()]);
    b.switch_to_block(head);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, index, limit);
    let next = b.ins().iadd_imm(index, 1);
    let incremented = b.ins().iadd_imm(used, 1);
    b.ins().brif(more, bounds, &[], done, &[]);
    b.switch_to_block(bounds);
    let at = b.ins().iadd(start, index);
    let fits = b.ins().icmp(IntCC::UnsignedLessThan, at, input_len);
    b.ins().brif(fits, read, &[], empty, &[]);
    b.switch_to_block(read);
    let address = indexed(&mut b, input, at);
    let frequency = b.ins().uload32(mem, address, 0);
    let active = b.ins().icmp_imm(IntCC::NotEqual, frequency, 0);
    b.ins().brif(active, space, &[], empty, &[]);
    b.switch_to_block(empty);
    let skip = b.ins().jump(head, &[next.into(), used.into()]);
    b.switch_to_block(space);
    let fits = b.ins().icmp(IntCC::UnsignedLessThan, used, output_len);
    let capacity_gate = b.ins().brif(fits, store, &[], error, &[]);
    b.switch_to_block(store);
    let high = b.ins().ishl_imm(frequency, 10);
    let key = b.ins().bor(index, high);
    let narrowed = b.ins().ireduce(types::I32, key);
    let address = indexed(&mut b, output, used);
    b.ins().store(mem, narrowed, address, 0);
    b.ins().jump(head, &[next.into(), incremented.into()]);
    b.switch_to_block(error);
    b.ins().return_(&[one, zero]);
    b.switch_to_block(done);
    b.ins().return_(&[zero, zero]);
    b.seal_all_blocks();
    b.finalize();
    assert!(prove(&f).is_some());

    let mut wrong_key = f.clone();
    let shift = f.dfg.value_def(high).unwrap_inst();
    wrong_key.dfg.replace(shift).ishl_imm(frequency, 9);
    assert!(prove(&wrong_key).is_none());

    let mut count_zero = f.clone();
    count_zero
        .dfg
        .replace(skip)
        .jump(head, &[next.into(), incremented.into()]);
    assert!(prove(&count_zero).is_none());

    let mut unexpected_fault = f.clone();
    unexpected_fault
        .dfg
        .replace(capacity_gate)
        .brif(fits, error, &[], store, &[]);
    assert!(prove(&unexpected_fault).is_none());

    // The divide traps, even though no later instruction reads its result.
    FuncCursor::new(&mut f).at_inst(skip).ins().udiv(one, zero);
    assert!(prove(&f).is_none());
}
