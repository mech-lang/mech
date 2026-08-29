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

#[cfg(test)]
use crate::FunctionArgs;
use crate::FunctionMatrixStoragePattern;
#[cfg(feature = "matrix")]
use crate::structures::{CopyMat, Matrix};
#[cfg(feature = "semantic-compiler")]
use crate::{BytecodeCompilerContext, Register};
use crate::{
    FunctionArgumentRole, FunctionMatrixRepresentation, FunctionRuntimeType,
    FunctionSignatureViolation, FunctionValueRepresentation, IncorrectNumberOfArguments, MResult,
    MechError, MechErrorKind, ReactiveCellId, Ref, RuntimeFunctionContract, RuntimeFunctionInputs,
    RuntimeFunctionSignature, RuntimeOutputAliasPolicy, SchemaBody, SchemaId, ShapeInstance, Value,
    ValueCell, ValueData, ValueDataDraft,
};

mod function_port_backing {
    pub trait Sealed {}
}

/// An exact runtime backing type that may be extracted through a function port.
///
/// This sealed marker deliberately excludes universal values, [`crate::ValueCell`],
/// legacy aggregate wrappers, and reference wrappers around those types.
/// Compatibility values remain available only through the explicit adapter
/// boundary.
///
/// ```compile_fail
/// use mech_core::{FunctionPortBacking, MechSet};
/// fn require<T: FunctionPortBacking>() {}
/// require::<MechSet>();
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
                if self.output.same_cell(input) {
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

    pub fn output_cell(&self) -> &ValueCell {
        &self.output
    }

    pub fn input_cells(&self) -> &[ValueCell] {
        &self.inputs
    }
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

#[cfg(feature = "matrix")]
pub(crate) fn matrix_from_cell<T>(
    cell: &ValueCell,
    role: FunctionArgumentRole,
) -> MResult<Matrix<T>>
where
    T: FunctionPortBacking + Clone,
{
    let FunctionValueRepresentation::Matrix {
        storage: FunctionMatrixStoragePattern::Exact(storage),
        ..
    } = cell.representation()
    else {
        return Err(function_matrix_type_mismatch::<T>(cell, role));
    };
    #[allow(
        unreachable_patterns,
        reason = "the fallback is reachable only in narrow matrix feature profiles"
    )]
    let matrix = match storage {
        #[cfg(feature = "matrix1")]
        FunctionMatrixRepresentation::Matrix1 => Matrix::Matrix1(
            cell.try_ref::<crate::Matrix1<T>>()
                .map_err(|_| function_matrix_type_mismatch::<T>(cell, role))?,
        ),
        #[cfg(feature = "matrix2")]
        FunctionMatrixRepresentation::Matrix2 => Matrix::Matrix2(
            cell.try_ref::<crate::Matrix2<T>>()
                .map_err(|_| function_matrix_type_mismatch::<T>(cell, role))?,
        ),
        #[cfg(feature = "matrix3")]
        FunctionMatrixRepresentation::Matrix3 => Matrix::Matrix3(
            cell.try_ref::<crate::Matrix3<T>>()
                .map_err(|_| function_matrix_type_mismatch::<T>(cell, role))?,
        ),
        #[cfg(feature = "matrix4")]
        FunctionMatrixRepresentation::Matrix4 => Matrix::Matrix4(
            cell.try_ref::<crate::Matrix4<T>>()
                .map_err(|_| function_matrix_type_mismatch::<T>(cell, role))?,
        ),
        #[cfg(feature = "matrix2x3")]
        FunctionMatrixRepresentation::Matrix2x3 => Matrix::Matrix2x3(
            cell.try_ref::<crate::Matrix2x3<T>>()
                .map_err(|_| function_matrix_type_mismatch::<T>(cell, role))?,
        ),
        #[cfg(feature = "matrix3x2")]
        FunctionMatrixRepresentation::Matrix3x2 => Matrix::Matrix3x2(
            cell.try_ref::<crate::Matrix3x2<T>>()
                .map_err(|_| function_matrix_type_mismatch::<T>(cell, role))?,
        ),
        #[cfg(feature = "row_vector2")]
        FunctionMatrixRepresentation::RowVector2 => Matrix::RowVector2(
            cell.try_ref::<crate::RowVector2<T>>()
                .map_err(|_| function_matrix_type_mismatch::<T>(cell, role))?,
        ),
        #[cfg(feature = "row_vector3")]
        FunctionMatrixRepresentation::RowVector3 => Matrix::RowVector3(
            cell.try_ref::<crate::RowVector3<T>>()
                .map_err(|_| function_matrix_type_mismatch::<T>(cell, role))?,
        ),
        #[cfg(feature = "row_vector4")]
        FunctionMatrixRepresentation::RowVector4 => Matrix::RowVector4(
            cell.try_ref::<crate::RowVector4<T>>()
                .map_err(|_| function_matrix_type_mismatch::<T>(cell, role))?,
        ),
        #[cfg(feature = "vector2")]
        FunctionMatrixRepresentation::Vector2 => Matrix::Vector2(
            cell.try_ref::<crate::Vector2<T>>()
                .map_err(|_| function_matrix_type_mismatch::<T>(cell, role))?,
        ),
        #[cfg(feature = "vector3")]
        FunctionMatrixRepresentation::Vector3 => Matrix::Vector3(
            cell.try_ref::<crate::Vector3<T>>()
                .map_err(|_| function_matrix_type_mismatch::<T>(cell, role))?,
        ),
        #[cfg(feature = "vector4")]
        FunctionMatrixRepresentation::Vector4 => Matrix::Vector4(
            cell.try_ref::<crate::Vector4<T>>()
                .map_err(|_| function_matrix_type_mismatch::<T>(cell, role))?,
        ),
        #[cfg(feature = "row_vectord")]
        FunctionMatrixRepresentation::RowVectorD => Matrix::RowDVector(
            cell.try_ref::<crate::RowDVector<T>>()
                .map_err(|_| function_matrix_type_mismatch::<T>(cell, role))?,
        ),
        #[cfg(feature = "vectord")]
        FunctionMatrixRepresentation::VectorD => Matrix::DVector(
            cell.try_ref::<crate::DVector<T>>()
                .map_err(|_| function_matrix_type_mismatch::<T>(cell, role))?,
        ),
        #[cfg(feature = "matrixd")]
        FunctionMatrixRepresentation::MatrixD => Matrix::DMatrix(
            cell.try_ref::<crate::DMatrix<T>>()
                .map_err(|_| function_matrix_type_mismatch::<T>(cell, role))?,
        ),
        _ => return Err(function_matrix_type_mismatch::<T>(cell, role)),
    };
    Ok(matrix)
}

#[cfg(feature = "matrix")]
fn function_matrix_type_mismatch<T>(cell: &ValueCell, role: FunctionArgumentRole) -> MechError {
    MechError::new(
        FunctionArgumentTypeMismatch {
            role,
            expected: type_name::<Matrix<T>>().to_string(),
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

    /// Extracts the exact typed input backing without exposing legacy values.
    ///
    /// ```compile_fail
    /// use mech_core::FunctionPortBacking;
    /// struct Unsupported;
    /// fn require<T: FunctionPortBacking>() {}
    /// require::<Unsupported>();
    /// ```
    pub fn try_ref<T: FunctionPortBacking>(self) -> MResult<Ref<T>> {
        self.invocation.inputs[self.index]
            .try_ref::<T>()
            .map_err(|_| {
                function_argument_type_mismatch::<T>(
                    &self.invocation.inputs[self.index],
                    FunctionArgumentRole::Input(self.index),
                )
            })
    }

    /// Extracts the exact typed matrix input wrapper without exposing legacy values.
    ///
    /// ```compile_fail
    /// use mech_core::FunctionPortBacking;
    /// struct Unsupported;
    /// fn require<T: FunctionPortBacking>() {}
    /// require::<Unsupported>();
    /// ```
    #[cfg(feature = "matrix")]
    pub fn try_matrix<T>(self) -> MResult<Matrix<T>>
    where
        T: FunctionPortBacking + Clone,
    {
        matrix_from_cell(
            &self.invocation.inputs[self.index],
            FunctionArgumentRole::Input(self.index),
        )
    }

    /// Extracts an exact typed matrix as the private copy-kernel interface.
    ///
    /// This retains the original typed matrix handles and never exposes a
    /// universal value or performs a canonical-to-legacy conversion.
    #[cfg(feature = "matrix")]
    pub fn try_copyable_matrix<T>(self) -> MResult<Box<dyn CopyMat<T>>>
    where
        T: FunctionPortBacking + Clone,
        #[cfg(feature = "semantic-compiler")]
        T: crate::CompileConst
            + crate::ConstElem
            + crate::AsValueKind
            + core::fmt::Debug
            + PartialEq,
    {
        Ok(self.try_matrix::<T>()?.get_copyable_matrix())
    }

    pub fn value(self) -> FunctionValueInput {
        FunctionValueInput {
            cell: self.invocation.inputs[self.index].clone(),
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
    /// Extracts the exact typed output backing without exposing legacy values.
    ///
    /// ```compile_fail
    /// use mech_core::FunctionPortBacking;
    /// struct Unsupported;
    /// fn require<T: FunctionPortBacking>() {}
    /// require::<Unsupported>();
    /// ```
    pub fn try_ref<T: FunctionPortBacking>(self) -> MResult<Ref<T>> {
        self.invocation.output.try_ref::<T>().map_err(|_| {
            function_argument_type_mismatch::<T>(
                &self.invocation.output,
                FunctionArgumentRole::Output,
            )
        })
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

    pub fn snapshot(&self) -> MResult<Value> {
        self.cell.snapshot()
    }

    pub const fn schema(&self) -> SchemaId {
        self.cell.schema()
    }

    pub const fn schema_key(&self) -> crate::SchemaKey {
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
        let next = self.cell.rebuild_set(elements)?;
        self.cell.replace(&next)
    }

    pub fn replace_set_drafts(&self, elements: Box<[ValueDataDraft]>) -> MResult<()> {
        let next = self.cell.rebuild_set_drafts(elements)?;
        self.cell.replace(&next)
    }

    pub fn replace_matrix_drafts(
        &self,
        dimensions: Box<[u64]>,
        elements: Box<[ValueDataDraft]>,
    ) -> MResult<()> {
        let next = self.cell.rebuild_matrix_drafts(dimensions, elements)?;
        self.cell.replace(&next)
    }

    pub const fn schema(&self) -> SchemaId {
        self.cell.schema()
    }

    pub const fn schema_key(&self) -> crate::SchemaKey {
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
    pub cell: ReactiveCellId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FunctionCellAliasViolation {
    pub input: usize,
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

#[cfg(feature = "matrix")]
pub(crate) fn matrix_descriptor<T>(matrix: &Matrix<T>) -> FunctionMatrixDescriptor
where
    T: core::fmt::Debug + Clone + PartialEq + 'static,
{
    let representation = match matrix {
        #[cfg(feature = "matrix1")]
        Matrix::Matrix1(_) => FunctionMatrixRepresentation::Matrix1,
        #[cfg(feature = "matrix2")]
        Matrix::Matrix2(_) => FunctionMatrixRepresentation::Matrix2,
        #[cfg(feature = "matrix3")]
        Matrix::Matrix3(_) => FunctionMatrixRepresentation::Matrix3,
        #[cfg(feature = "matrix4")]
        Matrix::Matrix4(_) => FunctionMatrixRepresentation::Matrix4,
        #[cfg(feature = "matrix2x3")]
        Matrix::Matrix2x3(_) => FunctionMatrixRepresentation::Matrix2x3,
        #[cfg(feature = "matrix3x2")]
        Matrix::Matrix3x2(_) => FunctionMatrixRepresentation::Matrix3x2,
        #[cfg(feature = "row_vector2")]
        Matrix::RowVector2(_) => FunctionMatrixRepresentation::RowVector2,
        #[cfg(feature = "row_vector3")]
        Matrix::RowVector3(_) => FunctionMatrixRepresentation::RowVector3,
        #[cfg(feature = "row_vector4")]
        Matrix::RowVector4(_) => FunctionMatrixRepresentation::RowVector4,
        #[cfg(feature = "vector2")]
        Matrix::Vector2(_) => FunctionMatrixRepresentation::Vector2,
        #[cfg(feature = "vector3")]
        Matrix::Vector3(_) => FunctionMatrixRepresentation::Vector3,
        #[cfg(feature = "vector4")]
        Matrix::Vector4(_) => FunctionMatrixRepresentation::Vector4,
        #[cfg(feature = "row_vectord")]
        Matrix::RowDVector(_) => FunctionMatrixRepresentation::RowVectorD,
        #[cfg(feature = "vectord")]
        Matrix::DVector(_) => FunctionMatrixRepresentation::VectorD,
        #[cfg(feature = "matrixd")]
        Matrix::DMatrix(_) => FunctionMatrixRepresentation::MatrixD,
    };
    FunctionMatrixDescriptor {
        representation,
        rows: matrix.rows(),
        cols: matrix.cols(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LegacyValue, ToValue, require_function_ref};

    #[cfg(feature = "f64")]
    fn scalar(value: f64) -> (Ref<f64>, LegacyValue) {
        let reference = Ref::new(value);
        let value = reference.to_value();
        (reference, value)
    }

    #[cfg(feature = "f64")]
    fn canonical_scalar(value: f64) -> (Ref<f64>, ValueCell) {
        let reference = Ref::new(value);
        let cell = ValueCell::from_inferred_ref(reference.clone(), None).unwrap();
        (reference, cell)
    }

    #[cfg(feature = "f64")]
    #[test]
    fn canonical_invocation_layouts_preserve_cells_handles_and_value_capabilities() {
        let (output, output_cell) = canonical_scalar(0.0);
        let (first, first_cell) = canonical_scalar(1.0);
        let (second, second_cell) = canonical_scalar(2.0);
        let (third, third_cell) = canonical_scalar(3.0);
        let (fourth, fourth_cell) = canonical_scalar(4.0);
        let invocation = FunctionInvocation::binary(
            output_cell.clone(),
            first_cell.clone(),
            second_cell.clone(),
        );
        let (output_port, first_port, second_port) = invocation.expect_binary().unwrap();

        assert!(output_port.try_ref::<f64>().unwrap().same_handle(&output));
        assert!(first_port.try_ref::<f64>().unwrap().same_handle(&first));
        assert!(second_port.try_ref::<f64>().unwrap().same_handle(&second));
        assert_eq!(first_port.value().schema(), first_cell.schema());
        assert_eq!(first_port.value().shape(), first_cell.shape().clone());
        assert!(matches!(
            first_port.value().snapshot().unwrap().data(),
            ValueData::F64(value) if value.to_f64() == 1.0
        ));

        let replacement = second_cell.snapshot().unwrap();
        output_port.value().replace(&replacement).unwrap();
        assert_eq!(*output.borrow(), 2.0);
        assert_eq!(output_port.value().schema(), output_cell.schema());
        assert_eq!(output_port.value().shape(), output_cell.shape().clone());

        let nullary = FunctionInvocation::nullary(output_cell.clone());
        assert!(
            nullary
                .expect_nullary()
                .unwrap()
                .try_ref::<f64>()
                .unwrap()
                .same_handle(&output)
        );
        let unary = FunctionInvocation::unary(output_cell.clone(), first_cell.clone());
        assert!(
            unary
                .expect_unary()
                .unwrap()
                .1
                .try_ref::<f64>()
                .unwrap()
                .same_handle(&first)
        );
        let ternary = FunctionInvocation::ternary(
            output_cell.clone(),
            first_cell.clone(),
            second_cell.clone(),
            third_cell.clone(),
        );
        let (_, ternary_first, ternary_second, ternary_third) = ternary.expect_ternary().unwrap();
        assert!(ternary_first.try_ref::<f64>().unwrap().same_handle(&first));
        assert!(
            ternary_second
                .try_ref::<f64>()
                .unwrap()
                .same_handle(&second)
        );
        assert!(ternary_third.try_ref::<f64>().unwrap().same_handle(&third));
        let quaternary = FunctionInvocation::quaternary(
            output_cell.clone(),
            first_cell.clone(),
            second_cell.clone(),
            third_cell.clone(),
            fourth_cell.clone(),
        );
        let (_, first_port, second_port, third_port, fourth_port) =
            quaternary.expect_quaternary().unwrap();
        assert!(first_port.try_ref::<f64>().unwrap().same_handle(&first));
        assert!(second_port.try_ref::<f64>().unwrap().same_handle(&second));
        assert!(third_port.try_ref::<f64>().unwrap().same_handle(&third));
        assert!(fourth_port.try_ref::<f64>().unwrap().same_handle(&fourth));
        let variadic = FunctionInvocation::variadic(
            output_cell,
            vec![first_cell, second_cell, third_cell, fourth_cell].into_boxed_slice(),
        );
        let (_, mut ports) = variadic.expect_variadic().unwrap();
        for expected in [&first, &second, &third, &fourth] {
            assert!(
                ports
                    .next()
                    .unwrap()
                    .try_ref::<f64>()
                    .unwrap()
                    .same_handle(expected)
            );
        }
        assert!(ports.next().is_none());
    }

    #[cfg(feature = "f64")]
    #[test]
    fn canonical_invocation_preserves_aliases_and_effect_unit_output() {
        let (_, shared) = canonical_scalar(3.0);
        let invocation = FunctionInvocation::unary(shared.clone(), shared.clone());
        assert!(
            invocation
                .output_cell()
                .same_cell(&invocation.input_cells()[0])
        );
        assert!(
            invocation
                .validate_contract(RuntimeFunctionContract::no_matrix(
                    RuntimeOutputAliasPolicy::AllowInputAlias,
                ))
                .is_ok()
        );
        assert!(
            invocation
                .validate_contract(RuntimeFunctionContract::no_matrix(
                    RuntimeOutputAliasPolicy::DisallowInputAlias,
                ))
                .unwrap_err()
                .kind_as::<FunctionCellAliasViolation>()
                .is_some()
        );

        let unit = ValueCell::from_inferred_value_data(
            crate::SchemaBody::Tuple(Vec::new().into_boxed_slice()),
            crate::ValueDataDraft::Tuple(Vec::new().into_boxed_slice()),
        )
        .unwrap();
        let effect = FunctionInvocation::nullary(unit);
        assert!(matches!(
            effect.output().value().snapshot().unwrap().data(),
            crate::ValueData::Tuple(elements) if elements.is_empty()
        ));
    }

    #[cfg(feature = "f64")]
    #[test]
    fn fixed_invocation_layouts_preserve_output_and_input_order() {
        let (output, output_value) = scalar(10.0);
        let (first, first_value) = scalar(1.0);
        let (second, second_value) = scalar(2.0);
        let (third, third_value) = scalar(3.0);
        let (fourth, fourth_value) = scalar(4.0);

        let nullary = FunctionInvocation::from(FunctionArgs::Nullary(output_value.clone()));
        assert!(
            nullary
                .expect_nullary()
                .unwrap()
                .try_ref::<f64>()
                .unwrap()
                .same_handle(&output)
        );

        let unary = FunctionInvocation::from(FunctionArgs::Unary(
            output_value.clone(),
            first_value.clone(),
        ));
        let (unary_output, unary_first) = unary.expect_unary().unwrap();
        assert!(unary_output.try_ref::<f64>().unwrap().same_handle(&output));
        assert!(unary_first.try_ref::<f64>().unwrap().same_handle(&first));

        let binary = FunctionInvocation::from(FunctionArgs::Binary(
            output_value.clone(),
            first_value.clone(),
            second_value.clone(),
        ));
        let (binary_output, binary_first, binary_second) = binary.expect_binary().unwrap();
        assert!(binary_output.try_ref::<f64>().unwrap().same_handle(&output));
        assert!(binary_first.try_ref::<f64>().unwrap().same_handle(&first));
        assert!(binary_second.try_ref::<f64>().unwrap().same_handle(&second));

        let ternary = FunctionInvocation::from(FunctionArgs::Ternary(
            output_value.clone(),
            first_value.clone(),
            second_value.clone(),
            third_value.clone(),
        ));
        let (ternary_output, ternary_first, ternary_second, ternary_third) =
            ternary.expect_ternary().unwrap();
        assert!(
            ternary_output
                .try_ref::<f64>()
                .unwrap()
                .same_handle(&output)
        );
        assert!(ternary_first.try_ref::<f64>().unwrap().same_handle(&first));
        assert!(
            ternary_second
                .try_ref::<f64>()
                .unwrap()
                .same_handle(&second)
        );
        assert!(ternary_third.try_ref::<f64>().unwrap().same_handle(&third));

        let quaternary = FunctionInvocation::from(FunctionArgs::Quaternary(
            output_value,
            first_value,
            second_value,
            third_value,
            fourth_value,
        ));
        let (
            quaternary_output,
            quaternary_first,
            quaternary_second,
            quaternary_third,
            quaternary_fourth,
        ) = quaternary.expect_quaternary().unwrap();
        assert!(
            quaternary_output
                .try_ref::<f64>()
                .unwrap()
                .same_handle(&output)
        );
        assert!(
            quaternary_first
                .try_ref::<f64>()
                .unwrap()
                .same_handle(&first)
        );
        assert!(
            quaternary_second
                .try_ref::<f64>()
                .unwrap()
                .same_handle(&second)
        );
        assert!(
            quaternary_third
                .try_ref::<f64>()
                .unwrap()
                .same_handle(&third)
        );
        assert!(
            quaternary_fourth
                .try_ref::<f64>()
                .unwrap()
                .same_handle(&fourth)
        );
    }

    #[cfg(feature = "f64")]
    #[test]
    fn variadic_invocation_is_an_exact_borrowed_cursor() {
        let (_, output) = scalar(10.0);
        let (first, first_value) = scalar(1.0);
        let (second, second_value) = scalar(2.0);
        let (third, third_value) = scalar(3.0);
        let invocation = FunctionInvocation::from(FunctionArgs::Variadic(
            output,
            vec![first_value, second_value, third_value],
        ));

        let (_, mut inputs) = invocation.expect_variadic().unwrap();
        assert_eq!(inputs.len(), 3);
        assert!(
            inputs
                .next()
                .unwrap()
                .try_ref::<f64>()
                .unwrap()
                .same_handle(&first)
        );
        assert_eq!(inputs.len(), 2);
        assert!(
            inputs
                .next()
                .unwrap()
                .try_ref::<f64>()
                .unwrap()
                .same_handle(&second)
        );
        assert!(
            inputs
                .next()
                .unwrap()
                .try_ref::<f64>()
                .unwrap()
                .same_handle(&third)
        );
        assert!(inputs.next().is_none());
        assert!(inputs.next().is_none());
        assert_eq!(invocation.inputs().size_hint(), (3, Some(3)));
        assert_eq!(
            core::mem::size_of::<FunctionInputPorts<'_>>(),
            core::mem::size_of::<&FunctionInvocation>() + core::mem::size_of::<usize>()
        );
    }

    #[cfg(feature = "f64")]
    #[test]
    fn input_lookup_and_layout_checks_remain_exact() {
        let (_, output) = scalar(10.0);
        let (_, first) = scalar(1.0);
        let (_, second) = scalar(2.0);
        let invocation =
            FunctionInvocation::from(FunctionArgs::Variadic(output, vec![first, second]));

        assert_eq!(invocation.input(0).unwrap().index(), 0);
        assert_eq!(invocation.input(1).unwrap().index(), 1);
        assert!(invocation.input(2).is_none());

        let error = invocation.expect_binary().unwrap_err();
        assert_eq!(error.kind_name(), "IncorrectNumberOfArguments");
        let arity = error.kind_as::<IncorrectNumberOfArguments>().unwrap();
        assert_eq!(arity.expected, 2);
        assert_eq!(arity.found, 2);
    }

    #[cfg(all(feature = "f64", feature = "bool"))]
    #[test]
    fn port_type_failures_report_the_exact_argument_role() {
        let (_, output) = scalar(10.0);
        let (_, input) = scalar(1.0);
        let invocation = FunctionInvocation::from(FunctionArgs::Unary(output, input));
        let (output, input) = invocation.expect_unary().unwrap();

        let input_error = input.try_ref::<bool>().unwrap_err();
        assert_eq!(
            input_error
                .kind_as::<FunctionArgumentTypeMismatch>()
                .unwrap()
                .role,
            FunctionArgumentRole::Input(0),
        );
        let output_error = output.try_ref::<bool>().unwrap_err();
        assert_eq!(
            output_error
                .kind_as::<FunctionArgumentTypeMismatch>()
                .unwrap()
                .role,
            FunctionArgumentRole::Output,
        );
    }

    #[cfg(feature = "f64")]
    #[test]
    fn invocation_ports_do_not_unwrap_typed_or_mutable_values() {
        let (_, output) = scalar(10.0);
        let (_, scalar) = scalar(1.0);
        let typed = LegacyValue::Typed(Box::new(scalar.clone()), crate::ValueKind::F64);
        let mutable = LegacyValue::MutableReference(Ref::new(scalar));

        for wrapped in [typed, mutable] {
            let invocation = FunctionInvocation::from(FunctionArgs::Unary(output.clone(), wrapped));
            let (_, input) = invocation.expect_unary().unwrap();
            assert!(input.try_ref::<f64>().is_err());
        }
    }

    #[cfg(all(feature = "f64", feature = "matrix", feature = "matrix2"))]
    #[test]
    fn matrix_ports_preserve_the_exact_backing_handle() {
        use crate::matrix::Matrix;
        use nalgebra::Matrix2;

        let matrix = Ref::new(Matrix2::<f64>::identity());
        let value = LegacyValue::MatrixF64(Matrix::Matrix2(matrix.clone()));
        let invocation = FunctionInvocation::from(FunctionArgs::Unary(LegacyValue::Empty, value));
        let (_, input) = invocation.expect_unary().unwrap();
        assert!(
            input
                .try_ref::<Matrix2<f64>>()
                .unwrap()
                .same_handle(&matrix)
        );

        let wrapped = input.try_matrix::<f64>().unwrap();
        let Matrix::Matrix2(wrapped) = wrapped else {
            panic!("matrix input port changed the fixed representation")
        };
        assert!(wrapped.same_handle(&matrix));
    }

    #[cfg(all(
        feature = "f64",
        feature = "matrix",
        feature = "matrixd",
        feature = "string"
    ))]
    #[test]
    fn matrix_input_ports_preserve_dynamic_handles_and_reject_wrong_values() {
        use crate::matrix::Matrix;
        use nalgebra::DMatrix;

        let matrix = Ref::new(DMatrix::from_row_slice(2, 2, &[1.0_f64, 2.0, 3.0, 4.0]));
        let value = LegacyValue::MatrixF64(Matrix::DMatrix(matrix.clone()));
        let invocation = FunctionInvocation::from(FunctionArgs::Binary(
            LegacyValue::Empty,
            value.clone(),
            Ref::new(9.0_f64).to_value(),
        ));
        let (_, matrix_input, scalar_input) = invocation.expect_binary().unwrap();

        let extracted = matrix_input.try_matrix::<f64>().unwrap();
        let Matrix::DMatrix(extracted) = extracted else {
            panic!("matrix input port changed the dynamic representation")
        };
        assert!(extracted.same_handle(&matrix));

        let scalar_error = scalar_input.try_matrix::<f64>().unwrap_err();
        assert_eq!(
            scalar_error
                .kind_as::<FunctionArgumentTypeMismatch>()
                .unwrap()
                .role,
            FunctionArgumentRole::Input(1),
        );

        let string_matrix = LegacyValue::MatrixString(Matrix::DMatrix(Ref::new(
            DMatrix::from_element(1, 1, "wrong".to_string()),
        )));
        let invocation =
            FunctionInvocation::from(FunctionArgs::Unary(LegacyValue::Empty, string_matrix));
        let (_, input) = invocation.expect_unary().unwrap();
        assert!(input.try_matrix::<f64>().is_err());
    }

    #[cfg(all(feature = "f64", feature = "matrix", feature = "matrix2"))]
    #[test]
    fn matrix_input_ports_do_not_traverse_value_wrappers() {
        use crate::matrix::Matrix;
        use nalgebra::Matrix2;

        let matrix = LegacyValue::MatrixF64(Matrix::Matrix2(Ref::new(Matrix2::identity())));
        let typed = LegacyValue::Typed(Box::new(matrix.clone()), matrix.kind());
        let mutable = LegacyValue::MutableReference(Ref::new(matrix));

        for wrapped in [typed, mutable] {
            let invocation =
                FunctionInvocation::from(FunctionArgs::Unary(LegacyValue::Empty, wrapped));
            let (_, input) = invocation.expect_unary().unwrap();
            assert!(input.try_matrix::<f64>().is_err());
        }
    }

    #[cfg(feature = "f64")]
    #[test]
    fn invocation_debug_output_is_opaque() {
        let (_, output) = scalar(9_876_543.25);
        let (_, input) = scalar(1_234_567.5);
        let invocation = FunctionInvocation::from(FunctionArgs::Unary(output, input));

        for debug in [
            format!("{invocation:?}"),
            format!("{:?}", invocation.output()),
            format!("{:?}", invocation.input(0).unwrap()),
            format!("{:?}", invocation.inputs()),
        ] {
            assert!(!debug.contains("9876543"));
            assert!(!debug.contains("1234567"));
            assert!(!debug.contains("0x"));
        }
        assert_eq!(
            format!("{invocation:?}"),
            "FunctionInvocation { layout: \"Unary\", input_count: 1 }"
        );
    }

    #[cfg(feature = "f64")]
    #[test]
    fn exact_scalar_refs_are_accepted_without_conversion() {
        let source = Ref::new(1.5_f64);
        let extracted =
            require_function_ref::<f64>(&source.to_value(), FunctionArgumentRole::Input(0))
                .unwrap();
        assert!(source.same_handle(&extracted));

        #[cfg(feature = "i8")]
        {
            let error = require_function_ref::<f64>(
                &LegacyValue::I8(Ref::new(1)),
                FunctionArgumentRole::Input(0),
            )
            .unwrap_err();
            assert_eq!(error.kind_name(), "FunctionArgumentTypeMismatch");
            let mismatch = error.kind_as::<FunctionArgumentTypeMismatch>().unwrap();
            assert_eq!(mismatch.role, FunctionArgumentRole::Input(0));
            assert!(mismatch.expected.contains("f64"));
            assert!(mismatch.found.contains("i8"));
        }
    }

    #[cfg(feature = "f64")]
    #[test]
    fn wrappers_are_not_implicitly_unwrapped() {
        let scalar = Ref::new(2.0_f64).to_value();
        let typed = LegacyValue::Typed(Box::new(scalar), crate::ValueKind::F64);
        assert!(require_function_ref::<f64>(&typed, FunctionArgumentRole::Output).is_err());

        let mutable = LegacyValue::MutableReference(Ref::new(Ref::new(2.0_f64).to_value()));
        assert!(require_function_ref::<f64>(&mutable, FunctionArgumentRole::Output).is_err());
        assert!(
            require_function_ref::<LegacyValue>(&mutable, FunctionArgumentRole::Output).is_ok()
        );
    }

    #[cfg(all(
        feature = "f64",
        feature = "matrix",
        feature = "matrix2",
        feature = "matrixd"
    ))]
    #[test]
    fn matrix_storage_is_part_of_the_exact_contract() {
        use crate::matrix::Matrix;
        use nalgebra::{DMatrix, Matrix2};

        let fixed = LegacyValue::MatrixF64(Matrix::Matrix2(Ref::new(Matrix2::identity())));
        let dynamic = LegacyValue::MatrixF64(Matrix::DMatrix(Ref::new(DMatrix::identity(2, 2))));

        assert!(require_function_ref::<Matrix2<f64>>(&fixed, FunctionArgumentRole::Output).is_ok());
        assert!(
            require_function_ref::<DMatrix<f64>>(&fixed, FunctionArgumentRole::Output).is_err()
        );
        assert!(
            require_function_ref::<Matrix2<f64>>(&dynamic, FunctionArgumentRole::Output).is_err()
        );
    }
}
