use crate::native_artifact::codegen::options::{CompilationControls, OptimizationOptions};
use crate::native_artifact::program::codegen::cfg_cleanup::share_repeated_bounds;
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, condcodes::IntCC, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Variant {
    Equivalent,
    OtherIndex,
    OtherLength,
    Undominated,
    FalseStore,
    FalseLoad,
    FailureArg,
    FailureOperand,
    Cycle,
}

fn invoke(variant: Variant, optimize: bool) -> Vec<(u64, [u64; 2])> {
    let mut module = JITModule::new(
        JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names()).unwrap(),
    );
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 4];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let id = module
        .declare_function("tally", Linkage::Local, &context.func.signature)
        .unwrap();
    let mut builder_context = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut builder_context);
    let entry = b.create_block();
    let first = b.create_block();
    let read = b.create_block();
    let default = b.create_block();
    let join = b.create_block();
    let write = b.create_block();
    let rejected = b.create_block();
    let return_error = b.create_block();
    b.set_cold_block(rejected);
    b.set_cold_block(return_error);
    b.append_block_param(join, types::I64);
    b.append_block_param(rejected, types::I64);
    b.append_block_param(return_error, types::I64);
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let [index, length, bypass, output]: [_; 4] = b.block_params(entry).try_into().unwrap();
    let zero = b.ins().iconst(types::I64, 0);
    let origin = b.ins().iconst(types::I64, 0x3dc7);
    if variant == Variant::Undominated {
        b.ins().brif(bypass, join, &[zero.into()], first, &[]);
    } else {
        b.ins().jump(first, &[]);
    }
    b.switch_to_block(first);
    let valid = b.ins().icmp(IntCC::UnsignedLessThan, index, length);
    let widened = b.ins().uextend(types::I64, valid);
    let condition = b.ins().icmp_imm(IntCC::Equal, widened, 1);
    b.ins().brif(condition, read, &[], default, &[]);
    b.switch_to_block(read);
    let current = b.ins().load(types::I64, MemFlags::new(), output, 0);
    b.ins().jump(join, &[current.into()]);
    b.switch_to_block(default);
    if variant == Variant::FalseStore {
        b.ins().store(MemFlags::new(), zero, output, 0);
    }
    let fallback = if variant == Variant::FalseLoad {
        b.ins().load(types::I64, MemFlags::new(), output, 0)
    } else {
        zero
    };
    if variant == Variant::Cycle {
        b.ins().jump(default, &[]);
    } else {
        b.ins().jump(join, &[fallback.into()]);
    }
    b.switch_to_block(join);
    let current = b.block_params(join)[0];
    let updated = b.ins().iadd_imm(current, 1);
    let one_based = b.ins().iadd_imm(index, 1);
    let ordinal = if variant == Variant::OtherIndex {
        one_based
    } else {
        b.ins().iadd_imm(one_based, -1)
    };
    let limit = if variant == Variant::OtherLength {
        b.ins().iadd_imm(length, 1)
    } else {
        length
    };
    let valid = b.ins().icmp(IntCC::UnsignedGreaterThan, limit, ordinal);
    let error_arg = if variant == Variant::FailureArg {
        updated
    } else {
        origin
    };
    b.ins()
        .brif(valid, write, &[], rejected, &[error_arg.into()]);
    b.switch_to_block(write);
    b.ins().store(MemFlags::new(), updated, output, 0);
    b.ins().return_(&[zero]);
    b.switch_to_block(rejected);
    let mut origin = b.block_params(rejected)[0];
    if variant == Variant::FailureOperand {
        origin = updated;
    }
    b.ins().store(MemFlags::new(), origin, output, 8);
    let status = b.ins().iconst(types::I64, 0x19301);
    b.ins().jump(return_error, &[status.into()]);
    b.switch_to_block(return_error);
    let status = b.block_params(return_error)[0];
    b.ins().return_(&[status]);
    b.seal_all_blocks();
    b.finalize();

    context.verify(module.isa()).unwrap();
    if optimize {
        let controls = CompilationControls::new(OptimizationOptions::default());
        let changes = share_repeated_bounds(&controls, &mut context, module.isa()).unwrap();
        assert_eq!(
            changes,
            usize::from(variant == Variant::Equivalent),
            "{variant:?}"
        );
    }
    if variant == Variant::Cycle {
        return Vec::new();
    }
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    // SAFETY: the fixture's ABI is four words. Its only memory access is to
    // either of the two live U64 cells in the last argument.
    let entry: unsafe extern "C" fn(u64, u64, u64, *mut u64) -> u64 =
        unsafe { core::mem::transmute(module.get_finalized_function(id)) };
    let mut output = [20, 0];
    [(0, 1, 0), (1, 1, 0), (0, 1, 0), (u64::MAX, 1, 1), (0, 0, 1)]
        .into_iter()
        .map(|(index, length, bypass)| {
            let result = unsafe { entry(index, length, bypass, output.as_mut_ptr()) };
            (result, output)
        })
        .collect()
}

#[test]
fn repeated_bounds_keep_the_current_visit_and_original_rejection() {
    let expected = vec![
        (0, [21, 0]),
        (0x19301, [21, 0x3dc7]),
        (0, [22, 0x3dc7]),
        (0x19301, [22, 0x3dc7]),
        (0x19301, [22, 0x3dc7]),
    ];
    assert_eq!(invoke(Variant::Equivalent, false), expected);
    assert_eq!(invoke(Variant::Equivalent, true), expected);
}

#[test]
fn repeated_bounds_require_the_same_test_and_an_effect_free_route() {
    for variant in [
        Variant::OtherIndex,
        Variant::OtherLength,
        Variant::Undominated,
        Variant::FalseStore,
        Variant::FalseLoad,
        Variant::FailureArg,
        Variant::FailureOperand,
        Variant::Cycle,
    ] {
        assert_eq!(invoke(variant, true), invoke(variant, false), "{variant:?}");
    }
}
