#[cfg(feature = "matrix")]
use mech_core::structures::matrix::Matrix;
use mech_core::*;

#[cfg(feature = "matrixd")]
use nalgebra::DMatrix;

#[cfg(feature = "matrixd")]
use itertools::Itertools;
use num_traits::{One, Zero};
use std::fmt::Debug;
use std::ops::{Add, AddAssign, Div, Mul, Sub};
use std::sync::LazyLock;

static PURE_N_CHOOSE_K_SCALAR_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
            ]
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
    });

#[cfg(feature = "matrixd")]
static PURE_N_CHOOSE_K_MATRIX_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
            ]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::Build {
                postcondition: ShapeContractReference {
                    module_path: vec!["combinatorics".to_owned()].into_boxed_slice(),
                    contract_name: "n-choose-k-matrix-output".to_owned(),
                },
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NChooseKResultUnrepresentable {
    pub operand_type: &'static str,
}

impl MechErrorKind for NChooseKResultUnrepresentable {
    fn name(&self) -> &str {
        "NChooseKResultUnrepresentable"
    }

    fn message(&self) -> String {
        format!(
            "n-choose-k result is not representable by operand type {}",
            self.operand_type,
        )
    }
}

fn greatest_common_divisor(mut lhs: u128, mut rhs: u128) -> u128 {
    while rhs != 0 {
        let remainder = lhs % rhs;
        lhs = rhs;
        rhs = remainder;
    }
    lhs
}

/// Computes an exact integer binomial coefficient without forming the
/// potentially overflowing `result * numerator` intermediate used by the
/// generic numeric kernel.
fn checked_integer_n_choose_k(n: u128, k: u128) -> Option<u128> {
    if k > n {
        return Some(0);
    }
    let k = k.min(n - k);
    let mut result = 1_u128;
    for divisor in 1..=k {
        let mut numerator = n - k + divisor;
        let mut denominator = divisor;

        let numerator_gcd = greatest_common_divisor(numerator, denominator);
        numerator /= numerator_gcd;
        denominator /= numerator_gcd;

        let result_gcd = greatest_common_divisor(result, denominator);
        result /= result_gcd;
        denominator /= result_gcd;
        debug_assert_eq!(denominator, 1);

        result = result.checked_mul(numerator)?;
    }
    Some(result)
}

pub trait RuntimeNChooseK: Copy {
    fn runtime_n_choose_k(self, k: Self) -> Option<Self>;
}

trait NChooseKSelection: Copy {
    fn selection_count(self) -> Option<u128>;

    fn result_maximum() -> Option<u128> {
        None
    }
}

macro_rules! impl_runtime_integer_n_choose_k {
    ($($type:ty),+ $(,)?) => {
        $(
            impl RuntimeNChooseK for $type {
                fn runtime_n_choose_k(self, k: Self) -> Option<Self> {
                    let n = u128::try_from(self).ok()?;
                    let k = u128::try_from(k).ok()?;
                    <$type>::try_from(checked_integer_n_choose_k(n, k)?).ok()
                }
            }

            impl NChooseKSelection for $type {
                fn selection_count(self) -> Option<u128> {
                    u128::try_from(self).ok()
                }

                fn result_maximum() -> Option<u128> {
                    u128::try_from(<$type>::MAX).ok()
                }
            }
        )+
    };
}

impl_runtime_integer_n_choose_k!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

impl RuntimeNChooseK for f32 {
    fn runtime_n_choose_k(self, k: Self) -> Option<Self> {
        Some(crate::kernels::n_choose_k::scalar(self, k))
    }
}

impl NChooseKSelection for f32 {
    fn selection_count(self) -> Option<u128> {
        (self.is_finite() && self >= 0.0 && self.fract() == 0.0 && self <= u128::MAX as f32)
            .then_some(self as u128)
    }
}

impl RuntimeNChooseK for f64 {
    fn runtime_n_choose_k(self, k: Self) -> Option<Self> {
        Some(crate::kernels::n_choose_k::scalar(self, k))
    }
}

impl NChooseKSelection for f64 {
    fn selection_count(self) -> Option<u128> {
        (self.is_finite() && self >= 0.0 && self.fract() == 0.0 && self <= u128::MAX as f64)
            .then_some(self as u128)
    }
}

#[cfg(feature = "rational")]
impl RuntimeNChooseK for R64 {
    fn runtime_n_choose_k(self, k: Self) -> Option<Self> {
        let n = u128::try_from(*self.numer()).ok()?;
        let k = u128::try_from(*k.numer()).ok()?;
        let result = i64::try_from(checked_integer_n_choose_k(n, k)?).ok()?;
        Some(R64::new(result, 1))
    }
}


#[cfg(feature = "rational")]
impl NChooseKSelection for R64 {
    fn selection_count(self) -> Option<u128> {
        (*self.denom() == 1)
            .then(|| u128::try_from(*self.numer()).ok())
            .flatten()
    }

    fn result_maximum() -> Option<u128> {
        Some(i64::MAX as u128)
    }
}

#[cfg(feature = "complex")]
impl RuntimeNChooseK for C64 {
    fn runtime_n_choose_k(self, k: Self) -> Option<Self> {
        Some(crate::kernels::n_choose_k::scalar(self, k))
    }
}


#[cfg(feature = "complex")]
impl NChooseKSelection for C64 {
    fn selection_count(self) -> Option<u128> {
        let value = self.0;
        (value.re.is_finite()
            && value.im.is_finite()
            && value.im == 0.0
            && value.re >= 0.0
            && value.re.fract() == 0.0
            && value.re <= u128::MAX as f64)
            .then_some(value.re as u128)
    }
}

fn validate_n_choose_k_typed<T: NChooseKSelection>(n: T, k: T) -> MResult<()> {
    let contract = "n_choose_k_scalar";
    let n = n.selection_count().ok_or_else(|| {
        function_shape_contract_violation(
            contract,
            "input 0 must be a finite, non-negative whole-number scalar",
        )
    })?;
    let k = k.selection_count().ok_or_else(|| {
        function_shape_contract_violation(
            contract,
            "input 1 must be a finite, non-negative whole-number scalar",
        )
    })?;
    let steps = if k > n { 0 } else { k.min(n - k) };
    if steps > crate::kernels::n_choose_k::MAX_SCALAR_STEPS {
        return Err(function_shape_contract_violation(
            contract,
            format!(
                "selection requires {steps} kernel steps, exceeding the bytecode v1 limit of {}",
                crate::kernels::n_choose_k::MAX_SCALAR_STEPS,
            ),
        ));
    }
    if let Some(maximum) = T::result_maximum()
        && checked_integer_n_choose_k(n, k).is_none_or(|result| result > maximum)
    {
        return Err(function_shape_contract_violation(
            contract,
            format!("selection result exceeds the operand representation maximum {maximum}"),
        ));
    }
    Ok(())
}

fn canonical_scalar_selection(value: &ValueData) -> Option<(u128, Option<u128>)> {
    Some(match value {
        #[cfg(feature = "u8")]
        ValueData::U8(value) => ((*value).into(), Some(u8::MAX.into())),
        #[cfg(feature = "u16")]
        ValueData::U16(value) => ((*value).into(), Some(u16::MAX.into())),
        #[cfg(feature = "u32")]
        ValueData::U32(value) => ((*value).into(), Some(u32::MAX.into())),
        #[cfg(feature = "u64")]
        ValueData::U64(value) => ((*value).into(), Some(u64::MAX.into())),
        #[cfg(feature = "u128")]
        ValueData::U128(value) => (*value, Some(u128::MAX)),
        #[cfg(feature = "i8")]
        ValueData::I8(value) => (u128::try_from(*value).ok()?, Some(i8::MAX as u128)),
        #[cfg(feature = "i16")]
        ValueData::I16(value) => (u128::try_from(*value).ok()?, Some(i16::MAX as u128)),
        #[cfg(feature = "i32")]
        ValueData::I32(value) => (u128::try_from(*value).ok()?, Some(i32::MAX as u128)),
        #[cfg(feature = "i64")]
        ValueData::I64(value) => (u128::try_from(*value).ok()?, Some(i64::MAX as u128)),
        #[cfg(feature = "i128")]
        ValueData::I128(value) => (u128::try_from(*value).ok()?, Some(i128::MAX as u128)),
        #[cfg(feature = "f32")]
        ValueData::F32(value) => {
            let value = value.to_f32();
            if !value.is_finite()
                || value < 0.0
                || value.fract() != 0.0
                || value > u128::MAX as f32
            {
                return None;
            }
            (value as u128, None)
        }
        #[cfg(feature = "f64")]
        ValueData::F64(value) => {
            let value = value.to_f64();
            if !value.is_finite()
                || value < 0.0
                || value.fract() != 0.0
                || value > u128::MAX as f64
            {
                return None;
            }
            (value as u128, None)
        }
        #[cfg(feature = "rational")]
        ValueData::Rational64(value) => {
            if value.denominator() != 1 {
                return None;
            }
            (
                u128::try_from(value.numerator()).ok()?,
                Some(i64::MAX as u128),
            )
        }
        #[cfg(feature = "complex")]
        ValueData::Complex64(value) => {
            let real = value.real().to_f64();
            let imaginary = value.imaginary().to_f64();
            if !real.is_finite()
                || !imaginary.is_finite()
                || imaginary != 0.0
                || real < 0.0
                || real.fract() != 0.0
                || real > u128::MAX as f64
            {
                return None;
            }
            (real as u128, None)
        }
        _ => return None,
    })
}

pub(crate) fn validate_canonical_n_choose_k_scalar_contract(
    _output: &ValueCell,
    inputs: &[ValueCell],
) -> MResult<()> {
    let contract = "n_choose_k_scalar";
    let n = inputs
        .first()
        .ok_or_else(|| function_shape_contract_violation(contract, "missing input 0"))?
        .snapshot()?;
    let k = inputs
        .get(1)
        .ok_or_else(|| function_shape_contract_violation(contract, "missing input 1"))?
        .snapshot()?;
    let (n, result_maximum) = canonical_scalar_selection(n.data()).ok_or_else(|| {
        function_shape_contract_violation(
            contract,
            "input 0 must be a finite, non-negative whole-number scalar",
        )
    })?;
    let (k, _) = canonical_scalar_selection(k.data()).ok_or_else(|| {
        function_shape_contract_violation(
            contract,
            "input 1 must be a finite, non-negative whole-number scalar",
        )
    })?;
    let steps = if k > n { 0 } else { k.min(n - k) };
    if steps > crate::kernels::n_choose_k::MAX_SCALAR_STEPS {
        return Err(function_shape_contract_violation(
            contract,
            format!(
                "selection requires {steps} kernel steps, exceeding the bytecode v1 limit of {}",
                crate::kernels::n_choose_k::MAX_SCALAR_STEPS,
            ),
        ));
    }
    if let Some(maximum) = result_maximum
        && checked_integer_n_choose_k(n, k).is_none_or(|result| result > maximum)
    {
        return Err(function_shape_contract_violation(
            contract,
            format!("selection result exceeds the operand representation maximum {maximum}"),
        ));
    }
    Ok(())
}

#[cfg(all(feature = "matrix", feature = "matrixd"))]
fn checked_combination_count(n: usize, k: usize) -> Option<usize> {
    let k = k.min(n.saturating_sub(k));
    let mut result = 1usize;
    for divisor in 1..=k {
        result = result.checked_mul(n - k + divisor)? / divisor;
    }
    Some(result)
}

#[cfg(all(feature = "matrix", feature = "matrixd"))]
pub(crate) fn validate_canonical_n_choose_k_matrix_contract(
    output: &ValueCell,
    inputs: &[ValueCell],
) -> MResult<()> {
    let contract = "n_choose_k_matrix";
    let input = inputs
        .first()
        .ok_or_else(|| function_shape_contract_violation(contract, "missing matrix input"))?;
    let SchemaBody::Matrix {
        dimensions: input_dimensions,
        ..
    } = input.closed_schema_body()?
    else {
        return Err(function_shape_contract_violation(
            contract,
            "input 0 must be matrix-backed",
        ));
    };
    let SchemaBody::Matrix {
        dimensions: output_dimensions,
        ..
    } = output.closed_schema_body()?
    else {
        return Err(function_shape_contract_violation(
            contract,
            "output must be matrix-backed",
        ));
    };
    if !matches!(
        output.representation(),
        FunctionValueRepresentation::Matrix {
            storage: FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::MatrixD),
            ..
        }
    ) {
        return Err(function_shape_contract_violation(
            contract,
            "output must use MatrixD storage",
        ));
    }
    let dimensions = |dimensions: &[DimensionExpr]| -> MResult<(usize, usize)> {
        let [DimensionExpr::Constant(rows), DimensionExpr::Constant(columns)] = dimensions else {
            return Err(function_shape_contract_violation(
                contract,
                "matrix dimensions must be resolved",
            ));
        };
        Ok((
            usize::try_from(*rows).unwrap_or(usize::MAX),
            usize::try_from(*columns).unwrap_or(usize::MAX),
        ))
    };
    let (input_rows, input_columns) = dimensions(&input_dimensions)?;
    let (output_rows, output_columns) = dimensions(&output_dimensions)?;
    let k = inputs
        .get(1)
        .ok_or_else(|| function_shape_contract_violation(contract, "missing input 1"))?
        .snapshot()?;
    let k = canonical_scalar_selection(k.data())
        .and_then(|(value, _)| usize::try_from(value).ok())
        .ok_or_else(|| {
            function_shape_contract_violation(contract, "input 1 must be a selection scalar")
        })?;
    let n = input_rows.checked_mul(input_columns).ok_or_else(|| {
        function_shape_contract_violation(contract, "input element count overflowed usize")
    })?;
    if k == 0 || k > n {
        return Err(function_shape_contract_violation(
            contract,
            format!("selection size {k} is outside 1..={n}"),
        ));
    }
    let combinations = checked_combination_count(n, k).ok_or_else(|| {
        function_shape_contract_violation(contract, "combination count overflowed usize")
    })?;
    if matches!(
        output.extent_evolution(),
        ExtentEvolution::Fixed | ExtentEvolution::ActivationFixed
    ) && (output_rows != k || output_columns != combinations)
    {
        return Err(function_shape_contract_violation(
            contract,
            format!(
                "output is {output_rows}x{output_columns}, expected {k}x{combinations}"
            ),
        ));
    }
    Ok(())
}

#[cfg(all(feature = "matrix", feature = "matrixd"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NChooseKMatrixSelectionInvalid {
    pub available: usize,
    pub requested: Option<usize>,
}

#[cfg(all(feature = "matrix", feature = "matrixd"))]
impl MechErrorKind for NChooseKMatrixSelectionInvalid {
    fn name(&self) -> &str {
        "NChooseKMatrixSelectionInvalid"
    }

    fn message(&self) -> String {
        match self.requested {
            Some(requested) => format!(
                "matrix n-choose-k requested {requested} elements from {} available; expected 1..={}",
                self.available, self.available,
            ),
            None => "matrix n-choose-k selection must be a finite, non-negative whole number representable as usize".to_string(),
        }
    }
}

#[cfg(all(feature = "matrix", feature = "matrixd"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NChooseKMatrixResultTooLarge {
    pub available: usize,
    pub requested: usize,
}

#[cfg(all(feature = "matrix", feature = "matrixd"))]
impl MechErrorKind for NChooseKMatrixResultTooLarge {
    fn name(&self) -> &str {
        "NChooseKMatrixResultTooLarge"
    }

    fn message(&self) -> String {
        format!(
            "matrix n-choose-k result for {} choose {} exceeds addressable allocation",
            self.available, self.requested,
        )
    }
}

#[cfg(all(feature = "matrix", feature = "matrixd"))]
fn invalid_matrix_selection(available: usize, requested: usize) -> MechError {
    MechError::new(
        NChooseKMatrixSelectionInvalid {
            available,
            requested: Some(requested),
        },
        None,
    )
    .with_compiler_loc()
}

#[cfg(all(feature = "matrix", feature = "matrixd"))]
fn invalid_matrix_selection_value() -> MechError {
    MechError::new(
        NChooseKMatrixSelectionInvalid {
            available: 0,
            requested: None,
        },
        None,
    )
    .with_compiler_loc()
}

#[cfg(all(feature = "matrix", feature = "matrixd"))]
fn matrix_result_too_large(available: usize, requested: usize) -> MechError {
    MechError::new(
        NChooseKMatrixResultTooLarge {
            available,
            requested,
        },
        None,
    )
    .with_compiler_loc()
}

// Combinatorics N Choose K----------------------------------------------------

#[derive(Debug)]
pub struct NChooseK<T> {
    n: Ref<T>,
    k: Ref<T>,
    out: Ref<T>,
}
impl<T> MechFunctionFactory for NChooseK<T>
where
    T: Copy
        + Debug
        + Clone
        + Sync
        + Send
        + 'static
        + Add<Output = T>
        + AddAssign
        + Sub<Output = T>
        + Div<Output = T>
        + Zero
        + One
        + FunctionRuntimeType
        + RuntimeNChooseK
        + NChooseKSelection
        + PartialEq
        + PartialOrd,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + ConstElem,
    T: FunctionStateBacking,
{
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::binary(T::REPRESENTATION, T::REPRESENTATION, T::REPRESENTATION);

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, n, k) = invocation.expect_binary()?;
        let n: Ref<T> = n.try_ref()?;
        let k: Ref<T> = k.try_ref()?;
        let out: Ref<T> = out.try_ref()?;
        Ok(Box::new(Self { n, k, out }))
    }

}
impl<T> MechFunctionImpl for NChooseK<T>
where
    T: Copy
        + Debug
        + Clone
        + Sync
        + Send
        + 'static
        + Add<Output = T>
        + AddAssign
        + Sub<Output = T>
        + Div<Output = T>
        + Mul<Output = T>
        + Zero
        + One
        + RuntimeNChooseK
        + NChooseKSelection
        + PartialEq
        + PartialOrd,
    T: FunctionStateBacking,
{
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_ref(&self.out))
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
    }
    fn solve_result(&self) -> MResult<()> {
        // Validate every reactive solve, not only bytecode installation: host
        // or live-resource producers may replace a previously safe operand.
        validate_n_choose_k_typed(*self.n.borrow(), *self.k.borrow())?;
        let next = (*self.n.borrow())
            .runtime_n_choose_k(*self.k.borrow())
            .ok_or_else(|| {
                MechError::new(
                    NChooseKResultUnrepresentable {
                        operand_type: std::any::type_name::<T>(),
                    },
                    None,
                )
                .with_compiler_loc()
            })?;
        *self.out.borrow_mut() = next;
        Ok(())
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_N_CHOOSE_K_SCALAR_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}

#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for NChooseK<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + RuntimeNChooseK,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("NChooseK<{}>", <T as FunctionRuntimeType>::REPRESENTATION);
        compile_binop!(name, self.out, self.n, self.k, ctx);
    }
}
#[cfg(all(feature = "matrix", feature = "matrixd"))]
#[derive(Debug)]
pub struct NChooseKMatrix<T> {
    n: Matrix<T>,
    k: Ref<T>,
    #[cfg(feature = "semantic-compiler")]
    out: Ref<DMatrix<T>>,
    output_value: FunctionValueOutput,
}

#[cfg(all(feature = "matrix", feature = "matrixd"))]
fn n_choose_k_matrix_result<T>(n: &Matrix<T>, requested: usize) -> MResult<DMatrix<T>>
where
    T: Copy + Clone + Debug + PartialEq + 'static,
{
    let elements = n.as_vec();
    let available = elements.len();
    if requested == 0 || requested > available {
        return Err(invalid_matrix_selection(available, requested));
    }
    let combination_count = checked_combination_count(available, requested)
        .ok_or_else(|| matrix_result_too_large(available, requested))?;
    let element_count = requested
        .checked_mul(combination_count)
        .ok_or_else(|| matrix_result_too_large(available, requested))?;
    let mut flat_data = Vec::new();
    flat_data
        .try_reserve_exact(element_count)
        .map_err(|_| matrix_result_too_large(available, requested))?;
    for combination in elements.iter().copied().combinations(requested) {
        flat_data.extend(combination);
    }
    if flat_data.len() != element_count {
        return Err(matrix_result_too_large(available, requested));
    }
    Ok(DMatrix::from_vec(requested, combination_count, flat_data))
}
#[cfg(all(feature = "matrix", feature = "matrixd"))]
impl<T> MechFunctionFactory for NChooseKMatrix<T>
where
    T: Copy
        + CanonicalMatrixElementBacking
        + Debug
        + Clone
        + Sync
        + Send
        + 'static
        + std::fmt::Display
        + Add<Output = T>
        + AddAssign
        + Sub<Output = T>
        + Div<Output = T>
        + Zero
        + One
        + FunctionRuntimeType
        + NChooseKSelection
        + PartialEq
        + PartialOrd,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + ConstElem,
    T: FunctionPortBacking,
    Matrix<T>: FunctionRuntimeType,
    DMatrix<T>: FunctionRuntimeType + FunctionStateBacking,
{
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        <DMatrix<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix<T> as FunctionRuntimeType>::REPRESENTATION,
        T::REPRESENTATION,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, n, k) = invocation.expect_binary()?;
        let output_value = out.value();
        let n: Matrix<T> = n.try_matrix()?;
        let k: Ref<T> = k.try_ref()?;
        #[cfg(feature = "semantic-compiler")]
        let out: Ref<DMatrix<T>> = out.try_ref()?;
        #[cfg(not(feature = "semantic-compiler"))]
        out.try_ref::<DMatrix<T>>()?;
        Ok(Box::new(Self {
            n,
            k,
            #[cfg(feature = "semantic-compiler")]
            out,
            output_value,
        }))
    }

}
#[cfg(all(feature = "matrix", feature = "matrixd"))]
impl<T> MechFunctionImpl for NChooseKMatrix<T>
where
    T: Copy
        + CanonicalMatrixElementBacking
        + Debug
        + Clone
        + Sync
        + Send
        + 'static
        + std::fmt::Display
        + Add<Output = T>
        + AddAssign
        + Sub<Output = T>
        + Div<Output = T>
        + Zero
        + One
        + NChooseKSelection
        + PartialEq
        + PartialOrd,
    DMatrix<T>: FunctionStateBacking,
{
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(self.output_value.state_port())
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![self.output_value.state_port()]))
    }
    fn solve_result(&self) -> MResult<()> {
        let requested = self
            .k
            .borrow()
            .selection_count()
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(invalid_matrix_selection_value)?;
        let next = n_choose_k_matrix_result(&self.n, requested)?;
        let (rows, columns) = (next.nrows(), next.ncols());
        let mut elements = Vec::with_capacity(rows.saturating_mul(columns));
        for row in 0..rows {
            for column in 0..columns {
                elements.push(next[(row, column)].data_draft());
            }
        }
        self.output_value.replace_matrix_drafts(
            vec![rows as u64, columns as u64].into_boxed_slice(),
            elements.into_boxed_slice(),
        )
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_N_CHOOSE_K_MATRIX_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "matrix", feature = "matrixd", feature = "semantic-compiler"))]
impl<T> MechFunctionCompiler for NChooseKMatrix<T>
where
    T: CanonicalMatrixElementBacking + ConstElem + CompileConst + FunctionRuntimeType,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("NChooseKMatrix<{}>", <T as FunctionRuntimeType>::REPRESENTATION);
        let out = compile_register!(self.out, ctx);
        let n = compile_register_mat!(self.n, ctx);
        let k = compile_register_brrw!(self.k, ctx);
        ctx.emit_binop(hash_str(&name), out, n, k);
        Ok(out)
    }
}
#[cfg(feature = "source")]
fn specialize_n_choose_k_scalar<T>(
    n: &SpecializationInput,
    k: &SpecializationInput,
    resolved_output: &ResolvedType,
) -> MResult<SpecializedFunction>
where
    T: FunctionStateBacking,
    NChooseK<T>: MechFunctionFactory,
{
    let invocation = FunctionInvocation::binary(
        n.cell()?
            .detached_clone()?
            .with_resolved_output_type(resolved_output)?,
        n.cell()?.clone(),
        k.cell()?.clone(),
    );
    let implementation = NChooseK::<T>::new_invocation(invocation.clone())?;
    Ok(SpecializedFunction::new(FunctionInstance::new(
        implementation,
        invocation,
    )))
}

#[cfg(all(feature = "source", feature = "matrix", feature = "matrixd"))]
fn specialize_n_choose_k_matrix<T>(
    n: &SpecializationInput,
    k: &SpecializationInput,
    resolved_output: &ResolvedType,
) -> MResult<SpecializedFunction>
where
    T: Copy
        + Debug
        + Clone
        + Sync
        + Send
        + 'static
        + std::fmt::Display
        + Add<Output = T>
        + AddAssign
        + Sub<Output = T>
        + Div<Output = T>
        + Zero
        + One
        + FunctionRuntimeType
        + NChooseKSelection
        + CanonicalMatrixElementBacking
        + FunctionPortBacking
        + PartialEq
        + PartialOrd,
    #[cfg(feature = "semantic-compiler")]
    T: ConstElem + CompileConst,
    Matrix<T>: FunctionRuntimeType,
    DMatrix<T>: FunctionRuntimeType + FunctionStateBacking,
{
    let matrix = n.try_matrix::<T>(0)?;
    let requested = k
        .try_ref::<T>()?
        .borrow()
        .selection_count()
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(invalid_matrix_selection_value)?;
    let output = n_choose_k_matrix_result(&matrix, requested)?;
    let (rows, columns) = (output.nrows(), output.ncols());
    let output = ValueCell::from_exact_matrix_ref(Ref::new(output), rows, columns)?
        .with_resolved_output_type(resolved_output)?;
    let invocation = FunctionInvocation::binary(output, n.cell()?.clone(), k.cell()?.clone());
    let implementation = NChooseKMatrix::<T>::new_invocation(invocation.clone())?;
    Ok(SpecializedFunction::new(FunctionInstance::new(
        implementation,
        invocation,
    )))
}

#[cfg(feature = "source")]
macro_rules! try_n_choose_k_type {
    ($n:ident, $k:ident, $type:ty, $resolved_output:expr) => {{
        let scalar = <NChooseK<$type> as MechFunctionFactory>::SIGNATURE;
        if let RuntimeFunctionInputs::Binary(expected_n, expected_k) = scalar.inputs
            && expected_n.matches($n.representation().unwrap_or(FunctionValueRepresentation::Empty))
            && expected_k.matches($k.representation().unwrap_or(FunctionValueRepresentation::Empty))
        {
            return specialize_n_choose_k_scalar::<$type>($n, $k, $resolved_output);
        }
        #[cfg(all(feature = "matrix", feature = "matrixd"))]
        {
            let matrix = <NChooseKMatrix<$type> as MechFunctionFactory>::SIGNATURE;
            if let RuntimeFunctionInputs::Binary(expected_n, expected_k) = matrix.inputs
                && expected_n.matches($n.representation().unwrap_or(FunctionValueRepresentation::Empty))
                && expected_k.matches($k.representation().unwrap_or(FunctionValueRepresentation::Empty))
            {
                return specialize_n_choose_k_matrix::<$type>($n, $k, $resolved_output);
            }
        }
    }};
}

#[cfg(feature = "source")]
pub struct CombinatoricsNChooseK {}

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for CombinatoricsNChooseK {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        if invocation.len() != 2 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 2,
                    found: invocation.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let n = invocation.input(0).expect("validated n-choose-k input");
        let k = invocation.input(1).expect("validated n-choose-k selection");
        let resolved_output = context.resolved_output(0)?;
        #[cfg(feature = "u8")]
        try_n_choose_k_type!(n, k, u8, resolved_output);
        #[cfg(feature = "u16")]
        try_n_choose_k_type!(n, k, u16, resolved_output);
        #[cfg(feature = "u32")]
        try_n_choose_k_type!(n, k, u32, resolved_output);
        #[cfg(feature = "u64")]
        try_n_choose_k_type!(n, k, u64, resolved_output);
        #[cfg(feature = "u128")]
        try_n_choose_k_type!(n, k, u128, resolved_output);
        #[cfg(feature = "i8")]
        try_n_choose_k_type!(n, k, i8, resolved_output);
        #[cfg(feature = "i16")]
        try_n_choose_k_type!(n, k, i16, resolved_output);
        #[cfg(feature = "i32")]
        try_n_choose_k_type!(n, k, i32, resolved_output);
        #[cfg(feature = "i64")]
        try_n_choose_k_type!(n, k, i64, resolved_output);
        #[cfg(feature = "i128")]
        try_n_choose_k_type!(n, k, i128, resolved_output);
        #[cfg(feature = "f32")]
        try_n_choose_k_type!(n, k, f32, resolved_output);
        #[cfg(feature = "f64")]
        try_n_choose_k_type!(n, k, f64, resolved_output);
        #[cfg(feature = "rational")]
        try_n_choose_k_type!(n, k, R64, resolved_output);
        #[cfg(feature = "complex")]
        try_n_choose_k_type!(n, k, C64, resolved_output);
        Err(MechError::new(
            FunctionArgumentTypeMismatch {
                role: FunctionArgumentRole::Input(0),
                expected: "matching scalar or matrix n-choose-k inputs".into(),
                found: format!(
                    "{:?} and {:?}",
                    n.representation(),
                    k.representation(),
                ),
            },
            None,
        )
        .with_compiler_loc())
    }
}

#[cfg(all(test, feature = "f64"))]
mod canonical_scalar_tests {
    use super::*;

    fn f64_value(cell: &ValueCell) -> f64 {
        let snapshot = cell.snapshot().unwrap();
        let ValueData::F64(value) = snapshot.data() else {
            panic!("expected f64 n-choose-k output")
        };
        value.to_f64()
    }

    #[test]
    fn scalar_n_choose_k_uses_exact_ports_and_typed_rollback() {
        let output = ValueCell::from_exact(0.0_f64).unwrap();
        let alias = output.clone();
        let selection = ValueCell::from_exact(2.0_f64).unwrap();
        let function = NChooseK::<f64>::new_invocation(FunctionInvocation::binary(
            output.clone(),
            ValueCell::from_exact(5.0_f64).unwrap(),
            selection.clone(),
        ))
        .unwrap();
        function.solve_result().unwrap();
        assert_eq!(f64_value(&output), 10.0);
        assert!(output.same_cell(&alias));
        assert_eq!(
            function.reactive_output_cell_ids(),
            vec![output.reactive_cell_id()]
        );

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_state(function.as_ref())?;
            selection.replace(&ValueCell::from_exact(1.5_f64)?.snapshot()?)?;
            assert!(function.solve_result().is_err());
            assert_eq!(f64_value(&output), 10.0);
            output.replace(&ValueCell::from_exact(99.0_f64)?.snapshot()?)?;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(f64_value(&output), 10.0);

        assert!(NChooseK::<f64>::new_invocation(FunctionInvocation::binary(
            ValueCell::from_exact(0.0_f64).unwrap(),
            ValueCell::from_exact(5.0_f64).unwrap(),
            ValueCell::from_exact(2_usize).unwrap(),
        ))
        .is_err());
        assert!(NChooseK::<f64>::new_invocation(FunctionInvocation::unary(
            ValueCell::from_exact(0.0_f64).unwrap(),
            ValueCell::from_exact(5.0_f64).unwrap(),
        ))
        .is_err());
    }
}

#[cfg(all(test, feature = "f64", feature = "matrix2", feature = "matrixd"))]
mod canonical_matrix_tests {
    use super::*;
    use nalgebra::Matrix2;

    #[test]
    fn matrix_n_choose_k_preserves_input_and_output_identity_across_extents() {
        let input = Ref::new(Matrix2::new(1.0_f64, 2.0, 3.0, 4.0));
        let input_alias = input.clone();
        let selection = ValueCell::from_exact(2.0_f64).unwrap();
        let out = Ref::new(DMatrix::<f64>::zeros(2, 1));
        let out_alias = out.clone();
        let output = ValueCell::from_exact_matrix_ref(out.clone(), 2, 1).unwrap();
        let function = NChooseKMatrix::<f64>::new_invocation(FunctionInvocation::binary(
            output.clone(),
            ValueCell::from_exact_matrix_ref(input.clone(), 2, 2).unwrap(),
            selection.clone(),
        ))
        .unwrap();
        function.solve_result().unwrap();
        assert!(input.same_handle(&input_alias));
        assert!(out.same_handle(&out_alias));
        assert_eq!(out.borrow().shape(), (2, 6));

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_state(function.as_ref())?;
            selection.replace(&ValueCell::from_exact(3.0_f64)?.snapshot()?)?;
            function.solve_result()?;
            assert_eq!(out.borrow().shape(), (3, 4));
            selection.replace(&ValueCell::from_exact(0.0_f64)?.snapshot()?)?;
            assert!(function.solve_result().is_err());
            assert_eq!(out.borrow().shape(), (3, 4));
            *out.borrow_mut() = DMatrix::from_element(1, 1, 99.0);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert!(out.same_handle(&out_alias));
        assert_eq!(out.borrow().shape(), (2, 6));

        assert!(NChooseKMatrix::<f64>::new_invocation(FunctionInvocation::binary(
            output,
            ValueCell::from_exact(4.0_f64).unwrap(),
            ValueCell::from_exact(2.0_f64).unwrap(),
        ))
        .is_err());
    }
}
