//! Semantic source-plan recorder shared by artifact compilation and bytecode v1 production.

use core::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::LazyLock;

use crate::{
    AccessMode, AliasPolicy, ApplicationRequirement, BytecodeCompilerContext, BytecodeInstruction,
    BytecodeProgram, BytecodeRegisterIdentity, BytecodeValidationError, ChangeDetectionPolicy,
    DeliveryMode, EncodedConstant, ExternalInteraction, InputPortLayout, InputPortPolicy,
    LegacyValue, MResult, MechError, OperationContractDeclaration, OutputConstruction,
    OutputPortPolicy, ParsedProgram, Register, ShapeRule, ValueKind,
    compare_application_requirements, compile_value_register, hash_str,
    value_kind_from_runtime_type, write_bytecode,
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
    /// The legacy runtime instruction used to retain current invariant
    /// behavior. It must not become a `ProgramArtifact` node.
    IntegrityMarker,
    /// Executable legacy variable-definition instruction whose semantic
    /// declaration is already represented by symbol and slot metadata.
    DeclarationMarker,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledSymbolDefinition {
    pub id: u64,
    pub name: String,
    pub register: Register,
    pub mutable: bool,
    /// Source/compiler definition order, assigned densely.
    pub ordinal: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompiledIntegrityConstraint {
    pub result_register: Register,
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
    pub instruction_contracts: Vec<Option<&'static OperationContractDeclaration>>,
    /// Dense source-plan node identity, parallel to `program.instructions`.
    pub instruction_source_nodes: Vec<Option<u32>>,
    /// Dense and parallel to the register space. `None` is permitted only for
    /// registers that never participate in semantic artifact data.
    pub register_kinds: Vec<Option<ValueKind>>,
    /// Exact current cardinality for map/set registers. Dense and parallel to
    /// the register space; other register families carry `None`.
    pub register_collection_cardinalities: Vec<Option<usize>>,
    /// Source-declaration initializer constant, dense and parallel to the
    /// register space. This is compilation sidecar metadata, not an
    /// executable instruction.
    pub register_state_initializers: Vec<Option<u32>>,
    /// Exact first-definition order, unlike the canonically sorted symbol map.
    pub symbol_definitions: Vec<CompiledSymbolDefinition>,
    pub return_register: Register,
    pub integrity_constraints: Vec<CompiledIntegrityConstraint>,
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
    dictionary: BTreeMap<u64, String>,
    runtime_function_names: BTreeMap<u64, String>,
    mutable_symbols: BTreeSet<u64>,
    pending_constants: Vec<EncodedConstant>,
    requirements: BTreeSet<CanonicalRequirement>,
    pending_requirements: Vec<ApplicationRequirement>,
    instructions: Vec<BytecodeInstruction>,
    instruction_roles: Vec<Option<CompiledInstructionRole>>,
    instruction_contracts: Vec<Option<&'static OperationContractDeclaration>>,
    instruction_source_nodes: Vec<Option<u32>>,
    register_kinds: BTreeMap<Register, ValueKind>,
    register_collection_cardinalities: BTreeMap<Register, usize>,
    register_state_initializers: BTreeMap<Register, u32>,
    next_register_kind_override: Option<ValueKind>,
    symbol_definitions: Vec<CompiledSymbolDefinition>,
    current_node_kind: Option<CompiledNodeKind>,
    current_node_contract: Option<&'static OperationContractDeclaration>,
    current_source_node: Option<u32>,
    next_source_node: u32,
    integrity_constraints: Vec<CompiledIntegrityConstraint>,
    next_register: Register,
}

impl Default for CompileCtx {
    fn default() -> Self {
        Self {
            reg_map: HashMap::new(),
            symbols: BTreeMap::new(),
            symbol_ptrs: BTreeMap::new(),
            dictionary: BTreeMap::new(),
            runtime_function_names: BTreeMap::new(),
            mutable_symbols: BTreeSet::new(),
            pending_constants: Vec::new(),
            requirements: BTreeSet::new(),
            pending_requirements: Vec::new(),
            instructions: Vec::new(),
            instruction_roles: Vec::new(),
            instruction_contracts: Vec::new(),
            instruction_source_nodes: Vec::new(),
            register_kinds: BTreeMap::new(),
            register_collection_cardinalities: BTreeMap::new(),
            register_state_initializers: BTreeMap::new(),
            next_register_kind_override: None,
            symbol_definitions: Vec::new(),
            current_node_kind: None,
            current_node_contract: None,
            current_source_node: None,
            next_source_node: 0,
            integrity_constraints: Vec::new(),
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

    /// Resolve an interpreted value to the register that carries its identity.
    ///
    /// Planned outputs reuse their producer register. A final value that was
    /// not part of the plan (for example, a trailing literal) is materialized
    /// exactly once so `Return` still represents the source block's result.
    pub fn resolve_value_register(&mut self, value: &LegacyValue) -> MResult<Register> {
        let fallback = std::ptr::from_ref(value).addr();
        let register = compile_value_register(value, fallback, self)?;
        let kind = value.kind();
        self.record_register_kind_exact(register, kind)?;
        Ok(register)
    }

    fn record_register_kind_exact(&mut self, register: Register, kind: ValueKind) -> MResult<()> {
        if let Some(existing) = self.register_kinds.get(&register).cloned() {
            if existing != kind {
                match (&existing, &kind) {
                    (existing, ValueKind::Reference(incoming)) if existing == incoming.as_ref() => {
                        self.register_kinds.insert(register, kind);
                    }
                    (ValueKind::Reference(existing), incoming) if existing.as_ref() == incoming => {
                    }
                    (
                        ValueKind::Table(existing_columns, existing_rows),
                        ValueKind::Table(incoming_columns, incoming_rows),
                    ) if existing_columns == incoming_columns
                        && (*existing_rows == 0 || *incoming_rows == 0) =>
                    {
                        // A zero row count is the source compiler's dynamic
                        // table shape. Once planning materializes the final
                        // output, retain the concrete row count on the same
                        // producer register.
                        if *incoming_rows != 0 {
                            self.register_kinds.insert(register, kind);
                        }
                    }
                    _ => {
                        return invalid(format!(
                            "register {register} has existing ValueKind {existing:?}, incoming ValueKind {kind:?}",
                        ));
                    }
                }
            }
        } else {
            self.register_kinds.insert(register, kind);
        }
        Ok(())
    }

    fn remove_instruction(&mut self, index: usize) {
        self.instructions.remove(index);
        self.instruction_roles.remove(index);
        self.instruction_contracts.remove(index);
        self.instruction_source_nodes.remove(index);
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
        self.begin_plan_node_with_contract(kind, None)
    }

    pub fn begin_plan_node_with_contract(
        &mut self,
        kind: CompiledNodeKind,
        contract: Option<&'static OperationContractDeclaration>,
    ) -> MResult<()> {
        if self.current_node_kind.is_some() {
            return invalid("cannot begin a bytecode plan node while another node is active");
        }
        self.current_node_kind = Some(kind);
        self.current_node_contract = contract;
        self.current_source_node = Some(self.next_source_node);
        self.next_source_node = self
            .next_source_node
            .checked_add(1)
            .ok_or_else(|| invalid::<()>("source plan node identity exceeds u32").unwrap_err())?;
        Ok(())
    }

    pub fn end_plan_node(&mut self) {
        self.current_node_kind = None;
        self.current_node_contract = None;
        self.current_source_node = None;
    }

    pub fn record_integrity_constraint(&mut self, result_register: Register) -> MResult<()> {
        if result_register >= self.next_register {
            return invalid(format!(
                "integrity result register {result_register} is outside register count {}",
                self.next_register,
            ));
        }
        self.integrity_constraints
            .push(CompiledIntegrityConstraint { result_register });
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
        self.instruction_source_nodes.push(None);
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
        if self.instruction_source_nodes.len() != self.instructions.len() {
            return invalid(format!(
                "instruction source node count {} does not match instruction count {}",
                self.instruction_source_nodes.len(),
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
        let mut instruction_source_nodes = self.instruction_source_nodes.clone();
        instruction_source_nodes.push(None);

        let mut register_kinds = vec![None; self.next_register as usize];
        for (register, kind) in &self.register_kinds {
            let target = register_kinds.get_mut(*register as usize).ok_or_else(|| {
                invalid::<()>(format!(
                    "recorded register kind {register} is outside register count {}",
                    self.next_register,
                ))
                .unwrap_err()
            })?;
            *target = Some(kind.clone());
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
            instruction_source_nodes,
            register_kinds,
            register_collection_cardinalities,
            register_state_initializers,
            symbol_definitions: self.symbol_definitions.clone(),
            return_register,
            integrity_constraints: self.integrity_constraints.clone(),
        })
    }

    pub fn finish(&mut self, return_register: Register) -> MResult<Vec<u8>> {
        let compiled = self.finish_program(return_register)?;
        let bytes = write_bytecode(&compiled.program)?;
        ParsedProgram::from_bytes(&bytes)?;
        Ok(bytes)
    }

    fn register_for_identity(&mut self, identity: BytecodeRegisterIdentity) -> (Register, bool) {
        if let Some(&register) = self.reg_map.get(&identity) {
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

    fn register_for_identity_with_initialization_status(
        &mut self,
        identity: &BytecodeRegisterIdentity,
    ) -> (Register, bool) {
        self.register_for_identity(identity.clone())
    }

    fn record_register_kind(&mut self, register: Register, kind: ValueKind) -> MResult<()> {
        let kind = self.next_register_kind_override.take().unwrap_or(kind);
        self.record_register_kind_exact(register, kind)
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

    fn override_next_register_kind(&mut self, kind: ValueKind) -> MResult<()> {
        if self.next_register_kind_override.is_none() {
            self.next_register_kind_override = Some(kind);
        }
        Ok(())
    }

    fn record_register_constant_kind(&mut self, register: Register, constant: u32) -> MResult<()> {
        let encoded = self
            .pending_constants
            .get(constant as usize)
            .ok_or_else(|| {
                invalid::<()>(format!("constant index {constant} is out of range")).unwrap_err()
            })?;
        let kind = value_kind_from_runtime_type(&encoded.runtime_type)?;
        self.record_register_kind_exact(register, kind)?;
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
            return Ok(());
        };
        self.remove_instruction(index);
        self.remove_constant_if_unreferenced(constant);
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
        if let Some(index) = self
            .pending_constants
            .iter()
            .position(|candidate| candidate == &constant)
        {
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
            return Ok(());
        }

        self.symbols.insert(symbol_id, register);
        self.symbol_ptrs.insert(symbol_id, pointer);
        self.dictionary.insert(symbol_id, name.to_owned());
        if let Some(kind) = self.register_kinds.get(&register).cloned() {
            if !matches!(kind, ValueKind::Reference(_)) {
                self.register_kinds
                    .insert(register, ValueKind::Reference(Box::new(kind)));
            }
        }
        if mutable {
            self.mutable_symbols.insert(symbol_id);
        }
        let ordinal = u32::try_from(self.symbol_definitions.len())
            .map_err(|_| invalid::<()>("symbol definition ordinal exceeds u32").unwrap_err())?;
        self.symbol_definitions.push(CompiledSymbolDefinition {
            id: symbol_id,
            name: name.to_owned(),
            register,
            mutable,
            ordinal,
        });
        Ok(())
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
        self.instruction_source_nodes.push(None);
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
            .push(Some(&PURE_COMPOSITE_PACK_CONTRACT));
        // This is a compiler-owned record/tuple/etc. construction node, not a
        // second lowering of the source plan node whose input requested it.
        self.instruction_source_nodes.push(None);
    }

    fn emit_nullop(&mut self, function: u64, destination: Register) {
        self.instructions.push(BytecodeInstruction::RuntimeNullary {
            function,
            dst: destination,
        });
        self.instruction_roles
            .push(self.current_node_kind.map(CompiledInstructionRole::Node));
        self.instruction_contracts.push(self.current_node_contract);
        self.instruction_source_nodes.push(self.current_source_node);
    }

    fn emit_unop(&mut self, function: u64, destination: Register, source: Register) {
        self.instructions.push(BytecodeInstruction::RuntimeUnary {
            function,
            dst: destination,
            src: source,
        });
        self.instruction_roles
            .push(self.current_node_kind.map(CompiledInstructionRole::Node));
        self.instruction_contracts.push(self.current_node_contract);
        self.instruction_source_nodes.push(self.current_source_node);
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
        self.instruction_contracts.push(self.current_node_contract);
        self.instruction_source_nodes.push(self.current_source_node);
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
        self.instruction_source_nodes.push(None);
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
        self.instruction_contracts.push(self.current_node_contract);
        self.instruction_source_nodes.push(self.current_source_node);
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
        self.instruction_contracts.push(self.current_node_contract);
        self.instruction_source_nodes.push(self.current_source_node);
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
        self.instruction_contracts.push(self.current_node_contract);
        self.instruction_source_nodes.push(self.current_source_node);
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
        self.instruction_contracts.push(self.current_node_contract);
        self.instruction_source_nodes.push(self.current_source_node);
    }

    fn emit_resource_read(&mut self, requirement: u32, destination: Register) {
        self.instructions.push(BytecodeInstruction::ResourceRead {
            requirement,
            dst: destination,
        });
        self.instruction_roles
            .push(self.current_node_kind.map(CompiledInstructionRole::Node));
        self.instruction_contracts.push(self.current_node_contract);
        self.instruction_source_nodes.push(self.current_source_node);
    }

    fn emit_resource_write(&mut self, requirement: u32, destination: Register, source: Register) {
        self.instructions.push(BytecodeInstruction::ResourceWrite {
            requirement,
            dst: destination,
            src: source,
        });
        self.instruction_roles
            .push(self.current_node_kind.map(CompiledInstructionRole::Node));
        self.instruction_contracts.push(self.current_node_contract);
        self.instruction_source_nodes.push(self.current_source_node);
    }

    fn emit_resource_send(&mut self, requirement: u32, destination: Register, source: Register) {
        self.instructions.push(BytecodeInstruction::ResourceSend {
            requirement,
            dst: destination,
            src: source,
        });
        self.instruction_roles
            .push(self.current_node_kind.map(CompiledInstructionRole::Node));
        self.instruction_contracts.push(self.current_node_contract);
        self.instruction_source_nodes.push(self.current_source_node);
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AccessMode, AliasPolicy, ChangeDetectionPolicy, DeliveryMode, ExecutionHostFunctionRequest,
        ExecutionResourceRequest, ExternalInteraction, InputPortLayout, InputPortPolicy,
        OperationContractDeclaration, OutputConstruction, OutputPortPolicy, ResourceDelivery,
        ResourceIntent, RuntimeType, ShapeRule,
    };

    fn f64_constant(value: f64) -> EncodedConstant {
        EncodedConstant {
            runtime_type: RuntimeType::F64,
            alignment: 8,
            bytes: value.to_bits().to_le_bytes().to_vec(),
        }
    }

    fn allocate_registers(context: &mut CompileCtx, count: usize) -> Vec<Register> {
        (0..count)
            .map(|pointer| {
                context
                    .register_for_ptr_with_initialization_status(pointer)
                    .0
            })
            .collect()
    }

    fn initialize_registers(context: &mut CompileCtx, registers: &[Register]) -> u32 {
        let constant = context.intern_constant(f64_constant(0.0)).unwrap();
        for register in registers {
            context.emit_const_load(*register, constant);
        }
        constant
    }

    fn host_requirement(name: &str) -> ApplicationRequirement {
        ApplicationRequirement::HostFunction(ExecutionHostFunctionRequest {
            name: name.to_owned(),
        })
    }

    fn resource_requirement(base_uri: &str) -> ApplicationRequirement {
        ApplicationRequirement::Resource(ExecutionResourceRequest {
            base_uri: base_uri.to_owned(),
            path: "value".to_owned(),
            context_name: "ctx".to_owned(),
            operation: "read".to_owned(),
            intent: ResourceIntent::Read,
            delivery: ResourceDelivery::Snapshot,
        })
    }

    #[test]
    fn pointer_registers_are_initialized_once() {
        let mut context = CompileCtx::new();
        let (first, initializes_first) = context.register_for_ptr_with_initialization_status(100);
        let (same, initializes_same) = context.register_for_ptr_with_initialization_status(100);
        let (second, initializes_second) = context.register_for_ptr_with_initialization_status(200);

        assert_eq!(first, same);
        assert_ne!(first, second);
        assert!(initializes_first);
        assert!(!initializes_same);
        assert!(initializes_second);

        context.clear();
        assert_eq!(
            context.register_for_ptr_with_initialization_status(100),
            (0, true),
        );
    }

    #[test]
    fn runtime_producer_discards_an_earlier_planning_seed() {
        let value = LegacyValue::F64(crate::Ref::new(7.0));
        let fallback = std::ptr::from_ref(&value).addr();
        let mut context = CompileCtx::new();
        let seeded = compile_value_register(&value, fallback, &mut context).unwrap();

        let produced =
            crate::compile_runtime_produced_register(&value, fallback, &mut context).unwrap();
        let requirement = context
            .intern_requirement(resource_requirement("test://provider"))
            .unwrap();
        context.emit_resource_read(requirement, produced);
        let compiled = context.finish_program(produced).unwrap();

        assert_eq!(seeded, produced);
        assert!(compiled.program.constants.is_empty());
        assert!(!compiled.program.instructions.iter().any(|instruction| {
            matches!(instruction, BytecodeInstruction::ConstLoad { dst, .. } if *dst == produced)
        }));
        assert!(compiled.program.instructions.iter().any(|instruction| {
            matches!(instruction, BytecodeInstruction::ResourceRead { dst, .. } if *dst == produced)
        }));
    }

    #[test]
    fn runtime_producer_keeps_a_seed_constant_used_by_another_register() {
        let produced_value = LegacyValue::F64(crate::Ref::new(7.0));
        let retained_value = LegacyValue::F64(crate::Ref::new(7.0));
        let produced_fallback = std::ptr::from_ref(&produced_value).addr();
        let retained_fallback = std::ptr::from_ref(&retained_value).addr();
        let mut context = CompileCtx::new();
        let produced =
            compile_value_register(&produced_value, produced_fallback, &mut context).unwrap();
        let retained =
            compile_value_register(&retained_value, retained_fallback, &mut context).unwrap();

        crate::compile_runtime_produced_register(&produced_value, produced_fallback, &mut context)
            .unwrap();
        let requirement = context
            .intern_requirement(resource_requirement("test://provider"))
            .unwrap();
        context.emit_resource_read(requirement, produced);
        let compiled = context.finish_program(produced).unwrap();

        assert_eq!(compiled.program.constants.len(), 1);
        assert!(compiled.program.instructions.iter().any(|instruction| {
            matches!(instruction, BytecodeInstruction::ConstLoad { dst, .. } if *dst == retained)
        }));
        assert!(!compiled.program.instructions.iter().any(|instruction| {
            matches!(instruction, BytecodeInstruction::ConstLoad { dst, .. } if *dst == produced)
        }));
    }

    #[test]
    fn typed_wrappers_do_not_share_registers_with_bare_values() {
        for typed_first in [false, true] {
            let scalar = crate::Ref::new(7.0);
            let bare = LegacyValue::F64(scalar.clone());
            let typed = LegacyValue::Typed(
                Box::new(LegacyValue::F64(scalar)),
                crate::ValueKind::Option(Box::new(crate::ValueKind::F64)),
            );
            let typed_clone = typed.clone();
            let mut context = CompileCtx::new();

            let (first, second) = if typed_first {
                (
                    context.resolve_value_register(&typed).unwrap(),
                    context.resolve_value_register(&bare).unwrap(),
                )
            } else {
                (
                    context.resolve_value_register(&bare).unwrap(),
                    context.resolve_value_register(&typed).unwrap(),
                )
            };
            assert_ne!(first, second);
            assert_eq!(
                context.resolve_value_register(&typed_clone).unwrap(),
                if typed_first { first } else { second },
            );

            let parsed = ParsedProgram::from_bytes(&context.finish(second).unwrap()).unwrap();
            assert_eq!(parsed.constants.len(), 2);
            assert!(
                parsed
                    .constants
                    .iter()
                    .any(|constant| parsed.types[constant.type_id as usize] == RuntimeType::F64)
            );
            assert!(parsed.constants.iter().any(|constant| {
                parsed.types[constant.type_id as usize]
                    == RuntimeType::Option(Box::new(RuntimeType::F64))
            }));
            assert_eq!(
                parsed
                    .instructions
                    .iter()
                    .filter(|instruction| matches!(
                        instruction,
                        BytecodeInstruction::ConstLoad { .. }
                    ))
                    .count(),
                1,
            );
            assert_eq!(
                parsed
                    .instructions
                    .iter()
                    .filter(|instruction| matches!(
                        instruction,
                        BytecodeInstruction::CompositePack { .. }
                    ))
                    .count(),
                1,
            );
        }
    }

    #[test]
    fn complete_typed_annotations_are_part_of_register_identity() {
        let scalar = crate::Ref::new(7.0);
        let inner = Box::new(BytecodeRegisterIdentity::Cell(scalar.id() as usize));
        let option_f64 = BytecodeRegisterIdentity::Typed {
            inner: inner.clone(),
            annotation: crate::ValueKind::Option(Box::new(crate::ValueKind::F64)),
        };
        let option_u64 = BytecodeRegisterIdentity::Typed {
            inner,
            annotation: crate::ValueKind::Option(Box::new(crate::ValueKind::U64)),
        };
        let mut context = CompileCtx::new();

        let f64_register = context
            .register_for_identity_with_initialization_status(&option_f64)
            .0;
        let u64_register = context
            .register_for_identity_with_initialization_status(&option_u64)
            .0;

        assert_ne!(f64_register, u64_register);
        assert_eq!(
            context
                .register_for_identity_with_initialization_status(&option_f64)
                .0,
            f64_register,
        );
    }

    #[test]
    fn mutable_references_reuse_their_producer_register() {
        for mutable_first in [false, true] {
            let scalar = crate::Ref::new(7.0);
            let bare = LegacyValue::F64(scalar.clone());
            let mutable = LegacyValue::MutableReference(crate::Ref::new(LegacyValue::F64(scalar)));
            let mut context = CompileCtx::new();

            let (first, second) = if mutable_first {
                (
                    context.resolve_value_register(&mutable).unwrap(),
                    context.resolve_value_register(&bare).unwrap(),
                )
            } else {
                (
                    context.resolve_value_register(&bare).unwrap(),
                    context.resolve_value_register(&mutable).unwrap(),
                )
            };

            assert_eq!(first, second);
            let parsed = ParsedProgram::from_bytes(&context.finish(second).unwrap()).unwrap();
            assert_eq!(parsed.constants.len(), 1);
        }
    }

    #[test]
    fn conflicting_register_kinds_report_both_exact_kinds() {
        let mut context = CompileCtx::new();
        let register = allocate_registers(&mut context, 1)[0];
        context
            .record_register_kind(register, ValueKind::F64)
            .unwrap();

        let error = context
            .record_register_kind(register, ValueKind::Bool)
            .unwrap_err();
        let message = format!("{error:?}");

        assert!(
            message.contains(&format!("register {register}")),
            "{message}"
        );
        assert!(message.contains("existing ValueKind F64"), "{message}");
        assert!(message.contains("incoming ValueKind Bool"), "{message}");
    }

    #[test]
    fn composite_values_are_lowered_from_child_registers() {
        let scalar = crate::Ref::new(7.0);
        let bare = LegacyValue::F64(scalar.clone());
        let tuple = LegacyValue::Tuple(crate::Ref::new(crate::MechTuple::from_vec(vec![
            LegacyValue::F64(scalar.clone()),
            LegacyValue::F64(scalar),
        ])));
        let mut context = CompileCtx::new();

        let scalar_register = context.resolve_value_register(&bare).unwrap();
        let tuple_register = context.resolve_value_register(&tuple).unwrap();
        let parsed = ParsedProgram::from_bytes(&context.finish(tuple_register).unwrap()).unwrap();

        assert!(parsed.instructions.iter().any(|instruction| matches!(
            instruction,
            BytecodeInstruction::CompositePack { dst, children, .. }
                if *dst == tuple_register
                    && children == &[scalar_register, scalar_register]
        )));
        assert!(!parsed.instructions.iter().any(|instruction| matches!(
            instruction,
            BytecodeInstruction::ConstLoad { dst, .. } if *dst == tuple_register
        )));
    }

    #[test]
    fn composite_helpers_inside_register_steps_are_combinational() {
        let mut context = CompileCtx::new();
        let registers = allocate_registers(&mut context, 2);
        context.begin_plan_node(CompiledNodeKind::Register).unwrap();
        context.emit_composite_pack(registers[0], 0, vec![registers[1]]);
        context.emit_unop(1, registers[0], registers[1]);
        context.end_plan_node();

        assert_eq!(
            context.instruction_roles,
            vec![
                Some(CompiledInstructionRole::Node(
                    CompiledNodeKind::Combinational
                )),
                Some(CompiledInstructionRole::Node(CompiledNodeKind::Register)),
            ]
        );
    }

    #[test]
    fn semantic_contracts_and_source_nodes_are_parallel_to_instructions() {
        let declaration = Box::leak(Box::new(OperationContractDeclaration {
            inputs: InputPortLayout::Fixed(
                vec![InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                }]
                .into_boxed_slice(),
            ),
            outputs: vec![OutputPortPolicy {
                access: AccessMode::Write,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::FullWrite {
                    shape: ShapeRule::Declared,
                },
                alias: AliasPolicy::NoAlias,
                change_detection: ChangeDetectionPolicy::ExactScalar,
            }]
            .into_boxed_slice(),
            interaction: ExternalInteraction::Pure,
        }));
        let mut context = CompileCtx::new();
        let registers = allocate_registers(&mut context, 2);
        initialize_registers(&mut context, &registers);
        context
            .begin_plan_node_with_contract(CompiledNodeKind::Combinational, Some(declaration))
            .unwrap();
        context.emit_unop(1, registers[1], registers[0]);
        context.end_plan_node();

        let compiled = context.finish_program(registers[1]).unwrap();
        assert_eq!(
            compiled.instruction_contracts.len(),
            compiled.program.instructions.len()
        );
        assert_eq!(
            compiled.instruction_source_nodes.len(),
            compiled.program.instructions.len()
        );
        assert_eq!(compiled.instruction_contracts[2], Some(&*declaration));
        assert_eq!(compiled.instruction_source_nodes[2], Some(0));
        assert_eq!(compiled.instruction_contracts[3], None);
        assert_eq!(compiled.instruction_source_nodes[3], None);
    }

    #[test]
    fn constants_are_interned_and_self_validated() {
        let mut context = CompileCtx::new();
        let register = allocate_registers(&mut context, 1)[0];
        let first = context.intern_constant(f64_constant(3.0)).unwrap();
        let duplicate = context.intern_constant(f64_constant(3.0)).unwrap();
        assert_eq!(first, duplicate);
        context.emit_const_load(register, first);

        let parsed = ParsedProgram::from_bytes(&context.finish(register).unwrap()).unwrap();
        assert_eq!(parsed.constants.len(), 1);
        assert_eq!(parsed.decode_constants().unwrap().len(), 1);
        assert_eq!(
            parsed.instructions,
            vec![
                BytecodeInstruction::ConstLoad {
                    dst: register,
                    constant: first,
                },
                BytecodeInstruction::Return { src: register },
            ],
        );
    }

    #[test]
    fn finish_orders_constants_by_first_instruction_reference() {
        let mut context = CompileCtx::new();
        let registers = allocate_registers(&mut context, 2);
        let skipped = context.intern_constant(f64_constant(1.0)).unwrap();
        let first = context.intern_constant(f64_constant(2.0)).unwrap();
        let second = context.intern_constant(f64_constant(3.0)).unwrap();
        assert_eq!((skipped, first, second), (0, 1, 2));
        context.emit_const_load(registers[0], second);
        context.emit_const_load(registers[1], first);

        let compiled = context.finish_program(registers[1]).unwrap();
        assert_eq!(
            compiled.program.constants,
            vec![f64_constant(3.0), f64_constant(2.0)]
        );
        assert!(matches!(
            compiled.program.instructions[0],
            BytecodeInstruction::ConstLoad { constant: 0, .. }
        ));
        assert!(matches!(
            compiled.program.instructions[1],
            BytecodeInstruction::ConstLoad { constant: 1, .. }
        ));
    }

    #[test]
    fn symbol_and_dictionary_order_is_deterministic() {
        fn compile(reverse: bool) -> Vec<u8> {
            let mut context = CompileCtx::new();
            let registers = allocate_registers(&mut context, 2);
            initialize_registers(&mut context, &registers);
            let definitions = [
                (10usize, registers[0], "alpha", false),
                (20usize, registers[1], "omega", true),
            ];
            for index in if reverse { [1, 0] } else { [0, 1] } {
                let (pointer, register, name, mutable) = definitions[index];
                context
                    .define_symbol(pointer, register, name, mutable)
                    .unwrap();
            }
            context.finish(registers[0]).unwrap()
        }

        assert_eq!(compile(false), compile(true));
    }

    #[test]
    fn conflicting_and_empty_symbols_are_rejected() {
        let mut context = CompileCtx::new();
        let registers = allocate_registers(&mut context, 2);
        assert!(context.define_symbol(1, registers[0], "", false).is_err());
        context
            .define_symbol(1, registers[0], "answer", false)
            .unwrap();
        assert!(
            context
                .define_symbol(2, registers[1], "answer", false)
                .is_err()
        );
    }

    #[test]
    fn requirements_are_canonicalized_and_instruction_indexes_are_remapped() {
        fn compile(resource_first: bool) -> Vec<u8> {
            let mut context = CompileCtx::new();
            let registers = allocate_registers(&mut context, 2);
            initialize_registers(&mut context, &registers[..1]);
            let host = host_requirement("cli/stdout");
            let resource = resource_requirement("context://input");
            let (host_id, resource_id) = if resource_first {
                let resource_id = context.intern_requirement(resource).unwrap();
                let host_id = context.intern_requirement(host).unwrap();
                (host_id, resource_id)
            } else {
                let host_id = context.intern_requirement(host).unwrap();
                let resource_id = context.intern_requirement(resource).unwrap();
                (host_id, resource_id)
            };
            context.emit_host_call(host_id, registers[0], Vec::new());
            context.emit_resource_read(resource_id, registers[1]);
            context.finish(registers[1]).unwrap()
        }

        let resource_first = compile(true);
        assert_eq!(resource_first, compile(false));
        let parsed = ParsedProgram::from_bytes(&resource_first).unwrap();
        assert!(matches!(
            parsed.requirements[0],
            ApplicationRequirement::HostFunction(_)
        ));
        assert!(matches!(
            parsed.instructions[1],
            BytecodeInstruction::HostCall { requirement: 0, .. }
        ));
        assert!(matches!(
            parsed.instructions[2],
            BytecodeInstruction::ResourceRead { requirement: 1, .. }
        ));
    }

    #[test]
    fn finish_appends_one_final_return_and_is_repeatable() {
        let mut context = CompileCtx::new();
        let register = allocate_registers(&mut context, 1)[0];
        let constant = initialize_registers(&mut context, &[register]);
        let first = context.finish(register).unwrap();
        let second = context.finish(register).unwrap();
        assert_eq!(first, second);

        let parsed = ParsedProgram::from_bytes(&first).unwrap();
        assert_eq!(
            parsed.instructions,
            vec![
                BytecodeInstruction::ConstLoad {
                    dst: register,
                    constant,
                },
                BytecodeInstruction::Return { src: register },
            ],
        );
    }

    #[test]
    fn all_instruction_shapes_round_trip() {
        let mut context = CompileCtx::new();
        let registers = allocate_registers(&mut context, 8);
        initialize_registers(&mut context, &registers[..7]);
        let host = context
            .intern_requirement(host_requirement("cli/stdout"))
            .unwrap();
        let resource = context
            .intern_requirement(resource_requirement("context://input"))
            .unwrap();

        context.emit_nullop(1, registers[0]);
        context.emit_unop(2, registers[1], registers[0]);
        context.emit_binop(3, registers[2], registers[0], registers[1]);
        context.emit_ternop(4, registers[3], registers[0], registers[1], registers[2]);
        context.emit_quadop(
            5,
            registers[4],
            registers[0],
            registers[1],
            registers[2],
            registers[3],
        );
        context.emit_varop(6, registers[5], registers[..5].to_vec());
        context.emit_host_call(host, registers[5], registers[..2].to_vec());
        context.emit_resource_read(resource, registers[7]);
        context.emit_resource_write(resource, registers[5], registers[7]);
        context.emit_resource_send(resource, registers[6], registers[5]);

        let parsed = ParsedProgram::from_bytes(&context.finish(registers[6]).unwrap()).unwrap();
        assert_eq!(parsed.instructions.len(), 18);
        assert_eq!(
            parsed.instructions.last(),
            Some(&BytecodeInstruction::Return { src: registers[6] }),
        );
    }

    #[test]
    fn finish_rejects_out_of_range_return_register() {
        assert!(CompileCtx::new().finish(0).is_err());
    }
}
