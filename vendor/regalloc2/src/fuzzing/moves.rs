//! Fuzz the parallel-move resolver.

use crate::moves::{MoveAndScratchResolver, ParallelMoves};
use crate::{Allocation, PReg, RegClass, SpillSlot};
use arbitrary::{Arbitrary, Result, Unstructured};
use std::collections::{HashMap, HashSet};
use std::{vec, vec::Vec};

fn is_stack_alloc(alloc: Allocation) -> bool {
    // Treat registers 20..=29 as fixed stack slots.
    if let Some(reg) = alloc.as_reg() {
        reg.index() > 20
    } else {
        alloc.is_stack()
    }
}

///
#[derive(Clone, Debug)]
pub struct TestCase {
    moves: Vec<(Allocation, Allocation)>,
    available_pregs: Vec<Allocation>,
}

impl Arbitrary<'_> for TestCase {
    fn arbitrary(u: &mut Unstructured) -> Result<Self> {
        let mut ret = TestCase {
            moves: vec![],
            available_pregs: vec![],
        };
        let mut written = HashSet::new();
        // An arbitrary sequence of moves between registers 0 to 29
        // inclusive.
        while bool::arbitrary(u)? {
            let src = if bool::arbitrary(u)? {
                let reg = u.int_in_range(0..=29)?;
                Allocation::reg(PReg::new(reg, RegClass::Int))
            } else {
                let slot = u.int_in_range(0..=31)?;
                Allocation::stack(SpillSlot::new(slot))
            };
            let dst = if bool::arbitrary(u)? {
                let reg = u.int_in_range(0..=29)?;
                Allocation::reg(PReg::new(reg, RegClass::Int))
            } else {
                let slot = u.int_in_range(0..=31)?;
                Allocation::stack(SpillSlot::new(slot))
            };

            // Stop if we are going to write a reg more than once:
            // that creates an invalid parallel move set.
            if written.contains(&dst) {
                break;
            }
            written.insert(dst);

            ret.moves.push((src, dst));
        }

        // We might have some unallocated registers free for scratch
        // space...
        for i in 0..u.int_in_range(0..=2)? {
            let reg = PReg::new(30 + i, RegClass::Int);
            ret.available_pregs.push(Allocation::reg(reg));
        }
        Ok(ret)
    }
}

pub fn check(t: TestCase) {
    let mut par = ParallelMoves::new();
    for &(src, dst) in &t.moves {
        par.add(src, dst, ());
    }

    let moves = par.resolve();
    log::trace!("raw resolved moves: {:?}", moves);

    // Resolve uses of scratch reg and stack-to-stack moves with the scratch
    // resolver.
    let mut avail = t.available_pregs.clone();
    let find_free_reg = || avail.pop();
    let mut next_slot = 32;
    let get_stackslot = || {
        let slot = next_slot;
        next_slot += 1;
        Allocation::stack(SpillSlot::new(slot))
    };
    let preferred_victim = PReg::new(0, RegClass::Int);
    let scratch_resolver = MoveAndScratchResolver {
        find_free_reg,
        get_stackslot,
        is_stack_alloc,
        borrowed_scratch_reg: preferred_victim,
    };
    let moves = scratch_resolver.compute(moves);
    log::trace!("resolved moves: {:?}", moves);

    // Compute the final source reg for each dest reg in the original
    // parallel-move set.
    let mut final_src_per_dest: HashMap<Allocation, Allocation> = HashMap::new();
    for &(src, dst) in &t.moves {
        final_src_per_dest.insert(dst, src);
    }
    log::trace!("expected final state: {:?}", final_src_per_dest);

    // Simulate the sequence of moves.
    let mut locations: HashMap<Allocation, Allocation> = HashMap::new();
    for (src, dst, _) in moves {
        let data = locations.get(&src).cloned().unwrap_or(src);
        locations.insert(dst, data);
    }
    log::trace!("simulated final state: {:?}", locations);

    // Assert that the expected register-moves occurred.
    for (reg, data) in locations {
        if let Some(&expected_data) = final_src_per_dest.get(&reg) {
            assert_eq!(expected_data, data);
        } else {
            if data != reg {
                // If not just the original value, then this location has been
                // modified, but it was not part of the original parallel move.
                // It must have been an available preg or a scratch stackslot.
                assert!(
                    t.available_pregs.contains(&reg)
                        || (reg.is_stack() && reg.as_stack().unwrap().index() >= 32)
                );
            }
        }
    }
}

#[test]
fn smoke() {
    arbtest::arbtest(|u| {
        let test_case = TestCase::arbitrary(u)?;
        check(test_case);
        Ok(())
    })
    .budget_ms(1_000);
}
