use crate::*;

#[derive(Debug)]
struct IoPrintlnMatrix<T,Mat> {
  e0: Ref<Mat>,
  _marker: PhantomData<T>,
}
impl<T,Mat> MechFunction for IoPrintlnMatrix<T,Mat>
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

macro_rules! impl_print_match_arms {
  ($arg:expr, $($input_type:ident, $value_string:tt),+) => {
    paste!{
      match $arg {
        $(
          #[cfg(all(feature = $value_string, feature = "Matrix1"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Matrix1(input))) => Ok(Box::new(IoPrintlnMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "Matrix2"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Matrix2(input))) => Ok(Box::new(IoPrintlnMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "Matrix3"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Matrix3(input))) => Ok(Box::new(IoPrintlnMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "Matrix4"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Matrix4(input))) => Ok(Box::new(IoPrintlnMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "Vector2"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Vector2(input))) => Ok(Box::new(IoPrintlnMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "Vector3"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Vector3(input))) => Ok(Box::new(IoPrintlnMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "Vector4"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Vector4(input))) => Ok(Box::new(IoPrintlnMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "RowVector2"))]
          (Value::[<Matrix $input_type:camel>](Matrix::RowVector2(input))) => Ok(Box::new(IoPrintlnMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "RowVector3"))]
          (Value::[<Matrix $input_type:camel>](Matrix::RowVector3(input))) => Ok(Box::new(IoPrintlnMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "RowVector4"))]
          (Value::[<Matrix $input_type:camel>](Matrix::RowVector4(input))) => Ok(Box::new(IoPrintlnMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "RowVectorD"))]
          (Value::[<Matrix $input_type:camel>](Matrix::RowDVector(input))) => Ok(Box::new(IoPrintlnMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
          (Value::[<Matrix $input_type:camel>](Matrix::DMatrix(input))) => Ok(Box::new(IoPrintlnMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "VectorD"))]
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

impl<T> MechFunction for IoPrintlnScalar<T> 
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

fn impl_print_fxn(source_value: Value) -> MResult<Box<dyn MechFunction>>  {
  if source_value.is_scalar() {
    match source_value {
      #[cfg(feature = "I8")]
      Value::I8(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "I16")]
      Value::I16(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "I32")]
      Value::I32(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "I64")]
      Value::I64(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "I128")]
      Value::I128(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "U8")]
      Value::U8(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "U16")]
      Value::U16(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "U32")]
      Value::U32(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "U64")]
      Value::U64(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "U128")]
      Value::U128(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "F32")]
      Value::F32(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "F64")]
      Value::F64(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "String")]
      Value::String(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "Bool")]
      Value::Bool(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "ComplexNumber")]
      Value::ComplexNumber(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      #[cfg(feature = "RationalNumber")]
      Value::RationalNumber(value) => { return Ok(Box::new(IoPrintlnScalar { e0: value })); }
      _ => todo!(),
    }
  }
  impl_print_match_arms!(
    (source_value),
    i8,   "I8",
    i16,  "I16",
    i32,  "I32",
    i64,  "I64",
    i128, "I128",
    u8,   "U8",
    u16,  "U16",
    u32,  "U32",
    u64,  "U64",
    u128, "U128",
    F32,  "F32",
    F64,  "F64",
    bool, "Bool",
    String, "String",
    ComplexNumber, "ComplexNumber",
    RationalNumber, "RationalNumber"
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