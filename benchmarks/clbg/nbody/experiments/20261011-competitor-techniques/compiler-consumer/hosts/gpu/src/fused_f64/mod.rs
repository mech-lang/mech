//! Experimental fixed-shape f64 backend for the CLBG study.
//! Lowers canonical artifacts, never recognizes n-body source or body counts.
//! Pure arithmetic, comparisons, static selectors and indexed updates; other programs
//! are rejected. Resident activation supplies the authoritative initial state.
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use mech_core::snapshot::{SequenceView, ValueData};
use mech_core::{CellSlotId, DimensionExpr, FloatWidth, SchemaBody, SchemaId};
use mech_engine::{
    ArtifactSource, BindingDeclaration, ProducerReference, ProgramArtifact, SlotRole,
};

mod native;
pub use native::{JitKernel, MapKernel, GridKernel};
mod resident;
pub use resident::{ResidentKernel, ResidentGrid};
#[cfg(feature = "fused-cli")]
pub mod map_runner;
#[cfg(feature = "fused-cli")]
pub mod runner;

#[derive(Clone, Copy, Debug)]
enum Operand {
    Constant(f64),
    Register(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Sqrt,
    Neg,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    And,
    Or,
    Not,
    Fma,
    Fmsub,
    Fnmadd,
    RefinedInverseCube,
    InverseCubeSqrt,
}

#[derive(Clone, Debug)]
struct Instruction {
    op: Op,
    inputs: Vec<Operand>,
}

#[derive(Clone)]
struct Tensor {
    rows: usize,
    columns: usize,
    values: Vec<Operand>,
}

impl Tensor {
    fn at(&self, row: usize, column: usize) -> Operand {
        self.values[row + self.rows * column]
    }
    fn broadcast(&self, row: usize, column: usize) -> Operand {
        self.at(
            if self.rows == 1 { 0 } else { row },
            if self.columns == 1 { 0 } else { column },
        )
    }
    fn indices(&self, bound: usize) -> Result<Vec<usize>, String> {
        self.values
            .iter()
            .map(|operand| match operand {
                Operand::Constant(x)
                    if x.is_finite() && *x >= 1.0 && x.fract() == 0.0 && *x <= bound as f64 =>
                {
                    Ok(*x as usize - 1)
                }
                _ => Err(
                    "selector must contain constant, in-bounds, one-based integer positions".into(),
                ),
            })
            .collect()
    }
}

pub struct NativeInput {
    pub name: String,
    pub offset: usize,
    pub rows: usize,
    pub columns: usize,
}

pub struct FusedProgram {
    pub states: Vec<(CellSlotId, usize, usize)>,
    pub initial: Vec<f64>,
    pub inputs: Vec<NativeInput>,
    instructions: Vec<Instruction>,
    updates: Vec<Operand>,
    simd_pairs: BTreeSet<usize>,
    state_shapes: Vec<(usize, usize)>,
    outputs: BTreeMap<String, Tensor>,
    boolean_outputs: BTreeSet<String>,
    schedule: bool,
    pack_state_tails: bool,
    consumer_simd: bool,
}

impl FusedProgram {
    /// Permit FMA in state-dependent recurrence arithmetic, preserving the
    /// rounding of immutable input expressions such as coordinate mapping.
    pub fn contract_loop_fma(&mut self) {
        let state_registers = self
            .states
            .iter()
            .flat_map(|(_, offset, length)| *offset..*offset + *length)
            .collect::<BTreeSet<_>>();
        let mut dependent = BTreeSet::new();
        for (index, instruction) in self.instructions.iter().enumerate() {
            if instruction.inputs.iter().any(|operand| match operand {
                Operand::Register(register) if *register < self.initial.len() => {
                    state_registers.contains(register)
                }
                Operand::Register(register) => dependent.contains(&(register - self.initial.len())),
                Operand::Constant(_) => false,
            }) {
                dependent.insert(index);
            }
        }
        let original = self.instructions.clone();
        self.contract_fma();
        for (index, instruction) in self.instructions.iter_mut().enumerate() {
            // Do not absorb a previously rounded immutable product into a
            // state addition. Its inputs would bypass the preserved product.
            let immutable_product = original[index].inputs.iter().any(|operand| {
                if let Operand::Register(register) = operand
                    && *register >= self.initial.len()
                {
                    let producer = register - self.initial.len();
                    original[producer].op == Op::Mul && !dependent.contains(&producer)
                } else {
                    false
                }
            });
            if !dependent.contains(&index) || immutable_product {
                *instruction = original[index].clone();
            }
        }
    }

    /// Opt-in algebraic reduction of x^-1.5, including a surrounding multiply,
    /// to numerator / (x * sqrt(x)). Changes rounding and intermediate range.
    pub fn lower_algebraic_powers(&mut self) {
        for instruction in &mut self.instructions {
            if instruction.op == Op::Pow
                && matches!(instruction.inputs[1], Operand::Constant(x) if x == -1.5)
            {
                instruction.op = Op::InverseCubeSqrt;
                instruction.inputs = vec![Operand::Constant(1.0), instruction.inputs[0]];
            }
        }
        let original = self.instructions.clone();
        for instruction in &mut self.instructions {
            if instruction.op != Op::Mul {
                continue;
            }
            for (scale, inverse) in [
                (instruction.inputs[0], instruction.inputs[1]),
                (instruction.inputs[1], instruction.inputs[0]),
            ] {
                if let Operand::Register(register) = inverse
                    && register >= self.initial.len()
                {
                    let inverse = &original[register - self.initial.len()];
                    if inverse.op == Op::InverseCubeSqrt
                        && matches!(inverse.inputs[0], Operand::Constant(x) if x == 1.0)
                    {
                        instruction.op = Op::InverseCubeSqrt;
                        instruction.inputs = vec![scale, inverse.inputs[1]];
                        break;
                    }
                }
            }
        }
    }
    /// Schedule pure arithmetic by demand while keeping SIMD pairs together.
    /// This changes only the order of independent instructions, not rounding.
    pub fn schedule_dependencies(&mut self) {
        self.schedule = true;
    }

    /// Pack leftover scalar lanes from odd-width state rows together. The
    /// external state layout and the arithmetic graph remain unchanged.
    pub fn pack_state_tails(&mut self) {
        self.pack_state_tails = true;
    }

    /// Keep SIMD pairs demanded together by state results or other packed
    /// arithmetic. Scalar broadcasts do not force their producers into
    /// vectors. This avoids packing values merely because they are adjacent.
    pub fn select_consumer_simd(&mut self) {
        self.consumer_simd = true;
    }

    fn consumer_simd_pairs(&self, roots: &[Operand]) -> BTreeSet<usize> {
        let base = self.initial.len();
        let roots = roots.iter().filter_map(|operand| match operand {
            Operand::Register(index) => Some(*index),
            Operand::Constant(_) => None,
        }).collect::<BTreeSet<_>>();
        let mut pending = self.simd_pairs.iter().copied().filter(|index| {
            roots.contains(&(base + index)) && roots.contains(&(base + index + 1))
        }).collect::<Vec<_>>();
        let mut selected = BTreeSet::new();
        while let Some(index) = pending.pop() {
            if self.instructions[index].op == Op::Pow || !selected.insert(index) {
                continue;
            }
            for (&a,&b) in self.instructions[index].inputs.iter().zip(&self.instructions[index+1].inputs) {
                if let (Operand::Register(a),Operand::Register(b)) = (a,b)
                    && a != b && a >= base && b >= base
                {
                    let first = a.min(b);
                    if a.max(b) == first + 1 && self.simd_pairs.contains(&(first-base)) {
                        pending.push(first-base);
                    }
                }
            }
        }
        selected
    }

    /// Canonicalize constant signs feeding fused products. Negative products
    /// keep their original value for all other consumers and named outputs;
    /// FMA consumers absorb the sign instead, sharing positive coefficients.
    /// This uses the finite-arithmetic assumptions of FMA contraction.
    pub fn canonicalize_product_signs(&mut self) {
        let original = std::mem::take(&mut self.instructions);
        let pairs = std::mem::take(&mut self.simd_pairs);
        let base = self.initial.len();
        let eligible = original.iter()
            .filter(|instruction| matches!(instruction.op, Op::Fma | Op::Fnmadd))
            .flat_map(|instruction| &instruction.inputs[..2])
            .filter_map(|operand| match operand {
                Operand::Register(index) if *index >= base => Some(*index - base),
                _ => None,
            }).collect::<BTreeSet<_>>();
        let mut mapped = (0..base).map(Operand::Register).collect::<Vec<_>>();
        mapped.resize(base + original.len(), Operand::Constant(0.0));
        let mut positive = BTreeMap::<usize, Operand>::new();
        let remap = |operand: Operand, mapped: &[Operand]| match operand {
            Operand::Register(index) => mapped[index],
            constant => constant,
        };
        let append = |program: &mut Self, instruction: Instruction| {
            let output = Operand::Register(base + program.instructions.len());
            program.instructions.push(instruction);
            output
        };
        let mut index = 0;
        while index < original.len() {
            let count = if pairs.contains(&index) { 2 } else { 1 };
            let mut outputs = Vec::new();
            let mut negative = Vec::new();
            for (lane, instruction) in original[index..index+count].iter().enumerate() {
                let mut op = instruction.op;
                let mut inputs = instruction.inputs.iter().map(|operand| remap(*operand,&mapped)).collect::<Vec<_>>();
                let mut negated = false;
                if op == Op::Mul && eligible.contains(&(index+lane)) {
                    for operand in &mut inputs {
                        if let Operand::Constant(coefficient) = operand
                            && coefficient.is_finite() && coefficient.is_sign_negative()
                        {
                            *coefficient = coefficient.abs();
                            negated = !negated;
                        }
                    }
                }
                if matches!(op, Op::Fma | Op::Fnmadd) {
                    let mut flip = false;
                    for slot in 0..2 {
                        if let Operand::Register(register) = instruction.inputs[slot]
                            && let Some(value) = positive.get(&register)
                        {
                            inputs[slot] = *value;
                            flip = !flip;
                        }
                    }
                    if flip { op = if op == Op::Fma { Op::Fnmadd } else { Op::Fma }; }
                }
                outputs.push(append(self,Instruction{op,inputs}));
                negative.push(negated);
            }
            if count == 2 { self.mark_simd_pairs(&outputs); }
            let mut final_outputs = Vec::new();
            for (lane,(&output,&negated)) in outputs.iter().zip(&negative).enumerate() {
                let register = base + index + lane;
                let value = if negated {
                    positive.insert(register,output);
                    append(self,Instruction{op:Op::Neg,inputs:vec![output]})
                } else { output };
                mapped[register] = value;
                final_outputs.push(value);
            }
            if count == 2 && negative.iter().all(|value| *value) {
                self.mark_simd_pairs(&final_outputs);
            }
            index += count;
        }
        for operand in &mut self.updates { *operand = remap(*operand,&mapped); }
        for tensor in self.outputs.values_mut() {
            for operand in &mut tensor.values { *operand = remap(*operand,&mapped); }
        }
    }

    /// Elide the positive-zero seed of a squared-value reduction. The square
    /// cannot produce negative zero; all subsequent summands keep their order.
    pub fn simplify_square_seeds(&mut self) {
        for instruction in &mut self.instructions {
            if instruction.op == Op::Fma
                && matches!(instruction.inputs[2], Operand::Constant(x) if x.to_bits() == 0)
                && match (instruction.inputs[0], instruction.inputs[1]) {
                    (Operand::Register(a), Operand::Register(b)) => a == b,
                    (Operand::Constant(a), Operand::Constant(b)) => a.to_bits() == b.to_bits(),
                    _ => false,
                }
            {
                instruction.op = Op::Mul;
                instruction.inputs.truncate(2);
            }
        }
    }

    /// Explicitly permit reassociation of a fused constant-scaled product.
    /// Move its constant multiplication to the earlier-ready operand, allowing
    /// it to overlap an expensive independent dependency. This changes rounding
    /// and requires the same finite-value assumptions as fast-math arithmetic.
    pub fn reassociate_constant_products(&mut self) {
        let original = std::mem::take(&mut self.instructions);
        let pairs = std::mem::take(&mut self.simd_pairs);
        let base = self.initial.len();
        let mut mapped = (0..base).map(Operand::Register).collect::<Vec<_>>();
        mapped.resize(base + original.len(), Operand::Constant(0.0));
        let mut ready = vec![0usize; base + original.len()];
        let delay = |operand: Operand, ready: &[usize]| match operand {
            Operand::Register(index) => ready[index],
            Operand::Constant(_) => 0,
        };
        // Relative dependency costs guide a graph decision, independent of
        // tensor dimensions, source names, or benchmark input values.
        for (index, instruction) in original.iter().enumerate() {
            let cost = match instruction.op {
                Op::Sqrt | Op::Div => 12,
                Op::InverseCubeSqrt => 28,
                Op::RefinedInverseCube => 36,
                Op::Pow => 50,
                _ => 4,
            };
            ready[base + index] = instruction
                .inputs
                .iter()
                .map(|operand| delay(*operand, &ready))
                .max()
                .unwrap_or(0)
                + cost;
        }
        let remap = |operand: Operand, mapped: &[Operand]| match operand {
            Operand::Register(index) => mapped[index],
            constant => constant,
        };
        let append = |program: &mut Self, instruction: Instruction| {
            let output = Operand::Register(base + program.instructions.len());
            program.instructions.push(instruction);
            output
        };
        let mut index = 0;
        while index < original.len() {
            let count = if pairs.contains(&index) && index + 1 < original.len() {
                2
            } else {
                1
            };
            let mut plans = Vec::new();
            for instruction in &original[index..index + count] {
                let mut plan = None;
                if matches!(instruction.op, Op::Fma | Op::Fmsub | Op::Fnmadd) {
                    for constant_slot in 0..2 {
                        let Operand::Constant(coefficient) = instruction.inputs[constant_slot]
                        else {
                            continue;
                        };
                        let Operand::Register(product) = instruction.inputs[1 - constant_slot]
                        else {
                            continue;
                        };
                        if !coefficient.is_finite() || product < base {
                            continue;
                        }
                        let product = &original[product - base];
                        if product.op != Op::Mul
                            || !product
                                .inputs
                                .iter()
                                .all(|x| matches!(x, Operand::Register(_)))
                        {
                            continue;
                        }
                        let a = product.inputs[0];
                        let b = product.inputs[1];
                        let (early, late) = if delay(a, &ready) <= delay(b, &ready) {
                            (a, b)
                        } else {
                            (b, a)
                        };
                        if delay(late, &ready) < delay(early, &ready) + 4 {
                            continue;
                        }
                        let scaled = append(
                            self,
                            Instruction {
                                op: Op::Mul,
                                inputs: vec![remap(early, &mapped), Operand::Constant(coefficient)],
                            },
                        );
                        plan = Some((constant_slot, scaled, late));
                        break;
                    }
                }
                plans.push(plan);
            }
            if count == 2 && plans.iter().all(Option::is_some) {
                self.mark_simd_pairs(&plans.iter().map(|x| x.unwrap().1).collect::<Vec<_>>());
            }
            let mut outputs = Vec::new();
            for (lane, instruction) in original[index..index + count].iter().enumerate() {
                let mut inputs = instruction
                    .inputs
                    .iter()
                    .map(|x| remap(*x, &mapped))
                    .collect::<Vec<_>>();
                if let Some((constant_slot, scaled, late)) = plans[lane] {
                    inputs[constant_slot] = remap(late, &mapped);
                    inputs[1 - constant_slot] = scaled;
                }
                let output = append(
                    self,
                    Instruction {
                        op: instruction.op,
                        inputs,
                    },
                );
                mapped[base + index + lane] = output;
                outputs.push(output);
            }
            if count == 2 {
                self.mark_simd_pairs(&outputs);
            }
            index += count;
        }
        for operand in &mut self.updates {
            *operand = remap(*operand, &mapped);
        }
        for tensor in self.outputs.values_mut() {
            for operand in &mut tensor.values {
                *operand = remap(*operand, &mapped);
            }
        }
    }

    fn instruction_order(&self, roots: &[Operand], simd: bool) -> Vec<usize> {
        let live = self.live_instructions(roots);
        if !self.schedule {
            return live.into_iter().collect();
        }
        let mut unit = (0..self.instructions.len()).collect::<Vec<_>>();
        let mut twins = BTreeSet::new();
        if simd {
            for &index in &self.simd_pairs {
                let a = &self.instructions[index];
                let b = &self.instructions[index + 1];
                if live.contains(&index)
                    && live.contains(&(index + 1))
                    && a.op == b.op
                    && a.op != Op::Pow
                {
                    unit[index + 1] = index;
                    twins.insert(index);
                }
            }
        }
        let mut pending = roots
            .iter()
            .rev()
            .filter_map(|operand| match operand {
                Operand::Register(register) if *register >= self.initial.len() => {
                    Some((unit[register - self.initial.len()], false))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut visited = BTreeSet::new();
        let mut order = Vec::new();
        while let Some((index, ready)) = pending.pop() {
            if ready {
                order.push(index);
                if twins.contains(&index) {
                    order.push(index + 1);
                }
            } else if visited.insert(index) {
                pending.push((index, true));
                let end = index + if twins.contains(&index) { 2 } else { 1 };
                for instruction in self.instructions[index..end].iter().rev() {
                    for operand in instruction.inputs.iter().rev() {
                        if let Operand::Register(register) = operand
                            && *register >= self.initial.len()
                        {
                            pending.push((unit[register - self.initial.len()], false));
                        }
                    }
                }
            }
        }
        order
    }

    /// Explicitly permit FMA contraction. This changes rounding and is never
    /// applied by the default, arithmetic-order-preserving compilation path.
    pub fn contract_fma(&mut self) {
        let original = self.instructions.clone();
        let multiply = |operand| match operand {
            Operand::Register(index) if index >= self.initial.len() => {
                let instruction = &original[index - self.initial.len()];
                (instruction.op == Op::Mul).then(|| instruction.inputs.clone())
            }
            _ => None,
        };
        // Absorb a finite negative coefficient into the fused operation's
        // sign. Positive/negative updates can then share the same coefficient
        // register instead of keeping two vector constants live.
        let positive_coefficient = |inputs: &mut Vec<Operand>| {
            for input in inputs {
                if let Operand::Constant(coefficient) = input
                    && coefficient.is_finite()
                    && coefficient.is_sign_negative()
                {
                    *coefficient = coefficient.abs();
                    return true;
                }
            }
            false
        };
        for instruction in &mut self.instructions {
            if !matches!(instruction.op, Op::Add | Op::Sub) {
                continue;
            }
            let a = instruction.inputs[0];
            let b = instruction.inputs[1];
            if let Some(mut inputs) = multiply(b) {
                let negative = positive_coefficient(&mut inputs);
                inputs.push(a);
                instruction.op = if (instruction.op == Op::Add) != negative {
                    Op::Fma
                } else {
                    Op::Fnmadd
                };
                instruction.inputs = inputs;
            } else if let Some(mut inputs) = multiply(a) {
                let negative = instruction.op == Op::Add && positive_coefficient(&mut inputs);
                inputs.push(b);
                instruction.op = if instruction.op == Op::Add {
                    if negative { Op::Fnmadd } else { Op::Fma }
                } else {
                    Op::Fmsub
                };
                instruction.inputs = inputs;
            }
        }
    }

    /// Opt-in mixed-precision reciprocal-square-root refinement for algebraic
    /// inverse-cube expressions. Requires positive inputs within f32's normal
    /// range, like other fast-math assumptions. Changes arithmetic and rounding.
    pub fn refine_reciprocal(&mut self) {
        let original = self.instructions.clone();
        let instruction_for = |operand| match operand {
            Operand::Register(index) if index >= self.initial.len() => {
                Some(&original[index - self.initial.len()])
            }
            _ => None,
        };
        let equal = |a: Operand, b: Operand| match (a, b) {
            (Operand::Register(a), Operand::Register(b)) => a == b,
            (Operand::Constant(a), Operand::Constant(b)) => a.to_bits() == b.to_bits(),
            _ => false,
        };
        for instruction in &mut self.instructions {
            if instruction.op == Op::Pow
                && matches!(instruction.inputs[1], Operand::Constant(x) if x == -1.5)
            {
                instruction.op = Op::RefinedInverseCube;
                instruction.inputs = vec![Operand::Constant(1.0), instruction.inputs[0]];
            } else if instruction.op == Op::Div
                && let Some(product) = instruction_for(instruction.inputs[1])
                && product.op == Op::Mul
            {
                for (q, root) in [
                    (product.inputs[0], product.inputs[1]),
                    (product.inputs[1], product.inputs[0]),
                ] {
                    if let Some(root) = instruction_for(root)
                        && root.op == Op::Sqrt
                        && equal(root.inputs[0], q)
                    {
                        instruction.op = Op::RefinedInverseCube;
                        instruction.inputs = vec![instruction.inputs[0], q];
                        break;
                    }
                }
            }
        }
    }
    /// Emit a native object from the same Cranelift IR used by the JIT.
    pub fn aot_object(&self) -> Result<Vec<u8>, String> {
        native::compile_object(self, false)
    }

    pub fn aot_object_simd(&self) -> Result<Vec<u8>, String> {
        native::compile_object(self, true)
    }

    pub fn aot_object_with_output(&self, output: &str, simd: bool) -> Result<Vec<u8>, String> {
        native::compile_object_with_output(self, simd, Some(output))
    }

    pub fn output_len(&self, name: &str) -> Result<usize, String> {
        Ok(self
            .outputs
            .get(name)
            .ok_or_else(|| format!("unknown or unsupported f64 output: {name}"))?
            .values
            .len())
    }

    pub fn output_is_boolean(&self, name: &str) -> bool {
        self.boolean_outputs.contains(name)
    }

    pub fn set_input(&self, state: &mut [f64], name: &str, values: &[f64]) -> Result<(), String> {
        if state.len() != self.initial.len() {
            return Err("native frame length mismatch".into());
        }
        let input = self
            .inputs
            .iter()
            .find(|input| input.name == name)
            .ok_or("unknown native input")?;
        if values.len() != input.rows * input.columns {
            return Err("native input shape mismatch".into());
        }
        state[input.offset..input.offset + values.len()].copy_from_slice(values);
        Ok(())
    }

    /// Emit a reset/iterate/observe function for a generic pure map operation.
    /// The source's scalar Boolean output controls termination.
    pub fn aot_map_object(&self, stop: &str, output: &str, simd: bool) -> Result<Vec<u8>, String> {
        native::compile_map_object(self, stop, output, simd, false)
    }

    /// Emit a Boolean map that returns output lane zero in the highest used bit.
    pub fn aot_bitmap_map_object(
        &self,
        stop: &str,
        output: &str,
        simd: bool,
    ) -> Result<Vec<u8>, String> {
        native::compile_map_object(self, stop, output, simd, true)
    }

    /// Source-defined grid traversal around a compiled Boolean recurrence.
    pub fn aot_grid_object(
        &self,
        stop: &str,
        output: &str,
        simd: bool,
        width: usize,
        column: usize,
        row: usize,
    ) -> Result<Vec<u8>, String> {
        native::compile_grid_object(self, stop, output, simd, width, column, row)
    }

    fn live_instructions(&self, roots: &[Operand]) -> BTreeSet<usize> {
        let mut live = BTreeSet::new();
        let mut pending = roots.to_vec();
        while let Some(operand) = pending.pop() {
            if let Operand::Register(register) = operand
                && register >= self.initial.len()
            {
                let index = register - self.initial.len();
                if live.insert(index) {
                    pending.extend(&self.instructions[index].inputs);
                }
            }
        }
        live
    }

    pub fn lower(
        artifact: &ProgramArtifact,
        initial: &BTreeMap<CellSlotId, Vec<f64>>,
    ) -> Result<Self, String> {
        if !artifact.constraints().is_empty() || !artifact.requirements().is_empty() {
            return Err(
                "fused experiment requires a pure program without integrity constraints".into(),
            );
        }
        let mut program = Self {
            states: Vec::new(),
            initial: Vec::new(),
            inputs: Vec::new(),
            instructions: Vec::new(),
            updates: Vec::new(),
            simd_pairs: BTreeSet::new(),
            state_shapes: Vec::new(),
            outputs: BTreeMap::new(),
            boolean_outputs: BTreeSet::new(),
            schedule: false,
            pack_state_tails: false,
            consumer_simd: false,
        };
        let mut slots = BTreeMap::new();
        for slot in artifact
            .slots()
            .iter()
            .filter(|slot| slot.role == SlotRole::State)
        {
            if boolean_schema(artifact, slot.schema) {
                return Err("native state must be f64".into());
            }
            let (rows, columns) = shape(artifact, slot.schema)?;
            program.state_shapes.push((rows, columns));
            let values = initial
                .get(&slot.slot)
                .ok_or("missing authoritative initial state")?;
            if values.len() != rows * columns {
                return Err("state shape mismatch".into());
            }
            if program
                .initial
                .len()
                .checked_add(values.len())
                .is_none_or(|length| length > i32::MAX as usize / 8)
            {
                return Err("state exceeds native constant-offset range".into());
            }
            let offset = program.initial.len();
            program.states.push((slot.slot, offset, values.len()));
            program.initial.extend(values);
            slots.insert(
                slot.slot,
                Tensor {
                    rows,
                    columns,
                    values: (offset..offset + values.len())
                        .map(Operand::Register)
                        .collect(),
                },
            );
        }
        for input in artifact.inputs() {
            if boolean_schema(artifact, input.schema) {
                return Err("native inputs must be f64".into());
            }
            let (rows, columns) = shape(artifact, input.schema)?;
            let offset = program.initial.len();
            if offset
                .checked_add(rows * columns)
                .is_none_or(|length| length > i32::MAX as usize / 8)
            {
                return Err("input frame exceeds native constant-offset range".into());
            }
            program.initial.resize(offset + rows * columns, 0.0);
            program.inputs.push(NativeInput {
                name: mech_engine::decode_source_input_name(&input.name)
                    .unwrap_or_else(|| input.name.clone()),
                offset,
                rows,
                columns,
            });
            slots.insert(
                input.slot,
                Tensor {
                    rows,
                    columns,
                    values: (offset..offset + rows * columns)
                        .map(Operand::Register)
                        .collect(),
                },
            );
        }
        let constants = |source: ArtifactSource,
                         slots: &BTreeMap<CellSlotId, Tensor>|
         -> Result<Tensor, String> {
            match source {
                ArtifactSource::Slot(slot) => slots
                    .get(&slot)
                    .cloned()
                    .ok_or_else(|| format!("slot {} is unavailable", slot.get())),
                ArtifactSource::Constant(id) => {
                    let value = artifact.constants().get(id).ok_or("missing constant")?;
                    let (rows, columns) = shape(artifact, value.schema())?;
                    let row_major = match value.data() {
                        ValueData::F64(x) => vec![x.to_f64()],
                        ValueData::Bool(x) => vec![f64::from(*x)],
                        ValueData::Matrix(matrix) => match matrix.elements() {
                            SequenceView::F64(xs) => xs.iter().map(|x| x.to_f64()).collect(),
                            SequenceView::Bool(xs) => xs.iter().copied().map(f64::from).collect(),
                            _ => return Err("only f64 constants are admitted".into()),
                        },
                        _ => return Err("only f64 constants are admitted".into()),
                    };
                    let mut values = Vec::with_capacity(rows * columns);
                    for column in 0..columns {
                        for row in 0..rows {
                            values.push(Operand::Constant(row_major[row * columns + column]));
                        }
                    }
                    Ok(Tensor {
                        rows,
                        columns,
                        values,
                    })
                }
            }
        };
        let mut updates = BTreeMap::new();
        for declaration in artifact.nodes() {
            let node = declaration
                .as_operation()
                .ok_or("control nodes require resident execution")?;
            let name = node.operation.canonical_name();
            let inputs = node
                .input_bindings
                .clone()
                .map(|index| match artifact.bindings()[index as usize] {
                    BindingDeclaration::Input { source, .. } => constants(source, &slots),
                    _ => Err("invalid input binding".into()),
                })
                .collect::<Result<Vec<_>, _>>()?;
            let outputs = node
                .output_bindings
                .clone()
                .map(|index| match artifact.bindings()[index as usize] {
                    BindingDeclaration::Output { target, .. } => Ok(target),
                    _ => Err("invalid output binding".into()),
                })
                .collect::<Result<Vec<_>, String>>()?;
            if outputs.len() != 1 {
                return Err(format!("{name}: expected one output"));
            }
            let target = outputs[0];
            let result = program
                .operation(&name, &inputs)
                .map_err(|error| format!("node {} {name}: {error}", node.node.get()))?;
            if artifact.slots()[target.get() as usize].role == SlotRole::State {
                if name != "core/assign" {
                    return Err("state writes must be whole-value commits".into());
                }
                if updates.insert(target, result).is_some() {
                    return Err("multiple state commits".into());
                }
            } else {
                slots.insert(target, result);
            }
        }
        for (slot, _, len) in &program.states {
            let update = updates.remove(slot).ok_or("state has no next value")?;
            if update.values.len() != *len {
                return Err("state update shape mismatch".into());
            }
            program.updates.extend(update.values);
        }
        for input in &program.inputs {
            program.updates.extend(
                (input.offset..input.offset + input.rows * input.columns).map(Operand::Register),
            );
        }
        for output in artifact.outputs() {
            let source = output
                .interactive_binding
                .as_ref()
                .map(|binding| binding.artifact_source)
                .unwrap_or_else(
                    || match artifact.slots()[output.source.get() as usize].producer {
                        ProducerReference::Output { source, .. } => source,
                        _ => ArtifactSource::Slot(output.source),
                    },
                );
            if let Ok(tensor) = constants(source, &slots) {
                let name = output
                    .interactive_binding
                    .as_ref()
                    .map(|binding| binding.lexical_name.clone())
                    .unwrap_or_else(|| output.name.clone());
                program.outputs.insert(name, tensor);
                if boolean_schema(artifact, output.schema) {
                    let name = output
                        .interactive_binding
                        .as_ref()
                        .map(|binding| binding.lexical_name.clone())
                        .unwrap_or_else(|| output.name.clone());
                    program.boolean_outputs.insert(name);
                }
            }
        }
        Ok(program)
    }

    fn emit(&mut self, mut op: Op, mut inputs: Vec<Operand>) -> Operand {
        // The same exact rewrite is used by dynamic array/source-callable IR.
        if op == Op::Div
            && let Operand::Constant(divisor) = inputs[1]
            && let Some(bits) =
                mech_engine::numeric_optimizations::exact_binary_reciprocal(divisor.to_bits())
        {
            op = Op::Mul;
            inputs[1] = Operand::Constant(f64::from_bits(bits));
        }
        let constants = inputs
            .iter()
            .map(|operand| match operand {
                Operand::Constant(x) => Some(*x),
                _ => None,
            })
            .collect::<Option<Vec<_>>>();
        if let Some(xs) = constants {
            return Operand::Constant(match op {
                Op::Add => xs[0] + xs[1],
                Op::Sub => xs[0] - xs[1],
                Op::Mul => xs[0] * xs[1],
                Op::Div => xs[0] / xs[1],
                Op::Pow => xs[0].powf(xs[1]),
                Op::Sqrt => xs[0].sqrt(),
                Op::Neg => -xs[0],
                Op::Lt => f64::from(xs[0] < xs[1]),
                Op::Le => f64::from(xs[0] <= xs[1]),
                Op::Gt => f64::from(xs[0] > xs[1]),
                Op::Ge => f64::from(xs[0] >= xs[1]),
                Op::Eq => f64::from(xs[0] == xs[1]),
                Op::Ne => f64::from(xs[0] != xs[1]),
                Op::And => f64::from(xs[0] != 0.0 && xs[1] != 0.0),
                Op::Or => f64::from(xs[0] != 0.0 || xs[1] != 0.0),
                Op::Not => f64::from(xs[0] == 0.0),
                _ => unreachable!("optimization-only instructions are created after lowering"),
            });
        }
        // Preserve arithmetic order; do not enable reassociation or FMA.
        if matches!(op, Op::Pow)
            && let Operand::Constant(exponent) = inputs[1]
        {
            if exponent == 2.0 {
                return self.emit(Op::Mul, vec![inputs[0], inputs[0]]);
            }
            // Only reduce half powers when a nonnegative expression proves
            // sqrt's domain, including its zero sign, matches powf.
            if exponent == 0.5 && self.nonnegative(inputs[0]) {
                return self.emit(Op::Sqrt, vec![inputs[0]]);
            }
        }
        let result = Operand::Register(self.initial.len() + self.instructions.len());
        self.instructions.push(Instruction { op, inputs });
        result
    }

    fn nonnegative(&self, operand: Operand) -> bool {
        match operand {
            Operand::Constant(x) => x >= 0.0 && !x.is_sign_negative(),
            Operand::Register(index) if index >= self.initial.len() => {
                let instruction = &self.instructions[index - self.initial.len()];
                match instruction.op {
                    Op::Mul => {
                        matches!((instruction.inputs[0], instruction.inputs[1]), (Operand::Register(a), Operand::Register(b)) if a == b)
                    }
                    Op::Add => instruction.inputs.iter().all(|x| self.nonnegative(*x)),
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn mark_simd_pairs(&mut self, values: &[Operand]) {
        for pair in values.chunks_exact(2) {
            let [Operand::Register(first), Operand::Register(second)] = pair else {
                continue;
            };
            if *first < self.initial.len() || *second != first + 1 {
                continue;
            }
            let index = first - self.initial.len();
            let a = &self.instructions[index];
            let b = &self.instructions[index + 1];
            if a.op == b.op
                && !b
                    .inputs
                    .iter()
                    .any(|x| matches!(x, Operand::Register(r) if r == first))
            {
                self.simd_pairs.insert(index);
            }
        }
    }

    fn operation(&mut self, name: &str, inputs: &[Tensor]) -> Result<Tensor, String> {
        match name {
            "core/assign" if inputs.len() == 1 => Ok(inputs[0].clone()),
            "matrix/transpose" if inputs.len() == 1 => {
                let source = &inputs[0];
                // A transpose changes the logical operand map, not the
                // arithmetic or physical state layout. This works for both
                // constants and live values without a runtime copy.
                let mut values = Vec::with_capacity(source.values.len());
                for column in 0..source.rows {
                    for row in 0..source.columns {
                        values.push(source.at(column, row));
                    }
                }
                Ok(Tensor {
                    rows: source.columns,
                    columns: source.rows,
                    values,
                })
            }
            "math/neg" | "logic/not" if inputs.len() == 1 => {
                let mut out = inputs[0].clone();
                for group in element_groups(out.rows, out.columns) {
                    let values = group
                        .iter()
                        .map(|i| {
                            out.values[*i] = self.emit(
                                if name == "logic/not" {
                                    Op::Not
                                } else {
                                    Op::Neg
                                },
                                vec![inputs[0].values[*i]],
                            );
                            out.values[*i]
                        })
                        .collect::<Vec<_>>();
                    self.mark_simd_pairs(&values);
                }
                Ok(out)
            }
            "math/add" | "math/sub" | "math/mul" | "math/div" | "math/pow" | "compare/lt"
            | "compare/lte" | "compare/gt" | "compare/gte" | "compare/eq" | "compare/neq"
            | "logic/and" | "logic/or"
                if inputs.len() == 2 =>
            {
                let op = match name {
                    "math/add" => Op::Add,
                    "math/sub" => Op::Sub,
                    "math/mul" => Op::Mul,
                    "math/div" => Op::Div,
                    "compare/lt" => Op::Lt,
                    "compare/lte" => Op::Le,
                    "compare/gt" => Op::Gt,
                    "compare/gte" => Op::Ge,
                    "compare/eq" => Op::Eq,
                    "compare/neq" => Op::Ne,
                    "logic/and" => Op::And,
                    "logic/or" => Op::Or,
                    _ => Op::Pow,
                };
                let a = &inputs[0];
                let b = &inputs[1];
                let rows = a.rows.max(b.rows);
                let columns = a.columns.max(b.columns);
                if ![a, b].iter().all(|x| {
                    (x.rows == 1 || x.rows == rows) && (x.columns == 1 || x.columns == columns)
                }) {
                    return Err("incompatible broadcasting shapes".into());
                }
                let mut values = vec![Operand::Constant(0.0); rows * columns];
                for group in element_groups(rows, columns) {
                    let group_values = group
                        .iter()
                        .map(|index| {
                            let row = index % rows;
                            let column = index / rows;
                            values[*index] = self
                                .emit(op, vec![a.broadcast(row, column), b.broadcast(row, column)]);
                            values[*index]
                        })
                        .collect::<Vec<_>>();
                    self.mark_simd_pairs(&group_values);
                }
                Ok(Tensor {
                    rows,
                    columns,
                    values,
                })
            }
            "stats/sum/row" | "stats/sum/column" if inputs.len() == 1 => {
                let source = &inputs[0];
                let (rows, columns) = if name.ends_with("/row") {
                    (1, source.columns)
                } else {
                    (source.rows, 1)
                };
                let across_rows = name.ends_with("/row");
                let terms = if across_rows {
                    source.rows
                } else {
                    source.columns
                };
                let mut values = vec![Operand::Constant(0.0); rows * columns];
                // Interleave independent reductions, preserving the summand
                // order within each output. Matching elements remain in the
                // same SIMD lanes for every stage of the reduction.
                for term in 0..terms {
                    for column in 0..columns {
                        for row in 0..rows {
                            let index = row + rows * column;
                            let input = if across_rows {
                                source.at(term, column)
                            } else {
                                source.at(row, term)
                            };
                            values[index] = self.emit(Op::Add, vec![values[index], input]);
                        }
                    }
                    self.mark_simd_pairs(&values);
                }
                Ok(Tensor {
                    rows,
                    columns,
                    values,
                })
            }
            "matrix/horzcat" | "matrix/vertcat" if !inputs.is_empty() => {
                let horizontal = name.ends_with("horzcat");
                let rows = if horizontal {
                    inputs[0].rows
                } else {
                    inputs.iter().map(|x| x.rows).sum()
                };
                let columns = if horizontal {
                    inputs.iter().map(|x| x.columns).sum()
                } else {
                    inputs[0].columns
                };
                if inputs.iter().any(|x| {
                    if horizontal {
                        x.rows != rows
                    } else {
                        x.columns != columns
                    }
                }) {
                    return Err("concatenation shape mismatch".into());
                }
                let mut values = Vec::new();
                if horizontal {
                    for input in inputs {
                        values.extend(&input.values);
                    }
                } else {
                    for column in 0..columns {
                        for input in inputs {
                            for row in 0..input.rows {
                                values.push(input.at(row, column));
                            }
                        }
                    }
                }
                Ok(Tensor {
                    rows,
                    columns,
                    values,
                })
            }
            "range/inclusive" if inputs.len() == 2 => {
                let start = scalar_constant(&inputs[0])?;
                let end = scalar_constant(&inputs[1])?;
                if start.fract() != 0.0
                    || end.fract() != 0.0
                    || start < 1.0
                    || end < start
                    || end - start > 4096.0
                {
                    return Err("unsupported static range".into());
                }
                let values = (start as usize..=end as usize)
                    .map(|x| Operand::Constant(x as f64))
                    .collect::<Vec<_>>();
                Ok(Tensor {
                    rows: values.len(),
                    columns: 1,
                    values,
                })
            }
            "combinatorics/n-choose-k" if inputs.len() == 2 => {
                let k = scalar_constant(&inputs[1])?;
                if k != 2.0 || inputs[0].values.len() > 64 {
                    return Err("experimental combinations support only static k=2".into());
                }
                let mut values = Vec::new();
                for i in 0..inputs[0].values.len() {
                    for j in i + 1..inputs[0].values.len() {
                        values.extend([inputs[0].values[i], inputs[0].values[j]]);
                    }
                }
                Ok(Tensor {
                    rows: 2,
                    columns: values.len() / 2,
                    values,
                })
            }
            "access/scalar" if inputs.len() == 2 || inputs.len() == 3 => {
                let source = &inputs[0];
                let selected = if inputs.len() == 2 {
                    let indices = inputs[1].indices(source.values.len())?;
                    if indices.len() != 1 {
                        return Err("scalar selector must have one position".into());
                    }
                    source.values[indices[0]]
                } else {
                    let rows = inputs[1].indices(source.rows)?;
                    let columns = inputs[2].indices(source.columns)?;
                    if rows.len() != 1 || columns.len() != 1 {
                        return Err("scalar selectors must have one position".into());
                    }
                    source.at(rows[0], columns[0])
                };
                Ok(Tensor {
                    rows: 1,
                    columns: 1,
                    values: vec![selected],
                })
            }
            "access/rows" | "access/range" if inputs.len() == 2 => {
                let source = &inputs[0];
                if name.ends_with("rows") {
                    let indices = inputs[1].indices(source.rows)?;
                    let values = (0..source.columns)
                        .flat_map(|column| indices.iter().map(move |row| source.at(*row, column)))
                        .collect();
                    Ok(Tensor {
                        rows: indices.len(),
                        columns: source.columns,
                        values,
                    })
                } else {
                    let indices = inputs[1].indices(source.values.len())?;
                    Ok(Tensor {
                        rows: indices.len(),
                        columns: 1,
                        values: indices.iter().map(|index| source.values[*index]).collect(),
                    })
                }
            }
            "core/assign/indexed-rows/add" | "core/assign/indexed-rows/sub"
                if inputs.len() == 3 =>
            {
                let mut out = inputs[0].clone();
                let update = &inputs[1];
                let indices = inputs[2].indices(out.rows)?;
                if update.rows != indices.len() || update.columns != out.columns {
                    return Err("indexed update shape mismatch".into());
                }
                let op = if name.ends_with("add") {
                    Op::Add
                } else {
                    Op::Sub
                };
                // Repeated selectors accumulate in source order, exactly as
                // resident indexed arithmetic; they are not parallel stores.
                for (row, target) in indices.iter().enumerate() {
                    let mut group = Vec::new();
                    for column in 0..out.columns {
                        let index = target + out.rows * column;
                        out.values[index] =
                            self.emit(op, vec![out.values[index], update.at(row, column)]);
                        group.push(out.values[index]);
                    }
                    self.mark_simd_pairs(&group);
                }
                Ok(out)
            }
            _ => Err("operation is outside the experimental fused subset".into()),
        }
    }

    pub fn c_kernel(&self) -> String {
        let mut code = String::from(
            "#include <stddef.h>\n#include <math.h>\nvoid mech_turns(double *restrict state, size_t turns) {\n",
        );
        for index in 0..self.initial.len() {
            writeln!(code, "  double s{index} = state[{index}];").unwrap();
        }
        code.push_str("  for (size_t turn = 0; turn < turns; ++turn) {\n");
        code.push_str(&self.c_arithmetic(&self.updates));
        let operand = |x| self.c_operand(x);
        for (index, update) in self.updates.iter().enumerate() {
            writeln!(code, "    const double next{index} = {};", operand(*update)).unwrap();
        }
        for index in 0..self.initial.len() {
            writeln!(code, "    s{index} = next{index};").unwrap();
        }
        code.push_str("  }\n");
        for index in 0..self.initial.len() {
            writeln!(code, "  state[{index}] = s{index};").unwrap();
        }
        code.push_str("}\n");
        code
    }

    pub fn c_observer(&self, output: &str) -> Result<String, String> {
        let values = &self
            .outputs
            .get(output)
            .ok_or_else(|| format!("unknown output: {output}"))?
            .values;
        let mut code = String::from("void mech_observe(const double *state, double *output) {\n");
        for index in 0..self.initial.len() {
            writeln!(code, "  const double s{index} = state[{index}];").unwrap();
        }
        code.push_str(&self.c_arithmetic(values));
        for (index, value) in values.iter().enumerate() {
            writeln!(code, "  output[{index}] = {};", self.c_operand(*value)).unwrap();
        }
        code.push_str("}\n");
        Ok(code)
    }

    fn c_arithmetic(&self, roots: &[Operand]) -> String {
        let mut code = String::new();
        let operand = |x| self.c_operand(x);
        let live = self.live_instructions(roots);
        for (index, instruction) in self.instructions.iter().enumerate() {
            if !live.contains(&index) {
                continue;
            }
            let a = operand(instruction.inputs[0]);
            let expression = match instruction.op {
                Op::Neg => format!("(-{a})"),
                Op::Sqrt => format!("sqrt({a})"),
                Op::Pow => format!("pow({a}, {})", operand(instruction.inputs[1])),
                Op::Fma | Op::Fmsub | Op::Fnmadd => {
                    let b = operand(instruction.inputs[1]);
                    let c = operand(instruction.inputs[2]);
                    match instruction.op {
                        Op::Fmsub => format!("fma({a}, {b}, -({c}))"),
                        Op::Fnmadd => format!("fma(-({a}), {b}, {c})"),
                        _ => format!("fma({a}, {b}, {c})"),
                    }
                }
                Op::RefinedInverseCube => {
                    let q = operand(instruction.inputs[1]);
                    writeln!(code, "    double seed{index} = (double)(1.0f / sqrtf((float)({q})));\n    seed{index} *= 1.5 - 0.5 * ({q}) * seed{index} * seed{index};\n    seed{index} *= 1.5 - 0.5 * ({q}) * seed{index} * seed{index};").unwrap();
                    format!("({a} * (seed{index} * (seed{index} * seed{index})))")
                }
                Op::InverseCubeSqrt => {
                    let q = operand(instruction.inputs[1]);
                    format!("({a} / ({q} * sqrt({q})))")
                }
                Op::Not => format!("({a} == 0.0 ? 1.0 : 0.0)"),
                Op::Lt | Op::Le | Op::Gt | Op::Ge | Op::Eq | Op::Ne | Op::And | Op::Or => {
                    let operator = match instruction.op {
                        Op::Lt => "<",
                        Op::Le => "<=",
                        Op::Gt => ">",
                        Op::Ge => ">=",
                        Op::Eq => "==",
                        Op::Ne => "!=",
                        Op::And => "&&",
                        Op::Or => "||",
                        _ => unreachable!(),
                    };
                    format!(
                        "({a} {operator} {} ? 1.0 : 0.0)",
                        operand(instruction.inputs[1])
                    )
                }
                op => format!(
                    "({a} {} {})",
                    match op {
                        Op::Add => "+",
                        Op::Sub => "-",
                        Op::Mul => "*",
                        Op::Div => "/",
                        _ => unreachable!(),
                    },
                    operand(instruction.inputs[1])
                ),
            };
            writeln!(code, "    const double r{index} = {expression};").unwrap();
        }
        code
    }

    fn c_operand(&self, operand: Operand) -> String {
        match operand {
            Operand::Constant(x) if x.is_nan() => "NAN".into(),
            Operand::Constant(x) if x == f64::INFINITY => "INFINITY".into(),
            Operand::Constant(x) if x == f64::NEG_INFINITY => "(-INFINITY)".into(),
            Operand::Constant(x) => format!("{x:.17e}"),
            Operand::Register(index) if index < self.initial.len() => format!("s{index}"),
            Operand::Register(index) => format!("r{}", index - self.initial.len()),
        }
    }
}

fn element_groups(rows: usize, columns: usize) -> Vec<Vec<usize>> {
    if columns == 1 {
        return vec![(0..rows).collect()];
    }
    (0..rows)
        .map(|row| (0..columns).map(|column| row + rows * column).collect())
        .collect()
}

fn scalar_constant(tensor: &Tensor) -> Result<f64, String> {
    match tensor.values.as_slice() {
        [Operand::Constant(x)] if x.is_finite() => Ok(*x),
        _ => Err("expected a finite static scalar".into()),
    }
}

fn shape(artifact: &ProgramArtifact, id: SchemaId) -> Result<(usize, usize), String> {
    match artifact.schemas().get(id).ok_or("missing schema")?.body() {
        SchemaBody::FloatingPoint(FloatWidth::W64) | SchemaBody::Bool => Ok((1, 1)),
        SchemaBody::Matrix {
            element,
            dimensions,
        } if matches!(
            element.as_ref(),
            SchemaBody::FloatingPoint(FloatWidth::W64) | SchemaBody::Bool
        ) =>
        {
            let dimensions = dimensions
                .iter()
                .map(|x| match x {
                    DimensionExpr::Constant(n) => {
                        usize::try_from(*n).map_err(|_| "oversized shape".into())
                    }
                    _ => Err("dynamic shapes are unsupported".into()),
                })
                .collect::<Result<Vec<_>, String>>()?;
            if dimensions.is_empty()
                || dimensions.len() > 2
                || dimensions.contains(&0)
                || dimensions.iter().any(|n| *n > 4096)
                || dimensions.iter().product::<usize>() > 4096
            {
                return Err("unsupported shape".into());
            }
            Ok((dimensions[0], dimensions.get(1).copied().unwrap_or(1)))
        }
        _ => Err("only fixed-shape f64 data is admitted".into()),
    }
}

fn boolean_schema(artifact: &ProgramArtifact, id: SchemaId) -> bool {
    match artifact.schemas().get(id).map(|schema| schema.body()) {
        Some(SchemaBody::Bool) => true,
        Some(SchemaBody::Matrix { element, .. }) => matches!(element.as_ref(), SchemaBody::Bool),
        _ => false,
    }
}
