use crate::native_artifact::codegen::options::{CompilationControls, OptimizationOptions};
use crate::native_artifact::program::codegen::cfg_cleanup::{
    prove_scalar_spans, reuse_selected_read_bounds,
};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    Checked,
    Bypass,
    WrappingEnd,
    InclusiveOffset,
    DifferentLength,
}

type Entry = unsafe extern "C" fn(*const u8, u64, u64, u64, u64, u64, u64, u64) -> u64;

fn compile(shape: Shape, optimize: bool) -> (cranelift_jit::JITModule, Entry) {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 8];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fb = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
        let entry = b.create_block();
        let initial = b.create_block();
        let window = b.create_block();
        let extent = b.create_block();
        let choose = b.create_block();
        let subtract = b.create_block();
        let clamp = b.create_block();
        let check = b.create_block();
        let increment = b.create_block();
        let load = b.create_block();
        let default = b.create_block();
        let rejected = b.create_block();
        b.append_block_params_for_function_params(entry);
        let base = b.append_block_param(check, types::I64);
        b.switch_to_block(entry);
        let [
            input,
            position,
            maximum,
            offset,
            delta,
            length,
            other_length,
            bypass,
        ] = b.block_params(entry).try_into().unwrap();
        if shape == Shape::Bypass {
            b.ins().brif(bypass, choose, &[], initial, &[]);
        } else {
            b.ins().jump(initial, &[]);
        }
        b.switch_to_block(initial);
        let end = b.ins().iadd(position, maximum);
        if shape == Shape::WrappingEnd {
            b.ins().jump(window, &[]);
        } else {
            let nonwrapping = b
                .ins()
                .icmp(IntCC::UnsignedGreaterThanOrEqual, end, position);
            b.ins().brif(nonwrapping, window, &[], rejected, &[]);
        }
        b.switch_to_block(window);
        let fits = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, end, length);
        b.ins().brif(fits, extent, &[], rejected, &[]);
        b.switch_to_block(extent);
        let relation = if shape == Shape::InclusiveOffset {
            IntCC::UnsignedLessThanOrEqual
        } else {
            IntCC::UnsignedLessThan
        };
        let short = b.ins().icmp(relation, offset, maximum);
        b.ins().brif(short, choose, &[], rejected, &[]);
        b.switch_to_block(choose);
        let nonwrapping = b
            .ins()
            .icmp(IntCC::UnsignedGreaterThanOrEqual, position, delta);
        b.ins().brif(nonwrapping, subtract, &[], clamp, &[]);
        b.switch_to_block(subtract);
        let earlier = b.ins().isub(position, delta);
        b.ins().jump(check, &[earlier.into()]);
        b.switch_to_block(clamp);
        let zero = b.ins().iconst(types::I64, 0);
        b.ins().jump(check, &[zero.into()]);
        b.switch_to_block(check);
        let index = b.ins().iadd(base, offset);
        let readable_length = if shape == Shape::DifferentLength {
            other_length
        } else {
            length
        };
        let readable = b
            .ins()
            .icmp(IntCC::UnsignedLessThan, index, readable_length);
        b.ins().brif(readable, increment, &[], default, &[]);
        b.switch_to_block(increment);
        let one = b.ins().iconst(types::I64, 1);
        let one_based = b.ins().iadd(index, one);
        let no_overflow = b
            .ins()
            .icmp(IntCC::UnsignedGreaterThanOrEqual, one_based, index);
        b.ins().brif(no_overflow, load, &[], rejected, &[]);
        b.switch_to_block(load);
        let displacement = b.ins().iadd_imm(one_based, -1);
        let address = b.ins().iadd(input, displacement);
        let byte = b.ins().load(types::I8, MemFlags::new(), address, 0);
        let byte = b.ins().uextend(types::I64, byte);
        b.ins().return_(&[byte]);
        b.switch_to_block(default);
        let zero = b.ins().iconst(types::I64, 0);
        b.ins().return_(&[zero]);
        b.switch_to_block(rejected);
        let sentinel = b.ins().iconst(types::I64, -1);
        b.ins().return_(&[sentinel]);
        b.seal_all_blocks();
        b.finalize();
    }
    if optimize {
        let controls = CompilationControls::new(OptimizationOptions::default());
        let expected = if shape == Shape::Checked { 2 } else { 0 };
        assert_eq!(
            prove_scalar_spans(&controls, &mut context, module.isa()).unwrap(),
            expected,
            "{shape:?}"
        );
    }
    context.verify(module.isa()).unwrap();
    let id = module
        .declare_function("scalar_span", Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    // Each selected raw byte read is within the supplied length of the live array.
    let call =
        unsafe { core::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) };
    (module, call)
}

#[test]
fn scalar_spans_keep_the_zero_default_when_the_proof_is_missing() {
    let input = [2u8, 4, 9, 13, 17, 29, 33, 41];
    for shape in [
        Shape::Checked,
        Shape::Bypass,
        Shape::WrappingEnd,
        Shape::InclusiveOffset,
        Shape::DifferentLength,
    ] {
        let (_before, baseline) = compile(shape, false);
        let (_after, optimized) = compile(shape, true);
        for (position, maximum, offset, delta, length, other_length) in [
            (3, 4, 0, 0, 8, 2),
            (3, 4, 3, 5, 8, 2),
            (3, 4, 3, 1, 8, 8),
            (3, 4, 4, 0, 7, 8),
            (3, 5, 4, 0, 8, 3),
            (5, 4, 2, 6, 8, 8),
            (u64::MAX, 2, 0, 0, 8, 8),
            (u64::MAX, 2, 1, 0, 8, 8),
        ] {
            for bypass in [0, 1] {
                let ptr = input.as_ptr();
                let want = unsafe {
                    baseline(
                        ptr,
                        position,
                        maximum,
                        offset,
                        delta,
                        length,
                        other_length,
                        bypass,
                    )
                };
                let got = unsafe {
                    optimized(
                        ptr,
                        position,
                        maximum,
                        offset,
                        delta,
                        length,
                        other_length,
                        bypass,
                    )
                };
                assert_eq!(want, got, "{shape:?}");
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReadRoute {
    Exact,
    Masked,
    MaskedWrongBit,
    MaskedRawWord,
    Reversed,
    AlternateEntry,
    OtherLength,
    LoadedIndex,
    Signed,
    Inclusive,
    DifferentStep,
}

type Read = unsafe extern "C" fn(*const u8, u64, u64, u64, u64, *mut u64) -> u64;

fn compile_read(route: ReadRoute, prove: bool) -> (cranelift_jit::JITModule, Read) {
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
        let check = b.create_block();
        let selected = b.create_block();
        let bounds = b.create_block();
        let read = b.create_block();
        let default = b.create_block();
        let overflow = b.create_block();
        let outside = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let [input, at, length, other, bypass, observed] =
            b.block_params(entry).try_into().unwrap();
        if route == ReadRoute::AlternateEntry {
            b.ins().brif(bypass, selected, &[], check, &[]);
        } else {
            b.ins().jump(check, &[]);
        }
        b.switch_to_block(check);
        match route {
            ReadRoute::Masked | ReadRoute::MaskedWrongBit => {
                let available = b.ins().icmp(IntCC::UnsignedLessThan, at, length);
                let bit = if route == ReadRoute::Masked { 1 } else { 2 };
                let available = b.ins().band_imm(available, bit);
                b.ins().brif(available, selected, &[], default, &[]);
            }
            ReadRoute::MaskedRawWord => {
                let available = b.ins().band_imm(at, 1);
                b.ins().brif(available, selected, &[], default, &[]);
            }
            ReadRoute::Reversed => {
                let unavailable = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, length, at);
                b.ins().brif(unavailable, default, &[], selected, &[]);
            }
            _ => {
                let relation = match route {
                    ReadRoute::Signed => IntCC::SignedLessThan,
                    ReadRoute::Inclusive => IntCC::UnsignedLessThanOrEqual,
                    _ => IntCC::UnsignedLessThan,
                };
                let available = b.ins().icmp(relation, at, length);
                let word = b.ins().uextend(types::I64, available);
                let available = b.ins().icmp_imm(IntCC::Equal, word, 1);
                b.ins().brif(available, selected, &[], default, &[]);
            }
        }
        b.switch_to_block(selected);
        b.ins().store(MemFlags::new(), other, observed, 0);
        let at = if route == ReadRoute::LoadedIndex {
            b.ins().load(types::I64, MemFlags::new(), observed, 0)
        } else {
            at
        };
        let step = b.ins().iconst(
            types::I64,
            if route == ReadRoute::DifferentStep {
                2
            } else {
                1
            },
        );
        let one_based = b.ins().iadd(step, at);
        let nonwrapping = b
            .ins()
            .icmp(IntCC::UnsignedGreaterThanOrEqual, one_based, at);
        b.ins().brif(nonwrapping, bounds, &[], overflow, &[]);
        b.switch_to_block(bounds);
        let length = if route == ReadRoute::OtherLength {
            other
        } else {
            length
        };
        let index = b.ins().iadd_imm(one_based, -1);
        let inside = b.ins().icmp(IntCC::UnsignedLessThan, index, length);
        b.ins().brif(inside, read, &[], outside, &[]);
        b.switch_to_block(read);
        let address = b.ins().iadd(input, index);
        let byte = b.ins().load(types::I8, MemFlags::new(), address, 0);
        let byte = b.ins().uextend(types::I64, byte);
        b.ins().return_(&[byte]);
        for (block, result) in [(default, 0), (overflow, -1), (outside, -2)] {
            b.switch_to_block(block);
            let value = b.ins().iconst(types::I64, result);
            b.ins().return_(&[value]);
        }
        b.seal_all_blocks();
        b.finalize();
    }
    if prove {
        let controls = CompilationControls::new(OptimizationOptions::default());
        let expected = if matches!(
            route,
            ReadRoute::Exact | ReadRoute::Masked | ReadRoute::Reversed
        ) {
            2
        } else {
            0
        };
        assert_eq!(
            reuse_selected_read_bounds(&controls, &mut context, module.isa()).unwrap(),
            expected,
            "{route:?}"
        );
    }
    context.verify(module.isa()).unwrap();
    let id = module
        .declare_function("selected_read", Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    // The controls supply live buffers for all successful read and store paths.
    let call =
        unsafe { core::mem::transmute::<*const u8, Read>(module.get_finalized_function(id)) };
    (module, call)
}

#[test]
fn scalar_spans_reuse_the_current_selected_read_bound() {
    let input = [2u8, 4, 9, 13, 17, 29, 33, 41];
    for route in [
        ReadRoute::Exact,
        ReadRoute::Masked,
        ReadRoute::MaskedWrongBit,
        ReadRoute::MaskedRawWord,
        ReadRoute::Reversed,
        ReadRoute::AlternateEntry,
        ReadRoute::OtherLength,
        ReadRoute::LoadedIndex,
        ReadRoute::Signed,
        ReadRoute::Inclusive,
        ReadRoute::DifferentStep,
    ] {
        let (_before, baseline) = compile_read(route, false);
        let (_after, optimized) = compile_read(route, true);
        for (at, length, other, bypass) in [
            (0, 0, 0, 0),
            (0, 8, 7, 0),
            (7, 8, 0, 0),
            (8, 8, 7, 0),
            (u64::MAX, 8, 2, 0),
            (u64::MAX, u64::MAX, 2, 0),
            (9, 8, 4, 1),
            (u64::MAX, 8, 4, 1),
            (2, 8, 0, 1),
            (4, 4, 4, 1),
            (4, 8, 3, 0),
            (1, 8, 0, 0),
        ] {
            let selected = match route {
                ReadRoute::AlternateEntry => bypass != 0 || at < length,
                ReadRoute::Signed => (at as i64) < length as i64,
                ReadRoute::Inclusive => at <= length,
                ReadRoute::MaskedWrongBit => false,
                ReadRoute::MaskedRawWord => at & 1 != 0,
                _ => at < length,
            };
            let expected_effect = if selected { other } else { u64::MAX };
            let expected = if !selected {
                0
            } else {
                let at = if route == ReadRoute::LoadedIndex {
                    other
                } else {
                    at
                };
                let step = if route == ReadRoute::DifferentStep {
                    2
                } else {
                    1
                };
                let length = if route == ReadRoute::OtherLength {
                    other
                } else {
                    length
                };
                match at.checked_add(step) {
                    None => u64::MAX,
                    Some(one_based) if one_based - 1 >= length => u64::MAX - 1,
                    Some(one_based) => u64::from(input[(one_based - 1) as usize]),
                }
            };
            for call in [baseline, optimized] {
                let mut effect = u64::MAX;
                let value = unsafe { call(input.as_ptr(), at, length, other, bypass, &mut effect) };
                assert_eq!(
                    (value, effect),
                    (expected, expected_effect),
                    "{route:?}, at={at}, length={length}, other={other}, bypass={bypass}"
                );
            }
        }
    }
}
