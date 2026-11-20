use super::*;

pub(super) fn body(module: &JITModule, mode: u8) -> (Context, BTreeMap<Value, (Inst, usize)>) {
    let mut ctx = module.make_context();
    ctx.func.signature.params = vec![AbiParam::new(types::I64); 3];
    ctx.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fc = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut ctx.func, &mut fc);
    let entry = b.create_block();
    let outer = b.create_block();
    let header = b.create_block();
    let body = b.create_block();
    let split = b.create_block();
    let left = b.create_block();
    let right = b.create_block();
    let join = b.create_block();
    let read = b.create_block();
    let absent = b.create_block();
    let value_join = b.create_block();
    let update = b.create_block();
    let done = b.create_block();
    let budget_error = b.create_block();
    let source_error = b.create_block();
    b.append_block_params_for_function_params(entry);
    let outer_at = b.append_block_param(outer, types::I64);
    let outer_count = b.append_block_param(outer, types::I64);
    let outer_sum = b.append_block_param(outer, types::I64);
    let at = b.append_block_param(header, types::I64);
    let count = b.append_block_param(header, types::I64);
    let sum = b.append_block_param(header, types::I64);
    let index = b.append_block_param(join, types::I64);
    let value = b.append_block_param(value_join, types::I64);
    b.switch_to_block(entry);
    let packet = b.block_params(entry)[1];
    let output = b.block_params(entry)[2];
    let mut loads = BTreeMap::new();
    let fields: [Value; 5] = core::array::from_fn(|word| {
        let value = b
            .ins()
            .load(types::I64, MemFlags::new(), packet, (word * 8) as i32);
        loads.insert(value, (b.func.dfg.value_def(value).unwrap_inst(), word));
        value
    });
    let [begin, mut end, mut length, pointer, budget] = fields;
    if mode == 3 {
        end = b.ins().iadd_imm(end, 1);
    }
    if mode == 5 {
        length = b.ins().iadd_imm(length, 1);
    }
    let marker = b.ins().load(types::I64, MemFlags::new(), output, 24);
    let marker = b.ins().iadd_imm(marker, 1);
    b.ins().store(MemFlags::new(), marker, output, 24);
    let zero = b.ins().iconst(types::I64, 0);
    for offset in [0, 8, 16] {
        b.ins().store(MemFlags::new(), zero, output, offset);
    }
    if mode == 8 {
        let alternate = b.create_block();
        b.ins().brif(
            budget,
            outer,
            &[begin.into(), zero.into(), zero.into()],
            alternate,
            &[],
        );
        b.switch_to_block(alternate);
        b.ins().jump(outer, &[end.into(), zero.into(), zero.into()]);
    } else {
        b.ins()
            .jump(outer, &[begin.into(), zero.into(), zero.into()]);
    }
    b.switch_to_block(outer);
    b.ins().jump(
        header,
        &[outer_at.into(), outer_count.into(), outer_sum.into()],
    );
    b.switch_to_block(header);
    let early = (mode == 7).then(|| b.ins().iadd_imm(at, 1));
    let active = b.ins().icmp(IntCC::NotEqual, at, end);
    b.ins().brif(active, body, &[], done, &[]);
    b.switch_to_block(body);
    let funded = b.ins().icmp(IntCC::UnsignedLessThan, count, budget);
    b.ins().brif(funded, split, &[], budget_error, &[]);
    b.switch_to_block(split);
    let parity = b.ins().band_imm(at, 1);
    b.ins().brif(parity, left, &[], right, &[]);
    for block in [left, right] {
        b.switch_to_block(block);
        b.ins().jump(join, &[at.into()]);
    }
    b.switch_to_block(join);
    let index = if mode == 4 {
        b.ins().iadd_imm(index, 1)
    } else {
        index
    };
    let comparison = if mode == 2 {
        IntCC::SignedLessThan
    } else {
        IntCC::UnsignedLessThan
    };
    let fits = b.ins().icmp(comparison, index, length);
    b.ins().brif(fits, read, &[], absent, &[]);
    b.switch_to_block(read);
    let address = b.ins().iadd(pointer, index);
    let byte = b.ins().uload8(types::I64, MemFlags::new(), address, 0);
    b.ins().jump(value_join, &[byte.into()]);
    b.switch_to_block(absent);
    b.ins().jump(value_join, &[zero.into()]);
    b.switch_to_block(value_join);
    let error = b.ins().icmp_imm(IntCC::Equal, value, 255);
    b.ins().brif(error, source_error, &[], update, &[]);
    b.switch_to_block(update);
    let count = b.ins().iadd_imm(count, 1);
    let sum = b.ins().iadd(sum, value);
    b.ins().store(MemFlags::new(), count, output, 8);
    b.ins().store(MemFlags::new(), sum, output, 16);
    let next = if let Some(early) = early {
        early
    } else if mode == 6 {
        b.ins().iadd(at, budget)
    } else {
        b.ins().iadd_imm(at, if mode == 1 { 2 } else { 1 })
    };
    let alternate = b.ins().band_imm(count, 1);
    b.ins().brif(
        alternate,
        outer,
        &[next.into(), count.into(), sum.into()],
        header,
        &[next.into(), count.into(), sum.into()],
    );
    b.switch_to_block(done);
    b.ins().return_(&[zero]);
    for (block, origin, status) in [(budget_error, 102, 2), (source_error, 101, 1)] {
        b.switch_to_block(block);
        let origin = b.ins().iconst(types::I64, origin);
        b.ins().store(MemFlags::new(), origin, output, 0);
        let status = b.ins().iconst(types::I64, status);
        b.ins().return_(&[status]);
    }
    b.seal_all_blocks();
    b.finalize();
    (ctx, loads)
}
