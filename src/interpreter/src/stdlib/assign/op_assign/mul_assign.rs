#[macro_use]
use crate::stdlib::*;
use super::*;

// Mul Assign -----------------------------------------------------------------

#[macro_export]
macro_rules! impl_mul_assign_match_arms {
  ($fxn_name:ident,$macro_name:ident, $arg:expr) => {
    paste!{
      [<impl_set_ $macro_name _match_arms>]!(
        $fxn_name,
        $arg,
        U8, "U8";
        U16, "U16";
        U32, "U32";
        U64, "U64";
        U128, "U128";
        I8, "I8";
        I16, "I16";
        I32, "I32";
        I64, "I64";
        U128, "U128";
        F32, "F32"; 
        F64, "F64" ;
        ComplexNumber, "ComplexNumber";
        RationalNumber, "RationalNumber";
      )
    }
  }
}

macro_rules! impl_mul_assign_range_fxn_s {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    impl_op_assign_range_fxn_s!($struct_name, $op, $ix);
  }
}

macro_rules! impl_mul_assign_range_fxn_v {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    impl_op_assign_range_fxn_v!($struct_name, $op, $ix);
  }
}

// x = 1 ----------------------------------------------------------------------

#[derive(Debug)]
struct MulAssignSS<T> {
  sink: Ref<T>,
  source: Ref<T>,
}
impl<T> MechFunction for MulAssignSS<T> 
where
  T: Debug + Clone + Sync + Send + 'static +
  Mul<Output = T> + MulAssign +
  PartialEq + PartialOrd,
  Ref<T>: ToValue
{
  fn solve(&self) {
    let sink_ptr = self.sink.as_ptr();
    let source_ptr = self.source.as_ptr();
    unsafe {
      *sink_ptr *= (*source_ptr).clone();
    }
  }
  fn out(&self) -> Value { self.sink.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[derive(Debug)]
pub struct MulAssignVV<T, MatA, MatB> {
  pub sink: Ref<MatA>,
  pub source: Ref<MatB>,
  _marker: PhantomData<T>,
}

impl<T, MatA, MatB> MechFunction for MulAssignVV<T, MatA, MatB>
where
  Ref<MatA>: ToValue,
  T: Debug + Clone + Sync + Send + 'static + MulAssign,
  for<'a> &'a MatA: IntoIterator<Item = &'a T>,
  for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
  for<'a> &'a MatB: IntoIterator<Item = &'a T>,
  MatA: Debug,
  MatB: Debug,
{
  fn solve(&self) {
    unsafe {
      let sink_ptr = self.sink.as_ptr();
      let source_ptr = self.source.as_ptr();
      let sink_ref: &mut MatA = &mut *sink_ptr;
      let source_ref: &MatB = &*source_ptr;
      for (dst, src) in (&mut *sink_ref).into_iter().zip((&*source_ref).into_iter()) {
        *dst *= src.clone();
      }
    }
  }
  fn out(&self) -> Value {self.sink.to_value()}
  fn to_string(&self) -> String {format!("{:#?}", self)}
}

fn mul_assign_value_fxn(sink: Value, source: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_op_assign_value_match_arms!(
    Mul,
    (sink, source),
    U8,  "U8";
    U16, "U16";
    U32, "U32";
    U64, "U64";
    U128, "U128";
    I8,  "I8";
    I16, "I16";
    I32, "I32";
    I64, "I64";
    U128, "U128";
    F32, "F32";
    F64, "F64";
    RationalNumber, "RationalNumber";
    ComplexNumber, "ComplexNumber";
  )
}

pub struct MulAssignValue {}
impl NativeFunctionCompiler for MulAssignValue {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink = arguments[0].clone();
    let source = arguments[1].clone();
    match mul_assign_value_fxn(sink.clone(),source.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (sink,source) {
          (Value::MutableReference(sink),Value::MutableReference(source)) => { mul_assign_value_fxn(sink.borrow().clone(),source.borrow().clone()) },
          (sink,Value::MutableReference(source)) => { mul_assign_value_fxn(sink.clone(),source.borrow().clone()) },
          (Value::MutableReference(sink),source) => { mul_assign_value_fxn(sink.borrow().clone(),source.clone()) },
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}


// x[1..3] *= 1 ----------------------------------------------------------------

macro_rules! mul_assign_1d_range {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        ($sink)[($ix)[i] - 1] *= ($source).clone();
      }
    }
  };}

macro_rules! mul_assign_1d_range_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        if $ix[i] == true {
          ($sink)[i] *= ($source).clone();
        }
      }
    }
  };}  

macro_rules! mul_assign_1d_range_vec {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        ($sink)[($ix)[i] - 1] *= ($source)[i].clone();
      }
    }
  };}

macro_rules! mul_assign_1d_range_vec_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        if $ix[i] == true {
          ($sink)[i] *= ($source)[i].clone();
        }
      }
    }
  };}  


impl_mul_assign_range_fxn_s!(MulAssign1DRS, mul_assign_1d_range,usize);
impl_mul_assign_range_fxn_s!(MulAssign1DRB, mul_assign_1d_range_b,bool);
impl_mul_assign_range_fxn_v!(MulAssign1DRV, mul_assign_1d_range_vec,usize);
impl_mul_assign_range_fxn_v!(MulAssign1DRVB,mul_assign_1d_range_vec_b,bool);

fn mul_assign_range_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_mul_assign_match_arms!(MulAssign1DR, range, (sink, ixes.as_slice(), source))
}

pub struct MulAssignRange {}
impl NativeFunctionCompiler for MulAssignRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match mul_assign_range_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match (sink,ixes,source) {
          (Value::MutableReference(sink),ixes,Value::MutableReference(source)) => { mul_assign_range_fxn(sink.borrow().clone(),source.borrow().clone(),ixes.clone()) },
          (sink,ixes,Value::MutableReference(source)) => { mul_assign_range_fxn(sink.clone(),source.borrow().clone(),ixes.clone()) },
          (Value::MutableReference(sink),ixes,source) => { mul_assign_range_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) },
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3,:] *= 1 ------------------------------------------------------------------

macro_rules! mul_assign_2d_vector_all {
  ($source:expr, $ix:expr, $sink:expr) => {
      for cix in 0..($sink).ncols() {
        for rix in $ix.iter() {
          ($sink).column_mut(cix)[rix - 1] *= ($source).clone();
        }
      }
    };}

macro_rules! mul_assign_2d_vector_all_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    for cix in 0..($sink).ncols() {
      for rix in 0..$ix.len() {
        if $ix[rix] == true {
          ($sink).column_mut(cix)[rix] *= ($source).clone();
        }
      }
    }
  };} 

macro_rules! mul_assign_2d_vector_all_mat {
  ($source:expr, $ix:expr, $sink:expr) => {
    for (i,rix) in (&$ix).iter().enumerate() {
      let mut sink_row = ($sink).row_mut(rix - 1);
      let src_row = ($source).row(i);
      for (dst, src) in sink_row.iter_mut().zip(src_row.iter()) {
        *dst *= *src;
      }
    }
  };}

macro_rules! mul_assign_2d_vector_all_mat_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    for (i,rix) in (&$ix).iter().enumerate() {
      if *rix == true {
        let mut sink_row = ($sink).row_mut(i);
        let src_row = ($source).row(i);
        for (dst, src) in sink_row.iter_mut().zip(src_row.iter()) {
          *dst *= *src;
        }
      }
    }
  };} 

impl_mul_assign_range_fxn_s!(MulAssign2DRAS, mul_assign_2d_vector_all,usize);
impl_mul_assign_range_fxn_s!(MulAssign2DRASB,mul_assign_2d_vector_all_b,bool);
impl_mul_assign_range_fxn_v!(MulAssign2DRAV, mul_assign_2d_vector_all_mat,usize);
impl_mul_assign_range_fxn_v!(MulAssign2DRAVB,mul_assign_2d_vector_all_mat_b,bool);

fn mul_assign_vec_all_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_mul_assign_match_arms!(MulAssign2DRA, range_all, (sink, ixes.as_slice(), source))
}

pub struct MulAssignRangeAll {}
impl NativeFunctionCompiler for MulAssignRangeAll {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match mul_assign_vec_all_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (sink,ixes,source) {
          (Value::MutableReference(sink),ixes,Value::MutableReference(source)) => { mul_assign_vec_all_fxn(sink.borrow().clone(),source.borrow().clone(),ixes.clone()) },
          (sink,ixes,Value::MutableReference(source)) => { mul_assign_vec_all_fxn(sink.clone(),source.borrow().clone(),ixes.clone()) },
          (Value::MutableReference(sink),ixes,source) => { mul_assign_vec_all_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) },
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}