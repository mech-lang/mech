#[macro_use]
use crate::stdlib::*;

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

macro_rules! impl_add_assign_fxn {
  ($struct_name:ident, $matrix_shape:ident, $source_matrix_shape:ty, $op:ident, $ix:ty) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<$source_matrix_shape>,
      ixes: Ref<DVector<$ix>>,
      sink: Ref<$matrix_shape<T>>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + 'static +
      Add<Output = T> + AddAssign +
      Zero + One +
      PartialEq + PartialOrd,
      Ref<$matrix_shape<T>>: ToValue
    {
      fn solve(&self) {
        unsafe {
          let ix_ptr = (*(self.ixes.as_ptr())).clone();
          let mut sink_ptr = (&mut *(self.sink.as_ptr()));
          let source_ptr = (*(self.source.as_ptr())).clone();
          $op!(source_ptr,ix_ptr,sink_ptr);
        }
      }
      fn out(&self) -> Value { self.sink.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }};}

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
        ($sink)[($ix)[i] - 1] += ($source);
      }
    }
  };}

macro_rules! add_assign_1d_range_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        if $ix[i] == true {
          ($sink)[i] += ($source);
        }
      }
    }
  };}  

macro_rules! add_assign_1d_range_vec {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        ($sink)[($ix)[i] - 1] += ($source)[i];
      }
    }
  };}

impl_add_assign_fxn!(AddAssign1DRRD,RowDVector,T,add_assign_1d_range,usize);
impl_add_assign_fxn!(AddAssign1DRVD,DVector,T,add_assign_1d_range,usize);
impl_add_assign_fxn!(AddAssign1DRMD,DMatrix,T,add_assign_1d_range,usize);
impl_add_assign_fxn!(AddAssign1DRR4,RowVector4,T,add_assign_1d_range,usize);
impl_add_assign_fxn!(AddAssign1DRR3,RowVector3,T,add_assign_1d_range,usize);
impl_add_assign_fxn!(AddAssign1DRR2,RowVector2,T,add_assign_1d_range,usize);
impl_add_assign_fxn!(AddAssign1DRV4,Vector4,T,add_assign_1d_range,usize);
impl_add_assign_fxn!(AddAssign1DRV3,Vector3,T,add_assign_1d_range,usize);
impl_add_assign_fxn!(AddAssign1DRV2,Vector2,T,add_assign_1d_range,usize);
impl_add_assign_fxn!(AddAssign1DRM4,Matrix4,T,add_assign_1d_range,usize);
impl_add_assign_fxn!(AddAssign1DRM3,Matrix3,T,add_assign_1d_range,usize);
impl_add_assign_fxn!(AddAssign1DRM2,Matrix2,T,add_assign_1d_range,usize);
impl_add_assign_fxn!(AddAssign1DRM1,Matrix1,T,add_assign_1d_range,usize);
impl_add_assign_fxn!(AddAssign1DRM2x3,Matrix2x3,T,add_assign_1d_range,usize);
impl_add_assign_fxn!(AddAssign1DRM3x2,Matrix3x2,T,add_assign_1d_range,usize);

impl_add_assign_fxn!(AddAssign1DRRDB,RowDVector,T,add_assign_1d_range_b,bool);
impl_add_assign_fxn!(AddAssign1DRVDB,DVector,T,add_assign_1d_range_b,bool);
impl_add_assign_fxn!(AddAssign1DRMDB,DMatrix,T,add_assign_1d_range_b,bool);
impl_add_assign_fxn!(AddAssign1DRR4B,RowVector4,T,add_assign_1d_range_b,bool);
impl_add_assign_fxn!(AddAssign1DRR3B,RowVector3,T,add_assign_1d_range_b,bool);
impl_add_assign_fxn!(AddAssign1DRR2B,RowVector2,T,add_assign_1d_range_b,bool);
impl_add_assign_fxn!(AddAssign1DRV4B,Vector4,T,add_assign_1d_range_b,bool);
impl_add_assign_fxn!(AddAssign1DRV3B,Vector3,T,add_assign_1d_range_b,bool);
impl_add_assign_fxn!(AddAssign1DRV2B,Vector2,T,add_assign_1d_range_b,bool);
impl_add_assign_fxn!(AddAssign1DRM4B,Matrix4,T,add_assign_1d_range_b,bool);
impl_add_assign_fxn!(AddAssign1DRM3B,Matrix3,T,add_assign_1d_range_b,bool);
impl_add_assign_fxn!(AddAssign1DRM2B,Matrix2,T,add_assign_1d_range_b,bool);
impl_add_assign_fxn!(AddAssign1DRM1B,Matrix1,T,add_assign_1d_range_b,bool);
impl_add_assign_fxn!(AddAssign1DRM2x3B,Matrix2x3,T,add_assign_1d_range_b,bool);
impl_add_assign_fxn!(AddAssign1DRM3x2B,Matrix3x2,T,add_assign_1d_range_b,bool);

impl_add_assign_fxn!(AddAssign1DRR4R4,RowVector4,RowVector4<T>,add_assign_1d_range_vec,usize);
impl_add_assign_fxn!(AddAssign1DRR4R3,RowVector4,RowVector3<T>,add_assign_1d_range_vec,usize);
impl_add_assign_fxn!(AddAssign1DRR4R2,RowVector4,RowVector2<T>,add_assign_1d_range_vec,usize);
impl_add_assign_fxn!(AddAssign1DRV4V4,Vector4,Vector4<T>,add_assign_1d_range_vec,usize);
impl_add_assign_fxn!(AddAssign1DRV4V3,Vector4,Vector3<T>,add_assign_1d_range_vec,usize);
impl_add_assign_fxn!(AddAssign1DRV4V2,Vector4,Vector2<T>,add_assign_1d_range_vec,usize);

impl_add_assign_fxn!(AddAssign1DRMDMD,DMatrix,DMatrix<T>,add_assign_1d_range_vec,usize);

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
        for rix in &$ix {
          ($sink).column_mut(cix)[rix - 1] += ($source);
        }
      }
    };}

macro_rules! add_assign_2d_vector_all_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    for cix in 0..($sink).ncols() {
      for rix in 0..$ix.len() {
        if $ix[rix] == true {
          ($sink).column_mut(cix)[rix] += ($source);
        }
      }
    }
  };} 


macro_rules! add_assign_2d_vector_all_mat {
  ($source:expr, $ix:expr, $sink:expr) => {
    for (i,rix) in (&$ix).iter().enumerate() {
      let mut row = ($sink).row_mut(rix - 1);
      row += ($source).row(i);
    }
  };}

macro_rules! add_assign_2d_vector_all_mat_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    for (i,rix) in (&$ix).iter().enumerate() {
      if *rix == true {
        let mut row = ($sink).row_mut(i);
        row += ($source).row(i);
      }
    }
  };} 

impl_add_assign_fxn!(AddAssign2DRAMD,DMatrix,T,add_assign_2d_vector_all,usize);
impl_add_assign_fxn!(AddAssign2DRAM4,Matrix4,T,add_assign_2d_vector_all,usize);
impl_add_assign_fxn!(AddAssign2DRAM3,Matrix3,T,add_assign_2d_vector_all,usize);
impl_add_assign_fxn!(AddAssign2DRAM2,Matrix2,T,add_assign_2d_vector_all,usize);
impl_add_assign_fxn!(AddAssign2DRAM1,Matrix1,T,add_assign_2d_vector_all,usize);
impl_add_assign_fxn!(AddAssign2DRAM2x3,Matrix2x3,T,add_assign_2d_vector_all,usize);
impl_add_assign_fxn!(AddAssign2DRAM3x2,Matrix3x2,T,add_assign_2d_vector_all,usize);

impl_add_assign_fxn!(AddAssign2DRAMDMD,DMatrix,DMatrix<T>,add_assign_2d_vector_all_mat,usize);

impl_add_assign_fxn!(AddAssign2DRAMDM2,DMatrix,Matrix2<T>,add_assign_2d_vector_all_mat,usize);
impl_add_assign_fxn!(AddAssign2DRAMDM2x3,DMatrix,Matrix2x3<T>,add_assign_2d_vector_all_mat,usize);
impl_add_assign_fxn!(AddAssign2DRAMDM3,DMatrix,Matrix3<T>,add_assign_2d_vector_all_mat,usize);
impl_add_assign_fxn!(AddAssign2DRAMDM3x2,DMatrix,Matrix3x2<T>,add_assign_2d_vector_all_mat,usize);
impl_add_assign_fxn!(AddAssign2DRAMDM4,DMatrix,Matrix4<T>,add_assign_2d_vector_all_mat,usize);

impl_add_assign_fxn!(AddAssign2DRAM2M2,Matrix2,Matrix2<T>,add_assign_2d_vector_all_mat,usize);
impl_add_assign_fxn!(AddAssign2DRAM2M3x2,Matrix2,Matrix3x2<T>,add_assign_2d_vector_all_mat,usize);
impl_add_assign_fxn!(AddAssign2DRAM2MD,Matrix2,DMatrix<T>,add_assign_2d_vector_all_mat,usize);

impl_add_assign_fxn!(AddAssign2DRAM3M3,Matrix3,Matrix3<T>,add_assign_2d_vector_all_mat,usize);
impl_add_assign_fxn!(AddAssign2DRAM3M2x3,Matrix3,Matrix2x3<T>,add_assign_2d_vector_all_mat,usize);
impl_add_assign_fxn!(AddAssign2DRAM3MD,Matrix3,DMatrix<T>,add_assign_2d_vector_all_mat,usize);

impl_add_assign_fxn!(AddAssign2DRAM3x2M3x2,Matrix3x2,Matrix3x2<T>,add_assign_2d_vector_all_mat,usize);
impl_add_assign_fxn!(AddAssign2DRAM3x2M2,Matrix3x2,Matrix2<T>,add_assign_2d_vector_all_mat,usize);
impl_add_assign_fxn!(AddAssign2DRAM3x2MD,Matrix3x2,DMatrix<T>,add_assign_2d_vector_all_mat,usize);

impl_add_assign_fxn!(AddAssign2DRAM2x3M2x3,Matrix2x3,Matrix2x3<T>,add_assign_2d_vector_all_mat,usize);
impl_add_assign_fxn!(AddAssign2DRAM2x3M3,Matrix2x3,Matrix3<T>,add_assign_2d_vector_all_mat,usize);
impl_add_assign_fxn!(AddAssign2DRAM2x3MD,Matrix2x3,DMatrix<T>,add_assign_2d_vector_all_mat,usize);

impl_add_assign_fxn!(AddAssign2DRAM4M4,Matrix4,Matrix4<T>,add_assign_2d_vector_all_mat,usize);
impl_add_assign_fxn!(AddAssign2DRAM4MD,Matrix4,DMatrix<T>,add_assign_2d_vector_all_mat,usize);

impl_add_assign_fxn!(AddAssign2DRAMDB,DMatrix,T,add_assign_2d_vector_all_b,bool);
impl_add_assign_fxn!(AddAssign2DRAM4B,Matrix4,T,add_assign_2d_vector_all_b,bool);
impl_add_assign_fxn!(AddAssign2DRAM3B,Matrix3,T,add_assign_2d_vector_all_b,bool);
impl_add_assign_fxn!(AddAssign2DRAM2B,Matrix2,T,add_assign_2d_vector_all_b,bool);
impl_add_assign_fxn!(AddAssign2DRAM1B,Matrix1,T,add_assign_2d_vector_all_b,bool);
impl_add_assign_fxn!(AddAssign2DRAM2x3B,Matrix2x3,T,add_assign_2d_vector_all_b,bool);
impl_add_assign_fxn!(AddAssign2DRAM3x2B,Matrix3x2,T,add_assign_2d_vector_all_b,bool);

impl_add_assign_fxn!(AddAssign2DRAMDMDB,DMatrix,DMatrix<T>,add_assign_2d_vector_all_mat_b,bool);
impl_add_assign_fxn!(AddAssign2DRAM2M2B,Matrix2,Matrix2<T>,add_assign_2d_vector_all_mat_b,bool);
impl_add_assign_fxn!(AddAssign2DRAM3M3B,Matrix3,Matrix3<T>,add_assign_2d_vector_all_mat_b,bool);
impl_add_assign_fxn!(AddAssign2DRAM4M4B,Matrix4,Matrix4<T>,add_assign_2d_vector_all_mat_b,bool);
impl_add_assign_fxn!(AddAssign2DRAM3x2M3x2B,Matrix3x2,Matrix3x2<T>,add_assign_2d_vector_all_mat_b,bool);
impl_add_assign_fxn!(AddAssign2DRAM2x3M2x3B,Matrix2x3,Matrix2x3<T>,add_assign_2d_vector_all_mat_b,bool);

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