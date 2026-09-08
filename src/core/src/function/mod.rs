pub mod argument;
pub mod catalog;
pub mod contract;
pub mod resident;
pub mod signature {
    pub use crate::function_signature::*;
}
pub mod specialization;
pub mod state;
pub use argument::*;
pub use catalog::*;
pub use contract::*;
pub use resident::*;
pub use signature::*;
pub use specialization::*;
pub use state::*;

use crate::nodes::*;
use crate::types::*;
use crate::*;

#[cfg(all(feature = "no_std", not(feature = "std")))]
use alloc::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};
#[cfg(all(feature = "no_std", not(feature = "std")))]
use hashbrown::HashSet as HashBrownSet;
#[cfg(feature = "functions")]
use indexmap::map::IndexMap;
#[cfg(all(feature = "no_std", not(feature = "std")))]
type HashSet<T> = HashBrownSet<T, core::hash::BuildHasherDefault<fxhash::FxHasher>>;
use core::fmt;
#[cfg(any(not(feature = "no_std"), feature = "std"))]
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
#[cfg(any(not(feature = "no_std"), feature = "std"))]
use std::rc::Rc;
#[cfg(feature = "pretty_print")]
use tabled::{
    builder::Builder,
    settings::{Alignment, Panel, Style},
};

// Functions ------------------------------------------------------------------

/// Program-local user-function definitions keyed by their stable name hash.
///
/// The backing map is intentionally opaque so callers cannot accidentally
/// replace a different name that happens to share the same stable ID.
#[derive(Clone, Default)]
pub struct UserFunctionTable {
    definitions: HashMap<u64, FunctionDefinition>,
}

impl UserFunctionTable {
    /// Resolves one exact source-visible name.
    pub fn resolve_name(&self, name: &str) -> Option<&FunctionDefinition> {
        let id = hash_str(name);
        self.definitions
            .get(&id)
            .filter(|definition| definition.name == name)
    }

    /// Inserts a definition, replacing an existing definition only when both
    /// definitions have the exact same name.
    pub fn insert_or_replace(
        &mut self,
        definition: FunctionDefinition,
    ) -> MResult<Option<FunctionDefinition>> {
        validate_user_function_definition(&definition)?;

        if let Some(existing) = self.definitions.get(&definition.id)
            && existing.name != definition.name
        {
            return Err(MechError::new(
                UserFunctionIdCollision {
                    id: definition.id,
                    existing_name: existing.name.clone(),
                    incoming_name: definition.name,
                },
                None,
            )
            .with_compiler_loc());
        }

        Ok(self.definitions.insert(definition.id, definition))
    }

    pub fn definitions(&self) -> impl ExactSizeIterator<Item = &FunctionDefinition> + '_ {
        self.definitions.values()
    }

    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    pub fn clear(&mut self) {
        self.definitions.clear();
    }
}

fn validate_user_function_definition(definition: &FunctionDefinition) -> MResult<()> {
    if definition.name.is_empty() {
        return Err(invalid_user_function_definition(
            definition,
            "name must not be empty",
        ));
    }

    let expected = hash_str(&definition.name);
    if expected != definition.id {
        return Err(invalid_user_function_definition(
            definition,
            format!(
                "name hashes to 0x{expected:016x}, not 0x{:016x}",
                definition.id,
            ),
        ));
    }

    Ok(())
}

fn invalid_user_function_definition(
    definition: &FunctionDefinition,
    reason: impl Into<String>,
) -> MechError {
    MechError::new(
        UserFunctionInvalidDefinition {
            id: definition.id,
            name: definition.name.clone(),
            reason: reason.into(),
        },
        None,
    )
    .with_compiler_loc()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserFunctionInvalidDefinition {
    pub id: u64,
    pub name: String,
    pub reason: String,
}

impl MechErrorKind for UserFunctionInvalidDefinition {
    fn name(&self) -> &str {
        "UserFunctionInvalidDefinition"
    }

    fn message(&self) -> String {
        format!(
            "invalid user function {:?} at ID 0x{:016x}: {}",
            self.name, self.id, self.reason,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserFunctionIdCollision {
    pub id: u64,
    pub existing_name: String,
    pub incoming_name: String,
}

impl MechErrorKind for UserFunctionIdCollision {
    fn name(&self) -> &str {
        "UserFunctionIdCollision"
    }

    fn message(&self) -> String {
        format!(
            "user function names {:?} and {:?} collide at ID 0x{:016x}",
            self.existing_name, self.incoming_name, self.id,
        )
    }
}

pub trait MechFunctionFactory {
    const SIGNATURE: RuntimeFunctionSignature;

    /// Closed R5 declaration of heap work beyond generic atomic publication.
    fn implementation_memory_class() -> ImplementationMemoryClass;

    /// Semantic memory contract declared by a statically registered runtime
    /// implementation. Fixed operation bindings must provide this authority
    /// before their constructor can be called.
    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        None
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>>
    where
        Self: Sized,
    {
        let _ = invocation;
        Err(MechError::new(
            CanonicalFunctionInvocationUnsupported {
                factory: core::any::type_name::<Self>(),
            },
            None,
        )
        .with_compiler_loc())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalFunctionInvocationUnsupported {
    pub factory: &'static str,
}

impl MechErrorKind for CanonicalFunctionInvocationUnsupported {
    fn name(&self) -> &str {
        "CanonicalFunctionInvocationUnsupported"
    }

    fn message(&self) -> String {
        format!(
            "function factory `{}` has no canonical invocation constructor",
            self.factory,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InitialSolvePolicy {
    Solve,
    PreserveSpecializedOutput,
}

pub trait MechFunctionImpl {
    /// Executes this implementation through the one maintained managed
    /// runtime entry. Implementations retain logical ports only; every
    /// physical view comes from `frame`.
    fn solve_managed(
        &self,
        frame: &mut KernelMemoryFrame<'_>,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus>;

    /// Resolves value-dependent output extents before physical acquisition.
    /// Most calls derive shapes solely from their closed type contract. Build
    /// operations such as ranges override this hook so a changed scalar input
    /// can produce a revised admitted stage before the kernel writes it.
    fn planned_output_shapes(&self) -> MResult<Option<Box<[ShapeInstance]>>> {
        Ok(None)
    }

    /// Resolves value-dependent output payload requirements before any
    /// result construction. Canonical builders override this hook so the R5
    /// call planner can admit the complete prospective footprint first.
    fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
        Ok(None)
    }

    fn initial_solve_policy(&self) -> InitialSolvePolicy {
        InitialSolvePolicy::Solve
    }
    /// Performs service-aware initialization that is required even when the
    /// specialized output itself was produced during deterministic planning.
    ///
    /// This hook must not recompute or replace that planned output. Most
    /// functions need no extra initialization, so the default is a no-op.
    fn initialize_preserved_output_with(
        &self,
        _: &mut KernelMemoryFrame<'_>,
        _: &mut dyn MechExecutionServices,
    ) -> MResult<()> {
        Ok(())
    }
    /// Returns the primary output as an exact typed state port.
    ///
    /// `None` means the canonical invocation output is authoritative and the
    /// implementation retains no separate exact primary output. `Some(port)`
    /// makes that exact typed port authoritative.
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        None
    }
    /// Returns the authoritative typed list of reactive output cells.
    ///
    /// `None` declares that the function has no additional reactive output
    /// state beyond the canonical invocation output. `Some(ports)` is
    /// authoritative, including `Some(Vec::new())`.
    fn reactive_output_state_ports(&self) -> Option<Vec<FunctionStatePort<'_>>> {
        self.primary_output_state_port().map(|output| vec![output])
    }
    /// Returns the authoritative typed list of transaction checkpoint cells.
    ///
    /// `None` declares that the function has no retained transaction state.
    /// `Some(ports)` is authoritative, including `Some(Vec::new())`.
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(None)
    }
    fn retained_state_ports(&self) -> MResult<Vec<FunctionStatePort<'_>>> {
        Ok(self.transaction_state_ports()?.unwrap_or_default())
    }
    fn capture_retained_state(&self, journal: &mut FunctionCheckpoint) -> MResult<()> {
        for state in self.retained_state_ports()? {
            state.capture_into(journal)?;
        }
        Ok(())
    }
    /// Returns canonical output cells retained by the implementation in
    /// addition to the invocation output. This is the value-preserving
    /// counterpart to [`Self::reactive_output_state_ports`]; it never erases
    /// the canonical cell binding.
    fn reactive_output_value_cells(&self) -> Vec<ValueCell> {
        Vec::new()
    }
    fn reactive_dependency_kinds(
        &self,
        _argument_count: usize,
    ) -> Option<Vec<ReactiveDependencyKind>> {
        None
    }
    fn reactive_dependency_scopes(
        &self,
        _argument_count: usize,
    ) -> Option<Vec<ReactiveDependencyScope>> {
        None
    }
    fn reactive_output_cell_ids(&self) -> Vec<CanonicalCellId> {
        let mut cells = Vec::new();

        if let Some(outputs) = self.reactive_output_state_ports() {
            for output in outputs {
                let cell = output.logical_cell_id();
                if !cells.contains(&cell) {
                    cells.push(cell);
                }
            }
        } else {
            for output in self.reactive_output_value_cells() {
                let cell = output.reactive_cell_id();
                if !cells.contains(&cell) {
                    cells.push(cell);
                }
            }
        }

        cells
    }
    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Combinational
    }
    /// Portable semantic metadata for the compiled operation represented by
    /// this specialized function. Current execution continues to use
    /// `RuntimeFunctionContract`; this declaration is consumed only while
    /// constructing a `ProgramArtifact`.
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        None
    }
    /// Canonical source-level operation represented by this specialized
    /// function. Concrete runtime factory names are deliberately excluded.
    fn semantic_operation_name(&self) -> Option<&str> {
        None
    }
    fn to_string(&self) -> String;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManagedExecutionScopeRequired;

impl MechErrorKind for ManagedExecutionScopeRequired {
    fn name(&self) -> &str {
        "ManagedExecutionScopeRequired"
    }

    fn message(&self) -> String {
        "managed execution requires a validated FunctionInstance scope".into()
    }
}

#[cfg(feature = "semantic-compiler")]
pub trait MechFunctionCompiler {
    /// Returns explicit cells whose identity must remain associated with the
    /// values consumed by other compiler implementations in this plan.
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        Vec::new()
    }

    /// Reserves registers whose initializer must come from declaration-time
    /// state before other plan nodes observe their live reactive values.
    fn reserve_bytecode_registers(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<()> {
        Ok(())
    }

    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register>;
}

#[cfg(feature = "semantic-compiler")]
pub trait MechFunction: MechFunctionImpl + MechFunctionCompiler {}
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunction for T where T: MechFunctionImpl + MechFunctionCompiler {}

#[cfg(not(feature = "semantic-compiler"))]
pub trait MechFunction: MechFunctionImpl {}
#[cfg(not(feature = "semantic-compiler"))]
impl<T> MechFunction for T where T: MechFunctionImpl {}

/// A runtime implementation bound to its visible canonical cells.
pub struct FunctionInstance {
    implementation: Box<dyn MechFunction>,
    invocation: FunctionInvocation,
    managed: ManagedFunctionBinding,
}

struct ManagedFunctionBinding {
    plan: Rc<CallMemoryPlan>,
    current: core::cell::RefCell<ManagedCallRealization>,
}

struct ManagedCallRealization {
    domain: MemoryDomain,
    plan: Rc<CallMemoryPlan>,
    realized: RealizedMemoryPlan,
    initialization: PreparedCallAccess,
    execution: PreparedCallAccess,
}

struct PreparedFunctionPublication {
    status: ReactiveSolveStatus,
    publication: Option<PreparedCellPublication>,
    next_realization: Option<ManagedCallRealization>,
    _execution_scope: Option<MemoryPlanScope>,
}

impl ManagedCallRealization {
    fn prepare(
        domain: MemoryDomain,
        plan: Rc<CallMemoryPlan>,
        invocation: &FunctionInvocation,
    ) -> MResult<Self> {
        let existing = invocation
            .output_cell()
            .managed_host_binding()?
            .filter(|live| {
                live.realized.domain() == domain.id()
                    && live
                        .realized
                        .call_plan()
                        .is_some_and(|existing| existing.as_ref() == plan.as_ref())
            });
        let realized = if let Some(existing) = existing {
            existing.realized
        } else {
            domain.prepare_call_memory_realization(plan)?
        };
        let plan = realized
            .call_plan()
            .expect("call realization retains immutable R5 authority")
            .clone();
        let initialization =
            domain.prepare_function_input_initialization(&realized, plan.as_ref(), invocation)?;
        let execution = domain.prepare_function_call(&realized, plan.as_ref(), invocation)?;
        Ok(Self {
            domain,
            plan,
            realized,
            initialization,
            execution,
        })
    }

    fn refreshed_plan(
        &self,
        invocation: &FunctionInvocation,
        output_shapes: Option<&[ShapeInstance]>,
        output_footprints: Option<&[CurrentMemoryFootprint]>,
    ) -> MResult<Option<Rc<CallMemoryPlan>>> {
        let planned_inputs = (0..self.plan.inputs.len())
            .map(|index| invocation.planned_input_cell(self.plan.as_ref(), index))
            .collect::<MResult<Vec<_>>>()?;
        let inputs_unchanged = planned_inputs
            .iter()
            .zip(self.plan.inputs.iter())
            .all(|(cell, port)| *cell.shape() == *port.descriptor.shape());
        let outputs_unchanged = output_shapes.is_none_or(|shapes| {
            shapes.len() == self.plan.outputs.len()
                && shapes
                    .iter()
                    .zip(self.plan.outputs.iter())
                    .all(|(shape, port)| shape == port.descriptor.shape())
        });
        let payload_dependent = self
            .plan
            .input_storage
            .iter()
            .chain(self.plan.output_storage.iter())
            .any(|storage| {
                matches!(
                    storage.slot,
                    PlannedSlotKind::StringHeader | PlannedSlotKind::CanonicalValueHandle
                )
            });
        if inputs_unchanged && outputs_unchanged && !payload_dependent {
            return Ok(None);
        }
        let inputs = planned_inputs
            .iter()
            .map(|cell| cell.resolved_descriptor())
            .collect::<MResult<Vec<_>>>()?
            .into_boxed_slice();
        let mut outputs = Vec::new();
        for (index, previous) in self.plan.bound_call.outputs().iter().enumerate() {
            if let Some(shape) = output_shapes.and_then(|shapes| shapes.get(index)) {
                outputs.push(ResolvedValueDescriptor::from_schema(
                    previous.schema().clone(),
                    shape.clone(),
                )?);
                continue;
            }
            let policy = &self.plan.bound_call.operation_descriptor().contract.outputs[index];
            let shape_rule = match policy.construction {
                OutputConstruction::FullWrite { shape } | OutputConstruction::Replace { shape } => {
                    shape
                }
                OutputConstruction::ReadModifyWrite { base_input, .. } => {
                    ShapeRule::SameAsInput { input: base_input }
                }
                OutputConstruction::Build { .. } => {
                    // Aggregate builders may change a dynamic cardinality
                    // without changing their closed descriptor shape. Their
                    // prospective payload witness, supplied separately,
                    // drives allocation and admission for this turn.
                    outputs.push(previous.clone());
                    continue;
                }
            };
            let extents = match shape_rule {
                ShapeRule::Declared => previous.current_extents()?,
                ShapeRule::SameAsInput { input } => inputs[input as usize].current_extents()?,
                ShapeRule::TransposeOf { input } => {
                    let mut extents = inputs[input as usize].current_extents()?;
                    if extents.len() != 2 {
                        return Err(MechError::new(MemoryPlanError::DescriptorMismatch, None)
                            .with_compiler_loc());
                    }
                    extents.swap(0, 1);
                    extents
                }
                ShapeRule::MatrixProduct { lhs, rhs } => {
                    let left = inputs[lhs as usize].current_extents()?;
                    let right = inputs[rhs as usize].current_extents()?;
                    if left.len() != 2 || right.len() != 2 || left[1] != right[0] {
                        return Err(MechError::new(MemoryPlanError::DescriptorMismatch, None)
                            .with_compiler_loc());
                    }
                    vec![left[0], right[1]].into_boxed_slice()
                }
            };
            let shape = shape_for_resolved_extents(previous.schema(), &extents)?;
            outputs.push(ResolvedValueDescriptor::from_schema(
                previous.schema().clone(),
                shape,
            )?);
        }
        let current = self
            .plan
            .bound_call
            .with_current_descriptors(inputs, outputs.into_boxed_slice())?;
        if !payload_dependent {
            let geometry_plan = replan_fixed_call_geometry(&self.plan, &current)
                .map_err(|error| MechError::new(error, None).with_compiler_loc())?;
            return Ok(Some(Rc::new(geometry_plan)));
        }

        let mut resolved = BTreeMap::new();
        for (index, cell) in planned_inputs.iter().enumerate() {
            let port = u16::try_from(index).map_err(|_| {
                MechError::new(MemoryPlanError::DescriptorArityMismatch, None).with_compiler_loc()
            })?;
            resolved.insert(
                (PortDirection::Input, port),
                cell.current_memory_footprint()?,
            );
        }
        if let Some(footprints) = output_footprints {
            if footprints.len() != self.plan.outputs.len() {
                return Err(
                    MechError::new(MemoryPlanError::DescriptorArityMismatch, None)
                        .with_compiler_loc(),
                );
            }
            for (index, footprint) in footprints.iter().copied().enumerate() {
                let port = u16::try_from(index).map_err(|_| {
                    MechError::new(MemoryPlanError::DescriptorArityMismatch, None)
                        .with_compiler_loc()
                })?;
                resolved.insert((PortDirection::Output, port), footprint);
            }
        } else {
            for index in 0..self.plan.outputs.len() {
                let port = u16::try_from(index).map_err(|_| {
                    MechError::new(MemoryPlanError::DescriptorArityMismatch, None)
                        .with_compiler_loc()
                })?;
                resolved.insert(
                    (PortDirection::Output, port),
                    invocation.output_cell().current_memory_footprint()?,
                );
            }
        }
        let published_outputs = [invocation.output_cell().current_memory_footprint()?];
        let plan =
            resolve_current_call_memory(&self.plan, &current, &resolved, Some(&published_outputs))
                .map_err(|error| MechError::new(error, None).with_compiler_loc())?;
        if plan == *self.plan {
            Ok(None)
        } else {
            Ok(Some(Rc::new(plan)))
        }
    }

    fn initialize_inputs(&self, invocation: &FunctionInvocation) -> MResult<()> {
        let mut frame = self
            .domain
            .acquire_call(&self.realized, &self.initialization)?;
        for (index, planned) in self.plan.inputs.iter().enumerate() {
            let input = invocation.planned_input_cell(self.plan.as_ref(), index)?;
            if !input.requires_planned_import(&self.realized)? {
                continue;
            }
            let object = self
                .domain
                .plan_object_key(self.realized.revision(), planned.object)?;
            if input.has_managed_canonical_storage()?
                && planned.value.storage.planned_slot() == PlannedSlotKind::CanonicalValueHandle
            {
                // Canonical inputs are immutable roots retained by their
                // logical cells. The call object is the admitted access and
                // accounting envelope, not a second serialized copy of the
                // value. Mark the complete envelope initialized only after
                // the binding and import authority have been validated.
                self.realized
                    .record_initialized(object, self.realized.binding(object)?.capacity_bytes())?;
                continue;
            }
            let value = input.snapshot()?;
            crate::cell_binding::initialize_managed_object_from_value(
                &mut frame,
                object,
                input.representation(),
                &value,
            )?;
        }
        Ok(())
    }
}

fn validate_transaction_authority(plan: &CallMemoryPlan) -> MResult<()> {
    if plan.outputs.len() != plan.transactions.len()
        || plan.outputs.len() != plan.aliases.len()
        || plan.outputs.len() != plan.output_storage.len()
    {
        return Err(MemoryRuntimeError::CandidateValidationFailed {
            object: None,
            reason: "every managed output requires one explicit transaction authority".into(),
        }
        .into());
    }
    let requirements = plan
        .bound_call
        .operation_descriptor()
        .contract
        .memory_requirements(plan.bound_call.inputs().len())
        .map_err(|_| MemoryRuntimeError::CandidateValidationFailed {
            object: None,
            reason: "managed transaction authority has an invalid operation contract".into(),
        })?;
    if requirements.outputs.len() != plan.outputs.len() {
        return Err(MemoryRuntimeError::CandidateValidationFailed {
            object: None,
            reason: "managed transaction authority differs from the output contract".into(),
        }
        .into());
    }
    let valid_stage = |id: MemoryObjectId, output: &PortMemoryPlan, ordinal: usize| {
        plan.allocations.iter().any(|allocation| {
            allocation.id == id
                && allocation.role == AllocationRole::TransactionStage
                && allocation.slot == Some(output.value.storage.planned_slot())
                && allocation.space == plan.output_storage[ordinal].space
        })
    };
    for (ordinal, ((output, transaction), requirement)) in plan
        .outputs
        .iter()
        .zip(plan.transactions.iter())
        .zip(requirements.outputs.iter())
        .enumerate()
    {
        let valid = match requirement.construction.as_ref() {
            None => matches!(transaction, TransactionRequirement::None),
            Some(_) => match requirement.alias {
                Some(AliasPolicy::InPlaceRequired { input }) => {
                    let target = plan.inputs.get(input as usize).map(|input| input.object);
                    matches!(plan.aliases.get(ordinal), Some(AliasDecision::InPlaceRequired { input: planned }) if *planned == input)
                        && matches!(transaction, TransactionRequirement::UndoSnapshot { target: actual, undo } if Some(*actual) == target && valid_stage(*undo, output, ordinal))
                }
                _ if matches!(
                    plan.target.kind,
                    MemoryTargetKind::ResidentCpu | MemoryTargetKind::Gpu
                ) && plan.output_storage[ordinal].lifetime == MemoryLifetime::Activation =>
                {
                    matches!(transaction, TransactionRequirement::DoubleBuffer { current, next } if *current == output.object && valid_stage(*next, output, ordinal))
                }
                _ => {
                    matches!(transaction, TransactionRequirement::StageAndSwap { current, staged } if *current == output.object && valid_stage(*staged, output, ordinal))
                }
            },
        };
        if !valid {
            return Err(MemoryRuntimeError::CandidateValidationFailed {
                object: Some(output.object),
                reason:
                    "managed transaction variant or object pair differs from the output contract"
                        .into(),
            }
            .into());
        }
    }
    Ok(())
}

impl FunctionInstance {
    pub fn new(
        implementation: Box<dyn MechFunction>,
        invocation: FunctionInvocation,
        plan: Rc<CallMemoryPlan>,
    ) -> MResult<Self> {
        validate_transaction_authority(&plan)?;
        invocation
            .check_operation_memory_contract(&plan.bound_call.operation_descriptor().contract)?;
        let domain = invocation.output_cell().memory_domain().ok_or_else(|| {
            MechError::from(MemoryRuntimeError::CandidateValidationFailed {
                object: None,
                reason: "owned function output has no memory session".into(),
            })
        })?;
        let current = ManagedCallRealization::prepare(domain, plan, &invocation)?;
        Ok(Self {
            implementation,
            invocation,
            managed: ManagedFunctionBinding {
                plan: current.plan.clone(),
                current: core::cell::RefCell::new(current),
            },
        })
    }

    pub fn solve_result(&self) -> MResult<()> {
        let mut services = NoMechExecutionServices;
        self.solve_result_with(&mut services)
    }

    pub fn solve_result_with(&self, services: &mut dyn MechExecutionServices) -> MResult<()> {
        self.solve_reactive_with(services).map(|_| ())
    }

    /// Initialization uses the same admitted ports and execution scope as a
    /// solve, but does not publish or recompute a preserved planned output.
    pub fn initialize_preserved_output_with(
        &self,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<()> {
        let current =
            self.managed.current.try_borrow().map_err(|_| {
                MechError::new(ManagedExecutionScopeRequired, None).with_compiler_loc()
            })?;
        current
            .domain
            .require_standalone_execution()
            .map_err(managed_scope_error)?;
        let _scope = current
            .domain
            .enter_realized_plan_point(&current.realized, MemoryPlanPoint::new(0))
            .map_err(managed_scope_error)?;
        current.initialize_inputs(&self.invocation)?;
        let mut frame = current
            .domain
            .acquire_call(&current.realized, &current.execution)?;
        self.implementation
            .initialize_preserved_output_with(&mut frame, services)
    }

    pub fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        let mut services = NoMechExecutionServices;
        self.solve_reactive_with(&mut services)
    }

    pub fn solve_reactive_with(
        &self,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        let mut prepared = self.prepare_reactive_publication(services)?;
        let status = prepared.status;
        if let Some(publication) = prepared.publication.take() {
            PreparedCellPublicationBatch::new(vec![publication])?
                .ready()?
                .commit();
        }
        // Effect-only calls have no cell publication to carry a revised input
        // geometry. Their successfully executed realization is nevertheless
        // the new cold-path binding authority and must be promoted after the
        // execution scope has validated it. Failed calls never reach here.
        self.promote_prepared_realization(&mut prepared);
        Ok(status)
    }

    /// Runs this implementation inside an executor-owned managed frame.
    ///
    /// The outer executor owns plan-point lifetime, transaction staging, and
    /// publication. This method deliberately performs none of those actions:
    /// it is the only entry used by program-wide executors that already hold
    /// the complete call scope, and it cannot create a nested independent
    /// publication.
    pub fn solve_in_scope(
        &self,
        frame: &mut KernelMemoryFrame<'_>,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        self.implementation.solve_managed(frame, services)
    }

    fn prepare_reactive_publication(
        &self,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<PreparedFunctionPublication> {
        let binding = &self.managed;
        let current = binding
            .current
            .try_borrow()
            .map_err(|_| MechError::new(ManagedExecutionScopeRequired, None).with_compiler_loc())?;
        current
            .domain
            .require_standalone_execution()
            .map_err(managed_scope_error)?;
        let output_shapes = self.implementation.planned_output_shapes()?;
        let output_footprints = self.implementation.planned_output_footprints()?;
        let candidate = current
            .refreshed_plan(
                &self.invocation,
                output_shapes.as_deref(),
                output_footprints.as_deref(),
            )?
            .map(|plan| {
                ManagedCallRealization::prepare(current.domain.clone(), plan, &self.invocation)
            })
            .transpose()?;
        let managed = candidate.as_ref().unwrap_or(&current);
        let _scope = managed
            .domain
            .enter_realized_plan_point(&managed.realized, MemoryPlanPoint::new(0))
            .map_err(managed_scope_error)?;
        managed.initialize_inputs(&self.invocation)?;
        let publication_shape = managed
            .plan
            .outputs
            .first()
            .map(|output| output.descriptor.shape().clone());
        let (status, staged_output) = {
            let mut frame = managed
                .domain
                .acquire_call(&managed.realized, &managed.execution)
                .map_err(MechError::from)?;
            let status = self.solve_in_scope(&mut frame, services)?;
            if managed.plan.outputs.is_empty() {
                (status, None)
            } else {
                if status == ReactiveSolveStatus::Unchanged {
                    // No publication is requested. A kernel may deliberately leave
                    // its stage uninitialized; the old binding and its own content
                    // version remain authoritative. Dropping a cold candidate here
                    // also leaves the previous executable realization intact.
                    return Ok(PreparedFunctionPublication {
                        status,
                        publication: None,
                        next_realization: None,
                        _execution_scope: None,
                    });
                }
                let (object, region) = frame.output_target(self.output(), 0)?;
                let value = match frame.take_staged_output_value(object) {
                    Some(value) => value,
                    None => {
                        let shape = publication_shape.as_ref().ok_or_else(|| {
                            MechError::new(MemoryPlanError::DescriptorArityMismatch, None)
                                .with_compiler_loc()
                        })?;
                        let data = crate::cell_binding::snapshot_managed_host_data(
                            &frame,
                            object,
                            self.output().representation(),
                        )?;
                        crate::cell_binding::finalize_draft(
                            self.output().schema(),
                            shape,
                            self.output().schema_table().as_ref(),
                            data,
                        )?
                    }
                };
                let undo = frame.take_undo_snapshot().map_err(MechError::from)?;
                (status, Some((object, region, value, undo)))
            }
        };
        let Some((output_object, output_region, value, undo)) = staged_output else {
            return Ok(PreparedFunctionPublication {
                status,
                publication: None,
                next_realization: candidate,
                _execution_scope: Some(_scope),
            });
        };
        let output = self.output();
        let binding = managed
            .realized
            .binding(output_object)
            .map_err(MechError::from)?;
        let publication = managed.domain.prepare_cell_publication_with_undo(
            &managed.realized,
            vec![CellPublicationCandidate {
                cell: output.clone(),
                object: output_object,
                binding,
                region: output_region,
                value,
                changed: status == ReactiveSolveStatus::Changed,
            }],
            undo,
        )?;
        let execution_scope = publication.requires_active_plan().then_some(_scope);
        Ok(PreparedFunctionPublication {
            status,
            publication: Some(publication),
            next_realization: candidate,
            _execution_scope: execution_scope,
        })
    }

    fn promote_prepared_realization(&self, prepared: &mut PreparedFunctionPublication) {
        if let Some(candidate) = prepared.next_realization.take() {
            *self.managed.current.borrow_mut() = candidate;
        }
    }

    pub fn invocation(&self) -> &FunctionInvocation {
        &self.invocation
    }

    pub fn memory_plan(&self) -> &CallMemoryPlan {
        self.managed.plan.as_ref()
    }

    pub fn output(&self) -> &ValueCell {
        self.invocation.output_cell()
    }

    pub fn inputs(&self) -> &[ValueCell] {
        self.invocation.input_cells()
    }

    pub fn implementation(&self) -> &(dyn MechFunction + 'static) {
        self.implementation.as_ref()
    }

    pub(crate) fn implementation_mut(&mut self) -> &mut (dyn MechFunction + 'static) {
        self.implementation.as_mut()
    }

    pub fn reactive_output_cell_ids(&self) -> Vec<CanonicalCellId> {
        let mut cells = vec![self.output().reactive_cell_id()];
        if let Some(outputs) = self.implementation.reactive_output_state_ports() {
            for output in outputs {
                let cell = output.logical_cell_id();
                if !cells.contains(&cell) {
                    cells.push(cell);
                }
            }
        } else {
            for output in self.implementation.reactive_output_value_cells() {
                let cell = output.reactive_cell_id();
                if !cells.contains(&cell) {
                    cells.push(cell);
                }
            }
        }
        cells
    }

    pub fn reactive_input_cell_ids(&self) -> Vec<CanonicalCellId> {
        self.inputs()
            .iter()
            .map(ValueCell::reactive_cell_id)
            .collect()
    }

    pub fn capture_state(&self, journal: &mut FunctionCheckpoint) -> MResult<()> {
        journal.capture_value_cell(self.output())?;
        self.implementation.capture_retained_state(journal)?;
        Ok(())
    }

    pub fn with_semantic_operation(self, operation: impl Into<Box<str>>) -> Self {
        let Self {
            implementation,
            invocation,
            managed,
        } = self;
        Self {
            implementation: with_semantic_operation(operation, implementation),
            invocation,
            managed,
        }
    }
}

fn managed_scope_error(error: MemoryRuntimeError) -> MechError {
    if matches!(error, MemoryRuntimeError::TurnInFlight) {
        MechError::new(ManagedExecutionScopeRequired, None).with_compiler_loc()
    } else {
        MechError::from(error)
    }
}

/// Test fixtures use the production semantic planner and admitted constructor;
/// no test-only executable can be created with missing memory authority.
#[cfg(test)]
pub(crate) fn test_planned_instance(
    implementation: Box<dyn MechFunction>,
    invocation: FunctionInvocation,
) -> FunctionInstance {
    let alias = invocation
        .input_cells()
        .iter()
        .position(|input| input.same_logical_cell(invocation.output_cell()))
        .map_or(AliasPolicy::NoAlias, |input| AliasPolicy::MayAlias {
            input: u16::try_from(input).unwrap(),
        });
    let declaration = OperationContractDeclaration {
        inputs: InputPortLayout::Variadic {
            prefix: Box::new([]),
            repeated: InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            },
            min_repetitions: 0,
        },
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    };
    SpecializedFunction::syntax_directed(
        (implementation, invocation),
        ResolvedOperationDescriptor::from_name("test/managed-fixture", declaration).unwrap(),
        RuntimeFunctionId::from_name("test/managed-fixture"),
        ExecutionTarget::DirectRuntime,
        ImplementationMemoryClass::NoAdditionalScratch,
    )
    .unwrap()
    .into_instance()
}

#[cfg(all(test, feature = "f64"))]
mod canonical_function_instance_tests {
    use super::*;

    struct CanonicalStateFunction {
        output: ManagedPort<f64>,
        hidden: Ref<f64>,
    }

    struct OutputOnlyFunction;

    struct ExplicitSemanticFunction;

    impl MechFunctionImpl for OutputOnlyFunction {
        fn solve_managed(
            &self,
            _frame: &mut KernelMemoryFrame<'_>,
            _services: &mut dyn MechExecutionServices,
        ) -> MResult<ReactiveSolveStatus> {
            (|| -> MResult<()> { Ok(()) })()?;
            Ok(ReactiveSolveStatus::Changed)
        }

        fn to_string(&self) -> String {
            "OutputOnlyFunction".into()
        }
    }

    impl MechFunctionImpl for ExplicitSemanticFunction {
        fn solve_managed(
            &self,
            _frame: &mut KernelMemoryFrame<'_>,
            _services: &mut dyn MechExecutionServices,
        ) -> MResult<ReactiveSolveStatus> {
            (|| -> MResult<()> { Ok(()) })()?;
            Ok(ReactiveSolveStatus::Changed)
        }

        fn semantic_operation_name(&self) -> Option<&str> {
            Some("test/specialized")
        }

        fn to_string(&self) -> String {
            "ExplicitSemanticFunction".into()
        }
    }

    struct RepeatedOutputFunction {
        output: ValueCell,
    }

    impl MechFunctionImpl for RepeatedOutputFunction {
        fn solve_managed(
            &self,
            _frame: &mut KernelMemoryFrame<'_>,
            _services: &mut dyn MechExecutionServices,
        ) -> MResult<ReactiveSolveStatus> {
            (|| -> MResult<()> { Ok(()) })()?;
            Ok(ReactiveSolveStatus::Changed)
        }

        fn retained_state_ports(&self) -> MResult<Vec<FunctionStatePort<'_>>> {
            Ok(vec![
                FunctionStatePort::from_cell(&self.output),
                FunctionStatePort::from_cell(&self.output),
            ])
        }

        fn to_string(&self) -> String {
            "RepeatedOutputFunction".into()
        }
    }

    impl MechFunctionImpl for CanonicalStateFunction {
        fn solve_managed(
            &self,
            frame: &mut KernelMemoryFrame<'_>,
            _services: &mut dyn MechExecutionServices,
        ) -> MResult<ReactiveSolveStatus> {
            (|| -> MResult<()> {
                frame.with_port_init_writer(&self.output, |writer| writer.write_next(2.0))?;
                *self.hidden.borrow_mut() += 1.0;
                Ok(())
            })()?;
            Ok(ReactiveSolveStatus::Changed)
        }

        fn retained_state_ports(&self) -> MResult<Vec<FunctionStatePort<'_>>> {
            Ok(vec![FunctionStatePort::from_ref(&self.hidden)])
        }

        fn to_string(&self) -> String {
            "CanonicalStateFunction".into()
        }
    }

    #[cfg(feature = "semantic-compiler")]
    impl MechFunctionCompiler for CanonicalStateFunction {
        fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            unreachable!("test function is never compiled")
        }
    }

    #[cfg(feature = "semantic-compiler")]
    impl MechFunctionCompiler for OutputOnlyFunction {
        fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            unreachable!("test function is never compiled")
        }
    }

    #[cfg(feature = "semantic-compiler")]
    impl MechFunctionCompiler for ExplicitSemanticFunction {
        fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            unreachable!("test function is never compiled")
        }
    }

    #[cfg(feature = "semantic-compiler")]
    impl MechFunctionCompiler for RepeatedOutputFunction {
        fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            unreachable!("test function is never compiled")
        }
    }

    #[test]
    fn bound_instances_capture_visible_output_once_even_when_retained_ports_repeat_it() {
        let output_cell = ValueCell::from_exact(1.0_f64).unwrap();

        let output_only = crate::function::test_planned_instance(
            Box::new(OutputOnlyFunction),
            FunctionInvocation::nullary(output_cell.clone()),
        );
        let mut journal = FunctionCheckpoint::new();
        output_only.capture_state(&mut journal).unwrap();
        assert_eq!(journal.cell_count(), 1);

        let repeated = crate::function::test_planned_instance(
            with_semantic_operation(
                "test/repeated-output",
                Box::new(RepeatedOutputFunction {
                    output: output_cell.clone(),
                }),
            ),
            FunctionInvocation::nullary(output_cell),
        );
        let mut journal = FunctionCheckpoint::new();
        repeated.capture_state(&mut journal).unwrap();
        assert_eq!(journal.cell_count(), 1);
    }

    #[test]
    fn semantic_wrappers_preserve_explicit_specialized_operation_identity() {
        let fallback = with_semantic_operation("test/source", Box::new(OutputOnlyFunction));
        assert_eq!(fallback.semantic_operation_name(), Some("test/source"));

        let specialized =
            with_semantic_operation("test/source", Box::new(ExplicitSemanticFunction));
        assert_eq!(
            specialized.semantic_operation_name(),
            Some("test/specialized")
        );
    }

    #[test]
    fn bound_instances_checkpoint_visible_output_and_hidden_state_without_legacy_methods() {
        let hidden = Ref::new(10.0_f64);
        let output_cell = ValueCell::from_exact(1.0_f64).unwrap();
        let invocation = FunctionInvocation::nullary(output_cell.clone());
        let instance = crate::function::test_planned_instance(
            Box::new(CanonicalStateFunction {
                output: ManagedPort::output(output_cell.clone()),
                hidden: hidden.clone(),
            }),
            invocation,
        );
        let mut journal = FunctionCheckpoint::new();

        assert!(instance.output().same_cell(&output_cell));
        instance.capture_state(&mut journal).unwrap();
        assert_eq!(journal.cell_count(), 2);
        instance.solve_result().unwrap();
        assert!(matches!(
            output_cell.snapshot().unwrap().data(),
            ValueData::F64(value) if value.to_f64() == 2.0
        ));
        assert_eq!(*hidden.borrow(), 11.0);
        journal.restore_before().unwrap();
        assert!(matches!(
            output_cell.snapshot().unwrap().data(),
            ValueData::F64(value) if value.to_f64() == 1.0
        ));
        assert_eq!(*hidden.borrow(), 10.0);
    }

    #[test]
    fn reactive_plans_retain_bound_instances_and_index_their_canonical_cells() {
        let hidden = Ref::new(10.0_f64);
        let output_cell = ValueCell::from_exact(1.0_f64).unwrap();
        let input_cell = ValueCell::from_exact(4.0_f64).unwrap();
        let invocation = FunctionInvocation::unary(output_cell.clone(), input_cell.clone());
        let instance = crate::function::test_planned_instance(
            Box::new(CanonicalStateFunction {
                output: ManagedPort::output(output_cell.clone()),
                hidden,
            }),
            invocation,
        );
        let plan = Plan::new();

        let node_id = plan.register_instance(instance).unwrap();
        let plan = plan.borrow();
        let node = plan.node(node_id).unwrap();
        let retained = node.function.instance().unwrap();

        assert!(retained.output().same_cell(&output_cell));
        assert!(retained.inputs()[0].same_cell(&input_cell));
        assert_eq!(node.outputs, vec![output_cell.reactive_cell_id()]);
        assert_eq!(
            node.inputs,
            vec![ReactiveDependency {
                cell: input_cell.reactive_cell_id(),
                kind: ReactiveDependencyKind::Reactive,
            }],
        );
    }
}

/// Attaches portable semantic identity to a selected runtime implementation.
///
/// Specialization is allowed to choose implementation-specific factory names,
/// but semantic artifacts and compute backends must continue to see the
/// source-level operation that was selected. An implementation may declare a
/// more specific portable operation identity; the attached identity is its
/// fallback when it does not.
pub fn with_semantic_operation(
    operation: impl Into<Box<str>>,
    function: Box<dyn MechFunction>,
) -> Box<dyn MechFunction> {
    Box::new(SemanticMechFunction {
        operation: operation.into(),
        function,
    })
}

struct SemanticMechFunction {
    operation: Box<str>,
    function: Box<dyn MechFunction>,
}

impl MechFunctionImpl for SemanticMechFunction {
    fn solve_managed(
        &self,
        frame: &mut KernelMemoryFrame<'_>,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        self.function.solve_managed(frame, services)
    }

    fn initial_solve_policy(&self) -> InitialSolvePolicy {
        self.function.initial_solve_policy()
    }

    fn initialize_preserved_output_with(
        &self,
        frame: &mut KernelMemoryFrame<'_>,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<()> {
        self.function
            .initialize_preserved_output_with(frame, services)
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        self.function.primary_output_state_port()
    }

    fn reactive_output_state_ports(&self) -> Option<Vec<FunctionStatePort<'_>>> {
        self.function.reactive_output_state_ports()
    }

    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        self.function.transaction_state_ports()
    }

    fn capture_retained_state(&self, journal: &mut FunctionCheckpoint) -> MResult<()> {
        self.function.capture_retained_state(journal)
    }

    fn reactive_output_value_cells(&self) -> Vec<ValueCell> {
        self.function.reactive_output_value_cells()
    }

    fn reactive_dependency_kinds(
        &self,
        argument_count: usize,
    ) -> Option<Vec<ReactiveDependencyKind>> {
        self.function.reactive_dependency_kinds(argument_count)
    }

    fn reactive_dependency_scopes(
        &self,
        argument_count: usize,
    ) -> Option<Vec<ReactiveDependencyScope>> {
        self.function.reactive_dependency_scopes(argument_count)
    }

    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        self.function.reactive_node_kind()
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        self.function.semantic_operation_contract()
    }

    fn semantic_operation_name(&self) -> Option<&str> {
        self.function
            .semantic_operation_name()
            .or(Some(&self.operation))
    }

    fn to_string(&self) -> String {
        self.function.to_string()
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for SemanticMechFunction {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        self.function.compiler_owned_value_cells()
    }

    fn reserve_bytecode_registers(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<()> {
        self.function.reserve_bytecode_registers(ctx)
    }

    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        self.function.compile(ctx)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuardFunctionSafety {
    /// Compilation constructs only the static graph (including kind and shape
    /// selection) without executing function behavior or reading live contents;
    /// reactive solving is deferred to the guard pulse.
    PureStatic,
    /// Compilation may execute work or lacks an explicit purity contract.
    Unsupported,
}

#[derive(Debug, Clone)]
pub struct TransactionStateUnsupportedError {
    pub function: String,
    pub reason: String,
}

impl MechErrorKind for TransactionStateUnsupportedError {
    fn name(&self) -> &str {
        "TransactionStateUnsupported"
    }
    fn message(&self) -> String {
        format!(
            "Cannot checkpoint retained transaction state for function '{}': {}.",
            self.function, self.reason,
        )
    }
}

#[derive(Debug, Clone)]
pub struct TransactionStateBorrowConflictError {
    pub function: String,
    pub component: &'static str,
}

impl MechErrorKind for TransactionStateBorrowConflictError {
    fn name(&self) -> &str {
        "TransactionStateBorrowConflict"
    }
    fn message(&self) -> String {
        format!(
            "Cannot inspect retained transaction state for function '{}' because {} is already borrowed.",
            self.function, self.component,
        )
    }
}

#[derive(Clone)]
pub struct FunctionDefinition {
    pub code: FunctionDefine,
    pub id: u64,
    pub name: String,
    pub input: IndexMap<u64, KindAnnotation>,
    pub output: IndexMap<u64, KindAnnotation>,
    pub symbols: SymbolTableRef,
    pub out: ValueCell,
    pub plan: Plan,
}

impl fmt::Debug for FunctionDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(feature = "pretty_print")]
        return fmt::Display::fmt(&self.pretty_print(), f);
        #[cfg(not(feature = "pretty_print"))]
        write!(
            f,
            "FunctionDefinition {{ id: {}, name: {}, input: {:?}, output: {:?}, symbols: {:?} }}",
            self.id,
            self.name,
            self.input,
            self.output,
            self.symbols.borrow()
        )
    }
}

#[cfg(feature = "pretty_print")]
impl PrettyPrint for FunctionDefinition {
    fn pretty_print(&self) -> String {
        let input_str = format!("{:#?}", self.input);
        let output_str = format!("{:#?}", self.output);
        let symbols_str = format!("{:#?}", self.symbols);
        let mut plan_str = "".to_string();
        for step in self.plan.borrow().iter() {
            plan_str = format!("{}  - {}\n", plan_str, step.to_string());
        }
        let data = vec![
            "📥 Input",
            &input_str,
            "📤 Output",
            &output_str,
            "🔣 Symbols",
            &symbols_str,
            "📋 Plan",
            &plan_str,
        ];
        let mut table = tabled::Table::new(data);
        table
            .with(Style::modern_rounded())
            .with(Panel::header(format!(
                "📈 UserFxn::{}\n({})",
                self.name,
                humanize(&self.id)
            )))
            .with(Alignment::left());
        format!("{table}")
    }
}

impl FunctionDefinition {
    pub fn new(id: u64, name: String, code: FunctionDefine) -> Self {
        Self {
            id,
            name,
            code,
            input: IndexMap::new(),
            output: IndexMap::new(),
            out: ValueCell::unit(),
            symbols: Ref::new(SymbolTable::new()),
            plan: Plan::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn solve_result(&self) -> MResult<ValueCell> {
        let plan_brrw = self.plan.borrow();
        for step in plan_brrw.iter() {
            step.solve_result()?;
        }
        Ok(self.out.clone())
    }

    pub fn out(&self) -> ValueCell {
        self.out.clone()
    }
}

// User Function --------------------------------------------------------------

pub struct UserFunction {
    pub fxn: FunctionDefinition,
}

// Reactive Plan
// ----------------------------------------------------------------------------

pub type ReactiveNodeId = usize;

/// Read-only registration information for a patterned activation.
///
/// This belongs to the plan, rather than to a turn, so consumers can inspect
/// the statically registered dispatch graph without relying on transient
/// scheduler state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternActivationRegistration {
    pub scope_pulse_node: ReactiveNodeId,
    pub selector_node: ReactiveNodeId,
    pub arms: Vec<PatternActivationArmRegistration>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternActivationArmRegistration {
    pub matcher_node: ReactiveNodeId,
    /// The structural-only finalizer for an unguarded arm, or the unmatched
    /// finalization path for a guarded arm.
    pub finalizer_node: ReactiveNodeId,
    pub guard: Option<PatternActivationGuardRegistration>,
    pub gate_node: ReactiveNodeId,
    pub pulse_cell: CanonicalCellId,
    /// Half-open range of plan nodes registered for this arm's body.
    pub body_node_start: usize,
    pub body_node_end: usize,
    pub captures: Vec<PatternActivationCaptureRegistration>,
}

/// Static graph information for one guarded patterned-activation arm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternActivationGuardRegistration {
    pub match_gate_node: ReactiveNodeId,
    pub guard_finalizer_node: ReactiveNodeId,
    /// Half-open range containing the ordinary guard-expression graph followed
    /// by its guard finalizer.
    pub guard_node_start: usize,
    pub guard_node_end: usize,
}

/// Stable storage made available to a single patterned activation arm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternActivationCaptureRegistration {
    pub id: u64,
    pub schema: SchemaBody,
    pub cell: CanonicalCellId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReactiveDependencyKind {
    Reactive,
    Sampled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReactiveDependencyScope {
    Recursive,
    Logical,
    Root,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReactiveNodeKind {
    Combinational,
    Register,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReactiveSolveStatus {
    Changed,
    Unchanged,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReactivePlanSolveOutcome {
    pub executed_nodes: Vec<ReactiveNodeId>,
    pub changed_nodes: Vec<ReactiveNodeId>,
    pub unchanged_nodes: Vec<ReactiveNodeId>,
    pub pending_register_nodes: Vec<ReactiveNodeId>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReactiveRegisterCommitOutcome {
    pub staged_nodes: Vec<ReactiveNodeId>,
    pub committed_nodes: Vec<ReactiveNodeId>,
    pub dirty_cells: Vec<CanonicalCellId>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReactiveTurnState {
    pub pending_register_nodes: Vec<ReactiveNodeId>,
}

impl ReactiveTurnState {
    pub fn has_pending_registers(&self) -> bool {
        !self.pending_register_nodes.is_empty()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReactiveTurnOutcome {
    pub before_commit: ReactivePlanSolveOutcome,
    pub register_commit: ReactiveRegisterCommitOutcome,
    pub after_commit: ReactivePlanSolveOutcome,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivationRegistrationScope {
    pub trigger_cells: Vec<CanonicalCellId>,
    pub local_combinational_cells: Vec<CanonicalCellId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReactiveDependency {
    pub cell: CanonicalCellId,
    pub kind: ReactiveDependencyKind,
}

pub struct ReactivePlanFunction {
    instance: FunctionInstance,
    identity: Rc<()>,
}

impl ReactivePlanFunction {
    fn new_instance(instance: FunctionInstance) -> Self {
        Self {
            instance,
            identity: Rc::new(()),
        }
    }

    pub fn as_ref(&self) -> &(dyn MechFunction + 'static) {
        self.instance.implementation()
    }

    pub fn instance(&self) -> Option<&FunctionInstance> {
        Some(&self.instance)
    }

    pub fn bound_call(&self) -> Option<&BoundCall> {
        Some(&self.instance.memory_plan().bound_call)
    }

    pub fn memory_plan(&self) -> Option<&CallMemoryPlan> {
        Some(self.instance.memory_plan())
    }

    pub fn solve_result(&self) -> MResult<()> {
        self.instance.solve_result()
    }

    pub fn solve_result_with(&self, services: &mut dyn MechExecutionServices) -> MResult<()> {
        self.instance.solve_result_with(services)
    }

    pub fn solve_reactive_with(
        &self,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        self.instance.solve_reactive_with(services)
    }

    fn capture_reactive_state(&self, journal: &mut CanonicalTurnJournal) -> MResult<()> {
        journal.capture_function_instance(&self.instance)
    }
}

impl core::ops::Deref for ReactivePlanFunction {
    type Target = dyn MechFunction;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl core::ops::DerefMut for ReactivePlanFunction {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.instance.implementation_mut()
    }
}

pub struct ReactivePlanNode {
    pub id: ReactiveNodeId,
    pub plan_index: usize,
    pub function: ReactivePlanFunction,
    pub inputs: Vec<ReactiveDependency>,
    pub outputs: Vec<CanonicalCellId>,
    pub kind: ReactiveNodeKind,
}

pub struct ReactivePlan {
    pub nodes: Vec<ReactivePlanNode>,
    pub reactive_consumers: HashMap<CanonicalCellId, Vec<ReactiveNodeId>>,
    pub sampled_consumers: HashMap<CanonicalCellId, Vec<ReactiveNodeId>>,
    pattern_activation_registrations: Vec<PatternActivationRegistration>,
    activation_sampled_cells: Vec<Vec<CanonicalCellId>>,
}

#[derive(Debug, Clone)]
pub struct ReactiveDependencyArityMismatchError {
    pub function: String,
    pub expected: usize,
    pub found: usize,
}

impl MechErrorKind for ReactiveDependencyArityMismatchError {
    fn name(&self) -> &str {
        "ReactiveDependencyArityMismatch"
    }

    fn message(&self) -> String {
        format!(
            "Reactive dependency arity mismatch for function '{}': expected {} dependency kinds, found {}.",
            self.function, self.expected, self.found,
        )
    }
}

#[derive(Debug, Clone)]
pub struct ReactiveDependencyScopeArityMismatchError {
    pub function: String,
    pub expected: usize,
    pub found: usize,
}

impl MechErrorKind for ReactiveDependencyScopeArityMismatchError {
    fn name(&self) -> &str {
        "ReactiveDependencyScopeArityMismatch"
    }

    fn message(&self) -> String {
        format!(
            "Reactive dependency scope arity mismatch for function '{}': expected argument count {}, provided scope count {}.",
            self.function, self.expected, self.found,
        )
    }
}

#[derive(Debug, Clone)]
pub struct ReactiveDependencyKindConflictError {
    pub function: String,
    pub cell: CanonicalCellId,
}

#[derive(Debug, Clone)]
pub struct ReactiveRegisterStagingUnsupportedError {
    pub function: String,
}
impl MechErrorKind for ReactiveRegisterStagingUnsupportedError {
    fn name(&self) -> &str {
        "ReactiveRegisterStagingUnsupported"
    }
    fn message(&self) -> String {
        format!(
            "Reactive register staging is not implemented for function '{}'.",
            self.function
        )
    }
}
#[derive(Debug, Clone)]
pub struct ReactiveRegisterNodeNotFoundError {
    pub node_id: ReactiveNodeId,
}
impl MechErrorKind for ReactiveRegisterNodeNotFoundError {
    fn name(&self) -> &str {
        "ReactiveRegisterNodeNotFound"
    }
    fn message(&self) -> String {
        format!("Reactive register node {} does not exist.", self.node_id)
    }
}
#[derive(Debug, Clone)]
pub struct ReactiveRegisterNodeKindError {
    pub node_id: ReactiveNodeId,
    pub actual: ReactiveNodeKind,
}
impl MechErrorKind for ReactiveRegisterNodeKindError {
    fn name(&self) -> &str {
        "ReactiveRegisterNodeKind"
    }
    fn message(&self) -> String {
        format!(
            "Reactive node {} must be a register for commit, but its kind is {:?}.",
            self.node_id, self.actual
        )
    }
}
#[derive(Debug, Clone)]
pub struct ReactiveRegisterOutputConflictError {
    pub cell: CanonicalCellId,
    pub first_node: ReactiveNodeId,
    pub second_node: ReactiveNodeId,
}
impl MechErrorKind for ReactiveRegisterOutputConflictError {
    fn name(&self) -> &str {
        "ReactiveRegisterOutputConflict"
    }
    fn message(&self) -> String {
        format!(
            "Reactive register nodes {} and {} both write output cell {:?}.",
            self.first_node, self.second_node, self.cell
        )
    }
}
#[derive(Debug, Clone)]
pub struct ReactiveRegisterStagedOutputMismatchError {
    pub node_id: ReactiveNodeId,
    pub expected: Vec<CanonicalCellId>,
    pub found: Vec<CanonicalCellId>,
}
impl MechErrorKind for ReactiveRegisterStagedOutputMismatchError {
    fn name(&self) -> &str {
        "ReactiveRegisterStagedOutputMismatch"
    }
    fn message(&self) -> String {
        format!(
            "Reactive register node {} staged outputs {:?}, but its registered outputs are {:?}.",
            self.node_id, self.found, self.expected
        )
    }
}

impl MechErrorKind for ReactiveDependencyKindConflictError {
    fn name(&self) -> &str {
        "ReactiveDependencyKindConflict"
    }

    fn message(&self) -> String {
        format!(
            "Reactive dependency kind conflict for function '{}': one node classified cell {:?} as both reactive and sampled.",
            self.function, self.cell,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ReactivePlanNodeCheckpoint {
    id: ReactiveNodeId,
    plan_index: usize,
    inputs: Vec<ReactiveDependency>,
    outputs: Vec<CanonicalCellId>,
    kind: ReactiveNodeKind,
    function_identity: ReactiveFunctionIdentity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReactivePlanCheckpoint {
    nodes: Vec<ReactivePlanNodeCheckpoint>,
    pattern_activation_registrations: Vec<PatternActivationRegistration>,
    activation_sampled_cells: Vec<Vec<CanonicalCellId>>,
}

impl ReactivePlanCheckpoint {
    pub fn node_len(&self) -> usize {
        self.nodes.len()
    }
}

/// A process-local structural checkpoint for a [`Plan`].
///
/// This checkpoint supports append-only plan elaboration. Removing or
/// replacing a function object that existed at capture time invalidates
/// restoration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanCheckpoint {
    reactive: ReactivePlanCheckpoint,
    activation_registration_scopes: Vec<ActivationRegistrationScope>,
}

impl PlanCheckpoint {
    pub fn node_len(&self) -> usize {
        self.reactive.node_len()
    }
}

#[derive(Debug, Clone)]
pub struct ReactivePlanRollbackInvariantError {
    pub checkpoint_nodes: usize,
    pub current_nodes: usize,
    pub checkpoint_registrations: usize,
    pub current_registrations: usize,
}

impl MechErrorKind for ReactivePlanRollbackInvariantError {
    fn name(&self) -> &str {
        "ReactivePlanRollbackInvariant"
    }
    fn message(&self) -> String {
        format!(
            "Cannot roll the reactive plan back from {} nodes and {} patterned registrations to {} nodes and {} patterned registrations.",
            self.current_nodes,
            self.current_registrations,
            self.checkpoint_nodes,
            self.checkpoint_registrations
        )
    }
}

#[derive(Debug, Clone)]
pub struct ReactivePlanFunctionIdentityError {
    pub node_id: ReactiveNodeId,
}

impl MechErrorKind for ReactivePlanFunctionIdentityError {
    fn name(&self) -> &str {
        "ReactivePlanFunctionIdentity"
    }
    fn message(&self) -> String {
        format!(
            "Cannot restore reactive node {} because its function identity changed.",
            self.node_id,
        )
    }
}

#[derive(Debug, Clone)]
pub struct ActivationRegistrationRollbackInvariantError {
    pub checkpoint_depth: usize,
    pub current_depth: usize,
}

impl MechErrorKind for ActivationRegistrationRollbackInvariantError {
    fn name(&self) -> &str {
        "ActivationRegistrationRollbackInvariant"
    }

    fn message(&self) -> String {
        format!(
            "Cannot roll the activation registration stack back from depth {} to future depth {}.",
            self.current_depth, self.checkpoint_depth,
        )
    }
}

#[derive(Debug, Clone)]
pub struct PlanCheckpointBorrowConflictError {
    pub phase: &'static str,
    pub component: &'static str,
}

impl MechErrorKind for PlanCheckpointBorrowConflictError {
    fn name(&self) -> &str {
        "PlanCheckpointBorrowConflict"
    }
    fn message(&self) -> String {
        format!(
            "Cannot borrow plan {} during {}.",
            self.component, self.phase,
        )
    }
}

#[derive(Debug, Clone)]
pub struct ReactivePlanCheckpointInvariantError {
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct ReactiveTurnCheckpointInvariantError {
    pub node_id: ReactiveNodeId,
    pub reason: String,
}

impl MechErrorKind for ReactiveTurnCheckpointInvariantError {
    fn name(&self) -> &str {
        "ReactiveTurnCheckpointInvariant"
    }

    fn message(&self) -> String {
        format!(
            "Cannot checkpoint pending reactive node {}: {}.",
            self.node_id, self.reason,
        )
    }
}

impl MechErrorKind for ReactivePlanCheckpointInvariantError {
    fn name(&self) -> &str {
        "ReactivePlanCheckpointInvariant"
    }

    fn message(&self) -> String {
        format!(
            "Cannot checkpoint an invalid reactive plan: {}.",
            self.reason
        )
    }
}

#[derive(Clone)]
struct ReactiveFunctionIdentity {
    owner: Rc<()>,
}

impl Debug for ReactiveFunctionIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReactiveFunctionIdentity")
            .finish_non_exhaustive()
    }
}

impl PartialEq for ReactiveFunctionIdentity {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.owner, &other.owner)
    }
}

impl Eq for ReactiveFunctionIdentity {}

fn reactive_function_identity(function: &ReactivePlanFunction) -> ReactiveFunctionIdentity {
    ReactiveFunctionIdentity {
        owner: function.identity.clone(),
    }
}

impl ReactivePlan {
    fn rebuild_consumer_indexes(&mut self) {
        self.reactive_consumers.clear();
        self.sampled_consumers.clear();

        for node in &self.nodes {
            for dependency in &node.inputs {
                let consumers = match dependency.kind {
                    ReactiveDependencyKind::Reactive => &mut self.reactive_consumers,
                    ReactiveDependencyKind::Sampled => &mut self.sampled_consumers,
                };

                let consumers = consumers.entry(dependency.cell).or_default();

                if !consumers.contains(&node.id) {
                    consumers.push(node.id);
                }
            }
        }
    }

    pub fn validate_checkpoint_invariants(&self, activation_scope_count: usize) -> MResult<()> {
        let invalid = |reason: String| {
            MechError::new(ReactivePlanCheckpointInvariantError { reason }, None)
                .with_compiler_loc()
        };
        let node_len = self.nodes.len();

        for (index, node) in self.nodes.iter().enumerate() {
            if node.id != index {
                return Err(invalid(format!(
                    "node at index {} has id {}",
                    index, node.id,
                )));
            }
            if node.plan_index != index {
                return Err(invalid(format!(
                    "node {} has plan index {}",
                    node.id, node.plan_index,
                )));
            }
        }

        let mut reactive_consumers: HashMap<CanonicalCellId, Vec<ReactiveNodeId>> =
            HashMap::default();
        let mut sampled_consumers: HashMap<CanonicalCellId, Vec<ReactiveNodeId>> =
            HashMap::default();
        for node in &self.nodes {
            for dependency in &node.inputs {
                let consumers = match dependency.kind {
                    ReactiveDependencyKind::Reactive => &mut reactive_consumers,
                    ReactiveDependencyKind::Sampled => &mut sampled_consumers,
                };
                let consumers = consumers.entry(dependency.cell).or_insert_with(Vec::new);
                if !consumers.contains(&node.id) {
                    consumers.push(node.id);
                }
            }
        }
        for consumers in reactive_consumers.values_mut() {
            consumers.sort_unstable();
        }
        for consumers in sampled_consumers.values_mut() {
            consumers.sort_unstable();
        }
        let mut indexed_reactive_consumers = self.reactive_consumers.clone();
        let mut indexed_sampled_consumers = self.sampled_consumers.clone();
        for consumers in indexed_reactive_consumers.values_mut() {
            consumers.sort_unstable();
        }
        for consumers in indexed_sampled_consumers.values_mut() {
            consumers.sort_unstable();
        }
        if reactive_consumers != indexed_reactive_consumers
            || sampled_consumers != indexed_sampled_consumers
        {
            return Err(invalid(
                "consumer indexes do not match node dependencies".into(),
            ));
        }

        let valid_node = |node: ReactiveNodeId| node < node_len;
        for registration in &self.pattern_activation_registrations {
            if !valid_node(registration.scope_pulse_node) || !valid_node(registration.selector_node)
            {
                return Err(invalid(
                    "pattern activation references a missing root node".into(),
                ));
            }
            for arm in &registration.arms {
                if !valid_node(arm.matcher_node)
                    || !valid_node(arm.finalizer_node)
                    || !valid_node(arm.gate_node)
                    || arm.body_node_start > arm.body_node_end
                    || arm.body_node_end > node_len
                {
                    return Err(invalid("pattern activation arm topology is invalid".into()));
                }
                if let Some(guard) = &arm.guard {
                    if !valid_node(guard.match_gate_node)
                        || !valid_node(guard.guard_finalizer_node)
                        || guard.guard_node_start > guard.guard_node_end
                        || guard.guard_node_end > node_len
                    {
                        return Err(invalid(
                            "pattern activation guard topology is invalid".into(),
                        ));
                    }
                }
            }
        }

        if self.activation_sampled_cells.len() != activation_scope_count {
            return Err(invalid(format!(
                "sampled-cell stack depth {} differs from activation-scope depth {}",
                self.activation_sampled_cells.len(),
                activation_scope_count,
            )));
        }

        Ok(())
    }

    pub fn preflight_rollback(&self, checkpoint: &ReactivePlanCheckpoint) -> MResult<()> {
        if checkpoint.nodes.len() > self.nodes.len() {
            return Err(MechError::new(
                ReactivePlanRollbackInvariantError {
                    checkpoint_nodes: checkpoint.nodes.len(),
                    current_nodes: self.nodes.len(),
                    checkpoint_registrations: checkpoint.pattern_activation_registrations.len(),
                    current_registrations: self.pattern_activation_registrations.len(),
                },
                None,
            ));
        }

        for (node, saved) in self.nodes.iter().zip(checkpoint.nodes.iter()) {
            let current_identity = reactive_function_identity(&node.function);
            if current_identity != saved.function_identity {
                return Err(MechError::new(
                    ReactivePlanFunctionIdentityError { node_id: saved.id },
                    None,
                ));
            }
        }

        Ok(())
    }

    pub fn apply_rollback_structure(&mut self, checkpoint: &ReactivePlanCheckpoint) {
        self.nodes.truncate(checkpoint.nodes.len());
        for (node, saved) in self.nodes.iter_mut().zip(checkpoint.nodes.iter()) {
            node.id = saved.id;
            node.plan_index = saved.plan_index;
            node.inputs = saved.inputs.clone();
            node.outputs = saved.outputs.clone();
            node.kind = saved.kind;
        }
        self.pattern_activation_registrations = checkpoint.pattern_activation_registrations.clone();
        self.activation_sampled_cells = checkpoint.activation_sampled_cells.clone();
    }

    pub fn rebuild_checkpoint_indexes(&mut self) {
        self.rebuild_consumer_indexes();
    }

    pub fn apply_rollback(&mut self, checkpoint: &ReactivePlanCheckpoint) {
        self.apply_rollback_structure(checkpoint);
        self.rebuild_checkpoint_indexes();
    }

    pub fn checkpoint(&self) -> ReactivePlanCheckpoint {
        ReactivePlanCheckpoint {
            nodes: self
                .nodes
                .iter()
                .map(|node| ReactivePlanNodeCheckpoint {
                    id: node.id,
                    plan_index: node.plan_index,
                    inputs: node.inputs.clone(),
                    outputs: node.outputs.clone(),
                    kind: node.kind,
                    function_identity: reactive_function_identity(&node.function),
                })
                .collect(),
            pattern_activation_registrations: self.pattern_activation_registrations.clone(),
            activation_sampled_cells: self.activation_sampled_cells.clone(),
        }
    }

    pub fn rollback(&mut self, checkpoint: ReactivePlanCheckpoint) -> MResult<()> {
        self.preflight_rollback(&checkpoint)?;
        self.apply_rollback(&checkpoint);
        Ok(())
    }

    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            reactive_consumers: HashMap::default(),
            sampled_consumers: HashMap::default(),
            pattern_activation_registrations: Vec::new(),
            activation_sampled_cells: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.reactive_consumers.clear();
        self.sampled_consumers.clear();
        self.pattern_activation_registrations.clear();
        self.activation_sampled_cells.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &ReactivePlanFunction> {
        self.nodes.iter().map(|node| &node.function)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut ReactivePlanFunction> {
        self.nodes.iter_mut().map(|node| &mut node.function)
    }

    pub fn capture_transaction_state(&self, journal: &mut FunctionCheckpoint) -> MResult<()> {
        for node in &self.nodes {
            node.function.instance.capture_state(journal)?;
        }
        Ok(())
    }

    pub fn last(&self) -> Option<&ReactivePlanFunction> {
        self.nodes.last().map(|node| &node.function)
    }

    pub fn push(&mut self, instance: FunctionInstance) -> MResult<ReactiveNodeId> {
        self.register_instance_with_activation(instance, None)
    }

    pub fn register_instance_with_activation(
        &mut self,
        instance: FunctionInstance,
        activation: Option<&ActivationRegistrationScope>,
    ) -> MResult<ReactiveNodeId> {
        self.register_bound_with_activation(instance, activation)
    }

    pub fn register_specialized_with_activation(
        &mut self,
        specialized: SpecializedFunction,
        activation: Option<&ActivationRegistrationScope>,
    ) -> MResult<ReactiveNodeId> {
        self.register_bound_with_activation(specialized.into_instance(), activation)
    }

    fn register_bound_with_activation(
        &mut self,
        instance: FunctionInstance,
        activation: Option<&ActivationRegistrationScope>,
    ) -> MResult<ReactiveNodeId> {
        let node_id = self.nodes.len();
        let plan_index = node_id;
        let argument_count = instance.inputs().len();
        let function = instance.implementation();
        let dependency_kinds = match function.reactive_dependency_kinds(argument_count) {
            Some(kinds) => {
                if kinds.len() != argument_count {
                    return Err(MechError::new(
                        ReactiveDependencyArityMismatchError {
                            function: function.to_string(),
                            expected: argument_count,
                            found: kinds.len(),
                        },
                        None,
                    ));
                }
                kinds
            }
            None => vec![ReactiveDependencyKind::Reactive; argument_count],
        };

        let dependency_scopes = match function.reactive_dependency_scopes(argument_count) {
            Some(scopes) => {
                if scopes.len() != argument_count {
                    return Err(MechError::new(
                        ReactiveDependencyScopeArityMismatchError {
                            function: function.to_string(),
                            expected: argument_count,
                            found: scopes.len(),
                        },
                        None,
                    ));
                }
                scopes
            }
            None => vec![ReactiveDependencyScope::Recursive; argument_count],
        };

        let node_kind = function.reactive_node_kind();
        let outputs = instance.reactive_output_cell_ids();
        let mut inputs = Vec::<ReactiveDependency>::new();

        if node_kind == ReactiveNodeKind::Register {
            for cell in &outputs {
                inputs.push(ReactiveDependency {
                    cell: *cell,
                    kind: ReactiveDependencyKind::Sampled,
                });
            }
        }

        for ((cell, kind), scope) in instance
            .inputs()
            .iter()
            .zip(dependency_kinds.iter())
            .zip(dependency_scopes.iter())
        {
            let cells = match scope {
                ReactiveDependencyScope::Recursive
                | ReactiveDependencyScope::Logical
                | ReactiveDependencyScope::Root => vec![cell.reactive_cell_id()],
                ReactiveDependencyScope::None => Vec::new(),
            };

            for cell in cells {
                let kind = activation.map_or(*kind, |scope| {
                    if scope.trigger_cells.contains(&cell)
                        || scope.local_combinational_cells.contains(&cell)
                    {
                        ReactiveDependencyKind::Reactive
                    } else {
                        ReactiveDependencyKind::Sampled
                    }
                });
                match inputs.iter().find(|dependency| dependency.cell == cell) {
                    Some(dependency) if dependency.kind == kind => {}
                    Some(dependency)
                        if node_kind == ReactiveNodeKind::Register
                            && outputs.contains(&cell)
                            && (dependency.kind == ReactiveDependencyKind::Sampled
                                || kind == ReactiveDependencyKind::Sampled) => {}
                    Some(_) => {
                        return Err(MechError::new(
                            ReactiveDependencyKindConflictError {
                                function: function.to_string(),
                                cell,
                            },
                            None,
                        ));
                    }
                    None => inputs.push(ReactiveDependency { cell, kind }),
                }
            }
        }

        if let Some(scope) = activation {
            for cell in &scope.trigger_cells {
                if !inputs.iter().any(|dependency| dependency.cell == *cell) {
                    inputs.push(ReactiveDependency {
                        cell: *cell,
                        kind: ReactiveDependencyKind::Reactive,
                    });
                }
            }
        }

        let node = ReactivePlanNode {
            id: node_id,
            plan_index,
            inputs,
            outputs,
            kind: node_kind,
            function: ReactivePlanFunction::new_instance(instance),
        };

        self.nodes.push(node);
        for dependency in &self.nodes[node_id].inputs {
            let consumers = match dependency.kind {
                ReactiveDependencyKind::Reactive => {
                    self.reactive_consumers.entry(dependency.cell).or_default()
                }
                ReactiveDependencyKind::Sampled => {
                    self.sampled_consumers.entry(dependency.cell).or_default()
                }
            };
            if !consumers.contains(&node_id) {
                consumers.push(node_id);
            }
        }
        Ok(node_id)
    }

    pub fn node(&self, node_id: ReactiveNodeId) -> Option<&ReactivePlanNode> {
        self.nodes.get(node_id)
    }

    pub fn pattern_activation_registrations(&self) -> &[PatternActivationRegistration] {
        &self.pattern_activation_registrations
    }

    pub fn register_pattern_activation(&mut self, registration: PatternActivationRegistration) {
        self.pattern_activation_registrations.push(registration);
    }

    /// Records an input that is read when another reactive cause schedules the
    /// node. Updating this cell alone must not schedule the node.
    pub fn add_sampled_dependency(
        &mut self,
        node_id: ReactiveNodeId,
        cell: CanonicalCellId,
    ) -> bool {
        let Some(node) = self.nodes.get_mut(node_id) else {
            return false;
        };
        if let Some(existing) = node
            .inputs
            .iter()
            .find(|dependency| dependency.cell == cell)
        {
            return existing.kind == ReactiveDependencyKind::Sampled
                || existing.kind == ReactiveDependencyKind::Reactive;
        }
        node.inputs.push(ReactiveDependency {
            cell,
            kind: ReactiveDependencyKind::Sampled,
        });
        let consumers = self.sampled_consumers.entry(cell).or_default();
        if !consumers.contains(&node_id) {
            consumers.push(node_id);
        }
        true
    }

    /// Records a cell that schedules this node. This is also used to repair
    /// direct combinational expression nodes that were appended while an
    /// activation-registration scope was active.
    pub fn add_reactive_dependency(
        &mut self,
        node_id: ReactiveNodeId,
        cell: CanonicalCellId,
    ) -> bool {
        let Some(node) = self.nodes.get_mut(node_id) else {
            return false;
        };
        if let Some(existing) = node
            .inputs
            .iter()
            .find(|dependency| dependency.cell == cell)
        {
            return existing.kind == ReactiveDependencyKind::Reactive;
        }
        node.inputs.push(ReactiveDependency {
            cell,
            kind: ReactiveDependencyKind::Reactive,
        });
        let consumers = self.reactive_consumers.entry(cell).or_default();
        if !consumers.contains(&node_id) {
            consumers.push(node_id);
        }
        true
    }

    pub fn reactive_consumers_for(&self, cell: CanonicalCellId) -> &[ReactiveNodeId] {
        self.reactive_consumers
            .get(&cell)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn sampled_consumers_for(&self, cell: CanonicalCellId) -> &[ReactiveNodeId] {
        self.sampled_consumers
            .get(&cell)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn solve_dirty_cells(
        &mut self,
        dirty_cells: &[CanonicalCellId],
    ) -> MResult<ReactivePlanSolveOutcome> {
        let mut services = NoMechExecutionServices;
        self.solve_dirty_cells_with_services(dirty_cells, &mut services)
    }

    pub fn solve_dirty_cells_with_services(
        &mut self,
        dirty_cells: &[CanonicalCellId],
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactivePlanSolveOutcome> {
        let mut outcome = ReactivePlanSolveOutcome::default();
        self.solve_dirty_cells_into_impl(dirty_cells, &mut outcome, None, services)?;
        Ok(outcome)
    }

    pub(crate) fn solve_dirty_cells_with_journal_and_services(
        &mut self,
        dirty_cells: &[CanonicalCellId],
        journal: &mut CanonicalTurnJournal,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactivePlanSolveOutcome> {
        let mut outcome = ReactivePlanSolveOutcome::default();
        self.solve_dirty_cells_into_with_journal_and_services(
            dirty_cells,
            &mut outcome,
            journal,
            services,
        )?;
        Ok(outcome)
    }

    fn solve_dirty_cells_into_with_journal_and_services(
        &mut self,
        dirty_cells: &[CanonicalCellId],
        outcome: &mut ReactivePlanSolveOutcome,
        journal: &mut CanonicalTurnJournal,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<()> {
        self.solve_dirty_cells_into_impl(dirty_cells, outcome, Some(journal), services)
    }

    fn solve_dirty_cells_into_impl(
        &mut self,
        dirty_cells: &[CanonicalCellId],
        outcome: &mut ReactivePlanSolveOutcome,
        mut journal: Option<&mut CanonicalTurnJournal>,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<()> {
        let dirty_cells = dirty_cells.iter().copied().collect::<HashSet<_>>();
        let mut work = BTreeSet::new();
        let mut processed = BTreeSet::new();

        for cell in dirty_cells.iter().copied() {
            for node_id in self.reactive_consumers_for(cell) {
                let node = &self.nodes[*node_id];
                work.insert((node.plan_index, node.id));
            }
        }

        while let Some((_, node_id)) = work.pop_first() {
            if !processed.insert(node_id) {
                continue;
            }

            let node = &self.nodes[node_id];
            if node.kind == ReactiveNodeKind::Register {
                outcome.pending_register_nodes.push(node.id);
                continue;
            }

            if let Some(journal) = journal.as_deref_mut() {
                node.function.capture_reactive_state(journal)?;
            }
            let status = node.function.solve_reactive_with(services)?;
            outcome.executed_nodes.push(node.id);
            match status {
                ReactiveSolveStatus::Changed => {
                    outcome.changed_nodes.push(node.id);
                    let outputs = node.outputs.clone();
                    for cell in outputs {
                        for consumer_id in self.reactive_consumers_for(cell) {
                            let consumer = &self.nodes[*consumer_id];
                            work.insert((consumer.plan_index, consumer.id));
                        }
                    }
                }
                ReactiveSolveStatus::Unchanged => outcome.unchanged_nodes.push(node.id),
            }
        }

        Ok(())
    }

    pub fn commit_pending_registers(
        &mut self,
        pending_nodes: &[ReactiveNodeId],
    ) -> MResult<ReactiveRegisterCommitOutcome> {
        self.commit_pending_registers_impl(pending_nodes, None)
    }

    pub(crate) fn commit_pending_registers_with_journal(
        &mut self,
        pending_nodes: &[ReactiveNodeId],
        journal: &mut CanonicalTurnJournal,
    ) -> MResult<ReactiveRegisterCommitOutcome> {
        self.commit_pending_registers_impl(pending_nodes, Some(journal))
    }

    fn commit_pending_registers_impl(
        &mut self,
        pending_nodes: &[ReactiveNodeId],
        mut journal: Option<&mut CanonicalTurnJournal>,
    ) -> MResult<ReactiveRegisterCommitOutcome> {
        let mut unique: HashSet<ReactiveNodeId> = HashSet::default();
        let mut ordered = BTreeSet::new();
        for node_id in pending_nodes.iter().copied() {
            if !unique.insert(node_id) {
                continue;
            }
            let node = self.nodes.get(node_id).ok_or_else(|| {
                MechError::new(ReactiveRegisterNodeNotFoundError { node_id }, None)
            })?;
            if node.kind != ReactiveNodeKind::Register {
                return Err(MechError::new(
                    ReactiveRegisterNodeKindError {
                        node_id,
                        actual: node.kind,
                    },
                    None,
                ));
            }
            ordered.insert((node.plan_index, node.id));
        }

        let mut owners: HashMap<CanonicalCellId, ReactiveNodeId> = HashMap::default();
        for (_, node_id) in &ordered {
            let node = &self.nodes[*node_id];
            for cell in &node.outputs {
                if let Some(first_node) = owners.insert(*cell, node.id) {
                    return Err(MechError::new(
                        ReactiveRegisterOutputConflictError {
                            cell: *cell,
                            first_node,
                            second_node: node.id,
                        },
                        None,
                    ));
                }
            }
        }

        if let Some(journal) = journal.as_deref_mut() {
            for (_, node_id) in &ordered {
                self.nodes[*node_id]
                    .function
                    .capture_reactive_state(journal)?;
            }
        }

        let mut staged: Vec<(ReactiveNodeId, PreparedFunctionPublication)> = Vec::new();
        let mut services = NoMechExecutionServices;
        for (_, node_id) in &ordered {
            let node = &self.nodes[*node_id];
            let found = node.function.instance.reactive_output_cell_ids();
            if found != node.outputs {
                return Err(MechError::new(
                    ReactiveRegisterStagedOutputMismatchError {
                        node_id: node.id,
                        expected: node.outputs.clone(),
                        found,
                    },
                    None,
                ));
            }
            let prepared = node
                .function
                .instance
                .prepare_reactive_publication(&mut services)?;
            staged.push((node.id, prepared));
        }

        let staged_nodes = staged.iter().map(|(id, _)| *id).collect();
        let mut outcome = ReactiveRegisterCommitOutcome {
            staged_nodes,
            ..Default::default()
        };
        let publications = staged
            .iter_mut()
            .filter_map(|(_, prepared)| prepared.publication.take())
            .collect();
        PreparedCellPublicationBatch::new(publications)?
            .ready()?
            .commit();
        for (node_id, mut prepared) in staged {
            self.nodes[node_id]
                .function
                .instance
                .promote_prepared_realization(&mut prepared);
            outcome.committed_nodes.push(node_id);
            if prepared.status == ReactiveSolveStatus::Changed {
                for cell in &self.nodes[node_id].outputs {
                    if !outcome.dirty_cells.contains(&cell) {
                        outcome.dirty_cells.push(*cell);
                    }
                }
            }
        }
        Ok(outcome)
    }
    /// Advances one synchronous reactive turn using this existing plan.
    ///
    /// Pre-commit and staging failures occur before register mutation. Post-commit
    /// propagation failures occur after the atomic register batch has been committed
    /// and are therefore not rolled back.
    pub fn advance_reactive_turn(
        &mut self,
        state: &mut ReactiveTurnState,
        dirty_cells: &[CanonicalCellId],
    ) -> MResult<ReactiveTurnOutcome> {
        let mut services = NoMechExecutionServices;
        self.advance_reactive_turn_with_services(state, dirty_cells, &mut services)
    }

    pub fn advance_reactive_turn_with_services(
        &mut self,
        state: &mut ReactiveTurnState,
        dirty_cells: &[CanonicalCellId],
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveTurnOutcome> {
        let before_commit = self.solve_dirty_cells_with_services(dirty_cells, services)?;
        let mut pending_register_nodes = core::mem::take(&mut state.pending_register_nodes);
        pending_register_nodes.extend(before_commit.pending_register_nodes.iter().copied());
        let register_commit = match self.commit_pending_registers(&pending_register_nodes) {
            Ok(outcome) => outcome,
            Err(error) => {
                state.pending_register_nodes = pending_register_nodes;
                return Err(error);
            }
        };
        state.pending_register_nodes.clear();
        let mut after_commit = ReactivePlanSolveOutcome::default();
        if let Err(error) = self.solve_dirty_cells_into_impl(
            &register_commit.dirty_cells,
            &mut after_commit,
            None,
            services,
        ) {
            state.pending_register_nodes = after_commit.pending_register_nodes;
            return Err(error);
        }
        state.pending_register_nodes = after_commit.pending_register_nodes.clone();
        Ok(ReactiveTurnOutcome {
            before_commit,
            register_commit,
            after_commit,
        })
    }

    pub(crate) fn advance_reactive_turn_with_journal_and_services(
        &mut self,
        state: &mut ReactiveTurnState,
        dirty_cells: &[CanonicalCellId],
        journal: &mut CanonicalTurnJournal,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveTurnOutcome> {
        let before_commit =
            self.solve_dirty_cells_with_journal_and_services(dirty_cells, journal, services)?;
        let mut pending_register_nodes = core::mem::take(&mut state.pending_register_nodes);
        pending_register_nodes.extend(before_commit.pending_register_nodes.iter().copied());
        let register_commit =
            match self.commit_pending_registers_with_journal(&pending_register_nodes, journal) {
                Ok(outcome) => outcome,
                Err(error) => {
                    state.pending_register_nodes = pending_register_nodes;
                    return Err(error);
                }
            };
        state.pending_register_nodes.clear();
        let mut after_commit = ReactivePlanSolveOutcome::default();
        if let Err(error) = self.solve_dirty_cells_into_with_journal_and_services(
            &register_commit.dirty_cells,
            &mut after_commit,
            journal,
            services,
        ) {
            state.pending_register_nodes = after_commit.pending_register_nodes;
            return Err(error);
        }
        state.pending_register_nodes = after_commit.pending_register_nodes.clone();
        Ok(ReactiveTurnOutcome {
            before_commit,
            register_commit,
            after_commit,
        })
    }
}

impl core::ops::Index<usize> for ReactivePlan {
    type Output = ReactivePlanFunction;

    fn index(&self, index: usize) -> &Self::Output {
        &self.nodes[index].function
    }
}

impl core::ops::IndexMut<usize> for ReactivePlan {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.nodes[index].function
    }
}

pub struct Plan(
    pub Ref<ReactivePlan>,
    pub Ref<Vec<ActivationRegistrationScope>>,
);

impl Clone for Plan {
    fn clone(&self) -> Self {
        Plan(self.0.clone(), self.1.clone())
    }
}

impl fmt::Debug for Plan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for p in self.0.borrow().iter() {
            writeln!(f, "{}", p.to_string())?;
        }
        Ok(())
    }
}

impl Plan {
    fn checkpoint_borrow_conflict(phase: &'static str, component: &'static str) -> MechError {
        MechError::new(PlanCheckpointBorrowConflictError { phase, component }, None)
            .with_compiler_loc()
    }

    pub fn checkpoint(&self) -> PlanCheckpoint {
        PlanCheckpoint {
            reactive: self.0.borrow().checkpoint(),
            activation_registration_scopes: self.1.borrow().clone(),
        }
    }

    /// Fallibly captures a structurally valid checkpoint without panicking on
    /// outstanding plan borrows.
    pub fn try_checkpoint(&self) -> MResult<PlanCheckpoint> {
        let reactive = self
            .0
            .try_borrow()
            .map_err(|_| Self::checkpoint_borrow_conflict("capture", "reactive graph"))?;
        let scopes = self
            .1
            .try_borrow()
            .map_err(|_| Self::checkpoint_borrow_conflict("capture", "activation scopes"))?;
        reactive.validate_checkpoint_invariants(scopes.len())?;
        Ok(PlanCheckpoint {
            reactive: reactive.checkpoint(),
            activation_registration_scopes: scopes.clone(),
        })
    }

    pub fn validate_checkpoint_invariants(&self) -> MResult<()> {
        let reactive = self.0.try_borrow().map_err(|_| {
            Self::checkpoint_borrow_conflict("checkpoint-validation", "reactive graph")
        })?;
        let scopes = self.1.try_borrow().map_err(|_| {
            Self::checkpoint_borrow_conflict("checkpoint-validation", "activation scopes")
        })?;
        reactive.validate_checkpoint_invariants(scopes.len())
    }

    pub fn validate_checkpoint_turn_state(&self, state: &ReactiveTurnState) -> MResult<()> {
        let reactive = self.0.try_borrow().map_err(|_| {
            Self::checkpoint_borrow_conflict("checkpoint-validation", "reactive graph")
        })?;
        for node_id in &state.pending_register_nodes {
            let Some(node) = reactive.nodes.get(*node_id) else {
                return Err(MechError::new(
                    ReactiveTurnCheckpointInvariantError {
                        node_id: *node_id,
                        reason: "the node does not exist".into(),
                    },
                    None,
                )
                .with_compiler_loc());
            };
            if node.kind != ReactiveNodeKind::Register {
                return Err(MechError::new(
                    ReactiveTurnCheckpointInvariantError {
                        node_id: *node_id,
                        reason: "the node is not a register".into(),
                    },
                    None,
                )
                .with_compiler_loc());
            }
        }
        Ok(())
    }

    pub fn preflight_rollback(&self, checkpoint: &PlanCheckpoint) -> MResult<()> {
        let reactive = self
            .0
            .try_borrow_mut()
            .map_err(|_| Self::checkpoint_borrow_conflict("restore", "reactive graph"))?;
        let _scopes = self
            .1
            .try_borrow_mut()
            .map_err(|_| Self::checkpoint_borrow_conflict("restore", "activation scopes"))?;
        reactive.preflight_rollback(&checkpoint.reactive)
    }

    pub fn apply_rollback_structure(&self, checkpoint: &PlanCheckpoint) {
        let mut reactive = self.0.borrow_mut();
        let mut scopes = self.1.borrow_mut();
        reactive.apply_rollback_structure(&checkpoint.reactive);
        *scopes = checkpoint.activation_registration_scopes.clone();
    }

    pub fn rebuild_checkpoint_indexes(&self) {
        self.0.borrow_mut().rebuild_checkpoint_indexes();
    }

    pub fn apply_rollback(&self, checkpoint: &PlanCheckpoint) {
        self.apply_rollback_structure(checkpoint);
        self.rebuild_checkpoint_indexes();
    }

    pub fn rollback(&self, checkpoint: PlanCheckpoint) -> MResult<()> {
        self.preflight_rollback(&checkpoint)?;
        self.apply_rollback(&checkpoint);
        Ok(())
    }

    pub fn capture_transaction_state(&self, journal: &mut FunctionCheckpoint) -> MResult<()> {
        self.0.borrow().capture_transaction_state(journal)
    }

    pub fn activation_registration_depth(&self) -> usize {
        self.1.borrow().len()
    }

    pub fn new() -> Self {
        Self(Ref::new(ReactivePlan::new()), Ref::new(Vec::new()))
    }

    pub fn borrow(&self) -> core::cell::Ref<'_, ReactivePlan> {
        self.0.borrow()
    }

    pub fn borrow_mut(&self) -> core::cell::RefMut<'_, ReactivePlan> {
        self.0.borrow_mut()
    }

    pub fn add_function(&self, instance: FunctionInstance) -> MResult<ReactiveNodeId> {
        self.register_instance(instance)
    }

    pub fn activation_registration_active(&self) -> bool {
        !self.1.borrow().is_empty()
    }
    pub fn push_activation_registration_scope(&self, trigger_cells: Vec<CanonicalCellId>) {
        self.push_activation_registration_scope_with_sampled_cells(trigger_cells, Vec::new());
    }
    pub fn push_activation_registration_scope_with_sampled_cells(
        &self,
        trigger_cells: Vec<CanonicalCellId>,
        sampled_cells: Vec<CanonicalCellId>,
    ) {
        self.1.borrow_mut().push(ActivationRegistrationScope {
            trigger_cells,
            local_combinational_cells: Vec::new(),
        });
        self.0
            .borrow_mut()
            .activation_sampled_cells
            .push(sampled_cells);
    }
    pub fn pop_activation_registration_scope(&self) {
        self.1.borrow_mut().pop();
        self.0.borrow_mut().activation_sampled_cells.pop();
    }
    pub fn register_instance(&self, instance: FunctionInstance) -> MResult<ReactiveNodeId> {
        let scope = self.1.borrow().last().cloned();
        let kind = instance.implementation().reactive_node_kind();
        let outputs = instance.reactive_output_cell_ids();
        let sampled_cells = self
            .0
            .borrow()
            .activation_sampled_cells
            .last()
            .cloned()
            .unwrap_or_default();
        let node = self
            .0
            .borrow_mut()
            .register_instance_with_activation(instance, scope.as_ref())?;
        if scope.is_some() && kind == ReactiveNodeKind::Combinational {
            if let Some(active) = self.1.borrow_mut().last_mut() {
                for cell in outputs {
                    if !sampled_cells.contains(&cell)
                        && !active.local_combinational_cells.contains(&cell)
                    {
                        active.local_combinational_cells.push(cell);
                    }
                }
            }
        }
        Ok(node)
    }

    pub fn register_specialized(
        &self,
        specialized: SpecializedFunction,
    ) -> MResult<ReactiveNodeId> {
        let scope = self.1.borrow().last().cloned();
        let kind = specialized.instance().implementation().reactive_node_kind();
        let outputs = specialized.instance().reactive_output_cell_ids();
        let sampled_cells = self
            .0
            .borrow()
            .activation_sampled_cells
            .last()
            .cloned()
            .unwrap_or_default();
        let node = self
            .0
            .borrow_mut()
            .register_specialized_with_activation(specialized, scope.as_ref())?;
        if scope.is_some() && kind == ReactiveNodeKind::Combinational {
            if let Some(active) = self.1.borrow_mut().last_mut() {
                for cell in outputs {
                    if !sampled_cells.contains(&cell)
                        && !active.local_combinational_cells.contains(&cell)
                    {
                        active.local_combinational_cells.push(cell);
                    }
                }
            }
        }
        Ok(node)
    }

    pub fn solve_dirty_cells(
        &self,
        dirty_cells: &[CanonicalCellId],
    ) -> MResult<ReactivePlanSolveOutcome> {
        self.0.borrow_mut().solve_dirty_cells(dirty_cells)
    }
    pub fn solve_dirty_cells_with_services(
        &self,
        dirty_cells: &[CanonicalCellId],
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactivePlanSolveOutcome> {
        self.0
            .borrow_mut()
            .solve_dirty_cells_with_services(dirty_cells, services)
    }

    pub fn commit_pending_registers(
        &self,
        pending_nodes: &[ReactiveNodeId],
    ) -> MResult<ReactiveRegisterCommitOutcome> {
        self.0.borrow_mut().commit_pending_registers(pending_nodes)
    }
    pub fn advance_reactive_turn(
        &self,
        state: &mut ReactiveTurnState,
        dirty_cells: &[CanonicalCellId],
    ) -> MResult<ReactiveTurnOutcome> {
        self.0
            .borrow_mut()
            .advance_reactive_turn(state, dirty_cells)
    }
    pub fn advance_reactive_turn_with_services(
        &self,
        state: &mut ReactiveTurnState,
        dirty_cells: &[CanonicalCellId],
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveTurnOutcome> {
        self.0
            .borrow_mut()
            .advance_reactive_turn_with_services(state, dirty_cells, services)
    }
    pub fn advance_reactive_turn_participating(
        &self,
        state: &mut ReactiveTurnState,
        dirty_cells: &[CanonicalCellId],
        participant: &mut ReactiveJournalParticipant<'_>,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveTurnOutcome> {
        self.0
            .borrow_mut()
            .advance_reactive_turn_with_journal_and_services(
                state,
                dirty_cells,
                participant.journal_mut(),
                services,
            )
    }

    pub fn get_functions(&self) -> core::cell::Ref<'_, ReactivePlan> {
        self.0.borrow()
    }

    pub fn pattern_activation_registrations(
        &self,
    ) -> core::cell::Ref<'_, Vec<PatternActivationRegistration>> {
        core::cell::Ref::map(self.0.borrow(), |plan| {
            &plan.pattern_activation_registrations
        })
    }

    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }
}

#[cfg(feature = "pretty_print")]
impl PrettyPrint for Plan {
    fn pretty_print(&self) -> String {
        let mut builder = Builder::default();
        let plan_brrw = self.0.borrow();

        if self.is_empty() {
            builder.push_record(vec!["".to_string()]);
        } else {
            let total = plan_brrw.len();
            let mut display_fxns: Vec<String> = Vec::new();

            let indices: Vec<usize> = if total > 30 {
                (0..10).chain((total - 10)..total).collect()
            } else {
                (0..total).collect()
            };

            for &ix in &indices {
                let fxn_str = plan_brrw[ix].to_string();
                let lines: Vec<&str> = fxn_str.lines().collect();

                let truncated = if lines.len() > 20 {
                    let mut t = Vec::new();
                    t.extend_from_slice(&lines[..10]);
                    t.push("…");
                    t.extend_from_slice(&lines[lines.len() - 10..]);
                    t.join("\n")
                } else {
                    lines.join("\n")
                };

                display_fxns.push(format!("{}. {}", ix + 1, truncated));
            }

            if total > 30 {
                display_fxns.insert(10, "…".to_string());
            }

            let mut row: Vec<String> = Vec::new();
            for plan_str in display_fxns {
                row.push(plan_str);
                if row.len() == 4 {
                    builder.push_record(row.clone());
                    row.clear();
                }
            }
            if !row.is_empty() {
                while row.len() < 4 {
                    row.push("".to_string());
                }
                builder.push_record(row);
            }
        }

        let mut table = builder.build();
        table
            .with(Style::modern_rounded())
            .with(Panel::header("📋 Plan"));

        format!("{table}")
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;

#[derive(Debug, Clone)]
pub struct IncorrectNumberOfArguments {
    pub expected: usize,
    pub found: usize,
}
impl MechErrorKind for IncorrectNumberOfArguments {
    fn name(&self) -> &str {
        "IncorrectNumberOfArguments"
    }

    fn message(&self) -> String {
        format!(
            "Expected {} arguments, but found {}",
            self.expected, self.found
        )
    }
}
