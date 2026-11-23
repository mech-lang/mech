//! Private reconstruction of typed ports and original resource descriptions.
//! No resource acquisition, capability grant, lowering or scheduling occurs.
use super::values;
use crate::native_artifact::*;
use mech_core::*;
use serde::{Deserialize, Deserializer};
#[cfg(test)]
use serde_json::Value as Json;

fn wire<'de, T: Deserialize<'de>, D: Deserializer<'de>>(image: D) -> Result<T, String> {
    T::deserialize(image).map_err(|e| e.to_string())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.Scalar")]
struct Scalar<'a> {
    kind: String,
    schema: SchemaId,
    schema_key: SchemaKey,
    #[serde(borrow)]
    descriptor: BuildOwnedImageRecord<'a>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.Type")]
struct Type<'a> {
    schema: SchemaId,
    #[serde(borrow)]
    definition: BuildOwnedImageRecord<'a>,
    #[serde(borrow)]
    scalar: Option<Scalar<'a>>,
}

pub(super) fn program_type<'de, D: Deserializer<'de>>(
    image: D,
    schemas: &SchemaTable,
) -> Result<NativeProgramType, String> {
    let image: Type<'_> = wire(image)?;
    let entry = schemas
        .entry(image.schema)
        .ok_or("image type schema is absent from schema table")?;
    let definition = entry.schema();
    values::matching_schema_record(image.definition, definition)?;
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
            let descriptor = values::descriptor_record(scalar.descriptor, definition)?;
            if scalar.schema != image.schema || scalar.schema_key != entry.key() {
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
        definition: std::sync::Arc::new(definition.clone()),
        scalar: scalar.map(std::sync::Arc::new),
    })
}

/// Per-image canonical type rows. Only immutable, safely constructed types are
/// shared. No live shapes, Values, owners, visits or invocation caches live here.
pub(super) struct TypeTable<'a> {
    schemas: &'a SchemaTable,
    entries: std::collections::HashMap<(SchemaId, u8), NativeProgramType>,
}
impl<'a> TypeTable<'a> {
    pub(super) fn new(schemas: &'a SchemaTable) -> Self {
        Self {
            schemas,
            entries: std::collections::HashMap::new(),
        }
    }
    pub(super) fn read<'de, D: Deserializer<'de>>(
        &mut self,
        image: D,
    ) -> Result<NativeProgramType, String> {
        let (schema, tag): (SchemaId, u8) = wire(image)?;
        if let Some(ty) = self.entries.get(&(schema, tag)) {
            return Ok(ty.clone());
        }
        let entry = self
            .schemas
            .entry(schema)
            .ok_or("table type schema is absent")?;
        let definition = entry.schema();
        let kind = match (tag, definition.body()) {
            (0, _) => None,
            (1, SchemaBody::Bool) => Some(NativeScalarKind::Bool),
            (2, SchemaBody::UnsignedInteger(IntegerWidth::W8)) => Some(NativeScalarKind::U8),
            (3, SchemaBody::UnsignedInteger(IntegerWidth::W16)) => Some(NativeScalarKind::U16),
            (4, SchemaBody::UnsignedInteger(IntegerWidth::W32)) => Some(NativeScalarKind::U32),
            (5, SchemaBody::UnsignedInteger(IntegerWidth::W64)) => Some(NativeScalarKind::U64),
            (6, SchemaBody::SignedInteger(IntegerWidth::W64)) => Some(NativeScalarKind::I64),
            (7, SchemaBody::FloatingPoint(FloatWidth::W64)) => Some(NativeScalarKind::F64),
            _ => return Err("table scalar kind differs from canonical schema".into()),
        };
        let scalar = kind
            .map(|kind| {
                if !definition.dimension_parameters().is_empty() {
                    return Err("table scalar schema has managed dimensions".to_owned());
                }
                let shape = definition
                    .instantiate_shape(Box::new([]))
                    .map_err(|e| format!("table scalar shape differs: {e:?}"))?;
                let descriptor = ResolvedValueDescriptor::from_schema(definition.clone(), shape)
                    .map_err(|e| e.to_string())?;
                Ok(std::sync::Arc::new(NativeScalarType {
                    kind,
                    schema,
                    schema_key: entry.key(),
                    descriptor,
                }))
            })
            .transpose()?;
        let ty = NativeProgramType {
            schema,
            definition: std::sync::Arc::new(definition.clone()),
            scalar,
        };
        self.entries.insert((schema, tag), ty.clone());
        Ok(ty)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.Request")]
struct Request {
    base_uri: String,
    path: String,
    context_name: String,
    operation: String,
    intent: ResourceIntent,
    delivery: ResourceDelivery,
}
pub(super) fn request<'de, D: Deserializer<'de>>(
    image: D,
) -> Result<ExecutionResourceRequest, String> {
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
#[serde(deny_unknown_fields, rename = "mech.native.image.Input")]
struct Input<'a> {
    input: InputId,
    slot: CellSlotId,
    name: String,
    #[serde(borrow)]
    ty: BuildOwnedImageRecord<'a>,
    value: u32,
}
pub(super) fn input<'de, D: Deserializer<'de>>(
    image: D,
    schemas: &SchemaTable,
) -> Result<NativeProgramInput, String> {
    let r: Input<'_> = wire(image)?;
    Ok(NativeProgramInput {
        input: r.input,
        slot: r.slot,
        name: r.name,
        ty: program_type(&mut r.ty.reader(), schemas)?,
        value: NativeMatchValue(r.value),
    })
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.Constant")]
struct Constant<'a> {
    value: u32,
    #[serde(borrow)]
    constant: BuildOwnedImageRecord<'a>,
}
pub(super) fn constant(
    image: BuildOwnedImageRecord<'_>,
    schemas: &SchemaTable,
) -> Result<NativeProgramConstant, String> {
    let r: Constant<'_> = image.decode().map_err(|e| e.to_string())?;
    Ok(NativeProgramConstant {
        value: NativeMatchValue(r.value),
        constant: values::constant_record(r.constant, schemas)?,
    })
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.Observation")]
struct Observation<'a> {
    node: NodeId,
    requirement: ApplicationRequirementId,
    #[serde(borrow)]
    request: BuildOwnedImageRecord<'a>,
    slot: CellSlotId,
    #[serde(borrow)]
    ty: BuildOwnedImageRecord<'a>,
    value: u32,
    ordinal: u32,
}
pub(super) fn observation<'de, D: Deserializer<'de>>(
    image: D,
    schemas: &SchemaTable,
) -> Result<NativeProgramObservation, String> {
    let r: Observation<'_> = wire(image)?;
    Ok(NativeProgramObservation {
        node: r.node,
        requirement: r.requirement,
        request: request(&mut r.request.reader())?,
        slot: r.slot,
        ty: program_type(&mut r.ty.reader(), schemas)?,
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
#[serde(deny_unknown_fields, rename = "mech.native.image.Effect")]
struct Effect<'a> {
    node: NodeId,
    requirement: ApplicationRequirementId,
    #[serde(borrow)]
    request: BuildOwnedImageRecord<'a>,
    payload: Source,
    #[serde(borrow)]
    ty: BuildOwnedImageRecord<'a>,
    value: u32,
    ordinal: u32,
}
pub(super) fn effect<'de, D: Deserializer<'de>>(
    image: D,
    schemas: &SchemaTable,
) -> Result<NativeProgramEffect, String> {
    let r: Effect<'_> = wire(image)?;
    Ok(NativeProgramEffect {
        node: r.node,
        requirement: r.requirement,
        request: request(&mut r.request.reader())?,
        payload: r.payload.into_source(),
        ty: program_type(&mut r.ty.reader(), schemas)?,
        value: NativeMatchValue(r.value),
        ordinal: r.ordinal,
    })
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.Binding")]
struct Binding {
    lexical_name: String,
    artifact_source: Source,
    storage: CellSlotId,
    output: OutputId,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.Output")]
struct Output<'a> {
    output: OutputId,
    slot: CellSlotId,
    name: String,
    interactive_binding: Option<Binding>,
    value: u32,
    #[serde(borrow)]
    ty: BuildOwnedImageRecord<'a>,
}
pub(super) fn output<'de, D: Deserializer<'de>>(
    image: D,
    schemas: &SchemaTable,
) -> Result<NativeProgramOutput, String> {
    let r: Output<'_> = wire(image)?;
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
        ty: program_type(&mut r.ty.reader(), schemas)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn with_image<T>(
        image: &Json,
        restore: impl for<'de> FnOnce(&mut BuildOwnedImageReader<'de>) -> Result<T, String>,
    ) -> Result<T, String> {
        let bytes = serde_json::to_vec(image).unwrap();
        let mut reader = BuildOwnedImageReader::from_image(&bytes).unwrap();
        let value = restore(&mut reader)?;
        reader.end().map_err(|e| e.to_string())?;
        Ok(value)
    }
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
            serde_json::to_value(with_image(&image, |reader| input(reader, &table)).unwrap())
                .unwrap(),
            image
        );
        let mut bad = ty;
        bad["scalar"]["kind"] = "Bool".into();
        assert!(with_image(&bad, |reader| program_type(reader, &table)).is_err());
        bad["scalar"]["kind"] = "U64".into();
        bad["scalar"]["schema"] = 1.into();
        assert!(with_image(&bad, |reader| program_type(reader, &table)).is_err());

        let (_, ty) = scalar_image();
        let mut bad = ty.clone();
        bad["definition"]["body"] = "Bool".into();
        assert!(with_image(&bad, |reader| program_type(reader, &table)).is_err());
        bad = ty.clone();
        bad["scalar"]["descriptor"][0]["body"] = "Bool".into();
        assert!(with_image(&bad, |reader| program_type(reader, &table)).is_err());
        bad = ty.clone();
        bad["scalar"]["schema_key"] = serde_json::to_value(SchemaKey::from_bytes([0; 32])).unwrap();
        assert!(with_image(&bad, |reader| program_type(reader, &table)).is_err());
        bad = ty;
        bad["schema"] = u32::MAX.into();
        assert!(with_image(&bad, |reader| program_type(reader, &table)).is_err());
    }

    #[test]
    fn canonical_type_rows_share_immutable_types_and_keep_scalar_presence() {
        let (table, image) = scalar_image();
        let mut types = TypeTable::new(&table);
        let row = serde_json::json!([0, 5]);
        let first = with_image(&row, |r| types.read(r)).unwrap();
        let second = with_image(&row, |r| types.read(r)).unwrap();
        assert_eq!(serde_json::to_value(&first).unwrap(), image);
        assert!(std::sync::Arc::ptr_eq(
            &first.definition,
            &second.definition
        ));
        assert!(std::sync::Arc::ptr_eq(
            first.scalar.as_ref().unwrap(),
            second.scalar.as_ref().unwrap()
        ));
        let non_scalar = with_image(&serde_json::json!([0, 0]), |r| types.read(r)).unwrap();
        assert!(non_scalar.scalar().is_none());
        assert_eq!(non_scalar.definition(), first.definition());
        for bad in [
            serde_json::json!([0, 7]),
            serde_json::json!([0, 255]),
            serde_json::json!([1, 5]),
            serde_json::json!([0]),
            serde_json::json!([0, 5, 0]),
            image,
        ] {
            assert!(with_image(&bad, |r| types.read(r)).is_err(), "{bad}");
        }
    }

    #[test]
    fn canonical_type_rows_bind_each_scalar_kind_to_its_actual_schema() {
        let bodies = [
            SchemaBody::Bool,
            SchemaBody::UnsignedInteger(IntegerWidth::W8),
            SchemaBody::UnsignedInteger(IntegerWidth::W16),
            SchemaBody::UnsignedInteger(IntegerWidth::W32),
            SchemaBody::UnsignedInteger(IntegerWidth::W64),
            SchemaBody::SignedInteger(IntegerWidth::W64),
            SchemaBody::FloatingPoint(FloatWidth::W64),
        ];
        for (index, body) in bodies.into_iter().enumerate() {
            let definition = SchemaDraft {
                dimension_parameters: Box::new([]),
                body,
            }
            .finalize()
            .unwrap();
            let table = values::schemas(&serde_json::json!([definition])).unwrap();
            let mut types = TypeTable::new(&table);
            let row = serde_json::json!([0, index + 1]);
            let ty = with_image(&row, |r| types.read(r)).unwrap();
            assert_eq!(ty.scalar().unwrap().descriptor().schema(), &definition);
            assert_eq!(ty.scalar().unwrap().schema_key(), definition.key());
            let foreign = serde_json::json!([0, if index == 0 { 2 } else { 1 }]);
            assert!(with_image(&foreign, |r| types.read(r)).is_err());
        }
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

    #[test]
    fn borrowed_resource_ports_preserve_request_and_type_associations() {
        let (table, ty) = scalar_image();
        let request = ExecutionResourceRequest {
            base_uri: "cli/stdin".into(),
            path: "text".into(),
            context_name: "in".into(),
            operation: "read".into(),
            intent: ResourceIntent::Read,
            delivery: ResourceDelivery::Snapshot,
        };
        let observation = serde_json::json!({"node":0,"requirement":0,"request":request,"slot":0,
            "ty":ty,"value":0,"ordinal":0});
        let effect = serde_json::json!({"node":0,"requirement":0,"request":request,"payload":{"Slot":0},
            "ty":ty,"value":1,"ordinal":0});
        for (image, is_effect) in [(observation, false), (effect, true)] {
            let json = serde_json::to_vec(&image).unwrap();
            let native = encode_native_metadata_image(&json).unwrap();
            for bytes in [&json, &native] {
                let mut reader = BuildOwnedImageReader::from_image(bytes).unwrap();
                // Inert wire metadata only; no resource is acquired or granted.
                let restored = if is_effect {
                    serde_json::to_value(super::effect(&mut reader, &table).unwrap()).unwrap()
                } else {
                    serde_json::to_value(super::observation(&mut reader, &table).unwrap()).unwrap()
                };
                reader.end().unwrap();
                assert_eq!(restored, image);
            }
        }
    }
}
