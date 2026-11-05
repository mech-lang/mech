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
