#[macro_use]
use crate::stdlib::*;

// ----------------------------------------------------------------------------
// Matrix Library
// ----------------------------------------------------------------------------

// MatMul ---------------------------------------------------------------------

macro_rules! mul_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { *$out = *$lhs * *$rhs; }
  };}

macro_rules! matmul_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { (*$lhs).mul_to(&*$rhs,&mut *$out); }
  };}

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

impl_mech_binop_fxn!(MatrixMatMul,generate_matmul_fxn);

// Transpose ------------------------------------------------------------------

macro_rules! transpose_op {
  ($arg:expr, $out:expr) => {
    unsafe { *$out = (*$arg).transpose(); }
  };}

impl_bool_urop!(TransposeM2, Matrix2<T>, Matrix2<T>, transpose_op);
impl_bool_urop!(TransposeM3, Matrix3<T>, Matrix3<T>, transpose_op);
impl_bool_urop!(TransposeM2x3, Matrix2x3<T>, Matrix3x2<T>, transpose_op);
impl_bool_urop!(TransposeM3x2, Matrix3x2<T>, Matrix2x3<T>, transpose_op);
impl_bool_urop!(TransposeR2, RowVector2<T>, Vector2<T>, transpose_op);
impl_bool_urop!(TransposeR3, RowVector3<T>, Vector3<T>, transpose_op);
impl_bool_urop!(TransposeR4, RowVector4<T>, Vector4<T>, transpose_op); 
impl_bool_urop!(TransposeRD, RowDVector<T>, DVector<T>, transpose_op);
impl_bool_urop!(TransposeVD, DVector<T>, RowDVector<T>, transpose_op);
impl_bool_urop!(TransposeMD, DMatrix<T>, DMatrix<T>, transpose_op);

macro_rules! generate_transpose_match_arms {
  ($arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    match $arg {
      $(
        $(
          Value::$matrix_kind(Matrix::<$target_type>::RowVector4(arg)) => {
            Ok(Box::new(TransposeR4{arg: arg.clone(), out: new_ref(Vector4::from_element($default)) }))
          },
          Value::$matrix_kind(Matrix::<$target_type>::RowVector3(arg)) => {
            Ok(Box::new(TransposeR3{arg: arg.clone(), out: new_ref(Vector3::from_element($default)) }))
          },
          Value::$matrix_kind(Matrix::<$target_type>::RowVector2(arg)) => {
            Ok(Box::new(TransposeR2{arg: arg.clone(), out: new_ref(Vector2::from_element($default)) }))
          },
          Value::$matrix_kind(Matrix::<$target_type>::Matrix2(arg)) => {
            Ok(Box::new(TransposeM2{arg, out: new_ref(Matrix2::from_element($default))}))
          },
          Value::$matrix_kind(Matrix::<$target_type>::Matrix3(arg)) => {
            Ok(Box::new(TransposeM3{arg, out: new_ref(Matrix3::from_element($default))}))
          },
          Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(arg)) => {
            Ok(Box::new(TransposeM2x3{arg, out: new_ref(Matrix3x2::from_element($default))}))
          },          
          Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(arg)) => {
            Ok(Box::new(TransposeM3x2{arg, out: new_ref(Matrix2x3::from_element($default))}))
          },          
          Value::$matrix_kind(Matrix::<$target_type>::RowDVector(arg)) => {
            let length = {arg.borrow().len()};
            Ok(Box::new(TransposeRD{arg, out: new_ref(DVector::from_element(length,$default))}))
          },
          Value::$matrix_kind(Matrix::<$target_type>::DVector(arg)) => {
            let length = {arg.borrow().len()};
            Ok(Box::new(TransposeVD{arg, out: new_ref(RowDVector::from_element(length,$default))}))
          },
          Value::$matrix_kind(Matrix::<$target_type>::DMatrix(arg)) => {
            let (rows,cols) = {arg.borrow().shape()};
            Ok(Box::new(TransposeMD{arg, out: new_ref(DMatrix::from_element(rows,cols,$default))}))
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
    Bool => MatrixBool, bool, false;
    I8   => MatrixI8,   i8,   i8::zero();
    I16  => MatrixI16,  i16,  i16::zero();
    I32  => MatrixI32,  i32,  i32::zero();
    I64  => MatrixI64,  i64,  i64::zero();
    I128 => MatrixI128, i128, i128::zero();
    U8   => MatrixU8,   u8,   u8::zero();
    U16  => MatrixU16,  u16,  u16::zero();
    U32  => MatrixU32,  u32,  u32::zero();
    U64  => MatrixU64,  u64,  u64::zero();
    U128 => MatrixU128, u128, u128::zero();
    F32  => MatrixF32,  F32,  F32::zero();
    F64  => MatrixF64,  F64,  F64::zero();
  )
}
  
impl_mech_urnop_fxn!(MatrixTranspose,generate_transpose_fxn);

// Access ---------------------------------------------------------------------

macro_rules! access_1d {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { *$out = (*$source).index(*$ix-1).clone() }
  };}

macro_rules! access_2d {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { *$out = (*$source).index(((*$ix)[0]-1,(*$ix)[1]-1)).clone() }
  };}

macro_rules! impl_access_fxn {
  ($struct_name:ident, $arg_type:ty, $ix_type:ty, $out_type:ty, $op:ident) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<$arg_type>,
      ixes: Ref<$ix_type>,
      out: Ref<$out_type>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$out_type>: ToValue
    {
      fn solve(&self) {
        let source_ptr = self.source.as_ptr();
        let ixes_ptr = self.ixes.as_ptr();
        let out_ptr = self.out.as_ptr();
        $op!(source_ptr,ixes_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }};}

impl_access_fxn!(Access1DR2, RowVector2<T>, usize, T, access_1d);
impl_access_fxn!(Access1DR3, RowVector3<T>, usize, T, access_1d);
impl_access_fxn!(Access1DR4, RowVector4<T>, usize, T, access_1d);
impl_access_fxn!(Access1DM2, Matrix2<T>, usize, T, access_1d);
impl_access_fxn!(Access1DM3, Matrix3<T>, usize, T, access_1d);
impl_access_fxn!(Access1DM2x3, Matrix2x3<T>, usize, T, access_1d);
impl_access_fxn!(Access1DM3x2, Matrix3x2<T>, usize, T, access_1d);
impl_access_fxn!(Access1DMD, DMatrix<T>, usize, T, access_1d);
impl_access_fxn!(Access1DRD, RowDVector<T>, usize, T, access_1d);
impl_access_fxn!(Access1DVD, DVector<T>, usize, T, access_1d);

impl_access_fxn!(Access2DR2, RowVector2<T>, RowVector2<usize>, T, access_2d);
impl_access_fxn!(Access2DR3, RowVector3<T>, RowVector2<usize>, T, access_2d);
impl_access_fxn!(Access2DR4, RowVector4<T>, RowVector2<usize>, T, access_2d);
impl_access_fxn!(Access2DM2, Matrix2<T>, RowVector2<usize>, T, access_2d);
impl_access_fxn!(Access2DM3, Matrix3<T>, RowVector2<usize>, T, access_2d);
impl_access_fxn!(Access2DM2x3, Matrix2x3<T>, RowVector2<usize>, T, access_2d);
impl_access_fxn!(Access2DM3x2, Matrix3x2<T>, RowVector2<usize>, T, access_2d);
impl_access_fxn!(Access2DMD, DMatrix<T>, RowVector2<usize>, T, access_2d);
impl_access_fxn!(Access2DRD, RowDVector<T>, RowVector2<usize>, T, access_2d);
impl_access_fxn!(Access2DVD, DVector<T>, RowVector2<usize>, T, access_2d);


macro_rules! generate_access_match_arms {
  ($arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    match $arg {
      $(
        $(
          (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), Value::Index(ix)) => {
            Ok(Box::new(Access1DR4{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), Value::Index(ix)) => {
            Ok(Box::new(Access1DR3{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), Value::Index(ix)) => {
            Ok(Box::new(Access1DR2{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), Value::Index(ix)) => {
            Ok(Box::new(Access1DM2{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), Value::Index(ix)) => {
            Ok(Box::new(Access1DM3{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), Value::Index(ix)) => {
            Ok(Box::new(Access1DM2x3{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), Value::Index(ix)) => {
            Ok(Box::new(Access1DM3x2{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), Value::Index(ix)) => {
            Ok(Box::new(Access1DMD{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), Value::Index(ix)) => {
            Ok(Box::new(Access1DRD{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)), Value::Index(ix)) => {
            Ok(Box::new(Access1DVD{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), Value::MatrixIndex(Matrix::<usize>::RowVector2(ix))) => {
            Ok(Box::new(Access2DR4{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), Value::MatrixIndex(Matrix::<usize>::RowVector2(ix))) => {
            Ok(Box::new(Access2DR3{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), Value::MatrixIndex(Matrix::<usize>::RowVector2(ix))) => {
            Ok(Box::new(Access2DR2{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), Value::MatrixIndex(Matrix::<usize>::RowVector2(ix))) => {
            Ok(Box::new(Access2DM2{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), Value::MatrixIndex(Matrix::<usize>::RowVector2(ix))) => {
            Ok(Box::new(Access2DM3{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), Value::MatrixIndex(Matrix::<usize>::RowVector2(ix))) => {
            Ok(Box::new(Access2DM2x3{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), Value::MatrixIndex(Matrix::<usize>::RowVector2(ix))) => {
            Ok(Box::new(Access2DM3x2{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), Value::MatrixIndex(Matrix::<usize>::RowVector2(ix))) => {
            Ok(Box::new(Access2DMD{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), Value::MatrixIndex(Matrix::<usize>::RowVector2(ix))) => {
            Ok(Box::new(Access2DRD{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)), Value::MatrixIndex(Matrix::<usize>::RowVector2(ix))) => {
            Ok(Box::new(Access2DVD{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
        )+
      )+
      x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
    }
  }
}

fn generate_access_fxn(lhs_value: Value, ixes: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_access_match_arms!(
    (lhs_value, ixes),
    Bool => MatrixBool, bool, false;
    I8 => MatrixI8, i8, i8::zero();
    I16  => MatrixI16,  i16, i16::zero();
    I32  => MatrixI32,  i32, i32::zero();
    I64 => MatrixI64,  i64, i64::zero();
    I128 => MatrixI128, i128, i128::zero();
    U8   => MatrixU8,   u8, u8::zero();
    U16  => MatrixU16,  u16, u16::zero();
    U32  => MatrixU32,  u32, u32::zero();
    U64  => MatrixU64,  u64, u64::zero();
    U128 => MatrixU128, u128, u128::zero();
    F32  => MatrixF32,  F32, F32::zero();
    F64  => MatrixF64,  F64, F64::zero();
  )
}

impl_mech_binop_fxn!(MatrixAccess,generate_access_fxn);