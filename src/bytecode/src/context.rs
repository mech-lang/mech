use core::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use mech_core::{
    ApplicationRequirement, BytecodeCompilerContext, BytecodeInstruction, BytecodeProgram,
    BytecodeRegisterIdentity, BytecodeValidationError, EncodedConstant, LegacyValue, MResult,
    MechError, OperationContractDeclaration, ParsedProgram, Register, ValueKind,
    compare_application_requirements, compile_value_register, hash_str,
    value_kind_from_runtime_type, write_bytecode,
};

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

        Ok(CompiledBytecode {
            program: BytecodeProgram {
                register_count: self.next_register,
                constants: self.pending_constants.clone(),
                symbols: self.symbols.clone(),
                mutable_symbols: self.mutable_symbols.clone(),
                instructions,
                dictionary: self.dictionary.clone(),
                requirements,
            },
            instruction_roles,
            instruction_contracts,
            instruction_source_nodes,
            register_kinds,
            register_collection_cardinalities,
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
        self.instruction_roles.push(
            self.current_node_kind
                .map(|_| CompiledInstructionRole::Node(CompiledNodeKind::Combinational)),
        );
        self.instruction_contracts.push(self.current_node_contract);
        self.instruction_source_nodes.push(self.current_source_node);
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
    runtime_type: &mech_core::RuntimeType,
    bytes: &[u8],
) -> MResult<Option<usize>> {
    match runtime_type {
        mech_core::RuntimeType::Map { .. } | mech_core::RuntimeType::Set { .. } => {
            let count = bytes.get(..4).ok_or_else(|| {
                invalid::<()>("collection constant is missing its element count").unwrap_err()
            })?;
            let count = u32::from_le_bytes(count.try_into().expect("four-byte slice"));
            Ok(Some(count as usize))
        }
        mech_core::RuntimeType::Option(inner) if bytes.first() == Some(&1) => {
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
    use mech_core::{
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
    fn typed_wrappers_do_not_share_registers_with_bare_values() {
        for typed_first in [false, true] {
            let scalar = mech_core::Ref::new(7.0);
            let bare = LegacyValue::F64(scalar.clone());
            let typed = LegacyValue::Typed(
                Box::new(LegacyValue::F64(scalar)),
                mech_core::ValueKind::Option(Box::new(mech_core::ValueKind::F64)),
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
        let scalar = mech_core::Ref::new(7.0);
        let inner = Box::new(BytecodeRegisterIdentity::Cell(scalar.id() as usize));
        let option_f64 = BytecodeRegisterIdentity::Typed {
            inner: inner.clone(),
            annotation: mech_core::ValueKind::Option(Box::new(mech_core::ValueKind::F64)),
        };
        let option_u64 = BytecodeRegisterIdentity::Typed {
            inner,
            annotation: mech_core::ValueKind::Option(Box::new(mech_core::ValueKind::U64)),
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
            let scalar = mech_core::Ref::new(7.0);
            let bare = LegacyValue::F64(scalar.clone());
            let mutable =
                LegacyValue::MutableReference(mech_core::Ref::new(LegacyValue::F64(scalar)));
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
        let scalar = mech_core::Ref::new(7.0);
        let bare = LegacyValue::F64(scalar.clone());
        let tuple = LegacyValue::Tuple(mech_core::Ref::new(mech_core::MechTuple::from_vec(vec![
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
            initialize_registers(&mut context, &registers);
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
            parsed.instructions[2],
            BytecodeInstruction::HostCall { requirement: 0, .. }
        ));
        assert!(matches!(
            parsed.instructions[3],
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
        let registers = allocate_registers(&mut context, 7);
        initialize_registers(&mut context, &registers);
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
        context.emit_resource_read(resource, registers[5]);
        context.emit_resource_write(resource, registers[5], registers[0]);
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
