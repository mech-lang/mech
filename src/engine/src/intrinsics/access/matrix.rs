#[macro_use]
use crate::intrinsics::*;
use nalgebra::{
    Dim, Scalar,
    base::{Matrix as naMatrix, Storage, StorageMut},
};
use std::fmt::Debug;
use std::marker::PhantomData;

// Access ---------------------------------------------------------------------

#[macro_export]
macro_rules! impl_access_fxn_new {
    ($op:tt, $fxn_name:ident, $arg:expr, $value_kind:ident, $value_string:tt) => {{
        let mut res: MResult<_> = Err(MechError::new(
            GenericError {
                msg: "No matching type found".to_string(),
            },
            None,
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

        let &(ref source, ref ixes) = &$arg;
        res.map_err(|_| {
            MechError::new(
                UnhandledFunctionArgumentIxesMono {
                    arg: (source.kind(), ixes.iter().map(|x| x.kind()).collect()),
                    fxn_name: stringify!($fxn_name).to_string(),
                },
                None,
            )
            .with_compiler_loc()
        })
    }};
}

macro_rules! access_1d {
    ($source:expr, $ix:expr, $out:expr) => {
        unsafe { *$out = (*$source).index(*$ix - 1).clone() }
    };
}

macro_rules! access_2d {
    ($source:expr, $ix1:expr, $ix2:expr, $out:expr) => {
        unsafe { *$out = (*$source).index((*$ix1 - 1, *$ix2 - 1)).clone() }
    };
}
macro_rules! access_1d_slice {
    ($source:expr, $ix:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$ix).len() {
                (&mut *$out)[i] = (*$source).index((&(*$ix))[i] - 1).clone();
            }
        }
    };
}

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
        }
    };
}

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
        }
    };
}

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
        }
    };
}

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
        }
    };
}

macro_rules! access_2d_slice {
    ($source:expr, $ix1:expr, $ix2:expr, $out:expr) => {
        unsafe {
            let nrows = (*$ix1).len();
            let ncols = (*$ix2).len();
            let mut out_ix = 0;
            for j in 0..ncols {
                for i in 0..nrows {
                    (&mut (*$out))[out_ix] = (*$source)
                        .index(((&(*$ix1))[i] - 1, (&(*$ix2))[j] - 1))
                        .clone();
                    out_ix += 1;
                }
            }
        }
    };
}

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
        }
    };
}

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
                (*$out).resize_horizontally_mut(j, (&(*$out))[0].clone());
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
        }
    };
}

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
        }
    };
}

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
        }
    };
}

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
        }
    };
}

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
        }
    };
}

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
        }
    };
}

macro_rules! access_col {
    ($source:expr, $ix:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$source).nrows() {
                (&mut (*$out))[i] = (*$source).index((i, *$ix - 1)).clone();
            }
        }
    };
}

macro_rules! access_row {
    ($source:expr, $ix:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$source).ncols() {
                (&mut (*$out))[i] = (*$source).index((*$ix - 1, i)).clone();
            }
        }
    };
}

macro_rules! access_1d_all {
    ($source:expr, $ix:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$source).len() {
                (&mut (*$out))[i] = (*$source).index(i).clone();
            }
        }
    };
}

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
        ConstElem + AsValueKind,
      #[cfg(feature = "compiler")]
      T: CompileConst,
      IxVec: ConstElem + AsNaKind + Debug + AsRef<[$ix]>,
      #[cfg(feature = "compiler")]
      IxVec: CompileConst,
      R1: Dim, C1: Dim, S1: StorageMut<T, R1, C1> + Clone + Debug,
      R2: Dim, C2: Dim, S2: Storage<T, R2, C2> + Clone + Debug,
      naMatrix<T, R1, C1, S1>: ConstElem + Debug + AsNaKind,
      #[cfg(feature = "compiler")]
      naMatrix<T, R1, C1, S1>: CompileConst,
      naMatrix<T, R2, C2, S2>: ConstElem + Debug + AsNaKind,
      #[cfg(feature = "compiler")]
      naMatrix<T, R2, C2, S2>: CompileConst,
    {
      fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
          FunctionArgs::Binary(out, arg1, arg2) => {
            let source: Ref<naMatrix<T, R2, C2, S2>> = arg1.try_function_ref(FunctionArgumentRole::Input(0))?;
            let ixes: Ref<IxVec> = arg2.try_function_ref(FunctionArgumentRole::Input(1))?;
            let sink: Ref<naMatrix<T, R1, C1, S1>> = out.try_function_ref(FunctionArgumentRole::Output)?;
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

      fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
      }
    }
    #[cfg(feature = "compiler")]
    impl<T, R1, C1, S1, R2, C2, S2, IxVec> MechFunctionCompiler for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
    where
      T: CompileConst + ConstElem + AsValueKind,
      IxVec: CompileConst + ConstElem + AsNaKind,
      naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + AsNaKind,
      naMatrix<T, R2, C2, S2>: CompileConst + ConstElem + AsNaKind,
    {
      fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("{}<{}{}{}{}>", stringify!($struct_name), T::as_value_kind(), naMatrix::<T, R1, C1, S1>::as_na_kind(), naMatrix::<T, R2, C2, S2>::as_na_kind(), IxVec::as_na_kind());
        compile_binop!(name, self.sink, self.source, self.ixes, ctx);
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
            T: Debug + Clone + Sync + Send + PartialEq + 'static + ConstElem + AsValueKind,
            #[cfg(feature = "compiler")]
            T: CompileConst,
            Ref<$out_type>: ToValue,
        {
            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                match args {
                    FunctionArgs::Binary(out, arg1, arg2) => {
                        let n: Ref<$arg_type> =
                            arg1.try_function_ref(FunctionArgumentRole::Input(0))?;
                        let k: Ref<$ix_type> =
                            arg2.try_function_ref(FunctionArgumentRole::Input(1))?;
                        let out: Ref<$out_type> =
                            out.try_function_ref(FunctionArgumentRole::Output)?;
                        Ok(Box::new($struct_name {
                            source: n,
                            ixes: k,
                            out,
                        }))
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
        impl<T> MechFunctionImpl for $struct_name<T>
        where
            T: Debug + Clone + Sync + Send + PartialEq + 'static,
            Ref<$out_type>: ToValue,
        {
            fn solve(&self) {
                let source_ptr = self.source.as_ptr();
                let ixes_ptr = self.ixes.as_ptr();
                let out_ptr = self.out.as_mut_ptr();
                $op!(source_ptr, ixes_ptr, out_ptr);
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
        impl<T> MechFunctionCompiler for $struct_name<T>
        where
            T: CompileConst + ConstElem + AsValueKind,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!("{}<{}>", stringify!($struct_name), T::as_value_kind());
                compile_binop!(name, self.out, self.source, self.ixes, ctx);
            }
        }
    };
}

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
            T: Debug + Clone + Sync + Send + PartialEq + 'static + ConstElem + AsValueKind,
            #[cfg(feature = "compiler")]
            T: CompileConst,
            Ref<$out_type>: ToValue,
        {
            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                match args {
                    FunctionArgs::Ternary(out, arg1, arg2, arg3) => {
                        let source: Ref<$arg_type> =
                            arg1.try_function_ref(FunctionArgumentRole::Input(0))?;
                        let ix1: Ref<$ix1_type> =
                            arg2.try_function_ref(FunctionArgumentRole::Input(1))?;
                        let ix2: Ref<$ix2_type> =
                            arg3.try_function_ref(FunctionArgumentRole::Input(2))?;
                        let out: Ref<$out_type> =
                            out.try_function_ref(FunctionArgumentRole::Output)?;
                        Ok(Box::new($struct_name {
                            source,
                            ix1,
                            ix2,
                            out,
                        }))
                    }
                    _ => Err(MechError::new(
                        IncorrectNumberOfArguments {
                            expected: 3,
                            found: args.len(),
                        },
                        None,
                    )
                    .with_compiler_loc()),
                }
            }
        }
        impl<T> MechFunctionImpl for $struct_name<T>
        where
            T: Debug + Clone + Sync + Send + PartialEq + 'static,
            Ref<$out_type>: ToValue,
        {
            fn solve(&self) {
                let source_ptr = self.source.as_ptr();
                let ix1_ptr = self.ix1.as_ptr();
                let ix2_ptr = self.ix2.as_ptr();
                let out_ptr = self.out.as_mut_ptr();
                $op!(source_ptr, ix1_ptr, ix2_ptr, out_ptr);
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
        impl<T> MechFunctionCompiler for $struct_name<T>
        where
            T: CompileConst + ConstElem + AsValueKind,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!("{}<{}>", stringify!($struct_name), T::as_value_kind());
                compile_ternop!(name, self.out, self.source, self.ix1, self.ix2, ctx);
            }
        }
    };
}

macro_rules! impl_access_fxn_shape {
    ($name:ident, $ix_type:ty, $out_type:ty, $fxn:ident) => {
        paste! {
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
    };
}

macro_rules! impl_access_fxn_shape2 {
    ($name:ident, $ix1_type:ty, $ix2_type:ty, $out_type:ty, $fxn:ident) => {
        paste! {
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
    };
}

// x[1]
impl_access_fxn_shape!(Access1DS, usize, T, access_1d);

// x[1,2]
impl_access_fxn_shape2!(Access2DSS, usize, usize, T, access_2d);

// x[1..3]
impl_access_fxn_shape!(Access1DVD, DVector<usize>, DVector<T>, access_1d_slice);
impl_access_fxn_shape!(
    Access1DVDb,
    DVector<bool>,
    DVector<T>,
    access_1d_slice_bool_v
);

// x[:]
impl_access_fxn_shape!(Access1DA, Value, DVector<T>, access_1d_all);

// x[:,1]
impl_access_fxn_shape!(Access2DAS, usize, DVector<T>, access_col);

// x[1,:]
#[cfg(feature = "matrix1")]
impl_access_fxn!(Access2DSAM1, Matrix1<T>, usize, Matrix1<T>, access_row);
#[cfg(all(feature = "matrix2", feature = "row_vector2"))]
impl_access_fxn!(Access2DSAM2, Matrix2<T>, usize, RowVector2<T>, access_row);
#[cfg(all(feature = "matrix3", feature = "row_vector3"))]
impl_access_fxn!(Access2DSAM3, Matrix3<T>, usize, RowVector3<T>, access_row);
#[cfg(all(feature = "matrix4", feature = "row_vector4"))]
impl_access_fxn!(Access2DSAM4, Matrix4<T>, usize, RowVector4<T>, access_row);
#[cfg(all(feature = "matrix2x3", feature = "row_vector3"))]
impl_access_fxn!(
    Access2DSAM2x3,
    Matrix2x3<T>,
    usize,
    RowVector3<T>,
    access_row
);
#[cfg(all(feature = "matrix3x2", feature = "row_vector2"))]
impl_access_fxn!(
    Access2DSAM3x2,
    Matrix3x2<T>,
    usize,
    RowVector2<T>,
    access_row
);
#[cfg(all(feature = "matrixd", feature = "row_vectord"))]
impl_access_fxn!(Access2DSAMD, DMatrix<T>, usize, RowDVector<T>, access_row);

// x[1..3,:]
impl_access_fxn_shape!(Access2DVDA, DVector<usize>, DMatrix<T>, access_2d_slice_all);
impl_access_fxn_shape!(
    Access2DVDbA,
    DVector<bool>,
    DMatrix<T>,
    access_2d_slice_all_bool
);

// x[2,1..3]
impl_access_fxn_shape2!(
    Access2DSVD,
    usize,
    DVector<usize>,
    RowDVector<T>,
    access_2d_row_slice
);
impl_access_fxn_shape2!(
    Access2DSVDb,
    usize,
    DVector<bool>,
    RowDVector<T>,
    access_2d_row_slice_bool
);

// x[1..3,2]
impl_access_fxn_shape2!(
    Access2DVDS,
    DVector<usize>,
    usize,
    DVector<T>,
    access_2d_col_slice
);
impl_access_fxn_shape2!(
    Access2DVDbS,
    DVector<bool>,
    usize,
    DVector<T>,
    access_2d_col_slice_bool
);

macro_rules! impl_access_match_arms {
    ($fxn_name:ident,$macro_name:ident, $arg:expr) => {
        paste! {
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
            F32  => MatrixF32,  f32,  f32::default(), "f32";
            F64  => MatrixF64,  f64,  f64::default(), "f64";
            String => MatrixString, String, String::default(), "string";
            C64 => MatrixC64, C64, C64::default(), "complex";
            R64 => MatrixR64, R64, R64::default(), "rational";
          )
        }
    };
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
              Ok(Box::new([<$fxn_name R4>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            (Value::$matrix_kind(Matrix::RowVector3(input)), [Value::Index(ix)]) => {
              Ok(Box::new([<$fxn_name R3>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            (Value::$matrix_kind(Matrix::RowVector2(input)), [Value::Index(ix)]) => {
              Ok(Box::new([<$fxn_name R2>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            (Value::$matrix_kind(Matrix::Vector4(input)),    [Value::Index(ix)]) => {
              Ok(Box::new([<$fxn_name V4>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            (Value::$matrix_kind(Matrix::Vector3(input)),    [Value::Index(ix)]) => {
              Ok(Box::new([<$fxn_name V3>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            (Value::$matrix_kind(Matrix::Vector2(input)),    [Value::Index(ix)]) => {
              Ok(Box::new([<$fxn_name V2>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (Value::$matrix_kind(Matrix::Matrix4(input)),    [Value::Index(ix)]) => {
              Ok(Box::new([<$fxn_name M4>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::$matrix_kind(Matrix::Matrix3(input)),    [Value::Index(ix)]) => {
              Ok(Box::new([<$fxn_name M3>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::$matrix_kind(Matrix::Matrix2(input)),    [Value::Index(ix)]) => {
              Ok(Box::new([<$fxn_name M2>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            (Value::$matrix_kind(Matrix::Matrix1(input)),    [Value::Index(ix)]) => {
              Ok(Box::new([<$fxn_name M1>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(input)),  [Value::Index(ix)]) => {
              Ok(Box::new([<$fxn_name M2x3>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(input)),  [Value::Index(ix)]) => {
              Ok(Box::new([<$fxn_name M3x2>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            (Value::$matrix_kind(Matrix::RowDVector(input)), [Value::Index(ix)]) => {
              Ok(Box::new([<$fxn_name RD>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            (Value::$matrix_kind(Matrix::DVector(input)),    [Value::Index(ix)]) => {
              Ok(Box::new([<$fxn_name VD>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::$matrix_kind(Matrix::DMatrix(input)),    [Value::Index(ix)]) => {
              Ok(Box::new([<$fxn_name MD>]  {source: input.clone(), ixes: ix.clone(), out: Ref::new($default) }))
            },
          )+
        )+
        (src, ix) => Err(MechError::new(UnhandledFunctionArgumentIxesMono { arg: (src.kind(), ix.iter().map(|x| x.kind()).collect()), fxn_name: stringify!($fxn_name).to_string() }, None).with_compiler_loc()),
      }
    }
  }
}

fn impl_access_scalar_fxn(lhs_value: Value, ixes: Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    impl_access_match_arms!(Access1DS, scalar, (lhs_value, ixes.as_slice()))
}

#[derive(Debug)]
struct MatrixAccessScalarValueF {
    source: Matrix<Value>,
    ix: Ref<usize>,
    out: Ref<Value>,
    element_kind: ValueKind,
}

impl MechFunctionImpl for MatrixAccessScalarValueF {
    fn solve(&self) {
        let ix = *self.ix.borrow();
        let value = self.source.index1d(ix);
        *self.out.borrow_mut() = match &self.element_kind {
            ValueKind::Option(_) => Value::Typed(Box::new(value), self.element_kind.clone()),
            _ => value,
        };
    }
    fn out(&self) -> Value {
        self.out.borrow().clone()
    }
    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(vec![Value::MutableReference(self.out.clone())])
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}

#[cfg(all(test, feature = "functions", feature = "matrixd"))]
mod matrix_access_scalar_value_transaction_tests {
    use super::*;

    #[test]
    fn transaction_state_retains_scalar_value_access_outer_output_ref() {
        let out = Ref::new(Value::Empty);
        let function = MatrixAccessScalarValueF {
            source: Matrix::from_vec(vec![Value::Empty], 1, 1),
            ix: Ref::new(1),
            out: out.clone(),
            element_kind: ValueKind::Any,
        };

        let values = function.transaction_state_values().unwrap();
        assert_eq!(values.len(), 1);
        match &values[0] {
            Value::MutableReference(root) => assert_eq!(root.addr(), out.addr()),
            other => panic!("expected mutable-reference transaction root, got {other:?}"),
        }
    }
}

#[cfg(feature = "compiler")]
impl MechFunctionCompiler for MatrixAccessScalarValueF {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let mut registers = [0, 0, 0];
        registers[0] = compile_register_brrw!(self.out, ctx);
        registers[1] = compile_register!(self.source, ctx);
        registers[2] = compile_register_brrw!(self.ix, ctx);
        ctx.emit_binop(
            hash_str("MatrixAccessScalarValueF"),
            registers[0],
            registers[1],
            registers[2],
        );
        Ok(registers[0])
    }
}

pub struct MatrixAccessScalar {}
impl FunctionSpecializer for MatrixAccessScalar {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() <= 1 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let ixes = arguments[1..].to_vec();
        let mat = arguments[0].clone();
        if let (Value::MatrixValue(source), [Value::Index(ix)]) = (mat.clone(), ixes.as_slice()) {
            let element_kind = match mat.kind() {
                ValueKind::Matrix(elem, _) => (*elem).clone(),
                _ => ValueKind::Any,
            };
            let init = match &element_kind {
                ValueKind::Option(_) => Value::Typed(Box::new(Value::Empty), element_kind.clone()),
                _ => Value::Empty,
            };
            return Ok(Box::new(MatrixAccessScalarValueF {
                source,
                ix: ix.clone(),
                out: Ref::new(init),
                element_kind,
            }));
        }
        match impl_access_scalar_fxn(mat.clone(), ixes.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (mat, ixes) {
                (Value::MutableReference(lhs), rhs_value) => {
                    impl_access_scalar_fxn(lhs.borrow().clone(), rhs_value.clone())
                }
                (src, ix) => Err(MechError::new(
                    UnhandledFunctionArgumentIxesMono {
                        arg: (src.kind(), ix.iter().map(|x| x.kind()).collect()),
                        fxn_name: "MatrixAccessScalar".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
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
              Ok(Box::new([<$fxn_name R4>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            (Value::$matrix_kind(Matrix::RowVector3(input)), [Value::Index(ix1),Value::Index(ix2)]) => {
              Ok(Box::new([<$fxn_name R3>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            (Value::$matrix_kind(Matrix::RowVector2(input)), [Value::Index(ix1),Value::Index(ix2)]) => {
              Ok(Box::new([<$fxn_name R2>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            (Value::$matrix_kind(Matrix::Vector4(input)),    [Value::Index(ix1),Value::Index(ix2)]) => {
              Ok(Box::new([<$fxn_name V4>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            (Value::$matrix_kind(Matrix::Vector3(input)),    [Value::Index(ix1),Value::Index(ix2)]) => {
              Ok(Box::new([<$fxn_name V3>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            (Value::$matrix_kind(Matrix::Vector2(input)),    [Value::Index(ix1),Value::Index(ix2)]) => {
              Ok(Box::new([<$fxn_name V2>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (Value::$matrix_kind(Matrix::Matrix4(input)),    [Value::Index(ix1),Value::Index(ix2)]) => {
              Ok(Box::new([<$fxn_name M4>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::$matrix_kind(Matrix::Matrix3(input)),    [Value::Index(ix1),Value::Index(ix2)]) => {
              Ok(Box::new([<$fxn_name M3>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::$matrix_kind(Matrix::Matrix2(input)),    [Value::Index(ix1),Value::Index(ix2)]) => {
              Ok(Box::new([<$fxn_name M2>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(input)),  [Value::Index(ix1),Value::Index(ix2)]) => {
              Ok(Box::new([<$fxn_name M2x3>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(input)),  [Value::Index(ix1),Value::Index(ix2)]) => {
              Ok(Box::new([<$fxn_name M3x2>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            (Value::$matrix_kind(Matrix::RowDVector(input)), [Value::Index(ix1),Value::Index(ix2)]) => {
              Ok(Box::new([<$fxn_name RD>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            (Value::$matrix_kind(Matrix::DVector(input)),    [Value::Index(ix1),Value::Index(ix2)]) => {
              Ok(Box::new([<$fxn_name VD>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::$matrix_kind(Matrix::DMatrix(input)),    [Value::Index(ix1),Value::Index(ix2)]) => {
              Ok(Box::new([<$fxn_name MD>]  {source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new($default) }))
            },
          )+
        )+
        (src, ix) => Err(MechError::new(UnhandledFunctionArgumentIxesMono { arg: (src.kind(), ix.iter().map(|x| x.kind()).collect()), fxn_name: stringify!($fxn_name).to_string() }, None).with_compiler_loc()),
      }
    }
  }
}

fn impl_access_scalar_scalar_fxn(
    lhs_value: Value,
    ixes: Vec<Value>,
) -> MResult<Box<dyn MechFunction>> {
    impl_access_match_arms!(Access2DSS, scalar_scalar, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessScalarScalar {}
impl FunctionSpecializer for MatrixAccessScalarScalar {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() <= 2 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let ixes = arguments[1..].to_vec();
        let mat = arguments[0].clone();
        match impl_access_scalar_scalar_fxn(mat.clone(), ixes.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (mat, ixes) {
                (Value::MutableReference(lhs), rhs_value) => {
                    impl_access_scalar_scalar_fxn(lhs.borrow().clone(), rhs_value.clone())
                }
                (src, ix) => Err(MechError::new(
                    UnhandledFunctionArgumentIxesMono {
                        arg: (src.kind(), ix.iter().map(|x| x.kind()).collect()),
                        fxn_name: "MatrixAccessScalarScalar".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
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
              Ok(Box::new(Access1DVDbR4{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::RowVector3(input)), [Value::MatrixBool(Matrix::DVector(ix))])     => {
              Ok(Box::new(Access1DVDbR3{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::RowVector2(input)), [Value::MatrixBool(Matrix::DVector(ix))])     => {
              Ok(Box::new(Access1DVDbR2{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::RowDVector(input)), [Value::MatrixBool(Matrix::DVector(ix))])     => {
              Ok(Box::new(Access1DVDbRD{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },

            // --

            #[cfg(all(feature = $value_string, feature = "vector4", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Vector4(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => {
              Ok(Box::new(Access1DVDbV4{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Vector3(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => {
              Ok(Box::new(Access1DVDbV3{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Vector2(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => {
              Ok(Box::new(Access1DVDbV2{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::DVector(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => {
              Ok(Box::new(Access1DVDbVD{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },

            // --

            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix4(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => {
              Ok(Box::new(Access1DVDbM4{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix3(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => {
              Ok(Box::new(Access1DVDbM3{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix2(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => {
              Ok(Box::new(Access1DVDbM2{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix1(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => {
              Ok(Box::new(Access1DVDbM1{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => {
              Ok(Box::new(Access1DVDbM3x2{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => {
              Ok(Box::new(Access1DVDbM2x3{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::DMatrix(input)), [Value::MatrixBool(Matrix::DVector(ix))])  => {
              Ok(Box::new(Access1DVDbMD{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },

            // --

            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            (Value::$matrix_kind(Matrix::RowVector4(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
              Ok(Box::new(Access1DVDR4{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            (Value::$matrix_kind(Matrix::RowVector3(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
              Ok(Box::new(Access1DVDR3{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            (Value::$matrix_kind(Matrix::RowVector2(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
              Ok(Box::new(Access1DVDR2{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            (Value::$matrix_kind(Matrix::RowDVector(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
              Ok(Box::new(Access1DVDRD{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },

            // --

            #[cfg(all(feature = $value_string, feature = "vector4"))]
            (Value::$matrix_kind(Matrix::Vector4(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
              Ok(Box::new(Access1DVDV4{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            (Value::$matrix_kind(Matrix::Vector3(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
              Ok(Box::new(Access1DVDV3{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            (Value::$matrix_kind(Matrix::Vector2(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
              Ok(Box::new(Access1DVDV2{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            (Value::$matrix_kind(Matrix::DVector(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
              Ok(Box::new(Access1DVDVD{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },

            // --

            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (Value::$matrix_kind(Matrix::Matrix4(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
              Ok(Box::new(Access1DVDM4{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::$matrix_kind(Matrix::Matrix3(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
              Ok(Box::new(Access1DVDM3{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::$matrix_kind(Matrix::Matrix2(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
              Ok(Box::new(Access1DVDM2{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            (Value::$matrix_kind(Matrix::Matrix1(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
              Ok(Box::new(Access1DVDM1{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(input)), [Value::MatrixIndex(Matrix::DVector(ix))]) => {
              Ok(Box::new(Access1DVDM3x2{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(input)), [Value::MatrixIndex(Matrix::DVector(ix))]) => {
              Ok(Box::new(Access1DVDM2x3{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::$matrix_kind(Matrix::DMatrix(input)), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
              Ok(Box::new(Access1DVDMD{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(ix.borrow().len(),$default)) }))
            },
          )+
        )+
        (src, ix) => Err(MechError::new(UnhandledFunctionArgumentIxesMono { arg: (src.kind(), ix.iter().map(|x| x.kind()).collect()), fxn_name: stringify!($fxn_name).to_string() }, None).with_compiler_loc()),
      }
    }
  }
}

fn impl_access_range_fxn(lhs_value: Value, ixes: Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    impl_access_match_arms!(Access1DR, range, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessRange {}
impl FunctionSpecializer for MatrixAccessRange {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() <= 1 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let ixes = arguments[1..].to_vec();
        let mat = arguments[0].clone();
        match impl_access_range_fxn(mat.clone(), ixes.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (mat, ixes) {
                (Value::MutableReference(lhs), rhs_value) => {
                    impl_access_range_fxn(lhs.borrow().clone(), rhs_value.clone())
                }
                (src, ix) => Err(MechError::new(
                    UnhandledFunctionArgumentIxesMono {
                        arg: (src.kind(), ix.iter().map(|x| x.kind()).collect()),
                        fxn_name: "MatrixAccessRange".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
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
    };
}

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
    };
}

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
    };
}

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
    };
}

macro_rules! impl_access_range_range_arms {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste!{
      match $arg {
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(source)),[Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))]) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VUU>] { source: source.clone(), ixes: (ix1.clone(), ix2.clone()), sink: Ref::new(DMatrix::from_element(ix1.borrow().len(), ix2.borrow().len(), $value_kind::default())), _marker: std::marker::PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "row_vectord", feature = "logical_indexing"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(source)),[Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))]) => {
          let rows = ix1.borrow().iter().filter(|x| **x).count();
          let cols = ix2.borrow().iter().filter(|x| **x).count();
          match (cols, rows) {
            #[cfg(feature = "matrixd")]
            (1, 1) => {
              box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { source: source.clone(), ixes: (ix1.clone(), ix2.clone()), sink: Ref::new(DMatrix::from_element(1, 1, $value_kind::default())), _marker: std::marker::PhantomData::default() })))
            },
            #[cfg(feature = "vectord")]
            (1, _) => {
              box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { source: source.clone(), ixes: (ix1.clone(), ix2.clone()), sink: Ref::new(DVector::from_element(rows, $value_kind::default())), _marker: std::marker::PhantomData::default() })))
            },
            #[cfg(feature = "row_vectord")]
            (_, 1) => {
              box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { source: source.clone(), ixes: (ix1.clone(), ix2.clone()), sink: Ref::new(RowDVector::from_element(cols, $value_kind::default())), _marker: std::marker::PhantomData::default() })))
            },
            #[cfg(feature = "matrixd")]
            _ => {
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
              box_mech_fxn(Ok(Box::new([<$fxn_name VUB>] { source: source.clone(), ixes: (ix1.clone(), ix2.clone()), sink: Ref::new(DMatrix::from_element(1, 1, $value_kind::default())), _marker: std::marker::PhantomData::default() })))
            },
            #[cfg(feature = "vectord")]
            (1, _) => {
              box_mech_fxn(Ok(Box::new([<$fxn_name VUB>] { source: source.clone(), ixes: (ix1.clone(), ix2.clone()), sink: Ref::new(DVector::from_element(rows, $value_kind::default())), _marker: std::marker::PhantomData::default() })))
            },
            #[cfg(feature = "row_vectord")]
            (_, 1) => {
              box_mech_fxn(Ok(Box::new([<$fxn_name VUB>] { source: source.clone(), ixes: (ix1.clone(), ix2.clone()), sink: Ref::new(RowDVector::from_element(cols, $value_kind::default())), _marker: std::marker::PhantomData::default() })))
            },
            #[cfg(feature = "matrixd")]
            _ => {
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
              box_mech_fxn(Ok(Box::new([<$fxn_name VBU>] { source: source.clone(), ixes: (ix1.clone(), ix2.clone()), sink: Ref::new(DMatrix::from_element(1, 1, $value_kind::default())), _marker: std::marker::PhantomData::default() })))
            },
            #[cfg(feature = "vectord")]
            (1, _) => {
              box_mech_fxn(Ok(Box::new([<$fxn_name VBU>] { source: source.clone(), ixes: (ix1.clone(), ix2.clone()), sink: Ref::new(DVector::from_element(rows, $value_kind::default())), _marker: std::marker::PhantomData::default() })))
            },
            #[cfg(feature = "row_vectord")]
            (_, 1) => {
              box_mech_fxn(Ok(Box::new([<$fxn_name VBU>] { source: source.clone(), ixes: (ix1.clone(), ix2.clone()), sink: Ref::new(RowDVector::from_element(cols, $value_kind::default())), _marker: std::marker::PhantomData::default() })))
            },
            #[cfg(feature = "matrixd")]
            _ => {
              box_mech_fxn(Ok(Box::new([<$fxn_name VBU>] { source: source.clone(), ixes: (ix1.clone(), ix2.clone()), sink: Ref::new(DMatrix::from_element(rows, cols, $value_kind::default())), _marker: std::marker::PhantomData::default() })))
            },
          }
        }
        (src, ix) => Err(MechError::new(
          UnhandledFunctionArgumentIxesMono{arg: (src.kind(), ix.iter().map(|x| x.kind()).collect()), fxn_name: stringify!($fxn_name).to_string()},
          None).with_compiler_loc()
        ),
      }
    }
  }
}

impl_range_range_fxn_v!(Access2DRRVBB, access_2d_range_range_vbb, bool, bool);
impl_range_range_fxn_v!(Access2DRRVBU, access_2d_range_range_vbu, bool, usize);
impl_range_range_fxn_v!(Access2DRRVUU, access_2d_range_range_vuu, usize, usize);
impl_range_range_fxn_v!(Access2DRRVUB, access_2d_range_range_vub, usize, bool);

fn matrix_access_range_range_fxn(
    source: Value,
    ixes: Vec<Value>,
) -> MResult<Box<dyn MechFunction>> {
    let arg = (source.clone(), ixes.as_slice());
    impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, u8, "u8")
        .or_else(|_| {
            impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, u16, "u16")
        })
        .or_else(|_| {
            impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, u32, "u32")
        })
        .or_else(|_| {
            impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, u64, "u64")
        })
        .or_else(|_| {
            impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, u128, "u128")
        })
        .or_else(|_| impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, i8, "i8"))
        .or_else(|_| {
            impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, i16, "i16")
        })
        .or_else(|_| {
            impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, i32, "i32")
        })
        .or_else(|_| {
            impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, i64, "i64")
        })
        .or_else(|_| {
            impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, i128, "i128")
        })
        .or_else(|_| {
            impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, f32, "f32")
        })
        .or_else(|_| {
            impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, f64, "f64")
        })
        .or_else(|_| {
            impl_access_fxn_new!(
                impl_access_range_range_arms,
                Access2DRR,
                arg,
                R64,
                "rational"
            )
        })
        .or_else(|_| {
            impl_access_fxn_new!(
                impl_access_range_range_arms,
                Access2DRR,
                arg,
                C64,
                "complex"
            )
        })
        .or_else(|_| {
            impl_access_fxn_new!(impl_access_range_range_arms, Access2DRR, arg, bool, "bool")
        })
        .or_else(|_| {
            impl_access_fxn_new!(
                impl_access_range_range_arms,
                Access2DRR,
                arg,
                String,
                "string"
            )
        })
        .map_err(|_| {
            MechError::new(
                UnhandledFunctionArgumentIxesMono {
                    arg: (source.kind(), ixes.iter().map(|x| x.kind()).collect()),
                    fxn_name: "MatrixAccessRangeRange".to_string(),
                },
                None,
            )
            .with_compiler_loc()
        })
}

pub struct MatrixAccessRangeRange {}
impl FunctionSpecializer for MatrixAccessRangeRange {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() <= 1 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let source: Value = arguments[0].clone();
        let ixes = arguments[1..].to_vec();
        match matrix_access_range_range_fxn(source.clone(), ixes.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match source {
                Value::MutableReference(source) => {
                    matrix_access_range_range_fxn(source.borrow().clone(), ixes.clone())
                }
                _ => Err(MechError::new(
                    UnhandledFunctionArgumentIxesMono {
                        arg: (source.kind(), ixes.iter().map(|x| x.kind()).collect()),
                        fxn_name: "MatrixAccessRangeRange".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
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
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            (Value::$matrix_kind(Matrix::RowVector4(input)), [Value::IndexAll]) => {
              Ok(Box::new(Access1DAR4  {source: input.clone(), ixes: Ref::new(Value::IndexAll), out: Ref::new(DVector::from_element(input.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            (Value::$matrix_kind(Matrix::RowVector3(input)), [Value::IndexAll]) => {
              Ok(Box::new(Access1DAR3  {source: input.clone(), ixes: Ref::new(Value::IndexAll), out: Ref::new(DVector::from_element(input.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            (Value::$matrix_kind(Matrix::RowVector2(input)), [Value::IndexAll]) => {
              Ok(Box::new(Access1DAR2  {source: input.clone(), ixes: Ref::new(Value::IndexAll), out: Ref::new(DVector::from_element(input.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            (Value::$matrix_kind(Matrix::Vector4(input)),    [Value::IndexAll]) => {
              Ok(Box::new(Access1DAV4  {source: input.clone(), ixes: Ref::new(Value::IndexAll), out: Ref::new(DVector::from_element(input.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            (Value::$matrix_kind(Matrix::Vector3(input)),    [Value::IndexAll]) => {
              Ok(Box::new(Access1DAV3  {source: input.clone(), ixes: Ref::new(Value::IndexAll), out: Ref::new(DVector::from_element(input.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            (Value::$matrix_kind(Matrix::Vector2(input)),    [Value::IndexAll]) => {
              Ok(Box::new(Access1DAV2  {source: input.clone(), ixes: Ref::new(Value::IndexAll), out: Ref::new(DVector::from_element(input.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            (Value::$matrix_kind(Matrix::RowDVector(input)), [Value::IndexAll]) => {
              Ok(Box::new(Access1DARD  {source: input.clone(), ixes: Ref::new(Value::IndexAll), out: Ref::new(DVector::from_element(input.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            (Value::$matrix_kind(Matrix::DVector(input)),    [Value::IndexAll]) => {
              Ok(Box::new(Access1DAVD  {source: input.clone(), ixes: Ref::new(Value::IndexAll), out: Ref::new(DVector::from_element(input.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (Value::$matrix_kind(Matrix::Matrix4(input)),    [Value::IndexAll]) => {
              Ok(Box::new(Access1DAM4  {source: input.clone(), ixes: Ref::new(Value::IndexAll), out: Ref::new(DVector::from_element(input.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::$matrix_kind(Matrix::Matrix3(input)),    [Value::IndexAll]) => {
              Ok(Box::new(Access1DAM3  {source: input.clone(), ixes: Ref::new(Value::IndexAll), out: Ref::new(DVector::from_element(input.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::$matrix_kind(Matrix::Matrix2(input)),    [Value::IndexAll]) => {
              Ok(Box::new(Access1DAM2  {source: input.clone(), ixes: Ref::new(Value::IndexAll), out: Ref::new(DVector::from_element(input.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(input)),  [Value::IndexAll]) => {
              Ok(Box::new(Access1DAM3x2{source: input.clone(), ixes: Ref::new(Value::IndexAll), out: Ref::new(DVector::from_element(input.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(input)),  [Value::IndexAll]) => {
              Ok(Box::new(Access1DAM2x3{source: input.clone(), ixes: Ref::new(Value::IndexAll), out: Ref::new(DVector::from_element(input.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::$matrix_kind(Matrix::DMatrix(input)),    [Value::IndexAll]) => {
              Ok(Box::new(Access1DAMD  {source: input.clone(), ixes: Ref::new(Value::IndexAll), out: Ref::new(DVector::from_element(input.borrow().len(),$default)) }))
            },
          )+
        )+
        (src, ix) => Err(MechError::new(
          UnhandledFunctionArgumentIxesMono{arg: (src.kind(), ix.iter().map(|x| x.kind()).collect()), fxn_name: stringify!($fxn_name).to_string()},
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
impl FunctionSpecializer for MatrixAccessAll {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() <= 1 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let ixes = arguments[1..].to_vec();
        let mat = arguments[0].clone();
        match impl_access_all_fxn(mat.clone(), ixes.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (mat, ixes) {
                (Value::MutableReference(lhs), rhs_value) => {
                    impl_access_all_fxn(lhs.borrow().clone(), rhs_value.clone())
                }
                (src, ix) => Err(MechError::new(
                    UnhandledFunctionArgumentIxesMono {
                        arg: (src.kind(), ix.iter().map(|x| x.kind()).collect()),
                        fxn_name: "MatrixAccessAll".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
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
              Ok(Box::new(Access2DASM4  {source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(input.borrow().nrows(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::$matrix_kind(Matrix::Matrix3(input)),    [Value::IndexAll,Value::Index(ix)]) => {
              Ok(Box::new(Access2DASM3  {source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(input.borrow().nrows(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::$matrix_kind(Matrix::Matrix2(input)),    [Value::IndexAll,Value::Index(ix)]) => {
              Ok(Box::new(Access2DASM2  {source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(input.borrow().nrows(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(input)),  [Value::IndexAll,Value::Index(ix)]) => {
              Ok(Box::new(Access2DASM2x3{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(input.borrow().nrows(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(input)),  [Value::IndexAll,Value::Index(ix)]) => {
              Ok(Box::new(Access2DASM3x2{source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(input.borrow().nrows(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::$matrix_kind(Matrix::DMatrix(input)),    [Value::IndexAll,Value::Index(ix)]) => {
              Ok(Box::new(Access2DASMD  {source: input.clone(), ixes: ix.clone(), out: Ref::new(DVector::from_element(input.borrow().nrows(),$default)) }))
            },
          )+
        )+
        (src, ix) => Err(MechError::new(
          UnhandledFunctionArgumentIxesMono{arg: (src.kind(), ix.iter().map(|x| x.kind()).collect()), fxn_name: stringify!($fxn_name).to_string()},
          None).with_compiler_loc()
        ),
      }
    }
  }
}

fn impl_access_all_scalar_fxn(
    lhs_value: Value,
    ixes: Vec<Value>,
) -> MResult<Box<dyn MechFunction>> {
    impl_access_match_arms!(Access2DAS, all_scalar, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessAllScalar {}
impl FunctionSpecializer for MatrixAccessAllScalar {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() <= 2 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let ixes = arguments[1..].to_vec();
        let mat = arguments[0].clone();
        match impl_access_all_scalar_fxn(mat.clone(), ixes.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (mat, ixes) {
                (Value::MutableReference(lhs), rhs_value) => {
                    impl_access_all_scalar_fxn(lhs.borrow().clone(), rhs_value.clone())
                }
                (src, ix) => Err(MechError::new(
                    UnhandledFunctionArgumentIxesMono {
                        arg: (src.kind(), ix.iter().map(|x| x.kind()).collect()),
                        fxn_name: "MatrixAccessAllScalar".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
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
              Ok(Box::new(Access2DSAM4{source: input.clone(), ixes: ix.clone(), out: Ref::new(RowVector4::from_element($default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "row_vector3"))]
            (Value::$matrix_kind(Matrix::Matrix3(input)), [Value::Index(ix),Value::IndexAll]) => {
              Ok(Box::new(Access2DSAM3{source: input.clone(), ixes: ix.clone(), out: Ref::new(RowVector3::from_element($default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "row_vector2"))]
            (Value::$matrix_kind(Matrix::Matrix2(input)), [Value::Index(ix),Value::IndexAll]) => {
              Ok(Box::new(Access2DSAM2{source: input.clone(), ixes: ix.clone(), out: Ref::new(RowVector2::from_element($default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "matrix1"))]
            (Value::$matrix_kind(Matrix::Matrix1(input)), [Value::Index(ix),Value::IndexAll]) => {
              Ok(Box::new(Access2DSAM1{source: input.clone(), ixes: ix.clone(), out: Ref::new(Matrix1::from_element($default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "row_vector2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(input)), [Value::Index(ix),Value::IndexAll]) => {
              Ok(Box::new(Access2DSAM3x2{source: input.clone(), ixes: ix.clone(), out: Ref::new(RowVector2::from_element($default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "row_vector3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(input)), [Value::Index(ix),Value::IndexAll]) => {
              Ok(Box::new(Access2DSAM2x3{source: input.clone(), ixes: ix.clone(), out: Ref::new(RowVector3::from_element($default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "row_vectord"))]
            (Value::$matrix_kind(Matrix::DMatrix(input)), [Value::Index(ix),Value::IndexAll]) => {
              Ok(Box::new(Access2DSAMD{source: input.clone(), ixes: ix.clone(), out: Ref::new(RowDVector::from_element(input.borrow().ncols(),$default)) }))
            },
          )+
        )+
        (src, ix) => Err(MechError::new(
          UnhandledFunctionArgumentIxesMono{arg: (src.kind(), ix.iter().map(|x| x.kind()).collect()), fxn_name: stringify!($fxn_name).to_string()},
          None).with_compiler_loc()
        ),
      }
    }
  }
}

fn impl_access_scalar_all_fxn(
    lhs_value: Value,
    ixes: Vec<Value>,
) -> MResult<Box<dyn MechFunction>> {
    impl_access_match_arms!(Access2DSA, scalar_all, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessScalarAll {}
impl FunctionSpecializer for MatrixAccessScalarAll {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() <= 2 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let ixes = arguments[1..].to_vec();
        let mat = arguments[0].clone();
        match impl_access_scalar_all_fxn(mat.clone(), ixes.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (mat, ixes) {
                (Value::MutableReference(lhs), rhs_value) => {
                    impl_access_scalar_all_fxn(lhs.borrow().clone(), rhs_value.clone())
                }
                (mat, ix) => Err(MechError::new(
                    UnhandledFunctionArgumentIxesMono {
                        arg: (mat.kind(), ix.iter().map(|x| x.kind()).collect()),
                        fxn_name: "MatrixAccessScalarAll".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}

// x[:,1..3] ---------------------------------------------------------------------

macro_rules! assign_2d_all_range_v {
    ($source:expr, $ix:expr, $sink:expr) => {{
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
    }};
}

macro_rules! assign_2d_all_range_vb {
    ($source:expr, $ix:expr, $sink:expr) => {{
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
    }};
}

macro_rules! impl_access_all_range_arms {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste!{
      match $arg {
        // All Vector
        #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(source)), [Value::IndexAll, Value::MatrixIndex(Matrix::DVector(ix))]) if source.borrow().nrows() == 1 => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>]{source: source.clone(), ixes: ix.clone(), sink: Ref::new(RowDVector::from_element(ix.borrow().len(), $value_kind::default())), _marker: std::marker::PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(source)), [Value::IndexAll, Value::MatrixIndex(Matrix::DVector(ix))]) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>]{source: source.clone(), ixes: ix.clone(), sink: Ref::new(DMatrix::from_element(source.borrow().nrows(), ix.borrow().len(), $value_kind::default())), _marker: std::marker::PhantomData::default() })))
        },
        // All Bool Vector
        #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(source)), [Value::IndexAll, Value::MatrixBool(Matrix::DVector(ix))]) if source.borrow().nrows() == 1 => {
          let cols = ix.borrow().iter().filter(|&&b| b).count();
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>]{source: source.clone(), ixes: ix.clone(), sink: Ref::new(RowDVector::from_element(cols, $value_kind::default())), _marker: std::marker::PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "logical_indexing", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(source)), [Value::IndexAll, Value::MatrixBool(Matrix::DVector(ix))]) if ix.borrow().iter().filter(|&&b| b).count() == 1 && source.borrow().nrows() != 1 => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>]{source: source.clone(), ixes: ix.clone(), sink: Ref::new(DVector::from_element(source.borrow().nrows(), $value_kind::default())), _marker: std::marker::PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "logical_indexing", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(source)), [Value::IndexAll, Value::MatrixBool(Matrix::DVector(ix))]) => {
          let cols = ix.borrow().iter().filter(|&&b| b).count();
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>]{source: source.clone(), ixes: ix.clone(), sink: Ref::new(DMatrix::from_element(source.borrow().nrows(), cols, $value_kind::default())), _marker: std::marker::PhantomData::default() })))
        },
        (sink, ix) => {
          Err(MechError::new(
            UnhandledFunctionArgumentIxesMono{arg: (sink.kind(), ix.iter().map(|x| x.kind()).collect()), fxn_name: stringify!($fxn_name).to_string()},
            None).with_compiler_loc()
          )
        }
      }
    }
  }
}

impl_all_fxn_v!(Access2DARV, assign_2d_all_range_v, usize);
impl_all_fxn_v!(Access2DARVB, assign_2d_all_range_vb, bool);

fn matrix_access_all_range_fxn(source: Value, ixes: Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    let arg = (source.clone(), ixes.as_slice());
    impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, u8, "u8")
        .or_else(|_| impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, u16, "u16"))
        .or_else(|_| impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, u32, "u32"))
        .or_else(|_| impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, u64, "u64"))
        .or_else(|_| {
            impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, u128, "u128")
        })
        .or_else(|_| impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, i8, "i8"))
        .or_else(|_| impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, i16, "i16"))
        .or_else(|_| impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, i32, "i32"))
        .or_else(|_| impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, i64, "i64"))
        .or_else(|_| {
            impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, i128, "i128")
        })
        .or_else(|_| impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, f32, "f32"))
        .or_else(|_| impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, f64, "f64"))
        .or_else(|_| {
            impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, R64, "rational")
        })
        .or_else(|_| {
            impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, C64, "complex")
        })
        .or_else(|_| {
            impl_access_fxn_new!(impl_access_all_range_arms, Access2DAR, arg, bool, "bool")
        })
        .or_else(|_| {
            impl_access_fxn_new!(
                impl_access_all_range_arms,
                Access2DAR,
                arg,
                String,
                "string"
            )
        })
        .map_err(|_| {
            MechError::new(
                UnhandledFunctionArgumentIxesMono {
                    arg: (source.kind(), ixes.iter().map(|x| x.kind()).collect()),
                    fxn_name: "MatrixAccessAllRange".to_string(),
                },
                None,
            )
            .with_compiler_loc()
        })
}

pub struct MatrixAccessAllRange {}
impl FunctionSpecializer for MatrixAccessAllRange {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() <= 1 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let source: Value = arguments[0].clone();
        let ixes = arguments[1..].to_vec();
        match matrix_access_all_range_fxn(source.clone(), ixes.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match source {
                Value::MutableReference(source) => {
                    matrix_access_all_range_fxn(source.borrow().clone(), ixes.clone())
                }
                x => Err(MechError::new(
                    UnhandledFunctionArgumentIxesMono {
                        arg: (x.kind(), ixes.iter().map(|x| x.kind()).collect()),
                        fxn_name: "MatrixAccessAllRange".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
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
              Ok(Box::new(Access2DVDAM4{source: input.clone(), ixes: ix.clone(), out: Ref::new(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::$matrix_kind(Matrix::Matrix3(input)), [Value::MatrixIndex(Matrix::DVector(ix)), Value::IndexAll]) => {
              Ok(Box::new(Access2DVDAM3{source: input.clone(), ixes: ix.clone(), out: Ref::new(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::$matrix_kind(Matrix::Matrix2(input)), [Value::MatrixIndex(Matrix::DVector(ix)), Value::IndexAll]) => {
              Ok(Box::new(Access2DVDAM2{source: input.clone(), ixes: ix.clone(), out: Ref::new(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(input)), [Value::MatrixIndex(Matrix::DVector(ix)), Value::IndexAll]) => {
              Ok(Box::new(Access2DVDAM3x2{source: input.clone(), ixes: ix.clone(), out: Ref::new(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(input)), [Value::MatrixIndex(Matrix::DVector(ix)), Value::IndexAll]) => {
              Ok(Box::new(Access2DVDAM2x3{source: input.clone(), ixes: ix.clone(), out: Ref::new(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::$matrix_kind(Matrix::DMatrix(input)), [Value::MatrixIndex(Matrix::DVector(ix)), Value::IndexAll]) => {
              Ok(Box::new(Access2DVDAMD{source: input.clone(), ixes: ix.clone(), out: Ref::new(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) }))
            },
            // Bool Vector All
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix4(input)), [Value::MatrixBool(Matrix::DVector(ix)), Value::IndexAll]) => {
              Ok(Box::new(Access2DVDbAM4{source: input.clone(), ixes: ix.clone(), out: Ref::new(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix3(input)), [Value::MatrixBool(Matrix::DVector(ix)), Value::IndexAll]) => {
              Ok(Box::new(Access2DVDbAM3{source: input.clone(), ixes: ix.clone(), out: Ref::new(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix2(input)), [Value::MatrixBool(Matrix::DVector(ix)), Value::IndexAll]) => {
              Ok(Box::new(Access2DVDbAM2{source: input.clone(), ixes: ix.clone(), out: Ref::new(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(input)), [Value::MatrixBool(Matrix::DVector(ix)), Value::IndexAll]) => {
              Ok(Box::new(Access2DVDbAM3x2{source: input.clone(), ixes: ix.clone(), out: Ref::new(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(input)), [Value::MatrixBool(Matrix::DVector(ix)), Value::IndexAll]) => {
              Ok(Box::new(Access2DVDbAM2x3{source: input.clone(), ixes: ix.clone(), out: Ref::new(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::DMatrix(input)), [Value::MatrixBool(Matrix::DVector(ix)), Value::IndexAll]) => {
              Ok(Box::new(Access2DVDbAMD{source: input.clone(), ixes: ix.clone(), out: Ref::new(DMatrix::from_element(ix.borrow().len(),input.borrow().ncols(),$default)) }))
            },
          )+
        )+
        (src, ixes) => Err(MechError::new(UnhandledFunctionArgumentIxesMono{arg: (src.kind(), ixes.iter().map(|x| x.kind()).collect()), fxn_name: "MatrixAccessRangeAll".to_string()}, None).with_compiler_loc()),
      }
    }
  }
}

fn impl_access_range_all_fxn(lhs_value: Value, ixes: Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    impl_access_match_arms!(Access2DRA, range_all, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessRangeAll {}
impl FunctionSpecializer for MatrixAccessRangeAll {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() <= 2 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let ixes = arguments[1..].to_vec();
        let mat = arguments[0].clone();
        match impl_access_range_all_fxn(mat.clone(), ixes.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (mat.clone(), ixes.clone()) {
                (Value::MutableReference(lhs), rhs_value) => {
                    impl_access_range_all_fxn(lhs.borrow().clone(), rhs_value.clone())
                }
                (src, ixes) => Err(MechError::new(
                    UnhandledFunctionArgumentIxesMono {
                        arg: (src.kind(), ixes.iter().map(|x| x.kind()).collect()),
                        fxn_name: "MatrixAccessRangeAll".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
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
              Ok(Box::new(Access2DVDSM4{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(DVector::from_element(ix1.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::$matrix_kind(Matrix::Matrix3(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::Index(ix2)]) => {
              Ok(Box::new(Access2DVDSM3{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(DVector::from_element(ix1.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::$matrix_kind(Matrix::Matrix2(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::Index(ix2)]) => {
              Ok(Box::new(Access2DVDSM2{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(DVector::from_element(ix1.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(input)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::Index(ix2)]) => {
              Ok(Box::new(Access2DVDSM2x3{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(DVector::from_element(ix1.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(input)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::Index(ix2)]) => {
              Ok(Box::new(Access2DVDSM3x2{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(DVector::from_element(ix1.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::$matrix_kind(Matrix::DMatrix(input)),   [Value::MatrixIndex(Matrix::DVector(ix1)), Value::Index(ix2)]) => {
              Ok(Box::new(Access2DVDSMD{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(DVector::from_element(ix1.borrow().len(),$default)) }))
            },
            // Bool Vector Scalar
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix4(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)]) => {
              Ok(Box::new(Access2DVDbSM4{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(DVector::from_element(ix1.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix3(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)]) => {
              Ok(Box::new(Access2DVDbSM3{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(DVector::from_element(ix1.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix2(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)]) => {
              Ok(Box::new(Access2DVDbSM2{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(DVector::from_element(ix1.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(input)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)]) => {
              Ok(Box::new(Access2DVDbSM2x3{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(DVector::from_element(ix1.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(input)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)]) => {
              Ok(Box::new(Access2DVDbSM3x2{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(DVector::from_element(ix1.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::DMatrix(input)),   [Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)]) => {
              Ok(Box::new(Access2DVDbSMD{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(DVector::from_element(ix1.borrow().len(),$default)) }))
            },)+
        )+
        (src, ixes) => Err(MechError::new(UnhandledFunctionArgumentIxesMono{arg: (src.kind(), ixes.iter().map(|x| x.kind()).collect()), fxn_name: "MatrixAccessRangeRange".to_string()}, None).with_compiler_loc()),
      }
    }
  }
}

fn impl_access_range_scalar_fxn(
    lhs_value: Value,
    ixes: Vec<Value>,
) -> MResult<Box<dyn MechFunction>> {
    impl_access_match_arms!(Access2DRS, range_scalar, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessRangeScalar {}
impl FunctionSpecializer for MatrixAccessRangeScalar {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() <= 2 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let ixes = arguments[1..].to_vec();
        let mat = arguments[0].clone();
        match impl_access_range_scalar_fxn(mat.clone(), ixes.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (mat, ixes) {
                (Value::MutableReference(lhs), rhs_value) => {
                    impl_access_range_scalar_fxn(lhs.borrow().clone(), rhs_value.clone())
                }
                (src, ixs) => Err(MechError::new(
                    UnhandledFunctionArgumentIxesMono {
                        arg: (src.kind(), ixs.iter().map(|x| x.kind()).collect()),
                        fxn_name: "MatrixAccessRangeScalar".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
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
              Ok(Box::new(Access2DSVDM4{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(RowDVector::from_element(ix2.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::$matrix_kind(Matrix::Matrix3(input)),   [Value::Index(ix1), Value::MatrixIndex(Matrix::DVector(ix2))]) => {
              Ok(Box::new(Access2DSVDM3{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(RowDVector::from_element(ix2.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::$matrix_kind(Matrix::Matrix2(input)),   [Value::Index(ix1), Value::MatrixIndex(Matrix::DVector(ix2))]) => {
              Ok(Box::new(Access2DSVDM2{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(RowDVector::from_element(ix2.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(input)), [Value::Index(ix1), Value::MatrixIndex(Matrix::DVector(ix2))]) => {
              Ok(Box::new(Access2DSVDM3x2{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(RowDVector::from_element(ix2.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(input)), [Value::Index(ix1), Value::MatrixIndex(Matrix::DVector(ix2))]) => {
              Ok(Box::new(Access2DSVDM2x3{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(RowDVector::from_element(ix2.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::$matrix_kind(Matrix::DMatrix(input)),   [Value::Index(ix1), Value::MatrixIndex(Matrix::DVector(ix2))]) => {
              Ok(Box::new(Access2DSVDMD{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(RowDVector::from_element(ix2.borrow().len(),$default)) }))
            },
            // Bool Scalar Vector
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix4(input)),   [Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))]) => {
              Ok(Box::new(Access2DSVDbM4{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(RowDVector::from_element(ix2.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix3(input)),   [Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))]) => {
              Ok(Box::new(Access2DSVDbM3{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(RowDVector::from_element(ix2.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix2(input)),   [Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))]) => {
              Ok(Box::new(Access2DSVDbM2{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(RowDVector::from_element(ix2.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(input)), [Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))]) => {
              Ok(Box::new(Access2DSVDbM3x2{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(RowDVector::from_element(ix2.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(input)), [Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))]) => {
              Ok(Box::new(Access2DSVDbM2x3{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(RowDVector::from_element(ix2.borrow().len(),$default)) }))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "logical_indexing"))]
            (Value::$matrix_kind(Matrix::DMatrix(input)),   [Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))]) => {
              Ok(Box::new(Access2DSVDbMD{source: input.clone(), ix1: ix1.clone(), ix2: ix2.clone(), out: Ref::new(RowDVector::from_element(ix2.borrow().len(),$default)) }))
            },)+
        )+
        (src,ix) => Err(MechError::new(UnhandledFunctionArgumentIxesMono{ arg: (src.kind(), ix.iter().map(|x| x.kind()).collect()), fxn_name: stringify!($fxn_name).to_string() }, None).with_compiler_loc()),
      }
    }
  }
}

fn impl_access_scalar_range_fxn(
    lhs_value: Value,
    ixes: Vec<Value>,
) -> MResult<Box<dyn MechFunction>> {
    impl_access_match_arms!(Access2DSR, scalar_range, (lhs_value, ixes.as_slice()))
}

pub struct MatrixAccessScalarRange {}

impl FunctionSpecializer for MatrixAccessScalarRange {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() <= 2 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let ixes = arguments[1..].to_vec();
        let mat = arguments[0].clone();
        match impl_access_scalar_range_fxn(mat.clone(), ixes.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (mat.clone(), ixes.clone()) {
                (Value::MutableReference(lhs), rhs_value) => {
                    impl_access_scalar_range_fxn(lhs.borrow().clone(), rhs_value.clone())
                }
                x => Err(MechError::new(
                    UnhandledFunctionArgumentIxesMono {
                        arg: (mat.kind(), ixes.iter().map(|x| x.kind()).collect()),
                        fxn_name: "MatrixAccessScalarRange".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}

// Runtime catalog -----------------------------------------------------------

// Keep the scalar list in one place so the explicit catalog follows the same
// feature and legacy-name quirks as the source dispatch macros above. In
// particular, C64/R64 use c64/r64 in the one-type factory names, but the
// older multi-shape factories use complex/rational.
macro_rules! for_each_access_scalar {
    ($callback:ident, ($($args:tt)*)) => {
        #[cfg(feature = "bool")]
        $callback!($($args)*; bool, "bool", "bool");
        #[cfg(feature = "i8")]
        $callback!($($args)*; i8, "i8", "i8");
        #[cfg(feature = "i16")]
        $callback!($($args)*; i16, "i16", "i16");
        #[cfg(feature = "i32")]
        $callback!($($args)*; i32, "i32", "i32");
        #[cfg(feature = "i64")]
        $callback!($($args)*; i64, "i64", "i64");
        #[cfg(feature = "i128")]
        $callback!($($args)*; i128, "i128", "i128");
        #[cfg(feature = "u8")]
        $callback!($($args)*; u8, "u8", "u8");
        #[cfg(feature = "u16")]
        $callback!($($args)*; u16, "u16", "u16");
        #[cfg(feature = "u32")]
        $callback!($($args)*; u32, "u32", "u32");
        #[cfg(feature = "u64")]
        $callback!($($args)*; u64, "u64", "u64");
        #[cfg(feature = "u128")]
        $callback!($($args)*; u128, "u128", "u128");
        #[cfg(feature = "f32")]
        $callback!($($args)*; f32, "f32", "f32");
        #[cfg(feature = "f64")]
        $callback!($($args)*; f64, "f64", "f64");
        #[cfg(feature = "string")]
        $callback!($($args)*; String, "string", "string");
        #[cfg(feature = "complex")]
        $callback!($($args)*; C64, "c64", "complex");
        #[cfg(feature = "rational")]
        $callback!($($args)*; R64, "r64", "rational");
    };
}

// The access catalog deliberately derives its native declaration and runtime
// registration from the same scalar traversal.  Keeping the shape and scalar
// features beside the concrete implementation prevents a native build from
// silently selecting a broader profile than the factory it installs.
macro_rules! declare_access_typed_scalar {
    (
        $factory:ident,
        [$($feature:literal),+ $(,)?];
        $scalar:ident,
        $runtime_name:literal,
        $cargo_scalar:literal
    ) => {
        paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(
                    feature = "access",
                    feature = $cargo_scalar,
                    $(feature = $feature),+
                ),
                registration: [<register_ $factory:snake _ $scalar:lower>],
                installer: [<install_ $factory:snake _ $scalar:lower>],
                name: concat!(stringify!($factory), "<", $runtime_name, ">"),
                factory: <$factory<$scalar> as MechFunctionFactory>::new,
                package: "mech-engine",
                crate_name: "mech_engine",
                installer_path: concat!(
                    "mech_engine::__mech_native::install_",
                    stringify!([<$factory:snake>]),
                    "_",
                    stringify!([<$scalar:lower>]),
                ),
                cargo_features: [
                    "access",
                    "native-link",
                    "runtime",
                    $cargo_scalar,
                    $($feature),+
                ],
            }
        }
    };
}

macro_rules! declare_access_typed_family {
    ($factory:ident, [$($feature:literal),+ $(,)?]) => {
        for_each_access_scalar!(declare_access_typed_scalar, ($factory, [$($feature),+]));
    };
}

macro_rules! install_access_typed_scalar {
    ($builder:expr, $factory:ident; $scalar:ident, $runtime_name:literal, $cargo_scalar:literal) => {
        paste! {
            crate::intrinsics::access::matrix::native_declarations::[<register_ $factory:snake _ $scalar:lower>]($builder)?;
        }
    };
}

macro_rules! install_access_typed_scalars {
    ($builder:expr, $factory:ident) => {{
        #[inline(never)]
        fn install(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
            for_each_access_scalar!(install_access_typed_scalar, (builder, $factory));
            Ok(())
        }

        install($builder)?;
    }};
}

macro_rules! install_access_shape {
    ($builder:expr, $feature:literal, $family:ident, $shape:ident) => {
        #[cfg(feature = $feature)]
        paste! {
            install_access_typed_scalars!($builder, [<$family $shape>]);
        }
    };
}

macro_rules! declare_access_shape {
    ($family:ident, $shape:ident, $feature:literal) => {
        paste! {
            declare_access_typed_family!([<$family $shape>], [$feature]);
        }
    };
}

macro_rules! declare_access_logical_shape {
    ($family:ident, $shape:ident, $feature:literal) => {
        paste! {
            declare_access_typed_family!([<$family $shape>], ["logical_indexing", $feature]);
        }
    };
}

macro_rules! for_each_access_shape {
    ($callback:ident, ($family:ident)) => {
        $callback!($family, M1, "matrix1");
        $callback!($family, M2, "matrix2");
        $callback!($family, M3, "matrix3");
        $callback!($family, M4, "matrix4");
        $callback!($family, M2x3, "matrix2x3");
        $callback!($family, M3x2, "matrix3x2");
        $callback!($family, MD, "matrixd");
        $callback!($family, V2, "vector2");
        $callback!($family, V3, "vector3");
        $callback!($family, V4, "vector4");
        $callback!($family, VD, "vectord");
        $callback!($family, R2, "row_vector2");
        $callback!($family, R3, "row_vector3");
        $callback!($family, R4, "row_vector4");
        $callback!($family, RD, "row_vectord");
    };
}

macro_rules! for_each_access_shape_without_matrix1 {
    ($callback:ident, ($family:ident)) => {
        $callback!($family, M2, "matrix2");
        $callback!($family, M3, "matrix3");
        $callback!($family, M4, "matrix4");
        $callback!($family, M2x3, "matrix2x3");
        $callback!($family, M3x2, "matrix3x2");
        $callback!($family, MD, "matrixd");
        $callback!($family, V2, "vector2");
        $callback!($family, V3, "vector3");
        $callback!($family, V4, "vector4");
        $callback!($family, VD, "vectord");
        $callback!($family, R2, "row_vector2");
        $callback!($family, R3, "row_vector3");
        $callback!($family, R4, "row_vector4");
        $callback!($family, RD, "row_vectord");
    };
}

macro_rules! for_each_access_matrix_shape {
    ($callback:ident, ($family:ident)) => {
        $callback!($family, M2, "matrix2");
        $callback!($family, M3, "matrix3");
        $callback!($family, M4, "matrix4");
        $callback!($family, M2x3, "matrix2x3");
        $callback!($family, M3x2, "matrix3x2");
        $callback!($family, MD, "matrixd");
    };
}

macro_rules! install_access_all_shapes {
    ($builder:expr, $family:ident) => {
        install_access_shape!($builder, "matrix1", $family, M1);
        install_access_shape!($builder, "matrix2", $family, M2);
        install_access_shape!($builder, "matrix3", $family, M3);
        install_access_shape!($builder, "matrix4", $family, M4);
        install_access_shape!($builder, "matrix2x3", $family, M2x3);
        install_access_shape!($builder, "matrix3x2", $family, M3x2);
        install_access_shape!($builder, "matrixd", $family, MD);
        install_access_shape!($builder, "vector2", $family, V2);
        install_access_shape!($builder, "vector3", $family, V3);
        install_access_shape!($builder, "vector4", $family, V4);
        install_access_shape!($builder, "vectord", $family, VD);
        install_access_shape!($builder, "row_vector2", $family, R2);
        install_access_shape!($builder, "row_vector3", $family, R3);
        install_access_shape!($builder, "row_vector4", $family, R4);
        install_access_shape!($builder, "row_vectord", $family, RD);
    };
}

macro_rules! install_access_shapes_without_matrix1 {
    ($builder:expr, $family:ident) => {
        install_access_shape!($builder, "matrix2", $family, M2);
        install_access_shape!($builder, "matrix3", $family, M3);
        install_access_shape!($builder, "matrix4", $family, M4);
        install_access_shape!($builder, "matrix2x3", $family, M2x3);
        install_access_shape!($builder, "matrix3x2", $family, M3x2);
        install_access_shape!($builder, "matrixd", $family, MD);
        install_access_shape!($builder, "vector2", $family, V2);
        install_access_shape!($builder, "vector3", $family, V3);
        install_access_shape!($builder, "vector4", $family, V4);
        install_access_shape!($builder, "vectord", $family, VD);
        install_access_shape!($builder, "row_vector2", $family, R2);
        install_access_shape!($builder, "row_vector3", $family, R3);
        install_access_shape!($builder, "row_vector4", $family, R4);
        install_access_shape!($builder, "row_vectord", $family, RD);
    };
}

macro_rules! install_access_matrix_shapes {
    ($builder:expr, $family:ident) => {
        install_access_shape!($builder, "matrix2", $family, M2);
        install_access_shape!($builder, "matrix3", $family, M3);
        install_access_shape!($builder, "matrix4", $family, M4);
        install_access_shape!($builder, "matrix2x3", $family, M2x3);
        install_access_shape!($builder, "matrix3x2", $family, M3x2);
        install_access_shape!($builder, "matrixd", $family, MD);
    };
}

macro_rules! declare_access_range_range_scalar {
    (
        $factory:ident,
        $output:ident,
        $input:ident,
        $ix1:ident,
        $ix1_scalar:ident,
        $ix2:ident,
        $ix2_scalar:ident,
        [$($feature:literal),+ $(,)?];
        $scalar:ident,
        $runtime_name:literal,
        $cargo_scalar:literal
    ) => {
        paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(
                    feature = "access",
                    feature = $cargo_scalar,
                    $(feature = $feature),+
                ),
                registration: [<register_ $factory:snake _ $output:snake _ $input:snake _ $ix1:snake _ $ix2:snake _ $scalar:lower>],
                installer: [<install_ $factory:snake _ $output:snake _ $input:snake _ $ix1:snake _ $ix2:snake _ $scalar:lower>],
                name: concat!(
                    stringify!($factory),
                    "<",
                    $cargo_scalar,
                    stringify!($output),
                    stringify!($input),
                    stringify!($ix1),
                    stringify!($ix2),
                    ">"
                ),
                factory: <$factory<
                    $scalar,
                    $output<$scalar>,
                    $input<$scalar>,
                    $ix1<$ix1_scalar>,
                    $ix2<$ix2_scalar>,
                > as MechFunctionFactory>::new,
                package: "mech-engine",
                crate_name: "mech_engine",
                installer_path: concat!(
                    "mech_engine::__mech_native::install_",
                    stringify!([<$factory:snake _ $output:snake _ $input:snake _ $ix1:snake _ $ix2:snake _ $scalar:lower>]),
                ),
                cargo_features: [
                    "access",
                    "native-link",
                    "runtime",
                    $cargo_scalar,
                    $($feature),+
                ],
            }
        }
    };
}

macro_rules! declare_access_range_range_family {
    (
        $factory:ident,
        $output:ident,
        $input:ident,
        $ix1:ident,
        $ix1_scalar:ident,
        $ix2:ident,
        $ix2_scalar:ident,
        [$($feature:literal),+ $(,)?]
    ) => {
        for_each_access_scalar!(
            declare_access_range_range_scalar,
            (
                $factory,
                $output,
                $input,
                $ix1,
                $ix1_scalar,
                $ix2,
                $ix2_scalar,
                [$($feature),+]
            )
        );
    };
}

macro_rules! declare_access_all_range_scalar {
    (
        $factory:ident,
        $output:ident,
        $input:ident,
        $ix:ident,
        $ix_scalar:ident,
        [$($feature:literal),+ $(,)?];
        $scalar:ident,
        $runtime_name:literal,
        $cargo_scalar:literal
    ) => {
        paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(
                    feature = "access",
                    feature = $cargo_scalar,
                    $(feature = $feature),+
                ),
                registration: [<register_ $factory:snake _ $output:snake _ $input:snake _ $ix:snake _ $scalar:lower>],
                installer: [<install_ $factory:snake _ $output:snake _ $input:snake _ $ix:snake _ $scalar:lower>],
                name: concat!(
                    stringify!($factory),
                    "<",
                    $cargo_scalar,
                    stringify!($output),
                    stringify!($input),
                    stringify!($ix),
                    ">"
                ),
                factory: <$factory<
                    $scalar,
                    $output<$scalar>,
                    $input<$scalar>,
                    $ix<$ix_scalar>,
                > as MechFunctionFactory>::new,
                package: "mech-engine",
                crate_name: "mech_engine",
                installer_path: concat!(
                    "mech_engine::__mech_native::install_",
                    stringify!([<$factory:snake _ $output:snake _ $input:snake _ $ix:snake _ $scalar:lower>]),
                ),
                cargo_features: [
                    "access",
                    "native-link",
                    "runtime",
                    $cargo_scalar,
                    $($feature),+
                ],
            }
        }
    };
}

macro_rules! declare_access_all_range_family {
    (
        $factory:ident,
        $output:ident,
        $input:ident,
        $ix:ident,
        $ix_scalar:ident,
        [$($feature:literal),+ $(,)?]
    ) => {
        for_each_access_scalar!(
            declare_access_all_range_scalar,
            ($factory, $output, $input, $ix, $ix_scalar, [$($feature),+])
        );
    };
}

macro_rules! install_access_range_range_scalar {
    (
        $builder:expr,
        $factory:ident,
        $output:ident,
        $input:ident,
        $ix1:ident,
        $ix1_scalar:ident,
        $ix2:ident,
        $ix2_scalar:ident;
        $scalar:ident,
        $runtime_name:literal,
        $assign_name:literal
    ) => {
        paste! {
            crate::intrinsics::access::matrix::native_declarations::[<register_ $factory:snake _ $output:snake _ $input:snake _ $ix1:snake _ $ix2:snake _ $scalar:lower>]($builder)?;
        }
    };
}

macro_rules! install_access_all_range_scalar {
    (
        $builder:expr,
        $factory:ident,
        $output:ident,
        $input:ident,
        $ix:ident,
        $ix_scalar:ident;
        $scalar:ident,
        $runtime_name:literal,
        $assign_name:literal
    ) => {
        paste! {
            crate::intrinsics::access::matrix::native_declarations::[<register_ $factory:snake _ $output:snake _ $input:snake _ $ix:snake _ $scalar:lower>]($builder)?;
        }
    };
}

macro_rules! install_access_dynamic_for_shape {
    ($builder:expr, $shape:ident) => {
        #[cfg(all(feature = "matrixd", feature = "vectord"))]
        for_each_access_scalar!(
            install_access_range_range_scalar,
            (
                $builder,
                Access2DRRVUU,
                DMatrix,
                $shape,
                DVector,
                usize,
                DVector,
                usize
            )
        );

        // The legacy bool/bool match arm required all three dynamic output
        // shapes even though it registered each output independently.
        #[cfg(all(
            feature = "matrixd",
            feature = "vectord",
            feature = "row_vectord",
            feature = "logical_indexing"
        ))]
        {
            for_each_access_scalar!(
                install_access_range_range_scalar,
                (
                    $builder,
                    Access2DRRVBB,
                    DMatrix,
                    $shape,
                    DVector,
                    bool,
                    DVector,
                    bool
                )
            );
            for_each_access_scalar!(
                install_access_range_range_scalar,
                (
                    $builder,
                    Access2DRRVBB,
                    DVector,
                    $shape,
                    DVector,
                    bool,
                    DVector,
                    bool
                )
            );
            for_each_access_scalar!(
                install_access_range_range_scalar,
                (
                    $builder,
                    Access2DRRVBB,
                    RowDVector,
                    $shape,
                    DVector,
                    bool,
                    DVector,
                    bool
                )
            );
        }

        #[cfg(all(feature = "matrixd", feature = "vectord", feature = "logical_indexing"))]
        for_each_access_scalar!(
            install_access_range_range_scalar,
            (
                $builder,
                Access2DRRVUB,
                DMatrix,
                $shape,
                DVector,
                usize,
                DVector,
                bool
            )
        );
        #[cfg(all(feature = "vectord", feature = "logical_indexing"))]
        for_each_access_scalar!(
            install_access_range_range_scalar,
            (
                $builder,
                Access2DRRVUB,
                DVector,
                $shape,
                DVector,
                usize,
                DVector,
                bool
            )
        );
        #[cfg(all(
            feature = "vectord",
            feature = "row_vectord",
            feature = "logical_indexing"
        ))]
        for_each_access_scalar!(
            install_access_range_range_scalar,
            (
                $builder,
                Access2DRRVUB,
                RowDVector,
                $shape,
                DVector,
                usize,
                DVector,
                bool
            )
        );

        #[cfg(all(feature = "matrixd", feature = "vectord", feature = "logical_indexing"))]
        for_each_access_scalar!(
            install_access_range_range_scalar,
            (
                $builder,
                Access2DRRVBU,
                DMatrix,
                $shape,
                DVector,
                bool,
                DVector,
                usize
            )
        );
        #[cfg(all(feature = "vectord", feature = "logical_indexing"))]
        for_each_access_scalar!(
            install_access_range_range_scalar,
            (
                $builder,
                Access2DRRVBU,
                DVector,
                $shape,
                DVector,
                bool,
                DVector,
                usize
            )
        );
        #[cfg(all(
            feature = "vectord",
            feature = "row_vectord",
            feature = "logical_indexing"
        ))]
        for_each_access_scalar!(
            install_access_range_range_scalar,
            (
                $builder,
                Access2DRRVBU,
                RowDVector,
                $shape,
                DVector,
                bool,
                DVector,
                usize
            )
        );

        #[cfg(all(feature = "row_vectord", feature = "vectord"))]
        for_each_access_scalar!(
            install_access_all_range_scalar,
            ($builder, Access2DARV, RowDVector, $shape, DVector, usize)
        );
        #[cfg(all(feature = "matrixd", feature = "vectord"))]
        for_each_access_scalar!(
            install_access_all_range_scalar,
            ($builder, Access2DARV, DMatrix, $shape, DVector, usize)
        );

        // This row-vector bool case intentionally lacked logical_indexing in
        // the legacy registration; preserve that source-visible quirk.
        #[cfg(all(feature = "row_vectord", feature = "vectord"))]
        for_each_access_scalar!(
            install_access_all_range_scalar,
            ($builder, Access2DARVB, RowDVector, $shape, DVector, bool)
        );
        #[cfg(all(feature = "matrixd", feature = "vectord", feature = "logical_indexing"))]
        {
            for_each_access_scalar!(
                install_access_all_range_scalar,
                ($builder, Access2DARVB, DVector, $shape, DVector, bool)
            );
            for_each_access_scalar!(
                install_access_all_range_scalar,
                ($builder, Access2DARVB, DMatrix, $shape, DVector, bool)
            );
        }
    };
}

macro_rules! install_access_dynamic_shape {
    ($builder:expr, $feature:literal, $shape:ident) => {{
        #[cfg(feature = $feature)]
        {
            #[inline(never)]
            fn install(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
                install_access_dynamic_for_shape!(builder, $shape);
                Ok(())
            }

            install($builder)?;
        }
    }};
}

macro_rules! declare_access_dynamic_for_shape {
    ($shape:ident, $shape_feature:literal) => {
        declare_access_range_range_family!(
            Access2DRRVUU,
            DMatrix,
            $shape,
            DVector,
            usize,
            DVector,
            usize,
            ["matrixd", "vectord", $shape_feature]
        );

        declare_access_range_range_family!(
            Access2DRRVBB,
            DMatrix,
            $shape,
            DVector,
            bool,
            DVector,
            bool,
            [
                "bool",
                "matrixd",
                "vectord",
                "row_vectord",
                "logical_indexing",
                $shape_feature
            ]
        );
        declare_access_range_range_family!(
            Access2DRRVBB,
            DVector,
            $shape,
            DVector,
            bool,
            DVector,
            bool,
            [
                "bool",
                "matrixd",
                "vectord",
                "row_vectord",
                "logical_indexing",
                $shape_feature
            ]
        );
        declare_access_range_range_family!(
            Access2DRRVBB,
            RowDVector,
            $shape,
            DVector,
            bool,
            DVector,
            bool,
            [
                "bool",
                "matrixd",
                "vectord",
                "row_vectord",
                "logical_indexing",
                $shape_feature
            ]
        );

        declare_access_range_range_family!(
            Access2DRRVUB,
            DMatrix,
            $shape,
            DVector,
            usize,
            DVector,
            bool,
            [
                "bool",
                "matrixd",
                "vectord",
                "logical_indexing",
                $shape_feature
            ]
        );
        declare_access_range_range_family!(
            Access2DRRVUB,
            DVector,
            $shape,
            DVector,
            usize,
            DVector,
            bool,
            ["bool", "vectord", "logical_indexing", $shape_feature]
        );
        declare_access_range_range_family!(
            Access2DRRVUB,
            RowDVector,
            $shape,
            DVector,
            usize,
            DVector,
            bool,
            [
                "bool",
                "vectord",
                "row_vectord",
                "logical_indexing",
                $shape_feature
            ]
        );

        declare_access_range_range_family!(
            Access2DRRVBU,
            DMatrix,
            $shape,
            DVector,
            bool,
            DVector,
            usize,
            [
                "bool",
                "matrixd",
                "vectord",
                "logical_indexing",
                $shape_feature
            ]
        );
        declare_access_range_range_family!(
            Access2DRRVBU,
            DVector,
            $shape,
            DVector,
            bool,
            DVector,
            usize,
            ["bool", "vectord", "logical_indexing", $shape_feature]
        );
        declare_access_range_range_family!(
            Access2DRRVBU,
            RowDVector,
            $shape,
            DVector,
            bool,
            DVector,
            usize,
            [
                "bool",
                "vectord",
                "row_vectord",
                "logical_indexing",
                $shape_feature
            ]
        );

        declare_access_all_range_family!(
            Access2DARV,
            RowDVector,
            $shape,
            DVector,
            usize,
            ["row_vectord", "vectord", $shape_feature]
        );
        declare_access_all_range_family!(
            Access2DARV,
            DMatrix,
            $shape,
            DVector,
            usize,
            ["matrixd", "vectord", $shape_feature]
        );

        declare_access_all_range_family!(
            Access2DARVB,
            RowDVector,
            $shape,
            DVector,
            bool,
            ["bool", "row_vectord", "vectord", $shape_feature]
        );
        declare_access_all_range_family!(
            Access2DARVB,
            DVector,
            $shape,
            DVector,
            bool,
            [
                "bool",
                "matrixd",
                "vectord",
                "logical_indexing",
                $shape_feature
            ]
        );
        declare_access_all_range_family!(
            Access2DARVB,
            DMatrix,
            $shape,
            DVector,
            bool,
            [
                "bool",
                "matrixd",
                "vectord",
                "logical_indexing",
                $shape_feature
            ]
        );
    };
}

pub(crate) mod native_declarations {
    use super::*;

    for_each_access_shape!(declare_access_shape, (Access1DS));
    for_each_access_shape_without_matrix1!(declare_access_shape, (Access2DSS));
    for_each_access_shape!(declare_access_shape, (Access1DVD));
    for_each_access_shape_without_matrix1!(declare_access_shape, (Access1DA));

    #[cfg(feature = "logical_indexing")]
    for_each_access_shape!(declare_access_shape, (Access1DVDb));

    for_each_access_matrix_shape!(declare_access_shape, (Access2DAS));
    for_each_access_matrix_shape!(declare_access_shape, (Access2DVDA));
    for_each_access_matrix_shape!(declare_access_shape, (Access2DVDS));
    for_each_access_matrix_shape!(declare_access_shape, (Access2DSVD));

    #[cfg(feature = "logical_indexing")]
    for_each_access_matrix_shape!(declare_access_shape, (Access2DVDbA));
    #[cfg(feature = "logical_indexing")]
    for_each_access_matrix_shape!(declare_access_shape, (Access2DVDbS));
    #[cfg(feature = "logical_indexing")]
    for_each_access_matrix_shape!(declare_access_shape, (Access2DSVDb));

    declare_access_typed_family!(Access2DSAM1, ["matrix1"]);
    declare_access_typed_family!(Access2DSAM2, ["matrix2", "row_vector2"]);
    declare_access_typed_family!(Access2DSAM3, ["matrix3", "row_vector3"]);
    declare_access_typed_family!(Access2DSAM4, ["matrix4", "row_vector4"]);
    declare_access_typed_family!(Access2DSAM2x3, ["matrix2x3", "row_vector3"]);
    declare_access_typed_family!(Access2DSAM3x2, ["matrix3x2", "row_vector2"]);
    declare_access_typed_family!(Access2DSAMD, ["matrixd", "row_vectord"]);

    declare_access_dynamic_for_shape!(Matrix1, "matrix1");
    declare_access_dynamic_for_shape!(Matrix2, "matrix2");
    declare_access_dynamic_for_shape!(Matrix3, "matrix3");
    declare_access_dynamic_for_shape!(Matrix4, "matrix4");
    declare_access_dynamic_for_shape!(Matrix2x3, "matrix2x3");
    declare_access_dynamic_for_shape!(Matrix3x2, "matrix3x2");
    declare_access_dynamic_for_shape!(DMatrix, "matrixd");
    declare_access_dynamic_for_shape!(Vector2, "vector2");
    declare_access_dynamic_for_shape!(Vector3, "vector3");
    declare_access_dynamic_for_shape!(Vector4, "vector4");
    declare_access_dynamic_for_shape!(DVector, "vectord");
    declare_access_dynamic_for_shape!(RowVector2, "row_vector2");
    declare_access_dynamic_for_shape!(RowVector3, "row_vector3");
    declare_access_dynamic_for_shape!(RowVector4, "row_vector4");
    declare_access_dynamic_for_shape!(RowDVector, "row_vectord");
}

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    pub use super::native_declarations::*;
}

pub(super) fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_access_all_shapes!(builder, Access1DS);
    install_access_shapes_without_matrix1!(builder, Access2DSS);
    install_access_all_shapes!(builder, Access1DVD);
    install_access_shapes_without_matrix1!(builder, Access1DA);

    #[cfg(feature = "logical_indexing")]
    install_access_all_shapes!(builder, Access1DVDb);

    install_access_matrix_shapes!(builder, Access2DAS);
    install_access_matrix_shapes!(builder, Access2DVDA);
    install_access_matrix_shapes!(builder, Access2DVDS);
    install_access_matrix_shapes!(builder, Access2DSVD);

    #[cfg(feature = "logical_indexing")]
    {
        install_access_matrix_shapes!(builder, Access2DVDbA);
        install_access_matrix_shapes!(builder, Access2DVDbS);
        install_access_matrix_shapes!(builder, Access2DSVDb);
    }

    #[cfg(feature = "matrix1")]
    install_access_typed_scalars!(builder, Access2DSAM1);
    #[cfg(all(feature = "matrix2", feature = "row_vector2"))]
    install_access_typed_scalars!(builder, Access2DSAM2);
    #[cfg(all(feature = "matrix3", feature = "row_vector3"))]
    install_access_typed_scalars!(builder, Access2DSAM3);
    #[cfg(all(feature = "matrix4", feature = "row_vector4"))]
    install_access_typed_scalars!(builder, Access2DSAM4);
    #[cfg(all(feature = "matrix2x3", feature = "row_vector3"))]
    install_access_typed_scalars!(builder, Access2DSAM2x3);
    #[cfg(all(feature = "matrix3x2", feature = "row_vector2"))]
    install_access_typed_scalars!(builder, Access2DSAM3x2);
    #[cfg(all(feature = "matrixd", feature = "row_vectord"))]
    install_access_typed_scalars!(builder, Access2DSAMD);

    install_access_dynamic_shape!(builder, "matrix1", Matrix1);
    install_access_dynamic_shape!(builder, "matrix2", Matrix2);
    install_access_dynamic_shape!(builder, "matrix3", Matrix3);
    install_access_dynamic_shape!(builder, "matrix4", Matrix4);
    install_access_dynamic_shape!(builder, "matrix2x3", Matrix2x3);
    install_access_dynamic_shape!(builder, "matrix3x2", Matrix3x2);
    install_access_dynamic_shape!(builder, "matrixd", DMatrix);
    install_access_dynamic_shape!(builder, "vector2", Vector2);
    install_access_dynamic_shape!(builder, "vector3", Vector3);
    install_access_dynamic_shape!(builder, "vector4", Vector4);
    install_access_dynamic_shape!(builder, "vectord", DVector);
    install_access_dynamic_shape!(builder, "row_vector2", RowVector2);
    install_access_dynamic_shape!(builder, "row_vector3", RowVector3);
    install_access_dynamic_shape!(builder, "row_vector4", RowVector4);
    install_access_dynamic_shape!(builder, "row_vectord", RowDVector);

    Ok(())
}
