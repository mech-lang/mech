//! Native numeric-region analysis for resolved Mech program artifacts.
//!
//! This module establishes the build-time partitioning seam without changing
//! bytecode or generated-runtime behavior. Backends consume only regions they
//! accept; ordinary resident nodes remain the fallback for every rejection.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use mech_core::snapshot::SequenceView;
use mech_core::{
    CellSlotId, ConstantId, DimensionExpr, ExternalInteraction, FloatWidth, FunctionCatalog,
    NodeId, ParsedProgram, ReactiveInstanceId, ResidentValueKind, ResolvedOperationContract,
    SchemaBody, Value, ValueData,
};
use mech_engine::__resident::{
    ActivatedNode, ActivatedPlan, ActivationFacts, ResidentReadLocation, ResidentStorageClass,
    activate,
};
use mech_engine::artifact::{
    ArtifactSource, BindingDeclaration, ProducerReference, ProgramArtifact, SlotRole,
    decode_program_artifact_sections,
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum NativeNumericSource {
    Constant(u32),
    Slot(u32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeNumericOpcode {
    Broadcast,
    HorizontalConcatenate,
    VerticalConcatenate,
    MatrixMultiply,
    Transpose,
    Dot,
    Assign,
    Sin,
    Cos,
    Negate,
    Atan2,
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeNumericInstruction {
    pub node: u32,
    pub operation: String,
    pub opcode: NativeNumericOpcode,
    pub inputs: Vec<NativeNumericSource>,
    pub output: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeNumericShape {
    pub rows: u32,
    pub columns: u32,
}

impl NativeNumericShape {
    pub fn len(self) -> Option<usize> {
        usize::try_from(self.rows)
            .ok()?
            .checked_mul(usize::try_from(self.columns).ok()?)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeNumericSlotStorage {
    Activation,
    Input,
    State,
    Scratch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeNumericSlotLayout {
    pub slot: u32,
    pub storage: NativeNumericSlotStorage,
    pub offset: usize,
    pub shape: NativeNumericShape,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeNumericConstant {
    pub constant: u32,
    pub shape: NativeNumericShape,
    pub elements: Vec<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeNumericRegion {
    pub nodes: Vec<u32>,
    pub instructions: Vec<NativeNumericInstruction>,
    pub constants: Vec<NativeNumericConstant>,
    pub slots: Vec<NativeNumericSlotLayout>,
    pub live_inputs: Vec<u32>,
    pub live_outputs: Vec<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeNumericNodeRejection {
    pub node: u32,
    pub operation: String,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeNumericAnalysis {
    pub program_revision: [u8; 32],
    pub regions: Vec<NativeNumericRegion>,
    pub rejections: Vec<NativeNumericNodeRejection>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeNumericAnalysisError {
    reason: String,
}

impl NativeNumericAnalysisError {
    fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl fmt::Display for NativeNumericAnalysisError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "native numeric analysis failed: {}", self.reason)
    }
}

impl std::error::Error for NativeNumericAnalysisError {}

pub fn analyze_bytecode(
    bytecode: &[u8],
    catalog: &FunctionCatalog,
) -> Result<NativeNumericAnalysis, NativeNumericAnalysisError> {
    let parsed = ParsedProgram::from_bytes(bytecode)
        .map_err(|error| NativeNumericAnalysisError::new(error.display_message()))?;
    let artifact = decode_program_artifact_sections(&parsed.artifact).map_err(|error| {
        NativeNumericAnalysisError::new(format!("semantic artifact decode failed: {error:?}"))
    })?;
    let instance = activate(
        ReactiveInstanceId::new(0, 0),
        &artifact,
        catalog,
        &ActivationFacts::default(),
    )
    .map_err(|error| {
        NativeNumericAnalysisError::new(format!("resident activation failed: {error:?}"))
    })?;
    Ok(analyze_activated_artifact(&artifact, &instance.plan))
}

pub fn analyze_activated_artifact(
    artifact: &ProgramArtifact,
    plan: &ActivatedPlan,
) -> NativeNumericAnalysis {
    let mut opcodes = vec![None; plan.nodes.len()];
    let mut rejections = Vec::new();
    for (index, node) in plan.nodes.iter().enumerate() {
        match node_eligibility(artifact, plan, node) {
            Ok(opcode) => opcodes[index] = Some(opcode),
            Err(reason) => {
                let declaration = artifact_node(artifact, node.artifact_node);
                rejections.push(NativeNumericNodeRejection {
                    node: node.artifact_node.get(),
                    operation: declaration
                        .map(qualified_operation_name)
                        .unwrap_or_else(|| "<missing>".to_owned()),
                    reason,
                });
            }
        }
    }

    let activated_by_artifact = plan
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.artifact_node, index))
        .collect::<BTreeMap<_, _>>();
    let mut stages = vec![0_u32; plan.nodes.len()];
    for activated in &plan.topology.linear_node_order {
        let index = activated.get() as usize;
        let node = &plan.nodes[index];
        let Some(declaration) = artifact_node(artifact, node.artifact_node) else {
            continue;
        };
        let input_stage = node_input_sources(artifact, declaration)
            .into_iter()
            .filter_map(|source| match source {
                ArtifactSource::Slot(slot)
                    if slot_role(artifact, slot) != Some(SlotRole::State) =>
                {
                    slot_producer(artifact, slot)
                        .and_then(|producer| activated_by_artifact.get(&producer).copied())
                        .map(|producer| stages[producer])
                }
                ArtifactSource::Constant(_) | ArtifactSource::Slot(_) => None,
            })
            .max()
            .unwrap_or(0);
        stages[index] = if opcodes[index].is_some() {
            input_stage
        } else {
            input_stage.saturating_add(1)
        };
    }
    let mut disjoint = DisjointSet::new(plan.nodes.len());
    for (index, node) in plan.nodes.iter().enumerate() {
        if opcodes[index].is_none() {
            continue;
        }
        let Some(declaration) = artifact_node(artifact, node.artifact_node) else {
            continue;
        };
        for source in node_input_sources(artifact, declaration) {
            let ArtifactSource::Slot(slot) = source else {
                continue;
            };
            let Some(producer) = slot_producer(artifact, slot) else {
                continue;
            };
            let Some(&producer_index) = activated_by_artifact.get(&producer) else {
                continue;
            };
            if opcodes[producer_index].is_some() && stages[producer_index] == stages[index] {
                disjoint.union(index, producer_index);
            }
        }
    }

    let mut grouped = BTreeMap::<(u32, usize), Vec<usize>>::new();
    for node in &plan.topology.linear_node_order {
        let index = node.get() as usize;
        if opcodes[index].is_some() {
            grouped
                .entry((stages[index], disjoint.find(index)))
                .or_default()
                .push(index);
        }
    }

    let consumers = slot_consumers(artifact);
    let artifact_outputs = artifact
        .outputs()
        .iter()
        .map(|output| output.source)
        .collect::<BTreeSet<_>>();
    let constraint_inputs = artifact
        .constraints()
        .iter()
        .flat_map(|constraint| constraint.inputs.iter())
        .filter_map(|source| match source {
            ArtifactSource::Slot(slot) => Some(*slot),
            ArtifactSource::Constant(_) => None,
        })
        .collect::<BTreeSet<_>>();

    let mut regions = grouped
        .into_values()
        .map(|indices| {
            build_region(
                artifact,
                plan,
                &opcodes,
                &indices,
                &consumers,
                &artifact_outputs,
                &constraint_inputs,
            )
        })
        .collect::<Vec<_>>();
    regions.sort_by_key(|region| region.nodes.first().copied().unwrap_or(u32::MAX));
    rejections.sort_by_key(|rejection| rejection.node);

    NativeNumericAnalysis {
        program_revision: artifact.revision().into_bytes(),
        regions,
        rejections,
    }
}

fn node_eligibility(
    artifact: &ProgramArtifact,
    plan: &ActivatedPlan,
    node: &ActivatedNode,
) -> Result<NativeNumericOpcode, String> {
    let declaration = artifact_node(artifact, node.artifact_node)
        .ok_or_else(|| "artifact node declaration is missing".to_owned())?;
    let contract = artifact
        .contracts()
        .get(declaration.contract)
        .ok_or_else(|| "operation contract is missing".to_owned())?;
    let ResolvedOperationContract::Declared(contract) = contract else {
        return Err("legacy opaque operation contract has no numeric semantics".to_owned());
    };
    if contract.interaction != ExternalInteraction::Pure {
        return Err(format!(
            "external interaction {:?} must remain in the host/runtime path",
            contract.interaction
        ));
    }
    let opcode = lower_operation(
        &declaration.operation.module_path,
        &declaration.operation.operation_name,
    )
    .ok_or_else(|| "operation has no native numeric opcode lowering".to_owned())?;
    if node.write.region.kind != ResidentValueKind::F64 {
        return Err(format!(
            "output kind {:?} is not supported by the f64 numeric backend",
            node.write.region.kind
        ));
    }
    if plan.reads[node.reads.start as usize..node.reads.end as usize]
        .iter()
        .any(|read| read_region(*read).kind != ResidentValueKind::F64)
    {
        return Err("one or more inputs are not f64 resident values".to_owned());
    }
    let outputs = node_output_slots(artifact, declaration);
    if outputs.as_slice() != [node.write.slot] {
        return Err("numeric regions require exactly one resident output".to_owned());
    }
    Ok(opcode)
}

fn build_region(
    artifact: &ProgramArtifact,
    plan: &ActivatedPlan,
    opcodes: &[Option<NativeNumericOpcode>],
    indices: &[usize],
    consumers: &BTreeMap<CellSlotId, BTreeSet<NodeId>>,
    artifact_outputs: &BTreeSet<CellSlotId>,
    constraint_inputs: &BTreeSet<CellSlotId>,
) -> NativeNumericRegion {
    let members = indices
        .iter()
        .map(|index| plan.nodes[*index].artifact_node)
        .collect::<BTreeSet<_>>();
    let mut live_inputs = BTreeSet::new();
    let mut constants = BTreeSet::new();
    let mut live_outputs = BTreeSet::new();

    for index in indices {
        let node = &plan.nodes[*index];
        let Some(declaration) = artifact_node(artifact, node.artifact_node) else {
            continue;
        };
        for source in node_input_sources(artifact, declaration) {
            match source {
                ArtifactSource::Constant(constant) => {
                    constants.insert(constant.get());
                }
                ArtifactSource::Slot(slot) => {
                    let external = slot_role(artifact, slot) == Some(SlotRole::State)
                        || slot_producer(artifact, slot)
                            .is_none_or(|producer| !members.contains(&producer));
                    if external {
                        live_inputs.insert(slot.get());
                    }
                }
            }
        }
        for target in node_output_slots(artifact, declaration) {
            let consumed_outside = consumers
                .get(&target)
                .is_some_and(|nodes| nodes.iter().any(|node| !members.contains(node)));
            if consumed_outside
                || artifact_outputs.contains(&target)
                || constraint_inputs.contains(&target)
                || slot_role(artifact, target) == Some(SlotRole::State)
            {
                live_outputs.insert(target.get());
            }
        }
    }

    let nodes = indices
        .iter()
        .map(|index| plan.nodes[*index].artifact_node.get())
        .collect::<Vec<_>>();
    let instructions = indices
        .iter()
        .map(|index| {
            let activated = &plan.nodes[*index];
            let declaration = artifact_node(artifact, activated.artifact_node)
                .expect("eligible node has an artifact declaration");
            let outputs = node_output_slots(artifact, declaration);
            let [output] = outputs.as_slice() else {
                unreachable!("eligible node has exactly one output");
            };
            NativeNumericInstruction {
                node: activated.artifact_node.get(),
                operation: qualified_operation_name(declaration),
                opcode: opcodes[*index].expect("region contains only eligible nodes"),
                inputs: node_input_sources(artifact, declaration)
                    .into_iter()
                    .map(native_source)
                    .collect(),
                output: output.get(),
            }
        })
        .collect::<Vec<_>>();
    let referenced_slots = instructions
        .iter()
        .flat_map(|instruction| {
            instruction
                .inputs
                .iter()
                .filter_map(|source| match source {
                    NativeNumericSource::Constant(_) => None,
                    NativeNumericSource::Slot(slot) => Some(*slot),
                })
                .chain(std::iter::once(instruction.output))
        })
        .collect::<BTreeSet<_>>();
    let slots = referenced_slots
        .into_iter()
        .map(|slot| {
            let resolved = plan
                .slots
                .get(slot as usize)
                .filter(|resolved| resolved.artifact_id.get() == slot)
                .expect("eligible instruction slot has a resolved layout");
            NativeNumericSlotLayout {
                slot,
                storage: native_slot_storage(resolved.storage),
                offset: resolved.region.offset,
                shape: NativeNumericShape {
                    rows: resolved.region.shape.rows,
                    columns: resolved.region.shape.columns,
                },
            }
        })
        .collect();
    let constants = constants
        .into_iter()
        .map(|constant| {
            native_constant(artifact, ConstantId::new(constant))
                .expect("eligible instruction constant has a resolved f64 value")
        })
        .collect();

    NativeNumericRegion {
        nodes,
        instructions,
        constants,
        slots,
        live_inputs: live_inputs.into_iter().collect(),
        live_outputs: live_outputs.into_iter().collect(),
    }
}

fn artifact_node(
    artifact: &ProgramArtifact,
    node: NodeId,
) -> Option<&mech_engine::artifact::NodeDeclaration> {
    artifact
        .nodes()
        .get(node.get() as usize)
        .filter(|declaration| declaration.node == node)
}

fn node_input_sources(
    artifact: &ProgramArtifact,
    node: &mech_engine::artifact::NodeDeclaration,
) -> Vec<ArtifactSource> {
    let mut sources = artifact.bindings()
        [node.input_bindings.start as usize..node.input_bindings.end as usize]
        .iter()
        .filter_map(|binding| match binding {
            BindingDeclaration::Input {
                port_ordinal,
                source,
                ..
            } => Some((*port_ordinal, *source)),
            BindingDeclaration::Output { .. } => None,
        })
        .collect::<Vec<_>>();
    sources.sort_by_key(|(ordinal, _)| *ordinal);
    sources.into_iter().map(|(_, source)| source).collect()
}

fn node_output_slots(
    artifact: &ProgramArtifact,
    node: &mech_engine::artifact::NodeDeclaration,
) -> Vec<CellSlotId> {
    artifact.bindings()[node.output_bindings.start as usize..node.output_bindings.end as usize]
        .iter()
        .filter_map(|binding| match binding {
            BindingDeclaration::Output { target, .. } => Some(*target),
            BindingDeclaration::Input { .. } => None,
        })
        .collect()
}

fn slot_producer(artifact: &ProgramArtifact, slot: CellSlotId) -> Option<NodeId> {
    let declaration = artifact.slots().get(slot.get() as usize)?;
    match declaration.producer {
        ProducerReference::NodeOutput { node, .. } => Some(node),
        ProducerReference::Input(_) => None,
    }
}

fn slot_role(artifact: &ProgramArtifact, slot: CellSlotId) -> Option<SlotRole> {
    artifact
        .slots()
        .get(slot.get() as usize)
        .map(|slot| slot.role)
}

fn slot_consumers(artifact: &ProgramArtifact) -> BTreeMap<CellSlotId, BTreeSet<NodeId>> {
    let mut consumers = BTreeMap::<CellSlotId, BTreeSet<NodeId>>::new();
    for node in artifact.nodes() {
        for source in node_input_sources(artifact, node) {
            if let ArtifactSource::Slot(slot) = source {
                consumers.entry(slot).or_default().insert(node.node);
            }
        }
    }
    consumers
}

fn native_source(source: ArtifactSource) -> NativeNumericSource {
    match source {
        ArtifactSource::Constant(constant) => NativeNumericSource::Constant(constant.get()),
        ArtifactSource::Slot(slot) => NativeNumericSource::Slot(slot.get()),
    }
}

fn native_slot_storage(storage: ResidentStorageClass) -> NativeNumericSlotStorage {
    match storage {
        ResidentStorageClass::Constant => NativeNumericSlotStorage::Activation,
        ResidentStorageClass::Input => NativeNumericSlotStorage::Input,
        ResidentStorageClass::State => NativeNumericSlotStorage::State,
        ResidentStorageClass::Scratch => NativeNumericSlotStorage::Scratch,
    }
}

fn native_constant(
    artifact: &ProgramArtifact,
    id: ConstantId,
) -> Result<NativeNumericConstant, String> {
    let value = artifact
        .constants()
        .get(id)
        .ok_or_else(|| format!("constant {} is missing", id.get()))?;
    Ok(NativeNumericConstant {
        constant: id.get(),
        shape: constant_shape(artifact, value)?,
        elements: constant_elements(value)?,
    })
}

fn constant_shape(artifact: &ProgramArtifact, value: &Value) -> Result<NativeNumericShape, String> {
    let schema = artifact
        .schemas()
        .entry(value.schema())
        .ok_or_else(|| "constant schema is missing".to_owned())?
        .schema();
    match schema.body() {
        SchemaBody::FloatingPoint(FloatWidth::W64) => Ok(NativeNumericShape {
            rows: 1,
            columns: 1,
        }),
        SchemaBody::Matrix { dimensions, .. } if dimensions.len() == 2 => {
            let rows = evaluate_dimension(&dimensions[0], value.shape().parameter_values())?;
            let columns = evaluate_dimension(&dimensions[1], value.shape().parameter_values())?;
            Ok(NativeNumericShape {
                rows: u32::try_from(rows)
                    .map_err(|_| "constant row count exceeds u32".to_owned())?,
                columns: u32::try_from(columns)
                    .map_err(|_| "constant column count exceeds u32".to_owned())?,
            })
        }
        body => Err(format!("constant schema {body:?} is not fixed-shape f64")),
    }
}

fn evaluate_dimension(expression: &DimensionExpr, values: &[u64]) -> Result<u64, String> {
    match expression {
        DimensionExpr::Hole => Err("constant contains an unresolved dimension".to_owned()),
        DimensionExpr::Constant(value) => Ok(*value),
        DimensionExpr::Parameter(id) => values
            .get(id.get() as usize)
            .copied()
            .ok_or_else(|| "constant dimension parameter is missing".to_owned()),
        DimensionExpr::Add(terms) => terms.iter().try_fold(0_u64, |sum, term| {
            sum.checked_add(evaluate_dimension(term, values)?)
                .ok_or_else(|| "constant dimension overflow".to_owned())
        }),
        DimensionExpr::Multiply(terms) => terms.iter().try_fold(1_u64, |product, term| {
            product
                .checked_mul(evaluate_dimension(term, values)?)
                .ok_or_else(|| "constant dimension overflow".to_owned())
        }),
        DimensionExpr::Min(terms) => terms
            .iter()
            .map(|term| evaluate_dimension(term, values))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .min()
            .ok_or_else(|| "constant has an empty minimum dimension".to_owned()),
        DimensionExpr::Max(terms) => terms
            .iter()
            .map(|term| evaluate_dimension(term, values))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .max()
            .ok_or_else(|| "constant has an empty maximum dimension".to_owned()),
    }
}

fn constant_elements(value: &Value) -> Result<Vec<u64>, String> {
    match value.data() {
        ValueData::F64(value) => Ok(vec![value.bits()]),
        ValueData::Matrix(matrix) => match matrix.elements() {
            SequenceView::F64(values) => Ok(values.iter().map(|value| value.bits()).collect()),
            other => Err(format!("matrix constant elements {other:?} are not f64")),
        },
        other => Err(format!("constant {other:?} is not f64")),
    }
}

fn read_region(read: ResidentReadLocation) -> mech_engine::__resident::ResidentRegion {
    match read {
        ResidentReadLocation::Constant(region)
        | ResidentReadLocation::Input(region)
        | ResidentReadLocation::Scratch(region) => region,
        ResidentReadLocation::State { region, .. } => region,
    }
}

fn qualified_operation_name(node: &mech_engine::artifact::NodeDeclaration) -> String {
    if node.operation.module_path.is_empty() {
        node.operation.operation_name.clone()
    } else {
        format!(
            "{}/{}",
            node.operation.module_path.join("/"),
            node.operation.operation_name
        )
    }
}

fn lower_operation(module: &[String], name: &str) -> Option<NativeNumericOpcode> {
    if module != ["runtime"] {
        return None;
    }
    if name.starts_with("HorizontalConcatenate") {
        Some(NativeNumericOpcode::HorizontalConcatenate)
    } else if name.starts_with("VerticalConcatenate") {
        Some(NativeNumericOpcode::VerticalConcatenate)
    } else if name.starts_with("MatMul") {
        Some(NativeNumericOpcode::MatrixMultiply)
    } else if name.starts_with("Transpose") {
        Some(NativeNumericOpcode::Transpose)
    } else if name.starts_with("Dot") {
        Some(NativeNumericOpcode::Dot)
    } else if name.starts_with("Assign<") {
        Some(NativeNumericOpcode::Assign)
    } else if name.starts_with("ConvertScalarToMat2") {
        Some(NativeNumericOpcode::Broadcast)
    } else if name.starts_with("MathSin") {
        Some(NativeNumericOpcode::Sin)
    } else if name.starts_with("MathCos") {
        Some(NativeNumericOpcode::Cos)
    } else if name.starts_with("Negate") {
        Some(NativeNumericOpcode::Negate)
    } else if name.starts_with("Atan2") {
        Some(NativeNumericOpcode::Atan2)
    } else if binary_operation_is_lowerable(name, "Add") {
        Some(NativeNumericOpcode::Add)
    } else if binary_operation_is_lowerable(name, "Sub") {
        Some(NativeNumericOpcode::Subtract)
    } else if binary_operation_is_lowerable(name, "Mul") {
        Some(NativeNumericOpcode::Multiply)
    } else if binary_operation_is_lowerable(name, "Div") {
        Some(NativeNumericOpcode::Divide)
    } else {
        None
    }
}

fn binary_operation_is_lowerable(name: &str, prefix: &str) -> bool {
    name.starts_with(prefix) && !name.starts_with(&format!("{prefix}Assign"))
}

struct DisjointSet {
    parent: Vec<usize>,
}

impl DisjointSet {
    fn new(len: usize) -> Self {
        Self {
            parent: (0..len).collect(),
        }
    }

    fn find(&mut self, value: usize) -> usize {
        let parent = self.parent[value];
        if parent == value {
            value
        } else {
            let root = self.find(parent);
            self.parent[value] = root;
            root
        }
    }

    fn union(&mut self, left: usize, right: usize) {
        let left = self.find(left);
        let right = self.find(right);
        if left != right {
            let (root, child) = if left < right {
                (left, right)
            } else {
                (right, left)
            };
            self.parent[child] = root;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowerable_binary_families_exclude_read_modify_write_variants() {
        assert_eq!(
            lower_operation(&["runtime".to_owned()], "AddMDMD<f64>"),
            Some(NativeNumericOpcode::Add)
        );
        assert_eq!(
            lower_operation(&["runtime".to_owned()], "MulSS<f64>"),
            Some(NativeNumericOpcode::Multiply)
        );
        assert_eq!(
            lower_operation(
                &["runtime".to_owned()],
                "AddAssign2DRAV<f64DMatrixDMatrixDVector>",
            ),
            None
        );
        assert_eq!(
            lower_operation(&["runtime".to_owned()], "NChooseKMatrix<f64>"),
            None
        );
        assert_eq!(
            lower_operation(&["custom".to_owned()], "AddSS<f64>"),
            None,
            "concrete names are meaningful only in the trusted runtime module",
        );
    }

    #[test]
    fn disjoint_set_uses_deterministic_smallest_root() {
        let mut set = DisjointSet::new(5);
        set.union(4, 2);
        set.union(3, 4);
        assert_eq!(set.find(2), 2);
        assert_eq!(set.find(3), 2);
        assert_eq!(set.find(4), 2);
    }
}
