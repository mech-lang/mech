#[cfg(feature = "no_std")]
use alloc::{format, string::String, vec, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{format, string::String, vec, vec::Vec};

use crate::{
    ApplicationRequirement, ExecutionHostFunctionRequest, ExecutionResourceRequest,
    FunctionCatalog, FunctionInvocation, FunctionValueRepresentation, MResult, MechError,
    MechErrorKind, ResourceIntent, RuntimeFunctionId, Value, ValueCell, ValueData,
};

use super::{BytecodeInstruction, ParsedProgram};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BytecodeRuntimeContractViolation {
    pub instruction: u32,
    pub function_id: Option<u64>,
    pub function_name: Option<String>,
    pub reason: String,
}

impl MechErrorKind for BytecodeRuntimeContractViolation {
    fn name(&self) -> &str {
        "BytecodeRuntimeContractViolation"
    }

    fn message(&self) -> String {
        let function = match (&self.function_name, self.function_id) {
            (Some(name), Some(id)) => format!(" for runtime function {name} (0x{id:016x})"),
            (None, Some(id)) => format!(" for runtime function 0x{id:016x}"),
            _ => String::new(),
        };
        format!(
            "bytecode instruction {}{} violates its runtime contract: {}",
            self.instruction, function, self.reason
        )
    }
}

fn violation(
    instruction: usize,
    function_id: Option<u64>,
    function_name: Option<String>,
    reason: impl Into<String>,
) -> MechError {
    MechError::new(
        BytecodeRuntimeContractViolation {
            instruction: u32::try_from(instruction).unwrap_or(u32::MAX),
            function_id,
            function_name,
            reason: reason.into(),
        },
        None,
    )
    .with_compiler_loc()
}

fn violation_with_source(
    instruction: usize,
    function_id: Option<u64>,
    function_name: Option<String>,
    source: MechError,
) -> MechError {
    violation(
        instruction,
        function_id,
        function_name,
        source.simple_message(),
    )
    .with_source(source)
}

fn detached_resource_read_output(instruction: usize, value: Value) -> MResult<Value> {
    if matches!(value.data(), ValueData::Tuple(values) if values.is_empty()) {
        return Err(violation(
            instruction,
            None,
            None,
            "ResourceRead resolver returned Empty; the first provider value must establish a concrete representation",
        ));
    }

    validate_stable_value_update(&value, &value)
        .map_err(|error| violation_with_source(instruction, None, None, error))?;
    Ok(value)
}

pub struct BytecodeHostCallContract<'a> {
    pub instruction: u32,
    pub request: &'a ExecutionHostFunctionRequest,
    pub output_seed: &'a Value,
    pub arguments: &'a [Value],
}

pub struct BytecodeResourceReadContract<'a> {
    pub instruction: u32,
    pub request: &'a ExecutionResourceRequest,
}

pub struct BytecodeResourceWriteContract<'a> {
    pub instruction: u32,
    pub request: &'a ExecutionResourceRequest,
    pub output_seed: &'a Value,
    pub source: &'a Value,
}

pub trait BytecodeExternalContractResolver {
    fn validate_host_call(&mut self, contract: BytecodeHostCallContract<'_>) -> MResult<Value>;

    /// Returns a detached concrete representative of the provider-owned first
    /// value for runtime-contract planning. The representative is ephemeral
    /// validation evidence: it is not serialized, interned as a constant, or
    /// included in program identity.
    fn validate_resource_read(
        &mut self,
        contract: BytecodeResourceReadContract<'_>,
    ) -> MResult<Value>;

    fn validate_resource_write(
        &mut self,
        contract: BytecodeResourceWriteContract<'_>,
    ) -> MResult<()>;
}

pub struct StructuralExternalContractResolver;

impl BytecodeExternalContractResolver for StructuralExternalContractResolver {
    fn validate_host_call(&mut self, contract: BytecodeHostCallContract<'_>) -> MResult<Value> {
        Ok(contract.output_seed.clone())
    }

    fn validate_resource_read(
        &mut self,
        contract: BytecodeResourceReadContract<'_>,
    ) -> MResult<Value> {
        Err(violation(
            contract.instruction as usize,
            None,
            None,
            format!(
                "ResourceRead for {:?} requires an external contract resolver to provide the provider-owned output representation",
                contract.request,
            ),
        ))
    }

    fn validate_resource_write(
        &mut self,
        contract: BytecodeResourceWriteContract<'_>,
    ) -> MResult<()> {
        if matches!(contract.output_seed.data(), ValueData::Tuple(values) if values.is_empty()) {
            return Ok(());
        }
        Err(violation(
            contract.instruction as usize,
            None,
            None,
            format!(
                "resource write/send destination must have an Empty seed, found {:?}",
                contract.output_seed.data().kind(),
            ),
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StableValueUpdateContractViolation {
    pub current: FunctionValueRepresentation,
    pub incoming: FunctionValueRepresentation,
    pub reason: String,
}

impl MechErrorKind for StableValueUpdateContractViolation {
    fn name(&self) -> &str {
        "StableValueUpdateContractViolation"
    }

    fn message(&self) -> String {
        format!(
            "stable value update from {:?} to {:?} violates its contract: {}",
            self.current, self.incoming, self.reason,
        )
    }
}

fn stable_update_violation(
    current: FunctionValueRepresentation,
    incoming: FunctionValueRepresentation,
    reason: impl Into<String>,
) -> MechError {
    MechError::new(
        StableValueUpdateContractViolation {
            current,
            incoming,
            reason: reason.into(),
        },
        None,
    )
    .with_compiler_loc()
}

pub fn validate_stable_value_update(current: &Value, incoming: &Value) -> MResult<()> {
    let current_representation = ValueCell::from_snapshot(current.clone())?.representation();
    let incoming_representation = ValueCell::from_snapshot(incoming.clone())?.representation();
    if current_representation != incoming_representation {
        return Err(stable_update_violation(
            current_representation,
            incoming_representation,
            "the canonical runtime representation differs",
        ));
    }
    if current.schema_key() != incoming.schema_key() {
        return Err(stable_update_violation(
            current_representation,
            incoming_representation,
            "the canonical semantic schema differs",
        ));
    }
    if current.shape() != incoming.shape() {
        return Err(stable_update_violation(
            current_representation,
            incoming_representation,
            "the canonical shape differs",
        ));
    }
    Ok(())
}

impl ParsedProgram {
    /// Validates catalog-aware bytecode contracts without mutating interpreter
    /// or runtime state. Programs containing `ResourceRead` require
    /// `validate_runtime_contracts_with` and a trusted resolver that supplies
    /// the provider-owned output representation.
    pub fn validate_runtime_contracts(&self, catalog: &FunctionCatalog) -> MResult<()> {
        let mut resolver = StructuralExternalContractResolver;
        self.validate_runtime_contracts_with(catalog, &mut resolver)
    }

    pub fn validate_runtime_contracts_with<R>(
        &self,
        catalog: &FunctionCatalog,
        resolver: &mut R,
    ) -> MResult<()>
    where
        R: BytecodeExternalContractResolver,
    {
        let constants = self
            .decode_constants()
            .map_err(|error| violation_with_source(0, None, None, error))?;
        let constant_cells = self
            .decode_constant_cells()
            .map_err(|error| violation_with_source(0, None, None, error))?;
        let mut registers = vec![None::<ValueCell>; self.header.register_count as usize];

        for (instruction_index, instruction) in self.instructions.iter().enumerate() {
            if let BytecodeInstruction::ConstLoad { dst, constant } = instruction {
                let value = constant_cells
                    .get(*constant as usize)
                    .ok_or_else(|| {
                        violation(
                            instruction_index,
                            None,
                            None,
                            format!("constant {constant} is out of range"),
                        )
                    })?
                    .detached_clone()
                    .map_err(|error| violation_with_source(instruction_index, None, None, error))?;
                let destination = registers.get_mut(*dst as usize).ok_or_else(|| {
                    violation(
                        instruction_index,
                        None,
                        None,
                        format!("register {dst} is out of range"),
                    )
                })?;
                *destination = Some(value);
                continue;
            }

            if let BytecodeInstruction::CompositePack {
                dst,
                template,
                children,
            } = instruction
            {
                let template = constants.get(*template as usize).ok_or_else(|| {
                    violation(
                        instruction_index,
                        None,
                        None,
                        format!("composite template {template} is out of range"),
                    )
                })?;
                let children = children
                    .iter()
                    .map(|child| {
                        registers
                            .get(*child as usize)
                            .and_then(Option::as_ref)
                            .ok_or_else(|| {
                                violation(
                                    instruction_index,
                                    None,
                                    None,
                                    format!("composite child register {child} has no seed"),
                                )
                            })?
                            .snapshot()
                            .map_err(|error| {
                                violation_with_source(instruction_index, None, None, error)
                            })
                    })
                    .collect::<MResult<Vec<_>>>()?;
                let value = crate::rebuild_canonical_bytecode_composite(template, children)
                    .map_err(|error| violation_with_source(instruction_index, None, None, error))?;
                registers[*dst as usize] =
                    Some(ValueCell::from_snapshot(value).map_err(|error| {
                        violation_with_source(instruction_index, None, None, error)
                    })?);
                continue;
            }

            let register = |index: u32| -> MResult<ValueCell> {
                registers
                    .get(index as usize)
                    .and_then(Option::as_ref)
                    .cloned()
                    .ok_or_else(|| {
                        violation(
                            instruction_index,
                            instruction.runtime_function(),
                            None,
                            format!("register {index} has no detached planning value"),
                        )
                    })
            };

            match instruction {
                BytecodeInstruction::ConstLoad { .. }
                | BytecodeInstruction::CompositePack { .. } => unreachable!(),
                BytecodeInstruction::RuntimeNullary { function, dst } => {
                    self.validate_runtime_instruction(
                        catalog,
                        instruction_index,
                        *function,
                        FunctionInvocation::nullary(register(*dst)?),
                    )?;
                }
                BytecodeInstruction::RuntimeUnary { function, dst, src } => {
                    self.validate_runtime_instruction(
                        catalog,
                        instruction_index,
                        *function,
                        FunctionInvocation::unary(register(*dst)?, register(*src)?),
                    )?;
                }
                BytecodeInstruction::RuntimeBinary {
                    function,
                    dst,
                    lhs,
                    rhs,
                } => {
                    self.validate_runtime_instruction(
                        catalog,
                        instruction_index,
                        *function,
                        FunctionInvocation::binary(
                            register(*dst)?,
                            register(*lhs)?,
                            register(*rhs)?,
                        ),
                    )?;
                }
                BytecodeInstruction::RuntimeTernary {
                    function,
                    dst,
                    a,
                    b,
                    c,
                } => {
                    self.validate_runtime_instruction(
                        catalog,
                        instruction_index,
                        *function,
                        FunctionInvocation::ternary(
                            register(*dst)?,
                            register(*a)?,
                            register(*b)?,
                            register(*c)?,
                        ),
                    )?;
                }
                BytecodeInstruction::RuntimeQuaternary {
                    function,
                    dst,
                    a,
                    b,
                    c,
                    d,
                } => {
                    self.validate_runtime_instruction(
                        catalog,
                        instruction_index,
                        *function,
                        FunctionInvocation::quaternary(
                            register(*dst)?,
                            register(*a)?,
                            register(*b)?,
                            register(*c)?,
                            register(*d)?,
                        ),
                    )?;
                }
                BytecodeInstruction::RuntimeVariadic {
                    function,
                    dst,
                    arguments,
                } => {
                    let arguments = arguments
                        .iter()
                        .map(|argument| register(*argument))
                        .collect::<MResult<Vec<_>>>()?;
                    self.validate_runtime_instruction(
                        catalog,
                        instruction_index,
                        *function,
                        FunctionInvocation::variadic(register(*dst)?, arguments.into_boxed_slice()),
                    )?;
                }
                BytecodeInstruction::HostCall {
                    requirement,
                    dst,
                    arguments,
                } => {
                    let request = match self.requirements.get(*requirement as usize) {
                        Some(ApplicationRequirement::HostFunction(request)) => request,
                        _ => {
                            return Err(violation(
                                instruction_index,
                                None,
                                None,
                                format!(
                                    "HostCall requirement {requirement} must be a HostFunction"
                                ),
                            ));
                        }
                    };
                    let output_seed = register(*dst)?.snapshot().map_err(|error| {
                        violation_with_source(instruction_index, None, None, error)
                    })?;
                    let arguments = arguments
                        .iter()
                        .map(|argument| {
                            register(*argument)?.snapshot().map_err(|error| {
                                violation_with_source(instruction_index, None, None, error)
                            })
                        })
                        .collect::<MResult<Vec<_>>>()?;
                    let planned = resolver.validate_host_call(BytecodeHostCallContract {
                        instruction: u32::try_from(instruction_index).unwrap_or(u32::MAX),
                        request,
                        output_seed: &output_seed,
                        arguments: &arguments,
                    })?;
                    validate_stable_value_update(&output_seed, &planned).map_err(|error| {
                        violation_with_source(instruction_index, None, None, error)
                    })?;
                    registers[*dst as usize] =
                        Some(ValueCell::from_snapshot(planned).map_err(|error| {
                            violation_with_source(instruction_index, None, None, error)
                        })?);
                }
                BytecodeInstruction::ResourceRead { requirement, dst } => {
                    let request = self.resource_requirement(
                        instruction_index,
                        *requirement,
                        ResourceIntent::Read,
                    )?;
                    let destination = registers.get_mut(*dst as usize).ok_or_else(|| {
                        violation(
                            instruction_index,
                            None,
                            None,
                            format!("register {dst} is out of range"),
                        )
                    })?;
                    if destination.is_some() {
                        return Err(violation(
                            instruction_index,
                            None,
                            None,
                            format!(
                                "ResourceRead destination register {dst} already has a planned value"
                            ),
                        ));
                    }
                    let planned =
                        resolver.validate_resource_read(BytecodeResourceReadContract {
                            instruction: u32::try_from(instruction_index).unwrap_or(u32::MAX),
                            request,
                        })?;
                    *destination = Some(
                        ValueCell::from_snapshot(detached_resource_read_output(
                            instruction_index,
                            planned,
                        )?)
                        .map_err(|error| {
                            violation_with_source(instruction_index, None, None, error)
                        })?,
                    );
                }
                BytecodeInstruction::ResourceWrite {
                    requirement,
                    dst,
                    src,
                } => {
                    let request = self.resource_requirement(
                        instruction_index,
                        *requirement,
                        ResourceIntent::Assign,
                    )?;
                    let output_seed = register(*dst)?.snapshot().map_err(|error| {
                        violation_with_source(instruction_index, None, None, error)
                    })?;
                    let source = register(*src)?.snapshot().map_err(|error| {
                        violation_with_source(instruction_index, None, None, error)
                    })?;
                    resolver.validate_resource_write(BytecodeResourceWriteContract {
                        instruction: u32::try_from(instruction_index).unwrap_or(u32::MAX),
                        request,
                        output_seed: &output_seed,
                        source: &source,
                    })?;
                }
                BytecodeInstruction::ResourceSend {
                    requirement,
                    dst,
                    src,
                } => {
                    let request = self.resource_requirement(
                        instruction_index,
                        *requirement,
                        ResourceIntent::Send,
                    )?;
                    let output_seed = register(*dst)?.snapshot().map_err(|error| {
                        violation_with_source(instruction_index, None, None, error)
                    })?;
                    let source = register(*src)?.snapshot().map_err(|error| {
                        violation_with_source(instruction_index, None, None, error)
                    })?;
                    resolver.validate_resource_write(BytecodeResourceWriteContract {
                        instruction: u32::try_from(instruction_index).unwrap_or(u32::MAX),
                        request,
                        output_seed: &output_seed,
                        source: &source,
                    })?;
                }
                BytecodeInstruction::Return { src } => {
                    register(*src)?;
                }
            }
        }
        Ok(())
    }

    fn validate_runtime_instruction(
        &self,
        catalog: &FunctionCatalog,
        instruction: usize,
        function: u64,
        invocation: FunctionInvocation,
    ) -> MResult<()> {
        let id = RuntimeFunctionId::from_raw(function);
        let entry = catalog.runtime_entry(id).ok_or_else(|| {
            violation(
                instruction,
                Some(function),
                None,
                "runtime function is absent from the trusted catalog",
            )
        })?;
        entry.validate_invocation(&invocation).map_err(|error| {
            violation_with_source(instruction, Some(function), Some(entry.name.clone()), error)
        })
    }

    fn resource_requirement(
        &self,
        instruction: usize,
        requirement: u32,
        expected: ResourceIntent,
    ) -> MResult<&ExecutionResourceRequest> {
        match self.requirements.get(requirement as usize) {
            Some(ApplicationRequirement::Resource(request)) if request.intent == expected => {
                Ok(request)
            }
            actual => Err(violation(
                instruction,
                None,
                None,
                format!(
                    "resource requirement {requirement} must have intent {expected:?}, found {actual:?}"
                ),
            )),
        }
    }
}
