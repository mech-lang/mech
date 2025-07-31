#[macro_use]
use crate::stdlib::*;

// Convert --------------------------------------------------------------------

#[derive(Debug)]
struct ConvertSEnum {
  out: Value,
}
impl MechFunction for ConvertSEnum
{
  fn solve(&self) { }
  fn out(&self) -> Value { self.out.clone() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[derive(Debug)]
struct ConvertMatrixBasic<F, T> {
  arg: Matrix<F>,
  out: Matrix<T>,
}

impl<F, T> MechFunction for ConvertMatrixBasic<F, T>
where
  Matrix<T>: ToValue,
  T: LossyFrom<F> + Clone + Debug + PartialEq + 'static,
  F: Clone + Debug + PartialEq + 'static,
{
  fn solve(&self) {
  let arg_vec = self.arg.as_vec();
  self.out.set(arg_vec.iter().cloned().map(T::lossy_from).collect::<Vec<T>>());
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String {
    format!("{:#?}", self)
  }
}

#[derive(Debug)]
struct ConvertMat2Table<T> {
  arg: Matrix<T>,
  out: Ref<MechTable>,
}
impl<T> MechFunction for ConvertMat2Table<T>
where T: Debug + Clone + PartialEq + Into<Value> + 'static,
{
  fn solve(&self) {
    let arg = &self.arg;
    let mut out_table = self.out.borrow_mut();
    let (rows, cols) = (arg.rows(), arg.cols());

    for (col_ix, (ix, (col_kind, out_col))) in out_table.data.iter_mut().enumerate() {
      for row_ix in 0..rows {
        let value = arg.index2d(row_ix + 1, col_ix + 1).clone().into();
        let converted_value = value.convert_to(col_kind).unwrap();
        out_col.set_index1d(row_ix, converted_value);
      }
    }
  }
  fn out(&self) -> Value { Value::Table(self.out.clone()) }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[derive(Debug)]
struct ConvertSRationalToF64 {
  arg: Ref<RationalNumber>,
  out: Ref<F64>,
}

impl MechFunction for ConvertSRationalToF64 {
  fn solve(&self) {
    let arg_ptr = self.arg.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe{ *out_ptr = (*arg_ptr).into(); }
  }
  fn out(&self) -> Value { Value::F64(self.out.clone()) }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

macro_rules! impl_conversion_match_arms {
  ($arg:expr, $($input_type:ident => $($target_type:ident),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          (Value::[<Matrix $input_type:camel>](mat), Value::Kind(ValueKind::Table(tbl, sze))) => {
            let in_shape = mat.shape();
            let tbl_cols = tbl.len();
            let mat_knd = ValueKind::[<$input_type:camel>];
            // Verify the table has the correct number of columns
            if in_shape[1] != tbl_cols {
              return Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("Matrix has {} columns, but table expects {}", in_shape[1], tbl_cols), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
            }
            // Verify each column of the matrix can be converted to the target type of the table
            for (_, knd) in &tbl {
              if *knd == mat_knd {
                continue;
              } else if mat_knd.is_convertible_to(knd) {
                continue;
              } else {
                return Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("Matrix column type {} does not match table column type {}", mat_knd, knd), id: line!(), kind: MechErrorKind::None});
              }
            }
            // Create a blank table, with as many rows as the matrix has
            let out = MechTable::from_kind(ValueKind::Table(tbl.clone(), in_shape[0]))?;
            Ok(Box::new(ConvertMat2Table::<$input_type>{arg: mat.clone(), out: new_ref(out)}))
          }
          $(
            (Value::[<$input_type:camel>](arg), Value::Kind(ValueKind::[<$target_type:camel>])) => {Ok(Box::new(ConvertScalarToScalarBasic::<$input_type,$target_type>{arg: arg.clone(), out: new_ref($target_type::zero())}))},
            (Value::[<Matrix $input_type:camel>](arg), Value::Kind(ValueKind::Matrix(kind,size))) => {
              match *kind {
                ValueKind::U8 => {let in_shape = arg.shape();let out = u8::to_matrix(vec![0; in_shape[0]*in_shape[1]], in_shape[0], in_shape[1]);Ok(Box::new(ConvertMatrixBasic::<$input_type,u8>{arg: arg.clone(), out}))}
                ValueKind::U16 => {let in_shape = arg.shape();let out = u16::to_matrix(vec![0; in_shape[0]*in_shape[1]], in_shape[0], in_shape[1]);Ok(Box::new(ConvertMatrixBasic::<$input_type,u16>{arg: arg.clone(), out}))}
                ValueKind::U32 => {let in_shape = arg.shape();let out = u32::to_matrix(vec![0; in_shape[0]*in_shape[1]], in_shape[0], in_shape[1]);Ok(Box::new(ConvertMatrixBasic::<$input_type,u32>{arg: arg.clone(), out}))}
                ValueKind::U64 => {let in_shape = arg.shape();let out = u64::to_matrix(vec![0; in_shape[0]*in_shape[1]], in_shape[0], in_shape[1]);Ok(Box::new(ConvertMatrixBasic::<$input_type,u64>{arg: arg.clone(), out}))}
                ValueKind::U128 => {let in_shape = arg.shape();let out = u128::to_matrix(vec![0; in_shape[0]*in_shape[1]], in_shape[0], in_shape[1]);Ok(Box::new(ConvertMatrixBasic::<$input_type,u128>{arg: arg.clone(), out}))}
                ValueKind::I8 =>  {let in_shape = arg.shape();let out = i8::to_matrix(vec![0; in_shape[0]*in_shape[1]], in_shape[0],  in_shape[1]);Ok(Box::new(ConvertMatrixBasic::<$input_type,i8>{arg: arg.clone(), out}))}
                ValueKind::I16 => {let in_shape = arg.shape();let out = i16::to_matrix(vec![0; in_shape[0]*in_shape[1]], in_shape[0], in_shape[1]);Ok(Box::new(ConvertMatrixBasic::<$input_type,i16>{arg: arg.clone(), out}))}
                ValueKind::I32 => {let in_shape = arg.shape();let out = i32::to_matrix(vec![0; in_shape[0]*in_shape[1]], in_shape[0], in_shape[1]);Ok(Box::new(ConvertMatrixBasic::<$input_type,i32>{arg: arg.clone(), out}))}
                ValueKind::I64 => {let in_shape = arg.shape();let out = i64::to_matrix(vec![0; in_shape[0]*in_shape[1]], in_shape[0], in_shape[1]);Ok(Box::new(ConvertMatrixBasic::<$input_type,i64>{arg: arg.clone(), out}))}
                ValueKind::I128 => {let in_shape = arg.shape();let out = i128::to_matrix(vec![0; in_shape[0]*in_shape[1]], in_shape[0], in_shape[1]);Ok(Box::new(ConvertMatrixBasic::<$input_type,i128>{arg: arg.clone(), out}))}
                ValueKind::F32 => {let in_shape = arg.shape();let out = F32::to_matrix(vec![F32::zero(); in_shape[0]*in_shape[1]], in_shape[0], in_shape[1]);Ok(Box::new(ConvertMatrixBasic::<$input_type,F32>{arg: arg.clone(), out}))}
                ValueKind::F64 => {let in_shape = arg.shape();let out = F64::to_matrix(vec![F64::zero(); in_shape[0]*in_shape[1]], in_shape[0], in_shape[1]);Ok(Box::new(ConvertMatrixBasic::<$input_type,F64>{arg: arg.clone(), out}))}
                _ => todo!(),
              }
            },
          )+
        )+
        (Value::RationalNumber(ref rat), Value::Kind(ValueKind::F64)) => {
          Ok(Box::new(ConvertSRationalToF64{arg: rat.clone(), out: new_ref(F64::zero())}))
        }
        (Value::Atom(varian_id), Value::Kind(ValueKind::Enum(enum_id))) => {
          let variants = vec![(varian_id,None)];
          let enm = MechEnum{id: enum_id, variants};
          let val = Value::Enum(Box::new(enm.clone()));
          Ok(Box::new(ConvertSEnum{out: val}))
        }
        x => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
      }
    };
  }
}

#[derive(Debug)]
pub struct ConvertScalarToScalar<F, T> {
    pub arg: Ref<F>,
    pub out: Ref<T>,
}

impl<F, T> MechFunction for ConvertScalarToScalar<F, T>
where
    Ref<T>: ToValue,
    F: LosslessInto<T> + Debug + Clone,
    T: Debug,
{
  fn solve(&self) {
    let arg_ptr = self.arg.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {
      let out_ref: &mut T = &mut *out_ptr;
      let arg_ref: &F = &*arg_ptr;
      *out_ref = arg_ref.clone().lossless_into();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

pub trait LossyFrom<T> {
    fn lossy_from(value: T) -> Self;
}

macro_rules! impl_lossy_from {
  ($($from:ty => $($to:ty),*);* $(;)?) => {
    $(
      $(
        impl LossyFrom<$from> for $to {
          fn lossy_from(value: $from) -> Self {
              value as $to
          }
        }
      )*
    )*
  };
}

impl_lossy_from!(u8 => u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
impl_lossy_from!(u16 => u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
impl_lossy_from!(u32 => u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
impl_lossy_from!(u64 => u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
impl_lossy_from!(i8 => u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
impl_lossy_from!(i16 => u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
impl_lossy_from!(i32 => u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
impl_lossy_from!(i64 => u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
impl_lossy_from!(i128 => u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
impl_lossy_from!(u128 => u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

macro_rules! impl_lossy_from_wrapper {
  ($wrapper:ident, $inner:ty, $($prim:ty),*) => {
    $(
      impl LossyFrom<$wrapper> for $prim {
        fn lossy_from(value: $wrapper) -> Self {
          value.0 as $prim
        }
      }
      impl LossyFrom<$prim> for $wrapper {
        fn lossy_from(value: $prim) -> Self {
          $wrapper(value as $inner)
        }
      }
    )*
  };
}

impl_lossy_from_wrapper!(F64, f64, u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);
impl_lossy_from_wrapper!(F32, f32, u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);

impl LossyFrom<F64> for F32 {
  fn lossy_from(value: F64) -> Self {
    F32(value.0 as f32)
  }
}

impl LossyFrom<F32> for F64 {
  fn lossy_from(value: F32) -> Self {
    F64(value.0 as f64)
  }
}

impl LossyFrom<F64> for F64 {
  fn lossy_from(value: F64) -> Self {
    F64(value.0)
  }
}

impl LossyFrom<F32> for F32 {
  fn lossy_from(value: F32) -> Self {
    F32(value.0)
  }
}

impl LossyFrom<F64> for RationalNumber {
  fn lossy_from(value: F64) -> Self {
    RationalNumber::from(value)
  }
}

#[derive(Debug)]
pub struct ConvertScalarToScalarBasic<F, T> {
  pub arg: Ref<F>,
  pub out: Ref<T>,
}

impl<F, T> MechFunction for ConvertScalarToScalarBasic<F, T>
where
  Ref<T>: ToValue,
  F: Debug + Clone,
  T: Debug + LossyFrom<F>,
{
  fn solve(&self) {
    let arg_ptr = self.arg.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {
      let out_ref: &mut T = &mut *out_ptr;
      let arg_ref: &F = &*arg_ptr;
      *out_ref = T::lossy_from(arg_ref.clone());
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

fn impl_conversion_fxn(source_value: Value, target_kind: Value) -> MResult<Box<dyn MechFunction>>  {
  match (&source_value, &target_kind) {
    (Value::RationalNumber(r), Value::Kind(ValueKind::F64)) => {return Ok(Box::new(ConvertScalarToScalar::<RationalNumber, F64>{arg: r.clone(),out: new_ref(F64::zero()),}));}
    (Value::RationalNumber(r), Value::Kind(ValueKind::String)) => {return Ok(Box::new(ConvertScalarToScalar::<RationalNumber, String>{arg: r.clone(),out: new_ref(String::default()),}));}
    (Value::MatrixString(ref mat), Value::Kind(ValueKind::Table(tbl, sze))) => {
      let in_shape = mat.shape();
      // Verify the table has the correct number of columns
      if in_shape[1] != tbl.len() {
        return Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("Matrix has {} columns, but table expects {}", in_shape[1], tbl.len()), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
      }
      // Create a blank table, with as many rows as the matrix has
      let out = MechTable::from_kind(ValueKind::Table(tbl.clone(), in_shape[0]))?;
      return Ok(Box::new(ConvertMat2Table::<String>{arg: mat.clone(), out: new_ref(out)}));
    }
    (Value::MatrixBool(ref mat), Value::Kind(ValueKind::Table(tbl, sze))) => {
      let in_shape = mat.shape();
      // Verify the table has the correct number of columns
      if in_shape[1] != tbl.len() {
        return Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("Matrix has {} columns, but table expects {}", in_shape[1], tbl.len()), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
      }
      // Create a blank table, with as many rows as the matrix has
      let out = MechTable::from_kind(ValueKind::Table(tbl.clone(), in_shape[0]))?;
      return Ok(Box::new(ConvertMat2Table::<bool>{arg: mat.clone(), out: new_ref(out)}));
    }
    _ =>(),
  }
  impl_conversion_match_arms!(
    (source_value, target_kind),
    i8   => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    i16  => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    i32  => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    i64  => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    i128 => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    u8   => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    u16  => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    u32  => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    u64  => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    u128 => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    F32  => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    F64  => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64, RationalNumber;
  )
}

pub struct ConvertKind {}

impl NativeFunctionCompiler for ConvertKind {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let source_value = arguments[0].clone();
    let target_kind = arguments[1].clone();
    match impl_conversion_fxn(source_value.clone(), target_kind.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match source_value {
          Value::MutableReference(rhs) => impl_conversion_fxn(rhs.borrow().clone(), target_kind.clone()),
          Value::Atom(atom_id) => impl_conversion_fxn(source_value, target_kind.clone()),
          Value::MatrixU8(ref mat) => impl_conversion_fxn(source_value, target_kind.clone()),
          Value::MatrixU16(ref mat) => impl_conversion_fxn(source_value, target_kind.clone()),
          Value::MatrixU32(ref mat) => impl_conversion_fxn(source_value, target_kind.clone()),
          Value::MatrixU64(ref mat) => impl_conversion_fxn(source_value, target_kind.clone()),
          Value::MatrixU128(ref mat) => impl_conversion_fxn(source_value, target_kind.clone()),
          Value::MatrixI8(ref mat) => impl_conversion_fxn(source_value, target_kind.clone()),
          Value::MatrixI16(ref mat) => impl_conversion_fxn(source_value, target_kind.clone()),
          Value::MatrixI32(ref mat) => impl_conversion_fxn(source_value, target_kind.clone()),  
          Value::MatrixI64(ref mat) => impl_conversion_fxn(source_value, target_kind.clone()),
          Value::MatrixI128(ref mat) => impl_conversion_fxn(source_value, target_kind.clone()),
          Value::MatrixF32(ref mat) => impl_conversion_fxn(source_value, target_kind.clone()),
          Value::MatrixF64(ref mat) => impl_conversion_fxn(source_value, target_kind.clone()),
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}