use crate::*;
use mech_core::*;


#[derive(Debug)]
struct IoPrintMatrix<T> {
  e0: Ref<Matrix2<T>>,
}
impl<T> MechFunction for IoPrintMatrix<T>
where
  T: Clone + Sync + Send + 'static + std::fmt::Display + std::fmt::Debug
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      for i in 0..e0_ptr.len() {
        print!("{} ", e0_ptr[i]);
      }  
    }
  }
  fn out(&self) -> Value { Value::Empty }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

macro_rules! impl_print_match_arms {
  ($arg:expr, $($input_type:ident),+) => {
    paste!{
      match $arg {
        $(
          #[cfg(all(feature = "$input_type", feature = "Matrix1"))]
          (Value::[<Matrix $input_type:camel>](Matrix::Matrix1(input))) => Ok(Box::new(IoPrintMatrix::<$input_type>{e0: input.clone()})),
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_print_fxn(source_value: Value) -> MResult<Box<dyn MechFunction>>  {
  impl_print_match_arms!(
    (source_value),
    i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, f32, f64, String,
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