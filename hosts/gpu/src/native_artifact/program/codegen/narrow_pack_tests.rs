//! Native narrow-field packs, dirty physical inputs and shared-use fallbacks.
use cranelift_codegen::{
    Context,
    ir::{AbiParam, InstBuilder, MemFlags, Type, Value, condcodes::IntCC, types},
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[derive(Clone, Copy, Debug)]
enum Form {
    Basic,
    Swap,
    Binary,
    Shared,
    ZeroLow,
    OnesLow,
    ZeroHigh,
    Same,
    Joined,
    Loaded,
    InterveningStore,
    Signed,
    Raw,
    Variable,
    NonPack,
}

#[derive(Clone, Copy, Debug)]
enum Logical {
    Or,
    And,
    Xor,
}
const SENTINEL: u64 = 0xfeed_face_dead_beef;
type Entry = unsafe extern "C" fn(u64, u64, u64, *mut u64, u64) -> u64;

fn reduce(b: &mut FunctionBuilder<'_>, ty: Type, value: Value) -> Value {
    if ty == types::I64 {
        value
    } else {
        b.ins().ireduce(ty, value)
    }
}

fn wide(b: &mut FunctionBuilder<'_>, ty: Type, value: Value) -> Value {
    if ty == types::I64 {
        value
    } else {
        b.ins().uextend(types::I64, value)
    }
}

fn fixture(
    module: &JITModule,
    form: Form,
    op: Logical,
    ty: Type,
    narrow: Type,
    amount: u64,
    capture: bool,
) -> Context {
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
    let [a, rhs, variable, output, admitted] = b.block_params(entry)[..] else {
        unreachable!()
    };
    let valid = b.ins().icmp_imm(IntCC::NotEqual, admitted, 0);
    b.ins().brif(valid, work, &[], failure, &[]);
    b.switch_to_block(failure);
    let error = b.ins().iconst(types::I64, -1);
    b.ins().return_(&[error]);
    b.switch_to_block(work);
    let mut low = reduce(&mut b, narrow, a);
    let mut high = reduce(&mut b, narrow, rhs);
    match form {
        Form::ZeroLow => low = b.ins().iconst(narrow, 0),
        Form::OnesLow => low = b.ins().iconst(narrow, -1),
        Form::ZeroHigh => high = b.ins().iconst(narrow, 0),
        Form::Same => high = low,
        Form::Joined => {
            let join = b.create_block();
            let joined = b.append_block_param(join, narrow);
            let choose = b.ins().band_imm(variable, 1);
            b.ins()
                .brif(choose, join, &[low.into()], join, &[high.into()]);
            b.switch_to_block(join);
            low = joined;
        }
        Form::Loaded => {
            low = b.ins().load(narrow, MemFlags::new(), output, 0);
            high = b.ins().load(narrow, MemFlags::new(), output, 8);
        }
        Form::InterveningStore => {
            low = b.ins().load(narrow, MemFlags::new(), output, 0);
            b.ins().store(MemFlags::new(), rhs, output, 0);
            high = b.ins().load(narrow, MemFlags::new(), output, 0);
        }
        _ => {}
    }
    let low_wide = if matches!(form, Form::NonPack) {
        reduce(&mut b, ty, a)
    } else {
        b.ins().uextend(ty, low)
    };
    let high_wide = match form {
        Form::Signed => b.ins().sextend(ty, high),
        Form::Raw => reduce(&mut b, ty, rhs),
        _ => b.ins().uextend(ty, high),
    };
    let shifted = match form {
        Form::Binary => {
            let count = b.ins().iconst(ty, amount as i64);
            b.ins().ishl(high_wide, count)
        }
        Form::Variable => {
            let count = reduce(&mut b, ty, variable);
            b.ins().ishl(high_wide, count)
        }
        _ => b.ins().ishl_imm(high_wide, amount as i64),
    };
    let (left, right) = if matches!(form, Form::Swap) {
        (shifted, low_wide)
    } else {
        (low_wide, shifted)
    };
    let packed = match op {
        Logical::Or => b.ins().bor(left, right),
        Logical::And => b.ins().band(left, right),
        Logical::Xor => b.ins().bxor(left, right),
    };
    let full = wide(&mut b, ty, packed);
    let signed = if ty == types::I64 {
        packed
    } else {
        b.ins().sextend(types::I64, packed)
    };
    for (at, value) in [full, signed, a, rhs, variable].into_iter().enumerate() {
        b.ins()
            .store(MemFlags::new(), value, output, (at * 8) as i32);
    }
    b.ins().store(MemFlags::new(), low, output, 40);
    b.ins().store(MemFlags::new(), high, output, 44);
    let (lo, hi) = if matches!(form, Form::Shared) {
        (wide(&mut b, ty, low_wide), wide(&mut b, ty, high_wide))
    } else {
        (
            b.ins().iconst(types::I64, 0x1122),
            b.ins().iconst(types::I64, 0x3344),
        )
    };
    b.ins().store(MemFlags::new(), lo, output, 48);
    b.ins().store(MemFlags::new(), hi, output, 56);
    b.ins().return_(&[full]);
    b.seal_all_blocks();
    b.finalize();
    context.set_disasm(capture);
    context
}

fn mask(bits: u32) -> u64 {
    u64::MAX >> (64 - bits)
}

fn exercise(group: &str, forms: &[Form]) {
    let words = [
        0,
        1,
        0x7f,
        0x80,
        0xff,
        0xffff,
        0x8000_0000,
        u32::MAX as u64,
        1 << 32,
        1 << 63,
        0xfeed_beef_8000_0001,
        u64::MAX,
    ];
    let mut samples = words
        .into_iter()
        .flat_map(|a| words.into_iter().map(move |rhs| (a, rhs, a ^ rhs)))
        .collect::<Vec<_>>();
    let mut state = 0x98d5_a7ce_6e84_1632u64;
    for _ in 0..64 {
        let mut word = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        samples.push((word(), word(), word()));
    }
    let amounts = [
        0,
        1,
        7,
        8,
        15,
        16,
        23,
        24,
        31,
        32,
        33,
        47,
        48,
        55,
        56,
        63,
        64,
        65,
        u64::MAX,
    ];
    let mut observations = 0;
    let mut bodies = Vec::new();
    for opt in ["none", "speed"] {
        let mut module = JITModule::new(
            JITBuilder::with_flags(
                &[("opt_level", opt), ("regalloc_checker", "true")],
                default_libcall_names(),
            )
            .unwrap(),
        );
        let mut entries = Vec::new();
        for &form in forms {
            for (ty, narrow) in [
                (types::I32, types::I8),
                (types::I32, types::I16),
                (types::I64, types::I8),
                (types::I64, types::I16),
                (types::I64, types::I32),
            ] {
                for amount in amounts {
                    if matches!(form, Form::Variable) && amount != 0 {
                        continue;
                    }
                    for op in [Logical::Or, Logical::And, Logical::Xor] {
                        let capture = (ty == types::I32
                            && narrow == types::I16
                            && [16, 32, 33].contains(&amount))
                            || (ty == types::I64
                                && narrow == types::I32
                                && [32, 64, 65].contains(&amount));
                        let mut context = fixture(&module, form, op, ty, narrow, amount, capture);
                        let name = format!(
                            "narrow_pack_{group}_{opt}_{form:?}_{op:?}_{ty}_{narrow}_{amount}"
                        );
                        let id = module
                            .declare_function(&name, Linkage::Local, &context.func.signature)
                            .unwrap();
                        module.define_function(id, &mut context).unwrap();
                        let code = context.compiled_code().unwrap();
                        bodies.push(serde_json::json!({"name":name,"form":format!("{form:?}"),"op":format!("{op:?}"),"opt":opt,"bits":ty.bits(),"narrow_bits":narrow.bits(),"amount":amount,"body":code.buffer.data(),"vcode":code.vcode}));
                        entries.push((id, form, op, ty, narrow, amount));
                    }
                }
            }
        }
        module.finalize_definitions().unwrap();
        for (id, form, op, ty, narrow, amount) in entries {
            let call: Entry = unsafe { std::mem::transmute(module.get_finalized_function(id)) };
            for &(a, rhs, variable) in &samples {
                let (bits, n) = (ty.bits(), narrow.bits());
                let mut low = a & mask(n);
                let mut high = rhs & mask(n);
                match form {
                    Form::ZeroLow => low = 0,
                    Form::OnesLow => low = mask(n),
                    Form::ZeroHigh => high = 0,
                    Form::Same => high = low,
                    Form::Joined if variable & 1 == 0 => low = high,
                    _ => {}
                }
                let lo = if matches!(form, Form::NonPack) {
                    a & mask(bits)
                } else {
                    low
                };
                let hi = match form {
                    Form::Signed => ((((high << (64 - n)) as i64) >> (64 - n)) as u64) & mask(bits),
                    Form::Raw => rhs & mask(bits),
                    _ => high,
                };
                let shift = (if matches!(form, Form::Variable) {
                    variable
                } else {
                    amount
                }) & u64::from(bits - 1);
                let shifted = hi.wrapping_shl(shift as u32) & mask(bits);
                let expected = match op {
                    Logical::Or => lo | shifted,
                    Logical::And => lo & shifted,
                    Logical::Xor => lo ^ shifted,
                };
                let signed = if bits == 64 {
                    expected
                } else {
                    (expected as i32 as i64) as u64
                };
                let mut field = SENTINEL.to_ne_bytes();
                let bytes = (n / 8) as usize;
                field[..bytes].copy_from_slice(&low.to_ne_bytes()[..bytes]);
                field[4..4 + bytes].copy_from_slice(&high.to_ne_bytes()[..bytes]);
                let (shared_low, shared_high) = if matches!(form, Form::Shared) {
                    (lo, hi)
                } else {
                    (0x1122, 0x3344)
                };
                let mut memory = [SENTINEL; 9];
                if matches!(form, Form::Loaded | Form::InterveningStore) {
                    memory[0] = a;
                    memory[1] = rhs;
                }
                let got = unsafe { call(a, rhs, variable, memory.as_mut_ptr(), 1) };
                assert_eq!(
                    got, expected,
                    "{opt}/{form:?}/{op:?}/{ty}/{narrow}/{amount}/{a:x}/{rhs:x}/{variable:x}"
                );
                assert_eq!(
                    memory,
                    [
                        expected,
                        signed,
                        a,
                        rhs,
                        variable,
                        u64::from_ne_bytes(field),
                        shared_low,
                        shared_high,
                        SENTINEL
                    ]
                );
                assert_eq!(
                    unsafe { call(a, rhs, variable, std::ptr::null_mut(), 0) },
                    u64::MAX
                );
                observations += 2;
            }
        }
    }
    if let Some(path) = std::env::var_os("MECH_NARROW_PACK_PROBE") {
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
fn unsigned_packs_preserve_dirty_shared_and_joined_inputs() {
    exercise(
        "unsigned",
        &[
            Form::Basic,
            Form::Swap,
            Form::Binary,
            Form::Shared,
            Form::ZeroLow,
            Form::OnesLow,
            Form::ZeroHigh,
            Form::Same,
            Form::Joined,
            Form::Loaded,
            Form::InterveningStore,
        ],
    );
}

#[test]
fn other_logical_operands_keep_required_bits_and_guards() {
    exercise(
        "fallback",
        &[Form::Signed, Form::Raw, Form::Variable, Form::NonPack],
    );
}

#[derive(Clone, Copy, Debug)]
enum FieldForm {
    Basic,
    Swap,
    Binary,
    Shared,
    Joined,
    Loaded,
    InterveningStore,
    Signed,
    Variable,
    Unmasked,
    Unsigned8,
    SameUnsigned8,
    Unsigned16,
    Unsigned32,
    Loaded8,
    Masked8,
}

fn field_fixture(
    module: &JITModule,
    form: FieldForm,
    op: Logical,
    ty: Type,
    masks: (u64, u64, u64),
) -> Context {
    let (base_mask, field_mask, amount) = masks;
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
    let [a, rhs, variable, output, admitted] = b.block_params(entry)[..] else {
        unreachable!()
    };
    let valid = b.ins().icmp_imm(IntCC::NotEqual, admitted, 0);
    b.ins().brif(valid, work, &[], failure, &[]);
    b.switch_to_block(failure);
    let error = b.ins().iconst(types::I64, -1);
    b.ins().return_(&[error]);
    b.switch_to_block(work);
    let mut low = reduce(&mut b, ty, a);
    let mut high = reduce(&mut b, ty, rhs);
    let narrow = match form {
        FieldForm::Unsigned8 | FieldForm::SameUnsigned8 => Some(types::I8),
        FieldForm::Unsigned16 => Some(types::I16),
        FieldForm::Unsigned32 if ty == types::I64 => Some(types::I32),
        _ => None,
    };
    if let Some(narrow) = narrow {
        high = reduce(&mut b, narrow, rhs);
        high = b.ins().uextend(ty, high);
    }
    match form {
        FieldForm::SameUnsigned8 => low = high,
        FieldForm::Masked8 => high = b.ins().band_imm(high, 0xff),
        FieldForm::Loaded8 => {
            low = b.ins().load(ty, MemFlags::new(), output, 0);
            let byte = b.ins().load(types::I8, MemFlags::new(), output, 8);
            high = b.ins().uextend(ty, byte);
        }
        FieldForm::Joined => {
            let join = b.create_block();
            let selected = b.append_block_param(join, ty);
            let choose = b.ins().band_imm(variable, 1);
            b.ins()
                .brif(choose, join, &[low.into()], join, &[high.into()]);
            b.switch_to_block(join);
            low = selected;
        }
        FieldForm::Loaded => {
            low = b.ins().load(ty, MemFlags::new(), output, 0);
            high = b.ins().load(ty, MemFlags::new(), output, 8);
        }
        FieldForm::InterveningStore => {
            low = b.ins().load(ty, MemFlags::new(), output, 0);
            b.ins().store(MemFlags::new(), rhs, output, 0);
            high = b.ins().load(ty, MemFlags::new(), output, 0);
        }
        _ => {}
    }
    let (base, shifted, field) = if matches!(form, FieldForm::Binary) {
        let bm = b.ins().iconst(ty, base_mask as i64);
        let fm = b.ins().iconst(ty, field_mask as i64);
        let shift = b.ins().iconst(ty, amount as i64);
        let base = b.ins().band(bm, low);
        let shifted = b.ins().ushr(high, shift);
        (base, shifted, b.ins().band(fm, shifted))
    } else {
        let base = b.ins().band_imm(low, base_mask as i64);
        let shifted = match form {
            FieldForm::Signed => b.ins().sshr_imm(high, amount as i64),
            FieldForm::Variable => {
                let count = reduce(&mut b, ty, variable);
                b.ins().ushr(high, count)
            }
            _ => b.ins().ushr_imm(high, amount as i64),
        };
        let field = if matches!(form, FieldForm::Unmasked) {
            shifted
        } else {
            b.ins().band_imm(shifted, field_mask as i64)
        };
        (base, shifted, field)
    };
    let (left, right) = if matches!(form, FieldForm::Swap) {
        (field, base)
    } else {
        (base, field)
    };
    let result = match op {
        Logical::Or => b.ins().bor(left, right),
        Logical::And => b.ins().band(left, right),
        Logical::Xor => b.ins().bxor(left, right),
    };
    let full = wide(&mut b, ty, result);
    let signed = if ty == types::I64 {
        result
    } else {
        b.ins().sextend(types::I64, result)
    };
    for (at, value) in [full, signed, a, rhs, variable].into_iter().enumerate() {
        b.ins()
            .store(MemFlags::new(), value, output, (at * 8) as i32);
    }
    for (at, value) in [base, field, shifted].into_iter().enumerate() {
        let value = if matches!(form, FieldForm::Shared) {
            wide(&mut b, ty, value)
        } else {
            b.ins().iconst(types::I64, 0x1122 + at as i64)
        };
        b.ins()
            .store(MemFlags::new(), value, output, 40 + (at * 8) as i32);
    }
    b.ins().return_(&[full]);
    b.seal_all_blocks();
    b.finalize();
    context.set_disasm(true);
    context
}

#[test]
fn masked_field_inserts_preserve_values_effects_and_fallbacks() {
    let words = [
        0,
        1,
        7,
        0x80,
        0xff,
        0xffff,
        0x8000_0000,
        u32::MAX as u64,
        1 << 32,
        1 << 63,
        0xfeed_beef_8000_0001,
        u64::MAX,
    ];
    let mut samples = words
        .into_iter()
        .flat_map(|a| words.into_iter().map(move |rhs| (a, rhs, a ^ rhs)))
        .collect::<Vec<_>>();
    let mut seed = 0x87de_534a_3419_ffe3u64;
    for _ in 0..64 {
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        samples.push((next(), next(), next()));
    }
    let mut bodies = Vec::new();
    let mut observations = 0;
    for opt in ["none", "speed"] {
        let mut module = JITModule::new(
            JITBuilder::with_flags(
                &[("opt_level", opt), ("regalloc_checker", "true")],
                default_libcall_names(),
            )
            .unwrap(),
        );
        let mut entries = Vec::new();
        for ty in [types::I32, types::I64] {
            let top = 1u64 << (ty.bits() - 1);
            let specs = [
                (1, 6, 5),
                (7, 0xf8, 2),
                (0xff00, 0xff, 8),
                (0xff, 0xffff_0000, 1),
                (top, 0x1e, u64::from(ty.bits() - 1)),
                (1, top, u64::from(ty.bits() - 1)),
                (0x55, 0x300, 3),
                (3, 6, 5),
                (1, 0xa, 5),
                (1, 0, 5),
                (0, 6, 5),
                (u64::MAX, 6, 5),
                (top, 1, 0),
                (1, 6, u64::from(ty.bits()) + 5),
                (1, 6, u64::MAX),
                (0xff, top.wrapping_sub(0x100), 1),
            ];
            for form in [
                FieldForm::Basic,
                FieldForm::Swap,
                FieldForm::Binary,
                FieldForm::Shared,
                FieldForm::Joined,
                FieldForm::Loaded,
                FieldForm::InterveningStore,
                FieldForm::Signed,
                FieldForm::Variable,
                FieldForm::Unmasked,
                FieldForm::Unsigned8,
                FieldForm::SameUnsigned8,
                FieldForm::Unsigned16,
                FieldForm::Unsigned32,
                FieldForm::Loaded8,
                FieldForm::Masked8,
            ] {
                for (spec, masks) in specs.into_iter().enumerate() {
                    for op in [Logical::Or, Logical::And, Logical::Xor] {
                        let mut context = field_fixture(&module, form, op, ty, masks);
                        let name = format!("masked_field_{opt}_{form:?}_{op:?}_{ty}_{spec}");
                        let id = module
                            .declare_function(&name, Linkage::Local, &context.func.signature)
                            .unwrap();
                        module.define_function(id, &mut context).unwrap();
                        let code = context.compiled_code().unwrap();
                        bodies.push(serde_json::json!({"name":name,"opt":opt,"form":format!("{form:?}"),"op":format!("{op:?}"),"bits":ty.bits(),"spec":spec,"masks":masks,"body":code.buffer.data(),"vcode":code.vcode}));
                        entries.push((id, form, op, ty, masks));
                    }
                }
            }
        }
        module.finalize_definitions().unwrap();
        for (id, form, op, ty, (base_mask, field_mask, amount)) in entries {
            // The retained module owns this exact five-word C entry. Every
            // admitted access fits the live initialized nine-word buffer.
            let call: Entry = unsafe { std::mem::transmute(module.get_finalized_function(id)) };
            for &(a, rhs, variable) in &samples {
                let bits = ty.bits();
                let high = rhs
                    & match form {
                        FieldForm::Unsigned8
                        | FieldForm::SameUnsigned8
                        | FieldForm::Loaded8
                        | FieldForm::Masked8 => 0xff,
                        FieldForm::Unsigned16 => 0xffff,
                        FieldForm::Unsigned32 => u32::MAX as u64,
                        _ => mask(bits),
                    };
                let low = if matches!(form, FieldForm::SameUnsigned8) {
                    high
                } else if matches!(form, FieldForm::Joined) && variable & 1 == 0 {
                    rhs
                } else {
                    a
                } & mask(bits);
                let count = if matches!(form, FieldForm::Variable) {
                    variable
                } else {
                    amount
                } & u64::from(bits - 1);
                let shifted = if matches!(form, FieldForm::Signed) {
                    ((((high << (64 - bits)) as i64) >> (64 - bits)) >> count) as u64 & mask(bits)
                } else {
                    high >> count
                };
                let base = low & base_mask;
                let field = if matches!(form, FieldForm::Unmasked) {
                    shifted
                } else {
                    shifted & field_mask
                };
                let expected = match op {
                    Logical::Or => base | field,
                    Logical::And => base & field,
                    Logical::Xor => base ^ field,
                };
                let signed = if bits == 64 {
                    expected
                } else {
                    (expected as i32 as i64) as u64
                };
                let shared = if matches!(form, FieldForm::Shared) {
                    [base, field, shifted]
                } else {
                    [0x1122, 0x1123, 0x1124]
                };
                let mut memory = [SENTINEL; 9];
                if matches!(
                    form,
                    FieldForm::Loaded | FieldForm::InterveningStore | FieldForm::Loaded8
                ) {
                    memory[0] = a;
                    memory[1] = rhs;
                }
                assert_eq!(
                    unsafe { call(a, rhs, variable, memory.as_mut_ptr(), 1) },
                    expected,
                    "{opt}/{form:?}/{op:?}/{ty}/{base_mask:x}/{field_mask:x}/{amount}/{a:x}/{rhs:x}"
                );
                assert_eq!(
                    memory,
                    [
                        expected, signed, a, rhs, variable, shared[0], shared[1], shared[2],
                        SENTINEL
                    ]
                );
                assert_eq!(
                    unsafe { call(a, rhs, variable, std::ptr::null_mut(), 0) },
                    u64::MAX
                );
                observations += 2;
            }
        }
    }
    if let Some(path) = std::env::var_os("MECH_NARROW_PACK_PROBE") {
        let path = std::path::PathBuf::from(path);
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(
            path.join("masked-fields.json"),
            serde_json::to_vec(
                &serde_json::json!({"native_observations":observations,"functions":bodies}),
            )
            .unwrap(),
        )
        .unwrap();
    }
}
