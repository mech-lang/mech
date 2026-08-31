use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};

#[cfg(any(feature = "f32", feature = "f64", feature = "matrix"))]
use mech_core::ValueData;
use mech_core::{MResult, MechError, MechErrorKind, Value, ValueCell};
#[cfg(feature = "matrix")]
use mech_core::{SchemaBody, snapshot::SequenceView, structures::Matrix as ValueMatrix};

pub const DEFAULT_HOST_INPUT_CAPACITY: usize = 1024;

/// Detached host or compiler-initialization value.
///
/// Matrix variants use Mech's logical row-major order. Exact nalgebra
/// backings are populated at this boundary without exposing their physical
/// column-major layout to hosts.
#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeHostInputValue {
    Unit,
    Bool(bool),
    String(String),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    F32(f32),
    F64(f64),
    Index(usize),
    BoolMatrix {
        rows: usize,
        columns: usize,
        values: Vec<bool>,
    },
    IndexMatrix {
        rows: usize,
        columns: usize,
        values: Vec<usize>,
    },
    F64Matrix {
        rows: usize,
        columns: usize,
        values: Vec<f64>,
    },
    F32Matrix {
        rows: usize,
        columns: usize,
        values: Vec<f32>,
    },
}

impl RuntimeHostInputValue {
    /// Converts a detached host input into the canonical immutable value used
    /// by execution, resource, and public host boundaries.
    pub fn into_value(self) -> MResult<Value> {
        let cell = match self {
            RuntimeHostInputValue::Unit => ValueCell::unit(),
            #[cfg(feature = "bool")]
            RuntimeHostInputValue::Bool(value) => ValueCell::from_exact(value)?,
            #[cfg(not(feature = "bool"))]
            RuntimeHostInputValue::Bool(_) => {
                return Err(input_error(
                    "RuntimeHostInputValueUnsupported",
                    "bool host input values require the `bool` feature",
                ));
            }
            #[cfg(feature = "string")]
            RuntimeHostInputValue::String(value) => ValueCell::from_exact(value)?,
            #[cfg(not(feature = "string"))]
            RuntimeHostInputValue::String(_) => {
                return Err(input_error(
                    "RuntimeHostInputValueUnsupported",
                    "string host input values require the `string` feature",
                ));
            }
            #[cfg(feature = "u8")]
            RuntimeHostInputValue::U8(value) => ValueCell::from_exact(value)?,
            #[cfg(not(feature = "u8"))]
            RuntimeHostInputValue::U8(_) => return Err(unsupported_host_input("u8")),
            #[cfg(feature = "u16")]
            RuntimeHostInputValue::U16(value) => ValueCell::from_exact(value)?,
            #[cfg(not(feature = "u16"))]
            RuntimeHostInputValue::U16(_) => return Err(unsupported_host_input("u16")),
            #[cfg(feature = "u32")]
            RuntimeHostInputValue::U32(value) => ValueCell::from_exact(value)?,
            #[cfg(not(feature = "u32"))]
            RuntimeHostInputValue::U32(_) => return Err(unsupported_host_input("u32")),
            #[cfg(feature = "u64")]
            RuntimeHostInputValue::U64(value) => ValueCell::from_exact(value)?,
            #[cfg(not(feature = "u64"))]
            RuntimeHostInputValue::U64(_) => return Err(unsupported_host_input("u64")),
            #[cfg(feature = "u128")]
            RuntimeHostInputValue::U128(value) => ValueCell::from_exact(value)?,
            #[cfg(not(feature = "u128"))]
            RuntimeHostInputValue::U128(_) => return Err(unsupported_host_input("u128")),
            #[cfg(feature = "i8")]
            RuntimeHostInputValue::I8(value) => ValueCell::from_exact(value)?,
            #[cfg(not(feature = "i8"))]
            RuntimeHostInputValue::I8(_) => return Err(unsupported_host_input("i8")),
            #[cfg(feature = "i16")]
            RuntimeHostInputValue::I16(value) => ValueCell::from_exact(value)?,
            #[cfg(not(feature = "i16"))]
            RuntimeHostInputValue::I16(_) => return Err(unsupported_host_input("i16")),
            #[cfg(feature = "i32")]
            RuntimeHostInputValue::I32(value) => ValueCell::from_exact(value)?,
            #[cfg(not(feature = "i32"))]
            RuntimeHostInputValue::I32(_) => return Err(unsupported_host_input("i32")),
            #[cfg(feature = "i64")]
            RuntimeHostInputValue::I64(value) => ValueCell::from_exact(value)?,
            #[cfg(not(feature = "i64"))]
            RuntimeHostInputValue::I64(_) => return Err(unsupported_host_input("i64")),
            #[cfg(feature = "i128")]
            RuntimeHostInputValue::I128(value) => ValueCell::from_exact(value)?,
            #[cfg(not(feature = "i128"))]
            RuntimeHostInputValue::I128(_) => return Err(unsupported_host_input("i128")),
            #[cfg(feature = "f32")]
            RuntimeHostInputValue::F32(value) => ValueCell::from_exact(value)?,
            #[cfg(not(feature = "f32"))]
            RuntimeHostInputValue::F32(_) => return Err(unsupported_host_input("f32")),
            #[cfg(feature = "f64")]
            RuntimeHostInputValue::F64(value) => ValueCell::from_exact(value)?,
            #[cfg(not(feature = "f64"))]
            RuntimeHostInputValue::F64(_) => return Err(unsupported_host_input("f64")),
            RuntimeHostInputValue::Index(0) => {
                return Err(input_error(
                    "RuntimeHostInputValueInvalid",
                    "index host input values must be in 1..=max",
                ));
            }
            RuntimeHostInputValue::Index(value) => ValueCell::from_exact(value)?,
            #[cfg(feature = "matrix")]
            RuntimeHostInputValue::BoolMatrix {
                rows,
                columns,
                values,
            } => exact_matrix_input(host_input_matrix(values, rows, columns)?, rows, columns)?,
            #[cfg(not(feature = "matrix"))]
            RuntimeHostInputValue::BoolMatrix { .. } => {
                return Err(unsupported_host_input("bool matrix"));
            }
            #[cfg(feature = "matrix")]
            RuntimeHostInputValue::IndexMatrix {
                rows,
                columns,
                values,
            } => {
                validate_matrix_input(rows, columns, values.len())?;
                if values.contains(&0) {
                    return Err(input_error(
                        "RuntimeHostInputValueInvalid",
                        "index matrix host input values must be in 1..=max",
                    ));
                }
                exact_matrix_input(host_input_matrix(values, rows, columns)?, rows, columns)?
            }
            #[cfg(not(feature = "matrix"))]
            RuntimeHostInputValue::IndexMatrix { .. } => {
                return Err(unsupported_host_input("index matrix"));
            }
            #[cfg(all(feature = "matrix", feature = "f64"))]
            RuntimeHostInputValue::F64Matrix {
                rows,
                columns,
                values,
            } => exact_matrix_input(host_input_matrix(values, rows, columns)?, rows, columns)?,
            #[cfg(all(feature = "matrix", feature = "f32"))]
            RuntimeHostInputValue::F32Matrix {
                rows,
                columns,
                values,
            } => exact_matrix_input(host_input_matrix(values, rows, columns)?, rows, columns)?,
            #[cfg(not(all(feature = "matrix", feature = "f32")))]
            RuntimeHostInputValue::F32Matrix { .. } => {
                return Err(unsupported_host_input("f32 matrix"));
            }
            #[cfg(not(all(feature = "matrix", feature = "f64")))]
            RuntimeHostInputValue::F64Matrix { .. } => {
                return Err(unsupported_host_input("f64 matrix"));
            }
        };
        cell.snapshot()
    }

    /// Detaches a canonical numeric scalar or matrix for host input queues.
    pub fn from_numeric_value(value: &Value) -> MResult<Self> {
        match value.data() {
            #[cfg(feature = "f32")]
            ValueData::F32(value) => Ok(Self::F32(value.to_f32())),
            #[cfg(feature = "f64")]
            ValueData::F64(value) => Ok(Self::F64(value.to_f64())),
            #[cfg(feature = "matrix")]
            ValueData::Matrix(matrix) => {
                let schemas = value.schemas().ok_or_else(|| {
                    input_error(
                        "RuntimeNumericValueUnsupported",
                        "canonical matrix input does not retain its schema table",
                    )
                })?;
                let schema = schemas.entry(value.schema()).ok_or_else(|| {
                    input_error(
                        "RuntimeNumericValueUnsupported",
                        "canonical matrix input schema is absent",
                    )
                })?;
                let SchemaBody::Matrix { dimensions, .. } = schema.schema().body() else {
                    return Err(input_error(
                        "RuntimeNumericValueUnsupported",
                        "canonical matrix payload has a non-matrix schema",
                    ));
                };
                let [rows, columns] = dimensions.as_ref() else {
                    return Err(input_error(
                        "RuntimeNumericValueUnsupported",
                        "runtime host matrices must have exactly two dimensions",
                    ));
                };
                let rows =
                    usize::try_from(value.shape().resolve_dimension(rows).map_err(|error| {
                        input_error("RuntimeNumericValueUnsupported", format!("{error:?}"))
                    })?)
                    .map_err(|_| {
                        input_error(
                            "RuntimeNumericValueUnsupported",
                            "matrix row count does not fit the host platform",
                        )
                    })?;
                let columns =
                    usize::try_from(value.shape().resolve_dimension(columns).map_err(|error| {
                        input_error("RuntimeNumericValueUnsupported", format!("{error:?}"))
                    })?)
                    .map_err(|_| {
                        input_error(
                            "RuntimeNumericValueUnsupported",
                            "matrix column count does not fit the host platform",
                        )
                    })?;
                match matrix.elements() {
                    #[cfg(feature = "f32")]
                    SequenceView::F32(values) => Ok(Self::F32Matrix {
                        rows,
                        columns,
                        values: values.iter().map(|value| value.to_f32()).collect(),
                    }),
                    #[cfg(feature = "f64")]
                    SequenceView::F64(values) => Ok(Self::F64Matrix {
                        rows,
                        columns,
                        values: values.iter().map(|value| value.to_f64()).collect(),
                    }),
                    _ => Err(input_error(
                        "RuntimeNumericValueUnsupported",
                        "canonical matrix element type is not a supported numeric host input",
                    )),
                }
            }
            _ => Err(input_error(
                "RuntimeNumericValueUnsupported",
                format!(
                    "canonical value kind `{}` cannot become a detached runtime input",
                    value.data().kind()
                ),
            )),
        }
    }
}

#[cfg(not(all(
    feature = "bool",
    feature = "string",
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
    feature = "f64",
    feature = "matrix"
)))]
fn unsupported_host_input(name: &str) -> MechError {
    input_error(
        "RuntimeHostInputValueUnsupported",
        format!("{name} host input values require the corresponding runtime feature"),
    )
}

#[cfg(feature = "matrix")]
fn exact_matrix_input<T>(matrix: ValueMatrix<T>, rows: usize, columns: usize) -> MResult<ValueCell>
where
    T: mech_core::CanonicalMatrixElementBacking,
{
    validate_matrix_input(rows, columns, rows.saturating_mul(columns))?;
    macro_rules! exact {
        ($reference:expr) => {
            ValueCell::from_exact_matrix_ref($reference, rows, columns)
        };
    }
    #[allow(
        unreachable_patterns,
        reason = "dependency feature unification can expose matrix storage variants that this runtime profile did not enable"
    )]
    match matrix {
        #[cfg(feature = "matrix1")]
        ValueMatrix::Matrix1(reference) => exact!(reference),
        #[cfg(feature = "matrix2")]
        ValueMatrix::Matrix2(reference) => exact!(reference),
        #[cfg(feature = "matrix3")]
        ValueMatrix::Matrix3(reference) => exact!(reference),
        #[cfg(feature = "matrix4")]
        ValueMatrix::Matrix4(reference) => exact!(reference),
        #[cfg(feature = "matrix2x3")]
        ValueMatrix::Matrix2x3(reference) => exact!(reference),
        #[cfg(feature = "matrix3x2")]
        ValueMatrix::Matrix3x2(reference) => exact!(reference),
        #[cfg(feature = "row_vector2")]
        ValueMatrix::RowVector2(reference) => exact!(reference),
        #[cfg(feature = "row_vector3")]
        ValueMatrix::RowVector3(reference) => exact!(reference),
        #[cfg(feature = "row_vector4")]
        ValueMatrix::RowVector4(reference) => exact!(reference),
        #[cfg(feature = "vector2")]
        ValueMatrix::Vector2(reference) => exact!(reference),
        #[cfg(feature = "vector3")]
        ValueMatrix::Vector3(reference) => exact!(reference),
        #[cfg(feature = "vector4")]
        ValueMatrix::Vector4(reference) => exact!(reference),
        #[cfg(feature = "row_vectord")]
        ValueMatrix::RowDVector(reference) => exact!(reference),
        #[cfg(feature = "vectord")]
        ValueMatrix::DVector(reference) => exact!(reference),
        #[cfg(feature = "matrixd")]
        ValueMatrix::DMatrix(reference) => exact!(reference),
        _ => Err(input_error(
            "RuntimeHostInputValueUnsupported",
            "host matrix storage is unavailable in this runtime feature profile",
        )),
    }
}

#[cfg(feature = "matrix")]
fn host_input_matrix<T: mech_core::CanonicalMatrixElementBacking>(
    values: Vec<T>,
    rows: usize,
    columns: usize,
) -> MResult<ValueMatrix<T>> {
    validate_matrix_input(rows, columns, values.len())?;
    let mut physical = Vec::with_capacity(values.len());
    for column in 0..columns {
        for row in 0..rows {
            physical.push(values[row * columns + column].clone());
        }
    }
    Ok(ValueMatrix::from_vec(physical, rows, columns))
}

#[cfg(feature = "matrix")]
fn validate_matrix_input(rows: usize, columns: usize, value_count: usize) -> MResult<()> {
    let expected = rows.checked_mul(columns).ok_or_else(|| {
        input_error(
            "RuntimeHostInputMatrixInvalid",
            "host input matrix dimensions overflow",
        )
    })?;
    if rows == 0 || columns == 0 || expected != value_count {
        return Err(input_error(
            "RuntimeHostInputMatrixInvalid",
            "host input matrix dimensions must be nonzero and match the value count",
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeHostInputSource {
    base_uri: String,
    path: String,
}

impl RuntimeHostInputSource {
    pub fn new(base_uri: impl Into<String>, path: impl Into<String>) -> MResult<Self> {
        let raw_base_uri = base_uri.into();
        let base_uri = crate::resource::canonicalize_resource_base_uri(&raw_base_uri)?;
        let path = path.into().trim_matches('/').to_string();
        Ok(Self { base_uri, path })
    }

    pub fn base_uri(&self) -> &str {
        &self.base_uri
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeHostInputUpdate {
    pub source: RuntimeHostInputSource,
    pub value: RuntimeHostInputValue,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeHostInput {
    pub updates: Vec<RuntimeHostInputUpdate>,
    coalescing_group: Option<RuntimeHostInputCoalescingGroup>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimeHostInputCoalescingGroup {
    scope: String,
    sequence: u64,
}

impl RuntimeHostInput {
    pub fn new(updates: Vec<RuntimeHostInputUpdate>) -> MResult<Self> {
        let input = Self {
            updates,
            coalescing_group: None,
        };
        input.validate()?;
        Ok(input)
    }

    pub fn single(source: RuntimeHostInputSource, value: RuntimeHostInputValue) -> Self {
        Self {
            updates: vec![RuntimeHostInputUpdate { source, value }],
            coalescing_group: None,
        }
    }

    /// Keep packets with the same group eligible for latest-value coalescing,
    /// while preserving a resident-turn boundary between different groups.
    /// Drivers use this for event gestures whose state packets may collapse
    /// together, but whose distinct activations must never collapse together.
    /// The group is scoped to the packet's resource base URI so equal sequence
    /// numbers from independent host instances cannot merge.
    pub fn with_coalescing_group(mut self, group: u64) -> Self {
        let scope = self
            .updates
            .first()
            .expect("validated host inputs contain at least one update")
            .source
            .base_uri()
            .to_owned();
        self.coalescing_group = Some(RuntimeHostInputCoalescingGroup {
            scope,
            sequence: group,
        });
        self
    }

    #[cfg(feature = "resident-routing")]
    pub(crate) fn coalescing_group(&self) -> Option<&RuntimeHostInputCoalescingGroup> {
        self.coalescing_group.as_ref()
    }

    pub fn validate(&self) -> MResult<()> {
        if self.updates.is_empty() {
            return Err(input_error(
                "RuntimeHostInputEmpty",
                "host input packet must contain at least one update",
            ));
        }
        let mut sources = HashSet::with_capacity(self.updates.len());
        for update in &self.updates {
            if !sources.insert(update.source.clone()) {
                return Err(input_error(
                    "RuntimeHostInputDuplicateSource",
                    "host input packet contains duplicate sources",
                ));
            }
            if self
                .coalescing_group
                .as_ref()
                .is_some_and(|group| group.scope != update.source.base_uri())
            {
                return Err(input_error(
                    "RuntimeHostInputCoalescingScopeInvalid",
                    "coalesced host input packets must stay within one resource base URI",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct RuntimeHostInputOutcome {
    pub update_count: usize,
    pub ignored_update_count: usize,
    pub binding_count: usize,
    /// Resident turn produced by this drain batch. When several packets are
    /// coalesced, the single resident decision is attached to the final
    /// packet outcome; all earlier packet outcomes contain `None`.
    #[cfg(feature = "resident-routing")]
    pub resident_turn: Option<crate::ResidentExternalTurnOutcome>,
}

#[derive(Debug)]
pub(crate) struct RuntimeHostInputQueueState {
    pub(crate) queue: VecDeque<RuntimeHostInput>,
    pub(crate) capacity: usize,
    pub(crate) closed: bool,
}

impl RuntimeHostInputQueueState {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            capacity,
            closed: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RuntimeIngress {
    queue: RuntimeHostInputQueue,
}

impl RuntimeIngress {
    pub(crate) fn new(queue: RuntimeHostInputQueue) -> Self {
        Self { queue }
    }

    pub fn submit(&self, input: RuntimeHostInput) -> MResult<()> {
        input.validate()?;
        let mut guard = self.queue.lock().map_err(|_| {
            input_error(
                "RuntimeIngressUnavailable",
                "host input queue lock is poisoned",
            )
        })?;
        if guard.closed {
            return Err(input_error(
                "RuntimeIngressClosed",
                "host input queue is closed",
            ));
        }
        if guard.queue.len() >= guard.capacity {
            return Err(input_error(
                "RuntimeIngressFull",
                "host input queue is full",
            ));
        }
        guard.queue.push_back(input);
        Ok(())
    }

    /// Submits the newest packet for a stable source set. If a pending packet
    /// already contains exactly the same sources, it is replaced in place so
    /// unrelated packet ordering is preserved. Otherwise this behaves like an
    /// ordered submission and respects queue capacity.
    pub fn submit_latest(&self, input: RuntimeHostInput) -> MResult<()> {
        input.validate()?;
        let mut guard = self.queue.lock().map_err(|_| {
            input_error(
                "RuntimeIngressUnavailable",
                "host input queue lock is poisoned",
            )
        })?;
        if guard.closed {
            return Err(input_error(
                "RuntimeIngressClosed",
                "host input queue is closed",
            ));
        }
        if let Some(existing) = guard
            .queue
            .iter_mut()
            .find(|pending| same_input_sources(pending, &input))
        {
            *existing = input;
            return Ok(());
        }
        if guard.queue.len() >= guard.capacity {
            return Err(input_error(
                "RuntimeIngressFull",
                "host input queue is full",
            ));
        }
        guard.queue.push_back(input);
        Ok(())
    }

    pub fn is_closed(&self) -> MResult<bool> {
        Ok(self
            .queue
            .lock()
            .map_err(|_| {
                input_error(
                    "RuntimeIngressUnavailable",
                    "host input queue lock is poisoned",
                )
            })?
            .closed)
    }
}

fn same_input_sources(left: &RuntimeHostInput, right: &RuntimeHostInput) -> bool {
    left.updates.len() == right.updates.len()
        && left.updates.iter().all(|left| {
            right
                .updates
                .iter()
                .any(|right| right.source == left.source)
        })
}

/// Platform-neutral active host input driver.
///
/// `attach`, `start`, and `stop` are called on the runtime thread. `start`
/// must be idempotent or reject an already-live state clearly, and `stop` must
/// be idempotent. Background workers must not retain a runtime pointer or
/// `Value`; they submit only owned `RuntimeHostInput` packets through cloned
/// `RuntimeIngress` handles.
pub trait RuntimeHostInputDriver: std::fmt::Debug {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool;
    fn attach(&mut self, ingress: RuntimeIngress) -> MResult<()>;
    fn start(&mut self) -> MResult<()>;
    fn stop(&mut self) -> MResult<()>;
    fn is_live(&self) -> bool;
}

pub(crate) type RuntimeHostInputQueue = Arc<Mutex<RuntimeHostInputQueueState>>;

#[derive(Debug, Clone)]
pub struct RuntimeHostInputError {
    pub name: &'static str,
    pub message: String,
}
impl MechErrorKind for RuntimeHostInputError {
    fn name(&self) -> &str {
        self.name
    }
    fn message(&self) -> String {
        self.message.clone()
    }
}
pub(crate) fn input_error(name: &'static str, message: impl Into<String>) -> MechError {
    MechError::new(
        RuntimeHostInputError {
            name,
            message: message.into(),
        },
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(path: &str) -> RuntimeHostInputSource {
        RuntimeHostInputSource::new("test://clock/ticks/", path).unwrap()
    }

    fn packet(path: &str, value: f64) -> RuntimeHostInput {
        RuntimeHostInput::single(source(path), RuntimeHostInputValue::F64(value))
    }

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn source_constructor_canonicalizes_base_and_path() {
        let source = RuntimeHostInputSource::new("test://clock/ticks/", "/value/").unwrap();
        assert_eq!(source.base_uri(), "test://clock/ticks");
        assert_eq!(source.path(), "value");
    }

    #[test]
    fn source_constructor_rejects_invalid_resource_uris() {
        assert!(RuntimeHostInputSource::new("clock/ticks", "value").is_err());
        assert!(RuntimeHostInputSource::new("://clock", "value").is_err());
        assert!(RuntimeHostInputSource::new("test://", "value").is_err());
    }

    #[test]
    fn host_input_transport_is_send_sync() {
        assert_send_sync::<RuntimeHostInputValue>();
        assert_send_sync::<RuntimeHostInput>();
        assert_send_sync::<RuntimeIngress>();
    }

    #[test]
    fn explicit_unit_is_the_canonical_empty_tuple() {
        let unit = RuntimeHostInputValue::Unit.into_value().unwrap();
        assert!(matches!(unit.data(), mech_core::ValueData::Tuple(values) if values.is_empty()));
    }

    #[cfg(all(feature = "matrix", feature = "f32", feature = "f64"))]
    #[test]
    fn nonsquare_numeric_matrices_round_trip_in_row_major_order() {
        let f32_values = [
            1.0_f32.to_bits(),
            (-0.0_f32).to_bits(),
            3.5_f32.to_bits(),
            (-4.25_f32).to_bits(),
            5.0_f32.to_bits(),
            6.75_f32.to_bits(),
        ];
        let f32_original = RuntimeHostInputValue::F32Matrix {
            rows: 2,
            columns: 3,
            values: f32_values.map(f32::from_bits).to_vec(),
        }
        .into_value()
        .unwrap();
        let f32_schema = f32_original.schema_key();
        let f32_detached = RuntimeHostInputValue::from_numeric_value(&f32_original).unwrap();
        let RuntimeHostInputValue::F32Matrix {
            rows,
            columns,
            values,
        } = &f32_detached
        else {
            panic!("f32 matrix did not detach as a matrix")
        };
        assert_eq!((*rows, *columns), (2, 3));
        assert_eq!(
            values
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            f32_values,
        );
        let f32_round_trip = f32_detached.into_value().unwrap();
        assert_eq!(f32_round_trip.schema_key(), f32_schema);
        assert_eq!(f32_round_trip.shape().parameter_values(), &[2, 3]);
        let ValueData::Matrix(matrix) = f32_round_trip.data() else {
            panic!("f32 round trip did not produce a matrix")
        };
        let SequenceView::F32(values) = matrix.elements() else {
            panic!("f32 round trip changed its element schema")
        };
        assert_eq!(
            values
                .iter()
                .map(|value| value.to_f32().to_bits())
                .collect::<Vec<_>>(),
            f32_values,
        );

        let f64_values = [
            1.0_f64.to_bits(),
            (-0.0_f64).to_bits(),
            3.5_f64.to_bits(),
            (-4.25_f64).to_bits(),
            5.0_f64.to_bits(),
            6.75_f64.to_bits(),
        ];
        let f64_original = RuntimeHostInputValue::F64Matrix {
            rows: 2,
            columns: 3,
            values: f64_values.map(f64::from_bits).to_vec(),
        }
        .into_value()
        .unwrap();
        let f64_schema = f64_original.schema_key();
        let f64_detached = RuntimeHostInputValue::from_numeric_value(&f64_original).unwrap();
        let RuntimeHostInputValue::F64Matrix {
            rows,
            columns,
            values,
        } = &f64_detached
        else {
            panic!("f64 matrix did not detach as a matrix")
        };
        assert_eq!((*rows, *columns), (2, 3));
        assert_eq!(
            values
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            f64_values,
        );
        let f64_round_trip = f64_detached.into_value().unwrap();
        assert_eq!(f64_round_trip.schema_key(), f64_schema);
        assert_eq!(f64_round_trip.shape().parameter_values(), &[2, 3]);
        let ValueData::Matrix(matrix) = f64_round_trip.data() else {
            panic!("f64 round trip did not produce a matrix")
        };
        let SequenceView::F64(values) = matrix.elements() else {
            panic!("f64 round trip changed its element schema")
        };
        assert_eq!(
            values
                .iter()
                .map(|value| value.to_f64().to_bits())
                .collect::<Vec<_>>(),
            f64_values,
        );
    }

    #[test]
    fn detached_indexes_are_one_based() {
        assert!(RuntimeHostInputValue::Index(0).into_value().is_err());
        assert!(
            RuntimeHostInputValue::IndexMatrix {
                rows: 1,
                columns: 2,
                values: vec![1, 0],
            }
            .into_value()
            .is_err()
        );
        assert!(RuntimeHostInputValue::Index(1).into_value().is_ok());
    }

    #[test]
    fn cloned_ingress_preserves_fifo_and_enforces_capacity() {
        let queue = Arc::new(Mutex::new(RuntimeHostInputQueueState::new(2)));
        let ingress = RuntimeIngress::new(queue.clone());
        let cloned = ingress.clone();
        ingress.submit(packet("a", 1.0)).unwrap();
        cloned.submit(packet("b", 2.0)).unwrap();
        let error = format!("{:?}", ingress.submit(packet("c", 3.0)).unwrap_err());
        assert!(error.contains("RuntimeIngressFull"));

        let mut guard = queue.lock().unwrap();
        assert_eq!(
            guard.queue.pop_front().unwrap().updates[0].source.path(),
            "a"
        );
        assert_eq!(
            guard.queue.pop_front().unwrap().updates[0].source.path(),
            "b"
        );
    }

    #[test]
    fn latest_submission_replaces_same_sources_without_reordering_unrelated_packets() {
        let queue = Arc::new(Mutex::new(RuntimeHostInputQueueState::new(2)));
        let ingress = RuntimeIngress::new(queue.clone());
        ingress.submit(packet("a", 1.0)).unwrap();
        ingress.submit(packet("b", 2.0)).unwrap();
        ingress.submit_latest(packet("a", 3.0)).unwrap();

        let mut guard = queue.lock().unwrap();
        let first = guard.queue.pop_front().unwrap();
        let second = guard.queue.pop_front().unwrap();
        assert_eq!(first.updates[0].source.path(), "a");
        assert!(matches!(
            first.updates[0].value,
            RuntimeHostInputValue::F64(3.0)
        ));
        assert_eq!(second.updates[0].source.path(), "b");
    }

    #[test]
    fn closed_ingress_rejects_new_submissions_but_preserves_queued_packets() {
        let queue = Arc::new(Mutex::new(RuntimeHostInputQueueState::new(2)));
        let ingress = RuntimeIngress::new(queue.clone());
        ingress.submit(packet("a", 1.0)).unwrap();
        queue.lock().unwrap().closed = true;
        let error = format!("{:?}", ingress.submit(packet("b", 2.0)).unwrap_err());
        assert!(error.contains("RuntimeIngressClosed"));
        assert!(ingress.is_closed().unwrap());
        assert_eq!(queue.lock().unwrap().queue.len(), 1);
    }

    #[test]
    fn empty_packets_and_duplicate_sources_are_rejected() {
        assert!(RuntimeHostInput::new(Vec::new()).is_err());
        let duplicate = source("value");
        let error = format!(
            "{:?}",
            RuntimeHostInput::new(vec![
                RuntimeHostInputUpdate {
                    source: duplicate.clone(),
                    value: RuntimeHostInputValue::F64(1.0)
                },
                RuntimeHostInputUpdate {
                    source: duplicate,
                    value: RuntimeHostInputValue::F64(2.0)
                },
            ])
            .unwrap_err()
        );
        assert!(error.contains("RuntimeHostInputDuplicateSource"));
    }
}
