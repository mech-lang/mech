#[cfg(feature = "no_std")]
use alloc::{format, string::String, vec, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{format, string::String, vec, vec::Vec};

use crate::{
    ApplicationRequirement, ExecutionHostFunctionRequest, ExecutionResourceRequest, FunctionArgs,
    FunctionCatalog, FunctionMatrixElement, FunctionValueRepresentation, LegacyValue, MResult,
    MechError, MechErrorKind, ResourceIntent, RuntimeFunctionId,
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

fn detached_resource_read_output(instruction: usize, value: LegacyValue) -> MResult<LegacyValue> {
    let value = value
        .try_deep_snapshot()
        .map_err(|error| violation_with_source(instruction, None, None, error))?;

    if matches!(value, LegacyValue::Empty) {
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
    pub output_seed: &'a LegacyValue,
    pub arguments: &'a [LegacyValue],
}

pub struct BytecodeResourceReadContract<'a> {
    pub instruction: u32,
    pub request: &'a ExecutionResourceRequest,
}

pub struct BytecodeResourceWriteContract<'a> {
    pub instruction: u32,
    pub request: &'a ExecutionResourceRequest,
    pub output_seed: &'a LegacyValue,
    pub source: &'a LegacyValue,
}

pub trait BytecodeExternalContractResolver {
    fn validate_host_call(
        &mut self,
        contract: BytecodeHostCallContract<'_>,
    ) -> MResult<LegacyValue>;

    /// Returns a detached concrete representative of the provider-owned first
    /// value for runtime-contract planning. The representative is ephemeral
    /// validation evidence: it is not serialized, interned as a constant, or
    /// included in program identity.
    fn validate_resource_read(
        &mut self,
        contract: BytecodeResourceReadContract<'_>,
    ) -> MResult<LegacyValue>;

    fn validate_resource_write(
        &mut self,
        contract: BytecodeResourceWriteContract<'_>,
    ) -> MResult<()>;
}

pub struct StructuralExternalContractResolver;

impl BytecodeExternalContractResolver for StructuralExternalContractResolver {
    fn validate_host_call(
        &mut self,
        contract: BytecodeHostCallContract<'_>,
    ) -> MResult<LegacyValue> {
        Ok(contract.output_seed.clone())
    }

    fn validate_resource_read(
        &mut self,
        contract: BytecodeResourceReadContract<'_>,
    ) -> MResult<LegacyValue> {
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
        if contract.output_seed == &LegacyValue::Empty {
            return Ok(());
        }
        Err(violation(
            contract.instruction as usize,
            None,
            None,
            format!(
                "resource write/send destination must have an Empty seed, found {:?}",
                contract.output_seed.kind(),
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
    current: &LegacyValue,
    incoming: &LegacyValue,
    reason: impl Into<String>,
) -> MechError {
    MechError::new(
        StableValueUpdateContractViolation {
            current: FunctionValueRepresentation::from_value(current),
            incoming: FunctionValueRepresentation::from_value(incoming),
            reason: reason.into(),
        },
        None,
    )
    .with_compiler_loc()
}

fn composite_schema_matches(current: &LegacyValue, incoming: &LegacyValue) -> bool {
    match (current, incoming) {
        #[cfg(feature = "record")]
        (LegacyValue::Record(current), LegacyValue::Record(incoming)) => {
            current.borrow().kind() == incoming.borrow().kind()
        }
        #[cfg(feature = "map")]
        (LegacyValue::Map(current), LegacyValue::Map(incoming)) => {
            let current = current.borrow();
            let incoming = incoming.borrow();
            current.key_kind == incoming.key_kind
                && current.value_kind == incoming.value_kind
                && current.num_elements == incoming.num_elements
                && current.map.len() == incoming.map.len()
                && current.map.keys().all(|key| incoming.map.contains_key(key))
        }
        #[cfg(feature = "set")]
        (LegacyValue::Set(current), LegacyValue::Set(incoming)) => {
            let current = current.borrow();
            let incoming = incoming.borrow();
            current.kind == incoming.kind && current.max_elements == incoming.max_elements
        }
        #[cfg(feature = "table")]
        (LegacyValue::Table(current), LegacyValue::Table(incoming)) => {
            let current = current.borrow();
            let incoming = incoming.borrow();
            current.rows == incoming.rows
                && current.cols == incoming.cols
                && current.data.len() == incoming.data.len()
                && current.data.iter().zip(incoming.data.iter()).all(
                    |((current_id, (current_kind, _)), (incoming_id, (incoming_kind, _)))| {
                        current_id == incoming_id
                            && current_kind == incoming_kind
                            && current.col_names.get(current_id)
                                == incoming.col_names.get(incoming_id)
                    },
                )
        }
        #[cfg(feature = "tuple")]
        (LegacyValue::Tuple(current), LegacyValue::Tuple(incoming)) => {
            current.borrow().kind() == incoming.borrow().kind()
        }
        _ => false,
    }
}

pub fn validate_stable_value_update(current: &LegacyValue, incoming: &LegacyValue) -> MResult<()> {
    if let (
        LegacyValue::Typed(current_inner, current_annotation),
        LegacyValue::Typed(incoming_inner, incoming_annotation),
    ) = (current, incoming)
    {
        if current_annotation != incoming_annotation {
            return Err(stable_update_violation(
                current,
                incoming,
                "typed annotations differ",
            ));
        }
        return validate_stable_value_update(current_inner, incoming_inner);
    }
    if matches!(current, LegacyValue::Typed(_, _)) || matches!(incoming, LegacyValue::Typed(_, _)) {
        return Err(stable_update_violation(
            current,
            incoming,
            "typed values are not implicitly unwrapped",
        ));
    }
    if matches!(current, LegacyValue::MutableReference(_))
        || matches!(incoming, LegacyValue::MutableReference(_))
    {
        return Err(stable_update_violation(
            current,
            incoming,
            "mutable references are not implicitly unwrapped",
        ));
    }
    if matches!(
        (current, incoming),
        (LegacyValue::Empty, LegacyValue::Empty)
    ) {
        return Ok(());
    }
    if matches!(current, LegacyValue::IndexAll) || matches!(incoming, LegacyValue::IndexAll) {
        return Err(stable_update_violation(
            current,
            incoming,
            "IndexAll is a selector and has no stable scalar backing",
        ));
    }

    let current_representation = FunctionValueRepresentation::from_value(current);
    let incoming_representation = FunctionValueRepresentation::from_value(incoming);
    if current_representation != incoming_representation {
        return Err(stable_update_violation(
            current,
            incoming,
            "the exact value backing, matrix element type, or matrix storage differs",
        ));
    }

    if matches!(
        current_representation,
        FunctionValueRepresentation::Matrix { .. }
    ) && current.shape() != incoming.shape()
    {
        return Err(stable_update_violation(
            current,
            incoming,
            format!(
                "matrix dimensions differ: current is {:?}, incoming is {:?}",
                current.shape(),
                incoming.shape(),
            ),
        ));
    }

    match current_representation {
        FunctionValueRepresentation::Record
        | FunctionValueRepresentation::Map
        | FunctionValueRepresentation::Set
        | FunctionValueRepresentation::Table
        | FunctionValueRepresentation::Tuple => {
            if !composite_schema_matches(current, incoming) {
                return Err(stable_update_violation(
                    current,
                    incoming,
                    "the composite semantic schema differs",
                ));
            }
        }
        FunctionValueRepresentation::Empty => {
            return Err(stable_update_violation(
                current,
                incoming,
                "only bare Empty values share the stable empty backing",
            ));
        }
        FunctionValueRepresentation::Id | FunctionValueRepresentation::Kind => {
            return Err(stable_update_violation(
                current,
                incoming,
                "the immediate value has no stable mutable backing",
            ));
        }
        FunctionValueRepresentation::Matrix {
            element: FunctionMatrixElement::Value,
            ..
        } => {
            return Err(stable_update_violation(
                current,
                incoming,
                "heterogeneous value matrices have no stable whole-value assignment",
            ));
        }
        FunctionValueRepresentation::AnyValue | FunctionValueRepresentation::MutableValueCell => {
            return Err(stable_update_violation(
                current,
                incoming,
                "the outer value representation is not stable-updateable",
            ));
        }
        _ => {}
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
        let mut registers = vec![None::<LegacyValue>; self.header.register_count as usize];

        for (instruction_index, instruction) in self.instructions.iter().enumerate() {
            if let BytecodeInstruction::ConstLoad { dst, constant } = instruction {
                let value = constants
                    .get(*constant as usize)
                    .ok_or_else(|| {
                        violation(
                            instruction_index,
                            None,
                            None,
                            format!("constant {constant} is out of range"),
                        )
                    })?
                    .try_deep_snapshot()
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
                            .cloned()
                            .ok_or_else(|| {
                                violation(
                                    instruction_index,
                                    None,
                                    None,
                                    format!("composite child register {child} has no seed"),
                                )
                            })
                    })
                    .collect::<MResult<Vec<_>>>()?;
                let value = crate::rebuild_bytecode_composite(template, children)
                    .map_err(|error| violation_with_source(instruction_index, None, None, error))?;
                registers[*dst as usize] = Some(value);
                continue;
            }

            let register = |index: u32| -> MResult<LegacyValue> {
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
                        FunctionArgs::Nullary(register(*dst)?),
                    )?;
                }
                BytecodeInstruction::RuntimeUnary { function, dst, src } => {
                    self.validate_runtime_instruction(
                        catalog,
                        instruction_index,
                        *function,
                        FunctionArgs::Unary(register(*dst)?, register(*src)?),
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
                        FunctionArgs::Binary(register(*dst)?, register(*lhs)?, register(*rhs)?),
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
                        FunctionArgs::Ternary(
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
                        FunctionArgs::Quaternary(
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
                        FunctionArgs::Variadic(register(*dst)?, arguments),
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
                    let output_seed = register(*dst)?;
                    let arguments = arguments
                        .iter()
                        .map(|argument| register(*argument))
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
                    registers[*dst as usize] = Some(planned);
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
                    *destination = Some(detached_resource_read_output(instruction_index, planned)?);
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
                    let output_seed = register(*dst)?;
                    let source = register(*src)?;
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
                    let output_seed = register(*dst)?;
                    let source = register(*src)?;
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
        args: FunctionArgs,
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
        entry.validate_args(&args).map_err(|error| {
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

#[cfg(all(test, feature = "f64"))]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;
    #[cfg(all(feature = "matrix2", feature = "matrixd"))]
    use crate::FunctionValueRepresentation;
    use crate::{
        ApplicationRequirement, BytecodeProgram, EncodedConstant, ExecutionHostFunctionRequest,
        ExecutionResourceRequest, FunctionArgumentRole, FunctionCatalogBuilder,
        FunctionRuntimeType, MechFunction, MechFunctionFactory, MechFunctionImpl, Ref,
        ResourceDelivery, RuntimeFunctionSignature, RuntimeType, ToValue, write_bytecode,
    };
    #[cfg(feature = "semantic-compiler")]
    use crate::{BytecodeCompilerContext, MechFunctionCompiler, Register};

    #[derive(Debug)]
    struct ExactF64Binary {
        out: Ref<f64>,
    }

    impl MechFunctionFactory for ExactF64Binary {
        const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
            <f64 as FunctionRuntimeType>::REPRESENTATION,
            <f64 as FunctionRuntimeType>::REPRESENTATION,
            <f64 as FunctionRuntimeType>::REPRESENTATION,
        );

        fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
            match args {
                FunctionArgs::Binary(out, lhs, rhs) => {
                    let out = out.try_function_ref(FunctionArgumentRole::Output)?;
                    let _: Ref<f64> = lhs.try_function_ref(FunctionArgumentRole::Input(0))?;
                    let _: Ref<f64> = rhs.try_function_ref(FunctionArgumentRole::Input(1))?;
                    Ok(Box::new(Self { out }))
                }
                _ => Err(MechError::new(
                    crate::IncorrectNumberOfArguments {
                        expected: 2,
                        found: args.len(),
                    },
                    None,
                )),
            }
        }
    }

    impl MechFunctionImpl for ExactF64Binary {
        fn solve_result(&self) -> MResult<()> {
            Ok(())
        }

        fn out(&self) -> LegacyValue {
            self.out.to_value()
        }

        fn to_string(&self) -> String {
            "ExactF64Binary".into()
        }

        fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
            Ok(self.reactive_output_values())
        }
    }

    #[cfg(feature = "semantic-compiler")]
    impl MechFunctionCompiler for ExactF64Binary {
        fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    fn scalar(runtime_type: RuntimeType, bytes: Vec<u8>) -> EncodedConstant {
        let alignment = if runtime_type == RuntimeType::I8 {
            1
        } else {
            8
        };
        EncodedConstant {
            runtime_type,
            alignment,
            bytes,
        }
    }

    fn parsed_runtime_program(
        constants: Vec<EncodedConstant>,
        instruction: BytecodeInstruction,
    ) -> ParsedProgram {
        let mut instructions = constants
            .iter()
            .enumerate()
            .map(|(register, _)| BytecodeInstruction::ConstLoad {
                dst: register as u32,
                constant: register as u32,
            })
            .collect::<Vec<_>>();
        instructions.push(instruction);
        instructions.push(BytecodeInstruction::Return { src: 0 });
        ParsedProgram::from_bytes(
            &write_bytecode(&BytecodeProgram {
                register_count: constants.len() as u32,
                constants,
                symbols: BTreeMap::new(),
                mutable_symbols: BTreeSet::new(),
                instructions,
                dictionary: BTreeMap::new(),
                requirements: Vec::new(),
            })
            .unwrap(),
        )
        .unwrap()
    }

    fn catalog() -> FunctionCatalog {
        let mut builder = FunctionCatalogBuilder::new();
        builder
            .insert_runtime_factory::<ExactF64Binary>(
                "ExactF64Binary",
                crate::RuntimeFunctionContract::no_matrix(
                    crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
                ),
            )
            .unwrap();
        builder.build().unwrap()
    }

    fn exact_read_request() -> ExecutionResourceRequest {
        ExecutionResourceRequest {
            base_uri: "test://provider/root".into(),
            path: "item".into(),
            context_name: "root".into(),
            operation: "read".into(),
            intent: ResourceIntent::Read,
            delivery: ResourceDelivery::Live,
        }
    }

    fn unseeded_resource_read_program() -> (ParsedProgram, Vec<u8>) {
        let bytes = write_bytecode(&BytecodeProgram {
            register_count: 1,
            constants: Vec::new(),
            symbols: BTreeMap::new(),
            mutable_symbols: BTreeSet::new(),
            instructions: vec![
                BytecodeInstruction::ResourceRead {
                    requirement: 0,
                    dst: 0,
                },
                BytecodeInstruction::Return { src: 0 },
            ],
            dictionary: BTreeMap::new(),
            requirements: vec![ApplicationRequirement::Resource(exact_read_request())],
        })
        .unwrap();
        (ParsedProgram::from_bytes(&bytes).unwrap(), bytes)
    }

    struct RecordingReadResolver {
        output: LegacyValue,
        calls: usize,
        requests: Vec<ExecutionResourceRequest>,
    }

    impl RecordingReadResolver {
        fn new(output: LegacyValue) -> Self {
            Self {
                output,
                calls: 0,
                requests: Vec::new(),
            }
        }
    }

    impl BytecodeExternalContractResolver for RecordingReadResolver {
        fn validate_host_call(
            &mut self,
            contract: BytecodeHostCallContract<'_>,
        ) -> MResult<LegacyValue> {
            Ok(contract.output_seed.clone())
        }

        fn validate_resource_read(
            &mut self,
            contract: BytecodeResourceReadContract<'_>,
        ) -> MResult<LegacyValue> {
            let BytecodeResourceReadContract {
                instruction: _,
                request,
            } = contract;
            self.calls += 1;
            self.requests.push(request.clone());
            Ok(self.output.clone())
        }

        fn validate_resource_write(
            &mut self,
            _contract: BytecodeResourceWriteContract<'_>,
        ) -> MResult<()> {
            Ok(())
        }
    }

    fn assert_contract_violation(program: &ParsedProgram, expected: &str) {
        let error = program.validate_runtime_contracts(&catalog()).unwrap_err();
        assert_eq!(error.kind_name(), "BytecodeRuntimeContractViolation");
        assert!(
            error.kind_message().contains(expected),
            "expected `{expected}` in `{}`",
            error.kind_message()
        );
    }

    #[cfg(all(feature = "matrix2", feature = "matrixd"))]
    fn f64_matrix_constant(storage: crate::MatrixStorage, rows: u32, cols: u32) -> EncodedConstant {
        f64_matrix_constant_with_offset(storage, rows, cols, 0)
    }

    #[cfg(all(feature = "matrix2", feature = "matrixd"))]
    fn f64_matrix_constant_with_offset(
        storage: crate::MatrixStorage,
        rows: u32,
        cols: u32,
        offset: u32,
    ) -> EncodedConstant {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&rows.to_le_bytes());
        bytes.extend_from_slice(&cols.to_le_bytes());
        for index in 0..rows.saturating_mul(cols) {
            let value = f64::from(index + 1 + offset);
            bytes.extend_from_slice(&value.to_bits().to_le_bytes());
        }
        EncodedConstant {
            runtime_type: RuntimeType::Matrix {
                element: Box::new(RuntimeType::F64),
                storage,
                rows,
                cols,
            },
            alignment: 8,
            bytes,
        }
    }

    #[cfg(all(feature = "matrix2", feature = "matrixd"))]
    struct FactoryMustNotRun;

    #[cfg(all(feature = "matrix2", feature = "matrixd"))]
    impl MechFunctionFactory for FactoryMustNotRun {
        const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
            FunctionValueRepresentation::AnyValue,
            FunctionValueRepresentation::AnyValue,
            FunctionValueRepresentation::AnyValue,
        );

        fn new(_args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
            panic!("shape and alias contracts must reject before invoking the factory")
        }
    }

    #[cfg(all(feature = "matrix2", feature = "matrixd", feature = "vectord"))]
    #[test]
    fn malicious_matrix_relations_and_output_aliases_fail_as_bytecode_contracts() {
        use crate::MatrixStorage;

        let mut catalog = FunctionCatalogBuilder::new();
        for (name, contract) in [
            (
                "AddMDMD<f64>",
                crate::RuntimeFunctionContract::same_shape(
                    crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
                ),
            ),
            (
                "AddM2M2<f64>",
                crate::RuntimeFunctionContract::same_shape(
                    crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
                ),
            ),
            (
                "MatMulMDMD<f64>",
                crate::RuntimeFunctionContract::matrix_product(
                    crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
                ),
            ),
            (
                "MatrixSolveMDVD<f64>",
                crate::RuntimeFunctionContract::linear_solve(
                    crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
                ),
            ),
        ] {
            catalog
                .insert_runtime_factory::<FactoryMustNotRun>(name, contract)
                .unwrap();
        }
        let catalog = catalog.build().unwrap();

        let dynamic = |rows, cols, offset| {
            f64_matrix_constant_with_offset(MatrixStorage::MatrixD, rows, cols, offset)
        };
        let fixed2 = |offset| f64_matrix_constant_with_offset(MatrixStorage::Matrix2, 2, 2, offset);
        let cases = [
            (
                "AddMDMD<f64>",
                vec![dynamic(2, 2, 0), dynamic(2, 2, 10), dynamic(3, 3, 0)],
                (0, 1, 2),
            ),
            (
                "AddMDMD<f64>",
                vec![dynamic(3, 3, 0), dynamic(2, 2, 0), dynamic(2, 2, 10)],
                (0, 1, 2),
            ),
            (
                "AddMDMD<f64>",
                vec![dynamic(2, 2, 0), dynamic(2, 2, 10)],
                (0, 0, 1),
            ),
            (
                "AddMDMD<f64>",
                vec![dynamic(2, 2, 0), dynamic(2, 2, 10)],
                (1, 0, 1),
            ),
            ("AddM2M2<f64>", vec![fixed2(0), fixed2(10)], (0, 0, 1)),
            (
                "MatMulMDMD<f64>",
                vec![dynamic(2, 4, 0), dynamic(2, 3, 0), dynamic(2, 4, 10)],
                (0, 1, 2),
            ),
            (
                "MatMulMDMD<f64>",
                vec![dynamic(3, 4, 0), dynamic(2, 3, 0), dynamic(3, 4, 10)],
                (0, 1, 2),
            ),
            (
                "MatrixSolveMDVD<f64>",
                vec![dynamic(3, 1, 0), dynamic(2, 3, 0), dynamic(3, 1, 10)],
                (0, 1, 2),
            ),
            (
                "MatrixSolveMDVD<f64>",
                vec![dynamic(2, 1, 0), dynamic(2, 2, 0), dynamic(3, 1, 0)],
                (0, 1, 2),
            ),
        ];

        for (name, constants, (dst, lhs, rhs)) in cases {
            let program = parsed_runtime_program(
                constants,
                BytecodeInstruction::RuntimeBinary {
                    function: RuntimeFunctionId::from_name(name).raw(),
                    dst,
                    lhs,
                    rhs,
                },
            );
            let error = program.validate_runtime_contracts(&catalog).unwrap_err();
            assert_eq!(
                error.kind_name(),
                "BytecodeRuntimeContractViolation",
                "{name}"
            );
            assert!(error.kind_message().contains(name));
        }
    }

    #[test]
    fn rejects_valid_runtime_id_with_wrong_scalar_inputs_and_output() {
        let function = RuntimeFunctionId::from_name("ExactF64Binary").raw();
        let f64_constant =
            |value: f64| scalar(RuntimeType::F64, value.to_bits().to_le_bytes().to_vec());
        let i8_constant = || scalar(RuntimeType::I8, vec![1]);

        let wrong_input = parsed_runtime_program(
            vec![f64_constant(1.0), i8_constant(), f64_constant(2.0)],
            BytecodeInstruction::RuntimeBinary {
                function,
                dst: 0,
                lhs: 1,
                rhs: 2,
            },
        );
        assert_contract_violation(&wrong_input, "Input(0)");

        let wrong_output = parsed_runtime_program(
            vec![i8_constant(), f64_constant(1.0), f64_constant(2.0)],
            BytecodeInstruction::RuntimeBinary {
                function,
                dst: 0,
                lhs: 1,
                rhs: 2,
            },
        );
        assert_contract_violation(&wrong_output, "Output");
    }

    #[test]
    fn rejects_opcode_and_factory_arity_mismatch() {
        let function = RuntimeFunctionId::from_name("ExactF64Binary").raw();
        let program = parsed_runtime_program(
            vec![
                scalar(RuntimeType::F64, 1.0_f64.to_bits().to_le_bytes().to_vec()),
                scalar(RuntimeType::F64, 2.0_f64.to_bits().to_le_bytes().to_vec()),
            ],
            BytecodeInstruction::RuntimeUnary {
                function,
                dst: 0,
                src: 1,
            },
        );
        assert_contract_violation(&program, "IncorrectNumberOfArguments");
    }

    #[test]
    fn external_opcodes_require_the_exact_requirement_kind_and_intent() {
        fn parsed(
            instruction: BytecodeInstruction,
            requirement: ApplicationRequirement,
        ) -> ParsedProgram {
            let resource_read = matches!(instruction, BytecodeInstruction::ResourceRead { .. });
            let (constants, mut instructions) = if resource_read {
                (
                    vec![EncodedConstant {
                        runtime_type: RuntimeType::F64,
                        alignment: 8,
                        bytes: 1.0_f64.to_bits().to_le_bytes().to_vec(),
                    }],
                    vec![BytecodeInstruction::ConstLoad {
                        dst: 1,
                        constant: 0,
                    }],
                )
            } else {
                (
                    vec![
                        EncodedConstant {
                            runtime_type: RuntimeType::Empty,
                            alignment: 1,
                            bytes: Vec::new(),
                        },
                        EncodedConstant {
                            runtime_type: RuntimeType::F64,
                            alignment: 8,
                            bytes: 1.0_f64.to_bits().to_le_bytes().to_vec(),
                        },
                    ],
                    vec![
                        BytecodeInstruction::ConstLoad {
                            dst: 0,
                            constant: 0,
                        },
                        BytecodeInstruction::ConstLoad {
                            dst: 1,
                            constant: 1,
                        },
                    ],
                )
            };
            instructions.push(instruction);
            instructions.push(BytecodeInstruction::Return { src: 0 });
            ParsedProgram::from_bytes(
                &write_bytecode(&BytecodeProgram {
                    register_count: 2,
                    constants,
                    symbols: BTreeMap::new(),
                    mutable_symbols: BTreeSet::new(),
                    instructions,
                    dictionary: BTreeMap::new(),
                    requirements: vec![requirement],
                })
                .unwrap(),
            )
            .unwrap()
        }

        fn resource(intent: ResourceIntent) -> ApplicationRequirement {
            ApplicationRequirement::Resource(ExecutionResourceRequest {
                base_uri: "test://provider".into(),
                path: "value".into(),
                context_name: "test".into(),
                operation: match intent {
                    ResourceIntent::Read => "read",
                    ResourceIntent::Assign => "write",
                    ResourceIntent::Send => "operate",
                }
                .into(),
                intent,
                delivery: ResourceDelivery::Snapshot,
            })
        }

        let empty_catalog = FunctionCatalog::empty();
        let cases = [
            (
                parsed(
                    BytecodeInstruction::HostCall {
                        requirement: 0,
                        dst: 0,
                        arguments: vec![1],
                    },
                    resource(ResourceIntent::Read),
                ),
                "HostFunction",
            ),
            (
                parsed(
                    BytecodeInstruction::ResourceRead {
                        requirement: 0,
                        dst: 0,
                    },
                    ApplicationRequirement::HostFunction(ExecutionHostFunctionRequest {
                        name: "test/host".into(),
                    }),
                ),
                "Read",
            ),
            (
                parsed(
                    BytecodeInstruction::ResourceWrite {
                        requirement: 0,
                        dst: 0,
                        src: 1,
                    },
                    resource(ResourceIntent::Send),
                ),
                "Assign",
            ),
            (
                parsed(
                    BytecodeInstruction::ResourceSend {
                        requirement: 0,
                        dst: 0,
                        src: 1,
                    },
                    resource(ResourceIntent::Assign),
                ),
                "Send",
            ),
        ];

        for (program, expected) in cases {
            let error = program
                .validate_runtime_contracts(&empty_catalog)
                .unwrap_err();
            assert_eq!(error.kind_name(), "BytecodeRuntimeContractViolation");
            assert!(error.kind_message().contains(expected));
        }
    }

    #[test]
    fn resource_writes_require_empty_output_seeds() {
        let program = ParsedProgram::from_bytes(
            &write_bytecode(&BytecodeProgram {
                register_count: 2,
                constants: vec![
                    EncodedConstant {
                        runtime_type: RuntimeType::F64,
                        alignment: 8,
                        bytes: 0.0_f64.to_bits().to_le_bytes().to_vec(),
                    },
                    EncodedConstant {
                        runtime_type: RuntimeType::String,
                        alignment: 1,
                        bytes: b"payload".to_vec(),
                    },
                ],
                symbols: BTreeMap::new(),
                mutable_symbols: BTreeSet::new(),
                instructions: vec![
                    BytecodeInstruction::ConstLoad {
                        dst: 0,
                        constant: 0,
                    },
                    BytecodeInstruction::ConstLoad {
                        dst: 1,
                        constant: 1,
                    },
                    BytecodeInstruction::ResourceSend {
                        requirement: 0,
                        dst: 0,
                        src: 1,
                    },
                    BytecodeInstruction::Return { src: 0 },
                ],
                dictionary: BTreeMap::new(),
                requirements: vec![ApplicationRequirement::Resource(ExecutionResourceRequest {
                    base_uri: "test://provider/output".into(),
                    path: "line".into(),
                    context_name: "output".into(),
                    operation: "write".into(),
                    intent: ResourceIntent::Send,
                    delivery: ResourceDelivery::Snapshot,
                })],
            })
            .unwrap(),
        )
        .unwrap();

        assert_contract_violation(&program, "must have an Empty seed");
    }

    #[cfg(all(feature = "matrix2", feature = "matrixd"))]
    #[test]
    fn stable_updates_require_exact_matrix_storage() {
        use crate::structures::matrix::Matrix as ValueMatrix;
        use nalgebra::{DMatrix, Matrix2};

        let fixed = LegacyValue::MatrixF64(ValueMatrix::Matrix2(Ref::new(Matrix2::identity())));
        let dynamic =
            LegacyValue::MatrixF64(ValueMatrix::DMatrix(Ref::new(DMatrix::identity(2, 2))));

        let error = validate_stable_value_update(&fixed, &dynamic).unwrap_err();
        assert_eq!(error.kind_name(), "StableValueUpdateContractViolation");
        assert!(error.kind_message().contains("matrix storage differs"));
    }

    #[cfg(feature = "matrixd")]
    #[test]
    fn stable_updates_reject_dynamic_matrix_dimension_changes() {
        use crate::structures::matrix::Matrix as ValueMatrix;
        use nalgebra::DMatrix;

        let current = LegacyValue::MatrixF64(ValueMatrix::DMatrix(Ref::new(DMatrix::zeros(2, 3))));
        let incoming = LegacyValue::MatrixF64(ValueMatrix::DMatrix(Ref::new(DMatrix::zeros(5, 7))));

        let error = validate_stable_value_update(&current, &incoming).unwrap_err();
        assert_eq!(error.kind_name(), "StableValueUpdateContractViolation");
        assert!(error.kind_message().contains("matrix dimensions differ"));
    }

    #[cfg(all(feature = "map", any(feature = "string", feature = "variable_define")))]
    #[test]
    fn stable_updates_reject_map_key_topology_changes() {
        let map = |key: &str| {
            LegacyValue::Map(Ref::new(crate::MechMap::from_typed_vec(
                crate::ValueKind::String,
                crate::ValueKind::F64,
                1,
                vec![(
                    LegacyValue::String(Ref::new(key.to_owned())),
                    LegacyValue::F64(Ref::new(1.0)),
                )],
            )))
        };

        let error = validate_stable_value_update(&map("before"), &map("after")).unwrap_err();
        assert_eq!(error.kind_name(), "StableValueUpdateContractViolation");
        assert!(error.kind_message().contains("composite semantic schema"));
    }

    #[test]
    fn stable_updates_do_not_implicitly_unwrap_typed_values() {
        let typed = LegacyValue::Typed(
            Box::new(LegacyValue::F64(Ref::new(1.0))),
            crate::ValueKind::F64,
        );
        let untyped = LegacyValue::F64(Ref::new(2.0));

        let error = validate_stable_value_update(&typed, &untyped).unwrap_err();
        assert_eq!(error.kind_name(), "StableValueUpdateContractViolation");
        assert!(error.kind_message().contains("not implicitly unwrapped"));
    }

    #[test]
    fn unseeded_resource_read_is_planned_from_external_resolver() {
        let (program, _) = unseeded_resource_read_program();
        let mut resolver = RecordingReadResolver::new(LegacyValue::F64(Ref::new(42.0)));

        program
            .validate_runtime_contracts_with(&FunctionCatalog::empty(), &mut resolver)
            .unwrap();

        assert_eq!(resolver.calls, 1);
        assert_eq!(resolver.requests, vec![exact_read_request()]);
    }

    #[test]
    fn resource_read_resolver_cannot_return_empty() {
        let (program, _) = unseeded_resource_read_program();
        let mut resolver = RecordingReadResolver::new(LegacyValue::Empty);

        let error = program
            .validate_runtime_contracts_with(&FunctionCatalog::empty(), &mut resolver)
            .unwrap_err();

        assert_eq!(error.kind_name(), "BytecodeRuntimeContractViolation");
        assert!(
            error
                .kind_message()
                .contains("first provider value must establish a concrete representation")
        );
    }

    #[test]
    fn structural_runtime_contract_validation_requires_resource_resolver() {
        let (program, _) = unseeded_resource_read_program();

        let error = program
            .validate_runtime_contracts(&FunctionCatalog::empty())
            .unwrap_err();
        assert_eq!(error.kind_name(), "BytecodeRuntimeContractViolation");
        assert!(
            error
                .kind_message()
                .contains("requires an external contract resolver")
        );

        let mut resolver = RecordingReadResolver::new(LegacyValue::F64(Ref::new(42.0)));
        program
            .validate_runtime_contracts_with(&FunctionCatalog::empty(), &mut resolver)
            .unwrap();
    }

    #[test]
    fn resource_read_planning_is_payload_independent() {
        let (program, original_bytes) = unseeded_resource_read_program();
        let mut first = RecordingReadResolver::new(LegacyValue::F64(Ref::new(1.0)));
        let mut second = RecordingReadResolver::new(LegacyValue::F64(Ref::new(91.0)));

        program
            .validate_runtime_contracts_with(&FunctionCatalog::empty(), &mut first)
            .unwrap();
        program
            .validate_runtime_contracts_with(&FunctionCatalog::empty(), &mut second)
            .unwrap();

        assert_eq!(first.calls, 1);
        assert_eq!(second.calls, 1);
        assert_eq!(
            write_bytecode(&BytecodeProgram {
                register_count: 1,
                constants: Vec::new(),
                symbols: BTreeMap::new(),
                mutable_symbols: BTreeSet::new(),
                instructions: vec![
                    BytecodeInstruction::ResourceRead {
                        requirement: 0,
                        dst: 0,
                    },
                    BytecodeInstruction::Return { src: 0 },
                ],
                dictionary: BTreeMap::new(),
                requirements: vec![ApplicationRequirement::Resource(exact_read_request())],
            })
            .unwrap(),
            original_bytes
        );
    }

    #[test]
    fn resource_read_contract_has_no_output_seed() {
        let request = exact_read_request();
        let contract = BytecodeResourceReadContract {
            instruction: 7,
            request: &request,
        };
        assert_eq!(contract.instruction, 7);
        assert_eq!(contract.request, &request);
    }

    #[cfg(feature = "matrix")]
    #[test]
    fn resource_read_resolver_result_must_be_stable_updateable() {
        use crate::structures::matrix::Matrix as ValueMatrix;
        use nalgebra::DVector;

        let unstable =
            LegacyValue::MatrixValue(ValueMatrix::DVector(Ref::new(DVector::from_vec(vec![
                LegacyValue::F64(Ref::new(1.0)),
            ]))));
        let (program, _) = unseeded_resource_read_program();
        let mut resolver = RecordingReadResolver::new(unstable);

        let error = program
            .validate_runtime_contracts_with(&FunctionCatalog::empty(), &mut resolver)
            .unwrap_err();

        assert_eq!(error.kind_name(), "BytecodeRuntimeContractViolation");
        assert!(
            error
                .full_chain_message()
                .contains("heterogeneous value matrices have no stable whole-value assignment")
        );
    }

    #[cfg(all(feature = "matrix2", feature = "matrixd"))]
    #[test]
    fn resource_read_result_validates_downstream_runtime_contract() {
        use crate::structures::matrix::Matrix as ValueMatrix;
        use nalgebra::DMatrix;

        #[derive(Debug)]
        struct PlanningMatrixBinary {
            output: LegacyValue,
        }

        impl MechFunctionFactory for PlanningMatrixBinary {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                FunctionValueRepresentation::AnyValue,
                FunctionValueRepresentation::AnyValue,
                FunctionValueRepresentation::AnyValue,
            );

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                let FunctionArgs::Binary(output, _, _) = args else {
                    return Err(MechError::new(
                        crate::IncorrectNumberOfArguments {
                            expected: 2,
                            found: args.len(),
                        },
                        None,
                    ));
                };
                Ok(Box::new(Self { output }))
            }
        }

        impl MechFunctionImpl for PlanningMatrixBinary {
            fn solve_result(&self) -> MResult<()> {
                Ok(())
            }

            fn out(&self) -> LegacyValue {
                self.output.clone()
            }

            fn to_string(&self) -> String {
                "PlanningMatrixBinary".into()
            }

            fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
                Ok(self.reactive_output_values())
            }
        }

        #[cfg(feature = "semantic-compiler")]
        impl MechFunctionCompiler for PlanningMatrixBinary {
            fn compile(&self, _context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                Ok(0)
            }
        }

        let function = RuntimeFunctionId::from_name("PlanningMatrixBinary").raw();
        let bytes = write_bytecode(&BytecodeProgram {
            register_count: 3,
            constants: vec![
                f64_matrix_constant(crate::MatrixStorage::MatrixD, 2, 1),
                f64_matrix_constant_with_offset(crate::MatrixStorage::MatrixD, 2, 1, 10),
            ],
            symbols: BTreeMap::new(),
            mutable_symbols: BTreeSet::new(),
            instructions: vec![
                BytecodeInstruction::ResourceRead {
                    requirement: 0,
                    dst: 0,
                },
                BytecodeInstruction::ConstLoad {
                    dst: 1,
                    constant: 0,
                },
                BytecodeInstruction::ConstLoad {
                    dst: 2,
                    constant: 1,
                },
                BytecodeInstruction::RuntimeBinary {
                    function,
                    dst: 1,
                    lhs: 0,
                    rhs: 2,
                },
                BytecodeInstruction::Return { src: 1 },
            ],
            dictionary: BTreeMap::new(),
            requirements: vec![ApplicationRequirement::Resource(exact_read_request())],
        })
        .unwrap();
        let program = ParsedProgram::from_bytes(&bytes).unwrap();
        let mut catalog = FunctionCatalogBuilder::new();
        catalog
            .insert_runtime_factory::<PlanningMatrixBinary>(
                "PlanningMatrixBinary",
                crate::RuntimeFunctionContract::same_shape(
                    crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
                ),
            )
            .unwrap();
        let catalog = catalog.build().unwrap();

        let mut matching = RecordingReadResolver::new(LegacyValue::MatrixF64(
            ValueMatrix::DMatrix(Ref::new(DMatrix::zeros(2, 1))),
        ));
        program
            .validate_runtime_contracts_with(&catalog, &mut matching)
            .unwrap();

        let mut incompatible = RecordingReadResolver::new(LegacyValue::MatrixF64(
            ValueMatrix::DMatrix(Ref::new(DMatrix::zeros(3, 1))),
        ));
        let error = program
            .validate_runtime_contracts_with(&catalog, &mut incompatible)
            .unwrap_err();
        assert_eq!(error.kind_name(), "BytecodeRuntimeContractViolation");
        assert!(error.kind_message().contains("shape"));
    }

    #[cfg(all(feature = "matrix2", feature = "matrixd"))]
    #[test]
    fn rejects_fixed_and_dynamic_matrix_storage_mismatch() {
        use crate::structures::matrix::Matrix as ValueMatrix;
        use nalgebra::Matrix2;

        #[derive(Debug)]
        struct ExactMatrix2Binary {
            out: Ref<Matrix2<f64>>,
        }
        impl MechFunctionFactory for ExactMatrix2Binary {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <Matrix2<f64> as FunctionRuntimeType>::REPRESENTATION,
                <Matrix2<f64> as FunctionRuntimeType>::REPRESENTATION,
                <Matrix2<f64> as FunctionRuntimeType>::REPRESENTATION,
            );

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                match args {
                    FunctionArgs::Binary(out, lhs, rhs) => {
                        let out = out.try_function_ref(FunctionArgumentRole::Output)?;
                        let _: Ref<Matrix2<f64>> =
                            lhs.try_function_ref(FunctionArgumentRole::Input(0))?;
                        let _: Ref<Matrix2<f64>> =
                            rhs.try_function_ref(FunctionArgumentRole::Input(1))?;
                        Ok(Box::new(Self { out }))
                    }
                    _ => unreachable!(),
                }
            }
        }
        impl MechFunctionImpl for ExactMatrix2Binary {
            fn solve_result(&self) -> MResult<()> {
                Ok(())
            }
            fn out(&self) -> LegacyValue {
                LegacyValue::MatrixF64(ValueMatrix::Matrix2(self.out.clone()))
            }
            fn to_string(&self) -> String {
                "ExactMatrix2Binary".into()
            }
            fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
                Ok(self.reactive_output_values())
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl MechFunctionCompiler for ExactMatrix2Binary {
            fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                Ok(0)
            }
        }

        const NAME: &str = "ExactMatrix2Binary";
        let function = RuntimeFunctionId::from_name(NAME).raw();
        let program = parsed_runtime_program(
            vec![
                f64_matrix_constant_with_offset(crate::MatrixStorage::Matrix2, 2, 2, 0),
                f64_matrix_constant(crate::MatrixStorage::MatrixD, 2, 2),
                f64_matrix_constant_with_offset(crate::MatrixStorage::Matrix2, 2, 2, 10),
            ],
            BytecodeInstruction::RuntimeBinary {
                function,
                dst: 0,
                lhs: 1,
                rhs: 2,
            },
        );
        let mut builder = FunctionCatalogBuilder::new();
        builder
            .insert_runtime_factory::<ExactMatrix2Binary>(
                NAME,
                crate::RuntimeFunctionContract::same_shape(
                    crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
                ),
            )
            .unwrap();
        let error = program
            .validate_runtime_contracts(&builder.build().unwrap())
            .unwrap_err();
        assert_eq!(error.kind_name(), "BytecodeRuntimeContractViolation");
        assert!(error.kind_message().contains("Input(0)"));
    }
}
