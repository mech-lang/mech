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

#[cfg(feature = "f64")]
pub(crate) fn bind_resident_n_choose_k_f64(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let ResolvedOperationContract::Declared(contract) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    if contract.interaction != ExternalInteraction::Pure
        || contract.inputs.len() != 2
        || request.inputs.len() != 2
        || contract.outputs.len() != 1
        || contract
            .inputs
            .iter()
            .zip(request.inputs)
            .any(|(port, layout)| {
                port.schema != layout.schema_id
                    || port.access != AccessMode::Read
                    || port.delivery != DeliveryMode::Signal
            })
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    let output = &contract.outputs[0];
    if output.schema != request.output.schema_id
        || output.access != AccessMode::Write
        || output.delivery != DeliveryMode::Signal
        || output.construction
            != (OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            })
        || output.alias != AliasPolicy::NoAlias
        || output.change_detection != ChangeDetectionPolicy::ExactScalar
        || request.inputs.iter().any(|input| {
            input.kind != ResidentValueKind::F64 || input.shape != ResidentShape::SCALAR
        })
        || request.output.kind != ResidentValueKind::F64
        || request.output.shape != ResidentShape::SCALAR
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    Ok(BoundResidentKernel::new_f64_output_2(
        resident_n_choose_k_f64,
        Box::new([]),
    ))
}

#[cfg(feature = "f64")]
fn resident_n_choose_k_f64(
    _kernel: &BoundResidentKernel,
    n: &[f64],
    k: &[f64],
    output: &mut [f64],
) -> Result<bool, ResidentKernelError> {
    let ([n], [k], [output]) = (n, k, output) else {
        return Err(ResidentKernelError::InvalidShape);
    };
    if !crate::kernels::n_choose_k::supports_f64(*n, *k) {
        return Err(ResidentKernelError::InvalidInput);
    }
    let next = crate::kernels::n_choose_k::scalar(*n, *k);
    let changed = output.to_bits() != next.to_bits();
    *output = next;
    Ok(changed)
}

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

#[cfg(test)]
fn scalar_integer_result_max(value: &LegacyValue) -> Option<u128> {
    match value {
        #[cfg(feature = "u8")]
        LegacyValue::U8(_) => Some(u8::MAX.into()),
        #[cfg(feature = "u16")]
        LegacyValue::U16(_) => Some(u16::MAX.into()),
        #[cfg(feature = "u32")]
        LegacyValue::U32(_) => Some(u32::MAX.into()),
        #[cfg(feature = "u64")]
        LegacyValue::U64(_) => Some(u64::MAX.into()),
        #[cfg(feature = "u128")]
        LegacyValue::U128(_) => Some(u128::MAX),
        #[cfg(feature = "i8")]
        LegacyValue::I8(_) => Some(i8::MAX as u128),
        #[cfg(feature = "i16")]
        LegacyValue::I16(_) => Some(i16::MAX as u128),
        #[cfg(feature = "i32")]
        LegacyValue::I32(_) => Some(i32::MAX as u128),
        #[cfg(feature = "i64")]
        LegacyValue::I64(_) => Some(i64::MAX as u128),
        #[cfg(feature = "i128")]
        LegacyValue::I128(_) => Some(i128::MAX as u128),
        #[cfg(feature = "rational")]
        LegacyValue::R64(_) => Some(i64::MAX as u128),
        _ => None,
    }
}

#[cfg(test)]
fn scalar_selection_count(value: &LegacyValue) -> Option<u128> {
    match value {
        #[cfg(feature = "u8")]
        LegacyValue::U8(value) => Some(u128::from(*value.borrow())),
        #[cfg(feature = "u16")]
        LegacyValue::U16(value) => Some(u128::from(*value.borrow())),
        #[cfg(feature = "u32")]
        LegacyValue::U32(value) => Some(u128::from(*value.borrow())),
        #[cfg(feature = "u64")]
        LegacyValue::U64(value) => Some(u128::from(*value.borrow())),
        #[cfg(feature = "u128")]
        LegacyValue::U128(value) => Some(*value.borrow()),
        #[cfg(feature = "i8")]
        LegacyValue::I8(value) => u128::try_from(*value.borrow()).ok(),
        #[cfg(feature = "i16")]
        LegacyValue::I16(value) => u128::try_from(*value.borrow()).ok(),
        #[cfg(feature = "i32")]
        LegacyValue::I32(value) => u128::try_from(*value.borrow()).ok(),
        #[cfg(feature = "i64")]
        LegacyValue::I64(value) => u128::try_from(*value.borrow()).ok(),
        #[cfg(feature = "i128")]
        LegacyValue::I128(value) => u128::try_from(*value.borrow()).ok(),
        #[cfg(feature = "f32")]
        LegacyValue::F32(value) => {
            let value = *value.borrow();
            (value.is_finite() && value >= 0.0 && value.fract() == 0.0 && value <= u128::MAX as f32)
                .then_some(value as u128)
        }
        #[cfg(feature = "f64")]
        LegacyValue::F64(value) => {
            let value = *value.borrow();
            (value.is_finite() && value >= 0.0 && value.fract() == 0.0 && value <= u128::MAX as f64)
                .then_some(value as u128)
        }
        #[cfg(feature = "r64")]
        LegacyValue::R64(value) => {
            let value = value.borrow();
            (*value.denom() == 1)
                .then(|| u128::try_from(*value.numer()).ok())
                .flatten()
        }
        #[cfg(feature = "c64")]
        LegacyValue::C64(value) => {
            let value = value.borrow().0;
            (value.re.is_finite()
                && value.im.is_finite()
                && value.im == 0.0
                && value.re >= 0.0
                && value.re.fract() == 0.0
                && value.re <= u128::MAX as f64)
                .then_some(value.re as u128)
        }
        _ => None,
    }
}

#[cfg(test)]
fn validate_n_choose_k_scalar_values(n: &LegacyValue, k: &LegacyValue) -> MResult<()> {
    let contract = "n_choose_k_scalar";
    let result_maximum = scalar_integer_result_max(n);
    let n = scalar_selection_count(n).ok_or_else(|| {
        function_shape_contract_violation(
            contract,
            "input 0 must be a finite, non-negative whole-number scalar",
        )
    })?;
    let k = scalar_selection_count(k).ok_or_else(|| {
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

#[cfg(test)]
pub(crate) fn validate_n_choose_k_scalar_contract(args: &FunctionArgs) -> MResult<()> {
    let contract = "n_choose_k_scalar";
    let n = args
        .input_value(0)
        .ok_or_else(|| function_shape_contract_violation(contract, "missing input 0"))?;
    let k = args
        .input_value(1)
        .ok_or_else(|| function_shape_contract_violation(contract, "missing input 1"))?;
    validate_n_choose_k_scalar_values(n, k)
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

#[cfg(all(
    test,
    feature = "matrix",
    feature = "matrixd",
    feature = "i8"
))]
fn matrix_selection_size(value: &LegacyValue) -> Option<usize> {
    match value {
        LegacyValue::Index(value) => Some(*value.borrow()),
        _ => usize::try_from(scalar_selection_count(value)?).ok(),
    }
}

#[cfg(all(
    test,
    feature = "matrix",
    feature = "matrixd",
    feature = "i8"
))]
pub(crate) fn validate_n_choose_k_matrix_contract(args: &FunctionArgs) -> MResult<()> {
    let contract = "n_choose_k_matrix";
    let input = args
        .input_value(0)
        .ok_or_else(|| function_shape_contract_violation(contract, "missing matrix input"))?
        .function_matrix_descriptor(FunctionArgumentRole::Input(0))?
        .ok_or_else(|| {
            function_shape_contract_violation(contract, "input 0 must be matrix-backed")
        })?;
    let output = args
        .output_value()
        .function_matrix_descriptor(FunctionArgumentRole::Output)?
        .ok_or_else(|| {
            function_shape_contract_violation(contract, "output must be matrix-backed")
        })?;
    if output.representation != FunctionMatrixRepresentation::MatrixD {
        return Err(function_shape_contract_violation(
            contract,
            format!(
                "output must use MatrixD storage, found {:?}",
                output.representation,
            ),
        ));
    }
    let k = args
        .input_value(1)
        .and_then(matrix_selection_size)
        .ok_or_else(|| {
            function_shape_contract_violation(contract, "input 1 must be a selection scalar")
        })?;
    let n = input.rows.checked_mul(input.cols).ok_or_else(|| {
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
    if output.rows != k || output.cols != combinations {
        return Err(function_shape_contract_violation(
            contract,
            format!(
                "output is {}x{}, expected {k}x{combinations}",
                output.rows, output.cols,
            ),
        ));
    }
    Ok(())
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
        + AsValueKind
        + RuntimeNChooseK
        + NChooseKSelection
        + PartialEq
        + PartialOrd,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + ConstElem,
    Ref<T>: ToValue,
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
    Ref<T>: ToValue,
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

#[cfg(all(test, feature = "f64"))]
mod scalar_contract_tests {
    use super::*;

    fn framed(payload: &[u8]) -> Vec<u8> {
        let mut framed = (payload.len() as u32).to_le_bytes().to_vec();
        framed.extend_from_slice(payload);
        framed
    }

    fn decoded_scalar_wrapper(reference: bool) -> LegacyValue {
        let payload = 1.0_f64.to_le_bytes();
        let (runtime_type, bytes) = if reference {
            (
                RuntimeType::Reference(Box::new(RuntimeType::F64)),
                framed(&payload),
            )
        } else {
            let mut bytes = vec![1];
            bytes.extend(framed(&payload));
            (RuntimeType::Option(Box::new(RuntimeType::F64)), bytes)
        };
        legacy_values_from_encoded_bytecode_constants(&[EncodedConstant {
            runtime_type,
            alignment: 8,
            bytes,
        }])
        .unwrap()
        .pop()
        .unwrap()
    }

    fn factory_error(result: MResult<Box<dyn MechFunction>>) -> MechError {
        match result {
            Ok(_) => panic!("factory unexpectedly accepted invalid arguments"),
            Err(error) => error,
        }
    }

    fn args(n: f64, k: f64) -> FunctionArgs {
        FunctionArgs::Binary(
            LegacyValue::F64(Ref::new(0.0)),
            LegacyValue::F64(Ref::new(n)),
            LegacyValue::F64(Ref::new(k)),
        )
    }

    #[test]
    fn scalar_contract_rejects_non_finite_fractional_and_unbounded_selections() {
        for (n, k) in [
            (10.0, f64::INFINITY),
            (f64::NAN, 1.0),
            (10.0, 1.5),
            (-1.0, 0.0),
            (2_000_002.0, 1_000_001.0),
            (1.0e100, 1.0e50),
        ] {
            let error = validate_n_choose_k_scalar_contract(&args(n, k)).unwrap_err();
            assert_eq!(error.kind_name(), "FunctionShapeContractViolation");
        }
        validate_n_choose_k_scalar_contract(&args(10.0, 2.0)).unwrap();
        validate_n_choose_k_scalar_contract(&args(2.0, 10.0)).unwrap();
    }

    #[test]
    fn scalar_runtime_revalidates_live_selection_values() {
        let k = Ref::new(2.0);
        let function = NChooseK {
            n: Ref::new(10.0),
            k: k.clone(),
            out: Ref::new(0.0),
        };
        function.solve_result().unwrap();
        *k.borrow_mut() = f64::INFINITY;
        let error = function.solve_result().unwrap_err();
        assert_eq!(error.kind_name(), "FunctionShapeContractViolation");
    }

    #[test]
    fn scalar_factory_ports_are_exact_and_restore_output_state() {
        let n = Ref::new(5.0_f64);
        let k = Ref::new(2.0_f64);
        let legacy_out = Ref::new(0.0_f64);
        let invocation_out = Ref::new(0.0_f64);
        let invocation_alias = invocation_out.clone();
        let legacy = NChooseK::<f64>::new(FunctionArgs::Binary(
            legacy_out.to_value(),
            n.to_value(),
            k.to_value(),
        ))
        .unwrap();
        let invocation = NChooseK::<f64>::new_invocation(
            FunctionArgs::Binary(invocation_out.to_value(), n.to_value(), k.to_value()).into(),
        )
        .unwrap();
        legacy.solve_result().unwrap();
        invocation.solve_result().unwrap();
        assert_eq!(*legacy_out.borrow(), *invocation_out.borrow());
        assert_eq!(*invocation_out.borrow(), 10.0);
        assert_eq!(
            invocation.reactive_output_cell_ids(),
            invocation_out.to_value().reactive_root_cell_ids(),
        );

        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*invocation)?;
            *invocation_out.borrow_mut() = 99.0;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert!(invocation_out.same_handle(&invocation_alias));
        assert_eq!(*invocation_out.borrow(), 10.0);
    }

    #[test]
    fn scalar_factory_ports_reject_wrong_layout_type_and_wrappers() {
        let out = Ref::new(0.0_f64);
        let n = Ref::new(5.0_f64);
        let k = Ref::new(2.0_f64);

        let layout = factory_error(NChooseK::<f64>::new_invocation(
            FunctionArgs::Unary(out.to_value(), n.to_value()).into(),
        ));
        let arity = layout.kind_as::<IncorrectNumberOfArguments>().unwrap();
        assert_eq!((arity.expected, arity.found), (2, 1));

        assert!(
            NChooseK::<f64>::new_invocation(
                FunctionArgs::Binary(out.to_value(), Ref::new(5_usize).to_value(), k.to_value(),)
                    .into(),
            )
            .is_err()
        );
        for wrapped in [decoded_scalar_wrapper(false), decoded_scalar_wrapper(true)] {
            assert!(
                NChooseK::<f64>::new_invocation(
                    FunctionArgs::Binary(out.to_value(), wrapped, k.to_value()).into(),
                )
                .is_err()
            );
        }
    }

    #[test]
    fn scalar_error_is_atomic_and_prior_state_remains_restorable() {
        let n = Ref::new(10.0_f64);
        let out = Ref::new(0.0_f64);
        let function = NChooseK::<f64>::new_invocation(
            FunctionArgs::Binary(out.to_value(), n.to_value(), Ref::new(2.0_f64).to_value()).into(),
        )
        .unwrap();
        function.solve_result().unwrap();
        let successful = *out.borrow();

        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*function)?;
            *n.borrow_mut() = f64::NAN;
            let error = function.solve_result().unwrap_err();
            assert_eq!(error.kind_name(), "FunctionShapeContractViolation");
            assert_eq!(*out.borrow(), successful);
            *out.borrow_mut() = -1.0;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(*out.borrow(), successful);
    }
}

#[cfg(all(test, feature = "u8"))]
mod integer_scalar_tests {
    use super::*;

    #[test]
    fn integer_n_choose_k_avoids_intermediate_overflow_and_retains_output_on_error() {
        let n = Ref::new(20_u8);
        let k = Ref::new(2_u8);
        let out = Ref::new(17_u8);
        let legacy_out = Ref::new(0_u8);
        let legacy = NChooseK::<u8>::new(FunctionArgs::Binary(
            legacy_out.to_value(),
            n.to_value(),
            k.to_value(),
        ))
        .unwrap();
        let function = NChooseK::<u8>::new_invocation(
            FunctionArgs::Binary(out.to_value(), n.to_value(), k.to_value()).into(),
        )
        .unwrap();

        legacy.solve_result().unwrap();
        function.solve_result().unwrap();
        assert_eq!(*legacy_out.borrow(), *out.borrow());
        assert_eq!(*out.borrow(), 190);

        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*function)?;
            *n.borrow_mut() = 30;
            let error = function.solve_result().unwrap_err();
            assert_eq!(error.kind_name(), "FunctionShapeContractViolation");
            assert_eq!(*out.borrow(), 190);
            *out.borrow_mut() = 0;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(*out.borrow(), 190);
    }
}

#[cfg(all(test, feature = "i8"))]
mod signed_scalar_port_tests {
    use super::*;

    #[test]
    fn signed_integer_factories_are_equivalent() {
        let legacy_out = Ref::new(0_i8);
        let invocation_out = Ref::new(0_i8);
        let args = |out: &Ref<i8>| {
            FunctionArgs::Binary(
                out.to_value(),
                Ref::new(5_i8).to_value(),
                Ref::new(2_i8).to_value(),
            )
        };
        let legacy = NChooseK::<i8>::new(args(&legacy_out)).unwrap();
        let invocation = NChooseK::<i8>::new_invocation(args(&invocation_out).into()).unwrap();
        legacy.solve_result().unwrap();
        invocation.solve_result().unwrap();
        assert_eq!(*legacy_out.borrow(), *invocation_out.borrow());
    }
}

#[cfg(all(test, feature = "rational"))]
mod rational_scalar_port_tests {
    use super::*;

    #[test]
    fn rational_factories_are_equivalent() {
        let legacy_out = Ref::new(R64::default());
        let invocation_out = Ref::new(R64::default());
        let args = |out: &Ref<R64>| {
            FunctionArgs::Binary(
                out.to_value(),
                Ref::new(R64::new(5, 1)).to_value(),
                Ref::new(R64::new(2, 1)).to_value(),
            )
        };
        let legacy = NChooseK::<R64>::new(args(&legacy_out)).unwrap();
        let invocation = NChooseK::<R64>::new_invocation(args(&invocation_out).into()).unwrap();
        legacy.solve_result().unwrap();
        invocation.solve_result().unwrap();
        assert_eq!(*legacy_out.borrow(), *invocation_out.borrow());
    }
}

#[cfg(all(test, feature = "complex"))]
mod complex_scalar_port_tests {
    use super::*;

    #[test]
    fn complex_factories_are_equivalent() {
        let legacy_out = Ref::new(C64::default());
        let invocation_out = Ref::new(C64::default());
        let args = |out: &Ref<C64>| {
            FunctionArgs::Binary(
                out.to_value(),
                Ref::new(C64::new(5.0, 0.0)).to_value(),
                Ref::new(C64::new(2.0, 0.0)).to_value(),
            )
        };
        let legacy = NChooseK::<C64>::new(args(&legacy_out)).unwrap();
        let invocation = NChooseK::<C64>::new_invocation(args(&invocation_out).into()).unwrap();
        legacy.solve_result().unwrap();
        invocation.solve_result().unwrap();
        assert_eq!(*legacy_out.borrow(), *invocation_out.borrow());
    }
}

#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for NChooseK<T>
where
    T: ConstElem + CompileConst + AsValueKind + RuntimeNChooseK,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("NChooseK<{}>", T::as_value_kind());
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
        + AsValueKind
        + NChooseKSelection
        + PartialEq
        + PartialOrd,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + ConstElem,
    Ref<T>: ToValue,
    Ref<DMatrix<T>>: ToValue,
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
    Ref<T>: ToValue,
    Ref<DMatrix<T>>: ToValue,
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
    T: ConstElem + CompileConst + AsValueKind,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("NChooseKMatrix<{}>", T::as_value_kind());
        let out = compile_register!(self.out, ctx);
        let n = compile_register!(self.n, ctx);
        let k = compile_register_brrw!(self.k, ctx);
        ctx.emit_binop(hash_str(&name), out, n, k);
        Ok(out)
    }
}
#[cfg(all(test, feature = "matrix", feature = "matrixd", feature = "f64"))]
mod transaction_state_tests {
    use super::*;

    fn framed(payload: &[u8]) -> Vec<u8> {
        let mut framed = (payload.len() as u32).to_le_bytes().to_vec();
        framed.extend_from_slice(payload);
        framed
    }

    fn decoded_matrix_wrapper(reference: bool) -> LegacyValue {
        let mut payload = Vec::new();
        payload.extend_from_slice(&2_u32.to_le_bytes());
        payload.extend_from_slice(&1_u32.to_le_bytes());
        payload.extend_from_slice(&1.0_f64.to_le_bytes());
        payload.extend_from_slice(&2.0_f64.to_le_bytes());
        let matrix_type = RuntimeType::Matrix {
            element: Box::new(RuntimeType::F64),
            storage: MatrixStorage::MatrixD,
            rows: 2,
            cols: 1,
        };
        let (runtime_type, bytes) = if reference {
            (
                RuntimeType::Reference(Box::new(matrix_type)),
                framed(&payload),
            )
        } else {
            let mut bytes = vec![1];
            bytes.extend(framed(&payload));
            (RuntimeType::Option(Box::new(matrix_type)), bytes)
        };
        legacy_values_from_encoded_bytecode_constants(&[EncodedConstant {
            runtime_type,
            alignment: 8,
            bytes,
        }])
        .unwrap()
        .pop()
        .unwrap()
    }

    fn decoded_index_matrix() -> LegacyValue {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&2_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u64.to_le_bytes());
        bytes.extend_from_slice(&2_u64.to_le_bytes());
        legacy_values_from_encoded_bytecode_constants(&[EncodedConstant {
            runtime_type: RuntimeType::Matrix {
                element: Box::new(RuntimeType::Index),
                storage: MatrixStorage::MatrixD,
                rows: 2,
                cols: 1,
            },
            alignment: 8,
            bytes,
        }])
        .unwrap()
        .pop()
        .unwrap()
    }

    fn factory_error(result: MResult<Box<dyn MechFunction>>) -> MechError {
        match result {
            Ok(_) => panic!("factory unexpectedly accepted invalid arguments"),
            Err(error) => error,
        }
    }

    #[test]
    fn matrix_combinations_expose_value_backed_transaction_state() {
        let n = f64::to_matrix(vec![1.0, 2.0], 2, 1);
        let out = Ref::new(DMatrix::from_element(1, 1, 0.0));
        let function = NChooseKMatrix::<f64>::new_invocation(
            FunctionArgs::Binary(out.to_value(), n.to_value(), Ref::new(1.0).to_value()).into(),
        )
        .unwrap();
        assert_eq!(function.transaction_state_ports().unwrap().unwrap().len(), 1);
        assert_eq!(
            function.reactive_output_cell_ids(),
            out.to_value().reactive_root_cell_ids()
        );
    }

    #[test]
    fn matrix_factory_ports_preserve_dynamic_input_handles_and_match_legacy_factory() {
        let n_ref = Ref::new(DMatrix::from_vec(3, 1, vec![1.0, 2.0, 3.0]));
        let n_alias = n_ref.clone();
        let n = Matrix::DMatrix(n_ref.clone());
        let k = Ref::new(2.0_f64);
        let legacy_out = Ref::new(DMatrix::zeros(1, 1));
        let invocation_out = Ref::new(DMatrix::zeros(1, 1));
        let invocation_alias = invocation_out.clone();
        let legacy = NChooseKMatrix::<f64>::new(FunctionArgs::Binary(
            legacy_out.to_value(),
            n.to_value(),
            k.to_value(),
        ))
        .unwrap();
        let invocation = NChooseKMatrix::<f64>::new_invocation(
            FunctionArgs::Binary(invocation_out.to_value(), n.to_value(), k.to_value()).into(),
        )
        .unwrap();

        n_ref.borrow_mut()[(0, 0)] = 4.0;
        legacy.solve_result().unwrap();
        invocation.solve_result().unwrap();

        assert!(n_ref.same_handle(&n_alias));
        assert!(invocation_out.same_handle(&invocation_alias));
        assert_eq!(*legacy_out.borrow(), *invocation_out.borrow());
        assert_eq!(invocation_out.borrow().shape(), (2, 3));
        assert_eq!(
            invocation_out.borrow().as_slice(),
            &[4.0, 2.0, 4.0, 3.0, 2.0, 3.0]
        );
        assert_eq!(
            invocation.reactive_output_cell_ids(),
            invocation_out.to_value().reactive_root_cell_ids(),
        );

        let successful = invocation_out.borrow().clone();
        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*invocation)?;
            *invocation_out.borrow_mut() = DMatrix::from_element(2, 2, 99.0);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert!(invocation_out.same_handle(&invocation_alias));
        assert_eq!(*invocation_out.borrow(), successful);
    }

    #[test]
    fn matrix_factory_ports_reject_layout_scalar_element_type_and_wrappers() {
        let out: Ref<DMatrix<f64>> = Ref::new(DMatrix::zeros(1, 1));
        let k = Ref::new(1.0_f64);
        let n = f64::to_matrixd(vec![1.0, 2.0], 2, 1);

        let layout = factory_error(NChooseKMatrix::<f64>::new_invocation(
            FunctionArgs::Unary(out.to_value(), n.to_value()).into(),
        ));
        let arity = layout.kind_as::<IncorrectNumberOfArguments>().unwrap();
        assert_eq!((arity.expected, arity.found), (2, 1));

        for wrong_input in [
            Ref::new(1.0_f64).to_value(),
            decoded_index_matrix(),
            decoded_matrix_wrapper(false),
            decoded_matrix_wrapper(true),
        ] {
            assert!(
                NChooseKMatrix::<f64>::new_invocation(
                    FunctionArgs::Binary(out.to_value(), wrong_input, k.to_value()).into(),
                )
                .is_err()
            );
        }
    }

    #[test]
    fn matrix_combinations_reuse_one_output_root_across_shape_changes() {
        let n = f64::to_matrix(vec![1.0, 2.0, 3.0], 3, 1);
        let k = Ref::new(1.0);
        let out = Ref::new(DMatrix::from_element(1, 1, 0.0));
        let function = NChooseKMatrix::<f64>::new_invocation(
            FunctionArgs::Binary(out.to_value(), n.to_value(), k.to_value()).into(),
        )
        .unwrap();
        let output_alias = out.clone();

        function.solve_result().unwrap();
        assert_eq!(out.borrow().shape(), (1, 3));
        assert_eq!(out.borrow().as_slice(), &[1.0, 2.0, 3.0]);

        *k.borrow_mut() = 2.0;
        function.solve_result().unwrap();
        assert!(out.same_handle(&output_alias));
        assert_eq!(out.borrow().shape(), (2, 3));
        assert_eq!(out.borrow().as_slice(), &[1.0, 2.0, 1.0, 3.0, 2.0, 3.0]);

        *k.borrow_mut() = 3.0;
        function.solve_result().unwrap();
        assert!(out.same_handle(&output_alias));
        assert_eq!(out.borrow().shape(), (3, 1));
        assert_eq!(out.borrow().as_slice(), &[1.0, 2.0, 3.0]);
    }

    #[test]
    fn invalid_matrix_selection_is_structured_and_retains_previous_output() {
        let n = f64::to_matrix(vec![1.0, 2.0, 3.0], 3, 1);
        let k = Ref::new(1.0);
        let out = Ref::new(DMatrix::from_element(1, 1, 0.0));
        let function = NChooseKMatrix::<f64>::new_invocation(
            FunctionArgs::Binary(out.to_value(), n.to_value(), k.to_value()).into(),
        )
        .unwrap();
        function.solve_result().unwrap();
        let expected = out.borrow().clone();
        let out_alias = out.clone();

        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*function)?;
            for invalid in [0.0, 4.0] {
                *k.borrow_mut() = invalid;
                let error = function.solve_result().unwrap_err();
                assert_eq!(error.kind_name(), "NChooseKMatrixSelectionInvalid");
                assert_eq!(*out.borrow(), expected);
            }
            *out.borrow_mut() = DMatrix::from_element(2, 2, -1.0);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert!(out.same_handle(&out_alias));
        assert_eq!(*out.borrow(), expected);
    }
}

#[cfg(all(
    test,
    feature = "matrix",
    feature = "matrix2",
    feature = "matrixd",
    feature = "f64"
))]
mod fixed_matrix_port_tests {
    use super::*;
    use nalgebra::Matrix2;

    #[test]
    fn fixed_matrix_input_retains_exact_representation_and_inner_handle() {
        let n_ref = Ref::new(Matrix2::from_vec(vec![1.0, 2.0, 3.0, 4.0]));
        let n_alias = n_ref.clone();
        let n = Matrix::Matrix2(n_ref.clone());
        let out: Ref<DMatrix<f64>> = Ref::new(DMatrix::zeros(1, 1));
        let function = NChooseKMatrix::<f64>::new_invocation(
            FunctionArgs::Binary(out.to_value(), n.to_value(), Ref::new(1.0_f64).to_value()).into(),
        )
        .unwrap();

        n_ref.borrow_mut()[(0, 0)] = 8.0;
        function.solve_result().unwrap();

        assert!(n_ref.same_handle(&n_alias));
        assert_eq!(out.borrow().shape(), (1, 4));
        assert_eq!(out.borrow().as_slice(), &[8.0, 2.0, 3.0, 4.0]);
    }
}

#[cfg(all(test, feature = "matrix", feature = "matrixd", feature = "i8"))]
mod signed_matrix_selection_tests {
    use super::*;

    #[test]
    fn negative_matrix_selection_is_structured_during_planning_and_reactive_solves() {
        let n = i8::to_matrix(vec![1, 2, 3], 3, 1);
        let k = Ref::new(-1_i8);
        let out = Ref::new(DMatrix::from_element(1, 1, 19_i8));
        let args = FunctionArgs::Binary(out.to_value(), n.to_value(), k.to_value());

        let planning_error = validate_n_choose_k_matrix_contract(&args).unwrap_err();
        assert_eq!(planning_error.kind_name(), "FunctionShapeContractViolation");

        let function = NChooseKMatrix::<i8>::new_invocation(args.clone().into()).unwrap();
        *k.borrow_mut() = 1;
        function.solve_result().unwrap();
        let previous = out.borrow().clone();

        *k.borrow_mut() = -1;
        let runtime_error = function.solve_result().unwrap_err();
        assert_eq!(runtime_error.kind_name(), "NChooseKMatrixSelectionInvalid");
        assert_eq!(*out.borrow(), previous);
    }
}

#[cfg(feature = "source")]
fn specialize_n_choose_k_scalar<T>(
    n: &SpecializationInput,
    k: &SpecializationInput,
) -> MResult<SpecializedFunction>
where
    T: FunctionStateBacking,
    NChooseK<T>: MechFunctionFactory,
{
    let invocation = FunctionInvocation::binary(
        n.cell()?.detached_clone()?,
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
        + AsValueKind
        + NChooseKSelection
        + CanonicalMatrixElementBacking
        + FunctionPortBacking
        + PartialEq
        + PartialOrd,
    #[cfg(feature = "semantic-compiler")]
    T: ConstElem + CompileConst,
    Ref<T>: ToValue,
    Ref<DMatrix<T>>: ToValue,
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
    let output = ValueCell::from_exact_matrix_ref(Ref::new(output), rows, columns)?;
    let invocation = FunctionInvocation::binary(output, n.cell()?.clone(), k.cell()?.clone());
    let implementation = NChooseKMatrix::<T>::new_invocation(invocation.clone())?;
    Ok(SpecializedFunction::new(FunctionInstance::new(
        implementation,
        invocation,
    )))
}

#[cfg(feature = "source")]
macro_rules! try_n_choose_k_type {
    ($n:ident, $k:ident, $type:ty) => {{
        let scalar = <NChooseK<$type> as MechFunctionFactory>::SIGNATURE;
        if let RuntimeFunctionInputs::Binary(expected_n, expected_k) = scalar.inputs
            && expected_n.matches($n.representation().unwrap_or(FunctionValueRepresentation::Empty))
            && expected_k.matches($k.representation().unwrap_or(FunctionValueRepresentation::Empty))
        {
            return specialize_n_choose_k_scalar::<$type>($n, $k);
        }
        #[cfg(all(feature = "matrix", feature = "matrixd"))]
        {
            let matrix = <NChooseKMatrix<$type> as MechFunctionFactory>::SIGNATURE;
            if let RuntimeFunctionInputs::Binary(expected_n, expected_k) = matrix.inputs
                && expected_n.matches($n.representation().unwrap_or(FunctionValueRepresentation::Empty))
                && expected_k.matches($k.representation().unwrap_or(FunctionValueRepresentation::Empty))
            {
                return specialize_n_choose_k_matrix::<$type>($n, $k);
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
        _context: &mut SpecializationContext<'_>,
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
        #[cfg(feature = "u8")]
        try_n_choose_k_type!(n, k, u8);
        #[cfg(feature = "u16")]
        try_n_choose_k_type!(n, k, u16);
        #[cfg(feature = "u32")]
        try_n_choose_k_type!(n, k, u32);
        #[cfg(feature = "u64")]
        try_n_choose_k_type!(n, k, u64);
        #[cfg(feature = "u128")]
        try_n_choose_k_type!(n, k, u128);
        #[cfg(feature = "i8")]
        try_n_choose_k_type!(n, k, i8);
        #[cfg(feature = "i16")]
        try_n_choose_k_type!(n, k, i16);
        #[cfg(feature = "i32")]
        try_n_choose_k_type!(n, k, i32);
        #[cfg(feature = "i64")]
        try_n_choose_k_type!(n, k, i64);
        #[cfg(feature = "i128")]
        try_n_choose_k_type!(n, k, i128);
        #[cfg(feature = "f32")]
        try_n_choose_k_type!(n, k, f32);
        #[cfg(feature = "f64")]
        try_n_choose_k_type!(n, k, f64);
        #[cfg(feature = "rational")]
        try_n_choose_k_type!(n, k, R64);
        #[cfg(feature = "complex")]
        try_n_choose_k_type!(n, k, C64);
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
