use super::*;
use cranelift_codegen::ir::AbiParam;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

fn fixture(module: &JITModule, variant: &str) -> (Context, Certificate) {
    let mut c = module.make_context();
    c.func.signature.params = vec![AbiParam::new(types::I64); 4];
    c.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fb = FunctionBuilderContext::new();
    let certificate;
    {
        let mut b = FunctionBuilder::new(&mut c.func, &mut fb);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        let header = b.create_block();
        let body = b.create_block();
        let back = b.create_block();
        let exit = b.create_block();
        let mismatch = b.create_block();
        b.append_block_param(header, types::I64);
        if variant == "carried" {
            b.append_block_param(header, types::I64);
        }
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        let mut initial = vec![args[2].into()];
        if variant == "carried" {
            initial.push(args[2].into());
        }
        // Model the real certificate's ordered-entry gate. The constructed
        // buffers cover every tested limit; failed admission keeps scalar exit.
        let admitted = b.create_block();
        let refused = b.create_block();
        let ordered = b
            .ins()
            .icmp(IntCC::UnsignedLessThanOrEqual, args[2], args[3]);
        b.ins().brif(ordered, admitted, &[], refused, &[]);
        b.switch_to_block(refused);
        b.ins().return_(&[args[2]]);
        b.switch_to_block(admitted);
        if variant == "extra_entry" {
            let alternate = b.create_block();
            let choose = b.ins().icmp_imm(IntCC::Equal, args[2], 0);
            b.ins().brif(choose, header, &initial, alternate, &[]);
            b.switch_to_block(alternate);
        }
        b.ins().jump(header, &initial);
        b.switch_to_block(header);
        let at = b.block_params(header)[0];
        let sum = b.ins().iadd_imm(at, 8);
        let valid = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, sum, args[3]);
        b.ins().brif(valid, body, &[], exit, &[]);
        b.switch_to_block(body);
        let a = b.ins().iadd(args[0], at);
        let d = b.ins().iadd(args[1], at);
        let flags = MemFlags::new().with_endianness(if variant == "big" {
            Endianness::Big
        } else {
            Endianness::Little
        });
        let a = b.ins().load(types::I64, flags, a, 0);
        let d = b.ins().load(types::I64, flags, d, 0);
        if variant == "store" {
            b.ins().store(MemFlags::new(), a, args[0], 0);
        }
        let difference = b.ins().bxor(a, d);
        // Use ordinary Icmp with a constant, as the shared source emitter does.
        let zero = b.ins().iconst(types::I64, 0);
        let equal = if variant == "immediate" {
            b.ins().icmp_imm(IntCC::Equal, difference, 0)
        } else {
            b.ins().icmp(IntCC::Equal, difference, zero)
        };
        b.ins().brif(equal, back, &[], mismatch, &[]);
        b.switch_to_block(back);
        let next = b
            .ins()
            .iadd_imm(at, if variant == "stride" { 12 } else { 8 });
        let mut next_args = vec![next.into()];
        if variant == "carried" {
            next_args.push(next.into());
        }
        b.ins().jump(header, &next_args);
        b.switch_to_block(mismatch);
        b.ins().return_(&[at]);
        b.switch_to_block(exit);
        b.ins().return_(&[at]);
        b.seal_all_blocks();
        b.finalize();
        certificate = Certificate {
            header,
            blocks: BTreeSet::from([header, body, back]),
            source_header: 7,
            reuse_scope: None,
        };
    }
    (c, certificate)
}

#[test]
fn compact_wide_equal_preserves_first_bad_word_tails_and_current_buffers() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let (mut c, certificate) = fixture(&module, "normal");
    assert_eq!(optimize(&mut c, &[certificate], false).wider, [7]);
    cranelift_codegen::verify_function(&c.func, module.isa()).unwrap();
    let id = module
        .declare_function("wide_equality_control", Linkage::Local, &c.func.signature)
        .unwrap();
    module.define_function(id, &mut c).unwrap();
    module.finalize_definitions().unwrap();
    let function: unsafe extern "C" fn(*const u8, *const u8, u64, u64) -> u64 =
        unsafe { std::mem::transmute(module.get_finalized_function(id)) };
    for offset in 0..16usize {
        for start in 0..17u64 {
            for limit in [
                0, 7, 8, 9, 15, 16, 17, 23, 24, 25, 31, 32, 33, 39, 40, 41, 47, 48, 49, 63, 64, 65,
                79, 80, 95, 96, 97, 127, 128, 129, 191, 192, 193, 255, 256, 257,
            ] {
                let left = [0u8; 304];
                for (mismatch, bit) in (0..=272usize).flat_map(|at| [1u8, 128].map(|bit| (at, bit)))
                {
                    let mut right = [0u8; 304];
                    if mismatch < 272 {
                        right[offset + mismatch] = bit;
                    }
                    let mut expected = start;
                    while expected + 8 <= limit {
                        let at = offset + expected as usize;
                        if left[at..at + 8] != right[at..at + 8] {
                            break;
                        }
                        expected += 8;
                    }
                    let actual = unsafe {
                        function(
                            left.as_ptr().add(offset),
                            right.as_ptr().add(offset),
                            start,
                            limit,
                        )
                    };
                    assert_eq!(actual, expected, "{offset} {start} {limit} {mismatch}");
                }
            }
        }
    }
}

#[test]
fn compact_wide_equal_keeps_scalar_backedge_after_entering_tiers() {
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let (mut context, certificate) = fixture(&module, "normal");
    let header = certificate.header;
    context.compute_cfg();
    context.compute_domtree();
    let predecessors = context
        .cfg
        .pred_iter(header)
        .map(|predecessor| predecessor.inst)
        .collect::<Vec<_>>();
    let back = predecessors
        .iter()
        .copied()
        .find(|&inst| {
            context
                .domtree
                .dominates(header, inst, &context.func.layout)
        })
        .unwrap();
    let entry = predecessors.into_iter().find(|&inst| inst != back).unwrap();
    assert_eq!(optimize(&mut context, &[certificate], false).wider, [7]);
    let destinations = |inst| {
        context.func.dfg.insts[inst]
            .branch_destination(
                &context.func.dfg.jump_tables,
                &context.func.dfg.exception_tables,
            )
            .iter()
            .map(|destination| destination.block(&context.func.dfg.value_lists))
            .collect::<Vec<_>>()
    };
    assert_eq!(destinations(back), vec![header]);
    assert_ne!(destinations(entry), vec![header]);
    cranelift_codegen::verify_function(&context.func, module.isa()).unwrap();
}

#[test]
fn compact_wide_equal_refuses_effects_other_steps_extra_state_or_unproved_geometry() {
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    for variant in [
        "store",
        "stride",
        "carried",
        "big",
        "immediate",
        "extra_entry",
    ] {
        let (mut c, certificate) = fixture(&module, variant);
        let before = c.func.display().to_string();
        assert!(
            optimize(&mut c, &[certificate], false).wider.is_empty(),
            "{variant}"
        );
        assert_eq!(c.func.display().to_string(), before);
    }
    let (mut c, mut certificate) = fixture(&module, "normal");
    certificate.blocks.clear();
    assert!(optimize(&mut c, &[certificate], false).wider.is_empty());
}

#[test]
fn vector_nonzero_branch_preserves_every_bit_and_normalized_observation() {
    // The instruction-selector rule also serves ordinary vector predicates.
    // An intervening aliased write must not change the captured vector, and a
    // separately stored Boolean must remain exactly zero or one.
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut functions = Vec::new();
    for ty in [
        types::I8X16,
        types::I16X8,
        types::I32X4,
        types::I64X2,
        types::F32X4,
        types::F64X2,
    ] {
        for clear in [false, true] {
            let mut c = module.make_context();
            c.func.signature.params = vec![AbiParam::new(types::I64); 2];
            c.func.signature.returns = vec![AbiParam::new(types::I64)];
            let mut fb = FunctionBuilderContext::new();
            {
                let mut b = FunctionBuilder::new(&mut c.func, &mut fb);
                let entry = b.create_block();
                let yes = b.create_block();
                let no = b.create_block();
                b.append_block_params_for_function_params(entry);
                b.switch_to_block(entry);
                let args = b.block_params(entry).to_vec();
                let captured = b.ins().load(ty, MemFlags::new(), args[0], 0);
                let predicate = b.ins().vany_true(captured);
                b.ins().store(MemFlags::new(), predicate, args[1], 0);
                if clear {
                    let zero = b.ins().iconst(types::I64, 0);
                    b.ins().store(MemFlags::new(), zero, args[0], 0);
                    b.ins().store(MemFlags::new(), zero, args[0], 8);
                }
                b.ins().brif(predicate, yes, &[], no, &[]);
                for (block, offset) in [(yes, 16), (no, 24)] {
                    b.switch_to_block(block);
                    let current = b.ins().load(types::I64, MemFlags::new(), args[0], offset);
                    b.ins().return_(&[current]);
                }
                b.seal_all_blocks();
                b.finalize();
            }
            cranelift_codegen::verify_function(&c.func, module.isa()).unwrap();
            let id = module
                .declare_function(
                    &format!("vector_truth_{ty}_{clear}"),
                    Linkage::Local,
                    &c.func.signature,
                )
                .unwrap();
            module.define_function(id, &mut c).unwrap();
            functions.push((id, clear));
        }
    }
    module.finalize_definitions().unwrap();
    let mut patterns = vec![[0u8; 16]];
    for bit in 0..128 {
        let mut bytes = [0u8; 16];
        bytes[bit / 8] = 1 << (bit % 8);
        patterns.push(bytes);
    }
    patterns.extend([[255; 16], [128; 16], [85; 16], [170; 16]]);
    let mut calls = 0;
    for (id, clear) in functions {
        let function: unsafe extern "C" fn(*mut u8, *mut u8) -> u64 =
            unsafe { std::mem::transmute(module.get_finalized_function(id)) };
        for bytes in &patterns {
            let mut input = [0u8; 32];
            input[..16].copy_from_slice(bytes);
            let yes = 0x123456789abcdef0u64;
            let no = 0xfedcba9876543210u64;
            input[16..24].copy_from_slice(&yes.to_ne_bytes());
            input[24..32].copy_from_slice(&no.to_ne_bytes());
            let mut observed = 42;
            let actual = unsafe { function(input.as_mut_ptr(), &mut observed) };
            let truth = bytes.iter().any(|&v| v != 0);
            assert_eq!(actual, if truth { yes } else { no });
            assert_eq!(observed, u8::from(truth));
            assert_eq!(&input[..16], if clear { &[0u8; 16] } else { bytes });
            calls += 1;
        }
    }
    assert_eq!(calls, 1596);
}
