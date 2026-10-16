use std::collections::{BTreeMap, BTreeSet};

use mech_core::{
    FunctionCatalog, MResult, MechError, NativeValueFeature, ReactiveInstanceId, ResourceIntent,
    native_features_for_schema_body,
};
use mech_engine::{
    __resident::{ActivationFacts, ResidentIntegrityMode, activate_external},
    ProgramArtifact,
};
use mech_runtime::{
    ArtifactResourceSchemaPreflightReport, ResourceSchemaPreflight,
    artifact_has_closed_resource_payloads, preflight_artifact_resource_schemas,
    validate_artifact_resource_read_schema,
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
    catalog: &FunctionCatalog,
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
    let mut engine_features = BTreeSet::new();
    for operation in artifact.operation_references() {
        if let Some(feature) =
            required_resident_feature(&operation.module_path, &operation.operation_name)
        {
            engine_features.insert(feature.to_owned());
        }
    }
    // Planning has already activated this artifact with the configured
    // catalog. Kernel binding uses the ABI loader exactly when no catalog
    // factory owns the operation. Resource providers and direct integrity
    // checks do not use that binding path.
    if artifact
        .kernel_operation_references()
        .iter()
        .any(|operation| {
            catalog
                .resident_factory(&operation.module_path, &operation.operation_name)
                .is_none()
        })
    {
        engine_features.insert("dynamic-modules".to_owned());
    }
    let engine_features = engine_features.into_iter().collect();

    ArtifactNativeFeatureAnalysis {
        value_features,
        engine_features,
    }
}

fn required_resident_feature(module_path: &[String], operation_name: &str) -> Option<&'static str> {
    match (module_path, operation_name) {
        ([module], "parse-u64" | "format") if module == "number" => Some("pure_text"),
        ([module], "length") if module == "sequence" => Some("pure_text"),
        ([module], _) if module == "convert" => Some("convert"),
        ([module], _) if module == "table" => Some("table"),
        _ => None,
    }
}

/// Plans the external contracts carried by a canonical retained artifact.
///
/// Artifact bytecode intentionally has no shadow instruction stream. Native
/// planning validates every declared resource schema and provider contract.
/// Read representatives supply shape/representation facts only. Value-domain
/// checks may additionally evaluate a fully closed, observation-free artifact.
pub(crate) fn plan_artifact_external_contracts(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    resolver: &mut NativeBytecodeContractResolver<'_>,
) -> MResult<ArtifactResourceSchemaPreflightReport> {
    let limit = resolver.planning_step_limit()?;
    mech_engine::__resident::with_planning_step_limit(limit, || {
        plan_artifact_external_contracts_with_limit(artifact, catalog, resolver)
    })
    .map_err(|limit| {
        artifact_error(format!(
            "native planning exceeds the configured {limit} step limit"
        ))
    })?
}

fn plan_artifact_external_contracts_with_limit(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    resolver: &mut NativeBytecodeContractResolver<'_>,
) -> MResult<ArtifactResourceSchemaPreflightReport> {
    let mut observation_values = BTreeMap::new();
    let mut facts = ActivationFacts::default();
    let mut effect_requirements = BTreeMap::new();

    let report = preflight_artifact_resource_schemas(artifact, |site| {
        let request = site.request;
        if request.intent != ResourceIntent::Read {
            effect_requirements.insert(site.node, (site.requirement, request.clone()));
            return resolver.preflight_artifact_resource_write(&site);
        }
        resolver.validate_artifact_resource_contract(&site)?;
        let value = resolver.plan_artifact_resource_read(site.node, request)?;
        validate_artifact_resource_read_schema(artifact, &site, &value)?;
        let output_slot = site.observation_output.ok_or_else(|| {
            artifact_error(format!(
                "resource observation node {} has no output binding",
                site.node.get()
            ))
        })?;
        facts.slot_shapes.insert(output_slot, value.shape().clone());
        if observation_values.insert(site.node, value).is_some() {
            return Err(artifact_error(format!(
                "resource observation node {} is duplicated",
                site.node.get()
            )));
        }
        Ok(ResourceSchemaPreflight::Validated)
    })?;

    let mut instance = activate_external(
        ReactiveInstanceId::new(0x4e41_5449, 0),
        artifact,
        catalog,
        &facts,
        ResidentIntegrityMode::Checked,
    )
    .map_err(|error| artifact_error(format!("resident activation failed: {error:?}")))?;

    let _representative_schema_checks = instance
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
            value
                .rebind(input.schema, &input.shape, artifact.schemas())
                .map_err(|error| {
                    artifact_error(format!(
                        "resource observation node {} does not match artifact input {}: {error:?}",
                        node.get(),
                        input.slot.get(),
                    ))
                })?;
            Ok(())
        })
        .collect::<MResult<Vec<_>>>()?;
    // Shape facts admit storage, but only rebinding validates the provider's
    // value type against the retained input schema, including read-only plans.
    if effect_requirements.is_empty() || !artifact_has_closed_resource_payloads(artifact) {
        // A deferred schema check stays explicit in the report; actual live
        // preparation still validates each real effect before publication.
        return Ok(report);
    }
    let prepared = instance
        .prepare_initial_turn_values(&[])
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
    // Dormant effects were already structurally preflighted by the visitor.
    prepared.abort();
    Ok(report)
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
