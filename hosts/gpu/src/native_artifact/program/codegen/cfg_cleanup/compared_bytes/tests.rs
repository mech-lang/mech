use super::*;
use cranelift_codegen::ir::AbiParam;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

fn read(
    b: &mut FunctionBuilder<'_>,
    pointer: Value,
    index: Value,
    length: Value,
    marker: Value,
    write: bool,
) -> (Value, Value) {
    let yes = b.create_block();
    let no = b.create_block();
    let join = b.create_block();
    b.append_block_param(join, types::I64);
    b.append_block_param(join, types::I64);
    let valid = b.ins().icmp(IntCC::UnsignedLessThan, index, length);
    let masked = b.ins().band_imm(valid, 1);
    b.ins().brif(masked, yes, &[], no, &[]);
    b.switch_to_block(yes);
    let at = b.ins().iadd(pointer, index);
    let at = b.ins().iadd_imm(at, 1);
    let at = b.ins().iadd_imm(at, -1);
    let value = b.ins().uload8(types::I64, MemFlags::new(), at, 0);
    if write {
        let byte = b.ins().iconst(types::I8, 7);
        b.ins().store(MemFlags::new(), byte, pointer, 0);
    }
    b.ins().jump(join, &[marker.into(), value.into()]);
    b.switch_to_block(no);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().jump(join, &[marker.into(), zero.into()]);
    b.switch_to_block(join);
    let values = b.block_params(join);
    (values[0], values[1])
}

fn compile(
    optimize: bool,
    write_before: bool,
    write_inside: bool,
    advanced: bool,
    admitted: bool,
    bounded: u8,
) -> (
    JITModule,
    unsafe extern "C" fn(*mut u8, u64, u64, u64, u64) -> u64,
    usize,
    usize,
) {
    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", "speed"), ("enable_verifier", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let mut c = module.make_context();
    c.func.signature.params = vec![AbiParam::new(types::I64); 5];
    c.func.signature.returns.push(AbiParam::new(types::I64));
    let mut fc = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut c.func, &mut fc);
    let entry = b.create_block();
    let left = b.create_block();
    let right = b.create_block();
    let position = b.create_block();
    let forwarded = b.create_block();
    let equal = b.create_block();
    let order = b.create_block();
    b.append_block_param(position, types::I64);
    b.append_block_param(forwarded, types::I64);
    b.append_block_params_for_function_params(entry);
    for _ in 0..3 {
        b.append_block_param(order, types::I64);
    }
    b.switch_to_block(entry);
    let args = b.block_params(entry).to_vec();
    let pointer = args[0];
    let choose = b.ins().band_imm(args[4], 1);
    b.ins().brif(choose, left, &[], right, &[]);
    b.switch_to_block(left);
    b.ins().jump(position, &[args[2].into()]);
    b.switch_to_block(right);
    b.ins().jump(position, &[args[3].into()]);
    b.switch_to_block(position);
    let index = b.block_params(position)[0];
    b.ins().jump(forwarded, &[index.into()]);
    b.switch_to_block(forwarded);
    let transported_index = b.block_params(forwarded)[0];
    // These two names identify the same current dynamic phi. The producer
    // and consumer deliberately use different forwarding layers.
    let producer_length = if bounded == 2 {
        b.ins().iadd_imm(args[1], -1)
    } else {
        args[1]
    };
    if bounded != 0 {
        let first = b
            .ins()
            .icmp(IntCC::UnsignedLessThan, index, producer_length);
        let second = b
            .ins()
            .icmp(IntCC::UnsignedLessThan, args[3], producer_length);
        let valid = b.ins().band(first, second);
        let reads = b.create_block();
        let exit = b.create_block();
        b.ins().brif(valid, reads, &[], exit, &[]);
        b.switch_to_block(exit);
        b.ins().return_(&[args[4]]);
        b.switch_to_block(reads);
    }
    let at = b.ins().iadd(pointer, index);
    let at = b.ins().iadd_imm(at, -1);
    let at = b.ins().iadd_imm(at, 1);
    let a = b.ins().uload8(types::I64, MemFlags::new(), at, 0);
    let at = b.ins().iadd(pointer, args[3]);
    let z = b.ins().uload8(types::I64, MemFlags::new(), at, 0);
    let bounds = if bounded != 0 {
        vec![
            BoundedRead::new(b.func, a, 0).unwrap(),
            BoundedRead::new(b.func, z, 0).unwrap(),
        ]
    } else {
        Vec::new()
    };
    if write_before {
        let byte = b.ins().iconst(types::I8, 255);
        b.ins().store(MemFlags::new(), byte, pointer, 0);
    }
    let same = b.ins().icmp(IntCC::Equal, a, z);
    b.ins().brif(
        same,
        equal,
        &[],
        order,
        &[args[4].into(), transported_index.into(), args[3].into()],
    );
    b.switch_to_block(equal);
    let i = if advanced {
        b.ins().iadd_imm(transported_index, 1)
    } else {
        transported_index
    };
    b.ins()
        .jump(order, &[args[4].into(), i.into(), args[3].into()]);
    b.switch_to_block(order);
    let state = b.block_params(order).to_vec();
    let (marker, a) = read(&mut b, pointer, state[1], args[1], state[0], write_inside);
    let (marker, z) = read(&mut b, pointer, state[2], args[1], marker, false);
    let a = b.ins().ishl_imm(a, 8);
    let value = b.ins().bor(a, z);
    let value = b.ins().bxor(value, marker);
    b.ins().return_(&[value]);
    b.seal_all_blocks();
    b.finalize();
    c.flowgraph();
    let reused = if optimize {
        super::fold(
            &mut c,
            &if admitted {
                BTreeSet::from([pointer])
            } else {
                BTreeSet::new()
            },
            &bounds,
            &BTreeMap::from([(0, (pointer, producer_length))]),
        )
    } else {
        0
    };
    let choices = c
        .func
        .layout
        .blocks()
        .flat_map(|block| c.func.layout.block_insts(block))
        .filter(|&inst| c.func.dfg.insts[inst].opcode() == Op::Select)
        .count();
    c.verify(module.isa()).unwrap();
    let id = module
        .declare_function("compared_bytes", Linkage::Local, &c.func.signature)
        .unwrap();
    module.define_function(id, &mut c).unwrap();
    module.finalize_definitions().unwrap();
    // SAFETY: This live finalized module owns the exact signature above.
    let call = unsafe { std::mem::transmute(module.get_finalized_function(id)) };
    (module, call, reused, choices)
}

#[test]
fn compared_bytes_keep_later_defaults_payloads_and_different_equal_edge_indices() {
    let mut observations = 0;
    for advanced in [false, true] {
        let (_old, old, _, _) = compile(false, false, false, advanced, true, 0);
        let (_new, new, reused, _) = compile(true, false, false, advanced, true, 0);
        assert!(reused > 0);
        for data in [
            [0u8; 8],
            [255; 8],
            [0, 1, 2, 3, 4, 5, 6, 7],
            [255, 0, 128, 1, 127, 254, 17, 99],
        ] {
            for i in 0..6 {
                for j in 0..6 {
                    for length in [0, 1, 2, 4, 8, u64::MAX] {
                        for marker in [0, 1, u64::MAX, 0x8000_0000_0000_0000] {
                            let mut a = data;
                            let mut z = data;
                            // SAFETY: Both live modules use this exact signature; every
                            // unconditional and optional advanced load fits these arrays.
                            let (x, y) = unsafe {
                                (
                                    old(a.as_mut_ptr(), length, i, j, marker),
                                    new(z.as_mut_ptr(), length, i, j, marker),
                                )
                            };
                            assert_eq!(x, y);
                            assert_eq!(a, z);
                            observations += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(observations, 6912);
}

#[test]
fn compared_bytes_decline_writes_and_absent_immutable_view_authority() {
    for (before, inside, admitted) in [
        (true, false, true),
        (false, true, true),
        (false, false, false),
    ] {
        let (_old, old, _, _) = compile(false, before, inside, false, admitted, 0);
        let (_new, new, reused, _) = compile(true, before, inside, false, admitted, 0);
        assert_eq!(reused, 0);
        for length in [0, 1, 8] {
            let mut a = [9, 1, 2, 3, 4, 5, 6, 7];
            let mut z = a;
            // SAFETY: Both modules remain live; all accesses fit the buffers.
            let (x, y) = unsafe {
                (
                    old(a.as_mut_ptr(), length, 0, 1, 42),
                    new(z.as_mut_ptr(), length, 0, 1, 42),
                )
            };
            assert_eq!(x, y);
            assert_eq!(a, z);
        }
    }
}

#[test]
fn compared_bytes_share_only_the_same_admitted_logical_bounds() {
    let mut observations = 0;
    for bounded in [1, 2] {
        for advanced in [false, true] {
            let (_old, old, _, _) = compile(false, false, false, advanced, true, bounded);
            let (_new, new, reused, choices) = compile(true, false, false, advanced, true, bounded);
            assert!(reused > 0);
            if bounded == 1 {
                assert_eq!(choices, 0);
            } else {
                assert!(choices > 0);
            }
            for data in [
                [0u8; 8],
                [255; 8],
                [0, 1, 2, 3, 4, 5, 6, 7],
                [255, 0, 128, 1, 127, 254, 17, 99],
            ] {
                for i in 0..6 {
                    for j in 0..6 {
                        for length in [0, 1, 2, 4, 8, u64::MAX] {
                            for marker in [0, 1, u64::MAX, 0x8000_0000_0000_0000] {
                                let mut a = data;
                                let mut z = data;
                                // SAFETY: Both exact entry signatures are owned by their live
                                // modules; every possible physical load fits these arrays.
                                let (x, y) = unsafe {
                                    (
                                        old(a.as_mut_ptr(), length, i, j, marker),
                                        new(z.as_mut_ptr(), length, i, j, marker),
                                    )
                                };
                                assert_eq!(x, y);
                                assert_eq!(a, z);
                                observations += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(observations, 13824);
}
