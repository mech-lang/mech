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
impl_binop!(MatMulM1M1, Matrix1<T>, Matrix1<T>, Matrix1<T>,matmul_op);
impl_binop!(MatMulM2M2, Matrix2<T>, Matrix2<T>, Matrix2<T>,matmul_op);
impl_binop!(MatMulM3M3, Matrix3<T>, Matrix3<T>, Matrix3<T>,matmul_op);
impl_binop!(MatMulM4M4, Matrix4<T>, Matrix4<T>, Matrix4<T>,matmul_op);
impl_binop!(MatMulR2V2, RowVector2<T>,Vector2<T>,Matrix1<T>,matmul_op);
impl_binop!(MatMulR3V3, RowVector3<T>,Vector3<T>,Matrix1<T>,matmul_op);
impl_binop!(MatMulR4V4, RowVector4<T>,Vector4<T>,Matrix1<T>,matmul_op);
impl_binop!(MatMulV2R2, Vector2<T>, RowVector2<T>, Matrix2<T>,matmul_op);
impl_binop!(MatMulV3R3, Vector3<T>, RowVector3<T>, Matrix3<T>,matmul_op);
impl_binop!(MatMulV4R4, Vector4<T>, RowVector4<T>, Matrix4<T>,matmul_op);
impl_binop!(MatMulRDVD, RowDVector<T>, DVector<T>, Matrix1<T>,matmul_op);
impl_binop!(MatMulVDRD, DVector<T>,RowDVector<T>,DMatrix<T>,matmul_op);
impl_binop!(MatMulMDMD, DMatrix<T>,DMatrix<T>,DMatrix<T>,matmul_op);

macro_rules! impl_matmul_match_arms {
  ($arg:expr, $($lhs_type:ident, $rhs_type:ident => $($matrix_kind:ident, $target_type:ident),+);+ $(;)?) => {
    match $arg {
      $(
        $(
          (Value::$lhs_type(lhs), Value::$rhs_type(rhs)) => Ok(Box::new(MatMulScalar { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref($target_type::zero()) })),
          (Value::$matrix_kind(Matrix::<$target_type>::Vector4(lhs)), Value::$matrix_kind(Matrix::<$target_type>::RowVector4(rhs))) => Ok(Box::new(MatMulV4R4 { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Matrix4::from_element($target_type::zero())) })),
          (Value::$matrix_kind(Matrix::<$target_type>::Vector3(lhs)), Value::$matrix_kind(Matrix::<$target_type>::RowVector3(rhs))) => Ok(Box::new(MatMulV3R3 { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Matrix3::from_element($target_type::zero())) })),
          (Value::$matrix_kind(Matrix::<$target_type>::Vector2(lhs)), Value::$matrix_kind(Matrix::<$target_type>::RowVector2(rhs))) => Ok(Box::new(MatMulV2R2 { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Matrix2::from_element($target_type::zero())) })),
          (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Vector4(rhs))) => Ok(Box::new(MatMulR4V4 { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Matrix1::from_element($target_type::zero())) })),
          (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Vector3(rhs))) => Ok(Box::new(MatMulR3V3 { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Matrix1::from_element($target_type::zero())) })),
          (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Vector2(rhs))) => Ok(Box::new(MatMulR2V2 { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Matrix1::from_element($target_type::zero())) })),
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix1(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Matrix1(rhs))) => Ok(Box::new(MatMulM1M1{lhs, rhs, out: new_ref(Matrix1::from_element($target_type::zero()))})),
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Matrix2(rhs))) => Ok(Box::new(MatMulM2M2{lhs, rhs, out: new_ref(Matrix2::from_element($target_type::zero()))})),
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Matrix3(rhs))) => Ok(Box::new(MatMulM3M3{lhs, rhs, out: new_ref(Matrix3::from_element($target_type::zero()))})),
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Matrix4(rhs))) => Ok(Box::new(MatMulM4M4{lhs, rhs, out: new_ref(Matrix4::from_element($target_type::zero()))})),
          (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(rhs))) => Ok(Box::new(MatMulM2x3M3x2{lhs, rhs, out: new_ref(Matrix2::from_element($target_type::zero()))})),          
          (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(lhs)), Value::$matrix_kind(Matrix::<$target_type>::DVector(rhs))) => Ok(Box::new(MatMulRDVD{lhs, rhs, out: new_ref(Matrix1::from_element($target_type::zero()))})),
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

fn impl_matmul_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_matmul_match_arms!(
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

impl_mech_binop_fxn!(MatrixMatMul,impl_matmul_fxn);

// Transpose ------------------------------------------------------------------

macro_rules! transpose_op {
  ($arg:expr, $out:expr) => {
    unsafe { *$out = (*$arg).transpose(); }
  };}

impl_bool_urop!(TransposeM1, Matrix1<T>, Matrix1<T>, transpose_op);
impl_bool_urop!(TransposeM2, Matrix2<T>, Matrix2<T>, transpose_op);
impl_bool_urop!(TransposeM3, Matrix3<T>, Matrix3<T>, transpose_op);
impl_bool_urop!(TransposeM4, Matrix4<T>, Matrix4<T>, transpose_op);
impl_bool_urop!(TransposeM2x3, Matrix2x3<T>, Matrix3x2<T>, transpose_op);
impl_bool_urop!(TransposeM3x2, Matrix3x2<T>, Matrix2x3<T>, transpose_op);
impl_bool_urop!(TransposeV2, Vector2<T>, RowVector2<T>, transpose_op);
impl_bool_urop!(TransposeV3, Vector3<T>, RowVector3<T>, transpose_op);
impl_bool_urop!(TransposeV4, Vector4<T>, RowVector4<T>, transpose_op); 
impl_bool_urop!(TransposeR2, RowVector2<T>, Vector2<T>, transpose_op);
impl_bool_urop!(TransposeR3, RowVector3<T>, Vector3<T>, transpose_op);
impl_bool_urop!(TransposeR4, RowVector4<T>, Vector4<T>, transpose_op); 
impl_bool_urop!(TransposeRD, RowDVector<T>, DVector<T>, transpose_op);
impl_bool_urop!(TransposeVD, DVector<T>, RowDVector<T>, transpose_op);
impl_bool_urop!(TransposeMD, DMatrix<T>, DMatrix<T>, transpose_op);

macro_rules! impl_transpose_match_arms {
  ($arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    match $arg {
      $(
        $(
          Value::$matrix_kind(Matrix::<$target_type>::Vector4(arg))    => Ok(Box::new(TransposeV4{arg: arg.clone(), out: new_ref(RowVector4::from_element($default)) })),
          Value::$matrix_kind(Matrix::<$target_type>::Vector3(arg))    => Ok(Box::new(TransposeV3{arg: arg.clone(), out: new_ref(RowVector3::from_element($default)) })),
          Value::$matrix_kind(Matrix::<$target_type>::Vector2(arg))    => Ok(Box::new(TransposeV2{arg: arg.clone(), out: new_ref(RowVector2::from_element($default)) })),
          Value::$matrix_kind(Matrix::<$target_type>::DVector(arg))    => Ok(Box::new(TransposeVD{arg: arg.clone(), out: new_ref(RowDVector::from_element(arg.borrow().len(),$default))})),
          Value::$matrix_kind(Matrix::<$target_type>::Matrix1(arg))    => Ok(Box::new(TransposeM1{arg: arg.clone(), out: new_ref(Matrix1::from_element($default))})),
          Value::$matrix_kind(Matrix::<$target_type>::Matrix2(arg))    => Ok(Box::new(TransposeM2{arg: arg.clone(), out: new_ref(Matrix2::from_element($default))})),
          Value::$matrix_kind(Matrix::<$target_type>::Matrix3(arg))    => Ok(Box::new(TransposeM3{arg: arg.clone(), out: new_ref(Matrix3::from_element($default))})),
          Value::$matrix_kind(Matrix::<$target_type>::Matrix4(arg))    => Ok(Box::new(TransposeM4{arg: arg.clone(), out: new_ref(Matrix4::from_element($default))})),
          Value::$matrix_kind(Matrix::<$target_type>::RowVector4(arg)) => Ok(Box::new(TransposeR4{arg: arg.clone(), out: new_ref(Vector4::from_element($default)) })),
          Value::$matrix_kind(Matrix::<$target_type>::RowVector3(arg)) => Ok(Box::new(TransposeR3{arg: arg.clone(), out: new_ref(Vector3::from_element($default)) })),
          Value::$matrix_kind(Matrix::<$target_type>::RowVector2(arg)) => Ok(Box::new(TransposeR2{arg: arg.clone(), out: new_ref(Vector2::from_element($default)) })),
          Value::$matrix_kind(Matrix::<$target_type>::RowDVector(arg)) => Ok(Box::new(TransposeRD{arg: arg.clone(), out: new_ref(DVector::from_element(arg.borrow().len(),$default))})),
          Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(arg))  => Ok(Box::new(TransposeM2x3{arg: arg.clone(), out: new_ref(Matrix3x2::from_element($default))})),          
          Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(arg))  => Ok(Box::new(TransposeM3x2{arg: arg.clone(), out: new_ref(Matrix2x3::from_element($default))})),          
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

fn impl_transpose_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_transpose_match_arms!(
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
  
impl_mech_urnop_fxn!(MatrixTranspose,impl_transpose_fxn);

// Access ---------------------------------------------------------------------

macro_rules! access_1d {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { *$out = (*$source).index(*$ix - 1).clone() }
  };}

macro_rules! access_2d {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      let ix1 = (*$ix).0;
      let ix2 = (*$ix).1;
      *$out = (*$source).index((ix1 - 1, ix2 - 1)).clone() 
    }
  };}
macro_rules! access_1d_slice {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      for i in 0..(*$ix).len() {
        (*$out)[i] = (*$source).index((*$ix)[i] - 1).clone();
      }
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
        (*$out).resize_horizontally_mut(j, (*$out)[0]);
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
        (*$out).resize_vertically_mut(j, (*$out)[0]);
      }
      j = 0;
      for i in 0..(*$source).len() {
        if (*$ix)[i] == true {
          (*$out)[j] = (*$source).index(i).clone();
          j += 1;
        }
      }
    }};}    

macro_rules! access_2d_row_slice_bool {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      let scalar_ix = &(*$ix).0;
      let vec_ix = &(*$ix).1;
      let mut j = 0;
      let out_len = (*$out).len();
      for i in 0..vec_ix.len() {
        if vec_ix[i] == true {
          j += 1;
        }
      }
      if j != out_len {
        (*$out).resize_horizontally_mut(j, (*$out)[0]);
      }
      j = 0;
      for i in 0..vec_ix.len() {
        if vec_ix[i] == true {
          (*$out)[j] = (*$source).index((scalar_ix - 1, i)).clone();
          j += 1;
        }
      }
    }};}

macro_rules! access_2d_col_slice_bool {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      let vec_ix = &(*$ix).0;
      let scalar_ix = &(*$ix).1;
      let mut j = 0;
      let out_len = (*$out).len();
      for i in 0..vec_ix.len() {
        if vec_ix[i] == true {
          j += 1;
        }
      }
      if j != out_len {
        (*$out).resize_vertically_mut(j, (*$out)[0]);
      }
      j = 0;
      for i in 0..vec_ix.len() {
        if vec_ix[i] == true {
          (*$out)[j] = (*$source).index((i, scalar_ix - 1)).clone();
          j += 1;
        }
      }
    }};}    

macro_rules! access_2d_slice {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      let nrows = (*$ix).0.len();
      let ncols = (*$ix).1.len();
      let mut out_ix = 0;
      for j in 0..ncols {
        for i in 0..nrows {
          (*$out)[out_ix] = (*$source).index(((*$ix).0[i] - 1, (*$ix).1[j] - 1)).clone();
          out_ix += 1;
        }
      }
    }};}

macro_rules! access_2d_slice_bool {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      let ix1 = &(*$ix).0;
      let ix2 = &(*$ix).1;
      let mut j = 0;
      let out_len = (*$out).len();
      for i in 0..ix1.len() {
        if ix1[i] == true {
          j += 1;
        }
      }
      if j != (*$out).nrows() {
        (*$out).resize_vertically_mut(j, (*$out)[0]);
      }
      j = 0;
      for k in 0..ix2.len() {
        for i in 0..ix1.len() {
          if ix1[i] == true {
            (*$out)[j] = (*$source).index((i, ix2[k] - 1)).clone();
            j += 1;
          }
        }
      }
    }};}  
    
macro_rules! access_2d_slice_bool2 {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      let ix1 = &(*$ix).0;
      let ix2 = &(*$ix).1;
      let mut j = 0;
      let out_len = (*$out).len();
      for i in 0..ix2.len() {
        if ix2[i] == true {
          j += 1;
        }
      }
      if j != (*$out).ncols() {
        (*$out).resize_horizontally_mut(j, (*$out)[0]);
      }
      j = 0;
      for k in 0..ix2.len() {
        for i in 0..ix1.len() {
          if ix2[k] == true {
            (*$out)[j] = (*$source).index((ix1[i] - 1, k)).clone();
            j += 1;
          }
        }
      }
    }};}    

macro_rules! access_2d_slice_bool_bool {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      let ix1 = &(*$ix).0;
      let ix2 = &(*$ix).1;
      let mut k = 0;
      let mut j = 0;
      let out_len = (*$out).len();
      for i in 0..ix1.len() {
        if ix1[i] == true {
          j += 1;
        }
      }
      for i in 0..ix2.len() {
        if ix2[i] == true {
          k += 1;
        }
      }
      if j != (*$out).nrows() || k != (*$out).ncols() {
        (*$out).resize_mut(j, k, (*$out)[0]);
      }
      let mut out_ix = 0;
      for k in 0..ix2.len() {
        for j in 0..ix1.len() {
          if ix1[j] == true && ix2[k] == true {
            (*$out)[out_ix] = (*$source).index((j, k)).clone();
            out_ix += 1;
          }
        }
      }
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

macro_rules! access_2d_slice_all {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      let n_rows = (*$source).nrows();
      let n_cols = (*$ix).ncols();
      let mut out_ix = 0;
      for c in 0..n_cols {
        for r in 0..n_rows {
          (*$out)[out_ix] = (*$source).index(((*$ix)[r] - 1),c).clone();
          out_ix += 1;
        }
      }
    }};}

macro_rules! access_2d_slice_all_bool {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      let vec_ix = &(*$ix);
      let mut j = 0;
      let out_len = (*$out).len();
      for i in 0..vec_ix.len() {
        if vec_ix[i] == true {
          j += 1;
        }
      }
      if j != out_len {
        (*$out).resize_vertically_mut(j, (*$out)[0]);
      }
      j = 0;
      for i in 0..vec_ix.len() {
        for k in 0..(*$source).ncols() {
          if vec_ix[i] == true {
            (*$out)[j] = (*$source).index((i, k)).clone();
            j += 1;
          }
        }
      }
    }};}

macro_rules! access_2d_all_slice_bool {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      let vec_ix = &(*$ix);
      let mut j = 0;
      let out_len = (*$out).len();
      for i in 0..vec_ix.len() {
        if vec_ix[i] == true {
          j += 1;
        }
      }
      if j != out_len {
        (*$out).resize_horizontally_mut(j, (*$out)[0]);
      }
      j = 0;
      for k in 0..(*$source).nrows() {
        for i in 0..vec_ix.len() {
          if vec_ix[i] == true {
            (*$out)[j] = (*$source).index((k, i)).clone();
            j += 1;
          }
        }
      }
    }};}

macro_rules! access_2d_all_slice {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      let n_rows = (*$source).nrows();
      let n_cols = (*$ix).ncols();
      let mut out_ix = 0;
      for c in 0..n_cols {
        for r in 0..n_rows {
          (*$out)[out_ix] = (*$source).index((r, (*$ix)[c] - 1)).clone();
          out_ix += 1;
        }
      }
    }};}

macro_rules! access_2d_row_slice {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      let ix1 = &(*$ix).0;
      let ix2 = &(*$ix).1;
      let out_cols = ix2.ncols();
      let mut out_ix = 0;
      for c in 0..out_cols {
        (*$out)[out_ix] = (*$source).index((ix1 - 1, ix2[c] - 1)).clone();
        out_ix += 1;
      }
    }};}    

macro_rules! access_2d_col_slice {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      let ix1 = &(*$ix).0;
      let ix2 = &(*$ix).1;
      let out_rows = ix1.ncols();
      let mut out_ix = 0;
      for c in 0..out_rows {
        (*$out)[out_ix] = (*$source).index((ix1[c] - 1, ix2 - 1)).clone();
        out_ix += 1;
      }
    }};}    

macro_rules! access_col {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      for i in 0..(*$source).nrows() {
        (*$out)[i] = (*$source).index((i, *$ix - 1)).clone();
      }
    }};}

macro_rules! access_row {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      for i in 0..(*$source).nrows() {
        (*$out)[i] = (*$source).index((*$ix - 1, i)).clone();
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
impl_access_fxn_shape!(Access2DSS, (usize,usize), T, access_2d);

// x[1..3]
impl_access_fxn_shape!(Access1DVD, DVector<usize>, DVector<T>, access_1d_slice);
impl_access_fxn_shape!(Access1DRD, RowDVector<usize>, RowDVector<T>, access_1d_slice);

impl_access_fxn_shape!(Access1DRDb, RowDVector<bool>, RowDVector<T>, access_1d_slice_bool);
impl_access_fxn_shape!(Access1DVDb, DVector<bool>, DVector<T>, access_1d_slice_bool_v);

// x[1..3,1..3]
impl_access_fxn_shape!(Access2DRDRD, (RowDVector<usize>,RowDVector<usize>), DMatrix<T>, access_2d_slice);
impl_access_fxn_shape!(Access2DVDVD, (DVector<usize>,DVector<usize>),       DMatrix<T>, access_2d_slice);
impl_access_fxn_shape!(Access2DVDRD, (DVector<usize>,RowDVector<usize>),    DMatrix<T>, access_2d_slice);
impl_access_fxn_shape!(Access2DRDVD, (RowDVector<usize>,DVector<usize>),    DMatrix<T>, access_2d_slice);

impl_access_fxn_shape!(Access2DRDbRD, (RowDVector<bool>,RowDVector<usize>), DMatrix<T>, access_2d_slice_bool);
impl_access_fxn_shape!(Access2DRDRDb, (RowDVector<usize>,RowDVector<bool>), DMatrix<T>, access_2d_slice_bool2);
impl_access_fxn_shape!(Access2DRDbRDb, (RowDVector<bool>,RowDVector<bool>), DMatrix<T>, access_2d_slice_bool_bool);

impl_access_fxn_shape!(Access2DVDbVD, (DVector<bool>,DVector<usize>), DMatrix<T>, access_2d_slice_bool);
impl_access_fxn_shape!(Access2DVDVDb, (DVector<usize>,DVector<bool>), DMatrix<T>, access_2d_slice_bool2);
impl_access_fxn_shape!(Access2DVDbVDb, (DVector<bool>,DVector<bool>), DMatrix<T>, access_2d_slice_bool_bool);

impl_access_fxn_shape!(Access2DRDbVD, (RowDVector<bool>,DVector<usize>), DMatrix<T>, access_2d_slice_bool);
impl_access_fxn_shape!(Access2DRDVDb, (RowDVector<usize>,DVector<bool>), DMatrix<T>, access_2d_slice_bool2);
impl_access_fxn_shape!(Access2DRDbVDb, (RowDVector<bool>,DVector<bool>), DMatrix<T>, access_2d_slice_bool_bool);

impl_access_fxn_shape!(Access2DVDbRD, (DVector<bool>,RowDVector<usize>), DMatrix<T>, access_2d_slice_bool);
impl_access_fxn_shape!(Access2DVDRDb, (DVector<usize>,RowDVector<bool>), DMatrix<T>, access_2d_slice_bool2);
impl_access_fxn_shape!(Access2DVDbRDb, (DVector<bool>,RowDVector<bool>), DMatrix<T>, access_2d_slice_bool_bool);

// x[:]
impl_access_fxn_shape!(Access1DA, Value, DVector<T>, access_1d_all);

// x[:,1]
impl_access_fxn_shape!(Access2DAS, usize, DVector<T>, access_col);

// x[1,:]
impl_access_fxn_shape!(Access2DSA, usize, RowDVector<T>, access_row);

// x[1..3,:]
impl_access_fxn_shape!(Access2DRDA, RowDVector<usize>, DMatrix<T>, access_2d_slice2_all);
impl_access_fxn_shape!(Access2DVDA, DVector<usize>,    DMatrix<T>, access_2d_slice3_all);
impl_access_fxn_shape!(Access2DVDbA, DVector<bool>,    DMatrix<T>, access_2d_slice_all_bool);
impl_access_fxn_shape!(Access2DRDbA, RowDVector<bool>, DMatrix<T>, access_2d_slice_all_bool);

// x[:,1..3]
impl_access_fxn_shape!(Access2DARD, RowDVector<usize>, DMatrix<T>, access_2d_all_slice);
impl_access_fxn_shape!(Access2DAVD, DVector<usize>,    DMatrix<T>, access_2d_all_slice);
impl_access_fxn_shape!(Access2DAVDb, DVector<bool>,    DMatrix<T>, access_2d_all_slice_bool);
impl_access_fxn_shape!(Access2DARDb, RowDVector<bool>, DMatrix<T>, access_2d_all_slice_bool);

// x[2,1..3]
impl_access_fxn_shape!(Access2DSRD,  (usize, RowDVector<usize>), RowDVector<T>, access_2d_row_slice);
impl_access_fxn_shape!(Access2DSVD,  (usize, DVector<usize>),    RowDVector<T>, access_2d_row_slice);
impl_access_fxn_shape!(Access2DSRDb, (usize, RowDVector<bool>),  RowDVector<T>, access_2d_row_slice_bool);
impl_access_fxn_shape!(Access2DSVDb, (usize, DVector<bool>),     RowDVector<T>, access_2d_row_slice_bool);

// x[1..3,2]
impl_access_fxn_shape!(Access2DRDS,  (RowDVector<usize>, usize), DVector<T>, access_2d_col_slice);
impl_access_fxn_shape!(Access2DVDS,  (DVector<usize>, usize),    DVector<T>, access_2d_col_slice);
impl_access_fxn_shape!(Access2DRDbS, (RowDVector<bool>, usize),  DVector<T>, access_2d_col_slice_bool);
impl_access_fxn_shape!(Access2DVDbS, (DVector<bool>, usize),     DVector<T>, access_2d_col_slice_bool);

macro_rules! impl_access_match_arms {
  ($fxn_name:ident,$macro_name:ident, $arg:expr) => {
    paste!{
      [<impl_access_ $macro_name _match_arms>]!(
        $fxn_name,
        $arg,
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
  }
}

// x[1] -----------------------------------------------------------------------

macro_rules! impl_access_scalar_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), [Value::Index(ix)]) => Ok(Box::new([<$fxn_name R4>]  {source: input.clone(), ixes: ix.clone(), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::Index(ix)]) => Ok(Box::new([<$fxn_name R3>]  {source: input.clone(), ixes: ix.clone(), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), [Value::Index(ix)]) => Ok(Box::new([<$fxn_name R2>]  {source: input.clone(), ixes: ix.clone(), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)),    [Value::Index(ix)]) => Ok(Box::new([<$fxn_name V4>]  {source: input.clone(), ixes: ix.clone(), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)),    [Value::Index(ix)]) => Ok(Box::new([<$fxn_name V3>]  {source: input.clone(), ixes: ix.clone(), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)),    [Value::Index(ix)]) => Ok(Box::new([<$fxn_name V2>]  {source: input.clone(), ixes: ix.clone(), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)),    [Value::Index(ix)]) => Ok(Box::new([<$fxn_name M2>]  {source: input.clone(), ixes: ix.clone(), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)),    [Value::Index(ix)]) => Ok(Box::new([<$fxn_name M3>]  {source: input.clone(), ixes: ix.clone(), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)),    [Value::Index(ix)]) => Ok(Box::new([<$fxn_name M4>]  {source: input.clone(), ixes: ix.clone(), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)),  [Value::Index(ix)]) => Ok(Box::new([<$fxn_name M2x3>]{source: input.clone(), ixes: ix.clone(), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)),  [Value::Index(ix)]) => Ok(Box::new([<$fxn_name M3x2>]{source: input.clone(), ixes: ix.clone(), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),    [Value::Index(ix)]) => Ok(Box::new([<$fxn_name MD>]  {source: input.clone(), ixes: ix.clone(), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::Index(ix)]) => Ok(Box::new([<$fxn_name RD>]  {source: input.clone(), ixes: ix.clone(), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)),    [Value::Index(ix)]) => Ok(Box::new([<$fxn_name VD>]  {source: input.clone(), ixes: ix.clone(), out: new_ref($default) })),
          )+
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_access_scalar_fxn(lhs_value: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_access_match_arms!(Access1DS, scalar, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessScalar {}
impl NativeFunctionCompiler for MatrixAccessScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match impl_access_scalar_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { impl_access_scalar_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

#[derive(Debug)]
struct Set1DSR2 {
  source: Ref<F64>,
  ixes: Ref<usize>,
  sink: Ref<RowVector2<F64>>,
}
impl MechFunction for Set1DSR2 {
  fn solve(&self) {
    let sink_ptr = self.sink.as_ptr();
    let ixes_ptr = self.ixes.as_ptr();
    let source_ptr = self.source.as_ptr();
    unsafe {
      (*sink_ptr)[*ixes_ptr - 1] = (*source_ptr).clone();
    }
  }
  fn out(&self) -> Value { Value::MatrixF64(Matrix::RowVector2(self.sink.clone())) }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

pub struct MatrixSetScalar {}
impl NativeFunctionCompiler for MatrixSetScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match (sink, ixes.as_slice(), source) {
      (Value::MatrixF64(Matrix::<F64>::RowVector2(input)),
        [Value::Index(ix)], 
        Value::F64(source)) => {
        Ok(Box::new(Set1DSR2{sink: input.clone(), ixes: ix.clone(), source: source.clone()}))
      }
      x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
    }
  }
}

// x[1,2] ---------------------------------------------------------------------

macro_rules! impl_access_scalar_scalar_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), [Value::Index(ix1),Value::Index(ix2)]) => Ok(Box::new([<$fxn_name R4>]  {source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::Index(ix1),Value::Index(ix2)]) => Ok(Box::new([<$fxn_name R3>]  {source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), [Value::Index(ix1),Value::Index(ix2)]) => Ok(Box::new([<$fxn_name R2>]  {source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)),    [Value::Index(ix1),Value::Index(ix2)]) => Ok(Box::new([<$fxn_name V4>]  {source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)),    [Value::Index(ix1),Value::Index(ix2)]) => Ok(Box::new([<$fxn_name V3>]  {source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)),    [Value::Index(ix1),Value::Index(ix2)]) => Ok(Box::new([<$fxn_name V2>]  {source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)),    [Value::Index(ix1),Value::Index(ix2)]) => Ok(Box::new([<$fxn_name M2>]  {source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)),    [Value::Index(ix1),Value::Index(ix2)]) => Ok(Box::new([<$fxn_name M3>]  {source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)),    [Value::Index(ix1),Value::Index(ix2)]) => Ok(Box::new([<$fxn_name M4>]  {source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)),  [Value::Index(ix1),Value::Index(ix2)]) => Ok(Box::new([<$fxn_name M2x3>]{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)),  [Value::Index(ix1),Value::Index(ix2)]) => Ok(Box::new([<$fxn_name M3x2>]{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),    [Value::Index(ix1),Value::Index(ix2)]) => Ok(Box::new([<$fxn_name MD>]  {source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::Index(ix1),Value::Index(ix2)]) => Ok(Box::new([<$fxn_name RD>]  {source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)),    [Value::Index(ix1),Value::Index(ix2)]) => Ok(Box::new([<$fxn_name VD>]  {source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref($default) })),
          )+
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: 315, kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_access_scalar_scalar_fxn(lhs_value: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_access_match_arms!(Access2DSS, scalar_scalar, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessScalarScalar {}
impl NativeFunctionCompiler for MatrixAccessScalarScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match impl_access_scalar_scalar_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { impl_access_scalar_scalar_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3] --------------------------------------------------------------------

macro_rules! impl_access_range_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbR2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), [Value::MatrixBool(Matrix::DVector(ix))])     => Ok(Box::new(Access1DVDbR2{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbR3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::MatrixBool(Matrix::DVector(ix))])     => Ok(Box::new(Access1DVDbR3{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbR4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), [Value::MatrixBool(Matrix::DVector(ix))])     => Ok(Box::new(Access1DVDbR4{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbRD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::MatrixBool(Matrix::DVector(ix))])     => Ok(Box::new(Access1DVDbRD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),   

            // --

            (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbV2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDbV2{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbV3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDbV3{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbV4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDbV4{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbVD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDbVD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),   

            // --

            (Value::$matrix_kind(Matrix::<$target_type>::Matrix1(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbM1{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix1(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDbM1{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbM2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDbM2{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbM3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDbM3{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbM4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDbM4{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),              
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbM3x2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDbM3x2{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),              
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbM2x3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDbM2x3{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),              
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbMD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDbMD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),   

            // --

            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDR2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDR2{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDR3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDR3{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDR4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDR4{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),                
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDRD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDRD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),   

            // --

            (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDV2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDV2{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDV3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDV3{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDV4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDV4{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),                
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDVD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDVD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),   

            // --

            (Value::$matrix_kind(Matrix::<$target_type>::Matrix1(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDM1{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix1(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDM1{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDM2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDM2{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDM3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDM3{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDM4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDM4{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),                
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))]) => Ok(Box::new(Access1DRDM3x2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::MatrixIndex(Matrix::DVector(ix))]) => Ok(Box::new(Access1DVDM3x2{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))]) => Ok(Box::new(Access1DRDM2x3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::MatrixIndex(Matrix::DVector(ix))]) => Ok(Box::new(Access1DVDM2x3{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))]) => Ok(Box::new(Access1DRDMD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDMD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),   
          )+
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_access_range_fxn(lhs_value: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_access_match_arms!(Access1DR, range, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessRange {}
impl NativeFunctionCompiler for MatrixAccessRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match impl_access_range_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { impl_access_range_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3,1..3] ---------------------------------------------------------------

macro_rules! impl_access_range_range_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDRDM3x2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDRDM2x3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix1(input)), [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDRDM1{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDRDM2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDRDM3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDRDM4{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDRDMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),

            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDVDM3x2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDVDM2x3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix1(input)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDVDM1{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDVDM2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDVDM3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDVDM4{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDVDMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
           
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDVDM3x2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDVDM2x3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix1(input)),   [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDVDM1{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)),   [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDVDM2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)),   [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDVDM3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)),   [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDVDM4{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDVDMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
           
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDRDM3x2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDRDM2x3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix1(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDRDM1{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDRDM2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDRDM3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDRDM4{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDRDMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
           
            // --

            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDbRDM3x2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDbRDM2x3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix1(input)),   [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDbRDM1{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)),   [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDbRDM2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)),   [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDbRDM3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)),   [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDbRDM4{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDbRDMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),

            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDRDbM3x2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDRDbM2x3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix1(input)),   [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDRDbM1{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)),   [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDRDbM2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)),   [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDRDbM3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)),   [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDRDbM4{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDRDbMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),

            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDbRDbM3x2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDbRDbM2x3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix1(input)),   [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDbRDbM1{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)),   [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDbRDbM2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)),   [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDbRDbM3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)),   [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDbRDbM4{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DRDbRDbMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            
            // --

            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDbVDM3x2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDbVDM2x3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix1(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDbVDM1{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDbVDM2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDbVDM3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDbVDM4{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDbVDMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),

            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDVDbM3x2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDVDbM2x3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix1(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDVDbM1{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDVDbM2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDVDbM3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDVDbM4{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDVDbMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),

            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDbVDbM3x2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDbVDbM2x3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix1(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDbVDbM1{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDbVDbM2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDbVDbM3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDbVDbM4{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDbVDbMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            
            // --
            
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDbVDM3x2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDbVDM2x3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix1(input)),   [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDbVDM1{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)),   [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDbVDM2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)),   [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDbVDM3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)),   [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDbVDM4{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDbVDMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),

            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDVDbM3x2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDVDbM2x3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix1(input)),   [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDVDbM1{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)),   [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDVDbM2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)),   [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDVDbM3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)),   [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDVDbM4{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDVDbMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),

            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDbVDbM3x2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDbVDbM2x3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix1(input)),   [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDbVDbM1{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)),   [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDbVDbM2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)),   [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDbVDbM3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)),   [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDbVDbM4{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DRDbVDbMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            
            // --
              
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDbRDM3x2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDbRDM2x3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix1(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDbRDM1{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDbRDM2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDbRDM3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDbRDM4{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDbRDMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),

            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDRDbM3x2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDRDbM2x3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix1(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDRDbM1{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDRDbM2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDRDbM3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDRDbM4{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDRDbMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),

            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDbRDbM3x2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDbRDbM2x3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix1(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDbRDbM1{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDbRDbM2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDbRDbM3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDbRDbM4{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DVDbRDbMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
          
          )+
        )+
        // Check other sizes
        (lhs_value, ixes) => {
          match impl_access_range_scalar_fxn(lhs_value.clone(), ixes.to_vec()) {
            Ok(res) => Ok(res),
            Err(_) => impl_access_scalar_range_fxn(lhs_value, ixes.to_vec()),
          }
        }
      }
    }
  }
}

fn impl_access_range_range_fxn(lhs_value: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_access_match_arms!(Access2DRR, range_range, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessRangeRange {}
impl NativeFunctionCompiler for MatrixAccessRangeRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match impl_access_range_range_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { impl_access_range_range_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[:] -----------------------------------------------------------------------

macro_rules! impl_access_all_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)),  [Value::IndexAll]) => Ok(Box::new(Access1DAM3x2{source: input.clone(), ixes: new_ref(Value::IndexAll), out: new_ref(DVector::from_element(input.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)),  [Value::IndexAll]) => Ok(Box::new(Access1DAM2x3{source: input.clone(), ixes: new_ref(Value::IndexAll), out: new_ref(DVector::from_element(input.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)),    [Value::IndexAll]) => Ok(Box::new(Access1DAM2  {source: input.clone(), ixes: new_ref(Value::IndexAll), out: new_ref(DVector::from_element(input.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)),    [Value::IndexAll]) => Ok(Box::new(Access1DAM3  {source: input.clone(), ixes: new_ref(Value::IndexAll), out: new_ref(DVector::from_element(input.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)),    [Value::IndexAll]) => Ok(Box::new(Access1DAM4  {source: input.clone(), ixes: new_ref(Value::IndexAll), out: new_ref(DVector::from_element(input.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),    [Value::IndexAll]) => Ok(Box::new(Access1DAMD  {source: input.clone(), ixes: new_ref(Value::IndexAll), out: new_ref(DVector::from_element(input.borrow().len(),$default)) })),
          )+
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_access_all_fxn(lhs_value: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_access_match_arms!(Access1DA, all, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessAll {}
impl NativeFunctionCompiler for MatrixAccessAll {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match impl_access_all_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { impl_access_all_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[:,2] ---------------------------------------------------------------------

macro_rules! impl_access_all_scalar_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)),  [Value::IndexAll,Value::Index(ix)]) => Ok(Box::new(Access2DASM2x3{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(input.borrow().nrows(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)),  [Value::IndexAll,Value::Index(ix)]) => Ok(Box::new(Access2DASM3x2{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(input.borrow().nrows(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)),    [Value::IndexAll,Value::Index(ix)]) => Ok(Box::new(Access2DASM2  {source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(input.borrow().nrows(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)),    [Value::IndexAll,Value::Index(ix)]) => Ok(Box::new(Access2DASM3  {source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(input.borrow().nrows(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)),    [Value::IndexAll,Value::Index(ix)]) => Ok(Box::new(Access2DASM4  {source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(input.borrow().nrows(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),    [Value::IndexAll,Value::Index(ix)]) => Ok(Box::new(Access2DASMD  {source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(input.borrow().nrows(),$default)) })),
          )+
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_access_all_scalar_fxn(lhs_value: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_access_match_arms!(Access2DAS, all_scalar, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessAllScalar {}
impl NativeFunctionCompiler for MatrixAccessAllScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match impl_access_all_scalar_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { impl_access_all_scalar_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[2,:] ---------------------------------------------------------------------

macro_rules! impl_access_scalar_all_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::Index(ix),Value::IndexAll]) => Ok(Box::new(Access2DSAM2x3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(input.borrow().nrows(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::Index(ix),Value::IndexAll]) => Ok(Box::new(Access2DSAM3x2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(input.borrow().nrows(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix1(input)), [Value::Index(ix),Value::IndexAll]) => Ok(Box::new(Access2DSAM1{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(input.borrow().nrows(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::Index(ix),Value::IndexAll]) => Ok(Box::new(Access2DSAM2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(input.borrow().nrows(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::Index(ix),Value::IndexAll]) => Ok(Box::new(Access2DSAM3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(input.borrow().nrows(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::Index(ix),Value::IndexAll]) => Ok(Box::new(Access2DSAM4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(input.borrow().nrows(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::Index(ix),Value::IndexAll]) => Ok(Box::new(Access2DSAMD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(input.borrow().nrows(),$default)) })),
          )+
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_access_scalar_all_fxn(lhs_value: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_access_match_arms!(Access2DSA, scalar_all, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessScalarAll {}
impl NativeFunctionCompiler for MatrixAccessScalarAll {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match impl_access_scalar_all_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { impl_access_scalar_all_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[:,1..3] ---------------------------------------------------------------------

macro_rules! impl_access_all_range_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            // All Vector
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::IndexAll, Value::MatrixIndex(Matrix::DVector(ix))]) => Ok(Box::new(Access2DAVDM2{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(2,ix.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::IndexAll, Value::MatrixIndex(Matrix::DVector(ix))]) => Ok(Box::new(Access2DAVDM3{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(3,ix.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::IndexAll, Value::MatrixIndex(Matrix::DVector(ix))]) => Ok(Box::new(Access2DAVDM4{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(4,ix.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::IndexAll, Value::MatrixIndex(Matrix::DVector(ix))]) => Ok(Box::new(Access2DAVDMD{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(input.borrow().ncols(),ix.borrow().len(),$default)) })),
            // All Row Vector
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::IndexAll, Value::MatrixIndex(Matrix::RowDVector(ix))]) => Ok(Box::new(Access2DARDM2{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(2,ix.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::IndexAll, Value::MatrixIndex(Matrix::RowDVector(ix))]) => Ok(Box::new(Access2DARDM3{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(3,ix.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::IndexAll, Value::MatrixIndex(Matrix::RowDVector(ix))]) => Ok(Box::new(Access2DARDM4{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(4,ix.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::IndexAll, Value::MatrixIndex(Matrix::RowDVector(ix))]) => Ok(Box::new(Access2DARDMD{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(input.borrow().nrows(),ix.borrow().len(),$default)) })),
            // All Bool Vector
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::IndexAll, Value::MatrixBool(Matrix::DVector(ix))]) => Ok(Box::new(Access2DAVDbM2{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::IndexAll, Value::MatrixBool(Matrix::DVector(ix))]) => Ok(Box::new(Access2DAVDbM3{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::IndexAll, Value::MatrixBool(Matrix::DVector(ix))]) => Ok(Box::new(Access2DAVDbM4{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::IndexAll, Value::MatrixBool(Matrix::DVector(ix))]) => Ok(Box::new(Access2DAVDbMD{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) })),
            // All Bool Row
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::IndexAll, Value::MatrixBool(Matrix::RowDVector(ix))]) => Ok(Box::new(Access2DARDbM2{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::IndexAll, Value::MatrixBool(Matrix::RowDVector(ix))]) => Ok(Box::new(Access2DARDbM3{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::IndexAll, Value::MatrixBool(Matrix::RowDVector(ix))]) => Ok(Box::new(Access2DARDbM4{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::IndexAll, Value::MatrixBool(Matrix::RowDVector(ix))]) => Ok(Box::new(Access2DARDbMD{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) })),
          )+
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_access_all_range_fxn(lhs_value: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_access_match_arms!(Access2DAR, all_range, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessAllRange {}
impl NativeFunctionCompiler for MatrixAccessAllRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match impl_access_all_range_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { impl_access_all_range_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3,:] ---------------------------------------------------------------------

macro_rules! impl_access_range_all_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            // Vector All
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixIndex(Matrix::DVector(ix)), Value::IndexAll]) => Ok(Box::new(Access2DVDAM2{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixIndex(Matrix::DVector(ix)), Value::IndexAll]) => Ok(Box::new(Access2DVDAM3{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::MatrixIndex(Matrix::DVector(ix)), Value::IndexAll]) => Ok(Box::new(Access2DVDAM4{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixIndex(Matrix::DVector(ix)), Value::IndexAll]) => Ok(Box::new(Access2DVDAMD{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) })),
            // Row Vector All
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixIndex(Matrix::RowDVector(ix)), Value::IndexAll]) => Ok(Box::new(Access2DRDAM2{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixIndex(Matrix::RowDVector(ix)), Value::IndexAll]) => Ok(Box::new(Access2DRDAM3{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::MatrixIndex(Matrix::RowDVector(ix)), Value::IndexAll]) => Ok(Box::new(Access2DRDAM4{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixIndex(Matrix::RowDVector(ix)), Value::IndexAll]) => Ok(Box::new(Access2DRDAMD{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) })),
            // Bool Vector All
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixBool(Matrix::DVector(ix)), Value::IndexAll]) => Ok(Box::new(Access2DVDbAM2{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixBool(Matrix::DVector(ix)), Value::IndexAll]) => Ok(Box::new(Access2DVDbAM3{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::MatrixBool(Matrix::DVector(ix)), Value::IndexAll]) => Ok(Box::new(Access2DVDbAM4{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixBool(Matrix::DVector(ix)), Value::IndexAll]) => Ok(Box::new(Access2DVDbAMD{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) })),
            // Bool Row Vector All
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixBool(Matrix::RowDVector(ix)), Value::IndexAll]) => Ok(Box::new(Access2DRDbAM2{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixBool(Matrix::RowDVector(ix)), Value::IndexAll]) => Ok(Box::new(Access2DRDbAM3{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::MatrixBool(Matrix::RowDVector(ix)), Value::IndexAll]) => Ok(Box::new(Access2DRDbAM4{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixBool(Matrix::RowDVector(ix)), Value::IndexAll]) => Ok(Box::new(Access2DRDbAMD{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) })),
          )+
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_access_range_all_fxn(lhs_value: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_access_match_arms!(Access2DRA, range_all, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessRangeAll {}
impl NativeFunctionCompiler for MatrixAccessRangeAll {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match impl_access_range_all_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { impl_access_range_all_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3,2] ---------------------------------------------------------------------

macro_rules! impl_access_range_scalar_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            // Vector Scalar
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DVDSM2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DVector::from_element(ix1.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DVDSM3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DVector::from_element(ix1.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DVDSM4{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DVector::from_element(ix1.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DVDSM2x3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DVector::from_element(ix1.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DVDSM3x2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DVector::from_element(ix1.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DVDSMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DVector::from_element(ix1.borrow().len(),$default)) })),
            // Bool Vector Scalar
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DVDbSM2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DVector::from_element(ix1.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DVDbSM3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DVector::from_element(ix1.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DVDbSM4{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DVector::from_element(ix1.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DVDbSM2x3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DVector::from_element(ix1.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DVDbSM3x2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DVector::from_element(ix1.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DVDbSMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DVector::from_element(ix1.borrow().len(),$default)) })),
            // Row Scalar
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)),   [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DRDSM2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DVector::from_element(ix1.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)),   [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DRDSM3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DVector::from_element(ix1.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)),   [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DRDSM4{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DVector::from_element(ix1.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DRDSM2x3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DVector::from_element(ix1.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DRDSM3x2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DVector::from_element(ix1.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::MatrixIndex(Matrix::RowDVector(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DRDSMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DVector::from_element(ix1.borrow().len(),$default)) })),
            // Bool Row Scalar
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)),   [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DRDbSM2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DVector::from_element(ix1.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)),   [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DRDbSM3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DVector::from_element(ix1.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)),   [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DRDbSM4{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DVector::from_element(ix1.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DRDbSM2x3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DVector::from_element(ix1.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DRDbSM3x2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DVector::from_element(ix1.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::MatrixBool(Matrix::RowDVector(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DRDbSMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DVector::from_element(ix1.borrow().len(),$default)) })),
          )+
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_access_range_scalar_fxn(lhs_value: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_access_match_arms!(Access2DRS, range_scalar, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessRangeScalar {}
impl NativeFunctionCompiler for MatrixAccessRangeScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match impl_access_range_scalar_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { impl_access_range_scalar_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[2,1..3] ---------------------------------------------------------------------

macro_rules! impl_access_scalar_range_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            // Vector Scalar
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)),   [Value::Index(ix1), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DSVDM2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowDVector::from_element(ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)),   [Value::Index(ix1), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DSVDM3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowDVector::from_element(ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)),   [Value::Index(ix1), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DSVDM4{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowDVector::from_element(ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::Index(ix1), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DSVDM2x3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowDVector::from_element(ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::Index(ix1), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DSVDM3x2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowDVector::from_element(ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::Index(ix1), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DSVDMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowDVector::from_element(ix2.borrow().len(),$default)) })),
            // Bool Vector Scalar
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)),   [Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DSVDbM2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowDVector::from_element(ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)),   [Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DSVDbM3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowDVector::from_element(ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)),   [Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DSVDbM4{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowDVector::from_element(ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DSVDbM2x3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowDVector::from_element(ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DSVDbM3x2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowDVector::from_element(ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DSVDbMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowDVector::from_element(ix2.borrow().len(),$default)) })),
            // Row Scalar
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)),   [Value::Index(ix1), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DSRDM2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowDVector::from_element(ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)),   [Value::Index(ix1), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DSRDM3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowDVector::from_element(ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)),   [Value::Index(ix1), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DSRDM4{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowDVector::from_element(ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::Index(ix1), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DSRDM2x3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowDVector::from_element(ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::Index(ix1), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DSRDM3x2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowDVector::from_element(ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::Index(ix1), Value::MatrixIndex(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DSRDMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowDVector::from_element(ix2.borrow().len(),$default)) })),
            // Bool Row Scalar
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)),   [Value::Index(ix1), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DSRDbM2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowDVector::from_element(ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)),   [Value::Index(ix1), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DSRDbM3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowDVector::from_element(ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)),   [Value::Index(ix1), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DSRDbM4{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowDVector::from_element(ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::Index(ix1), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DSRDbM2x3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowDVector::from_element(ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::Index(ix1), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DSRDbM3x2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowDVector::from_element(ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::Index(ix1), Value::MatrixBool(Matrix::RowDVector(ix2))]) => Ok(Box::new(Access2DSRDbMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowDVector::from_element(ix2.borrow().len(),$default)) })),
          )+
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}
fn impl_access_scalar_range_fxn(lhs_value: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_access_match_arms!(Access2DSR, scalar_range, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessScalarRange {}
impl NativeFunctionCompiler for MatrixAccessScalarRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match impl_access_scalar_range_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { impl_access_scalar_range_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}