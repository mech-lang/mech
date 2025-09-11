#[macro_use]
use crate::*;
use super::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Add Assign -----------------------------------------------------------------

// We will mostly use the assign macros for this

#[macro_export]
macro_rules! impl_add_assign_match_arms {
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
macro_rules! impl_add_assign_range_fxn_s {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    impl_op_assign_range_fxn_s!($struct_name, $op, $ix);
  }
}

#[cfg(feature = "matrix")]
macro_rules! impl_add_assign_range_fxn_v {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    impl_op_assign_range_fxn_v!($struct_name, $op, $ix);
  }
}

// x += 1 ----------------------------------------------------------------------

impl_assign_scalar_scalar!(Add, +=);
impl_assign_vector_vector!(Add, +=);

pub fn add_assign_math_fxn(sink: Value, source: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_op_assign_value_match_arms!(
    Add,
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

pub struct AddAssignMath {}
impl NativeFunctionCompiler for AddAssignMath {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink = arguments[0].clone();
    let source = arguments[1].clone();
    match add_assign_math_fxn(sink.clone(),source.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match (sink,source) {
          (Value::MutableReference(sink),Value::MutableReference(source)) => { add_assign_math_fxn(sink.borrow().clone(),source.borrow().clone()) },
          (sink,Value::MutableReference(source)) => { add_assign_math_fxn(sink.clone(),source.borrow().clone()) },
          (Value::MutableReference(sink),source) => { add_assign_math_fxn(sink.borrow().clone(),source.clone()) },
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3] += 1 ----------------------------------------------------------------

macro_rules! add_assign_1d_range {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        ($sink)[($ix)[i] - 1] += ($source).clone();
      }
    }
  };}

macro_rules! add_assign_1d_range_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        if $ix[i] == true {
          ($sink)[i] += ($source).clone();
        }
      }
    }
  };}  

macro_rules! add_assign_1d_range_vec {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        ($sink)[($ix)[i] - 1] += ($source)[i].clone();
      }
    }
  };}

macro_rules! add_assign_1d_range_vec_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        if $ix[i] == true {
          ($sink)[i] += ($source)[i].clone();
        }
      }
    }
  };}

#[cfg(feature = "matrix")]
impl_add_assign_range_fxn_s!(AddAssign1DRS,add_assign_1d_range,usize);
#[cfg(feature = "matrix")]
impl_add_assign_range_fxn_s!(AddAssign1DRB,add_assign_1d_range_b,bool);
#[cfg(feature = "matrix")]
impl_add_assign_range_fxn_v!(AddAssign1DRV,add_assign_1d_range_vec,usize);
#[cfg(feature = "matrix")]
impl_add_assign_range_fxn_v!(AddAssign1DRVB,add_assign_1d_range_vec_b,bool);

fn add_assign_range_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_add_assign_match_arms!(AddAssign1DR, range, (sink, ixes.as_slice(), source))
}

//register_fxn_descriptor!(AddAssign1DRS, u8, u16, u32, u64, u128, i8, i16, i32, i64, F32, F64, R64, C64);
//register_fxn_descriptor!(AddAssign1DRB, u8, u16, u32, u64, u128, i8, i16, i32, i64, F32, F64, R64, C64);
/*inventory::submit! {
  #[cfg(feature = $type_string)]
  FunctionDescriptor {
    name: concat!(stringify!(AddAssign1DRV), "<", stringify!([<$type:lower>]), ">"),
    ptr: $struct_name::<$type>::new,
  }
}*/

inventory::submit! {
  FunctionDescriptor {
    name: concat!(stringify!(AddAssign1DRV), "<f64>"),
    ptr: AddAssign1DRV::<F64,RowVector2<F64>,RowVector2<F64>,RowVector2<usize>>::new,
  }
}
//register_fxn_descriptor!(AddAssign1DRVB, u8, u16, u32, u64, u128, i8, i16, i32, i64, F32, F64, R64, C64);

pub struct AddAssignRange {}
impl NativeFunctionCompiler for AddAssignRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match add_assign_range_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match (sink,ixes,source) {
          (Value::MutableReference(sink),ixes,Value::MutableReference(source)) => { add_assign_range_fxn(sink.borrow().clone(),source.borrow().clone(),ixes.clone()) },
          (sink,ixes,Value::MutableReference(source)) => { add_assign_range_fxn(sink.clone(),source.borrow().clone(),ixes.clone()) },
          (Value::MutableReference(sink),ixes,source) => { add_assign_range_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) },
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3,:] += 1 ------------------------------------------------------------------

macro_rules! add_assign_2d_vector_all {
  ($source:expr, $ix:expr, $sink:expr) => {
      for cix in 0..($sink).ncols() {
        for rix in $ix.iter() {
          ($sink).column_mut(cix)[rix - 1] += ($source).clone();
        }
      }
    };}

macro_rules! add_assign_2d_vector_all_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    for cix in 0..($sink).ncols() {
      for rix in 0..$ix.len() {
        if $ix[rix] == true {
          ($sink).column_mut(cix)[rix] += ($source).clone();
        }
      }
    }
  };} 

macro_rules! add_assign_2d_vector_all_mat {
  ($source:expr, $ix:expr, $sink:expr) => {
    for (i,rix) in (&$ix).iter().enumerate() {
      let mut sink_row = ($sink).row_mut(rix - 1);
      let src_row = ($source).row(i);
      for (dst, src) in sink_row.iter_mut().zip(src_row.iter()) {
        *dst += *src;
      }
    }
  };}

macro_rules! add_assign_2d_vector_all_mat_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    for (i,rix) in (&$ix).iter().enumerate() {
      if *rix == true {
        let mut sink_row = ($sink).row_mut(i);
        let src_row = ($source).row(i);
        for (dst, src) in sink_row.iter_mut().zip(src_row.iter()) {
          *dst += *src;
        }
      }
    }
  };} 

#[cfg(feature = "matrix")]
impl_add_assign_range_fxn_s!(AddAssign2DRAS, add_assign_2d_vector_all,usize);
#[cfg(feature = "matrix")]
impl_add_assign_range_fxn_s!(AddAssign2DRASB,add_assign_2d_vector_all_b,bool);
#[cfg(feature = "matrix")]
impl_add_assign_range_fxn_v!(AddAssign2DRAV, add_assign_2d_vector_all_mat,usize);
#[cfg(feature = "matrix")]
impl_add_assign_range_fxn_v!(AddAssign2DRAVB,add_assign_2d_vector_all_mat_b,bool);

fn add_assign_vec_all_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_add_assign_match_arms!(AddAssign2DRA, range_all, (sink, ixes.as_slice(), source))
}

pub struct AddAssignRangeAll {}
impl NativeFunctionCompiler for AddAssignRangeAll {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match add_assign_vec_all_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (sink,ixes,source) {
          (Value::MutableReference(sink),ixes,Value::MutableReference(source)) => { add_assign_vec_all_fxn(sink.borrow().clone(),source.borrow().clone(),ixes.clone()) },
          (sink,ixes,Value::MutableReference(source)) => { add_assign_vec_all_fxn(sink.clone(),source.borrow().clone(),ixes.clone()) },
          (Value::MutableReference(sink),ixes,source) => { add_assign_vec_all_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) },
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}