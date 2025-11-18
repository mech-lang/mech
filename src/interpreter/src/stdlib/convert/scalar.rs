#[macro_use]
use crate::stdlib::*;

// Convert --------------------------------------------------------------------

#[cfg(feature = "enum")]
#[derive(Debug)]
struct ConvertSEnum {
  out: Ref<MechEnum>,
}
#[cfg(feature = "enum")]
impl MechFunctionFactory for ConvertSEnum {
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Unary(out, _) => {
        let out: Ref<MechEnum> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(Self {out}))
      },
      _ => Err(MechError2::new(
          IncorrectNumberOfArguments { expected: 1, found: args.len() },
          None
        ).with_compiler_loc()
      ),
    }
  }
}
#[cfg(feature = "enum")]
impl MechFunctionImpl for ConvertSEnum
{
  fn solve(&self) { }
  fn out(&self) -> Value { Value::Enum(self.out.clone()) }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(all(feature = "compiler", feature = "enum"))]
impl MechFunctionCompiler for ConvertSEnum {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("ConvertSEnum<enum>");
    compile_nullop!(name, self.out, ctx, FeatureFlag::Builtin(FeatureKind::Convert));
  }
}
#[cfg(feature = "enum")]
register_descriptor! {
  FunctionDescriptor {
    name: "ConvertSEnum<enum>",
    ptr: ConvertSEnum::new,
  }
}

#[cfg(all(feature = "matrix", feature = "table"))]
#[derive(Debug)]
struct ConvertMat2Table<T> {
  arg: Matrix<T>,
  out: Ref<MechTable>,
}

#[cfg(all(feature = "matrix", feature = "table"))]
impl<T> MechFunctionImpl for ConvertMat2Table<T>
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
#[cfg(all(feature = "compiler", feature = "matrix", feature = "table"))]
impl<T> MechFunctionCompiler for ConvertMat2Table<T> 
where
  T: ConstElem + CompileConst + AsValueKind,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let mut registers = [0,0];

    registers[0] = compile_register_brrw!(self.out, ctx);
    registers[1] = compile_register!(self.arg, ctx);

    ctx.features.insert(FeatureFlag::Builtin(FeatureKind::Convert));

    ctx.emit_unop(
      hash_str("ConvertMat2Table"),
      registers[0],
      registers[1],
    );

    return Ok(registers[0]);
  }
}

#[cfg(all(feature = "rational", feature = "f64"))]
#[derive(Debug)]
struct ConvertSRationalToF64 {
  arg: Ref<R64>,
  out: Ref<F64>,
}

#[cfg(all(feature = "rational", feature = "f64"))]
impl MechFunctionImpl for ConvertSRationalToF64 {
  fn solve(&self) {
    let arg_ptr = self.arg.as_ptr();
    let out_ptr = self.out.as_mut_ptr();
    unsafe{ *out_ptr = (*arg_ptr).into(); }
  }
  fn out(&self) -> Value { Value::F64(self.out.clone()) }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(all(feature = "compiler", feature = "rational", feature = "f64"))]
impl MechFunctionCompiler for ConvertSRationalToF64 {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("ConvertSRationalToF64<f64>");
    compile_unop!(name, self.out, self.arg, ctx, FeatureFlag::Builtin(FeatureKind::Convert));
  }
}

macro_rules! impl_conversion_match_arms {
  ($arg:expr, $($input_type:ident, $input_type_string:tt => $($target_type:ident, $target_type_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          #[cfg(all(feature = "matrix", feature = "table", feature = $input_type_string))]
          (Value::[<Matrix $input_type:camel>](mat), Value::Kind(ValueKind::Table(tbl, sze))) => {
            let in_shape = mat.shape();
            let tbl_cols = tbl.len();
            let mat_knd = ValueKind::[<$input_type:camel>];
            // Verify the table has the correct number of columns
            if in_shape[1] != tbl_cols {
              return Err(MechError2::new(
                ConvertIncorrectNumberOfColumnsError{from: in_shape[1], to: tbl_cols},
                None,
              ).with_compiler_loc());
            }
            // Verify each column of the matrix can be converted to the target type of the table
            for (_, knd) in &tbl {
              if *knd == mat_knd {
                continue;
              } else if mat_knd.is_convertible_to(knd) {
                continue;
              } else {
                return Err(MechError2::new(
                  ColumnConvertKindMismatchError{from: mat_knd, to: knd.clone()},
                  None,
                ).with_compiler_loc());
              }
            }
            // Create a blank table, with as many rows as the matrix has
            let out = MechTable::from_kind(ValueKind::Table(tbl.clone(), in_shape[0]))?;
            Ok(Box::new(ConvertMat2Table::<$input_type>{arg: mat.clone(), out: Ref::new(out)}))
          }
          $(
            #[cfg(all(feature = $input_type_string, feature = $target_type_string))]
            (Value::[<$input_type:camel>](arg), Value::Kind(ValueKind::[<$target_type:camel>])) => {Ok(Box::new(ConvertScalarToScalarBasic{arg: arg.clone(), out: Ref::new($target_type::default())}))},
          )+
        )+
        #[cfg(feature = "rational")]
        (Value::R64(ref rat), Value::Kind(ValueKind::F64)) => {
          Ok(Box::new(ConvertSRationalToF64{arg: rat.clone(), out: Ref::new(F64::default())}))
        }
        #[cfg(all(feature = "atom", feature = "enum"))]
        (Value::Atom(variant_id), Value::Kind(ValueKind::Enum(enum_id))) => {
          let variants = vec![(variant_id.borrow().0,None)];
          let enm = MechEnum{id: enum_id, variants};
          let val = Ref::new(enm.clone());
          Ok(Box::new(ConvertSEnum{out: val}))
        }
        x => Err(MechError2::new(
            UnsupportedConversionError{from: x.0.kind(), to: x.1.kind()},
            None,
          ).with_compiler_loc()
        ),
      }
    }
  }
}

#[derive(Debug)]
pub struct ConvertScalarToScalar<F, T> {
  pub arg: Ref<F>,
  pub out: Ref<T>,
}

impl<F, T> MechFunctionImpl for ConvertScalarToScalar<F, T>
where
  Ref<T>: ToValue,
  F: LosslessInto<T> + Debug + Clone,
  T: Debug,
{
  fn solve(&self) {
    let arg_ptr = self.arg.as_ptr();
    let out_ptr = self.out.as_mut_ptr();
    unsafe {
      let out_ref: &mut T = &mut *out_ptr;
      let arg_ref: &F = &*arg_ptr;
      *out_ref = arg_ref.clone().lossless_into();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl<F, T> MechFunctionCompiler for ConvertScalarToScalar<F, T> 
where
  F: ConstElem + CompileConst + AsValueKind,
  T: ConstElem + CompileConst + AsValueKind,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("ConvertScalarToScalar<{},{}>", F::as_value_kind(), T::as_value_kind());
    compile_unop!(name, self.out, self.arg, ctx, FeatureFlag::Builtin(FeatureKind::Convert));
  }
}

#[derive(Debug)]
pub struct ConvertScalarToScalarBasic<F, T> {
  pub arg: Ref<F>,
  pub out: Ref<T>,
}

impl<F, T> MechFunctionImpl for ConvertScalarToScalarBasic<F, T>
where
  Ref<T>: ToValue,
  F: Debug + Clone,
  T: Debug + LossyFrom<F>,
{
  fn solve(&self) {
    let arg_ptr = self.arg.as_ptr();
    let out_ptr = self.out.as_mut_ptr();
    unsafe {
      let out_ref: &mut T = &mut *out_ptr;
      let arg_ref: &F = &*arg_ptr;
      *out_ref = T::lossy_from(arg_ref.clone());
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl<F,T> MechFunctionCompiler for ConvertScalarToScalarBasic<F, T> 
where
  F: ConstElem + CompileConst + AsValueKind,
  T: ConstElem + CompileConst + AsValueKind,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("ConvertScalarToScalarBasic<{},{}>", F::as_value_kind(), T::as_value_kind());
    compile_unop!(name, self.out, self.arg, ctx, FeatureFlag::Builtin(FeatureKind::Convert));
  }
}

fn impl_conversion_fxn(source_value: Value, target_kind: Value) -> MResult<Box<dyn MechFunction>>  {
  match (&source_value, &target_kind) {
    #[cfg(all(feature = "rational", feature = "f64"))]
    (Value::R64(r), Value::Kind(ValueKind::F64)) => {return Ok(Box::new(ConvertScalarToScalar{arg: r.clone(),out: Ref::new(F64::default()),}));}
    #[cfg(all(feature = "matrix", feature = "table", feature = "string"))]
    (Value::MatrixString(ref mat), Value::Kind(ValueKind::Table(tbl, sze))) => {
      let in_shape = mat.shape();
      // Verify the table has the correct number of columns
      if in_shape[1] != tbl.len() {
        return Err(MechError2::new(
          ConvertIncorrectNumberOfColumnsError{from: in_shape[1], to: tbl.len()},
          None,
        ).with_compiler_loc());
      }
      // Create a blank table, with as many rows as the matrix has
      let out = MechTable::from_kind(ValueKind::Table(tbl.clone(), in_shape[0]))?;
      return Ok(Box::new(ConvertMat2Table::<String>{arg: mat.clone(), out: Ref::new(out)}));
    }
    #[cfg(all(feature = "matrix", feature = "table", feature = "bool"))]
    (Value::MatrixBool(ref mat), Value::Kind(ValueKind::Table(tbl, sze))) => {
      let in_shape = mat.shape();
      // Verify the table has the correct number of columns
      if in_shape[1] != tbl.len() {
        return Err(MechError2::new(
          ConvertIncorrectNumberOfColumnsError{from: in_shape[1], to: tbl.len()},
          None,
        ).with_compiler_loc());
      }
      // Create a blank table, with as many rows as the matrix has
      let out = MechTable::from_kind(ValueKind::Table(tbl.clone(), in_shape[0]))?;
      return Ok(Box::new(ConvertMat2Table::<bool>{arg: mat.clone(), out: Ref::new(out)}));
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
    F64, "f64" => String, "string", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", F32, "f32", F64, "f64", R64, "rational";
    R64, "rational" => String, "string", F64, "f64";
    String, "string" => String, "string";
    bool, "bool" => String, "string", bool, "bool";
  )
}

pub struct ConvertKind {}

impl NativeFunctionCompiler for ConvertKind {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let source_value = arguments[0].clone();
    let target_kind = arguments[1].clone();
    match impl_conversion_fxn(source_value.clone(), target_kind.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match source_value {
          Value::MutableReference(rhs) => impl_conversion_fxn(rhs.borrow().clone(), target_kind.clone()),
          #[cfg(feature = "atom")]
          Value::Atom(ref atom_id) => impl_conversion_fxn(source_value, target_kind.clone()),
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
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind2 { arg: (arguments[0].clone(), arguments[1].clone()), fxn_name: "convert/scalar".to_string() },
              None,
            ).with_compiler_loc()
          ),
        }
      }
    }
  }
}

#[derive(Debug)]
pub struct ColumnConvertKindMismatchError {
  pub from: ValueKind,
  pub to: ValueKind,
}

impl MechErrorKind2 for ColumnConvertKindMismatchError {
  fn name(&self) -> &str { "ColumnTypeMismatch" }
  fn message(&self) -> String {
    format!(
      "Matrix column kind {:?} does not match table column kind {:?}. Conversion requires the element types to be compatible.",
      self.from, self.to
    )
  }
}

#[derive(Debug)]
pub struct ConvertIncorrectNumberOfColumnsError {
  pub from: usize,
  pub to: usize,
}
impl MechErrorKind2 for ConvertIncorrectNumberOfColumnsError {
  fn name(&self) -> &str { "IncorrectNumberOfColumns" }
  fn message(&self) -> String {
    format!(
      "Matrix has {} columns, but table expects {}. Column count must match for assignment.",
      self.from, self.to
    )
  }
}
