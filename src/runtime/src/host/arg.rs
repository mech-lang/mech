//! Canonical host argument helpers.

use crate::RuntimeValueSnapshot;
use mech_core::{MResult, MechError, MechErrorKind, Value, ValueCell, ValueData};

pub trait HostArgumentValue {
    fn host_argument_value(&self) -> &Value;
}

impl HostArgumentValue for Value {
    fn host_argument_value(&self) -> &Value {
        self
    }
}

impl HostArgumentValue for RuntimeValueSnapshot {
    fn host_argument_value(&self) -> &Value {
        self.value()
    }
}

#[derive(Debug, Clone)]
pub struct HostArgumentError {
    pub function: String,
    pub reason: String,
}

impl HostArgumentError {
    pub fn new(function: impl Into<String>, reason: impl Into<String>) -> Self {
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
        format!("Invalid arguments for `{}`: {}", self.function, self.reason)
    }
}

fn argument_error(function: &str, reason: impl Into<String>) -> MechError {
    MechError::new(HostArgumentError::new(function, reason), None)
}

fn wrong_type(function: &str, index: usize, expected: &str, actual: &Value) -> MechError {
    argument_error(
        function,
        format!(
            "expected {expected} argument {index}, got {}",
            actual.data().kind()
        ),
    )
}

pub fn host_arg(function: &str, args: &[impl HostArgumentValue], index: usize) -> MResult<Value> {
    args.get(index)
        .map(|value| value.host_argument_value().clone())
        .ok_or_else(|| argument_error(function, format!("missing argument {index}")))
}

pub fn host_arg_cloned(
    function: &str,
    args: &[impl HostArgumentValue],
    index: usize,
) -> MResult<Value> {
    host_arg(function, args, index)
}

pub fn host_arg_raw(function: &str, args: &[Value], index: usize) -> MResult<Value> {
    host_arg(function, args, index)
}

pub fn host_arg_resolved(
    function: &str,
    args: &[impl HostArgumentValue],
    index: usize,
) -> MResult<Value> {
    host_arg(function, args, index)
}

pub fn host_args_tail(
    function: &str,
    args: &[impl HostArgumentValue],
    start: usize,
) -> MResult<Vec<Value>> {
    if start > args.len() {
        return Err(argument_error(
            function,
            format!("tail start {start} is past argument count {}", args.len()),
        ));
    }
    Ok(args[start..]
        .iter()
        .map(|value| value.host_argument_value().clone())
        .collect())
}

pub fn expect_arity(
    function: &str,
    args: &[impl HostArgumentValue],
    expected: usize,
) -> MResult<()> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(argument_error(
            function,
            format!("expected {expected} arguments, got {}", args.len()),
        ))
    }
}

pub fn expect_min_arity(
    function: &str,
    args: &[impl HostArgumentValue],
    min: usize,
) -> MResult<()> {
    if args.len() >= min {
        Ok(())
    } else {
        Err(argument_error(
            function,
            format!("expected at least {min} arguments, got {}", args.len()),
        ))
    }
}

pub fn expect_max_arity(
    function: &str,
    args: &[impl HostArgumentValue],
    max: usize,
) -> MResult<()> {
    if args.len() <= max {
        Ok(())
    } else {
        Err(argument_error(
            function,
            format!("expected at most {max} arguments, got {}", args.len()),
        ))
    }
}

pub fn expect_arity_between(
    function: &str,
    args: &[impl HostArgumentValue],
    min: usize,
    max: usize,
) -> MResult<()> {
    if (min..=max).contains(&args.len()) {
        Ok(())
    } else {
        Err(argument_error(
            function,
            format!(
                "expected between {min} and {max} arguments, got {}",
                args.len()
            ),
        ))
    }
}

pub fn expect_no_args(function: &str, args: &[impl HostArgumentValue]) -> MResult<()> {
    expect_arity(function, args, 0)
}

pub fn is_empty_value(value: &Value) -> bool {
    matches!(value.data(), ValueData::Tuple(values) if values.is_empty())
        || matches!(value.data(), ValueData::Option(None))
}

pub fn host_arg_optional(
    function: &str,
    args: &[impl HostArgumentValue],
    index: usize,
) -> MResult<Option<Value>> {
    if index >= args.len() {
        return Ok(None);
    }
    let value = host_arg(function, args, index)?;
    Ok((!is_empty_value(&value)).then_some(value))
}

pub fn host_arg_optional_value(
    function: &str,
    args: &[impl HostArgumentValue],
    index: usize,
) -> MResult<Option<Value>> {
    host_arg_optional(function, args, index)
}

macro_rules! scalar_arg {
    ($name:ident, $type:ty, $variant:ident, $convert:expr) => {
        pub fn $name(
            function: &str,
            args: &[impl HostArgumentValue],
            index: usize,
        ) -> MResult<$type> {
            let value = host_arg(function, args, index)?;
            match value.data() {
                ValueData::$variant(value) => Ok($convert(value)),
                _ => Err(wrong_type(function, index, stringify!($type), &value)),
            }
        }
    };
}

scalar_arg!(host_arg_u8, u8, U8, |value: &u8| *value);
scalar_arg!(host_arg_u16, u16, U16, |value: &u16| *value);
scalar_arg!(host_arg_u32, u32, U32, |value: &u32| *value);
scalar_arg!(host_arg_u64, u64, U64, |value: &u64| *value);
scalar_arg!(host_arg_u128, u128, U128, |value: &u128| *value);
scalar_arg!(host_arg_i8, i8, I8, |value: &i8| *value);
scalar_arg!(host_arg_i16, i16, I16, |value: &i16| *value);
scalar_arg!(host_arg_i32, i32, I32, |value: &i32| *value);
scalar_arg!(host_arg_i64, i64, I64, |value: &i64| *value);
scalar_arg!(host_arg_i128, i128, I128, |value: &i128| *value);

#[cfg(feature = "f32")]
scalar_arg!(
    host_arg_f32,
    f32,
    F32,
    |value: &mech_core::snapshot::F32Bits| value.to_f32()
);
#[cfg(feature = "f64")]
scalar_arg!(
    host_arg_f64,
    f64,
    F64,
    |value: &mech_core::snapshot::F64Bits| value.to_f64()
);
#[cfg(feature = "bool")]
scalar_arg!(host_arg_bool, bool, Bool, |value: &bool| *value);

#[cfg(feature = "string")]
pub fn host_arg_string(
    function: &str,
    args: &[impl HostArgumentValue],
    index: usize,
) -> MResult<String> {
    let value = host_arg(function, args, index)?;
    match value.data() {
        ValueData::String(value) => Ok(value.to_string()),
        _ => Err(wrong_type(function, index, "string", &value)),
    }
}

#[cfg(feature = "string")]
pub fn host_arg_strict_string(
    function: &str,
    args: &[impl HostArgumentValue],
    index: usize,
) -> MResult<String> {
    host_arg_string(function, args, index)
}

pub fn host_arg_index(
    function: &str,
    args: &[impl HostArgumentValue],
    index: usize,
) -> MResult<usize> {
    let value = host_arg(function, args, index)?;
    match value.data() {
        ValueData::Index(value) => usize::try_from(*value)
            .map_err(|_| argument_error(function, format!("index argument {index} is too large"))),
        _ => Err(wrong_type(function, index, "index", &value)),
    }
}

pub fn host_arg_id(function: &str, args: &[impl HostArgumentValue], index: usize) -> MResult<u64> {
    let value = host_arg(function, args, index)?;
    match value.data() {
        ValueData::Id(value) => Ok(*value),
        _ => Err(wrong_type(function, index, "id", &value)),
    }
}

macro_rules! optional_arg {
    ($name:ident, $target:ty, $required:ident) => {
        pub fn $name(
            function: &str,
            args: &[impl HostArgumentValue],
            index: usize,
        ) -> MResult<Option<$target>> {
            if index >= args.len() || is_empty_value(args[index].host_argument_value()) {
                Ok(None)
            } else {
                $required(function, args, index).map(Some)
            }
        }
    };
}

#[cfg(feature = "string")]
optional_arg!(host_arg_optional_string, String, host_arg_string);
#[cfg(feature = "bool")]
optional_arg!(host_arg_optional_bool, bool, host_arg_bool);
optional_arg!(host_arg_optional_u64, u64, host_arg_u64);
optional_arg!(host_arg_optional_i64, i64, host_arg_i64);
#[cfg(feature = "f64")]
optional_arg!(host_arg_optional_f64, f64, host_arg_f64);

fn exact_value<T: mech_core::CanonicalCellBacking>(value: T) -> Value {
    ValueCell::from_exact(value)
        .and_then(|cell| cell.snapshot())
        .expect("supported exact host values have a canonical schema")
}

pub fn value_empty() -> Value {
    ValueCell::unit()
        .snapshot()
        .expect("canonical unit snapshot is valid")
}

#[cfg(feature = "string")]
pub fn value_string(value: impl Into<String>) -> Value {
    exact_value(value.into())
}
#[cfg(feature = "bool")]
pub fn value_bool(value: bool) -> Value {
    exact_value(value)
}
#[cfg(feature = "u8")]
pub fn value_u8(value: u8) -> Value {
    exact_value(value)
}
#[cfg(feature = "u16")]
pub fn value_u16(value: u16) -> Value {
    exact_value(value)
}
#[cfg(feature = "u32")]
pub fn value_u32(value: u32) -> Value {
    exact_value(value)
}
#[cfg(feature = "u64")]
pub fn value_u64(value: u64) -> Value {
    exact_value(value)
}
#[cfg(feature = "u128")]
pub fn value_u128(value: u128) -> Value {
    exact_value(value)
}
#[cfg(feature = "i8")]
pub fn value_i8(value: i8) -> Value {
    exact_value(value)
}
#[cfg(feature = "i16")]
pub fn value_i16(value: i16) -> Value {
    exact_value(value)
}
#[cfg(feature = "i32")]
pub fn value_i32(value: i32) -> Value {
    exact_value(value)
}
#[cfg(feature = "i64")]
pub fn value_i64(value: i64) -> Value {
    exact_value(value)
}
#[cfg(feature = "i128")]
pub fn value_i128(value: i128) -> Value {
    exact_value(value)
}
#[cfg(feature = "f32")]
pub fn value_f32(value: f32) -> Value {
    exact_value(value)
}
#[cfg(feature = "f64")]
pub fn value_f64(value: f64) -> Value {
    exact_value(value)
}
pub fn value_index(value: usize) -> Value {
    exact_value(value)
}

pub trait FromHostValue: Sized {
    fn from_host_value(
        function: &str,
        args: &[impl HostArgumentValue],
        index: usize,
    ) -> MResult<Self>;
}

pub trait IntoHostValue {
    fn into_host_value(self) -> Value;
}

impl FromHostValue for Value {
    fn from_host_value(
        function: &str,
        args: &[impl HostArgumentValue],
        index: usize,
    ) -> MResult<Self> {
        host_arg(function, args, index)
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
        args: &[impl HostArgumentValue],
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
        args: &[impl HostArgumentValue],
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

#[cfg(any(
    feature = "u8",
    feature = "u16",
    feature = "u32",
    feature = "u64",
    feature = "u128",
    feature = "i8",
    feature = "i16",
    feature = "i32",
    feature = "i64",
    feature = "i128",
    feature = "f32",
    feature = "f64"
))]
macro_rules! host_numeric {
    ($type:ty, $arg:ident, $value:ident) => {
        impl FromHostValue for $type {
            fn from_host_value(
                function: &str,
                args: &[impl HostArgumentValue],
                index: usize,
            ) -> MResult<Self> {
                $arg(function, args, index)
            }
        }
        impl IntoHostValue for $type {
            fn into_host_value(self) -> Value {
                $value(self)
            }
        }
    };
}

#[cfg(feature = "u8")]
host_numeric!(u8, host_arg_u8, value_u8);
#[cfg(feature = "u16")]
host_numeric!(u16, host_arg_u16, value_u16);
#[cfg(feature = "u32")]
host_numeric!(u32, host_arg_u32, value_u32);
#[cfg(feature = "u64")]
host_numeric!(u64, host_arg_u64, value_u64);
#[cfg(feature = "u128")]
host_numeric!(u128, host_arg_u128, value_u128);
#[cfg(feature = "i8")]
host_numeric!(i8, host_arg_i8, value_i8);
#[cfg(feature = "i16")]
host_numeric!(i16, host_arg_i16, value_i16);
#[cfg(feature = "i32")]
host_numeric!(i32, host_arg_i32, value_i32);
#[cfg(feature = "i64")]
host_numeric!(i64, host_arg_i64, value_i64);
#[cfg(feature = "i128")]
host_numeric!(i128, host_arg_i128, value_i128);
#[cfg(feature = "f32")]
host_numeric!(f32, host_arg_f32, value_f32);
#[cfg(feature = "f64")]
host_numeric!(f64, host_arg_f64, value_f64);

impl<T: FromHostValue> FromHostValue for Option<T> {
    fn from_host_value(
        function: &str,
        args: &[impl HostArgumentValue],
        index: usize,
    ) -> MResult<Self> {
        if index >= args.len() || is_empty_value(args[index].host_argument_value()) {
            Ok(None)
        } else {
            T::from_host_value(function, args, index).map(Some)
        }
    }
}

impl<T: IntoHostValue> IntoHostValue for Option<T> {
    fn into_host_value(self) -> Value {
        self.map(IntoHostValue::into_host_value)
            .unwrap_or_else(value_empty)
    }
}

pub fn host_arg_as<T: FromHostValue>(
    function: &str,
    args: &[impl HostArgumentValue],
    index: usize,
) -> MResult<T> {
    T::from_host_value(function, args, index)
}

pub fn host_return<T: IntoHostValue>(value: T) -> Value {
    value.into_host_value()
}

macro_rules! host_calls {
    ($(($name:ident, $result:ident, $count:literal, $(($generic:ident, $index:literal)),*)),* $(,)?) => {
        $(
            pub fn $name<$($generic,)* R>(
                function: &str,
                args: &[impl HostArgumentValue],
                f: impl FnOnce($($generic),*) -> R,
            ) -> MResult<Value>
            where
                $($generic: FromHostValue,)*
                R: IntoHostValue,
            {
                expect_arity(function, args, $count)?;
                Ok(f($($generic::from_host_value(function, args, $index)?),*).into_host_value())
            }

            pub fn $result<$($generic,)* R>(
                function: &str,
                args: &[impl HostArgumentValue],
                f: impl FnOnce($($generic),*) -> MResult<R>,
            ) -> MResult<Value>
            where
                $($generic: FromHostValue,)*
                R: IntoHostValue,
            {
                expect_arity(function, args, $count)?;
                Ok(f($($generic::from_host_value(function, args, $index)?),*)?.into_host_value())
            }
        )*
    };
}

host_calls!(
    (host_call0, host_call_result0, 0,),
    (host_call1, host_call_result1, 1, (A, 0)),
    (host_call2, host_call_result2, 2, (A, 0), (B, 1)),
    (host_call3, host_call_result3, 3, (A, 0), (B, 1), (C, 2)),
    (
        host_call4,
        host_call_result4,
        4,
        (A, 0),
        (B, 1),
        (C, 2),
        (D, 3)
    ),
);
