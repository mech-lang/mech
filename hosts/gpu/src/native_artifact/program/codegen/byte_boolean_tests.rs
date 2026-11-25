//! Native witnesses for fully clean byte Boolean definitions and their fallbacks.
use cranelift_codegen::{
    Context,
    ir::{AbiParam, InstBuilder, MemFlags, Type, condcodes::IntCC, types},
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::JITModule;
use cranelift_module::{Linkage, Module};

#[derive(Clone, Copy, Debug)]
enum Form {
    And,
    Or,
    Xor,
    Nested,
    Alias,
    SharedLeaf,
    XorOne,
    OrZero,
    Parameter,
    MixedLeft,
    MixedRight,
    Arithmetic,
    Phi,
    SignedLoad,
    Not,
    TooDeep,
    TooWide,
}

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
const SENTINEL: u64 = 0xfeed_face_dead_beef;
type Entry = unsafe extern "C" fn(u64, u64, u64, *mut u64, u64) -> u64;

fn narrow(
    b: &mut FunctionBuilder<'_>,
    value: cranelift_codegen::ir::Value,
    ty: Type,
) -> cranelift_codegen::ir::Value {
    if ty == types::I64 {
        value
    } else {
        b.ins().ireduce(ty, value)
    }
}

fn fixture(module: &JITModule, form: Form, widths: [Type; 2], cc: [IntCC; 2]) -> Context {
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 5];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fc = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
    let entry = b.create_block();
    let work = b.create_block();
    let failure = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let [a, rhs, salt, output, admitted] = b.block_params(entry)[..] else {
        unreachable!()
    };
    let valid = b.ins().icmp_imm(IntCC::NotEqual, admitted, 0);
    b.ins().brif(valid, work, &[], failure, &[]);
    b.switch_to_block(failure);
    let error = b.ins().iconst(types::I64, -1);
    b.ins().return_(&[error]);
    b.switch_to_block(work);
    let x = narrow(&mut b, a, widths[0]);
    let y = narrow(&mut b, rhs, widths[0]);
    let first = b.ins().icmp(cc[0], x, y);
    let x = narrow(&mut b, rhs, widths[1]);
    let y = narrow(&mut b, a, widths[1]);
    let second = b.ins().icmp(cc[1], x, y);
    let raw = b.ins().ireduce(types::I8, a);
    let value = match form {
        Form::And => b.ins().band(first, second),
        Form::Or | Form::SharedLeaf => b.ins().bor(first, second),
        Form::Xor => b.ins().bxor(first, second),
        Form::Nested => {
            let third = b.ins().icmp_imm(IntCC::NotEqual, salt, 0);
            let left = b.ins().bor(first, second);
            let right = b.ins().bxor(first, third);
            b.ins().band(left, right)
        }
        Form::Alias => {
            let alias = b.ins().iconst(types::I8, 0);
            let inst = b.func.dfg.value_def(alias).inst().unwrap();
            b.func.dfg.detach_inst_results(inst);
            b.func.layout.remove_inst(inst);
            b.func.dfg.change_to_alias(alias, first);
            b.ins().bor(alias, second)
        }
        Form::XorOne => {
            let root = b.ins().bor(first, second);
            let one = b.ins().iconst(types::I8, 1);
            b.ins().bxor(root, one)
        }
        Form::OrZero => {
            let root = b.ins().bor(first, second);
            let zero = b.ins().iconst(types::I8, 0);
            b.ins().bor(root, zero)
        }
        Form::Parameter => raw,
        Form::MixedLeft => b.ins().bor(raw, second),
        Form::MixedRight => b.ins().band(first, raw),
        Form::Arithmetic => {
            let other = b.ins().ireduce(types::I8, rhs);
            let sum = b.ins().iadd(raw, other);
            b.ins().bor(sum, second)
        }
        Form::Phi => {
            let other = b.ins().ireduce(types::I8, rhs);
            let join = b.create_block();
            let joined = b.append_block_param(join, types::I8);
            let choose = b.ins().band_imm(salt, 1);
            b.ins()
                .brif(choose, join, &[raw.into()], join, &[other.into()]);
            b.switch_to_block(join);
            b.ins().bor(joined, first)
        }
        Form::SignedLoad => {
            let signed = b.ins().sload8(types::I32, MemFlags::new(), output, 48);
            let byte = b.ins().ireduce(types::I8, signed);
            b.ins().bor(byte, first)
        }
        Form::Not => b.ins().bnot(first),
        Form::TooDeep => {
            let mut root = first;
            for _ in 0..40 {
                root = b.ins().bxor(root, second);
            }
            root
        }
        Form::TooWide => {
            let mut root = first;
            for _ in 0..7 {
                root = b.ins().bor(root, root);
            }
            root
        }
    };
    let full = b.ins().uextend(types::I64, value);
    let signed = b.ins().sextend(types::I64, value);
    for (at, word) in [full, signed, a, rhs, salt].into_iter().enumerate() {
        b.ins()
            .store(MemFlags::new(), word, output, (at * 8) as i32);
    }
    b.ins().store(MemFlags::new(), value, output, 40);
    if matches!(form, Form::SharedLeaf) {
        b.ins().store(MemFlags::new(), first, output, 41);
    }
    b.ins().return_(&[full]);
    b.seal_all_blocks();
    b.finalize();
    context.set_disasm(true);
    context
}

fn comparison(a: u64, b: u64, ty: Type, cc: IntCC) -> u8 {
    let shift = 64 - ty.bits();
    let (a, b) = (a & (u64::MAX >> shift), b & (u64::MAX >> shift));
    let (sa, sb) = (
        ((a << shift) as i64) >> shift,
        ((b << shift) as i64) >> shift,
    );
    u8::from(match cc {
        IntCC::Equal => a == b,
        IntCC::NotEqual => a != b,
        IntCC::SignedLessThan => sa < sb,
        IntCC::SignedLessThanOrEqual => sa <= sb,
        IntCC::SignedGreaterThan => sa > sb,
        IntCC::SignedGreaterThanOrEqual => sa >= sb,
        IntCC::UnsignedLessThan => a < b,
        IntCC::UnsignedLessThanOrEqual => a <= b,
        IntCC::UnsignedGreaterThan => a > b,
        IntCC::UnsignedGreaterThanOrEqual => a >= b,
    })
}

fn expected(form: Form, first: u8, second: u8, a: u64, rhs: u64, salt: u64) -> u8 {
    match form {
        Form::And => first & second,
        Form::Or | Form::Alias | Form::SharedLeaf | Form::OrZero => first | second,
        Form::Xor => first ^ second,
        Form::Nested => (first | second) & (first ^ u8::from(salt != 0)),
        Form::XorOne => (first | second) ^ 1,
        Form::Parameter => a as u8,
        Form::MixedLeft => a as u8 | second,
        Form::MixedRight => first & a as u8,
        Form::Arithmetic => (a as u8).wrapping_add(rhs as u8) | second,
        Form::Phi => (if salt & 1 != 0 { a } else { rhs }) as u8 | first,
        Form::SignedLoad => SENTINEL as u8 | first,
        Form::Not => !first,
        Form::TooDeep | Form::TooWide => first,
    }
}

fn exercise(group: &str, forms: &[Form]) {
    let words = [
        0,
        1,
        2,
        0x7f,
        0x80,
        0xff,
        0x100,
        0x7fff,
        0x8000,
        0xffff,
        0x7fff_ffff,
        0x8000_0000,
        u32::MAX as u64,
        1 << 63,
        0xfeed_beef_8000_0001,
        u64::MAX,
    ];
    let mut samples = words
        .into_iter()
        .flat_map(|a| words.into_iter().map(move |rhs| (a, rhs, a ^ rhs)))
        .collect::<Vec<_>>();
    let mut state = 0x849d_4f65_27e1_a5c3u64;
    for _ in 0..128 {
        let mut word = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        samples.push((word(), word(), word()));
    }
    let mut bodies = Vec::new();
    let mut observations = 0;
    for opt in ["none", "speed"] {
        let mut module = crate::native_artifact::codegen::backend::jit(&[
            ("opt_level", opt),
            ("regalloc_checker", "true"),
        ])
        .unwrap();
        let mut entries = Vec::new();
        for &form in forms {
            for widths in [
                [types::I8, types::I64],
                [types::I16, types::I32],
                [types::I32, types::I16],
                [types::I64, types::I8],
            ] {
                for (i, first) in CONDITIONS.into_iter().enumerate() {
                    let cc = [first, CONDITIONS[(i + 3) % CONDITIONS.len()]];
                    let mut context = fixture(&module, form, widths, cc);
                    let name = format!("byte_boolean_{group}_{opt}_{form:?}_{}_{i}", widths[0]);
                    let id = module
                        .declare_function(&name, Linkage::Local, &context.func.signature)
                        .unwrap();
                    module.define_function(id, &mut context).unwrap();
                    let code = context.compiled_code().unwrap();
                    bodies.push(serde_json::json!({"name":name,"form":format!("{form:?}"),"opt":opt,"body":code.buffer.data(),"vcode":code.vcode}));
                    entries.push((id, form, widths, cc));
                }
            }
        }
        module.finalize_definitions().unwrap();
        for (id, form, widths, cc) in entries {
            let call: Entry = unsafe { std::mem::transmute(module.get_finalized_function(id)) };
            for &(a, rhs, salt) in &samples {
                let first = comparison(a, rhs, widths[0], cc[0]);
                let second = comparison(rhs, a, widths[1], cc[1]);
                let value = expected(form, first, second, a, rhs, salt);
                let mut memory = [SENTINEL; 7];
                let got = unsafe { call(a, rhs, salt, memory.as_mut_ptr(), 1) };
                assert_eq!(
                    got,
                    u64::from(value),
                    "{opt}/{form:?}/{widths:?}/{cc:?}/{a:x}/{rhs:x}/{salt:x}"
                );
                let mut byte_word = SENTINEL & !255 | u64::from(value);
                if matches!(form, Form::SharedLeaf) {
                    byte_word = byte_word & !0xff00 | (u64::from(first) << 8);
                }
                assert_eq!(
                    memory,
                    [
                        u64::from(value),
                        (value as i8 as i64) as u64,
                        a,
                        rhs,
                        salt,
                        byte_word,
                        SENTINEL
                    ]
                );
                assert_eq!(
                    unsafe { call(a, rhs, salt, std::ptr::null_mut(), 0) },
                    u64::MAX
                );
                observations += 2;
            }
        }
    }
    if let Some(path) = std::env::var_os("MECH_BOOLEAN_EXTENSION_PROBE") {
        let path = std::path::PathBuf::from(path);
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(
            path.join(format!("{group}.json")),
            serde_json::to_vec(
                &serde_json::json!({"native_observations":observations,"functions":bodies}),
            )
            .unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn clean_boolean_trees_preserve_native_values_and_shared_users() {
    exercise(
        "clean",
        &[
            Form::And,
            Form::Or,
            Form::Xor,
            Form::Nested,
            Form::Alias,
            Form::SharedLeaf,
            Form::XorOne,
            Form::OrZero,
        ],
    );
}

#[test]
fn unknown_and_over_budget_byte_definitions_keep_their_extensions() {
    exercise(
        "fallback",
        &[
            Form::Parameter,
            Form::MixedLeft,
            Form::MixedRight,
            Form::Arithmetic,
            Form::Phi,
            Form::SignedLoad,
            Form::Not,
            Form::TooDeep,
            Form::TooWide,
        ],
    );
}
