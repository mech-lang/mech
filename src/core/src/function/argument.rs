#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec,
    vec::Vec,
};
#[cfg(not(feature = "no_std"))]
use std::string::{String, ToString};

use core::{any::type_name, fmt};

use crate::FunctionMatrixStoragePattern;
#[cfg(feature = "semantic-compiler")]
use crate::{BytecodeCompilerContext, Register};
use crate::{
    CanonicalCellId, FunctionArgumentRole, FunctionMatrixRepresentation, FunctionRuntimeType,
    FunctionSignatureViolation, FunctionValueRepresentation, IncorrectNumberOfArguments, MResult,
    ManagedPort, MechError, MechErrorKind, OperationContractDeclaration, OperationContractError,
    PortMemoryRequirement, PortStorageCompatibilityError, Ref, RuntimeFunctionContract,
    RuntimeFunctionInputs, RuntimeFunctionSignature, RuntimeOutputAliasPolicy, SchemaBody,
    SchemaId, ShapeInstance, StorageTopology, Value, ValueCell, ValueData, ValueDataDraft,
};

mod function_port_backing {
    pub trait Sealed {}
}

/// An exact runtime backing type that may be extracted through a function port.
///
/// This sealed marker deliberately excludes erased universal values,
/// [`crate::ValueCell`], aggregate wrappers, and reference wrappers around
/// those types.
///
/// ```compile_fail
/// use mech_core::{FunctionPortBacking, ValueCell};
/// fn require<T: FunctionPortBacking>() {}
/// require::<ValueCell>();
/// ```
///
/// ```compile_fail
/// use mech_core::{matrix::Matrix, FunctionPortBacking};
/// fn require<T: FunctionPortBacking>() {}
/// require::<Matrix<f64>>();
/// ```
pub trait FunctionPortBacking:
    function_port_backing::Sealed + FunctionRuntimeType + 'static
{
}

impl<T> FunctionPortBacking for T where
    T: function_port_backing::Sealed + FunctionRuntimeType + 'static
{
}

macro_rules! scalar_function_port_backing {
    ($type:ty, $feature:literal) => {
        #[cfg(feature = $feature)]
        impl function_port_backing::Sealed for $type {}
    };
}

scalar_function_port_backing!(u8, "u8");
scalar_function_port_backing!(u16, "u16");
scalar_function_port_backing!(u32, "u32");
scalar_function_port_backing!(u64, "u64");
scalar_function_port_backing!(u128, "u128");
scalar_function_port_backing!(i8, "i8");
scalar_function_port_backing!(i16, "i16");
scalar_function_port_backing!(i32, "i32");
scalar_function_port_backing!(i64, "i64");
scalar_function_port_backing!(i128, "i128");
scalar_function_port_backing!(f32, "f32");
scalar_function_port_backing!(f64, "f64");
scalar_function_port_backing!(bool, "bool");
scalar_function_port_backing!(String, "string");

impl function_port_backing::Sealed for usize {}

#[cfg(feature = "complex")]
impl function_port_backing::Sealed for crate::C64 {}

#[cfg(feature = "rational")]
impl function_port_backing::Sealed for crate::R64 {}

macro_rules! exact_matrix_function_port_backing {
    ($type:ident, $feature:literal) => {
        #[cfg(feature = $feature)]
        impl<T: FunctionPortBacking> function_port_backing::Sealed for crate::$type<T> {}
    };
}

exact_matrix_function_port_backing!(Matrix1, "matrix1");
exact_matrix_function_port_backing!(Matrix2, "matrix2");
exact_matrix_function_port_backing!(Matrix3, "matrix3");
exact_matrix_function_port_backing!(Matrix4, "matrix4");
exact_matrix_function_port_backing!(Matrix2x3, "matrix2x3");
exact_matrix_function_port_backing!(Matrix3x2, "matrix3x2");
exact_matrix_function_port_backing!(RowVector2, "row_vector2");
exact_matrix_function_port_backing!(RowVector3, "row_vector3");
exact_matrix_function_port_backing!(RowVector4, "row_vector4");
exact_matrix_function_port_backing!(RowDVector, "row_vectord");
exact_matrix_function_port_backing!(Vector2, "vector2");
exact_matrix_function_port_backing!(Vector3, "vector3");
exact_matrix_function_port_backing!(Vector4, "vector4");
exact_matrix_function_port_backing!(DVector, "vectord");
exact_matrix_function_port_backing!(DMatrix, "matrixd");

#[derive(Clone)]
pub struct FunctionInvocation {
    layout: FunctionInvocationLayout,
    output: ValueCell,
    inputs: Box<[ValueCell]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FunctionInvocationLayout {
    Nullary,
    Unary,
    Binary,
    Ternary,
    Quaternary,
    Variadic,
}

#[derive(Clone, Copy)]
pub struct FunctionInputPort<'a> {
    invocation: &'a FunctionInvocation,
    index: usize,
}

#[derive(Clone, Copy)]
pub struct FunctionOutputPort<'a> {
    invocation: &'a FunctionInvocation,
}

pub struct FunctionInputPorts<'a> {
    invocation: &'a FunctionInvocation,
    next: usize,
}

impl FunctionInvocation {
    pub fn nullary(output: ValueCell) -> Self {
        Self::from_cells(
            FunctionInvocationLayout::Nullary,
            output,
            Vec::new().into_boxed_slice(),
        )
        .expect("nullary invocation layout is valid")
    }

    pub fn unary(output: ValueCell, input: ValueCell) -> Self {
        Self::from_cells(
            FunctionInvocationLayout::Unary,
            output,
            vec![input].into_boxed_slice(),
        )
        .expect("unary invocation layout is valid")
    }

    pub fn binary(output: ValueCell, first: ValueCell, second: ValueCell) -> Self {
        Self::from_cells(
            FunctionInvocationLayout::Binary,
            output,
            vec![first, second].into_boxed_slice(),
        )
        .expect("binary invocation layout is valid")
    }

    pub fn ternary(
        output: ValueCell,
        first: ValueCell,
        second: ValueCell,
        third: ValueCell,
    ) -> Self {
        Self::from_cells(
            FunctionInvocationLayout::Ternary,
            output,
            vec![first, second, third].into_boxed_slice(),
        )
        .expect("ternary invocation layout is valid")
    }

    pub fn quaternary(
        output: ValueCell,
        first: ValueCell,
        second: ValueCell,
        third: ValueCell,
        fourth: ValueCell,
    ) -> Self {
        Self::from_cells(
            FunctionInvocationLayout::Quaternary,
            output,
            vec![first, second, third, fourth].into_boxed_slice(),
        )
        .expect("quaternary invocation layout is valid")
    }

    pub fn variadic(output: ValueCell, inputs: Box<[ValueCell]>) -> Self {
        Self::from_cells(FunctionInvocationLayout::Variadic, output, inputs)
            .expect("variadic invocation layout is valid")
    }

    pub fn from_cells(
        layout: FunctionInvocationLayout,
        output: ValueCell,
        inputs: Box<[ValueCell]>,
    ) -> MResult<Self> {
        let expected = match layout {
            FunctionInvocationLayout::Nullary => Some(0),
            FunctionInvocationLayout::Unary => Some(1),
            FunctionInvocationLayout::Binary => Some(2),
            FunctionInvocationLayout::Ternary => Some(3),
            FunctionInvocationLayout::Quaternary => Some(4),
            FunctionInvocationLayout::Variadic => None,
        };
        if expected.is_some_and(|expected| expected != inputs.len()) {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: expected.expect("fixed invocation layout"),
                    found: inputs.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        Ok(Self {
            layout,
            output,
            inputs,
        })
    }

    pub fn input_count(&self) -> usize {
        self.inputs.len()
    }

    /// Resolves the semantic input ordinal used by a call-memory plan. A
    /// read-modify-write output may be coalesced out of the physical runtime
    /// invocation while remaining an explicit semantic input to planning.
    pub(crate) fn planned_input_cell<'a>(
        &'a self,
        plan: &crate::CallMemoryPlan,
        input: usize,
    ) -> MResult<&'a ValueCell> {
        if plan.inputs.len() == self.inputs.len() {
            return self
                .inputs
                .get(input)
                .ok_or_else(|| self.layout_error(input + 1));
        }
        if plan.inputs.len() != self.inputs.len().saturating_add(1) {
            return Err(function_memory_contract_error(
                FunctionMemoryContractViolationReason::OperationContractDerivation {
                    error: crate::OperationContractError::PortCountMismatch {
                        direction: crate::PortDirection::Input,
                        expected: plan.inputs.len() as u64,
                        actual: self.inputs.len() as u64,
                    },
                },
            ));
        }
        let base = plan
            .bound_call
            .operation_descriptor()
            .contract
            .outputs
            .iter()
            .find_map(|output| match output.construction {
                crate::OutputConstruction::ReadModifyWrite { base_input, .. } => {
                    Some(base_input as usize)
                }
                _ => None,
            })
            .ok_or_else(|| self.layout_error(plan.inputs.len()))?;
        if input == base {
            return Ok(&self.output);
        }
        let physical = input - usize::from(input > base);
        self.inputs
            .get(physical)
            .ok_or_else(|| self.layout_error(plan.inputs.len()))
    }

    pub fn output(&self) -> FunctionOutputPort<'_> {
        FunctionOutputPort { invocation: self }
    }

    pub fn input(&self, index: usize) -> Option<FunctionInputPort<'_>> {
        self.inputs.get(index).map(|_| FunctionInputPort {
            invocation: self,
            index,
        })
    }

    pub fn inputs(&self) -> FunctionInputPorts<'_> {
        FunctionInputPorts {
            invocation: self,
            next: 0,
        }
    }

    pub fn expect_nullary(&self) -> MResult<FunctionOutputPort<'_>> {
        if self.layout == FunctionInvocationLayout::Nullary {
            Ok(self.output())
        } else {
            Err(self.layout_error(0))
        }
    }

    pub fn expect_unary(&self) -> MResult<(FunctionOutputPort<'_>, FunctionInputPort<'_>)> {
        if self.layout == FunctionInvocationLayout::Unary {
            Ok((self.output(), self.input(0).expect("unary input")))
        } else {
            Err(self.layout_error(1))
        }
    }

    pub fn expect_binary(
        &self,
    ) -> MResult<(
        FunctionOutputPort<'_>,
        FunctionInputPort<'_>,
        FunctionInputPort<'_>,
    )> {
        if self.layout == FunctionInvocationLayout::Binary {
            Ok((
                self.output(),
                self.input(0).expect("binary left input"),
                self.input(1).expect("binary right input"),
            ))
        } else {
            Err(self.layout_error(2))
        }
    }

    pub fn expect_ternary(
        &self,
    ) -> MResult<(
        FunctionOutputPort<'_>,
        FunctionInputPort<'_>,
        FunctionInputPort<'_>,
        FunctionInputPort<'_>,
    )> {
        if self.layout == FunctionInvocationLayout::Ternary {
            Ok((
                self.output(),
                self.input(0).expect("ternary first input"),
                self.input(1).expect("ternary second input"),
                self.input(2).expect("ternary third input"),
            ))
        } else {
            Err(self.layout_error(3))
        }
    }

    pub fn expect_quaternary(
        &self,
    ) -> MResult<(
        FunctionOutputPort<'_>,
        FunctionInputPort<'_>,
        FunctionInputPort<'_>,
        FunctionInputPort<'_>,
        FunctionInputPort<'_>,
    )> {
        if self.layout == FunctionInvocationLayout::Quaternary {
            Ok((
                self.output(),
                self.input(0).expect("quaternary first input"),
                self.input(1).expect("quaternary second input"),
                self.input(2).expect("quaternary third input"),
                self.input(3).expect("quaternary fourth input"),
            ))
        } else {
            Err(self.layout_error(4))
        }
    }

    pub fn expect_variadic(&self) -> MResult<(FunctionOutputPort<'_>, FunctionInputPorts<'_>)> {
        if self.layout == FunctionInvocationLayout::Variadic {
            Ok((self.output(), self.inputs()))
        } else {
            Err(self.layout_error(self.input_count()))
        }
    }

    pub(crate) fn normalize_for_signature(self, signature: RuntimeFunctionSignature) -> Self {
        if !matches!(signature.inputs, RuntimeFunctionInputs::Variadic { .. }) {
            return self;
        }
        Self {
            layout: FunctionInvocationLayout::Variadic,
            ..self
        }
    }

    fn layout_error(&self, expected: usize) -> MechError {
        MechError::new(
            IncorrectNumberOfArguments {
                expected,
                found: self.input_count(),
            },
            None,
        )
        .with_compiler_loc()
    }

    fn layout_name(&self) -> &'static str {
        match self.layout {
            FunctionInvocationLayout::Nullary => "Nullary",
            FunctionInvocationLayout::Unary => "Unary",
            FunctionInvocationLayout::Binary => "Binary",
            FunctionInvocationLayout::Ternary => "Ternary",
            FunctionInvocationLayout::Quaternary => "Quaternary",
            FunctionInvocationLayout::Variadic => "Variadic",
        }
    }

    pub fn validate_signature(&self, signature: RuntimeFunctionSignature) -> MResult<()> {
        let expected_layout = match signature.inputs {
            RuntimeFunctionInputs::Nullary => FunctionInvocationLayout::Nullary,
            RuntimeFunctionInputs::Unary(_) => FunctionInvocationLayout::Unary,
            RuntimeFunctionInputs::Binary(_, _) => FunctionInvocationLayout::Binary,
            RuntimeFunctionInputs::Ternary(_, _, _) => FunctionInvocationLayout::Ternary,
            RuntimeFunctionInputs::Quaternary(_, _, _, _) => FunctionInvocationLayout::Quaternary,
            RuntimeFunctionInputs::Variadic { .. } => FunctionInvocationLayout::Variadic,
        };
        if self.layout != expected_layout {
            return Err(self.layout_error(expected_signature_input_count(
                signature,
                self.input_count(),
            )));
        }
        validate_cell_representation(
            &self.output,
            signature.output,
            crate::FunctionArgumentRole::Output,
        )?;
        let expected_inputs = expected_signature_inputs(signature, self.input_count());
        for (index, (cell, expected)) in self.inputs.iter().zip(expected_inputs).enumerate() {
            validate_cell_representation(
                cell,
                expected,
                crate::FunctionArgumentRole::Input(index),
            )?;
        }
        Ok(())
    }

    pub fn validate_contract(&self, contract: RuntimeFunctionContract) -> MResult<()> {
        if contract.output_alias == RuntimeOutputAliasPolicy::DisallowInputAlias {
            for (index, input) in self.inputs.iter().enumerate() {
                if self.output.same_writable_storage(input) {
                    return Err(
                        MechError::new(FunctionCellAliasViolation { input: index }, None)
                            .with_compiler_loc(),
                    );
                }
            }
        }
        let output = canonical_matrix_descriptor(&self.output)?;
        let inputs = self
            .inputs
            .iter()
            .map(canonical_matrix_descriptor)
            .collect::<MResult<Vec<_>>>()?;
        crate::function::contract::validate_canonical_shapes(
            contract,
            &self.output,
            &self.inputs,
            output,
            &inputs,
        )?;
        Ok(())
    }

    /// Performs the opt-in operation-memory check without changing production validation.
    pub fn check_operation_memory_contract(
        &self,
        declaration: &OperationContractDeclaration,
    ) -> MResult<()> {
        let direct = declaration.memory_requirements(self.input_count());
        let (requirements, coalesced_input) = match direct {
            Ok(requirements) => (requirements, None),
            Err(direct_error) => {
                let Some(base_input) = coalesced_read_modify_write_input(declaration) else {
                    return Err(function_memory_contract_error(
                        FunctionMemoryContractViolationReason::OperationContractDerivation {
                            error: direct_error,
                        },
                    ));
                };
                let semantic_input_count = self.input_count().checked_add(1).ok_or_else(|| {
                    function_memory_contract_error(
                        FunctionMemoryContractViolationReason::OperationContractDerivation {
                            error: direct_error.clone(),
                        },
                    )
                })?;
                let requirements = declaration
                    .memory_requirements(semantic_input_count)
                    .map_err(|_| {
                        function_memory_contract_error(
                            FunctionMemoryContractViolationReason::OperationContractDerivation {
                                error: direct_error,
                            },
                        )
                    })?;
                (requirements, Some(base_input))
            }
        };

        for (index, requirement) in requirements.inputs.iter().enumerate() {
            let cell = self.semantic_input_cell(index, coalesced_input)?;
            check_invocation_cell_requirement(cell, requirement).map_err(|error| {
                function_memory_contract_error(FunctionMemoryContractViolationReason::InputPort {
                    index,
                    error,
                })
            })?;
        }

        match requirements.outputs.as_ref() {
            [] => {
                let schemas = self.output.schema_table();
                let schema = schemas
                    .get(self.output.schema())
                    .expect("function output schema remains present");
                let is_unit = matches!(schema.body(), SchemaBody::Tuple(elements) if elements.is_empty())
                    && self.output.storage_capabilities().topology
                        == StorageTopology::CanonicalValue;
                if !is_unit {
                    return Err(function_memory_contract_error(
                        FunctionMemoryContractViolationReason::ZeroOutputBridgeIsNotUnit,
                    ));
                }
            }
            [requirement] => {
                check_invocation_cell_requirement(&self.output, requirement).map_err(|error| {
                    function_memory_contract_error(
                        FunctionMemoryContractViolationReason::OutputPort { error },
                    )
                })?;
                self.check_operation_output_alias(requirement, coalesced_input)?;
            }
            outputs => {
                return Err(function_memory_contract_error(
                    FunctionMemoryContractViolationReason::MultipleSemanticOutputsUnsupported {
                        outputs: outputs.len(),
                    },
                ));
            }
        }
        Ok(())
    }

    fn semantic_input_cell(
        &self,
        input: usize,
        coalesced_input: Option<usize>,
    ) -> MResult<&ValueCell> {
        if coalesced_input == Some(input) {
            return Ok(&self.output);
        }
        let physical_input = match coalesced_input {
            Some(base) if input > base => input - 1,
            _ => input,
        };
        self.inputs.get(physical_input).ok_or_else(|| {
            function_memory_contract_error(
                FunctionMemoryContractViolationReason::InvalidDeclaredAliasInput {
                    input: u16::try_from(input).unwrap_or(u16::MAX),
                    inputs: self.input_count() + usize::from(coalesced_input.is_some()),
                },
            )
        })
    }

    fn declared_alias_input(
        &self,
        input: u16,
        coalesced_input: Option<usize>,
    ) -> MResult<&ValueCell> {
        self.semantic_input_cell(usize::from(input), coalesced_input)
    }

    fn check_operation_output_alias(
        &self,
        requirement: &PortMemoryRequirement,
        coalesced_input: Option<usize>,
    ) -> MResult<()> {
        let Some(alias) = requirement.alias else {
            return Ok(());
        };
        match alias {
            crate::AliasPolicy::NoAlias => {
                for (index, input) in self.inputs.iter().enumerate() {
                    if self.output.same_writable_storage(input) {
                        return Err(function_memory_contract_error(
                            FunctionMemoryContractViolationReason::NoAliasViolation {
                                input: index,
                            },
                        ));
                    }
                }
            }
            crate::AliasPolicy::MayAlias { input } => {
                let designated = self.declared_alias_input(input, coalesced_input)?;
                let semantic_input_count =
                    self.input_count() + usize::from(coalesced_input.is_some());
                for index in 0..semantic_input_count {
                    let candidate = self.semantic_input_cell(index, coalesced_input)?;
                    if self.output.same_writable_storage(candidate)
                        && !designated.same_writable_storage(candidate)
                    {
                        return Err(function_memory_contract_error(
                            FunctionMemoryContractViolationReason::MayAliasViolation {
                                declared_input: input,
                                unrelated_input: index,
                            },
                        ));
                    }
                }
            }
            crate::AliasPolicy::InPlaceRequired { input } => {
                let designated = self.declared_alias_input(input, coalesced_input)?;
                if !self.output.same_writable_storage(designated) {
                    return Err(function_memory_contract_error(
                        FunctionMemoryContractViolationReason::InPlaceRequiredViolation { input },
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn output_cell(&self) -> &ValueCell {
        &self.output
    }

    pub fn input_cells(&self) -> &[ValueCell] {
        &self.inputs
    }
}

fn check_invocation_cell_requirement(
    cell: &ValueCell,
    requirement: &PortMemoryRequirement,
) -> Result<(), PortStorageCompatibilityError> {
    let schemas = cell.schema_table();
    let schema = schemas
        .get(cell.schema())
        .expect("function invocation schema remains present");
    let shape = cell.shape().clone();
    crate::check_port_storage_compatibility(
        schema,
        &shape,
        requirement,
        &cell.storage_capabilities(),
    )
}

pub(crate) fn coalesced_read_modify_write_input(
    declaration: &OperationContractDeclaration,
) -> Option<usize> {
    let [output] = declaration.outputs.as_ref() else {
        return None;
    };
    let crate::OutputConstruction::ReadModifyWrite { base_input, .. } = &output.construction else {
        return None;
    };
    let alias_input = match output.alias {
        crate::AliasPolicy::MayAlias { input } | crate::AliasPolicy::InPlaceRequired { input } => {
            input
        }
        crate::AliasPolicy::NoAlias => return None,
    };
    (alias_input == *base_input).then_some(usize::from(*base_input))
}

fn function_memory_contract_error(reason: FunctionMemoryContractViolationReason) -> MechError {
    MechError::new(FunctionMemoryContractViolation { reason }, None).with_compiler_loc()
}

pub(crate) fn canonical_matrix_descriptor(
    cell: &ValueCell,
) -> MResult<Option<FunctionMatrixDescriptor>> {
    let FunctionValueRepresentation::Matrix { storage, .. } = cell.representation() else {
        return Ok(None);
    };
    let schemas = cell.schema_table();
    let Some(schema) = schemas.entry(cell.schema()) else {
        return Ok(None);
    };
    let crate::SchemaBody::Matrix { dimensions, .. } = schema.schema().body() else {
        return Ok(None);
    };
    let [rows, cols] = dimensions.as_ref() else {
        return Ok(None);
    };
    let rows = usize::try_from(cell.shape().resolve_dimension(rows)?).map_err(|_| {
        crate::function_shape_contract_violation("matrix", "row count exceeds usize")
    })?;
    let cols = usize::try_from(cell.shape().resolve_dimension(cols)?).map_err(|_| {
        crate::function_shape_contract_violation("matrix", "column count exceeds usize")
    })?;
    let representation = match storage {
        FunctionMatrixStoragePattern::Exact(representation) => representation,
        FunctionMatrixStoragePattern::AnyStorage => FunctionMatrixRepresentation::MatrixD,
    };
    Ok(Some(FunctionMatrixDescriptor {
        representation,
        rows,
        cols,
    }))
}

fn expected_signature_input_count(
    signature: RuntimeFunctionSignature,
    variadic_count: usize,
) -> usize {
    match signature.inputs {
        RuntimeFunctionInputs::Nullary => 0,
        RuntimeFunctionInputs::Unary(_) => 1,
        RuntimeFunctionInputs::Binary(_, _) => 2,
        RuntimeFunctionInputs::Ternary(_, _, _) => 3,
        RuntimeFunctionInputs::Quaternary(_, _, _, _) => 4,
        RuntimeFunctionInputs::Variadic { .. } => variadic_count,
    }
}

fn expected_signature_inputs(
    signature: RuntimeFunctionSignature,
    variadic_count: usize,
) -> Vec<FunctionValueRepresentation> {
    match signature.inputs {
        RuntimeFunctionInputs::Nullary => Vec::new(),
        RuntimeFunctionInputs::Unary(input) => vec![input],
        RuntimeFunctionInputs::Binary(first, second) => vec![first, second],
        RuntimeFunctionInputs::Ternary(first, second, third) => vec![first, second, third],
        RuntimeFunctionInputs::Quaternary(first, second, third, fourth) => {
            vec![first, second, third, fourth]
        }
        RuntimeFunctionInputs::Variadic { element } => vec![element; variadic_count],
    }
}

fn validate_cell_representation(
    cell: &ValueCell,
    expected: FunctionValueRepresentation,
    role: FunctionArgumentRole,
) -> MResult<()> {
    let found = cell.representation();
    if expected.matches(found) {
        Ok(())
    } else {
        Err(MechError::new(
            FunctionSignatureViolation {
                role,
                expected,
                found,
            },
            None,
        )
        .with_compiler_loc())
    }
}

fn function_argument_type_mismatch<T>(cell: &ValueCell, role: FunctionArgumentRole) -> MechError {
    MechError::new(
        FunctionArgumentTypeMismatch {
            role,
            expected: type_name::<Ref<T>>().to_string(),
            found: format!("{:?}", cell.representation()),
        },
        None,
    )
    .with_compiler_loc()
}

impl fmt::Debug for FunctionInvocation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FunctionInvocation")
            .field("layout", &self.layout_name())
            .field("input_count", &self.input_count())
            .finish()
    }
}

impl FunctionInputPort<'_> {
    pub const fn index(self) -> usize {
        self.index
    }

    /// Borrows an explicitly pinned external input.
    ///
    /// ```compile_fail
    /// use mech_core::FunctionPortBacking;
    /// struct Unsupported;
    /// fn require<T: FunctionPortBacking>() {}
    /// require::<Unsupported>();
    /// ```
    pub fn try_external_ref<T: FunctionPortBacking>(self) -> MResult<Ref<T>> {
        self.invocation.inputs[self.index]
            .try_ref::<T>()
            .map_err(|_| {
                function_argument_type_mismatch::<T>(
                    &self.invocation.inputs[self.index],
                    FunctionArgumentRole::Input(self.index),
                )
            })
    }

    /// Binds a scalar logical input without retaining its physical backing.
    /// The active managed frame resolves the cell's current allocation at
    /// every invocation.
    pub fn try_managed<T: FunctionPortBacking>(self) -> MResult<ManagedPort<T>> {
        let cell = &self.invocation.inputs[self.index];
        validate_cell_representation(
            cell,
            T::REPRESENTATION,
            FunctionArgumentRole::Input(self.index),
        )?;
        Ok(ManagedPort::input(cell.clone(), self.index))
    }

    /// Binds this physical input at its semantic ordinal when a coalesced
    /// read-modify-write base output precedes it in the operation contract.
    pub fn try_managed_at<T: FunctionPortBacking>(
        self,
        semantic_input: usize,
    ) -> MResult<ManagedPort<T>> {
        let cell = &self.invocation.inputs[self.index];
        validate_cell_representation(
            cell,
            T::REPRESENTATION,
            FunctionArgumentRole::Input(self.index),
        )?;
        Ok(ManagedPort::input(cell.clone(), semantic_input))
    }

    /// Binds either a scalar value or a dense matrix by its fixed-width
    /// element kind. This is the canonical constructor used by shared kernel
    /// families: storage shape is supplied by the call plan, not retained in
    /// the port capability.
    pub fn try_managed_element<T: FunctionPortBacking>(self) -> MResult<ManagedPort<T>> {
        let cell = &self.invocation.inputs[self.index];
        let representation = cell.representation();
        if representation == T::REPRESENTATION {
            return Ok(ManagedPort::input(cell.clone(), self.index));
        }
        #[cfg(feature = "matrix")]
        if let FunctionValueRepresentation::Matrix { element, .. } = representation
            && element == crate::matrix_element_for_representation(T::REPRESENTATION)
        {
            return Ok(ManagedPort::input(cell.clone(), self.index));
        }
        Err(function_argument_type_mismatch::<T>(
            cell,
            FunctionArgumentRole::Input(self.index),
        ))
    }

    pub fn try_managed_element_at<T: FunctionPortBacking>(
        self,
        semantic_input: usize,
    ) -> MResult<ManagedPort<T>> {
        let cell = &self.invocation.inputs[self.index];
        let representation = cell.representation();
        if representation == T::REPRESENTATION {
            return Ok(ManagedPort::input(cell.clone(), semantic_input));
        }
        #[cfg(feature = "matrix")]
        if let FunctionValueRepresentation::Matrix { element, .. } = representation
            && element == crate::matrix_element_for_representation(T::REPRESENTATION)
        {
            return Ok(ManagedPort::input(cell.clone(), semantic_input));
        }
        Err(function_argument_type_mismatch::<T>(
            cell,
            FunctionArgumentRole::Input(self.index),
        ))
    }

    /// Binds a matrix logical input by element capability rather than by an
    /// owning matrix representation.
    #[cfg(feature = "matrix")]
    pub fn try_managed_matrix<T: FunctionPortBacking>(self) -> MResult<ManagedPort<T>> {
        let cell = &self.invocation.inputs[self.index];
        let expected = crate::matrix_element_for_representation(T::REPRESENTATION);
        match cell.representation() {
            FunctionValueRepresentation::Matrix { element, .. } if element == expected => {
                Ok(ManagedPort::input(cell.clone(), self.index))
            }
            _ => Err(function_argument_type_mismatch::<T>(
                cell,
                FunctionArgumentRole::Input(self.index),
            )),
        }
    }

    pub fn value(self) -> FunctionValueInput {
        FunctionValueInput {
            cell: self.invocation.inputs[self.index].clone(),
            index: self.index,
        }
    }
}

impl fmt::Debug for FunctionInputPort<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FunctionInputPort")
            .field("index", &self.index)
            .finish()
    }
}

impl FunctionOutputPort<'_> {
    /// Borrows an explicitly pinned external output.
    ///
    /// ```compile_fail
    /// use mech_core::FunctionPortBacking;
    /// struct Unsupported;
    /// fn require<T: FunctionPortBacking>() {}
    /// require::<Unsupported>();
    /// ```
    pub fn try_external_ref<T: FunctionPortBacking>(self) -> MResult<Ref<T>> {
        self.invocation.output.try_ref::<T>().map_err(|_| {
            function_argument_type_mismatch::<T>(
                &self.invocation.output,
                FunctionArgumentRole::Output,
            )
        })
    }

    /// Binds a scalar logical output without retaining its physical backing.
    pub fn try_managed<T: FunctionPortBacking>(self) -> MResult<ManagedPort<T>> {
        let cell = &self.invocation.output;
        validate_cell_representation(cell, T::REPRESENTATION, FunctionArgumentRole::Output)?;
        Ok(ManagedPort::output(cell.clone()))
    }

    /// Binds a read view of an output that is also the declared base input of
    /// a read-modify-write operation. The operation contract remains the
    /// authority for whether this semantic input ordinal is valid.
    pub fn try_managed_base_input<T: FunctionPortBacking>(
        self,
        semantic_input: usize,
    ) -> MResult<ManagedPort<T>> {
        let cell = &self.invocation.output;
        validate_cell_representation(cell, T::REPRESENTATION, FunctionArgumentRole::Output)?;
        Ok(ManagedPort::input(cell.clone(), semantic_input))
    }

    /// Element-kind form of [`Self::try_managed_base_input`] for dense values.
    pub fn try_managed_element_base_input<T: FunctionPortBacking>(
        self,
        semantic_input: usize,
    ) -> MResult<ManagedPort<T>> {
        let cell = &self.invocation.output;
        let representation = cell.representation();
        if representation == T::REPRESENTATION {
            return Ok(ManagedPort::input(cell.clone(), semantic_input));
        }
        #[cfg(feature = "matrix")]
        if let FunctionValueRepresentation::Matrix { element, .. } = representation
            && element == crate::matrix_element_for_representation(T::REPRESENTATION)
        {
            return Ok(ManagedPort::input(cell.clone(), semantic_input));
        }
        Err(function_argument_type_mismatch::<T>(
            cell,
            FunctionArgumentRole::Output,
        ))
    }

    /// Binds either a scalar output or a dense matrix by its fixed-width
    /// element kind; the call plan remains the geometry authority.
    pub fn try_managed_element<T: FunctionPortBacking>(self) -> MResult<ManagedPort<T>> {
        let cell = &self.invocation.output;
        let representation = cell.representation();
        if representation == T::REPRESENTATION {
            return Ok(ManagedPort::output(cell.clone()));
        }
        #[cfg(feature = "matrix")]
        if let FunctionValueRepresentation::Matrix { element, .. } = representation
            && element == crate::matrix_element_for_representation(T::REPRESENTATION)
        {
            return Ok(ManagedPort::output(cell.clone()));
        }
        Err(function_argument_type_mismatch::<T>(
            cell,
            FunctionArgumentRole::Output,
        ))
    }

    /// Binds a matrix logical output by element capability rather than by an
    /// owning matrix representation.
    #[cfg(feature = "matrix")]
    pub fn try_managed_matrix<T: FunctionPortBacking>(self) -> MResult<ManagedPort<T>> {
        let cell = &self.invocation.output;
        let expected = crate::matrix_element_for_representation(T::REPRESENTATION);
        match cell.representation() {
            FunctionValueRepresentation::Matrix { element, .. } if element == expected => {
                Ok(ManagedPort::output(cell.clone()))
            }
            _ => Err(function_argument_type_mismatch::<T>(
                cell,
                FunctionArgumentRole::Output,
            )),
        }
    }

    pub fn value(self) -> FunctionValueOutput {
        FunctionValueOutput {
            cell: self.invocation.output.clone(),
        }
    }
}

#[derive(Clone)]
pub struct FunctionValueInput {
    cell: ValueCell,
    index: usize,
}

#[derive(Clone)]
pub struct FunctionValueOutput {
    cell: ValueCell,
}

impl FunctionValueInput {
    /// Returns the canonical input cell retained by this invocation value.
    pub const fn cell(&self) -> &ValueCell {
        &self.cell
    }

    pub(crate) const fn managed_role(&self) -> crate::ManagedPortRole {
        crate::ManagedPortRole::Input(self.index)
    }

    pub fn snapshot(&self) -> MResult<Value> {
        self.cell.snapshot()
    }

    pub fn schema(&self) -> SchemaId {
        self.cell.schema()
    }

    pub fn schema_key(&self) -> crate::SchemaKey {
        self.cell.schema_key()
    }

    pub fn shape(&self) -> ShapeInstance {
        self.cell.shape().clone()
    }

    pub fn representation(&self) -> FunctionValueRepresentation {
        self.cell.representation()
    }

    pub fn snapshot_eq(&self, other: &Self) -> MResult<bool> {
        self.cell.snapshot_eq(&other.cell)
    }

    pub fn set_contains(&self, candidate: &Self) -> MResult<bool> {
        let SchemaBody::Set { element, .. } = self.cell.closed_schema_body()? else {
            return self.cell.set_contains(&candidate.cell);
        };
        if candidate.cell.closed_schema_body()? != *element {
            return Ok(false);
        }
        self.cell.set_contains(&candidate.cell)
    }

    pub fn set_elements(&self) -> MResult<Box<[ValueData]>> {
        self.cell.set_elements()
    }

    pub fn set_element_drafts(&self) -> MResult<Box<[ValueDataDraft]>> {
        self.cell.set_element_drafts()
    }

    pub fn set_elements_after_insert(&self, candidate: &Self) -> MResult<Box<[ValueData]>> {
        self.cell.set_elements_after_insert(&candidate.cell)
    }

    pub fn set_elements_after_remove(&self, candidate: &Self) -> MResult<Box<[ValueData]>> {
        self.cell.set_elements_after_remove(&candidate.cell)
    }

    pub fn set_union_elements(&self, other: &Self) -> MResult<Box<[ValueData]>> {
        self.cell.set_union_elements(&other.cell)
    }

    pub fn set_intersection_elements(&self, other: &Self) -> MResult<Box<[ValueData]>> {
        self.cell.set_intersection_elements(&other.cell)
    }

    pub fn set_difference_elements(&self, other: &Self) -> MResult<Box<[ValueData]>> {
        self.cell.set_difference_elements(&other.cell)
    }

    pub fn set_symmetric_difference_elements(&self, other: &Self) -> MResult<Box<[ValueData]>> {
        self.cell.set_symmetric_difference_elements(&other.cell)
    }

    pub fn set_relation(&self, other: &Self, relation: crate::SetValueRelation) -> MResult<bool> {
        self.cell.set_relation(&other.cell, relation)
    }

    #[cfg(feature = "semantic-compiler")]
    pub fn compile_register(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        crate::compile_value_cell_register(&self.cell, context)
    }
}

impl FunctionValueOutput {
    /// Returns the canonical output cell retained by this invocation value.
    pub const fn cell(&self) -> &ValueCell {
        &self.cell
    }

    pub fn snapshot(&self) -> MResult<Value> {
        self.cell.snapshot()
    }

    pub fn replace(&self, value: &Value) -> MResult<()> {
        self.cell.replace(value)
    }

    pub fn replace_set(&self, elements: Box<[ValueData]>) -> MResult<()> {
        let next = self.build_set(elements)?;
        self.cell.replace(&next)
    }

    pub fn build_set(&self, elements: Box<[ValueData]>) -> MResult<Value> {
        self.cell.rebuild_set(elements)
    }

    pub fn replace_set_drafts(&self, elements: Box<[ValueDataDraft]>) -> MResult<()> {
        let next = self.build_set_drafts(elements)?;
        self.cell.replace(&next)
    }

    pub fn build_set_drafts(&self, elements: Box<[ValueDataDraft]>) -> MResult<Value> {
        self.cell.rebuild_set_drafts(elements)
    }

    pub fn replace_matrix_drafts(
        &self,
        dimensions: Box<[u64]>,
        elements: Box<[ValueDataDraft]>,
    ) -> MResult<()> {
        let next = self.cell.rebuild_matrix_drafts(dimensions, elements)?;
        self.cell.replace(&next)
    }

    pub fn schema(&self) -> SchemaId {
        self.cell.schema()
    }

    pub fn schema_key(&self) -> crate::SchemaKey {
        self.cell.schema_key()
    }

    pub fn shape(&self) -> ShapeInstance {
        self.cell.shape().clone()
    }

    pub fn representation(&self) -> FunctionValueRepresentation {
        self.cell.representation()
    }

    pub fn state_port(&self) -> crate::FunctionStatePort<'_> {
        crate::FunctionStatePort::from_cell(&self.cell)
    }

    #[cfg(feature = "semantic-compiler")]
    pub fn compile_register(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        crate::compile_value_cell_register(&self.cell, context)
    }
}

impl fmt::Debug for FunctionValueInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FunctionValueInput")
            .field("schema_key", &self.cell.schema_key())
            .field("shape", &self.cell.shape())
            .finish()
    }
}

impl fmt::Debug for FunctionValueOutput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FunctionValueOutput")
            .field("schema_key", &self.cell.schema_key())
            .field("shape", &self.cell.shape())
            .finish()
    }
}

impl fmt::Debug for FunctionOutputPort<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("FunctionOutputPort(Output)")
    }
}

impl<'a> Iterator for FunctionInputPorts<'a> {
    type Item = FunctionInputPort<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let port = self.invocation.input(self.next)?;
        self.next += 1;
        Some(port)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len();
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for FunctionInputPorts<'_> {
    fn len(&self) -> usize {
        self.invocation.input_count().saturating_sub(self.next)
    }
}

impl core::iter::FusedIterator for FunctionInputPorts<'_> {}

impl fmt::Debug for FunctionInputPorts<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FunctionInputPorts")
            .field("next", &self.next)
            .field("remaining", &self.len())
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionArgumentTypeMismatch {
    pub role: FunctionArgumentRole,
    pub expected: String,
    pub found: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FunctionMatrixDescriptor {
    pub representation: FunctionMatrixRepresentation,
    pub rows: usize,
    pub cols: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionArgumentAliasViolation {
    pub input: usize,
    pub cell: CanonicalCellId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FunctionCellAliasViolation {
    pub input: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionMemoryContractViolation {
    pub reason: FunctionMemoryContractViolationReason,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FunctionMemoryContractViolationReason {
    OperationContractDerivation {
        error: OperationContractError,
    },
    InputPort {
        index: usize,
        error: PortStorageCompatibilityError,
    },
    OutputPort {
        error: PortStorageCompatibilityError,
    },
    ZeroOutputBridgeIsNotUnit,
    MultipleSemanticOutputsUnsupported {
        outputs: usize,
    },
    InvalidDeclaredAliasInput {
        input: u16,
        inputs: usize,
    },
    NoAliasViolation {
        input: usize,
    },
    MayAliasViolation {
        declared_input: u16,
        unrelated_input: usize,
    },
    InPlaceRequiredViolation {
        input: u16,
    },
}

impl MechErrorKind for FunctionMemoryContractViolation {
    fn name(&self) -> &str {
        "FunctionMemoryContractViolation"
    }

    fn message(&self) -> String {
        match &self.reason {
            FunctionMemoryContractViolationReason::OperationContractDerivation { error } => {
                format!("operation memory requirement derivation failed: {error:?}")
            }
            FunctionMemoryContractViolationReason::InputPort { index, error } => {
                format!("input port {index} does not satisfy its memory requirement: {error}")
            }
            FunctionMemoryContractViolationReason::OutputPort { error } => {
                format!("output port does not satisfy its memory requirement: {error}")
            }
            FunctionMemoryContractViolationReason::ZeroOutputBridgeIsNotUnit => {
                "zero-output operation requires a canonical unit compatibility output".to_string()
            }
            FunctionMemoryContractViolationReason::MultipleSemanticOutputsUnsupported {
                outputs,
            } => {
                format!("the current invocation bridge cannot represent {outputs} semantic outputs")
            }
            FunctionMemoryContractViolationReason::InvalidDeclaredAliasInput { input, inputs } => {
                format!(
                    "declared alias input {input} is outside the invocation input count {inputs}"
                )
            }
            FunctionMemoryContractViolationReason::NoAliasViolation { input } => {
                format!("output shares physical storage with forbidden input {input}")
            }
            FunctionMemoryContractViolationReason::MayAliasViolation {
                declared_input,
                unrelated_input,
            } => format!(
                "output may alias input {declared_input}, but shares unrelated input {unrelated_input} storage"
            ),
            FunctionMemoryContractViolationReason::InPlaceRequiredViolation { input } => {
                format!("output does not share the physical storage required by input {input}")
            }
        }
    }
}

impl MechErrorKind for FunctionCellAliasViolation {
    fn name(&self) -> &str {
        "FunctionCellAliasViolation"
    }

    fn message(&self) -> String {
        format!(
            "function output aliases canonical input cell {}",
            self.input
        )
    }
}

impl MechErrorKind for FunctionArgumentAliasViolation {
    fn name(&self) -> &str {
        "FunctionArgumentAliasViolation"
    }

    fn message(&self) -> String {
        format!(
            "function output aliases input {} through reactive root cell {}",
            self.input,
            self.cell.get(),
        )
    }
}

impl MechErrorKind for FunctionArgumentTypeMismatch {
    fn name(&self) -> &str {
        "FunctionArgumentTypeMismatch"
    }

    fn message(&self) -> String {
        format!(
            "function argument {:?} requires exact runtime representation {}, found {}",
            self.role, self.expected, self.found,
        )
    }
}

#[cfg(all(test, feature = "f64"))]
mod operation_memory_tests {
    use super::*;
    use crate::{
        AliasPolicy, ChangeDetectionPolicy, DeliveryMode, ExternalInteraction, InputPortLayout,
        InputPortPolicy, OutputConstruction, OutputPortPolicy, RegionPolicy, ShapeRule,
    };

    fn declaration(alias: AliasPolicy) -> OperationContractDeclaration {
        OperationContractDeclaration {
            inputs: InputPortLayout::Fixed(
                vec![InputPortPolicy {
                    access: crate::AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                }]
                .into_boxed_slice(),
            ),
            outputs: vec![OutputPortPolicy {
                access: crate::AccessMode::Write,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::FullWrite {
                    shape: ShapeRule::Declared,
                },
                alias,
                change_detection: ChangeDetectionPolicy::KernelReported,
            }]
            .into_boxed_slice(),
            interaction: ExternalInteraction::Pure,
        }
    }

    fn reason(error: MechError) -> FunctionMemoryContractViolationReason {
        error
            .kind_as::<FunctionMemoryContractViolation>()
            .expect("operation memory check returns its structured error")
            .reason
            .clone()
    }

    #[test]
    fn operation_aliases_follow_storage_when_logical_identity_disagrees() {
        let first = ValueCell::from_exact(1_f64).unwrap();
        let detached = first.detached_clone().unwrap();
        let same_logical_different_storage =
            ValueCell::test_with_identity_and_payload(&first, &detached).unwrap();
        let different_logical_same_storage =
            ValueCell::test_with_identity_and_payload(&detached, &first).unwrap();

        assert!(first.same_logical_cell(&same_logical_different_storage));
        assert!(!first.same_storage(&same_logical_different_storage));
        FunctionInvocation::unary(same_logical_different_storage.clone(), first.clone())
            .check_operation_memory_contract(&declaration(AliasPolicy::NoAlias))
            .unwrap();
        assert_eq!(
            reason(
                FunctionInvocation::unary(same_logical_different_storage, first.clone())
                    .check_operation_memory_contract(&declaration(AliasPolicy::InPlaceRequired {
                        input: 0
                    },))
                    .unwrap_err(),
            ),
            FunctionMemoryContractViolationReason::InPlaceRequiredViolation { input: 0 }
        );

        assert!(!first.same_logical_cell(&different_logical_same_storage));
        assert!(first.same_storage(&different_logical_same_storage));
        assert_eq!(
            reason(
                FunctionInvocation::unary(different_logical_same_storage.clone(), first.clone())
                    .check_operation_memory_contract(&declaration(AliasPolicy::NoAlias))
                    .unwrap_err(),
            ),
            FunctionMemoryContractViolationReason::NoAliasViolation { input: 0 }
        );
        FunctionInvocation::unary(different_logical_same_storage, first)
            .check_operation_memory_contract(&declaration(AliasPolicy::InPlaceRequired {
                input: 0,
            }))
            .unwrap();
    }

    #[cfg(feature = "string")]
    #[test]
    fn shared_immutable_roots_are_not_writable_output_aliases() {
        let output = ValueCell::from_exact("seed".to_owned()).unwrap();
        let input = output.detached_clone().unwrap();

        assert!(output.same_storage(&input));
        assert!(!output.same_writable_storage(&input));
        FunctionInvocation::unary(output, input)
            .validate_contract(RuntimeFunctionContract::no_matrix(
                RuntimeOutputAliasPolicy::DisallowInputAlias,
            ))
            .unwrap();
    }

    #[test]
    fn coalesced_read_modify_write_output_satisfies_its_semantic_base_input() {
        let declaration = OperationContractDeclaration {
            inputs: InputPortLayout::Fixed(
                vec![
                    InputPortPolicy {
                        access: crate::AccessMode::Read,
                        delivery: DeliveryMode::Signal,
                    },
                    InputPortPolicy {
                        access: crate::AccessMode::Read,
                        delivery: DeliveryMode::Signal,
                    },
                ]
                .into_boxed_slice(),
            ),
            outputs: vec![OutputPortPolicy {
                access: crate::AccessMode::ReadWrite,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::ReadModifyWrite {
                    base_input: 0,
                    regions: RegionPolicy::WholeValue,
                },
                alias: AliasPolicy::InPlaceRequired { input: 0 },
                change_detection: ChangeDetectionPolicy::KernelReported,
            }]
            .into_boxed_slice(),
            interaction: ExternalInteraction::Pure,
        };
        let destination = ValueCell::from_exact(1_f64).unwrap();
        let source = ValueCell::from_exact(2_f64).unwrap();

        FunctionInvocation::unary(destination, source)
            .check_operation_memory_contract(&declaration)
            .unwrap();
    }
}
