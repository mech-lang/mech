//! Capability admission and fused WGSL lowering for typed Mech programs.

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use mech_compute::{
    BinaryOperation, ComputeKernel, ComputeProgram, ElementwiseInstruction, ElementwiseIr,
    ElementwiseLowering, ElementwiseOperation, ElementwiseStateStorage, ElementwiseStoragePlan,
    UnaryOperation, build_compute_region_interface, display_operation, elementwise_lowering,
    plan_compute_artifact, turn_required_nodes,
};
pub use mech_compute::{
    ComputeAdmissionError as GpuAdmissionError, ComputeDiagnostic as GpuDiagnostic,
    ComputeDiagnosticCode as GpuDiagnosticCode, ComputeExecutionTarget as ExecutionTarget,
    ComputePhysicalPlan as HybridPlacementPlan, NodePlacement, PlacementViolation, SlotPlacement,
    SlotResidence, TransferBoundary, TransferDirection,
};
use mech_core::snapshot::SequenceView;
use mech_core::{
    AccessMode, AliasPolicy, CellSlotId, ChangeDetectionPolicy, DeliveryMode, DimensionExpr,
    ExternalInteraction, FloatWidth, NodeId, OutputConstruction, ResolvedOperationContract,
    SchemaBody, SchemaId, ValueData,
};
use mech_engine::{
    ArtifactSource, BindingDeclaration, ComputeRegionDeclaration, ProducerReference,
    ProgramArtifact, SlotRole,
};

mod batched;
pub use batched::*;
#[cfg(feature = "native")]
mod native;
#[cfg(feature = "native")]
pub use native::*;
#[cfg(feature = "native")]
mod compute_provider;
#[cfg(feature = "native")]
pub use compute_provider::*;
#[cfg(feature = "native")]
mod compute_backends;
#[cfg(feature = "native")]
pub use compute_backends::*;
pub const WORKGROUP_SIZE: u32 = 64;

#[cfg(test)]
fn empty_compute_program() -> ComputeProgram {
    ComputeProgram::new(
        Default::default(),
        Default::default(),
        ComputeKernel::Elementwise(Default::default()),
    )
}

/// Converts a Mech matrix snapshot into the row-major storage order consumed
/// by generated GPU and fused CPU kernels.
pub fn column_major_to_row_major<T: Copy + Default>(
    rows: usize,
    columns: usize,
    values: &[T],
) -> Result<Vec<T>, String> {
    mech_compute::column_major_to_row_major(&[rows as u64, columns as u64], values)
        .map_err(|error| error.to_string())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuBindingAccess {
    Read,
    ReadWrite,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuBinding {
    pub binding: u32,
    pub name: String,
    pub access: GpuBindingAccess,
    pub elements: u64,
    kind: GpuBindingKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuBindingRole {
    Input,
    StateRead,
    StateWrite,
    Output,
}

impl GpuBinding {
    pub const fn role(&self) -> GpuBindingRole {
        match self.kind {
            GpuBindingKind::Input(_) => GpuBindingRole::Input,
            GpuBindingKind::StateRead(_) => GpuBindingRole::StateRead,
            GpuBindingKind::StateWrite(_) => GpuBindingRole::StateWrite,
            GpuBindingKind::Output(_) => GpuBindingRole::Output,
        }
    }

    pub const fn slot(&self) -> CellSlotId {
        match self.kind {
            GpuBindingKind::Input(slot)
            | GpuBindingKind::StateRead(slot)
            | GpuBindingKind::StateWrite(slot)
            | GpuBindingKind::Output(slot) => slot,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GpuBindingKind {
    Input(CellSlotId),
    StateRead(CellSlotId),
    StateWrite(CellSlotId),
    Output(CellSlotId),
}

#[derive(Clone, Debug)]
struct KernelOutput {
    name: String,
    source: CellSlotId,
    elements: u64,
    dimensions: Vec<u64>,
    #[cfg_attr(not(feature = "native"), allow(dead_code))]
    binding: u32,
}

#[derive(Clone, Debug)]
struct KernelState {
    slot: CellSlotId,
    source: ArtifactSource,
    elements: u64,
    initializer: Vec<f32>,
}

#[derive(Clone, Debug)]
pub struct GpuProgram {
    compute: ComputeProgram,
    wgsl: String,
    bindings: Vec<GpuBinding>,
    outputs: Vec<KernelOutput>,
    states: Vec<KernelState>,
    input_slots: BTreeMap<CellSlotId, (String, u64, u32)>,
    constants: BTreeMap<mech_core::ConstantId, Vec<f32>>,
    dispatch_elements: u64,
}

pub struct ResidentCpuSession<'a> {
    program: &'a GpuProgram,
    slots: BTreeMap<CellSlotId, Vec<f32>>,
}

pub struct OwnedResidentCpuSession {
    program: GpuProgram,
    slots: BTreeMap<CellSlotId, Vec<f32>>,
}

impl GpuProgram {
    /// Materializes backend-private bindings and shader text from the
    /// self-contained backend-neutral program. No compiler artifact is needed
    /// after this boundary.
    pub fn from_compute_program(compute: &ComputeProgram) -> Result<Self, GpuAdmissionError> {
        if !matches!(compute.kernel(), ComputeKernel::Elementwise(_)) {
            return Err(compute_program_rejection(
                GpuDiagnosticCode::OperationUnsupported,
                "the elementwise backend cannot compile a fixed-shape kernel",
            ));
        }
        let storage = compute.elementwise_storage().ok_or_else(|| {
            compute_program_rejection(
                GpuDiagnosticCode::ArtifactMalformed,
                "the compute program has no elementwise storage plan",
            )
        })?;
        let mut bindings = Vec::new();
        let mut input_slots = BTreeMap::new();
        for input in &compute.interface().inputs {
            let elements = u64::try_from(input.elements().map_err(|error| {
                compute_program_rejection(
                    GpuDiagnosticCode::ShapeMismatch,
                    format!("input `{}` has an invalid shape: {error}", input.name),
                )
            })?)
            .map_err(|_| {
                compute_program_rejection(
                    GpuDiagnosticCode::ShapeMismatch,
                    format!("input `{}` element count exceeds u64", input.name),
                )
            })?;
            let binding = bindings.len() as u32;
            bindings.push(GpuBinding {
                binding,
                name: input.name.to_string(),
                access: GpuBindingAccess::Read,
                elements,
                kind: GpuBindingKind::Input(input.slot),
            });
            input_slots.insert(input.slot, (input.name.to_string(), elements, binding));
        }

        let states = storage
            .states
            .iter()
            .map(|state| KernelState {
                slot: state.slot,
                source: state.source,
                elements: state.elements,
                initializer: state.initializer.to_vec(),
            })
            .collect::<Vec<_>>();
        let mut state_write_bindings = BTreeMap::new();
        for state in &states {
            let binding = bindings.len() as u32;
            bindings.push(GpuBinding {
                binding,
                name: format!("state.{}.read", state.slot.get()),
                access: GpuBindingAccess::Read,
                elements: state.elements,
                kind: GpuBindingKind::StateRead(state.slot),
            });
        }
        for state in &states {
            let binding = bindings.len() as u32;
            bindings.push(GpuBinding {
                binding,
                name: format!("state.{}.write", state.slot.get()),
                access: GpuBindingAccess::ReadWrite,
                elements: state.elements,
                kind: GpuBindingKind::StateWrite(state.slot),
            });
            state_write_bindings.insert(state.slot, binding);
        }

        let mut outputs = Vec::new();
        for output in &compute.interface().outputs {
            let elements = u64::try_from(output.elements().map_err(|error| {
                compute_program_rejection(
                    GpuDiagnosticCode::ShapeMismatch,
                    format!("output `{}` has an invalid shape: {error}", output.name),
                )
            })?)
            .map_err(|_| {
                compute_program_rejection(
                    GpuDiagnosticCode::ShapeMismatch,
                    format!("output `{}` element count exceeds u64", output.name),
                )
            })?;
            let binding = if let Some(binding) = state_write_bindings.get(&output.slot) {
                *binding
            } else {
                let binding = bindings.len() as u32;
                bindings.push(GpuBinding {
                    binding,
                    name: output.name.to_string(),
                    access: GpuBindingAccess::ReadWrite,
                    elements,
                    kind: GpuBindingKind::Output(output.slot),
                });
                binding
            };
            outputs.push(KernelOutput {
                name: output.name.to_string(),
                source: output.slot,
                elements,
                dimensions: output.dimensions.to_vec(),
                binding,
            });
        }

        let mut program = Self {
            compute: compute.clone(),
            wgsl: String::new(),
            bindings,
            outputs,
            states,
            input_slots,
            constants: storage
                .constants
                .iter()
                .map(|(id, values)| (*id, values.to_vec()))
                .collect(),
            dispatch_elements: storage.dispatch_elements,
        };
        program.wgsl = program.generate_wgsl();
        Ok(program)
    }

    pub fn compute_program(&self) -> &ComputeProgram {
        &self.compute
    }

    pub fn wgsl(&self) -> &str {
        &self.wgsl
    }

    pub fn bindings(&self) -> &[GpuBinding] {
        &self.bindings
    }

    pub fn outputs(&self) -> impl Iterator<Item = (&str, CellSlotId, u64)> {
        self.outputs
            .iter()
            .map(|output| (output.name.as_str(), output.source, output.elements))
    }

    pub fn output_dimensions(&self, slot: CellSlotId) -> Option<&[u64]> {
        self.outputs
            .iter()
            .find(|output| output.source == slot)
            .map(|output| output.dimensions.as_slice())
    }

    pub fn state_initializers(&self) -> impl Iterator<Item = (CellSlotId, u64, &[f32])> {
        self.states
            .iter()
            .map(|state| (state.slot, state.elements, state.initializer.as_slice()))
    }

    pub const fn dispatch_elements(&self) -> u64 {
        self.dispatch_elements
    }

    pub fn workgroup_count(&self) -> u32 {
        self.dispatch_elements.div_ceil(u64::from(WORKGROUP_SIZE)) as u32
    }

    fn generate_wgsl(&self) -> String {
        let mut shader = String::from("// Generated from a typed Mech ComputeProgram.\n");
        for binding in &self.bindings {
            let (name, access) = match binding.kind {
                GpuBindingKind::Input(slot) => (format!("input_{}", slot.get()), "read"),
                GpuBindingKind::StateRead(slot) => (format!("state_read_{}", slot.get()), "read"),
                GpuBindingKind::StateWrite(slot) => {
                    (format!("state_write_{}", slot.get()), "read_write")
                }
                GpuBindingKind::Output(slot) => (format!("output_{}", slot.get()), "read_write"),
            };
            shader.push_str(&format!(
                "@group(0) @binding({}) var<storage, {access}> {name}: array<f32>;\n",
                binding.binding
            ));
        }
        shader.push_str(&format!(
            "\n@compute @workgroup_size({WORKGROUP_SIZE})\nfn main(@builtin(global_invocation_id) gid: vec3<u32>) {{\n  let index = gid.x;\n  if (index >= {}u) {{ return; }}\n",
            self.dispatch_elements
        ));
        let ComputeKernel::Elementwise(ir) = self.compute.kernel() else {
            unreachable!("elementwise GPU program contains a fixed-shape kernel")
        };
        for instruction in &ir.instructions {
            match instruction {
                ElementwiseInstruction::Apply {
                    operation,
                    inputs,
                    output,
                    elements,
                } => {
                    let inputs = inputs
                        .iter()
                        .map(|source| self.wgsl_source(*source, *elements))
                        .collect::<Vec<_>>();
                    shader.push_str(&format!(
                        "  var slot_{} = 0.0;\n  if (index < {elements}u) {{ slot_{} = {}; }}\n",
                        output.get(),
                        output.get(),
                        wgsl_elementwise_expression(*operation, &inputs)
                    ));
                }
                ElementwiseInstruction::Concat2 {
                    left,
                    right,
                    output,
                    left_elements,
                    right_elements,
                } => {
                    let elements = instruction.elements();
                    let left = self.wgsl_source_at(*left, *left_elements, "index");
                    let local_index = format!("concat_index_{}", output.get());
                    let right = self.wgsl_source_at(*right, *right_elements, &local_index);
                    shader.push_str(&wgsl_concat2_instruction(
                        *output,
                        *left_elements,
                        elements,
                        &left,
                        &right,
                    ));
                }
            }
        }
        for state in &self.states {
            let source = self.wgsl_source(state.source, state.elements);
            shader.push_str(&format!(
                "  if (index < {}u) {{ state_write_{}[index] = {source}; }}\n",
                state.elements,
                state.slot.get()
            ));
        }
        for output in &self.outputs {
            if self.states.iter().any(|state| state.slot == output.source) {
                continue;
            }
            let source = if self.input_slots.contains_key(&output.source) {
                self.wgsl_slot(output.source, output.elements)
            } else {
                format!("slot_{}", output.source.get())
            };
            if output.elements == self.dispatch_elements {
                shader.push_str(&format!(
                    "  output_{}[index] = {source};\n",
                    output.source.get()
                ));
            } else {
                shader.push_str(&format!(
                    "  if (index < {}u) {{ output_{}[index] = {source}; }}\n",
                    output.elements,
                    output.source.get()
                ));
            }
        }
        shader.push_str("}\n");
        shader
    }

    fn wgsl_source(&self, source: ArtifactSource, consumer_elements: u64) -> String {
        self.wgsl_source_at(source, consumer_elements, "index")
    }

    fn wgsl_source_at(
        &self,
        source: ArtifactSource,
        consumer_elements: u64,
        index: &str,
    ) -> String {
        match source {
            ArtifactSource::Slot(slot) => self.wgsl_slot_at(slot, consumer_elements, index),
            ArtifactSource::Constant(constant) => {
                let values = &self.constants[&constant];
                if values.len() == 1 {
                    format_wgsl_f32(values[0])
                } else {
                    let elements = values.len() as u64;
                    let rendered = values
                        .iter()
                        .copied()
                        .map(format_wgsl_f32)
                        .collect::<Vec<_>>()
                        .join(", ");
                    let index = wgsl_broadcast_index(elements, consumer_elements, index);
                    format!("array<f32, {elements}>({rendered})[{index}]")
                }
            }
        }
    }

    fn wgsl_slot(&self, slot: CellSlotId, consumer_elements: u64) -> String {
        self.wgsl_slot_at(slot, consumer_elements, "index")
    }

    fn wgsl_slot_at(&self, slot: CellSlotId, consumer_elements: u64, index: &str) -> String {
        let elements = self
            .compute
            .elementwise_storage()
            .expect("elementwise GPU program has storage")
            .slot_elements[&slot];
        if self.input_slots.contains_key(&slot) {
            let index = wgsl_broadcast_index(elements, consumer_elements, index);
            format!("input_{}[{index}]", slot.get())
        } else if self.states.iter().any(|state| state.slot == slot) {
            let index = wgsl_broadcast_index(elements, consumer_elements, index);
            format!("state_read_{}[{index}]", slot.get())
        } else {
            format!("slot_{}", slot.get())
        }
    }

    /// Executes the admitted fused graph without transactional runtime machinery.
    /// This is the CPU backend and the reference used to check a GPU dispatch.
    pub fn run_cpu(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BTreeMap<String, Vec<f32>>, CpuExecutionError> {
        let mut session = self.prepare_cpu(inputs)?;
        session.dispatch_turns(1)?;
        session.outputs()
    }

    pub fn prepare_cpu(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<ResidentCpuSession<'_>, CpuExecutionError> {
        let slots = self.initial_cpu_slots(inputs)?;
        Ok(ResidentCpuSession {
            program: self,
            slots,
        })
    }

    pub fn into_cpu(
        self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<OwnedResidentCpuSession, CpuExecutionError> {
        let slots = self.initial_cpu_slots(inputs)?;
        Ok(OwnedResidentCpuSession {
            program: self,
            slots,
        })
    }

    fn initial_cpu_slots(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BTreeMap<CellSlotId, Vec<f32>>, CpuExecutionError> {
        let mut slots = BTreeMap::<CellSlotId, Vec<f32>>::new();
        for (slot, (name, elements, _)) in &self.input_slots {
            let values = inputs
                .get(name)
                .ok_or_else(|| CpuExecutionError::MissingInput { name: name.clone() })?;
            if values.len() != *elements as usize {
                return Err(CpuExecutionError::InputLength {
                    name: name.clone(),
                    expected: *elements,
                    actual: values.len(),
                });
            }
            slots.insert(*slot, values.clone());
        }
        for state in &self.states {
            slots.insert(state.slot, state.initializer.clone());
        }
        Ok(slots)
    }

    fn update_cpu_inputs(
        &self,
        slots: &mut BTreeMap<CellSlotId, Vec<f32>>,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<(), CpuExecutionError> {
        for (name, values) in inputs {
            let Some((slot, (_, elements, _))) = self
                .input_slots
                .iter()
                .find(|(_, (input_name, _, _))| input_name == name)
            else {
                return Err(CpuExecutionError::UnknownInput { name: name.clone() });
            };
            if values.len() != *elements as usize {
                return Err(CpuExecutionError::InputLength {
                    name: name.clone(),
                    expected: *elements,
                    actual: values.len(),
                });
            }
            slots.insert(*slot, values.clone());
        }
        Ok(())
    }

    fn execute_cpu_turn(
        &self,
        slots: &mut BTreeMap<CellSlotId, Vec<f32>>,
    ) -> Result<(), CpuExecutionError> {
        let ComputeKernel::Elementwise(ir) = self.compute.kernel() else {
            unreachable!("elementwise GPU program contains a fixed-shape kernel")
        };
        for instruction in &ir.instructions {
            let (output_slot, output) = match instruction {
                ElementwiseInstruction::Apply {
                    operation,
                    inputs,
                    output,
                    elements,
                } => {
                    let mut values = Vec::with_capacity(*elements as usize);
                    for index in 0..*elements as usize {
                        let inputs = inputs
                            .iter()
                            .map(|source| {
                                cpu_source_value(
                                    *source,
                                    index,
                                    *elements as usize,
                                    slots,
                                    &self.constants,
                                )
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        values.push(operation.apply(&inputs));
                    }
                    (*output, values)
                }
                ElementwiseInstruction::Concat2 {
                    left,
                    right,
                    output,
                    left_elements,
                    right_elements,
                } => {
                    let mut values = Vec::with_capacity(instruction.elements() as usize);
                    for index in 0..*left_elements as usize {
                        values.push(cpu_source_value(
                            *left,
                            index,
                            *left_elements as usize,
                            slots,
                            &self.constants,
                        )?);
                    }
                    for index in 0..*right_elements as usize {
                        values.push(cpu_source_value(
                            *right,
                            index,
                            *right_elements as usize,
                            slots,
                            &self.constants,
                        )?);
                    }
                    (*output, values)
                }
            };
            slots.insert(output_slot, output);
        }
        let next_states = self
            .states
            .iter()
            .map(|state| {
                let values = (0..state.elements as usize)
                    .map(|index| {
                        cpu_source_value(
                            state.source,
                            index,
                            state.elements as usize,
                            slots,
                            &self.constants,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok((state.slot, values))
            })
            .collect::<Result<Vec<_>, CpuExecutionError>>()?;
        for (slot, values) in next_states {
            slots.insert(slot, values);
        }
        Ok(())
    }

    fn cpu_outputs(
        &self,
        slots: &BTreeMap<CellSlotId, Vec<f32>>,
    ) -> Result<BTreeMap<String, Vec<f32>>, CpuExecutionError> {
        self.outputs
            .iter()
            .map(|output| {
                slots
                    .get(&output.source)
                    .cloned()
                    .map(|values| (output.name.clone(), values))
                    .ok_or(CpuExecutionError::MissingSlot {
                        slot: output.source,
                    })
            })
            .collect()
    }

    fn cpu_output(
        &self,
        slots: &BTreeMap<CellSlotId, Vec<f32>>,
        name: &str,
    ) -> Result<Vec<f32>, CpuExecutionError> {
        let output = self
            .outputs
            .iter()
            .find(|output| output.name == name)
            .ok_or_else(|| CpuExecutionError::UnknownOutput {
                name: name.to_owned(),
            })?;
        slots
            .get(&output.source)
            .cloned()
            .ok_or(CpuExecutionError::MissingSlot {
                slot: output.source,
            })
    }
}

fn compute_program_rejection(
    code: GpuDiagnosticCode,
    detail: impl Into<String>,
) -> GpuAdmissionError {
    GpuAdmissionError {
        diagnostics: vec![GpuDiagnostic {
            code,
            node: None,
            operation: None,
            detail: detail.into(),
        }],
    }
}

impl ResidentCpuSession<'_> {
    pub fn update_inputs(
        &mut self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<(), CpuExecutionError> {
        self.program.update_cpu_inputs(&mut self.slots, inputs)
    }

    pub fn dispatch_turns(&mut self, turns: u32) -> Result<(), CpuExecutionError> {
        if turns == 0 {
            return Err(CpuExecutionError::ZeroTurns);
        }
        for _ in 0..turns {
            self.program.execute_cpu_turn(&mut self.slots)?;
        }
        Ok(())
    }

    pub fn outputs(&self) -> Result<BTreeMap<String, Vec<f32>>, CpuExecutionError> {
        self.program.cpu_outputs(&self.slots)
    }

    pub fn output(&self, name: &str) -> Result<Vec<f32>, CpuExecutionError> {
        self.program.cpu_output(&self.slots, name)
    }
}

impl OwnedResidentCpuSession {
    pub(crate) fn program_ref(&self) -> &GpuProgram {
        &self.program
    }

    pub fn update_inputs(
        &mut self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<(), CpuExecutionError> {
        self.program.update_cpu_inputs(&mut self.slots, inputs)
    }

    pub fn dispatch_turns(&mut self, turns: u32) -> Result<(), CpuExecutionError> {
        if turns == 0 {
            return Err(CpuExecutionError::ZeroTurns);
        }
        for _ in 0..turns {
            self.program.execute_cpu_turn(&mut self.slots)?;
        }
        Ok(())
    }

    pub fn outputs(&self) -> Result<BTreeMap<String, Vec<f32>>, CpuExecutionError> {
        self.program.cpu_outputs(&self.slots)
    }

    pub fn output(&self, name: &str) -> Result<Vec<f32>, CpuExecutionError> {
        self.program.cpu_output(&self.slots, name)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CpuExecutionError {
    ZeroTurns,
    MissingInput {
        name: String,
    },
    UnknownInput {
        name: String,
    },
    UnknownOutput {
        name: String,
    },
    InputLength {
        name: String,
        expected: u64,
        actual: usize,
    },
    MissingSlot {
        slot: CellSlotId,
    },
    IndexOutOfBounds {
        slot: CellSlotId,
        index: usize,
    },
    MissingConstant {
        constant: mech_core::ConstantId,
    },
}

impl fmt::Display for CpuExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for CpuExecutionError {}

#[derive(Clone, Debug, Default)]
pub struct GpuHost;

impl GpuHost {
    /// Explains placement and transfer boundaries without selecting a backend.
    pub fn plan(&self, artifact: &ProgramArtifact) -> HybridPlacementPlan {
        plan_compute_artifact(artifact, artifact.compute_regions())
    }

    pub fn compile(&self, artifact: &ProgramArtifact) -> Result<GpuProgram, GpuAdmissionError> {
        self.compile_for_regions(artifact, artifact.compute_regions())
    }

    fn compile_for_regions(
        &self,
        artifact: &ProgramArtifact,
        regions: &[ComputeRegionDeclaration],
    ) -> Result<GpuProgram, GpuAdmissionError> {
        let plan = self.plan(artifact);
        let mut diagnostics = plan
            .violations
            .iter()
            .map(|violation| GpuDiagnostic {
                code: GpuDiagnosticCode::PlacementConstraintUnsatisfied,
                node: violation.node,
                operation: None,
                detail: format!("region `{}`: {}", violation.region, violation.reason),
            })
            .collect::<Vec<_>>();
        for region in regions
            .iter()
            .filter(|region| region.placement == mech_core::ComputePlacement::Cpu)
        {
            diagnostics.push(GpuDiagnostic {
                code: GpuDiagnosticCode::PlacementConstraintUnsatisfied,
                node: region.nodes.first().copied(),
                operation: None,
                detail: format!(
                    "region `{}` requires CPU execution and cannot be lowered by the GPU-only executor",
                    region.name,
                ),
            });
        }
        if plan.regions.len() > 1 {
            diagnostics.push(GpuDiagnostic {
                code: GpuDiagnosticCode::PlacementConstraintUnsatisfied,
                node: None,
                operation: None,
                detail: format!(
                    "{} GPU regions require a mixed multi-kernel executor; the current executor accepts one region",
                    plan.regions.len(),
                ),
            });
        }
        if !diagnostics.is_empty() {
            return Err(GpuAdmissionError { diagnostics });
        }
        Compiler::new(artifact).compile()
    }

    pub fn compile_cpu(&self, artifact: &ProgramArtifact) -> Result<GpuProgram, GpuAdmissionError> {
        let regions = artifact.compute_regions();
        let diagnostics = regions
            .iter()
            .filter(|region| region.placement == mech_core::ComputePlacement::Gpu)
            .map(|region| GpuDiagnostic {
                code: GpuDiagnosticCode::PlacementConstraintUnsatisfied,
                node: region.nodes.first().copied(),
                operation: None,
                detail: format!(
                    "region `{}` requires GPU execution and cannot run under the CPU executor",
                    region.name,
                ),
            })
            .collect::<Vec<_>>();
        if !diagnostics.is_empty() {
            return Err(GpuAdmissionError { diagnostics });
        }
        Compiler::new(artifact).compile()
    }
}

struct Compiler<'a> {
    artifact: &'a ProgramArtifact,
    diagnostics: Vec<GpuDiagnostic>,
    slot_elements: BTreeMap<CellSlotId, u64>,
    input_slots: BTreeMap<CellSlotId, (String, u64, u32)>,
    constants: BTreeMap<mech_core::ConstantId, Vec<f32>>,
    operations: Vec<ElementwiseInstruction>,
    outputs: Vec<KernelOutput>,
    state_slots: BTreeMap<CellSlotId, PendingState>,
    composite_packs: BTreeMap<CellSlotId, Vec<ArtifactSource>>,
    bindings: Vec<GpuBinding>,
}

#[derive(Clone, Debug)]
struct PendingState {
    source: Option<ArtifactSource>,
    elements: u64,
    initializer: Vec<f32>,
    read_binding: Option<u32>,
    write_binding: Option<u32>,
}

impl<'a> Compiler<'a> {
    fn new(artifact: &'a ProgramArtifact) -> Self {
        Self {
            artifact,
            diagnostics: Vec::new(),
            slot_elements: BTreeMap::new(),
            input_slots: BTreeMap::new(),
            constants: BTreeMap::new(),
            operations: Vec::new(),
            outputs: Vec::new(),
            state_slots: BTreeMap::new(),
            composite_packs: BTreeMap::new(),
            bindings: Vec::new(),
        }
    }

    fn compile(mut self) -> Result<GpuProgram, GpuAdmissionError> {
        self.validate_program_surface();
        self.lower_inputs();
        self.lower_nodes();
        self.lower_state_writes();
        self.lower_outputs();
        if !self.diagnostics.is_empty() {
            return Err(GpuAdmissionError {
                diagnostics: self.diagnostics,
            });
        }

        let dispatch_elements = self
            .outputs
            .iter()
            .map(|output| output.elements)
            .chain(self.state_slots.values().map(|state| state.elements))
            .max()
            .unwrap_or(1);
        let states = self
            .state_slots
            .iter()
            .map(|(slot, state)| KernelState {
                slot: *slot,
                source: state.source.expect("validated state has a producer source"),
                elements: state.elements,
                initializer: state.initializer.clone(),
            })
            .collect::<Vec<_>>();
        let storage_states = self
            .state_slots
            .into_iter()
            .map(|(slot, state)| ElementwiseStateStorage {
                slot,
                source: state.source.expect("validated state has a producer source"),
                elements: state.elements,
                initializer: state.initializer.into(),
            })
            .collect::<Vec<_>>();
        let interface =
            build_compute_region_interface(self.artifact, self.artifact.compute_regions().first())?;
        let plan = plan_compute_artifact(self.artifact, self.artifact.compute_regions());
        let kernel = ComputeKernel::Elementwise(ElementwiseIr {
            instructions: self.operations.into_boxed_slice(),
        });
        let compute = ComputeProgram::new(interface, plan, kernel).with_elementwise_storage(
            ElementwiseStoragePlan {
                slot_elements: self.slot_elements.clone(),
                constants: self
                    .constants
                    .iter()
                    .map(|(id, values)| (*id, values.clone().into()))
                    .collect(),
                states: storage_states.into_boxed_slice(),
                dispatch_elements,
            },
        );
        let mut program = GpuProgram {
            compute,
            wgsl: String::new(),
            bindings: self.bindings,
            outputs: self.outputs,
            states,
            input_slots: self.input_slots,
            constants: self.constants,
            dispatch_elements,
        };
        program.wgsl = program.generate_wgsl();
        Ok(program)
    }

    fn validate_program_surface(&mut self) {
        if !self.artifact.constraints().is_empty() {
            self.reject(
                GpuDiagnosticCode::IntegrityConstraintsUnsupported,
                None,
                None,
                "integrity constraints require transactional validation and are not admitted",
            );
        }
        for slot in self.artifact.slots() {
            if slot.role == SlotRole::Output {
                continue;
            }
            if self.is_composite_pack_slot(slot.slot) {
                continue;
            }
            match self.schema_elements(slot.schema) {
                Ok(elements) => {
                    self.slot_elements.insert(slot.slot, elements);
                    if slot.role == SlotRole::State {
                        match self.state_initializer(slot) {
                            Ok(initializer) => {
                                self.state_slots.insert(
                                    slot.slot,
                                    PendingState {
                                        source: None,
                                        elements,
                                        initializer,
                                        read_binding: None,
                                        write_binding: None,
                                    },
                                );
                            }
                            Err((code, detail)) => self.reject(
                                code,
                                producer_node(self.artifact, slot.producer),
                                None,
                                format!("state slot {}: {detail}", slot.slot.get()),
                            ),
                        }
                    }
                }
                Err((code, detail)) => self.reject(
                    code,
                    producer_node(self.artifact, slot.producer),
                    None,
                    format!("slot {}: {detail}", slot.slot.get()),
                ),
            }
        }
    }

    fn lower_inputs(&mut self) {
        let turn_nodes = turn_required_nodes(self.artifact);
        let mut required_slots = BTreeSet::new();
        for node in self
            .artifact
            .nodes()
            .iter()
            .filter(|node| turn_nodes.contains(&node.node))
        {
            for binding in node.input_bindings.clone() {
                if let Some(BindingDeclaration::Input {
                    source: ArtifactSource::Slot(slot),
                    ..
                }) = self.artifact.bindings().get(binding as usize)
                {
                    required_slots.insert(*slot);
                }
            }
        }
        required_slots.extend(self.artifact.outputs().iter().filter_map(|output| {
            match published_source(self.artifact, output.source) {
                ArtifactSource::Constant(_) => None,
                ArtifactSource::Slot(slot) => Some(slot),
            }
        }));
        for input in self.artifact.inputs() {
            if !required_slots.contains(&input.slot) {
                continue;
            }
            let Some(elements) = self.slot_elements.get(&input.slot).copied() else {
                continue;
            };
            let binding = self.bindings.len() as u32;
            self.bindings.push(GpuBinding {
                binding,
                name: input.name.clone(),
                access: GpuBindingAccess::Read,
                elements,
                kind: GpuBindingKind::Input(input.slot),
            });
            self.input_slots
                .insert(input.slot, (input.name.clone(), elements, binding));
        }
        let state_slots = self.state_slots.keys().copied().collect::<Vec<_>>();
        for slot in state_slots {
            let elements = self.state_slots[&slot].elements;
            let binding = self.bindings.len() as u32;
            self.bindings.push(GpuBinding {
                binding,
                name: format!("state.{}.read", slot.get()),
                access: GpuBindingAccess::Read,
                elements,
                kind: GpuBindingKind::StateRead(slot),
            });
            self.state_slots.get_mut(&slot).unwrap().read_binding = Some(binding);
        }
    }

    fn lower_nodes(&mut self) {
        let turn_nodes = turn_required_nodes(self.artifact);
        for node in self.artifact.nodes() {
            if !turn_nodes.contains(&node.node) {
                continue;
            }
            let operation_name = display_operation(&node.operation);
            if node.operation.module_path.as_ref() == ["core"]
                && node.operation.operation_name == "composite-pack"
            {
                let mut inputs = node
                    .input_bindings
                    .clone()
                    .filter_map(|index| match self.artifact.bindings().get(index as usize) {
                        Some(BindingDeclaration::Input { source, .. }) => Some(*source),
                        Some(_) | None => None,
                    })
                    .collect::<Vec<_>>();
                let output = node.output_bindings.clone().find_map(|index| {
                    match self.artifact.bindings().get(index as usize) {
                        Some(BindingDeclaration::Output { target, .. }) => Some(*target),
                        Some(_) | None => None,
                    }
                });
                if let Some(output) = output {
                    let output_schema = self.artifact.slots()[output.get() as usize].schema;
                    let has_template = inputs.first().is_some_and(|source| match source {
                        ArtifactSource::Constant(constant) => self
                            .artifact
                            .constants()
                            .get(*constant)
                            .is_some_and(|value| value.schema() == output_schema),
                        ArtifactSource::Slot(_) => false,
                    });
                    if has_template {
                        inputs.remove(0);
                    }
                }
                if let Some(output) = output.filter(|_| !inputs.is_empty()) {
                    self.composite_packs.insert(output, inputs);
                } else {
                    self.reject(
                        GpuDiagnosticCode::ArtifactMalformed,
                        Some(node.node),
                        Some(operation_name),
                        "composite pack must have inputs and one output",
                    );
                }
                continue;
            }
            let state_targets = node
                .output_bindings
                .clone()
                .filter_map(|index| match self.artifact.bindings().get(index as usize) {
                    Some(BindingDeclaration::Output { target, .. })
                        if self.state_slots.contains_key(target) =>
                    {
                        Some(*target)
                    }
                    Some(_) | None => None,
                })
                .collect::<Vec<_>>();
            if !state_targets.is_empty() {
                self.lower_state_commit(node, &operation_name, &state_targets);
                continue;
            }
            let Some(lowering) = elementwise_lowering(&node.operation) else {
                self.reject(
                    GpuDiagnosticCode::OperationUnsupported,
                    Some(node.node),
                    Some(operation_name),
                    "the GPU host supports only admitted element-wise arithmetic and trigonometry",
                );
                continue;
            };
            let host_proven_concatenation = matches!(
                lowering,
                ElementwiseLowering::Apply(ElementwiseOperation::Identity)
                    | ElementwiseLowering::Concat2
            );
            if !host_proven_concatenation
                && !self.admit_contract(node.node, &operation_name, node.contract)
            {
                continue;
            }
            let inputs = node
                .input_bindings
                .clone()
                .filter_map(|index| match self.artifact.bindings().get(index as usize) {
                    Some(BindingDeclaration::Input { source, .. }) => Some(*source),
                    Some(_) | None => None,
                })
                .collect::<Vec<_>>();
            let outputs = node
                .output_bindings
                .clone()
                .filter_map(|index| match self.artifact.bindings().get(index as usize) {
                    Some(BindingDeclaration::Output { target, .. }) => Some(*target),
                    Some(_) | None => None,
                })
                .collect::<Vec<_>>();
            if inputs.len() != lowering.arity() || outputs.len() != 1 {
                self.reject(
                    GpuDiagnosticCode::ArityUnsupported,
                    Some(node.node),
                    Some(operation_name),
                    format!(
                        "expected {} inputs and one output, found {} inputs and {} outputs",
                        lowering.arity(),
                        inputs.len(),
                        outputs.len()
                    ),
                );
                continue;
            }
            let Some(output_elements) = self.slot_elements.get(&outputs[0]).copied() else {
                continue;
            };
            let mut input_elements = Vec::with_capacity(2);
            let mut sources_valid = true;
            for source in &inputs {
                match self.source_elements(*source, node.node, &operation_name) {
                    Some(elements) => input_elements.push(elements),
                    None => sources_valid = false,
                }
            }
            if !sources_valid {
                continue;
            }
            let output_dimensions = self.slot_dimensions(outputs[0]);
            let shapes_compatible = match lowering {
                ElementwiseLowering::Concat2 => {
                    input_elements.len() == 2
                        && input_elements[0].checked_add(input_elements[1]) == Some(output_elements)
                }
                _ => inputs
                    .iter()
                    .zip(&input_elements)
                    .all(|(source, elements)| {
                        *elements == 1
                            || *elements == output_elements
                            || self.source_dimensions(*source).is_some_and(|dimensions| {
                                block_broadcast_dimensions(&dimensions, &output_dimensions)
                            })
                    }),
            };
            if !shapes_compatible {
                self.reject(
                    GpuDiagnosticCode::ShapeMismatch,
                    Some(node.node),
                    Some(operation_name),
                    format!(
                        "input element counts {input_elements:?} cannot broadcast to output count {output_elements}"
                    ),
                );
                continue;
            }
            if lowering != ElementwiseLowering::Concat2 {
                if let Some(source) = inputs.iter().find(|source| {
                    self.derived_broadcast_requires_materialization(**source, output_elements)
                }) {
                    self.reject(
                        GpuDiagnosticCode::DerivedBroadcastRequiresMaterialization,
                        Some(node.node),
                        Some(operation_name),
                        format!(
                            "derived source {source:?} must be materialized before remapped broadcasting"
                        ),
                    );
                    continue;
                }
            }
            let instruction = match lowering {
                ElementwiseLowering::Concat2 => ElementwiseInstruction::Concat2 {
                    left: inputs[0],
                    right: inputs[1],
                    output: outputs[0],
                    left_elements: input_elements[0],
                    right_elements: input_elements[1],
                },
                ElementwiseLowering::Apply(operation) => ElementwiseInstruction::Apply {
                    operation,
                    inputs: inputs.into_boxed_slice(),
                    output: outputs[0],
                    elements: output_elements,
                },
            };
            self.operations.push(instruction);
        }
    }

    fn lower_state_commit(
        &mut self,
        node: &mech_engine::NodeDeclaration,
        operation_name: &str,
        state_targets: &[CellSlotId],
    ) {
        if state_targets.len() != 1
            || node.operation.module_path.as_ref() != ["core"]
            || node.operation.operation_name != "assign"
        {
            self.reject(
                GpuDiagnosticCode::StateUnsupported,
                Some(node.node),
                Some(operation_name.to_owned()),
                "GPU state currently requires one whole-value Assign register",
            );
            return;
        }
        if !self.admit_state_contract(node.node, operation_name, node.contract) {
            return;
        }
        let inputs = node
            .input_bindings
            .clone()
            .filter_map(|index| match self.artifact.bindings().get(index as usize) {
                Some(BindingDeclaration::Input { source, .. }) => Some(*source),
                Some(_) | None => None,
            })
            .collect::<Vec<_>>();
        if inputs.len() != 1 {
            self.reject(
                GpuDiagnosticCode::ArityUnsupported,
                Some(node.node),
                Some(operation_name.to_owned()),
                format!("state Assign expected one input, found {}", inputs.len()),
            );
            return;
        }
        let target = state_targets[0];
        let Some(source_elements) = self.source_elements(inputs[0], node.node, operation_name)
        else {
            return;
        };
        if source_elements != self.state_slots[&target].elements {
            self.reject(
                GpuDiagnosticCode::ShapeMismatch,
                Some(node.node),
                Some(operation_name.to_owned()),
                format!(
                    "state source has {source_elements} elements but target has {}",
                    self.state_slots[&target].elements
                ),
            );
            return;
        }
        self.state_slots.get_mut(&target).unwrap().source = Some(inputs[0]);
    }

    fn lower_state_writes(&mut self) {
        let slots = self.state_slots.keys().copied().collect::<Vec<_>>();
        for slot in slots {
            if self.state_slots[&slot].source.is_none() {
                self.reject(
                    GpuDiagnosticCode::ArtifactMalformed,
                    producer_node(
                        self.artifact,
                        self.artifact.slots()[slot.get() as usize].producer,
                    ),
                    None,
                    format!("state slot {} has no admitted producer", slot.get()),
                );
                continue;
            }
            let elements = self.state_slots[&slot].elements;
            let binding = self.bindings.len() as u32;
            self.bindings.push(GpuBinding {
                binding,
                name: format!("state.{}.write", slot.get()),
                access: GpuBindingAccess::ReadWrite,
                elements,
                kind: GpuBindingKind::StateWrite(slot),
            });
            self.state_slots.get_mut(&slot).unwrap().write_binding = Some(binding);
        }
    }

    fn lower_outputs(&mut self) {
        let outputs = self.artifact.outputs().to_vec();
        for output in outputs {
            let source = published_source(self.artifact, output.source);
            let physical = self
                .composite_packs
                .get(&match source {
                    ArtifactSource::Slot(slot) => slot,
                    ArtifactSource::Constant(_) => output.source,
                })
                .map(|sources| {
                    sources
                        .iter()
                        .enumerate()
                        .map(|(index, source)| (format!("{}.{index}", output.name), *source))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_else(|| vec![(output.name.clone(), source)]);
            for (name, source) in physical {
                let ArtifactSource::Slot(source) = source else {
                    self.reject(
                        GpuDiagnosticCode::ConstantUnsupported,
                        None,
                        None,
                        format!("output {name} is a constant; constant outputs are not admitted"),
                    );
                    continue;
                };
                let Some(elements) = self.slot_elements.get(&source).copied() else {
                    continue;
                };
                let dimensions = self.slot_dimensions(source);
                if let Some(state) = self.state_slots.get(&source) {
                    let Some(binding) = state.write_binding else {
                        continue;
                    };
                    self.outputs.push(KernelOutput {
                        name,
                        source,
                        elements,
                        dimensions,
                        binding,
                    });
                    continue;
                }
                let binding = self.bindings.len() as u32;
                self.bindings.push(GpuBinding {
                    binding,
                    name: name.clone(),
                    access: GpuBindingAccess::ReadWrite,
                    elements,
                    kind: GpuBindingKind::Output(source),
                });
                self.outputs.push(KernelOutput {
                    name,
                    source,
                    elements,
                    dimensions,
                    binding,
                });
            }
        }
    }

    fn is_composite_pack_slot(&self, slot: CellSlotId) -> bool {
        let Some(declaration) = self.artifact.slots().get(slot.get() as usize) else {
            return false;
        };
        let mech_engine::ProducerReference::NodeOutput { node, .. } = declaration.producer else {
            return false;
        };
        self.artifact
            .nodes()
            .get(node.get() as usize)
            .is_some_and(|node| {
                node.operation.module_path.as_ref() == ["core"]
                    && node.operation.operation_name == "composite-pack"
            })
    }

    fn admit_contract(
        &mut self,
        node: NodeId,
        operation: &str,
        contract_id: mech_core::OperationContractId,
    ) -> bool {
        let Some(contract) = self.artifact.contracts().get(contract_id) else {
            self.reject(
                GpuDiagnosticCode::ArtifactMalformed,
                Some(node),
                Some(operation.to_owned()),
                format!("operation contract {} does not exist", contract_id.get()),
            );
            return false;
        };
        let ResolvedOperationContract::Declared(contract) = contract else {
            self.reject(
                GpuDiagnosticCode::OpaqueOperationContract,
                Some(node),
                Some(operation.to_owned()),
                "operation has a LegacyOpaque contract; the host cannot prove GPU safety",
            );
            return false;
        };
        if contract.interaction != ExternalInteraction::Pure {
            self.reject(
                GpuDiagnosticCode::ExternalInteractionUnsupported,
                Some(node),
                Some(operation.to_owned()),
                format!(
                    "operation interaction {:?} is not pure",
                    contract.interaction
                ),
            );
            return false;
        }
        let inputs_admitted = contract.inputs.iter().all(|input| {
            input.access == AccessMode::Read && input.delivery == DeliveryMode::Signal
        });
        let outputs_admitted = contract.outputs.iter().all(|output| {
            output.access == AccessMode::Write
                && output.delivery == DeliveryMode::Signal
                && matches!(output.construction, OutputConstruction::FullWrite { .. })
                && output.alias == AliasPolicy::NoAlias
                && matches!(
                    output.change_detection,
                    ChangeDetectionPolicy::KernelReported | ChangeDetectionPolicy::ExactScalar
                )
        });
        if !inputs_admitted || !outputs_admitted {
            self.reject(
                GpuDiagnosticCode::PortContractUnsupported,
                Some(node),
                Some(operation.to_owned()),
                "ports must be signal/read inputs and signal/full-write/no-alias outputs",
            );
            return false;
        }
        true
    }

    fn admit_state_contract(
        &mut self,
        node: NodeId,
        operation: &str,
        contract_id: mech_core::OperationContractId,
    ) -> bool {
        let Some(contract) = self.artifact.contracts().get(contract_id) else {
            self.reject(
                GpuDiagnosticCode::ArtifactMalformed,
                Some(node),
                Some(operation.to_owned()),
                format!("operation contract {} does not exist", contract_id.get()),
            );
            return false;
        };
        let ResolvedOperationContract::Declared(contract) = contract else {
            self.reject(
                GpuDiagnosticCode::OpaqueOperationContract,
                Some(node),
                Some(operation.to_owned()),
                "state operation has a LegacyOpaque contract",
            );
            return false;
        };
        let admitted = contract.interaction == ExternalInteraction::Pure
            && contract.inputs.len() == 1
            && contract.inputs[0].access == AccessMode::Read
            && contract.inputs[0].delivery == DeliveryMode::Signal
            && contract.outputs.len() == 1
            && contract.outputs[0].access == AccessMode::Write
            && contract.outputs[0].delivery == DeliveryMode::Signal
            && matches!(
                contract.outputs[0].construction,
                OutputConstruction::Replace { .. } | OutputConstruction::FullWrite { .. }
            )
            && contract.outputs[0].alias == AliasPolicy::NoAlias;
        if !admitted {
            self.reject(
                GpuDiagnosticCode::PortContractUnsupported,
                Some(node),
                Some(operation.to_owned()),
                "state Assign must be a pure full write with no aliases",
            );
        }
        admitted
    }

    fn state_initializer(
        &self,
        slot: &mech_engine::SlotDeclaration,
    ) -> Result<Vec<f32>, (GpuDiagnosticCode, String)> {
        let Some(mech_engine::InitializerReference::Constant(constant)) = slot.initializer else {
            return Err((
                GpuDiagnosticCode::StateUnsupported,
                "state has no constant initializer".to_owned(),
            ));
        };
        let value = self.artifact.constants().get(constant).ok_or_else(|| {
            (
                GpuDiagnosticCode::ArtifactMalformed,
                format!("initializer constant {} does not exist", constant.get()),
            )
        })?;
        let values = match value.data() {
            ValueData::F32(value) => vec![value.to_f32()],
            ValueData::Matrix(matrix) => match matrix.elements() {
                SequenceView::F32(values) => values.iter().map(|value| value.to_f32()).collect(),
                _ => {
                    return Err((
                        GpuDiagnosticCode::ConstantUnsupported,
                        "initializer is not an f32 matrix".to_owned(),
                    ));
                }
            },
            _ => {
                return Err((
                    GpuDiagnosticCode::ConstantUnsupported,
                    "initializer is not scalar f32 or an f32 matrix".to_owned(),
                ));
            }
        };
        if values.len() != self.slot_elements[&slot.slot] as usize {
            return Err((
                GpuDiagnosticCode::ShapeMismatch,
                format!(
                    "initializer has {} elements but the state schema has {}",
                    values.len(),
                    self.slot_elements[&slot.slot]
                ),
            ));
        }
        Ok(values)
    }

    fn schema_elements(&self, schema: SchemaId) -> Result<u64, (GpuDiagnosticCode, String)> {
        let schema = self.artifact.schemas().get(schema).ok_or_else(|| {
            (
                GpuDiagnosticCode::ArtifactMalformed,
                "schema does not exist".to_owned(),
            )
        })?;
        match schema.body() {
            SchemaBody::FloatingPoint(FloatWidth::W32) => Ok(1),
            SchemaBody::Matrix {
                element,
                dimensions,
            } if matches!(element.as_ref(), SchemaBody::FloatingPoint(FloatWidth::W32)) => {
                let mut elements = 1_u64;
                for dimension in dimensions {
                    let DimensionExpr::Constant(extent) = dimension else {
                        return Err((
                            GpuDiagnosticCode::DynamicShapeUnsupported,
                            format!("matrix dimension {dimension:?} is not compile-time constant"),
                        ));
                    };
                    elements = elements.checked_mul(*extent).ok_or_else(|| {
                        (
                            GpuDiagnosticCode::SchemaUnsupported,
                            "matrix element count overflows u64".to_owned(),
                        )
                    })?;
                }
                Ok(elements)
            }
            body => Err((
                GpuDiagnosticCode::SchemaUnsupported,
                format!("schema {body:?} is not scalar f32 or a fixed-size f32 matrix"),
            )),
        }
    }

    fn slot_dimensions(&self, slot: CellSlotId) -> Vec<u64> {
        let Some(declaration) = self.artifact.slots().get(slot.get() as usize) else {
            return Vec::new();
        };
        self.schema_dimensions(declaration.schema)
    }

    fn schema_dimensions(&self, schema: SchemaId) -> Vec<u64> {
        let Some(schema) = self.artifact.schemas().get(schema) else {
            return Vec::new();
        };
        match schema.body() {
            SchemaBody::Matrix { dimensions, .. } => dimensions
                .iter()
                .filter_map(|dimension| match dimension {
                    DimensionExpr::Constant(extent) => Some(*extent),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    fn source_dimensions(&self, source: ArtifactSource) -> Option<Vec<u64>> {
        match source {
            ArtifactSource::Slot(slot) => Some(self.slot_dimensions(slot)),
            ArtifactSource::Constant(constant) => self
                .artifact
                .constants()
                .get(constant)
                .map(|value| self.schema_dimensions(value.schema())),
        }
    }

    fn derived_broadcast_requires_materialization(
        &self,
        source: ArtifactSource,
        consumer_elements: u64,
    ) -> bool {
        let ArtifactSource::Slot(slot) = source else {
            return false;
        };
        if self.slot_elements.get(&slot) == Some(&consumer_elements) {
            return false;
        }
        self.artifact
            .slots()
            .get(slot.get() as usize)
            .is_some_and(|slot| slot.role == SlotRole::Derived)
    }

    fn source_elements(
        &mut self,
        source: ArtifactSource,
        node: NodeId,
        operation: &str,
    ) -> Option<u64> {
        match source {
            ArtifactSource::Slot(slot) => self.slot_elements.get(&slot).copied().or_else(|| {
                self.reject(
                    GpuDiagnosticCode::ArtifactMalformed,
                    Some(node),
                    Some(operation.to_owned()),
                    format!("input slot {} has no admitted schema", slot.get()),
                );
                None
            }),
            ArtifactSource::Constant(constant) => {
                let Some(value) = self.artifact.constants().get(constant) else {
                    self.reject(
                        GpuDiagnosticCode::ArtifactMalformed,
                        Some(node),
                        Some(operation.to_owned()),
                        format!("constant {} does not exist", constant.get()),
                    );
                    return None;
                };
                match value.data() {
                    ValueData::F32(value) => {
                        self.constants.insert(constant, vec![value.to_f32()]);
                        Some(1)
                    }
                    ValueData::Matrix(matrix) => match matrix.elements() {
                        SequenceView::F32(values) => {
                            let values = values
                                .iter()
                                .map(|value| value.to_f32())
                                .collect::<Vec<_>>();
                            let elements = values.len() as u64;
                            self.constants.insert(constant, values);
                            Some(elements)
                        }
                        _ => {
                            self.reject(
                                GpuDiagnosticCode::ConstantUnsupported,
                                Some(node),
                                Some(operation.to_owned()),
                                "only scalar and matrix f32 constants can be embedded",
                            );
                            None
                        }
                    },
                    _ => {
                        self.reject(
                            GpuDiagnosticCode::ConstantUnsupported,
                            Some(node),
                            Some(operation.to_owned()),
                            "only scalar f32 constants can be embedded",
                        );
                        None
                    }
                }
            }
        }
    }

    fn reject(
        &mut self,
        code: GpuDiagnosticCode,
        node: Option<NodeId>,
        operation: Option<String>,
        detail: impl Into<String>,
    ) {
        self.diagnostics.push(GpuDiagnostic {
            code,
            node,
            operation,
            detail: detail.into(),
        });
    }
}

fn producer_node(
    artifact: &ProgramArtifact,
    producer: mech_engine::ProducerReference,
) -> Option<NodeId> {
    match producer {
        mech_engine::ProducerReference::Input(_) => None,
        mech_engine::ProducerReference::NodeOutput { node, .. } => Some(node),
        mech_engine::ProducerReference::Output { source, .. } => match source {
            ArtifactSource::Constant(_) => None,
            ArtifactSource::Slot(slot) => artifact
                .slots()
                .get(slot.get() as usize)
                .and_then(|slot| producer_node(artifact, slot.producer)),
        },
    }
}

fn published_source(artifact: &ProgramArtifact, slot: CellSlotId) -> ArtifactSource {
    match artifact.slots()[slot.get() as usize].producer {
        ProducerReference::Output { source, .. } => source,
        _ => ArtifactSource::Slot(slot),
    }
}

fn wgsl_broadcast_index(elements: u64, consumer_elements: u64, index: &str) -> String {
    if elements == 1 {
        "0u".to_owned()
    } else if elements == consumer_elements {
        index.to_owned()
    } else if consumer_elements % elements == 0 {
        format!("({index}) / {}u", consumer_elements / elements)
    } else {
        index.to_owned()
    }
}

fn wgsl_elementwise_expression(operation: ElementwiseOperation, inputs: &[String]) -> String {
    match operation {
        ElementwiseOperation::Binary(operation) => {
            let operator = match operation {
                BinaryOperation::Add => "+",
                BinaryOperation::Subtract => "-",
                BinaryOperation::Multiply => "*",
                BinaryOperation::Divide => "/",
            };
            format!("{} {operator} {}", inputs[0], inputs[1])
        }
        ElementwiseOperation::Unary(operation) => {
            let function = match operation {
                UnaryOperation::Sin => "sin",
                UnaryOperation::Cos => "cos",
            };
            format!("{function}({})", inputs[0])
        }
        ElementwiseOperation::Atan2 => format!("atan2({}, {})", inputs[0], inputs[1]),
        ElementwiseOperation::Identity => inputs[0].clone(),
    }
}

fn wgsl_concat2_instruction(
    output: CellSlotId,
    left_elements: u64,
    elements: u64,
    left: &str,
    right: &str,
) -> String {
    let output = output.get();
    let local_index = format!("concat_index_{output}");
    format!(
        "  var slot_{output} = 0.0;\n  if (index < {left_elements}u) {{\n    slot_{output} = {left};\n  }} else if (index < {elements}u) {{\n    let {local_index} = index - {left_elements}u;\n    slot_{output} = {right};\n  }}\n"
    )
}

fn block_broadcast_dimensions(input: &[u64], output: &[u64]) -> bool {
    if input.len() != output.len() {
        return false;
    }
    let mut expanded = false;
    for (input, output) in input.iter().zip(output) {
        if input == output {
            if expanded && *input != 1 {
                return false;
            }
        } else if *input == 1 {
            expanded = true;
        } else {
            return false;
        }
    }
    expanded
}

fn format_wgsl_f32(value: f32) -> String {
    if value.is_nan() {
        return "bitcast<f32>(0x7fc00000u)".to_owned();
    }
    if value == f32::INFINITY {
        return "bitcast<f32>(0x7f800000u)".to_owned();
    }
    if value == f32::NEG_INFINITY {
        return "bitcast<f32>(0xff800000u)".to_owned();
    }
    let formatted = value.to_string();
    if formatted.contains(['.', 'e', 'E']) {
        formatted
    } else {
        format!("{formatted}.0")
    }
}

fn cpu_source_value(
    source: ArtifactSource,
    index: usize,
    consumer_elements: usize,
    slots: &BTreeMap<CellSlotId, Vec<f32>>,
    constants: &BTreeMap<mech_core::ConstantId, Vec<f32>>,
) -> Result<f32, CpuExecutionError> {
    match source {
        ArtifactSource::Constant(constant) => {
            let values = constants
                .get(&constant)
                .ok_or(CpuExecutionError::MissingConstant { constant })?;
            let source_index = broadcast_index(values.len(), index, consumer_elements);
            values
                .get(source_index)
                .copied()
                .ok_or(CpuExecutionError::MissingConstant { constant })
        }
        ArtifactSource::Slot(slot) => {
            let values = slots
                .get(&slot)
                .ok_or(CpuExecutionError::MissingSlot { slot })?;
            let source_index = broadcast_index(values.len(), index, consumer_elements);
            values
                .get(source_index)
                .copied()
                .ok_or(CpuExecutionError::IndexOutOfBounds { slot, index })
        }
    }
}

fn broadcast_index(source_elements: usize, index: usize, consumer_elements: usize) -> usize {
    if source_elements == 1 {
        0
    } else if source_elements == consumer_elements {
        index
    } else if consumer_elements % source_elements == 0 {
        index / (consumer_elements / source_elements)
    } else {
        index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::ConstantId;

    fn concat_program(left: &[f32], right: &[f32]) -> GpuProgram {
        let left_constant = ConstantId::new(0);
        let right_constant = ConstantId::new(1);
        let output = CellSlotId::new(0);
        let left_elements = left.len() as u64;
        let right_elements = right.len() as u64;
        let elements = left_elements.checked_add(right_elements).unwrap();
        let instruction = ElementwiseInstruction::Concat2 {
            left: ArtifactSource::Constant(left_constant),
            right: ArtifactSource::Constant(right_constant),
            output,
            left_elements,
            right_elements,
        };
        let compute = ComputeProgram::new(
            Default::default(),
            Default::default(),
            ComputeKernel::Elementwise(ElementwiseIr {
                instructions: vec![instruction].into_boxed_slice(),
            }),
        );
        let constants = BTreeMap::from([
            (left_constant, left.to_vec()),
            (right_constant, right.to_vec()),
        ]);

        GpuProgram {
            compute,
            wgsl: String::new(),
            bindings: Vec::new(),
            outputs: vec![KernelOutput {
                name: "result".to_owned(),
                source: output,
                elements,
                dimensions: vec![elements],
                binding: 0,
            }],
            states: Vec::new(),
            input_slots: BTreeMap::new(),
            constants,
            dispatch_elements: elements,
        }
    }

    fn assert_concat_cpu_wgsl_parity(left: &[f32], right: &[f32]) {
        let program = concat_program(left, right);
        let expected = left.iter().chain(right).copied().collect::<Vec<_>>();
        assert_eq!(
            program.run_cpu(&BTreeMap::new()).unwrap()["result"],
            expected
        );

        let left_elements = left.len() as u64;
        let elements = expected.len() as u64;
        let wgsl = wgsl_concat2_instruction(
            CellSlotId::new(0),
            left_elements,
            elements,
            "left[index]",
            "right[concat_index_0]",
        );
        assert!(wgsl.contains(&format!("if (index < {left_elements}u)")));
        assert!(wgsl.contains(&format!("let concat_index_0 = index - {left_elements}u")));
        assert!(wgsl.contains(&format!("else if (index < {elements}u)")));
        assert!(wgsl.contains("right[concat_index_0]"));
    }

    #[test]
    fn unequal_concatenation_has_cpu_wgsl_indexing_parity() {
        assert_concat_cpu_wgsl_parity(&[1.0, 2.0], &[3.0, 4.0, 5.0]);
    }

    #[test]
    fn equal_concatenation_has_cpu_wgsl_indexing_parity() {
        assert_concat_cpu_wgsl_parity(&[1.0, 2.0], &[3.0, 4.0]);
    }
}
