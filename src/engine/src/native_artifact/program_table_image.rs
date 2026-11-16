//! Compiler-emitted type references into the same canonical schema table.
//! These rows are data; checked-build confinement and live admission still apply.
use super::*;
use serde::{Serialize, Serializer, ser::SerializeStruct};

struct Types<'a>(&'a [NativeProgramType]);
impl Serialize for Types<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.iter().map(|ty| {
            let scalar = match ty.scalar_kind() {
                None => 0u8,
                Some(NativeScalarKind::Bool) => 1,
                Some(NativeScalarKind::U8) => 2,
                Some(NativeScalarKind::U16) => 3,
                Some(NativeScalarKind::U32) => 4,
                Some(NativeScalarKind::U64) => 5,
                Some(NativeScalarKind::I64) => 6,
                Some(NativeScalarKind::F64) => 7,
            };
            (ty.schema(), scalar)
        }))
    }
}

impl NativeProgramArtifactPlan {
    /// Emit a runtime table with each complete selected library definition once.
    /// Only immutable descriptive definitions are shared. The exact source
    /// header, operands, result and lexical ownership stay at each original site.
    /// Constants never pass through serde_json::Value, preserving U128/I128.
    pub fn serialize_build_owned_shared_template_plan<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        use serde::ser::Error;
        let mut blocks = serde_json::to_value(&self.blocks).map_err(S::Error::custom)?;
        let mut iterations = serde_json::to_value(&self.iterations).map_err(S::Error::custom)?;
        let mut collections = serde_json::to_value(&self.collections).map_err(S::Error::custom)?;
        let mut table = TemplateImages::default();
        for records in [&mut blocks, &mut iterations, &mut collections] {
            table.freeze(records).map_err(S::Error::custom)?;
        }
        let mut fields = serializer.serialize_struct("NativeProgramSharedTemplatePlan", 16)?;
        fields.serialize_field("revision", &self.revision)?;
        fields.serialize_field("resource_requirements", &self.resource_requirements)?;
        fields.serialize_field("observations", &self.observations)?;
        fields.serialize_field("effects", &self.effects)?;
        fields.serialize_field("inputs", &self.inputs)?;
        fields.serialize_field("constants", &self.constants)?;
        fields.serialize_field("values", &Types(&self.values))?;
        fields.serialize_field("value_scopes", &self.value_scopes)?;
        fields.serialize_field("blocks", &blocks)?;
        fields.serialize_field("iterations", &iterations)?;
        fields.serialize_field("collections", &collections)?;
        fields.serialize_field("outputs", &self.outputs)?;
        fields.serialize_field("entry", &self.entry)?;
        fields.serialize_field("activation_nodes", &self.activation_nodes)?;
        fields.serialize_field("activation_boundary", &self.activation_boundary)?;
        fields.serialize_field("template_definitions", &table.leaves)?;
        fields.end()
    }

    /// Serialize fixed type rows as canonical schema references. The canonical
    /// definitions remain in the enclosing image's schema table. This does not
    /// serialize pointers, runtime state, or permission to execute the plan.
    pub fn serialize_build_owned_table_plan<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut fields = serializer.serialize_struct("NativeProgramTablePlan", 15)?;
        fields.serialize_field("revision", &self.revision)?;
        fields.serialize_field("resource_requirements", &self.resource_requirements)?;
        fields.serialize_field("observations", &self.observations)?;
        fields.serialize_field("effects", &self.effects)?;
        fields.serialize_field("inputs", &self.inputs)?;
        fields.serialize_field("constants", &self.constants)?;
        fields.serialize_field("values", &Types(&self.values))?;
        fields.serialize_field("value_scopes", &self.value_scopes)?;
        fields.serialize_field("blocks", &self.blocks)?;
        fields.serialize_field("iterations", &self.iterations)?;
        fields.serialize_field("collections", &self.collections)?;
        fields.serialize_field("outputs", &self.outputs)?;
        fields.serialize_field("entry", &self.entry)?;
        fields.serialize_field("activation_nodes", &self.activation_nodes)?;
        fields.serialize_field("activation_boundary", &self.activation_boundary)?;
        fields.end()
    }
}

/// Build-time image compaction, never runtime JSON rewriting. The three input
/// inventories contain compiler metadata only; source constant payloads and
/// schemas are serialized through their original exact-width serializers.
#[derive(Default)]
struct TemplateImages {
    leaves: Vec<serde_json::Value>,
    indices: std::collections::BTreeMap<Vec<u8>, u32>,
}
impl TemplateImages {
    fn freeze(&mut self, value: &mut serde_json::Value) -> Result<(), String> {
        use serde_json::Value;
        match value {
            Value::Object(fields)
                if fields.contains_key("selected")
                    && fields.contains_key("selected_key")
                    && fields.contains_key("origin") =>
            {
                let mut selected = fields
                    .remove("selected")
                    .ok_or("missing selected template")?;
                let tags = selected
                    .as_object_mut()
                    .ok_or("selected template is not an enum")?;
                if tags.len() != 1
                    || !tags
                        .keys()
                        .all(|tag| matches!(tag.as_str(), "Root" | "Lexical"))
                {
                    return Err("unsupported selected template image".into());
                }
                let row = tags
                    .values_mut()
                    .next()
                    .unwrap()
                    .as_object_mut()
                    .ok_or("selected template has no source row")?;
                let leaf = row
                    .remove("leaf")
                    .ok_or("selected template has no definition")?;
                let key = serde_json::to_vec(&leaf).map_err(|error| error.to_string())?;
                let index = if let Some(index) = self.indices.get(&key) {
                    *index
                } else {
                    let index = u32::try_from(self.leaves.len())
                        .map_err(|_| "too many selected template definitions")?;
                    self.leaves.push(leaf);
                    self.indices.insert(key, index);
                    index
                };
                row.insert("leaf".into(), Value::from(index));
                *value = selected;
            }
            Value::Object(fields) => {
                for child in fields.values_mut() {
                    self.freeze(child)?;
                }
            }
            Value::Array(values) => {
                for child in values {
                    self.freeze(child)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
