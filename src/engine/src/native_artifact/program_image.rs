//! Compiler-side plan image encoding. This format grants no execution authority.
//! There is deliberately no public deserializer for constructor-private proofs.
use super::NativeProgramArtifactPlan;
use mech_core::{Value, ValueDraft};
use serde::{Serialize, Serializer};

// ResolvedValueDescriptor::from_schema reconstructs and validates the resolved
// type from these exact canonical schema/shape authorities; no Debug encoding.
pub(super) fn serialize_descriptor<S: Serializer>(
    value: &mech_core::ResolvedValueDescriptor,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    (value.schema(), value.shape().parameter_values()).serialize(serializer)
}

/// Encode the compiler-selected plan, not source or an alternative evaluator.
/// This is the plan section of the forthcoming build-owned descriptor image;
/// it is not yet a loadable descriptor and must not replace runtime validation.
pub fn encode_native_program_plan_image(
    plan: &NativeProgramArtifactPlan,
) -> Result<Vec<u8>, serde_json::Error> {
    #[derive(Serialize)]
    struct Image<'a> {
        format: &'static str,
        version: u32,
        plan: &'a NativeProgramArtifactPlan,
    }
    serde_json::to_vec(&Image {
        format: "mech.build-owned-native-plan",
        version: 1,
        plan,
    })
}

pub(super) fn serialize_constant<S: Serializer>(
    value: &Value,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let data = value
        .canonical_data_draft()
        .map_err(|error| serde::ser::Error::custom(format!("{error:?}")))?;
    ValueDraft {
        schema: value.schema(),
        shape_values: value.shape().parameter_values().to_vec().into_boxed_slice(),
        data,
    }
    .serialize(serializer)
}
