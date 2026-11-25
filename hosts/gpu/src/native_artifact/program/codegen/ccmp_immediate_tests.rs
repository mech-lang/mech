//! Execute immediate comparison chains across carry, overflow and false edges.
use cranelift_codegen::ir::{AbiParam, InstBuilder, Type, condcodes::IntCC, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

type Run = unsafe extern "C" fn(u64, u64, u64, u64) -> u64;
const CONDITIONS: [IntCC; 10] = [
    IntCC::Equal,
    IntCC::NotEqual,
    IntCC::SignedLessThan,
    IntCC::SignedLessThanOrEqual,
    IntCC::SignedGreaterThan,
    IntCC::SignedGreaterThanOrEqual,
    IntCC::UnsignedLessThan,
    IntCC::UnsignedLessThanOrEqual,
    IntCC::UnsignedGreaterThan,
    IntCC::UnsignedGreaterThanOrEqual,
];
const WORDS: [u64; 14] = [
    0,
    1,
    8,
    31,
    32,
    0x7fff_ffff,
    0x8000_0000,
    0xffff_ffff,
    i64::MAX as u64,
    i64::MIN as u64,
    u64::MAX - 31,
    u64::MAX - 9,
    u64::MAX - 1,
    u64::MAX,
];

fn compare(cc: IntCC, a: u64, b: u64, ty: Type) -> bool {
    let shift = 64 - ty.bits();
    let mask = u64::MAX >> shift;
    let (a, b) = (a & mask, b & mask);
    let signed = |x| ((x << shift) as i64) >> shift;
    match cc {
        IntCC::Equal => a == b,
        IntCC::NotEqual => a != b,
        IntCC::SignedLessThan => signed(a) < signed(b),
        IntCC::SignedLessThanOrEqual => signed(a) <= signed(b),
        IntCC::SignedGreaterThan => signed(a) > signed(b),
        IntCC::SignedGreaterThanOrEqual => signed(a) >= signed(b),
        IntCC::UnsignedLessThan => a < b,
        IntCC::UnsignedLessThanOrEqual => a <= b,
        IntCC::UnsignedGreaterThan => a > b,
        IntCC::UnsignedGreaterThanOrEqual => a >= b,
    }
}

fn compile(
    ty: Type,
    constant: i64,
    cc: IntCC,
    swapped: bool,
    opt: &str,
    enabled: bool,
) -> (JITModule, Run, String) {
    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", opt), ("regalloc_checker", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let mut context = module.make_context();
    context.set_disasm(true);
    context.func.aarch64_ccmp_immediates = enabled;
    context.func.signature.params = vec![AbiParam::new(types::I64); 4];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut storage = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut storage);
        let entry = b.create_block();
        let yes = b.create_block();
        let no = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let [x, y, z, seed] = b.block_params(entry).try_into().unwrap();
        let gate = b.ins().icmp(IntCC::UnsignedLessThan, x, y);
        let z = if ty == types::I64 {
            z
        } else {
            b.ins().ireduce(ty, z)
        };
        let k = b.ins().iconst(ty, constant);
        let comparison = if swapped {
            b.ins().icmp(cc, k, z)
        } else {
            b.ins().icmp(cc, z, k)
        };
        let condition = b.ins().band(gate, comparison);
        b.ins().brif(condition, yes, &[], no, &[]);
        for (block, bias) in [(yes, 17), (no, -29)] {
            b.switch_to_block(block);
            let result = b.ins().iadd_imm(seed, bias);
            b.ins().return_(&[result]);
        }
        b.seal_all_blocks();
        b.finalize();
    }
    let id = module
        .declare_function(
            "conditional_immediate",
            Linkage::Local,
            &context.func.signature,
        )
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    let code = context
        .compiled_code()
        .unwrap()
        .vcode
        .as_ref()
        .unwrap()
        .clone();
    module.finalize_definitions().unwrap();
    let run = unsafe { std::mem::transmute::<_, Run>(module.get_finalized_function(id)) };
    (module, run, code)
}

#[test]
fn conditional_immediates_preserve_all_integer_conditions_and_false_edges() {
    for opt in ["none", "speed"] {
        for ty in [types::I32, types::I64] {
            for constant in [-33, -32, -31, -9, -1, 0, 1, 8, 31, 32, 33] {
                for cc in CONDITIONS {
                    for swapped in [false, true] {
                        let (_module, run, _) = compile(ty, constant, cc, swapped, opt, true);
                        for z in WORDS.into_iter().chain([
                            constant as u64,
                            (constant as u64).wrapping_sub(1),
                            (constant as u64).wrapping_add(1),
                        ]) {
                            for (x, y) in [(0, 1), (1, 0), (u64::MAX, 0), (0, u64::MAX)] {
                                let seed = z.wrapping_mul(0x1234_5678_9abc_def1);
                                let (a, b) = if swapped {
                                    (constant as u64, z)
                                } else {
                                    (z, constant as u64)
                                };
                                let taken = x < y && compare(cc, a, b, ty);
                                let expected = if taken {
                                    seed.wrapping_add(17)
                                } else {
                                    seed.wrapping_sub(29)
                                };
                                assert_eq!(
                                    unsafe { run(x, y, z, seed) },
                                    expected,
                                    "{opt} {ty} {constant} {cc:?} swapped={swapped} z={z:x} gate={x:x}<{y:x}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn conditional_immediates_emit_compare_and_add_forms_and_remain_opt_in() {
    for ty in [types::I32, types::I64] {
        for (constant, mnemonic) in [(8, "ccmp"), (-9, "ccmn")] {
            let (_new, _, code) = compile(
                ty,
                constant,
                IntCC::UnsignedLessThanOrEqual,
                false,
                "none",
                true,
            );
            let assembly = code.split(";; REGALLOC_INPUT_BEGIN").next().unwrap();
            assert!(
                assembly.lines().any(|line| line.contains(mnemonic)
                    && line.contains(&format!("#{}", constant.abs()))),
                "{code}"
            );
            let (_old, _, code) = compile(
                ty,
                constant,
                IntCC::UnsignedLessThanOrEqual,
                false,
                "none",
                false,
            );
            assert!(
                !code
                    .split(";; REGALLOC_INPUT_BEGIN")
                    .next()
                    .unwrap()
                    .contains("ccmn"),
                "{code}"
            );
        }
    }
}

#[test]
fn conditional_immediates_keep_narrow_integer_normalization() {
    for ty in [types::I8, types::I16] {
        for cc in CONDITIONS {
            let (_module, run, _) = compile(ty, -9, cc, false, "none", true);
            for z in WORDS {
                let expected = if compare(cc, z, (-9i64) as u64, ty) {
                    17u64
                } else {
                    29u64.wrapping_neg()
                };
                assert_eq!(unsafe { run(0, 1, z, 0) }, expected, "{ty} {cc:?} {z:x}");
            }
        }
    }
}
