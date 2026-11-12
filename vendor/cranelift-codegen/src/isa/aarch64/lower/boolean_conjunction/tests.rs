use super::*;
use crate::cursor::{Cursor, FuncCursor};
use crate::ir::{Function, InstBuilder};
use alloc::string::ToString;

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

fn fixture(ty: Type, cc: IntCC, count: usize, wrappers: usize) -> (Function, Value, Value) {
    let mut f = Function::new();
    let block = f.dfg.make_block();
    f.layout.append_block(block);
    let x = f.dfg.append_block_param(block, ty);
    let y = f.dfg.append_block_param(block, ty);
    let mut cursor = FuncCursor::new(&mut f).at_bottom(block);
    let leaf = cursor.ins().icmp(cc, x, y);
    let mut root = leaf;
    for _ in 1..count {
        let next = cursor.ins().icmp(cc, x, y);
        root = cursor.ins().band(root, next);
    }
    for _ in 0..wrappers {
        root = cursor.ins().uextend(types::I64, root);
        root = cursor.ins().band_imm(root, 1);
        root = cursor.ins().ireduce(types::I8, root);
    }
    cursor.ins().return_(&[root]);
    (f, root, leaf)
}

#[test]
fn comparison_conjunction_proves_every_integer_condition_and_exact_width() {
    for ty in [types::I8, types::I16, types::I32, types::I64] {
        for cc in CONDITIONS {
            let (f, root, _) = fixture(ty, cc, 2, 0);
            let chain = derive(&f.dfg, root, |_| true).unwrap();
            let comparisons = chain.iter().collect::<alloc::vec::Vec<_>>();
            assert_eq!(comparisons.len(), 2);
            assert!(comparisons.iter().all(|c| c.cc == cc && c.ty == ty));
        }
    }
}

#[test]
fn comparison_conjunction_requires_all_consumed_wrappers_and_leaves_unique() {
    let (f, root, leaf) = fixture(types::I64, IntCC::NotEqual, 3, 1);
    assert!(derive(&f.dfg, root, |_| true).is_some());
    assert!(derive(&f.dfg, root, |v| v != leaf).is_none());
    assert!(derive(&f.dfg, root, |v| v != root).is_none());
    let mut visited = alloc::vec::Vec::new();
    derive(&f.dfg, root, |v| {
        visited.push(v);
        true
    })
    .unwrap();
    for shared in visited {
        assert!(derive(&f.dfg, root, |v| v != shared).is_none());
    }
}

#[test]
fn comparison_conjunction_declines_raw_boolean_phi_float_and_unsupported_nodes() {
    let (mut f, _, _) = fixture(types::I64, IntCC::Equal, 2, 0);
    let block = f.layout.entry_block().unwrap();
    let raw = f.dfg.append_block_param(block, types::I8);
    assert!(derive(&f.dfg, raw, |_| true).is_none());
    let float = f.dfg.append_block_param(block, types::F64);
    let end = f.layout.last_inst(block).unwrap();
    let mut cursor = FuncCursor::new(&mut f).at_inst(end);
    let float_test = cursor
        .ins()
        .fcmp(crate::ir::condcodes::FloatCC::Equal, float, float);
    let mixed = cursor.ins().band(float_test, raw);
    let masked_raw = cursor.ins().band_imm(raw, 1);
    let phi = cursor.func.dfg.make_block();
    cursor.func.layout.append_block(phi);
    let joined = cursor.func.dfg.append_block_param(phi, types::I8);
    for value in [mixed, masked_raw, float_test, joined] {
        assert!(derive(&f.dfg, value, |_| true).is_none());
    }
}

#[test]
fn comparison_conjunction_depth_leaf_and_work_bounds_decline_without_emission() {
    for count in [1, MAX_COMPARISONS + 1] {
        let (f, root, _) = fixture(types::I64, IntCC::NotEqual, count, 0);
        assert!(derive(&f.dfg, root, |_| true).is_none());
    }
    let (f, root, _) = fixture(types::I64, IntCC::NotEqual, MAX_COMPARISONS, 0);
    assert_eq!(
        derive(&f.dfg, root, |_| true).unwrap().iter().count(),
        MAX_COMPARISONS
    );
    let (f, root, _) = fixture(types::I64, IntCC::NotEqual, 2, MAX_DEPTH);
    let before = f.display().to_string();
    let mut visits = 0;
    assert!(
        derive(&f.dfg, root, |_| {
            visits += 1;
            true
        })
        .is_none()
    );
    assert!(visits <= MAX_NODES && visits <= MAX_DEPTH);
    assert_eq!(before, f.display().to_string());
}

// Independent architectural condition evaluation of all NZCV combinations.
fn condition(cc: IntCC, flags: u32) -> bool {
    let n = flags & 8 != 0;
    let z = flags & 4 != 0;
    let c = flags & 2 != 0;
    let v = flags & 1 != 0;
    match cc {
        IntCC::Equal => z,
        IntCC::NotEqual => !z,
        IntCC::UnsignedLessThan => !c,
        IntCC::UnsignedGreaterThanOrEqual => c,
        IntCC::UnsignedGreaterThan => c && !z,
        IntCC::UnsignedLessThanOrEqual => !c || z,
        IntCC::SignedLessThan => n != v,
        IntCC::SignedGreaterThanOrEqual => n == v,
        IntCC::SignedGreaterThan => !z && n == v,
        IntCC::SignedLessThanOrEqual => z || n != v,
    }
}

#[test]
fn comparison_conjunction_false_flags_keep_every_false_prefix_false() {
    for previous in CONDITIONS {
        for next in CONDITIONS {
            let false_next = false_flags(next).bits();
            assert!(!condition(next, false_next));
            for prior_flags in 0..16 {
                for next_flags in 0..16 {
                    let actual = if condition(previous, prior_flags) {
                        next_flags
                    } else {
                        false_next
                    };
                    assert_eq!(
                        condition(next, actual),
                        condition(previous, prior_flags) && condition(next, next_flags)
                    );
                }
            }
        }
    }
}
