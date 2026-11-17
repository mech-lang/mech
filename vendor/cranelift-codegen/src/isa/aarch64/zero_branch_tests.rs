use crate::Context;
use crate::cursor::{Cursor, FuncCursor};
use crate::ir::{
    AbiParam, Function, InstBuilder, Signature, Type, UserFuncName, condcodes::IntCC, types,
};
use crate::isa::{self, CallConv};
use crate::settings;
use alloc::vec::Vec;
use cranelift_control::ControlPlane;

fn branch(
    ty: Type,
    comparison: IntCC,
    limit: i64,
    tested_bit: Option<i64>,
    direct_zero_branch: bool,
) -> Vec<u32> {
    let mut signature = Signature::new(CallConv::AppleAarch64);
    signature.params.push(AbiParam::new(ty));
    signature.returns.push(AbiParam::new(types::I64));
    let mut func = Function::with_name_signature(UserFuncName::user(0, 0), signature);
    func.aarch64_branch_on_zero = direct_zero_branch;
    let mut block = || {
        let block = func.dfg.make_block();
        func.layout.append_block(block);
        block
    };
    let entry = block();
    let taken = block();
    let other = block();
    let param = func.dfg.append_block_param(entry, ty);

    let mut cursor = FuncCursor::new(&mut func);
    cursor.goto_bottom(entry);
    let tested = match tested_bit {
        Some(mask) => cursor.ins().band_imm(param, mask),
        None => param,
    };
    let value = cursor.ins().icmp_imm(comparison, tested, limit);
    cursor.ins().brif(value, taken, &[], other, &[]);
    for (block, sentinel) in [(taken, 123), (other, 731)] {
        cursor.goto_bottom(block);
        let result = cursor.ins().iconst(types::I64, sentinel);
        cursor.ins().return_(&[result]);
    }

    let target = isa::lookup("aarch64-apple-darwin".parse().unwrap())
        .unwrap()
        .finish(settings::Flags::new(settings::builder()))
        .unwrap();
    let mut context = Context::for_function(func);
    let code = context
        .compile(&*target, &mut ControlPlane::default())
        .unwrap();
    code.code_buffer()
        .chunks_exact(4)
        .map(|word| u32::from_le_bytes(word.try_into().unwrap()))
        .collect()
}

#[test]
fn zero_compare_branches_use_the_operand_width() {
    for ty in [types::I32, types::I64] {
        for cc in [
            IntCC::Equal,
            IntCC::NotEqual,
            IntCC::UnsignedLessThanOrEqual,
            IntCC::UnsignedGreaterThan,
        ] {
            let control = branch(ty, cc, 0, None, false);
            let optimized = branch(ty, cc, 0, None, true);
            let zero_branch = |word: &u32| word & 0x7e00_0000 == 0x3400_0000;
            assert!(!control.iter().any(zero_branch));
            let inst = optimized.iter().find(|word| zero_branch(word)).unwrap();
            assert_eq!(inst >> 31, u32::from(ty == types::I64));
            assert_eq!(control.len() - optimized.len(), 1);
        }
    }
}

#[test]
fn subregister_nonzero_limit_and_bit_tests_keep_their_lowering() {
    for (ty, limit, mask) in [
        (types::I8, 0, None),
        (types::I64, 3, None),
        (types::I64, 0, Some(1 << 13)),
    ] {
        assert_eq!(
            branch(ty, IntCC::Equal, limit, mask, true),
            branch(ty, IntCC::Equal, limit, mask, false),
        );
    }
}
