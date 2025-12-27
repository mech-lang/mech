use crate::*;
use mech_core::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Greater Than ---------------------------------------------------------------

macro_rules! concat_scalar_lhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        //(&mut (*$out))[i] = (&(*$lhs))[i] > (*$rhs);
      }}};}

macro_rules! concat_scalar_rhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$rhs).len() {
        //(&mut (*$out))[i] = (*$lhs) > (&(*$rhs))[i];
      }}};}

macro_rules! concat_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        //(&mut (*$out))[i] = (&(*$lhs))[i] > (&(*$rhs))[i];
      }}};}

macro_rules! concat_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      //(*$out) = (*$lhs) > (*$rhs);
    }};}

macro_rules! concat_mat_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut col, lhs_col) in out_deref.column_iter_mut().zip(lhs_deref.column_iter()) {
        for i in 0..col.len() {
          //col[i] = lhs_col[i] > rhs_deref[i];
        }
      }
    }
  };}   
      
macro_rules! concat_vec_mat_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        let mut out_deref = &mut (*$out);
        let lhs_deref = &(*$lhs);
        let rhs_deref = &(*$rhs);
        for (mut col, rhs_col) in out_deref.column_iter_mut().zip(rhs_deref.column_iter()) {
          for i in 0..col.len() {
            //col[i] = lhs_deref[i] > rhs_col[i];
          }
        }
      }
  };}
  
macro_rules! concat_mat_row_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut row, lhs_row) in out_deref.row_iter_mut().zip(lhs_deref.row_iter()) {
          for i in 0..row.len() {
          //row[i] = lhs_row[i] > rhs_deref[i];
          }
      }
      }
  };}

macro_rules! concat_row_mat_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut row, rhs_row) in out_deref.row_iter_mut().zip(rhs_deref.row_iter()) {
          for i in 0..row.len() {
          //row[i] = lhs_deref[i] > rhs_row[i];
          }
      }
      }
  };}    


#[derive(Debug)]
struct ConcatSS {
  lhs: Ref<String>,
  rhs: Ref<String>,
  out: Ref<String>,
}
impl MechFunctionFactory for ConcatSS {
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Binary(out, arg1, arg2) => {
        let lhs: Ref<String> = unsafe { arg1.as_unchecked() }.clone();
        let rhs: Ref<String> = unsafe { arg2.as_unchecked() }.clone();
        let out: Ref<String> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(Self {lhs, rhs, out }))
      },
      _ => Err(MechError2::new(
          IncorrectNumberOfArguments { expected: 2, found: args.len() }, 
          None
        ).with_compiler_loc()
      ),
    }
  }
}
impl MechFunctionImpl for ConcatSS {
  fn solve(&self) {
    let lhs_ptr = self.lhs.as_ptr();
    let rhs_ptr = self.rhs.as_ptr();
    let out_ptr = self.out.as_mut_ptr();
    unsafe {
      (*out_ptr) = (*lhs_ptr).clone() + &(*rhs_ptr);
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
impl MechFunctionCompiler for ConcatSS {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("{}<{}>", stringify!(ConcatSS), String::as_value_kind());
    compile_binop!(name, self.out, self.lhs, self.rhs, ctx, FeatureFlag::Builtin(FeatureKind::Concat));
  }
}

//impl_string_fxns!(Concat);

fn impl_concat_fxn(lhs_value: Value, rhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  match (lhs_value, rhs_value) {
    // Scalar Scalar
    #[cfg(all(feature = "string"))]
    (Value::String(lhs), Value::String(rhs)) => {
      //$registrar!([<$lib SS>], $target_type, $value_string);
      Ok(Box::new(ConcatSS{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(String::default()) }))
    },
    _ => todo!(),
  }
}

//impl_mech_binop_fxn!(StringConcat,impl_concat_fxn,"string/concat");  


pub struct StringConcat {}
impl NativeFunctionCompiler for StringConcat {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 2, found: arguments.len() }, None).with_compiler_loc());
    }
    let lhs_value = arguments[0].clone();
    let rhs_value = arguments[1].clone();
    match impl_concat_fxn(lhs_value.clone(), rhs_value.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (lhs_value,rhs_value) {
          (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {impl_concat_fxn(lhs.borrow().clone(), rhs.borrow().clone())}
          (lhs_value,Value::MutableReference(rhs)) => { impl_concat_fxn(lhs_value.clone(), rhs.borrow().clone())}
          (Value::MutableReference(lhs),rhs_value) => { impl_concat_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          (lhs, rhs) => Err(MechError2::new(
              UnhandledFunctionArgumentKind2 { arg: (lhs.kind(), rhs.kind()), fxn_name: stringify!(StringConcat).to_string() },
              None
            ).with_compiler_loc()
          ),            
        }
      }
    }
  }
}
#[cfg(not(target_arch = "wasm32"))]
inventory::submit! {
  FunctionCompilerDescriptor {
    name: "string/concat",
    ptr: &StringConcat{},
  }
}
  