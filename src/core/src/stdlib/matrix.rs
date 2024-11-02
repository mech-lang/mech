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
impl_binop!(MatMulRDVD, RowDVector<T>, DVector<T>, DMatrix<T>,matmul_op);
impl_binop!(MatMulVDRD, DVector<T>,RowDVector<T>,DMatrix<T>,matmul_op);
impl_binop!(MatMulMDMD, DMatrix<T>,DMatrix<T>,DMatrix<T>,matmul_op);

macro_rules! impl_matmul_match_arms {
  ($arg:expr, $($lhs_type:ident, $rhs_type:ident => $($matrix_kind:ident, $target_type:ident),+);+ $(;)?) => {
    match $arg {
      $(
        $(
          (Value::$lhs_type(lhs), Value::$rhs_type(rhs)) => Ok(Box::new(MatMulScalar { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref($target_type::zero()) })),
          (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(lhs)), Value::$matrix_kind(Matrix::<$target_type>::DVector(rhs))) => Ok(Box::new(MatMulRDVD{lhs, rhs, out: new_ref(DMatrix::from_element(1,1,$target_type::zero()))})),
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

impl_bool_urop!(TransposeRD, RowDVector<T>, DVector<T>, transpose_op);
impl_bool_urop!(TransposeVD, DVector<T>, RowDVector<T>, transpose_op);
impl_bool_urop!(TransposeMD, DMatrix<T>, DMatrix<T>, transpose_op);

macro_rules! impl_transpose_match_arms {
  ($arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    match $arg {
      $(
        $(
          Value::$matrix_kind(Matrix::<$target_type>::DVector(arg))    => Ok(Box::new(TransposeVD{arg: arg.clone(), out: new_ref(RowDVector::from_element(arg.borrow().len(),$default))})),
          Value::$matrix_kind(Matrix::<$target_type>::RowDVector(arg)) => Ok(Box::new(TransposeRD{arg: arg.clone(), out: new_ref(DVector::from_element(arg.borrow().len(),$default))})),
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

macro_rules! access_2d_slice_all {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      let n_cols = (*$source).ncols();
      let n_rows = (*$ix).nrows();
      let mut out_ix = 0;
      for c in 0..n_cols {
        for r in 0..n_rows {
          (*$out)[out_ix] = (*$source).index(((*$ix)[r] - 1, c)).clone();
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
      let n_cols = (*$ix).nrows();
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
      let out_cols = ix2.nrows();
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
      let out_rows = ix1.nrows();
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
impl_access_fxn_shape!(Access1DVDb, DVector<bool>, DVector<T>, access_1d_slice_bool_v);

// x[1..3,1..3]
impl_access_fxn_shape!(Access2DVDVD, (DVector<usize>,DVector<usize>), DMatrix<T>, access_2d_slice);
impl_access_fxn_shape!(Access2DVDbVD, (DVector<bool>,DVector<usize>), DMatrix<T>, access_2d_slice_bool);
impl_access_fxn_shape!(Access2DVDVDb, (DVector<usize>,DVector<bool>), DMatrix<T>, access_2d_slice_bool2);
impl_access_fxn_shape!(Access2DVDbVDb, (DVector<bool>,DVector<bool>), DMatrix<T>, access_2d_slice_bool_bool);

// x[:]
impl_access_fxn_shape!(Access1DA, Value, DVector<T>, access_1d_all);

// x[:,1]
impl_access_fxn_shape!(Access2DAS, usize, DVector<T>, access_col);

// x[1,:]
impl_access_fxn_shape!(Access2DSA, usize, RowDVector<T>, access_row);

// x[1..3,:]
impl_access_fxn_shape!(Access2DVDA, DVector<usize>,    DMatrix<T>, access_2d_slice_all);
impl_access_fxn_shape!(Access2DVDbA, DVector<bool>,    DMatrix<T>, access_2d_slice_all_bool);

// x[:,1..3]
impl_access_fxn_shape!(Access2DAVD, DVector<usize>,    DMatrix<T>, access_2d_all_slice);
impl_access_fxn_shape!(Access2DAVDb, DVector<bool>,    DMatrix<T>, access_2d_all_slice_bool);

// x[2,1..3]
impl_access_fxn_shape!(Access2DSVD,  (usize, DVector<usize>),    RowDVector<T>, access_2d_row_slice);
impl_access_fxn_shape!(Access2DSVDb, (usize, DVector<bool>),     RowDVector<T>, access_2d_row_slice_bool);

// x[1..3,2]
impl_access_fxn_shape!(Access2DVDS,  (DVector<usize>, usize),    DVector<T>, access_2d_col_slice);
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

// x[1,2] ---------------------------------------------------------------------

macro_rules! impl_access_scalar_scalar_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
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
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::MatrixBool(Matrix::DVector(ix))]) => Ok(Box::new(Access1DVDbRD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),   
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)),    [Value::MatrixBool(Matrix::DVector(ix))]) => Ok(Box::new(Access1DVDbVD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),   
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),    [Value::MatrixBool(Matrix::DVector(ix))]) => Ok(Box::new(Access1DVDbMD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),   
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(input)), [Value::MatrixIndex(Matrix::DVector(ix))]) => Ok(Box::new(Access1DVDRD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),   
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(input)),    [Value::MatrixIndex(Matrix::DVector(ix))]) => Ok(Box::new(Access1DVDVD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),   
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),    [Value::MatrixIndex(Matrix::DVector(ix))]) => Ok(Box::new(Access1DVDMD{source: input.clone(), ixes: ix.clone(), out: new_ref(DVector::from_element(ix.borrow().len(),$default)) })),   
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
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDVDMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDbVDMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDVDbMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DVDbVDbMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DMatrix::from_element(ix1.borrow().len(),ix2.borrow().len(),$default)) })),
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
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::IndexAll, Value::MatrixIndex(Matrix::DVector(ix))]) => Ok(Box::new(Access2DAVDMD{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(input.borrow().nrows(), ix.borrow().len(),$default)) })),
            // All Bool Vector
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::IndexAll, Value::MatrixBool(Matrix::DVector(ix))]) => Ok(Box::new(Access2DAVDbMD{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(input.borrow().nrows(), ix.borrow().len(),$default)) })),
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
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixIndex(Matrix::DVector(ix)), Value::IndexAll]) => Ok(Box::new(Access2DVDAMD{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) })),
            // Bool Vector All
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)), [Value::MatrixBool(Matrix::DVector(ix)), Value::IndexAll]) => Ok(Box::new(Access2DVDbAMD{source: input.clone(), ixes: ix.clone(), out: new_ref(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) })),
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
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DVDSMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DVector::from_element(ix1.borrow().len(),$default)) })),
            // Bool Vector Scalar
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)]) => Ok(Box::new(Access2DVDbSMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(DVector::from_element(ix1.borrow().len(),$default)) })),
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
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::Index(ix1), Value::MatrixIndex(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DSVDMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowDVector::from_element(ix2.borrow().len(),$default)) })),
            // Bool Vector Scalar
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(input)),   [Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))]) => Ok(Box::new(Access2DSVDbMD{source: input.clone(), ixes: new_ref((ix1.borrow().clone(),ix2.borrow().clone())), out: new_ref(RowDVector::from_element(ix2.borrow().len(),$default)) })),
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

// Set -----------------------------------------------------------------

macro_rules! impl_set_match_arms {
  ($fxn_name:ident,$macro_name:ident, $arg:expr) => {
    paste!{
      [<impl_set_ $macro_name _match_arms>]!(
        $fxn_name,
        $arg,
        Bool;
        U8;
        U16;
        U32;
        U64;
        U128;
        I8;
        I16;
        I32;
        I64;
        U128;
        F32;  
        F64; 
      )
    }
  }
}

// x[1] := 1 ------------------------------------------------------------------

macro_rules! impl_set_scalar_fxn {
  ($struct_name:ident, $matrix_shape:ident) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<T>,
      ixes: Ref<usize>,
      sink: Ref<$matrix_shape<T>>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$matrix_shape<T>>: ToValue
    {
      fn solve(&self) {
        let sink_ptr = self.sink.as_ptr();
        let ixes_ptr = self.ixes.as_ptr();
        let source_ptr = self.source.as_ptr();
        unsafe {
          (*sink_ptr)[*ixes_ptr - 1] = (*source_ptr).clone();
        }
      }
      fn out(&self) -> Value { self.sink.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }};}

impl_set_scalar_fxn!(Set1DSRD,RowDVector); 
impl_set_scalar_fxn!(Set1DSVD,DVector); 
impl_set_scalar_fxn!(Set1DSMD,DMatrix); 

macro_rules! impl_set_scalar_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident);+ $(;)?) => {
    paste!{
      match $arg {
        $(
            (Value::[<Matrix $value_kind>](Matrix::DMatrix(input)),   [Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MD>] { sink: input.clone(), ixes: ix.clone(), source: source.clone() })),
            (Value::[<Matrix $value_kind>](Matrix::RowDVector(input)),[Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name RD>] { sink: input.clone(), ixes: ix.clone(), source: source.clone() })),
            (Value::[<Matrix $value_kind>](Matrix::DVector(input)),   [Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name VD>] { sink: input.clone(), ixes: ix.clone(), source: source.clone() })),
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_scalar_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set1DS, scalar, (sink, ixes.as_slice(), source))
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
    match impl_set_scalar_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_scalar_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3] := 1 ------------------------------------------------------------------

macro_rules! impl_set_range_fxn {
  ($struct_name:ident, $matrix_shape:ident) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<T>,
      ixes: Ref<DVector<usize>>,
      sink: Ref<$matrix_shape<T>>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$matrix_shape<T>>: ToValue
    {
      fn solve(&self) {
        let sink_ptr = self.sink.as_ptr();
        let ixes_ptr = self.ixes.as_ptr();
        let source_ptr = self.source.as_ptr();
        unsafe { 
          for i in 0..(*ixes_ptr).len() {
            (*sink_ptr)[(*ixes_ptr)[i] - 1] = (*source_ptr).clone();
          }
        }
      }
      fn out(&self) -> Value { self.sink.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }};}

impl_set_range_fxn!(Set1DRRD,RowDVector); 
impl_set_range_fxn!(Set1DRVD,DVector); 
impl_set_range_fxn!(Set1DRMD,DMatrix); 

macro_rules! impl_set_range_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident);+ $(;)?) => {
    paste!{
      match $arg {
        $(
            (Value::[<Matrix $value_kind>](Matrix::DMatrix(input)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MD>] { sink: input.clone(), ixes: ix.clone(), source: source.clone() })),
            (Value::[<Matrix $value_kind>](Matrix::RowDVector(input)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name RD>] { sink: input.clone(), ixes: ix.clone(), source: source.clone() })),
            (Value::[<Matrix $value_kind>](Matrix::DVector(input)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name VD>] { sink: input.clone(), ixes: ix.clone(), source: source.clone() })),
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_range_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set1DR, range, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetRange {}
impl NativeFunctionCompiler for MatrixSetRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_range_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_range_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[:] := 1 ------------------------------------------------------------------

macro_rules! impl_set_all_fxn {
  ($struct_name:ident, $matrix_shape:ident) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<T>,
      sink: Ref<$matrix_shape<T>>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$matrix_shape<T>>: ToValue
    {
      fn solve(&self) {
        let sink_ptr = self.sink.as_ptr();
        let source_ptr = self.source.as_ptr();
        unsafe { 
          for i in 0..(*sink_ptr).len() {
            (*sink_ptr)[i] = (*source_ptr).clone();
          }
        }
      }
      fn out(&self) -> Value { self.sink.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }};}

impl_set_all_fxn!(Set1DARD,RowDVector); 
impl_set_all_fxn!(Set1DAVD,DVector); 
impl_set_all_fxn!(Set1DAMD,DMatrix); 

macro_rules! impl_set_all_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident);+ $(;)?) => {
    paste!{
      match $arg {
        $(
            (Value::[<Matrix $value_kind>](Matrix::DMatrix(input)),   [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MD>] { sink: input.clone(), source: source.clone() })),
            (Value::[<Matrix $value_kind>](Matrix::RowDVector(input)),[Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name RD>] { sink: input.clone(), source: source.clone() })),
            (Value::[<Matrix $value_kind>](Matrix::DVector(input)),   [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name VD>] { sink: input.clone(), source: source.clone() })),
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_all_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set1DA, all, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetAll {}
impl NativeFunctionCompiler for MatrixSetAll {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_all_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_all_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1,1] := 1 ----------------------------------------------------------------

macro_rules! impl_set_scalar_scalar_fxn {
  ($struct_name:ident, $matrix_shape:ident) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<T>,
      ixes: (Ref<usize>,Ref<usize>),
      sink: Ref<$matrix_shape<T>>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$matrix_shape<T>>: ToValue
    {
      fn solve(&self) {
        let sink_ptr = self.sink.as_ptr();
        let (ixx,ixy) = &self.ixes;
        let ixx_ptr = ixx.as_ptr();
        let ixy_ptr = ixy.as_ptr();
        let source_ptr = self.source.as_ptr();
        unsafe {
          (*sink_ptr).column_mut(*ixy_ptr - 1)[*ixx_ptr - 1] = (*source_ptr).clone();
        }
      }
      fn out(&self) -> Value { self.sink.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }};}

impl_set_scalar_scalar_fxn!(Set2DSSRD,RowDVector); 
impl_set_scalar_scalar_fxn!(Set2DSSVD,DVector); 
impl_set_scalar_scalar_fxn!(Set2DSSMD,DMatrix); 

macro_rules! impl_set_scalar_scalar_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident);+ $(;)?) => {
    paste!{
      match $arg {
        $(
            (Value::[<Matrix $value_kind>](Matrix::DMatrix(input)),   [Value::Index(ixx),Value::Index(ixy)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MD>] { sink: input.clone(),   ixes: (ixx.clone(),ixy.clone()), source: source.clone() })),
            (Value::[<Matrix $value_kind>](Matrix::RowDVector(input)),[Value::Index(ixx),Value::Index(ixy)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name RD>] { sink: input.clone(),   ixes: (ixx.clone(),ixy.clone()), source: source.clone() })),
            (Value::[<Matrix $value_kind>](Matrix::DVector(input)),   [Value::Index(ixx),Value::Index(ixy)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name VD>] { sink: input.clone(),   ixes: (ixx.clone(),ixy.clone()), source: source.clone() })),
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_scalar_scalar_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set2DSS, scalar_scalar, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetScalarScalar {}
impl NativeFunctionCompiler for MatrixSetScalarScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_scalar_scalar_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_scalar_scalar_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[:,1] := 1 ----------------------------------------------------------------

macro_rules! impl_set_all_scalar_fxn {
  ($struct_name:ident, $matrix_shape:ident) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<T>,
      ix: Ref<usize>,
      sink: Ref<$matrix_shape<T>>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$matrix_shape<T>>: ToValue
    {
      fn solve(&self) {
        let sink_ptr = self.sink.as_ptr();
        let ix_ptr = self.ix.as_ptr();
        let source_ptr = self.source.as_ptr();
        unsafe {
          for i in 0..(*sink_ptr).nrows() {
            (*sink_ptr).column_mut(*ix_ptr - 1)[i] = (*source_ptr).clone();
          }
        }
      }
      fn out(&self) -> Value { self.sink.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }};}

impl_set_all_scalar_fxn!(Set2DASRD,RowDVector); 
impl_set_all_scalar_fxn!(Set2DASVD,DVector); 
impl_set_all_scalar_fxn!(Set2DASMD,DMatrix); 

macro_rules! impl_set_all_scalar_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident);+ $(;)?) => {
    paste!{
      match $arg {
        $(
            (Value::[<Matrix $value_kind>](Matrix::DMatrix(input)),   [Value::IndexAll, Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MD>] { sink: input.clone(), ix: ix.clone(), source: source.clone() })),
            (Value::[<Matrix $value_kind>](Matrix::RowDVector(input)),[Value::IndexAll, Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name RD>] { sink: input.clone(), ix: ix.clone(), source: source.clone() })),
            (Value::[<Matrix $value_kind>](Matrix::DVector(input)),   [Value::IndexAll, Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name VD>] { sink: input.clone(), ix: ix.clone(), source: source.clone() })),
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_all_scalar_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set2DAS, all_scalar, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetAllScalar {}
impl NativeFunctionCompiler for MatrixSetAllScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_all_scalar_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_all_scalar_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1,:] := 1 ----------------------------------------------------------------

macro_rules! impl_set_scalar_all_fxn {
  ($struct_name:ident, $matrix_shape:ident) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<T>,
      ix: Ref<usize>,
      sink: Ref<$matrix_shape<T>>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$matrix_shape<T>>: ToValue
    {
      fn solve(&self) {
        let sink_ptr = self.sink.as_ptr();
        let ix_ptr = self.ix.as_ptr();
        let source_ptr = self.source.as_ptr();
        unsafe {
          for i in 0..(*sink_ptr).ncols() {
            (*sink_ptr).row_mut(*ix_ptr - 1)[i] = (*source_ptr).clone();
          }
        }
      }
      fn out(&self) -> Value { self.sink.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }};}

impl_set_scalar_all_fxn!(Set2DSARD,RowDVector); 
impl_set_scalar_all_fxn!(Set2DSAVD,DVector); 
impl_set_scalar_all_fxn!(Set2DSAMD,DMatrix); 

macro_rules! impl_set_scalar_all_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident);+ $(;)?) => {
    paste!{
      match $arg {
        $(
            (Value::[<Matrix $value_kind>](Matrix::DMatrix(input)),   [Value::Index(ix), Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MD>] { sink: input.clone(), ix: ix.clone(), source: source.clone() })),
            (Value::[<Matrix $value_kind>](Matrix::RowDVector(input)),[Value::Index(ix), Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name RD>] { sink: input.clone(), ix: ix.clone(), source: source.clone() })),
            (Value::[<Matrix $value_kind>](Matrix::DVector(input)),   [Value::Index(ix), Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name VD>] { sink: input.clone(), ix: ix.clone(), source: source.clone() })),
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_scalar_all_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set2DSA, scalar_all, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetScalarAll {}
impl NativeFunctionCompiler for MatrixSetScalarAll {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_scalar_all_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_scalar_all_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3,1] := 1 ------------------------------------------------------------------

macro_rules! impl_set_range_scalar_fxn {
  ($struct_name:ident, $matrix_shape:ident) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<T>,
      ixes: (Ref<DVector<usize>>,Ref<usize>),
      sink: Ref<$matrix_shape<T>>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$matrix_shape<T>>: ToValue
    {
      fn solve(&self) {
        let sink_ptr = self.sink.as_ptr();
        let (range_ix,scalar_ix) = &self.ixes;
        let range_ix_ptr = range_ix.as_ptr();
        let scalar_ix_ptr = scalar_ix.as_ptr();
        let source_ptr = self.source.as_ptr();
        unsafe { 
          for rix in &*range_ix_ptr {
            (*sink_ptr).row_mut(rix - 1)[*scalar_ix_ptr - 1] = (*source_ptr).clone();
          }
        }
      }
      fn out(&self) -> Value { self.sink.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }};}

impl_set_range_scalar_fxn!(Set2DRSRD,RowDVector); 
impl_set_range_scalar_fxn!(Set2DRSVD,DVector); 
impl_set_range_scalar_fxn!(Set2DRSMD,DMatrix); 

macro_rules! impl_set_range_scalar_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident);+ $(;)?) => {
    paste!{
      match $arg {
        $(
            (Value::[<Matrix $value_kind>](Matrix::DMatrix(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MD>] { sink: input.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
            (Value::[<Matrix $value_kind>](Matrix::RowDVector(input)),[Value::MatrixIndex(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name RD>] { sink: input.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
            (Value::[<Matrix $value_kind>](Matrix::DVector(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name VD>] { sink: input.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_range_scalar_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set2DRS, range_scalar, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetRangeScalar {}
impl NativeFunctionCompiler for MatrixSetRangeScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_range_scalar_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_range_scalar_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1,1..3] := 1 ------------------------------------------------------------------

macro_rules! impl_set_scalar_range_fxn {
  ($struct_name:ident, $matrix_shape:ident) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<T>,
      ixes: (Ref<usize>,Ref<DVector<usize>>),
      sink: Ref<$matrix_shape<T>>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$matrix_shape<T>>: ToValue
    {
      fn solve(&self) {
        let sink_ptr = self.sink.as_ptr();
        let (scalar_ix,range_ix) = &self.ixes;
        let range_ix_ptr = range_ix.as_ptr();
        let scalar_ix_ptr = scalar_ix.as_ptr();
        let source_ptr = self.source.as_ptr();
        unsafe { 
          for rix in &*range_ix_ptr {
            (*sink_ptr).column_mut(rix - 1)[*scalar_ix_ptr - 1] = (*source_ptr).clone();
          }
        }
      }
      fn out(&self) -> Value { self.sink.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }};}

impl_set_scalar_range_fxn!(Set2DSRRD,RowDVector); 
impl_set_scalar_range_fxn!(Set2DSRVD,DVector); 
impl_set_scalar_range_fxn!(Set2DSRMD,DMatrix); 

macro_rules! impl_set_scalar_range_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident);+ $(;)?) => {
    paste!{
      match $arg {
        $(
            (Value::[<Matrix $value_kind>](Matrix::DMatrix(input)),   [Value::Index(ix1),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MD>] { sink: input.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
            (Value::[<Matrix $value_kind>](Matrix::RowDVector(input)),[Value::Index(ix1),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name RD>] { sink: input.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
            (Value::[<Matrix $value_kind>](Matrix::DVector(input)),   [Value::Index(ix1),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name VD>] { sink: input.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_scalar_range_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set2DSR, scalar_range, (sink, ixes.as_slice(), source))
}
pub struct MatrixSetScalarRange {}
impl NativeFunctionCompiler for MatrixSetScalarRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_scalar_range_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_scalar_range_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3,1..3] := 1 ------------------------------------------------------------------

macro_rules! impl_set_range_range_fxn {
  ($struct_name:ident, $matrix_shape:ident) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<T>,
      ixes: (Ref<DVector<usize>>,Ref<DVector<usize>>),
      sink: Ref<$matrix_shape<T>>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$matrix_shape<T>>: ToValue
    {
      fn solve(&self) {
        let sink_ptr = self.sink.as_ptr();
        let (range_ix1,range_ix2) = &self.ixes;
        let range_ix1_ptr = range_ix1.as_ptr();
        let range_ix2_ptr = range_ix2.as_ptr();
        let source_ptr = self.source.as_ptr();
        unsafe { 
          for cix in &*range_ix2_ptr {
            for rix in &*range_ix1_ptr {
              (*sink_ptr).column_mut(cix - 1)[rix - 1] = (*source_ptr).clone();
            }
          }
        }
      }
      fn out(&self) -> Value { self.sink.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }};}

impl_set_range_range_fxn!(Set2DRRRD,RowDVector); 
impl_set_range_range_fxn!(Set2DRRVD,DVector); 
impl_set_range_range_fxn!(Set2DRRMD,DMatrix); 

macro_rules! impl_set_range_range_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident);+ $(;)?) => {
    paste!{
      match $arg {
        $(
            (Value::[<Matrix $value_kind>](Matrix::DMatrix(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MD>] { sink: input.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
            (Value::[<Matrix $value_kind>](Matrix::RowDVector(input)),[Value::MatrixIndex(Matrix::DVector(ix1)),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name RD>] { sink: input.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
            (Value::[<Matrix $value_kind>](Matrix::DVector(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name VD>] { sink: input.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_range_range_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set2DRR, range_range, (sink, ixes.as_slice(), source))
}
pub struct MatrixSetRangeRange {}
impl NativeFunctionCompiler for MatrixSetRangeRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_range_range_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_range_range_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}