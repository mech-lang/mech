#[macro_use]
use crate::*;
use super::*;
use num_traits::*;

// Mul Assign -----------------------------------------------------------------

#[macro_export]
macro_rules! impl_mul_assign_match_arms {
  ($fxn_name:ident,$macro_name:ident, $arg:expr) => {
    paste!{
      [<impl_set_ $macro_name _match_arms>]!(
        $fxn_name,
        $arg,
        U8, "u8";
        U16, "u16";
        U32, "u32";
        U64, "u64";
        U128, "u128";
        I8, "i8";
        I16, "i16";
        I32, "i32";
        I64, "i64";
        U128, "u128";
        F32, "f32"; 
        F64, "f64" ;
        ComplexNumber, "complex";
        RationalNumber, "rational";
      )
    }
  }
}

#[cfg(feature = "matrix")]
macro_rules! impl_mul_assign_range_fxn_s {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    impl_op_assign_range_fxn_s!($struct_name, $op, $ix);
  }
}

#[cfg(feature = "matrix")]
macro_rules! impl_mul_assign_range_fxn_v {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    impl_op_assign_range_fxn_v!($struct_name, $op, $ix);
  }
}

// x = 1 ----------------------------------------------------------------------

impl_assign_scalar_scalar!(Mul, *=);
impl_assign_vector_vector!(Mul, *=);

fn mul_assign_value_fxn(sink: Value, source: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_op_assign_value_match_arms!(
    Mul,
    (sink, source),
    U8,  "u8";
    U16, "u16";
    U32, "u32";
    U64, "u64";
    U128, "u128";
    I8,  "i8";
    I16, "i16";
    I32, "i32";
    I64, "i64";
    U128, "u128";
    F32, "f32";
    F64, "f64";
    RationalNumber, "rational";
    ComplexNumber, "complex";
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


#[cfg(feature = "matrix")]
impl_mul_assign_range_fxn_s!(MulAssign1DRS, mul_assign_1d_range,usize);
#[cfg(feature = "matrix")]
impl_mul_assign_range_fxn_s!(MulAssign1DRB, mul_assign_1d_range_b,bool);
#[cfg(feature = "matrix")]
impl_mul_assign_range_fxn_v!(MulAssign1DRV, mul_assign_1d_range_vec,usize);
#[cfg(feature = "matrix")]
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

#[cfg(feature = "matrix")]
impl_mul_assign_range_fxn_s!(MulAssign2DRAS, mul_assign_2d_vector_all,usize);
#[cfg(feature = "matrix")]
impl_mul_assign_range_fxn_s!(MulAssign2DRASB,mul_assign_2d_vector_all_b,bool);
#[cfg(feature = "matrix")]
impl_mul_assign_range_fxn_v!(MulAssign2DRAV, mul_assign_2d_vector_all_mat,usize);
#[cfg(feature = "matrix")]
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