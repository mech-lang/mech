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
    unsafe { 
      let ix1 = (*$ix).0;
      let ix2 = (*$ix).1;
      *$out = (*$source).index((ix1-1,ix2-1)).clone() 
    }
  };}

macro_rules! access_1d_slice2 {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      (*$out)[0] = (*$source).index((*$ix)[0]-1).clone();
      (*$out)[1] = (*$source).index((*$ix)[1]-1).clone();
    }
  };}

macro_rules! access_1d_slice3 {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      (*$out)[0] = (*$source).index((*$ix)[0]-1).clone();
      (*$out)[1] = (*$source).index((*$ix)[1]-1).clone();
      (*$out)[2] = (*$source).index((*$ix)[2]-1).clone();
    }};}

macro_rules! access_1d_slice_bool {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      let mut j = 0;
      let out_len = (*$out).len();
      for i in 0..(*$ix).len() {
        if (*$ix)[i] == true {
          j += 1;
        }
      }
      if j != out_len {
        (*$out).resize_horizontally_mut(j,(*$out)[0]);
      }
      j = 0;
      for i in 0..(*$source).len() {
        if (*$ix)[i] == true {
          (*$out)[j] = (*$source).index(i).clone();
          j += 1;
        }
      }
    }};}

macro_rules! access_1d_slice_bool_v {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      let mut j = 0;
      let out_len = (*$out).len();
      for i in 0..(*$ix).len() {
        if (*$ix)[i] == true {
          j += 1;
        }
      }
      if j != out_len {
        (*$out).resize_vertically_mut(j,(*$out)[0]);
      }
      j = 0;
      for i in 0..(*$source).len() {
        if (*$ix)[i] == true {
          (*$out)[j] = (*$source).index(i).clone();
          j += 1;
        }
      }
    }};}    

macro_rules! access_2d_slice2 {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      (*$out)[0] = (*$source).index(((*$ix).0[0]-1,(*$ix).1[0]-1)).clone();
      (*$out)[1] = (*$source).index(((*$ix).0[1]-1,(*$ix).1[0]-1)).clone();
      (*$out)[2] = (*$source).index(((*$ix).0[0]-1,(*$ix).1[1]-1)).clone();
      (*$out)[3] = (*$source).index(((*$ix).0[1]-1,(*$ix).1[1]-1)).clone();
    }};}

macro_rules! access_2d_slice2_all {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      let n_cols = (*$out).ncols();
      let mut out_ix = 0;
      for i in 0..n_cols {
        (*$out)[out_ix] = (*$source).index(((*$ix)[0] - 1, i)).clone();
        out_ix += 1;
        (*$out)[out_ix] = (*$source).index(((*$ix)[1] - 1, i)).clone();
        out_ix += 1;
      }}};}

macro_rules! access_2d_slice3_all {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      let n_cols = (*$out).ncols();
      let mut out_ix = 0;
      for i in 0..n_cols {
        (*$out)[out_ix] = (*$source).index(((*$ix)[0] - 1, i)).clone();
        out_ix += 1;
        (*$out)[out_ix] = (*$source).index(((*$ix)[1] - 1, i)).clone();
        out_ix += 1;
        (*$out)[out_ix] = (*$source).index(((*$ix)[2] - 1, i)).clone();
        out_ix += 1;
      }}};}

macro_rules! access_2d_all_slice2 {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      let n_rows = (*$source).nrows();
      let n_cols = (*$ix).ncols();
      let mut out_ix = 0;
      for c in 0..n_cols {
        for r in 0..n_rows {
          (*$out)[out_ix] = (*$source).index((r,(*$ix)[c] - 1)).clone();
          out_ix += 1;
        }
      }
    }};}

macro_rules! access_col {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      for i in 0..(*$source).nrows() {
        (*$out)[i] = (*$source).index((i,*$ix-1)).clone();
      }
    }};}

macro_rules! access_row {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      for i in 0..(*$source).nrows() {
        (*$out)[i] = (*$source).index((*$ix-1,i)).clone();
      }
    }};}

macro_rules! access_1d_all {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      for i in 0..(*$source).len() {
        (*$out)[i] = (*$source).index(i).clone();
      }
    }};}

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

macro_rules! impl_access_fxn_shape {
  ($name:ident, $ix_type:ty, $out_type:ty, $fxn:ident) => {
    paste!{
      impl_access_fxn!([<$name V2>],   Vector2<T>,    $ix_type, $out_type, $fxn);
      impl_access_fxn!([<$name V3>],   Vector3<T>,    $ix_type, $out_type, $fxn);
      impl_access_fxn!([<$name V4>],   Vector4<T>,    $ix_type, $out_type, $fxn);
      impl_access_fxn!([<$name R2>],   RowVector2<T>, $ix_type, $out_type, $fxn);
      impl_access_fxn!([<$name R3>],   RowVector3<T>, $ix_type, $out_type, $fxn);
      impl_access_fxn!([<$name R4>],   RowVector4<T>, $ix_type, $out_type, $fxn);
      impl_access_fxn!([<$name M1>],   Matrix1<T>,    $ix_type, $out_type, $fxn);
      impl_access_fxn!([<$name M2>],   Matrix2<T>,    $ix_type, $out_type, $fxn);
      impl_access_fxn!([<$name M3>],   Matrix3<T>,    $ix_type, $out_type, $fxn);
      impl_access_fxn!([<$name M4>],   Matrix4<T>,    $ix_type, $out_type, $fxn);
      impl_access_fxn!([<$name M2x3>], Matrix2x3<T>,  $ix_type, $out_type, $fxn);
      impl_access_fxn!([<$name M3x2>], Matrix3x2<T>,  $ix_type, $out_type, $fxn);
      impl_access_fxn!([<$name MD>],   DMatrix<T>,    $ix_type, $out_type, $fxn);
      impl_access_fxn!([<$name RD>],   RowDVector<T>, $ix_type, $out_type, $fxn);
      impl_access_fxn!([<$name VD>],   DVector<T>,    $ix_type, $out_type, $fxn);
    }
  };}

// x[1]
impl_access_fxn_shape!(Access1DS, usize, T, access_1d);

// x[1,2]
impl_access_fxn_shape!(Access2DS, (usize,usize), T, access_2d);

// x[1..3]
impl_access_fxn_shape!(Access1DV2, Vector2<usize>, Vector2<T>, access_1d_slice2);
impl_access_fxn_shape!(Access1DV3, Vector3<usize>, Vector3<T>, access_1d_slice3);
impl_access_fxn_shape!(Access1DR2, RowVector2<usize>, RowVector2<T>, access_1d_slice2);
impl_access_fxn_shape!(Access1DR3, RowVector3<usize>, RowVector3<T>, access_1d_slice3);

impl_access_fxn_shape!(Access1DR2b, RowVector2<bool>, RowDVector<T>, access_1d_slice_bool);
impl_access_fxn_shape!(Access1DR3b, RowVector3<bool>, RowDVector<T>, access_1d_slice_bool);
impl_access_fxn_shape!(Access1DR4b, RowVector4<bool>, RowDVector<T>, access_1d_slice_bool);
impl_access_fxn_shape!(Access1DV2b, Vector2<bool>, DVector<T>, access_1d_slice_bool_v);
impl_access_fxn_shape!(Access1DV3b, Vector3<bool>, DVector<T>, access_1d_slice_bool_v);
impl_access_fxn_shape!(Access1DV4b, Vector4<bool>, DVector<T>, access_1d_slice_bool_v);

// x[1..3,1..3]
impl_access_fxn_shape!(Access2DR2, (RowVector2<usize>,RowVector2<usize>), Matrix2<T>, access_2d_slice2);

// x[:]
impl_access_fxn_shape!(Access1DA, Value, DVector<T>, access_1d_all);

// x[:,1]
impl_access_fxn_shape!(Access2DAS, usize, DVector<T>, access_col);

// x[1,:]
impl_access_fxn_shape!(Access2DSA, usize, RowDVector<T>, access_row);

// x[1..3,:]
impl_access_fxn_shape!(Access2DR2A, RowVector2<usize>, DMatrix<T>, access_2d_slice2_all);
impl_access_fxn_shape!(Access2DR3A, RowVector3<usize>, DMatrix<T>, access_2d_slice3_all);

// x[:,1..3]
impl_access_fxn_shape!(Access2DAR2, RowVector2<usize>, DMatrix<T>, access_2d_all_slice2);

// x.x,y,z

macro_rules! generate_access_match_arms {
  ($arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    match $arg {
      $(
        $(
          (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), [Value::Index(ix)]) => {
            Ok(Box::new(Access1DSR4{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::Index(ix)]) => {
            Ok(Box::new(Access1DSR3{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), [Value::Index(ix)]) => {
            Ok(Box::new(Access1DSR2{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)), [Value::Index(ix)]) => {
            Ok(Box::new(Access1DSV4{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)), [Value::Index(ix)]) => {
            Ok(Box::new(Access1DSV3{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)), [Value::Index(ix)]) => {
            Ok(Box::new(Access1DSV2{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::Index(ix)]) => {
            Ok(Box::new(Access1DSM2{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::Index(ix)]) => {
            Ok(Box::new(Access1DSM3{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::Index(ix)]) => {
            Ok(Box::new(Access1DSM2x3{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::Index(ix)]) => {
            Ok(Box::new(Access1DSM3x2{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::Index(ix)]) => {
            Ok(Box::new(Access1DSMD{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::Index(ix)]) => {
            Ok(Box::new(Access1DSRD{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)), [Value::Index(ix)]) => {
            Ok(Box::new(Access1DSVD{source: input.clone(), ixes: ix.clone(), out: new_ref($default) }))
          },
          // x[1,2]
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::Index(ix1),Value::Index(ix2)]) => {
            Ok(Box::new(Access2DSM2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::Index(ix1),Value::Index(ix2)]) => {
            Ok(Box::new(Access2DSM3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::Index(ix1),Value::Index(ix2)]) => {
            Ok(Box::new(Access2DSM2x3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::Index(ix1),Value::Index(ix2)]) => {
            Ok(Box::new(Access2DSM3x2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref($default) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::Index(ix1),Value::Index(ix2)]) => {
            Ok(Box::new(Access2DSMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref($default) }))
          },
          // x[1..3]
          (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::MatrixBool(Matrix::RowVector3(ix))]) => {
            let len = input.borrow().len();
            Ok(Box::new(Access1DR3bR3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(len,$default)) }))
          },    
          (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::MatrixIndex(Matrix::Vector2(ix))]) => {
            Ok(Box::new(Access1DV2R3{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector2::from_element($default)) }))
          },          
          (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::MatrixIndex(Matrix::RowVector2(ix))]) => {
            Ok(Box::new(Access1DR2R3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector2::from_element($default)) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::MatrixIndex(Matrix::RowVector2(ix))]) => {
            Ok(Box::new(Access1DR2RD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector2::from_element($default)) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::MatrixIndex(Matrix::RowVector3(ix))]) => {
            Ok(Box::new(Access1DR3RD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector3::from_element($default)) }))
          },
          // x[1..3,1..3]
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixIndex(Matrix::RowVector2(ix1)), Value::MatrixIndex(Matrix::RowVector2(ix2))]) => {
            Ok(Box::new(Access2DR2M3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(Matrix2::from_element($default)) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixIndex(Matrix::RowVector2(ix1)), Value::MatrixIndex(Matrix::RowVector2(ix2))]) => {
            Ok(Box::new(Access2DR2MD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(Matrix2::from_element($default)) }))
          },
          // x[:]
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::IndexAll]) => {
            Ok(Box::new(Access1DAM3x2{source: input.clone(), ixes: new_ref(Value::IndexAll), out: new_ref(DVector::from_element(6,$default)) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::IndexAll]) => {
            Ok(Box::new(Access1DAM3{source: input.clone(), ixes: new_ref(Value::IndexAll), out: new_ref(DVector::from_element(9,$default)) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::IndexAll]) => {
            Ok(Box::new(Access1DAM2{source: input.clone(), ixes: new_ref(Value::IndexAll), out: new_ref(DVector::from_element(4,$default)) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::IndexAll]) => {
            let len = input.borrow().len();
            Ok(Box::new(Access1DAMD{source: input.clone(), ixes: new_ref(Value::IndexAll), out: new_ref(DVector::from_element(len,$default)) }))
          },
          // x[:,2]
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::IndexAll,Value::Index(ix)]) => {
            Ok(Box::new(Access2DASM2{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(2,$default)) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::IndexAll,Value::Index(ix)]) => {
            Ok(Box::new(Access2DASM3{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(3,$default)) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::IndexAll,Value::Index(ix)]) => {
            let len = input.borrow().nrows();
            Ok(Box::new(Access2DASMD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(len,$default)) }))
          },
          // x[2,:]
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::Index(ix),Value::IndexAll]) => {
            Ok(Box::new(Access2DSAM2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(2,$default)) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::Index(ix),Value::IndexAll]) => {
            Ok(Box::new(Access2DSAM3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(3,$default)) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::Index(ix),Value::IndexAll]) => {
            let len = input.borrow().ncols();
            Ok(Box::new(Access2DSAMD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(len,$default)) }))
          },
          // x[1..3,:]
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixIndex(Matrix::RowVector2(ix)), Value::IndexAll]) => {
            let cols = input.borrow().ncols();
            Ok(Box::new(Access2DR2AM3{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(2,3,$default)) }))
          },
          (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixIndex(Matrix::RowVector2(ix)), Value::IndexAll]) => {
            let cols = input.borrow().ncols();
            Ok(Box::new(Access2DR2AMD{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(2,cols,$default)) }))
          },
          // x[:,1..3]
          (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::IndexAll, Value::MatrixIndex(Matrix::RowVector2(ix))]) => {
            let rows = input.borrow().nrows();
            Ok(Box::new(Access2DAR2MD{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(rows,2,$default)) }))
          },
          // x[:,1..3]
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::IndexAll, Value::MatrixIndex(Matrix::RowVector2(ix))]) => {
            Ok(Box::new(Access2DAR2M3{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(3,2,$default)) }))
          },
        )+
      )+
      x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: 314, kind: MechErrorKind::UnhandledFunctionArgumentKind }),
    }
  }
}

fn generate_access_fxn(lhs_value: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  generate_access_match_arms!(
    (lhs_value, ixes.as_slice()),
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

pub struct MatrixAccess {}
impl NativeFunctionCompiler for MatrixAccess {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match generate_access_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { generate_access_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

macro_rules! generate_access_formula_match_arms {
  ($fxn_name:ident, $ix:tt, $ixes:expr, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
              // x[1]
              (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), $ix) => {
                Ok(Box::new([<$fxn_name R4>]{source: input.clone(), ixes: $ixes, out: new_ref($default) }))
              },
              (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), $ix) => {
                Ok(Box::new([<$fxn_name R3>]{source: input.clone(), ixes: $ixes, out: new_ref($default) }))
              },
              (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), $ix) => {
                Ok(Box::new([<$fxn_name R2>]{source: input.clone(), ixes: $ixes, out: new_ref($default) }))
              },
              (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)), $ix) => {
                Ok(Box::new([<$fxn_name V4>]{source: input.clone(), ixes: $ixes, out: new_ref($default) }))
              },
              (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)), $ix) => {
                Ok(Box::new([<$fxn_name V3>]{source: input.clone(), ixes: $ixes, out: new_ref($default) }))
              },
              (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)), $ix) => {
                Ok(Box::new([<$fxn_name V2>]{source: input.clone(), ixes: $ixes, out: new_ref($default) }))
              },
              (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), $ix) => {
                Ok(Box::new([<$fxn_name M2>]{source: input.clone(), ixes: $ixes, out: new_ref($default) }))
              },
              (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), $ix) => {
                Ok(Box::new([<$fxn_name M3>]{source: input.clone(), ixes: $ixes, out: new_ref($default) }))
              },
              (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), $ix) => {
                Ok(Box::new([<$fxn_name M4>]{source: input.clone(), ixes: $ixes, out: new_ref($default) }))
              },
              (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), $ix) => {
                Ok(Box::new([<$fxn_name M2x3>]{source: input.clone(), ixes: $ixes, out: new_ref($default) }))
              },
              (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), $ix) => {
                Ok(Box::new([<$fxn_name M3x2>]{source: input.clone(), ixes: $ixes, out: new_ref($default) }))
              },
              (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), $ix) => {
                Ok(Box::new([<$fxn_name MD>]{source: input.clone(), ixes: $ixes, out: new_ref($default) }))
              },
              (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), $ix) => {
                Ok(Box::new([<$fxn_name RD>]{source: input.clone(), ixes: $ixes, out: new_ref($default) }))
              },
              (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)), $ix) => {
                Ok(Box::new([<$fxn_name VD>]{source: input.clone(), ixes: $ixes, out: new_ref($default) }))
              },
          
          )+
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: 314, kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn generate_access_formula_fxn(lhs_value: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  generate_access_formula_match_arms!(
    Access1DS,
    [Value::Index(ix)],ix.clone(),
    (lhs_value, ixes.as_slice()),
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

pub struct MatrixAccessFormula {}
impl NativeFunctionCompiler for MatrixAccessFormula {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match generate_access_formula_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { generate_access_formula_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}