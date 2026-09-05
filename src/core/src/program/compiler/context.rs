//! Semantic source-plan recorder shared by artifact compilation and bytecode v1 production.

use core::{cmp::Ordering, ops::Range};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::LazyLock;

use crate::{
    AccessMode, AliasPolicy, ApplicationRequirement, BytecodeCompilerContext, BytecodeInstruction,
    BytecodeProgram, BytecodeRegisterIdentity, BytecodeValidationError, ChangeDetectionPolicy,
    CompiledMatrixLiteral, ComputePlacement, DeliveryMode, EncodedConstant, ExternalInteraction,
    InputPortLayout, InputPortPolicy, MResult, MechError, OperationContractDeclaration,
    OutputConstruction, OutputPortPolicy, ParsedProgram, Register, ShapeRule, ValueCell,
    compare_application_requirements, hash_str, write_bytecode,
};

static PURE_COMPOSITE_PACK_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Variadic {
            prefix: vec![InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
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
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::AlwaysChanged,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompiledNodeKind {
    Combinational,
    Register,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompiledInstructionRole {
    Node(CompiledNodeKind),
    /// The bytecode-v1 marker instruction used to retain invariant metadata.
    /// It must not become a `ProgramArtifact` node.
    IntegrityMarker,
    /// Executable bytecode-v1 variable-definition instruction whose semantic
    /// declaration is already represented by symbol and slot metadata.
    DeclarationMarker,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledSymbolDefinition {
    pub id: u64,
    pub name: String,
    pub register: Register,
    pub mutable: bool,
    /// Whether the declaration belongs to the root namespace exposed through
    /// bytecode symbol lookup. Function-local declarations remain available
    /// for state classification without becoming public symbols.
    pub root_visible: bool,
    /// Source/compiler definition order, assigned densely.
    pub ordinal: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledIntegrityConstraint {
    pub name: String,
    pub result_register: Register,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledComputeRegion {
    pub name: String,
    pub placement: ComputePlacement,
    /// Dense source-plan node identities covered by this named section.
    pub source_nodes: Range<u32>,
}

#[derive(Clone, Debug)]
pub struct CompiledBytecode {
    pub program: BytecodeProgram,
    /// Exact canonical names for runtime instructions emitted by source
    /// extensions that are not members of the immutable static catalog.
    /// This sidecar is consumed while constructing the semantic artifact;
    /// the artifact itself carries the portable operation reference.
    pub runtime_function_names: BTreeMap<u64, String>,
    /// Parallel to `program.instructions`.
    pub instruction_roles: Vec<Option<CompiledInstructionRole>>,
    /// Portable semantic declaration captured from each specialized source
    /// node, parallel to `program.instructions`.
    pub instruction_contracts: Vec<Option<OperationContractDeclaration>>,
    /// Canonical source-level operation, parallel to `program.instructions`.
    /// Runtime factory identities remain in executable bytecode only.
    pub instruction_operations: Vec<Option<String>>,
    /// Dense source-plan node identity, parallel to `program.instructions`.
    pub instruction_source_nodes: Vec<Option<u32>>,
    /// Immutable semantic call certificate for each executable instruction,
    /// parallel to `program.instructions`.
    pub instruction_type_bindings: Vec<Option<crate::BoundCall>>,
    /// Canonical schema authority for registers owned by canonical cells.
    /// Dense and parallel to the register space.
    pub register_schemas: Vec<Option<crate::SchemaBody>>,
    /// Complete semantic descriptor for every canonical register, dense and
    /// parallel to the register space.
    pub register_type_descriptors: Vec<Option<crate::ResolvedValueDescriptor>>,
    /// Compiler-control absence registers, kept distinct from canonical unit.
    pub absent_registers: BTreeSet<Register>,
    /// Exact current cardinality for map/set registers. Dense and parallel to
    /// the register space; other register families carry `None`.
    pub register_collection_cardinalities: Vec<Option<usize>>,
    /// Source-declaration initializer constant, dense and parallel to the
    /// register space. This is compilation sidecar metadata, not an
    /// executable instruction.
    pub register_state_initializers: Vec<Option<u32>>,
    /// Generic matrix constructions keyed by their destination register.
    /// This semantic sidecar is intentionally not serialized in bytecode v1.
    pub matrix_literals: BTreeMap<Register, CompiledMatrixLiteral>,
    /// Exact first-definition order, unlike the canonically sorted symbol map.
    pub symbol_definitions: Vec<CompiledSymbolDefinition>,
    pub return_register: Register,
    pub integrity_constraints: Vec<CompiledIntegrityConstraint>,
    pub compute_regions: Vec<CompiledComputeRegion>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CanonicalRequirement(ApplicationRequirement);

impl Ord for CanonicalRequirement {
    fn cmp(&self, other: &Self) -> Ordering {
        compare_application_requirements(&self.0, &other.0)
    }
}

impl PartialOrd for CanonicalRequirement {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug)]
pub struct CompileCtx {
    reg_map: HashMap<BytecodeRegisterIdentity, Register>,
    symbols: BTreeMap<u64, Register>,
    symbol_ptrs: BTreeMap<u64, usize>,
    retained_symbol_cells: BTreeMap<String, ValueCell>,
    retained_value_cells: BTreeMap<usize, ValueCell>,
    dictionary: BTreeMap<u64, String>,
    runtime_function_names: BTreeMap<u64, String>,
    mutable_symbols: BTreeSet<u64>,
    pending_constants: Vec<EncodedConstant>,
    requirements: BTreeSet<CanonicalRequirement>,
    pending_requirements: Vec<ApplicationRequirement>,
    instructions: Vec<BytecodeInstruction>,
    instruction_roles: Vec<Option<CompiledInstructionRole>>,
    instruction_contracts: Vec<Option<OperationContractDeclaration>>,
    instruction_operations: Vec<Option<String>>,
    instruction_source_nodes: Vec<Option<u32>>,
    instruction_type_bindings: Vec<Option<crate::BoundCall>>,
    register_schemas: BTreeMap<Register, crate::SchemaBody>,
    register_schema_conflicts: BTreeSet<Register>,
    register_type_descriptors: BTreeMap<Register, crate::ResolvedValueDescriptor>,
    absent_registers: BTreeSet<Register>,
    register_collection_cardinalities: BTreeMap<Register, usize>,
    register_state_initializers: BTreeMap<Register, u32>,
    matrix_literals: BTreeMap<Register, CompiledMatrixLiteral>,
    runtime_produced_registers: BTreeSet<Register>,
    symbol_definitions: Vec<CompiledSymbolDefinition>,
    current_node_kind: Option<CompiledNodeKind>,
    current_node_contract: Option<OperationContractDeclaration>,
    current_node_operation: Option<String>,
    current_source_node: Option<u32>,
    current_node_type_binding: Option<crate::BoundCall>,
    next_source_node: u32,
    integrity_constraints: Vec<CompiledIntegrityConstraint>,
    compute_regions: Vec<CompiledComputeRegion>,
    next_register: Register,
}

impl Default for CompileCtx {
    fn default() -> Self {
        Self {
            reg_map: HashMap::new(),
            symbols: BTreeMap::new(),
            symbol_ptrs: BTreeMap::new(),
            retained_symbol_cells: BTreeMap::new(),
            retained_value_cells: BTreeMap::new(),
            dictionary: BTreeMap::new(),
            runtime_function_names: BTreeMap::new(),
            mutable_symbols: BTreeSet::new(),
            pending_constants: Vec::new(),
            requirements: BTreeSet::new(),
            pending_requirements: Vec::new(),
            instructions: Vec::new(),
            instruction_roles: Vec::new(),
            instruction_contracts: Vec::new(),
            instruction_operations: Vec::new(),
            instruction_source_nodes: Vec::new(),
            instruction_type_bindings: Vec::new(),
            register_schemas: BTreeMap::new(),
            register_schema_conflicts: BTreeSet::new(),
            register_type_descriptors: BTreeMap::new(),
            absent_registers: BTreeSet::new(),
            register_collection_cardinalities: BTreeMap::new(),
            register_state_initializers: BTreeMap::new(),
            matrix_literals: BTreeMap::new(),
            runtime_produced_registers: BTreeSet::new(),
            symbol_definitions: Vec::new(),
            current_node_kind: None,
            current_node_contract: None,
            current_node_operation: None,
            current_source_node: None,
            current_node_type_binding: None,
            next_source_node: 0,
            integrity_constraints: Vec::new(),
            compute_regions: Vec::new(),
            next_register: 0,
        }
    }
}

impl CompileCtx {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Retains the explicit outer cell that owns a source symbol until its
    /// declaration step supplies the canonical producer register.
    pub fn retain_compiler_symbol_cell(&mut self, name: &str, cell: &ValueCell) -> MResult<()> {
        if let Some(existing) = self.retained_symbol_cells.get(name) {
            if !existing.same_cell(cell) {
                return invalid(format!(
                    "compiler symbol {name:?} has conflicting retained value cells",
                ));
            }
            return Ok(());
        }
        self.retained_symbol_cells
            .insert(name.to_owned(), cell.clone());
        self.retain_compiler_value_cell(cell)?;
        Ok(())
    }

    /// Retains a canonical compiler-owned cell. The cell's opaque storage
    /// identity is already the complete bytecode register identity; no
    /// payload inspection or compatibility projection is required.
    pub fn retain_compiler_value_cell(&mut self, cell: &ValueCell) -> MResult<()> {
        let identity = cell.compiler_identity();
        if let Some(existing) = self.retained_value_cells.get(&identity) {
            if !existing.same_cell(cell) {
                return invalid(
                    "canonical compiler cell identity was recycled during one compilation",
                );
            }
            return Ok(());
        }
        self.retained_value_cells.insert(identity, cell.clone());
        Ok(())
    }

    /// Retained symbols are associated directly when their canonical
    /// declaration is defined. This hook is a no-op for callers that finish
    /// association eagerly.
    pub fn associate_retained_symbol_cells_with_existing_value_registers(&mut self) -> MResult<()> {
        Ok(())
    }

    fn remove_instruction(&mut self, index: usize) {
        self.instructions.remove(index);
        self.instruction_roles.remove(index);
        self.instruction_contracts.remove(index);
        self.instruction_operations.remove(index);
        self.instruction_source_nodes.remove(index);
        self.instruction_type_bindings.remove(index);
    }

    fn remove_constant_if_unreferenced(&mut self, constant: u32) {
        let instruction_references =
            self.instructions
                .iter()
                .any(|instruction| match instruction {
                    BytecodeInstruction::ConstLoad {
                        constant: referenced,
                        ..
                    }
                    | BytecodeInstruction::CompositePack {
                        template: referenced,
                        ..
                    } => *referenced == constant,
                    _ => false,
                });
        let state_references = self
            .register_state_initializers
            .values()
            .any(|referenced| *referenced == constant);
        if instruction_references || state_references {
            return;
        }

        self.pending_constants.remove(constant as usize);
        for instruction in &mut self.instructions {
            let referenced = match instruction {
                BytecodeInstruction::ConstLoad { constant, .. } => Some(constant),
                BytecodeInstruction::CompositePack { template, .. } => Some(template),
                _ => None,
            };
            if let Some(referenced) = referenced
                && *referenced > constant
            {
                *referenced -= 1;
            }
        }
        for initializer in self.register_state_initializers.values_mut() {
            if *initializer > constant {
                *initializer -= 1;
            }
        }
    }

    pub fn begin_plan_node(&mut self, kind: CompiledNodeKind) -> MResult<()> {
        if self.current_node_kind.is_some() {
            return invalid("cannot begin a bytecode plan node while another node is active");
        }
        self.current_node_kind = Some(kind);
        self.current_node_operation = None;
        self.current_node_contract = None;
        self.current_node_type_binding = None;
        self.current_source_node = Some(self.next_source_node);
        self.next_source_node = self
            .next_source_node
            .checked_add(1)
            .ok_or_else(|| invalid::<()>("source plan node identity exceeds u32").unwrap_err())?;
        Ok(())
    }

    pub fn begin_plan_node_with_type_binding(
        &mut self,
        kind: CompiledNodeKind,
        type_binding: &crate::BoundCall,
    ) -> MResult<()> {
        if self.current_node_kind.is_some() {
            return invalid("cannot begin a bytecode plan node while another node is active");
        }
        type_binding.operation_descriptor().validate()?;
        self.current_node_kind = Some(kind);
        self.current_node_operation = Some(
            type_binding
                .operation_descriptor()
                .canonical_name
                .to_string(),
        );
        self.current_node_contract = Some(type_binding.operation_descriptor().contract.clone());
        self.current_node_type_binding = Some(type_binding.clone());
        self.current_source_node = Some(self.next_source_node);
        self.next_source_node = self
            .next_source_node
            .checked_add(1)
            .ok_or_else(|| invalid::<()>("source plan node identity exceeds u32").unwrap_err())?;
        Ok(())
    }

    pub fn end_plan_node(&mut self) {
        self.current_node_kind = None;
        self.current_node_operation = None;
        self.current_node_contract = None;
        self.current_source_node = None;
        self.current_node_type_binding = None;
    }

    pub fn record_integrity_constraint(
        &mut self,
        name: String,
        result_register: Register,
    ) -> MResult<()> {
        if result_register >= self.next_register {
            return invalid(format!(
                "integrity result register {result_register} is outside register count {}",
                self.next_register,
            ));
        }
        self.integrity_constraints
            .push(CompiledIntegrityConstraint {
                name,
                result_register,
            });
        Ok(())
    }

    pub fn record_compute_region(
        &mut self,
        name: String,
        placement: ComputePlacement,
        source_nodes: Range<u32>,
    ) -> MResult<()> {
        if name.is_empty() {
            return invalid("compute region name cannot be empty");
        }
        if source_nodes.is_empty() {
            return invalid(format!("compute region `{name}` cannot be empty"));
        }
        if source_nodes.end > self.next_source_node {
            return invalid(format!(
                "compute region `{name}` ends at source node {}, but only {} source nodes exist",
                source_nodes.end, self.next_source_node,
            ));
        }
        if self
            .compute_regions
            .iter()
            .any(|region| region.name == name)
        {
            return invalid(format!("compute region `{name}` is defined more than once"));
        }
        if let Some(region) = self.compute_regions.iter().find(|region| {
            source_nodes.start < region.source_nodes.end
                && region.source_nodes.start < source_nodes.end
        }) {
            return invalid(format!(
                "compute region `{name}` overlaps compute region `{}`",
                region.name,
            ));
        }
        self.compute_regions.push(CompiledComputeRegion {
            name,
            placement,
            source_nodes,
        });
        Ok(())
    }

    pub fn emit_integrity_marker(
        &mut self,
        function: u64,
        destination: Register,
        arguments: Vec<Register>,
    ) {
        self.instructions
            .push(BytecodeInstruction::RuntimeVariadic {
                function,
                dst: destination,
                arguments,
            });
        self.instruction_roles
            .push(Some(CompiledInstructionRole::IntegrityMarker));
        self.instruction_contracts.push(None);
        self.instruction_operations.push(None);
        self.instruction_source_nodes.push(None);
        self.instruction_type_bindings.push(None);
    }

    pub fn finish_program(&mut self, return_register: Register) -> MResult<CompiledBytecode> {
        if return_register >= self.next_register {
            return invalid(format!(
                "return register {return_register} is outside register count {}",
                self.next_register,
            ));
        }

        if self.instruction_roles.len() != self.instructions.len() {
            return invalid(format!(
                "instruction role count {} does not match instruction count {}",
                self.instruction_roles.len(),
                self.instructions.len(),
            ));
        }
        if self.instruction_contracts.len() != self.instructions.len() {
            return invalid(format!(
                "instruction contract count {} does not match instruction count {}",
                self.instruction_contracts.len(),
                self.instructions.len(),
            ));
        }
        if self.instruction_operations.len() != self.instructions.len() {
            return invalid(format!(
                "instruction operation count {} does not match instruction count {}",
                self.instruction_operations.len(),
                self.instructions.len(),
            ));
        }
        if self.instruction_source_nodes.len() != self.instructions.len() {
            return invalid(format!(
                "instruction source node count {} does not match instruction count {}",
                self.instruction_source_nodes.len(),
                self.instructions.len(),
            ));
        }
        if self.instruction_type_bindings.len() != self.instructions.len() {
            return invalid(format!(
                "instruction type binding count {} does not match instruction count {}",
                self.instruction_type_bindings.len(),
                self.instructions.len(),
            ));
        }

        let requirements = self
            .requirements
            .iter()
            .map(|requirement| requirement.0.clone())
            .collect::<Vec<_>>();
        let requirement_remap = self
            .pending_requirements
            .iter()
            .map(|requirement| {
                requirements
                    .binary_search_by(|candidate| {
                        compare_application_requirements(candidate, requirement)
                    })
                    .map(|index| index as u32)
                    .map_err(|_| {
                        invalid::<()>("pending application requirement was not finalized")
                            .unwrap_err()
                    })
            })
            .collect::<MResult<Vec<_>>>()?;

        let mut instructions = self.instructions.clone();
        for instruction in &mut instructions {
            remap_instruction_requirement(instruction, &requirement_remap)?;
        }
        let (constants, constant_remap) =
            canonicalize_instruction_constants(&mut instructions, &self.pending_constants)?;
        instructions.push(BytecodeInstruction::Return {
            src: return_register,
        });
        let mut instruction_roles = self.instruction_roles.clone();
        instruction_roles.push(None);
        let mut instruction_contracts = self.instruction_contracts.clone();
        instruction_contracts.push(None);
        let mut instruction_operations = self.instruction_operations.clone();
        instruction_operations.push(None);
        let mut instruction_source_nodes = self.instruction_source_nodes.clone();
        instruction_source_nodes.push(None);
        let mut instruction_type_bindings = self.instruction_type_bindings.clone();
        instruction_type_bindings.push(None);

        let mut register_schemas = vec![None; self.next_register as usize];
        for (register, schema) in &self.register_schemas {
            let target = register_schemas
                .get_mut(*register as usize)
                .ok_or_else(|| {
                    invalid::<()>(format!(
                        "recorded register schema {register} is outside register count {}",
                        self.next_register,
                    ))
                    .unwrap_err()
                })?;
            *target = Some(schema.clone());
        }
        let mut register_type_descriptors = vec![None; self.next_register as usize];
        for (register, descriptor) in &self.register_type_descriptors {
            let target = register_type_descriptors
                .get_mut(*register as usize)
                .ok_or_else(|| {
                    invalid::<()>(format!(
                        "recorded register type descriptor {register} is outside register count {}",
                        self.next_register,
                    ))
                    .unwrap_err()
                })?;
            *target = Some(descriptor.clone());
        }
        complete_register_type_descriptors(
            &instructions,
            &instruction_type_bindings,
            &self.absent_registers,
            &mut register_type_descriptors,
        )?;
        for register in &self.register_schema_conflicts {
            if register_type_descriptors
                .get(*register as usize)
                .is_none_or(Option::is_none)
            {
                return invalid(format!(
                    "register {register} has conflicting physical schemas without a bound semantic type descriptor",
                ));
            }
        }
        // The resolved descriptor is the R4 schema authority. Compiler
        // constants can still contribute a closed physical schema body while
        // their source call carries a dynamic semantic dimension; once the
        // immutable BoundCall completes the descriptor sidecar, keep the
        // compatibility body aligned with that certificate.
        for (schema, descriptor) in register_schemas.iter_mut().zip(&register_type_descriptors) {
            if let Some(descriptor) = descriptor {
                *schema = Some(descriptor.schema().body().clone());
            }
        }
        let mut register_collection_cardinalities = vec![None; self.next_register as usize];
        for (register, cardinality) in &self.register_collection_cardinalities {
            let target = register_collection_cardinalities
                .get_mut(*register as usize)
                .ok_or_else(|| {
                    invalid::<()>(format!(
                        "recorded collection cardinality {register} is outside register count {}",
                        self.next_register,
                    ))
                    .unwrap_err()
                })?;
            *target = Some(*cardinality);
        }
        let mut register_state_initializers = vec![None; self.next_register as usize];
        for (register, constant) in &self.register_state_initializers {
            let target = register_state_initializers
                .get_mut(*register as usize)
                .ok_or_else(|| {
                    invalid::<()>(format!(
                        "recorded state initializer register {register} is outside register count {}",
                        self.next_register,
                    ))
                    .unwrap_err()
                })?;
            *target = Some(
                constant_remap
                    .get(*constant as usize)
                    .copied()
                    .flatten()
                    .ok_or_else(|| {
                        invalid::<()>("state initializer constant is not referenced by bytecode")
                            .unwrap_err()
                    })?,
            );
        }

        Ok(CompiledBytecode {
            program: BytecodeProgram {
                register_count: self.next_register,
                constants,
                symbols: self.symbols.clone(),
                mutable_symbols: self.mutable_symbols.clone(),
                instructions,
                dictionary: self.dictionary.clone(),
                requirements,
            },
            runtime_function_names: self.runtime_function_names.clone(),
            instruction_roles,
            instruction_contracts,
            instruction_operations,
            instruction_source_nodes,
            instruction_type_bindings,
            register_schemas,
            register_type_descriptors,
            absent_registers: self.absent_registers.clone(),
            register_collection_cardinalities,
            register_state_initializers,
            matrix_literals: self.matrix_literals.clone(),
            symbol_definitions: self.symbol_definitions.clone(),
            return_register,
            integrity_constraints: self.integrity_constraints.clone(),
            compute_regions: self.compute_regions.clone(),
        })
    }

    pub fn finish(&mut self, return_register: Register) -> MResult<Vec<u8>> {
        let compiled = self.finish_program(return_register)?;
        let bytes = write_bytecode(&compiled.program)?;
        ParsedProgram::from_bytes(&bytes)?;
        Ok(bytes)
    }

    fn define_symbol_with_visibility(
        &mut self,
        pointer: usize,
        register: Register,
        name: &str,
        mutable: bool,
        root_visible: bool,
    ) -> MResult<()> {
        if name.is_empty() {
            return invalid("bytecode symbol name must not be empty");
        }
        if register >= self.next_register {
            return invalid(format!(
                "symbol register {register} is outside register count {}",
                self.next_register,
            ));
        }

        let symbol_id = hash_str(name);
        if root_visible {
            if let Some(existing_name) = self.dictionary.get(&symbol_id) {
                if existing_name != name {
                    return invalid(format!(
                        "bytecode symbol hash collision between {existing_name:?} and {name:?}",
                    ));
                }
                if self.symbols.get(&symbol_id) != Some(&register)
                    || self.symbol_ptrs.get(&symbol_id) != Some(&pointer)
                    || self.mutable_symbols.contains(&symbol_id) != mutable
                {
                    return invalid(format!(
                        "conflicting bytecode symbol definition for {name:?}",
                    ));
                }
                self.associate_retained_symbol_cell(name, register)?;
                return Ok(());
            }

            self.symbols.insert(symbol_id, register);
            self.symbol_ptrs.insert(symbol_id, pointer);
            self.dictionary.insert(symbol_id, name.to_owned());
            if mutable {
                self.mutable_symbols.insert(symbol_id);
            }
            self.associate_retained_symbol_cell(name, register)?;
        }

        let ordinal = u32::try_from(self.symbol_definitions.len())
            .map_err(|_| invalid::<()>("symbol definition ordinal exceeds u32").unwrap_err())?;
        self.symbol_definitions.push(CompiledSymbolDefinition {
            id: symbol_id,
            name: name.to_owned(),
            register,
            mutable,
            root_visible,
            ordinal,
        });
        Ok(())
    }

    fn associate_retained_symbol_cell(&mut self, name: &str, register: Register) -> MResult<()> {
        let Some(cell) = self.retained_symbol_cells.get(name).cloned() else {
            return Ok(());
        };
        let address = cell.compiler_identity();
        let identity = BytecodeRegisterIdentity::Typed {
            inner: Box::new(BytecodeRegisterIdentity::Cell(address)),
            annotation: cell.schema_key(),
        };
        if let Some(existing) = self.reg_map.get(&identity) {
            if *existing != register {
                return invalid(format!(
                    "compiler symbol {name:?} retained cell already owns register {existing}, incoming register {register}",
                ));
            }
        } else {
            self.reg_map.insert(identity, register);
        }
        let descriptor = cell.resolved_descriptor()?;
        if let Some(existing) = self.register_type_descriptors.get(&register) {
            if existing != &descriptor {
                return invalid(format!(
                    "compiler symbol {name:?} disagrees with register {register}'s semantic descriptor",
                ));
            }
        } else {
            self.register_type_descriptors
                .insert(register, descriptor.clone());
        }
        self.register_schemas
            .insert(register, descriptor.schema().body().clone());
        Ok(())
    }

    fn register_for_identity(&mut self, identity: BytecodeRegisterIdentity) -> (Register, bool) {
        if let Some(&register) = self.reg_map.get(&identity) {
            return (register, false);
        }
        if let BytecodeRegisterIdentity::Cell(address) = &identity {
            let typed = self
                .reg_map
                .iter()
                .filter_map(|(candidate, register)| match candidate {
                    BytecodeRegisterIdentity::Typed { inner, annotation }
                        if matches!(inner.as_ref(), BytecodeRegisterIdentity::Cell(candidate) if candidate == address) =>
                    {
                        Some((*annotation, *register))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            let bound_matches = typed
                .iter()
                .filter_map(|(annotation, register)| {
                    self.current_node_type_binding
                        .as_ref()
                        .is_some_and(|binding| {
                            binding
                                .inputs()
                                .iter()
                                .chain(binding.outputs())
                                .any(|descriptor| descriptor.schema().key() == *annotation)
                        })
                        .then_some(*register)
                })
                .collect::<BTreeSet<_>>();
            let resolved = if bound_matches.len() == 1 {
                bound_matches.first().copied()
            } else {
                let registers = typed
                    .into_iter()
                    .map(|(_, register)| register)
                    .collect::<BTreeSet<_>>();
                (registers.len() == 1)
                    .then(|| *registers.first().expect("one typed register exists"))
            };
            if let Some(register) = resolved {
                self.reg_map.insert(identity, register);
                return (register, false);
            }
        }
        if let BytecodeRegisterIdentity::Typed { inner, .. } = &identity
            && let BytecodeRegisterIdentity::Cell(address) = inner.as_ref()
            && !self.reg_map.keys().any(|candidate| {
                matches!(
                    candidate,
                    BytecodeRegisterIdentity::Typed { inner, .. }
                        if matches!(inner.as_ref(), BytecodeRegisterIdentity::Cell(candidate) if candidate == address)
                )
            })
            && let Some(register) = self
                .reg_map
                .get(&BytecodeRegisterIdentity::Cell(*address))
                .copied()
        {
            self.reg_map.insert(identity, register);
            return (register, false);
        }
        let register = self.next_register;
        self.next_register = self
            .next_register
            .checked_add(1)
            .expect("bytecode register space exhausted");
        self.reg_map.insert(identity, register);
        (register, true)
    }
}

impl BytecodeCompilerContext for CompileCtx {
    fn function_id(&mut self, canonical_name: &str) -> MResult<u64> {
        if canonical_name.is_empty() {
            return invalid("runtime function name must not be empty");
        }
        let id = hash_str(canonical_name);
        if let Some(existing) = self.runtime_function_names.get(&id)
            && existing != canonical_name
        {
            return invalid(format!(
                "runtime function hash collision between {existing:?} and {canonical_name:?}",
            ));
        }
        self.runtime_function_names
            .insert(id, canonical_name.to_owned());
        Ok(id)
    }

    fn register_for_ptr_with_initialization_status(&mut self, pointer: usize) -> (Register, bool) {
        self.register_for_identity(BytecodeRegisterIdentity::Cell(pointer))
    }

    fn retain_canonical_cell(&mut self, cell: &ValueCell) -> MResult<()> {
        self.retain_compiler_value_cell(cell)
    }

    fn register_for_identity_with_initialization_status(
        &mut self,
        identity: &BytecodeRegisterIdentity,
    ) -> (Register, bool) {
        self.register_for_identity(identity.clone())
    }

    fn record_register_schema(
        &mut self,
        register: Register,
        schema: crate::SchemaBody,
    ) -> MResult<()> {
        if let Some(descriptor) = self.register_type_descriptors.get(&register) {
            // Physical constant encoders still report a closed storage body.
            // Once a semantic descriptor exists, that compatibility metadata
            // cannot replace or contradict the R4 type authority.
            if descriptor.schema().body() != &schema {
                return Ok(());
            }
            self.register_schemas.insert(register, schema);
            return Ok(());
        }
        if let Some(existing) = self.register_schemas.get(&register) {
            if existing != &schema {
                // Reservation runs before source nodes are emitted, so a
                // mutable backing can expose both its closed initializer and
                // parameterized live schema before its BoundCall is visible.
                // Defer this physical disagreement until finalization, where
                // the immutable semantic descriptor must resolve it.
                self.register_schema_conflicts.insert(register);
                return Ok(());
            }
        } else {
            self.register_schemas.insert(register, schema);
        }
        Ok(())
    }

    fn record_register_type_descriptor(
        &mut self,
        register: Register,
        descriptor: crate::ResolvedValueDescriptor,
    ) -> MResult<()> {
        if let Some(existing) = self.register_type_descriptors.get(&register) {
            if existing != &descriptor {
                return invalid(format!(
                    "register {register} has conflicting semantic type descriptors",
                ));
            }
        } else {
            self.register_type_descriptors.insert(register, descriptor);
        }
        Ok(())
    }

    fn record_absent_register(&mut self, register: Register) -> MResult<()> {
        if self.register_schemas.contains_key(&register) {
            return invalid(format!(
                "register {register} cannot be both canonical and source-absent",
            ));
        }
        self.absent_registers.insert(register);
        Ok(())
    }

    fn record_register_constant_metadata(
        &mut self,
        register: Register,
        constant: u32,
    ) -> MResult<()> {
        let encoded = self
            .pending_constants
            .get(constant as usize)
            .ok_or_else(|| {
                invalid::<()>(format!("constant index {constant} is out of range")).unwrap_err()
            })?;
        let Some(cardinality) =
            encoded_collection_cardinality(&encoded.runtime_type, &encoded.bytes)?
        else {
            return Ok(());
        };
        if let Some(existing) = self.register_collection_cardinalities.get(&register) {
            if *existing != cardinality {
                return invalid(format!(
                    "register {register} has existing collection cardinality {existing}, incoming cardinality {cardinality}",
                ));
            }
        } else {
            self.register_collection_cardinalities
                .insert(register, cardinality);
        }
        Ok(())
    }

    fn record_register_constant_schema(
        &mut self,
        register: Register,
        constant: u32,
    ) -> MResult<()> {
        let encoded = self
            .pending_constants
            .get(constant as usize)
            .ok_or_else(|| {
                invalid::<()>(format!("constant index {constant} is out of range")).unwrap_err()
            })?;
        let schema =
            crate::program::bytecode::constants::runtime_schema_body(&encoded.runtime_type)?;
        self.record_register_schema(register, schema)?;
        self.record_register_constant_metadata(register, constant)
    }

    fn record_runtime_produced_register(&mut self, register: Register) -> MResult<()> {
        if self.register_state_initializers.contains_key(&register) {
            return invalid(format!(
                "state register {register} cannot be replaced by a runtime-produced value"
            ));
        }
        let initializers = self
            .instructions
            .iter()
            .enumerate()
            .filter_map(|(index, instruction)| match instruction {
                BytecodeInstruction::ConstLoad { dst, constant } if *dst == register => {
                    Some((index, *constant))
                }
                BytecodeInstruction::CompositePack { dst, template, .. } if *dst == register => {
                    Some((index, *template))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        if initializers.len() > 1 {
            return invalid(format!(
                "runtime-produced register {register} has more than one provisional initializer"
            ));
        }
        let Some((index, constant)) = initializers.first().copied() else {
            self.matrix_literals.remove(&register);
            self.runtime_produced_registers.insert(register);
            return Ok(());
        };
        self.remove_instruction(index);
        self.remove_constant_if_unreferenced(constant);
        self.matrix_literals.remove(&register);
        self.runtime_produced_registers.insert(register);
        Ok(())
    }

    fn register_is_runtime_produced(&self, register: Register) -> bool {
        self.runtime_produced_registers.contains(&register)
    }

    fn record_matrix_literal(&mut self, literal: CompiledMatrixLiteral) -> MResult<()> {
        if literal.output >= self.next_register {
            return invalid(format!(
                "matrix literal output register {} is outside register count {}",
                literal.output, self.next_register,
            ));
        }
        if let Some(element) = literal
            .elements
            .iter()
            .find(|element| element.register() >= self.next_register)
        {
            return invalid(format!(
                "matrix literal element register {} is outside register count {}",
                element.register(),
                self.next_register,
            ));
        }
        if self.runtime_produced_registers.contains(&literal.output) {
            return invalid(format!(
                "runtime-produced register {} has no executable matrix literal construction",
                literal.output,
            ));
        }
        if let Some(existing) = self.matrix_literals.get(&literal.output) {
            if existing != &literal {
                return invalid(format!(
                    "matrix literal output register {} has conflicting descriptors",
                    literal.output,
                ));
            }
            return Ok(());
        }
        self.matrix_literals.insert(literal.output, literal);
        Ok(())
    }

    fn record_state_initializer(&mut self, register: Register, constant: u32) -> MResult<()> {
        if self.pending_constants.get(constant as usize).is_none() {
            return invalid(format!("constant index {constant} is out of range"));
        }
        if let Some(existing) = self.register_state_initializers.get(&register).copied() {
            if existing != constant {
                return invalid(format!(
                    "register {register} has existing state initializer {existing}, incoming initializer {constant}",
                ));
            }
            return Ok(());
        }
        let seed_instruction = self
            .instructions
            .iter()
            .position(|instruction| match instruction {
                BytecodeInstruction::ConstLoad { dst, .. } => *dst == register,
                _ => false,
            })
            .ok_or_else(|| {
                invalid::<()>(format!(
                    "state register {register} has no declaration seed ConstLoad"
                ))
                .unwrap_err()
            })?;
        let BytecodeInstruction::ConstLoad { constant: seed, .. } =
            self.instructions[seed_instruction]
        else {
            unreachable!("seed instruction was filtered as ConstLoad");
        };
        if seed == constant {
            self.register_state_initializers.insert(register, constant);
            return Ok(());
        }
        if let BytecodeInstruction::ConstLoad {
            constant: target, ..
        } = &mut self.instructions[seed_instruction]
        {
            *target = constant;
        }
        self.register_state_initializers.insert(register, constant);
        self.remove_constant_if_unreferenced(seed);
        Ok(())
    }

    fn intern_constant(&mut self, constant: EncodedConstant) -> MResult<u32> {
        if let Some(index) = self.pending_constants.iter().position(|candidate| {
            candidate.runtime_type == constant.runtime_type && candidate.bytes == constant.bytes
        }) {
            self.pending_constants[index].alignment = self.pending_constants[index]
                .alignment
                .max(constant.alignment);
            return u32::try_from(index)
                .map_err(|_| invalid::<()>("constant index exceeds u32").unwrap_err());
        }
        let index = u32::try_from(self.pending_constants.len())
            .map_err(|_| invalid::<()>("constant index exceeds u32").unwrap_err())?;
        self.pending_constants.push(constant);
        Ok(index)
    }

    fn define_symbol(
        &mut self,
        pointer: usize,
        register: Register,
        name: &str,
        mutable: bool,
    ) -> MResult<()> {
        self.define_symbol_with_visibility(pointer, register, name, mutable, true)
    }

    fn define_local_symbol(
        &mut self,
        pointer: usize,
        register: Register,
        name: &str,
        mutable: bool,
    ) -> MResult<()> {
        self.define_symbol_with_visibility(pointer, register, name, mutable, false)
    }

    fn intern_requirement(&mut self, requirement: ApplicationRequirement) -> MResult<u32> {
        if let Some(index) = self
            .pending_requirements
            .iter()
            .position(|candidate| candidate == &requirement)
        {
            return u32::try_from(index)
                .map_err(|_| invalid::<()>("requirement index exceeds u32").unwrap_err());
        }
        let index = u32::try_from(self.pending_requirements.len())
            .map_err(|_| invalid::<()>("requirement index exceeds u32").unwrap_err())?;
        self.requirements
            .insert(CanonicalRequirement(requirement.clone()));
        self.pending_requirements.push(requirement);
        Ok(index)
    }

    fn emit_const_load(&mut self, destination: Register, constant: u32) {
        self.instructions.push(BytecodeInstruction::ConstLoad {
            dst: destination,
            constant,
        });
        self.instruction_roles.push(None);
        self.instruction_contracts.push(None);
        self.instruction_operations.push(None);
        self.instruction_source_nodes.push(None);
        self.instruction_type_bindings.push(None);
    }

    fn emit_composite_pack(
        &mut self,
        destination: Register,
        template: u32,
        children: Vec<Register>,
    ) {
        self.instructions.push(BytecodeInstruction::CompositePack {
            dst: destination,
            template,
            children,
        });
        // Composite construction is itself a semantic node even when a tuple,
        // record, or other literal is compiled outside a surrounding reactive
        // source node.
        self.instruction_roles
            .push(Some(CompiledInstructionRole::Node(
                CompiledNodeKind::Combinational,
            )));
        self.instruction_contracts
            .push(Some(PURE_COMPOSITE_PACK_CONTRACT.clone()));
        self.instruction_operations
            .push(Some("core/composite-pack".to_owned()));
        // This is a compiler-owned record/tuple/etc. construction node, not a
        // second lowering of the source plan node whose input requested it.
        self.instruction_source_nodes.push(None);
        self.instruction_type_bindings.push(None);
    }

    fn emit_nullop(&mut self, function: u64, destination: Register) {
        self.instructions.push(BytecodeInstruction::RuntimeNullary {
            function,
            dst: destination,
        });
        self.instruction_roles
            .push(self.current_node_kind.map(CompiledInstructionRole::Node));
        self.instruction_contracts
            .push(self.current_node_contract.clone());
        self.instruction_operations
            .push(self.current_node_operation.clone());
        self.instruction_source_nodes.push(self.current_source_node);
        self.instruction_type_bindings
            .push(self.current_node_type_binding.clone());
    }

    fn emit_unop(&mut self, function: u64, destination: Register, source: Register) {
        self.instructions.push(BytecodeInstruction::RuntimeUnary {
            function,
            dst: destination,
            src: source,
        });
        self.instruction_roles
            .push(self.current_node_kind.map(CompiledInstructionRole::Node));
        self.instruction_contracts
            .push(self.current_node_contract.clone());
        self.instruction_operations
            .push(self.current_node_operation.clone());
        self.instruction_source_nodes.push(self.current_source_node);
        self.instruction_type_bindings
            .push(self.current_node_type_binding.clone());
    }

    fn emit_binop(&mut self, function: u64, destination: Register, lhs: Register, rhs: Register) {
        self.instructions.push(BytecodeInstruction::RuntimeBinary {
            function,
            dst: destination,
            lhs,
            rhs,
        });
        self.instruction_roles
            .push(self.current_node_kind.map(CompiledInstructionRole::Node));
        self.instruction_contracts
            .push(self.current_node_contract.clone());
        self.instruction_operations
            .push(self.current_node_operation.clone());
        self.instruction_source_nodes.push(self.current_source_node);
        self.instruction_type_bindings
            .push(self.current_node_type_binding.clone());
    }

    fn emit_declaration_binary(
        &mut self,
        function: u64,
        destination: Register,
        first: Register,
        second: Register,
    ) {
        self.instructions.push(BytecodeInstruction::RuntimeBinary {
            function,
            dst: destination,
            lhs: first,
            rhs: second,
        });
        self.instruction_roles
            .push(Some(CompiledInstructionRole::DeclarationMarker));
        self.instruction_contracts.push(None);
        self.instruction_operations.push(None);
        self.instruction_source_nodes.push(None);
        self.instruction_type_bindings.push(None);
    }

    fn emit_ternop(
        &mut self,
        function: u64,
        destination: Register,
        a: Register,
        b: Register,
        c: Register,
    ) {
        self.instructions.push(BytecodeInstruction::RuntimeTernary {
            function,
            dst: destination,
            a,
            b,
            c,
        });
        self.instruction_roles
            .push(self.current_node_kind.map(CompiledInstructionRole::Node));
        self.instruction_contracts
            .push(self.current_node_contract.clone());
        self.instruction_operations
            .push(self.current_node_operation.clone());
        self.instruction_source_nodes.push(self.current_source_node);
        self.instruction_type_bindings
            .push(self.current_node_type_binding.clone());
    }

    fn emit_quadop(
        &mut self,
        function: u64,
        destination: Register,
        a: Register,
        b: Register,
        c: Register,
        d: Register,
    ) {
        self.instructions
            .push(BytecodeInstruction::RuntimeQuaternary {
                function,
                dst: destination,
                a,
                b,
                c,
                d,
            });
        self.instruction_roles
            .push(self.current_node_kind.map(CompiledInstructionRole::Node));
        self.instruction_contracts
            .push(self.current_node_contract.clone());
        self.instruction_operations
            .push(self.current_node_operation.clone());
        self.instruction_source_nodes.push(self.current_source_node);
        self.instruction_type_bindings
            .push(self.current_node_type_binding.clone());
    }

    fn emit_varop(&mut self, function: u64, destination: Register, arguments: Vec<Register>) {
        self.instructions
            .push(BytecodeInstruction::RuntimeVariadic {
                function,
                dst: destination,
                arguments,
            });
        self.instruction_roles
            .push(self.current_node_kind.map(CompiledInstructionRole::Node));
        self.instruction_contracts
            .push(self.current_node_contract.clone());
        self.instruction_operations
            .push(self.current_node_operation.clone());
        self.instruction_source_nodes.push(self.current_source_node);
        self.instruction_type_bindings
            .push(self.current_node_type_binding.clone());
    }

    fn emit_host_call(
        &mut self,
        requirement: u32,
        destination: Register,
        arguments: Vec<Register>,
    ) {
        self.instructions.push(BytecodeInstruction::HostCall {
            requirement,
            dst: destination,
            arguments,
        });
        self.instruction_roles
            .push(self.current_node_kind.map(CompiledInstructionRole::Node));
        self.instruction_contracts
            .push(self.current_node_contract.clone());
        self.instruction_operations
            .push(self.current_node_operation.clone());
        self.instruction_source_nodes.push(self.current_source_node);
        self.instruction_type_bindings
            .push(self.current_node_type_binding.clone());
    }

    fn emit_resource_read(&mut self, requirement: u32, destination: Register) {
        self.instructions.push(BytecodeInstruction::ResourceRead {
            requirement,
            dst: destination,
        });
        self.instruction_roles
            .push(self.current_node_kind.map(CompiledInstructionRole::Node));
        self.instruction_contracts
            .push(self.current_node_contract.clone());
        self.instruction_operations
            .push(self.current_node_operation.clone());
        self.instruction_source_nodes.push(self.current_source_node);
        self.instruction_type_bindings
            .push(self.current_node_type_binding.clone());
    }

    fn emit_resource_write(&mut self, requirement: u32, destination: Register, source: Register) {
        self.instructions.push(BytecodeInstruction::ResourceWrite {
            requirement,
            dst: destination,
            src: source,
        });
        self.instruction_roles
            .push(self.current_node_kind.map(CompiledInstructionRole::Node));
        self.instruction_contracts
            .push(self.current_node_contract.clone());
        self.instruction_operations
            .push(self.current_node_operation.clone());
        self.instruction_source_nodes.push(self.current_source_node);
        self.instruction_type_bindings
            .push(self.current_node_type_binding.clone());
    }

    fn emit_resource_send(&mut self, requirement: u32, destination: Register, source: Register) {
        self.instructions.push(BytecodeInstruction::ResourceSend {
            requirement,
            dst: destination,
            src: source,
        });
        self.instruction_roles
            .push(self.current_node_kind.map(CompiledInstructionRole::Node));
        self.instruction_contracts
            .push(self.current_node_contract.clone());
        self.instruction_operations
            .push(self.current_node_operation.clone());
        self.instruction_source_nodes.push(self.current_source_node);
        self.instruction_type_bindings
            .push(self.current_node_type_binding.clone());
    }
}

fn instruction_registers(instruction: &BytecodeInstruction) -> (Option<Register>, Vec<Register>) {
    match instruction {
        BytecodeInstruction::ConstLoad { dst, .. } => (Some(*dst), Vec::new()),
        BytecodeInstruction::CompositePack { dst, children, .. }
        | BytecodeInstruction::RuntimeVariadic {
            dst,
            arguments: children,
            ..
        }
        | BytecodeInstruction::HostCall {
            dst,
            arguments: children,
            ..
        } => (Some(*dst), children.clone()),
        BytecodeInstruction::RuntimeNullary { dst, .. }
        | BytecodeInstruction::ResourceRead { dst, .. } => (Some(*dst), Vec::new()),
        BytecodeInstruction::RuntimeUnary { dst, src, .. }
        | BytecodeInstruction::ResourceWrite { dst, src, .. }
        | BytecodeInstruction::ResourceSend { dst, src, .. } => (Some(*dst), vec![*src]),
        BytecodeInstruction::RuntimeBinary { dst, lhs, rhs, .. } => (Some(*dst), vec![*lhs, *rhs]),
        BytecodeInstruction::RuntimeTernary { dst, a, b, c, .. } => (Some(*dst), vec![*a, *b, *c]),
        BytecodeInstruction::RuntimeQuaternary {
            dst, a, b, c, d, ..
        } => (Some(*dst), vec![*a, *b, *c, *d]),
        BytecodeInstruction::Return { src } => (None, vec![*src]),
    }
}

fn record_compiled_descriptor(
    descriptors: &mut [Option<crate::ResolvedValueDescriptor>],
    register: Register,
    expected: &crate::ResolvedValueDescriptor,
) -> MResult<()> {
    let slot = descriptors.get_mut(register as usize).ok_or_else(|| {
        invalid::<()>(format!(
            "semantic type binding references register {register} outside the register table",
        ))
        .unwrap_err()
    })?;
    match slot {
        Some(actual) if !actual.has_same_type_contract(expected) => invalid(format!(
            "register {register} has conflicting bound semantic type descriptors {actual:?} and {expected:?}",
        )),
        Some(_) => Ok(()),
        None => {
            *slot = Some(expected.clone());
            Ok(())
        }
    }
}

/// Completes the dense register descriptor sidecar from the immutable bound
/// call carried by each executable source node. Legacy physical compiler
/// helpers may record only a canonical schema; they are not allowed to leave
/// the semantic descriptor absent from the finished compiler product.
fn complete_register_type_descriptors(
    instructions: &[BytecodeInstruction],
    bindings: &[Option<crate::BoundCall>],
    absent_registers: &BTreeSet<Register>,
    descriptors: &mut [Option<crate::ResolvedValueDescriptor>],
) -> MResult<()> {
    for (instruction, binding) in instructions.iter().zip(bindings) {
        let Some(binding) = binding else {
            continue;
        };
        let (destination, mut inputs) = instruction_registers(instruction);
        if binding.inputs().len() == inputs.len() + 1 {
            let base_input = binding
                .operation_descriptor()
                .contract
                .outputs
                .iter()
                .find_map(|output| match output.construction {
                    crate::OutputConstruction::ReadModifyWrite { base_input, .. } => {
                        Some(base_input as usize)
                    }
                    _ => None,
                });
            let Some(base_input) = base_input else {
                return invalid("semantic binding input arity exceeds the instruction operands");
            };
            let Some(destination) = destination else {
                return invalid("read/modify/write binding has no destination register");
            };
            if base_input > inputs.len() {
                return invalid("read/modify/write base input is outside the binding inputs");
            }
            inputs.insert(base_input, destination);
        }
        if inputs.len() != binding.inputs().len() {
            return invalid(format!(
                "instruction has {} semantic inputs but its binding declares {}",
                inputs.len(),
                binding.inputs().len(),
            ));
        }
        for (register, expected) in inputs.into_iter().zip(binding.inputs()) {
            if !absent_registers.contains(&register) {
                record_compiled_descriptor(descriptors, register, expected)?;
            }
        }
        if binding.outputs().len() != 1 {
            return invalid(format!(
                "executable binding declares {} outputs instead of one",
                binding.outputs().len(),
            ));
        }
        if let Some(destination) = destination
            && !absent_registers.contains(&destination)
        {
            record_compiled_descriptor(descriptors, destination, &binding.outputs()[0])?;
        }
    }
    Ok(())
}

fn encoded_collection_cardinality(
    runtime_type: &crate::RuntimeType,
    bytes: &[u8],
) -> MResult<Option<usize>> {
    match runtime_type {
        crate::RuntimeType::Map { .. } | crate::RuntimeType::Set { .. } => {
            let count = bytes.get(..4).ok_or_else(|| {
                invalid::<()>("collection constant is missing its element count").unwrap_err()
            })?;
            let count = u32::from_le_bytes(count.try_into().expect("four-byte slice"));
            Ok(Some(count as usize))
        }
        crate::RuntimeType::Option(inner) if bytes.first() == Some(&1) => {
            let length = bytes.get(1..5).ok_or_else(|| {
                invalid::<()>("option constant is missing its child length").unwrap_err()
            })?;
            let length = u32::from_le_bytes(length.try_into().expect("four-byte slice")) as usize;
            let child = bytes
                .get(
                    5..5_usize.checked_add(length).ok_or_else(|| {
                        invalid::<()>("option constant child length overflow").unwrap_err()
                    })?,
                )
                .ok_or_else(|| {
                    invalid::<()>("option constant child payload is truncated").unwrap_err()
                })?;
            encoded_collection_cardinality(inner, child)
        }
        _ => Ok(None),
    }
}

fn remap_instruction_requirement(
    instruction: &mut BytecodeInstruction,
    remap: &[u32],
) -> MResult<()> {
    let requirement = match instruction {
        BytecodeInstruction::HostCall { requirement, .. }
        | BytecodeInstruction::ResourceRead { requirement, .. }
        | BytecodeInstruction::ResourceWrite { requirement, .. }
        | BytecodeInstruction::ResourceSend { requirement, .. } => requirement,
        _ => return Ok(()),
    };
    *requirement = remap.get(*requirement as usize).copied().ok_or_else(|| {
        invalid::<()>("instruction requirement index is out of range").unwrap_err()
    })?;
    Ok(())
}

fn canonicalize_instruction_constants(
    instructions: &mut [BytecodeInstruction],
    pending: &[EncodedConstant],
) -> MResult<(Vec<EncodedConstant>, Vec<Option<u32>>)> {
    let mut constants = Vec::new();
    let mut remap = vec![None; pending.len()];
    for instruction in instructions {
        let constant = match instruction {
            BytecodeInstruction::ConstLoad { constant, .. } => constant,
            BytecodeInstruction::CompositePack { template, .. } => template,
            _ => continue,
        };
        let old = *constant as usize;
        let value = pending.get(old).ok_or_else(|| {
            invalid::<()>("instruction constant index is out of range").unwrap_err()
        })?;
        let canonical = match remap[old] {
            Some(canonical) => canonical,
            None => {
                let canonical = u32::try_from(constants.len()).map_err(|_| {
                    invalid::<()>("canonical constant count exceeds u32").unwrap_err()
                })?;
                constants.push(value.clone());
                remap[old] = Some(canonical);
                canonical
            }
        };
        *constant = canonical;
    }
    Ok((constants, remap))
}

fn invalid<T>(reason: impl Into<String>) -> MResult<T> {
    Err(MechError::new(
        BytecodeValidationError {
            reason: reason.into(),
        },
        None,
    )
    .with_compiler_loc())
}
