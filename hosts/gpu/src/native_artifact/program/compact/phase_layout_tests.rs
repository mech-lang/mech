use cranelift_codegen::Context;
use cranelift_codegen::control::ControlPlane;
use cranelift_codegen::ir::{AbiParam, ConstantData, InstBuilder, MemFlags, TrapCode, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{DataDescription, DataId, Linkage, Module, default_libcall_names};
use std::collections::BTreeSet;

const MASKS: [[u32; 4]; 2] = [
    [0x0123_4567, 0x89ab_cdef, 0xdead_beef, 0x1020_3040],
    [0xfedc_ba98, 0x7654_3210, 0x5566_7788, 0xa1b2_c3d4],
];

fn leaf(module: &mut JITModule, data: [DataId; 2]) -> Context {
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 3];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let globals = data.map(|id| module.declare_data_in_func(id, &mut context.func));
    let mut fb = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
    let entry = b.create_block();
    let select = b.create_block();
    let left = b.create_block();
    let right = b.create_block();
    let error = b.create_block();
    b.set_cold_block(error);
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let [mode, input, output] = b.block_params(entry)[..] else {
        unreachable!()
    };
    let bad = b
        .ins()
        .icmp_imm(cranelift_codegen::ir::condcodes::IntCC::Equal, mode, -1);
    b.ins().brif(bad, error, &[], select, &[]);
    b.switch_to_block(select);
    let bit = b.ins().band_imm(mode, 1);
    b.ins().brif(bit, right, &[], left, &[]);
    for (index, block) in [left, right].into_iter().enumerate() {
        b.switch_to_block(block);
        let bytes = MASKS[index]
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect::<Vec<_>>();
        let constant = b.func.dfg.constants.insert(ConstantData::from(bytes));
        let mask = b.ins().vconst(types::I32X4, constant);
        let value = b.ins().load(types::I32X4, MemFlags::new(), input, 0);
        let value = b.ins().bxor(value, mask);
        b.ins().store(MemFlags::new(), value, output, 0);
        let address = b.ins().global_value(types::I64, globals[index]);
        let value = b.ins().load(types::I64, MemFlags::new(), address, 0);
        let value = b.ins().iadd(value, mode);
        b.ins().return_(&[value]);
    }
    b.switch_to_block(error);
    b.ins().trap(TrapCode::user(1).unwrap());
    b.seal_all_blocks();
    b.finalize();
    context
}

fn target(word: u32, index: usize, shift: u32, bits: u32) -> usize {
    let extend = 32 - bits;
    let offset = (((word >> shift) << extend) as i32) >> extend;
    index.checked_add_signed(offset as isize).unwrap()
}

fn profile(bytes: &[u8]) -> (Vec<u32>, u32) {
    let words = bytes
        .chunks_exact(4)
        .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
        .collect::<Vec<_>>();
    let literal = words
        .iter()
        .enumerate()
        .filter_map(|(at, &word)| {
            (word & 0x3b00_0000 == 0x1800_0000).then(|| target(word, at, 5, 19))
        })
        .min()
        .expect("native vector constants use a trailing literal pool");
    assert_eq!(literal % 4, 0);
    let mut end = literal;
    while words[end - 1] == 0xd503_201f {
        end -= 1;
    }
    let mut heads = BTreeSet::from([0]);
    for (at, &word) in words[..end].iter().enumerate() {
        let field = if word & 0xfc00_0000 == 0x1400_0000 {
            Some((0, 26))
        } else if word & 0xff00_0010 == 0x5400_0000 || word & 0x7e00_0000 == 0x3400_0000 {
            Some((5, 19))
        } else if word & 0x7e00_0000 == 0x3600_0000 {
            Some((5, 14))
        } else {
            None
        };
        if let Some((shift, bits)) = field {
            let to = target(word, at, shift, bits);
            assert!(to < end);
            heads.insert(to);
        }
        let terminal = word & 0xffff_fc1f == 0xd65f_0000
            || matches!(word, 0xd65f_0bff | 0xd65f_0fff)
            || word & 0xffe0_001f == 0xd420_0000
            || word & 0xffff_0000 == 0;
        if (field.is_some() || terminal) && at + 1 < end {
            heads.insert(at + 1);
        }
    }
    let order = std::iter::once(0)
        .chain(heads.into_iter().filter(|&v| v != 0).rev())
        .map(|v| (v * 4) as u32)
        .collect();
    (order, (literal * 4) as u32)
}

#[test]
fn measured_leaf_layout_preserves_literals_external_targets_traps_and_native_results() {
    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[
                ("opt_level", "speed"),
                ("enable_verifier", "true"),
                ("preserve_frame_pointers", "true"),
                ("unwind_info", "true"),
            ],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let values = [0x0123_4567_89ab_cdefu64, 0xfedc_ba98_7654_3210];
    let data = std::array::from_fn(|i| {
        let id = module
            .declare_data(
                &format!("layout_constant_{i}"),
                Linkage::Local,
                false,
                false,
            )
            .unwrap();
        let mut desc = DataDescription::new();
        desc.define(values[i].to_le_bytes().to_vec().into_boxed_slice());
        module.define_data(id, &desc).unwrap();
        id
    });
    let mut original = leaf(&mut module, data);
    original
        .compile(module.isa(), &mut ControlPlane::default())
        .unwrap();
    let before = &original.compiled_code().unwrap().buffer;
    assert!(before.relocs().len() >= 4);
    assert!(!before.traps().is_empty());
    assert!(!before.unwind_info.is_empty());
    let (order, literal) = profile(before.data());
    let mut changed = leaf(&mut module, data);
    changed.func.aarch64_block_layout = order.clone();
    changed.func.aarch64_block_layout_literal_start = Some(literal);
    let id = module
        .declare_function(
            "measured_literal_leaf",
            Linkage::Local,
            &changed.func.signature,
        )
        .unwrap();
    let defined = module.define_function(id, &mut changed);
    assert!(
        defined.is_ok(),
        "{defined:?}; literal {literal}; order {order:?}; relocations {:?}; unwind {:?}; words {:x?}",
        before.relocs(),
        before.unwind_info,
        before
            .data()
            .chunks_exact(4)
            .map(|x| u32::from_le_bytes(x.try_into().unwrap()))
            .collect::<Vec<_>>()
    );
    let after = &changed.compiled_code().unwrap().buffer;
    assert_ne!(before.data(), after.data());
    assert_eq!(before.relocs().len(), after.relocs().len());
    assert_eq!(before.unwind_info, after.unwind_info);
    for old in before.relocs() {
        let moved = after
            .relocs()
            .iter()
            .find(|r| r.kind == old.kind && r.target == old.target && r.addend == old.addend)
            .unwrap();
        assert_eq!(
            &before.data()[old.offset as usize..old.offset as usize + 4],
            &after.data()[moved.offset as usize..moved.offset as usize + 4]
        );
    }
    assert_eq!(before.traps().len(), after.traps().len());
    let mut unmatched = after.traps().iter().collect::<Vec<_>>();
    for old in before.traps() {
        let index = unmatched
            .iter()
            .position(|moved| {
                old.code == moved.code
                    && before.data()[old.offset as usize..old.offset as usize + 4]
                        == after.data()[moved.offset as usize..moved.offset as usize + 4]
            })
            .unwrap();
        unmatched.remove(index);
    }
    assert!(unmatched.is_empty());
    for bad_literal in [None, Some(1), Some(literal + 16)] {
        let mut bad = leaf(&mut module, data);
        bad.func.aarch64_block_layout = order.clone();
        bad.func.aarch64_block_layout_literal_start = bad_literal;
        assert!(
            bad.compile(module.isa(), &mut ControlPlane::default())
                .is_err()
        );
    }
    for duplicate in [false, true] {
        let mut bad = leaf(&mut module, data);
        bad.func.aarch64_block_layout = order.clone();
        bad.func.aarch64_block_layout_literal_start = Some(literal);
        if duplicate {
            bad.func.aarch64_block_layout[1] = 0;
        } else {
            bad.func.aarch64_block_layout.pop();
        }
        assert!(
            bad.compile(module.isa(), &mut ControlPlane::default())
                .is_err()
        );
    }
    module.finalize_definitions().unwrap();
    type Call = unsafe extern "C" fn(u64, *const u32, *mut u32) -> u64;
    // SAFETY: the live module owns this exact three-argument native signature.
    let call: Call = unsafe { std::mem::transmute(module.get_finalized_function(id)) };
    for mode in 0..128u64 {
        let input = std::array::from_fn::<_, 4, _>(|i| {
            (mode as u32)
                .wrapping_mul(0x8765_4321)
                .rotate_left(i as u32 * 7)
        });
        let prior = input;
        let mut output = [0xa5a5_a5a5; 6];
        // SAFETY: the complete four-word vectors are initialized and disjoint.
        let result = unsafe { call(mode, input.as_ptr(), output.as_mut_ptr().add(1)) };
        let which = mode as usize & 1;
        assert_eq!(result, values[which].wrapping_add(mode));
        assert_eq!(input, prior);
        assert_eq!(output[0], 0xa5a5_a5a5);
        assert_eq!(output[5], 0xa5a5_a5a5);
        for i in 0..4 {
            assert_eq!(output[i + 1], input[i] ^ MASKS[which][i]);
        }
    }
}

#[test]
fn measured_leaf_layout_preserves_position_independent_external_relocations() {
    use cranelift_codegen::binemit::Reloc;
    use cranelift_codegen::ir::GlobalValueData;
    use cranelift_codegen::settings::{self, Configurable};

    // The executable test above uses near data references. Object emission also
    // uses a GOT pair, which must retain both its target and linker-owned bits.
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let data = std::array::from_fn(|i| {
        module
            .declare_data(&format!("layout_import_{i}"), Linkage::Import, false, false)
            .unwrap()
    });
    let mut flags = settings::builder();
    for (name, value) in [
        ("opt_level", "speed"),
        ("is_pic", "true"),
        ("enable_verifier", "true"),
        ("preserve_frame_pointers", "true"),
        ("unwind_info", "true"),
    ] {
        flags.set(name, value).unwrap();
    }
    let isa = cranelift_codegen::isa::lookup(module.isa().triple().clone())
        .unwrap()
        .finish(settings::Flags::new(flags))
        .unwrap();
    let mut make_leaf = || {
        let mut context = leaf(&mut module, data);
        for value in context.func.global_values.values_mut() {
            if let GlobalValueData::Symbol { colocated, .. } = value {
                *colocated = false;
            }
        }
        context
    };
    let mut original = make_leaf();
    original
        .compile(isa.as_ref(), &mut ControlPlane::default())
        .unwrap();
    let before = &original.compiled_code().unwrap().buffer;
    assert_eq!(before.relocs().len(), 4);
    for kind in [Reloc::Aarch64AdrGotPage21, Reloc::Aarch64Ld64GotLo12Nc] {
        assert_eq!(before.relocs().iter().filter(|r| r.kind == kind).count(), 2);
    }
    let (order, literal) = profile(before.data());
    let mut changed = make_leaf();
    changed.func.aarch64_block_layout = order;
    changed.func.aarch64_block_layout_literal_start = Some(literal);
    changed
        .compile(isa.as_ref(), &mut ControlPlane::default())
        .unwrap();
    let after = &changed.compiled_code().unwrap().buffer;
    assert_ne!(before.data(), after.data());
    assert_eq!(before.unwind_info, after.unwind_info);
    let mut unmatched = after.relocs().iter().collect::<Vec<_>>();
    for old in before.relocs() {
        let index = unmatched
            .iter()
            .position(|moved| {
                old.kind == moved.kind
                    && old.target == moved.target
                    && old.addend == moved.addend
                    && before.data()[old.offset as usize..old.offset as usize + 4]
                        == after.data()[moved.offset as usize..moved.offset as usize + 4]
            })
            .unwrap();
        unmatched.remove(index);
    }
    assert!(unmatched.is_empty());
}
