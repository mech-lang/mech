//! Host argument helpers.
//!
//! These helpers are for authors of runtime `HostFunction`s. They keep demo,
//! integration, and embedder host functions from repeating argument extraction,
//! arity checks, and `Value` construction boilerplate.
//!
//! The scalar helpers use Mech's existing `Value::as_*` conversion methods where
//! possible. Compound values are exposed as borrowed/cloned `Value`s here rather
//! than overfitting this runtime crate to every internal container layout.

use mech_core::{
  MResult, MechError, MechErrorKind, Ref, Value, ValueKind,
};

// -----------------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct HostArgumentError {
  pub function: String,
  pub reason: String,
}

impl HostArgumentError {
  pub fn new(
    function: impl Into<String>,
    reason: impl Into<String>,
  ) -> Self {
    Self {
      function: function.into(),
      reason: reason.into(),
    }
  }
}

impl MechErrorKind for HostArgumentError {
  fn name(&self) -> &str {
    "HostArgument"
  }

  fn message(&self) -> String {
    format!(
      "Invalid arguments for `{}`: {}",
      self.function,
      self.reason,
    )
  }
}

fn host_argument_error(
  function: &str,
  reason: impl Into<String>,
) -> MechError {
  MechError::new(
    HostArgumentError::new(function, reason),
    None,
  )
}

fn wrong_type_error(
  function: &str,
  index: usize,
  expected: &str,
  actual: &Value,
) -> MechError {
  host_argument_error(
    function,
    format!(
      "expected {} argument {}, got {:?}",
      expected,
      index,
      actual,
    ),
  )
}

// -----------------------------------------------------------------------------
// Generic argument access / arity
// -----------------------------------------------------------------------------

pub fn host_arg<'a>(
  function: &str,
  args: &'a [Value],
  index: usize,
) -> MResult<&'a Value> {
  args.get(index).ok_or_else(|| {
    host_argument_error(
      function,
      format!("missing argument {}", index),
    )
  })
}

pub fn host_arg_cloned(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<Value> {
  Ok(host_arg(function, args, index)?.clone())
}

pub fn host_arg_resolved(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<Value> {
  match host_arg(function, args, index)? {
    Value::MutableReference(value) => Ok(value.borrow().clone()),
    Value::Typed(value, _) => todo!(),
    other => Ok(other.clone()),
  }
}

pub fn host_args_tail(
  function: &str,
  args: &[Value],
  start: usize,
) -> MResult<Vec<Value>> {
  if start > args.len() {
    return Err(host_argument_error(
      function,
      format!(
        "tail start {} is past argument count {}",
        start,
        args.len(),
      ),
    ));
  }

  Ok(args[start..].to_vec())
}

pub fn expect_arity(
  function: &str,
  args: &[Value],
  expected: usize,
) -> MResult<()> {
  if args.len() == expected {
    return Ok(());
  }

  Err(host_argument_error(
    function,
    format!(
      "expected {} arguments, got {}",
      expected,
      args.len(),
    ),
  ))
}

pub fn expect_min_arity(
  function: &str,
  args: &[Value],
  min: usize,
) -> MResult<()> {
  if args.len() >= min {
    return Ok(());
  }

  Err(host_argument_error(
    function,
    format!(
      "expected at least {} arguments, got {}",
      min,
      args.len(),
    ),
  ))
}

pub fn expect_max_arity(
  function: &str,
  args: &[Value],
  max: usize,
) -> MResult<()> {
  if args.len() <= max {
    return Ok(());
  }

  Err(host_argument_error(
    function,
    format!(
      "expected at most {} arguments, got {}",
      max,
      args.len(),
    ),
  ))
}

pub fn expect_arity_between(
  function: &str,
  args: &[Value],
  min: usize,
  max: usize,
) -> MResult<()> {
  if args.len() >= min && args.len() <= max {
    return Ok(());
  }

  Err(host_argument_error(
    function,
    format!(
      "expected between {} and {} arguments, got {}",
      min,
      max,
      args.len(),
    ),
  ))
}

pub fn expect_no_args(
  function: &str,
  args: &[Value],
) -> MResult<()> {
  expect_arity(function, args, 0)
}

pub fn is_empty_value(value: &Value) -> bool {
  matches!(value, Value::Empty)
}

pub fn host_arg_optional(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<Option<Value>> {
  if index >= args.len() {
    return Ok(None);
  }

  let value = host_arg(function, args, index)?;

  if is_empty_value(value) {
    return Ok(None);
  }

  Ok(Some(value.clone()))
}

// -----------------------------------------------------------------------------
// Strings / booleans
// -----------------------------------------------------------------------------

pub fn host_arg_string(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<String> {
  match host_arg(function, args, index)? {
    Value::String(value) => Ok(value.borrow().clone()),
    other => Err(wrong_type_error(function, index, "string", other)),
  }
}

pub fn host_arg_strict_string(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<String> {
  host_arg_string(function, args, index)
}

pub fn host_arg_optional_string(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<Option<String>> {
  if index >= args.len() {
    return Ok(None);
  }

  if is_empty_value(host_arg(function, args, index)?) {
    return Ok(None);
  }

  Ok(Some(host_arg_string(function, args, index)?))
}

pub fn host_arg_bool(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<bool> {
  match host_arg(function, args, index)? {
    Value::Bool(value) => Ok(*value.borrow()),
    other => Err(wrong_type_error(function, index, "bool", other)),
  }
}

pub fn host_arg_optional_bool(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<Option<bool>> {
  if index >= args.len() {
    return Ok(None);
  }

  if is_empty_value(host_arg(function, args, index)?) {
    return Ok(None);
  }

  Ok(Some(host_arg_bool(function, args, index)?))
}

// -----------------------------------------------------------------------------
// Unsigned integers
// -----------------------------------------------------------------------------

pub fn host_arg_u8(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<u8> {
  Ok(*host_arg(function, args, index)?.as_u8()?.borrow())
}

pub fn host_arg_u16(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<u16> {
  Ok(*host_arg(function, args, index)?.as_u16()?.borrow())
}

pub fn host_arg_u32(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<u32> {
  Ok(*host_arg(function, args, index)?.as_u32()?.borrow())
}

pub fn host_arg_u64(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<u64> {
  Ok(*host_arg(function, args, index)?.as_u64()?.borrow())
}

pub fn host_arg_u128(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<u128> {
  Ok(*host_arg(function, args, index)?.as_u128()?.borrow())
}

pub fn host_arg_optional_u64(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<Option<u64>> {
  if index >= args.len() {
    return Ok(None);
  }

  if is_empty_value(host_arg(function, args, index)?) {
    return Ok(None);
  }

  Ok(Some(host_arg_u64(function, args, index)?))
}

// -----------------------------------------------------------------------------
// Signed integers
// -----------------------------------------------------------------------------

pub fn host_arg_i8(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<i8> {
  Ok(*host_arg(function, args, index)?.as_i8()?.borrow())
}

pub fn host_arg_i16(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<i16> {
  Ok(*host_arg(function, args, index)?.as_i16()?.borrow())
}

pub fn host_arg_i32(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<i32> {
  Ok(*host_arg(function, args, index)?.as_i32()?.borrow())
}

pub fn host_arg_i64(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<i64> {
  Ok(*host_arg(function, args, index)?.as_i64()?.borrow())
}

pub fn host_arg_i128(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<i128> {
  Ok(*host_arg(function, args, index)?.as_i128()?.borrow())
}

pub fn host_arg_optional_i64(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<Option<i64>> {
  if index >= args.len() {
    return Ok(None);
  }

  if is_empty_value(host_arg(function, args, index)?) {
    return Ok(None);
  }

  Ok(Some(host_arg_i64(function, args, index)?))
}

// -----------------------------------------------------------------------------
// Floats
// -----------------------------------------------------------------------------

#[cfg(feature = "f32")]
pub fn host_arg_f32(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<f32> {
  Ok(*host_arg(function, args, index)?.as_f32()?.borrow())
}

#[cfg(feature = "f64")]
pub fn host_arg_f64(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<f64> {
  Ok(*host_arg(function, args, index)?.as_f64()?.borrow())
}

#[cfg(feature = "f64")]
pub fn host_arg_optional_f64(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<Option<f64>> {
  if index >= args.len() {
    return Ok(None);
  }

  if is_empty_value(host_arg(function, args, index)?) {
    return Ok(None);
  }

  Ok(Some(host_arg_f64(function, args, index)?))
}

// -----------------------------------------------------------------------------
// Index / IDs / atoms / enums / kind values
// -----------------------------------------------------------------------------

pub fn host_arg_index(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<usize> {
  match host_arg(function, args, index)? {
    Value::Index(value) => Ok(*value.borrow()),
    value => Ok(value.as_usize()?),
  }
}

pub fn host_arg_id(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<u64> {
  match host_arg(function, args, index)? {
    Value::Id(value) => Ok(*value),
    other => Err(wrong_type_error(function, index, "id", other)),
  }
}

pub fn host_arg_enum(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<mech_core::MechEnum> {
  match host_arg(function, args, index)? {
    Value::Enum(value) => Ok(value.borrow().clone()),
    other => Err(wrong_type_error(function, index, "enum", other)),
  }
}

pub fn host_arg_kind(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<ValueKind> {
  match host_arg(function, args, index)? {
    Value::Kind(kind) => Ok(kind.clone()),
    other => Err(wrong_type_error(function, index, "kind", other)),
  }
}

// -----------------------------------------------------------------------------
// Compound values
// -----------------------------------------------------------------------------

pub fn host_arg_reference_value(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<Value> {
  match host_arg(function, args, index)? {
    Value::MutableReference(value) => Ok(value.borrow().clone()),
    other => Err(wrong_type_error(function, index, "mutable reference", other)),
  }
}

pub fn host_arg_deref_cloned(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<Value> {
  match host_arg(function, args, index)? {
    Value::MutableReference(value) => Ok(value.borrow().clone()),
    other => Ok(other.clone()),
  }
}

#[cfg(feature = "tuple")]
pub fn host_arg_tuple(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<mech_core::MechTuple> {
  match host_arg(function, args, index)? {
    Value::Tuple(value) => Ok(value.borrow().clone()),
    other => Err(wrong_type_error(function, index, "tuple", other)),
  }
}

#[cfg(feature = "record")]
pub fn host_arg_record(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<mech_core::MechRecord> {
  match host_arg(function, args, index)? {
    Value::Record(value) => Ok(value.borrow().clone()),
    other => Err(wrong_type_error(function, index, "record", other)),
  }
}

#[cfg(feature = "table")]
pub fn host_arg_table(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<mech_core::MechTable> {
  match host_arg(function, args, index)? {
    Value::Table(value) => Ok(value.borrow().clone()),
    other => Err(wrong_type_error(function, index, "table", other)),
  }
}

#[cfg(feature = "map")]
pub fn host_arg_map(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<mech_core::MechMap> {
  match host_arg(function, args, index)? {
    Value::Map(value) => Ok(value.borrow().clone()),
    other => Err(wrong_type_error(function, index, "map", other)),
  }
}

#[cfg(feature = "set")]
pub fn host_arg_set(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<mech_core::MechSet> {
  match host_arg(function, args, index)? {
    Value::Set(value) => Ok(value.borrow().clone()),
    other => Err(wrong_type_error(function, index, "set", other)),
  }
}

#[cfg(feature = "matrix")]
pub fn host_arg_matrix_index(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<mech_core::Matrix<usize>> {
  match host_arg(function, args, index)? {
    Value::MatrixIndex(value) => Ok(value.clone()),
    other => Err(wrong_type_error(function, index, "matrix<index>", other)),
  }
}

#[cfg(all(feature = "matrix", feature = "bool"))]
pub fn host_arg_matrix_bool(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<mech_core::Matrix<bool>> {
  match host_arg(function, args, index)? {
    Value::MatrixBool(value) => Ok(value.clone()),
    other => Err(wrong_type_error(function, index, "matrix<bool>", other)),
  }
}

#[cfg(all(feature = "matrix", feature = "u8"))]
pub fn host_arg_matrix_u8(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<mech_core::Matrix<u8>> {
  match host_arg(function, args, index)? {
    Value::MatrixU8(value) => Ok(value.clone()),
    other => Err(wrong_type_error(function, index, "matrix<u8>", other)),
  }
}

#[cfg(all(feature = "matrix", feature = "u16"))]
pub fn host_arg_matrix_u16(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<mech_core::Matrix<u16>> {
  match host_arg(function, args, index)? {
    Value::MatrixU16(value) => Ok(value.clone()),
    other => Err(wrong_type_error(function, index, "matrix<u16>", other)),
  }
}

#[cfg(all(feature = "matrix", feature = "u32"))]
pub fn host_arg_matrix_u32(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<mech_core::Matrix<u32>> {
  match host_arg(function, args, index)? {
    Value::MatrixU32(value) => Ok(value.clone()),
    other => Err(wrong_type_error(function, index, "matrix<u32>", other)),
  }
}

#[cfg(all(feature = "matrix", feature = "u64"))]
pub fn host_arg_matrix_u64(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<mech_core::Matrix<u64>> {
  match host_arg(function, args, index)? {
    Value::MatrixU64(value) => Ok(value.clone()),
    other => Err(wrong_type_error(function, index, "matrix<u64>", other)),
  }
}

#[cfg(all(feature = "matrix", feature = "u128"))]
pub fn host_arg_matrix_u128(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<mech_core::Matrix<u128>> {
  match host_arg(function, args, index)? {
    Value::MatrixU128(value) => Ok(value.clone()),
    other => Err(wrong_type_error(function, index, "matrix<u128>", other)),
  }
}

#[cfg(all(feature = "matrix", feature = "i8"))]
pub fn host_arg_matrix_i8(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<mech_core::Matrix<i8>> {
  match host_arg(function, args, index)? {
    Value::MatrixI8(value) => Ok(value.clone()),
    other => Err(wrong_type_error(function, index, "matrix<i8>", other)),
  }
}

#[cfg(all(feature = "matrix", feature = "i16"))]
pub fn host_arg_matrix_i16(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<mech_core::Matrix<i16>> {
  match host_arg(function, args, index)? {
    Value::MatrixI16(value) => Ok(value.clone()),
    other => Err(wrong_type_error(function, index, "matrix<i16>", other)),
  }
}

#[cfg(all(feature = "matrix", feature = "i32"))]
pub fn host_arg_matrix_i32(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<mech_core::Matrix<i32>> {
  match host_arg(function, args, index)? {
    Value::MatrixI32(value) => Ok(value.clone()),
    other => Err(wrong_type_error(function, index, "matrix<i32>", other)),
  }
}

#[cfg(all(feature = "matrix", feature = "i64"))]
pub fn host_arg_matrix_i64(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<mech_core::Matrix<i64>> {
  match host_arg(function, args, index)? {
    Value::MatrixI64(value) => Ok(value.clone()),
    other => Err(wrong_type_error(function, index, "matrix<i64>", other)),
  }
}

#[cfg(all(feature = "matrix", feature = "i128"))]
pub fn host_arg_matrix_i128(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<mech_core::Matrix<i128>> {
  match host_arg(function, args, index)? {
    Value::MatrixI128(value) => Ok(value.clone()),
    other => Err(wrong_type_error(function, index, "matrix<i128>", other)),
  }
}

#[cfg(all(feature = "matrix", feature = "f32"))]
pub fn host_arg_matrix_f32(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<mech_core::Matrix<f32>> {
  match host_arg(function, args, index)? {
    Value::MatrixF32(value) => Ok(value.clone()),
    other => Err(wrong_type_error(function, index, "matrix<f32>", other)),
  }
}

#[cfg(all(feature = "matrix", feature = "f64"))]
pub fn host_arg_matrix_f64(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<mech_core::Matrix<f64>> {
  match host_arg(function, args, index)? {
    Value::MatrixF64(value) => Ok(value.clone()),
    other => Err(wrong_type_error(function, index, "matrix<f64>", other)),
  }
}

#[cfg(all(feature = "matrix", feature = "string"))]
pub fn host_arg_matrix_string(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<mech_core::Matrix<String>> {
  match host_arg(function, args, index)? {
    Value::MatrixString(value) => Ok(value.clone()),
    other => Err(wrong_type_error(function, index, "matrix<string>", other)),
  }
}

#[cfg(feature = "matrix")]
pub fn host_arg_matrix_value_matrix(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<mech_core::Matrix<Value>> {
  match host_arg(function, args, index)? {
    Value::MatrixValue(value) => Ok(value.clone()),
    other => Err(wrong_type_error(function, index, "matrix<value>", other)),
  }
}

pub fn host_arg_optional_value(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<Option<Value>> {
  if index >= args.len() {
    return Ok(None);
  }

  let value = host_arg(function, args, index)?;

  if is_empty_value(value) {
    return Ok(None);
  }

  Ok(Some(value.clone()))
}

// -----------------------------------------------------------------------------
// Value constructors
// -----------------------------------------------------------------------------

pub fn value_empty() -> Value {
  Value::Empty
}

#[cfg(feature = "string")]
pub fn value_string(value: impl Into<String>) -> Value {
  Value::String(Ref::new(value.into()))
}

#[cfg(feature = "bool")]
pub fn value_bool(value: bool) -> Value {
  Value::Bool(Ref::new(value))
}

#[cfg(feature = "u8")]
pub fn value_u8(value: u8) -> Value {
  Value::U8(Ref::new(value))
}

#[cfg(feature = "u16")]
pub fn value_u16(value: u16) -> Value {
  Value::U16(Ref::new(value))
}

#[cfg(feature = "u32")]
pub fn value_u32(value: u32) -> Value {
  Value::U32(Ref::new(value))
}

#[cfg(feature = "u64")]
pub fn value_u64(value: u64) -> Value {
  Value::U64(Ref::new(value))
}

#[cfg(feature = "u128")]
pub fn value_u128(value: u128) -> Value {
  Value::U128(Ref::new(value))
}

#[cfg(feature = "i8")]
pub fn value_i8(value: i8) -> Value {
  Value::I8(Ref::new(value))
}

#[cfg(feature = "i16")]
pub fn value_i16(value: i16) -> Value {
  Value::I16(Ref::new(value))
}

#[cfg(feature = "i32")]
pub fn value_i32(value: i32) -> Value {
  Value::I32(Ref::new(value))
}

#[cfg(feature = "i64")]
pub fn value_i64(value: i64) -> Value {
  Value::I64(Ref::new(value))
}

#[cfg(feature = "i128")]
pub fn value_i128(value: i128) -> Value {
  Value::I128(Ref::new(value))
}

#[cfg(feature = "f32")]
pub fn value_f32(value: f32) -> Value {
  Value::F32(Ref::new(value))
}

#[cfg(feature = "f64")]
pub fn value_f64(value: f64) -> Value {
  Value::F64(Ref::new(value))
}

pub fn value_index(value: usize) -> Value {
  Value::Index(Ref::new(value))
}

pub fn value_id(value: u64) -> Value {
  Value::Id(value)
}

pub fn value_atom(value: mech_core::MechAtom) -> Value {
  Value::Atom(Ref::new(value))
}

pub fn value_enum(value: mech_core::MechEnum) -> Value {
  Value::Enum(Ref::new(value))
}

pub fn value_kind(kind: ValueKind) -> Value {
  Value::Kind(kind)
}

pub fn value_empty_kind(kind: ValueKind) -> Value {
  Value::EmptyKind(kind)
}

// -----------------------------------------------------------------------------
// Rust conversion traits
// -----------------------------------------------------------------------------

pub trait FromHostValue: Sized {
  fn from_host_value(
    function: &str,
    args: &[Value],
    index: usize,
  ) -> MResult<Self>;
}

pub trait IntoHostValue {
  fn into_host_value(self) -> Value;
}

impl FromHostValue for Value {
  fn from_host_value(
    function: &str,
    args: &[Value],
    index: usize,
  ) -> MResult<Self> {
    host_arg_cloned(function, args, index)
  }
}

impl IntoHostValue for Value {
  fn into_host_value(self) -> Value {
    self
  }
}

#[cfg(feature = "string")]
impl FromHostValue for String {
  fn from_host_value(
    function: &str,
    args: &[Value],
    index: usize,
  ) -> MResult<Self> {
    host_arg_string(function, args, index)
  }
}

#[cfg(feature = "string")]
impl IntoHostValue for String {
  fn into_host_value(self) -> Value {
    value_string(self)
  }
}

#[cfg(feature = "string")]
impl IntoHostValue for &str {
  fn into_host_value(self) -> Value {
    value_string(self)
  }
}

#[cfg(feature = "bool")]
impl FromHostValue for bool {
  fn from_host_value(
    function: &str,
    args: &[Value],
    index: usize,
  ) -> MResult<Self> {
    host_arg_bool(function, args, index)
  }
}

#[cfg(feature = "bool")]
impl IntoHostValue for bool {
  fn into_host_value(self) -> Value {
    value_bool(self)
  }
}

macro_rules! impl_host_numeric {
  ($rust:ty, $arg_fn:ident, $value_fn:ident) => {
    impl FromHostValue for $rust {
      fn from_host_value(
        function: &str,
        args: &[Value],
        index: usize,
      ) -> MResult<Self> {
        $arg_fn(function, args, index)
      }
    }

    impl IntoHostValue for $rust {
      fn into_host_value(self) -> Value {
        $value_fn(self)
      }
    }
  };
}

#[cfg(feature = "u8")]
impl_host_numeric!(u8, host_arg_u8, value_u8);
#[cfg(feature = "u16")]
impl_host_numeric!(u16, host_arg_u16, value_u16);
#[cfg(feature = "u32")]
impl_host_numeric!(u32, host_arg_u32, value_u32);
#[cfg(feature = "u64")]
impl_host_numeric!(u64, host_arg_u64, value_u64);
#[cfg(feature = "u128")]
impl_host_numeric!(u128, host_arg_u128, value_u128);
#[cfg(feature = "i8")]
impl_host_numeric!(i8, host_arg_i8, value_i8);
#[cfg(feature = "i16")]
impl_host_numeric!(i16, host_arg_i16, value_i16);
#[cfg(feature = "i32")]
impl_host_numeric!(i32, host_arg_i32, value_i32);
#[cfg(feature = "i64")]
impl_host_numeric!(i64, host_arg_i64, value_i64);
#[cfg(feature = "i128")]
impl_host_numeric!(i128, host_arg_i128, value_i128);
#[cfg(feature = "f32")]
impl_host_numeric!(f32, host_arg_f32, value_f32);
#[cfg(feature = "f64")]
impl_host_numeric!(f64, host_arg_f64, value_f64);

impl<T> FromHostValue for Option<T>
where
  T: FromHostValue,
{
  fn from_host_value(
    function: &str,
    args: &[Value],
    index: usize,
  ) -> MResult<Self> {
    if index >= args.len() {
      return Ok(None);
    }

    if is_empty_value(host_arg(function, args, index)?) {
      return Ok(None);
    }

    Ok(Some(T::from_host_value(function, args, index)?))
  }
}

impl<T> IntoHostValue for Option<T>
where
  T: IntoHostValue,
{
  fn into_host_value(self) -> Value {
    match self {
      Some(value) => value.into_host_value(),
      None => Value::Empty,
    }
  }
}

// -----------------------------------------------------------------------------
// Typed extraction shortcuts
// -----------------------------------------------------------------------------

pub fn host_arg_as<T: FromHostValue>(
  function: &str,
  args: &[Value],
  index: usize,
) -> MResult<T> {
  T::from_host_value(function, args, index)
}

pub fn host_return<T: IntoHostValue>(value: T) -> Value {
  value.into_host_value()
}

pub fn host_call0<R>(
  function: &str,
  args: &[Value],
  f: impl FnOnce() -> R,
) -> MResult<Value>
where
  R: IntoHostValue,
{
  expect_arity(function, args, 0)?;
  Ok(f().into_host_value())
}

pub fn host_call1<A, R>(
  function: &str,
  args: &[Value],
  f: impl FnOnce(A) -> R,
) -> MResult<Value>
where
  A: FromHostValue,
  R: IntoHostValue,
{
  expect_arity(function, args, 1)?;

  let a = A::from_host_value(function, args, 0)?;

  Ok(f(a).into_host_value())
}

pub fn host_call2<A, B, R>(
  function: &str,
  args: &[Value],
  f: impl FnOnce(A, B) -> R,
) -> MResult<Value>
where
  A: FromHostValue,
  B: FromHostValue,
  R: IntoHostValue,
{
  expect_arity(function, args, 2)?;

  let a = A::from_host_value(function, args, 0)?;
  let b = B::from_host_value(function, args, 1)?;

  Ok(f(a, b).into_host_value())
}

pub fn host_call3<A, B, C, R>(
  function: &str,
  args: &[Value],
  f: impl FnOnce(A, B, C) -> R,
) -> MResult<Value>
where
  A: FromHostValue,
  B: FromHostValue,
  C: FromHostValue,
  R: IntoHostValue,
{
  expect_arity(function, args, 3)?;

  let a = A::from_host_value(function, args, 0)?;
  let b = B::from_host_value(function, args, 1)?;
  let c = C::from_host_value(function, args, 2)?;

  Ok(f(a, b, c).into_host_value())
}

pub fn host_call4<A, B, C, D, R>(
  function: &str,
  args: &[Value],
  f: impl FnOnce(A, B, C, D) -> R,
) -> MResult<Value>
where
  A: FromHostValue,
  B: FromHostValue,
  C: FromHostValue,
  D: FromHostValue,
  R: IntoHostValue,
{
  expect_arity(function, args, 4)?;

  let a = A::from_host_value(function, args, 0)?;
  let b = B::from_host_value(function, args, 1)?;
  let c = C::from_host_value(function, args, 2)?;
  let d = D::from_host_value(function, args, 3)?;

  Ok(f(a, b, c, d).into_host_value())
}

pub fn host_call_result0<R>(
  function: &str,
  args: &[Value],
  f: impl FnOnce() -> MResult<R>,
) -> MResult<Value>
where
  R: IntoHostValue,
{
  expect_arity(function, args, 0)?;
  Ok(f()?.into_host_value())
}

pub fn host_call_result1<A, R>(
  function: &str,
  args: &[Value],
  f: impl FnOnce(A) -> MResult<R>,
) -> MResult<Value>
where
  A: FromHostValue,
  R: IntoHostValue,
{
  expect_arity(function, args, 1)?;

  let a = A::from_host_value(function, args, 0)?;

  Ok(f(a)?.into_host_value())
}

pub fn host_call_result2<A, B, R>(
  function: &str,
  args: &[Value],
  f: impl FnOnce(A, B) -> MResult<R>,
) -> MResult<Value>
where
  A: FromHostValue,
  B: FromHostValue,
  R: IntoHostValue,
{
  expect_arity(function, args, 2)?;

  let a = A::from_host_value(function, args, 0)?;
  let b = B::from_host_value(function, args, 1)?;

  Ok(f(a, b)?.into_host_value())
}

pub fn host_call_result3<A, B, C, R>(
  function: &str,
  args: &[Value],
  f: impl FnOnce(A, B, C) -> MResult<R>,
) -> MResult<Value>
where
  A: FromHostValue,
  B: FromHostValue,
  C: FromHostValue,
  R: IntoHostValue,
{
  expect_arity(function, args, 3)?;

  let a = A::from_host_value(function, args, 0)?;
  let b = B::from_host_value(function, args, 1)?;
  let c = C::from_host_value(function, args, 2)?;

  Ok(f(a, b, c)?.into_host_value())
}

pub fn host_call_result4<A, B, C, D, R>(
  function: &str,
  args: &[Value],
  f: impl FnOnce(A, B, C, D) -> MResult<R>,
) -> MResult<Value>
where
  A: FromHostValue,
  B: FromHostValue,
  C: FromHostValue,
  D: FromHostValue,
  R: IntoHostValue,
{
  expect_arity(function, args, 4)?;

  let a = A::from_host_value(function, args, 0)?;
  let b = B::from_host_value(function, args, 1)?;
  let c = C::from_host_value(function, args, 2)?;
  let d = D::from_host_value(function, args, 3)?;

  Ok(f(a, b, c, d)?.into_host_value())
}