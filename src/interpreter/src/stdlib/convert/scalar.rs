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

#[cfg(all(feature = "matrix", feature = "table"))]
#[derive(Debug)]
struct ConvertMat2Table<T> {
  arg: Matrix<T>,
  out: Ref<MechTable>,
}

#[cfg(all(feature = "matrix", feature = "table"))]
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

#[cfg(all(feature = "rational", feature = "f64"))]
#[derive(Debug)]
struct ConvertSRationalToF64 {
  arg: Ref<RationalNumber>,
  out: Ref<F64>,
}

#[cfg(all(feature = "rational", feature = "f64"))]
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
  ($arg:expr, $($input_type:ident, $input_type_string:tt => $($target_type:ident, $target_type_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          #[cfg(all(feature = "matrix", feature = "table"))]
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
            #[cfg(all(feature = $input_type_string, feature = $target_type_string))]
            (Value::[<$input_type:camel>](arg), Value::Kind(ValueKind::[<$target_type:camel>])) => {Ok(Box::new(ConvertScalarToScalarBasic{arg: arg.clone(), out: new_ref($target_type::default())}))},
          )+
        )+
        #[cfg(feature = "rational")]
        (Value::RationalNumber(ref rat), Value::Kind(ValueKind::F64)) => {
          Ok(Box::new(ConvertSRationalToF64{arg: rat.clone(), out: new_ref(F64::zero())}))
        }
        #[cfg(all(feature = "atom", feature = "enum"))]
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
    #[cfg(feature = "rational")]
    (Value::RationalNumber(r), Value::Kind(ValueKind::F64)) => {return Ok(Box::new(ConvertScalarToScalar{arg: r.clone(),out: new_ref(F64::zero()),}));}
    #[cfg(all(feature = "matrix", feature = "table"))]
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
    #[cfg(all(feature = "matrix", feature = "table", feature = "bool"))]
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
    i8, "i8" => String, "string", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", F32, "f32", F64, "f64";
    i16, "i16" => String, "string", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", F32, "f32", F64, "f64";
    i32, "i32" => String, "string", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", F32, "f32", F64, "f64";
    i64, "i64" => String, "string", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", F32, "f32", F64, "f64";
    i128, "i128" => String, "string", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", F32, "f32", F64, "f64";
    u8, "u8" => String, "string", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", F32, "f32", F64, "f64";
    u16, "u16" => String, "string", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", F32, "f32", F64, "f64";
    u32, "u32" => String, "string", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", F32, "f32", F64, "f64";
    u64, "u64" => String, "string", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", F32, "f32", F64, "f64";
    u128, "u128" => String, "string", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", F32, "f32", F64, "f64";
    F32, "f32" => String, "string", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", F32, "f32", F64, "f64";
    F64, "f64" => String, "string", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", F32, "f32", F64, "f64", RationalNumber, "rational";
    RationalNumber, "rational" => String, "string", F64, "f64";
    String, "string" => String, "string";
    bool, "bool" => String, "string", bool, "bool";
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
          #[cfg(feature = "atom")]
          Value::Atom(atom_id) => impl_conversion_fxn(source_value, target_kind.clone()),
          #[cfg(all(feature = "matrix", feature = "u8"))]
          Value::MatrixU8(ref mat) => impl_conversion_fxn(source_value, target_kind.clone()),
          #[cfg(all(feature = "matrix", feature = "u16"))]
          Value::MatrixU16(ref mat) => impl_conversion_fxn(source_value, target_kind.clone()),
          #[cfg(all(feature = "matrix", feature = "u32"))]
          Value::MatrixU32(ref mat) => impl_conversion_fxn(source_value, target_kind.clone()),
          #[cfg(all(feature = "matrix", feature = "u64"))]
          Value::MatrixU64(ref mat) => impl_conversion_fxn(source_value, target_kind.clone()),
          #[cfg(all(feature = "matrix", feature = "u128"))]
          Value::MatrixU128(ref mat) => impl_conversion_fxn(source_value, target_kind.clone()),
          #[cfg(all(feature = "matrix", feature = "i8"))]
          Value::MatrixI8(ref mat) => impl_conversion_fxn(source_value, target_kind.clone()),
          #[cfg(all(feature = "matrix", feature = "i16"))]
          Value::MatrixI16(ref mat) => impl_conversion_fxn(source_value, target_kind.clone()),
          #[cfg(all(feature = "matrix", feature = "i32"))]
          Value::MatrixI32(ref mat) => impl_conversion_fxn(source_value, target_kind.clone()),
          #[cfg(all(feature = "matrix", feature = "i64"))]
          Value::MatrixI64(ref mat) => impl_conversion_fxn(source_value, target_kind.clone()),
          #[cfg(all(feature = "matrix", feature = "i128"))]
          Value::MatrixI128(ref mat) => impl_conversion_fxn(source_value, target_kind.clone()),
          #[cfg(all(feature = "matrix", feature = "f32"))]
          Value::MatrixF32(ref mat) => impl_conversion_fxn(source_value, target_kind.clone()),
          #[cfg(all(feature = "matrix", feature = "f64"))]
          Value::MatrixF64(ref mat) => impl_conversion_fxn(source_value, target_kind.clone()),
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}