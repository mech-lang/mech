#![cfg(target_arch = "aarch64")]
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, condcodes::IntCC};
use cranelift_codegen::ir::{Function, Opcode, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};

#[derive(Clone, Copy, Debug)]
enum Placement {
    Dominator,
    Sibling,
    Later,
    DifferentBase,
    Narrow,
    Plain,
    CrowdedSibling,
}

fn effects(f: &Function) -> Vec<Opcode> {
    f.layout
        .blocks()
        .flat_map(|block| f.layout.block_insts(block))
        .filter(|&inst| {
            matches!(
                f.dfg.insts[inst].opcode(),
                Opcode::PrefetchWrite
                    | Opcode::Load
                    | Opcode::Uload32
                    | Opcode::Store
                    | Opcode::Return
            )
        })
        .map(|inst| match f.dfg.insts[inst].opcode() {
            Opcode::Uload32 => Opcode::Load,
            op => op,
        })
        .collect()
}

fn compile(
    module: &mut JITModule,
    placement: Placement,
    bias: i64,
    prefetch: bool,
    optimized: bool,
    name: &str,
) -> FuncId {
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 3];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut state = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut state);
    let entry = b.create_block();
    let work = b.create_block();
    let body = b.create_block();
    let failed = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let [base, index, limit] = b.block_params(entry).try_into().unwrap();
    match placement {
        Placement::Dominator => {
            b.ins().iadd_imm(base, bias);
        }
        Placement::DifferentBase => {
            let other = b.ins().bxor_imm(base, 1);
            b.ins().iadd_imm(other, bias);
        }
        Placement::Narrow => {
            let narrow = b.ins().ireduce(types::I32, base);
            b.ins().iadd_imm(narrow, bias);
        }
        _ => {}
    }
    let allowed = b.ins().icmp(IntCC::UnsignedLessThan, index, limit);
    b.ins().brif(allowed, work, &[], failed, &[]);
    b.switch_to_block(failed);
    let error = b.ins().iconst(types::I64, 0xfeed);
    b.ins().return_(&[error]);
    b.switch_to_block(work);
    if matches!(placement, Placement::Sibling | Placement::CrowdedSibling) {
        let sibling = b.create_block();
        let bit = b.ins().band_imm(index, 1);
        b.ins().brif(bit, sibling, &[], body, &[]);
        b.switch_to_block(sibling);
        let count = if matches!(placement, Placement::CrowdedSibling) {
            70
        } else {
            1
        };
        for _ in 0..count {
            b.ins().iadd_imm(base, bias);
        }
        b.ins().jump(body, &[]);
    } else {
        b.ins().jump(body, &[]);
    }
    b.switch_to_block(body);
    let biased = if matches!(placement, Placement::Plain) {
        base
    } else {
        b.ins().iadd_imm(base, bias)
    };
    let bytes = b.ins().ishl_imm(index, 3);
    let first = b.ins().iadd(biased, bytes);
    let next = b.ins().iadd_imm(bytes, 8);
    // Exercise both commutative positions. The same biased value also feeds
    // ordinary memory and an observable address word.
    let second = b.ins().iadd(next, biased);
    if prefetch {
        b.ins().prefetch_write(first);
        b.ins().prefetch_write(second);
    }
    if matches!(placement, Placement::Later) {
        b.ins().iadd_imm(base, bias);
    }
    let loaded = if prefetch {
        b.ins().load(types::I64, MemFlags::new(), first, 0)
    } else {
        b.ins().uload32(MemFlags::new(), first, 0)
    };
    let changed = b.ins().bxor_imm(loaded, 0xdeadbeef);
    b.ins().store(MemFlags::new(), changed, second, 0);
    let result = b.ins().bxor(loaded, first);
    b.ins().return_(&[result]);
    b.seal_all_blocks();
    b.finalize();
    context.verify(module.isa()).unwrap();
    let before = effects(&context.func);
    context.func.aarch64_hoist_prefetch_bases = optimized && prefetch;
    context.func.aarch64_hoist_indexed_load_bases = optimized && !prefetch;
    context
        .optimize(
            module.isa(),
            &mut cranelift_codegen::control::ControlPlane::default(),
        )
        .unwrap();
    assert_eq!(
        effects(&context.func),
        before,
        "effect order and hint count"
    );
    context.verify(module.isa()).unwrap();
    let id = module
        .declare_function(name, Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    id
}

fn check_wrapping_addresses(prefetch: bool) {
    let mut inputs = vec![
        0,
        1,
        2,
        0xffff,
        u32::MAX as u64,
        1 << 32,
        1 << 63,
        u64::MAX - 1,
    ];
    let mut state = 0x123456789abcdef0u64;
    for _ in 0..1024 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        inputs.push(state & !1);
    }
    let mut calls = 0;
    for opt in ["none", "speed"] {
        for placement in [
            Placement::Dominator,
            Placement::Sibling,
            Placement::Later,
            Placement::DifferentBase,
            Placement::Narrow,
            Placement::Plain,
            Placement::CrowdedSibling,
        ] {
            for bias in [i64::MIN, -0x40000, -17, 0, 1, 0x20000, i64::MAX] {
                let mut module = JITModule::new(
                    JITBuilder::with_flags(
                        &[("opt_level", opt), ("regalloc_checker", "true")],
                        cranelift_module::default_libcall_names(),
                    )
                    .unwrap(),
                );
                let original = compile(&mut module, placement, bias, prefetch, false, "original");
                let shared = compile(&mut module, placement, bias, prefetch, true, "shared");
                module.finalize_definitions().unwrap();
                // SAFETY: both owned JIT functions have precisely this declared
                // ABI. The module stays live through every invocation below.
                let (original, shared): (
                    unsafe extern "C" fn(u64, u64, u64) -> u64,
                    unsafe extern "C" fn(u64, u64, u64) -> u64,
                ) = unsafe {
                    (
                        std::mem::transmute(module.get_finalized_function(original)),
                        std::mem::transmute(module.get_finalized_function(shared)),
                    )
                };
                for &index in &inputs {
                    let mut a = [0x0123456789abcdefu64, 0, 0xfedcba9876543210];
                    let mut b = a;
                    let bias = if matches!(placement, Placement::Plain) {
                        0
                    } else {
                        bias
                    };
                    let offset = (bias as u64).wrapping_add(index.wrapping_mul(8));
                    let first_a = a.as_mut_ptr() as u64;
                    let first_b = b.as_mut_ptr() as u64;
                    // SAFETY: wrapping cancellation places both hints and the
                    // load/store in the first two cells of their live arrays.
                    let results = unsafe {
                        (
                            original(first_a.wrapping_sub(offset), index, u64::MAX),
                            shared(first_b.wrapping_sub(offset), index, u64::MAX),
                        )
                    };
                    calls += 2;
                    assert_eq!(results.0 ^ first_a, results.1 ^ first_b);
                    assert_eq!(a, b, "{placement:?} {index:#x}");
                    let loaded = if prefetch {
                        a[0]
                    } else {
                        u64::from(u32::from_ne_bytes(
                            a[0].to_ne_bytes()[..4].try_into().unwrap(),
                        ))
                    };
                    assert_eq!(a[1], loaded ^ 0xdeadbeef);
                    assert_eq!(a[2], 0xfedcba9876543210);
                }
                for (base, index, limit) in [(0, 0, 0), (0, 1, 1), (u64::MAX, u64::MAX, 1)] {
                    // SAFETY: the unsigned guard rejects these inputs before
                    // either hint, load or store, including the null base.
                    unsafe {
                        assert_eq!(original(base, index, limit), 0xfeed);
                        assert_eq!(shared(base, index, limit), 0xfeed);
                    }
                    calls += 2;
                }
            }
        }
    }
    assert_eq!(calls, 202_860);
    eprintln!("biased base native calls (prefetch {prefetch}): {calls}");
}

#[test]
fn prefetch_bases_preserve_wrapping_addresses_and_shared_memory_uses() {
    check_wrapping_addresses(true);
}

#[test]
fn indexed_load_bases_preserve_wrapping_addresses_and_guarded_memory_uses() {
    check_wrapping_addresses(false);
}
