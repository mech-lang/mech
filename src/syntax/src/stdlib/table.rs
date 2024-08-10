#[macro_use]
use crate::stdlib::*;

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
  
  macro_rules! access_2d_slice {
    ($source:expr, $ix:expr, $out:expr) => {
      unsafe { 
        let nrows = (*$ix).0.len();
        let ncols = (*$ix).1.len();
        let mut out_ix = 0;
        for j in 0..ncols {
          for i in 0..nrows {
            (*$out)[out_ix] = (*$source).index(((*$ix).0[i]-1,(*$ix).1[j]-1)).clone();
            out_ix += 1;
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
  
  macro_rules! access_2d_all_slice {
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
  
  macro_rules! access_2d_row_slice {
    ($source:expr, $ix:expr, $out:expr) => {
      unsafe { 
        let ix1 = &(*$ix).0;
        let ix2 = &(*$ix).1;
        let out_cols = ix2.ncols();
        let mut out_ix = 0;
        for c in 0..out_cols {
          (*$out)[out_ix] = (*$source).index((ix1 - 1,ix2[c] - 1)).clone();
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
  impl_access_fxn_shape!(Access1DVD, DVector<usize>, DVector<T>, access_1d_slice);
  impl_access_fxn_shape!(Access1DRD, RowDVector<usize>, RowDVector<T>, access_1d_slice);
  
  impl_access_fxn_shape!(Access1DRDb, RowDVector<bool>, RowDVector<T>, access_1d_slice_bool);
  impl_access_fxn_shape!(Access1DVDb, DVector<bool>, DVector<T>, access_1d_slice_bool_v);
  
  // x[1..3,1..3]
  impl_access_fxn_shape!(Access2DRDRD, (RowDVector<usize>,RowDVector<usize>), DMatrix<T>, access_2d_slice);
  
  // x[:]
  impl_access_fxn_shape!(Access1DA, Value, DVector<T>, access_1d_all);
  
  // x[:,1]
  impl_access_fxn_shape!(Access2DAS, usize, DVector<T>, access_col);
  
  // x[1,:]
  impl_access_fxn_shape!(Access2DSA, usize, RowDVector<T>, access_row);
  
  // x[1..3,:]
  impl_access_fxn_shape!(Access2DRDA, RowDVector<usize>, DMatrix<T>, access_2d_slice2_all);
  impl_access_fxn_shape!(Access2DVDA, DVector<usize>, DMatrix<T>, access_2d_slice3_all);
  
  // x[:,1..3]
  impl_access_fxn_shape!(Access2DARD, RowDVector<usize>, DMatrix<T>, access_2d_all_slice);
  impl_access_fxn_shape!(Access2DAVD, DVector<usize>, DMatrix<T>, access_2d_all_slice);
  
  
  // x[2,1..3]
  impl_access_fxn_shape!(Access2DSRD, (usize, RowDVector<usize>), RowDVector<T>, access_2d_row_slice);
  impl_access_fxn_shape!(Access2DSVD, (usize, DVector<usize>), RowDVector<T>, access_2d_row_slice);
  
  // x[1..3,2]
  impl_access_fxn_shape!(Access2DRDS, (RowDVector<usize>, usize), DVector<T>, access_2d_col_slice);
  impl_access_fxn_shape!(Access2DVDS, (DVector<usize>, usize), DVector<T>, access_2d_col_slice);
  
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