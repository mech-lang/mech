use super::*;
use std::collections::{BTreeMap, BTreeSet};

#[cfg(target_arch = "aarch64")]
#[test]
fn horizontal_minima_keep_unsigned_values_ties_and_vector_identity() {
    use super::emit::minimum_word;
    use cranelift_codegen::ir::condcodes::IntCC;
    use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, types};
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
    use cranelift_jit::{JITBuilder, JITModule};
    use cranelift_module::{Linkage, Module, default_libcall_names};

    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", "speed"), ("enable_verifier", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let mut entries = Vec::new();
    for mode in 0..5 {
        let mut context = module.make_context();
        context.set_disasm(true);
        context.func.signature.params = vec![AbiParam::new(types::I64); 3];
        context.func.signature.returns = vec![AbiParam::new(types::I32)];
        let mut builder = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let [input, other, output] = b.block_params(entry)[..] else {
            unreachable!()
        };
        let values = b.ins().load(types::I32X4, MemFlags::new(), input, 0);
        let other = b.ins().load(types::I32X4, MemFlags::new(), other, 0);
        let minimum = if mode == 3 {
            // Almost the same tree, but the last lane belongs to another
            // vector. A same-vector reduction would be incorrect here.
            let a = b.ins().extractlane(values, 0);
            let b1 = b.ins().extractlane(values, 1);
            let c = b.ins().extractlane(values, 2);
            let d = b.ins().extractlane(other, 3);
            let low = b.ins().umin(a, b1);
            let high = b.ins().umin(c, d);
            b.ins().umin(low, high)
        } else {
            minimum_word(&mut b, values)
        };
        let broadcast = b.ins().splat(types::I32X4, minimum);
        if mode != 0 {
            b.ins().store(MemFlags::new(), broadcast, output, 0);
        }
        let result = if mode == 1 {
            b.ins().iconst(types::I32, 0)
        } else if mode == 4 {
            // Costs tied across different lanes still choose the earliest
            // original index, including an unchanged all-missing sentinel.
            let equal = b.ins().icmp(IntCC::Equal, values, broadcast);
            let missing = b.ins().iconst(types::I32, -1);
            let missing = b.ins().splat(types::I32X4, missing);
            let indices = b.ins().bitselect(equal, other, missing);
            minimum_word(&mut b, indices)
        } else {
            minimum
        };
        b.ins().return_(&[result]);
        b.seal_all_blocks();
        b.finalize();
        let id = module
            .declare_function(
                &format!("horizontal_minimum_{mode}"),
                Linkage::Local,
                &context.func.signature,
            )
            .unwrap();
        module.define_function(id, &mut context).unwrap();
        let assembly = context.compiled_code().unwrap().vcode.as_ref().unwrap();
        assert_eq!(assembly.contains("uminv"), mode != 3, "{assembly}");
        entries.push(id);
    }
    module.finalize_definitions().unwrap();
    type Entry = unsafe extern "C" fn(*const u32, *const u32, *mut u32) -> u32;
    let mut seed = 0x0123_4567_89ab_cdefu64;
    for (mode, id) in entries.into_iter().enumerate() {
        // SAFETY: the live module owns each function with this exact signature.
        let call: Entry = unsafe { std::mem::transmute(module.get_finalized_function(id)) };
        for sample in 0..512 {
            let mut next = || {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                seed as u32
            };
            let mut input = std::array::from_fn::<_, 4, _>(|_| next());
            let mut indices = std::array::from_fn::<_, 4, _>(|_| next());
            match sample % 8 {
                0 => input = [u32::MAX; 4],
                1 => input = [0; 4],
                2 => input = [u32::MAX, 0x8000_0000, 0x7fff_ffff, 0xffff_fffe],
                3 => input = [7, 7, 8, 7],
                4 => indices = [u32::MAX; 4],
                _ => {}
            }
            let minimum = if mode == 3 {
                *[input[0], input[1], input[2], indices[3]]
                    .iter()
                    .min()
                    .unwrap()
            } else {
                *input.iter().min().unwrap()
            };
            let expected = match mode {
                1 => 0,
                4 => input
                    .iter()
                    .zip(indices)
                    .filter_map(|(&cost, index)| (cost == minimum).then_some(index))
                    .min()
                    .unwrap(),
                _ => minimum,
            };
            let old_input = input;
            let old_indices = indices;
            let mut output = [0xa5a5_a5a5; 6];
            // SAFETY: input vectors and disjoint output have their full live
            // initialized extent. The output has one guard word on each side.
            let actual =
                unsafe { call(input.as_ptr(), indices.as_ptr(), output.as_mut_ptr().add(1)) };
            assert_eq!(actual, expected, "mode {mode}, sample {sample}");
            assert_eq!(input, old_input);
            assert_eq!(indices, old_indices);
            assert_eq!(output[0], 0xa5a5_a5a5);
            assert_eq!(output[5], 0xa5a5_a5a5);
            assert_eq!(
                output[1..5],
                [if mode == 0 { 0xa5a5_a5a5 } else { minimum }; 4]
            );
        }
    }
}

#[cfg(all(target_arch = "aarch64", target_endian = "little"))]
#[test]
fn reverse_packed_low_words_keeps_lane_order_and_unaligned_storage() {
    use super::cost::reverse_low_words;
    use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, types};
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
    use cranelift_jit::{JITBuilder, JITModule};
    use cranelift_module::{Linkage, Module, default_libcall_names};

    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", "speed"), ("enable_verifier", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 2];
    let mut builder = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
    let entry = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let [input, output] = b.block_params(entry)[..] else {
        unreachable!()
    };
    let first = b.ins().load(types::I8X16, MemFlags::new(), input, 0);
    let second = b.ins().load(types::I8X16, MemFlags::new(), input, 16);
    let result = reverse_low_words(&mut b, first, second);
    b.ins().store(MemFlags::new(), result, output, 0);
    b.ins().return_(&[]);
    b.seal_all_blocks();
    b.finalize();
    let id = module
        .declare_function(
            "reverse_packed_words",
            Linkage::Local,
            &context.func.signature,
        )
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    type Entry = unsafe extern "C" fn(*const u8, *mut u8);
    // SAFETY: the module stays live and its declared signature matches Entry.
    let call: Entry = unsafe { std::mem::transmute(module.get_finalized_function(id)) };
    for offset in 0..16 {
        for seed in [0u64, 1, u32::MAX.into(), u64::MAX, 0x0123_4567_89ab_cdef] {
            let words = std::array::from_fn::<_, 4, _>(|i| {
                seed.wrapping_add(0x071b_4a93_de68_25cfu64.wrapping_mul(i as u64))
            });
            let mut input = [0xa5; 64];
            for (i, word) in words.into_iter().enumerate() {
                input[offset + i * 8..offset + i * 8 + 8].copy_from_slice(&word.to_le_bytes());
            }
            let prior = input;
            let mut output = [0x5a; 64];
            // SAFETY: each unaligned pointer has its full initialized extent;
            // the exclusive output and shared input belong to separate arrays.
            unsafe { call(input.as_ptr().add(offset), output.as_mut_ptr().add(offset)) };
            assert_eq!(input, prior);
            assert!(output[..offset].iter().all(|&value| value == 0x5a));
            assert!(output[offset + 16..].iter().all(|&value| value == 0x5a));
            for (i, expected) in words.into_iter().rev().enumerate() {
                let actual = u32::from_le_bytes(
                    output[offset + i * 4..offset + i * 4 + 4]
                        .try_into()
                        .unwrap(),
                );
                assert_eq!(actual, expected as u32);
            }
        }
    }
}

fn candidate() -> Candidate {
    Candidate {
        tag: 100,
        active: 0,
        terminal: 1,
        induction: 101,
        count: 9,
        next_tag: 104,
        next_induction: 105,
    }
}
fn graph(nodes: Vec<Node>) -> Graph {
    Graph {
        nodes,
        advances: vec![BTreeMap::new()],
        exhausted: Vec::new(),
        written: BTreeSet::from([100, 101, 102, 103, 150]),
        terminal_inputs: Some(BTreeSet::new()),
    }
}

#[test]
fn ordered_reduction_keeps_reverse_length_bound_to_its_current_array() {
    let c = candidate();
    let mut g = graph(vec![
        Node::Initial(101),
        Node::Constant(3),
        Node::Scalar(Op::U64WrappingAdd, vec![0, 1]),
        Node::Length(0),
        Node::Scalar(Op::U64WrappingSub, vec![3, 2]),
        Node::Constant(1),
        Node::Scalar(Op::U64WrappingAdd, vec![4, 5]),
        Node::Read { array: 1, index: 6 },
    ]);
    let windows = [Window {
        induction: 101,
        count: 9,
        array: 1,
        delta: 3,
        reverse: true,
    }];
    assert!(
        Normalize {
            graph: &g,
            candidate: &c,
            windows: &windows,
            fuel: 128
        }
        .cost(7)
        .is_none()
    );
    g.nodes[3] = Node::Length(1);
    assert_eq!(
        Normalize {
            graph: &g,
            candidate: &c,
            windows: &windows,
            fuel: 128
        }
        .cost(7),
        Some(Cost::Read(windows[0]))
    );
}

#[test]
fn ordered_reduction_refuses_overwritten_captures_and_recursive_expressions() {
    let c = candidate();
    let g = graph(vec![
        Node::Initial(150),
        Node::Initial(500),
        Node::Scalar(Op::U64WrappingAdd, vec![2, 1]),
    ]);
    assert!(
        Normalize {
            graph: &g,
            candidate: &c,
            windows: &[],
            fuel: 128
        }
        .cost(0)
        .is_none()
    );
    assert_eq!(
        Normalize {
            graph: &g,
            candidate: &c,
            windows: &[],
            fuel: 128
        }
        .cost(1),
        Some(Cost::Word(500))
    );
    assert!(
        Normalize {
            graph: &g,
            candidate: &c,
            windows: &[],
            fuel: 128
        }
        .cost(2)
        .is_none()
    );
    assert!(
        Normalize {
            graph: &g,
            candidate: &c,
            windows: &[],
            fuel: 128
        }
        .item(0)
        .is_none()
    );
}

#[test]
fn ordered_reduction_requires_every_terminal_dependency_to_survive_elision() {
    let mut g = graph(vec![
        Node::Initial(100),
        Node::Initial(101),
        Node::Initial(102),
        Node::Initial(103),
        Node::Initial(150),
        Node::Initial(500),
        Node::Scalar(Op::U64Eq, vec![0, 0]),
    ]);
    g.terminal_inputs = Some(BTreeSet::from([0, 1, 2, 3, 5, 6]));
    assert!(terminal_local(&g, 100, 4));
    g.terminal_inputs.as_mut().unwrap().insert(4);
    assert!(!terminal_local(&g, 100, 4));
    g.terminal_inputs = None;
    assert!(!terminal_local(&g, 100, 4));
}

#[test]
fn ordered_reduction_requires_every_exhausted_payload_on_every_path() {
    let mut g = graph(vec![
        Node::Initial(101),
        Node::Initial(102),
        Node::Initial(103),
        Node::Constant(0),
        Node::Unknown,
    ]);
    assert!(!exhaustion_preserves_payload(&g, 100, 104, 4));
    let identity = BTreeMap::from([(105, 0), (106, 1), (107, 2)]);
    g.exhausted.push(identity.clone());
    assert!(exhaustion_preserves_payload(&g, 100, 104, 4));
    for slot in 105..108 {
        for replacement in [None, Some(3), Some(4), Some(usize::MAX)] {
            let mut edge = identity.clone();
            if let Some(id) = replacement {
                edge.insert(slot, id);
            } else {
                edge.remove(&slot);
            }
            g.exhausted.push(edge);
            assert!(
                !exhaustion_preserves_payload(&g, 100, 104, 4),
                "slot={slot}, replacement={replacement:?}"
            );
            g.exhausted.pop();
        }
    }
    g.exhausted.push(identity);
    assert!(exhaustion_preserves_payload(&g, 100, 104, 4));
}

#[cfg(all(target_arch = "aarch64", target_endian = "little"))]
#[test]
fn reverse_cursor_preindexed_load_respects_current_words() {
    use cranelift_codegen::ir::condcodes::IntCC;
    use cranelift_codegen::ir::{AbiParam, Endianness, InstBuilder, MemFlags, types};
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
    use cranelift_jit::{JITBuilder, JITModule};
    use cranelift_module::{Linkage, Module, default_libcall_names};

    type Scan = unsafe extern "C" fn(*const u8, u64, u64) -> u64;
    for (stride, offset, may_trap, big, uses_prior) in [
        (8, 0, false, false, false),
        (4, 0, false, false, false),
        (8, 0, false, false, true),
        (8, 4, false, false, false),
        (8, 0, true, false, false),
        (8, 0, false, true, false),
    ] {
        let mut module = JITModule::new(
            JITBuilder::with_flags(
                &[("opt_level", "speed"), ("regalloc_checker", "true")],
                default_libcall_names(),
            )
            .unwrap(),
        );
        let mut context = module.make_context();
        context.set_disasm(true);
        context.func.signature.params = vec![AbiParam::new(types::I64); 3];
        context.func.signature.returns = vec![AbiParam::new(types::I64)];
        let mut builder = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
            let entry = b.create_block();
            let init = b.create_block();
            let body = b.create_block();
            let done = b.create_block();
            b.append_block_params_for_function_params(entry);
            for _ in 0..3 {
                b.append_block_param(body, types::I64);
            }
            b.append_block_param(done, types::I64);
            b.switch_to_block(entry);
            let [base, start, count] = b.block_params(entry).try_into().unwrap();
            let zero = b.ins().iconst(types::I64, 0);
            let empty = b.ins().icmp_imm(IntCC::Equal, count, 0);
            let invalid = b.ins().icmp(IntCC::UnsignedGreaterThan, count, start);
            let reject = b.ins().bor(empty, invalid);
            b.ins().brif(reject, done, &[zero.into()], init, &[]);
            b.switch_to_block(init);
            let displacement = b.ins().imul_imm(start, stride);
            let prior = b.ins().iadd(base, displacement);
            b.ins()
                .jump(body, &[prior.into(), count.into(), zero.into()]);
            b.switch_to_block(body);
            let [prior, remaining, acc] = b.block_params(body).try_into().unwrap();
            let pointer = b.ins().iadd_imm(prior, -stride);
            let flags = if may_trap {
                MemFlags::new()
            } else {
                MemFlags::new().with_notrap()
            };
            let flags = if big {
                flags.with_endianness(Endianness::Big)
            } else {
                flags
            };
            let word = b.ins().load(types::I32, flags, pointer, offset);
            let word = b.ins().uextend(types::I64, word);
            let acc = b.ins().imul_imm(acc, 33);
            let acc = b.ins().iadd(acc, word);
            let acc = if uses_prior {
                let prior_offset = b.ins().isub(prior, base);
                b.ins().bxor(acc, prior_offset)
            } else {
                acc
            };
            let next = b.ins().iadd_imm(remaining, -1);
            let more = b.ins().icmp_imm(IntCC::NotEqual, next, 0);
            b.ins().brif(
                more,
                body,
                &[pointer.into(), next.into(), acc.into()],
                done,
                &[acc.into()],
            );
            b.switch_to_block(done);
            let result = b.block_params(done)[0];
            b.ins().return_(&[result]);
            b.seal_all_blocks();
            b.finalize();
        }
        let id = module
            .declare_function("reverse_cursor", Linkage::Local, &context.func.signature)
            .unwrap();
        let compiled = module.define_function(id, &mut context);
        if big {
            // This ISA rejects non-native endian loads in the existing
            // selector. The writeback rule must not bypass that rejection.
            let error = compiled.unwrap_err().to_string();
            assert!(
                error.contains("load.i32") && error.contains("big"),
                "{error}"
            );
            continue;
        }
        compiled.unwrap();
        let vcode = context.compiled_code().unwrap().vcode.as_ref().unwrap();
        let writeback = vcode
            .lines()
            .any(|line| line.trim_start().starts_with("ldr w") && line.trim_end().ends_with("]!"));
        assert_eq!(writeback, offset == 0 && !may_trap && !big, "{vcode}");
        module.finalize_definitions().unwrap();
        // SAFETY: The live finalized JIT owns this exact C signature.
        let call =
            unsafe { core::mem::transmute::<*const u8, Scan>(module.get_finalized_function(id)) };
        let table: [u8; 512] = std::array::from_fn(|i| (i as u8).wrapping_mul(97));
        for start in [0, 1, 7, 13, 32, 64] {
            for count in [0, 1, 2, 3, 15, 16, 19, 64, 65] {
                let mut expected = 0u64;
                if count <= start {
                    for visited in 0..count {
                        let prior = (start - visited) * stride as u64;
                        let index = (prior - stride as u64 + offset as u64) as usize;
                        let bytes = table[index..index + 4].try_into().unwrap();
                        let word = if big {
                            u32::from_be_bytes(bytes)
                        } else {
                            u32::from_le_bytes(bytes)
                        };
                        expected = expected.wrapping_mul(33).wrapping_add(u64::from(word));
                        if uses_prior {
                            expected ^= prior;
                        }
                    }
                }
                // SAFETY: The initialized table covers every read reached
                // after the count and start guards in the generated entry.
                assert_eq!(unsafe { call(table.as_ptr(), start, count) }, expected);
            }
        }
    }
}
