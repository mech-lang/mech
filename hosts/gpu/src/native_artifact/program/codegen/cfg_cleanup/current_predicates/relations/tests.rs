use crate::native_artifact::codegen::options::{CompilationControls, OptimizationOptions};
use crate::native_artifact::program::codegen::cfg_cleanup::reuse_dominating_window_facts;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    Same,
    FreshLoad,
    Bypass,
    DifferentWidth,
    SignedBound,
}

type Entry = unsafe extern "C" fn(*mut u64, u64, u64, u64, u64, u64) -> u64;

fn compile(shape: Shape, optimize: bool) -> (cranelift_jit::JITModule, Entry) {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 6];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fb = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
        let entry = b.create_block();
        let initial = b.create_block();
        let initial_bound = b.create_block();
        let body = b.create_block();
        let check_order = b.create_block();
        let accept = b.create_block();
        let fail_overflow = b.create_block();
        let fail_bound = b.create_block();
        let fail_later_overflow = b.create_block();
        let fail_later_bound = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let [storage, position, width, candidate, new_bound, bypass] =
            b.block_params(entry).try_into().unwrap();
        let flags = MemFlags::new();
        let bound = b.ins().load(types::I64, flags, storage, 0);
        if shape == Shape::Bypass {
            b.ins().brif(bypass, body, &[], initial, &[]);
        } else {
            b.ins().jump(initial, &[]);
        }
        b.switch_to_block(initial);
        let sum = b.ins().iadd(position, width);
        let no_overflow = b
            .ins()
            .icmp(IntCC::UnsignedGreaterThanOrEqual, sum, position);
        b.ins()
            .brif(no_overflow, initial_bound, &[], fail_overflow, &[]);
        b.switch_to_block(initial_bound);
        let readable = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, sum, bound);
        let widened = b.ins().uextend(types::I64, readable);
        b.ins().brif(widened, body, &[], fail_bound, &[]);
        b.switch_to_block(body);
        b.ins().store(flags, new_bound, storage, 0);
        let width = if shape == Shape::DifferentWidth {
            b.ins().iadd_imm(width, 1)
        } else {
            width
        };
        let sum = b.ins().iadd(width, position);
        let no_overflow = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, position, sum);
        b.ins()
            .brif(no_overflow, check_order, &[], fail_later_overflow, &[]);
        b.switch_to_block(check_order);
        let bound = if shape == Shape::FreshLoad {
            b.ins().load(types::I64, flags, storage, 0)
        } else {
            bound
        };
        let cond = if shape == Shape::SignedBound {
            IntCC::SignedLessThanOrEqual
        } else {
            IntCC::UnsignedLessThanOrEqual
        };
        let readable = b.ins().icmp(cond, sum, bound);
        let precedes = b.ins().icmp(IntCC::UnsignedLessThan, candidate, position);
        let both = b.ins().band(readable, precedes);
        b.ins().brif(both, accept, &[], fail_later_bound, &[]);
        for (block, status) in [
            (accept, 0),
            (fail_overflow, 1),
            (fail_bound, 2),
            (fail_later_overflow, 3),
            (fail_later_bound, 4),
        ] {
            b.switch_to_block(block);
            let status = b.ins().iconst(types::I64, status);
            b.ins().return_(&[status]);
        }
        b.seal_all_blocks();
        b.finalize();
    }
    if optimize {
        let controls = CompilationControls::new(OptimizationOptions::default());
        let expected = match shape {
            Shape::Same => 2,
            Shape::FreshLoad | Shape::SignedBound => 1,
            Shape::Bypass | Shape::DifferentWidth => 0,
        };
        assert_eq!(
            reuse_dominating_window_facts(&controls, &mut context, module.isa()).unwrap(),
            expected,
            "{shape:?}",
        );
    }
    context.verify(module.isa()).unwrap();
    let id = module
        .declare_function("window_facts", Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    // The verified function reads and writes one live U64 through its pointer.
    let call =
        unsafe { core::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) };
    (module, call)
}

#[test]
fn selected_window_facts_preserve_rejections_and_current_memory() {
    for shape in [
        Shape::Same,
        Shape::FreshLoad,
        Shape::Bypass,
        Shape::DifferentWidth,
        Shape::SignedBound,
    ] {
        let (_before, before) = compile(shape, false);
        let (_after, after) = compile(shape, true);
        for (position, width, bound, candidate, replacement) in [
            (8, 3, 16, 4, 0),
            (8, 3, 16, 8, 20),
            (8, 3, 10, 4, 16),
            (u64::MAX, 1, u64::MAX, 0, 16),
            (u64::MAX - 1, 1, u64::MAX, 0, 16),
            (8, 0, u64::MAX, 7, 32),
        ] {
            for bypass in [0, 1] {
                let mut left = bound;
                let mut right = bound;
                let expected =
                    unsafe { before(&mut left, position, width, candidate, replacement, bypass) };
                let observed =
                    unsafe { after(&mut right, position, width, candidate, replacement, bypass) };
                assert_eq!((observed, right), (expected, left), "{shape:?}");
            }
        }
    }
}
