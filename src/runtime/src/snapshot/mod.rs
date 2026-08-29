//! Detached values and summaries returned by the public runtime API.

use std::fmt::{Debug, Display, Formatter};

use mech_core::snapshot::{
    Complex32Bits, Complex64Bits, F32Bits, F64Bits, Rational64Value, SequenceView, ValueDataKind,
};
use mech_core::{MResult, MechError, SchemaId, SchemaKey, Value, ValueCell, ValueData};

use crate::CapabilityId;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

/// An owned immutable canonical runtime value.
#[derive(Clone)]
pub struct RuntimeValueSnapshot {
    value: Value,
}

impl RuntimeValueSnapshot {
    pub fn from_value(value: Value) -> MResult<Self> {
        ValueCell::from_snapshot(value.clone())?;
        Ok(Self { value })
    }

    pub fn empty() -> Self {
        Self {
            value: ValueCell::unit()
                .snapshot()
                .expect("canonical unit snapshot is valid"),
        }
    }

    /// Returns `true` exactly for the canonical empty tuple used as unit.
    ///
    /// ```
    /// use mech_runtime::RuntimeValueSnapshot;
    ///
    /// assert!(RuntimeValueSnapshot::empty().is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        matches!(self.value.data(), ValueData::Tuple(values) if values.is_empty())
    }

    pub fn kind(&self) -> ValueDataKind {
        self.value.data().kind()
    }

    pub const fn schema(&self) -> SchemaId {
        self.value.schema()
    }

    pub const fn schema_key(&self) -> SchemaKey {
        self.value.schema_key()
    }

    pub const fn value(&self) -> &Value {
        &self.value
    }

    /// Returns the portable, single-line Mech representation used by value
    /// events in every interactive host.
    pub fn format_canonical_inline(&self) -> String {
        format_value_inline(&self.value, usize::MAX)
    }

    /// Returns the bounded canonical representation used by interactive
    /// hosts. Elision never mutates or replaces the detached value.
    pub fn format_repl_inline(&self, max_elements: usize) -> String {
        format_value_inline(&self.value, max_elements)
    }

    /// Returns the detached value's rich HTML projection without cloning its
    /// owned value graph. Browser inspection can therefore publish `ans` and
    /// render a popup from one captured snapshot.
    #[cfg(feature = "pretty_print")]
    pub fn format_html(&self) -> String {
        format!(
            "<span class='mech-value'>{}</span>",
            escape_html(&self.format_canonical_inline())
        )
    }

    /// Returns the rich browser projection governed by the same aggregate
    /// traversal budget as textual REPL output and inline document values.
    #[cfg(feature = "pretty_print")]
    pub fn format_repl_html(&self, max_elements: usize) -> String {
        format!(
            "<span class='mech-value'>{}</span>",
            escape_html(&self.format_repl_inline(max_elements))
        )
    }

    pub fn to_value(&self) -> Value {
        self.value.clone()
    }

    pub fn into_value(self) -> Value {
        self.value
    }
}

impl TryFrom<Value> for RuntimeValueSnapshot {
    type Error = MechError;

    fn try_from(value: Value) -> MResult<Self> {
        Self::from_value(value)
    }
}

impl TryFrom<&Value> for RuntimeValueSnapshot {
    type Error = MechError;

    fn try_from(value: &Value) -> MResult<Self> {
        Self::from_value(value.clone())
    }
}

pub trait TryIntoRuntimeValueSnapshot {
    fn try_into_runtime_value_snapshot(self) -> MResult<RuntimeValueSnapshot>;
}

impl TryIntoRuntimeValueSnapshot for RuntimeValueSnapshot {
    fn try_into_runtime_value_snapshot(self) -> MResult<RuntimeValueSnapshot> {
        Ok(self)
    }
}

impl TryIntoRuntimeValueSnapshot for Value {
    fn try_into_runtime_value_snapshot(self) -> MResult<RuntimeValueSnapshot> {
        RuntimeValueSnapshot::from_value(self)
    }
}

impl PartialEq for RuntimeValueSnapshot {
    fn eq(&self, other: &Self) -> bool {
        let (Some(left), Some(right)) = (self.value.schemas(), other.value.schemas()) else {
            return false;
        };
        self.value
            .snapshot_eq(&left, &other.value, &right)
            .unwrap_or(false)
    }
}

impl Debug for RuntimeValueSnapshot {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.value, formatter)
    }
}

impl Display for RuntimeValueSnapshot {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.format_canonical_inline())
    }
}

fn format_value_inline(value: &Value, max_elements: usize) -> String {
    let mut remaining = max_elements;
    format_data(
        value.data(),
        value.shape().parameter_values(),
        &mut remaining,
    )
}

fn format_data(data: &ValueData, shape: &[u64], remaining: &mut usize) -> String {
    macro_rules! scalar {
        ($value:expr) => {{
            if *remaining == 0 {
                return "…".into();
            }
            *remaining -= 1;
            $value.to_string()
        }};
    }
    match data {
        ValueData::Dynamic(value) => match value.value() {
            Some(value) => format_data(value.data(), value.shape().parameter_values(), remaining),
            None => scalar!("_"),
        },
        ValueData::U8(value) => scalar!(value),
        ValueData::U16(value) => scalar!(value),
        ValueData::U32(value) => scalar!(value),
        ValueData::U64(value) => scalar!(value),
        ValueData::U128(value) => scalar!(value),
        ValueData::I8(value) => scalar!(value),
        ValueData::I16(value) => scalar!(value),
        ValueData::I32(value) => scalar!(value),
        ValueData::I64(value) => scalar!(value),
        ValueData::I128(value) => scalar!(value),
        ValueData::F32(value) => scalar!(value.to_f32()),
        ValueData::F64(value) => scalar!(value.to_f64()),
        ValueData::Complex32(value) => scalar!(format!(
            "{}+{}i",
            value.real().to_f32(),
            value.imaginary().to_f32()
        )),
        ValueData::Complex64(value) => scalar!(format!(
            "{}+{}i",
            value.real().to_f64(),
            value.imaginary().to_f64()
        )),
        ValueData::Rational64(value) => {
            scalar!(format!("{}/{}", value.numerator(), value.denominator()))
        }
        ValueData::Bool(value) => scalar!(value),
        ValueData::String(value) => scalar!(format!("{:?}", value)),
        ValueData::Id(value) => scalar!(format!("0x{value:016x}")),
        ValueData::Index(value) => scalar!(value),
        ValueData::Atom => scalar!(":"),
        ValueData::Option(None) => scalar!("none"),
        ValueData::Option(Some(value)) => {
            format!("some({})", format_data(value, &[], remaining))
        }
        ValueData::Tuple(values) => format_sequence("(", ")", values, remaining),
        ValueData::Record(value) => format_sequence("{", "}", value.fields(), remaining),
        ValueData::Matrix(value) => format_matrix(value.elements(), shape, remaining),
        ValueData::Set(value) => {
            let values = value
                .elements()
                .iter()
                .map(|value| value.data())
                .collect::<Vec<_>>();
            format_refs("{", "}", &values, remaining)
        }
        ValueData::Map(value) => {
            let mut entries = Vec::new();
            for entry in value.entries() {
                if *remaining == 0 {
                    entries.push("…".into());
                    break;
                }
                entries.push(format!(
                    "{}: {}",
                    format_data(entry.key().data(), &[], remaining),
                    format_data(entry.value(), &[], remaining)
                ));
            }
            format!("{{{}}}", entries.join(", "))
        }
        ValueData::Enum(value) => match value.payload() {
            Some(payload) => format!(
                ":{}({})",
                value.ordinal(),
                format_data(payload, &[], remaining)
            ),
            None => scalar!(format!(":{}", value.ordinal())),
        },
        ValueData::Table(value) => {
            let mut columns = Vec::new();
            for index in 0..value.len() {
                if let Some(column) = value.column(index) {
                    columns.push(format_sequence_view(column, remaining));
                }
            }
            format!("table({})", columns.join(", "))
        }
        ValueData::Type(value) => scalar!(format!("{value:?}")),
    }
}

fn format_sequence(open: &str, close: &str, values: &[ValueData], remaining: &mut usize) -> String {
    let refs = values.iter().collect::<Vec<_>>();
    format_refs(open, close, &refs, remaining)
}

fn format_refs(open: &str, close: &str, values: &[&ValueData], remaining: &mut usize) -> String {
    let mut output = Vec::new();
    for value in values {
        if *remaining == 0 {
            output.push("…".into());
            break;
        }
        output.push(format_data(value, &[], remaining));
    }
    format!("{open}{}{close}", output.join(", "))
}

fn format_matrix(values: SequenceView<'_>, shape: &[u64], remaining: &mut usize) -> String {
    let columns = shape.get(1).copied().unwrap_or(1) as usize;
    let elements = sequence_strings(values, remaining);
    if columns == 0 {
        return "[]".into();
    }
    let rows = elements
        .chunks(columns)
        .map(|row| row.join(" "))
        .collect::<Vec<_>>();
    format!("[{}]", rows.join("; "))
}

fn format_sequence_view(values: SequenceView<'_>, remaining: &mut usize) -> String {
    format!("[{}]", sequence_strings(values, remaining).join(", "))
}

fn sequence_strings(values: SequenceView<'_>, remaining: &mut usize) -> Vec<String> {
    macro_rules! values {
        ($values:expr, $convert:expr) => {{
            let mut output = Vec::new();
            for value in $values {
                if *remaining == 0 {
                    output.push("…".into());
                    break;
                }
                *remaining -= 1;
                output.push($convert(value));
            }
            output
        }};
    }
    match values {
        SequenceView::U8(values) => values!(values, |value: &u8| value.to_string()),
        SequenceView::U16(values) => values!(values, |value: &u16| value.to_string()),
        SequenceView::U32(values) => values!(values, |value: &u32| value.to_string()),
        SequenceView::U64(values) => values!(values, |value: &u64| value.to_string()),
        SequenceView::U128(values) => values!(values, |value: &u128| value.to_string()),
        SequenceView::I8(values) => values!(values, |value: &i8| value.to_string()),
        SequenceView::I16(values) => values!(values, |value: &i16| value.to_string()),
        SequenceView::I32(values) => values!(values, |value: &i32| value.to_string()),
        SequenceView::I64(values) => values!(values, |value: &i64| value.to_string()),
        SequenceView::I128(values) => values!(values, |value: &i128| value.to_string()),
        SequenceView::F32(values) => values!(values, |value: &F32Bits| value.to_f32().to_string()),
        SequenceView::F64(values) => values!(values, |value: &F64Bits| value.to_f64().to_string()),
        SequenceView::Complex32(values) => values!(values, |value: &Complex32Bits| {
            format!("{}+{}i", value.real().to_f32(), value.imaginary().to_f32())
        }),
        SequenceView::Complex64(values) => values!(values, |value: &Complex64Bits| {
            format!("{}+{}i", value.real().to_f64(), value.imaginary().to_f64())
        }),
        SequenceView::Rational64(values) => {
            values!(values, |value: &Rational64Value| format!(
                "{}/{}",
                value.numerator(),
                value.denominator()
            ))
        }
        SequenceView::Bool(values) => values!(values, |value: &bool| value.to_string()),
        SequenceView::String(values) => {
            values!(values, |value: &Box<str>| format!("{:?}", value))
        }
        SequenceView::Id(values) => values!(values, |value: &u64| format!("0x{value:016x}")),
        SequenceView::Index(values) => values!(values, |value: &u64| value.to_string()),
        SequenceView::Unit(count) => {
            let values = vec![ValueData::Atom; usize::try_from(count).unwrap_or(usize::MAX)];
            values
                .iter()
                .map(|value| format_data(value, &[], remaining))
                .collect()
        }
        SequenceView::Values(values) => values
            .iter()
            .map(|value| format_data(value, &[], remaining))
            .collect(),
    }
}

#[cfg(feature = "pretty_print")]
fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeCapabilitySnapshot {
    pub id: CapabilityId,
    pub subject: String,
    pub revocable: bool,
    pub delegable: bool,
    pub attenuable: bool,
    pub max_uses: Option<u64>,
}
