use super::*;
use cranelift_codegen::cursor::Cursor;
use cranelift_codegen::ir::{AbiParam, InstBuilder, condcodes::IntCC};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};
use std::sync::Arc;

#[test]
fn shared_code_pages_decline_invalid_or_nonbeneficial_storage() {
    assert!(pack(&[], 6).is_none());
    assert!(pack(&[0; 16], 6).is_none());
    assert!(pack(&[0; 65537], 6).is_none());
    for shift in [0, 3, 13, 63, 255] {
        assert!(pack(&[0; 4096], shift).is_none());
    }
    let mut distinct = vec![0; 257 * 16];
    for index in 0..257usize {
        distinct[index * 16..index * 16 + 2].copy_from_slice(&(index as u16).to_le_bytes());
    }
    assert!(
        pack(&distinct, 4).is_none(),
        "a byte directory cannot name 257 pages"
    );
}

#[test]
fn shared_code_pages_native_lookup_matches_every_admitted_word() {
    type Call = unsafe extern "C" fn(*const u8, u64, u64) -> u64;
    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", "speed"), ("regalloc_checker", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let mut definitions = Vec::new();
    for (case, (size, length, high_shift, bias)) in
        [(256usize, 65536u64, 7u64, 14u8), (128, 65535, 6, 3)]
            .into_iter()
            .enumerate()
    {
        let key = super::super::CodeKey {
            bytes: (0..size).map(|i| (i % 64) as u8).collect::<Vec<_>>().into(),
            input_length: length,
            high_shift,
            bias,
        };
        let expected: Arc<[u8]> = key.encode().into();
        for shift in [
            None,
            Some(4),
            Some(5),
            Some(6),
            Some(7),
            Some(8),
            Some(9),
            Some(10),
            Some(11),
            Some(12),
        ] {
            let packed = shift.map(|shift| pack(&expected, shift).unwrap());
            let storage = packed.as_ref().map_or_else(
                || expected.to_vec().into_boxed_slice(),
                |packed| packed.bytes.clone(),
            );
            let mut context = module.make_context();
            context.func.signature.params = vec![AbiParam::new(types::I64); 3];
            context.func.signature.returns = vec![AbiParam::new(types::I64)];
            let mut fc = FunctionBuilderContext::new();
            let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
            let entry = b.create_block();
            let lookup = b.create_block();
            let reject = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let [codes, word, length] = b.block_params(entry).try_into().unwrap();
            let in_range = b.ins().icmp(IntCC::UnsignedLessThan, word, length);
            b.ins().brif(in_range, lookup, &[], reject, &[]);
            b.switch_to_block(reject);
            let status = b.ins().iconst(types::I64, -1);
            b.ins().return_(&[status]);
            b.switch_to_block(lookup);
            let zero = b.ins().iconst(types::I64, 0);
            let result = b.ins().return_(&[zero]);
            b.seal_all_blocks();
            b.finalize();
            let mut cursor = FuncCursor::new(&mut context.func).at_inst(result);
            let code = load_code(&mut cursor, codes, word, packed.as_ref());
            cursor.func.dfg.replace(result).return_(&[code]);
            context.verify(module.isa()).unwrap();
            let name = format!("shared_code_pages_{case}_{shift:?}");
            let id = module
                .declare_function(&name, Linkage::Local, &context.func.signature)
                .unwrap();
            module.define_function(id, &mut context).unwrap();
            definitions.push((id, storage, expected.clone()));
        }
    }
    module.finalize_definitions().unwrap();
    let mut compared = 0;
    for (id, storage, expected) in definitions {
        // The declared ABI is pointer, unsigned word, unsigned length -> word.
        // All table bytes remain initialized and immutable throughout the calls.
        let call: Call = unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        for (word, &code) in expected.iter().enumerate() {
            let got = unsafe { call(storage.as_ptr(), word as u64, expected.len() as u64) };
            assert_eq!(got, u64::from(code));
            compared += 1;
        }
        for word in [
            expected.len() as u64,
            expected.len() as u64 + 1,
            u32::MAX as u64,
            1 << 63,
            u64::MAX,
        ] {
            // A rejected word must not touch even a null table address.
            assert_eq!(
                unsafe { call(core::ptr::null(), word, expected.len() as u64) },
                u64::MAX
            );
            assert_eq!(unsafe { call(core::ptr::null(), word, 0) }, u64::MAX);
            compared += 2;
        }
    }
    eprintln!("shared code pages differential native calls: {compared}");
}
