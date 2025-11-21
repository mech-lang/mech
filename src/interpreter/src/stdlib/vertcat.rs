#[macro_use]
use crate::stdlib::*;

macro_rules! register_vertical_concatenate_fxn {
  ($name:ident) => {
    register_fxn_descriptor!(
      $name, 
      bool, "bool", 
      String, "string", 
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
      f32, "f32", 
      f64, "f64", 
      C64, "c64", 
      R64, "r64"
    );
  };
}

macro_rules! register_fxns {
  ($op:ident) => {
    $op!(bool, "bool");
    $op!(String, "string");
    $op!(u8, "u8");
    $op!(u16, "u16");
    $op!(u32, "u32");
    $op!(u64, "u64");
    $op!(u128, "u128");
    $op!(i8, "i8");
    $op!(i16, "i16");
    $op!(i32, "i32");
    $op!(i64, "i64");
    $op!(i128, "i128");
    $op!(f64, "f64");
    $op!(f32, "f32");
    $op!(R64, "r64");
    $op!(C64, "c64");
  }
}


// Vertical Concatenate -----------------------------------------------------

macro_rules! vertcat_two_args {
  ($fxn:ident, $e0:ident, $e1:ident, $out:ident, $opt:ident) => {
    #[derive(Debug)]
    struct $fxn<T> {
      e0: Ref<$e0<T>>,
      e1: Ref<$e1<T>>,
      out: Ref<$out<T>>,
    }
    impl<T> MechFunctionFactory for $fxn<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static +
      ConstElem + CompileConst + AsValueKind,
      Ref<$out<T>>: ToValue
    {
      fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
          FunctionArgs::Binary(out, arg0, arg1) => {
            let e0: Ref<$e0<T>> = unsafe { arg0.as_unchecked() }.clone();
            let e1: Ref<$e1<T>> = unsafe { arg1.as_unchecked() }.clone();
            let out: Ref<$out<T>> = unsafe { out.as_unchecked() }.clone();
            Ok(Box::new(Self { e0, e1, out }))
          },
          _ => Err(MechError2::new(IncorrectNumberOfArguments{expected: 2, found: args.len()}, None).with_compiler_loc())
        }
      }
    }
    impl<T> MechFunctionImpl for $fxn<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$out<T>>: ToValue
    {
      fn solve(&self) { 
        unsafe {
          let e0_ptr = (*(self.e0.as_ptr())).clone();
          let e1_ptr = (*(self.e1.as_ptr())).clone();
          let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
          $opt!(out_ptr, e0_ptr, e1_ptr);
        }
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }
    #[cfg(feature = "compiler")]
    impl<T> MechFunctionCompiler for $fxn<T> 
    where
      T: ConstElem + CompileConst + AsValueKind
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let name = format!("{}<{}{}{}{}>", stringify!($fxn), T::as_value_kind(), stringify!($out), stringify!($e0), stringify!($e1));
        compile_binop!(name, self.out, self.e0, self.e1, ctx, FeatureFlag::Builtin(FeatureKind::VertCat));
      }
    }
    macro_rules! register_vertcat_fxn {
      ($type:ty, $type_string:tt) => {
        paste!{ 
          #[cfg(feature = $type_string)]
          register_descriptor! {
            FunctionDescriptor {
            name: concat!(stringify!($fxn), "<", stringify!([<$type:lower>]), stringify!($out), stringify!($e0), stringify!($e1), ">"),
            ptr: $fxn::<$type>::new,
            }
          }
        }
      };
    }
    register_fxns!(register_vertcat_fxn);
  };
}

macro_rules! vertcat_three_args {
  ($fxn:ident, $e0:ident, $e1:ident, $e2:ident, $out:ident, $opt:ident) => {
    #[derive(Debug)]
    struct $fxn<T> {
      e0: Ref<$e0<T>>,
      e1: Ref<$e1<T>>,
      e2: Ref<$e2<T>>,
      out: Ref<$out<T>>,
    }
    impl<T> MechFunctionFactory for $fxn<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static +
      ConstElem + CompileConst + AsValueKind,
      Ref<$out<T>>: ToValue
    {
      fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
          FunctionArgs::Ternary(out, arg0, arg1, arg2) => {
            let e0: Ref<$e0<T>> = unsafe { arg0.as_unchecked() }.clone();
            let e1: Ref<$e1<T>> = unsafe { arg1.as_unchecked() }.clone();
            let e2: Ref<$e2<T>> = unsafe { arg2.as_unchecked() }.clone();
            let out: Ref<$out<T>> = unsafe { out.as_unchecked() }.clone();
            Ok(Box::new(Self { e0, e1, e2, out }))
          },
          _ => Err(MechError2::new(IncorrectNumberOfArguments{expected: 3, found: args.len()}, None).with_compiler_loc())
        }
      }
    }
    impl<T> MechFunctionImpl for $fxn<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$out<T>>: ToValue
    {
      fn solve(&self) { 
        unsafe {
          let e0_ptr = (*(self.e0.as_ptr())).clone();
          let e1_ptr = (*(self.e1.as_ptr())).clone();
          let e2_ptr = (*(self.e2.as_ptr())).clone();
          let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
          $opt!(out_ptr,e0_ptr,e1_ptr,e2_ptr);
        }
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }
    #[cfg(feature = "compiler")]
    impl<T> MechFunctionCompiler for $fxn<T> 
    where
      T: ConstElem + CompileConst + AsValueKind + AsValueKind
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let name = format!("{}<{}>", stringify!($fxn), T::as_value_kind());
        compile_ternop!(name, self.out, self.e0, self.e1, self.e2, ctx, FeatureFlag::Builtin(FeatureKind::VertCat));
      }
    }
    register_vertical_concatenate_fxn!($fxn);
  };} 
  
macro_rules! vertcat_four_args {
  ($fxn:ident, $e0:ident, $e1:ident, $e2:ident, $e3:ident, $out:ident, $opt:ident) => {
    #[derive(Debug)]
    struct $fxn<T> {
      e0: Ref<$e0<T>>,
      e1: Ref<$e1<T>>,
      e2: Ref<$e2<T>>,
      e3: Ref<$e3<T>>,
      out: Ref<$out<T>>,
    }
    impl<T> MechFunctionFactory for $fxn<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static +
      ConstElem + CompileConst + AsValueKind,
      Ref<$out<T>>: ToValue
    {
      fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
          FunctionArgs::Quaternary(out, arg0, arg1, arg2, arg3) => {
            let e0: Ref<$e0<T>> = unsafe { arg0.as_unchecked() }.clone();
            let e1: Ref<$e1<T>> = unsafe { arg1.as_unchecked() }.clone();
            let e2: Ref<$e2<T>> = unsafe { arg2.as_unchecked() }.clone();
            let e3: Ref<$e3<T>> = unsafe { arg3.as_unchecked() }.clone();
            let out: Ref<$out<T>> = unsafe { out.as_unchecked() }.clone();
            Ok(Box::new(Self { e0, e1, e2, e3, out }))
          },
          _ => Err(MechError2::new(IncorrectNumberOfArguments{expected: 4, found: args.len()}, None).with_compiler_loc())
        }
      }
    }
    impl<T> MechFunctionImpl for $fxn<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$out<T>>: ToValue
    {
      fn solve(&self) { 
        unsafe {
          let e0_ptr = (*(self.e0.as_ptr())).clone();
          let e1_ptr = (*(self.e1.as_ptr())).clone();
          let e2_ptr = (*(self.e2.as_ptr())).clone();
          let e3_ptr = (*(self.e3.as_ptr())).clone();
          let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
          $opt!(out_ptr,e0_ptr,e1_ptr,e2_ptr,e3_ptr);
        }
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }
    #[cfg(feature = "compiler")]
    impl<T> MechFunctionCompiler for $fxn<T> 
    where
      T: ConstElem + CompileConst + AsValueKind
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let name = format!("{}<{}>", stringify!($fxn), T::as_value_kind());
        compile_quadop!(name, self.out, self.e0, self.e1, self.e2, self.e3, ctx, FeatureFlag::Builtin(FeatureKind::VertCat));
      }
    }
    register_vertical_concatenate_fxn!($fxn);
  };}
  
// VerticalConcatenateTwoArgs -------------------------------------------------

#[cfg(feature = "matrixd")]
struct VerticalConcatenateTwoArgs<T> {
  e0: Box<dyn CopyMat<T>>,
  e1: Box<dyn CopyMat<T>>,
  out: Ref<DMatrix<T>>,
}
#[cfg(feature = "matrixd")]
impl<T> MechFunctionFactory for VerticalConcatenateTwoArgs<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static +
  ConstElem + CompileConst + AsValueKind,
  Ref<DMatrix<T>>: ToValue
{
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Binary(out, arg0, arg1) => {
        let e0: Box<dyn CopyMat<T>> = unsafe { arg0.get_copyable_matrix_unchecked::<T>() };
        let e1: Box<dyn CopyMat<T>> = unsafe { arg1.get_copyable_matrix_unchecked::<T>() };
        let out: Ref<DMatrix<T>> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(Self { e0, e1, out }))
      },
      _ => Err(MechError2::new(IncorrectNumberOfArguments{expected: 2, found: args.len()}, None).with_compiler_loc())
    }
  }
}
#[cfg(feature = "matrixd")]
impl<T> MechFunctionImpl for VerticalConcatenateTwoArgs<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DMatrix<T>>: ToValue
{
  fn solve(&self) {
    let offset = self.e0.copy_into_row_major(&self.out,0);
    self.e1.copy_into_row_major(&self.out,offset);
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("VerticalConcatenateTwoArgs\n{:#?}", self.out) }
}
#[cfg(feature = "matrixd")]
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for VerticalConcatenateTwoArgs<T> 
where
  T: ConstElem + CompileConst + AsValueKind + AsValueKind
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let mut registers = [0, 0, 0];

    registers[0] = compile_register!(self.out, ctx);
    registers[1] = compile_register_mat!(self.e0, ctx);
    registers[2] = compile_register_mat!(self.e1, ctx);

    ctx.features.insert(FeatureFlag::Builtin(FeatureKind::VertCat));

    ctx.emit_binop(
      hash_str(&format!("VerticalConcatenateTwoArgs<{}>", T::as_value_kind())),
      registers[0],
      registers[1],
      registers[2],
    );

    Ok(registers[0])    
  }
}
#[cfg(feature = "matrixd")]
register_vertical_concatenate_fxn!(VerticalConcatenateTwoArgs);

// VerticalConcatenateThreeArgs -----------------------------------------------
    
#[cfg(feature = "matrixd")]
struct VerticalConcatenateThreeArgs<T> {
  e0: Box<dyn CopyMat<T>>,
  e1: Box<dyn CopyMat<T>>,
  e2: Box<dyn CopyMat<T>>,
  out: Ref<DMatrix<T>>,
}
#[cfg(feature = "matrixd")]
impl<T> MechFunctionFactory for VerticalConcatenateThreeArgs<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static +
  ConstElem + CompileConst + AsValueKind,
  Ref<DMatrix<T>>: ToValue
{
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Ternary(out, arg0, arg1, arg2) => {
        let e0: Box<dyn CopyMat<T>> = unsafe { arg0.get_copyable_matrix_unchecked::<T>() };
        let e1: Box<dyn CopyMat<T>> = unsafe { arg1.get_copyable_matrix_unchecked::<T>() };
        let e2: Box<dyn CopyMat<T>> = unsafe { arg2.get_copyable_matrix_unchecked::<T>() };
        let out: Ref<DMatrix<T>> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(Self { e0, e1, e2, out }))
      },
      _ => Err(MechError2::new(IncorrectNumberOfArguments{expected: 3, found: args.len()}, None).with_compiler_loc())
    }
  }
}
#[cfg(feature = "matrixd")]
impl<T> MechFunctionImpl for VerticalConcatenateThreeArgs<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DMatrix<T>>: ToValue
{
  fn solve(&self) {
    let mut offset = self.e0.copy_into_row_major(&self.out,0);
    offset += self.e1.copy_into_row_major(&self.out,offset);
    self.e2.copy_into_row_major(&self.out,offset);
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("VerticalConcatenateThreeArgs\n{:#?}", self.out) }
}
#[cfg(feature = "matrixd")]
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for VerticalConcatenateThreeArgs<T> 
where
  T: ConstElem + CompileConst + AsValueKind + AsValueKind
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let mut registers = [0, 0, 0, 0];

    registers[0] = compile_register!(self.out, ctx);
    registers[1] = compile_register_mat!(self.e0, ctx);
    registers[2] = compile_register_mat!(self.e1, ctx);
    registers[3] = compile_register_mat!(self.e2, ctx);

    ctx.features.insert(FeatureFlag::Builtin(FeatureKind::VertCat));

    ctx.emit_ternop(
      hash_str(&format!("VerticalConcatenateThreeArgs<{}>", T::as_value_kind())),
      registers[0],
      registers[1],
      registers[2],
      registers[3],
    );
    Ok(registers[0])    
  }
}
#[cfg(feature = "matrixd")]
register_vertical_concatenate_fxn!(VerticalConcatenateThreeArgs);

// VerticalConcatenateFourArgs ------------------------------------------------

#[cfg(feature = "matrixd")]
struct VerticalConcatenateFourArgs<T> {
  e0: Box<dyn CopyMat<T>>,
  e1: Box<dyn CopyMat<T>>,
  e2: Box<dyn CopyMat<T>>,
  e3: Box<dyn CopyMat<T>>,
  out: Ref<DMatrix<T>>,
}
#[cfg(feature = "matrixd")]
impl<T> MechFunctionFactory for VerticalConcatenateFourArgs<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static +
  ConstElem + CompileConst + AsValueKind,
  Ref<DMatrix<T>>: ToValue
{
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Quaternary(out, arg0, arg1, arg2, arg3) => {
        let e0: Box<dyn CopyMat<T>> = unsafe { arg0.get_copyable_matrix_unchecked::<T>() };
        let e1: Box<dyn CopyMat<T>> = unsafe { arg1.get_copyable_matrix_unchecked::<T>() };
        let e2: Box<dyn CopyMat<T>> = unsafe { arg2.get_copyable_matrix_unchecked::<T>() };
        let e3: Box<dyn CopyMat<T>> = unsafe { arg3.get_copyable_matrix_unchecked::<T>() };
        let out: Ref<DMatrix<T>> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(Self { e0, e1, e2, e3, out }))
      },
      _ => Err(MechError2::new(IncorrectNumberOfArguments{expected: 4, found: args.len()}, None).with_compiler_loc())
    }
  }
}
#[cfg(feature = "matrixd")]
impl<T> MechFunctionImpl for VerticalConcatenateFourArgs<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DMatrix<T>>: ToValue
{
  fn solve(&self) {
    let mut offset = self.e0.copy_into_row_major(&self.out,0);
    offset += self.e1.copy_into_row_major(&self.out,offset);
    offset += self.e2.copy_into_row_major(&self.out,offset);
    self.e3.copy_into_row_major(&self.out,offset);

  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("VerticalConcatenateFourArgs\n{:#?}", self.out) }
}
#[cfg(feature = "matrixd")]
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for VerticalConcatenateFourArgs<T> 
where
  T: ConstElem + CompileConst + AsValueKind
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
let mut registers = [0, 0, 0, 0, 0];

    registers[0] = compile_register!(self.out, ctx);
    registers[1] = compile_register_mat!(self.e0, ctx);
    registers[2] = compile_register_mat!(self.e1, ctx);
    registers[3] = compile_register_mat!(self.e2, ctx);
    registers[4] = compile_register_mat!(self.e3, ctx);

    ctx.features.insert(FeatureFlag::Builtin(FeatureKind::VertCat));

    ctx.emit_quadop(
      hash_str(&format!("VerticalConcatenateFourArgs<{}>", T::as_value_kind())),
      registers[0],
      registers[1],
      registers[2],
      registers[3],
      registers[4],
    );
    Ok(registers[0])
  }
}
#[cfg(feature = "matrixd")]
register_vertical_concatenate_fxn!(VerticalConcatenateFourArgs);

// VerticalConcatenateNArgs ---------------------------------------------------

#[cfg(feature = "matrixd")]
struct VerticalConcatenateNArgs<T> {
  e0: Vec<Box<dyn CopyMat<T>>>,
  out: Ref<DMatrix<T>>,
}
#[cfg(feature = "matrixd")]
impl<T> MechFunctionFactory for VerticalConcatenateNArgs<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static +
  ConstElem + CompileConst + AsValueKind,
  Ref<DMatrix<T>>: ToValue
{
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Variadic(out, arg0) => {
        let mut e0: Vec<Box<dyn CopyMat<T>>> = Vec::new();
        for arg in arg0 {
          let mat: Box<dyn CopyMat<T>> = unsafe { arg.get_copyable_matrix_unchecked::<T>() };
          e0.push(mat);
        }
        let out: Ref<DMatrix<T>> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(Self { e0, out }))
      },
      _ => Err(MechError2::new(IncorrectNumberOfArguments{expected: 0, found: args.len()}, None).with_compiler_loc())
    }
  }
}
#[cfg(feature = "matrixd")]
impl<T> MechFunctionImpl for VerticalConcatenateNArgs<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DMatrix<T>>: ToValue
{
  fn solve(&self) {
    let mut offset = 0;
    for e in &self.e0 {
      offset += e.copy_into_row_major(&self.out,offset);
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("VerticalConcatenateNArgs\n{:#?}", self.out) }
}
#[cfg(feature = "matrixd")]
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for VerticalConcatenateNArgs<T> 
where
  T: ConstElem + CompileConst + AsValueKind
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let mut registers = [0, 0];

    registers[0] = compile_register!(self.out, ctx);

    let mut mat_regs = Vec::new();
    for e in &self.e0 {
      let e_addr = e.addr();
      let e_reg = ctx.alloc_register_for_ptr(e_addr);
      let e_const_id = e.compile_const_mat(ctx).unwrap();
      ctx.emit_const_load(e_reg, e_const_id);
      mat_regs.push(e_reg);
    }
    ctx.features.insert(FeatureFlag::Builtin(FeatureKind::HorzCat));
    ctx.emit_varop(
      hash_str(&format!("VerticalConcatenateNArgs<{}>", T::as_value_kind())),
      registers[0],
      mat_regs,
    );
    Ok(registers[0])
  }
}
#[cfg(feature = "matrixd")]
register_vertical_concatenate_fxn!(VerticalConcatenateNArgs);

// VerticalConcatenateVec -----------------------------------------------------

macro_rules! vertical_concatenate {
  ($name:ident, $vec_size:expr) => {
    paste!{
      #[derive(Debug)]
      struct $name<T> {
        out: Ref<[<$vec_size>]<T>>,
      }
      impl<T> MechFunctionFactory for $name<T>
      where
        T: Debug + Clone + Sync + Send + PartialEq + 'static +
        ConstElem + CompileConst + AsValueKind,
        Ref<[<$vec_size>]<T>>: ToValue
      {
        fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
          match args {
            FunctionArgs::Unary(out, _arg0) => {
              let out: Ref<[<$vec_size>]<T>> = unsafe { out.as_unchecked() }.clone();
              Ok(Box::new(Self { out }))
            },
            _ => Err(MechError2::new(IncorrectNumberOfArguments{expected: 1, found: args.len()}, None).with_compiler_loc())
          }
        }
      }
      impl<T> MechFunctionImpl for $name<T> 
      where
        T: Debug + Clone + Sync + Send + PartialEq + 'static,
        Ref<[<$vec_size>]<T>>: ToValue
      {
        fn solve(&self) {}
        fn out(&self) -> Value { self.out.to_value() }
        fn to_string(&self) -> String { format!("{:#?}", self) }
      }
      #[cfg(feature = "compiler")]
      impl<T> MechFunctionCompiler for $name<T> 
      where
        T: ConstElem + CompileConst + AsValueKind + AsValueKind
      {
        fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
          let name = format!("{}<{}>", stringify!($name), T::as_value_kind());
          compile_unop!(name, self.out, self.out, ctx, FeatureFlag::Builtin(FeatureKind::VertCat));
        }
      }
      register_vertical_concatenate_fxn!($name);
    }
  };}  

// VerticalConcatenateVD2 -----------------------------------------------------

#[cfg(feature = "vectord")]
struct VerticalConcatenateVD2<T> {
  e0: Box<dyn CopyMat<T>>,
  e1: Box<dyn CopyMat<T>>,
  out: Ref<DVector<T>>,
}
#[cfg(feature = "vectord")]
impl<T> MechFunctionFactory for VerticalConcatenateVD2<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static +
  ConstElem + CompileConst + AsValueKind,
  Ref<DVector<T>>: ToValue
{
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Binary(out, arg0, arg1) => {
        let e0: Box<dyn CopyMat<T>> = unsafe { arg0.get_copyable_matrix_unchecked::<T>() };
        let e1: Box<dyn CopyMat<T>> = unsafe { arg1.get_copyable_matrix_unchecked::<T>() };
        let out: Ref<DVector<T>> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(Self { e0, e1, out }))
      },
      _ => Err(MechError2::new(IncorrectNumberOfArguments{expected: 2, found: args.len()}, None).with_compiler_loc())
    }
  }
}
#[cfg(feature = "vectord")]
impl<T> MechFunctionImpl for VerticalConcatenateVD2<T> 
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DVector<T>>: ToValue
{
  fn solve(&self) {   
    let mut offset = self.e0.copy_into_v(&self.out,0);
    self.e1.copy_into_v(&self.out,offset);
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("VerticalConcatenateVD2\n{:#?}", self.out) }
}
#[cfg(feature = "vectord")]
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for VerticalConcatenateVD2<T> 
where
  T: ConstElem + CompileConst + AsValueKind
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let mut registers = [0, 0, 0];

    registers[0] = compile_register!(self.out, ctx);

    let lhs_addr = self.e0.addr();
    let lhs_reg = ctx.alloc_register_for_ptr(lhs_addr);
    let lhs_const_id = self.e0.compile_const_mat(ctx).unwrap();
    ctx.emit_const_load(lhs_reg, lhs_const_id);
    registers[1] = lhs_reg;

    let rhs_addr = self.e1.addr();
    let rhs_reg = ctx.alloc_register_for_ptr(rhs_addr);
    let rhs_const_id = self.e1.compile_const_mat(ctx).unwrap();
    ctx.emit_const_load(rhs_reg, rhs_const_id);
    registers[2] = rhs_reg;

    ctx.features.insert(FeatureFlag::Builtin(FeatureKind::HorzCat));

    ctx.emit_binop(
      hash_str(&format!("VerticalConcatenateVD2<{}>", T::as_value_kind())),
      registers[0],
      registers[1],
      registers[2],
    );

    Ok(registers[0])
  }
}
#[cfg(feature = "vectord")]
register_vertical_concatenate_fxn!(VerticalConcatenateVD2);

// VerticalConcatenateVD3 -----------------------------------------------------

#[cfg(feature = "vectord")]
struct VerticalConcatenateVD3<T> {
  e0: Box<dyn CopyMat<T>>,
  e1: Box<dyn CopyMat<T>>,
  e2: Box<dyn CopyMat<T>>,
  out: Ref<DVector<T>>,
}
#[cfg(feature = "vectord")]
impl<T> MechFunctionFactory for VerticalConcatenateVD3<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static +
  ConstElem + CompileConst + AsValueKind,
  Ref<DVector<T>>: ToValue
{
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Ternary(out, arg0, arg1, arg2) => {
        let e0: Box<dyn CopyMat<T>> = unsafe { arg0.get_copyable_matrix_unchecked::<T>() };
        let e1: Box<dyn CopyMat<T>> = unsafe { arg1.get_copyable_matrix_unchecked::<T>() };
        let e2: Box<dyn CopyMat<T>> = unsafe { arg2.get_copyable_matrix_unchecked::<T>() };
        let out: Ref<DVector<T>> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(Self { e0, e1, e2, out }))
      },
      _ => Err(MechError2::new(IncorrectNumberOfArguments{expected: 3, found: args.len()}, None).with_compiler_loc())
    }
  }
}
#[cfg(feature = "vectord")]
impl<T> MechFunctionImpl for VerticalConcatenateVD3<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DVector<T>>: ToValue
{
  fn solve(&self) {   
    let mut offset = self.e0.copy_into_v(&self.out,0);
    offset += self.e1.copy_into_v(&self.out,offset);
    self.e2.copy_into_v(&self.out,offset);
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("VerticalConcatenateVD3\n{:#?}", self.out) }
}
#[cfg(feature = "vectord")]
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for VerticalConcatenateVD3<T> 
where
  T: ConstElem + CompileConst + AsValueKind
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let mut registers = [0, 0, 0, 0];

    registers[0] = compile_register!(self.out, ctx);
    registers[1] = compile_register_mat!(self.e0, ctx);
    registers[2] = compile_register_mat!(self.e1, ctx);
    registers[3] = compile_register_mat!(self.e2, ctx);

    ctx.features.insert(FeatureFlag::Builtin(FeatureKind::HorzCat));

    ctx.emit_ternop(
      hash_str(&format!("VerticalConcatenateVD3<{}>", T::as_value_kind())),
      registers[0],
      registers[1],
      registers[2],
      registers[3],
    );
    Ok(registers[0])
  }
}
#[cfg(feature = "vectord")]
register_vertical_concatenate_fxn!(VerticalConcatenateVD3);

// VerticalConcatenateVD4 -----------------------------------------------------

#[cfg(feature = "vectord")]
struct VerticalConcatenateVD4<T> {
  e0: Box<dyn CopyMat<T>>,
  e1: Box<dyn CopyMat<T>>,
  e2: Box<dyn CopyMat<T>>,
  e3: Box<dyn CopyMat<T>>,
  out: Ref<DVector<T>>,
}
#[cfg(feature = "vectord")]
impl<T> MechFunctionFactory for VerticalConcatenateVD4<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static +
  ConstElem + CompileConst + AsValueKind,
  Ref<DVector<T>>: ToValue
{
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Quaternary(out, arg0, arg1, arg2, arg3) => {
        let e0: Box<dyn CopyMat<T>> = unsafe { arg0.get_copyable_matrix_unchecked::<T>() };
        let e1: Box<dyn CopyMat<T>> = unsafe { arg1.get_copyable_matrix_unchecked::<T>() };
        let e2: Box<dyn CopyMat<T>> = unsafe { arg2.get_copyable_matrix_unchecked::<T>() };
        let e3: Box<dyn CopyMat<T>> = unsafe { arg3.get_copyable_matrix_unchecked::<T>() };
        let out: Ref<DVector<T>> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(Self { e0, e1, e2, e3, out }))
      },
      _ => Err(MechError2::new(IncorrectNumberOfArguments{expected: 4, found: args.len()}, None).with_compiler_loc())
    }
  }
}
#[cfg(feature = "vectord")]
impl<T> MechFunctionImpl for VerticalConcatenateVD4<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DVector<T>>: ToValue
{
  fn solve(&self) {   
    let mut offset = self.e0.copy_into_v(&self.out,0);
    offset += self.e1.copy_into_v(&self.out,offset);
    offset += self.e2.copy_into_v(&self.out,offset);
    self.e3.copy_into_v(&self.out,offset);
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("VerticalConcatenateVD3\n{:#?}", self.out) }
}
#[cfg(feature = "vectord")]
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for VerticalConcatenateVD4<T> 
where
  T: ConstElem + CompileConst + AsValueKind
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let mut registers = [0, 0, 0, 0, 0];

    registers[0] = compile_register!(self.out, ctx);
    registers[1] = compile_register_mat!(self.e0, ctx);
    registers[2] = compile_register_mat!(self.e1, ctx);
    registers[3] = compile_register_mat!(self.e2, ctx);
    registers[4] = compile_register_mat!(self.e3, ctx);

    ctx.features.insert(FeatureFlag::Builtin(FeatureKind::HorzCat));

    ctx.emit_quadop(
      hash_str(&format!("VerticalConcatenateVD4<{}>", T::as_value_kind())),
      registers[0],
      registers[1],
      registers[2],
      registers[3],
      registers[4],
    );
    Ok(registers[0])
  }
}
#[cfg(feature = "vectord")]
register_vertical_concatenate_fxn!(VerticalConcatenateVD4);

// VerticalConcatenateVDN -----------------------------------------------------

#[cfg(feature = "vectord")]
struct VerticalConcatenateVDN<T> {
  scalar: Vec<(Ref<T>,usize)>,
  matrix: Vec<(Box<dyn CopyMat<T>>,usize)>,
  out: Ref<DVector<T>>,
}
#[cfg(feature = "vectord")]
impl<T> MechFunctionFactory for VerticalConcatenateVDN<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static +
  ConstElem + CompileConst + AsValueKind,
  Ref<DVector<T>>: ToValue
{
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Variadic(out, vargs) => {
        let mut scalar: Vec<(Ref<T>,usize)> = Vec::new();
        let mut matrix: Vec<(Box<dyn CopyMat<T>>,usize)> = Vec::new();
        for (i, arg) in vargs.into_iter().enumerate() {
          let kind = arg.kind();
          if arg.is_scalar() {
            let scalar_ref = unsafe { arg.as_unchecked::<T>() };
            scalar.push((scalar_ref.clone(), i));
          } else {
            let mat_ref: Box<dyn CopyMat<T>> = unsafe { arg.get_copyable_matrix_unchecked::<T>() };
            matrix.push((mat_ref, i));
          }
        }
        let out: Ref<DVector<T>> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(Self { scalar, matrix, out }))
      },
      _ => Err(MechError2::new(IncorrectNumberOfArguments{expected: 0, found: args.len()}, None).with_compiler_loc())
    }
  }
}
#[cfg(feature = "vectord")]
impl<T> MechFunctionImpl for VerticalConcatenateVDN<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DVector<T>>: ToValue
{
  fn solve(&self) {
    unsafe {
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      for (e,i) in &self.matrix {
        e.copy_into_v(&self.out,*i);
      }
      for (e,i) in &self.scalar {
        out_ptr[*i] = e.borrow().clone();
      }
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("VerticalConcatenateVDN\n{:#?}", self.out) }
}
#[cfg(feature = "vectord")]
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for VerticalConcatenateVDN<T> 
where
  T: ConstElem + CompileConst + AsValueKind
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let mut registers = [0, 0];

    registers[0] = compile_register!(self.out, ctx);

    let mut mat_regs = Vec::new();
    for (e,_) in &self.matrix {
      mat_regs.push(compile_register_mat!(e, ctx));
    }
    ctx.features.insert(FeatureFlag::Builtin(FeatureKind::HorzCat));
    ctx.emit_varop(
      hash_str(&format!("VerticalConcatenateVDN<{}>", T::as_value_kind())),
      registers[0],
      mat_regs,
    );
    Ok(registers[0])
  }
}
#[cfg(feature = "vectord")]
register_vertical_concatenate_fxn!(VerticalConcatenateVDN);

// VerticalConcatenateS1 ------------------------------------------------------

#[cfg(feature = "matrix1")]
#[derive(Debug)]
struct VerticalConcatenateS1<T> {
  out: Ref<Matrix1<T>>,
}
#[cfg(feature = "matrix1")]
impl<T> MechFunctionFactory for VerticalConcatenateS1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static +
  ConstElem + CompileConst + AsValueKind,
  Ref<Matrix1<T>>: ToValue
{
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Unary(out, _arg0) => {
        let out: Ref<Matrix1<T>> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(Self { out }))
      },
      _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("VerticalConcatenateS1 requires 1 argument, got {:?}", args), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments})
    }
  }
}
#[cfg(feature = "matrix1")]
impl<T> MechFunctionImpl for VerticalConcatenateS1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<Matrix1<T>>: ToValue
{
  fn solve(&self) {}
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(all(feature = "matrix1", feature = "compiler"))]
impl<T> MechFunctionCompiler for VerticalConcatenateS1<T> 
where
  T: ConstElem + CompileConst + AsValueKind
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("VerticalConcatenateS1<{}>", T::as_value_kind());
    compile_nullop!(name, self.out, ctx, FeatureFlag::Builtin(FeatureKind::VertCat));
  }
}
#[cfg(feature = "matrix1")]
register_vertical_concatenate_fxn!(VerticalConcatenateS1);

// VerticalConcatenateS2 ------------------------------------------------------

#[cfg(feature = "vector2")]
vertical_concatenate!(VerticalConcatenateS2,Vector2);

// VerticalConcatenateS3 ------------------------------------------------------

#[cfg(feature = "vector3")]
vertical_concatenate!(VerticalConcatenateS3,Vector3);

// VerticalConcatenateS4 ------------------------------------------------------

#[cfg(feature = "vector4")]
vertical_concatenate!(VerticalConcatenateS4,Vector4);

// VerticalConcatenateV2 ------------------------------------------------------

#[cfg(feature = "vector2")]
vertical_concatenate!(VerticalConcatenateV2,Vector2);

// VerticalConcatenateV3 ------------------------------------------------------

#[cfg(feature = "vector3")]
vertical_concatenate!(VerticalConcatenateV3,Vector3);

// VerticalConcatenateV4 ------------------------------------------------------

#[cfg(feature = "vector4")]
vertical_concatenate!(VerticalConcatenateV4,Vector4);

// VerticalConcatenateM2 ------------------------------------------------------

#[cfg(feature = "matrix2")]
vertical_concatenate!(VerticalConcatenateM2,Matrix2);

// VerticalConcatenateM3 ------------------------------------------------------

#[cfg(feature = "matrix3")]
vertical_concatenate!(VerticalConcatenateM3,Matrix3);

// VerticalConcatenateM2x3 ----------------------------------------------------

#[cfg(feature = "matrix2x3")]
vertical_concatenate!(VerticalConcatenateM2x3,Matrix2x3);

// VerticalConcatenateM3x2 ----------------------------------------------------

#[cfg(feature = "matrix3x2")]
vertical_concatenate!(VerticalConcatenateM3x2,Matrix3x2);

// VerticalConcatenateM4 ------------------------------------------------------

#[cfg(feature = "matrix4")]
vertical_concatenate!(VerticalConcatenateM4,Matrix4);

// VerticalConcatenateMD ------------------------------------------------------

#[cfg(feature = "matrixd")]
vertical_concatenate!(VerticalConcatenateMD,DMatrix);

// VerticalConcatenateVD ------------------------------------------------------

#[cfg(feature = "vectord")]
vertical_concatenate!(VerticalConcatenateVD,DVector);

// VerticalConcatenateSD ------------------------------------------------------

#[cfg(feature = "vectord")]
#[derive(Debug)]
struct VerticalConcatenateSD<T> {
  out: Ref<DVector<T>>,
}
#[cfg(feature = "vectord")]
impl<T> MechFunctionFactory for VerticalConcatenateSD<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static +
  ConstElem + CompileConst + AsValueKind,
  Ref<DVector<T>>: ToValue
{
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Unary(out, _arg0) => {
        let out: Ref<DVector<T>> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(Self { out }))
      },
      _ => Err(MechError2::new(
          IncorrectNumberOfArguments { expected: 1, found: args.len() }, 
          None
        ).with_compiler_loc()
      ),
    }
  }
}
#[cfg(feature = "vectord")]
impl<T> MechFunctionImpl for VerticalConcatenateSD<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DVector<T>>: ToValue
{
  fn solve(&self) { }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(all(feature = "vectord", feature = "compiler"))]
impl<T> MechFunctionCompiler for VerticalConcatenateSD<T> 
where
  T: ConstElem + CompileConst + AsValueKind
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("VerticalConcatenateSD<{}>", T::as_value_kind());
    compile_nullop!(name, self.out, ctx, FeatureFlag::Builtin(FeatureKind::VertCat));
  }
}
#[cfg(feature = "vectord")]
register_vertical_concatenate_fxn!(VerticalConcatenateSD);

// VerticalConcatenateM1M1 ----------------------------------------------------

macro_rules! vertcat_m1m1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
  };}
#[cfg(all(feature = "matrix1", feature = "vector2"))]
vertcat_two_args!(VerticalConcatenateM1M1,Matrix1,Matrix1,Vector2,vertcat_m1m1);

// VerticalConcatenateV2V2 ----------------------------------------------------

macro_rules! vertcat_r2r2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e1[1].clone();
  };}
#[cfg(all(feature = "vector2", feature = "vector4"))]
vertcat_two_args!(VerticalConcatenateV2V2,Vector2,Vector2,Vector4,vertcat_r2r2);

// VerticalConcatenateM1V3 ----------------------------------------------------

macro_rules! vertcat_m1r3 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e1[1].clone();
    $out[3] = $e1[2].clone();
  };}
#[cfg(all(feature = "matrix1", feature = "vector3", feature = "vector4"))]
vertcat_two_args!(VerticalConcatenateM1V3,Matrix1,Vector3,Vector4,vertcat_m1r3);

// VerticalConcatenateV3M1 ----------------------------------------------------

macro_rules! vertcat_r3m1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e0[2].clone();
    $out[3] = $e1[0].clone();
  };}
#[cfg(all(feature = "vector3", feature = "matrix1", feature = "vector4"))]
vertcat_two_args!(VerticalConcatenateV3M1,Vector3,Matrix1,Vector4,vertcat_r3m1);

// VerticalConcatenateM1V2 ----------------------------------------------------

macro_rules! vertcat_m1r2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e1[1].clone();
  };
}
#[cfg(all(feature = "matrix1", feature = "vector2", feature = "vector3"))]
vertcat_two_args!(VerticalConcatenateM1V2, Matrix1, Vector2, Vector3, vertcat_m1r2);

// VerticalConcatenateV2M1 ----------------------------------------------------

macro_rules! vertcat_r2m1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e1[0].clone();
  };
}
#[cfg(all(feature = "vector2", feature = "matrix1", feature = "vector3"))]
vertcat_two_args!(VerticalConcatenateV2M1, Vector2, Matrix1, Vector3, vertcat_r2m1);

// VerticalConcatenateM1M1M1 --------------------------------------------------

macro_rules! vertcat_m1m1m1 {
  ($out:expr, $e0:expr,$e1:expr,$e2:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e2[0].clone();
  };
}
#[cfg(all(feature = "matrix1", feature = "vector3"))]
vertcat_three_args!(VerticalConcatenateM1M1M1,Matrix1,Matrix1,Matrix1,Vector3, vertcat_m1m1m1);

// VerticalConcatenateM1M1V2 --------------------------------------------------

macro_rules! vertcat_m1m1v2 {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e2[0].clone();
    $out[3] = $e2[1].clone();
  };
}
#[cfg(all(feature = "matrix1", feature = "vector2", feature = "vector4"))]
vertcat_three_args!(VerticalConcatenateM1M1V2, Matrix1, Matrix1, Vector2, Vector4, vertcat_m1m1v2);

// VerticalConcatenateM1V2M1 --------------------------------------------------

macro_rules! vertcat_m1r2m1 {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e1[1].clone();
    $out[3] = $e2[0].clone();
  };
}
#[cfg(all(feature = "matrix1", feature = "vector2", feature = "vector4"))]
vertcat_three_args!(VerticalConcatenateM1V2M1, Matrix1, Vector2, Matrix1, Vector4, vertcat_m1r2m1);

// VerticalConcatenateV2M1M1 --------------------------------------------------

macro_rules! vertcat_r2m1m1 {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e2[0].clone();
  };
}
#[cfg(all(feature = "vector2", feature = "matrix1", feature = "vector4"))]
vertcat_three_args!(VerticalConcatenateV2M1M1, Vector2, Matrix1, Matrix1, Vector4, vertcat_r2m1m1);

// VerticalConcatenateM1M1M1M1 ------------------------------------------------

#[cfg(all(feature = "matrix1", feature = "vector4"))]
#[derive(Debug)]
struct VerticalConcatenateM1M1M1M1<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<Matrix1<T>>,
  e2: Ref<Matrix1<T>>,
  e3: Ref<Matrix1<T>>,
  out: Ref<Vector4<T>>,
}
#[cfg(all(feature = "matrix1", feature = "vector4"))]
impl<T> MechFunctionFactory for VerticalConcatenateM1M1M1M1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static +
  ConstElem + CompileConst + AsValueKind,
  Ref<Vector4<T>>: ToValue
{
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Quaternary(out, arg0, arg1, arg2, arg3) => {
        let e0: Ref<Matrix1<T>> = unsafe { arg0.as_unchecked() }.clone();
        let e1: Ref<Matrix1<T>> = unsafe { arg1.as_unchecked() }.clone();
        let e2: Ref<Matrix1<T>> = unsafe { arg2.as_unchecked() }.clone();
        let e3: Ref<Matrix1<T>> = unsafe { arg3.as_unchecked() }.clone();
        let out: Ref<Vector4<T>> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(Self { e0, e1, e2, e3, out }))
      },
      _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("VerticalConcatenateM1M1M1M1 requires 4 arguments, got {:?}", args), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments})
    }
  }
}
#[cfg(all(feature = "matrix1", feature = "vector4"))]
impl<T> MechFunctionImpl for VerticalConcatenateM1M1M1M1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<Vector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let e2_ptr = (*(self.e2.as_ptr())).clone();
      let e3_ptr = (*(self.e3.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e1_ptr[0].clone();
      out_ptr[2] = e2_ptr[0].clone();
      out_ptr[3] = e3_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(all(feature = "matrix1", feature = "vector4", feature = "compiler"))]
impl<T> MechFunctionCompiler for VerticalConcatenateM1M1M1M1<T> 
where
  T: ConstElem + CompileConst + AsValueKind
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("VerticalConcatenateM1M1M1M1<{}>", T::as_value_kind());
    compile_quadop!(name, self.out, self.e0, self.e1, self.e2, self.e3, ctx, FeatureFlag::Builtin(FeatureKind::VertCat));
  }
}
#[cfg(all(feature = "matrix1", feature = "vector4"))]
register_vertical_concatenate_fxn!(VerticalConcatenateM1M1M1M1);

// Mixed Type Vertical Concatenations -----------------------------------------

macro_rules! vertcat_r2r2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[2] = $e0[1].clone();
    $out[1] = $e1[0].clone();
    $out[3] = $e1[1].clone();
  };
}
#[cfg(all(feature = "row_vector2", feature = "matrix2"))]
vertcat_two_args!(VerticalConcatenateR2R2, RowVector2, RowVector2, Matrix2, vertcat_r2r2);

// VerticalConcatenateR3R3 ----------------------------------------------------

macro_rules! vertcat_r3r3 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[2] = $e0[1].clone();
    $out[4] = $e0[2].clone();
    $out[1] = $e1[0].clone();
    $out[3] = $e1[1].clone();
    $out[5] = $e1[2].clone();
  };
}
#[cfg(all(feature = "row_vector3", feature = "matrix2x3"))]
vertcat_two_args!(VerticalConcatenateR3R3, RowVector3, RowVector3, Matrix2x3, vertcat_r3r3);

// VerticalConcatenateR2M2 ----------------------------------------------------

macro_rules! vertcat_r2m2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[3] = $e0[1].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e1[1].clone();
    $out[4] = $e1[2].clone();
    $out[5] = $e1[3].clone();
  };
}
#[cfg(all(feature = "row_vector2", feature = "matrix2", feature = "matrix3x2"))]
vertcat_two_args!(VerticalConcatenateR2M2, RowVector2, Matrix2, Matrix3x2, vertcat_r2m2);

// VerticalConcatenateM2R2 ----------------------------------------------------

macro_rules! vertcat_m2r2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[3] = $e0[2].clone();
    $out[4] = $e0[3].clone();
    $out[2] = $e1[0].clone();
    $out[5] = $e1[1].clone();
  };
}
#[cfg(all(feature = "matrix2", feature = "row_vector2", feature = "matrix3x2"))]
vertcat_two_args!(VerticalConcatenateM2R2, Matrix2, RowVector2, Matrix3x2, vertcat_m2r2);

// VerticalConcatenateM2x3R3 --------------------------------------------------

macro_rules! vertcat_m2x3r3 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[3] = $e0[2].clone();
    $out[4] = $e0[3].clone();
    $out[6] = $e0[4].clone();
    $out[7] = $e0[5].clone();
    $out[2] = $e1[0].clone();
    $out[5] = $e1[1].clone();
    $out[8] = $e1[2].clone();
  };
}
#[cfg(all(feature = "matrix2x3", feature = "row_vector3", feature = "matrix3"))]
vertcat_two_args!(VerticalConcatenateM2x3R3, Matrix2x3, RowVector3, Matrix3, vertcat_m2x3r3);

// VerticalConcatenateR3M2x3 --------------------------------------------------

macro_rules! vertcat_r3m2x3 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[3] = $e0[1].clone();
    $out[6] = $e0[2].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e1[1].clone();
    $out[4] = $e1[2].clone();
    $out[5] = $e1[3].clone();
    $out[7] = $e1[4].clone();
    $out[8] = $e1[5].clone();
  };
}
#[cfg(all(feature = "row_vector3", feature = "matrix2x3", feature = "matrix3"))]
vertcat_two_args!(VerticalConcatenateR3M2x3, RowVector3, Matrix2x3, Matrix3, vertcat_r3m2x3);

// VerticalConcatenateMDR4 ----------------------------------------------------

macro_rules! vertcat_mdr4 {
  ($out:expr, $e0:expr, $e1:expr) => {
    let e0_len = $e0.len();
    for i in 0..e0_len {
      $out[i] = $e0[i].clone();
    }
    let offset = e0_len;
    $out[offset] = $e1[0].clone();
    $out[offset + 1] = $e1[1].clone();
    $out[offset + 2] = $e1[2].clone();
    $out[offset + 3] = $e1[3].clone();
  };
}
#[cfg(all(feature = "matrixd", feature = "row_vector4", feature = "matrix4"))]
vertcat_two_args!(VerticalConcatenateMDR4, DMatrix, RowVector4, Matrix4, vertcat_mdr4);

// VerticalConcatenateMDMD ----------------------------------------------------

macro_rules! vertcat_mdmd {
  ($out:expr, $e0:expr, $e1:expr) => {
    let dest_rows = $out.nrows();
    let mut offset = 0;
    let mut dest_ix = 0;

    let src_rows = $e0.nrows();
    let stride = dest_rows - src_rows;
    dest_ix = offset;
    for ix in 0..$e0.len() {
      $out[dest_ix] = $e0[ix].clone();
      dest_ix += ((ix + 1) % src_rows == 0) as usize * stride + 1;
    }
    offset += src_rows;

    let src_rows = $e1.nrows();
    let stride = dest_rows - src_rows;
    dest_ix = offset;
    for ix in 0..$e1.len() {
      $out[dest_ix] = $e1[ix].clone();
      dest_ix += ((ix + 1) % src_rows == 0) as usize * stride + 1;
    }
  };
}
#[cfg(all(feature = "matrixd", feature = "matrix4"))]
vertcat_two_args!(VerticalConcatenateMDMD, DMatrix, DMatrix, Matrix4, vertcat_mdmd);

// VerticalConcatenateR4MD ----------------------------------------------------

#[cfg(all(feature = "matrixd", feature = "matrix4", feature = "row_vector4"))]
vertcat_two_args!(VerticalConcatenateR4MD, RowVector4, DMatrix, Matrix4, vertcat_mdmd);

// VerticalConcatenateR2R2R2 ----------------------------------------------------

macro_rules! vertcat_mdmdmd {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    let dest_rows = $out.nrows();
    let mut offset = 0;
    let mut dest_ix = 0;

    let src_rows = $e0.nrows();
    let stride = dest_rows - src_rows;
    dest_ix = offset;
    for ix in 0..$e0.len() {
      $out[dest_ix] = $e0[ix].clone();
      dest_ix += ((ix + 1) % src_rows == 0) as usize * stride + 1;
    }
    offset += src_rows;

    let src_rows = $e1.nrows();
    let stride = dest_rows - src_rows;
    dest_ix = offset;
    for ix in 0..$e1.len() {
      $out[dest_ix] = $e1[ix].clone();
      dest_ix += ((ix + 1) % src_rows == 0) as usize * stride + 1;
    }
    offset += src_rows;

    let src_rows = $e2.nrows();
    let stride = dest_rows - src_rows;
    dest_ix = offset;
    for ix in 0..$e2.len() {
      $out[dest_ix] = $e2[ix].clone();
      dest_ix += ((ix + 1) % src_rows == 0) as usize * stride + 1;
    }
  };
}

#[cfg(all(feature = "row_vector2", feature = "matrix3x2"))]
vertcat_three_args!(VerticalConcatenateR2R2R2, RowVector2, RowVector2, RowVector2, Matrix3x2, vertcat_mdmdmd);

// VerticalConcatenateR3R3R3 --------------------------------------------------

#[cfg(all(feature = "row_vector3", feature = "matrix3"))]
vertcat_three_args!(VerticalConcatenateR3R3R3, RowVector3, RowVector3, RowVector3, Matrix3, vertcat_mdmdmd);

// VerticalConcatenateR4R4MD --------------------------------------------------

#[cfg(all(feature = "row_vector4", feature = "matrixd", feature = "matrix4"))]
vertcat_three_args!(VerticalConcatenateR4R4MD, RowVector4, RowVector4, DMatrix, Matrix4, vertcat_mdmdmd);

// VerticalConcatenateR4MDR4 --------------------------------------------------

#[cfg(all(feature = "row_vector4", feature = "matrixd", feature = "row_vector4", feature = "matrix4"))]
vertcat_three_args!(VerticalConcatenateR4MDR4, RowVector4, DMatrix, RowVector4, Matrix4, vertcat_mdmdmd);

// VerticalConcatenateMDR4R4 --------------------------------------------------

#[cfg(all(feature = "matrixd", feature = "row_vector4", feature = "row_vector4", feature = "matrix4"))]
vertcat_three_args!(VerticalConcatenateMDR4R4, DMatrix, RowVector4, RowVector4, Matrix4, vertcat_mdmdmd);

// VerticalConcatenateR4R4R4R4 ------------------------------------------------

macro_rules! vertcat_mdmdmdmd {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr, $e3:expr) => {
    let dest_rows = $out.nrows();
    let mut offset = 0;
    let mut dest_ix = 0;

    let src_rows = $e0.nrows();
    let stride = dest_rows - src_rows;
    dest_ix = offset;
    for ix in 0..$e0.len() {
      $out[dest_ix] = $e0[ix].clone();
      dest_ix += ((ix + 1) % src_rows == 0) as usize * stride + 1;
    }
    offset += src_rows;

    let src_rows = $e1.nrows();
    let stride = dest_rows - src_rows;
    dest_ix = offset;
    for ix in 0..$e1.len() {
      $out[dest_ix] = $e1[ix].clone();
      dest_ix += ((ix + 1) % src_rows == 0) as usize * stride + 1;
    }
    offset += src_rows;

    let src_rows = $e2.nrows();
    let stride = dest_rows - src_rows;
    dest_ix = offset;
    for ix in 0..$e2.len() {
      $out[dest_ix] = $e2[ix].clone();
      dest_ix += ((ix + 1) % src_rows == 0) as usize * stride + 1;
    }
    offset += src_rows;

    let src_rows = $e3.nrows();
    let stride = dest_rows - src_rows;
    dest_ix = offset;
    for ix in 0..$e3.len() {
      $out[dest_ix] = $e3[ix].clone();
      dest_ix += ((ix + 1) % src_rows == 0) as usize * stride + 1;
    }
  };
}

#[cfg(all(feature = "matrix4", feature = "row_vector4"))]
vertcat_four_args!(VerticalConcatenateR4R4R4R4, RowVector4, RowVector4, RowVector4, RowVector4, Matrix4, vertcat_mdmdmdmd);

macro_rules! impl_vertcat_arms {
  ($kind:ident, $args:expr, $default:expr) => {
    paste!{
    {
      let arguments = $args;  
      let rows = arguments[0].shape()[0];
      let rows:usize = arguments.iter().fold(0, |acc, x| acc + x.shape()[0]);
      let columns:usize = arguments[0].shape()[1];
      let nargs = arguments.len();
      let kinds: Vec<ValueKind> = arguments.iter().map(|x| x.kind()).collect::<Vec<ValueKind>>();
      let no_refs = !kinds.iter().any(|x| {
        match x {
          ValueKind::Reference(_) => true,
          ValueKind::Matrix(_,_) => true,
          _ => false,
      }});
      if no_refs {
        let mat: Vec<$kind> = arguments.iter().flat_map(|v| v.[<as_vec $kind:lower>]().unwrap()).collect::<Vec<$kind>>();
        fn to_column_major<T: Clone>(out: &[Value], row_n: usize, col_n: usize, extract_fn: impl Fn(&Value) -> MResult<Vec<T>> + Clone) -> Vec<T> {
          (0..col_n).flat_map(|col| out.iter().map({let value = extract_fn.clone();move |row| value(row).unwrap()[col].clone()})).collect()
        }
        let mat = to_column_major(&arguments, rows, columns, |v| v.[<as_vec $kind:lower>]());
        match (rows,columns) {
          #[cfg(feature = "matrix1")]
          (1,1) => {return Ok(Box::new(VerticalConcatenateS1{out:Ref::new(Matrix1::from_vec(mat))}));}
          #[cfg(feature = "vector2")]
          (2,1) => {return Ok(Box::new(VerticalConcatenateS2{out:Ref::new(Vector2::from_vec(mat))}));}
          #[cfg(feature = "vector3")]
          (3,1) => {return Ok(Box::new(VerticalConcatenateS3{out:Ref::new(Vector3::from_vec(mat))}));}
          #[cfg(feature = "vector4")]
          (4,1) => {return Ok(Box::new(VerticalConcatenateS4{out:Ref::new(Vector4::from_vec(mat))}));}
          #[cfg(feature = "vectord")]
          (m,1) => {return Ok(Box::new(VerticalConcatenateSD{out:Ref::new(DVector::from_vec(mat))}));}
          #[cfg(feature = "matrix2")]
          (2,2) => {return Ok(Box::new(VerticalConcatenateM2{out:Ref::new(Matrix2::from_vec(mat))}));}
          #[cfg(feature = "matrix3")]
          (3,3) => {return Ok(Box::new(VerticalConcatenateM3{out:Ref::new(Matrix3::from_vec(mat))}));}
          #[cfg(feature = "matrix4")]
          (4,4) => {return Ok(Box::new(VerticalConcatenateM4{out:Ref::new(Matrix4::from_vec(mat))}));}
          #[cfg(feature = "matrix2x3")]
          (2,3) => {return Ok(Box::new(VerticalConcatenateM2x3{out:Ref::new(Matrix2x3::from_vec(mat))}));}
          #[cfg(feature = "matrix3x2")]
          (3,2) => {return Ok(Box::new(VerticalConcatenateM3x2{out:Ref::new(Matrix3x2::from_vec(mat))}));}
          #[cfg(feature = "matrixd")]
          (m,n) => {return Ok(Box::new(VerticalConcatenateMD{out:Ref::new(DMatrix::from_vec(m,n,mat))}));}
          _ => Err(MechError2::new(
            FeatureNotEnabledError,
            None
          ).with_compiler_loc()),
        }
      } else {
        match (nargs,rows,columns) {
          #[cfg(feature = "vector2")]
          (1,2,1) => {
            match &arguments[..] {
              // r2
              [Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e0))] => {
                return Ok(Box::new(VerticalConcatenateV2{out: e0.clone()}));
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "vector3")]
          (1,3,1) => {
            match &arguments[..] {
              // r3
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Vector3(ref e0)) => {
                    return Ok(Box::new(VerticalConcatenateV3{out: e0.clone()}));
                  }
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "vector4")]
          (1,4,1) => {
            match &arguments[..] {
              // r4
              [Value::[<Matrix $kind:camel>](Matrix::Vector4(ref e0))] => {
                return Ok(Box::new(VerticalConcatenateV4{out: e0.clone()}));
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "vectord")]
          (1,m,1) => {
            match &arguments[..] {
              // rd
              [Value::[<Matrix $kind:camel>](Matrix::DVector(ref e0))] => {
                return Ok(Box::new(VerticalConcatenateVD{out: e0.clone()}));
              }
              _ => todo!(),
            }
          }
          #[cfg(all(feature = "matrix1", feature = "vector2"))]
          (2,2,1) => {
            let mut out = Vector2::from_element($default);
            match &arguments[..] {
              // m1m1
              [Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1))] => {
                return Ok(Box::new(VerticalConcatenateM1M1{e0: e0.clone(), e1: e1.clone(), out: Ref::new(out)}));
              }
              _ => todo!(),
            }
          }
          #[cfg(all(feature = "matrix1", feature = "vector3", feature = "vector2"))]
          (2,3,1) => {
            let mut out = Vector3::from_element($default);
            match &arguments[..] {
              //m1v2
              [Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e1))] => {
                return Ok(Box::new(VerticalConcatenateM1V2{e0: e0.clone(), e1: e1.clone(), out: Ref::new(out)}));
              }
              //v2m1
              [Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1))] => {
                return Ok(Box::new(VerticalConcatenateV2M1{e0: e0.clone(), e1: e1.clone(), out: Ref::new(out)}));
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "vector4")]
          (2,4,1) => {
            let mut out = Vector4::from_element($default);
            match &arguments[..] {
              // m1v3
              #[cfg(all(feature = "matrix1", feature = "vector3"))]
              [Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Vector3(ref e1))] => {
                return Ok(Box::new(VerticalConcatenateM1V3{e0: e0.clone(), e1: e1.clone(), out: Ref::new(out)}));
              }
              // v3m1
              #[cfg(all(feature = "matrix1", feature = "vector3"))]
              [Value::[<Matrix $kind:camel>](Matrix::Vector3(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1))] => {
                return Ok(Box::new(VerticalConcatenateV3M1{e0: e0.clone(), e1: e1.clone(), out: Ref::new(out)}));
              }
              // v2v2
              #[cfg(feature = "vector2")]
              [Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e1))] => {
                return Ok(Box::new(VerticalConcatenateV2V2{e0: e0.clone(), e1: e1.clone(), out: Ref::new(out)}));
              }              
              _ => todo!(),
            }
          } 
          #[cfg(feature = "vectord")]
          (2,m,1) => {
            let mut out = DVector::from_element(m,$default);
            match &arguments[..] {
              [Value::[<Matrix $kind:camel>](e0),Value::[<Matrix $kind:camel>](e1)] => {
                let e0 = e0.get_copyable_matrix();
                let e1 = e1.get_copyable_matrix();
                return Ok(Box::new(VerticalConcatenateVD2{e0, e1, out: Ref::new(out)}));
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "vector3")]
          (3,3,1) => {  
            let mut out = Vector3::from_element($default);
            match &arguments[..] {
              // m1 m1 m1
              #[cfg(feature = "matrix1")]
              [Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2))] => {
                return Ok(Box::new(VerticalConcatenateM1M1M1{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out)}));
              }    
              _ => todo!()
            }
          }
          #[cfg(all(feature = "matrix1", feature = "vector2", feature = "vector4"))]
          (3,4,1) => {
            let mut out = Vector4::from_element($default);
            match &arguments[..] {
              // m1 m1 v2
              [Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)),Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e2))] => {
                return Ok(Box::new(VerticalConcatenateM1M1V2{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out)}));
              }
              // m1 v2 m1
              [Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e1)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2))] => {
                return Ok(Box::new(VerticalConcatenateM1V2M1{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out)}));
              }
              // v2 m1 m1
              [Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2))] => {
                return Ok(Box::new(VerticalConcatenateV2M1M1{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out)}));
              }
              _ => todo!()
            }
          }
          #[cfg(feature = "vectord")]
          (3,m,1) => {
            let mut out = DVector::from_element(m,$default);
            match &arguments[..] {
              [Value::[<Matrix $kind:camel>](e0),Value::[<Matrix $kind:camel>](e1),Value::[<Matrix $kind:camel>](e2)] => {
                let e0 = e0.get_copyable_matrix();
                let e1 = e1.get_copyable_matrix();
                let e2 = e2.get_copyable_matrix();
                return Ok(Box::new(VerticalConcatenateVD3{e0, e1, e2, out: Ref::new(out)}));
              }
              _ => todo!(),
            }
          }
          #[cfg(all(feature = "matrix1", feature = "vector4"))]
          (4,4,1) => {
            let mut out = Vector4::from_element($default);
            match &arguments[..] {
              // m1 m1 m1 m1
              [Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e3))] => {
                return Ok(Box::new(VerticalConcatenateM1M1M1M1{ e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), e3: e3.clone(), out: Ref::new(out) }));
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "vectord")]
          (4,m,1) => {
            let mut out = DVector::from_element(m,$default);
            match &arguments[..] {
              [Value::[<Matrix $kind:camel>](e0),Value::[<Matrix $kind:camel>](e1),Value::[<Matrix $kind:camel>](e2),Value::[<Matrix $kind:camel>](e3)] => {
                let e0 = e0.get_copyable_matrix();
                let e1 = e1.get_copyable_matrix();
                let e2 = e2.get_copyable_matrix();
                let e3 = e3.get_copyable_matrix();
                return Ok(Box::new(VerticalConcatenateVD4{e0, e1, e2, e3, out: Ref::new(out)}));
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "vectord")]
          (l,m,1) => {
            let mut out = DVector::from_element(m,$default);
            let mut matrix_args: Vec<(Box<dyn CopyMat<$kind>>,usize)> = vec![];
            let mut scalar_args: Vec<(Ref<$kind>,usize)> = vec![];
            let mut i = 0;
            for arg in arguments.iter() {
              match &arg {
                Value::[<$kind:camel>](e0) => {
                  scalar_args.push((e0.clone(),i));
                  i += 1;
                }
                Value::[<Matrix $kind:camel>](e0) => {
                  matrix_args.push((e0.get_copyable_matrix(),i));
                  i += e0.shape()[0];
                }
                _ => todo!(),
              }
            }
            return Ok(Box::new(VerticalConcatenateVDN{scalar: scalar_args, matrix: matrix_args, out: Ref::new(out)}));
          }
          #[cfg(feature = "matrix2")]
          (2,2,2) => {
            let mut out = Matrix2::from_element($default);
            match &arguments[..] {
              // v2v2
              #[cfg(feature = "row_vector2")]
              [Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e0)),Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e1))] => {return Ok(Box::new(VerticalConcatenateR2R2{e0: e0.clone(), e1: e1.clone(), out: Ref::new(out)}));}
              _ => todo!(),
            }
          }
          #[cfg(feature = "matrix2x3")]
          (2,2,3) => {
            let mut out = Matrix2x3::from_element($default);
            match &arguments[..] {
              // r3r3
              #[cfg(feature = "row_vector3")]
              [Value::[<Matrix $kind:camel>](Matrix::RowVector3(ref e0)),Value::[<Matrix $kind:camel>](Matrix::RowVector3(ref e1))] => {return Ok(Box::new(VerticalConcatenateR3R3{e0: e0.clone(), e1: e1.clone(), out: Ref::new(out)}));}
              _ => todo!(),
            }
          }
          #[cfg(feature = "matrix3x2")]
          (2,3,2) => {
            let mut out = Matrix3x2::from_element($default);
            match &arguments[..] {
              // v2m2
              #[cfg(all(feature = "row_vector2", feature = "matrix2"))]
              [Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Matrix2(ref e1))] => {
                return Ok(Box::new(VerticalConcatenateR2M2{e0: e0.clone(), e1: e1.clone(), out: Ref::new(out)}));
              }
              // m2v2
              #[cfg(all(feature = "matrix2", feature = "row_vector2"))]
              [Value::[<Matrix $kind:camel>](Matrix::Matrix2(ref e0)), Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e1))] => {
                return Ok(Box::new(VerticalConcatenateM2R2{e0: e0.clone(), e1: e1.clone(), out: Ref::new(out)}));
              }
              _ => todo!(),
            }
            
          }
          #[cfg(feature = "matrix3")]
          (2,3,3) => {
            let mut out = Matrix3::from_element($default);
            match &arguments[..] {
              // v3m3x2
              #[cfg(all(feature = "row_vector3", feature = "matrix2x3"))]
              [Value::[<Matrix $kind:camel>](Matrix::RowVector3(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Matrix2x3(ref e1))] => {
                return Ok(Box::new(VerticalConcatenateR3M2x3 { e0: e0.clone(), e1: e1.clone(), out: Ref::new(out) }));
              }
              // m3x2v3
              #[cfg(all(feature = "matrix2x3", feature = "row_vector3"))]
              [Value::[<Matrix $kind:camel>](Matrix::Matrix2x3(ref e0)), Value::[<Matrix $kind:camel>](Matrix::RowVector3(ref e1))] => {
                return Ok(Box::new(VerticalConcatenateM2x3R3 { e0: e0.clone(), e1: e1.clone(), out: Ref::new(out) }));
              }
              _ => todo!(),
            }
            
          }
          #[cfg(feature = "matrix4")]
          (2,4,4) => {
            let mut out = Matrix4::from_element($default);
            match &arguments[..] {
              // r4md
              #[cfg(all(feature = "row_vector4", feature = "matrixd"))]
              [Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e0)), Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e1))] => Ok(Box::new(VerticalConcatenateR4MD{e0:e0.clone(),e1:e1.clone(),out:Ref::new(out)})),
              // mdr4
              #[cfg(all(feature = "matrixd", feature = "row_vector4"))]
              [Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e0)), Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e1))] => Ok(Box::new(VerticalConcatenateMDR4{e0:e0.clone(),e1:e1.clone(),out:Ref::new(out)})),
              // mdmd
              #[cfg(feature = "matrixd")]
              [Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e0)), Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e1))] => Ok(Box::new(VerticalConcatenateMDMD{e0:e0.clone(),e1:e1.clone(),out:Ref::new(out)})),
              _ => todo!(),
            }
            
          }
          #[cfg(feature = "matrixd")]
          (2,m,n) => {
            let mut out = DMatrix::from_element(m,n,$default);
            match &arguments[..] {
              [Value::[<Matrix $kind:camel>](m0), Value::[<Matrix $kind:camel>](m1)] => {
                let e0 = m0.get_copyable_matrix();
                let e1 = m1.get_copyable_matrix();
                Ok(Box::new(VerticalConcatenateTwoArgs{e0, e1, out: Ref::new(out)}))
              }
              _ => todo!(),
            }            
          }
          #[cfg(feature = "matrix3x2")]
          (3,3,2) => {
            let mut out = Matrix3x2::from_element($default);
            match &arguments[..] {
              // r2r2r2
              #[cfg(feature = "row_vector2")]
              [Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e0)),Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e1)),Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e2))]=>Ok(Box::new(VerticalConcatenateR2R2R2{e0:e0.clone(),e1:e1.clone(),e2:e2.clone(),out:Ref::new(out)})),
              _ => todo!(),
            }
          }
          #[cfg(feature = "matrix3")]
          (3,3,3) => {
            let mut out = Matrix3::from_element($default);
            match &arguments[..] {
              // r3r3r3
              #[cfg(feature = "row_vector3")]
              [Value::[<Matrix $kind:camel>](Matrix::RowVector3(ref e0)),Value::[<Matrix $kind:camel>](Matrix::RowVector3(ref e1)),Value::[<Matrix $kind:camel>](Matrix::RowVector3(ref e2))]=>Ok(Box::new(VerticalConcatenateR3R3R3{e0:e0.clone(),e1:e1.clone(),e2:e2.clone(),out:Ref::new(out)})),
              _ => todo!(),
            }
          }
          #[cfg(feature = "matrix4")]
          (3,4,4) => {
            let mut out = Matrix4::from_element($default);
            match &arguments[..] {
              // r4r4md
              #[cfg(all(feature = "row_vector4", feature = "matrixd"))]
              [Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e0)),Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e1)),Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e2))]=>Ok(Box::new(VerticalConcatenateR4R4MD{e0:e0.clone(),e1:e1.clone(),e2:e2.clone(),out:Ref::new(out)})),
              // r4mdr4
              #[cfg(all(feature = "row_vector4", feature = "matrixd"))]
              [Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e0)),Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e1)),Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e2))]=>Ok(Box::new(VerticalConcatenateR4MDR4{e0:e0.clone(),e1:e1.clone(),e2:e2.clone(),out:Ref::new(out)})),
              // mdr4r4
              #[cfg(all(feature = "row_vector4", feature = "matrixd"))]
              [Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e0)),Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e1)),Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e2))]=>Ok(Box::new(VerticalConcatenateMDR4R4{e0:e0.clone(),e1:e1.clone(),e2:e2.clone(),out:Ref::new(out)})),
              _ => todo!(),
            }
          }
          #[cfg(feature = "matrixd")]
          (3,m,n) => {
            let mut out = DMatrix::from_element(m,n,$default);
            match &arguments[..] {
              [Value::[<Matrix $kind:camel>](m0),Value::[<Matrix $kind:camel>](m1),Value::[<Matrix $kind:camel>](m2)] => {
                let e0 = m0.get_copyable_matrix();
                let e1 = m1.get_copyable_matrix();
                let e2 = m2.get_copyable_matrix();
                Ok(Box::new(VerticalConcatenateThreeArgs{e0,e1,e2,out:Ref::new(out)}))
              }   
              _ => todo!(),
            }
          }
          #[cfg(feature = "matrix4")]
          (4,4,4) => {
            let mut out = Matrix4::from_element($default);
            match &arguments[..] {
              // r4r4r4r4
              #[cfg(feature = "row_vector4")]
              [Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e0)),Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e1)),Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e2)),Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e3))]=>Ok(Box::new(VerticalConcatenateR4R4R4R4{e0:e0.clone(),e1:e1.clone(),e2:e2.clone(),e3:e3.clone(),out:Ref::new(out)})),
              _ => todo!(),
            }
          }
          #[cfg(feature = "matrixd")]
          (4,m,n) => {
            let mut out = DMatrix::from_element(m,n,$default);
            match &arguments[..] {
              [Value::[<Matrix $kind:camel>](m0),Value::[<Matrix $kind:camel>](m1),Value::[<Matrix $kind:camel>](m2),Value::[<Matrix $kind:camel>](m3)] => {
                let e0 = m0.get_copyable_matrix();
                let e1 = m1.get_copyable_matrix();
                let e2 = m2.get_copyable_matrix();
                let e3 = m3.get_copyable_matrix();
                Ok(Box::new(VerticalConcatenateFourArgs{e0,e1,e2,e3,out:Ref::new(out)}))
              }   
              _ => todo!(),
            }
          }
          #[cfg(feature = "matrixd")]
          (l,m,n) => {
            let mut out = DMatrix::from_element(m,n,$default);
            let mut args = vec![];
            for arg in arguments {
              match arg {
                Value::[<Matrix $kind:camel>](m0) => {
                  let e0 = m0.get_copyable_matrix();
                  args.push(e0);
                }
                _ => todo!(),
              }
            }
            Ok(Box::new(VerticalConcatenateNArgs{e0: args, out:Ref::new(out)}))
          }
          _ => {return Err(MechError2::new(
                UnhandledFunctionArgumentKindVarg { arg: arguments.iter().map(|x| x.kind()).collect(), fxn_name: "matrix/vertcat".to_string() },
                None
              ).with_compiler_loc()
            );
          }
        }
  }}}}}

fn impl_vertcat_fxn(arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {

  let kinds: Vec<ValueKind> = arguments.iter().map(|x| x.kind()).collect::<Vec<ValueKind>>();
  let target_kind = kinds[0].clone();

  #[cfg(feature = "f64")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::F64) { return impl_vertcat_arms!(f64, arguments, f64::default()) } }

  #[cfg(feature = "f32")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::F32) { return impl_vertcat_arms!(f32, arguments, f32::default()) } }

  #[cfg(feature = "u8")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::U8)  { return impl_vertcat_arms!(u8,  arguments, u8::default()) } }

  #[cfg(feature = "u16")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::U16) { return impl_vertcat_arms!(u16, arguments, u16::default()) } }

  #[cfg(feature = "u32")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::U32) { return impl_vertcat_arms!(u32, arguments, u32::default()) } }

  #[cfg(feature = "u64")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::U64) { return impl_vertcat_arms!(u64, arguments, u64::default()) } }

  #[cfg(feature = "u128")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::U128){ return impl_vertcat_arms!(u128, arguments, u128::default()) } }

  #[cfg(feature = "bool")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::Bool) { return impl_vertcat_arms!(bool, arguments, bool::default()) } }

  #[cfg(feature = "string")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::String) { return impl_vertcat_arms!(String, arguments, String::default()) } }

  #[cfg(feature = "rational")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::R64) { return impl_vertcat_arms!(R64, arguments, R64::default()) } }

  #[cfg(feature = "complex")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::C64) { return impl_vertcat_arms!(C64, arguments, C64::default()) } }

  Err(MechError2::new(
      UnhandledFunctionArgumentKindVarg { arg: arguments.iter().map(|x| x.kind()).collect(), fxn_name: "matrix/vertcat".to_string() },
      None
    ).with_compiler_loc()
  )
}


pub struct MatrixVertCat {}
impl NativeFunctionCompiler for MatrixVertCat {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    impl_vertcat_fxn(arguments)
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "matrix/vertcat",
    ptr: &MatrixVertCat{},
  }
}

#[derive(Debug, Clone)]
pub struct VerticalConcatenateDimensionMismatch {
  pub rows: usize,
  pub cols: usize,
}
impl MechErrorKind2 for VerticalConcatenateDimensionMismatch {
  fn name(&self) -> &str { "VerticalConcatenateDimensionMismatch" }
  fn message(&self) -> String {
    format!("Cannot vertically concatenate matrices/vectors with dimensions ({}, {})", self.rows, self.cols)
  }
}