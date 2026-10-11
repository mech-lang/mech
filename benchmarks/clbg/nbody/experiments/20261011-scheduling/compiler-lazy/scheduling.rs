//! Experimental scheduling of the existing arithmetic DAG. No operation,
//! operand order, or floating-point association is changed here.

use super::{FusedProgram, Op, Operand, native};
use std::collections::{BTreeMap, BTreeSet};

/// Work backwards from the outputs, closing one dependency branch before
/// opening another when that reduces the values crossing the schedule frontier.
/// The frontier models registers, including already selected SIMD state groups;
/// it is a scheduling estimate rather than the backend's final register count.
pub(super) fn bottom_up(program: &FusedProgram, roots: &[Operand], simd: bool) -> Vec<usize> {
    let live = program.live_instructions(roots);
    let base = program.initial.len();
    let mut unit = (0..program.instructions.len()).collect::<Vec<_>>();
    let mut twins = BTreeSet::new();
    if simd {
        for &index in &program.simd_pairs {
            let Some(second) = program.instructions.get(index + 1) else {
                continue;
            };
            let first = &program.instructions[index];
            // A dependent second lane must stay scalar. Treating it as a
            // single scheduling unit would hide a real SSA dependency.
            let independent = first.inputs.iter().chain(&second.inputs).all(|operand| {
                !matches!(operand, Operand::Register(register)
                    if *register == base + index || *register == base + index + 1)
            });
            if live.contains(&index)
                && live.contains(&(index + 1))
                && unit[index] == index
                && unit[index + 1] == index + 1
                && first.op == second.op
                && first.op != Op::Pow
                && independent
            {
                unit[index + 1] = index;
                twins.insert(index);
            }
        }
    }

    let mut values = (0..base).collect::<Vec<_>>();
    if simd {
        for chunk in native::simd_state_groups(program) {
            for &register in &chunk {
                values[register] = chunk[0];
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
    let mut depth = BTreeMap::<usize, usize>::new();
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
        let mut upstream = 0;
        for &register in &registers {
            if register >= base {
                let producer = register - base;
                users.entry(producer).or_default().insert(index);
                upstream = upstream.max(depth[&producer]);
            }
        }
        let latency = match program.instructions[index].op {
            Op::Sqrt | Op::Div => 12,
            Op::InverseCubeSqrt => 28,
            Op::RefinedInverseCube => 36,
            Op::Pow => 50,
            _ => 4,
        };
        depth.insert(index, upstream + latency);
        inputs.insert(index, registers);
    }
    let mut pending = units
        .iter()
        .map(|&index| (index, users.get(&index).map_or(0, BTreeSet::len)))
        .collect::<BTreeMap<_, _>>();
    let mut ready = pending
        .iter()
        .filter_map(|(&index, &count)| (count == 0).then_some(index))
        .collect::<BTreeSet<_>>();
    let mut frontier = roots
        .iter()
        .filter_map(|operand| match operand {
            Operand::Register(register) => Some(values[*register]),
            Operand::Constant(_) => None,
        })
        .collect::<BTreeSet<_>>();
    let mut exposed = BTreeMap::<usize, usize>::new();
    let mut reverse_order = Vec::with_capacity(units.len());
    while !ready.is_empty() {
        let index = *ready
            .iter()
            .min_by_key(|&&index| {
                let births = inputs[&index].difference(&frontier).count();
                let after =
                    frontier.len() + births - usize::from(frontier.contains(&(base + index)));
                // Under equal pressure, follow the dependency branch just
                // exposed by the previous selection. This differs from the
                // forward scheduler, which prioritizes long downstream paths
                // and can start several independent branches together.
                // Shorter upstream paths go later in the forward order when
                // pressure and branch locality tie, preserving an opportunity
                // to overlap long-latency arithmetic earlier.
                (
                    after,
                    std::cmp::Reverse(exposed.get(&index).copied().unwrap_or(0)),
                    depth[&index],
                    index,
                )
            })
            .unwrap();
        ready.remove(&index);
        reverse_order.push(index);
        frontier.remove(&(base + index));
        frontier.extend(&inputs[&index]);
        for &register in &inputs[&index] {
            if register >= base {
                let producer = register - base;
                let count = pending.get_mut(&producer).unwrap();
                *count -= 1;
                if *count == 0 {
                    ready.insert(producer);
                    exposed.insert(producer, reverse_order.len());
                }
            }
        }
    }
    assert_eq!(
        reverse_order.len(),
        units.len(),
        "reverse arithmetic scheduling must remain acyclic"
    );
    let mut order = Vec::with_capacity(live.len());
    for index in reverse_order.into_iter().rev() {
        order.push(index);
        if twins.contains(&index) {
            order.push(index + 1);
        }
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fused_f64::Instruction;

    fn program(initial: Vec<f64>, instructions: Vec<Instruction>) -> FusedProgram {
        FusedProgram {
            states: Vec::new(),
            initial,
            inputs: Vec::new(),
            instructions,
            updates: Vec::new(),
            simd_pairs: BTreeSet::new(),
            state_shapes: Vec::new(),
            outputs: BTreeMap::new(),
            boolean_outputs: BTreeSet::new(),
            schedule: false,
            pack_state_tails: false,
            schedule_regions: 0,
            schedule_policy: 0,
            lazy_simd_lanes: false,
        }
    }

    fn instruction(op: Op, inputs: &[Operand]) -> Instruction {
        Instruction {
            op,
            inputs: inputs.to_vec(),
        }
    }

    fn evaluate(program: &FusedProgram, order: &[usize], roots: &[Operand]) -> Vec<u64> {
        let base = program.initial.len();
        let mut registers = program.initial.clone();
        registers.resize(base + program.instructions.len(), f64::NAN);
        let mut emitted = BTreeSet::new();
        for &index in order {
            let instruction = &program.instructions[index];
            let inputs = instruction
                .inputs
                .iter()
                .map(|operand| match operand {
                    Operand::Constant(value) => *value,
                    Operand::Register(register) => {
                        assert!(*register < base || emitted.contains(&(*register - base)));
                        registers[*register]
                    }
                })
                .collect::<Vec<_>>();
            registers[base + index] = match instruction.op {
                Op::Add => inputs[0] + inputs[1],
                Op::Sub => inputs[0] - inputs[1],
                Op::Mul => inputs[0] * inputs[1],
                Op::Div => inputs[0] / inputs[1],
                Op::Sqrt => inputs[0].sqrt(),
                Op::Fma => inputs[0].mul_add(inputs[1], inputs[2]),
                _ => panic!("operation not used by schedule fixtures"),
            };
            assert!(emitted.insert(index));
        }
        roots
            .iter()
            .map(|operand| match operand {
                Operand::Register(register) => registers[*register].to_bits(),
                Operand::Constant(value) => value.to_bits(),
            })
            .collect()
    }

    fn peak_scalar_frontier(program: &FusedProgram, order: &[usize], roots: &[Operand]) -> usize {
        let mut frontier = roots
            .iter()
            .filter_map(|operand| match operand {
                Operand::Register(register) => Some(*register),
                Operand::Constant(_) => None,
            })
            .collect::<BTreeSet<_>>();
        let mut peak = frontier.len();
        for &index in order.iter().rev() {
            frontier.remove(&(program.initial.len() + index));
            frontier.extend(
                program.instructions[index]
                    .inputs
                    .iter()
                    .filter_map(|operand| match operand {
                        Operand::Register(register) => Some(*register),
                        Operand::Constant(_) => None,
                    }),
            );
            peak = peak.max(frontier.len());
        }
        peak
    }

    #[test]
    fn closes_reduction_branches_with_lower_peak_and_identical_arithmetic() {
        use Operand::{Constant as C, Register as R};
        let mut instructions = Vec::new();
        for lane in 0..4 {
            instructions.push(instruction(Op::Mul, &[R(0), C(lane as f64 + 1.25)]));
        }
        for lane in 0..4 {
            instructions.push(instruction(Op::Add, &[R(1 + lane), C(0.375)]));
        }
        for lane in 0..4 {
            instructions.push(instruction(Op::Div, &[R(5 + lane), C(3.75)]));
        }
        instructions.extend([
            instruction(Op::Add, &[R(9), R(10)]),
            instruction(Op::Add, &[R(11), R(12)]),
            instruction(Op::Add, &[R(13), R(14)]),
            instruction(Op::Mul, &[R(0), C(999.0)]), // Unobserved work stays dead.
        ]);
        let mut program = program(vec![1.5], instructions);
        let roots = [R(15)];
        let original = program
            .live_instructions(&roots)
            .into_iter()
            .collect::<Vec<_>>();
        let order = bottom_up(&program, &roots, false);
        assert_eq!(order.len(), 15);
        assert!(!order.contains(&15));
        assert!(
            peak_scalar_frontier(&program, &order, &roots)
                < peak_scalar_frontier(&program, &original, &roots)
        );
        for value in [1.5, -0.0, -1e100, f64::INFINITY, f64::NAN] {
            program.initial[0] = value;
            assert_eq!(
                evaluate(&program, &order, &roots),
                evaluate(&program, &original, &roots)
            );
        }
        assert_eq!(bottom_up(&program, &roots, false), order);
    }

    #[test]
    fn preserves_simd_pairs_and_shared_dependencies_but_rejects_dependent_lanes() {
        use Operand::{Constant as C, Register as R};
        let mut program = program(
            vec![2.0, 3.0],
            vec![
                instruction(Op::Mul, &[R(0), C(1.25)]),
                instruction(Op::Mul, &[R(1), C(1.25)]),
                instruction(Op::Add, &[R(2), R(3)]),
                instruction(Op::Add, &[R(4), C(2.0)]), // Cannot pack with its producer.
                instruction(Op::Fma, &[R(2), R(5), R(3)]),
            ],
        );
        program.simd_pairs.extend([0, 2]);
        let roots = [R(6), R(2), R(6)]; // Shared producer and duplicate root.
        let order = bottom_up(&program, &roots, true);
        assert!(order.windows(2).any(|pair| pair == [0, 1]));
        assert_eq!(
            evaluate(&program, &order, &roots),
            evaluate(&program, &[0, 1, 2, 3, 4], &roots)
        );
        assert!(bottom_up(&program, &[C(1.0), R(0)], true).is_empty());
    }

    #[test]
    fn existing_state_lane_groups_change_pressure_without_changing_results() {
        use Operand::{Constant as C, Register as R};
        let mut program = program(
            vec![2.0, 3.0, 4.0, 5.0],
            vec![
                instruction(Op::Mul, &[R(0), C(1.25)]),
                instruction(Op::Mul, &[R(1), C(2.5)]),
                instruction(Op::Mul, &[R(2), C(3.75)]),
                instruction(Op::Mul, &[R(3), C(5.0)]),
                instruction(Op::Add, &[R(4), R(6)]),
                instruction(Op::Add, &[R(5), R(7)]),
                instruction(Op::Add, &[R(8), R(9)]),
            ],
        );
        program.states.push((mech_core::CellSlotId::new(0), 0, 4));
        program.state_shapes.push((2, 2));
        let roots = [R(10)];
        let scalar = bottom_up(&program, &roots, false);
        let grouped = bottom_up(&program, &roots, true);
        assert_eq!(native::simd_state_groups(&program).len(), 2);
        assert_eq!(
            evaluate(&program, &scalar, &roots),
            evaluate(&program, &grouped, &roots)
        );
        assert_eq!(grouped.len(), 7);
    }
}
