use super::*;
use cranelift_codegen::ir::{AbiParam, MemFlags};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};

#[derive(Clone, Copy, Debug)]
enum Form {
    Nested,
    Half,
    Negative,
    Masked,
    Truncated,
    ThreeTerms,
    Canonical,
}

impl Form {
    fn offset(self, index: u64, extra: u64) -> u64 {
        match self {
            Self::Nested => index.wrapping_mul(2).wrapping_add(0x20000).wrapping_mul(2),
            Self::Half => (index << 1).wrapping_add(0x20000),
            Self::Negative => index.wrapping_sub(17).wrapping_mul(8),
            Self::Masked => (index & 0xffff).wrapping_mul(4).wrapping_add(0x40000),
            Self::Truncated => u64::from((index as u32).wrapping_add(1))
                .wrapping_mul(4)
                .wrapping_add(0x40000),
            Self::ThreeTerms => index
                .wrapping_mul(4)
                .wrapping_add(extra)
                .wrapping_add(0x40000),
            Self::Canonical => index.wrapping_mul(4).wrapping_add(0x40000),
        }
    }
    fn changed(self) -> usize {
        usize::from(!matches!(self, Self::ThreeTerms | Self::Canonical))
    }
}

fn compile(
    module: &mut JITModule,
    form: Form,
    mode: Option<bool>,
    guard: bool,
    name: &str,
) -> FuncId {
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 3];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut state = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut state);
    let entry = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let [base, index, extra] = b.block_params(entry).try_into().unwrap();
    if guard {
        let read = b.create_block();
        let failure = b.create_block();
        let allowed = b.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::UnsignedLessThan,
            index,
            extra,
        );
        b.ins().brif(allowed, read, &[], failure, &[]);
        b.switch_to_block(failure);
        let error = b.ins().iconst(types::I64, 0xfeed);
        b.ins().return_(&[error]);
        b.switch_to_block(read);
    }
    let offset = match form {
        Form::Nested => {
            let two = b.ins().iconst(types::I64, 2);
            let scaled = b.ins().imul(index, two);
            let bias = b.ins().iconst(types::I64, 0x20000);
            let shifted = b.ins().iadd(scaled, bias);
            let one_based = b.ins().iadd_imm(shifted, 1);
            let ordinal = b.ins().iadd_imm(one_based, -1);
            b.ins().imul_imm(ordinal, 2)
        }
        Form::Half => {
            let scaled = b.ins().ishl_imm(index, 1);
            b.ins().iadd_imm(scaled, 0x20000)
        }
        Form::Negative => {
            let seventeen = b.ins().iconst(types::I64, 17);
            let ordinal = b.ins().isub(index, seventeen);
            b.ins().imul_imm(ordinal, 8)
        }
        Form::Masked => {
            let low = b.ins().band_imm(index, 0xffff);
            let scaled = b.ins().imul_imm(low, 4);
            b.ins().iadd_imm(scaled, 0x40000)
        }
        Form::Truncated => {
            let low = b.ins().ireduce(types::I32, index);
            let incremented = b.ins().iadd_imm(low, 1);
            let wide = b.ins().uextend(types::I64, incremented);
            let scaled = b.ins().imul_imm(wide, 4);
            b.ins().iadd_imm(scaled, 0x40000)
        }
        Form::ThreeTerms => {
            let scaled = b.ins().imul_imm(index, 4);
            let sum = b.ins().iadd(scaled, extra);
            b.ins().iadd_imm(sum, 0x40000)
        }
        Form::Canonical => b.ins().ishl_imm(index, 2),
    };
    let base = if matches!(form, Form::Canonical) {
        b.ins().iadd_imm(base, 0x40000)
    } else {
        base
    };
    let pointer = b.ins().iadd(base, offset);
    let loaded = if matches!(form, Form::Half) {
        b.ins().uload16(types::I64, MemFlags::new(), pointer, 0)
    } else {
        b.ins().load(types::I64, MemFlags::new(), pointer, 0)
    };
    let changed = b.ins().bxor_imm(loaded, 0xdeadbeef);
    b.ins().store(MemFlags::new(), changed, pointer, 0);
    b.ins().return_(&[loaded]);
    b.seal_all_blocks();
    b.finalize();
    if let Some(hoist) = mode {
        assert_eq!(
            normalize(&mut context.func, hoist),
            form.changed(),
            "{form:?}"
        );
        assert_eq!(
            normalize(&mut context.func, hoist),
            0,
            "normalization is stable"
        );
    }
    context.verify(module.isa()).unwrap();
    let id = module
        .declare_function(name, Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    id
}

fn pair(form: Form, hoist: bool, guard: bool) -> (JITModule, FuncId, FuncId) {
    let mut module =
        JITModule::new(JITBuilder::new(cranelift_module::default_libcall_names()).unwrap());
    let original = compile(&mut module, form, None, guard, "original_address");
    let optimized = compile(&mut module, form, Some(hoist), guard, "normalized_address");
    module.finalize_definitions().unwrap();
    (module, original, optimized)
}

#[test]
fn indexed_addresses_preserve_wrapping_casts_masks_and_memory_effects() {
    let mut inputs = vec![
        0,
        1,
        0x7fff,
        0xffff,
        0x10000,
        u32::MAX as u64,
        1 << 32,
        1 << 63,
        u64::MAX,
    ];
    let mut state = 0x123456789abcdef0u64;
    for _ in 0..1024 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        inputs.push(state);
    }
    for hoist in [false, true] {
        for form in [
            Form::Nested,
            Form::Half,
            Form::Negative,
            Form::Masked,
            Form::Truncated,
            Form::ThreeTerms,
            Form::Canonical,
        ] {
            let (module, original, optimized) = pair(form, hoist, false);
            // SAFETY: both JIT functions have exactly this declared signature;
            // module stays alive, and each computed address is the aligned
            // eight-byte cell in its corresponding live array, even on wrap.
            let (original, optimized): (
                unsafe extern "C" fn(u64, u64, u64) -> u64,
                unsafe extern "C" fn(u64, u64, u64) -> u64,
            ) = unsafe {
                (
                    std::mem::transmute(module.get_finalized_function(original)),
                    std::mem::transmute(module.get_finalized_function(optimized)),
                )
            };
            for &index in &inputs {
                let extra = index.rotate_left(19);
                let mut a = [0x0123456789abcdefu64, 0xfedcba9876543210];
                let mut b = a;
                let offset = form.offset(index, extra);
                let base_a = (a.as_mut_ptr() as u64).wrapping_sub(offset);
                let base_b = (b.as_mut_ptr() as u64).wrapping_sub(offset);
                // SAFETY: the wrapping cancellation above makes both effective
                // addresses valid for the load and store; signatures and owners
                // are those retained immediately above.
                let results = unsafe {
                    (
                        original(base_a, index, extra),
                        optimized(base_b, index, extra),
                    )
                };
                assert_eq!(results.0, results.1, "{form:?} {index:#x}");
                assert_eq!(a, b, "{form:?} {index:#x}");
                assert_eq!(a[1], 0xfedcba9876543210);
            }
        }
    }
}

#[test]
fn indexed_addresses_keep_the_guard_before_memory_access() {
    for hoist in [false, true] {
        let (module, original, optimized) = pair(Form::Nested, hoist, true);
        // SAFETY: both finalized functions have the declared ABI and remain
        // owned by module; the rejected indices below return before any access.
        let (original, optimized): (
            unsafe extern "C" fn(u64, u64, u64) -> u64,
            unsafe extern "C" fn(u64, u64, u64) -> u64,
        ) = unsafe {
            (
                std::mem::transmute(module.get_finalized_function(original)),
                std::mem::transmute(module.get_finalized_function(optimized)),
            )
        };
        for (base, index, limit) in [
            (0, 0, 0),
            (u64::MAX, 1, 1),
            (0, 1 << 63, 4),
            (0, u64::MAX, u64::MAX),
        ] {
            // SAFETY: index >= limit in each case, so neither function can
            // dereference the intentionally invalid base.
            let results = unsafe { (original(base, index, limit), optimized(base, index, limit)) };
            assert_eq!(results, (0xfeed, 0xfeed));
        }
    }
}
