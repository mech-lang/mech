#[macro_use]
use crate::*;

use std::fmt::Debug;
use std::marker::PhantomData;

#[cfg(feature = "matrix")]
use nalgebra::{
  base::{Matrix as naMatrix, Storage, StorageMut},
  Dim, Scalar,
};

#[cfg(feature = "add_assign")]
pub mod add_assign;
#[cfg(feature = "sub_assign")]
pub mod sub_assign;
#[cfg(feature = "div_assign")]
pub mod div_assign;
#[cfg(feature = "mul_assign")]
pub mod mul_assign;

#[cfg(feature = "add_assign")]
pub use self::add_assign::*;
#[cfg(feature = "sub_assign")]
pub use self::sub_assign::*;
#[cfg(feature = "div_assign")]
pub use self::div_assign::*;
#[cfg(feature = "mul_assign")]
pub use self::mul_assign::*;

#[macro_export]
macro_rules! impl_op_assign_range_fxn_s {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    #[derive(Debug)]
    pub struct $struct_name<T, MatA, IxVec> {
      pub source: Ref<T>,
      pub ixes: Ref<IxVec>,
      pub sink: Ref<MatA>,
      pub _marker: PhantomData<T>,
    }
    impl<T, R1: 'static, C1: 'static, S1: 'static, IxVec: 'static> MechFunctionFactory for $struct_name<T, naMatrix<T, R1, C1, S1>, IxVec>
    where
      Ref<naMatrix<T, R1, C1, S1>>: ToValue,
      T: Copy + Debug + Clone + Sync + Send + 'static +
        Div<Output = T> + DivAssign +
        Add<Output = T> + AddAssign +
        Sub<Output = T> + SubAssign +
        Mul<Output = T> + MulAssign +
        Zero + One +
        PartialEq + PartialOrd +
        CompileConst + ConstElem + AsValueKind,
      IxVec: CompileConst + ConstElem + Debug + AsRef<[$ix]> + AsNaKind,
      R1: Dim, C1: Dim, S1: StorageMut<T, R1, C1> + Clone + Debug,
      naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + Debug + AsNaKind,
    {
      fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
          FunctionArgs::Binary(out, arg1, arg2) => {
            let source: Ref<T> = unsafe { arg1.as_unchecked() }.clone();
            let ixes: Ref<IxVec> = unsafe { arg2.as_unchecked() }.clone();
            let sink: Ref<naMatrix<T, R1, C1, S1>> = unsafe { out.as_unchecked() }.clone();
            Ok(Box::new(Self { sink, source, ixes, _marker: PhantomData::default() }))
          },
          _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{} requires 3 arguments, got {:?}", stringify!($struct_name), args), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments})
        }
      }
    }
    impl<T, R1, C1, S1, IxVec> MechFunctionImpl for $struct_name<T, naMatrix<T, R1, C1, S1>, IxVec>
    where
      Ref<naMatrix<T, R1, C1, S1>>: ToValue,
      T: Copy + Debug + Clone + Sync + Send + 'static +
        Div<Output = T> + DivAssign +
        Add<Output = T> + AddAssign +
        Sub<Output = T> + SubAssign +
        Mul<Output = T> + MulAssign +
        Zero + One +
        PartialEq + PartialOrd,
      IxVec: AsRef<[$ix]> + Debug,
      R1: Dim, C1: Dim, S1: StorageMut<T, R1, C1> + Clone + Debug,
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
    impl<T, R1, C1, S1, IxVec> MechFunctionCompiler for $struct_name<T, naMatrix<T, R1, C1, S1>, IxVec> 
    where
      T: CompileConst + ConstElem + AsValueKind,
      IxVec: CompileConst + ConstElem + AsNaKind,
      naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + AsNaKind,
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let name = format!("{}<{}{}{}>", stringify!($struct_name), T::as_value_kind(), naMatrix::<T, R1, C1, S1>::as_na_kind(), IxVec::as_na_kind());
        compile_binop!(name, self.sink, self.source, self.ixes, ctx, FeatureFlag::Builtin(FeatureKind::OpAssign));
      }
    }};}

#[macro_export]
macro_rules! impl_op_assign_range_fxn_v {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    #[cfg(feature = "matrix")]
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
      T: Copy + Debug + Clone + Sync + Send + 'static +
        Div<Output = T> + DivAssign +
        Add<Output = T> + AddAssign +
        Sub<Output = T> + SubAssign +
        Mul<Output = T> + MulAssign +
        Zero + One +
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
      T: Copy + Debug + Clone + Sync + Send + 'static +
        Div<Output = T> + DivAssign +
        Add<Output = T> + AddAssign +
        Sub<Output = T> + SubAssign +
        Mul<Output = T> + MulAssign +
        Zero + One +
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
  };}

//impl_set_range_arms
#[macro_export]
macro_rules! op_assign_range_fxn {
  ($op_fxn_name:tt, $fxn_name:ident) => {
    paste::paste! {
      fn $op_fxn_name(sink: Value, source: Value, ixes: Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        let arg = (sink, ixes.as_slice(), source);
                     impl_assign_fxn!(impl_set_range_arms, $fxn_name, arg, u8, "u8")
        .or_else(|_| impl_assign_fxn!(impl_set_range_arms, $fxn_name, arg, u16, "u16"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_arms, $fxn_name, arg, u32, "u32"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_arms, $fxn_name, arg, u64, "u64"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_arms, $fxn_name, arg, u128, "u128"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_arms, $fxn_name, arg, i8, "i8"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_arms, $fxn_name, arg, i16, "i16"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_arms, $fxn_name, arg, i32, "i32"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_arms, $fxn_name, arg, i64, "i64"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_arms, $fxn_name, arg, F32, "f32"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_arms, $fxn_name, arg, F64, "f64"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_arms, $fxn_name, arg, R64, "rational"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_arms, $fxn_name, arg, C64, "complex"))
        .map_err(|_| MechError { file: file!().to_string(), tokens: vec![], msg: format!("Unsupported argument: {:?}", &arg), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
      }
    }
  }
}

//impl_set_range_all_arms
#[macro_export]
macro_rules! op_assign_range_all_fxn {
  ($op_fxn_name:tt, $fxn_name:ident) => {
    paste::paste! {
      fn $op_fxn_name(sink: Value, source: Value, ixes: Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        let arg = (sink, ixes.as_slice(), source);
                     impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, u8, "u8")
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, u16, "u16"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, u32, "u32"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, u64, "u64"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, u128, "u128"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, i8, "i8"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, i16, "i16"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, i32, "i32"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, i64, "i64"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, F32, "f32"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, F64, "f64"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, R64, "rational"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, C64, "complex"))
        .map_err(|_| MechError { file: file!().to_string(), tokens: vec![], msg: format!("Unsupported argument: {:?}", &arg), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind})
      }
    }
  }
}

#[macro_export]
macro_rules! impl_assign_scalar_scalar {
  ($op_name:tt, $op_fn:tt) => {
    paste::paste! {
      #[derive(Debug)]
      struct [<$op_name AssignSS>]<T> {
        sink: Ref<T>,
        source: Ref<T>,
      }
      impl<T> MechFunctionFactory for [<$op_name AssignSS>]<T>
      where
        T: Debug + Clone + Sync + Send + 'static +
           $op_name<Output = T> + [<$op_name Assign>] +
           PartialEq + PartialOrd + CompileConst + ConstElem + AsValueKind,
        Ref<T>: ToValue
      {
        fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
          match args {
            FunctionArgs::Unary(out, arg1) => {
              let source: Ref<T> = unsafe { arg1.as_unchecked() }.clone();
              let sink: Ref<T> = unsafe { out.as_unchecked() }.clone();
              Ok(Box::new(Self { sink, source }))
            },
            _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{} requires 2 arguments, got {:?}", stringify!($struct_name), args), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments})
          }    
        }    
      }
      impl<T> MechFunctionImpl for [<$op_name AssignSS>]<T>
      where
        T: Debug + Clone + Sync + Send + 'static +
           $op_name<Output = T> + [<$op_name Assign>] +
           PartialEq + PartialOrd,
        Ref<T>: ToValue
      {
        fn solve(&self) {
          let sink_ptr = self.sink.as_mut_ptr();
          let source_ptr = self.source.as_ptr();
          unsafe {
            *sink_ptr $op_fn (*source_ptr).clone();
          }
        }
        fn out(&self) -> Value { self.sink.to_value() }
        fn to_string(&self) -> String { format!("{:#?}", self) }
      }
      #[cfg(feature = "compiler")]
      impl<T> MechFunctionCompiler for [<$op_name AssignSS>]<T> 
      where
        T: CompileConst + ConstElem + AsValueKind,
      {
        fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
          let name = format!("{}AssignSS<{}>", stringify!($op_name), T::as_value_kind());
          compile_unop!(name, self.sink, self.source, ctx, FeatureFlag::Builtin(FeatureKind::Assign) );
        }
      }
      register_fxn_descriptor!([<$op_name AssignSS>], 
        u8, "u8", 
        u16, "u16",
        u32, "u32",
        u64, "u64",
        u128, "u128",
        i8, "i8",
        i16, "i16",
        i32, "i32",
        i64, "i64",
        i128, "i128",
        F32, "f32",
        F64, "f64",
        R64, "r64",
        C64, "c64"
      );
    }
  };
}

#[macro_export]
macro_rules! impl_assign_vector_vector {
  ($op_name:tt, $op_fn:tt) => {
    paste::paste! {
      #[derive(Debug)]
      pub struct [<$op_name AssignVV>]<T, MatA, MatB> {
        pub sink: Ref<MatA>,
        pub source: Ref<MatB>,
        _marker: PhantomData<T>,
      }
      impl<T, MatA, MatB> MechFunctionFactory for [<$op_name AssignVV>]<T, MatA, MatB>
      where
        Ref<MatA>: ToValue,
        T: Debug + Clone + Sync + Send + 'static + [<$op_name Assign>] +
        CompileConst + ConstElem + AsValueKind,
        for<'a> &'a MatA: IntoIterator<Item = &'a T>,
        for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
        for<'a> &'a MatB: IntoIterator<Item = &'a T>,
        MatA: Debug + CompileConst + ConstElem + AsValueKind + 'static,
        MatB: Debug + CompileConst + ConstElem + AsValueKind + 'static,
      {
        fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
          match args {
            FunctionArgs::Unary(out, arg1) => {
              let source: Ref<MatB> = unsafe { arg1.as_unchecked() }.clone();
              let sink: Ref<MatA> = unsafe { out.as_unchecked() }.clone();
              Ok(Box::new(Self { sink, source, _marker: PhantomData::default() }))
            },
            _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{} requires 2 arguments, got {:?}", stringify!($struct_name), args), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments})
          }    
        }    
      }
      impl<T, MatA, MatB> MechFunctionImpl for [<$op_name AssignVV>]<T, MatA, MatB>
      where
        Ref<MatA>: ToValue,
        T: Debug + Clone + Sync + Send + 'static + [<$op_name Assign>],
        for<'a> &'a MatA: IntoIterator<Item = &'a T>,
        for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
        for<'a> &'a MatB: IntoIterator<Item = &'a T>,
        MatA: Debug,
        MatB: Debug,
      {
        fn solve(&self) {
          unsafe {
            let sink_ptr = self.sink.as_mut_ptr();
            let source_ptr = self.source.as_ptr();
            let sink_ref: &mut MatA = &mut *sink_ptr;
            let source_ref: &MatB = &*source_ptr;
            for (dst, src) in (&mut *sink_ref).into_iter().zip((&*source_ref).into_iter()) {
              *dst $op_fn src.clone();
            }
          }
        }
        fn out(&self) -> Value {self.sink.to_value()}
        fn to_string(&self) -> String {format!("{:#?}", self)}
      }
      #[cfg(feature = "compiler")]
      impl<T, MatA, MatB> MechFunctionCompiler for [<$op_name AssignVV>]<T, MatA, MatB> 
      where
        T: CompileConst + ConstElem + AsValueKind,
        MatA: CompileConst + ConstElem + AsValueKind,
        MatB: CompileConst + ConstElem + AsValueKind,
      {
        fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
          let name = format!("{}AssignVV<{}>", stringify!($op_name), MatA::as_value_kind());
          compile_unop!(name, self.sink, self.source, ctx, FeatureFlag::Builtin(FeatureKind::OpAssign) );
        }
      }
      impl_register_op_assign_vv_all!([<$op_name AssignVV>]);
    }
  };
}

#[macro_export]
macro_rules! impl_assign_vector_scalar {
  ($op_name:tt, $op_fn:tt) => {
    paste::paste! {
      #[derive(Debug)]
      pub struct [<$op_name AssignVS>]<T, MatA> {
        pub sink: Ref<MatA>,
        pub source: Ref<T>,
        _marker: PhantomData<T>,
      }
      impl<T, MatA> MechFunctionFactory for [<$op_name AssignVS>]<T, MatA>
      where
        Ref<MatA>: ToValue,
        T: Debug + Clone + Sync + Send + 'static + [<$op_name Assign>] +
        CompileConst + ConstElem + AsValueKind,
        for<'a> &'a MatA: IntoIterator<Item = &'a T>,
        for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
        MatA: Debug + CompileConst + ConstElem + AsValueKind + 'static,
      {
        fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
          match args {
            FunctionArgs::Binary(out, arg1, arg2) => {
              let source: Ref<T> = unsafe { arg2.as_unchecked() }.clone();
              let sink: Ref<MatA> = unsafe { out.as_unchecked() }.clone();
              Ok(Box::new(Self { sink, source, _marker: PhantomData::default() }))
            },
            _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{} requires 2 arguments, got {:?}", stringify!($struct_name), args), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments})
          }    
        }    
      }
      impl<T, MatA> MechFunctionImpl for [<$op_name AssignVS>]<T, MatA>
      where
        Ref<MatA>: ToValue,
        T: Debug + Clone + Sync + Send + 'static + [<$op_name Assign>],
        for<'a> &'a MatA: IntoIterator<Item = &'a T>,
        for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
        MatA: Debug,
      {
        fn solve(&self) {
          unsafe {
            let sink_ptr = self.sink.as_mut_ptr();
            let source_ptr = self.source.as_ptr();
            let sink_ref: &mut MatA = &mut *sink_ptr;
            let source_ref: &T = &*source_ptr;
            for dst in (&mut *sink_ref).into_iter() {
              *dst $op_fn source_ref.clone();
            }
          }
        }
        fn out(&self) -> Value {self.sink.to_value()}
        fn to_string(&self) -> String {format!("{:#?}", self)}
      }
      #[cfg(feature = "compiler")]
      impl<T, MatA> MechFunctionCompiler for [<$op_name AssignVS>]<T, MatA> 
      where
        T: CompileConst + ConstElem + AsValueKind,
        MatA: CompileConst + ConstElem + AsValueKind,
      {
        fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
          let name = format!("{}AssignVS<{}>", stringify!($op_name), MatA::as_value_kind());
          compile_unop!(name, self.sink, self.source, ctx, FeatureFlag::Builtin(FeatureKind::OpAssign) );
        }
      }
    }
  }
}

#[macro_export]
macro_rules! register_op_assign_vv {
  ($op:ident, $type:ty, $size:ty, $size_string:tt) => {
    paste!{
      inventory::submit! {
        FunctionDescriptor {
          name: concat!(stringify!($op),"<[",stringify!([<$type:lower>]),"]:", $size_string, ">"),
          ptr: $op::<$type,$size<$type>,$size<$type>>::new,
        }
      }
    }
  };}

#[macro_export]
macro_rules! register_op_assign_vv_all {
  ($op:ident, $ty:ty, $ty_feature:literal) => {
    #[cfg(feature = "row_vector4")]
    register_op_assign_vv!($op, $ty, RowVector4, "1,4");
    #[cfg(feature = "row_vector3")]
    register_op_assign_vv!($op, $ty, RowVector3, "1,3");
    #[cfg(feature = "row_vector2")]
    register_op_assign_vv!($op, $ty, RowVector2, "1,2");
    #[cfg(feature = "vector2")]
    register_op_assign_vv!($op, $ty, Vector2, "2,1");
    #[cfg(feature = "vector3")]
    register_op_assign_vv!($op, $ty, Vector3, "3,1");
    #[cfg(feature = "vector4")]
    register_op_assign_vv!($op, $ty, Vector4, "4,1");
    #[cfg(feature = "matrix1")]
    register_op_assign_vv!($op, $ty, Matrix1, "1,1");
    #[cfg(feature = "matrix2")]
    register_op_assign_vv!($op, $ty, Matrix2, "2,2");
    #[cfg(feature = "matrix3")]
    register_op_assign_vv!($op, $ty, Matrix3, "3,3");
    #[cfg(feature = "matrix4")]
    register_op_assign_vv!($op, $ty, Matrix4, "4,4");
    #[cfg(feature = "matrix2x3")]
    register_op_assign_vv!($op, $ty, Matrix2x3, "2,3");
    #[cfg(feature = "matrix3x2")]
    register_op_assign_vv!($op, $ty, Matrix3x2, "3,2");
    #[cfg(feature = "vectord")]
    register_op_assign_vv!($op, $ty, DVector, "0,1");
    #[cfg(feature = "matrixd")]
    register_op_assign_vv!($op, $ty, DMatrix, "0,0");
    #[cfg(feature = "row_vectord")]
    register_op_assign_vv!($op, $ty, RowDVector, "1,0");
  };
}

#[macro_export]
macro_rules! impl_register_op_assign_vv_all {
  ($macro_name:ident) => {
    #[cfg(feature = "u8")]
    register_op_assign_vv_all!($macro_name, u8, "u8");
    #[cfg(feature = "u16")]
    register_op_assign_vv_all!($macro_name, u16, "u16");
    #[cfg(feature = "u32")]
    register_op_assign_vv_all!($macro_name, u32, "u32");
    #[cfg(feature = "u64")]
    register_op_assign_vv_all!($macro_name, u64, "u64");
    #[cfg(feature = "u128")]
    register_op_assign_vv_all!($macro_name, u128, "u128");
    #[cfg(feature = "i8")]
    register_op_assign_vv_all!($macro_name, i8, "i8");
    #[cfg(feature = "i16")]
    register_op_assign_vv_all!($macro_name, i16, "i16");
    #[cfg(feature = "i32")]
    register_op_assign_vv_all!($macro_name, i32, "i32");
    #[cfg(feature = "i64")]
    register_op_assign_vv_all!($macro_name, i64, "i64");
    #[cfg(feature = "i128")]
    register_op_assign_vv_all!($macro_name, i128, "i128");
    #[cfg(feature = "f32")]
    register_op_assign_vv_all!($macro_name, F32, "f32");
    #[cfg(feature = "f64")]
    register_op_assign_vv_all!($macro_name, F64, "f64");
    #[cfg(feature = "r64")]
    register_op_assign_vv_all!($macro_name, R64, "r64");
    #[cfg(feature = "c64")]
    register_op_assign_vv_all!($macro_name, C64, "c64");
  };
}

#[macro_export]
macro_rules! impl_op_assign_value_match_arms {
  ($op:tt, $arg:expr,$($value_kind:ident, $feature:tt);+ $(;)?) => {
    paste::paste! {
      match $arg {
        $(
          #[cfg(feature = $feature)]
          (Value::$value_kind(sink), Value::$value_kind(source)) => Ok(Box::new([<$op AssignSS>]{ sink: sink.clone(), source: source.clone() })),
          #[cfg(all(feature = $feature, feature = "matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)), Value::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)), Value::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), Value::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), Value::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)), Value::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)), Value::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), Value::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "vector2"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector2(sink)), Value::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "vector3"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector3(sink)), Value::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)), Value::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "vectord"))]
          (Value::[<Matrix $value_kind>](Matrix::DVector(sink)), Value::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "row_vector2"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector2(sink)), Value::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})), 
          #[cfg(all(feature = $feature, feature = "row_vector3"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector3(sink)), Value::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})), 
          #[cfg(all(feature = $feature, feature = "row_vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)), Value::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})), 
          #[cfg(all(feature = $feature, feature = "row_vectord"))]
          (Value::[<Matrix $value_kind>](Matrix::RowDVector(sink)), Value::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})), 
        )+
        x => Err(MechError {file: file!().to_string(),tokens: vec![],msg: format!("Unhandled args {:?}", x),id: line!(),kind: MechErrorKind::UnhandledFunctionArgumentKind,}),
      }
    }
  };
}