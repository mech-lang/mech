use super::fold;
use cranelift_codegen::Context;
use cranelift_codegen::ir::{AbiParam, InstBuilder, condcodes::IntCC, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

type Entry = unsafe extern "C" fn(u64, u64) -> u64;
const LIMIT: i64 = u32::MAX as i64;
const REJECTED: u64 = u64::MAX;

fn compile(
    module: &mut JITModule,
    name: &str,
    mut context: Context,
    expected: Option<usize>,
) -> Entry {
    context.verify(module.isa()).unwrap();
    if let Some(expected) = expected {
        context.compute_cfg();
        context.compute_domtree();
        assert_eq!(
            fold(&mut context.func, &context.cfg, &context.domtree),
            expected
        );
        context.compute_cfg();
        context.compute_domtree();
        context.verify(module.isa()).unwrap();
    }
    let id = module
        .declare_function(name, Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    // Both compiled fixtures have the two I64 argument and one I64 result ABI.
    unsafe { std::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) }
}

fn context(module: &JITModule) -> Context {
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 2];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    context
}

fn pack(module: &JITModule, wide_arm: bool) -> Context {
    let mut context = context(module);
    let mut state = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut state);
    let entry = b.create_block();
    let direct = b.create_block();
    let assemble = b.create_block();
    let join = b.create_block();
    let done = b.create_block();
    let bad = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.append_block_param(join, types::I64);
    b.switch_to_block(entry);
    let value = b.block_params(entry)[0];
    let choose = b.block_params(entry)[1];
    b.ins().brif(choose, direct, &[], assemble, &[]);
    b.switch_to_block(direct);
    let word = b.ins().ireduce(types::I32, value);
    let word = b.ins().uextend(types::I64, word);
    b.ins().jump(join, &[word.into()]);
    b.switch_to_block(assemble);
    let mut word = b.ins().iconst(types::I64, 0);
    for shift in [0, 8, 16, 24] {
        let byte = b.ins().ushr_imm(value, shift);
        let byte = b.ins().ireduce(types::I8, byte);
        let byte = b.ins().uextend(types::I64, byte);
        let count = b.ins().iconst(types::I64, shift);
        let count = b.ins().band_imm(count, 63);
        let shifted = b.ins().ishl(byte, count);
        word = b.ins().bor(word, shifted);
    }
    b.ins()
        .jump(join, &[if wide_arm { value } else { word }.into()]);
    b.switch_to_block(join);
    let word = b.block_params(join)[0];
    let valid = b
        .ins()
        .icmp_imm(IntCC::UnsignedLessThanOrEqual, word, LIMIT);
    b.ins().brif(valid, done, &[], bad, &[]);
    b.switch_to_block(done);
    b.ins().return_(&[word]);
    b.switch_to_block(bad);
    let error = b.ins().iconst(types::I64, REJECTED as i64);
    b.ins().return_(&[error]);
    b.seal_all_blocks();
    b.finalize();
    context
}

#[test]
fn a_packed_join_preserves_the_width_of_each_incoming_value() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut checked = 0;
    for wide_arm in [false, true] {
        let before = pack(&module, wide_arm);
        let after = pack(&module, wide_arm);
        let before = compile(
            &mut module,
            &format!("pack_before_{wide_arm}"),
            before,
            None,
        );
        let after = compile(
            &mut module,
            &format!("pack_after_{wide_arm}"),
            after,
            Some(usize::from(!wide_arm)),
        );
        for word in [
            0,
            1,
            u64::from(u32::MAX),
            1 << 32,
            0x1234_5678_ffee_ddcc,
            u64::MAX,
        ] {
            for choose in [0, 1, u64::MAX] {
                let expected = if wide_arm && choose == 0 && word > LIMIT as u64 {
                    REJECTED
                } else {
                    word & LIMIT as u64
                };
                assert_eq!(unsafe { before(word, choose) }, expected);
                assert_eq!(unsafe { after(word, choose) }, expected);
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 36);
}

fn carried(module: &JITModule, shift: bool) -> Context {
    let mut context = context(module);
    let mut state = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut state);
    let entry = b.create_block();
    let header = b.create_block();
    let branch = b.create_block();
    let step = b.create_block();
    let done = b.create_block();
    let bad = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.append_block_param(header, types::I64);
    b.append_block_param(header, types::I64);
    b.switch_to_block(entry);
    let raw = b.block_params(entry)[0];
    let visits = b.block_params(entry)[1];
    let raw = b.ins().ireduce(types::I32, raw);
    let initial = b.ins().uextend(types::I64, raw);
    b.ins().jump(header, &[initial.into(), visits.into()]);
    b.switch_to_block(header);
    let word = b.block_params(header)[0];
    let visits = b.block_params(header)[1];
    let invalid = b.ins().icmp_imm(IntCC::UnsignedGreaterThan, word, LIMIT);
    b.ins().brif(invalid, bad, &[], branch, &[]);
    b.switch_to_block(branch);
    b.ins().brif(visits, step, &[], done, &[]);
    b.switch_to_block(step);
    let next = if shift {
        b.ins().ishl_imm(word, 1)
    } else {
        b.ins().bor_imm(word, 1 << 31)
    };
    let visits = b.ins().iadd_imm(visits, -1);
    b.ins().jump(header, &[next.into(), visits.into()]);
    b.switch_to_block(done);
    b.ins().return_(&[word]);
    b.switch_to_block(bad);
    let error = b.ins().iconst(types::I64, REJECTED as i64);
    b.ins().return_(&[error]);
    b.seal_all_blocks();
    b.finalize();
    context
}

#[test]
fn a_loop_width_includes_the_new_values_produced_by_backedges() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut checked = 0;
    for shift in [false, true] {
        let before = carried(&module, shift);
        let after = carried(&module, shift);
        let before = compile(&mut module, &format!("loop_before_{shift}"), before, None);
        let after = compile(
            &mut module,
            &format!("loop_after_{shift}"),
            after,
            Some(usize::from(!shift)),
        );
        for raw in [0, 1, 0x8000_0000, 0x1_8000_0000, u64::MAX] {
            for visits in [0, 1, 2, 15, 31, 64] {
                let result = unsafe { before(raw, visits) };
                assert_eq!(unsafe { after(raw, visits) }, result);
                if shift && raw & 0x8000_0000 != 0 && visits != 0 {
                    assert_eq!(result, REJECTED);
                }
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 60);
}

#[derive(Clone, Copy, Debug)]
enum LengthCase {
    Proved,
    ShortFloor,
    ChangedLength,
    WideBackedge,
    Join,
}

const FLOOR_REJECTED: u64 = u64::MAX - 1;

fn current_length_loop(module: &JITModule, case: LengthCase) -> Context {
    let mut context = context(module);
    let mut state = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut state);
    let entry = b.create_block();
    let selected = b.create_block();
    let unselected = b.create_block();
    let header = b.create_block();
    let good = b.create_block();
    let bad = b.create_block();
    let step = b.create_block();
    let done = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.append_block_param(header, types::I64);
    b.append_block_param(header, types::I64);
    b.switch_to_block(entry);
    let raw = b.block_params(entry)[0];
    let length = b.block_params(entry)[1];
    let floor = if matches!(case, LengthCase::ShortFloor) {
        256
    } else {
        1024
    };
    let enough = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, length, floor);
    let enough = b.ins().uextend(types::I64, enough);
    let two = b.ins().iconst(types::I64, 2);
    let one = b.ins().iconst(types::I64, 1);
    let masked = b.ins().band_imm(raw, 255);
    let doubled = b.ins().imul(masked, two);
    let initial = b.ins().iadd(doubled, one);
    b.ins().brif(enough, selected, &[], unselected, &[]);
    b.switch_to_block(selected);
    b.ins().jump(header, &[initial.into(), two.into()]);
    b.switch_to_block(unselected);
    if matches!(case, LengthCase::Join) {
        b.ins().jump(header, &[initial.into(), two.into()]);
    } else {
        let rejected = b.ins().iconst(types::I64, FLOOR_REJECTED as i64);
        b.ins().return_(&[rejected]);
    }
    b.switch_to_block(header);
    let index = b.block_params(header)[0];
    let visits = b.block_params(header)[1];
    let one_based = b.ins().iadd(index, one);
    let ordinal = b.ins().iadd_imm(one_based, -1);
    let current = if matches!(case, LengthCase::ChangedLength) {
        b.ins().ushr_imm(length, 2)
    } else {
        length
    };
    let invalid = b
        .ins()
        .icmp(IntCC::UnsignedGreaterThanOrEqual, ordinal, current);
    b.ins().brif(invalid, bad, &[], good, &[]);
    b.switch_to_block(good);
    b.ins().brif(visits, step, &[], done, &[]);
    b.switch_to_block(step);
    let next = if matches!(case, LengthCase::WideBackedge) {
        raw
    } else {
        index
    };
    let visits = b.ins().iadd_imm(visits, -1);
    b.ins().jump(header, &[next.into(), visits.into()]);
    b.switch_to_block(done);
    b.ins().return_(&[ordinal]);
    b.switch_to_block(bad);
    let rejected = b.ins().iconst(types::I64, REJECTED as i64);
    b.ins().return_(&[rejected]);
    b.seal_all_blocks();
    b.finalize();
    context
}

#[test]
fn length_proofs_follow_the_current_selected_edge_and_every_loop_input() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut checked = 0;
    for case in [
        LengthCase::Proved,
        LengthCase::ShortFloor,
        LengthCase::ChangedLength,
        LengthCase::WideBackedge,
        LengthCase::Join,
    ] {
        let original = current_length_loop(&module, case);
        let optimized = current_length_loop(&module, case);
        let original = compile(
            &mut module,
            &format!("length_original_{case:?}"),
            original,
            None,
        );
        let optimized = compile(
            &mut module,
            &format!("length_optimized_{case:?}"),
            optimized,
            Some(usize::from(matches!(case, LengthCase::Proved))),
        );
        for raw in [0u64, 1, 127, 128, 255, 256, 1023, 1024, 1025, u64::MAX] {
            for length in [0, 1, 255, 256, 511, 512, 1023, 1024, 1025, u64::MAX] {
                let initial = (raw & 255) * 2 + 1;
                let last = if matches!(case, LengthCase::WideBackedge) {
                    raw
                } else {
                    initial
                };
                let floor = if matches!(case, LengthCase::ShortFloor) {
                    256
                } else {
                    1024
                };
                let current = if matches!(case, LengthCase::ChangedLength) {
                    length >> 2
                } else {
                    length
                };
                let expected = if length < floor && !matches!(case, LengthCase::Join) {
                    FLOOR_REJECTED
                } else if initial >= current || last >= current {
                    REJECTED
                } else {
                    last
                };
                assert_eq!(
                    unsafe { original(raw, length) },
                    expected,
                    "{case:?} {raw} {length}"
                );
                assert_eq!(
                    unsafe { optimized(raw, length) },
                    expected,
                    "{case:?} {raw} {length}"
                );
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 500);
}

#[test]
fn default_pipeline_proves_join_widths_and_ablation_retains_the_guard() {
    use crate::native_artifact::codegen::options::{
        CompilationControls, OptimizationOptions, OptimizationPass,
    };
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    for enabled in [false, true] {
        let mut context = pack(&module, false);
        let options = if enabled {
            OptimizationOptions::default()
        } else {
            OptimizationOptions::without(OptimizationPass::ProvedWordWidths)
        };
        let controls = CompilationControls::new(options);
        let report = super::super::optimize(&controls, &mut context, module.isa()).unwrap();
        assert_eq!(report.known_width_branches_removed, usize::from(enabled));
        let entry = compile(
            &mut module,
            &format!("pipeline_widths_{enabled}"),
            context,
            None,
        );
        for word in [0, 1, u64::from(u32::MAX), 1 << 32, u64::MAX] {
            for choose in [0, 1, u64::MAX] {
                assert_eq!(unsafe { entry(word, choose) }, word & LIMIT as u64);
            }
        }
    }
}

#[test]
fn zero_extending_loads_keep_exact_width_facts_but_signed_and_full_words_do_not() {
    use cranelift_codegen::ir::MemFlags;
    for width in [1u8, 2, 4, 8] {
        for signed in [false, true] {
            let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
            let mut context = context(&module);
            let mut state = FunctionBuilderContext::new();
            {
                let mut b = FunctionBuilder::new(&mut context.func, &mut state);
                let entry = b.create_block();
                let good = b.create_block();
                let bad = b.create_block();
                b.append_block_params_for_function_params(entry);
                b.switch_to_block(entry);
                let pointer = b.block_params(entry)[0];
                let ty = match width {
                    1 => types::I8,
                    2 => types::I16,
                    4 => types::I32,
                    8 => types::I64,
                    _ => unreachable!(),
                };
                let value = if signed && width < 8 {
                    let value = b.ins().load(ty, MemFlags::new(), pointer, 0);
                    b.ins().sextend(types::I64, value)
                } else {
                    crate::native_artifact::program::codegen::numeric_words::load_unsigned_word(
                        &mut b,
                        ty,
                        MemFlags::new(),
                        pointer,
                        0,
                    )
                };
                let maximum = if width < 8 {
                    u64::MAX >> (64 - width as u32 * 8)
                } else {
                    u32::MAX as u64
                };
                let valid = b
                    .ins()
                    .icmp_imm(IntCC::UnsignedLessThanOrEqual, value, maximum as i64);
                b.ins().brif(valid, good, &[], bad, &[]);
                b.switch_to_block(good);
                b.ins().return_(&[value]);
                b.switch_to_block(bad);
                let rejected = b.ins().iconst(types::I64, REJECTED as i64);
                b.ins().return_(&[rejected]);
                b.seal_all_blocks();
                b.finalize();
            }
            let entry = compile(
                &mut module,
                &format!("packed_{width}_{signed}"),
                context,
                Some(usize::from(!signed && width < 8)),
            );
            let bytes = [0xffu8; 8];
            // SAFETY: all width-byte cells are initialized for this owned call.
            let observed = unsafe { entry(bytes.as_ptr() as u64, 0) };
            let expected = if signed || width == 8 {
                REJECTED
            } else {
                u64::MAX >> (64 - width as u32 * 8)
            };
            assert_eq!(observed, expected);
        }
    }
}
