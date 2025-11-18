#[macro_use]
use crate::stdlib::*;
use std::marker::PhantomData;
use std::fmt::Debug;
use nalgebra::{
  base::{Matrix as naMatrix, Storage, StorageMut},
  Dim, Scalar,
};



// Access ---------------------------------------------------------------------

#[macro_export]
macro_rules! impl_access_fxn_new {
  ($op:tt, $fxn_name:ident, $arg:expr, $value_kind:ident, $value_string:tt) => {{
    let mut res: MResult<_> = Err(MechError2::new(
      GenericError { msg: "No matching type found".to_string() },
      None
    ));
    
    #[cfg(feature = "row_vector2")]
    {
      res = res.or_else(|_| $op!($fxn_name, RowVector2, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "row_vector3")]
    {
      res = res.or_else(|_| $op!($fxn_name, RowVector3, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "row_vector4")]
    {
      res = res.or_else(|_| $op!($fxn_name, RowVector4, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "vector2")]
    {
      res = res.or_else(|_| $op!($fxn_name, Vector2, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "vector3")]
    {
      res = res.or_else(|_| $op!($fxn_name, Vector3, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "vector4")]
    {
      res = res.or_else(|_| $op!($fxn_name, Vector4, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "matrix1")]
    {
      res = res.or_else(|_| $op!($fxn_name, Matrix1, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "matrix2")]
    {
      res = res.or_else(|_| $op!($fxn_name, Matrix2, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "matrix3")]
    {
      res = res.or_else(|_| $op!($fxn_name, Matrix3, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "matrix4")]
    {
      res = res.or_else(|_| $op!($fxn_name, Matrix4, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "matrix2x3")]
    {
      res = res.or_else(|_| $op!($fxn_name, Matrix2x3, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "matrix3x2")]
    {
      res = res.or_else(|_| $op!($fxn_name, Matrix3x2, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "matrixd")]
    {
      res = res.or_else(|_| $op!($fxn_name, DMatrix, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "row_vectord")]
    {
      res = res.or_else(|_| $op!($fxn_name, RowDVector, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "vectord")]
    {
      res = res.or_else(|_| $op!($fxn_name, DVector, &$arg, $value_kind, $value_string));
    }

    let (ref source, ref ixes) = &$arg;
    res.map_err(|_| MechError2::new(
      UnhandledFunctionArgumentIxesMono {
        arg: (source.clone(), ixes.to_vec()),
        fxn_name: stringify!($fxn_name).to_string(),
      },
      None,
    ).with_compiler_loc())
  }}
}

macro_rules! access_1d {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { *$out = (*$source).index(*$ix - 1).clone() }
  };}

macro_rules! access_2d {
  ($source:expr, $ix1:expr, $ix2:expr, $out:expr) => {
    unsafe { 
      *$out = (*$source).index((*$ix1 - 1, *$ix2 - 1)).clone() 
    }
  };}
macro_rules! access_1d_slice {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      for i in 0..(*$ix).len() {
        ((&mut *$out))[i] = (*$source).index((&(*$ix))[i] - 1).clone();
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
        (*$out).resize_vertically_mut(j, (&(*$out))[0].clone());
      }
      j = 0;
      for i in 0..(*$source).len() {
        if (*$ix)[i] == true {
          (&mut (*$out))[j] = (*$source).index(i).clone();
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
        if (&(*$ix))[i] == true {
          j += 1;
        }
      }
      if j != out_len {
        (*$out).resize_vertically_mut(j, (&(*$out))[0].clone());
      }
      j = 0;
      for i in 0..(*$source).len() {
        if (&(*$ix))[i] == true {
          (&mut (*$out))[j] = (*$source).index(i).clone();
          j += 1;
        }
      }
    }};}    

macro_rules! access_2d_row_slice_bool {
  ($source:expr, $ix1:expr, $ix2:expr, $out:expr) => {
    unsafe { 
      let scalar_ix = &(*$ix1);
      let vec_ix = &(*$ix2);
      let mut j = 0;
      let out_len = (*$out).len();
      for i in 0..vec_ix.len() {
        if vec_ix[i] == true {
          j += 1;
        }
      }
      if j != out_len {
        (*$out).resize_horizontally_mut(j, (&(*$out))[0].clone());
      }
      j = 0;
      for i in 0..vec_ix.len() {
        if vec_ix[i] == true {
          (&mut (*$out))[j] = (*$source).index((scalar_ix - 1, i)).clone();
          j += 1;
        }
      }
    }};}

macro_rules! access_2d_col_slice_bool {
  ($source:expr, $ix1:expr, $ix2:expr, $out:expr) => {
    unsafe { 
      let vec_ix = &(*$ix1);
      let scalar_ix = &(*$ix2);
      let mut j = 0;
      let out_len = (*$out).len();
      for i in 0..vec_ix.len() {
        if vec_ix[i] == true {
          j += 1;
        }
      }
      if j != out_len {
        (*$out).resize_vertically_mut(j, (&(*$out))[0].clone());
      }
      j = 0;
      for i in 0..vec_ix.len() {
        if vec_ix[i] == true {
          (&mut (*$out))[j] = (*$source).index((i, scalar_ix - 1)).clone();
          j += 1;
        }
      }
    }};}    

macro_rules! access_2d_slice {
  ($source:expr, $ix1:expr, $ix2:expr, $out:expr) => {
    unsafe { 
      let nrows = (*$ix1).len();
      let ncols = (*$ix2).len();
      let mut out_ix = 0;
      for j in 0..ncols {
        for i in 0..nrows {
          (&mut (*$out))[out_ix] = (*$source).index(((&(*$ix1))[i] - 1, (&(*$ix2))[j] - 1)).clone();
          out_ix += 1;
        }
      }
    }};}

macro_rules! access_2d_slice_bool {
  ($source:expr, $ix1:expr, $ix2:expr, $out:expr) => {
    unsafe { 
      let ix1 = &(*$ix1);
      let ix2 = &(*$ix2);
      let mut j = 0;
      let out_len = (*$out).len();
      for i in 0..ix1.len() {
        if ix1[i] == true {
          j += 1;
        }
      }
      if j != (*$out).nrows() {
        (*$out).resize_vertically_mut(j, (&(*$out))[0].clone());
      }
      j = 0;
      for k in 0..ix2.len() {
        for i in 0..ix1.len() {
          if ix1[i] == true {
            (&mut (*$out))[j] = (*$source).index((i, ix2[k] - 1)).clone();
            j += 1;
          }
        }
      }
    }};}  
    
macro_rules! access_2d_slice_bool2 {
  ($source:expr, $ix1:expr, $ix2:expr, $out:expr) => {
    unsafe { 
      let ix1 = &(*$ix1);
      let ix2 = &(*$ix2);
      let mut j = 0;
      let out_len = (*$out).len();
      for i in 0..ix2.len() {
        if ix2[i] == true {
          j += 1;
        }
      }
      if j != (*$out).ncols() {
        (*$out).resize_horizontally_mut(j, (& (*$out))[0].clone());
      }
      j = 0;
      for k in 0..ix2.len() {
        for i in 0..ix1.len() {
          if ix2[k] == true {
            (&mut (*$out))[j] = (*$source).index((ix1[i] - 1, k)).clone();
            j += 1;
          }
        }
      }
    }};}    

macro_rules! access_2d_slice_bool_bool {
  ($source:expr, $ix1:expr, $ix2:expr, $out:expr) => {
    unsafe { 
      let ix1 = &(*$ix1);
      let ix2 = &(*$ix2);
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
        (*$out).resize_mut(j, k, (&(*$out))[0].clone());
      }
      let mut out_ix = 0;
      for k in 0..ix2.len() {
        for j in 0..ix1.len() {
          if ix1[j] == true && ix2[k] == true {
            (&mut (*$out))[out_ix] = (*$source).index((j, k)).clone();
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
          (&mut (*$out))[out_ix] = (*$source).index(((&(*$ix))[r] - 1, c)).clone();
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
        (*$out).resize_vertically_mut(j, (&mut (*$out))[0].clone());
      }
      j = 0;
      for i in 0..vec_ix.len() {
        for k in 0..(*$source).ncols() {
          if vec_ix[i] == true {
            (&mut (*$out))[j] = (*$source).index((i, k)).clone();
            j += 1;
          }
        }
      }
    }};}

macro_rules! access_2d_row_slice {
  ($source:expr, $ix1:expr, $ix2:expr, $out:expr) => {
    unsafe { 
      let ix1 = &(*$ix1);
      let ix2 = &(*$ix2);
      let out_cols = ix2.nrows();
      let mut out_ix = 0;
      for c in 0..out_cols {
        (&mut (*$out))[out_ix] = (*$source).index((ix1 - 1, ix2[c] - 1)).clone();
        out_ix += 1;
      }
    }};}    

macro_rules! access_2d_col_slice {
  ($source:expr, $ix1:expr, $ix2:expr, $out:expr) => {
    unsafe { 
      let ix1 = &(*$ix1);
      let ix2 = &(*$ix2);
      let out_rows = ix1.nrows();
      let mut out_ix = 0;
      for c in 0..out_rows {
        (&mut (*$out))[out_ix] = (*$source).index((ix1[c] - 1, ix2 - 1)).clone();
        out_ix += 1;
      }
    }};}    

macro_rules! access_col {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      for i in 0..(*$source).nrows() {
        (&mut (*$out))[i] = (*$source).index((i, *$ix - 1)).clone();
      }
    }};}

macro_rules! access_row {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      for i in 0..(*$source).ncols() {
        (&mut (*$out))[i] = (*$source).index((*$ix - 1, i)).clone();
      }
    }};}

macro_rules! access_1d_all {
  ($source:expr, $ix:expr, $out:expr) => {
    unsafe { 
      for i in 0..(*$source).len() {
        (&mut (*$out))[i] = (*$source).index(i).clone();
      }
    }};}


/*#[macro_export]
macro_rules! impl_access_all_fxn_v {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    #[derive(Debug)]
    pub struct $struct_name<T, MatA, MatB, IxVec> {
      pub source: Ref<MatB>,
      pub ixes: Ref<IxVec>,
      pub sink: Ref<MatA>,
      pub _marker: PhantomData<T>,
    }
    impl<T, R1: 'static, C1: 'static, S1: 'static, R2: 'static, C2: 'static, S2: 'static, IxVec: 'static> MechFunctionFactory for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
    where
      Ref<naMatrix<T, R1, C1, S1>>: ToValue,
      Ref<naMatrix<T, R2, C2, S2>>: ToValue,
      T: Debug + Clone + Sync + Send + 'static +
        PartialEq + PartialOrd +
        CompileConst + ConstElem + AsValueKind,
      IxVec: CompileConst + ConstElem + AsNaKind + Debug + AsRef<[$ix]>,
      R1: Dim, C1: Dim, S1: StorageMut<T, R1, C1> + Clone + Debug,
      R2: Dim, C2: Dim, S2: Storage<T, R2, C2> + Clone + Debug,
      naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + Debug + AsNaKind,
      naMatrix<T, R2, C2, S2>: CompileConst + ConstElem + Debug + AsNaKind,
    {
      fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
          FunctionArgs::Binary(out, arg1, arg2) => {
            let source: Ref<naMatrix<T, R2, C2, S2>> = unsafe { arg1.as_unchecked() }.clone();
            let ixes: Ref<IxVec> = unsafe { arg2.as_unchecked() }.clone();
            let sink: Ref<naMatrix<T, R1, C1, S1>> = unsafe { out.as_unchecked() }.clone();
            Ok(Box::new(Self { sink, source, ixes, _marker: PhantomData::default() }))
          },
          _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{} requires 3 arguments, got {:?}", stringify!($struct_name), args), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments})
        }
      }
    }
    impl<T, R1, C1, S1, R2, C2, S2, IxVec>
      MechFunctionImpl for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
    where
      Ref<naMatrix<T, R1, C1, S1>>: ToValue,
      T: Debug + Clone + Sync + Send + 'static +
         PartialEq + PartialOrd,
      IxVec: AsRef<[$ix]> + Debug,
      R1: Dim, C1: Dim, S1: StorageMut<T, R1, C1> + Clone + Debug,
      R2: Dim, C2: Dim, S2: Storage<T, R2, C2> + Clone + Debug,
    {
      fn solve(&self) {
        unsafe {
          let sink_ptr = &mut *self.sink.as_mut_ptr();
          let source_ptr = &*self.source.as_ptr();
          let ix_ptr = &(*self.ixes.as_ptr()).as_ref();
          $op!(source_ptr,ix_ptr,sink_ptr);
        }
      }
      fn out(&self) -> Value {self.sink.to_value()}
      fn to_string(&self) -> String {format!("{:#?}", self)}
    }
    #[cfg(feature = "compiler")]
    impl<T, R1, C1, S1, R2, C2, S2, IxVec> MechFunctionCompiler for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec> 
    where
      T: CompileConst + ConstElem + AsValueKind,
      IxVec: CompileConst + ConstElem + AsNaKind,
      naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + AsNaKind,
      naMatrix<T, R2, C2, S2>: CompileConst + ConstElem + AsNaKind,
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let name = format!("{}<{}{}{}{}>", stringify!($struct_name), T::as_value_kind(), naMatrix::<T, R1, C1, S1>::as_na_kind(), naMatrix::<T, R2, C2, S2>::as_na_kind(), IxVec::as_na_kind());
        compile_binop!(name, self.sink, self.source, self.ixes, ctx, FeatureFlag::Builtin(FeatureKind::OpAssign));
      }
    }  
  };}*/

macro_rules! impl_access_fxn {
  ($struct_name:ident, $arg_type:ty, $ix_type:ty, $out_type:ty, $op:ident) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<$arg_type>,
      ixes: Ref<$ix_type>,
      out: Ref<$out_type>,
    }
    impl<T> MechFunctionFactory for $struct_name<T> 
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static +
         CompileConst + ConstElem + AsValueKind,
      Ref<$out_type>: ToValue
    {
      fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
          FunctionArgs::Binary(out, arg1, arg2) => {
            let n: Ref<$arg_type> = unsafe{ arg1.as_unchecked().clone() };
            let k: Ref<$ix_type> = unsafe{ arg2.as_unchecked().clone() };
            let out: Ref<$out_type> = unsafe{ out.as_unchecked().clone() };
            Ok(Box::new($struct_name{source: n,ixes: k,out}))
          }
          _ => Err(MechError2::new(IncorrectNumberOfArguments{expected: 2, found: args.len()}, None).with_compiler_loc()),
        }
      }
    }
    impl<T> MechFunctionImpl for $struct_name<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$out_type>: ToValue
    {
      fn solve(&self) {
        let source_ptr = self.source.as_ptr();
        let ixes_ptr = self.ixes.as_ptr();
        let out_ptr = self.out.as_mut_ptr();
        $op!(source_ptr,ixes_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }
    #[cfg(feature = "compiler")]
    impl<T> MechFunctionCompiler for $struct_name<T> 
    where
      T: CompileConst + ConstElem + AsValueKind,
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let name = format!("{}<{}>", stringify!($struct_name), T::as_value_kind());
        compile_binop!(name, self.out, self.source, self.ixes, ctx, FeatureFlag::Builtin(FeatureKind::Access));
      }
    }};}

macro_rules! impl_access_fxn2 {
  ($struct_name:ident, $arg_type:ty, $ix1_type:ty, $ix2_type:ty, $out_type:ty, $op:ident) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<$arg_type>,
      ix1: Ref<$ix1_type>,
      ix2: Ref<$ix2_type>,
      out: Ref<$out_type>,
    }
    impl<T> MechFunctionFactory for $struct_name<T> 
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static +
         CompileConst + ConstElem + AsValueKind,
      Ref<$out_type>: ToValue
    {
      fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
          FunctionArgs::Ternary(out, arg1, arg2, arg3) => {
            let source: Ref<$arg_type> = unsafe{ arg1.as_unchecked().clone() };
            let ix1: Ref<$ix1_type> = unsafe{ arg2.as_unchecked().clone() };
            let ix2: Ref<$ix2_type> = unsafe{ arg3.as_unchecked().clone() };
            let out: Ref<$out_type> = unsafe{ out.as_unchecked().clone() };
            Ok(Box::new($struct_name{source ,ix1, ix2, out}))
          }
          _ => Err(MechError2::new(IncorrectNumberOfArguments{expected: 3, found: args.len()}, None).with_compiler_loc()),
        }
      }
    }
    impl<T> MechFunctionImpl for $struct_name<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$out_type>: ToValue
    {
      fn solve(&self) {
        let source_ptr = self.source.as_ptr();
        let ix1_ptr = self.ix1.as_ptr();
        let ix2_ptr = self.ix2.as_ptr();
        let out_ptr = self.out.as_mut_ptr();
        $op!(source_ptr,ix1_ptr,ix2_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }
    #[cfg(feature = "compiler")]
    impl<T> MechFunctionCompiler for $struct_name<T> 
    where
      T: CompileConst + ConstElem + AsValueKind,
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let name = format!("{}<{}>", stringify!($struct_name), T::as_value_kind());
        compile_ternop!(name, self.out, self.source, self.ix1, self.ix2, ctx, FeatureFlag::Builtin(FeatureKind::Access) );
      }
    }};}    

macro_rules! impl_access_fxn_shape {
  ($name:ident, $ix_type:ty, $out_type:ty, $fxn:ident) => {
    paste!{
      #[cfg(feature = "matrix1")]
      impl_access_fxn!([<$name M1>],   Matrix1<T>,    $ix_type, $out_type, $fxn);
      #[cfg(feature = "matrix2")]
      impl_access_fxn!([<$name M2>],   Matrix2<T>,    $ix_type, $out_type, $fxn);
      #[cfg(feature = "matrix3")]
      impl_access_fxn!([<$name M3>],   Matrix3<T>,    $ix_type, $out_type, $fxn);
      #[cfg(feature = "matrix4")]
      impl_access_fxn!([<$name M4>],   Matrix4<T>,    $ix_type, $out_type, $fxn);
      #[cfg(feature = "matrix2x3")]
      impl_access_fxn!([<$name M2x3>], Matrix2x3<T>,  $ix_type, $out_type, $fxn);
      #[cfg(feature = "matrix3x2")]
      impl_access_fxn!([<$name M3x2>], Matrix3x2<T>,  $ix_type, $out_type, $fxn);
      #[cfg(feature = "matrixd")]
      impl_access_fxn!([<$name MD>],   DMatrix<T>,    $ix_type, $out_type, $fxn);
      #[cfg(feature = "vector2")]
      impl_access_fxn!([<$name V2>],   Vector2<T>,    $ix_type, $out_type, $fxn);
      #[cfg(feature = "vector3")]
      impl_access_fxn!([<$name V3>],   Vector3<T>,    $ix_type, $out_type, $fxn);
      #[cfg(feature = "vector4")]
      impl_access_fxn!([<$name V4>],   Vector4<T>,    $ix_type, $out_type, $fxn);
      #[cfg(feature = "vectord")]
      impl_access_fxn!([<$name VD>],   DVector<T>,    $ix_type, $out_type, $fxn);
      #[cfg(feature = "row_vector2")]
      impl_access_fxn!([<$name R2>],   RowVector2<T>, $ix_type, $out_type, $fxn);
      #[cfg(feature = "row_vector3")]
      impl_access_fxn!([<$name R3>],   RowVector3<T>, $ix_type, $out_type, $fxn);
      #[cfg(feature = "row_vector4")]
      impl_access_fxn!([<$name R4>],   RowVector4<T>, $ix_type, $out_type, $fxn);
      #[cfg(feature = "row_vectord")]
      impl_access_fxn!([<$name RD>],   RowDVector<T>, $ix_type, $out_type, $fxn);
    }
  };}

macro_rules! impl_access_fxn_shape2 {
  ($name:ident, $ix1_type:ty, $ix2_type:ty, $out_type:ty, $fxn:ident) => {
    paste!{
      #[cfg(feature = "matrix1")]
      impl_access_fxn2!([<$name M1>],   Matrix1<T>,    $ix1_type, $ix2_type, $out_type, $fxn);
      #[cfg(feature = "matrix2")]
      impl_access_fxn2!([<$name M2>],   Matrix2<T>,    $ix1_type, $ix2_type, $out_type, $fxn);
      #[cfg(feature = "matrix3")]
      impl_access_fxn2!([<$name M3>],   Matrix3<T>,    $ix1_type, $ix2_type, $out_type, $fxn);
      #[cfg(feature = "matrix4")]
      impl_access_fxn2!([<$name M4>],   Matrix4<T>,    $ix1_type, $ix2_type, $out_type, $fxn);
      #[cfg(feature = "matrix2x3")]
      impl_access_fxn2!([<$name M2x3>], Matrix2x3<T>,  $ix1_type, $ix2_type, $out_type, $fxn);
      #[cfg(feature = "matrix3x2")]
      impl_access_fxn2!([<$name M3x2>], Matrix3x2<T>,  $ix1_type, $ix2_type, $out_type, $fxn);
      #[cfg(feature = "matrixd")]
      impl_access_fxn2!([<$name MD>],   DMatrix<T>,    $ix1_type, $ix2_type, $out_type, $fxn);
      #[cfg(feature = "vector2")]
      impl_access_fxn2!([<$name V2>],   Vector2<T>,    $ix1_type, $ix2_type, $out_type, $fxn);
      #[cfg(feature = "vector3")]
      impl_access_fxn2!([<$name V3>],   Vector3<T>,    $ix1_type, $ix2_type, $out_type, $fxn);
      #[cfg(feature = "vector4")]
      impl_access_fxn2!([<$name V4>],   Vector4<T>,    $ix1_type, $ix2_type, $out_type, $fxn);
      #[cfg(feature = "vectord")]
      impl_access_fxn2!([<$name VD>],   DVector<T>,    $ix1_type, $ix2_type, $out_type, $fxn);
      #[cfg(feature = "row_vector2")]
      impl_access_fxn2!([<$name R2>],   RowVector2<T>, $ix1_type, $ix2_type, $out_type, $fxn);
      #[cfg(feature = "row_vector3")]
      impl_access_fxn2!([<$name R3>],   RowVector3<T>, $ix1_type, $ix2_type, $out_type, $fxn);
      #[cfg(feature = "row_vector4")]
      impl_access_fxn2!([<$name R4>],   RowVector4<T>, $ix1_type, $ix2_type, $out_type, $fxn);
      #[cfg(feature = "row_vectord")]
      impl_access_fxn2!([<$name RD>],   RowDVector<T>, $ix1_type, $ix2_type, $out_type, $fxn);
    }
  };}  

// x[1]
impl_access_fxn_shape!(Access1DS, usize, T, access_1d);

// x[1,2]
impl_access_fxn_shape2!(Access2DSS, usize, usize, T, access_2d);

// x[1..3]
impl_access_fxn_shape!(Access1DVD, DVector<usize>, DVector<T>, access_1d_slice);
impl_access_fxn_shape!(Access1DVDb, DVector<bool>, DVector<T>, access_1d_slice_bool_v);

// x[:]
impl_access_fxn_shape!(Access1DA, Value, DVector<T>, access_1d_all);

// x[:,1]
impl_access_fxn_shape!(Access2DAS, usize, DVector<T>, access_col);

// x[1,:]
#[cfg(feature = "matrix1")]
impl_access_fxn!(Access2DSAM1,   Matrix1<T>,    usize, Matrix1<T>, access_row);
#[cfg(all(feature = "matrix2", feature = "row_vector2"))]
impl_access_fxn!(Access2DSAM2,   Matrix2<T>,    usize, RowVector2<T>, access_row);
#[cfg(all(feature = "matrix3", feature = "row_vector3"))]
impl_access_fxn!(Access2DSAM3,   Matrix3<T>,    usize, RowVector3<T>, access_row);
#[cfg(all(feature = "matrix4", feature = "row_vector4"))]
impl_access_fxn!(Access2DSAM4,   Matrix4<T>,    usize, RowVector4<T>, access_row);
#[cfg(all(feature = "matrix2x3", feature = "row_vector3"))]
impl_access_fxn!(Access2DSAM2x3, Matrix2x3<T>,  usize, RowVector3<T>, access_row);
#[cfg(all(feature = "matrix3x2", feature = "row_vector2"))]
impl_access_fxn!(Access2DSAM3x2, Matrix3x2<T>,  usize, RowVector2<T>, access_row);
#[cfg(all(feature = "matrixd", feature = "row_vectord"))]
impl_access_fxn!(Access2DSAMD,   DMatrix<T>,    usize, RowDVector<T>, access_row);

// x[1..3,:]
impl_access_fxn_shape!(Access2DVDA, DVector<usize>,    DMatrix<T>, access_2d_slice_all);
impl_access_fxn_shape!(Access2DVDbA, DVector<bool>,    DMatrix<T>, access_2d_slice_all_bool);

// x[2,1..3]
impl_access_fxn_shape2!(Access2DSVD,  usize, DVector<usize>,    RowDVector<T>, access_2d_row_slice);
impl_access_fxn_shape2!(Access2DSVDb, usize, DVector<bool>,     RowDVector<T>, access_2d_row_slice_bool);

// x[1..3,2]
impl_access_fxn_shape2!(Access2DVDS,  DVector<usize>, usize,    DVector<T>, access_2d_col_slice);
impl_access_fxn_shape2!(Access2DVDbS, DVector<bool>, usize,     DVector<T>, access_2d_col_slice_bool);

macro_rules! impl_access_match_arms {
  ($fxn_name:ident,$macro_name:ident, $arg:expr) => {
    paste!{
      [<impl_access_ $macro_name _match_arms>]!(
        $fxn_name,
        $arg,
        Bool => MatrixBool, bool, bool::default(), "bool";
        I8   => MatrixI8,   i8,   i8::default(),  "i8";
        I16  => MatrixI16,  i16,  i16::default(), "i16";
        I32  => MatrixI32,  i32,  i32::default(), "i32";
        I64  => MatrixI64,  i64,  i64::default(), "i64";
        I128 => MatrixI128, i128, i128::default(), "i128";
        U8   => MatrixU8,   u8,   u8::default(), "u8";
        U16  => MatrixU16,  u16,  u16::default(), "u16";
        U32  => MatrixU32,  u32,  u32::default(), "u32";
        U64  => MatrixU64,  u64,  u64::default(), "u64";
        U128 => MatrixU128, u128, u128::default(), "u128";
        F32  => MatrixF32,  F32,  F32::default(), "f32";
        F64  => MatrixF64,  F64,  F64::default(), "f64";
        String => MatrixString, String, String::default(), "string";
        C64 => MatrixC64, C64, C64::default(), "complex";
        R64 => MatrixR64, R64, R64::default(), "rational";
      )
    }
  }
}

// x[1] -----------------------------------------------------------------------

macro_rules! impl_access_scalar_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]              
            (Value::$matrix_kind(Matrix::RowVector4(input)), [Value::Index(ix)]) => {
              register_fxn_descriptor_inner!([<$fxn_name R4>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name R4>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]              
            (Value::$matrix_kind(Matrix::RowVector3(input)), [Value::Index(ix)]) => {
              register_fxn_descriptor_inner!([<$fxn_name R3>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name R3>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]              
            (Value::$matrix_kind(Matrix::RowVector2(input)), [Value::Index(ix)]) => {
              register_fxn_descriptor_inner!([<$fxn_name R2>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name R2>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "vector4"))]              
            (Value::$matrix_kind(Matrix::Vector4(input)),    [Value::Index(ix)]) => {
              register_fxn_descriptor_inner!([<$fxn_name V4>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name V4>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "vector3"))]              
            (Value::$matrix_kind(Matrix::Vector3(input)),    [Value::Index(ix)]) => {
              register_fxn_descriptor_inner!([<$fxn_name V3>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name V3>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "vector2"))]              
            (Value::$matrix_kind(Matrix::Vector2(input)),    [Value::Index(ix)]) => {
              register_fxn_descriptor_inner!([<$fxn_name V2>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name V2>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix4"))]              
            (Value::$matrix_kind(Matrix::Matrix4(input)),    [Value::Index(ix)]) => {
              register_fxn_descriptor_inner!([<$fxn_name M4>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name M4>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3"))]              
            (Value::$matrix_kind(Matrix::Matrix3(input)),    [Value::Index(ix)]) => {
              register_fxn_descriptor_inner!([<$fxn_name M3>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name M3>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2"))]              
            (Value::$matrix_kind(Matrix::Matrix2(input)),    [Value::Index(ix)]) => {
              register_fxn_descriptor_inner!([<$fxn_name M2>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name M2>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix1"))]              
            (Value::$matrix_kind(Matrix::Matrix1(input)),    [Value::Index(ix)]) => {
              register_fxn_descriptor_inner!([<$fxn_name M1>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name M1>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]              
            (Value::$matrix_kind(Matrix::Matrix2x3(input)),  [Value::Index(ix)]) => {
              register_fxn_descriptor_inner!([<$fxn_name M2x3>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name M2x3>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]              
            (Value::$matrix_kind(Matrix::Matrix3x2(input)),  [Value::Index(ix)]) => {
              register_fxn_descriptor_inner!([<$fxn_name M3x2>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name M3x2>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]              
            (Value::$matrix_kind(Matrix::RowDVector(input)), [Value::Index(ix)]) => {
              register_fxn_descriptor_inner!([<$fxn_name RD>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name RD>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "vectord"))]              
            (Value::$matrix_kind(Matrix::DVector(input)),    [Value::Index(ix)]) => {
              register_fxn_descriptor_inner!([<$fxn_name VD>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name VD>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd"))]              
            (Value::$matrix_kind(Matrix::DMatrix(input)),    [Value::Index(ix)]) => {
              register_fxn_descriptor_inner!([<$fxn_name MD>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name MD>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
          )+
        )+
        (src, ix) => Err(MechError2::new(UnhandledFunctionArgumentIxesMono { arg: (src.clone(), ix.to_vec()), fxn_name: stringify!($fxn_name).to_string() }, None).with_compiler_loc()),
      }
    }
  }
}

fn impl_access_scalar_fxn(lhs_value: Value, ixes: Vec<Value>) -> MResult<Box<dyn MechFunction>> {
  impl_access_match_arms!(Access1DS, scalar, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessScalar {}
impl NativeFunctionCompiler for MatrixAccessScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match impl_access_scalar_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { impl_access_scalar_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          (src, ix) => Err(MechError2::new(UnhandledFunctionArgumentIxesMono { arg: (src.clone(), ix.to_vec()), fxn_name: "MatrixAccessScalar".to_string() }, None).with_compiler_loc()),
        }
      }
    }
  }
}

// x[1,2] ---------------------------------------------------------------------

macro_rules! impl_access_scalar_scalar_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            (Value::$matrix_kind(Matrix::RowVector4(input)), [Value::Index(ix1),Value::Index(ix2)]) => {
              register_fxn_descriptor_inner!([<$fxn_name R4>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name R4>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            (Value::$matrix_kind(Matrix::RowVector3(input)), [Value::Index(ix1),Value::Index(ix2)]) => {
              register_fxn_descriptor_inner!([<$fxn_name R3>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name R3>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            (Value::$matrix_kind(Matrix::RowVector2(input)), [Value::Index(ix1),Value::Index(ix2)]) => {
              register_fxn_descriptor_inner!([<$fxn_name R2>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name R2>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            (Value::$matrix_kind(Matrix::Vector4(input)),    [Value::Index(ix1),Value::Index(ix2)]) => {
              register_fxn_descriptor_inner!([<$fxn_name V4>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name V4>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            (Value::$matrix_kind(Matrix::Vector3(input)),    [Value::Index(ix1),Value::Index(ix2)]) => {
              register_fxn_descriptor_inner!([<$fxn_name V3>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name V3>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            (Value::$matrix_kind(Matrix::Vector2(input)),    [Value::Index(ix1),Value::Index(ix2)]) => {
              register_fxn_descriptor_inner!([<$fxn_name V2>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name V2>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (Value::$matrix_kind(Matrix::Matrix4(input)),    [Value::Index(ix1),Value::Index(ix2)]) => {
              register_fxn_descriptor_inner!([<$fxn_name M4>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name M4>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::$matrix_kind(Matrix::Matrix3(input)),    [Value::Index(ix1),Value::Index(ix2)]) => {
              register_fxn_descriptor_inner!([<$fxn_name M3>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name M3>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::$matrix_kind(Matrix::Matrix2(input)),    [Value::Index(ix1),Value::Index(ix2)]) => {
              register_fxn_descriptor_inner!([<$fxn_name M2>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name M2>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(input)),  [Value::Index(ix1),Value::Index(ix2)]) => {
              register_fxn_descriptor_inner!([<$fxn_name M2x3>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name M2x3>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(input)),  [Value::Index(ix1),Value::Index(ix2)]) => {
              register_fxn_descriptor_inner!([<$fxn_name M3x2>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name M3x2>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            (Value::$matrix_kind(Matrix::RowDVector(input)), [Value::Index(ix1),Value::Index(ix2)]) => {
              register_fxn_descriptor_inner!([<$fxn_name RD>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name RD>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            (Value::$matrix_kind(Matrix::DVector(input)),    [Value::Index(ix1),Value::Index(ix2)]) => {
              register_fxn_descriptor_inner!([<$fxn_name VD>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name VD>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::$matrix_kind(Matrix::DMatrix(input)),    [Value::Index(ix1),Value::Index(ix2)]) => {
              register_fxn_descriptor_inner!([<$fxn_name MD>], $target_type, $value_string);
              Ok(Box::new([<$fxn_name MD>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
          )+
        )+
        (src, ix) => Err(MechError2::new(UnhandledFunctionArgumentIxesMono { arg: (src.clone(), ix.to_vec()), fxn_name: stringify!($fxn_name).to_string() }, None).with_compiler_loc()),
      }
    }
  }
}

fn impl_access_scalar_scalar_fxn(lhs_value: Value, ixes: Vec<Value>) -> MResult<Box<dyn MechFunction>> {
  impl_access_match_arms!(Access2DSS, scalar_scalar, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessScalarScalar {}
impl NativeFunctionCompiler for MatrixAccessScalarScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 2 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match impl_access_scalar_scalar_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { impl_access_scalar_scalar_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          (src, ix) => Err(MechError2::new(UnhandledFunctionArgumentIxesMono { arg: (src.clone(), ix.to_vec()), fxn_name: "MatrixAccessScalarScalar".to_string() }, None).with_compiler_loc()),
        }
      }
    }
  }
}

// x[1..3] --------------------------------------------------------------------

macro_rules! impl_access_range_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::RowVector4(input)), [Value::MatrixBool(Matrix::DVector(ix))])     => {
              register_fxn_descriptor_inner!(Access1DVDbR4, $target_type, $value_string);
              Ok(Box::new(Access1DVDbR4{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },    
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::RowVector3(input)), [Value::MatrixBool(Matrix::DVector(ix))])     => {
              register_fxn_descriptor_inner!(Access1DVDbR3, $target_type, $value_string);
              Ok(Box::new(Access1DVDbR3{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },    
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::RowVector2(input)), [Value::MatrixBool(Matrix::DVector(ix))])     => {
              register_fxn_descriptor_inner!(Access1DVDbR2, $target_type, $value_string);
              Ok(Box::new(Access1DVDbR2{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },    
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::RowDVector(input)), [Value::MatrixBool(Matrix::DVector(ix))])     => {
              register_fxn_descriptor_inner!(Access1DVDbRD, $target_type, $value_string);
              Ok(Box::new(Access1DVDbRD{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },   

            // --

            #[cfg(all(feature = $value_string, feature = "vector4", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Vector4(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => {
              register_fxn_descriptor_inner!(Access1DVDbV4, $target_type, $value_string);
              Ok(Box::new(Access1DVDbV4{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },    
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Vector3(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => {
              register_fxn_descriptor_inner!(Access1DVDbV3, $target_type, $value_string);
              Ok(Box::new(Access1DVDbV3{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },    
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Vector2(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => {
              register_fxn_descriptor_inner!(Access1DVDbV2, $target_type, $value_string);
              Ok(Box::new(Access1DVDbV2{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },    
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::DVector(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => {
              register_fxn_descriptor_inner!(Access1DVDbVD, $target_type, $value_string);
              Ok(Box::new(Access1DVDbVD{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },   

            // -- 

            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix4(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => {
              register_fxn_descriptor_inner!(Access1DVDbM4, $target_type, $value_string);
              Ok(Box::new(Access1DVDbM4{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },              
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix3(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => {
              register_fxn_descriptor_inner!(Access1DVDbM3, $target_type, $value_string);
              Ok(Box::new(Access1DVDbM3{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },    
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix2(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => {
              register_fxn_descriptor_inner!(Access1DVDbM2, $target_type, $value_string);
              Ok(Box::new(Access1DVDbM2{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },    
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix1(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => {
              register_fxn_descriptor_inner!(Access1DVDbM1, $target_type, $value_string);
              Ok(Box::new(Access1DVDbM1{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },    
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => {
              register_fxn_descriptor_inner!(Access1DVDbM3x2, $target_type, $value_string);
              Ok(Box::new(Access1DVDbM3x2{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },              
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => {
              register_fxn_descriptor_inner!(Access1DVDbM2x3, $target_type, $value_string);
              Ok(Box::new(Access1DVDbM2x3{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },              
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::DMatrix(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => {
              register_fxn_descriptor_inner!(Access1DVDbMD, $target_type, $value_string);
              Ok(Box::new(Access1DVDbMD{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },   

            // --

            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            (Value::$matrix_kind(Matrix::RowVector4(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
              register_fxn_descriptor_inner!(Access1DVDR4, $target_type, $value_string);
              Ok(Box::new(Access1DVDR4{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },                
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            (Value::$matrix_kind(Matrix::RowVector3(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
              register_fxn_descriptor_inner!(Access1DVDR3, $target_type, $value_string);
              Ok(Box::new(Access1DVDR3{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },    
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            (Value::$matrix_kind(Matrix::RowVector2(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
              register_fxn_descriptor_inner!(Access1DVDR2, $target_type, $value_string);
              Ok(Box::new(Access1DVDR2{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },    
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            (Value::$matrix_kind(Matrix::RowDVector(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
              register_fxn_descriptor_inner!(Access1DVDRD, $target_type, $value_string);
              Ok(Box::new(Access1DVDRD{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },   

            // --

            #[cfg(all(feature = $value_string, feature = "vector4"))]
            (Value::$matrix_kind(Matrix::Vector4(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
              register_fxn_descriptor_inner!(Access1DVDV4, $target_type, $value_string);
              Ok(Box::new(Access1DVDV4{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },                
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            (Value::$matrix_kind(Matrix::Vector3(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
              register_fxn_descriptor_inner!(Access1DVDV3, $target_type, $value_string);
              Ok(Box::new(Access1DVDV3{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },    
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            (Value::$matrix_kind(Matrix::Vector2(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
              register_fxn_descriptor_inner!(Access1DVDV2, $target_type, $value_string);
              Ok(Box::new(Access1DVDV2{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },    
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            (Value::$matrix_kind(Matrix::DVector(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
              register_fxn_descriptor_inner!(Access1DVDVD, $target_type, $value_string);
              Ok(Box::new(Access1DVDVD{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },   

            // --

            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (Value::$matrix_kind(Matrix::Matrix4(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
              register_fxn_descriptor_inner!(Access1DVDM4, $target_type, $value_string);
              Ok(Box::new(Access1DVDM4{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },                
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::$matrix_kind(Matrix::Matrix3(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
              register_fxn_descriptor_inner!(Access1DVDM3, $target_type, $value_string);
              Ok(Box::new(Access1DVDM3{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },    
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::$matrix_kind(Matrix::Matrix2(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
              register_fxn_descriptor_inner!(Access1DVDM2, $target_type, $value_string);
              Ok(Box::new(Access1DVDM2{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },    
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            (Value::$matrix_kind(Matrix::Matrix1(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
              register_fxn_descriptor_inner!(Access1DVDM1, $target_type, $value_string);
              Ok(Box::new(Access1DVDM1{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },    
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(input)), [Value::MatrixIndex(Matrix::DVector(ix))]) => {
              register_fxn_descriptor_inner!(Access1DVDM3x2, $target_type, $value_string);
              Ok(Box::new(Access1DVDM3x2{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },    
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(input)), [Value::MatrixIndex(Matrix::DVector(ix))]) => {
              register_fxn_descriptor_inner!(Access1DVDM2x3, $target_type, $value_string);
              Ok(Box::new(Access1DVDM2x3{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },    
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::$matrix_kind(Matrix::DMatrix(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
              register_fxn_descriptor_inner!(Access1DVDMD, $target_type, $value_string);
              Ok(Box::new(Access1DVDMD{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },   
          )+
        )+
        (src, ix) => Err(MechError2::new(UnhandledFunctionArgumentIxesMono { arg: (src.clone(), ix.to_vec()), fxn_name: stringify!($fxn_name).to_string() }, None).with_compiler_loc()),
      }
    }
  }
}

fn impl_access_range_fxn(lhs_value: Value, ixes: Vec<Value>) -> MResult<Box<dyn MechFunction>> {
  impl_access_match_arms!(Access1DR, range, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessRange {}
impl NativeFunctionCompiler for MatrixAccessRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match impl_access_range_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { impl_access_range_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          (src, ix) => Err(MechError2::new(UnhandledFunctionArgumentIxesMono { arg: (src.clone(), ix.to_vec()), fxn_name: "MatrixAccessRange".to_string() }, None).with_compiler_loc()),
        }
      }
    }
  }
}

// x[1..3,1..3] ---------------------------------------------------------------

macro_rules! access_2d_range_range_vbb {
  ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
    unsafe { 
      let mut sink_rix = 0;
      let mut sink_cix = 0;
      for r in 0..($ix1).len() {
        if ($ix1)[r] == true {
          for c in 0..($ix2).len() {
            if ($ix2)[c] == true {
              ($sink)[(sink_rix, sink_cix)] = ($source)[(r, c)].clone();
              sink_cix += 1;
            }
          }
          sink_cix = 0;
          sink_rix += 1;
        }
      }
    }
  };}

macro_rules! access_2d_range_range_vuu {
  ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
    unsafe { 
      let mut sink_rix = 0;
      let mut sink_cix = 0;
      for r in 0..($ix1).len() {
        let row = ($ix1)[r] - 1;
        for c in 0..($ix2).len() {
          let col = ($ix2)[c] - 1;
          ($sink)[(sink_rix, sink_cix)] = ($source)[(row, col)].clone();
          sink_cix += 1;
        }
        sink_cix = 0;
        sink_rix += 1;
      }
    }
  };}

macro_rules! access_2d_range_range_vub {
  ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
    unsafe { 
      let mut sink_rix = 0;
      let mut sink_cix = 0;
      for r in 0..($ix1).len() {
        let row = ($ix1)[r] - 1;
        for c in 0..($ix2).len() {
          if ($ix2)[c] == true {
            ($sink)[(sink_rix, sink_cix)] = ($source)[(row, c)].clone();
            sink_cix += 1;
          }
        }
        sink_cix = 0;
        sink_rix += 1;
      }
    }
  };}

macro_rules! access_2d_range_range_vbu {
  ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
    unsafe { 
      let mut sink_rix = 0;
      let mut sink_cix = 0;
      for r in 0..($ix1).len() {
        if ($ix1)[r] == true {
          for c in 0..($ix2).len() {
            let col = ($ix2)[c] - 1;
            ($sink)[(sink_rix, sink_cix)] = ($source)[(r, col)].clone();
            sink_cix += 1;
          }
          sink_cix = 0;
          sink_rix += 1;
        }
      }
    }
  };}

macro_rules! impl_access_range_range_arms {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste!{
      match $arg {
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(source)),[Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => {
          register_assign_srr2!([<$fxn_name VUU>], $value_kind, $value_string, DMatrix, $shape, DVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VUU>] { source: source.clone(), ixes: (ix1.clone(), ix2.clone()), sink: Ref::new(DMatrix::from_element(ix1.borrow().len(), ix2.borrow().len(), $value_kind::default())), _marker: std::marker::PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "row_vectord", feature = "logical_indexing"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(source)),[Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => {
          let rows = ix1.borrow().iter().filter(|x| **x).count();
          let cols = ix2.borrow().iter().filter(|x| **x).count();
          match (cols, rows) {
            #[cfg(feature = "matrixd")]
            (1, 1) => {
              register_assign_srr_b2!([<$fxn_name VBB>], $value_kind, $value_string, DMatrix, $shape, DVector, DVector);
              box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { source: source.clone(), ixes: (ix1.clone(), ix2.clone()), sink: Ref::new(DMatrix::from_element(1, 1, $value_kind::default())), _marker: std::marker::PhantomData::default() })))
            },
            #[cfg(feature = "vectord")]
            (1, _) => {
              register_assign_srr_b2!([<$fxn_name VBB>], $value_kind, $value_string, DVector, $shape, DVector, DVector);
              box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { source: source.clone(), ixes: (ix1.clone(), ix2.clone()), sink: Ref::new(DVector::from_element(rows, $value_kind::default())), _marker: std::marker::PhantomData::default() })))
            },
            #[cfg(feature = "row_vectord")]
            (_, 1) => {
              register_assign_srr_b2!([<$fxn_name VBB>], $value_kind, $value_string, RowDVector, $shape, DVector, DVector);
              box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { source: source.clone(), ixes: (ix1.clone(), ix2.clone()), sink: Ref::new(RowDVector::from_element(cols, $value_kind::default())), _marker: std::marker::PhantomData::default() })))
            },
            #[cfg(feature = "matrixd")]
            _ => {
              register_assign_srr_b2!([<$fxn_name VBB>], $value_kind, $value_string, DMatrix, $shape, DVector, DVector);
              box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { source: source.clone(), ixes: (ix1.clone(), ix2.clone()), sink: Ref::new(DMatrix::from_element(rows, cols, $value_kind::default())), _marker: std::marker::PhantomData::default() })))
            },
          }
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "logical_indexing"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(source)),[Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => {
          let cols = ix2.borrow().iter().filter(|x| **x).count();
          let rows = ix1.borrow().len();
          match (cols, rows) {
            #[cfg(feature = "matrixd")]
            (1, 1) => {
              register_assign_srr_ub2!([<$fxn_name VUB>], $value_kind, $value_string, DMatrix, $shape, DVector, DVector);
              box_mech_fxn(Ok(Box::new([<$fxn_name VUB>] { source: source.clone(), ixes: (ix1.clone(), ix2.clone()), sink: Ref::new(DMatrix::from_element(1, 1, $value_kind::default())), _marker: std::marker::PhantomData::default() })))
            },
            #[cfg(feature = "vectord")]
            (1, _) => {
              register_assign_srr_ub2!([<$fxn_name VUB>], $value_kind, $value_string, DVector, $shape, DVector, DVector);
              box_mech_fxn(Ok(Box::new([<$fxn_name VUB>] { source: source.clone(), ixes: (ix1.clone(), ix2.clone()), sink: Ref::new(DVector::from_element(rows, $value_kind::default())), _marker: std::marker::PhantomData::default() })))
            },
            #[cfg(feature = "row_vectord")]
            (_, 1) => {
              register_assign_srr_ub2!([<$fxn_name VUB>], $value_kind, $value_string, RowDVector, $shape, DVector, DVector);
              box_mech_fxn(Ok(Box::new([<$fxn_name VUB>] { source: source.clone(), ixes: (ix1.clone(), ix2.clone()), sink: Ref::new(RowDVector::from_element(cols, $value_kind::default())), _marker: std::marker::PhantomData::default() })))
            },
            #[cfg(feature = "matrixd")]
            _ => {
              register_assign_srr_ub2!([<$fxn_name VUB>], $value_kind, $value_string, DMatrix, $shape, DVector, DVector);
              box_mech_fxn(Ok(Box::new([<$fxn_name VUB>] { source: source.clone(), ixes: (ix1.clone(), ix2.clone()), sink: Ref::new(DMatrix::from_element(rows, cols, $value_kind::default())), _marker: std::marker::PhantomData::default() })))
            },
          }
        },
         #[cfg(all(feature = $value_string, feature = "vectord", feature = "logical_indexing"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(source)),[Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => {
          let cols = ix2.borrow().len();
          let rows = ix1.borrow().iter().filter(|x| **x).count();
          match (cols, rows) {
            #[cfg(feature = "matrixd")]
            (1, 1) => {
              register_assign_srr_bu2!([<$fxn_name VBU>], $value_kind, $value_string, DMatrix, $shape, DVector, DVector);
              box_mech_fxn(Ok(Box::new([<$fxn_name VBU>] { source: source.clone(), ixes: (ix1.clone(), ix2.clone()), sink: Ref::new(DMatrix::from_element(1, 1, $value_kind::default())), _marker: std::marker::PhantomData::default() })))
            },
            #[cfg(feature = "vectord")]
            (1, _) => {
              register_assign_srr_bu2!([<$fxn_name VBU>], $value_kind, $value_string, DVector, $shape, DVector, DVector);
              box_mech_fxn(Ok(Box::new([<$fxn_name VBU>] { source: source.clone(), ixes: (ix1.clone(), ix2.clone()), sink: Ref::new(DVector::from_element(rows, $value_kind::default())), _marker: std::marker::PhantomData::default() })))
            },
            #[cfg(feature = "row_vectord")]
            (_, 1) => {
              register_assign_srr_bu2!([<$fxn_name VBU>], $value_kind, $value_string, RowDVector, $shape, DVector, DVector);
              box_mech_fxn(Ok(Box::new([<$fxn_name VBU>] { source: source.clone(), ixes: (ix1.clone(), ix2.clone()), sink: Ref::new(RowDVector::from_element(cols, $value_kind::default())), _marker: std::marker::PhantomData::default() })))
            },
            #[cfg(feature = "matrixd")]
            _ => {
              register_assign_srr_bu2!([<$fxn_name VBU>], $value_kind, $value_string, DMatrix, $shape, DVector, DVector);
              box_mech_fxn(Ok(Box::new([<$fxn_name VBU>] { source: source.clone(), ixes: (ix1.clone(), ix2.clone()), sink: Ref::new(DMatrix::from_element(rows, cols, $value_kind::default())), _marker: std::marker::PhantomData::default() })))
            },
          }
        }
        (src, ix) => Err(MechError2::new(
          UnhandledFunctionArgumentIxesMono{arg: (src.clone(), ix.to_vec()), fxn_name: stringify!($fxn_name).to_string()}, 
          None).with_compiler_loc()
        ),
      }
    }
  }
}

impl_range_range_fxn_v!(Access2DRRVBB, access_2d_range_range_vbb, bool,  bool);
impl_range_range_fxn_v!(Access2DRRVBU, access_2d_range_range_vbu, bool,  usize);
impl_range_range_fxn_v!(Access2DRRVUU, access_2d_range_range_vuu, usize, usize);
impl_range_range_fxn_v!(Access2DRRVUB, access_2d_range_range_vub, usize, bool);

fn matrix_access_range_range_fxn(source: Value, ixes: Vec<Value>) -> MResult<Box<dyn MechFunction>> {
  let arg = (source.clone(), ixes.as_slice());
               impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, u8,   "u8")
  .or_else(|_| impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, u16,  "u16"))
  .or_else(|_| impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, u32,  "u32"))
  .or_else(|_| impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, u64,  "u64"))
  .or_else(|_| impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, u128, "u128"))
  .or_else(|_| impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, i8,   "i8"))
  .or_else(|_| impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, i16,  "i16"))
  .or_else(|_| impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, i32,  "i32"))
  .or_else(|_| impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, i64,  "i64"))
  .or_else(|_| impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, i128, "i128"))
  .or_else(|_| impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, F32,  "f32"))
  .or_else(|_| impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, F64,  "f64"))
  .or_else(|_| impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, R64,  "rational"))
  .or_else(|_| impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, C64,  "complex"))
  .or_else(|_| impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, bool, "bool"))
  .or_else(|_| impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, String, "string"))
  .map_err(|_| MechError2::new(UnhandledFunctionArgumentIxesMono{
      arg: (source, ixes), fxn_name: "MatrixAccessRangeRange".to_string()
    }, None).with_compiler_loc())
}
    
pub struct MatrixAccessRangeRange {}
impl NativeFunctionCompiler for MatrixAccessRangeRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let source: Value = arguments[0].clone();
    let ixes = arguments.clone().split_off(1);
    match matrix_access_range_range_fxn(source.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match source {
          Value::MutableReference(source) => { matrix_access_range_range_fxn(source.borrow().clone(), ixes.clone()) },
          _ => Err(MechError2::new(
            UnhandledFunctionArgumentIxesMono{arg: (source.clone(), ixes.clone()), fxn_name: "MatrixAccessRangeRange".to_string()}, 
            None).with_compiler_loc()
          ),
        }
      }
    }
  }
}

// x[:] -----------------------------------------------------------------------

macro_rules! impl_access_all_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
            $(
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (Value::$matrix_kind(Matrix::Matrix4(input)),    [Value::IndexAll]) => {
              register_fxn_descriptor_inner!(Access1DAM4, $target_type, $value_string);
              Ok(Box::new(Access1DAM4  {source: input.clone(), ixes: Ref::new(Value::IndexAll), out: Ref::new(DVector::from_element(input.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::$matrix_kind(Matrix::Matrix3(input)),    [Value::IndexAll]) => {
              register_fxn_descriptor_inner!(Access1DAM3, $target_type, $value_string);
              Ok(Box::new(Access1DAM3  {source: input.clone(), ixes: Ref::new(Value::IndexAll), out: Ref::new(DVector::from_element(input.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::$matrix_kind(Matrix::Matrix2(input)),    [Value::IndexAll]) => {
              register_fxn_descriptor_inner!(Access1DAM2, $target_type, $value_string);
              Ok(Box::new(Access1DAM2  {source: input.clone(), ixes: Ref::new(Value::IndexAll), out: Ref::new(DVector::from_element(input.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(input)),  [Value::IndexAll]) => {
              register_fxn_descriptor_inner!(Access1DAM3x2, $target_type, $value_string);
              Ok(Box::new(Access1DAM3x2{source: input.clone(), ixes: Ref::new(Value::IndexAll), out: Ref::new(DVector::from_element(input.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(input)),  [Value::IndexAll]) => {
              register_fxn_descriptor_inner!(Access1DAM2x3, $target_type, $value_string);
              Ok(Box::new(Access1DAM2x3{source: input.clone(), ixes: Ref::new(Value::IndexAll), out: Ref::new(DVector::from_element(input.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::$matrix_kind(Matrix::DMatrix(input)),    [Value::IndexAll]) => {
              register_fxn_descriptor_inner!(Access1DAMD, $target_type, $value_string);
              Ok(Box::new(Access1DAMD  {source: input.clone(), ixes: Ref::new(Value::IndexAll), out: Ref::new(DVector::from_element(input.borrow().len(),$default)) }))
            },
          )+
        )+
        (src, ix) => Err(MechError2::new(
          UnhandledFunctionArgumentIxesMono{arg: (src.clone(), ix.to_vec()), fxn_name: stringify!($fxn_name).to_string()}, 
          None).with_compiler_loc()
        ),
      }
    }
  }
}

fn impl_access_all_fxn(lhs_value: Value, ixes: Vec<Value>) -> MResult<Box<dyn MechFunction>> {
  impl_access_match_arms!(Access1DA, all, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessAll {}
impl NativeFunctionCompiler for MatrixAccessAll {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match impl_access_all_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { impl_access_all_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          (src, ix) => Err(MechError2::new(UnhandledFunctionArgumentIxesMono{arg: (src.clone(), ix.to_vec()), fxn_name: "MatrixAccessAll".to_string()}, None).with_compiler_loc()),
        }
      }
    }
  }
}

// x[:,2] ---------------------------------------------------------------------

macro_rules! impl_access_all_scalar_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
            $(
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (Value::$matrix_kind(Matrix::Matrix4(input)),    [Value::IndexAll,Value::Index(ix)]) => {
              register_fxn_descriptor_inner!(Access2DASM4, $target_type, $value_string);
              Ok(Box::new(Access2DASM4  {source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(input.borrow().nrows(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::$matrix_kind(Matrix::Matrix3(input)),    [Value::IndexAll,Value::Index(ix)]) => {
              register_fxn_descriptor_inner!(Access2DASM3, $target_type, $value_string);
              Ok(Box::new(Access2DASM3  {source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(input.borrow().nrows(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::$matrix_kind(Matrix::Matrix2(input)),    [Value::IndexAll,Value::Index(ix)]) => {
              register_fxn_descriptor_inner!(Access2DASM2, $target_type, $value_string);
              Ok(Box::new(Access2DASM2  {source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(input.borrow().nrows(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(input)),  [Value::IndexAll,Value::Index(ix)]) => {
              register_fxn_descriptor_inner!(Access2DASM2x3, $target_type, $value_string);
              Ok(Box::new(Access2DASM2x3{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(input.borrow().nrows(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(input)),  [Value::IndexAll,Value::Index(ix)]) => {
              register_fxn_descriptor_inner!(Access2DASM3x2, $target_type, $value_string);
              Ok(Box::new(Access2DASM3x2{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(input.borrow().nrows(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::$matrix_kind(Matrix::DMatrix(input)),    [Value::IndexAll,Value::Index(ix)]) => {
              register_fxn_descriptor_inner!(Access2DASMD, $target_type, $value_string);
              Ok(Box::new(Access2DASMD  {source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(input.borrow().nrows(),$default)) }))
            },
          )+
        )+
        (src, ix) => Err(MechError2::new(
          UnhandledFunctionArgumentIxesMono{arg: (src.clone(), ix.to_vec()), fxn_name: stringify!($fxn_name).to_string()}, 
          None).with_compiler_loc()
        ),
      }
    }
  }
}

fn impl_access_all_scalar_fxn(lhs_value: Value, ixes: Vec<Value>) -> MResult<Box<dyn MechFunction>> {
  impl_access_match_arms!(Access2DAS, all_scalar, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessAllScalar {}
impl NativeFunctionCompiler for MatrixAccessAllScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 2 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match impl_access_all_scalar_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { impl_access_all_scalar_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          (src, ix) => Err(MechError2::new(UnhandledFunctionArgumentIxesMono{arg: (src.clone(), ix.to_vec()), fxn_name: "MatrixAccessAllScalar".to_string()}, None).with_compiler_loc()),
        }
      }
    }
  }
}

// x[2,:] ---------------------------------------------------------------------

macro_rules! impl_access_scalar_all_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
            $(
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "row_vector4"))]
            (Value::$matrix_kind(Matrix::Matrix4(input)), [Value::Index(ix),Value::IndexAll]) => {
              register_fxn_descriptor_inner!(Access2DSAM4, $target_type, $value_string);
              Ok(Box::new(Access2DSAM4{source: input.clone(), ixes: ix.clone(), out: Ref::new(RowVector4::from_element($default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "row_vector3"))]
            (Value::$matrix_kind(Matrix::Matrix3(input)), [Value::Index(ix),Value::IndexAll]) => {
              register_fxn_descriptor_inner!(Access2DSAM3, $target_type, $value_string);
              Ok(Box::new(Access2DSAM3{source: input.clone(), ixes: ix.clone(), out: Ref::new(RowVector3::from_element($default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "row_vector2"))]
            (Value::$matrix_kind(Matrix::Matrix2(input)), [Value::Index(ix),Value::IndexAll]) => {
              register_fxn_descriptor_inner!(Access2DSAM2, $target_type, $value_string);
              Ok(Box::new(Access2DSAM2{source: input.clone(), ixes: ix.clone(), out: Ref::new(RowVector2::from_element($default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "matrix1"))]
            (Value::$matrix_kind(Matrix::Matrix1(input)), [Value::Index(ix),Value::IndexAll]) => {
              register_fxn_descriptor_inner!(Access2DSAM1, $target_type, $value_string);
              Ok(Box::new(Access2DSAM1{source: input.clone(), ixes: ix.clone(), out: Ref::new(Matrix1::from_element($default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "row_vector2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(input)), [Value::Index(ix),Value::IndexAll]) => {
              register_fxn_descriptor_inner!(Access2DSAM3x2, $target_type, $value_string);
              Ok(Box::new(Access2DSAM3x2{source: input.clone(), ixes: ix.clone(), out: Ref::new(RowVector2::from_element($default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "row_vector3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(input)), [Value::Index(ix),Value::IndexAll]) => {
              register_fxn_descriptor_inner!(Access2DSAM2x3, $target_type, $value_string);
              Ok(Box::new(Access2DSAM2x3{source: input.clone(), ixes: ix.clone(), out: Ref::new(RowVector3::from_element($default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "row_vectord"))]
            (Value::$matrix_kind(Matrix::DMatrix(input)), [Value::Index(ix),Value::IndexAll]) => {
              register_fxn_descriptor_inner!(Access2DSAMD, $target_type, $value_string);
              Ok(Box::new(Access2DSAMD{source: input.clone(), ixes: ix.clone(), out: Ref::new(RowDVector::from_element(input.borrow().ncols(),$default)) }))
            },
          )+
        )+
        (src, ix) => Err(MechError2::new(
          UnhandledFunctionArgumentIxesMono{arg: (src.clone(), ix.to_vec()), fxn_name: stringify!($fxn_name).to_string()}, 
          None).with_compiler_loc()
        ),
      }
    }
  }
}

fn impl_access_scalar_all_fxn(lhs_value: Value, ixes: Vec<Value>) -> MResult<Box<dyn MechFunction>> {
 impl_access_match_arms!(Access2DSA, scalar_all, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessScalarAll {}
impl NativeFunctionCompiler for MatrixAccessScalarAll {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 2 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match impl_access_scalar_all_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { impl_access_scalar_all_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError2::new(UnhandledFunctionArgumentIxesMono{arg: x.clone(), fxn_name: "MatrixAccessScalarAll".to_string()}, None).with_compiler_loc()),
        }
      }
    }
  }
}

// x[:,1..3] ---------------------------------------------------------------------

macro_rules! assign_2d_all_range_v {
  ($source:expr, $ix:expr, $sink:expr) => {
    {
      let mut sink_col_ix = 0;
      for i in 0..(*$ix).len() {
        let col_ix = $ix[i] - 1;
        let mut sink_col = ($sink).column_mut(sink_col_ix);
        let src_col = ($source).column(col_ix);
        for (dst, src) in sink_col.iter_mut().zip(src_col.iter()) {
          *dst = src.clone();
        }
        sink_col_ix += 1;
      }
    }
  };}

macro_rules! assign_2d_all_range_vb {
  ($source:expr, $ix:expr, $sink:expr) => {
    {
      let mut sink_col_ix = 0;
      for i in 0..(*$source).ncols() {
        if $ix[i] == true {
          let mut sink_col = ($sink).column_mut(sink_col_ix);
          let src_col = ($source).column(i);
          for (dst, src) in sink_col.iter_mut().zip(src_col.iter()) {
            *dst = src.clone();
          }
          sink_col_ix += 1;
        }
      }
    }
  };}

macro_rules! impl_access_all_range_arms {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste!{
      match $arg {
        // All Vector
        #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(source)), [Value::IndexAll, Value::MatrixIndex(Matrix::DVector(ix))]) if source.borrow().nrows() == 1 => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, RowDVector, $shape, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>]{source: source.clone(), ixes: ix.clone(), sink: Ref::new(RowDVector::from_element(ix.borrow().len(), $value_kind::default())), _marker: std::marker::PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(source)), [Value::IndexAll, Value::MatrixIndex(Matrix::DVector(ix))]) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, DMatrix, $shape, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>]{source: source.clone(), ixes: ix.clone(), sink: Ref::new(DMatrix::from_element(source.borrow().nrows(), ix.borrow().len(), $value_kind::default())), _marker: std::marker::PhantomData::default() })))
        },
        // All Bool Vector
        #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(source)), [Value::IndexAll, Value::MatrixBool(Matrix::DVector(ix))]) if source.borrow().nrows() == 1 => {
          let cols = ix.borrow().iter().filter(|&&b| b).count();
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, RowDVector, $shape, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>]{source: source.clone(), ixes: ix.clone(), sink: Ref::new(RowDVector::from_element(cols, $value_kind::default())), _marker: std::marker::PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "logical_indexing", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(source)), [Value::IndexAll, Value::MatrixBool(Matrix::DVector(ix))]) if ix.borrow().iter().filter(|&&b| b).count() == 1 && source.borrow().nrows() != 1 => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, DVector, $shape, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>]{source: source.clone(), ixes: ix.clone(), sink: Ref::new(DVector::from_element(source.borrow().nrows(), $value_kind::default())), _marker: std::marker::PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "logical_indexing", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(source)), [Value::IndexAll, Value::MatrixBool(Matrix::DVector(ix))]) => {
          let cols = ix.borrow().iter().filter(|&&b| b).count();
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, DMatrix, $shape, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>]{source: source.clone(), ixes: ix.clone(), sink: Ref::new(DMatrix::from_element(source.borrow().nrows(), cols, $value_kind::default())), _marker: std::marker::PhantomData::default() })))
        },
        (sink, ix) => {
          Err(MechError2::new(
            UnhandledFunctionArgumentIxesMono{arg: (sink.clone(), ix.to_vec()), fxn_name: stringify!($fxn_name).to_string()}, 
            None).with_compiler_loc()
          )
        }
      }
    }
  }
}

impl_all_fxn_v!(Access2DARV,  assign_2d_all_range_v,  usize);
impl_all_fxn_v!(Access2DARVB, assign_2d_all_range_vb, bool);

fn matrix_access_all_range_fxn(source: Value, ixes: Vec<Value>) -> MResult<Box<dyn MechFunction>> {
  let arg = (source.clone(), ixes.as_slice());
               impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, u8,   "u8")
  .or_else(|_| impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, u16,  "u16"))
  .or_else(|_| impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, u32,  "u32"))
  .or_else(|_| impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, u64,  "u64"))
  .or_else(|_| impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, u128, "u128"))
  .or_else(|_| impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, i8,   "i8"))
  .or_else(|_| impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, i16,  "i16"))
  .or_else(|_| impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, i32,  "i32"))
  .or_else(|_| impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, i64,  "i64"))
  .or_else(|_| impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, i128, "i128"))
  .or_else(|_| impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, F32,  "f32"))
  .or_else(|_| impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, F64,  "f64"))
  .or_else(|_| impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, R64,  "rational"))
  .or_else(|_| impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, C64,  "complex"))
  .or_else(|_| impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, bool, "bool"))
  .or_else(|_| impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, String, "string"))
  .map_err(|_| MechError2::new(UnhandledFunctionArgumentIxesMono{
      arg: (source, ixes), fxn_name: "MatrixAccessAllRange".to_string()
    }, None).with_compiler_loc())
}
    
pub struct MatrixAccessAllRange {}
impl NativeFunctionCompiler for MatrixAccessAllRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let source: Value = arguments[0].clone();
    let ixes = arguments.clone().split_off(1);
    match matrix_access_all_range_fxn(source.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match source {
          Value::MutableReference(source) => { matrix_access_all_range_fxn(source.borrow().clone(), ixes.clone()) },
          x => Err(MechError2::new(
            UnhandledFunctionArgumentIxesMono{arg: (x, ixes.to_vec()), fxn_name: "MatrixAccessAllRange".to_string()}, 
            None).with_compiler_loc()
          ),
        }
      }
    }
  }
}

// x[1..3,:] ---------------------------------------------------------------------

macro_rules! impl_access_range_all_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            // Vector All
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (Value::$matrix_kind(Matrix::Matrix4(input)), [Value::MatrixIndex(Matrix::DVector(ix)), Value::IndexAll]) => {
              register_fxn_descriptor_inner!(Access2DVDAM4, $target_type, $value_string);
              Ok(Box::new(Access2DVDAM4{source: input.clone(), ixes: ix.clone(), out: Ref::new(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::$matrix_kind(Matrix::Matrix3(input)), [Value::MatrixIndex(Matrix::DVector(ix)), Value::IndexAll]) => {
              register_fxn_descriptor_inner!(Access2DVDAM3, $target_type, $value_string);
              Ok(Box::new(Access2DVDAM3{source: input.clone(), ixes: ix.clone(), out: Ref::new(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::$matrix_kind(Matrix::Matrix2(input)), [Value::MatrixIndex(Matrix::DVector(ix)), Value::IndexAll]) => {
              register_fxn_descriptor_inner!(Access2DVDAM2, $target_type, $value_string);
              Ok(Box::new(Access2DVDAM2{source: input.clone(), ixes: ix.clone(), out: Ref::new(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(input)), [Value::MatrixIndex(Matrix::DVector(ix)), Value::IndexAll]) => {
              register_fxn_descriptor_inner!(Access2DVDAM3x2, $target_type, $value_string);
              Ok(Box::new(Access2DVDAM3x2{source: input.clone(), ixes: ix.clone(), out: Ref::new(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(input)), [Value::MatrixIndex(Matrix::DVector(ix)), Value::IndexAll]) => {
              register_fxn_descriptor_inner!(Access2DVDAM2x3, $target_type, $value_string);
              Ok(Box::new(Access2DVDAM2x3{source: input.clone(), ixes: ix.clone(), out: Ref::new(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::$matrix_kind(Matrix::DMatrix(input)), [Value::MatrixIndex(Matrix::DVector(ix)), Value::IndexAll]) => {
              register_fxn_descriptor_inner!(Access2DVDAMD, $target_type, $value_string);
              Ok(Box::new(Access2DVDAMD{source: input.clone(), ixes: ix.clone(), out: Ref::new(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) }))
            },
            // Bool Vector All
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix4(input)), [Value::MatrixBool(Matrix::DVector(ix)), Value::IndexAll]) => {
              register_fxn_descriptor_inner!(Access2DVDbAM4, $target_type, $value_string);
              Ok(Box::new(Access2DVDbAM4{source: input.clone(), ixes: ix.clone(), out: Ref::new(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix3(input)), [Value::MatrixBool(Matrix::DVector(ix)), Value::IndexAll]) => {
              register_fxn_descriptor_inner!(Access2DVDbAM3, $target_type, $value_string);
              Ok(Box::new(Access2DVDbAM3{source: input.clone(), ixes: ix.clone(), out: Ref::new(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix2(input)), [Value::MatrixBool(Matrix::DVector(ix)), Value::IndexAll]) => {
              register_fxn_descriptor_inner!(Access2DVDbAM2, $target_type, $value_string);
              Ok(Box::new(Access2DVDbAM2{source: input.clone(), ixes: ix.clone(), out: Ref::new(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(input)), [Value::MatrixBool(Matrix::DVector(ix)), Value::IndexAll]) => {
              register_fxn_descriptor_inner!(Access2DVDbAM3x2, $target_type, $value_string);
              Ok(Box::new(Access2DVDbAM3x2{source: input.clone(), ixes: ix.clone(), out: Ref::new(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(input)), [Value::MatrixBool(Matrix::DVector(ix)), Value::IndexAll]) => {
              register_fxn_descriptor_inner!(Access2DVDbAM2x3, $target_type, $value_string);
              Ok(Box::new(Access2DVDbAM2x3{source: input.clone(), ixes: ix.clone(), out: Ref::new(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::DMatrix(input)), [Value::MatrixBool(Matrix::DVector(ix)), Value::IndexAll]) => {
              register_fxn_descriptor_inner!(Access2DVDbAMD, $target_type, $value_string);
              Ok(Box::new(Access2DVDbAMD{source: input.clone(), ixes: ix.clone(), out: Ref::new(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) }))
            },
          )+
        )+
        (src, ixes) => Err(MechError2::new(UnhandledFunctionArgumentIxesMono{arg: (src, ixes.to_vec()), fxn_name: "MatrixAccessRangeAll".to_string()}, None).with_compiler_loc()),
      }
    }
  }
}

fn impl_access_range_all_fxn(lhs_value: Value, ixes: Vec<Value>) -> MResult<Box<dyn MechFunction>> {
  impl_access_match_arms!(Access2DRA, range_all, (lhs_value, ixes.as_slice()))
}

  pub struct MatrixAccessRangeAll {}
impl NativeFunctionCompiler for MatrixAccessRangeAll {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 2 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match impl_access_range_all_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat.clone(),ixes.clone()) {
          (Value::MutableReference(lhs),rhs_value) => { impl_access_range_all_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          (src, ixes) => Err(MechError2::new(UnhandledFunctionArgumentIxesMono{arg: (src, ixes.to_vec()), fxn_name: "MatrixAccessRangeAll".to_string()}, None).with_compiler_loc()),
        }
      }
    }
  }
}

// x[1..3,2] ---------------------------------------------------------------------

macro_rules! impl_access_range_scalar_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
            $(
            // Vector Scalar
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (Value::$matrix_kind(Matrix::Matrix4(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::Index(ix2)]) => {
              register_fxn_descriptor_inner!(Access2DVDSM4, $target_type, $value_string);
              Ok(Box::new(Access2DVDSM4{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(DVector::from_element(ix1.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::$matrix_kind(Matrix::Matrix3(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::Index(ix2)]) => {
              register_fxn_descriptor_inner!(Access2DVDSM3, $target_type, $value_string);
              Ok(Box::new(Access2DVDSM3{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(DVector::from_element(ix1.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::$matrix_kind(Matrix::Matrix2(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::Index(ix2)]) => {
              register_fxn_descriptor_inner!(Access2DVDSM2, $target_type, $value_string);
              Ok(Box::new(Access2DVDSM2{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(DVector::from_element(ix1.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(input)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::Index(ix2)]) => {
              register_fxn_descriptor_inner!(Access2DVDSM2x3, $target_type, $value_string);
              Ok(Box::new(Access2DVDSM2x3{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(DVector::from_element(ix1.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(input)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::Index(ix2)]) => {
              register_fxn_descriptor_inner!(Access2DVDSM3x2, $target_type, $value_string);
              Ok(Box::new(Access2DVDSM3x2{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(DVector::from_element(ix1.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::$matrix_kind(Matrix::DMatrix(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::Index(ix2)]) => {
              register_fxn_descriptor_inner!(Access2DVDSMD, $target_type, $value_string);
              Ok(Box::new(Access2DVDSMD{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(DVector::from_element(ix1.borrow().len(),$default)) }))
            },
            // Bool Vector Scalar
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix4(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)]) => {
              register_fxn_descriptor_inner!(Access2DVDbSM4, $target_type, $value_string);
              Ok(Box::new(Access2DVDbSM4{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(DVector::from_element(ix1.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix3(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)]) => {
              register_fxn_descriptor_inner!(Access2DVDbSM3, $target_type, $value_string);
              Ok(Box::new(Access2DVDbSM3{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(DVector::from_element(ix1.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix2(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)]) => {
              register_fxn_descriptor_inner!(Access2DVDbSM2, $target_type, $value_string);
              Ok(Box::new(Access2DVDbSM2{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(DVector::from_element(ix1.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(input)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)]) => {
              register_fxn_descriptor_inner!(Access2DVDbSM2x3, $target_type, $value_string);
              Ok(Box::new(Access2DVDbSM2x3{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(DVector::from_element(ix1.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(input)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)]) => {
              register_fxn_descriptor_inner!(Access2DVDbSM3x2, $target_type, $value_string);
              Ok(Box::new(Access2DVDbSM3x2{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(DVector::from_element(ix1.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::DMatrix(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)]) => {
              register_fxn_descriptor_inner!(Access2DVDbSMD, $target_type, $value_string);
              Ok(Box::new(Access2DVDbSMD{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(DVector::from_element(ix1.borrow().len(),$default)) }))
            },)+
        )+
        (src, ixes) => Err(MechError2::new(UnhandledFunctionArgumentIxesMono{arg: (src, ixes.to_vec()), fxn_name: "MatrixAccessRangeRange".to_string()}, None).with_compiler_loc()),
      }
    }
  }
}

fn impl_access_range_scalar_fxn(lhs_value: Value, ixes: Vec<Value>) -> MResult<Box<dyn MechFunction>> {
  impl_access_match_arms!(Access2DRS, range_scalar, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessRangeScalar {}
impl NativeFunctionCompiler for MatrixAccessRangeScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 2 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match impl_access_range_scalar_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { impl_access_range_scalar_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          (src,ixs) => Err(MechError2::new(UnhandledFunctionArgumentIxesMono { arg: (src, ixs), fxn_name: "MatrixAccessRangeScalar".to_string() }, None).with_compiler_loc()),
        }
      }
    }
  }
}

// x[2,1..3] ---------------------------------------------------------------------

macro_rules! impl_access_scalar_range_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            // Scalar Vector 
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (Value::$matrix_kind(Matrix::Matrix4(input)),   [Value::Index(ix1), Value::MatrixIndex(Matrix::DVector(ix2))]) => {
              register_fxn_descriptor_inner!(Access2DSVDM4, $target_type, $value_string);
              Ok(Box::new(Access2DSVDM4{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(RowDVector::from_element(ix2.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::$matrix_kind(Matrix::Matrix3(input)),   [Value::Index(ix1), Value::MatrixIndex(Matrix::DVector(ix2))]) => {
              register_fxn_descriptor_inner!(Access2DSVDM3, $target_type, $value_string);
              Ok(Box::new(Access2DSVDM3{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(RowDVector::from_element(ix2.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::$matrix_kind(Matrix::Matrix2(input)),   [Value::Index(ix1), Value::MatrixIndex(Matrix::DVector(ix2))]) => {
              register_fxn_descriptor_inner!(Access2DSVDM2, $target_type, $value_string);
              Ok(Box::new(Access2DSVDM2{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(RowDVector::from_element(ix2.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(input)), [Value::Index(ix1), Value::MatrixIndex(Matrix::DVector(ix2))]) => {
              register_fxn_descriptor_inner!(Access2DSVDM3x2, $target_type, $value_string);
              Ok(Box::new(Access2DSVDM3x2{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(RowDVector::from_element(ix2.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(input)), [Value::Index(ix1), Value::MatrixIndex(Matrix::DVector(ix2))]) => {
              register_fxn_descriptor_inner!(Access2DSVDM2x3, $target_type, $value_string);
              Ok(Box::new(Access2DSVDM2x3{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(RowDVector::from_element(ix2.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::$matrix_kind(Matrix::DMatrix(input)),   [Value::Index(ix1), Value::MatrixIndex(Matrix::DVector(ix2))]) => {
              register_fxn_descriptor_inner!(Access2DSVDMD, $target_type, $value_string);
              Ok(Box::new(Access2DSVDMD{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(RowDVector::from_element(ix2.borrow().len(),$default)) }))
            },
            // Bool Scalar Vector
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix4(input)),   [Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))]) => {
              register_fxn_descriptor_inner!(Access2DSVDbM4, $target_type, $value_string);
              Ok(Box::new(Access2DSVDbM4{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(RowDVector::from_element(ix2.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix3(input)),   [Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))]) => {
              register_fxn_descriptor_inner!(Access2DSVDbM3, $target_type, $value_string);
              Ok(Box::new(Access2DSVDbM3{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(RowDVector::from_element(ix2.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix2(input)),   [Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))]) => {
              register_fxn_descriptor_inner!(Access2DSVDbM2, $target_type, $value_string);
              Ok(Box::new(Access2DSVDbM2{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(RowDVector::from_element(ix2.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(input)), [Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))]) => {
              register_fxn_descriptor_inner!(Access2DSVDbM3x2, $target_type, $value_string);
              Ok(Box::new(Access2DSVDbM3x2{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(RowDVector::from_element(ix2.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(input)), [Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))]) => {
              register_fxn_descriptor_inner!(Access2DSVDbM2x3, $target_type, $value_string);
              Ok(Box::new(Access2DSVDbM2x3{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(RowDVector::from_element(ix2.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::DMatrix(input)),   [Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))]) => {
              register_fxn_descriptor_inner!(Access2DSVDbMD, $target_type, $value_string);
              Ok(Box::new(Access2DSVDbMD{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(RowDVector::from_element(ix2.borrow().len(),$default)) }))
            },)+
        )+
        (src,ix) => Err(MechError2::new(UnhandledFunctionArgumentIxesMono{ arg: (src.clone(), ix.to_vec()), fxn_name: stringify!($fxn_name).to_string() }, None).with_compiler_loc()),
      }
    }
  }
}

fn impl_access_scalar_range_fxn(lhs_value: Value, ixes: Vec<Value>) -> MResult<Box<dyn MechFunction>> {
  impl_access_match_arms!(Access2DSR, scalar_range, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessScalarRange {}

impl NativeFunctionCompiler for MatrixAccessScalarRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 2 {
      return Err(MechError2::new(IncorrectNumberOfArguments{expected: 1, found: arguments.len()}, None).with_compiler_loc());
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match impl_access_scalar_range_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat.clone(),ixes.clone()) {
          (Value::MutableReference(lhs),rhs_value) => { impl_access_scalar_range_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError2::new(UnhandledFunctionArgumentIxesMono{ arg: (mat.clone(), ixes.clone()), fxn_name: "MatrixAccessScalarRange".to_string() }, None).with_compiler_loc()),
        }
      }
    }
  }
}