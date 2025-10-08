use crate::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

#[derive(Debug)]
struct IoPrintlnMatrix<T,Mat> {
  e0: Ref<Mat>,
  _marker: PhantomData<T>,
}
impl<T,Mat> MechFunctionFactory for IoPrintlnMatrix<T,Mat>
where
  T: Clone + Sync + Send + 'static + Display + 
  CompileConst + ConstElem + AsValueKind +
  Debug,
  for<'a> &'a Mat: IntoIterator<Item = &'a T>,
  Mat: Debug + Clone +
       CompileConst + ConstElem + AsValueKind + 'static + Default,
{
  fn new(args: FunctionArgs) -> Result<Box<dyn MechFunction>, MechError> {
    match args {
      FunctionArgs::Nullary(out) => {
        let out: Ref<Value> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(Self {e0: Ref::new(Mat::default()), _marker: PhantomData::default()}))
      },
      _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("IoPrintlnMatrix requires 0 argument"), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments})
    }
  }
}
impl<T,Mat> MechFunctionImpl for IoPrintlnMatrix<T,Mat>
where
  T: Clone + Sync + Send + 'static + Display + Debug,
  for<'a> &'a Mat: IntoIterator<Item = &'a T>,
  Mat: Debug + Clone
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      for el in e0_ptr.into_iter() {
        #[cfg(not(target_arch = "wasm32"))]
        println!("{} ", el);
        #[cfg(target_arch = "wasm32")]
        log!("{} ", el);
      }  
    }
  }
  fn out(&self) -> Value { Value::Empty }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl<T,Mat> MechFunctionCompiler for IoPrintlnMatrix<T,Mat> 
where
  T: CompileConst + ConstElem + AsValueKind,
  Mat: CompileConst + ConstElem + AsValueKind,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("IoPrintlnMatrix<{}>", Mat::as_value_kind());
    compile_nullop!(name, self.e0, ctx, FeatureFlag::Custom(hash_str("io/print")) );
  }
}
impl_register_all!(IoPrintlnMatrix);

macro_rules! impl_print_match_arms {
  ($arg:expr, $($input_type:ident, $value_string:tt),+) => {
    paste!{
      match $arg {
        $(
          #[cfg(all(feature = $value_string, feature = "matrix1"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Matrix1(input))) => Ok(Box::new(IoPrintlnMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "matrix2"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Matrix2(input))) => Ok(Box::new(IoPrintlnMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "matrix3"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Matrix3(input))) => Ok(Box::new(IoPrintlnMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "matrix4"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Matrix4(input))) => Ok(Box::new(IoPrintlnMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "vector2"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Vector2(input))) => Ok(Box::new(IoPrintlnMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "vector3"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Vector3(input))) => Ok(Box::new(IoPrintlnMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "vector4"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Vector4(input))) => Ok(Box::new(IoPrintlnMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "row_vector2"))]
          (Value::[<Matrix $input_type:camel>](Matrix::RowVector2(input))) => Ok(Box::new(IoPrintlnMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "row_vector3"))]
          (Value::[<Matrix $input_type:camel>](Matrix::RowVector3(input))) => Ok(Box::new(IoPrintlnMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "row_vector4"))]
          (Value::[<Matrix $input_type:camel>](Matrix::RowVector4(input))) => Ok(Box::new(IoPrintlnMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "row_vectord"))]
          (Value::[<Matrix $input_type:camel>](Matrix::RowDVector(input))) => Ok(Box::new(IoPrintlnMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "matrixd"))]
          (Value::[<Matrix $input_type:camel>](Matrix::DMatrix(input))) => Ok(Box::new(IoPrintlnMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "vectord"))]
          (Value::[<Matrix $input_type:camel>](Matrix::DVector(input))) => Ok(Box::new(IoPrintlnMatrix{e0: input.clone(), _marker: PhantomData::default()})),
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

#[derive(Debug)]
struct IoPrintlnScalar<T> {
  e0: Ref<T>,
}
impl<T> MechFunctionFactory for IoPrintlnScalar<T> 
where
  T: Clone + Sync + Send + 'static + Display + 
  CompileConst + ConstElem + AsValueKind +
  Debug + Default,
{
  fn new(args: FunctionArgs) -> Result<Box<dyn MechFunction>, MechError> {
    match args {
      FunctionArgs::Nullary(out) => {
        let e0: Ref<T> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(Self {e0}))
      },
      _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("IoPrintlnScalar requires 0 argument"), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments})
    }
  }
}
impl<T> MechFunctionImpl for IoPrintlnScalar<T> 
  where T: Clone + Sync + Send + 'static + Display + Debug {
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      #[cfg(not(target_arch = "wasm32"))]
      print!("{} ", e0_ptr);
      #[cfg(target_arch = "wasm32")]
      log!("{} ", e0_ptr);
    }
  }
  fn out(&self) -> Value { Value::Empty }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for IoPrintlnScalar<T> 
where
  T: CompileConst + ConstElem + AsValueKind,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("IoPrintlnScalar<{}>", T::as_value_kind());
    compile_nullop!(name, self.e0, ctx, FeatureFlag::Custom(hash_str("io/print")) );
  }
}
register_fxn_descriptor!(IoPrintlnScalar, i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", F32, "f32", F64, "f64", bool, "bool", String, "string", C64, "complex", R64, "rational");

fn impl_print_fxn(source_value: Value) -> MResult<Box<dyn MechFunction>>  {
  if source_value.is_scalar() {
    match source_value {
      #[cfg(feature = "i8")]
      Value::I8(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "i16")]
      Value::I16(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "i32")]
      Value::I32(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "i64")]
      Value::I64(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "i128")]
      Value::I128(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "u8")]
      Value::U8(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "u16")]
      Value::U16(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "u32")]
      Value::U32(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "u64")]
      Value::U64(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "u128")]
      Value::U128(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "f32")]
      Value::F32(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "f64")]
      Value::F64(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "string")]
      Value::String(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "bool")]
      Value::Bool(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "complex")]
      Value::C64(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "rational")]
      Value::R64(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      _ => todo!(),
    }
  }
  impl_print_match_arms!(
    (source_value),
    i8,   "i8",
    i16,  "i16",
    i32,  "i32",
    i64,  "i64",
    i128, "i128",
    u8,   "u8",
    u16,  "u16",
    u32,  "u32",
    u64,  "u64",
    u128, "u128",
    F32,  "f32",
    F64,  "f64",
    bool, "bool",
    String, "string",
    C64, "complex",
    R64, "rational"
  )
}

pub struct IoPrintln {}

impl NativeFunctionCompiler for IoPrintln {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_print_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_print_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

register_descriptor!{
  FunctionCompilerDescriptor {
    name: "io/println",
    ptr: &IoPrintln{},
  }
}