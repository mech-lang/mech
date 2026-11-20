use super::*;

fn body(module: &JITModule, mode: u8) -> (Context, BTreeMap<Value, (Inst, usize)>) {
    let mut ctx = module.make_context();
    ctx.func.signature.params = vec![AbiParam::new(types::I64); 3];
    ctx.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fc = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut ctx.func, &mut fc);
    let entry = b.create_block();
    let header = b.create_block();
    let active = b.create_block();
    let choose = b.create_block();
    let small = b.create_block();
    let large = b.create_block();
    let window = b.create_block();
    let extent = b.create_block();
    let readable = b.create_block();
    let update = b.create_block();
    let done = b.create_block();
    let errors = [(); 4].map(|_| b.create_block());
    b.append_block_params_for_function_params(entry);
    let cursor = b.append_block_param(header, types::I64);
    let count = b.append_block_param(header, types::I64);
    let sum = b.append_block_param(header, types::I64);
    let saved = (mode == 4).then(|| b.append_block_param(header, types::I64));
    let width = b.append_block_param(window, types::I64);
    b.switch_to_block(entry);
    let args = b.block_params(entry).to_vec();
    let mut loads = BTreeMap::new();
    let fields: [Value; 5] = core::array::from_fn(|word| {
        let value = b
            .ins()
            .load(types::I64, MemFlags::new(), args[1], (word * 8) as i32);
        loads.insert(value, (b.func.dfg.value_def(value).unwrap_inst(), word));
        value
    });
    let [begin, end, length, cap, budget] = fields;
    let zero = b.ins().iconst(types::I64, 0);
    let marker = b.ins().load(types::I64, MemFlags::new(), args[2], 24);
    let marker = b.ins().iadd_imm(marker, 1);
    b.ins().store(MemFlags::new(), marker, args[2], 24);
    for offset in [0, 8, 16] {
        b.ins().store(MemFlags::new(), zero, args[2], offset);
    }
    let mut initial = vec![begin.into(), zero.into(), zero.into()];
    if saved.is_some() {
        initial.push(b.ins().isub(length, begin).into());
    }
    b.ins().jump(header, &initial);
    b.switch_to_block(header);
    let live = b.ins().icmp(IntCC::NotEqual, cursor, end);
    b.ins().brif(live, active, &[], done, &[]);
    b.switch_to_block(active);
    let funded = b.ins().icmp(IntCC::UnsignedLessThan, count, budget);
    b.ins().brif(funded, choose, &[], errors[0], &[]);
    b.switch_to_block(choose);
    let actual_length = if mode == 3 {
        b.ins().iadd_imm(length, 1)
    } else {
        length
    };
    let remaining = b.ins().isub(actual_length, cursor);
    let comparison = if mode == 1 {
        IntCC::SignedLessThan
    } else {
        IntCC::UnsignedLessThan
    };
    let selected = b.ins().icmp(comparison, remaining, cap);
    b.ins().brif(selected, small, &[], large, &[]);
    for (block, value) in [(small, remaining), (large, cap)] {
        b.switch_to_block(block);
        b.ins().jump(window, &[value.into()]);
    }
    b.switch_to_block(window);
    let base = if mode == 2 {
        b.ins().iadd_imm(cursor, 1)
    } else {
        let delta = b.ins().isub(cursor, cap);
        b.ins().iadd(cap, delta)
    };
    let stop = b.ins().iadd(base, saved.unwrap_or(width));
    let no_overflow = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, stop, base);
    b.ins().brif(no_overflow, extent, &[], errors[1], &[]);
    b.switch_to_block(extent);
    let fits = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, stop, length);
    b.ins().brif(fits, readable, &[], errors[2], &[]);
    b.switch_to_block(readable);
    let byte_fits = b.ins().icmp(IntCC::UnsignedLessThan, cursor, length);
    b.ins().brif(byte_fits, update, &[], errors[3], &[]);
    b.switch_to_block(update);
    let next_count = b.ins().iadd_imm(count, 1);
    let next_sum = b.ins().iadd(sum, stop);
    b.ins().store(MemFlags::new(), next_count, args[2], 8);
    b.ins().store(MemFlags::new(), next_sum, args[2], 16);
    let next = b.ins().iadd_imm(cursor, 1);
    let mut backedge = vec![next.into(), next_count.into(), next_sum.into()];
    if let Some(saved) = saved {
        backedge.push(saved.into());
    }
    b.ins().jump(header, &backedge);
    b.switch_to_block(done);
    b.ins().return_(&[zero]);
    for (index, block) in errors.into_iter().enumerate() {
        b.switch_to_block(block);
        let status = b.ins().iconst(types::I64, index as i64 + 1);
        let origin = b.ins().iconst(types::I64, index as i64 + 100);
        b.ins().store(MemFlags::new(), origin, args[2], 0);
        b.ins().return_(&[status]);
    }
    b.seal_all_blocks();
    b.finalize();
    (ctx, loads)
}

fn compile(mode: u8) -> (JITModule, Run, Run) {
    let module = JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", "speed"), ("regalloc_checker", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let (old, loads) = body(&module, mode);
    let mut fast = module.make_context();
    fast.func = old.func.clone();
    let spans = fold(&mut fast, &loads, &BTreeSet::from([2]), true);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].checks_removed, if mode == 0 { 3 } else { 1 });
    assert_eq!(
        spans[0].window_checks_removed,
        if mode == 0 { 2 } else { 0 }
    );
    if mode == 0 {
        let branches = |ctx: &Context| {
            ctx.func
                .layout
                .blocks()
                .filter(|&block| {
                    ctx.func
                        .layout
                        .last_inst(block)
                        .is_some_and(|inst| matches!(ctx.func.dfg.insts[inst], Data::Brif { .. }))
                })
                .count()
        };
        assert_eq!(branches(&old) - branches(&fast), 3);
    }
    if mode != 0 {
        let mut plain = module.make_context();
        plain.func = old.func.clone();
        assert_eq!(fold(&mut plain, &loads, &BTreeSet::from([2]), false), spans);
        assert_eq!(
            plain.func.display().to_string(),
            fast.func.display().to_string()
        );
    }
    finish(module, old, fast, spans)
}

#[test]
fn admitted_clamped_window_preserves_errors_effects_and_full_word_boundaries() {
    let (_module, old, guarded) = compile(0);
    let mut calls = 0;
    for begin in [0, 1, 7, 8, 15, u64::MAX - 1, u64::MAX] {
        for end in [0, 1, 8, 16, u64::MAX] {
            for length in [0, 1, 8, 16, u64::MAX - 1, u64::MAX] {
                for cap in [0, 1, 8, 258, u64::MAX] {
                    for budget in [0, 1, 2] {
                        for marker in [0, u64::MAX] {
                            let packet = [begin, end, length, cap, budget];
                            let mut before = [77, 88, 99, marker];
                            let mut after = before;
                            let status = invoke(old, &packet, &mut before);
                            assert_eq!(
                                (invoke(guarded, &packet, &mut after), after),
                                (status, before),
                                "{packet:?}"
                            );
                            calls += 2;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(calls, 12600);
}

#[test]
fn unsupported_clamps_keep_the_same_graph_as_plain_input_span_admission() {
    for mode in 1..=4 {
        let (_module, old, guarded) = compile(mode);
        for begin in [0, 1, 7, u64::MAX - 1] {
            for length in [0, 1, 8, u64::MAX] {
                for cap in [0, 1, 258, u64::MAX] {
                    let packet = [begin, length, length, cap, 2];
                    let mut before = [77, 88, 99, u64::MAX];
                    let mut after = before;
                    let status = invoke(old, &packet, &mut before);
                    assert_eq!(
                        (invoke(guarded, &packet, &mut after), after),
                        (status, before),
                        "{mode}/{packet:?}"
                    );
                }
            }
        }
    }
}
