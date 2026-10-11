//! Opt-in list scheduling with estimated operand availability.
//!
//! Each admitted SIMD pair is one scheduling unit. Independent instructions can
//! fill the estimated latency of expensive operations; the live-value bonus
//! prefers retiring temporaries when the target's register budget is crowded.
//! These costs select an order only. They never alter arithmetic or operands.

use super::{FusedProgram, Op, Operand, native};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn instruction_order(
    program: &FusedProgram,
    roots: &[Operand],
    simd: bool,
) -> Vec<usize> {
    let live = program.live_instructions(roots);
    let base = program.initial.len();
    let mut unit = (0..program.instructions.len()).collect::<Vec<_>>();
    let mut twins = BTreeSet::new();
    if simd {
        for &index in &program.simd_pairs {
            let a = &program.instructions[index];
            let b = &program.instructions[index + 1];
            if live.contains(&index)
                && live.contains(&(index + 1))
                && a.op == b.op
                && a.op != Op::Pow
                && !b
                    .inputs
                    .iter()
                    .any(|operand| matches!(operand, Operand::Register(r) if *r == base + index))
            {
                unit[index + 1] = index;
                twins.insert(index);
            }
        }
    }
    let mut values = (0..base).collect::<Vec<_>>();
    if simd {
        for group in native::simd_state_groups(program) {
            for &register in &group {
                values[register] = group[0];
            }
        }
    }
    values.extend(unit.iter().map(|index| base + index));
    let units = live
        .iter()
        .map(|index| unit[*index])
        .collect::<BTreeSet<_>>();
    let mut inputs = BTreeMap::<usize, BTreeSet<usize>>::new();
    let mut users = BTreeMap::<usize, BTreeSet<usize>>::new();
    let mut pending = BTreeMap::<usize, usize>::new();
    let mut uses = BTreeMap::<usize, usize>::new();
    for &index in &units {
        let end = index + if twins.contains(&index) { 2 } else { 1 };
        let registers = program.instructions[index..end]
            .iter()
            .flat_map(|instruction| &instruction.inputs)
            .filter_map(|operand| match operand {
                Operand::Register(register) => Some(values[*register]),
                Operand::Constant(_) => None,
            })
            .collect::<BTreeSet<_>>();
        pending.insert(index, registers.iter().filter(|r| **r >= base).count());
        for &register in &registers {
            *uses.entry(register).or_default() += 1;
            if register >= base {
                users.entry(register - base).or_default().insert(index);
            }
        }
        inputs.insert(index, registers);
    }
    for operand in roots {
        if let Operand::Register(register) = operand {
            *uses.entry(values[*register]).or_default() += 1;
        }
    }

    let latency = |index: usize| match program.instructions[index].op {
        Op::Sqrt | Op::Div => 12usize,
        Op::InverseCubeSqrt => 28,
        Op::RefinedInverseCube => 36,
        Op::Pow => 50,
        _ => 4,
    };
    let mut height = BTreeMap::<usize, usize>::new();
    for &index in units.iter().rev() {
        let downstream = users
            .get(&index)
            .into_iter()
            .flatten()
            .map(|user| height[user])
            .max()
            .unwrap_or(0);
        height.insert(index, downstream + latency(index));
    }

    let mut alive = uses
        .keys()
        .filter(|r| **r < base)
        .copied()
        .collect::<BTreeSet<_>>();
    let mut ready = pending
        .iter()
        .filter_map(|(&index, &count)| (count == 0).then_some(index))
        .collect::<BTreeSet<_>>();
    let mut available = vec![0usize; base + program.instructions.len()];
    // Two issue slots per estimated cycle. This is intentionally a modest
    // target-independent overlap model, rather than a claimed CPU simulator.
    let mut issue_slot = 0usize;
    let budget = if cfg!(target_arch = "aarch64") {
        24usize
    } else {
        12
    };
    let mut order = Vec::with_capacity(live.len());
    while !ready.is_empty() {
        let ready_at = |index: usize| {
            inputs[&index]
                .iter()
                .map(|register| available[*register])
                .max()
                .unwrap_or(0)
        };
        let first_available = ready.iter().map(|index| ready_at(*index)).min().unwrap();
        issue_slot = issue_slot.max(first_available);
        let weight = if alive.len() >= budget { 48 } else { 8 };
        let index = *ready
            .iter()
            .filter(|index| ready_at(**index) <= issue_slot)
            .max_by_key(|&&index| {
                let kills = inputs[&index].iter().filter(|r| uses[r] == 1).count();
                (height[&index] + weight * kills, std::cmp::Reverse(index))
            })
            .unwrap();
        ready.remove(&index);
        order.push(index);
        if twins.contains(&index) {
            order.push(index + 1);
        }
        available[base + index] = issue_slot + 2 * latency(index);
        issue_slot += 1;
        for register in &inputs[&index] {
            let count = uses.get_mut(register).unwrap();
            *count -= 1;
            if *count == 0 {
                alive.remove(register);
            }
        }
        if uses.get(&(base + index)).is_some_and(|count| *count > 0) {
            alive.insert(base + index);
        }
        for user in users.get(&index).into_iter().flatten() {
            let count = pending.get_mut(user).unwrap();
            *count -= 1;
            if *count == 0 {
                ready.insert(*user);
            }
        }
    }
    assert_eq!(
        order.len(),
        live.len(),
        "latency scheduling must remain acyclic"
    );
    order
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fused_f64::Instruction;

    fn fixture(instructions: Vec<Instruction>) -> FusedProgram {
        FusedProgram {
            states: Vec::new(),
            initial: vec![1.0; 4],
            inputs: Vec::new(),
            instructions,
            updates: Vec::new(),
            simd_pairs: BTreeSet::new(),
            state_shapes: Vec::new(),
            outputs: BTreeMap::new(),
            boolean_outputs: BTreeSet::new(),
            schedule: false,
            pack_state_tails: false,
            schedule_regions: 8,
            schedule_policy: 2,
        }
    }

    fn instruction(op: Op, inputs: &[usize]) -> Instruction {
        Instruction {
            op,
            inputs: inputs.iter().copied().map(Operand::Register).collect(),
        }
    }

    fn assert_dependencies(program: &FusedProgram, roots: &[Operand], order: &[usize]) {
        let mut seen = BTreeSet::new();
        for &index in order {
            for input in &program.instructions[index].inputs {
                if let Operand::Register(register) = input {
                    assert!(
                        *register < program.initial.len()
                            || seen.contains(&(*register - program.initial.len()))
                    );
                }
            }
            assert!(seen.insert(index), "duplicate instruction");
        }
        assert_eq!(seen, program.live_instructions(roots));
    }

    #[test]
    fn independent_work_fills_sqrt_latency_and_dead_work_is_excluded() {
        let program = fixture(vec![
            instruction(Op::Sqrt, &[0]),
            instruction(Op::Mul, &[1, 1]),
            instruction(Op::Add, &[5, 3]),
            instruction(Op::Div, &[4, 6]),
            instruction(Op::Neg, &[0]),
        ]);
        let roots = [Operand::Register(7)];
        let order = instruction_order(&program, &roots, false);
        assert_eq!(order, vec![0, 1, 2, 3]);
        assert_dependencies(&program, &roots, &order);
    }

    #[test]
    fn keeps_simd_pairs_adjacent_with_every_dependency_present() {
        let mut program = fixture(vec![
            instruction(Op::Sqrt, &[0]),
            instruction(Op::Sqrt, &[1]),
            instruction(Op::Mul, &[2, 3]),
            instruction(Op::Mul, &[3, 2]),
            instruction(Op::Add, &[4, 6]),
            instruction(Op::Add, &[5, 7]),
        ]);
        program.simd_pairs.extend([0, 2, 4]);
        let roots = [Operand::Register(8), Operand::Register(9)];
        for simd in [false, true] {
            let order = instruction_order(&program, &roots, simd);
            assert_dependencies(&program, &roots, &order);
            if simd {
                for &first in &program.simd_pairs {
                    let position = order.iter().position(|index| *index == first).unwrap();
                    assert_eq!(order[position + 1], first + 1);
                }
            }
        }
    }
}
