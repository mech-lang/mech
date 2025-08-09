#[macro_use]
use crate::stdlib::*;
use std::marker::PhantomData;

// Set -----------------------------------------------------------------

macro_rules! impl_set_match_arms {
  ($fxn_name:ident,$macro_name:ident, $arg:expr) => {
    paste!{
      [<impl_set_ $macro_name _match_arms>]!(
        $fxn_name,
        $arg,
        Bool, "Bool";
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
        String, "String"
      )
    }
  }
}

// x[1] = 1 ------------------------------------------------------------------
  
macro_rules! set_1d_set_scalar {
  ($sink:expr, $ix:expr, $source:expr) => {
    ($sink)[$ix - 1] = ($source).clone();
  };}  

macro_rules! set_1d_set_scalar_b {
  ($sink:expr, $ix:expr, $source:expr) => {
    if $ix {
      for iy in 0..$sink.len() {
        ($sink)[iy] = ($source).clone();
      }
    }
  };}  

macro_rules! impl_set_scalar_fxn {
  ($struct_name:ident, $matrix_shape:ident, $op:tt, $ix:ty) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<T>,
      ixes: Ref<$ix>,
      sink: Ref<$matrix_shape<T>>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$matrix_shape<T>>: ToValue
    {
      fn solve(&self) {
        unsafe {
          let ix_ptr = (*(self.ixes.as_ptr())).clone();
          let mut sink_ptr = (&mut *(self.sink.as_ptr()));
          let source_ptr = (*(self.source.as_ptr())).clone();
          $op!(sink_ptr,ix_ptr,source_ptr);
        }
      }
      fn out(&self) -> Value { self.sink.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }};}

#[cfg(feature = "RowVector4")]
impl_set_scalar_fxn!(Set1DSR4,RowVector4, set_1d_set_scalar, usize);
#[cfg(feature = "RowVector3")]
impl_set_scalar_fxn!(Set1DSR3,RowVector3, set_1d_set_scalar, usize);
#[cfg(feature = "RowVector2")]
impl_set_scalar_fxn!(Set1DSR2,RowVector2, set_1d_set_scalar, usize);
#[cfg(feature = "RowVectorD")]
impl_set_scalar_fxn!(Set1DSRD,RowDVector, set_1d_set_scalar, usize);
#[cfg(feature = "Vector4")]
impl_set_scalar_fxn!(Set1DSV4,Vector4, set_1d_set_scalar, usize);
#[cfg(feature = "Vector3")]
impl_set_scalar_fxn!(Set1DSV3,Vector3, set_1d_set_scalar, usize);
#[cfg(feature = "Vector2")]
impl_set_scalar_fxn!(Set1DSV2,Vector2, set_1d_set_scalar, usize);
#[cfg(feature = "VectorD")]
impl_set_scalar_fxn!(Set1DSVD,DVector, set_1d_set_scalar, usize);
#[cfg(feature = "MAtrix4")]
impl_set_scalar_fxn!(Set1DSM4,Matrix4, set_1d_set_scalar, usize);
#[cfg(feature = "Matrix3")]
impl_set_scalar_fxn!(Set1DSM3,Matrix3, set_1d_set_scalar, usize);
#[cfg(feature = "Matrix2")]
impl_set_scalar_fxn!(Set1DSM2,Matrix2, set_1d_set_scalar, usize);
#[cfg(feature = "MAtrix1")]
impl_set_scalar_fxn!(Set1DSM1,Matrix1, set_1d_set_scalar, usize);
#[cfg(feature = "Matrix2x3")]
impl_set_scalar_fxn!(Set1DSM2x3,Matrix2x3, set_1d_set_scalar, usize);
#[cfg(feature = "Matrix3x2")]
impl_set_scalar_fxn!(Set1DSM3x2,Matrix3x2, set_1d_set_scalar, usize);
#[cfg(feature = "MatrixD")]
impl_set_scalar_fxn!(Set1DSMD,DMatrix, set_1d_set_scalar, usize);

#[cfg(feature = "RowVector4")]
impl_set_scalar_fxn!(Set1DSR4B,RowVector4, set_1d_set_scalar_b, bool);
#[cfg(feature = "RowVector3")]
impl_set_scalar_fxn!(Set1DSR3B,RowVector3, set_1d_set_scalar_b, bool);
#[cfg(feature = "RowVector2")]
impl_set_scalar_fxn!(Set1DSR2B,RowVector2, set_1d_set_scalar_b, bool);
#[cfg(feature = "RowVectorD")]
impl_set_scalar_fxn!(Set1DSRDB,RowDVector, set_1d_set_scalar_b, bool);
#[cfg(feature = "Vector4")]
impl_set_scalar_fxn!(Set1DSV4B,Vector4, set_1d_set_scalar_b, bool);
#[cfg(feature = "Vector3")]
impl_set_scalar_fxn!(Set1DSV3B,Vector3, set_1d_set_scalar_b, bool);
#[cfg(feature = "Vector2")]
impl_set_scalar_fxn!(Set1DSV2B,Vector2, set_1d_set_scalar_b, bool);
#[cfg(feature = "VectorD")]
impl_set_scalar_fxn!(Set1DSVDB,DVector, set_1d_set_scalar_b, bool);
#[cfg(feature = "MAtrix4")]
impl_set_scalar_fxn!(Set1DSM4B,Matrix4, set_1d_set_scalar_b, bool);
#[cfg(feature = "Matrix3")]
impl_set_scalar_fxn!(Set1DSM3B,Matrix3, set_1d_set_scalar_b, bool);
#[cfg(feature = "Matrix2")]
impl_set_scalar_fxn!(Set1DSM2B,Matrix2, set_1d_set_scalar_b, bool);
#[cfg(feature = "MAtrix1")]
impl_set_scalar_fxn!(Set1DSM1B,Matrix1, set_1d_set_scalar_b, bool);
#[cfg(feature = "Matrix2x3")]
impl_set_scalar_fxn!(Set1DSM2x3B,Matrix2x3, set_1d_set_scalar_b, bool);
#[cfg(feature = "Matrix3x2")]
impl_set_scalar_fxn!(Set1DSM3x2B,Matrix3x2, set_1d_set_scalar_b, bool);
#[cfg(feature = "MatrixD")]
impl_set_scalar_fxn!(Set1DSMDB,DMatrix, set_1d_set_scalar_b, bool);

macro_rules! impl_set_scalar_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident,$value_string:tt);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          #[cfg(all(feature = $value_string, feature = "RowVector4"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)),[Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name R4>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "RowVector3"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector3(sink)),[Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name R3>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "RowVector2"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector2(sink)),[Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name R2>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)),   [Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name V4>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Vector3"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector3(sink)),   [Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name V3>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Vector2"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector2(sink)),   [Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name V2>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "MAtrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M4>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "MAtrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M1>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2x3>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3x2>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MD>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "RowVectorD"))]
          (Value::[<Matrix $value_kind>](Matrix::RowDVector(sink)),[Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name RD>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "VectorD"))]
          (Value::[<Matrix $value_kind>](Matrix::DVector(sink)),   [Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name VD>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
      
          #[cfg(all(feature = $value_string, feature = "RowVector4"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)),[Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name R4B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "RowVector3"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector3(sink)),[Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name R3B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "RowVector2"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector2(sink)),[Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name R2B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)),   [Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name V4B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Vector3"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector3(sink)),   [Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name V3B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Vector2"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector2(sink)),   [Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name V2B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "MAtrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M4B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "MAtrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M1B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2x3B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3x2B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MDB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "RowVectorD"))]
          (Value::[<Matrix $value_kind>](Matrix::RowDVector(sink)),[Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name RDB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "VectorD"))]
          (Value::[<Matrix $value_kind>](Matrix::DVector(sink)),   [Value::Bool(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name VDB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
        
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_scalar_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set1DS, scalar, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetScalar {}
impl NativeFunctionCompiler for MatrixSetScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_scalar_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_scalar_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3] = 1 ----------------------------------------------------------------

macro_rules! set_1d_range {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        ($sink)[($ix)[i] - 1] = ($source).clone();
      }
    }
  };}

macro_rules! set_1d_range_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        if $ix[i] == true {
          ($sink)[i] = ($source).clone();
        }
      }
    }
  };}  

macro_rules! set_1d_range_vec {
  ($source:expr, $ix:expr, $sink:expr) => {
    unsafe { 
      for i in 0..($ix).len() {
        ($sink)[($ix)[i] - 1] = ($source)[i].clone();
      }
    }
  };}  

#[macro_export]
macro_rules! impl_set_fxn {
  ($struct_name:ident, $matrix_shape:ident, $source_matrix_shape:ty, $op:ident, $ix:ty) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<$source_matrix_shape>,
      ixes: Ref<DVector<$ix>>,
      sink: Ref<$matrix_shape<T>>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static,
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

impl_set_fxn!(Set1DRRD,RowDVector,T,set_1d_range,usize);
impl_set_fxn!(Set1DRVD,DVector,T,set_1d_range,usize);
impl_set_fxn!(Set1DRMD,DMatrix,T,set_1d_range,usize);
impl_set_fxn!(Set1DRR4,RowVector4,T,set_1d_range,usize);
impl_set_fxn!(Set1DRR3,RowVector3,T,set_1d_range,usize);
impl_set_fxn!(Set1DRR2,RowVector2,T,set_1d_range,usize);
impl_set_fxn!(Set1DRV4,Vector4,T,set_1d_range,usize);
impl_set_fxn!(Set1DRV3,Vector3,T,set_1d_range,usize);
impl_set_fxn!(Set1DRV2,Vector2,T,set_1d_range,usize);
impl_set_fxn!(Set1DRM4,Matrix4,T,set_1d_range,usize);
impl_set_fxn!(Set1DRM3,Matrix3,T,set_1d_range,usize);
impl_set_fxn!(Set1DRM2,Matrix2,T,set_1d_range,usize);
impl_set_fxn!(Set1DRM1,Matrix1,T,set_1d_range,usize);
impl_set_fxn!(Set1DRM2x3,Matrix2x3,T,set_1d_range,usize);
impl_set_fxn!(Set1DRM3x2,Matrix3x2,T,set_1d_range,usize);

impl_set_fxn!(Set1DRRDB,RowDVector,T,set_1d_range_b,bool);
impl_set_fxn!(Set1DRVDB,DVector,T,set_1d_range_b,bool);
impl_set_fxn!(Set1DRMDB,DMatrix,T,set_1d_range_b,bool);
impl_set_fxn!(Set1DRR4B,RowVector4,T,set_1d_range_b,bool);
impl_set_fxn!(Set1DRR3B,RowVector3,T,set_1d_range_b,bool);
impl_set_fxn!(Set1DRR2B,RowVector2,T,set_1d_range_b,bool);
impl_set_fxn!(Set1DRV4B,Vector4,T,set_1d_range_b,bool);
impl_set_fxn!(Set1DRV3B,Vector3,T,set_1d_range_b,bool);
impl_set_fxn!(Set1DRV2B,Vector2,T,set_1d_range_b,bool);
impl_set_fxn!(Set1DRM4B,Matrix4,T,set_1d_range_b,bool);
impl_set_fxn!(Set1DRM3B,Matrix3,T,set_1d_range_b,bool);
impl_set_fxn!(Set1DRM2B,Matrix2,T,set_1d_range_b,bool);
impl_set_fxn!(Set1DRM1B,Matrix1,T,set_1d_range_b,bool);
impl_set_fxn!(Set1DRM2x3B,Matrix2x3,T,set_1d_range_b,bool);
impl_set_fxn!(Set1DRM3x2B,Matrix3x2,T,set_1d_range_b,bool);

impl_set_fxn!(Set1DRR4R4,RowVector4,RowVector4<T>,set_1d_range_vec,usize);
impl_set_fxn!(Set1DRR4R3,RowVector4,RowVector3<T>,set_1d_range_vec,usize);
impl_set_fxn!(Set1DRR4R2,RowVector4,RowVector2<T>,set_1d_range_vec,usize);
impl_set_fxn!(Set1DRV4V4,Vector4,Vector4<T>,set_1d_range_vec,usize);
impl_set_fxn!(Set1DRV4V3,Vector4,Vector3<T>,set_1d_range_vec,usize);
impl_set_fxn!(Set1DRV4V2,Vector4,Vector2<T>,set_1d_range_vec,usize);

impl_set_fxn!(Set1DRMDMD,DMatrix,DMatrix<T>,set_1d_range_vec,usize);

#[macro_export]
macro_rules! impl_set_range_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident,$value_string:tt);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          // Set vector
          #[cfg(all(feature = $value_string, feature = "RowVector4"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind>](Matrix::RowVector4(source))) => Ok(Box::new([<$fxn_name R4R4>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "RowVector4"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind>](Matrix::RowVector3(source))) => Ok(Box::new([<$fxn_name R4R3>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "RowVector4"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind>](Matrix::RowVector2(source))) => Ok(Box::new([<$fxn_name R4R2>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind>](Matrix::Vector4(source))) => Ok(Box::new([<$fxn_name V4V4>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind>](Matrix::Vector3(source))) => Ok(Box::new([<$fxn_name V4V3>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind>](Matrix::Vector2(source))) => Ok(Box::new([<$fxn_name V4V2>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),

          // Set Matrix
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) => Ok(Box::new([<$fxn_name MDMD>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          
          // Set scalar
          #[cfg(all(feature = $value_string, feature = "RowVector4"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name R4>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),            
          #[cfg(all(feature = $value_string, feature = "RowVector3"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector3(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name R3>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "RowVector2"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector2(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name R2>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name V4>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Vector3"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector3(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name V3>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Vector2"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector2(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name V2>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M4>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M1>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2x3>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3x2>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MD>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "RowVectorD"))]
          (Value::[<Matrix $value_kind>](Matrix::RowDVector(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name RD>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "VectorD"))]
          (Value::[<Matrix $value_kind>](Matrix::DVector(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name VD>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),

          // Bool
          #[cfg(all(feature = $value_string, feature = "RowVector4"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name R4B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "RowVector3"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector3(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name R3B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "RowVector2"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector2(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name R2B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name V4B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Vector3"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector3(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name V3B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Vector2"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector2(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name V2B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M4B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M1B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2x3B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3x2B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),            
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MDB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "VectorD"))]
          (Value::[<Matrix $value_kind>](Matrix::DVector(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name VDB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "RowVectorD"))]
          (Value::[<Matrix $value_kind>](Matrix::RowDVector(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name RDB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),                      
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_range_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set1DR, range, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetRange {}
impl NativeFunctionCompiler for MatrixSetRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_range_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        println!("FOFOFOFOOF: {:?}", x);
        match (sink,source) {
          (Value::MutableReference(sink),Value::MutableReference(source)) => { impl_set_range_fxn(sink.borrow().clone(),source.borrow().clone(),ixes.clone()) },
          (sink,Value::MutableReference(source)) => { impl_set_range_fxn(sink.clone(),source.borrow().clone(),ixes.clone()) },
          (Value::MutableReference(sink),source) => { impl_set_range_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) },
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[:] = 1 ------------------------------------------------------------------

macro_rules! impl_set_all_fxn {
  ($struct_name:ident, $matrix_shape:ident) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<T>,
      sink: Ref<$matrix_shape<T>>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$matrix_shape<T>>: ToValue
    {
      fn solve(&self) {
        unsafe { 
          let mut sink_ptr = (&mut *(self.sink.as_ptr()));
          let source_ptr = (*(self.source.as_ptr())).clone();
          for i in 0..(sink_ptr).len() {
            (sink_ptr)[i] = (source_ptr).clone();
          }
        }
      }
      fn out(&self) -> Value { self.sink.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }};}

impl_set_all_fxn!(Set1DARD,RowDVector); 
impl_set_all_fxn!(Set1DAVD,DVector); 
impl_set_all_fxn!(Set1DAMD,DMatrix); 
impl_set_all_fxn!(Set1DAR4,RowVector4);    
impl_set_all_fxn!(Set1DAR3,RowVector3);
impl_set_all_fxn!(Set1DAR2,RowVector2);
impl_set_all_fxn!(Set1DAV4,Vector4);    
impl_set_all_fxn!(Set1DAV3,Vector3);
impl_set_all_fxn!(Set1DAV2,Vector2);
impl_set_all_fxn!(Set1DAM4,Matrix4);    
impl_set_all_fxn!(Set1DAM3,Matrix3);
impl_set_all_fxn!(Set1DAM2,Matrix2);
impl_set_all_fxn!(Set1DAM1,Matrix1);
impl_set_all_fxn!(Set1DAM2x3,Matrix2x3);
impl_set_all_fxn!(Set1DAM3x2,Matrix3x2);

macro_rules! impl_set_all_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident, $value_string:tt);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          #[cfg(all(feature = $value_string, feature = "RowVector4"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name R4>] { sink: sink.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "RowVector3"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector3(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name R3>] { sink: sink.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "RowVector2"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector2(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name R2>] { sink: sink.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name V4>] { sink: sink.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Vector3"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector3(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name V3>] { sink: sink.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Vector2"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector2(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name V2>] { sink: sink.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M4>] { sink: sink.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3>] { sink: sink.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2>] { sink: sink.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M1>] { sink: sink.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2x3>] { sink: sink.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3x2>] { sink: sink.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MD>] { sink: sink.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "RowVectorD"))]
          (Value::[<Matrix $value_kind>](Matrix::RowDVector(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name RD>] { sink: sink.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "VectorD"))]
          (Value::[<Matrix $value_kind>](Matrix::DVector(sink)), [Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name VD>] { sink: sink.clone(), source: source.clone() })),
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_all_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set1DA, all, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetAll {}
impl NativeFunctionCompiler for MatrixSetAll {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_all_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_all_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1,1] = 1 ----------------------------------------------------------------

macro_rules! set_2d_scalar_scalar {
  ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
      ($sink).column_mut($ix2 - 1)[$ix1 - 1] = ($source).clone();
    };}

macro_rules! impl_set_scalar_scalar_fxn {
  ($struct_name:ident, $matrix_shape:ident, $op:tt) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<T>,
      ixes: (Ref<usize>,Ref<usize>),
      sink: Ref<$matrix_shape<T>>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$matrix_shape<T>>: ToValue
    {
      fn solve(&self) {
        unsafe {
          let mut sink_ptr = (&mut *(self.sink.as_ptr()));
          let source_ptr = (*(self.source.as_ptr())).clone();
          let (ix1,ix2) = &self.ixes;
          let ix1_ptr = (*(ix1.as_ptr())).clone();
          let ix2_ptr = (*(ix2.as_ptr())).clone();
          $op!(sink_ptr,ix1_ptr,ix2_ptr,source_ptr);
        }
      }
      fn out(&self) -> Value { self.sink.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }};}

impl_set_scalar_scalar_fxn!(Set2DSSMD,DMatrix,set_2d_scalar_scalar);
impl_set_scalar_scalar_fxn!(Set2DSSM4,Matrix4,set_2d_scalar_scalar);
impl_set_scalar_scalar_fxn!(Set2DSSM3,Matrix3,set_2d_scalar_scalar);
impl_set_scalar_scalar_fxn!(Set2DSSM2,Matrix2,set_2d_scalar_scalar);
impl_set_scalar_scalar_fxn!(Set2DSSM1,Matrix1,set_2d_scalar_scalar);
impl_set_scalar_scalar_fxn!(Set2DSSM2x3,Matrix2x3,set_2d_scalar_scalar);
impl_set_scalar_scalar_fxn!(Set2DSSM3x2,Matrix3x2,set_2d_scalar_scalar);

macro_rules! impl_set_scalar_scalar_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident, $value_string:tt);+ $(;)?) => {
    paste!{
      match $arg {
        $(              
          #[cfg(all(feature = $value_string, feature = "Matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::Index(ixx),Value::Index(ixy)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M4>] { sink: sink.clone(),   ixes: (ixx.clone(),ixy.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::Index(ixx),Value::Index(ixy)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3>] { sink: sink.clone(),   ixes: (ixx.clone(),ixy.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::Index(ixx),Value::Index(ixy)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2>] { sink: sink.clone(),   ixes: (ixx.clone(),ixy.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::Index(ixx),Value::Index(ixy)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M1>] { sink: sink.clone(),   ixes: (ixx.clone(),ixy.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::Index(ixx),Value::Index(ixy)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2x3>] { sink: sink.clone(), ixes: (ixx.clone(),ixy.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::Index(ixx),Value::Index(ixy)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3x2>] { sink: sink.clone(), ixes: (ixx.clone(),ixy.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::Index(ixx),Value::Index(ixy)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MD>] { sink: sink.clone(),   ixes: (ixx.clone(),ixy.clone()), source: source.clone() })),
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_scalar_scalar_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set2DSS, scalar_scalar, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetScalarScalar {}
impl NativeFunctionCompiler for MatrixSetScalarScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_scalar_scalar_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_scalar_scalar_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[:,1] = 1 -----------------------------------------------------------------

macro_rules! set_2d_all_scalar {
  ($sink:expr, $source:expr) => {
      for i in 0..$sink.nrows() {
        ($sink)[i] = ($source).clone();
      }
    };}

macro_rules! set_2d_all_vector {
  ($sink:expr, $source:expr) => {
      for i in 0..$sink.nrows() {
        ($sink)[i] = ($source)[i].clone();
      }
    };}
    
macro_rules! impl_set_all_scalar_fxn {
  ($struct_name:ident, $matrix_shape:ident, $source_type:ty,  $op:ident) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<$source_type>,
      ix: Ref<usize>,
      sink: Ref<$matrix_shape<T>>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$matrix_shape<T>>: ToValue
    {
      fn solve(&self) {
        unsafe {
          let ix_ptr = *(self.ix.as_ptr());
          let mut sink_ptr = (&mut *(self.sink.as_ptr())).column_mut(ix_ptr - 1);;
          let source_ptr = (*(self.source.as_ptr())).clone();
          $op!(sink_ptr,source_ptr);
        }
      }
      fn out(&self) -> Value { self.sink.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }};}

impl_set_all_scalar_fxn!(Set2DASMD,DMatrix, T, set_2d_all_scalar);
impl_set_all_scalar_fxn!(Set2DASM4,Matrix4, T, set_2d_all_scalar);
impl_set_all_scalar_fxn!(Set2DASM3,Matrix3, T, set_2d_all_scalar);
impl_set_all_scalar_fxn!(Set2DASM2,Matrix2, T, set_2d_all_scalar);
impl_set_all_scalar_fxn!(Set2DASM1,Matrix1, T, set_2d_all_scalar);
impl_set_all_scalar_fxn!(Set2DASM2x3,Matrix2x3, T, set_2d_all_scalar);
impl_set_all_scalar_fxn!(Set2DASM3x2,Matrix3x2, T, set_2d_all_scalar);

impl_set_all_scalar_fxn!(Set2DASM2x3V2,Matrix2x3, Vector2<T>, set_2d_all_vector);
impl_set_all_scalar_fxn!(Set2DASM3x2V3,Matrix3x2, Vector3<T>, set_2d_all_vector);

macro_rules! impl_set_all_scalar_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident, $value_string:tt);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          #[cfg(all(feature = $value_string, feature = "Matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::IndexAll, Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M4>] { sink: sink.clone(), ix: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::IndexAll, Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3>] { sink: sink.clone(), ix: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::IndexAll, Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2>] { sink: sink.clone(), ix: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::IndexAll, Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M1>] { sink: sink.clone(), ix: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::IndexAll, Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2x3>] { sink: sink.clone(), ix: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::IndexAll, Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3x2>] { sink: sink.clone(), ix: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::IndexAll, Value::Index(ix)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MD>] { sink: sink.clone(), ix: ix.clone(), source: source.clone() })),
          
          #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)),   [Value::IndexAll, Value::Index(ix)], Value::[<Matrix $value_kind>](Matrix::Vector2(source))) => Ok(Box::new([<$fxn_name M2x3V2>] { sink: sink.clone(), ix: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)),   [Value::IndexAll, Value::Index(ix)], Value::[<Matrix $value_kind>](Matrix::Vector3(source))) => Ok(Box::new([<$fxn_name M3x2V3>] { sink: sink.clone(), ix: ix.clone(), source: source.clone() })),
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_all_scalar_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set2DAS, all_scalar, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetAllScalar {}
impl NativeFunctionCompiler for MatrixSetAllScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_all_scalar_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_all_scalar_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}", x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1,:] = 1 -----------------------------------------------------------------

macro_rules! set_2d_scalar_all {
  ($sink:expr, $ix:expr, $source:expr) => {
      for i in 0..($sink).ncols() {
        ($sink).row_mut($ix - 1)[i] = ($source).clone();
      }
    };}

macro_rules! impl_set_scalar_all_fxn {
  ($struct_name:ident, $matrix_shape:ident, $op:tt) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<T>,
      ix: Ref<usize>,
      sink: Ref<$matrix_shape<T>>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$matrix_shape<T>>: ToValue
    {
      fn solve(&self) {
        unsafe {
          let ix_ptr = (*(self.ix.as_ptr())).clone();
          let mut sink_ptr = (&mut *(self.sink.as_ptr()));
          let source_ptr = (*(self.source.as_ptr())).clone();
          $op!(sink_ptr,ix_ptr,source_ptr);
        }
      }
      fn out(&self) -> Value { self.sink.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }};}

impl_set_scalar_all_fxn!(Set2DSAMD,DMatrix, set_2d_scalar_all);
impl_set_scalar_all_fxn!(Set2DSAM4,Matrix4, set_2d_scalar_all);
impl_set_scalar_all_fxn!(Set2DSAM3,Matrix3, set_2d_scalar_all);
impl_set_scalar_all_fxn!(Set2DSAM2,Matrix2, set_2d_scalar_all);
impl_set_scalar_all_fxn!(Set2DSAM1,Matrix1, set_2d_scalar_all);
impl_set_scalar_all_fxn!(Set2DSAM2x3,Matrix2x3, set_2d_scalar_all);
impl_set_scalar_all_fxn!(Set2DSAM3x2,Matrix3x2, set_2d_scalar_all);

macro_rules! impl_set_scalar_all_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident, $value_string:tt);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          #[cfg(all(feature = $value_string, feature = "Matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::Index(ix), Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M4>] { sink: sink.clone(), ix: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::Index(ix), Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3>] { sink: sink.clone(), ix: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::Index(ix), Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2>] { sink: sink.clone(), ix: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::Index(ix), Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M1>] { sink: sink.clone(), ix: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::Index(ix), Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2x3>] { sink: sink.clone(), ix: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::Index(ix), Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3x2>] { sink: sink.clone(), ix: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::Index(ix), Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MD>] { sink: sink.clone(), ix: ix.clone(), source: source.clone() })),
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_scalar_all_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set2DSA, scalar_all, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetScalarAll {}
impl NativeFunctionCompiler for MatrixSetScalarAll {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_scalar_all_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_scalar_all_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3,1] = 1 ------------------------------------------------------------------

macro_rules! set_2d_vector_scalar {
  ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
      for rix in &$ix1 {
        ($sink).row_mut(rix - 1)[$ix2 - 1] = ($source).clone();
      }
    };}

macro_rules! set_2d_vector_scalar_b {
  ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
    unsafe { 
      for rix in 0..($ix1).len() {
        if $ix1[rix] == true {
          ($sink).row_mut(rix)[$ix2 - 1] = ($source).clone();
        }
      }
    }
  };}  

macro_rules! impl_set_range_scalar_fxn {
  ($struct_name:ident, $matrix_shape:ident, $op:tt, $ix_type:ty) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<T>,
      ixes: (Ref<DVector<$ix_type>>,Ref<usize>),
      sink: Ref<$matrix_shape<T>>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$matrix_shape<T>>: ToValue
    {
      fn solve(&self) {
        unsafe { 
          let mut sink_ptr = (&mut *(self.sink.as_ptr()));
          let source_ptr = (*(self.source.as_ptr())).clone();
          let (ix1,ix2) = &self.ixes;
          let ix1_ptr = (*(ix1.as_ptr())).clone();
          let ix2_ptr = (*(ix2.as_ptr())).clone();
          $op!(sink_ptr,ix1_ptr,ix2_ptr,source_ptr);
        }
      }
      fn out(&self) -> Value { self.sink.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }};}

impl_set_range_scalar_fxn!(Set2DRSMD,DMatrix, set_2d_vector_scalar, usize);
impl_set_range_scalar_fxn!(Set2DRSM4,Matrix4, set_2d_vector_scalar, usize);
impl_set_range_scalar_fxn!(Set2DRSM3,Matrix3, set_2d_vector_scalar, usize);
impl_set_range_scalar_fxn!(Set2DRSM2,Matrix2, set_2d_vector_scalar, usize);
impl_set_range_scalar_fxn!(Set2DRSM1,Matrix1, set_2d_vector_scalar, usize);
impl_set_range_scalar_fxn!(Set2DRSM2x3,Matrix2x3, set_2d_vector_scalar, usize);
impl_set_range_scalar_fxn!(Set2DRSM3x2,Matrix3x2, set_2d_vector_scalar, usize);

impl_set_range_scalar_fxn!(Set2DRSMDB,DMatrix, set_2d_vector_scalar_b, bool);
impl_set_range_scalar_fxn!(Set2DRSM4B,Matrix4, set_2d_vector_scalar_b, bool);
impl_set_range_scalar_fxn!(Set2DRSM3B,Matrix3, set_2d_vector_scalar_b, bool);
impl_set_range_scalar_fxn!(Set2DRSM2B,Matrix2, set_2d_vector_scalar_b, bool);
impl_set_range_scalar_fxn!(Set2DRSM1B,Matrix1, set_2d_vector_scalar_b, bool);
impl_set_range_scalar_fxn!(Set2DRSM2x3B,Matrix2x3, set_2d_vector_scalar_b, bool);
impl_set_range_scalar_fxn!(Set2DRSM3x2B,Matrix3x2, set_2d_vector_scalar_b, bool);

macro_rules! impl_set_range_scalar_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident, $value_string:tt);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          #[cfg(all(feature = $value_string, feature = "Matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::MatrixIndex(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M4>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::MatrixIndex(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::MatrixIndex(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::MatrixIndex(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M1>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2x3>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3x2>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::MatrixIndex(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MD>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          
          #[cfg(all(feature = $value_string, feature = "Matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::MatrixBool(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M4B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::MatrixBool(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::MatrixBool(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::MatrixBool(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M1B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixBool(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2x3B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixBool(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3x2B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::MatrixBool(Matrix::DVector(ix1)),Value::Index(ix2)], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MDB>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
        
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_range_scalar_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set2DRS, range_scalar, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetRangeScalar {}
impl NativeFunctionCompiler for MatrixSetRangeScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_range_scalar_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_range_scalar_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1,1..3] = 1 ------------------------------------------------------------------

macro_rules! set_2d_scalar_vector {
  ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
      for rix in &$ix2 {
        ($sink).column_mut(rix - 1)[$ix1 - 1] = ($source).clone();
      }
    };}

macro_rules! set_2d_scalar_vector_b {
  ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
    unsafe { 
      for rix in 0..($ix2).len() {
        if $ix2[rix] == true {
          ($sink).row_mut(rix)[$ix1 - 1] = ($source).clone();
        }
      }
    }
  };}      

macro_rules! impl_set_scalar_range_fxn {
  ($struct_name:ident, $matrix_shape:ident, $op:tt, $ix_type:ty) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<T>,
      ixes: (Ref<usize>,Ref<DVector<$ix_type>>),
      sink: Ref<$matrix_shape<T>>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$matrix_shape<T>>: ToValue
    {
      fn solve(&self) {
        unsafe { 
          let mut sink_ptr = (&mut *(self.sink.as_ptr()));
          let source_ptr = (*(self.source.as_ptr())).clone();
          let (ix1,ix2) = &self.ixes;
          let ix1_ptr = (*(ix1.as_ptr())).clone();
          let ix2_ptr = (*(ix2.as_ptr())).clone();
          $op!(sink_ptr,ix1_ptr,ix2_ptr,source_ptr);
        }
      }
      fn out(&self) -> Value { self.sink.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }};}

impl_set_scalar_range_fxn!(Set2DSRMD,DMatrix, set_2d_scalar_vector, usize);
impl_set_scalar_range_fxn!(Set2DSRM4,Matrix4, set_2d_scalar_vector, usize);
impl_set_scalar_range_fxn!(Set2DSRM3,Matrix3, set_2d_scalar_vector, usize);
impl_set_scalar_range_fxn!(Set2DSRM2,Matrix2, set_2d_scalar_vector, usize);
impl_set_scalar_range_fxn!(Set2DSRM1,Matrix1, set_2d_scalar_vector, usize);
impl_set_scalar_range_fxn!(Set2DSRM2x3,Matrix2x3, set_2d_scalar_vector, usize);
impl_set_scalar_range_fxn!(Set2DSRM3x2,Matrix3x2, set_2d_scalar_vector, usize);

impl_set_scalar_range_fxn!(Set2DSRMDB,DMatrix, set_2d_scalar_vector_b, bool);
impl_set_scalar_range_fxn!(Set2DSRM4B,Matrix4, set_2d_scalar_vector_b, bool);
impl_set_scalar_range_fxn!(Set2DSRM3B,Matrix3, set_2d_scalar_vector_b, bool);
impl_set_scalar_range_fxn!(Set2DSRM2B,Matrix2, set_2d_scalar_vector_b, bool);
impl_set_scalar_range_fxn!(Set2DSRM1B,Matrix1, set_2d_scalar_vector_b, bool);
impl_set_scalar_range_fxn!(Set2DSRM2x3B,Matrix2x3, set_2d_scalar_vector_b, bool);
impl_set_scalar_range_fxn!(Set2DSRM3x2B,Matrix3x2, set_2d_scalar_vector_b, bool);

macro_rules! impl_set_scalar_range_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident, $value_string:tt);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          #[cfg(all(feature = $value_string, feature = "Matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::Index(ix1),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M4>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::Index(ix1),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::Index(ix1),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::Index(ix1),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M1>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::Index(ix1),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2x3>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::Index(ix1),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3x2>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::Index(ix1),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MD>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
        
          #[cfg(all(feature = $value_string, feature = "Matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::Index(ix1),Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M4B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::Index(ix1),Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::Index(ix1),Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::Index(ix1),Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M1B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::Index(ix1),Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2x3B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::Index(ix1),Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3x2B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::Index(ix1),Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MDB>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_scalar_range_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set2DSR, scalar_range, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetScalarRange {}
impl NativeFunctionCompiler for MatrixSetScalarRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_scalar_range_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_scalar_range_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3,1..3] = 1 ------------------------------------------------------------------

macro_rules! set_2d_vector_vector {
  ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
      for cix in &$ix1 {
        for rix in &$ix2 {
          ($sink).column_mut(cix - 1)[rix - 1] = ($source).clone();
        }
      }
    };}

macro_rules! set_2d_vector_vector_b {
  ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
    unsafe { 
      for cix in 0..$ix1.len() {
        for rix in 0..$ix2.len() {
          if $ix1[cix] == true && $ix2[rix] == true  {
            ($sink).row_mut(rix)[cix] = ($source).clone();
          }
        }
      }
    }
  };}  

macro_rules! impl_set_range_range_fxn {
  ($struct_name:ident, $matrix_shape:ident, $op:tt, $ix_type:ty) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<T>,
      ixes: (Ref<DVector<$ix_type>>,Ref<DVector<$ix_type>>),
      sink: Ref<$matrix_shape<T>>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$matrix_shape<T>>: ToValue
    {
      fn solve(&self) {
        unsafe { 
          let mut sink_ptr = (&mut *(self.sink.as_ptr()));
          let source_ptr = (*(self.source.as_ptr())).clone();
          let (ix1,ix2) = &self.ixes;
          let ix1_ptr = (*(ix1.as_ptr())).clone();
          let ix2_ptr = (*(ix2.as_ptr())).clone();
          $op!(sink_ptr,ix1_ptr,ix2_ptr,source_ptr);
        }
      }
      fn out(&self) -> Value { self.sink.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }};}

impl_set_range_range_fxn!(Set2DRRMD,DMatrix,set_2d_vector_vector, usize);
impl_set_range_range_fxn!(Set2DRRM4,Matrix4,set_2d_vector_vector, usize);
impl_set_range_range_fxn!(Set2DRRM3,Matrix3,set_2d_vector_vector, usize);
impl_set_range_range_fxn!(Set2DRRM2,Matrix2,set_2d_vector_vector, usize);
impl_set_range_range_fxn!(Set2DRRM1,Matrix1,set_2d_vector_vector, usize);
impl_set_range_range_fxn!(Set2DRRM2x3,Matrix2x3,set_2d_vector_vector, usize);
impl_set_range_range_fxn!(Set2DRRM3x2,Matrix3x2,set_2d_vector_vector, usize);

impl_set_range_range_fxn!(Set2DRRMDB,DMatrix,set_2d_vector_vector_b, bool);
impl_set_range_range_fxn!(Set2DRRM4B,Matrix4,set_2d_vector_vector_b, bool);
impl_set_range_range_fxn!(Set2DRRM3B,Matrix3,set_2d_vector_vector_b, bool);
impl_set_range_range_fxn!(Set2DRRM2B,Matrix2,set_2d_vector_vector_b, bool);
impl_set_range_range_fxn!(Set2DRRM1B,Matrix1,set_2d_vector_vector_b, bool);
impl_set_range_range_fxn!(Set2DRRM2x3B,Matrix2x3,set_2d_vector_vector_b, bool);
impl_set_range_range_fxn!(Set2DRRM3x2B,Matrix3x2,set_2d_vector_vector_b, bool);

macro_rules! impl_set_range_range_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident, $value_string:tt);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          #[cfg(all(feature = $value_string, feature = "Matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::MatrixIndex(Matrix::DVector(ix1)),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M4>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::MatrixIndex(Matrix::DVector(ix1)),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::MatrixIndex(Matrix::DVector(ix1)),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::MatrixIndex(Matrix::DVector(ix1)),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M1>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2x3>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3x2>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::MatrixIndex(Matrix::DVector(ix1)),Value::MatrixIndex(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MD>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
        
          #[cfg(all(feature = $value_string, feature = "Matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::MatrixBool(Matrix::DVector(ix1)),Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M4B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::MatrixBool(Matrix::DVector(ix1)),Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::MatrixBool(Matrix::DVector(ix1)),Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::MatrixBool(Matrix::DVector(ix1)),Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M1B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixBool(Matrix::DVector(ix1)),Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2x3B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixBool(Matrix::DVector(ix1)),Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3x2B>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::MatrixBool(Matrix::DVector(ix1)),Value::MatrixBool(Matrix::DVector(ix2))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MDB>] { sink: sink.clone(), ixes: (ix1.clone(), ix2.clone()), source: source.clone() })),
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_range_range_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set2DRR, range_range, (sink, ixes.as_slice(), source))
}
pub struct MatrixSetRangeRange {}
impl NativeFunctionCompiler for MatrixSetRangeRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_range_range_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_range_range_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[:,1..3] = 1 ------------------------------------------------------------------

macro_rules! set_2d_all_vector {
  ($sink:expr, $ix:expr, $source:expr) => {
      for cix in &$ix {
        for rix in 0..($sink).nrows() {
          ($sink).column_mut(cix - 1)[rix] = ($source).clone();
        }
      }
    };}

macro_rules! set_2d_all_vector_b {
  ($sink:expr, $ix:expr, $source:expr) => {
      for cix in 0..$ix.len() {
        for rix in 0..($sink).nrows() {
          if $ix[cix] == true {
            ($sink).column_mut(cix)[rix] = ($source).clone();
          }
        }
      }
    };}    

macro_rules! impl_set_all_range_fxn {  
  ($struct_name:ident, $matrix_shape:ident, $op:tt, $ix_type:ty) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<T>,
      ixes: Ref<DVector<$ix_type>>,
      sink: Ref<$matrix_shape<T>>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$matrix_shape<T>>: ToValue
    {
      fn solve(&self) {
        unsafe { 
          let ix_ptr = (*(self.ixes.as_ptr())).clone();
          let mut sink_ptr = (&mut *(self.sink.as_ptr()));
          let source_ptr = (*(self.source.as_ptr())).clone();
          $op!(sink_ptr,ix_ptr,source_ptr);
        }
      }
      fn out(&self) -> Value { self.sink.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }};}

impl_set_all_range_fxn!(Set2DARMD,DMatrix, set_2d_all_vector, usize);
impl_set_all_range_fxn!(Set2DARM4,Matrix4, set_2d_all_vector, usize);
impl_set_all_range_fxn!(Set2DARM3,Matrix3, set_2d_all_vector, usize);
impl_set_all_range_fxn!(Set2DARM2,Matrix2, set_2d_all_vector, usize);
impl_set_all_range_fxn!(Set2DARM1,Matrix1, set_2d_all_vector, usize);
impl_set_all_range_fxn!(Set2DARM2x3,Matrix2x3, set_2d_all_vector, usize);
impl_set_all_range_fxn!(Set2DARM3x2,Matrix3x2, set_2d_all_vector, usize);

impl_set_all_range_fxn!(Set2DARMDB,DMatrix, set_2d_all_vector_b, bool);
impl_set_all_range_fxn!(Set2DARM4B,Matrix4, set_2d_all_vector_b, bool);
impl_set_all_range_fxn!(Set2DARM3B,Matrix3, set_2d_all_vector_b, bool);
impl_set_all_range_fxn!(Set2DARM2B,Matrix2, set_2d_all_vector_b, bool);
impl_set_all_range_fxn!(Set2DARM1B,Matrix1, set_2d_all_vector_b, bool);
impl_set_all_range_fxn!(Set2DARM2x3B,Matrix2x3, set_2d_all_vector_b, bool);
impl_set_all_range_fxn!(Set2DARM3x2B,Matrix3x2, set_2d_all_vector_b, bool);

macro_rules! impl_set_all_range_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident, $value_string:tt);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          #[cfg(all(feature = $value_string, feature = "Matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::IndexAll,Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M4>] { sink: sink.clone(), ixes:   ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::IndexAll,Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3>] { sink: sink.clone(), ixes:   ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::IndexAll,Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2>] { sink: sink.clone(), ixes:   ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::IndexAll,Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M1>] { sink: sink.clone(), ixes:   ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::IndexAll,Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2x3>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::IndexAll,Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3x2>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::IndexAll,Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MD>] { sink: sink.clone(), ixes:   ix.clone(), source: source.clone() })),

          #[cfg(all(feature = $value_string, feature = "Matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::IndexAll,Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M4B>] { sink: sink.clone(), ixes:   ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::IndexAll,Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3B>] { sink: sink.clone(), ixes:   ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::IndexAll,Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2B>] { sink: sink.clone(), ixes:   ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::IndexAll,Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M1B>] { sink: sink.clone(), ixes:   ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::IndexAll,Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2x3B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::IndexAll,Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3x2B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::IndexAll,Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MDB>] { sink: sink.clone(), ixes:   ix.clone(), source: source.clone() })),
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_all_range_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_match_arms!(Set2DAR, all_range, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetAllRange {}
impl NativeFunctionCompiler for MatrixSetAllRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_all_range_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match sink {
          Value::MutableReference(sink) => { impl_set_all_range_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) }
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// x[1..3,:] = 1 ------------------------------------------------------------------

macro_rules! set_2d_vector_all {
  ($source:expr, $ix:expr, $sink:expr) => {
      for cix in 0..($sink).ncols() {
        for rix in &$ix {
          ($sink).column_mut(cix)[rix - 1] = ($source).clone();
        }
      }
    };}

macro_rules! set_2d_vector_all_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    for cix in 0..($sink).ncols() {
      for rix in 0..$ix.len() {
        if $ix[rix] == true {
          ($sink).column_mut(cix)[rix] = ($source).clone();
        }
      }
    }
  };} 

macro_rules! set_2d_vector_all_mat {
  ($source:expr, $ix:expr, $sink:expr) => {
    for (i,rix) in (&$ix).iter().enumerate() {
      let mut row = ($sink).row_mut(rix - 1);
      row.copy_from(&($source).row(i));
    }
  };}
  
macro_rules! set_2d_vector_all_mat_b {
  ($source:expr, $ix:expr, $sink:expr) => {
    for (i,rix) in (&$ix).iter().enumerate() {
      if *rix == true {
        let mut row = ($sink).row_mut(i);
        row.copy_from(&($source).row(i));
      }
    }
  };}   

impl_set_fxn!(Set2DRAMD,DMatrix,T,set_2d_vector_all,usize);
impl_set_fxn!(Set2DRAM4,Matrix4,T,set_2d_vector_all,usize);
impl_set_fxn!(Set2DRAM3,Matrix3,T,set_2d_vector_all,usize);
impl_set_fxn!(Set2DRAM2,Matrix2,T,set_2d_vector_all,usize);
impl_set_fxn!(Set2DRAM1,Matrix1,T,set_2d_vector_all,usize);
impl_set_fxn!(Set2DRAM2x3,Matrix2x3,T,set_2d_vector_all,usize);
impl_set_fxn!(Set2DRAM3x2,Matrix3x2,T,set_2d_vector_all,usize);

impl_set_fxn!(Set2DRAMDMD,DMatrix,DMatrix<T>,set_2d_vector_all_mat,usize);
impl_set_fxn!(Set2DRAMDM2,DMatrix,Matrix2<T>,set_2d_vector_all_mat,usize);
impl_set_fxn!(Set2DRAMDM2x3,DMatrix,Matrix2x3<T>,set_2d_vector_all_mat,usize);
impl_set_fxn!(Set2DRAMDM3,DMatrix,Matrix3<T>,set_2d_vector_all_mat,usize);
impl_set_fxn!(Set2DRAMDM3x2,DMatrix,Matrix3x2<T>,set_2d_vector_all_mat,usize);
impl_set_fxn!(Set2DRAMDM4,DMatrix,Matrix4<T>,set_2d_vector_all_mat,usize);

impl_set_fxn!(Set2DRAM2M2,Matrix2,Matrix2<T>,set_2d_vector_all_mat,usize);
impl_set_fxn!(Set2DRAM2M3x2,Matrix2,Matrix3x2<T>,set_2d_vector_all_mat,usize);
impl_set_fxn!(Set2DRAM2MD,Matrix2,DMatrix<T>,set_2d_vector_all_mat,usize);

impl_set_fxn!(Set2DRAM3M3,Matrix3,Matrix3<T>,set_2d_vector_all_mat,usize);
impl_set_fxn!(Set2DRAM3M2x3,Matrix3,Matrix2x3<T>,set_2d_vector_all_mat,usize);
impl_set_fxn!(Set2DRAM3MD,Matrix3,DMatrix<T>,set_2d_vector_all_mat,usize);

impl_set_fxn!(Set2DRAM3x2M3x2,Matrix3x2,Matrix3x2<T>,set_2d_vector_all_mat,usize);
impl_set_fxn!(Set2DRAM3x2M2,Matrix3x2,Matrix2<T>,set_2d_vector_all_mat,usize);
impl_set_fxn!(Set2DRAM3x2MD,Matrix3x2,DMatrix<T>,set_2d_vector_all_mat,usize);

impl_set_fxn!(Set2DRAM2x3M2x3,Matrix2x3,Matrix2x3<T>,set_2d_vector_all_mat,usize);
impl_set_fxn!(Set2DRAM2x3M3,Matrix2x3,Matrix3<T>,set_2d_vector_all_mat,usize);
impl_set_fxn!(Set2DRAM2x3MD,Matrix2x3,DMatrix<T>,set_2d_vector_all_mat,usize);

impl_set_fxn!(Set2DRAM4M4,Matrix4,Matrix4<T>,set_2d_vector_all_mat,usize);
impl_set_fxn!(Set2DRAM4MD,Matrix4,DMatrix<T>,set_2d_vector_all_mat,usize);

impl_set_fxn!(Set2DRAMDB,DMatrix,T,set_2d_vector_all_b,bool);
impl_set_fxn!(Set2DRAM4B,Matrix4,T,set_2d_vector_all_b,bool);
impl_set_fxn!(Set2DRAM3B,Matrix3,T,set_2d_vector_all_b,bool);
impl_set_fxn!(Set2DRAM2B,Matrix2,T,set_2d_vector_all_b,bool);
impl_set_fxn!(Set2DRAM1B,Matrix1,T,set_2d_vector_all_b,bool);
impl_set_fxn!(Set2DRAM2x3B,Matrix2x3,T,set_2d_vector_all_b,bool);
impl_set_fxn!(Set2DRAM3x2B,Matrix3x2,T,set_2d_vector_all_b,bool);

impl_set_fxn!(Set2DRAMDMDB,DMatrix,DMatrix<T>,set_2d_vector_all_mat_b,bool);
impl_set_fxn!(Set2DRAM2M2B,Matrix2,Matrix2<T>,set_2d_vector_all_mat_b,bool);
impl_set_fxn!(Set2DRAM3M3B,Matrix3,Matrix3<T>,set_2d_vector_all_mat_b,bool);
impl_set_fxn!(Set2DRAM4M4B,Matrix4,Matrix4<T>,set_2d_vector_all_mat_b,bool);
impl_set_fxn!(Set2DRAM3x2M3x2B,Matrix3x2,Matrix3x2<T>,set_2d_vector_all_mat_b,bool);
impl_set_fxn!(Set2DRAM2x3M2x3B,Matrix2x3,Matrix2x3<T>,set_2d_vector_all_mat_b,bool);

#[macro_export]
macro_rules! impl_set_range_all_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident, $value_string:tt);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          // Vector Scalar
          #[cfg(all(feature = $value_string, feature = "Matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M4>] { sink: sink.clone(), ixes:   ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3>] { sink: sink.clone(), ixes:   ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2>] { sink: sink.clone(), ixes:   ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M1>] { sink: sink.clone(), ixes:   ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2x3>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3x2>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MD>] { sink: sink.clone(), ixes:   ix.clone(), source: source.clone() })),
          
          // Vector Vector
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new($fxn_name{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "MatrixD", feature = "Matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix2(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new($fxn_name{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),

          #[cfg(all(feature = $value_string, feature = "MatrixD", feature = "Matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix2x3(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new($fxn_name{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "MatrixD", feature = "Matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix3(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new($fxn_name{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "MatrixD", feature = "Matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix3x2(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new($fxn_name{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "MatrixD", feature = "Matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix4(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new($fxn_name{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),

          #[cfg(all(feature = $value_string, feature = "Matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix2(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new($fxn_name{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2", feature = "Matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix3x2(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new($fxn_name{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2", feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new($fxn_name{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),

          #[cfg(all(feature = $value_string, feature = "Matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix3(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new($fxn_name{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3", feature = "Matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix2x3(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new($fxn_name{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3", feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new($fxn_name{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),

          #[cfg(all(feature = $value_string, feature = "Matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix4(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new($fxn_name{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "Matrix4", feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new($fxn_name{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),

          #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix2x3(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new($fxn_name{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2x3", feature = "Matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix3(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new($fxn_name{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2x3", feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new($fxn_name{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),

          #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix3x2(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new($fxn_name{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3x2", feature = "Matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix2(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new($fxn_name{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3x2", feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new($fxn_name{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),

          // Matrix Scalar Bool
          #[cfg(all(feature = $value_string, feature = "Matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M4B>] { sink: sink.clone(), ixes:   ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3B>] { sink: sink.clone(), ixes:   ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2B>] { sink: sink.clone(), ixes:   ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M1B>] { sink: sink.clone(), ixes:   ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M2x3B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name M3x2B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name MDB>] { sink: sink.clone(), ixes:   ix.clone(), source: source.clone() })),
        
          // Matrix Vector Bool
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name MDMDB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)), [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix2(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name M2M2B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)), [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix3(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name M3M3B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)), [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix4(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name M4M4B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix2x3(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name M2x3M2x3B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
          #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix3x2(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name M3x2M3x2B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone() })),
        )+
        x => {
          println!("{:#?}", x);
          Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind })
        }
      }
    }
  }
}

fn impl_set_range_all_fxn(sink: Value, source: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  todo!();
  //impl_set_match_arms!(Set2DRA, range_all, (sink, ixes.as_slice(), source))
}

pub struct MatrixSetRangeAll {}
impl NativeFunctionCompiler for MatrixSetRangeAll {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink: Value = arguments[0].clone();
    let source: Value = arguments[1].clone();
    let ixes = arguments.clone().split_off(2);
    match impl_set_range_all_fxn(sink.clone(),source.clone(),ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (sink,ixes,source) {
          (Value::MutableReference(sink),ixes,Value::MutableReference(source)) => { impl_set_range_all_fxn(sink.borrow().clone(),source.borrow().clone(),ixes.clone()) },
          (sink,ixes,Value::MutableReference(source)) => { impl_set_range_all_fxn(sink.clone(),source.borrow().clone(),ixes.clone()) },
          (Value::MutableReference(sink),ixes,source) => { impl_set_range_all_fxn(sink.borrow().clone(),source.clone(),ixes.clone()) },
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}