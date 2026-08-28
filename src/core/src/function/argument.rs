#[cfg(feature = "no_std")]
use alloc::string::{String, ToString};
#[cfg(all(feature = "no_std", test))]
use alloc::vec;
#[cfg(not(feature = "no_std"))]
use std::string::{String, ToString};

use core::{any::type_name, fmt};

#[cfg(feature = "matrix")]
use crate::structures::Matrix;
use crate::{
    FunctionArgs, FunctionRuntimeType, IncorrectNumberOfArguments, LegacyValue, MResult, MechError,
    MechErrorKind, ReactiveCellId, Ref, RuntimeFunctionSignature,
};

mod function_port_backing {
    pub trait Sealed {}
}

/// An exact runtime backing type that may be extracted through a function port.
///
/// This sealed marker deliberately excludes [`LegacyValue`], [`crate::ValueCell`],
/// and reference wrappers around either type. Universal legacy values remain
/// available only through the legacy factory boundary.
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

#[cfg(feature = "atom")]
impl function_port_backing::Sealed for crate::MechAtom {}

#[cfg(feature = "enum")]
impl function_port_backing::Sealed for crate::MechEnum {}

#[cfg(feature = "record")]
impl function_port_backing::Sealed for crate::MechRecord {}

#[cfg(feature = "map")]
impl function_port_backing::Sealed for crate::MechMap {}

#[cfg(feature = "set")]
impl function_port_backing::Sealed for crate::MechSet {}

#[cfg(feature = "table")]
impl function_port_backing::Sealed for crate::MechTable {}

#[cfg(feature = "tuple")]
impl function_port_backing::Sealed for crate::MechTuple {}

#[cfg(feature = "matrix")]
impl<T: FunctionPortBacking> function_port_backing::Sealed for Matrix<T> {}

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
    args: FunctionArgs,
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

impl From<FunctionArgs> for FunctionInvocation {
    fn from(args: FunctionArgs) -> Self {
        Self { args }
    }
}

impl FunctionInvocation {
    pub fn input_count(&self) -> usize {
        self.legacy_args().input_count()
    }

    pub fn output(&self) -> FunctionOutputPort<'_> {
        FunctionOutputPort { invocation: self }
    }

    pub fn input(&self, index: usize) -> Option<FunctionInputPort<'_>> {
        self.args.input_value(index).map(|_| FunctionInputPort {
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
        if matches!(self.args, FunctionArgs::Nullary(_)) {
            Ok(self.output())
        } else {
            Err(self.layout_error(0))
        }
    }

    pub fn expect_unary(&self) -> MResult<(FunctionOutputPort<'_>, FunctionInputPort<'_>)> {
        if matches!(self.args, FunctionArgs::Unary(_, _)) {
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
        if matches!(self.args, FunctionArgs::Binary(_, _, _)) {
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
        if matches!(self.args, FunctionArgs::Ternary(_, _, _, _)) {
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
        if matches!(self.args, FunctionArgs::Quaternary(_, _, _, _, _)) {
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
        if matches!(self.args, FunctionArgs::Variadic(_, _)) {
            Ok((self.output(), self.inputs()))
        } else {
            Err(self.layout_error(self.input_count()))
        }
    }

    pub(crate) fn normalize_for_signature(self, signature: RuntimeFunctionSignature) -> Self {
        Self {
            args: self.args.normalize_for_signature(signature),
        }
    }

    pub(crate) fn legacy_args(&self) -> &FunctionArgs {
        &self.args
    }

    pub(crate) fn into_legacy_args(self) -> FunctionArgs {
        self.args
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
        match self.args {
            FunctionArgs::Nullary(_) => "Nullary",
            FunctionArgs::Unary(_, _) => "Unary",
            FunctionArgs::Binary(_, _, _) => "Binary",
            FunctionArgs::Ternary(_, _, _, _) => "Ternary",
            FunctionArgs::Quaternary(_, _, _, _, _) => "Quaternary",
            FunctionArgs::Variadic(_, _) => "Variadic",
        }
    }
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
    /// use mech_core::{FunctionArgs, FunctionInvocation, LegacyValue, Ref};
    ///
    /// let legacy_cell = Ref::new(LegacyValue::Empty);
    /// let invocation = FunctionInvocation::from(FunctionArgs::Unary(
    ///     LegacyValue::Empty,
    ///     LegacyValue::MutableReference(legacy_cell),
    /// ));
    /// let (_, input) = invocation.expect_unary().unwrap();
    /// let _: Ref<LegacyValue> = input.try_ref::<LegacyValue>().unwrap();
    /// ```
    pub fn try_ref<T: FunctionPortBacking>(self) -> MResult<Ref<T>> {
        require_function_ref(
            self.invocation
                .args
                .input_value(self.index)
                .expect("function input port index remains valid"),
            FunctionArgumentRole::Input(self.index),
        )
    }

    /// Extracts the exact typed matrix input wrapper without exposing legacy values.
    ///
    /// ```compile_fail
    /// use mech_core::matrix::Matrix;
    /// use mech_core::{FunctionArgs, FunctionInvocation, LegacyValue};
    ///
    /// let invocation = FunctionInvocation::from(FunctionArgs::Unary(
    ///     LegacyValue::Empty,
    ///     LegacyValue::Empty,
    /// ));
    /// let (_, input) = invocation.expect_unary().unwrap();
    /// let _: Matrix<LegacyValue> = input.try_matrix::<LegacyValue>().unwrap();
    /// ```
    #[cfg(feature = "matrix")]
    pub fn try_matrix<T>(self) -> MResult<Matrix<T>>
    where
        T: FunctionPortBacking + Clone,
    {
        self.invocation
            .args
            .input_value(self.index)
            .expect("function input port index remains valid")
            .try_function_matrix(FunctionArgumentRole::Input(self.index))
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
    /// use mech_core::{FunctionArgs, FunctionInvocation, LegacyValue, Ref};
    ///
    /// let legacy_cell = Ref::new(LegacyValue::Empty);
    /// let invocation = FunctionInvocation::from(FunctionArgs::Nullary(
    ///     LegacyValue::MutableReference(legacy_cell),
    /// ));
    /// let output = invocation.expect_nullary().unwrap();
    /// let _: Ref<LegacyValue> = output.try_ref::<LegacyValue>().unwrap();
    /// ```
    pub fn try_ref<T: FunctionPortBacking>(self) -> MResult<Ref<T>> {
        require_function_ref(
            self.invocation.args.output_value(),
            FunctionArgumentRole::Output,
        )
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

/// Identifies the argument whose exact runtime representation was rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FunctionArgumentRole {
    Output,
    Input(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionArgumentTypeMismatch {
    pub role: FunctionArgumentRole,
    pub expected: String,
    pub found: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FunctionMatrixRepresentation {
    Matrix1,
    Matrix2,
    Matrix3,
    Matrix4,
    Matrix2x3,
    Matrix3x2,
    RowVector2,
    RowVector3,
    RowVector4,
    Vector2,
    Vector3,
    Vector4,
    RowVectorD,
    VectorD,
    MatrixD,
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
fn matrix_descriptor<T>(matrix: &Matrix<T>) -> FunctionMatrixDescriptor
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

impl LegacyValue {
    pub fn function_matrix_descriptor(
        &self,
        role: FunctionArgumentRole,
    ) -> MResult<Option<FunctionMatrixDescriptor>> {
        let descriptor = match self {
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixIndex(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "bool"))]
            LegacyValue::MatrixBool(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "u8"))]
            LegacyValue::MatrixU8(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "u16"))]
            LegacyValue::MatrixU16(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "u32"))]
            LegacyValue::MatrixU32(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "u64"))]
            LegacyValue::MatrixU64(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "u128"))]
            LegacyValue::MatrixU128(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "i8"))]
            LegacyValue::MatrixI8(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "i16"))]
            LegacyValue::MatrixI16(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "i32"))]
            LegacyValue::MatrixI32(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "i64"))]
            LegacyValue::MatrixI64(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "i128"))]
            LegacyValue::MatrixI128(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "f32"))]
            LegacyValue::MatrixF32(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "f64"))]
            LegacyValue::MatrixF64(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "string"))]
            LegacyValue::MatrixString(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "rational"))]
            LegacyValue::MatrixR64(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "complex"))]
            LegacyValue::MatrixC64(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixValue(matrix) => Some(matrix_descriptor(matrix)),
            LegacyValue::Typed(_, _) | LegacyValue::MutableReference(_) => {
                return Err(MechError::new(
                    FunctionArgumentTypeMismatch {
                        role,
                        expected: "an unwrapped scalar, nonmatrix, or exact matrix backing"
                            .to_string(),
                        found: self.exact_runtime_representation_name(),
                    },
                    None,
                )
                .with_compiler_loc());
            }
            _ => None,
        };
        Ok(descriptor)
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

/// Extracts only an exact `Ref<T>` backing representation.
///
/// This deliberately performs no scalar conversion, matrix reshaping, or
/// unwrapping of `Typed` and `MutableReference` values.
pub fn require_function_ref<T: 'static>(
    value: &LegacyValue,
    role: FunctionArgumentRole,
) -> MResult<Ref<T>> {
    value
        .exact_ref_any()
        .and_then(|backing| backing.downcast_ref::<Ref<T>>())
        .cloned()
        .ok_or_else(|| {
            MechError::new(
                FunctionArgumentTypeMismatch {
                    role,
                    expected: type_name::<Ref<T>>().to_string(),
                    found: value.exact_runtime_representation_name(),
                },
                None,
            )
            .with_compiler_loc()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToValue;

    #[cfg(feature = "f64")]
    fn scalar(value: f64) -> (Ref<f64>, LegacyValue) {
        let reference = Ref::new(value);
        let value = reference.to_value();
        (reference, value)
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
