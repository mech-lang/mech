#[macro_use]
use crate::stdlib::*;

// ----------------------------------------------------------------------------
// Logic Library
// ----------------------------------------------------------------------------

macro_rules! and_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {*$out = *$lhs && *$rhs;}
    };}

macro_rules! and_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] && (*$rhs)[i];
      }}};}
    
macro_rules! and_scalar_rhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$rhs).len() {
        (*$out)[i] = (*$lhs) && (*$rhs)[i];
      }}};}
      

macro_rules! and_scalar_lhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] && (*$rhs);
      }}};}

macro_rules! or_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {*$out = *$lhs || *$rhs;}
    };}

macro_rules! or_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] || (*$rhs)[i];
      }}};}
    
macro_rules! or_scalar_rhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$rhs).len() {
        (*$out)[i] = (*$lhs) || (*$rhs)[i];
      }}};}
      

macro_rules! or_scalar_lhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] || (*$rhs);
      }}};}

macro_rules! xor_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {*$out = *$lhs ^ *$rhs;}
    };}

macro_rules! xor_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] ^ (*$rhs)[i];
      }}};}
    
macro_rules! xor_scalar_rhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$rhs).len() {
        (*$out)[i] = (*$lhs) ^ (*$rhs)[i];
      }}};}
      

macro_rules! xor_scalar_lhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] ^ (*$rhs);
      }}};}      

macro_rules! not_op {
  ($arg:expr, $out:expr) => {
    unsafe {*$out = !*$arg;}
    };}

macro_rules! not_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        (*$out)[i] = !(*$arg)[i];
      }}};}

#[macro_export]
macro_rules! impl_logic_binop {
  ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
    #[derive(Debug)]
    struct $struct_name {
      lhs: Ref<$arg1_type>,
      rhs: Ref<$arg2_type>,
      out: Ref<$out_type>,
    }
    impl MechFunction for $struct_name {
      fn solve(&self) {
        let lhs_ptr = self.lhs.as_ptr();
        let rhs_ptr = self.rhs.as_ptr();
        let out_ptr = self.out.as_ptr();
        $op!(lhs_ptr,rhs_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }};}

#[macro_export]
macro_rules! impl_logic_urnop {
  ($struct_name:ident, $arg_type:ty, $out_type:ty, $op:ident) => {
    #[derive(Debug)]
    struct $struct_name {
      arg: Ref<$arg_type>,
      out: Ref<$out_type>,
    }
    impl MechFunction for $struct_name {
      fn solve(&self) {
        let arg_ptr = self.arg.as_ptr();
        let out_ptr = self.out.as_ptr();
        $op!(arg_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }};}

// And ------------------------------------------------------------------------

impl_logic_binop!(AndScalar, bool, bool, bool, and_op);
impl_logic_binop!(AndSM2x3, bool, Matrix2x3<bool>, Matrix2x3<bool>,and_scalar_rhs_op);
impl_logic_binop!(AndSM2, bool, Matrix2<bool>, Matrix2<bool>,and_scalar_rhs_op);
impl_logic_binop!(AndSM3, bool, Matrix3<bool>, Matrix3<bool>,and_scalar_rhs_op);
impl_logic_binop!(AndSR2, bool, RowVector2<bool>, RowVector2<bool>,and_scalar_rhs_op);
impl_logic_binop!(AndSR3, bool, RowVector3<bool>, RowVector3<bool>,and_scalar_rhs_op);
impl_logic_binop!(AndSR4, bool, RowVector4<bool>, RowVector4<bool>,and_scalar_rhs_op);
impl_logic_binop!(AndSRD, bool, RowDVector<bool>, RowDVector<bool>,and_scalar_rhs_op);
impl_logic_binop!(AndSVD, bool, DVector<bool>, DVector<bool>,and_scalar_rhs_op);
impl_logic_binop!(AndSMD, bool, DMatrix<bool>, DMatrix<bool>,and_scalar_rhs_op);
impl_logic_binop!(AndM2x3S, Matrix2x3<bool>, bool, Matrix2x3<bool>,and_scalar_lhs_op);
impl_logic_binop!(AndM2S, Matrix2<bool>, bool, Matrix2<bool>,and_scalar_lhs_op);
impl_logic_binop!(AndM3S, Matrix3<bool>, bool, Matrix3<bool>,and_scalar_lhs_op);
impl_logic_binop!(AndR2S, RowVector2<bool>, bool, RowVector2<bool>,and_scalar_lhs_op);
impl_logic_binop!(AndR3S, RowVector3<bool>, bool, RowVector3<bool>,and_scalar_lhs_op);
impl_logic_binop!(AndR4S, RowVector4<bool>, bool, RowVector4<bool>,and_scalar_lhs_op);
impl_logic_binop!(AndRDS, RowDVector<bool>, bool, RowDVector<bool>,and_scalar_lhs_op);
impl_logic_binop!(AndVDS, DVector<bool>, bool, DVector<bool>,and_scalar_lhs_op);
impl_logic_binop!(AndMDS, DMatrix<bool>, bool, DMatrix<bool>,and_scalar_lhs_op);
impl_logic_binop!(AndM2M2, Matrix2<bool>,Matrix2<bool>,Matrix2<bool>, and_vec_op);
impl_logic_binop!(AndM3M3, Matrix3<bool>,Matrix3<bool>,Matrix3<bool>, and_vec_op);
impl_logic_binop!(AndM2x3M2x3, Matrix2x3<bool>,Matrix2x3<bool>,Matrix2x3<bool>, and_vec_op);
impl_logic_binop!(AndR2R2, RowVector2<bool>, RowVector2<bool>, RowVector2<bool>, and_vec_op);
impl_logic_binop!(AndR3R3, RowVector3<bool>, RowVector3<bool>, RowVector3<bool>, and_vec_op);
impl_logic_binop!(AndR4R4, RowVector4<bool>, RowVector4<bool>, RowVector4<bool>, and_vec_op);
impl_logic_binop!(AndRDRD, RowDVector<bool>, RowDVector<bool>, RowDVector<bool>, and_vec_op);
impl_logic_binop!(AndVDVD, DVector<bool>,DVector<bool>,DVector<bool>, and_vec_op);
impl_logic_binop!(AndMDMD, DMatrix<bool>,DMatrix<bool>,DMatrix<bool>, and_vec_op);

fn generate_and_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_binop_match_arms!(
    And,
    (lhs_value, rhs_value),
    Bool, Bool => MatrixBool, bool, false;
  )
}

pub struct LogicAnd {}

impl NativeFunctionCompiler for LogicAnd {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let lhs_value = arguments[0].clone();
    let rhs_value = arguments[1].clone();
    match generate_and_fxn(lhs_value.clone(), rhs_value.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (lhs_value,rhs_value) {
          (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {generate_and_fxn(lhs.borrow().clone(), rhs.borrow().clone())}
          (lhs_value,Value::MutableReference(rhs)) => { generate_and_fxn(lhs_value.clone(), rhs.borrow().clone())}
          (Value::MutableReference(lhs),rhs_value) => { generate_and_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// Or ------------------------------------------------------------------------

impl_logic_binop!(OrScalar, bool, bool, bool, or_op);
impl_logic_binop!(OrSM2x3, bool, Matrix2x3<bool>, Matrix2x3<bool>,or_scalar_rhs_op);
impl_logic_binop!(OrSM2, bool, Matrix2<bool>, Matrix2<bool>,or_scalar_rhs_op);
impl_logic_binop!(OrSM3, bool, Matrix3<bool>, Matrix3<bool>,or_scalar_rhs_op);
impl_logic_binop!(OrSR2, bool, RowVector2<bool>, RowVector2<bool>,or_scalar_rhs_op);
impl_logic_binop!(OrSR3, bool, RowVector3<bool>, RowVector3<bool>,or_scalar_rhs_op);
impl_logic_binop!(OrSR4, bool, RowVector4<bool>, RowVector4<bool>,or_scalar_rhs_op);
impl_logic_binop!(OrSRD, bool, RowDVector<bool>, RowDVector<bool>,or_scalar_rhs_op);
impl_logic_binop!(OrSVD, bool, DVector<bool>, DVector<bool>,or_scalar_rhs_op);
impl_logic_binop!(OrSMD, bool, DMatrix<bool>, DMatrix<bool>,or_scalar_rhs_op);
impl_logic_binop!(OrM2x3S, Matrix2x3<bool>, bool, Matrix2x3<bool>,or_scalar_lhs_op);
impl_logic_binop!(OrM2S, Matrix2<bool>, bool, Matrix2<bool>,or_scalar_lhs_op);
impl_logic_binop!(OrM3S, Matrix3<bool>, bool, Matrix3<bool>,or_scalar_lhs_op);
impl_logic_binop!(OrR2S, RowVector2<bool>, bool, RowVector2<bool>,or_scalar_lhs_op);
impl_logic_binop!(OrR3S, RowVector3<bool>, bool, RowVector3<bool>,or_scalar_lhs_op);
impl_logic_binop!(OrR4S, RowVector4<bool>, bool, RowVector4<bool>,or_scalar_lhs_op);
impl_logic_binop!(OrRDS, RowDVector<bool>, bool, RowDVector<bool>,or_scalar_lhs_op);
impl_logic_binop!(OrVDS, DVector<bool>, bool, DVector<bool>,or_scalar_lhs_op);
impl_logic_binop!(OrMDS, DMatrix<bool>, bool, DMatrix<bool>,or_scalar_lhs_op);
impl_logic_binop!(OrM2M2, Matrix2<bool>,Matrix2<bool>,Matrix2<bool>, or_vec_op);
impl_logic_binop!(OrM3M3, Matrix3<bool>,Matrix3<bool>,Matrix3<bool>, or_vec_op);
impl_logic_binop!(OrM2x3M2x3, Matrix2x3<bool>,Matrix2x3<bool>,Matrix2x3<bool>, or_vec_op);
impl_logic_binop!(OrR2R2, RowVector2<bool>, RowVector2<bool>, RowVector2<bool>, or_vec_op);
impl_logic_binop!(OrR3R3, RowVector3<bool>, RowVector3<bool>, RowVector3<bool>, or_vec_op);
impl_logic_binop!(OrR4R4, RowVector4<bool>, RowVector4<bool>, RowVector4<bool>, or_vec_op);
impl_logic_binop!(OrRDRD, RowDVector<bool>, RowDVector<bool>, RowDVector<bool>, or_vec_op);
impl_logic_binop!(OrVDVD, DVector<bool>,DVector<bool>,DVector<bool>, or_vec_op);
impl_logic_binop!(OrMDMD, DMatrix<bool>,DMatrix<bool>,DMatrix<bool>, or_vec_op);

fn generate_or_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_binop_match_arms!(
    Or,
    (lhs_value, rhs_value),
    Bool, Bool => MatrixBool, bool, false;
  )
}

pub struct LogicOr {}

impl NativeFunctionCompiler for LogicOr {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let lhs_value = arguments[0].clone();
    let rhs_value = arguments[1].clone();
    match generate_or_fxn(lhs_value.clone(), rhs_value.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (lhs_value,rhs_value) {
          (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {generate_or_fxn(lhs.borrow().clone(), rhs.borrow().clone())}
          (lhs_value,Value::MutableReference(rhs)) => { generate_or_fxn(lhs_value.clone(), rhs.borrow().clone())}
          (Value::MutableReference(lhs),rhs_value) => { generate_or_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// Xor ------------------------------------------------------------------------

impl_logic_binop!(XorScalar, bool, bool, bool, xor_op);
impl_logic_binop!(XorSM2x3, bool, Matrix2x3<bool>, Matrix2x3<bool>,xor_scalar_rhs_op);
impl_logic_binop!(XorSM2, bool, Matrix2<bool>, Matrix2<bool>,xor_scalar_rhs_op);
impl_logic_binop!(XorSM3, bool, Matrix3<bool>, Matrix3<bool>,xor_scalar_rhs_op);
impl_logic_binop!(XorSR2, bool, RowVector2<bool>, RowVector2<bool>,xor_scalar_rhs_op);
impl_logic_binop!(XorSR3, bool, RowVector3<bool>, RowVector3<bool>,xor_scalar_rhs_op);
impl_logic_binop!(XorSR4, bool, RowVector4<bool>, RowVector4<bool>,xor_scalar_rhs_op);
impl_logic_binop!(XorSRD, bool, RowDVector<bool>, RowDVector<bool>,xor_scalar_rhs_op);
impl_logic_binop!(XorSVD, bool, DVector<bool>, DVector<bool>,xor_scalar_rhs_op);
impl_logic_binop!(XorSMD, bool, DMatrix<bool>, DMatrix<bool>,xor_scalar_rhs_op);
impl_logic_binop!(XorM2x3S, Matrix2x3<bool>, bool, Matrix2x3<bool>,xor_scalar_lhs_op);
impl_logic_binop!(XorM2S, Matrix2<bool>, bool, Matrix2<bool>,xor_scalar_lhs_op);
impl_logic_binop!(XorM3S, Matrix3<bool>, bool, Matrix3<bool>,xor_scalar_lhs_op);
impl_logic_binop!(XorR2S, RowVector2<bool>, bool, RowVector2<bool>,xor_scalar_lhs_op);
impl_logic_binop!(XorR3S, RowVector3<bool>, bool, RowVector3<bool>,xor_scalar_lhs_op);
impl_logic_binop!(XorR4S, RowVector4<bool>, bool, RowVector4<bool>,xor_scalar_lhs_op);
impl_logic_binop!(XorRDS, RowDVector<bool>, bool, RowDVector<bool>,xor_scalar_lhs_op);
impl_logic_binop!(XorVDS, DVector<bool>, bool, DVector<bool>,xor_scalar_lhs_op);
impl_logic_binop!(XorMDS, DMatrix<bool>, bool, DMatrix<bool>,xor_scalar_lhs_op);
impl_logic_binop!(XorM2M2, Matrix2<bool>,Matrix2<bool>,Matrix2<bool>, xor_vec_op);
impl_logic_binop!(XorM3M3, Matrix3<bool>,Matrix3<bool>,Matrix3<bool>, xor_vec_op);
impl_logic_binop!(XorM2x3M2x3, Matrix2x3<bool>,Matrix2x3<bool>,Matrix2x3<bool>, xor_vec_op);
impl_logic_binop!(XorR2R2, RowVector2<bool>, RowVector2<bool>, RowVector2<bool>, xor_vec_op);
impl_logic_binop!(XorR3R3, RowVector3<bool>, RowVector3<bool>, RowVector3<bool>, xor_vec_op);
impl_logic_binop!(XorR4R4, RowVector4<bool>, RowVector4<bool>, RowVector4<bool>, xor_vec_op);
impl_logic_binop!(XorRDRD, RowDVector<bool>, RowDVector<bool>, RowDVector<bool>, xor_vec_op);
impl_logic_binop!(XorVDVD, DVector<bool>,DVector<bool>,DVector<bool>, xor_vec_op);
impl_logic_binop!(XorMDMD, DMatrix<bool>,DMatrix<bool>,DMatrix<bool>, xor_vec_op);

fn generate_xor_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_binop_match_arms!(
    Xor,
    (lhs_value, rhs_value),
    Bool, Bool => MatrixBool, bool, false;
  )
}

pub struct LogicXor {}

impl NativeFunctionCompiler for LogicXor {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let lhs_value = arguments[0].clone();
    let rhs_value = arguments[1].clone();
    match generate_xor_fxn(lhs_value.clone(), rhs_value.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (lhs_value,rhs_value) {
          (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {generate_xor_fxn(lhs.borrow().clone(), rhs.borrow().clone())}
          (lhs_value,Value::MutableReference(rhs)) => { generate_xor_fxn(lhs_value.clone(), rhs.borrow().clone())}
          (Value::MutableReference(lhs),rhs_value) => { generate_xor_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// Not ------------------------------------------------------------------------

impl_logic_urnop!(NotScalar, bool, bool, not_op);
impl_logic_urnop!(NotM2, Matrix2<bool>, Matrix2<bool>, not_vec_op);
impl_logic_urnop!(NotM3, Matrix3<bool>, Matrix3<bool>, not_vec_op);
impl_logic_urnop!(NotM2x3, Matrix2x3<bool>, Matrix2x3<bool>, not_vec_op);
impl_logic_urnop!(NotR2, RowVector2<bool>, RowVector2<bool>, not_vec_op);
impl_logic_urnop!(NotR3, RowVector3<bool>, RowVector3<bool>, not_vec_op);
impl_logic_urnop!(NotR4, RowVector4<bool>, RowVector4<bool>, not_vec_op);
impl_logic_urnop!(NotRD, RowDVector<bool>, RowDVector<bool>, not_vec_op);
impl_logic_urnop!(NotVD, DVector<bool>, DVector<bool>, not_vec_op);
impl_logic_urnop!(NotMD, DMatrix<bool>, DMatrix<bool>, not_vec_op);

fn generate_not_fxn(arg_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_urnop_match_arms!(
    Not,
    (arg_value),
    Bool => MatrixBool, bool, false;
  )
}

pub struct LogicNot {}

impl NativeFunctionCompiler for LogicNot {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let arg_value = arguments[0].clone();
    match generate_not_fxn(arg_value.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match arg_value {
          (Value::MutableReference(arg)) => {generate_not_fxn(arg.borrow().clone())}
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}