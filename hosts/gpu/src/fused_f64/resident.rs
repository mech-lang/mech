//! Safe resident instruction tapes for admitted fixed-shape numeric graphs.
//! Opcode selection and liveness are resolved when loading; execution dispatches
//! standard scalar/SIMD operations without compiling executable machine code.
use super::{FusedProgram, Op, Operand};
use std::collections::{BTreeMap, BTreeSet};
use wide::f64x2;

#[derive(Clone)]
struct Instruction {
    op: Op,
    inputs: [[usize; 2]; 3],
    output: [usize; 2],
    paired: bool,
}
#[derive(Clone)]
struct Tape {
    instructions: Vec<Instruction>,
    output: Vec<usize>,
}
struct Loader<'a> {
    program: &'a FusedProgram,
    initial: Vec<f64>,
    constants: BTreeMap<u64, usize>,
}
impl Loader<'_> {
    fn operand(&mut self, value: Operand) -> usize {
        match value {
            Operand::Register(r) => r,
            Operand::Constant(value) => {
                if let Some(&r) = self.constants.get(&value.to_bits()) {
                    return r;
                }
                let r = self.initial.len();
                self.initial.push(value);
                self.constants.insert(value.to_bits(), r);
                r
            }
        }
    }
    fn tape(&mut self, roots: &[Operand], skip: &BTreeSet<usize>) -> Tape {
        let mut instructions = vec![];
        let order = self.program.instruction_order(roots, true);
        let mut cursor = 0;
        while cursor < order.len() {
            let index = order[cursor];
            cursor += 1;
            if skip.contains(&index) {
                continue;
            }
            let first = &self.program.instructions[index];
            let pair=self.program.simd_pairs.contains(&index)&&order.get(cursor)==Some(&(index+1))&&!skip.contains(&(index+1))&&first.op!=Op::Pow
                &&self.program.instructions.get(index+1).is_some_and(|other|other.op==first.op&&!other.inputs.iter().any(|v|matches!(v,Operand::Register(r) if *r==self.program.initial.len()+index)));
            let mut inputs = [[0; 2]; 3];
            for (i, &input) in first.inputs.iter().enumerate() {
                let a = self.operand(input);
                inputs[i] = [a, a];
            }
            if pair {
                for (i, &input) in self.program.instructions[index + 1]
                    .inputs
                    .iter()
                    .enumerate()
                {
                    inputs[i][1] = self.operand(input);
                }
                cursor += 1;
            }
            let r = self.program.initial.len() + index;
            instructions.push(Instruction {
                op: first.op,
                inputs,
                output: [r, r + usize::from(pair)],
                paired: pair,
            });
        }
        let output = roots.iter().map(|&r| self.operand(r)).collect();
        Tape {
            instructions,
            output,
        }
    }
}
fn scalar(op: Op, a: f64, b: f64, c: f64) -> f64 {
    match op {
        Op::Add => a + b,
        Op::Sub => a - b,
        Op::Mul => a * b,
        Op::Div => a / b,
        Op::Pow => a.powf(b),
        Op::Sqrt => a.sqrt(),
        Op::Neg => -a,
        Op::Lt => u8::from(a < b) as f64,
        Op::Le => u8::from(a <= b) as f64,
        Op::Gt => u8::from(a > b) as f64,
        Op::Ge => u8::from(a >= b) as f64,
        Op::Eq => u8::from(a == b) as f64,
        Op::Ne => u8::from(a != b) as f64,
        Op::And => u8::from(a != 0.0 && b != 0.0) as f64,
        Op::Or => u8::from(a != 0.0 || b != 0.0) as f64,
        Op::Not => u8::from(a == 0.0) as f64,
        Op::Fma => a.mul_add(b, c),
        Op::Fmsub => a.mul_add(b, -c),
        Op::Fnmadd => (-a).mul_add(b, c),
        Op::InverseCubeSqrt => a / (b * b.sqrt()),
        Op::RefinedInverseCube => {
            let inverse = 1.0 / b.sqrt();
            let square = inverse * inverse;
            let correction = 1.5 - 0.5 * b * square;
            a * (inverse * square) * correction
        }
    }
}
impl Tape {
    fn execute(&self, cells: &mut [f64]) {
        for instruction in &self.instructions {
            let [a, b, c] = instruction.inputs;
            if instruction.paired
                && matches!(
                    instruction.op,
                    Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Sqrt | Op::Neg
                )
            {
                let x = f64x2::from([cells[a[0]], cells[a[1]]]);
                let y = f64x2::from([cells[b[0]], cells[b[1]]]);
                let value: [f64; 2] = match instruction.op {
                    Op::Add => x + y,
                    Op::Sub => x - y,
                    Op::Mul => x * y,
                    Op::Div => x / y,
                    Op::Sqrt => x.sqrt(),
                    Op::Neg => -x,
                    _ => unreachable!(),
                }
                .into();
                cells[instruction.output[0]] = value[0];
                cells[instruction.output[1]] = value[1];
            } else {
                let first = scalar(instruction.op, cells[a[0]], cells[b[0]], cells[c[0]]);
                let second = instruction
                    .paired
                    .then(|| scalar(instruction.op, cells[a[1]], cells[b[1]], cells[c[1]]));
                cells[instruction.output[0]] = first;
                if let Some(second) = second {
                    cells[instruction.output[1]] = second;
                }
            }
        }
    }
}
/// Resident numeric execution uses the same admitted scalar DAG as the backends.
pub struct ResidentKernel {
    initial: Vec<f64>,
    state_len: usize,
    updates: Tape,
    outputs: BTreeMap<String, Tape>,
}
impl ResidentKernel {
    pub fn load(program: &FusedProgram) -> Self {
        let mut initial = program.initial.clone();
        initial.resize(initial.len() + program.instructions.len(), 0.0);
        let mut loader = Loader {
            program,
            initial,
            constants: BTreeMap::new(),
        };
        let updates = loader.tape(&program.updates, &BTreeSet::new());
        let outputs = program
            .outputs
            .iter()
            .map(|(name, tensor)| (name.clone(), loader.tape(&tensor.values, &BTreeSet::new())))
            .collect();
        Self {
            initial: loader.initial,
            state_len: program.initial.len(),
            updates,
            outputs,
        }
    }
    pub fn run(&self, state: &mut [f64], turns: usize) -> Result<(), String> {
        if state.len() != self.state_len {
            return Err("resident state length mismatch".into());
        }
        let mut cells = self.initial.clone();
        cells[..state.len()].copy_from_slice(state);
        // Updates have SSA outputs; commit all of them only after evaluating the
        // turn, so indexed writes and simultaneous state assignments retain order.
        let mut next = vec![0.0; self.updates.output.len()];
        for _ in 0..turns {
            self.updates.execute(&mut cells);
            for (value, &r) in next.iter_mut().zip(&self.updates.output) {
                *value = cells[r];
            }
            cells[..next.len()].copy_from_slice(&next);
        }
        state.copy_from_slice(&cells[..self.state_len]);
        Ok(())
    }
    pub fn observe(&self, state: &[f64], output: &str) -> Result<Vec<f64>, String> {
        if state.len() != self.state_len {
            return Err("resident state length mismatch".into());
        }
        let tape = self.outputs.get(output).ok_or("unknown resident output")?;
        let mut cells = self.initial.clone();
        cells[..state.len()].copy_from_slice(state);
        tape.execute(&mut cells);
        Ok(tape.output.iter().map(|&r| cells[r]).collect())
    }
}
/// Reusable reset/iterate/observe grid executor. Coordinate-independent and
/// iteration-independent expressions are separated using graph dependencies.
pub struct ResidentGrid {
    initial: Vec<f64>,
    reset: Vec<f64>,
    invariants: Tape,
    condition: Tape,
    step: Tape,
    output: Tape,
    width: usize,
    column: usize,
    row: usize,
    lanes: usize,
}
impl ResidentGrid {
    pub fn load(
        program: &FusedProgram,
        stop: &str,
        output: &str,
        width: usize,
        column: usize,
        row: usize,
    ) -> Result<Self, String> {
        if program.output_len(stop)? != 1
            || !program.output_is_boolean(stop)
            || !program.output_is_boolean(output)
        {
            return Err("resident grid requires Boolean termination and output".into());
        }
        let lanes = program.output_len(output)?;
        if !(1..=64).contains(&lanes) {
            return Err("invalid grid lanes".into());
        }
        let stop_roots = &program.outputs[stop].values;
        let output_roots = &program.outputs[output].values;
        let state = program
            .states
            .iter()
            .flat_map(|(_, offset, n)| *offset..offset + n)
            .collect::<BTreeSet<_>>();
        let mut varying = BTreeSet::new();
        for (i, operation) in program.instructions.iter().enumerate() {
            if operation.inputs.iter().any(|v|matches!(v,Operand::Register(r) if state.contains(r)||r.checked_sub(program.initial.len()).is_some_and(|j|varying.contains(&j)))){varying.insert(i);}
        }
        let mut roots = program.updates.clone();
        roots.extend(stop_roots);
        roots.extend(output_roots);
        let live = program.live_instructions(&roots);
        let invariant = live.difference(&varying).copied().collect::<BTreeSet<_>>();
        let invariant_roots = invariant
            .iter()
            .map(|&i| Operand::Register(program.initial.len() + i))
            .collect::<Vec<_>>();
        let mut initial = program.initial.clone();
        initial.resize(initial.len() + program.instructions.len(), 0.0);
        let mut loader = Loader {
            program,
            initial,
            constants: BTreeMap::new(),
        };
        let invariants = loader.tape(&invariant_roots, &BTreeSet::new());
        let condition = loader.tape(stop_roots, &invariant);
        let mut skip = invariant.clone();
        skip.extend(program.live_instructions(stop_roots));
        let step = loader.tape(&program.updates, &skip);
        let output = loader.tape(output_roots, &skip);
        Ok(Self {
            initial: loader.initial,
            reset: program.initial.clone(),
            invariants,
            condition,
            step,
            output,
            width,
            column,
            row,
            lanes,
        })
    }
    pub fn row(&self, width: usize, index: usize, output: &mut [u8]) -> Result<(), String> {
        if output.len() != width.div_ceil(8) {
            return Err("resident grid output size mismatch".into());
        }
        output.fill(0);
        let mut cells = self.initial.clone();
        let mut next = vec![0.0; self.step.output.len()];
        for x in (0..width).step_by(self.lanes) {
            cells[..self.reset.len()].copy_from_slice(&self.reset);
            cells[self.width] = width as f64;
            cells[self.row] = index as f64;
            for lane in 0..self.lanes {
                cells[self.column + lane] = (x + lane) as f64;
            }
            self.invariants.execute(&mut cells);
            loop {
                self.condition.execute(&mut cells);
                if cells[self.condition.output[0]] != 0.0 {
                    break;
                }
                self.step.execute(&mut cells);
                for (v, &r) in next.iter_mut().zip(&self.step.output) {
                    *v = cells[r];
                }
                cells[..next.len()].copy_from_slice(&next);
            }
            self.output.execute(&mut cells);
            for (lane, &r) in self.output.output.iter().enumerate() {
                let bit = x + lane;
                if bit < width && cells[r] != 0.0 {
                    output[bit / 8] |= 1 << (7 - bit % 8);
                }
            }
        }
        Ok(())
    }
}
