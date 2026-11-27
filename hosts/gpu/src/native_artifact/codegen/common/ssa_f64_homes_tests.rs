//! Actual shared homes across loop/branch joins and raw-word helper reentry.
use super::*;
use crate::native_artifact::codegen::options::{CompilationControls, OptimizationOptions};
use cranelift_codegen::ir::{AbiParam, condcodes::IntCC};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{Linkage, Module, default_libcall_names};

extern "C" fn change(bank: *mut u64, key: u64) {
    // SAFETY: the synchronous generated caller spills two initialized u64
    // homes to this live array before calling and reloads them afterwards.
    unsafe {
        *bank = (*bank).rotate_right(11) ^ key;
        *bank.add(1) ^= 0xd39a_4c17_805e_12bf;
    }
}

type Entry = unsafe extern "C" fn(*mut u64, u64, u64) -> u64;
fn compile(floating: bool) -> (cranelift_jit::JITModule, Entry) {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 3];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut frontend = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut frontend);
        let entry = b.create_block();
        let header = b.create_block();
        let body = b.create_block();
        let left = b.create_block();
        let right = b.create_block();
        let join = b.create_block();
        let done = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let [bank, rounds, callback] = b.block_params(entry).try_into().unwrap();
        let mut variables = BTreeMap::new();
        for slot in 0..2 {
            let variable = b.declare_var(types::I64);
            let value = b
                .ins()
                .load(types::I64, MemFlags::new(), bank, (slot * 8) as i32);
            b.def_var(variable, value);
            variables.insert(slot, (Home::Variable(variable), MemFlags::new()));
        }
        let literal = b.ins().iconst(types::I64, 42);
        variables.insert(2, (Home::Literal(literal), MemFlags::new()));
        let mut words = Words {
            variables,
            literal_write: Cell::new(false),
            tracked: RefCell::new(Vec::new()),
            promote_tracked: false,
            native_extent_slots: BTreeMap::new(),
            native_extent_values: RefCell::new(BTreeMap::new()),
        };
        if floating {
            assert_eq!(
                words.keep_f64_homes(&mut b, &BTreeSet::from([0, 2])),
                vec![0]
            );
        }
        assert!(matches!(words.variables[&1].0, Home::Variable(_)));
        let counter = b.declare_var(types::I64);
        let zero = b.ins().iconst(types::I64, 0);
        b.def_var(counter, zero);
        b.ins().jump(header, &[]);
        b.switch_to_block(header);
        let index = b.use_var(counter);
        let more = b.ins().icmp(IntCC::UnsignedLessThan, index, rounds);
        b.ins().brif(more, body, &[], done, &[]);
        b.switch_to_block(body);
        let odd = b.ins().band_imm(index, 1);
        b.ins().brif(odd, right, &[], left, &[]);
        b.switch_to_block(left);
        let value = words.read(&mut b, 0);
        let value = b.ins().rotl_imm(value, 7);
        words.write(&mut b, 0, value);
        b.ins().jump(join, &[]);
        b.switch_to_block(right);
        let value = words.read(&mut b, 0);
        let value = b.ins().bxor_imm(value, 0x9e37_79b9_7f4a_7c15u64 as i64);
        words.write(&mut b, 0, value);
        b.ins().jump(join, &[]);
        b.switch_to_block(join);
        let key = b.ins().iadd_imm(index, 1);
        let integer = words.read(&mut b, 1);
        let integer = b.ins().iadd(integer, key);
        words.write(&mut b, 1, integer);
        let live = BTreeSet::from([0, 1]);
        words.spill_live(&mut b, bank, &live).unwrap();
        let mut signature = module.make_signature();
        signature.params = vec![AbiParam::new(types::I64); 2];
        let signature = b.import_signature(signature);
        b.ins().call_indirect(signature, callback, &[bank, key]);
        words.reload_live(&mut b, bank, &live).unwrap();
        b.def_var(counter, key);
        b.ins().jump(header, &[]);
        b.switch_to_block(done);
        words.spill_live(&mut b, bank, &live).unwrap();
        let value = words.read(&mut b, 0);
        assert_eq!(words.read(&mut b, 2), literal);
        b.ins().return_(&[value]);
        b.seal_all_blocks();
        b.finalize();
    }
    let floating_joins = context
        .func
        .layout
        .blocks()
        .flat_map(|block| context.func.dfg.block_params(block))
        .filter(|&&value| context.func.dfg.value_type(value) == types::F64)
        .count();
    assert_eq!(
        floating_joins > 0,
        floating,
        "real floating loop/branch parameters"
    );
    let controls = CompilationControls::new(OptimizationOptions::default());
    crate::native_artifact::program::codegen::cfg_cleanup::optimize(
        &controls,
        &mut context,
        module.isa(),
    )
    .unwrap();
    context.verify(module.isa()).unwrap();
    let id = module
        .declare_function(
            "numeric_f64_home_reentry",
            Linkage::Local,
            &context.func.signature,
        )
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    // SAFETY: the owned function uses this exact three-word native signature.
    let run =
        unsafe { core::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) };
    (module, run)
}

#[test]
fn numeric_f64_ssa_homes_keep_arbitrary_bits_at_loop_joins_and_word_helper_reentry() {
    let mut seeds = vec![
        0,
        1,
        1 << 63,
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        0x7ff0_0000_0000_1234,
        0x7ff8_0000_0000_1234,
        u64::MAX,
    ];
    let mut random = 0x1234_5678_9abc_def0u64;
    for _ in 0..64 {
        random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
        seeds.push(random);
    }
    for floating in [false, true] {
        let (_module, run) = compile(floating);
        for &seed in &seeds {
            for rounds in [0, 1, 2, 3, 17] {
                let mut bank = [seed, 0x1234_5678_9abc_def0, 0xfeed_face_dead_beef];
                let mut expected = bank;
                for i in 0..rounds {
                    expected[0] = if i % 2 == 0 {
                        expected[0].rotate_left(7)
                    } else {
                        expected[0] ^ 0x9e37_79b9_7f4a_7c15
                    };
                    expected[1] = expected[1].wrapping_add(i + 1);
                    expected[0] = expected[0].rotate_right(11) ^ (i + 1);
                    expected[1] ^= 0xd39a_4c17_805e_12bf;
                }
                // SAFETY: all accesses stay in the first two initialized cells,
                // and the retained helper is called synchronously with their pointer.
                let result = unsafe { run(bank.as_mut_ptr(), rounds, change as *const () as u64) };
                assert_eq!(
                    result, expected[0],
                    "floating={floating}, seed={seed:x}, rounds={rounds}"
                );
                assert_eq!(bank, expected);
            }
        }
    }
}
