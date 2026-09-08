#[cfg(feature = "matrix")]
use mech_core::structures::matrix::Matrix;
use mech_core::*;

#[cfg(feature = "matrixd")]
use nalgebra::DMatrix;

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
            if !value.is_finite() || value < 0.0 || value.fract() != 0.0 || value > u128::MAX as f32
            {
                return None;
            }
            (value as u128, None)
        }
        #[cfg(feature = "f64")]
        ValueData::F64(value) => {
            let value = value.to_f64();
            if !value.is_finite() || value < 0.0 || value.fract() != 0.0 || value > u128::MAX as f64
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
        let [
            DimensionExpr::Constant(rows),
            DimensionExpr::Constant(columns),
        ] = dimensions
        else {
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
            format!("output is {output_rows}x{output_columns}, expected {k}x{combinations}"),
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
    n: ManagedPort<T>,
    k: ManagedPort<T>,
    out: ManagedPort<T>,
}
impl<T> MechFunctionFactory for NChooseK<T>
where
    T: ManagedElement
        + CanonicalMatrixElementBacking
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
        + FunctionPortBacking
        + PartialEq
        + PartialOrd,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + ConstElem,
    T: FunctionStateBacking,
{
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::binary(T::REPRESENTATION, T::REPRESENTATION, T::REPRESENTATION);

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, n, k) = invocation.expect_binary()?;
        Ok(Box::new(Self {
            n: n.try_managed_element::<T>()?,
            k: k.try_managed_element::<T>()?,
            out: out.try_managed_element::<T>()?,
        }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_N_CHOOSE_K_SCALAR_CONTRACT)
    }
}
impl<T> MechFunctionImpl for NChooseK<T>
where
    T: ManagedElement
        + CanonicalMatrixElementBacking
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
        Some(FunctionStatePort::from_cell(self.out.cell()))
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_cell(self.out.cell())]))
    }
    fn solve_managed(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        frame.with_binary_port_views(&self.n, &self.k, &self.out, |n, k, out| {
            if n.len() != 1 || k.len() != 1 || out.len() != 1 {
                return Err(function_shape_contract_violation(
                    "n_choose_k_scalar",
                    "scalar n-choose-k requires scalar input and output ports",
                ));
            }
            let n = n.get_column_major(0).expect("validated scalar n");
            let k = k.get_column_major(0).expect("validated scalar k");
            validate_n_choose_k_typed(n, k)?;
            let next = n.runtime_n_choose_k(k).ok_or_else(|| {
                MechError::new(
                    NChooseKResultUnrepresentable {
                        operand_type: std::any::type_name::<T>(),
                    },
                    None,
                )
                .with_compiler_loc()
            })?;
            out.try_fill_column_major(|_| Ok(next))
        })?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
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
        let output = compile_value_cell_register(self.out.cell(), ctx)?;
        let n = compile_value_cell_register(self.n.cell(), ctx)?;
        let k = compile_value_cell_register(self.k.cell(), ctx)?;
        ctx.emit_binop(hash_str(&name), output, n, k);
        Ok(output)
    }
}
#[cfg(all(feature = "matrix", feature = "matrixd"))]
#[derive(Debug)]
pub struct NChooseKMatrix<T> {
    n: ManagedPort<T>,
    k: ManagedPort<T>,
    out: ManagedPort<T>,
}

#[cfg(all(feature = "matrix", feature = "matrixd"))]
fn combination_element(
    available: usize,
    requested: usize,
    mut rank: usize,
    position: usize,
) -> Option<usize> {
    let mut previous = 0usize;
    for selected_position in 0..=position {
        let remaining = requested.checked_sub(selected_position + 1)?;
        let maximum = available.checked_sub(remaining + 1)?;
        let mut selected = previous;
        loop {
            if selected > maximum {
                return None;
            }
            let suffix = checked_combination_count(available - selected - 1, remaining)?;
            if rank < suffix {
                break;
            }
            rank = rank.checked_sub(suffix)?;
            selected = selected.checked_add(1)?;
        }
        previous = selected.checked_add(1)?;
        if selected_position == position {
            return Some(selected);
        }
    }
    None
}
#[cfg(all(feature = "matrix", feature = "matrixd"))]
impl<T> MechFunctionFactory for NChooseKMatrix<T>
where
    T: ManagedElement
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

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, n, k) = invocation.expect_binary()?;
        let representation = n.value().representation();
        if !matches!(representation, FunctionValueRepresentation::Matrix { .. }) {
            return Err(MechError::new(
                FunctionArgumentTypeMismatch {
                    role: FunctionArgumentRole::Input(0),
                    expected: "matrix-backed n-choose-k input".into(),
                    found: format!("{representation:?}"),
                },
                None,
            )
            .with_compiler_loc());
        }
        let _ = out.try_managed::<DMatrix<T>>()?;
        Ok(Box::new(Self {
            n: n.try_managed_element::<T>()?,
            k: k.try_managed_element::<T>()?,
            out: out.try_managed_element::<T>()?,
        }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_N_CHOOSE_K_MATRIX_CONTRACT)
    }
}
#[cfg(all(feature = "matrix", feature = "matrixd"))]
impl<T> MechFunctionImpl for NChooseKMatrix<T>
where
    T: ManagedElement
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
        Some(FunctionStatePort::from_cell(self.out.cell()))
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_cell(self.out.cell())]))
    }
    fn planned_output_shapes(&self) -> MResult<Option<Box<[ShapeInstance]>>> {
        let shape = self.n.cell().shape();
        let SchemaBody::Matrix { dimensions, .. } = self.n.cell().closed_schema_body()? else {
            return Err(function_shape_contract_violation(
                "n_choose_k_matrix",
                "input 0 must be matrix-backed",
            ));
        };
        let available = dimensions
            .iter()
            .map(|dimension| shape.resolve_dimension(dimension))
            .try_fold(1usize, |count, extent| {
                let extent = usize::try_from(extent.ok()?).ok()?;
                count.checked_mul(extent)
            })
            .ok_or_else(|| matrix_result_too_large(usize::MAX, 0))?;
        let selection = self.k.cell().snapshot()?;
        let requested = T::from_data(selection.data())
            .and_then(NChooseKSelection::selection_count)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(invalid_matrix_selection_value)?;
        if requested == 0 || requested > available {
            return Err(invalid_matrix_selection(available, requested));
        }
        let combinations = checked_combination_count(available, requested)
            .ok_or_else(|| matrix_result_too_large(available, requested))?;
        let output = self.out.cell().snapshot()?;
        let schemas = output.schemas().ok_or_else(|| {
            function_shape_contract_violation("n_choose_k_matrix", "missing output schema table")
        })?;
        let schema = schemas.entry(output.schema()).ok_or_else(|| {
            function_shape_contract_violation("n_choose_k_matrix", "missing output schema entry")
        })?;
        Ok(Some(
            vec![
                shape_for_resolved_extents(
                    schema.schema(),
                    &[requested as u64, combinations as u64],
                )
                .map_err(|error| MechError::new(error, None).with_compiler_loc())?,
            ]
            .into_boxed_slice(),
        ))
    }
    fn solve_managed(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        frame.with_binary_port_views(&self.n, &self.k, &self.out, |n, k, out| {
            if k.len() != 1 {
                return Err(invalid_matrix_selection_value());
            }
            let requested = k
                .get_column_major(0)
                .and_then(NChooseKSelection::selection_count)
                .and_then(|value| usize::try_from(value).ok())
                .ok_or_else(invalid_matrix_selection_value)?;
            let available = n.len();
            if requested == 0 || requested > available {
                return Err(invalid_matrix_selection(available, requested));
            }
            let combinations = checked_combination_count(available, requested)
                .ok_or_else(|| matrix_result_too_large(available, requested))?;
            if out.rows() != requested || out.columns() != combinations {
                return Err(function_shape_contract_violation(
                    "n_choose_k_matrix",
                    "resolved output geometry disagrees with the managed stage",
                ));
            }
            out.try_fill_column_major(|index| {
                let row = index % requested;
                let column = index / requested;
                let source = combination_element(available, requested, column, row)
                    .ok_or_else(|| matrix_result_too_large(available, requested))?;
                Ok(n.get_column_major(source)
                    .expect("validated n-choose-k source element"))
            })
        })?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
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
        let name = format!(
            "NChooseKMatrix<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        let out = compile_value_cell_register(self.out.cell(), ctx)?;
        let n = compile_value_cell_register(self.n.cell(), ctx)?;
        let k = compile_value_cell_register(self.k.cell(), ctx)?;
        ctx.emit_binop(hash_str(&name), out, n, k);
        Ok(out)
    }
}
#[cfg(feature = "source")]
#[cfg(all(feature = "source", feature = "matrix", feature = "matrixd"))]
fn n_choose_k_matrix_extents<T>(
    n: &SpecializationInput,
    k: &SpecializationInput,
) -> MResult<Box<[u64]>>
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
    let descriptor = n.matrix_descriptor()?.ok_or_else(|| {
        function_shape_contract_violation("n_choose_k_matrix", "input 0 must be matrix-backed")
    })?;
    let selection = k.cell()?.snapshot()?;
    let requested = T::from_data(selection.data())
        .and_then(NChooseKSelection::selection_count)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(invalid_matrix_selection_value)?;
    let available = descriptor
        .rows
        .checked_mul(descriptor.cols)
        .ok_or_else(|| matrix_result_too_large(descriptor.rows, requested))?;
    if requested == 0 || requested > available {
        return Err(invalid_matrix_selection(available, requested));
    }
    let combinations = checked_combination_count(available, requested)
        .ok_or_else(|| matrix_result_too_large(available, requested))?;
    Ok(vec![requested as u64, combinations as u64].into_boxed_slice())
}

#[cfg(feature = "source")]
macro_rules! try_n_choose_k_type {
    ($context:ident, $n:ident, $k:ident, $type:ty) => {{
        let scalar = <NChooseK<$type> as MechFunctionFactory>::SIGNATURE;
        if let RuntimeFunctionInputs::Binary(expected_n, expected_k) = scalar.inputs
            && expected_n.matches(
                $n.representation()
                    .unwrap_or(FunctionValueRepresentation::Empty),
            )
            && expected_k.matches(
                $k.representation()
                    .unwrap_or(FunctionValueRepresentation::Empty),
            )
        {
            return $context.bind_resolved_runtime(
                mech_core::RuntimeBindingSelector::Operation(
                    $context.resolved_call()?.operation.id,
                ),
                mech_core::ExecutionTarget::DirectRuntime,
                vec![Vec::<u64>::new().into_boxed_slice()].into_boxed_slice(),
                &[$n, $k],
            );
        }
        #[cfg(all(feature = "matrix", feature = "matrixd"))]
        {
            let matrix = <NChooseKMatrix<$type> as MechFunctionFactory>::SIGNATURE;
            if let RuntimeFunctionInputs::Binary(expected_n, expected_k) = matrix.inputs
                && expected_n.matches(
                    $n.representation()
                        .unwrap_or(FunctionValueRepresentation::Empty),
                )
                && expected_k.matches(
                    $k.representation()
                        .unwrap_or(FunctionValueRepresentation::Empty),
                )
            {
                let extents = n_choose_k_matrix_extents::<$type>($n, $k)?;
                return $context.bind_resolved_runtime(
                    mech_core::RuntimeBindingSelector::Operation(
                        $context.resolved_call()?.operation.id,
                    ),
                    mech_core::ExecutionTarget::DirectRuntime,
                    vec![extents].into_boxed_slice(),
                    &[$n, $k],
                );
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
        #[cfg(feature = "u8")]
        try_n_choose_k_type!(context, n, k, u8);
        #[cfg(feature = "u16")]
        try_n_choose_k_type!(context, n, k, u16);
        #[cfg(feature = "u32")]
        try_n_choose_k_type!(context, n, k, u32);
        #[cfg(feature = "u64")]
        try_n_choose_k_type!(context, n, k, u64);
        #[cfg(feature = "u128")]
        try_n_choose_k_type!(context, n, k, u128);
        #[cfg(feature = "i8")]
        try_n_choose_k_type!(context, n, k, i8);
        #[cfg(feature = "i16")]
        try_n_choose_k_type!(context, n, k, i16);
        #[cfg(feature = "i32")]
        try_n_choose_k_type!(context, n, k, i32);
        #[cfg(feature = "i64")]
        try_n_choose_k_type!(context, n, k, i64);
        #[cfg(feature = "i128")]
        try_n_choose_k_type!(context, n, k, i128);
        #[cfg(feature = "f32")]
        try_n_choose_k_type!(context, n, k, f32);
        #[cfg(feature = "f64")]
        try_n_choose_k_type!(context, n, k, f64);
        #[cfg(feature = "rational")]
        try_n_choose_k_type!(context, n, k, R64);
        #[cfg(feature = "complex")]
        try_n_choose_k_type!(context, n, k, C64);
        Err(MechError::new(
            FunctionArgumentTypeMismatch {
                role: FunctionArgumentRole::Input(0),
                expected: "matching scalar or matrix n-choose-k inputs".into(),
                found: format!("{:?} and {:?}", n.representation(), k.representation(),),
            },
            None,
        )
        .with_compiler_loc())
    }
}

#[cfg(test)]
fn test_managed_factory<F: MechFunctionFactory>(
    invocation: FunctionInvocation,
    operation: &'static str,
) -> SpecializedFunction {
    let implementation = F::new_invocation(invocation.clone()).unwrap();
    let contract = F::declared_operation_contract()
        .or_else(|| implementation.semantic_operation_contract())
        .expect("managed combinatorics fixture requires an operation contract");
    SpecializedFunction::syntax_directed(
        (implementation, invocation),
        ResolvedOperationDescriptor::from_name(operation, contract.clone()).unwrap(),
        RuntimeFunctionId::from_name(operation),
        ExecutionTarget::DirectRuntime,
        F::implementation_memory_class(),
    )
    .unwrap()
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
        let function = test_managed_factory::<NChooseK<f64>>(
            FunctionInvocation::binary(
                output.clone(),
                ValueCell::from_exact(5.0_f64).unwrap(),
                selection.clone(),
            ),
            "test/n-choose-k-scalar",
        );
        function.instance().solve_result().unwrap();
        assert_eq!(f64_value(&output), 10.0);
        assert!(output.same_cell(&alias));
        assert_eq!(
            function.instance().reactive_output_cell_ids(),
            vec![output.reactive_cell_id()]
        );

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_instance(function.instance())?;
            selection.replace(&ValueCell::from_exact(1.5_f64)?.snapshot()?)?;
            assert!(function.instance().solve_result().is_err());
            assert_eq!(f64_value(&output), 10.0);
            output.replace(&ValueCell::from_exact(99.0_f64)?.snapshot()?)?;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(f64_value(&output), 10.0);

        assert!(
            NChooseK::<f64>::new_invocation(FunctionInvocation::binary(
                ValueCell::from_exact(0.0_f64).unwrap(),
                ValueCell::from_exact(5.0_f64).unwrap(),
                ValueCell::from_exact(2_usize).unwrap(),
            ))
            .is_err()
        );
        assert!(
            NChooseK::<f64>::new_invocation(FunctionInvocation::unary(
                ValueCell::from_exact(0.0_f64).unwrap(),
                ValueCell::from_exact(5.0_f64).unwrap(),
            ))
            .is_err()
        );
    }
}

#[cfg(all(test, feature = "f64", feature = "matrix2", feature = "matrixd"))]
mod canonical_matrix_tests {
    use super::*;
    use nalgebra::Matrix2;

    #[test]
    fn matrix_n_choose_k_preserves_input_and_output_identity_across_extents() {
        let input = ValueCell::from_exact(Matrix2::new(1.0_f64, 2.0, 3.0, 4.0)).unwrap();
        let input_alias = input.clone();
        let selection = ValueCell::from_exact(2.0_f64).unwrap();
        let output = ValueCell::from_exact(DMatrix::<f64>::zeros(2, 1)).unwrap();
        let out_alias = output.clone();
        let function = test_managed_factory::<NChooseKMatrix<f64>>(
            FunctionInvocation::binary(output.clone(), input.clone(), selection.clone()),
            "test/n-choose-k-matrix",
        );
        function.instance().solve_result().unwrap();
        assert!(input.same_cell(&input_alias));
        assert!(output.same_cell(&out_alias));
        assert_eq!(output.shape().parameter_values(), &[2, 6]);

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_instance(function.instance())?;
            selection.replace(&ValueCell::from_exact(3.0_f64)?.snapshot()?)?;
            function.instance().solve_result()?;
            assert_eq!(output.shape().parameter_values(), &[3, 4]);
            selection.replace(&ValueCell::from_exact(0.0_f64)?.snapshot()?)?;
            assert!(function.instance().solve_result().is_err());
            assert_eq!(output.shape().parameter_values(), &[3, 4]);
            output
                .replace(&ValueCell::from_exact(DMatrix::from_element(1, 1, 99.0))?.snapshot()?)?;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert!(output.same_cell(&out_alias));
        assert_eq!(output.shape().parameter_values(), &[2, 6]);

        assert!(
            NChooseKMatrix::<f64>::new_invocation(FunctionInvocation::binary(
                output,
                ValueCell::from_exact(4.0_f64).unwrap(),
                ValueCell::from_exact(2.0_f64).unwrap(),
            ))
            .is_err()
        );
    }
}
