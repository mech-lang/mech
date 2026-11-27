//! Native word-extension boundaries, also usable as a focused integration test.
use cranelift_codegen::{
    Context,
    ir::{AbiParam, InstBuilder, MemFlags, condcodes::IntCC, types},
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[derive(Clone, Copy, Debug)]
enum Form {
    Immediate,
    BinaryConstant,
    Variable,
    Signed,
    Wide,
    Parameter,
    Joined,
}

type Entry = unsafe extern "C" fn(u64, u64, *mut u64, u64) -> u64;

fn fixture(module: &JITModule, form: Form, amount: u64) -> Context {
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 4];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fc = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
    let entry = b.create_block();
    let work = b.create_block();
    let failure = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let [input, variable, output, admitted] = b.block_params(entry)[..] else {
        unreachable!()
    };
    let valid = b.ins().icmp_imm(IntCC::NotEqual, admitted, 0);
    b.ins().brif(valid, work, &[], failure, &[]);
    b.switch_to_block(failure);
    let error = b.ins().iconst(types::I64, -1);
    b.ins().return_(&[error]);
    b.switch_to_block(work);
    // The host passes an I64, then ireduce can retain dirty upper bits.
    // Do not rely on a C ABI I32 parameter to clean the input for us.
    let low = b.ins().ireduce(types::I32, input);
    let word = match form {
        Form::Immediate => b.ins().ushr_imm(low, amount as i64),
        Form::BinaryConstant => {
            let count = b.ins().iconst(types::I32, amount as i64);
            b.ins().ushr(low, count)
        }
        Form::Variable => {
            let count = b.ins().ireduce(types::I32, variable);
            b.ins().ushr(low, count)
        }
        Form::Signed => b.ins().sshr_imm(low, amount as i64),
        Form::Wide => {
            let shifted = b.ins().ushr_imm(input, amount as i64);
            b.ins().ireduce(types::I32, shifted)
        }
        Form::Parameter => low,
        Form::Joined => {
            let shifted = b.ins().ushr_imm(low, amount as i64);
            let join = b.create_block();
            b.append_block_param(join, types::I32);
            let choose = b.ins().band_imm(variable, 1);
            b.ins()
                .brif(choose, join, &[low.into()], join, &[shifted.into()]);
            b.switch_to_block(join);
            b.block_params(join)[0]
        }
    };
    let full = b.ins().uextend(types::I64, word);
    let signed = b.ins().sextend(types::I64, word);
    // The same word has narrow, signed-wide and unsigned-wide users.
    // Store width, the original wide operands and adjacent memory matter.
    b.ins().store(MemFlags::new(), word, output, 0);
    b.ins().store(MemFlags::new(), signed, output, 8);
    b.ins().store(MemFlags::new(), input, output, 16);
    b.ins().store(MemFlags::new(), variable, output, 24);
    b.ins().return_(&[full]);
    b.seal_all_blocks();
    b.finalize();
    context.set_disasm(true);
    context
}

fn expected(form: Form, amount: u64, input: u64, variable: u64) -> u32 {
    let low = input as u32;
    match form {
        Form::Immediate | Form::BinaryConstant => low.wrapping_shr(amount as u32),
        Form::Variable => low.wrapping_shr(variable as u32),
        Form::Signed => (low as i32).wrapping_shr(amount as u32) as u32,
        Form::Wide => input.wrapping_shr(amount as u32) as u32,
        Form::Parameter => low,
        Form::Joined if variable & 1 != 0 => low,
        Form::Joined => low.wrapping_shr(amount as u32),
    }
}

fn exercise(group: &str, forms: &[Form]) {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let amounts = [0, 1, 7, 8, 16, 24, 31, 32, 33, 63, 64, 65, 127, u64::MAX];
    let mut entries = Vec::new();
    let mut bodies = Vec::new();
    for &form in forms {
        for amount in amounts {
            let mut context = fixture(&module, form, amount);
            let name = format!("word_extension_{group}_{form:?}_{amount}");
            let id = module
                .declare_function(&name, Linkage::Local, &context.func.signature)
                .unwrap();
            module.define_function(id, &mut context).unwrap();
            let code = context.compiled_code().unwrap();
            bodies.push(serde_json::json!({"name":name,"form":format!("{form:?}"),"amount":amount,"body":code.buffer.data(),"vcode":code.vcode}));
            entries.push((id, form, amount));
        }
    }
    module.finalize_definitions().unwrap();
    let calls = entries
        .into_iter()
        .map(|(id, form, amount)| {
            let call: Entry = unsafe { std::mem::transmute(module.get_finalized_function(id)) };
            (call, form, amount)
        })
        .collect::<Vec<_>>();
    let mut samples = vec![
        0,
        1,
        255,
        65535,
        0x8000_0000,
        u32::MAX as u64,
        1 << 32,
        0xfeed_beef_8000_0001,
        u64::MAX,
    ];
    let mut state = 0x849d_4f65_27e1_a5c3u64;
    for _ in 0..256 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        samples.push(state);
    }
    let mut observations = 0;
    for &(call, form, amount) in &calls {
        for &input in &samples {
            for variable in [0, 1, 16, 31, 32, 33, 63, 64, 65, u64::MAX] {
                let mut memory = [0xfeed_face_dead_beef; 5];
                let word = expected(form, amount, input, variable);
                let full = unsafe { call(input, variable, memory.as_mut_ptr(), 1) };
                assert_eq!(
                    full,
                    u64::from(word),
                    "{form:?}/{amount}/{input:x}/{variable}"
                );
                assert_eq!(
                    memory,
                    [
                        0xfeed_face_0000_0000 | u64::from(word),
                        (word as i32 as i64) as u64,
                        input,
                        variable,
                        0xfeed_face_dead_beef
                    ]
                );
                let rejected = unsafe { call(input, variable, std::ptr::null_mut(), 0) };
                assert_eq!(rejected, u64::MAX);
                observations += 2;
            }
        }
    }
    if let Some(path) = std::env::var_os("MECH_SHIFT_EXTENSION_PROBE") {
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
fn fixed_word_shift_extensions_preserve_dirty_inputs_and_shared_uses() {
    exercise("fixed", &[Form::Immediate, Form::BinaryConstant]);
}

#[test]
fn other_narrow_values_keep_their_required_extensions() {
    exercise(
        "fallback",
        &[
            Form::Variable,
            Form::Signed,
            Form::Wide,
            Form::Parameter,
            Form::Joined,
        ],
    );
}
