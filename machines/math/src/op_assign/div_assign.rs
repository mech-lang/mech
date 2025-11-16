#[macro_use]
use crate::*;
use super::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use std::ops::DivAssign;

// Div Assign -----------------------------------------------------------------

// We will mostly use the assign macros for this

#[macro_export]
macro_rules! impl_div_assign_match_arms {
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
        C64, "complex";
        R64, "rational";
      )
    }
  }
}

#[cfg(feature = "matrix")]
macro_rules! impl_div_assign_range_fxn_s {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    impl_op_assign_range_fxn_s!($struct_name, $op, $ix);
  }
}

#[cfg(feature = "matrix")]
macro_rules! impl_div_assign_range_fxn_v {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    impl_op_assign_range_fxn_v!($struct_name, $op, $ix);
  }
}

// x /= 1 ----------------------------------------------------------------------

impl_assign_scalar_scalar!(Div, /=);
impl_assign_vector_vector!(Div, /=);
impl_assign_vector_scalar!(Div, /=);
fn div_assign_value_fxn(sink: Value, source: Value) -> MResult<Box<dyn MechFunction>> {
  impl_op_assign_value_match_arms!(
    Div,
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
    R64, "rational";
    C64, "complex";
  )
}

pub struct DivAssignValue {}
impl NativeFunctionCompiler for DivAssignValue {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() },None).with_compiler_loc());
    }
    let sink = arguments[0].clone();
    let source = arguments[1].clone();
    match div_assign_value_fxn(sink.clone(),source.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match (sink,source) {
          (Value::MutableReference(sink),Value::MutableReference(source)) => { div_assign_value_fxn(sink.borrow().clone(),source.borrow().clone()) },
          (sink,Value::MutableReference(source)) => { div_assign_value_fxn(sink.clone(),source.borrow().clone()) },
          (Value::MutableReference(sink),source) => { div_assign_value_fxn(sink.borrow().clone(),source.clone()) },
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind2 { arg: x.clone(), fxn_name: "math/div-assign".to_string() },
              None
            ).with_compiler_loc()
          ),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "math/div-assign",
    ptr: &DivAssignValue{},
  }
}

// x[1..3] /= 1 ----------------------------------------------------------------

macro_rules! div_assign_1d_range {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        ($sink)[($ix)[i] - 1] /= *($source);
      }
    }
  };}

macro_rules! div_assign_1d_range_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        if $ix[i] == true {
          ($sink)[i] /= *($source);
        }
      }
    }
  };}  

macro_rules! div_assign_1d_range_vec {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        ($sink)[($ix)[i] - 1] /= ($source)[i];
      }
    }
  };}

macro_rules! div_assign_1d_range_vec_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        if $ix[i] == true {
          ($sink)[i] /= ($source)[i];
        }
      }
    }
  };}

#[cfg(feature = "matrix")]
impl_div_assign_range_fxn_s!(DivAssign1DRS,div_assign_1d_range,usize);
#[cfg(feature = "matrix")]
impl_div_assign_range_fxn_s!(DivAssign1DRB,div_assign_1d_range_b,bool);
#[cfg(feature = "matrix")]
impl_div_assign_range_fxn_v!(DivAssign1DRV,div_assign_1d_range_vec,usize);
#[cfg(feature = "matrix")]
impl_div_assign_range_fxn_v!(DivAssign1DRVB,div_assign_1d_range_vec_b,bool);

op_assign_range_fxn!(div_assign_range_fxn, DivAssign1DR);

pub struct DivAssignRange {}
impl NativeFunctionCompiler for DivAssignRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() },None).with_compiler_loc());
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match div_assign_range_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match (&sink, &ixes, &source) {
          (Value::MutableReference(sink),ixes,Value::MutableReference(source)) => { div_assign_range_fxn(sink.borrow().clone(),source.borrow().clone(),ixes.clone()) },
          (sink,ixes,Value::MutableReference(source)) => { div_assign_range_fxn(sink.clone(),source.borrow().clone(),ixes.clone()) },
          (Value::MutableReference(sink),ixes,source) => { div_assign_range_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) },
          x => Err(MechError2::new(
              UnhandledFunctionArgumentIxes { arg: (sink.clone(), ixes.to_vec(), source.clone()), fxn_name: "math/div-assign/range".to_string() },
              None
            ).with_compiler_loc()
          ),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "math/div-assign/range",
    ptr: &DivAssignRange{},
  }
}

// x[1..3,:] /= 1 ------------------------------------------------------------------


macro_rules! div_assign_2d_vector_all {
  ($source:expr, $ix:expr, $sink:expr) => {
    for val in ($sink).iter_mut() {
      *val /= (*$source);
    }
  };}

macro_rules! div_assign_2d_vector_all_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    let ncols = ($sink).ncols();
    for (i, val) in ($sink).iter_mut().enumerate() {
      let row = i / ncols;
      if $ix[row] {
        *val /= *($source);
      }
    }
  };}

macro_rules! div_assign_2d_vector_all_mat {
  ($source:expr, $ix:expr, $sink:expr) => {
    {
      let nsrc = $source.nrows();
      for (i, &rix) in $ix.iter().enumerate() {
        let row_index = rix - 1;
        let mut sink_row = $sink.row_mut(row_index);
        let src_row = $source.row(i % nsrc); // wrap around!
        for (dst, src) in sink_row.iter_mut().zip(src_row.iter()) {
          *dst /= *src;
        }
      }
    }
  };}

macro_rules! div_assign_2d_vector_all_mat_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    {
      let mut src_i = 0;
      for (i, rix) in (&$ix).iter().enumerate() {
        if *rix == true {
          let mut sink_row = ($sink).row_mut(i);
          let src_row = ($source).row(src_i);
          for (dst, src) in sink_row.iter_mut().zip(src_row.iter()) {
            *dst /= *src;
          }
          src_i += 1;
        }
      }
    }
  };}

#[cfg(feature = "matrix")]
impl_div_assign_range_fxn_s!(DivAssign2DRAS,div_assign_2d_vector_all,usize);
#[cfg(feature = "matrix")]
impl_div_assign_range_fxn_s!(DivAssign2DRASB,div_assign_2d_vector_all_b,bool);
#[cfg(feature = "matrix")]
impl_div_assign_range_fxn_v!(DivAssign2DRAV,div_assign_2d_vector_all_mat,usize);
#[cfg(feature = "matrix")]
impl_div_assign_range_fxn_v!(DivAssign2DRAVB,div_assign_2d_vector_all_mat_b,bool);

op_assign_range_all_fxn!(div_assign_range_all_fxn, DivAssign2DRA);

pub struct DivAssignRangeAll {}
impl NativeFunctionCompiler for DivAssignRangeAll {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() },None).with_compiler_loc());
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match div_assign_range_all_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (&sink,&ixes,&source) {
          (Value::MutableReference(sink),ixes,Value::MutableReference(source)) => { div_assign_range_all_fxn(sink.borrow().clone(),source.borrow().clone(),ixes.clone()) },
          (sink,ixes,Value::MutableReference(source)) => { div_assign_range_all_fxn(sink.clone(),source.borrow().clone(),ixes.clone()) },
          (Value::MutableReference(sink),ixes,source) => { div_assign_range_all_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) },
          _ => Err(MechError2::new(
              UnhandledFunctionArgumentIxes { arg: (sink.clone(), ixes.to_vec(), source.clone()), fxn_name: "math/div-assign/range-all".to_string() },
              None
            ).with_compiler_loc()
          ),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "math/div-assign/range-all",
    ptr: &DivAssignRangeAll{},
  }
}