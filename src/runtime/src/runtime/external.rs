use std::{collections::BTreeMap, sync::Arc};

#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use mech_core::{
    ApplicationRequirement, ExecutionResourceRequest, FunctionSpecializer, GuardFunctionSafety,
    InitialSolvePolicy, MResult, MechError, MechErrorKind, Ref, ResourceIntent, Value, ValueKind,
};
use mech_engine::{ExternalResourceReadFunction, ExternalResourceWriteFunction};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExternalRequirementCatalog {
    requirements: BTreeMap<String, ApplicationRequirement>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PlannedResourceValue {
    Empty,
    #[cfg(feature = "bool")]
    Bool(bool),
    #[cfg(feature = "f64")]
    F64(f64),
    #[cfg(feature = "string")]
    String(String),
}

impl PlannedResourceValue {
    pub(crate) fn capture(value: &Value) -> MResult<Self> {
        match value {
            Value::Empty => Ok(Self::Empty),
            #[cfg(feature = "bool")]
            Value::Bool(value) => Ok(Self::Bool(*value.borrow())),
            #[cfg(feature = "f64")]
            Value::F64(value) => Ok(Self::F64(*value.borrow())),
            #[cfg(feature = "string")]
            Value::String(value) => Ok(Self::String(value.borrow().clone())),
            other => Err(MechError::new(
                RuntimeResourcePlannedValueUnsupported { kind: other.kind() },
                None,
            )),
        }
    }

    fn to_value(&self) -> Value {
        match self {
            Self::Empty => Value::Empty,
            #[cfg(feature = "bool")]
            Self::Bool(value) => Value::Bool(Ref::new(*value)),
            #[cfg(feature = "f64")]
            Self::F64(value) => Value::F64(Ref::new(*value)),
            #[cfg(feature = "string")]
            Self::String(value) => Value::String(Ref::new(value.clone())),
        }
    }
}

#[derive(Clone)]
pub(crate) struct ExecutedResourceValue {
    recreate: Arc<dyn Fn() -> Value + Send + Sync>,
}

impl ExecutedResourceValue {
    fn new(recreate: impl Fn() -> Value + Send + Sync + 'static) -> Self {
        Self {
            recreate: Arc::new(recreate),
        }
    }

    pub(crate) fn capture(value: &Value) -> MResult<Self> {
        Self::capture_detached(value.try_deep_snapshot()?)
    }

    fn capture_detached(value: Value) -> MResult<Self> {
        macro_rules! scalar {
            ($value:expr, $variant:ident) => {{
                let value = $value.borrow().clone();
                Ok(Self::new(move || Value::$variant(Ref::new(value.clone()))))
            }};
        }

        match value {
            #[cfg(feature = "u8")]
            Value::U8(value) => scalar!(value, U8),
            #[cfg(feature = "u16")]
            Value::U16(value) => scalar!(value, U16),
            #[cfg(feature = "u32")]
            Value::U32(value) => scalar!(value, U32),
            #[cfg(feature = "u64")]
            Value::U64(value) => scalar!(value, U64),
            #[cfg(feature = "u128")]
            Value::U128(value) => scalar!(value, U128),
            #[cfg(feature = "i8")]
            Value::I8(value) => scalar!(value, I8),
            #[cfg(feature = "i16")]
            Value::I16(value) => scalar!(value, I16),
            #[cfg(feature = "i32")]
            Value::I32(value) => scalar!(value, I32),
            #[cfg(feature = "i64")]
            Value::I64(value) => scalar!(value, I64),
            #[cfg(feature = "i128")]
            Value::I128(value) => scalar!(value, I128),
            #[cfg(feature = "f32")]
            Value::F32(value) => scalar!(value, F32),
            #[cfg(feature = "f64")]
            Value::F64(value) => scalar!(value, F64),
            #[cfg(any(feature = "string", feature = "variable_define"))]
            Value::String(value) => scalar!(value, String),
            #[cfg(any(feature = "bool", feature = "variable_define"))]
            Value::Bool(value) => scalar!(value, Bool),
            #[cfg(feature = "complex")]
            Value::C64(value) => scalar!(value, C64),
            #[cfg(feature = "rational")]
            Value::R64(value) => scalar!(value, R64),
            Value::Index(value) => scalar!(value, Index),
            #[cfg(all(feature = "matrix", feature = "bool"))]
            Value::MatrixBool(value) => Ok(capture_executed_matrix(value, Value::MatrixBool)),
            #[cfg(all(feature = "matrix", feature = "u8"))]
            Value::MatrixU8(value) => Ok(capture_executed_matrix(value, Value::MatrixU8)),
            #[cfg(all(feature = "matrix", feature = "u16"))]
            Value::MatrixU16(value) => Ok(capture_executed_matrix(value, Value::MatrixU16)),
            #[cfg(all(feature = "matrix", feature = "u32"))]
            Value::MatrixU32(value) => Ok(capture_executed_matrix(value, Value::MatrixU32)),
            #[cfg(all(feature = "matrix", feature = "u64"))]
            Value::MatrixU64(value) => Ok(capture_executed_matrix(value, Value::MatrixU64)),
            #[cfg(all(feature = "matrix", feature = "u128"))]
            Value::MatrixU128(value) => Ok(capture_executed_matrix(value, Value::MatrixU128)),
            #[cfg(all(feature = "matrix", feature = "i8"))]
            Value::MatrixI8(value) => Ok(capture_executed_matrix(value, Value::MatrixI8)),
            #[cfg(all(feature = "matrix", feature = "i16"))]
            Value::MatrixI16(value) => Ok(capture_executed_matrix(value, Value::MatrixI16)),
            #[cfg(all(feature = "matrix", feature = "i32"))]
            Value::MatrixI32(value) => Ok(capture_executed_matrix(value, Value::MatrixI32)),
            #[cfg(all(feature = "matrix", feature = "i64"))]
            Value::MatrixI64(value) => Ok(capture_executed_matrix(value, Value::MatrixI64)),
            #[cfg(all(feature = "matrix", feature = "i128"))]
            Value::MatrixI128(value) => Ok(capture_executed_matrix(value, Value::MatrixI128)),
            #[cfg(all(feature = "matrix", feature = "f32"))]
            Value::MatrixF32(value) => Ok(capture_executed_matrix(value, Value::MatrixF32)),
            #[cfg(all(feature = "matrix", feature = "f64"))]
            Value::MatrixF64(value) => Ok(capture_executed_matrix(value, Value::MatrixF64)),
            #[cfg(all(feature = "matrix", feature = "string"))]
            Value::MatrixString(value) => Ok(capture_executed_matrix(value, Value::MatrixString)),
            #[cfg(all(feature = "matrix", feature = "rational"))]
            Value::MatrixR64(value) => Ok(capture_executed_matrix(value, Value::MatrixR64)),
            #[cfg(all(feature = "matrix", feature = "complex"))]
            Value::MatrixC64(value) => Ok(capture_executed_matrix(value, Value::MatrixC64)),
            Value::Typed(value, kind) => {
                let inner = Self::capture_detached(*value)?;
                Ok(Self::new(move || {
                    Value::Typed(Box::new(inner.to_value()), kind.clone())
                }))
            }
            Value::Empty => Ok(Self::new(|| Value::Empty)),
            other => Err(MechError::new(
                RuntimeResourceExecutedValueUnsupported { kind: other.kind() },
                None,
            )),
        }
    }

    fn to_value(&self) -> Value {
        (self.recreate)()
    }
}

#[cfg(feature = "matrix")]
fn capture_executed_matrix<T>(
    matrix: Matrix<T>,
    wrap: fn(Matrix<T>) -> Value,
) -> ExecutedResourceValue
where
    T: Clone + Send + Sync + 'static,
{
    macro_rules! matrix {
        ($value:expr, $variant:ident) => {{
            let value = $value.borrow().clone();
            ExecutedResourceValue::new(move || wrap(Matrix::$variant(Ref::new(value.clone()))))
        }};
    }

    match matrix {
        #[cfg(feature = "matrix1")]
        Matrix::Matrix1(value) => matrix!(value, Matrix1),
        #[cfg(feature = "matrix2")]
        Matrix::Matrix2(value) => matrix!(value, Matrix2),
        #[cfg(feature = "matrix3")]
        Matrix::Matrix3(value) => matrix!(value, Matrix3),
        #[cfg(feature = "matrix4")]
        Matrix::Matrix4(value) => matrix!(value, Matrix4),
        #[cfg(feature = "matrix2x3")]
        Matrix::Matrix2x3(value) => matrix!(value, Matrix2x3),
        #[cfg(feature = "matrix3x2")]
        Matrix::Matrix3x2(value) => matrix!(value, Matrix3x2),
        #[cfg(feature = "row_vector2")]
        Matrix::RowVector2(value) => matrix!(value, RowVector2),
        #[cfg(feature = "row_vector3")]
        Matrix::RowVector3(value) => matrix!(value, RowVector3),
        #[cfg(feature = "row_vector4")]
        Matrix::RowVector4(value) => matrix!(value, RowVector4),
        #[cfg(feature = "vector2")]
        Matrix::Vector2(value) => matrix!(value, Vector2),
        #[cfg(feature = "vector3")]
        Matrix::Vector3(value) => matrix!(value, Vector3),
        #[cfg(feature = "vector4")]
        Matrix::Vector4(value) => matrix!(value, Vector4),
        #[cfg(feature = "row_vectord")]
        Matrix::RowDVector(value) => matrix!(value, RowDVector),
        #[cfg(feature = "vectord")]
        Matrix::DVector(value) => matrix!(value, DVector),
        #[cfg(feature = "matrixd")]
        Matrix::DMatrix(value) => matrix!(value, DMatrix),
        #[allow(unreachable_patterns)]
        _ => unreachable!("matrix storage is not enabled in this runtime profile"),
    }
}

#[derive(Clone)]
pub(crate) enum RuntimeResourceInitialValue {
    Executed(ExecutedResourceValue),
    Planned(PlannedResourceValue),
}

impl RuntimeResourceInitialValue {
    pub(crate) fn executed(value: &Value) -> MResult<Self> {
        Ok(Self::Executed(ExecutedResourceValue::capture(value)?))
    }

    pub(crate) fn planned(value: &Value) -> MResult<Self> {
        Ok(Self::Planned(PlannedResourceValue::capture(value)?))
    }

    fn to_value(&self) -> Value {
        match self {
            Self::Executed(value) => value.to_value(),
            Self::Planned(value) => value.to_value(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct RuntimeResourceReadSpecializer {
    pub interpreter_id: u64,
    pub request: ExecutionResourceRequest,
    pub initial: RuntimeResourceInitialValue,
}

fn assert_send_sync<T: Send + Sync>() {}

const _: fn() = assert_send_sync::<ExecutedResourceValue>;
const _: fn() = assert_send_sync::<RuntimeResourceReadSpecializer>;

impl FunctionSpecializer for RuntimeResourceReadSpecializer {
    fn guard_safety(&self) -> GuardFunctionSafety {
        GuardFunctionSafety::Unsupported
    }

    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn mech_core::MechFunction>> {
        require_external_arity(arguments, 0)?;
        Ok(Box::new(ExternalResourceReadFunction {
            interpreter_id: self.interpreter_id,
            request: self.request.clone(),
            output: Ref::new(self.initial.to_value()),
            initial_solve_policy: InitialSolvePolicy::PreserveSpecializedOutput,
        }))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RuntimeResourceWriteSpecializer {
    pub request: ExecutionResourceRequest,
}

impl FunctionSpecializer for RuntimeResourceWriteSpecializer {
    fn guard_safety(&self) -> GuardFunctionSafety {
        GuardFunctionSafety::Unsupported
    }

    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn mech_core::MechFunction>> {
        require_external_arity(arguments, 1)?;
        Ok(Box::new(ExternalResourceWriteFunction {
            request: self.request.clone(),
            input: arguments[0].clone(),
            output: Ref::new(Value::Empty),
            initial_solve_policy: InitialSolvePolicy::PreserveSpecializedOutput,
        }))
    }
}

fn require_external_arity(arguments: &[Value], expected: usize) -> MResult<()> {
    if arguments.len() == expected {
        return Ok(());
    }
    Err(MechError::new(
        ExternalOperationArityMismatch {
            expected,
            found: arguments.len(),
        },
        None,
    ))
}

impl ExternalRequirementCatalog {
    pub fn register(&mut self, requirement: ApplicationRequirement) -> MResult<String> {
        let name = hidden_external_operation_name(&requirement)?;
        if let Some(existing) = self.requirements.get(&name) {
            if existing == &requirement {
                return Ok(name);
            }
            return Err(MechError::new(
                ExternalRequirementDigestCollision {
                    name,
                    existing: existing.clone(),
                    incoming: requirement,
                },
                None,
            ));
        }
        self.requirements.insert(name.clone(), requirement);
        Ok(name)
    }

    pub fn get(&self, name: &str) -> Option<&ApplicationRequirement> {
        self.requirements.get(name)
    }

    pub fn requirements(
        &self,
    ) -> impl ExactSizeIterator<Item = (&str, &ApplicationRequirement)> + '_ {
        self.requirements
            .iter()
            .map(|(name, requirement)| (name.as_str(), requirement))
    }

    pub fn len(&self) -> usize {
        self.requirements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.requirements.is_empty()
    }
}

pub fn hidden_external_operation_name(requirement: &ApplicationRequirement) -> MResult<String> {
    let (prefix, kind, intent, delivery, operation, context, base_uri, path) = match requirement {
        ApplicationRequirement::HostFunction(request) => (
            "host-call",
            1_u8,
            0_u8,
            0_u8,
            "",
            "",
            request.name.as_str(),
            "",
        ),
        ApplicationRequirement::Resource(request) => {
            let prefix = match request.intent {
                ResourceIntent::Read => "resource-read",
                ResourceIntent::Assign => "resource-write",
                ResourceIntent::Send => "resource-send",
            };
            (
                prefix,
                2_u8,
                request.intent as u8,
                request.delivery as u8,
                request.operation.as_str(),
                request.context_name.as_str(),
                request.base_uri.as_str(),
                request.path.as_str(),
            )
        }
    };

    let mut canonical = Vec::new();
    canonical.extend_from_slice(&[kind, intent, delivery]);
    append_string(&mut canonical, operation)?;
    append_string(&mut canonical, context)?;
    append_string(&mut canonical, base_uri)?;
    append_string(&mut canonical, path)?;

    let digest = blake3::hash(&canonical);
    Ok(format!("mech/external/{prefix}/{}", digest.to_hex()))
}

fn append_string(bytes: &mut Vec<u8>, value: &str) -> MResult<()> {
    let length = u32::try_from(value.len()).map_err(|_| {
        MechError::new(
            ExternalRequirementCanonicalizationOverflow {
                field_length: value.len(),
            },
            None,
        )
    })?;
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalRequirementDigestCollision {
    pub name: String,
    pub existing: ApplicationRequirement,
    pub incoming: ApplicationRequirement,
}

impl MechErrorKind for ExternalRequirementDigestCollision {
    fn name(&self) -> &str {
        "ExternalRequirementDigestCollision"
    }

    fn message(&self) -> String {
        format!(
            "Hidden external operation `{}` maps to distinct requirements: {:?} and {:?}.",
            self.name, self.existing, self.incoming,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalRequirementCanonicalizationOverflow {
    pub field_length: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeResourcePlannedValueUnsupported {
    pub kind: ValueKind,
}

impl MechErrorKind for RuntimeResourcePlannedValueUnsupported {
    fn name(&self) -> &str {
        "RuntimeResourcePlannedValueUnsupported"
    }

    fn message(&self) -> String {
        format!(
            "planned resource value kind {:?} cannot be retained by a source specializer",
            self.kind,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeResourceExecutedValueUnsupported {
    pub kind: ValueKind,
}

impl MechErrorKind for RuntimeResourceExecutedValueUnsupported {
    fn name(&self) -> &str {
        "RuntimeResourceExecutedValueUnsupported"
    }

    fn message(&self) -> String {
        format!(
            "executed resource value kind {:?} cannot be retained as a stable source value",
            self.kind,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalOperationArityMismatch {
    pub expected: usize,
    pub found: usize,
}

impl MechErrorKind for ExternalOperationArityMismatch {
    fn name(&self) -> &str {
        "ExternalOperationArityMismatch"
    }

    fn message(&self) -> String {
        format!(
            "hidden external operation expected {} arguments, found {}",
            self.expected, self.found,
        )
    }
}

impl MechErrorKind for ExternalRequirementCanonicalizationOverflow {
    fn name(&self) -> &str {
        "ExternalRequirementCanonicalizationOverflow"
    }

    fn message(&self) -> String {
        format!(
            "External requirement field length {} cannot be represented as u32.",
            self.field_length,
        )
    }
}

#[cfg(test)]
mod tests {
    use mech_core::{
        ApplicationRequirement, ExecutionHostFunctionRequest, ExecutionResourceRequest,
        FunctionSpecializer, InitialSolvePolicy, ResourceDelivery, ResourceIntent, Value,
    };

    use super::{
        ExternalRequirementCatalog, RuntimeResourceWriteSpecializer, hidden_external_operation_name,
    };

    fn request(intent: ResourceIntent) -> ApplicationRequirement {
        ApplicationRequirement::Resource(ExecutionResourceRequest {
            base_uri: "cli://stdout".into(),
            path: "line".into(),
            context_name: "out".into(),
            operation: "write".into(),
            intent,
            delivery: ResourceDelivery::Snapshot,
        })
    }

    #[test]
    fn names_are_stable_and_operation_specific() {
        let write = hidden_external_operation_name(&request(ResourceIntent::Assign)).unwrap();
        let send = hidden_external_operation_name(&request(ResourceIntent::Send)).unwrap();
        assert!(write.starts_with("mech/external/resource-write/"));
        assert!(send.starts_with("mech/external/resource-send/"));
        assert_eq!(write.len(), "mech/external/resource-write/".len() + 64);
        assert_ne!(write, send);
        assert_eq!(
            write,
            hidden_external_operation_name(&request(ResourceIntent::Assign)).unwrap(),
        );
    }

    #[test]
    fn exact_requirement_reuse_is_idempotent() {
        let mut catalog = ExternalRequirementCatalog::default();
        let requirement = request(ResourceIntent::Assign);
        let first = catalog.register(requirement.clone()).unwrap();
        let second = catalog.register(requirement).unwrap();
        assert_eq!(first, second);
        assert_eq!(catalog.len(), 1);
    }

    #[test]
    fn host_names_put_the_function_name_in_the_primary_field() {
        let requirement = ApplicationRequirement::HostFunction(ExecutionHostFunctionRequest {
            name: "test/host".to_string(),
        });
        let actual = hidden_external_operation_name(&requirement).unwrap();

        let mut canonical = vec![1_u8, 0, 0];
        canonical.extend_from_slice(&0_u32.to_le_bytes());
        canonical.extend_from_slice(&0_u32.to_le_bytes());
        canonical.extend_from_slice(&9_u32.to_le_bytes());
        canonical.extend_from_slice(b"test/host");
        canonical.extend_from_slice(&0_u32.to_le_bytes());
        let expected = format!(
            "mech/external/host-call/{}",
            blake3::hash(&canonical).to_hex()
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn digest_reuse_rejects_a_different_full_requirement() {
        let mut catalog = ExternalRequirementCatalog::default();
        let incoming = request(ResourceIntent::Assign);
        let name = hidden_external_operation_name(&incoming).unwrap();
        catalog
            .requirements
            .insert(name, request(ResourceIntent::Send));

        let error = catalog.register(incoming).unwrap_err();
        assert_eq!(error.kind_name(), "ExternalRequirementDigestCollision");
    }

    #[test]
    fn source_resource_writes_preserve_their_planned_output_policy() {
        let ApplicationRequirement::Resource(request) = request(ResourceIntent::Send) else {
            unreachable!("test helper always returns a resource requirement");
        };
        let function = RuntimeResourceWriteSpecializer { request }
            .specialize(&[Value::Empty])
            .unwrap();

        assert_eq!(
            function.initial_solve_policy(),
            InitialSolvePolicy::PreserveSpecializedOutput
        );
    }
}
