//! A tiled array interpreter for generic fixed-shape programs.
//!
//! The physical batch is component-major: adjacent SIMD vectors contain the
//! same component of independent instances. Operations are decoded once at
//! preparation, and each operation traverses a cache-sized column rather than
//! dispatching the complete scalar instruction stream for every SIMD vector.

use std::cell::OnceCell;

use super::*;

const DEFAULT_TILE_INSTANCES: usize = 128;

#[derive(Debug)]
struct ArrayBuffer {
    slot: CellSlotId,
    components: usize,
    values: Vec<f32x4>,
}

#[derive(Clone, Copy, Debug)]
struct ArrayLoad {
    column: usize,
    buffer: usize,
    component: usize,
}

#[derive(Clone, Copy, Debug)]
enum ArrayProduct {
    Add(usize),
    Subtract(usize),
    Multiply(usize, usize),
}

fn array_is_one_operand(operand: ScalarOperand) -> bool {
    matches!(operand, ScalarOperand::Constant(value) if value == 1.0)
}

fn array_is_negative_one_operand(operand: ScalarOperand) -> bool {
    matches!(operand, ScalarOperand::Constant(value) if value == -1.0)
}

#[derive(Clone, Debug)]
enum ArrayOperation {
    Copy(usize),
    Negate(usize),
    Absolute(usize),
    IsFinite(usize),
    Compare(ComparisonOperation, usize, usize),
    Logic(LogicOperation, usize, usize),
    Binary(BinaryOperation, usize, usize),
    Unary(UnaryOperation, usize),
    Atan2(usize, usize),
    Sum2([ArrayProduct; 2]),
    Sum3([ArrayProduct; 3]),
    Sum4([ArrayProduct; 4]),
    SumProducts(Box<[ArrayProduct]>),
}

#[derive(Clone, Debug)]
struct ArrayInstruction {
    output: usize,
    operation: ArrayOperation,
}

#[derive(Debug)]
struct ArrayPlan {
    columns: usize,
    constants: Vec<(usize, f32x4)>,
    inputs: Vec<ArrayLoad>,
    states: Vec<ArrayLoad>,
    instructions: Vec<ArrayInstruction>,
    updates: Vec<Vec<usize>>,
    predicates: Vec<ScalarPredicate>,
}

/// Resident component-major storage with tiled SIMD interpretation.
///
/// Preparation transposes inputs once. Repeated turns use the same buffers;
/// [`Self::state`] materializes a canonical instance-major snapshot only when
/// requested by the host. Integrity validation commits or rejects a complete
/// batch, including instances in the final partial SIMD vector.
#[derive(Debug)]
pub struct BatchedArrayCpuSession {
    program: Arc<FixedShapeKernel>,
    plan: ArrayPlan,
    groups: usize,
    tile_groups: usize,
    inputs: Vec<ArrayBuffer>,
    state: Vec<ArrayBuffer>,
    next_state: Vec<ArrayBuffer>,
    registers: Vec<f32x4>,
    snapshot: OnceCell<BTreeMap<CellSlotId, Vec<f32>>>,
    faults: BatchedFaultRecorder,
}

impl FixedShapeKernel {
    /// Prepares a component-major CPU array interpreter using cache-sized
    /// tiles. The source graph and floating-point operations are unchanged.
    pub fn prepare_array_cpu(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BatchedArrayCpuSession, BatchedExecutionError> {
        self.prepare_array_cpu_with_tile(inputs, DEFAULT_TILE_INSTANCES)
    }

    /// Selects the maximum number of independent instances in one CPU tile.
    /// Tile sizes are rounded up to a complete four-lane SIMD vector.
    pub fn prepare_array_cpu_with_tile(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
        tile_instances: usize,
    ) -> Result<BatchedArrayCpuSession, BatchedExecutionError> {
        if tile_instances == 0 {
            return Err(BatchedExecutionError::Native(
                "array CPU tile size must be nonzero".to_owned(),
            ));
        }
        let groups = (self.instances as usize).div_ceil(SIMD_LANES);
        let tile_groups = tile_instances.div_ceil(SIMD_LANES).min(groups.max(1));
        let expanded = self.expand_inputs(inputs)?;
        let inputs = self
            .inputs
            .iter()
            .map(|input| ArrayBuffer {
                slot: input.slot,
                components: input.shape.elements(),
                values: pack_array(
                    &expanded[&input.slot],
                    input.shape.elements(),
                    self.instances as usize,
                ),
            })
            .collect();
        let state = self
            .states
            .iter()
            .map(|state| ArrayBuffer {
                slot: state.slot,
                components: state.shape.elements(),
                values: state
                    .initializer
                    .iter()
                    .flat_map(|value| std::iter::repeat_n(f32x4::splat(*value), groups))
                    .collect(),
            })
            .collect::<Vec<_>>();
        let next_state = state
            .iter()
            .map(|state| ArrayBuffer {
                slot: state.slot,
                components: state.components,
                values: vec![f32x4::ZERO; state.values.len()],
            })
            .collect();
        let plan = ArrayPlan::new(self);
        let mut registers = vec![f32x4::ZERO; plan.columns * tile_groups];
        for (column, value) in &plan.constants {
            registers[column * tile_groups..(column + 1) * tile_groups].fill(*value);
        }
        Ok(BatchedArrayCpuSession {
            program: Arc::new(self.clone()),
            plan,
            groups,
            tile_groups,
            inputs,
            state,
            next_state,
            registers,
            snapshot: OnceCell::new(),
            faults: BatchedFaultRecorder::default(),
        })
    }
}

impl BatchedArrayCpuSession {
    /// Changes inputs atomically after validating every supplied name/shape.
    pub fn update_inputs(
        &mut self,
        updates: &BTreeMap<String, Vec<f32>>,
    ) -> Result<(), BatchedExecutionError> {
        let mut staged = Vec::with_capacity(updates.len());
        for (name, values) in updates {
            let (index, input) = self
                .program
                .inputs
                .iter()
                .enumerate()
                .find(|(_, input)| input.name == *name)
                .ok_or_else(|| BatchedExecutionError::MissingInput(name.clone()))?;
            let expanded = self.program.expand_input(input, values)?;
            staged.push((
                index,
                pack_array(
                    &expanded,
                    input.shape.elements(),
                    self.program.instances as usize,
                ),
            ));
        }
        for (index, values) in staged {
            self.inputs[index].values = values;
        }
        Ok(())
    }

    pub fn dispatch_turns(&mut self, turns: u32) -> Result<(), BatchedExecutionError> {
        if turns == 0 {
            return Err(BatchedExecutionError::ZeroTurns);
        }
        for _ in 0..turns {
            let attempted_turn = self.faults.next_turn();
            for first_group in (0..self.groups).step_by(self.tile_groups) {
                let count = (self.groups - first_group).min(self.tile_groups);
                load_array_columns(
                    &mut self.registers,
                    self.tile_groups,
                    count,
                    first_group,
                    self.groups,
                    &self.plan.inputs,
                    &self.inputs,
                );
                load_array_columns(
                    &mut self.registers,
                    self.tile_groups,
                    count,
                    first_group,
                    self.groups,
                    &self.plan.states,
                    &self.state,
                );
                for instruction in &self.plan.instructions {
                    instruction.execute(&mut self.registers, self.tile_groups, count);
                }
                // Scan in instance order, breaking ties in artifact constraint order.
                // Predicates traverse SIMD vectors, while inactive tail lanes
                // are excluded from the rejection mask.
                if !self.plan.predicates.is_empty()
                    && let Some((instance, constraint_index)) = self.first_fault(first_group, count)
                {
                    let constraint = &self.program.constraints[constraint_index];
                    return Err(self.faults.record(BatchedIntegrityFault {
                        attempted_turn,
                        instance: instance as u32,
                        constraint: constraint.id,
                        constraint_name: constraint.name.clone(),
                    }));
                }
                for (state, updates) in self.next_state.iter_mut().zip(&self.plan.updates) {
                    for (component, column) in updates.iter().enumerate() {
                        state.values[component * self.groups + first_group
                            ..component * self.groups + first_group + count]
                            .copy_from_slice(
                                &self.registers
                                    [column * self.tile_groups..column * self.tile_groups + count],
                            );
                    }
                }
            }
            std::mem::swap(&mut self.state, &mut self.next_state);
            self.snapshot.take();
        }
        Ok(())
    }

    fn first_fault(&self, first_group: usize, count: usize) -> Option<(usize, usize)> {
        for local_group in 0..count {
            let first_instance = (first_group + local_group) * SIMD_LANES;
            let active = (self.program.instances as usize - first_instance).min(SIMD_LANES);
            let active_mask = (1_u32 << active) - 1;
            let mut earliest: Option<(usize, usize)> = None;
            for (constraint, predicate) in self.plan.predicates.iter().enumerate() {
                let valid =
                    array_predicate_mask(predicate, &self.registers, self.tile_groups, local_group);
                let failed = (!valid).move_mask() as u32 & active_mask;
                if failed != 0 {
                    let candidate = (
                        first_instance + failed.trailing_zeros() as usize,
                        constraint,
                    );
                    earliest = Some(earliest.map_or(candidate, |old| old.min(candidate)));
                }
            }
            if earliest.is_some() {
                return earliest;
            }
        }
        None
    }

    pub fn state(&self) -> &BTreeMap<CellSlotId, Vec<f32>> {
        self.snapshot.get_or_init(|| {
            self.state
                .iter()
                .map(|state| {
                    let mut values = vec![0.0; state.components * self.program.instances as usize];
                    for component in 0..state.components {
                        for group in 0..self.groups {
                            let vector = state.values[component * self.groups + group].to_array();
                            for (lane, value) in vector.into_iter().enumerate() {
                                let instance = group * SIMD_LANES + lane;
                                if instance < self.program.instances as usize {
                                    values[instance * state.components + component] = value;
                                }
                            }
                        }
                    }
                    (state.slot, values)
                })
                .collect()
        })
    }

    pub const fn tile_instances(&self) -> usize {
        self.tile_groups * SIMD_LANES
    }

    /// Scratch columns after liveness-based reuse, including constants.
    pub const fn scratch_columns(&self) -> usize {
        self.plan.columns
    }

    pub fn scratch_bytes(&self) -> usize {
        self.registers.len() * std::mem::size_of::<f32x4>()
    }

    pub const fn fault_count(&self) -> u64 {
        self.faults.fault_count
    }

    pub const fn attempted_turns(&self) -> u64 {
        self.faults.attempted_turns()
    }

    pub fn last_fault(&self) -> Option<&BatchedIntegrityFault> {
        self.faults.last_fault.as_ref()
    }
}

fn pack_array(values: &[f32], components: usize, instances: usize) -> Vec<f32x4> {
    let groups = instances.div_ceil(SIMD_LANES);
    let mut packed = vec![f32x4::ZERO; groups * components];
    for component in 0..components {
        for group in 0..groups {
            let mut lanes = [0.0; SIMD_LANES];
            for (lane, value) in lanes.iter_mut().enumerate() {
                let instance = group * SIMD_LANES + lane;
                if instance < instances {
                    *value = values[instance * components + component];
                }
            }
            packed[component * groups + group] = f32x4::new(lanes);
        }
    }
    packed
}

#[allow(clippy::too_many_arguments)]
fn load_array_columns(
    registers: &mut [f32x4],
    stride: usize,
    count: usize,
    first_group: usize,
    groups: usize,
    loads: &[ArrayLoad],
    buffers: &[ArrayBuffer],
) {
    for load in loads {
        registers[load.column * stride..load.column * stride + count].copy_from_slice(
            &buffers[load.buffer].values[load.component * groups + first_group
                ..load.component * groups + first_group + count],
        );
    }
}

struct ArrayPlanBuilder {
    mapping: Vec<Option<usize>>,
    constants: BTreeMap<u32, usize>,
    free: Vec<usize>,
    columns: usize,
}

impl ArrayPlanBuilder {
    fn allocate(&mut self) -> usize {
        self.free.pop().unwrap_or_else(|| {
            let column = self.columns;
            self.columns += 1;
            column
        })
    }

    fn operand(&mut self, operand: ScalarOperand) -> usize {
        match operand {
            ScalarOperand::Register(register) => {
                self.mapping[register].expect("live array operand has a producer")
            }
            ScalarOperand::Constant(value) => {
                let key = value.to_bits();
                if let Some(column) = self.constants.get(&key) {
                    *column
                } else {
                    // Constants are initialized before executing a tile, so
                    // their columns cannot reuse an earlier instruction's
                    // live input column even if that input later expires.
                    let column = self.columns;
                    self.columns += 1;
                    self.constants.insert(key, column);
                    column
                }
            }
        }
    }

    fn predicate(&mut self, predicate: &ScalarPredicate) -> ScalarPredicate {
        let mut operand = |input| ScalarOperand::Register(self.operand(input));
        match predicate {
            ScalarPredicate::Value(input) => ScalarPredicate::Value(operand(*input)),
            ScalarPredicate::IsFinite(input) => ScalarPredicate::IsFinite(operand(*input)),
            ScalarPredicate::AbsoluteDifferenceWithin {
                left,
                right,
                tolerance,
            } => ScalarPredicate::AbsoluteDifferenceWithin {
                left: operand(*left),
                right: operand(*right),
                tolerance: operand(*tolerance),
            },
            ScalarPredicate::Compare {
                operation,
                left,
                right,
            } => ScalarPredicate::Compare {
                operation: *operation,
                left: operand(*left),
                right: operand(*right),
            },
            ScalarPredicate::All(inputs) => {
                ScalarPredicate::All(inputs.iter().map(|input| self.predicate(input)).collect())
            }
            ScalarPredicate::Logic { operation, inputs } => ScalarPredicate::Logic {
                operation: *operation,
                inputs: inputs.iter().map(|input| self.predicate(input)).collect(),
            },
        }
    }

    fn operation(&mut self, computation: &ScalarComputation) -> ArrayOperation {
        match computation {
            ScalarComputation::Copy(input) => ArrayOperation::Copy(self.operand(*input)),
            ScalarComputation::Negate(input) => ArrayOperation::Negate(self.operand(*input)),
            ScalarComputation::Absolute(input) => ArrayOperation::Absolute(self.operand(*input)),
            ScalarComputation::IsFinite(input) => ArrayOperation::IsFinite(self.operand(*input)),
            ScalarComputation::Compare {
                operation,
                left,
                right,
            } => ArrayOperation::Compare(*operation, self.operand(*left), self.operand(*right)),
            ScalarComputation::Logic { operation, inputs } => {
                let left = self.operand(inputs[0]);
                let right = inputs.get(1).map_or(left, |input| self.operand(*input));
                ArrayOperation::Logic(*operation, left, right)
            }
            ScalarComputation::Elementwise { operation, inputs } => match operation {
                ElementwiseOperation::Binary(operation) => ArrayOperation::Binary(
                    *operation,
                    self.operand(inputs[0]),
                    self.operand(inputs[1]),
                ),
                ElementwiseOperation::Unary(operation) => {
                    ArrayOperation::Unary(*operation, self.operand(inputs[0]))
                }
                ElementwiseOperation::Atan2 => {
                    ArrayOperation::Atan2(self.operand(inputs[0]), self.operand(inputs[1]))
                }
                ElementwiseOperation::Identity => ArrayOperation::Copy(self.operand(inputs[0])),
            },
            ScalarComputation::SumProducts(terms) => {
                let terms = terms
                    .iter()
                    .map(|(left, right)| {
                        if array_is_one_operand(*left) {
                            ArrayProduct::Add(self.operand(*right))
                        } else if array_is_one_operand(*right) {
                            ArrayProduct::Add(self.operand(*left))
                        } else if array_is_negative_one_operand(*left) {
                            ArrayProduct::Subtract(self.operand(*right))
                        } else if array_is_negative_one_operand(*right) {
                            ArrayProduct::Subtract(self.operand(*left))
                        } else {
                            ArrayProduct::Multiply(self.operand(*left), self.operand(*right))
                        }
                    })
                    .collect::<Vec<_>>();
                match terms.as_slice() {
                    [first, second] => ArrayOperation::Sum2([*first, *second]),
                    [first, second, third] => ArrayOperation::Sum3([*first, *second, *third]),
                    [first, second, third, fourth] => {
                        ArrayOperation::Sum4([*first, *second, *third, *fourth])
                    }
                    _ => ArrayOperation::SumProducts(terms.into_boxed_slice()),
                }
            }
        }
    }
}

impl ArrayPlan {
    fn new(program: &FixedShapeKernel) -> Self {
        // Retain only computations feeding published state or predicates.
        // This also avoids evaluating removed constraints in unchecked copies.
        let mut live = BTreeSet::new();
        for state in &program.states {
            for update in &state.update {
                collect_operand_register(*update, &mut live);
            }
        }
        for constraint in &program.constraints {
            constraint.predicate.collect_registers(&mut live);
        }
        let mut instructions = Vec::new();
        for instruction in program.fixed_ir().instructions.iter().rev() {
            if live.remove(&instruction.output) {
                instruction.computation.collect_registers(&mut live);
                instructions.push(instruction);
            }
        }
        instructions.reverse();
        let mut last_use = vec![None; program.fixed_ir().register_count];
        for (index, instruction) in instructions.iter().enumerate() {
            let mut sources = BTreeSet::new();
            instruction.computation.collect_registers(&mut sources);
            for source in sources {
                last_use[source] = Some(index);
            }
        }
        for state in &program.states {
            for update in &state.update {
                if let ScalarOperand::Register(register) = update {
                    last_use[*register] = Some(instructions.len());
                }
            }
        }
        for constraint in &program.constraints {
            let mut sources = BTreeSet::new();
            constraint.predicate.collect_registers(&mut sources);
            for source in sources {
                last_use[source] = Some(instructions.len());
            }
        }
        let mut builder = ArrayPlanBuilder {
            mapping: vec![None; last_use.len()],
            constants: BTreeMap::new(),
            free: Vec::new(),
            columns: 0,
        };
        let mut inputs = Vec::new();
        let mut states = Vec::new();
        for (buffer, input) in program.inputs.iter().enumerate() {
            let offset = program.register_offsets[&input.slot];
            for component in 0..input.shape.elements() {
                if last_use[offset + component].is_some() {
                    let column = builder.allocate();
                    builder.mapping[offset + component] = Some(column);
                    inputs.push(ArrayLoad {
                        column,
                        buffer,
                        component,
                    });
                }
            }
        }
        for (buffer, state) in program.states.iter().enumerate() {
            let offset = program.register_offsets[&state.slot];
            for component in 0..state.shape.elements() {
                if last_use[offset + component].is_some() {
                    let column = builder.allocate();
                    builder.mapping[offset + component] = Some(column);
                    states.push(ArrayLoad {
                        column,
                        buffer,
                        component,
                    });
                }
            }
        }
        let mut decoded = Vec::with_capacity(instructions.len());
        for (index, instruction) in instructions.iter().enumerate() {
            let operation = builder.operation(&instruction.computation);
            let output = builder.allocate();
            if let Some(previous) = builder.mapping[instruction.output].replace(output) {
                builder.free.push(previous);
            }
            decoded.push(ArrayInstruction { output, operation });
            let mut sources = BTreeSet::new();
            instruction.computation.collect_registers(&mut sources);
            for source in sources {
                if source != instruction.output && last_use[source] == Some(index) {
                    if let Some(column) = builder.mapping[source].take() {
                        builder.free.push(column);
                    }
                }
            }
        }
        let updates = program
            .states
            .iter()
            .map(|state| {
                state
                    .update
                    .iter()
                    .map(|input| builder.operand(*input))
                    .collect()
            })
            .collect();
        let predicates = program
            .constraints
            .iter()
            .map(|constraint| builder.predicate(&constraint.predicate))
            .collect();
        Self {
            columns: builder.columns,
            constants: builder
                .constants
                .into_iter()
                .map(|(bits, column)| (column, f32x4::splat(f32::from_bits(bits))))
                .collect(),
            inputs,
            states,
            instructions: decoded,
            updates,
            predicates,
        }
    }
}

/// The destination column is allocated separately from its operands. Split
/// borrowing lets the inner loops traverse slices without indexing the
/// register arena or evaluating operands for every SIMD vector.
fn unary_columns(
    registers: &mut [f32x4],
    stride: usize,
    count: usize,
    output: usize,
    input: usize,
) -> (&mut [f32x4], &[f32x4]) {
    let (before, output_and_after) = registers.split_at_mut(output * stride);
    let (destination, after) = output_and_after.split_at_mut(stride);
    let source = source_column(before, after, stride, count, output, input);
    (&mut destination[..count], source)
}

fn binary_columns(
    registers: &mut [f32x4],
    stride: usize,
    count: usize,
    output: usize,
    left: usize,
    right: usize,
) -> (&mut [f32x4], &[f32x4], &[f32x4]) {
    let (before, output_and_after) = registers.split_at_mut(output * stride);
    let (destination, after) = output_and_after.split_at_mut(stride);
    let left = source_column(before, after, stride, count, output, left);
    let right = source_column(before, after, stride, count, output, right);
    (&mut destination[..count], left, right)
}

fn product_columns<const N: usize>(
    registers: &mut [f32x4],
    stride: usize,
    count: usize,
    output: usize,
    inputs: [usize; N],
) -> (&mut [f32x4], [&[f32x4]; N]) {
    let (before, output_and_after) = registers.split_at_mut(output * stride);
    let (destination, after) = output_and_after.split_at_mut(stride);
    let sources = std::array::from_fn(|index| {
        source_column(before, after, stride, count, output, inputs[index])
    });
    (&mut destination[..count], sources)
}

fn source_column<'a>(
    before: &'a [f32x4],
    after: &'a [f32x4],
    stride: usize,
    count: usize,
    output: usize,
    input: usize,
) -> &'a [f32x4] {
    if input < output {
        &before[input * stride..input * stride + count]
    } else {
        let relative = input - output - 1;
        &after[relative * stride..relative * stride + count]
    }
}

#[inline]
fn apply_unary(
    registers: &mut [f32x4],
    stride: usize,
    count: usize,
    output: usize,
    input: usize,
    operation: impl Fn(f32x4) -> f32x4,
) {
    let (destination, source) = unary_columns(registers, stride, count, output, input);
    for (destination, source) in destination.iter_mut().zip(source) {
        *destination = operation(*source);
    }
}

#[inline]
fn apply_binary(
    registers: &mut [f32x4],
    stride: usize,
    count: usize,
    output: usize,
    left: usize,
    right: usize,
    operation: impl Fn(f32x4, f32x4) -> f32x4,
) {
    let (destination, left, right) = binary_columns(registers, stride, count, output, left, right);
    for ((destination, left), right) in destination.iter_mut().zip(left).zip(right) {
        *destination = operation(*left, *right);
    }
}

impl ArrayInstruction {
    fn execute(&self, registers: &mut [f32x4], stride: usize, count: usize) {
        let output = self.output;
        macro_rules! unary {
            ($input:expr, $operation:expr) => {
                apply_unary(registers, stride, count, output, $input, $operation)
            };
        }
        macro_rules! binary {
            ($left:expr, $right:expr, $operation:expr) => {
                apply_binary(registers, stride, count, output, $left, $right, $operation)
            };
        }
        // The term kind is selected outside the array loop. Distinct closure
        // types produce inlined add/subtract/FMA operations in each fused
        // loop, retaining the original left-to-right accumulation order.
        macro_rules! product {
            ($term:expr, |$left:ident, $right:ident, $apply:ident| $body:expr) => {
                match $term {
                    ArrayProduct::Add(source) => {
                        let $left = source;
                        let $right = source;
                        let $apply = |left: f32x4, _right: f32x4, sum: f32x4| sum + left;
                        $body
                    }
                    ArrayProduct::Subtract(source) => {
                        let $left = source;
                        let $right = source;
                        let $apply = |left: f32x4, _right: f32x4, sum: f32x4| sum - left;
                        $body
                    }
                    ArrayProduct::Multiply(left, right) => {
                        let $left = left;
                        let $right = right;
                        let $apply =
                            |left: f32x4, right: f32x4, sum: f32x4| left.mul_add(right, sum);
                        $body
                    }
                }
            };
        }
        match &self.operation {
            ArrayOperation::Copy(input) => {
                let (destination, source) = unary_columns(registers, stride, count, output, *input);
                destination.copy_from_slice(source);
            }
            // `wide` implements Neg as 0 - x, which loses the sign of zero.
            ArrayOperation::Negate(input) => unary!(*input, |value: f32x4| {
                value.flip_signs(f32x4::splat(-0.0))
            }),
            ArrayOperation::Absolute(input) => unary!(*input, f32x4::abs),
            ArrayOperation::IsFinite(input) => unary!(*input, |value: f32x4| {
                value.is_finite().blend(f32x4::ONE, f32x4::ZERO)
            }),
            ArrayOperation::Compare(operation, left, right) => {
                macro_rules! compare {
                    ($method:ident) => {
                        binary!(*left, *right, |a: f32x4, b: f32x4| {
                            a.$method(b).blend(f32x4::ONE, f32x4::ZERO)
                        })
                    };
                }
                match operation {
                    ComparisonOperation::Equal => compare!(cmp_eq),
                    ComparisonOperation::NotEqual => compare!(cmp_ne),
                    ComparisonOperation::Less => compare!(cmp_lt),
                    ComparisonOperation::Greater => compare!(cmp_gt),
                    ComparisonOperation::LessEqual => compare!(cmp_le),
                    ComparisonOperation::GreaterEqual => compare!(cmp_ge),
                }
            }
            ArrayOperation::Logic(operation, left, right) => {
                macro_rules! logic {
                    ($operator:tt) => {
                        binary!(*left, *right, |a: f32x4, b: f32x4| {
                            (a.cmp_ne(f32x4::ZERO) $operator b.cmp_ne(f32x4::ZERO))
                                .blend(f32x4::ONE, f32x4::ZERO)
                        })
                    };
                }
                match operation {
                    LogicOperation::And => logic!(&),
                    LogicOperation::Or => logic!(|),
                    LogicOperation::Xor => logic!(^),
                    LogicOperation::Not => unary!(*left, |value: f32x4| {
                        value.cmp_eq(f32x4::ZERO).blend(f32x4::ONE, f32x4::ZERO)
                    }),
                }
            }
            ArrayOperation::Binary(operation, left, right) => match operation {
                BinaryOperation::Add => binary!(*left, *right, |a, b| a + b),
                BinaryOperation::Subtract => binary!(*left, *right, |a, b| a - b),
                BinaryOperation::Multiply => binary!(*left, *right, |a, b| a * b),
                BinaryOperation::Divide => binary!(*left, *right, |a, b| a / b),
            },
            ArrayOperation::Unary(operation, input) => match operation {
                UnaryOperation::Sin => unary!(*input, f32x4::sin),
                UnaryOperation::Cos => unary!(*input, f32x4::cos),
                UnaryOperation::Sqrt => unary!(*input, f32x4::sqrt),
                UnaryOperation::Ceil => unary!(*input, f32x4::ceil),
            },
            ArrayOperation::Atan2(left, right) => binary!(*left, *right, |y: f32x4, x: f32x4| {
                // atan2's sign follows y, including negative zero. Restore it
                // after wide's internal arithmetic negation of a zero result.
                y.atan2(x).copysign(y)
            }),
            ArrayOperation::Sum2(terms) => product!(terms[0], |a, b, first| {
                product!(terms[1], |c, d, second| {
                    let (destination, [a, b, c, d]) =
                        product_columns(registers, stride, count, output, [a, b, c, d]);
                    for ((((destination, a), b), c), d) in
                        destination.iter_mut().zip(a).zip(b).zip(c).zip(d)
                    {
                        *destination = second(*c, *d, first(*a, *b, f32x4::ZERO));
                    }
                })
            }),
            ArrayOperation::Sum3(terms) => product!(terms[0], |a, b, first| {
                product!(terms[1], |c, d, second| {
                    product!(terms[2], |e, f, third| {
                        let (destination, [a, b, c, d, e, f]) =
                            product_columns(registers, stride, count, output, [a, b, c, d, e, f]);
                        for ((((((destination, a), b), c), d), e), f) in destination
                            .iter_mut()
                            .zip(a)
                            .zip(b)
                            .zip(c)
                            .zip(d)
                            .zip(e)
                            .zip(f)
                        {
                            let sum = first(*a, *b, f32x4::ZERO);
                            let sum = second(*c, *d, sum);
                            *destination = third(*e, *f, sum);
                        }
                    })
                })
            }),
            ArrayOperation::Sum4(terms) => product!(terms[0], |a, b, first| {
                product!(terms[1], |c, d, second| {
                    product!(terms[2], |e, f, third| {
                        product!(terms[3], |g, h, fourth| {
                            let (destination, [a, b, c, d, e, f, g, h]) = product_columns(
                                registers,
                                stride,
                                count,
                                output,
                                [a, b, c, d, e, f, g, h],
                            );
                            for ((((((((destination, a), b), c), d), e), f), g), h) in destination
                                .iter_mut()
                                .zip(a)
                                .zip(b)
                                .zip(c)
                                .zip(d)
                                .zip(e)
                                .zip(f)
                                .zip(g)
                                .zip(h)
                            {
                                let sum = first(*a, *b, f32x4::ZERO);
                                let sum = second(*c, *d, sum);
                                let sum = third(*e, *f, sum);
                                *destination = fourth(*g, *h, sum);
                            }
                        })
                    })
                })
            }),
            ArrayOperation::SumProducts(terms) => {
                registers[output * stride..output * stride + count].fill(f32x4::ZERO);
                for term in terms {
                    match *term {
                        ArrayProduct::Add(input) => {
                            let (destination, source) =
                                unary_columns(registers, stride, count, output, input);
                            for (destination, source) in destination.iter_mut().zip(source) {
                                *destination += *source;
                            }
                        }
                        ArrayProduct::Subtract(input) => {
                            let (destination, source) =
                                unary_columns(registers, stride, count, output, input);
                            for (destination, source) in destination.iter_mut().zip(source) {
                                *destination -= *source;
                            }
                        }
                        ArrayProduct::Multiply(left, right) => {
                            let (destination, left, right) =
                                binary_columns(registers, stride, count, output, left, right);
                            for ((destination, left), right) in
                                destination.iter_mut().zip(left).zip(right)
                            {
                                *destination = left.mul_add(*right, *destination);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn array_predicate_mask(
    predicate: &ScalarPredicate,
    registers: &[f32x4],
    stride: usize,
    group: usize,
) -> f32x4 {
    let operand = |input: ScalarOperand| match input {
        ScalarOperand::Register(column) => registers[column * stride + group],
        ScalarOperand::Constant(value) => f32x4::splat(value),
    };
    match predicate {
        ScalarPredicate::Value(input) => operand(*input).cmp_ne(f32x4::ZERO),
        ScalarPredicate::IsFinite(input) => operand(*input).is_finite(),
        ScalarPredicate::AbsoluteDifferenceWithin {
            left,
            right,
            tolerance,
        } => (operand(*left) - operand(*right))
            .abs()
            .cmp_le(operand(*tolerance)),
        ScalarPredicate::Compare {
            operation,
            left,
            right,
        } => comparison_mask_simd(*operation, operand(*left), operand(*right)),
        ScalarPredicate::All(inputs) => inputs.iter().fold(!f32x4::ZERO, |valid, input| {
            valid & array_predicate_mask(input, registers, stride, group)
        }),
        ScalarPredicate::Logic { operation, inputs } => {
            let left = array_predicate_mask(&inputs[0], registers, stride, group);
            match operation {
                LogicOperation::And => {
                    left & array_predicate_mask(&inputs[1], registers, stride, group)
                }
                LogicOperation::Or => {
                    left | array_predicate_mask(&inputs[1], registers, stride, group)
                }
                LogicOperation::Xor => {
                    left ^ array_predicate_mask(&inputs[1], registers, stride, group)
                }
                LogicOperation::Not => !left,
            }
        }
    }
}
