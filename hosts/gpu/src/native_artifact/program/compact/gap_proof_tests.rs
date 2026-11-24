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
