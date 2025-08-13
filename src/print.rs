use crate::*;

#[derive(Debug)]
struct IoPrintMatrix<T,Mat> {
  e0: Ref<Mat>,
  _marker: PhantomData<T>,
}
impl<T,Mat> MechFunction for IoPrintMatrix<T,Mat>
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
        print!("{} ", el);
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
          (Value::[<Matrix $input_type:camel>](Matrix::Matrix1(input))) => Ok(Box::new(IoPrintMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "Matrix2"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Matrix2(input))) => Ok(Box::new(IoPrintMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "Matrix3"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Matrix3(input))) => Ok(Box::new(IoPrintMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "Matrix4"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Matrix4(input))) => Ok(Box::new(IoPrintMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "Vector2"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Vector2(input))) => Ok(Box::new(IoPrintMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "Vector3"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Vector3(input))) => Ok(Box::new(IoPrintMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "Vector4"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Vector4(input))) => Ok(Box::new(IoPrintMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "RowVector2"))]
          (Value::[<Matrix $input_type:camel>](Matrix::RowVector2(input))) => Ok(Box::new(IoPrintMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "RowVector3"))]
          (Value::[<Matrix $input_type:camel>](Matrix::RowVector3(input))) => Ok(Box::new(IoPrintMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "RowVector4"))]
          (Value::[<Matrix $input_type:camel>](Matrix::RowVector4(input))) => Ok(Box::new(IoPrintMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "RowVectorD"))]
          (Value::[<Matrix $input_type:camel>](Matrix::RowDVector(input))) => Ok(Box::new(IoPrintMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
          (Value::[<Matrix $input_type:camel>](Matrix::DMatrix(input))) => Ok(Box::new(IoPrintMatrix{e0: input.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $value_string, feature = "VectorV"))]
          (Value::[<Matrix $input_type:camel>](Matrix::DVector(input))) => Ok(Box::new(IoPrintMatrix{e0: input.clone(), _marker: PhantomData::default()})),
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

#[derive(Debug)]
struct IoPrintScalar<T> {
  e0: Ref<T>,
}

impl<T> MechFunction for IoPrintScalar<T> 
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
      Value::I8(value) => { return Ok(Box::new(IoPrintScalar { e0: value })); }
      Value::I16(value) => { return Ok(Box::new(IoPrintScalar { e0: value })); }
      Value::I32(value) => { return Ok(Box::new(IoPrintScalar { e0: value })); }
      Value::I64(value) => { return Ok(Box::new(IoPrintScalar { e0: value })); }
      Value::I128(value) => { return Ok(Box::new(IoPrintScalar { e0: value })); }
      Value::U8(value) => { return Ok(Box::new(IoPrintScalar { e0: value })); }
      Value::U16(value) => { return Ok(Box::new(IoPrintScalar { e0: value })); }
      Value::U32(value) => { return Ok(Box::new(IoPrintScalar { e0: value })); }
      Value::U64(value) => { return Ok(Box::new(IoPrintScalar { e0: value })); }
      Value::U128(value) => { return Ok(Box::new(IoPrintScalar { e0: value })); }
      Value::F32(value) => { return Ok(Box::new(IoPrintScalar { e0: value })); }
      Value::F64(value) => { return Ok(Box::new(IoPrintScalar { e0: value })); }
      Value::String(value) => { return Ok(Box::new(IoPrintScalar { e0: value })); }
      Value::Bool(value) => { return Ok(Box::new(IoPrintScalar { e0: value })); }
      Value::ComplexNumber(value) => { return Ok(Box::new(IoPrintScalar { e0: value })); }
      Value::RationalNumber(value) => { return Ok(Box::new(IoPrintScalar { e0: value })); }
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

pub struct IoPrint {}

impl NativeFunctionCompiler for IoPrint {
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