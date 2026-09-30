use std::collections::{BTreeMap, BTreeSet};

use mech_core::{
    ApplicationRequirement, BindingId, FunctionCatalog, MResult, MechError, NativeValueFeature,
    ReactiveInstanceId, ResourceIntent, native_features_for_schema_body,
};
use mech_engine::{
    __resident::{ActivationFacts, CapturedValueInput, ResidentIntegrityMode, activate_external},
    BindingDeclaration, ProgramArtifact,
};

use crate::error::{NativeBuildErrorKind, native_build_error};

use super::requirements::NativeBytecodeContractResolver;

/// Exact Cargo features required to bind and execute one canonical artifact.
///
/// Artifact bytecode deliberately has no parallel legacy type or instruction
/// table. Native planning therefore derives its value closure from the
/// artifact schema arena and its gated resident closure from canonical
/// operation identities.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ArtifactNativeFeatureAnalysis {
    pub value_features: Vec<String>,
    pub engine_features: Vec<String>,
}

pub(crate) fn validate_artifact_requirement_reachability(
    artifact: &ProgramArtifact,
) -> MResult<()> {
    let referenced = artifact
        .nodes()
        .iter()
        .filter_map(|node| node.as_operation()?.requirement)
        .collect::<BTreeSet<_>>();
    if let Some((requirement, _)) = artifact
        .requirements()
        .iter()
        .find(|(requirement, _)| !referenced.contains(requirement))
    {
        return Err(artifact_error(format!(
            "application requirement {} is not owned by an artifact operation node",
            requirement.get()
        )));
    }
    Ok(())
}

pub(crate) fn analyze_artifact_native_features(
    artifact: &ProgramArtifact,
) -> ArtifactNativeFeatureAnalysis {
    let mut values = BTreeSet::new();
    for entry in artifact.schemas().entries() {
        native_features_for_schema_body(entry.schema().body(), &mut values);
    }

    let value_features = values
        .into_iter()
        .map(NativeValueFeature::cargo_feature)
        .map(str::to_owned)
        .collect();
    let engine_features = artifact
        .operation_references()
        .into_iter()
        .filter_map(|operation| {
            required_resident_feature(&operation.module_path, &operation.operation_name)
        })
        .map(str::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    ArtifactNativeFeatureAnalysis {
        value_features,
        engine_features,
    }
}

fn required_resident_feature(module_path: &[String], operation_name: &str) -> Option<&'static str> {
    match (module_path, operation_name) {
        ([module], _) if module == "convert" => Some("convert"),
        ([module], _) if module == "table" => Some("table"),
        _ => None,
    }
}

/// Plans the external contracts carried by a canonical retained artifact.
///
/// Artifact bytecode intentionally has no shadow instruction stream. Native
/// planning therefore evaluates the artifact itself, supplies provider-owned
/// observation values at its resident input boundary, and validates each
/// materialized effect payload with the same trusted planning providers used
/// by instruction bytecode.
pub(crate) fn plan_artifact_external_contracts(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    resolver: &mut NativeBytecodeContractResolver<'_>,
) -> MResult<()> {
    let mut observation_values = BTreeMap::new();
    let mut facts = ActivationFacts::default();
    let mut effect_requirements = BTreeMap::new();

    for node in artifact.nodes() {
        let Some(operation) = node.as_operation() else {
            continue;
        };
        let Some(requirement_id) = operation.requirement else {
            continue;
        };
        let Some(requirement) = artifact.requirements().get(requirement_id) else {
            return Err(artifact_error(format!(
                "external node {} references missing requirement {}",
                node.node.get(),
                requirement_id.get(),
            )));
        };
        let ApplicationRequirement::Resource(request) = requirement else {
            return Err(artifact_error(format!(
                "external node {} has a non-resource requirement",
                node.node.get(),
            )));
        };
        if request.intent != ResourceIntent::Read {
            effect_requirements.insert(node.node, (requirement_id, request));
            continue;
        }

        let value = resolver.plan_artifact_resource_read(node.node, request)?;
        let output_slot = node
            .output_bindings
            .clone()
            .map(BindingId::new)
            .find_map(
                |binding| match artifact.bindings().get(binding.get() as usize) {
                    Some(BindingDeclaration::Output {
                        node: owner,
                        target,
                        ..
                    }) if *owner == node.node => Some(*target),
                    _ => None,
                },
            )
            .ok_or_else(|| {
                artifact_error(format!(
                    "resource observation node {} has no output binding",
                    node.node.get()
                ))
            })?;
        facts.slot_shapes.insert(output_slot, value.shape().clone());
        if observation_values.insert(node.node, value).is_some() {
            return Err(artifact_error(format!(
                "resource observation node {} is duplicated",
                node.node.get()
            )));
        }
    }

    let mut instance = activate_external(
        ReactiveInstanceId::new(0x4e41_5449, 0),
        artifact,
        catalog,
        &facts,
        ResidentIntegrityMode::Checked,
    )
    .map_err(|error| artifact_error(format!("resident activation failed: {error:?}")))?;

    let captured = instance
        .plan
        .inputs
        .iter()
        .filter_map(|input| match input.source {
            mech_engine::__resident::ActivatedInputSource::Observation { node, .. } => {
                Some((input, node))
            }
            mech_engine::__resident::ActivatedInputSource::DeclaredInput { .. } => None,
        })
        .map(|(input, node)| {
            let value = observation_values.get(&node).ok_or_else(|| {
                artifact_error(format!(
                    "resource observation node {} has no planned provider value",
                    node.get()
                ))
            })?;
            let value = value
                .rebind(input.schema, &input.shape, artifact.schemas())
                .map_err(|error| {
                    artifact_error(format!(
                        "resource observation node {} does not match artifact input {}: {error:?}",
                        node.get(),
                        input.slot.get(),
                    ))
                })?;
            Ok((input.slot, value))
        })
        .collect::<MResult<Vec<_>>>()?;
    // Shape facts admit storage, but only rebinding validates the provider's
    // value type against the retained input schema, including read-only plans.
    if effect_requirements.is_empty() {
        return Ok(());
    }
    let inputs = captured
        .iter()
        .map(|(slot, value)| CapturedValueInput { slot: *slot, value })
        .collect::<Vec<_>>();
    let prepared = instance
        .prepare_initial_turn_values(&inputs)
        .map_err(|error| artifact_error(format!("resident planning turn failed: {error:?}")))?;

    let effects = prepared
        .effect_intents()
        .map(|effect| (effect.artifact_node, effect.requirement, effect.ordinal))
        .collect::<Vec<_>>();
    let mut materialized_effects = BTreeSet::new();
    for (node, requirement_id, ordinal) in effects {
        let Some((expected_requirement, request)) = effect_requirements.get(&node) else {
            return Err(artifact_error(format!(
                "resident planning materialized undeclared external effect node {}",
                node.get()
            )));
        };
        if *expected_requirement != requirement_id {
            return Err(artifact_error(format!(
                "external node {} materialized requirement {}, expected {}",
                node.get(),
                requirement_id.get(),
                expected_requirement.get(),
            )));
        }
        if !materialized_effects.insert(node) {
            return Err(artifact_error(format!(
                "external node {} materialized more than one planning effect",
                node.get(),
            )));
        }
        let payload = prepared
            .materialize_effect_payload(ordinal)
            .map_err(|error| {
                artifact_error(format!(
                    "external node {} payload materialization failed: {}",
                    node.get(),
                    error.display_message()
                ))
            })?;
        resolver.plan_artifact_resource_write(node, request, &payload)?;
    }
    // Initial publication intentionally leaves activation-gated effects
    // dormant. They still belong to the native application's authority and
    // must resolve an owner, grant, operation and path before generation. An
    // active effect additionally receives payload-specific planning above.
    for (node, (_, request)) in effect_requirements {
        if !materialized_effects.contains(&node) {
            resolver.preflight_artifact_resource_write(node, request)?;
        }
    }
    prepared.abort();
    Ok(())
}

fn artifact_error(reason: String) -> MechError {
    native_build_error(
        NativeBuildErrorKind::NativeProgramArtifactInvalid { reason },
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_gated_resident_modules_have_one_authoritative_mapping() {
        let module = |name: &str| vec![name.to_owned()];

        assert_eq!(
            required_resident_feature(&module("convert"), "kind"),
            Some("convert")
        );
        assert_eq!(
            required_resident_feature(&module("table"), "left-outer-join"),
            Some("table")
        );
        assert_eq!(required_resident_feature(&module("math"), "add"), None);
    }
}
