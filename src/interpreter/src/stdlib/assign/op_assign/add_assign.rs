#[macro_use]
use crate::stdlib::*;
use super::*;

// Add Assign -----------------------------------------------------------------

// We will mostly use the assign macros for this

#[macro_export]
macro_rules! impl_add_assign_match_arms {
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

macro_rules! impl_add_assign_range_fxn_s {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    impl_op_assign_range_fxn_s!($struct_name, $op, $ix);
  }
}

macro_rules! impl_add_assign_range_fxn_v {
  ($struct_name:ident, $op:ident, $ix:ty) => {
    impl_op_assign_range_fxn_v!($struct_name, $op, $ix);
  }
}

// x = 1 ----------------------------------------------------------------------

#[derive(Debug)]
struct AddAssignVV<T> {
  sink: Ref<T>,
  source: Ref<T>,
}
impl<T> MechFunction for AddAssignVV<T> 
where
  T: Debug + Clone + Sync + Send + 'static +
  Add<Output = T> + AddAssign +
  PartialEq + PartialOrd,
  Ref<T>: ToValue
{
  fn solve(&self) {
    unsafe {
      let mut sink_ptr = (&mut *(self.sink.as_ptr()));
      let source_ptr = &(*(self.source.as_ptr()));
      *sink_ptr += source_ptr.clone();
    }
  }
  fn out(&self) -> Value { self.sink.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[derive(Debug)]
struct AddAssignMDMD<T> {
  sink: Ref<DMatrix<T>>,
  source: Ref<DMatrix<T>>,
}
impl<T> MechFunction for AddAssignMDMD<T> 
where
  T: Debug + Clone + Sync + Send + 'static +
  Add<Output = T> + AddAssign +
  PartialEq + PartialOrd,
  Ref<DMatrix<T>>: ToValue
{
  fn solve(&self) {
    unsafe {
      let mut sink_ptr = (&mut *(self.sink.as_ptr()));
      let source_ptr = &(*(self.source.as_ptr()));
      *sink_ptr += source_ptr;
    }
  }
  fn out(&self) -> Value { self.sink.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[derive(Debug)]
struct TableAppendRecord {
  sink: Ref<MechTable>,
  source: Ref<MechRecord>,
}
impl MechFunction for TableAppendRecord {
  fn solve(&self) {
    unsafe {
      let mut sink_ptr = (&mut *(self.sink.as_ptr()));
      let source_ptr = &(*(self.source.as_ptr()));
      sink_ptr.append_record(source_ptr.clone());
    }
  }
  fn out(&self) -> Value { Value::Table(self.sink.clone()) }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[derive(Debug)]
struct TableAppendTable {
  sink: Ref<MechTable>,
  source: Ref<MechTable>,
}
impl MechFunction for TableAppendTable {
  fn solve(&self) {
    unsafe {
      let mut sink_ptr = (&mut *(self.sink.as_ptr()));
      let source_ptr = &(*(self.source.as_ptr()));
      sink_ptr.append_table(&source_ptr);
    }
  }
  fn out(&self) -> Value { Value::Table(self.sink.clone()) }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

fn add_assign_value_fxn(sink: Value, source: Value) -> Result<Box<dyn MechFunction>, MechError> {
  match (sink,source) {
    (Value::Table(tbl), Value::Record(rcrd)) => {
      tbl.borrow().check_record_schema(&rcrd.borrow())?;
      Ok(Box::new(TableAppendRecord{ sink: tbl, source: rcrd }))
    }
    (Value::Table(tbl_sink), Value::Table(tbl_src)) => {
      tbl_sink.borrow().check_table_schema(&tbl_src.borrow())?;
      Ok(Box::new(TableAppendTable{ sink: tbl_sink, source: tbl_src }))
    }
    (Value::U8(sink),Value::U8(source)) => Ok(Box::new(AddAssignVV{sink: sink.clone(), source: source.clone()})),
    (Value::U16(sink),Value::U16(source)) => Ok(Box::new(AddAssignVV{sink: sink.clone(), source: source.clone()})),
    (Value::U32(sink),Value::U32(source)) => Ok(Box::new(AddAssignVV{sink: sink.clone(), source: source.clone()})),
    (Value::U64(sink),Value::U64(source)) => Ok(Box::new(AddAssignVV{sink: sink.clone(), source: source.clone()})),
    (Value::U128(sink),Value::U128(source)) => Ok(Box::new(AddAssignVV{sink: sink.clone(), source: source.clone()})),
    (Value::I8(sink),Value::I8(source)) => Ok(Box::new(AddAssignVV{sink: sink.clone(), source: source.clone()})),
    (Value::I16(sink),Value::I16(source)) => Ok(Box::new(AddAssignVV{sink: sink.clone(), source: source.clone()})),
    (Value::I32(sink),Value::I32(source)) => Ok(Box::new(AddAssignVV{sink: sink.clone(), source: source.clone()})),
    (Value::I64(sink),Value::I64(source)) => Ok(Box::new(AddAssignVV{sink: sink.clone(), source: source.clone()})),
    (Value::I128(sink),Value::I128(source)) => Ok(Box::new(AddAssignVV{sink: sink.clone(), source: source.clone()})),
    (Value::F32(sink),Value::F32(source)) => Ok(Box::new(AddAssignVV{sink: sink.clone(), source: source.clone()})),
    (Value::F64(sink),Value::F64(source)) => Ok(Box::new(AddAssignVV{sink: sink.clone(), source: source.clone()})),
    (Value::MatrixF64(Matrix::Matrix1(sink)),Value::MatrixF64(Matrix::Matrix1(source))) => Ok(Box::new(AddAssignVV{sink: sink.clone(), source: source.clone()})),
    (Value::MatrixF64(Matrix::Matrix2(sink)),Value::MatrixF64(Matrix::Matrix2(source))) => Ok(Box::new(AddAssignVV{sink: sink.clone(), source: source.clone()})),
    (Value::MatrixF64(Matrix::Matrix2x3(sink)),Value::MatrixF64(Matrix::Matrix2x3(source))) => Ok(Box::new(AddAssignVV{sink: sink.clone(), source: source.clone()})),
    (Value::MatrixF64(Matrix::Matrix3x2(sink)),Value::MatrixF64(Matrix::Matrix3x2(source))) => Ok(Box::new(AddAssignVV{sink: sink.clone(), source: source.clone()})),
    (Value::MatrixF64(Matrix::Matrix3(sink)),Value::MatrixF64(Matrix::Matrix3(source))) => Ok(Box::new(AddAssignVV{sink: sink.clone(), source: source.clone()})),
    (Value::MatrixF64(Matrix::Matrix4(sink)),Value::MatrixF64(Matrix::Matrix4(source))) => Ok(Box::new(AddAssignVV{sink: sink.clone(), source: source.clone()})),
    (Value::MatrixF64(Matrix::DMatrix(sink)),Value::MatrixF64(Matrix::DMatrix(source))) => Ok(Box::new(AddAssignMDMD{sink: sink.clone(), source: source.clone()})),
    (Value::MatrixF64(Matrix::Vector2(sink)),Value::MatrixF64(Matrix::Vector2(source))) => Ok(Box::new(AddAssignVV{sink: sink.clone(), source: source.clone()})),
    (Value::MatrixF64(Matrix::Vector3(sink)),Value::MatrixF64(Matrix::Vector3(source))) => Ok(Box::new(AddAssignVV{sink: sink.clone(), source: source.clone()})),
    (Value::MatrixF64(Matrix::Vector4(sink)),Value::MatrixF64(Matrix::Vector4(source))) => Ok(Box::new(AddAssignVV{sink: sink.clone(), source: source.clone()})),
    (Value::MatrixF64(Matrix::DVector(sink)),Value::MatrixF64(Matrix::DVector(source))) => Ok(Box::new(AddAssignVV{sink: sink.clone(), source: source.clone()})),
    (Value::MatrixF64(Matrix::RowVector2(sink)),Value::MatrixF64(Matrix::RowVector2(source))) => Ok(Box::new(AddAssignVV{sink: sink.clone(), source: source.clone()})),
    (Value::MatrixF64(Matrix::RowVector3(sink)),Value::MatrixF64(Matrix::RowVector3(source))) => Ok(Box::new(AddAssignVV{sink: sink.clone(), source: source.clone()})),
    (Value::MatrixF64(Matrix::RowVector4(sink)),Value::MatrixF64(Matrix::RowVector4(source))) => Ok(Box::new(AddAssignVV{sink: sink.clone(), source: source.clone()})),
    (Value::MatrixF64(Matrix::RowDVector(sink)),Value::MatrixF64(Matrix::RowDVector(source))) => Ok(Box::new(AddAssignVV{sink: sink.clone(), source: source.clone()})),
    x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
  }
}

pub struct AddAssignValue {}
impl NativeFunctionCompiler for AddAssignValue {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink = arguments[0].clone();
    let source = arguments[1].clone();
    match add_assign_value_fxn(sink.clone(),source.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match (sink,source) {
          (Value::MutableReference(sink),Value::MutableReference(source)) => { add_assign_value_fxn(sink.borrow().clone(),source.borrow().clone()) },
          (sink,Value::MutableReference(source)) => { add_assign_value_fxn(sink.clone(),source.borrow().clone()) },
          (Value::MutableReference(sink),source) => { add_assign_value_fxn(sink.borrow().clone(),source.clone()) },
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

impl_add_assign_range_fxn_s!(AddAssign1DRS,add_assign_1d_range,usize);
impl_add_assign_range_fxn_s!(AddAssign1DRB,add_assign_1d_range_b,bool);
impl_add_assign_range_fxn_v!(AddAssign1DRV,add_assign_1d_range_vec,usize);
impl_add_assign_range_fxn_v!(AddAssign1DRVB,add_assign_1d_range_vec_b,bool);

fn add_assign_range_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_add_assign_match_arms!(AddAssign1DR, range, (sink, ixes.as_slice(), source))
}

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

impl_add_assign_range_fxn_s!(AddAssign2DRAS, add_assign_2d_vector_all,usize);
impl_add_assign_range_fxn_s!(AddAssign2DRASB,add_assign_2d_vector_all_b,bool);
impl_add_assign_range_fxn_v!(AddAssign2DRAV, add_assign_2d_vector_all_mat,usize);
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