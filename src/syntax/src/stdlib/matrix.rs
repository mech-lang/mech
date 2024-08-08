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
macro_rules! access_1d_slice {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      for i in 0..(*$ix).len() {
        (*$out)[i] = (*$source).index((*$ix)[i]-1).clone();
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

macro_rules! access_2d_row_slice2 {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      let ix1 = (*$ix).0;
      let ix2 = (*$ix).1;
      let out_cols = ix2.ncols();
      let mut out_ix = 0;
      for c in 0..out_cols {
        (*$out)[out_ix] = (*$source).index((ix1 - 1,ix2[c] - 1)).clone();
        out_ix += 1;
      }
    }};}    

macro_rules! access_2d_col_slice2 {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      let ix1 = (*$ix).0;
      let ix2 = (*$ix).1;
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
impl_access_fxn_shape!(Access2DSS, (usize,usize), T, access_2d);

// x[1..3]
impl_access_fxn_shape!(Access1DV2, Vector2<usize>, Vector2<T>, access_1d_slice);
impl_access_fxn_shape!(Access1DV3, Vector3<usize>, Vector3<T>, access_1d_slice);
impl_access_fxn_shape!(Access1DV4, Vector4<usize>, Vector4<T>, access_1d_slice);
impl_access_fxn_shape!(Access1DVD, DVector<usize>, DVector<T>, access_1d_slice);

impl_access_fxn_shape!(Access1DR2, RowVector2<usize>, RowVector2<T>, access_1d_slice);
impl_access_fxn_shape!(Access1DR3, RowVector3<usize>, RowVector3<T>, access_1d_slice);
impl_access_fxn_shape!(Access1DR4, RowVector4<usize>, RowVector4<T>, access_1d_slice);
impl_access_fxn_shape!(Access1DRD, RowDVector<usize>, RowDVector<T>, access_1d_slice);

impl_access_fxn_shape!(Access1DR2b, RowVector2<bool>, RowDVector<T>, access_1d_slice_bool);
impl_access_fxn_shape!(Access1DR3b, RowVector3<bool>, RowDVector<T>, access_1d_slice_bool);
impl_access_fxn_shape!(Access1DR4b, RowVector4<bool>, RowDVector<T>, access_1d_slice_bool);
impl_access_fxn_shape!(Access1DRDb, RowDVector<bool>, RowDVector<T>, access_1d_slice_bool);

impl_access_fxn_shape!(Access1DV2b, Vector2<bool>, DVector<T>, access_1d_slice_bool_v);
impl_access_fxn_shape!(Access1DV3b, Vector3<bool>, DVector<T>, access_1d_slice_bool_v);
impl_access_fxn_shape!(Access1DV4b, Vector4<bool>, DVector<T>, access_1d_slice_bool_v);
impl_access_fxn_shape!(Access1DVDb, DVector<bool>, DVector<T>, access_1d_slice_bool_v);

// x[1..3,1..3]
impl_access_fxn_shape!(Access2DR2R2, (RowVector2<usize>,RowVector2<usize>), Matrix2<T>, access_2d_slice2);

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
impl_access_fxn_shape!(Access2DAR3, RowVector3<usize>, DMatrix<T>, access_2d_all_slice2);


// x[2,1..3]
impl_access_fxn_shape!(Access2DSR2, (usize, RowVector2<usize>), RowVector2<T>, access_2d_row_slice2);

// x[1..3,2]
impl_access_fxn_shape!(Access2DR2S, (RowVector2<usize>, usize), Vector2<T>, access_2d_col_slice2);

// x.x,y,z

macro_rules! generate_access_match_arms {
  ($fxn_name:ident,$macro_name:ident, $arg:expr) => {
    paste!{
      [<generate_access_ $macro_name _match_arms>]!(
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

macro_rules! generate_access_scalar_match_arms {
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

fn generate_access_scalar_fxn(lhs_value: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  generate_access_match_arms!(Access1DS, scalar, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessScalar {}
impl NativeFunctionCompiler for MatrixAccessScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match generate_access_scalar_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { generate_access_scalar_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1,2] ---------------------------------------------------------------------

macro_rules! generate_access_scalar_scalar_match_arms {
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

fn generate_access_scalar_scalar_fxn(lhs_value: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  generate_access_match_arms!(Access2DSS, scalar_scalar, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessScalarScalar {}
impl NativeFunctionCompiler for MatrixAccessScalarScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match generate_access_scalar_scalar_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { generate_access_scalar_scalar_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3] --------------------------------------------------------------------

macro_rules! generate_access_range_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), [Value::MatrixBool(Matrix::RowVector2(ix))])  => Ok(Box::new(Access1DR2bR2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), [Value::MatrixBool(Matrix::RowVector3(ix))])  => Ok(Box::new(Access1DR3bR2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), [Value::MatrixBool(Matrix::RowVector4(ix))])  => Ok(Box::new(Access1DR4bR2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbR2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), [Value::MatrixBool(Matrix::Vector2(ix))])  => Ok(Box::new(Access1DV2bR2{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), [Value::MatrixBool(Matrix::Vector3(ix))])  => Ok(Box::new(Access1DV3bR2{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), [Value::MatrixBool(Matrix::Vector4(ix))])  => Ok(Box::new(Access1DV4bR2{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDbR2{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    

            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::MatrixBool(Matrix::RowVector2(ix))])  => Ok(Box::new(Access1DR2bR3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::MatrixBool(Matrix::RowVector3(ix))])  => Ok(Box::new(Access1DR3bR3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::MatrixBool(Matrix::RowVector4(ix))])  => Ok(Box::new(Access1DR4bR3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbR3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::MatrixBool(Matrix::Vector2(ix))])  => Ok(Box::new(Access1DV2bR3{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::MatrixBool(Matrix::Vector3(ix))])  => Ok(Box::new(Access1DV3bR3{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::MatrixBool(Matrix::Vector4(ix))])  => Ok(Box::new(Access1DV4bR3{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDbR3{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    

            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), [Value::MatrixBool(Matrix::RowVector2(ix))])  => Ok(Box::new(Access1DR2bR4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), [Value::MatrixBool(Matrix::RowVector3(ix))])  => Ok(Box::new(Access1DR3bR4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), [Value::MatrixBool(Matrix::RowVector4(ix))])  => Ok(Box::new(Access1DR4bR4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbR4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), [Value::MatrixBool(Matrix::Vector2(ix))])  => Ok(Box::new(Access1DV2bR4{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), [Value::MatrixBool(Matrix::Vector3(ix))])  => Ok(Box::new(Access1DV3bR4{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), [Value::MatrixBool(Matrix::Vector4(ix))])  => Ok(Box::new(Access1DV4bR4{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDbR4{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::MatrixBool(Matrix::RowVector2(ix))])  => Ok(Box::new(Access1DR2bRD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::MatrixBool(Matrix::RowVector3(ix))])  => Ok(Box::new(Access1DR3bRD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::MatrixBool(Matrix::RowVector4(ix))])  => Ok(Box::new(Access1DR4bRD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbRD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::MatrixBool(Matrix::Vector2(ix))])  => Ok(Box::new(Access1DV2bRD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::MatrixBool(Matrix::Vector3(ix))])  => Ok(Box::new(Access1DV3bRD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::MatrixBool(Matrix::Vector4(ix))])  => Ok(Box::new(Access1DV4bRD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDbRD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),   

            // --

            (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)), [Value::MatrixBool(Matrix::RowVector2(ix))])  => Ok(Box::new(Access1DR2bV2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)), [Value::MatrixBool(Matrix::RowVector3(ix))])  => Ok(Box::new(Access1DR3bV2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)), [Value::MatrixBool(Matrix::RowVector4(ix))])  => Ok(Box::new(Access1DR4bV2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbV2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)), [Value::MatrixBool(Matrix::Vector2(ix))])  => Ok(Box::new(Access1DV2bV2{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)), [Value::MatrixBool(Matrix::Vector3(ix))])  => Ok(Box::new(Access1DV3bV2{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)), [Value::MatrixBool(Matrix::Vector4(ix))])  => Ok(Box::new(Access1DV4bV2{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDbV2{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    

            (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)), [Value::MatrixBool(Matrix::RowVector2(ix))])  => Ok(Box::new(Access1DR2bV3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)), [Value::MatrixBool(Matrix::RowVector3(ix))])  => Ok(Box::new(Access1DR3bV3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)), [Value::MatrixBool(Matrix::RowVector4(ix))])  => Ok(Box::new(Access1DR4bV3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbV3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)), [Value::MatrixBool(Matrix::Vector2(ix))])  => Ok(Box::new(Access1DV2bV3{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)), [Value::MatrixBool(Matrix::Vector3(ix))])  => Ok(Box::new(Access1DV3bV3{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)), [Value::MatrixBool(Matrix::Vector4(ix))])  => Ok(Box::new(Access1DV4bV3{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDbV3{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    

            (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)), [Value::MatrixBool(Matrix::RowVector2(ix))])  => Ok(Box::new(Access1DR2bV4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)), [Value::MatrixBool(Matrix::RowVector3(ix))])  => Ok(Box::new(Access1DR3bV4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)), [Value::MatrixBool(Matrix::RowVector4(ix))])  => Ok(Box::new(Access1DR4bV4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbV4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)), [Value::MatrixBool(Matrix::Vector2(ix))])  => Ok(Box::new(Access1DV2bV4{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)), [Value::MatrixBool(Matrix::Vector3(ix))])  => Ok(Box::new(Access1DV3bV4{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)), [Value::MatrixBool(Matrix::Vector4(ix))])  => Ok(Box::new(Access1DV4bV4{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDbV4{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)), [Value::MatrixBool(Matrix::RowVector2(ix))])  => Ok(Box::new(Access1DR2bVD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)), [Value::MatrixBool(Matrix::RowVector3(ix))])  => Ok(Box::new(Access1DR3bVD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)), [Value::MatrixBool(Matrix::RowVector4(ix))])  => Ok(Box::new(Access1DR4bVD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbVD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)), [Value::MatrixBool(Matrix::Vector2(ix))])  => Ok(Box::new(Access1DV2bVD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)), [Value::MatrixBool(Matrix::Vector3(ix))])  => Ok(Box::new(Access1DV3bVD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)), [Value::MatrixBool(Matrix::Vector4(ix))])  => Ok(Box::new(Access1DV4bVD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDbVD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),   

            // --

            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixBool(Matrix::RowVector2(ix))])  => Ok(Box::new(Access1DR2bM2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixBool(Matrix::RowVector3(ix))])  => Ok(Box::new(Access1DR3bM2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixBool(Matrix::RowVector4(ix))])  => Ok(Box::new(Access1DR4bM2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbM2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixBool(Matrix::Vector2(ix))])  => Ok(Box::new(Access1DV2bM2{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixBool(Matrix::Vector3(ix))])  => Ok(Box::new(Access1DV3bM2{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixBool(Matrix::Vector4(ix))])  => Ok(Box::new(Access1DV4bM2{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDbM2{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    

            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixBool(Matrix::RowVector2(ix))])  => Ok(Box::new(Access1DR2bM3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixBool(Matrix::RowVector3(ix))])  => Ok(Box::new(Access1DR3bM3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixBool(Matrix::RowVector4(ix))])  => Ok(Box::new(Access1DR4bM3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbM3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixBool(Matrix::Vector2(ix))])  => Ok(Box::new(Access1DV2bM3{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixBool(Matrix::Vector3(ix))])  => Ok(Box::new(Access1DV3bM3{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixBool(Matrix::Vector4(ix))])  => Ok(Box::new(Access1DV4bM3{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDbM3{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    

            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::MatrixBool(Matrix::RowVector2(ix))])  => Ok(Box::new(Access1DR2bM4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::MatrixBool(Matrix::RowVector3(ix))])  => Ok(Box::new(Access1DR3bM4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::MatrixBool(Matrix::RowVector4(ix))])  => Ok(Box::new(Access1DR4bM4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbM4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::MatrixBool(Matrix::Vector2(ix))])  => Ok(Box::new(Access1DV2bM4{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::MatrixBool(Matrix::Vector3(ix))])  => Ok(Box::new(Access1DV3bM4{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::MatrixBool(Matrix::Vector4(ix))])  => Ok(Box::new(Access1DV4bM4{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDbM4{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixBool(Matrix::RowVector2(ix))])  => Ok(Box::new(Access1DR2bMD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixBool(Matrix::RowVector3(ix))])  => Ok(Box::new(Access1DR3bMD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixBool(Matrix::RowVector4(ix))])  => Ok(Box::new(Access1DR4bMD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixBool(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDbMD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixBool(Matrix::Vector2(ix))])  => Ok(Box::new(Access1DV2bMD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixBool(Matrix::Vector3(ix))])  => Ok(Box::new(Access1DV3bMD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixBool(Matrix::Vector4(ix))])  => Ok(Box::new(Access1DV4bMD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDbMD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),   

            // --

            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), [Value::MatrixIndex(Matrix::RowVector2(ix))])  => Ok(Box::new(Access1DR2R2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector2::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), [Value::MatrixIndex(Matrix::RowVector3(ix))])  => Ok(Box::new(Access1DR3R2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector3::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), [Value::MatrixIndex(Matrix::RowVector4(ix))])  => Ok(Box::new(Access1DR4R2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector4::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDR2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), [Value::MatrixIndex(Matrix::Vector2(ix))])  => Ok(Box::new(Access1DV2R2{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector2::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), [Value::MatrixIndex(Matrix::Vector3(ix))])  => Ok(Box::new(Access1DV3R2{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector3::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), [Value::MatrixIndex(Matrix::Vector4(ix))])  => Ok(Box::new(Access1DV4R2{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector4::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDR2{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    

            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::MatrixIndex(Matrix::RowVector2(ix))])  => Ok(Box::new(Access1DR2R3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector2::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::MatrixIndex(Matrix::RowVector3(ix))])  => Ok(Box::new(Access1DR3R3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector3::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::MatrixIndex(Matrix::RowVector4(ix))])  => Ok(Box::new(Access1DR4R3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector4::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDR3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::MatrixIndex(Matrix::Vector2(ix))])  => Ok(Box::new(Access1DV2R3{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector2::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::MatrixIndex(Matrix::Vector3(ix))])  => Ok(Box::new(Access1DV3R3{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector3::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::MatrixIndex(Matrix::Vector4(ix))])  => Ok(Box::new(Access1DV4R3{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector4::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDR3{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    

            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), [Value::MatrixIndex(Matrix::RowVector2(ix))])  => Ok(Box::new(Access1DR2R4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector2::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), [Value::MatrixIndex(Matrix::RowVector3(ix))])  => Ok(Box::new(Access1DR3R4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector3::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), [Value::MatrixIndex(Matrix::RowVector4(ix))])  => Ok(Box::new(Access1DR4R4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector4::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDR4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), [Value::MatrixIndex(Matrix::Vector2(ix))])  => Ok(Box::new(Access1DV2R4{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector2::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), [Value::MatrixIndex(Matrix::Vector3(ix))])  => Ok(Box::new(Access1DV3R4{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector3::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), [Value::MatrixIndex(Matrix::Vector4(ix))])  => Ok(Box::new(Access1DV4R4{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector4::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDR4{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::MatrixIndex(Matrix::RowVector2(ix))])  => Ok(Box::new(Access1DR2RD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector2::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::MatrixIndex(Matrix::RowVector3(ix))])  => Ok(Box::new(Access1DR3RD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector3::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::MatrixIndex(Matrix::RowVector4(ix))])  => Ok(Box::new(Access1DR4RD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector4::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDRD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::MatrixIndex(Matrix::Vector2(ix))])  => Ok(Box::new(Access1DV2RD{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector2::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::MatrixIndex(Matrix::Vector3(ix))])  => Ok(Box::new(Access1DV3RD{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector3::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::MatrixIndex(Matrix::Vector4(ix))])  => Ok(Box::new(Access1DV4RD{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector4::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDRD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),   

            // --

            (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)), [Value::MatrixIndex(Matrix::RowVector2(ix))])  => Ok(Box::new(Access1DR2V2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector2::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)), [Value::MatrixIndex(Matrix::RowVector3(ix))])  => Ok(Box::new(Access1DR3V2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector3::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)), [Value::MatrixIndex(Matrix::RowVector4(ix))])  => Ok(Box::new(Access1DR4V2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector4::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDV2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)), [Value::MatrixIndex(Matrix::Vector2(ix))])  => Ok(Box::new(Access1DV2V2{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector2::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)), [Value::MatrixIndex(Matrix::Vector3(ix))])  => Ok(Box::new(Access1DV3V2{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector3::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)), [Value::MatrixIndex(Matrix::Vector4(ix))])  => Ok(Box::new(Access1DV4V2{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector4::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDV2{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    

            (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)), [Value::MatrixIndex(Matrix::RowVector2(ix))])  => Ok(Box::new(Access1DR2V3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector2::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)), [Value::MatrixIndex(Matrix::RowVector3(ix))])  => Ok(Box::new(Access1DR3V3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector3::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)), [Value::MatrixIndex(Matrix::RowVector4(ix))])  => Ok(Box::new(Access1DR4V3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector4::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDV3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)), [Value::MatrixIndex(Matrix::Vector2(ix))])  => Ok(Box::new(Access1DV2V3{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector2::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)), [Value::MatrixIndex(Matrix::Vector3(ix))])  => Ok(Box::new(Access1DV3V3{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector3::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)), [Value::MatrixIndex(Matrix::Vector4(ix))])  => Ok(Box::new(Access1DV4V3{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector4::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDV3{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    

            (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)), [Value::MatrixIndex(Matrix::RowVector2(ix))])  => Ok(Box::new(Access1DR2V4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector2::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)), [Value::MatrixIndex(Matrix::RowVector3(ix))])  => Ok(Box::new(Access1DR3V4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector3::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)), [Value::MatrixIndex(Matrix::RowVector4(ix))])  => Ok(Box::new(Access1DR4V4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector4::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDV4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)), [Value::MatrixIndex(Matrix::Vector2(ix))])  => Ok(Box::new(Access1DV2V4{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector2::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)), [Value::MatrixIndex(Matrix::Vector3(ix))])  => Ok(Box::new(Access1DV3V4{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector3::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)), [Value::MatrixIndex(Matrix::Vector4(ix))])  => Ok(Box::new(Access1DV4V4{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector4::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDV4{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)), [Value::MatrixIndex(Matrix::RowVector2(ix))])  => Ok(Box::new(Access1DR2VD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector2::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)), [Value::MatrixIndex(Matrix::RowVector3(ix))])  => Ok(Box::new(Access1DR3VD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector3::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)), [Value::MatrixIndex(Matrix::RowVector4(ix))])  => Ok(Box::new(Access1DR4VD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector4::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDVD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)), [Value::MatrixIndex(Matrix::Vector2(ix))])  => Ok(Box::new(Access1DV2VD{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector2::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)), [Value::MatrixIndex(Matrix::Vector3(ix))])  => Ok(Box::new(Access1DV3VD{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector3::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)), [Value::MatrixIndex(Matrix::Vector4(ix))])  => Ok(Box::new(Access1DV4VD{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector4::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDVD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),   

            // --

            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixIndex(Matrix::RowVector2(ix))])  => Ok(Box::new(Access1DR2M2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector2::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixIndex(Matrix::RowVector3(ix))])  => Ok(Box::new(Access1DR3M2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector3::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixIndex(Matrix::RowVector4(ix))])  => Ok(Box::new(Access1DR4M2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector4::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDM2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixIndex(Matrix::Vector2(ix))])  => Ok(Box::new(Access1DV2M2{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector2::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixIndex(Matrix::Vector3(ix))])  => Ok(Box::new(Access1DV3M2{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector3::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixIndex(Matrix::Vector4(ix))])  => Ok(Box::new(Access1DV4M2{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector4::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDM2{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    

            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixIndex(Matrix::RowVector2(ix))])  => Ok(Box::new(Access1DR2M3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector2::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixIndex(Matrix::RowVector3(ix))])  => Ok(Box::new(Access1DR3M3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector3::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixIndex(Matrix::RowVector4(ix))])  => Ok(Box::new(Access1DR4M3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector4::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDM3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixIndex(Matrix::Vector2(ix))])  => Ok(Box::new(Access1DV2M3{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector2::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixIndex(Matrix::Vector3(ix))])  => Ok(Box::new(Access1DV3M3{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector3::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixIndex(Matrix::Vector4(ix))])  => Ok(Box::new(Access1DV4M3{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector4::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDM3{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    

            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::MatrixIndex(Matrix::RowVector2(ix))])  => Ok(Box::new(Access1DR2M4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector2::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::MatrixIndex(Matrix::RowVector3(ix))])  => Ok(Box::new(Access1DR3M4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector3::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::MatrixIndex(Matrix::RowVector4(ix))])  => Ok(Box::new(Access1DR4M4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector4::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDM4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::MatrixIndex(Matrix::Vector2(ix))])  => Ok(Box::new(Access1DV2M4{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector2::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::MatrixIndex(Matrix::Vector3(ix))])  => Ok(Box::new(Access1DV3M4{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector3::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::MatrixIndex(Matrix::Vector4(ix))])  => Ok(Box::new(Access1DV4M4{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector4::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDM4{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixIndex(Matrix::RowVector2(ix))])  => Ok(Box::new(Access1DR2MD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector2::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixIndex(Matrix::RowVector3(ix))])  => Ok(Box::new(Access1DR3MD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector3::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixIndex(Matrix::RowVector4(ix))])  => Ok(Box::new(Access1DR4MD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowVector4::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixIndex(Matrix::RowDVector(ix))])  => Ok(Box::new(Access1DRDMD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(ix.borrow().len(),$default)) })),    
            
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixIndex(Matrix::Vector2(ix))])  => Ok(Box::new(Access1DV2MD{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector2::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixIndex(Matrix::Vector3(ix))])  => Ok(Box::new(Access1DV3MD{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector3::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixIndex(Matrix::Vector4(ix))])  => Ok(Box::new(Access1DV4MD{source: input.clone(), ixes: ix.clone(), out: new_ref(Vector4::from_element($default)) })),    
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => Ok(Box::new(Access1DVDMD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),   

          )+
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn generate_access_range_fxn(lhs_value: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  generate_access_match_arms!(Access1DR, range, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessRange {}
impl NativeFunctionCompiler for MatrixAccessRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match generate_access_range_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { generate_access_range_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3,1..3] ---------------------------------------------------------------

macro_rules! generate_access_range_range_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix1(input)), [Value::MatrixIndex(Matrix::RowVector2(ix1)), Value::MatrixIndex(Matrix::RowVector2(ix2))]) => Ok(Box::new(Access2DR2R2M1{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(Matrix2::from_element($default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::MatrixIndex(Matrix::RowVector2(ix1)), Value::MatrixIndex(Matrix::RowVector2(ix2))]) => Ok(Box::new(Access2DR2R2M2{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(Matrix2::from_element($default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixIndex(Matrix::RowVector2(ix1)), Value::MatrixIndex(Matrix::RowVector2(ix2))]) => Ok(Box::new(Access2DR2R2M3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(Matrix2::from_element($default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixIndex(Matrix::RowVector2(ix1)), Value::MatrixIndex(Matrix::RowVector2(ix2))]) => Ok(Box::new(Access2DR2R2MD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(Matrix2::from_element($default)) })),
          )+
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn generate_access_range_range_fxn(lhs_value: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  generate_access_match_arms!(Access2DRR, range_range, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessRangeRange {}
impl NativeFunctionCompiler for MatrixAccessRangeRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match generate_access_range_range_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { generate_access_range_range_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[:] -----------------------------------------------------------------------

macro_rules! generate_access_all_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), [Value::IndexAll]) => Ok(Box::new(Access1DAR2  {source: input.clone(), ixes: new_ref(Value::IndexAll), out: new_ref(DVector::from_element(input.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::IndexAll]) => Ok(Box::new(Access1DAR3  {source: input.clone(), ixes: new_ref(Value::IndexAll), out: new_ref(DVector::from_element(input.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), [Value::IndexAll]) => Ok(Box::new(Access1DAR4  {source: input.clone(), ixes: new_ref(Value::IndexAll), out: new_ref(DVector::from_element(input.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::IndexAll]) => Ok(Box::new(Access1DARD  {source: input.clone(), ixes: new_ref(Value::IndexAll), out: new_ref(DVector::from_element(input.borrow().len(),$default)) })),

            (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)),    [Value::IndexAll]) => Ok(Box::new(Access1DAV2  {source: input.clone(), ixes: new_ref(Value::IndexAll), out: new_ref(DVector::from_element(input.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)),    [Value::IndexAll]) => Ok(Box::new(Access1DAV3  {source: input.clone(), ixes: new_ref(Value::IndexAll), out: new_ref(DVector::from_element(input.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)),    [Value::IndexAll]) => Ok(Box::new(Access1DAV4  {source: input.clone(), ixes: new_ref(Value::IndexAll), out: new_ref(DVector::from_element(input.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)),    [Value::IndexAll]) => Ok(Box::new(Access1DAVD  {source: input.clone(), ixes: new_ref(Value::IndexAll), out: new_ref(DVector::from_element(input.borrow().len(),$default)) })),

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

fn generate_access_all_fxn(lhs_value: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  generate_access_match_arms!(Access1DA, all, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessAll {}
impl NativeFunctionCompiler for MatrixAccessAll {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match generate_access_all_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { generate_access_all_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[:,2] ---------------------------------------------------------------------

macro_rules! generate_access_all_scalar_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), [Value::IndexAll,Value::Index(ix)]) => Ok(Box::new(Access2DASR2  {source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(input.borrow().nrows(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::IndexAll,Value::Index(ix)]) => Ok(Box::new(Access2DASR3  {source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(input.borrow().nrows(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), [Value::IndexAll,Value::Index(ix)]) => Ok(Box::new(Access2DASR4  {source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(input.borrow().nrows(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::IndexAll,Value::Index(ix)]) => Ok(Box::new(Access2DASRD  {source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(input.borrow().nrows(),$default)) })),

            (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)),    [Value::IndexAll,Value::Index(ix)]) => Ok(Box::new(Access2DASV2  {source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(input.borrow().nrows(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)),    [Value::IndexAll,Value::Index(ix)]) => Ok(Box::new(Access2DASV3  {source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(input.borrow().nrows(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)),    [Value::IndexAll,Value::Index(ix)]) => Ok(Box::new(Access2DASV4  {source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(input.borrow().nrows(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)),    [Value::IndexAll,Value::Index(ix)]) => Ok(Box::new(Access2DASVD  {source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(input.borrow().nrows(),$default)) })),

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

fn generate_access_all_scalar_fxn(lhs_value: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  generate_access_match_arms!(Access2DAS, all_scalar, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessAllScalar {}
impl NativeFunctionCompiler for MatrixAccessAllScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match generate_access_all_scalar_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { generate_access_all_scalar_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[2,:] ---------------------------------------------------------------------

macro_rules! generate_access_scalar_all_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(input)), [Value::Index(ix),Value::IndexAll]) => Ok(Box::new(Access2DSAR2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(input.borrow().nrows(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(input)), [Value::Index(ix),Value::IndexAll]) => Ok(Box::new(Access2DSAR3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(input.borrow().nrows(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(input)), [Value::Index(ix),Value::IndexAll]) => Ok(Box::new(Access2DSAR4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(input.borrow().nrows(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::Index(ix),Value::IndexAll]) => Ok(Box::new(Access2DSARD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(input.borrow().nrows(),$default)) })),

            (Value::$matrix_kind(Matrix::<$target_type>::Vector2(input)), [Value::Index(ix),Value::IndexAll]) => Ok(Box::new(Access2DSAV2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(input.borrow().nrows(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Vector3(input)), [Value::Index(ix),Value::IndexAll]) => Ok(Box::new(Access2DSAV3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(input.borrow().nrows(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Vector4(input)), [Value::Index(ix),Value::IndexAll]) => Ok(Box::new(Access2DSAV4{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(input.borrow().nrows(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)), [Value::Index(ix),Value::IndexAll]) => Ok(Box::new(Access2DSAVD{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(input.borrow().nrows(),$default)) })),

            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(input)), [Value::Index(ix),Value::IndexAll]) => Ok(Box::new(Access2DSAM2x3{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(input.borrow().nrows(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(input)), [Value::Index(ix),Value::IndexAll]) => Ok(Box::new(Access2DSAM3x2{source: input.clone(), ixes: ix.clone(), out: new_ref(RowDVector::from_element(input.borrow().nrows(),$default)) })),
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

fn generate_access_scalar_all_fxn(lhs_value: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  generate_access_match_arms!(Access2DSA, scalar_all, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessScalarAll {}
impl NativeFunctionCompiler for MatrixAccessScalarAll {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match generate_access_scalar_all_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { generate_access_scalar_all_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[:,1..3] ---------------------------------------------------------------------

macro_rules! generate_access_all_range_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(input)), [Value::IndexAll, Value::MatrixIndex(Matrix::RowVector2(ix))]) => Ok(Box::new(Access2DAR2M2{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(2,2,$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::IndexAll, Value::MatrixIndex(Matrix::RowVector2(ix))]) => Ok(Box::new(Access2DAR2M3{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(3,2,$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix4(input)), [Value::IndexAll, Value::MatrixIndex(Matrix::RowVector2(ix))]) => Ok(Box::new(Access2DAR2M4{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(4,2,$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::IndexAll, Value::MatrixIndex(Matrix::RowVector2(ix))]) => Ok(Box::new(Access2DAR2MD{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(input.borrow().nrows(),2,$default)) })),
          )+
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn generate_access_all_range_fxn(lhs_value: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  generate_access_match_arms!(Access2DAR, all_range, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessAllRange {}
impl NativeFunctionCompiler for MatrixAccessAllRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match generate_access_all_range_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { generate_access_all_range_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3,:] ---------------------------------------------------------------------

macro_rules! generate_access_range_all_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixIndex(Matrix::RowVector2(ix)), Value::IndexAll]) => Ok(Box::new(Access2DR2AM3{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(2,3,$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixIndex(Matrix::RowVector2(ix)), Value::IndexAll]) => Ok(Box::new(Access2DR2AMD{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(2,input.borrow().ncols(),$default)) })),
          )+
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn generate_access_range_all_fxn(lhs_value: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  generate_access_match_arms!(Access2DRA, range_all, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessRangeAll {}
impl NativeFunctionCompiler for MatrixAccessRangeAll {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match generate_access_range_all_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { generate_access_range_all_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3,2] ---------------------------------------------------------------------

macro_rules! generate_access_range_scalar_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::MatrixIndex(Matrix::RowVector2(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DR2SM3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(Vector2::from_element($default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixIndex(Matrix::RowVector2(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DR2SMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(Vector2::from_element($default)) })),
          )+
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn generate_access_range_scalar_fxn(lhs_value: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  generate_access_match_arms!(Access2DRS, range_scalar, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessRangeScalar {}
impl NativeFunctionCompiler for MatrixAccessRangeScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match generate_access_range_scalar_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { generate_access_range_scalar_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[2,1..3] ---------------------------------------------------------------------

macro_rules! generate_access_scalar_range_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(input)), [Value::Index(ix1), Value::MatrixIndex(Matrix::RowVector2(ix2))]) => Ok(Box::new(Access2DSR2M3{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowVector2::from_element($default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::Index(ix1), Value::MatrixIndex(Matrix::RowVector2(ix2))]) => Ok(Box::new(Access2DSR2MD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowVector2::from_element($default)) })),
          )+
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn generate_access_scalar_range_fxn(lhs_value: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  generate_access_match_arms!(Access2DSR, scalar_range, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessScalarRange {}
impl NativeFunctionCompiler for MatrixAccessScalarRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match generate_access_scalar_range_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { generate_access_scalar_range_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}