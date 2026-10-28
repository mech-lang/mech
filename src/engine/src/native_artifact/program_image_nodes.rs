//! Private reconstruction of typed ports and original resource descriptions.
//! No resource acquisition, capability grant, lowering or scheduling occurs.
use super::values;
use crate::native_artifact::*;
use mech_core::*;
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value as Json;

fn wire<T: DeserializeOwned>(image: &Json) -> Result<T, String> {
    T::deserialize(image).map_err(|e| e.to_string())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Scalar {
    kind: String,
    schema: SchemaId,
    schema_key: SchemaKey,
    descriptor: Json,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Type {
    schema: SchemaId,
    definition: Json,
    scalar: Option<Scalar>,
}

pub(super) fn program_type(
    image: &Json,
    schemas: &SchemaTable,
) -> Result<NativeProgramType, String> {
    let image: Type = wire(image)?;
    let definition = values::schema(&image.definition)?;
    if schemas.get(image.schema) != Some(&definition) {
        return Err("image type differs from schema table".into());
    }
    let scalar = image
        .scalar
        .map(|scalar| {
            let kind = match (scalar.kind.as_str(), definition.body()) {
                ("Bool", SchemaBody::Bool) => NativeScalarKind::Bool,
                ("U8", SchemaBody::UnsignedInteger(IntegerWidth::W8)) => NativeScalarKind::U8,
                ("U16", SchemaBody::UnsignedInteger(IntegerWidth::W16)) => NativeScalarKind::U16,
                ("U32", SchemaBody::UnsignedInteger(IntegerWidth::W32)) => NativeScalarKind::U32,

                ("U64", SchemaBody::UnsignedInteger(IntegerWidth::W64)) => NativeScalarKind::U64,
                ("I64", SchemaBody::SignedInteger(IntegerWidth::W64)) => NativeScalarKind::I64,
                ("F64", SchemaBody::FloatingPoint(FloatWidth::W64)) => NativeScalarKind::F64,
                _ => return Err("image scalar kind differs from schema".to_owned()),
            };
            let descriptor = values::descriptor(&scalar.descriptor)?;
            if scalar.schema != image.schema
                || scalar.schema_key != definition.key()
                || descriptor.schema() != &definition
            {
                return Err("image scalar descriptor differs from schema".into());
            }
            Ok(NativeScalarType {
                kind,
                schema: scalar.schema,
                schema_key: scalar.schema_key,
                descriptor,
            })
        })
        .transpose()?;
    Ok(NativeProgramType {
        schema: image.schema,
        definition,
        scalar,
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    base_uri: String,
    path: String,
    context_name: String,
    operation: String,
    intent: ResourceIntent,
    delivery: ResourceDelivery,
}
pub(super) fn request(image: &Json) -> Result<ExecutionResourceRequest, String> {
    let r: Request = wire(image)?;
    Ok(ExecutionResourceRequest {
        base_uri: r.base_uri,
        path: r.path,
        context_name: r.context_name,
        operation: r.operation,
        intent: r.intent,
        delivery: r.delivery,
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    input: InputId,
    slot: CellSlotId,
    name: String,
    ty: Json,
    value: u32,
}
pub(super) fn input(image: &Json, schemas: &SchemaTable) -> Result<NativeProgramInput, String> {
    let r: Input = wire(image)?;
    Ok(NativeProgramInput {
        input: r.input,
        slot: r.slot,
        name: r.name,
        ty: program_type(&r.ty, schemas)?,
        value: NativeMatchValue(r.value),
    })
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Constant<'a> {
    value: u32,
    #[serde(borrow)]
    constant: &'a serde_json::value::RawValue,
}
pub(super) fn constant(
    image: &serde_json::value::RawValue,
    schemas: &SchemaTable,
) -> Result<NativeProgramConstant, String> {
    let r: Constant<'_> = serde_json::from_str(image.get()).map_err(|e| e.to_string())?;
    Ok(NativeProgramConstant {
        value: NativeMatchValue(r.value),
        constant: values::constant_bytes(r.constant.get().as_bytes(), schemas)?,
    })
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    node: NodeId,
    requirement: ApplicationRequirementId,
    request: Json,
    slot: CellSlotId,
    ty: Json,
    value: u32,
    ordinal: u32,
}
pub(super) fn observation(
    image: &Json,
    schemas: &SchemaTable,
) -> Result<NativeProgramObservation, String> {
    let r: Observation = wire(image)?;
    Ok(NativeProgramObservation {
        node: r.node,
        requirement: r.requirement,
        request: request(&r.request)?,
        slot: r.slot,
        ty: program_type(&r.ty, schemas)?,
        value: NativeMatchValue(r.value),
        ordinal: r.ordinal,
    })
}

#[derive(Deserialize)]
enum Source {
    Constant(u32),
    Slot(CellSlotId),
}
impl Source {
    fn into_source(self) -> crate::ArtifactSource {
        match self {
            Self::Constant(id) => crate::ArtifactSource::Constant(ConstantId::new(id)),
            Self::Slot(id) => crate::ArtifactSource::Slot(id),
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Effect {
    node: NodeId,
    requirement: ApplicationRequirementId,
    request: Json,
    payload: Source,
    ty: Json,
    value: u32,
    ordinal: u32,
}
pub(super) fn effect(image: &Json, schemas: &SchemaTable) -> Result<NativeProgramEffect, String> {
    let r: Effect = wire(image)?;
    Ok(NativeProgramEffect {
        node: r.node,
        requirement: r.requirement,
        request: request(&r.request)?,
        payload: r.payload.into_source(),
        ty: program_type(&r.ty, schemas)?,
        value: NativeMatchValue(r.value),
        ordinal: r.ordinal,
    })
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    lexical_name: String,
    artifact_source: Source,
    storage: CellSlotId,
    output: OutputId,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Output {
    output: OutputId,
    slot: CellSlotId,
    name: String,
    interactive_binding: Option<Binding>,
    value: u32,
    ty: Json,
}
pub(super) fn output(image: &Json, schemas: &SchemaTable) -> Result<NativeProgramOutput, String> {
    let r: Output = wire(image)?;
    Ok(NativeProgramOutput {
        output: r.output,
        slot: r.slot,
        name: r.name,
        interactive_binding: r
            .interactive_binding
            .map(|b| crate::InteractiveSymbolBinding {
                lexical_name: b.lexical_name,
                artifact_source: b.artifact_source.into_source(),
                storage: b.storage,
                output: b.output,
            }),
        value: NativeMatchValue(r.value),
        ty: program_type(&r.ty, schemas)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scalar_image() -> (SchemaTable, Json) {
        let definition = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::UnsignedInteger(IntegerWidth::W64),
        }
        .finalize()
        .unwrap();
        let table = values::schemas(&serde_json::json!([definition])).unwrap();
        let image = serde_json::json!({"schema":0,"definition":definition,
            "scalar":{"kind":"U64","schema":0,"schema_key":definition.key(),"descriptor":[definition,[]]}});
        (table, image)
    }
    #[test]
    fn typed_ports_roundtrip_and_reject_inconsistent_scalar_metadata() {
        let (table, ty) = scalar_image();
        let image = serde_json::json!({"input":0,"slot":1,"name":"n","ty":ty,"value":2});
        assert_eq!(
            serde_json::to_value(input(&image, &table).unwrap()).unwrap(),
            image
        );
        let mut bad = ty;
        bad["scalar"]["kind"] = "Bool".into();
        assert!(program_type(&bad, &table).is_err());
        bad["scalar"]["kind"] = "U64".into();
        bad["scalar"]["schema"] = 1.into();
        assert!(program_type(&bad, &table).is_err());
    }
    #[test]
    fn resource_description_roundtrips_without_granting_authority() {
        let original = ExecutionResourceRequest {
            base_uri: "cli/stdin".into(),
            path: "text".into(),
            context_name: "in".into(),
            operation: "read".into(),
            intent: ResourceIntent::Read,
            delivery: ResourceDelivery::Snapshot,
        };
        let image = serde_json::to_value(&original).unwrap();
        assert_eq!(request(&image).unwrap(), original);
        let mut bad = image;
        bad["intent"] = "AuthorizeEverything".into();
        assert!(request(&bad).is_err());
    }
}
