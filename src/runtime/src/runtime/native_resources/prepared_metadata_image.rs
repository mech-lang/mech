//! Build-owned resource selection, not a capability grant or native executor.
use super::super::{
    CanonicalNativeResourceAction, CanonicalNativeResourcePlan, CanonicalNativeResourceSite, error,
    invalid,
};
use super::CanonicalNativeResourceMetadata;
use mech_core::{
    ApplicationRequirement, ApplicationRequirementId, CellSlotId, ExecutionResourceRequest,
    MResult, NodeId, ProgramRevision, ResourceDelivery, ResourceIntent, SchemaBody, SchemaId,
    SchemaKey, SchemaTable,
};
use mech_engine::{ApplicationRequirementTable, ArtifactSource};
use serde::{Deserialize, Serialize};

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
impl From<Request> for ExecutionResourceRequest {
    fn from(w: Request) -> Self {
        Self {
            base_uri: w.base_uri,
            path: w.path,
            context_name: w.context_name,
            operation: w.operation,
            intent: w.intent,
            delivery: w.delivery,
        }
    }
}
#[derive(Deserialize)]
enum Source {
    Constant(u32),
    Slot(CellSlotId),
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
enum Action {
    Observation { output: CellSlotId },
    Effect { payload: Source, ordinal: u32 },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Site {
    node: NodeId,
    requirement: ApplicationRequirementId,
    request: Request,
    schema: SchemaId,
    schema_key: SchemaKey,
    action: Action,
}
impl Site {
    fn restore(self) -> CanonicalNativeResourceSite {
        CanonicalNativeResourceSite {
            node: self.node,
            requirement: self.requirement,
            request: self.request.into(),
            schema: self.schema,
            schema_key: self.schema_key,
            action: match self.action {
                Action::Observation { output } => {
                    CanonicalNativeResourceAction::Observation { output }
                }
                Action::Effect { payload, ordinal } => CanonicalNativeResourceAction::Effect {
                    payload: match payload {
                        Source::Constant(id) => {
                            ArtifactSource::Constant(mech_core::ConstantId::new(id))
                        }
                        Source::Slot(id) => ArtifactSource::Slot(id),
                    },
                    ordinal,
                },
            },
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    revision: ProgramRevision,
    requirements: Vec<Request>,
    observations: Vec<Site>,
    effects: Vec<Site>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Image {
    format: String,
    version: u32,
    plan: Plan,
}

impl CanonicalNativeResourceMetadata {
    /// Export ALL original requirements and selected site/order metadata. This
    /// contains no provider, grant, transaction, retained value or native entry.
    pub fn encode_build_owned_image(&self) -> Result<Vec<u8>, serde_json::Error> {
        #[derive(Serialize)]
        struct Image<'a> {
            format: &'static str,
            version: u32,
            plan: &'a CanonicalNativeResourcePlan,
        }
        serde_json::to_vec(&Image {
            format: "mech.build-owned-native-resources",
            version: 1,
            plan: &self.plan,
        })
    }

    /// Restore compiler-prepared metadata without a program graph or scheduling.
    /// Every invocation still rechecks finite port caps and current authority.
    ///
    /// # Safety
    /// The image, expected revision and schemas must be unchanged products of
    /// the SAME validated canonical artifact used for the native Object. The
    /// build must pin that source/bytecode/Object and this image before linking.
    /// Do not treat an untrusted image or supplied digest as proof of complete
    /// requirements, source contracts or effect order. Match the native session
    /// to this revision and preserve the shared runtime's entire admission,
    /// provider capture, transaction, effect and shutdown protocol.
    pub unsafe fn decode_build_owned_image(
        bytes: &[u8],
        expected_revision: ProgramRevision,
        schemas: SchemaTable,
    ) -> MResult<Self> {
        let image: Image = serde_json::from_slice(bytes).map_err(|e| error(e.to_string()))?;
        if image.format != "mech.build-owned-native-resources"
            || image.version != 1
            || image.plan.revision != expected_revision
        {
            return invalid("resource image format or canonical revision differs");
        }
        let w = image.plan;
        let plan = CanonicalNativeResourcePlan {
            revision: w.revision,
            requirements: w.requirements.into_iter().map(Into::into).collect(),
            observations: w.observations.into_iter().map(Site::restore).collect(),
            effects: w.effects.into_iter().map(Site::restore).collect(),
        };
        // The prepared plan contains ALL requirements, including unused ones.
        // Rebuild the canonical requirement table, never infer it from sites.
        let requirements = ApplicationRequirementTable::from_canonical_entries(
            plan.requirements
                .iter()
                .cloned()
                .map(ApplicationRequirement::Resource)
                .collect(),
        )
        .map_err(|e| error(format!("resource image requirements: {e:?}")))?;
        if plan.requirements.iter().any(|r| {
            !matches!(
                (r.intent, r.delivery),
                (ResourceIntent::Read, ResourceDelivery::Live)
                    | (ResourceIntent::Send, ResourceDelivery::Snapshot)
            )
        }) {
            return invalid("resource image has unsupported delivery");
        }
        for (is_observation, sites) in [(true, &plan.observations), (false, &plan.effects)] {
            for site in sites.iter() {
                let schema = schemas
                    .get(site.schema)
                    .ok_or_else(|| error("resource image schema is absent"))?;
                if plan.requirements.get(site.requirement.get() as usize) != Some(&site.request)
                    || schema.key() != site.schema_key
                    || schema.body() != &SchemaBody::String
                    || !schema.dimension_parameters().is_empty()
                    || !matches!(
                        (is_observation, site.request.intent, site.action),
                        (
                            true,
                            ResourceIntent::Read,
                            CanonicalNativeResourceAction::Observation { .. }
                        ) | (
                            false,
                            ResourceIntent::Send,
                            CanonicalNativeResourceAction::Effect { .. }
                        )
                    )
                {
                    return invalid(
                        "resource image site differs from its original requirement or schema",
                    );
                }
            }
        }
        let metadata = Self {
            plan,
            requirements,
            schemas,
        };
        Ok(metadata)
    }
}
