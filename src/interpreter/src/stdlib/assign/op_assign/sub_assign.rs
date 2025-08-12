#[macro_use]
use crate::stdlib::*;
use super::*;

// Sub Assign -----------------------------------------------------------------

#[macro_export]
macro_rules! impl_sub_assign_match_arms {
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

macro_rules! impl_sub_assign_range_fxn_s {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    impl_op_assign_range_fxn_s!($struct_name, $op, $ix);
  }
}

macro_rules! impl_sub_assign_range_fxn_v {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    impl_op_assign_range_fxn_v!($struct_name, $op, $ix);
  }
}

// x = 1 ----------------------------------------------------------------------

impl_assign_scalar_scalar!(Sub, -=);
impl_assign_vector_vector!(Sub, -=);

fn sub_assign_value_fxn(sink: Value, source: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_op_assign_value_match_arms!(
    Sub,
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

pub struct SubAssignValue {}
impl NativeFunctionCompiler for SubAssignValue {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink = arguments[0].clone();
    let source = arguments[1].clone();
    match sub_assign_value_fxn(sink.clone(),source.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (sink,source) {
          (Value::MutableReference(sink),Value::MutableReference(source)) => { sub_assign_value_fxn(sink.borrow().clone(),source.borrow().clone()) },
          (sink,Value::MutableReference(source)) => { sub_assign_value_fxn(sink.clone(),source.borrow().clone()) },
          (Value::MutableReference(sink),source) => { sub_assign_value_fxn(sink.borrow().clone(),source.clone()) },
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3] -= 1 ----------------------------------------------------------------

macro_rules! sub_assign_1d_range {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        ($sink)[($ix)[i] - 1] -= ($source).clone();
      }
    }
  };}

macro_rules! sub_assign_1d_range_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        if $ix[i] == true {
          ($sink)[i] -= ($source).clone();
        }
      }
    }
  };}  

macro_rules! sub_assign_1d_range_vec {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        ($sink)[($ix)[i] - 1] -= ($source)[i].clone();
      }
    }
  };}

macro_rules! sub_assign_1d_range_vec_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        if $ix[i] == true {
          ($sink)[i] -= ($source)[i].clone();
        }
      }
    }
  };}  


impl_sub_assign_range_fxn_s!(SubAssign1DRS, sub_assign_1d_range,usize);
impl_sub_assign_range_fxn_s!(SubAssign1DRB, sub_assign_1d_range_b,bool);
impl_sub_assign_range_fxn_v!(SubAssign1DRV, sub_assign_1d_range_vec,usize);
impl_sub_assign_range_fxn_v!(SubAssign1DRVB,sub_assign_1d_range_vec_b,bool);

fn sub_assign_range_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_sub_assign_match_arms!(SubAssign1DR, range, (sink, ixes.as_slice(), source))
}

pub struct SubAssignRange {}
impl NativeFunctionCompiler for SubAssignRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match sub_assign_range_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match (sink,ixes,source) {
          (Value::MutableReference(sink),ixes,Value::MutableReference(source)) => { sub_assign_range_fxn(sink.borrow().clone(),source.borrow().clone(),ixes.clone()) },
          (sink,ixes,Value::MutableReference(source)) => { sub_assign_range_fxn(sink.clone(),source.borrow().clone(),ixes.clone()) },
          (Value::MutableReference(sink),ixes,source) => { sub_assign_range_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) },
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3,:] -= 1 ------------------------------------------------------------------

macro_rules! sub_assign_2d_vector_all {
  ($source:expr, $ix:expr, $sink:expr) => {
      for cix in 0..($sink).ncols() {
        for rix in $ix.iter() {
          ($sink).column_mut(cix)[rix - 1] -= ($source).clone();
        }
      }
    };}

macro_rules! sub_assign_2d_vector_all_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    for cix in 0..($sink).ncols() {
      for rix in 0..$ix.len() {
        if $ix[rix] == true {
          ($sink).column_mut(cix)[rix] -= ($source).clone();
        }
      }
    }
  };} 

macro_rules! sub_assign_2d_vector_all_mat {
  ($source:expr, $ix:expr, $sink:expr) => {
    for (i,rix) in (&$ix).iter().enumerate() {
      let mut sink_row = ($sink).row_mut(rix - 1);
      let src_row = ($source).row(i);
      for (dst, src) in sink_row.iter_mut().zip(src_row.iter()) {
        *dst -= *src;
      }
    }
  };}

macro_rules! sub_assign_2d_vector_all_mat_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    for (i,rix) in (&$ix).iter().enumerate() {
      if *rix == true {
        let mut sink_row = ($sink).row_mut(i);
        let src_row = ($source).row(i);
        for (dst, src) in sink_row.iter_mut().zip(src_row.iter()) {
          *dst -= *src;
        }
      }
    }
  };} 

impl_sub_assign_range_fxn_s!(SubAssign2DRAS, sub_assign_2d_vector_all,usize);
impl_sub_assign_range_fxn_s!(SubAssign2DRASB,sub_assign_2d_vector_all_b,bool);
impl_sub_assign_range_fxn_v!(SubAssign2DRAV, sub_assign_2d_vector_all_mat,usize);
impl_sub_assign_range_fxn_v!(SubAssign2DRAVB,sub_assign_2d_vector_all_mat_b,bool);

fn sub_assign_vec_all_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_sub_assign_match_arms!(SubAssign2DRA, range_all, (sink, ixes.as_slice(), source))
}

pub struct SubAssignRangeAll {}
impl NativeFunctionCompiler for SubAssignRangeAll {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match sub_assign_vec_all_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (sink,ixes,source) {
          (Value::MutableReference(sink),ixes,Value::MutableReference(source)) => { sub_assign_vec_all_fxn(sink.borrow().clone(),source.borrow().clone(),ixes.clone()) },
          (sink,ixes,Value::MutableReference(source)) => { sub_assign_vec_all_fxn(sink.clone(),source.borrow().clone(),ixes.clone()) },
          (Value::MutableReference(sink),ixes,source) => { sub_assign_vec_all_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) },
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}