use super::*;
use cranelift_codegen::Context;
use cranelift_codegen::ir::{AbiParam, MemFlags};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[derive(Clone, Copy, Debug)]
enum Mode {
    Plain,
    Copies,
    ColdConsumer,
    LowBit,
    Contradiction,
    DifferentBase,
    PlusTwo,
    Signed,
    NonStrict,
    Bypass,
    DuplicateEdges,
    RawMask,
    StaleLoopSnapshot,
    DeepCopies,
    OpaqueLoad,
    OpaqueCall,
    IndexFailure,
    BadBranch,
    ReversedSuccess,
    ReversedBad,
    SignedGuard,
    UnboundedTrue,
    DroppedMaskBound,
    Permuted,
}
const MODES: [Mode; 24] = [
    Mode::Plain,
    Mode::Copies,
    Mode::ColdConsumer,
    Mode::LowBit,
    Mode::Contradiction,
    Mode::DifferentBase,
    Mode::PlusTwo,
    Mode::Signed,
    Mode::NonStrict,
    Mode::Bypass,
    Mode::DuplicateEdges,
    Mode::RawMask,
    Mode::StaleLoopSnapshot,
    Mode::DeepCopies,
    Mode::OpaqueLoad,
    Mode::OpaqueCall,
    Mode::IndexFailure,
    Mode::BadBranch,
    Mode::ReversedSuccess,
    Mode::ReversedBad,
    Mode::SignedGuard,
    Mode::UnboundedTrue,
    Mode::DroppedMaskBound,
    Mode::Permuted,
];
fn expected_folds(mode: Mode) -> usize {
    usize::from(matches!(
        mode,
        Mode::Plain
            | Mode::Copies
            | Mode::ColdConsumer
            | Mode::LowBit
            | Mode::Contradiction
            | Mode::IndexFailure
            | Mode::BadBranch
            | Mode::ReversedSuccess
            | Mode::ReversedBad
    ))
}
unsafe extern "C" fn opaque_load_and_touch(memory: *mut u64) -> u64 {
    // SAFETY: the native fixture passes its live five-word observation array.
    unsafe {
        *memory.add(1) = 0x1234;
        *memory.add(4)
    }
}
fn function(mode: Mode) -> Context {
    let mut c = Context::new();
    c.func.signature.params = vec![AbiParam::new(types::I64); 4];
    c.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fc = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut c.func, &mut fc);
        let entry = b.create_block();
        let test = b.create_block();
        let body = b.create_block();
        let outside = b.create_block();
        let left = b.create_block();
        let right = b.create_block();
        let copied = b.create_block();
        let join = b.create_block();
        let add = b.create_block();
        let skip = b.create_block();
        let success = b.create_block();
        let overflow = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.append_block_param(copied, types::I64);
        if matches!(mode, Mode::Permuted) {
            b.append_block_param(copied, types::I64);
        }
        b.append_block_param(join, types::I64);
        b.append_block_param(join, types::I8);
        b.switch_to_block(entry);
        let x = b.block_params(entry)[0];
        let end = b.block_params(entry)[1];
        let switch = b.block_params(entry)[2];
        let memory = b.block_params(entry)[3];
        let zero = b.ins().iconst(types::I64, 0);
        b.ins().store(MemFlags::new(), zero, memory, 0);
        if matches!(mode, Mode::Bypass) {
            let one = b.ins().iconst(types::I8, 1);
            b.ins()
                .brif(switch, join, &[x.into(), one.into()], test, &[]);
        } else {
            b.ins().jump(test, &[]);
        }
        b.switch_to_block(test);
        let cc = match mode {
            Mode::Signed => IntCC::SignedLessThan,
            Mode::NonStrict => IntCC::UnsignedLessThanOrEqual,
            _ => IntCC::UnsignedLessThan,
        };
        let below = b.ins().icmp(cc, x, end);
        let below_test = if matches!(mode, Mode::LowBit | Mode::DroppedMaskBound) {
            b.ins().band_imm(
                below,
                if matches!(mode, Mode::DroppedMaskBound) {
                    2
                } else {
                    1
                },
            )
        } else {
            below
        };
        b.ins().brif(below_test, body, &[], outside, &[]);
        b.switch_to_block(outside);
        let flag = b.ins().iconst(
            types::I8,
            match mode {
                Mode::RawMask => 3,
                Mode::UnboundedTrue | Mode::DroppedMaskBound => 1,
                _ => 0,
            },
        );
        b.ins().jump(join, &[x.into(), flag.into()]);
        b.switch_to_block(body);
        if matches!(mode, Mode::IndexFailure) {
            let admitted = b.create_block();
            let failure = b.create_block();
            let good = b.ins().icmp(IntCC::UnsignedLessThan, x, switch);
            b.ins().brif(good, admitted, &[], failure, &[]);
            b.switch_to_block(failure);
            let site = b.ins().iconst(types::I64, 1708);
            b.ins().store(MemFlags::new(), site, memory, 0);
            let failed = b.ins().iconst(types::I64, -7);
            b.ins().return_(&[failed]);
            b.switch_to_block(admitted);
        }
        if matches!(mode, Mode::DuplicateEdges) {
            let one = b.ins().iconst(types::I8, 1);
            let changed = b.ins().iconst(types::I64, -1);
            b.ins().brif(
                switch,
                join,
                &[x.into(), one.into()],
                join,
                &[changed.into(), one.into()],
            );
        } else if matches!(mode, Mode::StaleLoopSnapshot) {
            // The saved x<end is true for the entry x, not a later loop phi.
            let header = b.create_block();
            let mutate = b.create_block();
            let done = b.create_block();
            b.append_block_param(header, types::I64);
            b.append_block_param(header, types::I8);
            b.append_block_param(header, types::I64);
            b.ins().jump(header, &[x.into(), below.into(), zero.into()]);
            b.switch_to_block(header);
            let current = b.block_params(header)[0];
            let saved = b.block_params(header)[1];
            let visit = b.block_params(header)[2];
            b.ins().brif(visit, done, &[], mutate, &[]);
            b.switch_to_block(mutate);
            let changed = b.ins().iconst(types::I64, -1);
            let one = b.ins().iconst(types::I64, 1);
            b.ins()
                .jump(header, &[changed.into(), saved.into(), one.into()]);
            b.switch_to_block(done);
            b.ins().jump(join, &[current.into(), saved.into()]);
        } else if matches!(mode, Mode::OpaqueCall) {
            let mut signature = cranelift_codegen::ir::Signature::new(b.func.signature.call_conv);
            signature.params.push(AbiParam::new(types::I64));
            signature.returns.push(AbiParam::new(types::I64));
            let sig = b.import_signature(signature);
            let address = b.ins().iconst(
                types::I64,
                opaque_load_and_touch as *const () as usize as i64,
            );
            let called = b.ins().call_indirect(sig, address, &[memory]);
            let loaded = b.inst_results(called)[0];
            let one = b.ins().iconst(types::I8, 1);
            b.ins().jump(join, &[loaded.into(), one.into()]);
        } else if matches!(mode, Mode::OpaqueLoad) {
            // This unrelated memory may be changed by the caller; it supplies
            // no identity or range fact even when an earlier x was checked.
            let loaded = b.ins().load(types::I64, MemFlags::new(), memory, 32);
            let one = b.ins().iconst(types::I8, 1);
            b.ins().jump(join, &[loaded.into(), one.into()]);
        } else {
            b.ins().brif(switch, left, &[], right, &[]);
        }
        b.switch_to_block(left);
        if matches!(mode, Mode::Permuted) {
            b.ins().jump(copied, &[x.into(), end.into()]);
        } else {
            b.ins().jump(copied, &[x.into()]);
        }
        b.switch_to_block(right);
        let other = if matches!(mode, Mode::DifferentBase) {
            b.ins().iconst(types::I64, -1)
        } else {
            x
        };
        if matches!(mode, Mode::Permuted) {
            b.ins().jump(copied, &[end.into(), x.into()]);
        } else {
            b.ins().jump(copied, &[other.into()]);
        }
        b.switch_to_block(copied);
        let mut current = b.block_params(copied)[0];
        if matches!(mode, Mode::DeepCopies) {
            for _ in 0..80 {
                let next = b.create_block();
                b.append_block_param(next, types::I64);
                b.ins().jump(next, &[current.into()]);
                b.switch_to_block(next);
                current = b.block_params(next)[0];
            }
        }
        let one = b.ins().iconst(
            types::I8,
            if matches!(mode, Mode::Contradiction) {
                0
            } else {
                1
            },
        );
        b.ins().jump(join, &[current.into(), one.into()]);
        b.switch_to_block(join);
        let base = b.block_params(join)[0];
        let flag = b.block_params(join)[1];
        let branch = if matches!(mode, Mode::LowBit | Mode::RawMask) {
            b.ins().band_imm(flag, 1)
        } else {
            flag
        };
        if matches!(mode, Mode::ColdConsumer) {
            let word = b.ins().uextend(types::I64, flag);
            b.ins().store(MemFlags::new(), word, memory, 8);
        }
        b.ins().brif(branch, add, &[], skip, &[]);
        b.switch_to_block(add);
        let next = b
            .ins()
            .iadd_imm(base, if matches!(mode, Mode::PlusTwo) { 2 } else { 1 });
        let good = match mode {
            Mode::BadBranch => b.ins().icmp(IntCC::UnsignedLessThan, next, base),
            Mode::ReversedSuccess => b.ins().icmp(IntCC::UnsignedLessThanOrEqual, base, next),
            Mode::ReversedBad => b.ins().icmp(IntCC::UnsignedGreaterThan, base, next),
            Mode::SignedGuard => b.ins().icmp(IntCC::SignedGreaterThanOrEqual, next, base),
            _ => b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, next, base),
        };
        // Keep another real consumer of the comparison and the original add.
        let word = b.ins().uextend(types::I64, good);
        b.ins().store(MemFlags::new(), word, memory, 16);
        let tested = if matches!(mode, Mode::LowBit) {
            b.ins().band_imm(word, 1)
        } else {
            good
        };
        if matches!(mode, Mode::BadBranch | Mode::ReversedBad) {
            b.ins().brif(tested, overflow, &[], success, &[]);
        } else {
            b.ins().brif(tested, success, &[], overflow, &[]);
        }
        b.switch_to_block(success);
        let debit = b.ins().iconst(types::I64, 0x2e1ae);
        b.ins().store(MemFlags::new(), debit, memory, 24);
        b.ins().return_(&[next]);
        b.switch_to_block(overflow);
        let origin = b.ins().iconst(types::I64, 1838);
        b.ins().store(MemFlags::new(), origin, memory, 0);
        let failed = b.ins().iconst(types::I64, -2);
        b.ins().return_(&[failed]);
        b.switch_to_block(skip);
        let skipped = b.ins().iconst(types::I64, -1);
        b.ins().return_(&[skipped]);
        b.seal_all_blocks();
        b.finalize();
    }
    c
}
fn apply(c: &mut Context, steps: usize) -> usize {
    c.compute_cfg();
    c.compute_domtree();
    let changed = optimize_bounded(&mut c.func, &c.cfg, &c.domtree, steps);
    c.compute_cfg();
    c.compute_domtree();
    changed
}
#[test]
fn exact_selected_phi_transport_is_bounded_and_keeps_hard_negative_guards() {
    for mode in MODES {
        let mut c = function(mode);
        assert_eq!(apply(&mut c, MAX_STEPS), expected_folds(mode), "{mode:?}");
        let mut zero_budget = function(mode);
        assert_eq!(apply(&mut zero_budget, 0), 0, "{mode:?}");
    }
}
#[test]
fn native_extremes_saved_predicates_cold_users_and_errors_match_original() {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(
            &[("opt_level", "speed"), ("preserve_frame_pointers", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let mut pairs = Vec::new();
    for (n, mode) in MODES.into_iter().enumerate() {
        let mut ids = Vec::new();
        for enabled in [false, true] {
            let mut c = function(mode);
            if enabled {
                assert_eq!(apply(&mut c, MAX_STEPS), expected_folds(mode), "{mode:?}");
            }
            c.verify(module.isa()).unwrap();
            let id = module
                .declare_function(
                    &format!("selected_increment_{n}_{enabled}"),
                    Linkage::Local,
                    &c.func.signature,
                )
                .unwrap();
            module.define_function(id, &mut c).unwrap();
            ids.push(id);
        }
        pairs.push((mode, ids));
    }
    module.finalize_definitions().unwrap();
    let values = [
        0,
        1,
        2,
        63,
        64,
        (1u64 << 63) - 1,
        1u64 << 63,
        u64::MAX - 2,
        u64::MAX - 1,
        u64::MAX,
    ];
    for (mode, ids) in pairs {
        // SAFETY: these live JIT definitions use the declared four-I64 ABI.
        let a: unsafe extern "C" fn(u64, u64, u64, *mut u64) -> u64 =
            unsafe { core::mem::transmute(module.get_finalized_function(ids[0])) };
        let b: unsafe extern "C" fn(u64, u64, u64, *mut u64) -> u64 =
            unsafe { core::mem::transmute(module.get_finalized_function(ids[1])) };
        for x in values {
            for end in values {
                for switch in [0, 1, 3, u64::MAX] {
                    for loaded in [0, u64::MAX] {
                        let mut before = [0xabcdu64; 5];
                        before[4] = loaded;
                        let mut after = before;
                        let got_before = unsafe { a(x, end, switch, before.as_mut_ptr()) };
                        let got_after = unsafe { b(x, end, switch, after.as_mut_ptr()) };
                        assert_eq!(
                            (got_before, before),
                            (got_after, after),
                            "{mode:?} x={x} end={end} switch={switch} loaded={loaded}"
                        );
                    }
                }
            }
        }
    }
}

fn width_function(ty: cranelift_codegen::ir::Type) -> Context {
    let mut c = Context::new();
    c.func.signature.params = vec![AbiParam::new(types::I64); 2];
    c.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fc = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut c.func, &mut fc);
        let entry = b.create_block();
        let body = b.create_block();
        let outside = b.create_block();
        let success = b.create_block();
        let overflow = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let x = b.block_params(entry)[0];
        let end = b.block_params(entry)[1];
        let (x, end) = if ty == types::I64 {
            (x, end)
        } else {
            (b.ins().ireduce(ty, x), b.ins().ireduce(ty, end))
        };
        let below = b.ins().icmp(IntCC::UnsignedLessThan, x, end);
        b.ins().brif(below, body, &[], outside, &[]);
        b.switch_to_block(body);
        let next = b.ins().iadd_imm(x, 1);
        let good = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, next, x);
        b.ins().brif(good, success, &[], overflow, &[]);
        b.switch_to_block(success);
        let result = if ty == types::I64 {
            next
        } else {
            b.ins().uextend(types::I64, next)
        };
        b.ins().return_(&[result]);
        b.switch_to_block(overflow);
        let bad = b.ins().iconst(types::I64, -2);
        b.ins().return_(&[bad]);
        b.switch_to_block(outside);
        let skip = b.ins().iconst(types::I64, -1);
        b.ins().return_(&[skip]);
        b.seal_all_blocks();
        b.finalize();
    }
    c
}
#[test]
fn native_width_boundaries_do_not_inherit_i64_proofs() {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(
            &[("opt_level", "speed"), ("preserve_frame_pointers", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let mut entries = Vec::new();
    for ty in [types::I8, types::I16, types::I32, types::I64] {
        let mut c = width_function(ty);
        assert_eq!(apply(&mut c, MAX_STEPS), usize::from(ty == types::I64));
        c.verify(module.isa()).unwrap();
        let id = module
            .declare_function(
                &format!("selected_increment_width_{ty}"),
                Linkage::Local,
                &c.func.signature,
            )
            .unwrap();
        module.define_function(id, &mut c).unwrap();
        entries.push((ty, id));
    }
    module.finalize_definitions().unwrap();
    for (ty, id) in entries {
        // SAFETY: live JIT function has exactly two native I64 arguments/results.
        let run: unsafe extern "C" fn(u64, u64) -> u64 =
            unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        let max = u64::MAX >> (64 - ty.bits());
        let inputs = [0, 1, 2, max / 2, max / 2 + 1, max - 1, max, u64::MAX];
        for x in inputs {
            for end in inputs {
                let (a, b) = (x & max, end & max);
                let expected = if a < b { a + 1 } else { u64::MAX };
                assert_eq!(unsafe { run(x, end) }, expected, "{ty} x={x} end={end}");
            }
        }
    }
}
