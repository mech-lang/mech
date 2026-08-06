use std::collections::BTreeMap;

use mech_core::{
    ApplicationRequirement, ExecutionResourceRequest, FunctionSpecializer, GuardFunctionSafety,
    InitialSolvePolicy, MResult, MechError, MechErrorKind, Ref, ResourceIntent, Value,
    ValueSnapshotRecreator,
};
use mech_engine::{ExternalResourceReadFunction, ExternalResourceWriteFunction};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExternalRequirementCatalog {
    requirements: BTreeMap<String, ApplicationRequirement>,
}

#[derive(Clone)]
pub(crate) struct ExecutedResourceValue {
    recreate: ValueSnapshotRecreator,
}

impl ExecutedResourceValue {
    pub(crate) fn capture(value: &Value) -> MResult<Self> {
        Ok(Self {
            recreate: ValueSnapshotRecreator::capture(value)?,
        })
    }

    fn to_value(&self) -> Value {
        self.recreate.to_value()
    }
}

#[derive(Clone)]
pub(crate) struct RuntimeResourceInitialValue(ExecutedResourceValue);

impl RuntimeResourceInitialValue {
    pub(crate) fn executed(value: &Value) -> MResult<Self> {
        Ok(Self(ExecutedResourceValue::capture(value)?))
    }

    pub(crate) fn planned(value: &Value) -> MResult<Self> {
        Ok(Self(ExecutedResourceValue::capture(value)?))
    }

    fn to_value(&self) -> Value {
        self.0.to_value()
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
        FunctionSpecializer, InitialSolvePolicy, Ref, ResourceDelivery, ResourceIntent, Value,
    };

    use super::{
        ExternalRequirementCatalog, RuntimeResourceInitialValue, RuntimeResourceWriteSpecializer,
        hidden_external_operation_name,
    };

    fn assert_resource_snapshot_round_trip(value: Value) {
        let expected = value.try_deep_snapshot().unwrap();
        for captured in [
            RuntimeResourceInitialValue::executed(&value).unwrap(),
            RuntimeResourceInitialValue::planned(&value).unwrap(),
        ] {
            let actual = captured.to_value();
            assert_eq!(actual, expected);
            assert_ne!(actual.reactive_cell_ids(), value.reactive_cell_ids());
        }
    }

    #[cfg(all(feature = "u64", feature = "matrix", feature = "f64"))]
    #[test]
    fn planned_resource_snapshots_cover_numeric_and_matrix_values() {
        assert_resource_snapshot_round_trip(Value::U64(Ref::new(42)));
        assert_resource_snapshot_round_trip(Value::Index(Ref::new(7)));
        assert_resource_snapshot_round_trip(Value::MatrixF64(mech_core::matrix::Matrix::from_vec(
            vec![1.0, 2.0, 3.0, 4.0],
            2,
            2,
        )));
    }

    #[cfg(all(
        feature = "u8",
        feature = "u64",
        feature = "f64",
        feature = "string",
        feature = "matrix",
        feature = "map",
        feature = "set",
        feature = "record",
        feature = "table",
        feature = "tuple",
        feature = "atom",
        feature = "enum"
    ))]
    #[test]
    fn resource_snapshots_cover_nested_composite_values() {
        let names = Ref::new(mech_core::Dictionary::from([
            (mech_core::hash_str("status"), "status".to_owned()),
            (mech_core::hash_str("ready"), "ready".to_owned()),
        ]));
        let enumeration = Value::Enum(Ref::new(mech_core::MechEnum {
            id: mech_core::hash_str("status"),
            variants: vec![(mech_core::hash_str("ready"), Some(Value::U64(Ref::new(9))))],
            names,
        }));
        let table_column = mech_core::hash_str("samples");
        let table = Value::Table(Ref::new(mech_core::MechTable::from_parts(
            2,
            1,
            vec![(
                table_column,
                mech_core::ValueKind::U8,
                mech_core::matrix::Matrix::from_vec(
                    vec![Value::U8(Ref::new(1)), Value::U8(Ref::new(2))],
                    2,
                    1,
                ),
            )],
            vec![(table_column, "samples".to_owned())],
        )));
        let value = Value::Record(Ref::new(mech_core::MechRecord::new(vec![
            (
                "tuple",
                Value::Tuple(Ref::new(mech_core::MechTuple::from_vec(vec![
                    Value::String(Ref::new("value".to_owned())),
                    Value::MutableReference(Ref::new(Value::F64(Ref::new(3.5)))),
                ]))),
            ),
            (
                "map",
                Value::Map(Ref::new(mech_core::MechMap::from_vec(vec![(
                    Value::String(Ref::new("key".to_owned())),
                    Value::U64(Ref::new(5)),
                )]))),
            ),
            (
                "set",
                Value::Set(Ref::new(mech_core::MechSet::from_vec(vec![
                    Value::U8(Ref::new(6)),
                    Value::U8(Ref::new(7)),
                ]))),
            ),
            ("table", table),
            ("enum", enumeration),
            (
                "atom",
                Value::Atom(Ref::new(mech_core::MechAtom::from_name("ready"))),
            ),
            (
                "matrix-value",
                Value::MatrixValue(mech_core::matrix::Matrix::from_vec(
                    vec![Value::U8(Ref::new(8)), Value::U8(Ref::new(9))],
                    2,
                    1,
                )),
            ),
            (
                "typed",
                Value::Typed(
                    Box::new(Value::U64(Ref::new(10))),
                    mech_core::ValueKind::Option(Box::new(mech_core::ValueKind::U64)),
                ),
            ),
        ])));
        assert_resource_snapshot_round_trip(value);
    }

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
