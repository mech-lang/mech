//! Capability admission and fused WGSL lowering for typed Mech programs.

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use mech_core::snapshot::SequenceView;
use mech_core::{
    AccessMode, AliasPolicy, CellSlotId, ChangeDetectionPolicy, DeliveryMode, DimensionExpr,
    ExternalInteraction, FloatWidth, NodeId, OutputConstruction, ResolvedOperationContract,
    SchemaBody, SchemaId, ValueData,
};
use mech_engine::{
    ArtifactSource, BindingDeclaration, OperationReference, ProducerReference, ProgramArtifact,
    SlotRole,
};

#[cfg(feature = "native")]
mod native;
#[cfg(feature = "native")]
pub use native::*;
mod placement;
pub use placement::*;

const WORKGROUP_SIZE: u32 = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuDiagnosticCode {
    IntegrityConstraintsUnsupported,
    StateUnsupported,
    OpaqueOperationContract,
    ExternalInteractionUnsupported,
    PortContractUnsupported,
    SchemaUnsupported,
    DynamicShapeUnsupported,
    OperationUnsupported,
    ArityUnsupported,
    ShapeMismatch,
    ConstantUnsupported,
    ArtifactMalformed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuDiagnostic {
    pub code: GpuDiagnosticCode,
    pub node: Option<NodeId>,
    pub operation: Option<String>,
    pub detail: String,
}

impl fmt::Display for GpuDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(node) = self.node {
            write!(formatter, "node {}: ", node.get())?;
        }
        if let Some(operation) = &self.operation {
            write!(formatter, "{operation}: ")?;
        }
        write!(formatter, "{}", self.detail)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuAdmissionError {
    diagnostics: Vec<GpuDiagnostic>,
}

impl GpuAdmissionError {
    pub fn diagnostics(&self) -> &[GpuDiagnostic] {
        &self.diagnostics
    }
}

impl fmt::Display for GpuAdmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "GPU host rejected the program with {} diagnostic(s)",
            self.diagnostics.len()
        )?;
        for diagnostic in &self.diagnostics {
            write!(formatter, "\n- {diagnostic}")?;
        }
        Ok(())
    }
}

impl Error for GpuAdmissionError {}

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
enum GpuBindingKind {
    Input(CellSlotId),
    StateRead(CellSlotId),
    StateWrite(CellSlotId),
    Output(CellSlotId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BinaryOperation {
    Add,
    Subtract,
    Multiply,
}

impl BinaryOperation {
    const fn wgsl(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
        }
    }

    fn apply(self, left: f32, right: f32) -> f32 {
        match self {
            Self::Add => left + right,
            Self::Subtract => left - right,
            Self::Multiply => left * right,
        }
    }
}

#[derive(Clone, Debug)]
struct KernelOperation {
    operation: BinaryOperation,
    inputs: [ArtifactSource; 2],
    output: CellSlotId,
    elements: u64,
}

#[derive(Clone, Debug)]
struct KernelOutput {
    name: String,
    source: CellSlotId,
    elements: u64,
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
    wgsl: String,
    bindings: Vec<GpuBinding>,
    operations: Vec<KernelOperation>,
    outputs: Vec<KernelOutput>,
    states: Vec<KernelState>,
    input_slots: BTreeMap<CellSlotId, (String, u64, u32)>,
    constants: BTreeMap<mech_core::ConstantId, f32>,
    dispatch_elements: u64,
}

pub struct ResidentCpuSession<'a> {
    program: &'a GpuProgram,
    slots: BTreeMap<CellSlotId, Vec<f32>>,
}

impl GpuProgram {
    pub fn wgsl(&self) -> &str {
        &self.wgsl
    }

    pub fn bindings(&self) -> &[GpuBinding] {
        &self.bindings
    }

    pub const fn dispatch_elements(&self) -> u64 {
        self.dispatch_elements
    }

    pub fn workgroup_count(&self) -> u32 {
        self.dispatch_elements.div_ceil(u64::from(WORKGROUP_SIZE)) as u32
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
        Ok(ResidentCpuSession {
            program: self,
            slots,
        })
    }

    fn execute_cpu_turn(
        &self,
        slots: &mut BTreeMap<CellSlotId, Vec<f32>>,
    ) -> Result<(), CpuExecutionError> {
        for operation in &self.operations {
            let mut output = Vec::with_capacity(operation.elements as usize);
            for index in 0..operation.elements as usize {
                let left = cpu_source_value(operation.inputs[0], index, slots, &self.constants)?;
                let right = cpu_source_value(operation.inputs[1], index, slots, &self.constants)?;
                output.push(operation.operation.apply(left, right));
            }
            slots.insert(operation.output, output);
        }
        let next_states = self
            .states
            .iter()
            .map(|state| {
                let values = (0..state.elements as usize)
                    .map(|index| cpu_source_value(state.source, index, slots, &self.constants))
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
}

impl ResidentCpuSession<'_> {
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CpuExecutionError {
    ZeroTurns,
    MissingInput {
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
    pub fn compile(&self, artifact: &ProgramArtifact) -> Result<GpuProgram, GpuAdmissionError> {
        Compiler::new(artifact).compile()
    }
}

struct Compiler<'a> {
    artifact: &'a ProgramArtifact,
    diagnostics: Vec<GpuDiagnostic>,
    slot_elements: BTreeMap<CellSlotId, u64>,
    input_slots: BTreeMap<CellSlotId, (String, u64, u32)>,
    constants: BTreeMap<mech_core::ConstantId, f32>,
    operations: Vec<KernelOperation>,
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
        let wgsl = self.generate_wgsl(dispatch_elements);
        let states = self
            .state_slots
            .into_iter()
            .map(|(slot, state)| KernelState {
                slot,
                source: state.source.expect("validated state has a producer source"),
                elements: state.elements,
                initializer: state.initializer,
            })
            .collect();
        Ok(GpuProgram {
            wgsl,
            bindings: self.bindings,
            operations: self.operations,
            outputs: self.outputs,
            states,
            input_slots: self.input_slots,
            constants: self.constants,
            dispatch_elements,
        })
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
                                producer_node(slot.producer),
                                None,
                                format!("state slot {}: {detail}", slot.slot.get()),
                            ),
                        }
                    }
                }
                Err((code, detail)) => self.reject(
                    code,
                    producer_node(slot.producer),
                    None,
                    format!("slot {}: {detail}", slot.slot.get()),
                ),
            }
        }
    }

    fn lower_inputs(&mut self) {
        for input in self.artifact.inputs() {
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
            // Definitions establish source-level names and input/output identity.
            // The compiler keeps them as bytecode markers, but downstream slots
            // already refer to the value they name, so they are not GPU work.
            if node.operation.module_path.as_ref() == ["runtime"]
                && node.operation.operation_name.starts_with("VariableDefine")
            {
                continue;
            }
            if node.operation.module_path.as_ref() == ["core"]
                && node.operation.operation_name == "composite-pack"
            {
                let inputs = node
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
            if !self.admit_contract(node.node, &operation_name, node.contract) {
                continue;
            }
            let Some(operation) = binary_operation(&node.operation) else {
                self.reject(
                    GpuDiagnosticCode::OperationUnsupported,
                    Some(node.node),
                    Some(operation_name),
                    "the GPU host supports only element-wise math/add, math/sub, and math/mul",
                );
                continue;
            };
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
            if inputs.len() != 2 || outputs.len() != 1 {
                self.reject(
                    GpuDiagnosticCode::ArityUnsupported,
                    Some(node.node),
                    Some(operation_name),
                    format!(
                        "expected two inputs and one output, found {} inputs and {} outputs",
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
            if input_elements
                .iter()
                .any(|elements| *elements != 1 && *elements != output_elements)
            {
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
            self.operations.push(KernelOperation {
                operation,
                inputs: [inputs[0], inputs[1]],
                output: outputs[0],
                elements: output_elements,
            });
        }
    }

    fn lower_state_commit(
        &mut self,
        node: &mech_engine::NodeDeclaration,
        operation_name: &str,
        state_targets: &[CellSlotId],
    ) {
        if state_targets.len() != 1
            || node.operation.module_path.as_ref() != ["runtime"]
            || !node.operation.operation_name.starts_with("Assign")
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
                    producer_node(self.artifact.slots()[slot.get() as usize].producer),
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
            let physical = self
                .composite_packs
                .get(&output.source)
                .map(|sources| {
                    sources
                        .iter()
                        .enumerate()
                        .map(|(index, source)| (format!("{}.{index}", output.name), *source))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_else(|| {
                    vec![(output.name.clone(), ArtifactSource::Slot(output.source))]
                });
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
                if let Some(state) = self.state_slots.get(&source) {
                    let Some(binding) = state.write_binding else {
                        continue;
                    };
                    self.outputs.push(KernelOutput {
                        name,
                        source,
                        elements,
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
                OutputConstruction::Replace { .. }
            )
            && contract.outputs[0].alias == AliasPolicy::NoAlias;
        if !admitted {
            self.reject(
                GpuDiagnosticCode::PortContractUnsupported,
                Some(node),
                Some(operation.to_owned()),
                "state Assign must be pure whole-value replacement with no aliases",
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
                        self.constants.insert(constant, value.to_f32());
                        Some(1)
                    }
                    ValueData::Matrix(matrix) => match matrix.elements() {
                        SequenceView::F32(_) => {
                            self.reject(
                                GpuDiagnosticCode::ConstantUnsupported,
                                Some(node),
                                Some(operation.to_owned()),
                                "matrix constants are not embedded by this GPU slice; expose the matrix as a host input",
                            );
                            None
                        }
                        _ => {
                            self.reject(
                                GpuDiagnosticCode::ConstantUnsupported,
                                Some(node),
                                Some(operation.to_owned()),
                                "only scalar f32 constants can be embedded",
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

    fn generate_wgsl(&self, dispatch_elements: u64) -> String {
        let mut shader = String::from("// Generated from a typed Mech ProgramArtifact.\n");
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
            "\n@compute @workgroup_size({WORKGROUP_SIZE})\nfn main(@builtin(global_invocation_id) gid: vec3<u32>) {{\n  let index = gid.x;\n  if (index >= {dispatch_elements}u) {{ return; }}\n"
        ));
        for operation in &self.operations {
            let left = self.wgsl_source(operation.inputs[0], operation.elements);
            let right = self.wgsl_source(operation.inputs[1], operation.elements);
            shader.push_str(&format!(
                "  let slot_{} = {left} {} {right};\n",
                operation.output.get(),
                operation.operation.wgsl()
            ));
        }
        for state in &self.state_slots {
            let source = self.wgsl_source(
                state.1.source.expect("validated state source"),
                state.1.elements,
            );
            shader.push_str(&format!(
                "  if (index < {}u) {{ state_write_{}[index] = {source}; }}\n",
                state.1.elements,
                state.0.get()
            ));
        }
        for output in &self.outputs {
            if self.state_slots.contains_key(&output.source) {
                continue;
            }
            let source = if self.input_slots.contains_key(&output.source) {
                self.wgsl_slot(output.source, output.elements)
            } else {
                format!("slot_{}", output.source.get())
            };
            if output.elements == dispatch_elements {
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
        match source {
            ArtifactSource::Slot(slot) => self.wgsl_slot(slot, consumer_elements),
            ArtifactSource::Constant(constant) => format_wgsl_f32(self.constants[&constant]),
        }
    }

    fn wgsl_slot(&self, slot: CellSlotId, consumer_elements: u64) -> String {
        let elements = self.slot_elements[&slot];
        if let Some((_, _, _)) = self.input_slots.get(&slot) {
            let index = if elements == 1 && consumer_elements != 1 {
                "0u"
            } else {
                "index"
            };
            format!("input_{}[{index}]", slot.get())
        } else if self.state_slots.contains_key(&slot) {
            let index = if elements == 1 && consumer_elements != 1 {
                "0u"
            } else {
                "index"
            };
            format!("state_read_{}[{index}]", slot.get())
        } else {
            format!("slot_{}", slot.get())
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

fn turn_required_nodes(artifact: &ProgramArtifact) -> BTreeSet<NodeId> {
    let mut required = BTreeSet::new();
    for node in artifact.nodes() {
        let writes_state = node.output_bindings.clone().any(|index| {
            matches!(
                artifact.bindings().get(index as usize),
                Some(BindingDeclaration::Output { target, .. })
                    if artifact.slots()[target.get() as usize].role == SlotRole::State
            )
        });
        if writes_state {
            visit_turn_node(artifact, node.node, &mut required);
        }
    }
    for output in artifact.outputs() {
        visit_turn_source(artifact, ArtifactSource::Slot(output.source), &mut required);
    }
    required
}

fn visit_turn_node(artifact: &ProgramArtifact, node: NodeId, required: &mut BTreeSet<NodeId>) {
    if !required.insert(node) {
        return;
    }
    let Some(declaration) = artifact.nodes().get(node.get() as usize) else {
        return;
    };
    for binding in declaration.input_bindings.clone() {
        if let Some(BindingDeclaration::Input { source, .. }) =
            artifact.bindings().get(binding as usize)
        {
            visit_turn_source(artifact, *source, required);
        }
    }
}

fn visit_turn_source(
    artifact: &ProgramArtifact,
    source: ArtifactSource,
    required: &mut BTreeSet<NodeId>,
) {
    let ArtifactSource::Slot(slot) = source else {
        return;
    };
    let Some(declaration) = artifact.slots().get(slot.get() as usize) else {
        return;
    };
    if declaration.role == SlotRole::State {
        return;
    }
    if let ProducerReference::NodeOutput { node, .. } = declaration.producer {
        visit_turn_node(artifact, node, required);
    }
}

fn producer_node(producer: mech_engine::ProducerReference) -> Option<NodeId> {
    match producer {
        mech_engine::ProducerReference::Input(_) => None,
        mech_engine::ProducerReference::NodeOutput { node, .. } => Some(node),
    }
}

fn display_operation(operation: &OperationReference) -> String {
    operation
        .module_path
        .iter()
        .chain(std::iter::once(&operation.operation_name))
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("/")
}

fn binary_operation(operation: &OperationReference) -> Option<BinaryOperation> {
    let canonical = display_operation(operation);
    match canonical.as_str() {
        "math/add" => Some(BinaryOperation::Add),
        "math/sub" | "math/subtract" => Some(BinaryOperation::Subtract),
        "math/mul" | "math/multiply" => Some(BinaryOperation::Multiply),
        _ if operation.module_path.as_ref() == ["runtime"]
            && operation.operation_name.starts_with("Add") =>
        {
            Some(BinaryOperation::Add)
        }
        _ if operation.module_path.as_ref() == ["runtime"]
            && operation.operation_name.starts_with("Sub") =>
        {
            Some(BinaryOperation::Subtract)
        }
        _ if operation.module_path.as_ref() == ["runtime"]
            && operation.operation_name.starts_with("Mul") =>
        {
            Some(BinaryOperation::Multiply)
        }
        _ => None,
    }
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
    slots: &BTreeMap<CellSlotId, Vec<f32>>,
    constants: &BTreeMap<mech_core::ConstantId, f32>,
) -> Result<f32, CpuExecutionError> {
    match source {
        ArtifactSource::Constant(constant) => constants
            .get(&constant)
            .copied()
            .ok_or(CpuExecutionError::MissingConstant { constant }),
        ArtifactSource::Slot(slot) => {
            let values = slots
                .get(&slot)
                .ok_or(CpuExecutionError::MissingSlot { slot })?;
            values
                .get(if values.len() == 1 { 0 } else { index })
                .copied()
                .ok_or(CpuExecutionError::IndexOutOfBounds { slot, index })
        }
    }
}
