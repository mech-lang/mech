#[cfg(feature = "matrix")]
use mech_core::structures::matrix::Matrix;
use mech_core::*;

#[cfg(feature = "matrixd")]
use nalgebra::DMatrix;

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
        || request
            .inputs
            .iter()
            .any(|input| input.kind != ResidentValueKind::F64 || input.shape != ResidentShape::SCALAR)
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
        )+
    };
}

impl_runtime_integer_n_choose_k!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

impl RuntimeNChooseK for f32 {
    fn runtime_n_choose_k(self, k: Self) -> Option<Self> {
        Some(crate::kernels::n_choose_k::scalar(self, k))
    }
}

impl RuntimeNChooseK for f64 {
    fn runtime_n_choose_k(self, k: Self) -> Option<Self> {
        Some(crate::kernels::n_choose_k::scalar(self, k))
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

#[cfg(feature = "complex")]
impl RuntimeNChooseK for C64 {
    fn runtime_n_choose_k(self, k: Self) -> Option<Self> {
        Some(crate::kernels::n_choose_k::scalar(self, k))
    }
}

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
            (value.is_finite()
                && value >= 0.0
                && value.fract() == 0.0
                && value <= u128::MAX as f32)
                .then_some(value as u128)
        }
        #[cfg(feature = "f64")]
        LegacyValue::F64(value) => {
            let value = *value.borrow();
            (value.is_finite()
                && value >= 0.0
                && value.fract() == 0.0
                && value <= u128::MAX as f64)
                .then_some(value as u128)
        }
        #[cfg(feature = "r64")]
        LegacyValue::R64(value) => {
            let value = value.borrow();
            (*value.denom() == 1).then(|| u128::try_from(*value.numer()).ok()).flatten()
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

pub(crate) fn validate_n_choose_k_scalar_contract(args: &FunctionArgs) -> MResult<()> {
    let contract = "n_choose_k_scalar";
    let n = args.input_value(0).ok_or_else(|| {
        function_shape_contract_violation(contract, "missing input 0")
    })?;
    let k = args.input_value(1).ok_or_else(|| {
        function_shape_contract_violation(contract, "missing input 1")
    })?;
    validate_n_choose_k_scalar_values(n, k)
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
fn matrix_selection_size(value: &LegacyValue) -> Option<usize> {
    match value {
        LegacyValue::Index(value) => Some(*value.borrow()),
        _ => usize::try_from(scalar_selection_count(value)?).ok(),
    }
}

#[cfg(all(feature = "matrix", feature = "matrixd"))]
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
        + PartialEq
        + PartialOrd,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + ConstElem,
    Ref<T>: ToValue,
    T: FunctionRuntimeType,
{
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::binary(T::REPRESENTATION, T::REPRESENTATION, T::REPRESENTATION);

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            FunctionArgs::Binary(out, arg1, arg2) => {
                let n: Ref<T> = arg1.try_function_ref(FunctionArgumentRole::Input(0))?;
                let k: Ref<T> = arg2.try_function_ref(FunctionArgumentRole::Input(1))?;
                let out: Ref<T> = out.try_function_ref(FunctionArgumentRole::Output)?;
                Ok(Box::new(Self { n, k, out }))
            }
            _ => Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 2,
                    found: args.len(),
                },
                None,
            )
            .with_compiler_loc()),
        }
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
        + PartialEq
        + PartialOrd,
    Ref<T>: ToValue,
{
    fn solve_result(&self) -> MResult<()> {
        // Validate every reactive solve, not only bytecode installation: host
        // or live-resource producers may replace a previously safe operand.
        validate_n_choose_k_scalar_values(&self.n.to_value(), &self.k.to_value())?;
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
    fn out(&self) -> LegacyValue {
        self.out.to_value()
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_N_CHOOSE_K_SCALAR_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }
}

#[cfg(all(test, feature = "f64"))]
mod scalar_contract_tests {
    use super::*;

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
}

#[cfg(all(test, feature = "u8"))]
mod integer_scalar_tests {
    use super::*;

    #[test]
    fn integer_n_choose_k_avoids_intermediate_overflow_and_retains_output_on_error() {
        let n = Ref::new(20_u8);
        let k = Ref::new(2_u8);
        let out = Ref::new(17_u8);
        let function = NChooseK {
            n: n.clone(),
            k,
            out: out.clone(),
        };

        function.solve_result().unwrap();
        assert_eq!(*out.borrow(), 190);

        *n.borrow_mut() = 30;
        let error = function.solve_result().unwrap_err();
        assert_eq!(error.kind_name(), "FunctionShapeContractViolation");
        assert_eq!(*out.borrow(), 190);
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
    out: Ref<DMatrix<T>>,
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
        + PartialEq
        + PartialOrd,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + ConstElem,
    Ref<T>: ToValue,
    Ref<DMatrix<T>>: ToValue,
    T: FunctionRuntimeType,
    Matrix<T>: FunctionRuntimeType,
    DMatrix<T>: FunctionRuntimeType,
{
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        <DMatrix<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix<T> as FunctionRuntimeType>::REPRESENTATION,
        T::REPRESENTATION,
    );

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            FunctionArgs::Binary(out, arg1, arg2) => {
                let n: Matrix<T> = arg1.try_function_matrix(FunctionArgumentRole::Input(0))?;
                let k: Ref<T> = arg2.try_function_ref(FunctionArgumentRole::Input(1))?;
                let out: Ref<DMatrix<T>> = out.try_function_ref(FunctionArgumentRole::Output)?;
                Ok(Box::new(Self { n, k, out }))
            }
            _ => Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 2,
                    found: args.len(),
                },
                None,
            )
            .with_compiler_loc()),
        }
    }
}
#[cfg(all(feature = "matrix", feature = "matrixd"))]
impl<T> MechFunctionImpl for NChooseKMatrix<T>
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
        + PartialEq
        + PartialOrd,
    Ref<T>: ToValue,
    Ref<DMatrix<T>>: ToValue,
{
    fn solve_result(&self) -> MResult<()> {
        let requested = matrix_selection_size(&self.k.to_value())
            .ok_or_else(invalid_matrix_selection_value)?;
        let next = n_choose_k_matrix_result(&self.n, requested)?;
        *self.out.borrow_mut() = next;
        Ok(())
    }
    fn out(&self) -> LegacyValue {
        self.out.to_value()
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_N_CHOOSE_K_MATRIX_CONTRACT)
    }
    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
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

    #[test]
    fn matrix_combinations_expose_value_backed_transaction_state() {
        let n = f64::to_matrix(vec![1.0, 2.0], 2, 1);
        let out = Ref::new(DMatrix::from_element(1, 1, 0.0));
        let function = NChooseKMatrix {
            n,
            k: Ref::new(1.0),
            out: out.clone(),
        };
        let state = function.transaction_state_values().unwrap();

        assert_eq!(state, vec![out.to_value()]);
    }

    #[test]
    fn matrix_combinations_reuse_one_output_root_across_shape_changes() {
        let n = f64::to_matrix(vec![1.0, 2.0, 3.0], 3, 1);
        let k = Ref::new(1.0);
        let out = Ref::new(DMatrix::from_element(1, 1, 0.0));
        let function = NChooseKMatrix {
            n,
            k: k.clone(),
            out: out.clone(),
        };
        let output_cell = out.id();

        function.solve_result().unwrap();
        assert_eq!(out.borrow().shape(), (1, 3));
        assert_eq!(out.borrow().as_slice(), &[1.0, 2.0, 3.0]);

        *k.borrow_mut() = 2.0;
        function.solve_result().unwrap();
        assert_eq!(out.id(), output_cell);
        assert_eq!(out.borrow().shape(), (2, 3));
        assert_eq!(out.borrow().as_slice(), &[1.0, 2.0, 1.0, 3.0, 2.0, 3.0]);

        *k.borrow_mut() = 3.0;
        function.solve_result().unwrap();
        assert_eq!(out.id(), output_cell);
        assert_eq!(out.borrow().shape(), (3, 1));
        assert_eq!(out.borrow().as_slice(), &[1.0, 2.0, 3.0]);
    }

    #[test]
    fn invalid_matrix_selection_is_structured_and_retains_previous_output() {
        let n = f64::to_matrix(vec![1.0, 2.0, 3.0], 3, 1);
        let k = Ref::new(1.0);
        let out = Ref::new(DMatrix::from_element(1, 1, 0.0));
        let function = NChooseKMatrix {
            n,
            k: k.clone(),
            out: out.clone(),
        };
        function.solve_result().unwrap();
        let expected = out.borrow().clone();

        for invalid in [0.0, 4.0] {
            *k.borrow_mut() = invalid;
            let error = function.solve_result().unwrap_err();
            assert_eq!(error.kind_name(), "NChooseKMatrixSelectionInvalid");
            assert_eq!(*out.borrow(), expected);
        }
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
        assert_eq!(
            planning_error.kind_name(),
            "FunctionShapeContractViolation"
        );

        let function = NChooseKMatrix {
            n,
            k: k.clone(),
            out: out.clone(),
        };
        *k.borrow_mut() = 1;
        function.solve_result().unwrap();
        let previous = out.borrow().clone();

        *k.borrow_mut() = -1;
        let runtime_error = function.solve_result().unwrap_err();
        assert_eq!(runtime_error.kind_name(), "NChooseKMatrixSelectionInvalid");
        assert_eq!(*out.borrow(), previous);
    }
}

#[cfg(all(feature = "source", feature = "matrix", feature = "matrixd"))]
fn n_choose_k_matrix_specialization<T>(n: Matrix<T>, k: Ref<T>) -> Box<dyn MechFunction>
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
        + PartialEq
        + PartialOrd,
    #[cfg(feature = "semantic-compiler")]
    T: ConstElem + CompileConst,
    Ref<T>: ToValue,
    Ref<DMatrix<T>>: ToValue,
{
    Box::new(NChooseKMatrix {
        n,
        k,
        out: Ref::new(DMatrix::from_element(1, 1, T::zero())),
    })
}

#[cfg(feature = "source")]
fn impl_combinatorics_n_choose_k_fxn(n: LegacyValue, k: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    match (n, k) {
        #[cfg(feature = "u8")]
        (LegacyValue::U8(n), LegacyValue::U8(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(u8::default()),
        })),
        #[cfg(feature = "u16")]
        (LegacyValue::U16(n), LegacyValue::U16(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(u16::default()),
        })),
        #[cfg(feature = "u32")]
        (LegacyValue::U32(n), LegacyValue::U32(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(u32::default()),
        })),
        #[cfg(feature = "u64")]
        (LegacyValue::U64(n), LegacyValue::U64(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(u64::default()),
        })),
        #[cfg(feature = "u128")]
        (LegacyValue::U128(n), LegacyValue::U128(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(u128::default()),
        })),
        #[cfg(feature = "i8")]
        (LegacyValue::I8(n), LegacyValue::I8(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(i8::default()),
        })),
        #[cfg(feature = "i16")]
        (LegacyValue::I16(n), LegacyValue::I16(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(i16::default()),
        })),
        #[cfg(feature = "i32")]
        (LegacyValue::I32(n), LegacyValue::I32(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(i32::default()),
        })),
        #[cfg(feature = "i64")]
        (LegacyValue::I64(n), LegacyValue::I64(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(i64::default()),
        })),
        #[cfg(feature = "i128")]
        (LegacyValue::I128(n), LegacyValue::I128(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(i128::default()),
        })),
        #[cfg(feature = "f32")]
        (LegacyValue::F32(n), LegacyValue::F32(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(f32::default()),
        })),
        #[cfg(feature = "f64")]
        (LegacyValue::F64(n), LegacyValue::F64(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(f64::default()),
        })),
        #[cfg(feature = "rational")]
        (LegacyValue::R64(n), LegacyValue::R64(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(R64::default()),
        })),
        #[cfg(feature = "complex")]
        (LegacyValue::C64(n), LegacyValue::C64(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(C64::default()),
        })),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "u8"))]
        (LegacyValue::MatrixU8(n), LegacyValue::U8(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "u16"))]
        (LegacyValue::MatrixU16(n), LegacyValue::U16(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "u32"))]
        (LegacyValue::MatrixU32(n), LegacyValue::U32(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "u64"))]
        (LegacyValue::MatrixU64(n), LegacyValue::U64(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "u128"))]
        (LegacyValue::MatrixU128(n), LegacyValue::U128(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "i8"))]
        (LegacyValue::MatrixI8(n), LegacyValue::I8(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "i16"))]
        (LegacyValue::MatrixI16(n), LegacyValue::I16(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "i32"))]
        (LegacyValue::MatrixI32(n), LegacyValue::I32(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "i64"))]
        (LegacyValue::MatrixI64(n), LegacyValue::I64(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "i128"))]
        (LegacyValue::MatrixI128(n), LegacyValue::I128(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "f32"))]
        (LegacyValue::MatrixF32(n), LegacyValue::F32(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "f64"))]
        (LegacyValue::MatrixF64(n), LegacyValue::F64(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "rational"))]
        (LegacyValue::MatrixR64(n), LegacyValue::R64(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "complex"))]
        (LegacyValue::MatrixC64(n), LegacyValue::C64(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        (n, k) => Err(MechError::new(
            UnhandledFunctionArgumentKind2 {
                arg: (n.kind(), k.kind()),
                fxn_name: "combinatorics/n-choose-k".to_string(),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

#[cfg(feature = "source")]
pub struct CombinatoricsNChooseK {}
#[cfg(feature = "source")]
impl FunctionSpecializer for CombinatoricsNChooseK {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() != 2 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 2,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let n = arguments[0].clone();
        let k = arguments[1].clone();

        match impl_combinatorics_n_choose_k_fxn(n.clone(), k.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (n, k) {
                (LegacyValue::MutableReference(n), LegacyValue::MutableReference(k)) => {
                    let n_brrw = n.borrow();
                    let k_brrw = k.borrow();
                    impl_combinatorics_n_choose_k_fxn(n_brrw.clone(), k_brrw.clone())
                }
                (n, LegacyValue::MutableReference(k)) => {
                    let k_brrw = k.borrow();
                    impl_combinatorics_n_choose_k_fxn(n.clone(), k_brrw.clone())
                }
                (LegacyValue::MutableReference(n), k) => {
                    let n_brrw = n.borrow();
                    impl_combinatorics_n_choose_k_fxn(n_brrw.clone(), k.clone())
                }
                (n, k) => Err(MechError::new(
                    UnhandledFunctionArgumentKind2 {
                        arg: (n.kind(), k.kind()),
                        fxn_name: "combinatorics/n-choose-k".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
