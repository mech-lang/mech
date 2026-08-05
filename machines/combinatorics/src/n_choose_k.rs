#[cfg(feature = "matrix")]
use mech_core::structures::matrix::Matrix;
use mech_core::value::ToUsize;
use mech_core::*;

#[cfg(feature = "matrixd")]
use nalgebra::DMatrix;

use itertools::Itertools;
use num_traits::{One, Zero};
use paste::paste;
use std::fmt::Debug;
use std::ops::{Add, AddAssign, Div, Mul, Sub};

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
fn matrix_selection_size(value: &Value) -> Option<usize> {
    match value {
        Value::Index(value) => Some(*value.borrow()),
        #[cfg(feature = "u8")]
        Value::U8(value) => Some(value.borrow().to_usize()),
        #[cfg(feature = "u16")]
        Value::U16(value) => Some(value.borrow().to_usize()),
        #[cfg(feature = "u32")]
        Value::U32(value) => Some(value.borrow().to_usize()),
        #[cfg(feature = "u64")]
        Value::U64(value) => Some(value.borrow().to_usize()),
        #[cfg(feature = "u128")]
        Value::U128(value) => Some(value.borrow().to_usize()),
        #[cfg(feature = "i8")]
        Value::I8(value) => Some(value.borrow().to_usize()),
        #[cfg(feature = "i16")]
        Value::I16(value) => Some(value.borrow().to_usize()),
        #[cfg(feature = "i32")]
        Value::I32(value) => Some(value.borrow().to_usize()),
        #[cfg(feature = "i64")]
        Value::I64(value) => Some(value.borrow().to_usize()),
        #[cfg(feature = "i128")]
        Value::I128(value) => Some(value.borrow().to_usize()),
        #[cfg(feature = "f32")]
        Value::F32(value) => Some(value.borrow().to_usize()),
        #[cfg(feature = "f64")]
        Value::F64(value) => Some(value.borrow().to_usize()),
        #[cfg(feature = "rational")]
        Value::R64(value) => Some(value.borrow().to_usize()),
        #[cfg(feature = "complex")]
        Value::C64(value) => Some(value.borrow().to_usize()),
        _ => None,
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
    pub requested: usize,
}

#[cfg(all(feature = "matrix", feature = "matrixd"))]
impl MechErrorKind for NChooseKMatrixSelectionInvalid {
    fn name(&self) -> &str {
        "NChooseKMatrixSelectionInvalid"
    }

    fn message(&self) -> String {
        format!(
            "matrix n-choose-k requested {} elements from {} available; expected 1..={}",
            self.requested, self.available, self.available,
        )
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
            requested,
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
        + PartialEq
        + PartialOrd,
    #[cfg(feature = "compiler")]
    T: CompileConst + ConstElem,
    Ref<T>: ToValue,
{
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
        + PartialEq
        + PartialOrd,
    Ref<T>: ToValue,
{
    fn solve(&self) {
        let n_ptr = self.n.as_ptr();
        let k_ptr = self.k.as_ptr();
        let out_ptr = self.out.as_mut_ptr();
        unsafe {
            let n = *n_ptr;
            let k = *k_ptr;
            *out_ptr = crate::kernels::n_choose_k::scalar(n, k);
        }
    }
    fn out(&self) -> Value {
        self.out.to_value()
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for NChooseK<T>
where
    T: ConstElem + CompileConst + AsValueKind,
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
fn n_choose_k_matrix_result<T>(n: &Matrix<T>, k: T) -> MResult<DMatrix<T>>
where
    T: Copy + Clone + Debug + PartialEq + ToUsize + 'static,
{
    let elements = n.as_vec();
    let available = elements.len();
    let requested = k.to_usize();
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
    Ok(DMatrix::from_vec(
        requested,
        combination_count,
        flat_data,
    ))
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
        + ToUsize
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
    #[cfg(feature = "compiler")]
    T: CompileConst + ConstElem,
    Ref<T>: ToValue,
    Ref<DMatrix<T>>: ToValue,
{
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
        + ToUsize
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
    fn solve(&self) {
        let _ = self.solve_result();
    }
    fn solve_result(&self) -> MResult<()> {
        let next = n_choose_k_matrix_result(&self.n, *self.k.borrow())?;
        *self.out.borrow_mut() = next;
        Ok(())
    }
    fn out(&self) -> Value {
        self.out.to_value()
    }
    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "matrix", feature = "matrixd", feature = "compiler"))]
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

#[cfg(all(feature = "source", feature = "matrix", feature = "matrixd"))]
fn n_choose_k_matrix_specialization<T>(n: Matrix<T>, k: Ref<T>) -> Box<dyn MechFunction>
where
    T: Copy
        + Debug
        + Clone
        + Sync
        + Send
        + 'static
        + ToUsize
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
    #[cfg(feature = "compiler")]
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
fn impl_combinatorics_n_choose_k_fxn(n: Value, k: Value) -> MResult<Box<dyn MechFunction>> {
    match (n, k) {
        #[cfg(feature = "u8")]
        (Value::U8(n), Value::U8(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(u8::default()),
        })),
        #[cfg(feature = "u16")]
        (Value::U16(n), Value::U16(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(u16::default()),
        })),
        #[cfg(feature = "u32")]
        (Value::U32(n), Value::U32(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(u32::default()),
        })),
        #[cfg(feature = "u64")]
        (Value::U64(n), Value::U64(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(u64::default()),
        })),
        #[cfg(feature = "u128")]
        (Value::U128(n), Value::U128(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(u128::default()),
        })),
        #[cfg(feature = "i8")]
        (Value::I8(n), Value::I8(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(i8::default()),
        })),
        #[cfg(feature = "i16")]
        (Value::I16(n), Value::I16(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(i16::default()),
        })),
        #[cfg(feature = "i32")]
        (Value::I32(n), Value::I32(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(i32::default()),
        })),
        #[cfg(feature = "i64")]
        (Value::I64(n), Value::I64(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(i64::default()),
        })),
        #[cfg(feature = "i128")]
        (Value::I128(n), Value::I128(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(i128::default()),
        })),
        #[cfg(feature = "f32")]
        (Value::F32(n), Value::F32(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(f32::default()),
        })),
        #[cfg(feature = "f64")]
        (Value::F64(n), Value::F64(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(f64::default()),
        })),
        #[cfg(feature = "rational")]
        (Value::R64(n), Value::R64(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(R64::default()),
        })),
        #[cfg(feature = "complex")]
        (Value::C64(n), Value::C64(k)) => Ok(Box::new(NChooseK {
            n: n,
            k: k,
            out: Ref::new(C64::default()),
        })),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "u8"))]
        (Value::MatrixU8(n), Value::U8(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "u16"))]
        (Value::MatrixU16(n), Value::U16(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "u32"))]
        (Value::MatrixU32(n), Value::U32(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "u64"))]
        (Value::MatrixU64(n), Value::U64(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "u128"))]
        (Value::MatrixU128(n), Value::U128(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "i8"))]
        (Value::MatrixI8(n), Value::I8(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "i16"))]
        (Value::MatrixI16(n), Value::I16(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "i32"))]
        (Value::MatrixI32(n), Value::I32(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "i64"))]
        (Value::MatrixI64(n), Value::I64(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "i128"))]
        (Value::MatrixI128(n), Value::I128(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "f32"))]
        (Value::MatrixF32(n), Value::F32(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "f64"))]
        (Value::MatrixF64(n), Value::F64(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "rational"))]
        (Value::MatrixR64(n), Value::R64(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
        #[cfg(all(feature = "matrix", feature = "matrixd", feature = "complex"))]
        (Value::MatrixC64(n), Value::C64(k)) => Ok(n_choose_k_matrix_specialization(n, k)),
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
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
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
                (Value::MutableReference(n), Value::MutableReference(k)) => {
                    let n_brrw = n.borrow();
                    let k_brrw = k.borrow();
                    impl_combinatorics_n_choose_k_fxn(n_brrw.clone(), k_brrw.clone())
                }
                (n, Value::MutableReference(k)) => {
                    let k_brrw = k.borrow();
                    impl_combinatorics_n_choose_k_fxn(n.clone(), k_brrw.clone())
                }
                (Value::MutableReference(n), k) => {
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
