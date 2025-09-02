#[macro_use]
use crate::stdlib::*;

// Horizontal Concatenate -----------------------------------------------------

macro_rules! horzcat_one_arg {
  ($fxn:ident, $e0:ident, $out:ident, $opt:ident) => {
    #[derive(Debug)]
    struct $fxn<T> {
      e0: Ref<$e0<T>>,
      out: Ref<$out<T>>,
    }
    impl<T> MechFunctionImpl for $fxn<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$out<T>>: ToValue
    {
      fn solve(&self) { 
        unsafe {
          let e0_ptr = (*(self.e0.as_ptr())).clone();
          let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
          $opt!(out_ptr,e0_ptr);
        }
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }
    #[cfg(feature = "compiler")]
    impl<T> MechFunctionCompiler for $fxn<T> 
    where
      T: ConstElem + CompileConst
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        compile_unop!(self.out, self.e0, ctx, FeatureFlag::Builtin(FeatureKind::HorzCat));
      }
    }
  };}

macro_rules! horzcat_two_args {
  ($fxn:ident, $e1:ident, $e2:ident, $out:ident, $opt:ident) => {
    #[derive(Debug)]
    struct $fxn<T> {
      e0: Ref<$e1<T>>,
      e1: Ref<$e2<T>>,
      out: Ref<$out<T>>,
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
          $opt!(out_ptr,e0_ptr,e1_ptr);
        }
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }
    #[cfg(feature = "compiler")]
    impl<T> MechFunctionCompiler for $fxn<T> 
    where
      T: ConstElem + CompileConst
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        compile_binop!(self.out, self.e0, self.e1, ctx, FeatureFlag::Builtin(FeatureKind::HorzCat));
      }
    }
  };}

macro_rules! horzcat_three_args {
  ($fxn:ident, $e0:ident, $e1:ident, $e2:ident, $out:ident, $opt:ident) => {
    #[derive(Debug)]
    struct $fxn<T> {
      e0: Ref<$e0<T>>,
      e1: Ref<$e1<T>>,
      e2: Ref<$e2<T>>,
      out: Ref<$out<T>>,
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
      T: ConstElem + CompileConst
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        compile_ternop!(self.out, self.e0, self.e1, self.e2, ctx, FeatureFlag::Builtin(FeatureKind::HorzCat));
      }
    }
  };} 
  
macro_rules! horzcat_four_args {
  ($fxn:ident, $e0:ident, $e1:ident, $e2:ident, $e3:ident, $out:ident, $opt:ident) => {
    #[derive(Debug)]
    struct $fxn<T> {
      e0: Ref<$e0<T>>,
      e1: Ref<$e1<T>>,
      e2: Ref<$e2<T>>,
      e3: Ref<$e3<T>>,
      out: Ref<$out<T>>,
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
      T: ConstElem + CompileConst
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        compile_quadop!(self.out, self.e0, self.e1, self.e2, self.e3, ctx, FeatureFlag::Builtin(FeatureKind::HorzCat));
      }
    }
  };}   

struct HorizontalConcatenateTwoArgs<T> {
  e0: Box<dyn CopyMat<T>>,
  e1: Box<dyn CopyMat<T>>,
  out: Ref<DMatrix<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateTwoArgs<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DMatrix<T>>: ToValue
{
  fn solve(&self) {
    let offset = self.e0.copy_into(&self.out, 0);
    self.e1.copy_into(&self.out, offset);
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("HorizontalConcatenateTwoArgs\n{:#?}", self.out) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateTwoArgs<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static + ConstElem + CompileConst,
  Ref<DMatrix<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}
    
struct HorizontalConcatenateThreeArgs<T> {
  e0: Box<dyn CopyMat<T>>,
  e1: Box<dyn CopyMat<T>>,
  e2: Box<dyn CopyMat<T>>,
  out: Ref<DMatrix<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateThreeArgs<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DMatrix<T>>: ToValue
{
  fn solve(&self) {
    let mut offset = self.e0.copy_into(&self.out,0);
    offset += self.e1.copy_into(&self.out,offset);
    self.e2.copy_into(&self.out,offset);
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("HorizontalConcatenateThreeArgs\n{:#?}", self.out) }
}
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateThreeArgs<T> {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

struct HorizontalConcatenateFourArgs<T> {
  e0: Box<dyn CopyMat<T>>,
  e1: Box<dyn CopyMat<T>>,
  e2: Box<dyn CopyMat<T>>,
  e3: Box<dyn CopyMat<T>>,
  out: Ref<DMatrix<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateFourArgs<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DMatrix<T>>: ToValue,
{
  fn solve(&self) {
    let mut offset = self.e0.copy_into(&self.out, 0);
    offset += self.e1.copy_into(&self.out, offset);
    offset += self.e2.copy_into(&self.out, offset);
    self.e3.copy_into(&self.out, offset);
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("HorizontalConcatenateFourArgs\n{:#?}", self.out) }
}
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateFourArgs<T>
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

struct HorizontalConcatenateNArgs<T> {
  e0: Vec<Box<dyn CopyMat<T>>>,
  out: Ref<DMatrix<T>>,
}

impl<T> MechFunctionImpl for HorizontalConcatenateNArgs<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DMatrix<T>>: ToValue,
{
  fn solve(&self) {
    let mut offset = 0;
    for e in &self.e0 {
      offset += e.copy_into(&self.out, offset);
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("HorizontalConcatenateNArgs\n{:#?}", self.out) }
}
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateNArgs<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DMatrix<T>>: ToValue,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

macro_rules! horizontal_concatenate {
  ($name:ident, $vec_size:expr) => {
    paste!{
      #[derive(Debug)]
      struct $name<T> {
        out: Ref<[<RowVector $vec_size>]<T>>,
      }

      impl<T> MechFunctionImpl for $name<T>
      where
        T: Debug + Clone + Sync + Send + PartialEq + 'static,
        Ref<[<RowVector $vec_size>]<T>>: ToValue
      {
        fn solve(&self) {}
        fn out(&self) -> Value { self.out.to_value() }
        fn to_string(&self) -> String { format!("{:#?}", self) }
      }

      #[cfg(feature = "compiler")]
      impl<T> MechFunctionCompiler for $name<T>
      where
        T: Debug + Clone + Sync + Send + PartialEq + 'static,
        Ref<[<RowVector $vec_size>]<T>>: ToValue
      {
        fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
          todo!();
        }
      }
    }
  };
}

#[derive(Debug)]
struct HorizontalConcatenateRD<T> {
  out: Ref<RowDVector<T>>,
}

impl<T> MechFunctionImpl for HorizontalConcatenateRD<T> 
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowDVector<T>>: ToValue
{
  fn solve(&self) {}
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateRD<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowDVector<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

struct HorizontalConcatenateRDN<T> {
  scalar: Vec<(Ref<T>,usize)>,
  matrix: Vec<(Box<dyn CopyMat<T>>,usize)>,
  out: Ref<RowDVector<T>>,
}

impl<T> MechFunctionImpl for HorizontalConcatenateRDN<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowDVector<T>>: ToValue
{
  fn solve(&self) {
    unsafe {
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      for (e, i) in &self.matrix {
        let _ = e.copy_into_r(&self.out, *i);
      }
      for (e, i) in &self.scalar {
        out_ptr[*i] = e.borrow().clone();
      }
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("HorizontalConcatenateRDN\n{:#?}", self.out) }
}
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateRDN<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowDVector<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateS1<T> {
  arg: Ref<T>,
  out: Ref<Matrix1<T>>,
}

impl<T> MechFunctionImpl for HorizontalConcatenateS1<T> 
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<Matrix1<T>>: ToValue
{
  fn solve(&self) {
    unsafe {
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = self.arg.borrow().clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateS1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<Matrix1<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateS2<T> {
  e0: Ref<T>,
  e1: Ref<T>,
  out: Ref<RowVector2<T>>,
}

impl<T> MechFunctionImpl for HorizontalConcatenateS2<T> 
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector2<T>>: ToValue
{
  fn solve(&self) {
    unsafe {
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = self.e0.borrow().clone();
      out_ptr[1] = self.e1.borrow().clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateS2<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector2<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateS3<T> {
  e0: Ref<T>,
  e1: Ref<T>,
  e2: Ref<T>,
  out: Ref<RowVector3<T>>,
}

impl<T> MechFunctionImpl for HorizontalConcatenateS3<T> 
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn solve(&self) {
    unsafe {
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = self.e0.borrow().clone();
      out_ptr[1] = self.e1.borrow().clone();
      out_ptr[2] = self.e2.borrow().clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateS3<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateS4<T> {
  e0: Ref<T>,
  e1: Ref<T>,
  e2: Ref<T>,
  e3: Ref<T>,
  out: Ref<RowVector4<T>>,
}

impl<T> MechFunctionImpl for HorizontalConcatenateS4<T> 
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) {
    unsafe {
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = self.e0.borrow().clone();
      out_ptr[1] = self.e1.borrow().clone();
      out_ptr[2] = self.e2.borrow().clone();
      out_ptr[3] = self.e3.borrow().clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateS4<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[cfg(feature = "row_vector2")]
horizontal_concatenate!(HorizontalConcatenateR2,2);
#[cfg(feature = "row_vector3")]
horizontal_concatenate!(HorizontalConcatenateR3,3);
#[cfg(feature = "row_vector4")]
horizontal_concatenate!(HorizontalConcatenateR4,4);

#[derive(Debug)]
struct HorizontalConcatenateSD<T> {
  out: Ref<RowDVector<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateSD<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowDVector<T>>: ToValue
{
  fn solve(&self) { }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSD<T>
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

macro_rules! horzcat_single {
  ($name:ident,$shape:ident) => {
    #[derive(Debug)]
    struct $name<T> {
      out: Ref<$shape<T>>,
    }
    impl<T> MechFunctionImpl for $name<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$shape<T>>: ToValue
    {
      fn solve(&self) { }
      fn out(&self) -> Value { self.out.to_value() }
       fn to_string(&self) -> String { format!("{:#?}", self) }
    }
    #[cfg(feature = "compiler")]
    impl<T> MechFunctionCompiler for $name<T> {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        todo!();
      }
    }
  }
}

#[cfg(feature = "matrix1")]
horzcat_single!(HorizontalConcatenateM1,Matrix1);
#[cfg(feature = "matrix2")]
horzcat_single!(HorizontalConcatenateM2,Matrix2);
#[cfg(feature = "matrix3")]
horzcat_single!(HorizontalConcatenateM3,Matrix3);
#[cfg(feature = "matrix4")]
horzcat_single!(HorizontalConcatenateM4,Matrix4);
#[cfg(feature = "matrix2x3")]
horzcat_single!(HorizontalConcatenateM2x3,Matrix2x3);
#[cfg(feature = "matrix3x2")]
horzcat_single!(HorizontalConcatenateM3x2,Matrix3x2);
#[cfg(feature = "matrixd")]
horzcat_single!(HorizontalConcatenateMD,DMatrix);
#[cfg(feature = "vector2")]
horzcat_single!(HorizontalConcatenateV2,Vector2);
#[cfg(feature = "vector3")]
horzcat_single!(HorizontalConcatenateV3,Vector3);
#[cfg(feature = "vector4")]
horzcat_single!(HorizontalConcatenateV4,Vector4);
#[cfg(feature = "vectord")]
horzcat_single!(HorizontalConcatenateVD,DVector);

#[derive(Debug)]
struct HorizontalConcatenateSR2<T> {
  e0: Ref<T>,
  e1: Ref<RowVector2<T>>,
  out: Ref<RowVector3<T>>,
}

impl<T> MechFunctionImpl for HorizontalConcatenateSR2<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_ptr.clone();
      out_ptr[1] = e1_ptr[0].clone();
      out_ptr[2] = e1_ptr[1].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSR2<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateR2S<T> {
  e0: Ref<RowVector2<T>>,
  e1: Ref<T>,
  out: Ref<RowVector3<T>>,
}

impl<T> MechFunctionImpl for HorizontalConcatenateR2S<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e0_ptr[1].clone();
      out_ptr[2] = self.e1.borrow().clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateR2S<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateSM1<T> {
  e0: Ref<T>,         
  e1: Ref<Matrix1<T>>,
  out: Ref<RowVector2<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateSM1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector2<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_val = self.e0.borrow().clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_val;
      out_ptr[1] = e1_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSM1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector2<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateM1S<T> {
  e0: Ref<Matrix1<T>>,  // Matrix1
  e1: Ref<T>,           // scalar
  out: Ref<RowVector2<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateM1S<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector2<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_val = self.e1.borrow().clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e1_val;
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1S<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector2<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateSSSM1<T> {
  e0: Ref<T>,
  e1: Ref<T>,
  e2: Ref<T>,
  e3: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateSSSM1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_val = self.e0.borrow().clone();
      let e1_val = self.e1.borrow().clone();
      let e2_val = self.e2.borrow().clone();
      let e3_ptr = (*(self.e3.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_val;
      out_ptr[1] = e1_val;
      out_ptr[2] = e2_val;
      out_ptr[3] = e3_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSSSM1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateSSM1S<T> {
  e0: Ref<T>,
  e1: Ref<T>,
  e2: Ref<Matrix1<T>>,
  e3: Ref<T>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateSSM1S<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_val = self.e0.borrow().clone();
      let e1_val = self.e1.borrow().clone();
      let e2_ptr = (*(self.e2.as_ptr())).clone();
      let e3_val = self.e3.borrow().clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_val;
      out_ptr[1] = e1_val;
      out_ptr[2] = e2_ptr[0].clone();
      out_ptr[3] = e3_val;
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSSM1S<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateSM1SS<T> {
  e0: Ref<T>,
  e1: Ref<Matrix1<T>>,
  e2: Ref<T>,
  e3: Ref<T>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateSM1SS<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_val = self.e0.borrow().clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let e2_val = self.e2.borrow().clone();
      let e3_val = self.e3.borrow().clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_val;
      out_ptr[1] = e1_ptr[0].clone();
      out_ptr[2] = e2_val;
      out_ptr[3] = e3_val;
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSM1SS<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateM1SSS<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<T>,
  e2: Ref<T>,
  e3: Ref<T>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateM1SSS<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_val = self.e1.borrow().clone();
      let e2_val = self.e2.borrow().clone();
      let e3_val = self.e3.borrow().clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e1_val;
      out_ptr[2] = e2_val;
      out_ptr[3] = e3_val;
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1SSS<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateSR3<T> {
  e0: Ref<T>,
  e1: Ref<RowVector3<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateSR3<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_ptr.clone();
      out_ptr[1] = e1_ptr[0].clone();
      out_ptr[2] = e1_ptr[1].clone();
      out_ptr[3] = e1_ptr[2].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSR3<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateR3S<T> {
  e0: Ref<RowVector3<T>>,
  e1: Ref<T>,
  out: Ref<RowVector4<T>>,
}

impl<T> MechFunctionImpl for HorizontalConcatenateR3S<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue,
{
  fn solve(&self) {
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = self.e1.borrow().clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e0_ptr[1].clone();
      out_ptr[2] = e0_ptr[2].clone();
      out_ptr[3] = e1_ptr.clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateR3S<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateSSM1<T> {
  e0: Ref<T>,           // scalar
  e1: Ref<T>,           // scalar
  e2: Ref<Matrix1<T>>,  // Matrix1
  out: Ref<RowVector3<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateSSM1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_val = self.e0.borrow().clone();
      let e1_val = self.e1.borrow().clone();
      let e2_ptr = (*(self.e2.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_val;
      out_ptr[1] = e1_val;
      out_ptr[2] = e2_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSSM1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateSM1S<T> {
  e0: Ref<T>,           // scalar
  e1: Ref<Matrix1<T>>,  // Matrix1
  e2: Ref<T>,           // scalar
  out: Ref<RowVector3<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateSM1S<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_val = self.e0.borrow().clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let e2_val = self.e2.borrow().clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_val;
      out_ptr[1] = e1_ptr[0].clone();
      out_ptr[2] = e2_val;
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSM1S<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateM1SS<T> {
  e0: Ref<Matrix1<T>>,  // Matrix1
  e1: Ref<T>,           // scalar
  e2: Ref<T>,           // scalar
  out: Ref<RowVector3<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateM1SS<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_val = self.e1.borrow().clone();
      let e2_val = self.e2.borrow().clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e1_val;
      out_ptr[2] = e2_val;
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1SS<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateSSR2<T> {
  e0: Ref<T>,
  e1: Ref<T>,
  e2: Ref<RowVector2<T>>,
  out: Ref<RowVector4<T>>,
}

impl<T> MechFunctionImpl for HorizontalConcatenateSSR2<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue,
{
  fn solve(&self) {
    unsafe {
      let e0_val = self.e0.borrow().clone();
      let e1_val = self.e1.borrow().clone();
      let e2_ptr = (*(self.e2.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_val;
      out_ptr[1] = e1_val;
      out_ptr[2] = e2_ptr[0].clone();
      out_ptr[3] = e2_ptr[1].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSSR2<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateSR2S<T> {
  e0: Ref<T>,
  e1: Ref<RowVector2<T>>,
  e2: Ref<T>,
  out: Ref<RowVector4<T>>,
}

impl<T> MechFunctionImpl for HorizontalConcatenateSR2S<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue,
{
  fn solve(&self) {
    unsafe {
      let e0_val = self.e0.borrow().clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let e2_val = self.e2.borrow().clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_val;
      out_ptr[1] = e1_ptr[0].clone();
      out_ptr[2] = e1_ptr[1].clone();
      out_ptr[3] = e2_val;
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSR2S<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateR2SS<T> {
  e0: Ref<RowVector2<T>>,
  e1: Ref<T>,
  e2: Ref<T>,
  out: Ref<RowVector4<T>>,
}

impl<T> MechFunctionImpl for HorizontalConcatenateR2SS<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue,
{
  fn solve(&self) {
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_val = self.e1.borrow().clone();
      let e2_val = self.e2.borrow().clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e0_ptr[1].clone();
      out_ptr[2] = e1_val;
      out_ptr[3] = e2_val;
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateR2SS<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateM1M1S<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<Matrix1<T>>,
  e2: Ref<T>,
  out: Ref<RowVector3<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateM1M1S<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let e2_val = self.e2.borrow().clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e1_ptr[0].clone();
      out_ptr[2] = e2_val;
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1M1S<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

macro_rules! horzcat_m1m1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
  };}
horzcat_two_args!(HorizontalConcatenateM1M1,Matrix1,Matrix1,RowVector2,horzcat_m1m1);

#[derive(Debug)]
struct HorizontalConcatenateM1SM1<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<T>,
  e2: Ref<Matrix1<T>>,
  out: Ref<RowVector3<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateM1SM1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn solve(&self) {
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_val = self.e1.borrow().clone();
      let e2_ptr = (*(self.e2.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e1_val;
      out_ptr[2] = e2_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1SM1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateSM1M1<T> {
  e0: Ref<T>,
  e1: Ref<Matrix1<T>>,
  e2: Ref<Matrix1<T>>,
  out: Ref<RowVector3<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateSM1M1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn solve(&self) {
    unsafe {
      let e0_val = self.e0.borrow().clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let e2_ptr = (*(self.e2.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_val;
      out_ptr[1] = e1_ptr[0].clone();
      out_ptr[2] = e2_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSM1M1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

macro_rules! horzcat_r2r2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e1[1].clone();
  };}
horzcat_two_args!(HorizontalConcatenateR2R2,RowVector2,RowVector2,RowVector4,horzcat_r2r2);

macro_rules! horzcat_m1r3 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e1[1].clone();
    $out[3] = $e1[2].clone();
  };}
horzcat_two_args!(HorizontalConcatenateM1R3,Matrix1,RowVector3,RowVector4,horzcat_m1r3);

macro_rules! horzcat_r3m1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e0[2].clone();
    $out[3] = $e1[0].clone();
  };}
horzcat_two_args!(HorizontalConcatenateR3M1,RowVector3,Matrix1,RowVector4,horzcat_r3m1);

#[derive(Debug)]
struct HorizontalConcatenateSM1R2<T> {
  e0: Ref<T>,
  e1: Ref<Matrix1<T>>,
  e2: Ref<RowVector2<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateSM1R2<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_val = self.e0.borrow().clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let e2_ptr = (*(self.e2.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_val;
      out_ptr[1] = e1_ptr[0].clone();
      out_ptr[2] = e2_ptr[0].clone();
      out_ptr[3] = e2_ptr[1].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSM1R2<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateM1SR2<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<T>,
  e2: Ref<RowVector2<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateM1SR2<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_val = self.e1.borrow().clone();
      let e2_ptr = (*(self.e2.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e1_val;
      out_ptr[2] = e2_ptr[0].clone();
      out_ptr[3] = e2_ptr[1].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1SR2<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}
  
#[derive(Debug)]
struct HorizontalConcatenateSM1SM1<T> {
  e0: Ref<T>,          
  e1: Ref<Matrix1<T>>, 
  e2: Ref<T>,          
  e3: Ref<Matrix1<T>>, 
  out: Ref<RowVector4<T>>,
}

impl<T> MechFunctionImpl for HorizontalConcatenateSM1SM1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) {
    unsafe {
      let e0_val = self.e0.borrow().clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let e2_val = self.e2.borrow().clone();
      let e3_ptr = (*(self.e3.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_val;
      out_ptr[1] = e1_ptr[0].clone();
      out_ptr[2] = e2_val;
      out_ptr[3] = e3_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSM1SM1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateM1R2S<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<RowVector2<T>>,
  e2: Ref<T>,
  out: Ref<RowVector4<T>>,
}

impl<T> MechFunctionImpl for HorizontalConcatenateM1R2S<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue,
{
  fn solve(&self) {
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let e2_val = self.e2.borrow().clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e1_ptr[0].clone();
      out_ptr[2] = e1_ptr[1].clone();
      out_ptr[3] = e2_val;
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1R2S<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateR2M1S<T> {
  e0: Ref<RowVector2<T>>,
  e1: Ref<Matrix1<T>>,
  e2: Ref<T>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateR2M1S<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let e2_val = self.e2.borrow().clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e0_ptr[1].clone();
      out_ptr[2] = e1_ptr[0].clone();
      out_ptr[3] = e2_val;
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateR2M1S<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateR2SM1<T> {
  e0: Ref<RowVector2<T>>, 
  e1: Ref<T>,             
  e2: Ref<Matrix1<T>>,    
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateR2SM1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue,
{
  fn solve(&self) {
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_val = self.e1.borrow().clone();
      let e2_ptr = (*(self.e2.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e0_ptr[1].clone();
      out_ptr[2] = e1_val;
      out_ptr[3] = e2_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateR2SM1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateSR2M1<T> {
  e0: Ref<T>,             
  e1: Ref<RowVector2<T>>, 
  e2: Ref<Matrix1<T>>,    
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateSR2M1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_val = self.e0.borrow().clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let e2_ptr = (*(self.e2.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_val;
      out_ptr[1] = e1_ptr[0].clone();
      out_ptr[2] = e1_ptr[1].clone();
      out_ptr[3] = e2_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSR2M1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateSSM1M1<T> {
  e0: Ref<T>,
  e1: Ref<T>,
  e2: Ref<Matrix1<T>>,
  e3: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateSSM1M1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) {
    unsafe {
      let e0_val = self.e0.borrow().clone();
      let e1_val = self.e1.borrow().clone();
      let e2_ptr = (*(self.e2.as_ptr())).clone();
      let e3_ptr = (*(self.e3.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_val;
      out_ptr[1] = e1_val;
      out_ptr[2] = e2_ptr[0].clone();
      out_ptr[3] = e3_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSSM1M1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateM1M1SS<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<Matrix1<T>>,
  e2: Ref<T>,
  e3: Ref<T>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateM1M1SS<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) {
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let e2_val = self.e2.borrow().clone();
      let e3_val = self.e3.borrow().clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e1_ptr[0].clone();
      out_ptr[2] = e2_val;
      out_ptr[3] = e3_val;
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1M1SS<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateSM1M1S<T> {
  e0: Ref<T>,
  e1: Ref<Matrix1<T>>,
  e2: Ref<Matrix1<T>>,
  e3: Ref<T>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateSM1M1S<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) {
    unsafe {
      let e0_val = self.e0.borrow().clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let e2_ptr = (*(self.e2.as_ptr())).clone();
      let e3_val = self.e3.borrow().clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_val;
      out_ptr[1] = e1_ptr[0].clone();
      out_ptr[2] = e2_ptr[0].clone();
      out_ptr[3] = e3_val;
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSM1M1S<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateM1SSM1<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<T>,
  e2: Ref<T>,
  e3: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateM1SSM1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) {
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_val = self.e1.borrow().clone();
      let e2_val = self.e2.borrow().clone();
      let e3_ptr = (*(self.e3.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e1_val;
      out_ptr[2] = e2_val;
      out_ptr[3] = e3_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1SSM1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateM1SM1S<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<T>,
  e2: Ref<Matrix1<T>>,
  e3: Ref<T>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateM1SM1S<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) {
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_val = self.e1.borrow().clone();
      let e2_ptr = (*(self.e2.as_ptr())).clone();
      let e3_val = self.e3.borrow().clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e1_val;
      out_ptr[2] = e2_ptr[0].clone();
      out_ptr[3] = e3_val;
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1SM1S<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

macro_rules! horzcat_m1r2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e1[1].clone();
  };
}
horzcat_two_args!(HorizontalConcatenateM1R2, Matrix1, RowVector2, RowVector3, horzcat_m1r2);

macro_rules! horzcat_r2m1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e1[0].clone();
  };
}
horzcat_two_args!(HorizontalConcatenateR2M1, RowVector2, Matrix1, RowVector3, horzcat_r2m1);

macro_rules! horzcat_m1m1m1 {
  ($out:expr, $e0:expr,$e1:expr,$e2:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e2[0].clone();
  };
}
horzcat_three_args!(HorizontalConcatenateM1M1M1,Matrix1,Matrix1,Matrix1,RowVector3, horzcat_m1m1m1);

macro_rules! horzcat_m1m1r2 {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e2[0].clone();
    $out[3] = $e2[1].clone();
  };
}
horzcat_three_args!(HorizontalConcatenateM1M1R2, Matrix1, Matrix1, RowVector2, RowVector4, horzcat_m1m1r2);

macro_rules! horzcat_m1r2m1 {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e1[1].clone();
    $out[3] = $e2[0].clone();
  };
}
horzcat_three_args!(HorizontalConcatenateM1R2M1, Matrix1, RowVector2, Matrix1, RowVector4, horzcat_m1r2m1);

macro_rules! horzcat_r2m1m1 {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e2[0].clone();
  };
}
horzcat_three_args!(HorizontalConcatenateR2M1M1, RowVector2, Matrix1, Matrix1, RowVector4, horzcat_r2m1m1);

#[derive(Debug)]
struct HorizontalConcatenateSM1M1M1<T> {
  e0: Ref<T>,
  e1: Ref<Matrix1<T>>,
  e2: Ref<Matrix1<T>>,
  e3: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}

impl<T> MechFunctionImpl for HorizontalConcatenateSM1M1M1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue,
{
  fn solve(&self) { 
    unsafe {
      let e0_val = self.e0.borrow().clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let e2_ptr = (*(self.e2.as_ptr())).clone();
      let e3_ptr = (*(self.e3.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_val;
      out_ptr[1] = e1_ptr[0].clone();
      out_ptr[2] = e2_ptr[0].clone();
      out_ptr[3] = e3_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSM1M1M1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateM1SM1M1<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<T>,
  e2: Ref<Matrix1<T>>,
  e3: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateM1SM1M1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_val = self.e1.borrow().clone();
      let e2_ptr = (*(self.e2.as_ptr())).clone();
      let e3_ptr = (*(self.e3.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e1_val;
      out_ptr[2] = e2_ptr[0].clone();
      out_ptr[3] = e3_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1SM1M1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateM1M1SM1<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<Matrix1<T>>,
  e2: Ref<T>,
  e3: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateM1M1SM1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let e2_val = self.e2.borrow().clone();
      let e3_ptr = (*(self.e3.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e1_ptr[0].clone();
      out_ptr[2] = e2_val;
      out_ptr[3] = e3_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1M1SM1<T> 
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateM1M1M1S<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<Matrix1<T>>,
  e2: Ref<Matrix1<T>>,
  e3: Ref<T>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunctionImpl for HorizontalConcatenateM1M1M1S<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let e2_ptr = (*(self.e2.as_ptr())).clone();
      let e3_val = self.e3.borrow().clone();
      let mut out_ptr = (&mut *(self.out.as_mut_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e1_ptr[0].clone();
      out_ptr[2] = e2_ptr[0].clone();
      out_ptr[3] = e3_val;
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1M1M1S<T> 
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct HorizontalConcatenateM1M1M1M1<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<Matrix1<T>>,
  e2: Ref<Matrix1<T>>,
  e3: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}

impl<T> MechFunctionImpl for HorizontalConcatenateM1M1M1M1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue,
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

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1M1M1M1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

macro_rules! horzcat_v2v2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e1[1].clone();
  };
}
horzcat_two_args!(HorizontalConcatenateV2V2, Vector2, Vector2, Matrix2, horzcat_v2v2);

macro_rules! horzcat_v3v3 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e0[2].clone();
    $out[3] = $e1[0].clone();
    $out[4] = $e1[1].clone();
    $out[5] = $e1[2].clone();
  };
}
horzcat_two_args!(HorizontalConcatenateV3V3, Vector3, Vector3, Matrix3x2, horzcat_v3v3);

macro_rules! horzcat_v2m2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e1[1].clone();
    $out[4] = $e1[2].clone();
    $out[5] = $e1[3].clone();
  };
}
horzcat_two_args!(HorizontalConcatenateV2M2, Vector2, Matrix2, Matrix2x3, horzcat_v2m2);

macro_rules! horzcat_m2v2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e0[2].clone();
    $out[3] = $e0[3].clone();
    $out[4] = $e1[0].clone();
    $out[5] = $e1[1].clone();
  };
}
horzcat_two_args!(HorizontalConcatenateM2V2, Matrix2, Vector2, Matrix2x3, horzcat_m2v2);

macro_rules! horzcat_m3x2v3 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e0[2].clone();
    $out[3] = $e0[3].clone();
    $out[4] = $e0[4].clone();
    $out[5] = $e0[5].clone();
    $out[6] = $e1[0].clone();
    $out[7] = $e1[1].clone();
    $out[8] = $e1[2].clone();
  };
}
horzcat_two_args!(HorizontalConcatenateM3x2V3, Matrix3x2, Vector3, Matrix3, horzcat_m3x2v3);

macro_rules! horzcat_v3m3x2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e0[2].clone();
    $out[3] = $e1[0].clone();
    $out[4] = $e1[1].clone();
    $out[5] = $e1[2].clone();
    $out[6] = $e1[3].clone();
    $out[7] = $e1[4].clone();
    $out[8] = $e1[5].clone();
  };
}
horzcat_two_args!(HorizontalConcatenateV3M3x2, Vector3, Matrix3x2, Matrix3, horzcat_v3m3x2);

macro_rules! horzcat_v4md {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e0[2].clone();
    $out[3] = $e0[3].clone();
    let offset = 4;
    for i in 0..$e1.len() {
      $out[i + offset] = $e1[i].clone();
    }
  };
}
horzcat_two_args!(HorizontalConcatenateV4MD, Vector4, DMatrix, Matrix4, horzcat_v4md);

macro_rules! horzcat_mdv4 {
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
horzcat_two_args!(HorizontalConcatenateMDV4, DMatrix, Vector4, Matrix4, horzcat_mdv4);

macro_rules! horzcat_mdmd {
  ($out:expr, $e0:expr, $e1:expr) => {
    let e0_len = $e0.len();
    for i in 0..e0_len {
      $out[i] = $e0[i].clone();
    }
    let offset = e0_len;
    for i in 0..$e1.len() {
      $out[i + offset] = $e1[i].clone();
    }
  };
}
horzcat_two_args!(HorizontalConcatenateMDMD, DMatrix, DMatrix, Matrix4, horzcat_mdmd);

macro_rules! horzcat_mdmdmd {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    let e0_len = $e0.len();
    for i in 0..e0_len {
      $out[i] = $e0[i].clone();
    }
    let offset = e0_len;
    for i in 0..$e1.len() {
      $out[i + offset] = $e1[i].clone();
    }
    let offset = offset + $e1.len();
    for i in 0..$e2.len() {
      $out[i + offset] = $e2[i].clone();
    }
  };
}

#[cfg(feature = "vector2")]
horzcat_three_args!(HorizontalConcatenateV2V2V2, Vector2, Vector2, Vector2, Matrix2x3, horzcat_mdmdmd);
#[cfg(feature = "vector3")]
horzcat_three_args!(HorizontalConcatenateV3V3V3, Vector3, Vector3, Vector3, Matrix3, horzcat_mdmdmd);
#[cfg(all(feature = "matrixd", feature = "vector4"))]
horzcat_three_args!(HorizontalConcatenateV4V4MD, Vector4, Vector4, DMatrix, Matrix4, horzcat_mdmdmd);
#[cfg(all(feature = "matrixd", feature = "vector4"))]
horzcat_three_args!(HorizontalConcatenateV4MDV4, Vector4, DMatrix, Vector4, Matrix4, horzcat_mdmdmd);
#[cfg(all(feature = "matrixd", feature = "vector4"))]
horzcat_three_args!(HorizontalConcatenateMDV4V4, DMatrix, Vector4, Vector4, Matrix4, horzcat_mdmdmd);


macro_rules! horzcat_mdmdmdmd {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr, $e3:expr) => {
    let e0_len = $e0.len();
    for i in 0..e0_len {
      $out[i] = $e0[i].clone();
    }
    let offset = e0_len;
    for i in 0..$e1.len() {
      $out[i + offset] = $e1[i].clone();
    }
    let offset = offset + $e1.len();
    for i in 0..$e2.len() {
      $out[i + offset] = $e2[i].clone();
    }
    let offset = offset + $e2.len();
    for i in 0..$e3.len() {
      $out[i + offset] = $e3[i].clone();
    }
  };
}

#[cfg(all(feature = "matrix4", feature = "vector4"))]
horzcat_four_args!(HorizontalConcatenateV4V4V4V4, Vector4, Vector4, Vector4, Vector4, Matrix4, horzcat_mdmdmdmd);

macro_rules! impl_horzcat_arms {
  ($kind:ident, $args:expr, $default:expr) => {
    paste!{
    {

      #[cfg(feature = "matrix")]
      fn extract_matrix(arg: &Value) -> MResult<Box<dyn CopyMat<$kind>>> {
        match arg {
          Value::[<Matrix $kind:camel>](m) => Ok(m.get_copyable_matrix()),
          Value::MutableReference(inner) => match &*inner.borrow() {
            Value::[<Matrix $kind:camel>](m) => Ok(m.get_copyable_matrix()),
            _ => Err(MechError { file: file!().to_string(), tokens: vec![], msg: format!("Expected a Matrix<{}> or MutableReference to Matrix<{}>, found {:?}", stringify!($kind), stringify!($kind), arg), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
          },
          _ => Err(MechError { file: file!().to_string(), tokens: vec![], msg: format!("Expected a Matrix<{}> or MutableReference to Matrix<{}>, found {:?}", stringify!($kind), stringify!($kind), arg), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
        }
      }
      #[cfg(feature = "row_vector2")] // get_r2
      fn get_r2(value: &Value) -> Option<Ref<RowVector2<$kind>>> { match value { Value::[<Matrix $kind:camel>](Matrix::RowVector2(v)) => Some(v.clone()), Value::MutableReference(inner) => match &*inner.borrow() { Value::[<Matrix $kind:camel>](Matrix::RowVector2(v)) => Some(v.clone()), _ => None, }, _ => None, } }
      #[cfg(not(feature = "row_vector2"))]
      fn get_r2(_value: &Value) -> Option<()> { None }

      #[cfg(feature = "row_vector3")] // get_r3
      fn get_r3(value: &Value) -> Option<Ref<RowVector3<$kind>>> { match value { Value::[<Matrix $kind:camel>](Matrix::RowVector3(v)) => Some(v.clone()), Value::MutableReference(inner) => match &*inner.borrow() { Value::[<Matrix $kind:camel>](Matrix::RowVector3(v)) => Some(v.clone()), _ => None, }, _ => None, } }
      #[cfg(not(feature = "row_vector3"))]
      fn get_r3(_value: &Value) -> Option<()> { None }

      #[cfg(feature = "row_vector4")] // get_r4
      fn get_r4(value: &Value) -> Option<Ref<RowVector4<$kind>>> { match value { Value::[<Matrix $kind:camel>](Matrix::RowVector4(v)) => Some(v.clone()), Value::MutableReference(inner) => match &*inner.borrow() { Value::[<Matrix $kind:camel>](Matrix::RowVector4(v)) => Some(v.clone()), _ => None, }, _ => None, } }
      #[cfg(not(feature = "row_vector4"))]
      fn get_r4(_value: &Value) -> Option<()> { None }

      #[cfg(feature = "vector2")] // get_v2
      fn get_v2(value: &Value) -> Option<Ref<Vector2<$kind>>> { match value { Value::[<Matrix $kind:camel>](Matrix::Vector2(v)) => Some(v.clone()), Value::MutableReference(inner) => match &*inner.borrow() { Value::[<Matrix $kind:camel>](Matrix::Vector2(v)) => Some(v.clone()), _ => None, }, _ => None, } }
      #[cfg(not(feature = "vector2"))]
      fn get_v2(_value: &Value) -> Option<()> { None }

      #[cfg(feature = "vector3")] // get_v3
      fn get_v3(value: &Value) -> Option<Ref<Vector3<$kind>>> { match value { Value::[<Matrix $kind:camel>](Matrix::Vector3(v)) => Some(v.clone()), Value::MutableReference(inner) => match &*inner.borrow() { Value::[<Matrix $kind:camel>](Matrix::Vector3(v)) => Some(v.clone()), _ => None, }, _ => None, } }
      #[cfg(not(feature = "vector3"))]
      fn get_v3(_value: &Value) -> Option<()> { None }

      #[cfg(feature = "vector4")] // get_v4
      fn get_v4(value: &Value) -> Option<Ref<Vector4<$kind>>> { match value { Value::[<Matrix $kind:camel>](Matrix::Vector4(v)) => Some(v.clone()), Value::MutableReference(inner) => match &*inner.borrow() { Value::[<Matrix $kind:camel>](Matrix::Vector4(v)) => Some(v.clone()), _ => None, }, _ => None, } }
      #[cfg(not(feature = "vector4"))]
      fn get_v4(_value: &Value) -> Option<()> { None }

      #[cfg(feature = "matrixd")] // get_md
      fn get_md(value: &Value) -> Option<Ref<DMatrix<$kind>>> { match value { Value::[<Matrix $kind:camel>](Matrix::DMatrix(v)) => Some(v.clone()), Value::MutableReference(inner) => match &*inner.borrow() { Value::[<Matrix $kind:camel>](Matrix::DMatrix(v)) => Some(v.clone()), _ => None, }, _ => None, } }
      #[cfg(not(feature = "matrixd"))]
      fn get_md(_value: &Value) -> Option<()> { None }

      #[cfg(feature = "row_vectord")] // get_rd
      fn get_rd(value: &Value) -> Option<Ref<RowDVector<$kind>>> { match value { Value::[<Matrix $kind:camel>](Matrix::RowDVector(v)) => Some(v.clone()), Value::MutableReference(inner) => match &*inner.borrow() { Value::[<Matrix $kind:camel>](Matrix::RowDVector(v)) => Some(v.clone()), _ => None, }, _ => None, } }
      #[cfg(not(feature = "row_vectord"))]
      fn get_rd(_value: &Value) -> Option<()> { None }

      #[cfg(feature = "matrix3x2")] // get_m3x2
      fn get_m3x2(value: &Value) -> Option<Ref<Matrix3x2<$kind>>> { match value { Value::[<Matrix $kind:camel>](Matrix::Matrix3x2(v)) => Some(v.clone()), Value::MutableReference(inner) => match &*inner.borrow() { Value::[<Matrix $kind:camel>](Matrix::Matrix3x2(v)) => Some(v.clone()), _ => None, }, _ => None, } }
      #[cfg(not(feature = "matrix3x2"))]
      fn get_m3x2(_value: &Value) -> Option<()> { None }

      #[cfg(feature = "matrix2x3")] // get_m2x3
      fn get_m2x3(value: &Value) -> Option<Ref<Matrix2x3<$kind>>> { match value { Value::[<Matrix $kind:camel>](Matrix::Matrix2x3(v)) => Some(v.clone()), Value::MutableReference(inner) => match &*inner.borrow() { Value::[<Matrix $kind:camel>](Matrix::Matrix2x3(v)) => Some(v.clone()), _ => None, }, _ => None, } }
      #[cfg(not(feature = "matrix2x3"))]
      fn get_m2x3(_value: &Value) -> Option<()> { None }

      #[cfg(feature = "matrix1")] // get_m1
      fn get_m1(value: &Value) -> Option<Ref<Matrix1<$kind>>> { match value { Value::[<Matrix $kind:camel>](Matrix::Matrix1(v)) => Some(v.clone()), Value::MutableReference(inner) => match &*inner.borrow() { Value::[<Matrix $kind:camel>](Matrix::Matrix1(v)) => Some(v.clone()), _ => None, }, _ => None, } }
      #[cfg(not(feature = "matrix1"))]
      fn get_m1(_value: &Value) -> Option<()> { None }

      #[cfg(feature = "matrix2")] // get_m2
      fn get_m2(value: &Value) -> Option<Ref<Matrix2<$kind>>> { match value { Value::[<Matrix $kind:camel>](Matrix::Matrix2(v)) => Some(v.clone()), Value::MutableReference(inner) => match &*inner.borrow() { Value::[<Matrix $kind:camel>](Matrix::Matrix2(v)) => Some(v.clone()), _ => None, }, _ => None, } }
      #[cfg(not(feature = "matrix2"))]
      fn get_m2(_value: &Value) -> Option<()> { None }

      #[cfg(feature = "matrix3")] // get_m3
      fn get_m3(value: &Value) -> Option<Ref<Matrix3<$kind>>> { match value { Value::[<Matrix $kind:camel>](Matrix::Matrix3(v)) => Some(v.clone()), Value::MutableReference(inner) => match &*inner.borrow() { Value::[<Matrix $kind:camel>](Matrix::Matrix3(v)) => Some(v.clone()), _ => None, }, _ => None, } }
      #[cfg(not(feature = "matrix3"))]
      fn get_m3(_value: &Value) -> Option<()> { None }

      #[cfg(feature = "matrix4")] // get_m4
      fn get_m4(value: &Value) -> Option<Ref<Matrix4<$kind>>> { match value { Value::[<Matrix $kind:camel>](Matrix::Matrix4(v)) => Some(v.clone()), Value::MutableReference(inner) => match &*inner.borrow() { Value::[<Matrix $kind:camel>](Matrix::Matrix4(v)) => Some(v.clone()), _ => None, }, _ => None, } }
      #[cfg(not(feature = "matrix4"))]
      fn get_m4(_value: &Value) -> Option<()> { None }

      fn get_s(value: &Value) -> Option<Ref<$kind>> { match value { Value::[<$kind:camel>](v) => Some(v.clone()), Value::MutableReference(inner) => match &*inner.borrow() { Value::[<$kind:camel>](v) => Some(v.clone()), _ => None, }, _ => None, } }

      let arguments = $args;
      let rows = arguments[0].shape()[0];
      let columns:usize = arguments.iter().fold(0, |acc, x| acc + x.shape()[1]);
      let rows:usize = arguments[0].shape()[0];
      let nargs = arguments.len();
      let kinds: Vec<ValueKind> = arguments.iter().map(|x| x.kind()).collect::<Vec<ValueKind>>();
      let no_refs = !kinds.iter().any(|x| {
        match x {
          ValueKind::Reference(_) => true,
          _ => false,
      }});
        match (nargs,rows,columns) {
          #[cfg(feature = "matrix1")]
          (1,1,1) => {
            let a_m1 = get_m1(&arguments[0]);
            let a_sc = get_s(&arguments[0]);
            match (a_m1, a_sc) {
              (Some(ref e0), None) => return Ok(Box::new(HorizontalConcatenateM1{out: e0.clone()})),
              (None, Some(ref e0)) => return Ok(Box::new(HorizontalConcatenateS1{arg: e0.clone(), out: Ref::new(Matrix1::from_element($default))})),
              _ => return Err(MechError { file: file!().to_string(), tokens: vec![], msg: format!("Expected a Matrix1<{}> or Scalar<{}> for horizontal concatenation, found {:?}", stringify!($kind), stringify!($kind), arguments), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
          #[cfg(feature = "row_vector2")]
          (1, 1, 2) => {
            let er2 = get_r2(&arguments[0]);
            match &er2 {
              Some(ref e0) => return Ok(Box::new(HorizontalConcatenateR2 {out: e0.clone() })),
              _ => return Err(MechError { file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
          #[cfg(feature = "row_vector3")]
          (1, 1, 3) => {
            let er3 = get_r3(&arguments[0]);
            match &er3 {
              Some(ref e0) => return Ok(Box::new(HorizontalConcatenateR3 { out: e0.clone() })),
              _ => return Err(MechError {file: file!().to_string(),tokens: vec![],msg: "".to_string(),id: line!(),kind: MechErrorKind::UnhandledFunctionArgumentKind}),
            }
          }
          #[cfg(feature = "row_vector4")]
          (1, 1, 4) => {
            let er4 = get_r4(&arguments[0]);
            match &er4 {
              Some(ref e0) => return Ok(Box::new(HorizontalConcatenateR4{out: e0.clone()})),
              _ => return Err(MechError{file: file!().to_string(),tokens: vec![],msg: "".to_string(),id: line!(),kind: MechErrorKind::UnhandledFunctionArgumentKind}),
            }
          }
          #[cfg(feature = "row_vectord")]
          (1, 1, n) => {
            let erd = get_rd(&arguments[0]);
            match &erd {
              Some(ref e0) => return Ok(Box::new(HorizontalConcatenateRD{out: e0.clone()})),
              _ => return Err(MechError{file: file!().to_string(),tokens: vec![],msg: "".to_string(),id: line!(),kind: MechErrorKind::UnhandledFunctionArgumentKind}),
            }
          }
          #[cfg(feature = "row_vector2")]
          (2,1,2) => {
            let mut out = RowVector2::from_element($default);
            let am1 = get_m1(&arguments[0]);
            let bm1 = get_m1(&arguments[1]);
            let asc = get_s(&arguments[0]);
            let bsc = get_s(&arguments[1]);
            match (am1, bm1, asc, bsc) {
              #[cfg(feature = "matrix1")]
              (Some(ref e0), Some(ref e1), None, None) => return Ok(Box::new(HorizontalConcatenateM1M1 { e0: e0.clone(), e1: e1.clone(), out: Ref::new(out) })),
              #[cfg(feature = "matrix1")]
              (Some(ref e0), None, None, Some(ref e1)) => return Ok(Box::new(HorizontalConcatenateM1S { e0: e0.clone(), e1: e1.clone(), out: Ref::new(out) })),
              #[cfg(feature = "matrix1")]
              (None, Some(ref e1), Some(ref e0), None) => return Ok(Box::new(HorizontalConcatenateSM1 { e0: e0.clone(), e1: e1.clone(), out: Ref::new(out) })),
              (None, None, Some(ref e0), Some(ref e1)) => return Ok(Box::new(HorizontalConcatenateS2 { e0: e0.clone(), e1: e1.clone(), out: Ref::new(out) })),
              _ => Err(MechError { file: file!().to_string(), tokens: vec![], msg: format!("Expected a Matrix1<{}> or Scalar<{}> for horizontal concatenation, found {:?}", stringify!($kind), stringify!($kind), arguments), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
          #[cfg(feature = "row_vector3")]
          (2,1,3) => {
            let mut out = RowVector3::from_element($default);
            let a_r2 = get_r2(&arguments[0]);
            let b_r2 = get_r2(&arguments[1]);
            let a_sc = get_s(&arguments[0]);
            let b_sc = get_s(&arguments[1]);
            let a_m1 = get_m1(&arguments[0]);
            let b_m1 = get_m1(&arguments[1]);
            match (a_r2, b_r2, a_sc, b_sc, a_m1, b_m1) {
              #[cfg(feature = "matrix1")]
              (Some(ref e0), _, _, _, _, Some(ref e1)) => return Ok(Box::new(HorizontalConcatenateR2M1 { e0: e0.clone(), e1: e1.clone(), out: Ref::new(out) })),
              #[cfg(feature = "row_vector2")]
              (Some(ref e0), _, _, Some(ref e1), _, _) => return Ok(Box::new(HorizontalConcatenateR2S { e0: e0.clone(), e1: e1.clone(), out: Ref::new(out) })),
              #[cfg(all(feature = "matrix1", feature = "row_vector2"))]
              (_, Some(ref e1), _, _, Some(ref e0), _) => return Ok(Box::new(HorizontalConcatenateM1R2 { e0: e0.clone(), e1: e1.clone(), out: Ref::new(out) })),
              #[cfg(feature = "row_vector2")]
              (_, Some(ref e1), Some(ref e0), _, _, _) => return Ok(Box::new(HorizontalConcatenateSR2 { e0: e0.clone(), e1: e1.clone(), out: Ref::new(out) })),
              _ => Err(MechError { file: file!().to_string(), tokens: vec![], msg: format!("Expected a RowVector2<{}>, Scalar<{}> or Matrix1<{}> for horizontal concatenation, found {:?}", stringify!($kind), stringify!($kind), stringify!($kind), arguments), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
          #[cfg(feature = "row_vector4")]
          (2,1,4) => {
            let mut out = RowVector4::from_element($default);
            let a_r3 = get_r3(&arguments[0]);
            let b_r3 = get_r3(&arguments[1]);
            let a_sc = get_s(&arguments[0]);
            let b_sc = get_s(&arguments[1]);
            let a_m1 = get_m1(&arguments[0]);
            let b_m1 = get_m1(&arguments[1]);
            let a_r2 = get_r2(&arguments[0]);
            let b_r2 = get_r2(&arguments[1]);
            match (a_r3, b_r3, a_sc, b_sc, a_m1, b_m1, a_r2, b_r2) {
              #[cfg(all(feature = "matrix1", feature = "row_vector3"))]
              (Some(ref e0), _, _, _, _, Some(ref e1), _, _) => return Ok(Box::new(HorizontalConcatenateR3M1 { e0: e0.clone(), e1: e1.clone(), out: Ref::new(out) })),
              #[cfg(feature = "row_vector3")]
              (Some(ref e0), _, _, Some(ref e1), _, _, _, _) => return Ok(Box::new(HorizontalConcatenateR3S { e0: e0.clone(), e1: e1.clone(), out: Ref::new(out) })),
              #[cfg(all(feature = "matrix1", feature = "row_vector3"))]
              (_, Some(ref e1), _, _, Some(ref e0), _, _, _) => return Ok(Box::new(HorizontalConcatenateM1R3 { e0: e0.clone(), e1: e1.clone(), out: Ref::new(out) })),
              #[cfg(feature = "row_vector3")]
              (_, Some(ref e1), Some(ref e0), _, _, _, _, _) => return Ok(Box::new(HorizontalConcatenateSR3 { e0: e0.clone(), e1: e1.clone(), out: Ref::new(out) })),
              #[cfg(feature = "row_vector2")]
              (_, _, _, _, _, _, Some(ref e0), Some(ref e1)) => return Ok(Box::new(HorizontalConcatenateR2R2 { e0: e0.clone(), e1: e1.clone(), out: Ref::new(out) })),
              _ => Err(MechError { file: file!().to_string(), tokens: vec![], msg: format!("Expected a RowVector3<{}>, Scalar<{}>, Matrix1<{}> or RowVector2<{}> for horizontal concatenation, found {:?}", stringify!($kind), stringify!($kind), stringify!($kind), stringify!($kind), arguments), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          } 
          #[cfg(feature = "row_vector3")]
          (3,1,3) => {  
            let mut out = RowVector3::from_element($default);
            let a_m1 = get_m1(&arguments[0]);
            let b_m1 = get_m1(&arguments[1]);
            let c_m1 = get_m1(&arguments[2]);
            let a_sc = get_s(&arguments[0]);
            let b_sc = get_s(&arguments[1]);
            let c_sc = get_s(&arguments[2]);
            match (a_m1, b_m1, c_m1, a_sc, b_sc, c_sc) {
              #[cfg(feature = "matrix1")]
              (_, _, _, Some(ref e0), Some(ref e1), Some(ref e2)) => return Ok(Box::new(HorizontalConcatenateS3 {e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out)})),
              #[cfg(feature = "matrix1")]
              (Some(ref e0), Some(ref e1), _, _, _, Some(ref e2)) => return Ok(Box::new(HorizontalConcatenateM1M1S { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out) })),
              #[cfg(feature = "matrix1")]
              (Some(ref e0), _, Some(ref e2), _, Some(ref e1), _) => return Ok(Box::new(HorizontalConcatenateM1SM1 { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out) })),
              #[cfg(feature = "matrix1")]
              (_, Some(ref e1), Some(ref e2), Some(ref e0), _, _) => return Ok(Box::new(HorizontalConcatenateSM1M1 { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out) })),
              #[cfg(feature = "matrix1")]
              (_, Some(ref e1), _, Some(ref e0), _, Some(ref e2)) => return Ok(Box::new(HorizontalConcatenateSM1S {e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out)})),
              #[cfg(feature = "matrix1")]
              (_, _, Some(ref e2), Some(ref e0), Some(ref e1), _) => return Ok(Box::new(HorizontalConcatenateSSM1 {e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out)})),
              #[cfg(feature = "matrix1")]
              (Some(ref e0), _, _, _, Some(ref e1), Some(ref e2)) => return Ok(Box::new(HorizontalConcatenateM1SS {e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out) })),
             #[cfg(feature = "matrix1")]
              (Some(ref e0), Some(ref e1), Some(ref e2), _, _, _) => return Ok(Box::new(HorizontalConcatenateM1M1M1 { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out) })),
              _ => return Err(MechError { file: file!().to_string(), tokens: vec![], msg: format!("Expected a Matrix1<{}> or Scalar<{}> for horizontal concatenation, found {:?}", stringify!($kind), stringify!($kind), arguments), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
          #[cfg(feature = "row_vector4")]
          (3,1,4) => {
            let mut out = RowVector4::from_element($default);
            let a_sc = get_s(&arguments[0]);
            let b_sc = get_s(&arguments[1]);
            let c_sc = get_s(&arguments[2]);
            let a_r2 = get_r2(&arguments[0]);
            let b_r2 = get_r2(&arguments[1]);
            let c_r2 = get_r2(&arguments[2]);
            let a_m1 = get_m1(&arguments[0]);
            let b_m1 = get_m1(&arguments[1]);
            let c_m1 = get_m1(&arguments[2]);
            match (a_sc, b_sc, c_sc, a_r2, b_r2, c_r2, a_m1, b_m1, c_m1) {
              #[cfg(feature = "row_vector2")]
              (Some(ref e0), Some(ref e1), _, _, _, Some(ref e2), _, _, _) => return Ok(Box::new(HorizontalConcatenateSSR2{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out)})),
              #[cfg(feature = "row_vector2")]
              (Some(ref e0), _, Some(ref e2), _, Some(ref e1), _, _, _, _) => return Ok(Box::new(HorizontalConcatenateSR2S{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out)})),
              #[cfg(feature = "row_vector2")]
              (_, Some(ref e1), Some(ref e2), Some(ref e0), _, _, _, _, _) => return Ok(Box::new(HorizontalConcatenateR2SS{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out)})),
              #[cfg(all(feature = "matrix1", feature = "row_vector2"))]
              (_, _, _, _, _, Some(ref e2), Some(ref e0), Some(ref e1), _) => return Ok(Box::new(HorizontalConcatenateM1M1R2{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out)})),
              #[cfg(all(feature = "matrix1", feature = "row_vector2"))]
              (_, _, _, _, Some(ref e1), _, Some(ref e0), _, Some(ref e2)) => return Ok(Box::new(HorizontalConcatenateM1R2M1{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out)})),
              #[cfg(all(feature = "matrix1", feature = "row_vector2"))]
              (_, _, _, Some(ref e0), _, _, _, Some(ref e1), Some(ref e2)) => return Ok(Box::new(HorizontalConcatenateR2M1M1{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out)})),
              #[cfg(all(feature = "matrix1", feature = "row_vector2"))]
              (Some(ref e0), _, _, _, _, Some(ref e2), _, Some(ref e1), _) => return Ok(Box::new(HorizontalConcatenateSM1R2{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out)})),
              #[cfg(all(feature = "matrix1", feature = "row_vector2"))]
              (Some(ref e0), _, _, _, Some(ref e1), _, _, _, Some(ref e2)) => return Ok(Box::new(HorizontalConcatenateSR2M1{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out)})),
              #[cfg(all(feature = "matrix1", feature = "row_vector2"))]
              (_, Some(ref e1), _, _, _, Some(ref e2), Some(ref e0), _, _) => return Ok(Box::new(HorizontalConcatenateM1SR2{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out)})),
              #[cfg(all(feature = "matrix1", feature = "row_vector2"))]
              (_, Some(ref e1), _, Some(ref e0), _, _, _, _, Some(ref e2)) => return Ok(Box::new(HorizontalConcatenateR2SM1{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out)})),
              #[cfg(all(feature = "matrix1", feature = "row_vector2"))]
              (_, _, Some(ref e2), _, Some(ref e1), _, Some(ref e0), _, _) => return Ok(Box::new(HorizontalConcatenateM1R2S{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out)})),
              #[cfg(all(feature = "matrix1", feature = "row_vector2"))]
              (_, _, Some(ref e2), Some(ref e0), _, _, _, Some(ref e1), _) => return Ok(Box::new(HorizontalConcatenateR2M1S{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out)})),
              _ => return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
            }
          }
          #[cfg(feature = "row_vector4")]
          (4,1,4) => {
            let mut out = RowVector4::from_element($default);
            let a_s = get_s(&arguments[0]);
            let b_s = get_s(&arguments[1]);
            let c_s = get_s(&arguments[2]);
            let d_s = get_s(&arguments[3]);
            let a_m1 = get_m1(&arguments[0]);
            let b_m1 = get_m1(&arguments[1]);
            let c_m1 = get_m1(&arguments[2]);
            let d_m1 = get_m1(&arguments[3]);
            match (a_s, b_s, c_s, d_s, a_m1, b_m1, c_m1, d_m1) {
              (Some(ref e0), Some(ref e1), Some(ref e2), Some(ref e3), _, _, _, _) => return Ok(Box::new(HorizontalConcatenateS4 { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), e3: e3.clone(), out: Ref::new(out) })),
              #[cfg(feature = "matrix1")]
              (Some(ref e0), Some(ref e1), Some(ref e2), _, _, _, _, Some(ref e3)) => return Ok(Box::new(HorizontalConcatenateSSSM1 { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), e3: e3.clone(), out: Ref::new(out) })),
              #[cfg(feature = "matrix1")]
              (Some(ref e0), Some(ref e1), _, Some(ref e3), _, _, Some(ref e2), _) => return Ok(Box::new(HorizontalConcatenateSSM1S { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), e3: e3.clone(), out: Ref::new(out) })),
              #[cfg(feature = "matrix1")]
              (Some(ref e0), _, Some(ref e2), Some(ref e3), _, Some(ref e1), _, _) => return Ok(Box::new(HorizontalConcatenateSM1SS { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), e3: e3.clone(), out: Ref::new(out) })),
              #[cfg(feature = "matrix1")]
              (_, Some(ref e1), Some(ref e2), Some(ref e3), Some(ref e0), _, _, _) => return Ok(Box::new(HorizontalConcatenateM1SSS { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), e3: e3.clone(), out: Ref::new(out) })),
              #[cfg(feature = "matrix1")]
              (Some(ref e0), Some(ref e1), _, _, _, Some(ref e2), _, Some(ref e3)) => return Ok(Box::new(HorizontalConcatenateSSM1M1 { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), e3: e3.clone(), out: Ref::new(out) })),
              #[cfg(feature = "matrix1")]
              (Some(ref e0), _, Some(ref e2), _, _, Some(ref e1), _, Some(ref e3)) => return Ok(Box::new(HorizontalConcatenateSM1SM1 { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), e3: e3.clone(), out: Ref::new(out) })),
              #[cfg(feature = "matrix1")]
              (_, _, Some(ref e2), Some(ref e3), Some(ref e0), Some(ref e1), _, _) => return Ok(Box::new(HorizontalConcatenateM1M1SS { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), e3: e3.clone(), out: Ref::new(out) })),
              #[cfg(feature = "matrix1")]
              (Some(ref e0), _, _, Some(ref e3), _, Some(ref e1), Some(ref e2), _) => return Ok(Box::new(HorizontalConcatenateSM1M1S { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), e3: e3.clone(), out: Ref::new(out) })),
              #[cfg(feature = "matrix1")]
              (_, Some(ref e1), Some(ref e2), _, Some(ref e0), _, _, Some(ref e3)) => return Ok(Box::new(HorizontalConcatenateM1SSM1 { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), e3: e3.clone(), out: Ref::new(out) })),
              #[cfg(feature = "matrix1")]
              (_, Some(ref e1), _, Some(ref e3), Some(ref e0), _, Some(ref e2), _) => return Ok(Box::new(HorizontalConcatenateM1SM1S { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), e3: e3.clone(), out: Ref::new(out) })),
              #[cfg(feature = "matrix1")]
              (Some(ref e0), _, _, _, _, Some(ref e1), Some(ref e2), Some(ref e3)) => return Ok(Box::new(HorizontalConcatenateSM1M1M1 { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), e3: e3.clone(), out: Ref::new(out) })),
              #[cfg(feature = "matrix1")]
              (_, Some(ref e1), _, _, Some(ref e0), _, Some(ref e2), Some(ref e3)) => return Ok(Box::new(HorizontalConcatenateM1SM1M1 { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), e3: e3.clone(), out: Ref::new(out) })),
              #[cfg(feature = "matrix1")]
              (_, _, Some(ref e2), _, Some(ref e0), Some(ref e1), _, Some(ref e3)) => return Ok(Box::new(HorizontalConcatenateM1M1SM1 { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), e3: e3.clone(), out: Ref::new(out) })),
              #[cfg(feature = "matrix1")]
              (_, _, _, Some(ref e3), Some(ref e0), Some(ref e1), Some(ref e2), _) => return Ok(Box::new(HorizontalConcatenateM1M1M1S { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), e3: e3.clone(), out: Ref::new(out) })),
              #[cfg(feature = "matrix1")]
              (_, _, _, _, Some(ref e0), Some(ref e1), Some(ref e2), Some(ref e3)) => return Ok(Box::new(HorizontalConcatenateM1M1M1M1 { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), e3: e3.clone(), out: Ref::new(out) })),
              _ => return Err(MechError { file: file!().to_string(), tokens: vec![], msg: format!("Expected a Scalar<{}> or Matrix1<{}> for horizontal concatenation, found {:?}", stringify!($kind), stringify!($kind), arguments), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
          #[cfg(feature = "row_vectord")]
          (m,1,n) => {
            let mut out = RowDVector::from_element(n,$default);
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
                  i += e0.shape()[1];
                }
                Value::MutableReference(e0) => {
                  match e0.borrow().clone() {
                    Value::[<Matrix $kind:camel>](e0) => {
                      matrix_args.push((e0.get_copyable_matrix(),i));
                      i += e0.shape()[1];
                    }
                    Value::[<$kind:camel>](e0) => {
                      scalar_args.push((e0.clone(),i));
                      i += 1;
                    }
                    _ => todo!(),
                  }
                }
                _ => todo!(),
              }
            }
            return Ok(Box::new(HorizontalConcatenateRDN{scalar: scalar_args, matrix: matrix_args, out: Ref::new(out)}));
          }
          #[cfg(feature = "vector2")]
          (1, 2, 1) => {
            let ev2 = get_v2(&arguments[0]);
            match &ev2 {
              Some(ref e0) => return Ok(Box::new(HorizontalConcatenateV2 { out: e0.clone() })),
              _ => return Err(MechError { file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
          #[cfg(feature = "matrix2")]
          (1, 2, 2) => {
            let em2 = get_m2(&arguments[0]);
            match &em2 {
              Some(ref e0) => return Ok(Box::new(HorizontalConcatenateM2 { out: e0.clone() })),
              _ => return Err(MechError { file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
          #[cfg(feature = "matrix2x3")]
          (1, 2, 3) => {
            let em2x3 = get_m2x3(&arguments[0]);
            match &em2x3 {
              Some(ref e0) => return Ok(Box::new(HorizontalConcatenateM2x3 { out: e0.clone() })),
              _ => return Err(MechError { file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
          #[cfg(feature = "vector3")]
          (1, 3, 1) => {
            let ev3 = get_v3(&arguments[0]);
            match &ev3 {
              Some(ref e0) => return Ok(Box::new(HorizontalConcatenateV3 { out: e0.clone() })),
              _ => return Err(MechError { file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
          #[cfg(feature = "matrix3x2")]
          (1, 3, 2) => {
            let am3x2 = get_m3x2(&arguments[0]);
            match &am3x2 {
              Some(ref e0) => return Ok(Box::new(HorizontalConcatenateM3x2{out: e0.clone()})),
              _ => return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
            }
          }
          #[cfg(feature = "matrix3")]
          (1, 3, 3) => {
            let em3 = get_m3(&arguments[0]);
            match &em3 {
              Some(ref e0) => return Ok(Box::new(HorizontalConcatenateM3 { out: e0.clone() })),
              _ => return Err(MechError { file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
          #[cfg(feature = "vector4")]
          (1, 4, 1) => {
            let ev4 = get_v4(&arguments[0]);
            match &ev4 {
              Some(ref e0) => return Ok(Box::new(HorizontalConcatenateV4 { out: e0.clone() })),
              _ => return Err(MechError { file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
          #[cfg(feature = "matrix4")]
          (1, 4, 4) => {
            let em4 = get_m4(&arguments[0]);
            match &em4 {
              Some(ref e0) => return Ok(Box::new(HorizontalConcatenateM4 { out: e0.clone() })),
              _ => return Err(MechError { file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
          #[cfg(feature = "matrixd")]
          (1, m, n) => {
            let emd = get_md(&arguments[0]);
            match &emd {
              Some(ref e0) => return Ok(Box::new(HorizontalConcatenateMD{out: e0.clone()})),
              _ => return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
            }
          }
          #[cfg(all(feature = "matrix2", feature ="vector2"))]
          (2, 2, 2) => {
            let mut out = Matrix2::from_element($default);
            let av2 = get_v2(&arguments[0]);
            let bv2 = get_v2(&arguments[1]);
            match (av2, bv2) {
              #[cfg(feature = "vector2")]
              (Some(e0), Some(e1)) => return Ok(Box::new(HorizontalConcatenateV2V2 { e0: e0.clone(), e1: e1.clone(), out: Ref::new(out) })),
              _ => return Err(MechError { file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
          #[cfg(feature = "matrix3x2")]
          (2, 3, 2) => {
            let mut out = Matrix3x2::from_element($default);
            let av3 = get_v3(&arguments[0]);
            let bv3 = get_v3(&arguments[1]);
            match (av3, bv3) {
              #[cfg(feature = "vector3")]
              (Some(e0), Some(e1)) => return Ok(Box::new(HorizontalConcatenateV3V3 { e0: e0.clone(), e1: e1.clone(), out: Ref::new(out) })),
              _ => return Err(MechError { file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
          #[cfg(feature = "matrix2x3")]
          (2,2,3) => {
            let mut out = Matrix2x3::from_element($default);
            let av2 = get_v2(&arguments[0]);
            let am2 = get_m2(&arguments[0]);
            let bv2 = get_v2(&arguments[1]);
            let bm2 = get_m2(&arguments[1]);
            match (av2, bv2, am2, bm2) {
              #[cfg(all(feature = "vector2", feature = "matrix2"))]
              (Some(ref e0), _, _, Some(ref e1)) => return Ok(Box::new(HorizontalConcatenateV2M2 { e0: e0.clone(), e1: e1.clone(), out: Ref::new(out) })),
              #[cfg(all(feature = "vector2", feature = "matrix2"))]
              (_, Some(ref e1), Some(ref e0), _) => return Ok(Box::new(HorizontalConcatenateM2V2 { e0: e0.clone(), e1: e1.clone(), out: Ref::new(out) })),
              _ => return Err(MechError { file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
          #[cfg(feature = "matrix3")]
          (2, 3, 3) => {
            let mut out = Matrix3::from_element($default);
            let av3 = get_v3(&arguments[0]);
            let am3x2 = get_m3x2(&arguments[0]);
            let bv3 = get_v3(&arguments[1]);
            let bm3x2 = get_m3x2(&arguments[1]);
            match (av3, bv3, am3x2, bm3x2) {
              #[cfg(all(feature = "vector3", feature = "matrix3x2"))]
              (Some(ref e0), _, _, Some(ref e1)) => return Ok(Box::new(HorizontalConcatenateV3M3x2 { e0: e0.clone(), e1: e1.clone(), out: Ref::new(out) })),
              #[cfg(all(feature = "vector3", feature = "matrix3x2"))]
              (_, Some(ref e1), Some(ref e0), _) => return Ok(Box::new(HorizontalConcatenateM3x2V3 { e0: e0.clone(), e1: e1.clone(), out: Ref::new(out) })),
              _ => return Err(MechError { file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
          #[cfg(feature = "matrix4")]
          (2, 4, 4) => {
            let mut out = Matrix4::from_element($default);
            let av4 = get_v4(&arguments[0]);
            let bv4 = get_v4(&arguments[1]);
            let amd = get_md(&arguments[0]);
            let bmd = get_md(&arguments[1]);
            match (av4, bv4, amd, bmd) {
              #[cfg(all(feature = "vector4", feature = "matrixd"))]
              (Some(ref e0), _, _, Some(ref e1)) => return Ok(Box::new(HorizontalConcatenateV4MD { e0: e0.clone(), e1: e1.clone(), out: Ref::new(out) })),
              #[cfg(all(feature = "vector4", feature = "matrixd"))]
              (_, Some(ref e1), Some(ref e0), _) => return Ok(Box::new(HorizontalConcatenateMDV4 { e0: e0.clone(), e1: e1.clone(), out: Ref::new(out) })),
              #[cfg(feature = "matrixd")]
              (_, _, Some(ref e0), Some(ref e1)) => return Ok(Box::new(HorizontalConcatenateMDMD { e0: e0.clone(), e1: e1.clone(), out: Ref::new(out) })),
              _ => return Err(MechError { file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
          #[cfg(feature = "matrixd")]
          (2,m,n) => {
            let mut out = DMatrix::from_element(m,n,$default);
            let e0 = extract_matrix(&arguments[0])?;
            let e1 = extract_matrix(&arguments[1])?;   
            Ok(Box::new(HorizontalConcatenateTwoArgs{e0,e1,out:Ref::new(out)}))
          }
          #[cfg(feature = "matrix2x3")]
          (3, 2, 3) => {
            let mut out = Matrix2x3::from_element($default);
            let av2 = get_v2(&arguments[0]);
            let bv2 = get_v2(&arguments[1]);
            let cv2 = get_v2(&arguments[2]);
            match (av2, bv2, cv2) {
              #[cfg(feature = "vector2")]
              (Some(ref e0), Some(ref e1), Some(ref e2)) => return Ok(Box::new(HorizontalConcatenateV2V2V2 { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out) })),
              _ => return Err(MechError { file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
          #[cfg(feature = "matrix3")]
          (3, 3, 3) => {
            let mut out = Matrix3::from_element($default);
            let av3 = get_v3(&arguments[0]);
            let bv3 = get_v3(&arguments[1]);
            let cv3 = get_v3(&arguments[2]);
            match (&av3, &bv3, &cv3) {
              #[cfg(feature = "vector3")]
              (Some(ref e0), Some(ref e1), Some(ref e2)) => return Ok(Box::new(HorizontalConcatenateV3V3V3 { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out) })),
              _ => return Err(MechError { file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
          #[cfg(feature = "matrix4")]
          (3, 4, 4) => {
            let mut out = Matrix4::from_element($default);
            let av4 = get_v4(&arguments[0]);
            let bv4 = get_v4(&arguments[1]);
            let cv4 = get_v4(&arguments[2]);
            let amd = get_md(&arguments[0]);
            let bmd = get_md(&arguments[1]);
            let cmd = get_md(&arguments[2]);
            match (av4, bv4, cv4, amd, bmd, cmd) {
              #[cfg(all(feature = "vector4", feature = "matrixd"))]
              (Some(ref e0), Some(ref e1), _, _, _, Some(ref e2)) => return Ok(Box::new(HorizontalConcatenateV4V4MD { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out) })),
              #[cfg(all(feature = "vector4", feature = "matrixd"))]
              (Some(ref e0), _, Some(ref e2), _, Some(ref e1), _) => return Ok(Box::new(HorizontalConcatenateV4MDV4 { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out) })),
              #[cfg(all(feature = "matrixd", feature = "vector4"))]
              (_, Some(ref e1), Some(ref e2), Some(ref e0), _, _) => return Ok(Box::new(HorizontalConcatenateMDV4V4 { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: Ref::new(out) })),
              _ => return Err(MechError { file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
          #[cfg(feature = "matrixd")]
          (3,m,n) => {
            let mut out = DMatrix::from_element(m, n, $default);
            let e0 = extract_matrix(&arguments[0])?;
            let e1 = extract_matrix(&arguments[1])?;
            let e2 = extract_matrix(&arguments[2])?;
            return Ok(Box::new(HorizontalConcatenateThreeArgs {e0,e1,e2,out: Ref::new(out)}));
          }
          #[cfg(feature = "matrix4")]
          (4, 4, 4) => {
            let mut out = Matrix4::from_element($default);
            let av4 = get_v4(&arguments[0]);
            let bv4 = get_v4(&arguments[1]);
            let cv4 = get_v4(&arguments[2]);
            let dv4 = get_v4(&arguments[3]);
            match (&av4, &bv4, &cv4, &dv4) {
              #[cfg(feature = "vector4")]
              (Some(ref e0), Some(ref e1), Some(ref e2), Some(ref e3)) => return Ok(Box::new(HorizontalConcatenateV4V4V4V4 { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), e3: e3.clone(), out: Ref::new(out) })),
              _ => return Err(MechError { file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
          #[cfg(feature = "matrixd")]
          (4,m,n) => {
            let mut out = DMatrix::from_element(m,n,$default);
            let e0 = extract_matrix(&arguments[0])?;
            let e1 = extract_matrix(&arguments[1])?;
            let e2 = extract_matrix(&arguments[2])?;
            let e3 = extract_matrix(&arguments[3])?;
            return Ok(Box::new(HorizontalConcatenateFourArgs {e0,e1,e2,e3,out: Ref::new(out)}));
          }
          #[cfg(feature = "matrixd")]
          (l,m,n) => {
            let mut out = DMatrix::from_element(m,n,$default);
            let mut args = vec![];
            for arg in arguments {
              let e0 = extract_matrix(&arg)?;
              args.push(e0);
            }
            Ok(Box::new(HorizontalConcatenateNArgs{e0: args, out:Ref::new(out.clone())}))
          }
          _ => {return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind});}
        }
  }}}}

fn impl_horzcat_fxn(arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
  // are they all the same?
  //let same = kinds.iter().all(|x| *x == target_kind);
  let kinds: Vec<ValueKind> = arguments.iter().map(|x| x.kind()).collect::<Vec<ValueKind>>();
  let target_kind = kinds[0].clone();

  #[cfg(feature = "f64")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::F64) { return impl_horzcat_arms!(F64, arguments, F64::default()) } }

  #[cfg(feature = "f32")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::F32) { return impl_horzcat_arms!(F32, arguments, F32::default()) } }

  #[cfg(feature = "u8")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::U8)  { return impl_horzcat_arms!(u8,  arguments, u8::default()) } }

  #[cfg(feature = "u16")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::U16) { return impl_horzcat_arms!(u16, arguments, u16::default()) } }

  #[cfg(feature = "u32")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::U32) { return impl_horzcat_arms!(u32, arguments, u32::default()) } }

  #[cfg(feature = "u64")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::U64) { return impl_horzcat_arms!(u64, arguments, u64::default()) } }

  #[cfg(feature = "u128")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::U128){ return impl_horzcat_arms!(u128,arguments, u128::default()) } }

  #[cfg(feature = "i8")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::I8)  { return impl_horzcat_arms!(i8,  arguments, i8::default()) } }

  #[cfg(feature = "i16")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::I16) { return impl_horzcat_arms!(i16, arguments, i16::default()) } }

  #[cfg(feature = "i32")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::I32) { return impl_horzcat_arms!(i32, arguments, i32::default()) } }

  #[cfg(feature = "i64")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::I64) { return impl_horzcat_arms!(i64, arguments, i64::default()) } }

  #[cfg(feature = "i128")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::I128){ return impl_horzcat_arms!(i128,arguments, i128::default()) } }

  #[cfg(feature = "bool")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::Bool) { return impl_horzcat_arms!(bool, arguments, bool::default()) } }

  #[cfg(feature = "string")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::String) { return impl_horzcat_arms!(String, arguments, String::default()) } }

  #[cfg(feature = "rational")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::RationalNumber) { return impl_horzcat_arms!(RationalNumber, arguments, RationalNumber::default()) } }

  #[cfg(feature = "complex")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::ComplexNumber) { return impl_horzcat_arms!(ComplexNumber, arguments, ComplexNumber::default()) } }

  Err(MechError {
    file: file!().to_string(),
    tokens: vec![],
    msg: format!("Horizontal concatenation not implemented for type {:?}", target_kind),
    id: line!(),
    kind: MechErrorKind::UnhandledFunctionArgumentKind,
  })
}


pub struct MatrixHorzCat {}
impl NativeFunctionCompiler for MatrixHorzCat {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    impl_horzcat_fxn(arguments)
  }
}