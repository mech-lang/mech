use super::*;
use cranelift_codegen::ir::{AbiParam, Endianness, InstBuilder, MemFlags, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module};

#[derive(Clone, Copy, Debug)]
enum Shape {
    Chain64,
    Chain32,
    LiveArithmetic,
    MixedFlags,
    Vector,
}
fn build(
    module: &mut cranelift_jit::JITModule,
    shape: Shape,
    selected: bool,
    serial: usize,
) -> cranelift_module::FuncId {
    let mut cx = module.make_context();
    cx.func.signature.params = vec![AbiParam::new(types::I64); 3];
    cx.func.signature.returns = vec![AbiParam::new(types::I64)];
    let id = module
        .declare_function(
            &format!("inverse_bits_{serial}"),
            Linkage::Local,
            &cx.func.signature,
        )
        .unwrap();
    let mut fc = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut cx.func, &mut fc);
        let root = b.create_block();
        let body = b.create_block();
        let error = b.create_block();
        b.append_block_params_for_function_params(root);
        b.switch_to_block(root);
        let args = b.block_params(root).to_vec();
        b.ins().brif(args[2], body, &[], error, &[]);
        b.switch_to_block(body);
        let flags = MemFlags::new();
        let value = match shape {
            Shape::Chain64 | Shape::Chain32 => {
                let (integer, float) = if matches!(shape, Shape::Chain32) {
                    (types::I32, types::F32)
                } else {
                    (types::I64, types::F64)
                };
                let mut x = if integer == types::I32 {
                    b.ins().ireduce(integer, args[0])
                } else {
                    args[0]
                };
                for _ in 0..2 {
                    let f = b.ins().bitcast(float, flags, x);
                    x = b.ins().bitcast(integer, flags, f);
                }
                if integer == types::I32 {
                    b.ins().uextend(types::I64, x)
                } else {
                    x
                }
            }
            Shape::LiveArithmetic => {
                let x = b.ins().bitcast(types::F64, flags, args[0]);
                let factor = b.ins().f64const(1.0000000000000002);
                let sum = b.ins().fmul(x, factor);
                let bits = b.ins().bitcast(types::I64, flags, sum);
                b.ins().store(flags, bits, args[1], 0);
                let restored = b.ins().bitcast(types::F64, flags, bits);
                let addend = b.ins().f64const(-1.0000000000000004);
                let result = b.ins().fadd(restored, addend);
                b.ins().bitcast(types::I64, flags, result)
            }
            Shape::MixedFlags => {
                let f = b.ins().bitcast(
                    types::F64,
                    flags.with_endianness(Endianness::Little),
                    args[0],
                );
                b.ins()
                    .bitcast(types::I64, flags.with_endianness(Endianness::Big), f)
            }
            Shape::Vector => {
                let flags = flags.with_endianness(Endianness::Little);
                let x = b.ins().load(types::I8X16, MemFlags::new(), args[0], 0);
                let f = b.ins().bitcast(types::I64X2, flags, x);
                let x = b.ins().bitcast(types::I8X16, flags, f);
                let byte = b.ins().extractlane(x, 0);
                b.ins().uextend(types::I64, byte)
            }
        };
        b.ins().return_(&[value]);
        b.switch_to_block(error);
        let sentinel = b.ins().iconst(types::I64, 0xdead);
        b.ins().return_(&[sentinel]);
        b.seal_all_blocks();
        b.finalize();
    }
    cx.verify(module.isa()).unwrap();
    if selected {
        let count = fold(&mut cx.func);
        assert_eq!(
            count,
            match shape {
                Shape::Chain64 | Shape::Chain32 => 2,
                Shape::LiveArithmetic => 1,
                _ => 0,
            }
        );
    }
    cx.verify(module.isa()).unwrap();
    module.define_function(id, &mut cx).unwrap();
    id
}
fn patterns() -> Vec<u64> {
    let mut xs = vec![
        0,
        1,
        1 << 63,
        u64::MAX,
        1.0f64.to_bits(),
        1.0000000000000002f64.to_bits(),
        (-7.0f64).to_bits(),
        f64::MAX.to_bits(),
        f64::INFINITY.to_bits(),
        f64::NEG_INFINITY.to_bits(),
        0x7ff8_0000_0000_0042,
        0x7ff0_0000_0000_0031,
        0xfff8_0000_0000_0064,
        0x7fc0_0042,
        0x7f80_0031,
    ];
    for bit in 0..64 {
        xs.extend([1 << bit, !(1 << bit)]);
    }
    let mut seed = 0x248f_924d_571a_630cu64;
    for _ in 0..128 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        xs.push(seed);
    }
    xs
}
#[test]
fn numeric_inverse_bitcasts_preserve_all_scalar_bits_and_reached_error_paths() {
    let mut module = crate::native_artifact::codegen::backend::jit(
        crate::native_artifact::codegen::backend::CONTROL_FLAGS,
    )
    .unwrap();
    let mut cases = Vec::new();
    for shape in [Shape::Chain64, Shape::Chain32] {
        for selected in [false, true] {
            let id = build(&mut module, shape, selected, cases.len());
            cases.push((id, shape));
        }
    }
    module.finalize_definitions().unwrap();
    for (id, shape) in cases {
        let call: extern "C" fn(u64, *mut u64, u64) -> u64 =
            unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        for input in patterns() {
            let expected = if matches!(shape, Shape::Chain32) {
                u64::from(input as u32)
            } else {
                input
            };
            let mut out = 0xcafe;
            assert_eq!(call(input, &mut out, 1), expected);
            assert_eq!(out, 0xcafe);
            assert_eq!(call(input, core::ptr::null_mut(), 0), 0xdead);
        }
    }
}
#[test]
fn numeric_inverse_bitcasts_keep_live_word_observations_and_exact_float_operation_order() {
    let mut module = crate::native_artifact::codegen::backend::jit(
        crate::native_artifact::codegen::backend::CONTROL_FLAGS,
    )
    .unwrap();
    let before = build(&mut module, Shape::LiveArithmetic, false, 0);
    let after = build(&mut module, Shape::LiveArithmetic, true, 1);
    module.finalize_definitions().unwrap();
    let before: extern "C" fn(u64, *mut u64, u64) -> u64 =
        unsafe { core::mem::transmute(module.get_finalized_function(before)) };
    let after: extern "C" fn(u64, *mut u64, u64) -> u64 =
        unsafe { core::mem::transmute(module.get_finalized_function(after)) };
    for bits in patterns() {
        let (mut a, mut b) = (0xcafe, 0xcafe);
        let first = before(bits, &mut a, 1);
        let second = after(bits, &mut b, 1);
        assert_eq!((first, a), (second, b), "{bits:#x}");
        assert_eq!(after(bits, core::ptr::null_mut(), 0), 0xdead);
    }
}
#[test]
fn numeric_inverse_bitcasts_leave_mixed_flags_and_vector_lane_reinterpretations_unchanged() {
    let mut module = crate::native_artifact::codegen::backend::jit(
        crate::native_artifact::codegen::backend::CONTROL_FLAGS,
    )
    .unwrap();
    let mut cases = Vec::new();
    for shape in [Shape::MixedFlags, Shape::Vector] {
        let id = build(&mut module, shape, true, cases.len());
        cases.push((id, shape));
    }
    module.finalize_definitions().unwrap();
    let bytes = [0x42u8; 16];
    for (id, shape) in cases {
        let call: extern "C" fn(u64, *mut u64, u64) -> u64 =
            unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        let input = if matches!(shape, Shape::Vector) {
            bytes.as_ptr() as u64
        } else {
            0x7ff0_0000_0000_0031
        };
        let expected = if matches!(shape, Shape::Vector) {
            0x42
        } else {
            input
        };
        assert_eq!(call(input, core::ptr::null_mut(), 1), expected);
    }
}
