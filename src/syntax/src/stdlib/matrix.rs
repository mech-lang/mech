#[macro_use]
use crate::stdlib::*;

// ----------------------------------------------------------------------------
// Matrix Library
// ----------------------------------------------------------------------------

macro_rules! matmul_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe { (*$lhs).mul_to(&*$rhs,&mut *$out); }
    };}
  
  macro_rules! mul_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe { *$out = *$lhs * *$rhs; }
    };}
  
  // MatMul ---------------------------------------------------------------------
  
  impl_binop!(MatMulScalar, T,T,T,mul_op);
  impl_binop!(MatMulM2x3M3x2, Matrix2x3<T>, Matrix3x2<T>, Matrix2<T>,matmul_op);
  impl_binop!(MatMulM2M2, Matrix2<T>, Matrix2<T>, Matrix2<T>,matmul_op);
  impl_binop!(MatMulM3M3, Matrix3<T>, Matrix3<T>, Matrix3<T>,matmul_op);
  impl_binop!(MatMulR2V2, RowVector2<T>,Vector2<T>,Matrix1<T>,matmul_op);
  impl_binop!(MatMulR3V3, RowVector3<T>,Vector3<T>,Matrix1<T>,matmul_op);
  impl_binop!(MatMulR4V4, RowVector4<T>,Vector4<T>,Matrix1<T>,matmul_op);
  impl_binop!(MatMulV2R2, Vector2<T>, RowVector2<T>, Matrix2<T>,matmul_op);
  impl_binop!(MatMulV3R3, Vector3<T>, RowVector3<T>, Matrix3<T>,matmul_op);
  impl_binop!(MatMulV4R4, Vector4<T>, RowVector4<T>, Matrix4<T>,matmul_op);
  impl_binop!(MatMulRDVD, RowDVector<T>, DVector<T>, Matrix1<T>,matmul_op);
  impl_binop!(MatMulVDRD, DVector<T>,RowDVector<T>,DMatrix<T>,matmul_op);
  impl_binop!(MatMulMDMD, DMatrix<T>,DMatrix<T>,DMatrix<T>,matmul_op);
  
  macro_rules! generate_matmul_match_arms {
    ($arg:expr, $($lhs_type:ident, $rhs_type:ident => $($matrix_kind:ident, $target_type:ident),+);+ $(;)?) => {
      match $arg {
        $(
          $(
            (Value::$lhs_type(lhs), Value::$rhs_type(rhs)) => {
              Ok(Box::new(MatMulScalar { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref($target_type::zero()) }))
            },
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Vector4(rhs))) => {
              Ok(Box::new(MatMulR4V4 { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Matrix1::from_element($target_type::zero())) }))
            },
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Vector3(rhs))) => {
              Ok(Box::new(MatMulR3V3 { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Matrix1::from_element($target_type::zero())) }))
            },
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Vector2(rhs))) => {
              Ok(Box::new(MatMulR2V2 { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Matrix1::from_element($target_type::zero())) }))
            },
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Matrix2(rhs))) => {
              Ok(Box::new(MatMulM2M2{lhs, rhs, out: new_ref(Matrix2::from_element($target_type::zero()))}))
            },
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Matrix3(rhs))) => {
              Ok(Box::new(MatMulM3M3{lhs, rhs, out: new_ref(Matrix3::from_element($target_type::zero()))}))
            },
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(rhs))) => {
              Ok(Box::new(MatMulM2x3M3x2{lhs, rhs, out: new_ref(Matrix2::from_element($target_type::zero()))}))
            },          
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(lhs)), Value::$matrix_kind(Matrix::<$target_type>::DVector(rhs))) => {
              let length = {lhs.borrow().len()};
              Ok(Box::new(MatMulRDVD{lhs, rhs, out: new_ref(Matrix1::from_element($target_type::zero()))}))
            },
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(lhs)), Value::$matrix_kind(Matrix::<$target_type>::RowDVector(rhs))) => {
              let rows = {lhs.borrow().len()};
              let cols = {rhs.borrow().len()};
              Ok(Box::new(MatMulVDRD{lhs, rhs, out: new_ref(DMatrix::from_element(rows,cols,$target_type::zero()))}))
            },
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(lhs)), Value::$matrix_kind(Matrix::<$target_type>::DMatrix(rhs))) => {
              let (rows,_) = {lhs.borrow().shape()};
              let (_,cols) = {rhs.borrow().shape()};
              Ok(Box::new(MatMulMDMD{lhs, rhs, out: new_ref(DMatrix::from_element(rows,cols,$target_type::zero()))}))
            },
          )+
        )+
        x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
  
  fn generate_matmul_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
    generate_matmul_match_arms!(
      (lhs_value, rhs_value),
      I8,   I8   => MatrixI8,   i8;
      I16,  I16  => MatrixI16,  i16;
      I32,  I32  => MatrixI32,  i32;
      I64,  I64  => MatrixI64,  i64;
      I128, I128 => MatrixI128, i128;
      U8,   U8   => MatrixU8,   u8;
      U16,  U16  => MatrixU16,  u16;
      U32,  U32  => MatrixU32,  u32;
      U64,  U64  => MatrixU64,  u64;
      U128, U128 => MatrixU128, u128;
      F32,  F32  => MatrixF32,  F32;
      F64,  F64  => MatrixF64,  F64;
    )
  }
  
  pub struct MatrixMatMul {}
  
  impl NativeFunctionCompiler for MatrixMatMul {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
      if arguments.len() != 2 {
        return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
      }
      let lhs_value = arguments[0].clone();
      let rhs_value = arguments[1].clone();
      match generate_matmul_fxn(lhs_value.clone(), rhs_value.clone()) {
        Ok(fxn) => Ok(fxn),
        Err(_) => {
          match (lhs_value,rhs_value) {
            (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {generate_matmul_fxn(lhs.borrow().clone(), rhs.borrow().clone())}
            (lhs_value,Value::MutableReference(rhs)) => { generate_matmul_fxn(lhs_value.clone(), rhs.borrow().clone())}
            (Value::MutableReference(lhs),rhs_value) => { generate_matmul_fxn(lhs.borrow().clone(), rhs_value.clone()) }
            x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
          }
        }
      }
    }
  }
  
  // Transpose ------------------------------------------------------------------
  
  macro_rules! impl_transpose_fxn {
    ($struct_name:ident, $arg_type:ty, $out_type:ty) => {
      #[derive(Debug)]
      struct $struct_name<T> {
        input: Ref<$arg_type>,
        out: Ref<$out_type>,
      }
      impl<T> MechFunction for $struct_name<T>
      where
        T: Copy + Debug + Clone + Sync + Send + Neg<Output = T> + PartialEq + 'static,
        Ref<$out_type>: ToValue
      {
        fn solve(&self) {
          let input_ptr = self.input.as_ptr();
          let output_ptr = self.out.as_ptr();
          unsafe { *output_ptr = (*input_ptr).transpose(); }
        }
        fn out(&self) -> Value { self.out.to_value() }
        fn to_string(&self) -> String { format!("{:?}", self) }
      }};}
  
  impl_transpose_fxn!(TransposeM2, Matrix2<T>, Matrix2<T>);
  impl_transpose_fxn!(TransposeM3, Matrix3<T>, Matrix3<T>);
  impl_transpose_fxn!(TransposeM2x3, Matrix2x3<T>, Matrix3x2<T>);
  impl_transpose_fxn!(TransposeM3x2, Matrix3x2<T>, Matrix2x3<T>);
  impl_transpose_fxn!(TransposeR2, RowVector2<T>, Vector2<T>);
  impl_transpose_fxn!(TransposeR3, RowVector3<T>, Vector3<T>);
  impl_transpose_fxn!(TransposeR4, RowVector4<T>, Vector4<T>);
  
  macro_rules! impl_transpose_fxn_dynamic {
    ($struct_name:ident, $arg_type:ty, $out_type:ty) => {
      #[derive(Debug)]
      struct $struct_name<T> {
        input: Ref<$arg_type>,
        out: Ref<$out_type>,
      }
      impl<T> MechFunction for $struct_name<T>
      where
        T: Copy + Debug + Clone + Sync + Send + ClosedNeg + PartialEq + 'static,
        Ref<$out_type>: ToValue
      {
        fn solve(&self) {
          let input_ptr = self.input.borrow();
          let output_ptr = self.out.as_ptr();
          unsafe { *output_ptr = input_ptr.clone().transpose(); }
        }
        fn out(&self) -> Value { self.out.to_value() }
        fn to_string(&self) -> String { format!("{:?}", self) }
      }};}
  
  impl_transpose_fxn_dynamic!(TransposeRD, RowDVector<T>, DVector<T>);
  impl_transpose_fxn_dynamic!(TransposeVD, DVector<T>, RowDVector<T>);
  impl_transpose_fxn_dynamic!(TransposeMD, DMatrix<T>, DMatrix<T>);
  
  macro_rules! generate_transpose_match_arms {
    ($arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident),+);+ $(;)?) => {
      match $arg {
        $(
          $(
            Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)) => {
              Ok(Box::new(TransposeR4{input: input.clone(), out: new_ref(Vector4::from_element($target_type::zero())) }))
            },
            Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)) => {
              Ok(Box::new(TransposeR3{input: input.clone(), out: new_ref(Vector3::from_element($target_type::zero())) }))
            },
            Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)) => {
              Ok(Box::new(TransposeR2{input: input.clone(), out: new_ref(Vector2::from_element($target_type::zero())) }))
            },
            Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)) => {
              Ok(Box::new(TransposeM2{input, out: new_ref(Matrix2::from_element($target_type::zero()))}))
            },
            Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)) => {
              Ok(Box::new(TransposeM3{input, out: new_ref(Matrix3::from_element($target_type::zero()))}))
            },
            Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)) => {
              Ok(Box::new(TransposeM2x3{input, out: new_ref(Matrix3x2::from_element($target_type::zero()))}))
            },          
            Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)) => {
              Ok(Box::new(TransposeM3x2{input, out: new_ref(Matrix2x3::from_element($target_type::zero()))}))
            },          
            Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)) => {
              let length = {input.borrow().len()};
              Ok(Box::new(TransposeRD{input, out: new_ref(DVector::from_element(length,$target_type::zero()))}))
            },
            Value::$matrix_kind(Matrix::<$target_type>::DVector(input)) => {
              let length = {input.borrow().len()};
              Ok(Box::new(TransposeVD{input, out: new_ref(RowDVector::from_element(length,$target_type::zero()))}))
            },
            Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)) => {
              let (rows,cols) = {input.borrow().shape()};
              Ok(Box::new(TransposeMD{input, out: new_ref(DMatrix::from_element(rows,cols,$target_type::zero()))}))
            },
          )+
        )+
        x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
  
  fn generate_transpose_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
    generate_transpose_match_arms!(
      (lhs_value),
      I8 => MatrixI8, i8;
      I16 => MatrixI16, i16;
      I32 => MatrixI32, i32;
      I64 => MatrixI64, i64;
      I128 => MatrixI128, i128;
      F32 => MatrixF32, F32;
      F64 => MatrixF64, F64;
    )
  }
  
  pub struct MatrixTranspose {}
  
  impl NativeFunctionCompiler for MatrixTranspose {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
      if arguments.len() != 1 {
        return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
      }
      let input = arguments[0].clone();
      match generate_transpose_fxn(input.clone()) {
        Ok(fxn) => Ok(fxn),
        Err(_) => {
          match (input) {
            (Value::MutableReference(input)) => {generate_transpose_fxn(input.borrow().clone())}
            x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
          }
        }
      }
    }
  }