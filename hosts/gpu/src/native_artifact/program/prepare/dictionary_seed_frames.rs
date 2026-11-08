//! Applicability for the canonical aggregate immediately feeding a retained
//! String/U64 Map owner. Descriptive only: no seed bypass or runtime authority.
use mech_core::{IntegerWidth, SchemaBody};
use mech_engine::native_artifact::{
    NativeCollectionPrimitive as Primitive, NativeIterationAction as Action,
    NativeIterationTerminator as Terminator, NativeIterationValueScope as Scope,
    NativeMapOwnerWords, NativeMatchTerminator as ScalarTerminator, NativeMatchValue as Value,
    NativeProgramArtifactPlan as Plan, NativeProgramReadRole as Read,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct Region {
    pub owner: usize,
    pub producer: u32,
    pub initialize: u32,
    pub seed: Value,
    pub operands: Vec<Value>,
    pub map_ordinal: usize,
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(in crate::native_artifact) struct Layout {
    pub regions: Vec<Region>,
}
impl Layout {
    pub fn producer(&self, site: usize) -> Option<&Region> {
        self.regions.iter().find(|r| r.producer as usize == site)
    }
    pub fn initializer(&self, site: usize) -> Option<&Region> {
        self.regions.iter().find(|r| r.initialize as usize == site)
    }
    pub fn derive(plan: &Plan, owners: &NativeMapOwnerWords) -> Result<Self, &'static str> {
        if owners.regions().is_empty()
            || plan.blocks().len() > 16_384
            || plan.values().len() > 65_536
        {
            return Ok(Self::default());
        }
        plan.validate_current_visits()?;
        let facts = plan.facts();
        let mut result = Self::default();
        for (owner, region) in owners.regions().iter().enumerate() {
            let tail = region.tail();
            let seed = tail.seed();
            if plan.value_scopes()[seed.get() as usize]
                != (Scope::Immutable {
                    iteration: tail.parent(),
                })
                || owners.initializer(tail.initialize() as usize) != Some(owner)
                || facts.read_roles(seed)? != [Read::LoopSeed(region.iteration())]
            {
                continue;
            }
            let [definition] = facts.definition_sites(seed.get())? else {
                continue;
            };
            let Some(producer) = definition.site() else {
                continue;
            };
            let block = &plan.blocks()[producer as usize];
            let Action::LibraryCall {
                template,
                operands,
                output,
            } = block.action()
            else {
                continue;
            };
            if *output != seed
                || block.iteration() != tail.parent()
                || !matches!(template.primitive(), Primitive::CompositePack(n)
                    if n == operands.len() && n > 0 && n <= 32)
                || !matches!(block.terminator(),
                    Terminator::Scalar(ScalarTerminator::Jump(next)) if next == tail.initialize())
                || !facts.strictly_dominates(producer, tail.initialize())?
            {
                continue;
            }
            // No native activation may suspend this new, as-yet-unimplemented
            // private frame between its selected producer and original import.
            if let Some(boundary) = plan.activation_boundary()
                && (boundary == producer
                    || boundary == tail.initialize()
                    || !facts.strictly_dominates(boundary, producer)?)
            {
                continue;
            }
            let ty = &plan.values()[seed.get() as usize];
            if !ty.definition().dimension_parameters().is_empty() {
                continue;
            }
            let fields: Vec<&SchemaBody> = match ty.definition().body() {
                SchemaBody::Tuple(fields) => fields.iter().collect(),
                SchemaBody::Record(fields) => fields.iter().map(|f| &f.schema).collect(),
                _ => continue,
            };
            if fields.len() != operands.len() || template.inputs().len() != operands.len() {
                continue;
            }
            let mut map_ordinal = None;
            let mut valid = true;
            for (ordinal, ((field, operand), input)) in fields
                .iter()
                .zip(operands.iter())
                .zip(template.inputs())
                .enumerate()
            {
                let value = &plan.values()[operand.get() as usize];
                if !input.dimension_parameters().is_empty()
                    || !value.definition().dimension_parameters().is_empty()
                    || value.definition().body() != *field
                    || input.body() != *field
                {
                    valid = false;
                    break;
                }
                match *field {
                    SchemaBody::UnsignedInteger(IntegerWidth::W64) => {}
                    SchemaBody::Map { key, value, .. }
                        if key.as_ref() == &SchemaBody::String
                            && value.as_ref()
                                == &SchemaBody::UnsignedInteger(IntegerWidth::W64)
                            && map_ordinal.is_none() =>
                    {
                        map_ordinal = Some(ordinal)
                    }
                    _ => {
                        valid = false;
                        break;
                    }
                }
            }
            if valid && let Some(map_ordinal) = map_ordinal {
                result.regions.push(Region {
                    owner,
                    producer,
                    initialize: tail.initialize(),
                    seed,
                    operands: operands.to_vec(),
                    map_ordinal,
                });
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_artifact::program::prepare::matrix_map_keys::tests::artifact;
    fn source() -> String {
        let declarations = super::super::matrix_map_keys::tests::source(
            "at + 1u64",
            "at",
            "at + 1u64",
            "histogram",
        );
        let declarations = declarations.split("\nbuild(").next().unwrap();
        format!(
            "{declarations}\ntags := tags<[string]>\nhistogram := map/histogram(tags)\nkeys := map/keys(histogram)\nend := sequence/length(keys)\nindex := renumber(keys,0u64,end,histogram)\nindex\n"
        )
    }
    fn lower(a: &mech_engine::ProgramArtifact) -> Plan {
        mech_engine::native_artifact::lower_native_program_artifact(
            a,
            &mech_stdlib::source_catalog(),
        )
        .unwrap()
    }
    fn owners(p: &Plan) -> NativeMapOwnerWords {
        p.facts().buffer_frame_owner_layouts().unwrap().1
    }
    #[test]
    fn dictionary_seed_frames_prove_adjacent_private_generic_map_seed_and_decoded_plan() {
        let a = artifact(&source());
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&a).unwrap(),
        )
        .unwrap();
        let original = lower(&a);
        let proof = Layout::derive(&original, &owners(&original)).unwrap();
        assert_eq!(proof.regions.len(), 1, "{proof:?}");
        assert_eq!(proof.regions[0].operands.len(), 3);
        assert_eq!(proof.regions[0].map_ordinal, 2);
        let restored = lower(&decoded);
        assert_eq!(
            proof,
            Layout::derive(&restored, &owners(&restored)).unwrap()
        );
    }
    #[test]
    fn dictionary_seed_frames_keep_public_map_alias_distinct_from_private_seed_frame() {
        let private = source();
        let declarations = private.strip_suffix("index\n").unwrap();
        let alias_source = format!("{declarations}(histogram,index)\n");
        let artifact = artifact(&alias_source);
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
        )
        .unwrap();
        for artifact in [&artifact, &decoded] {
            let plan = lower(artifact);
            let owned = owners(&plan);
            assert_eq!(owned.regions().len(), 1);
            let proof = Layout::derive(&plan, &owned).unwrap();
            assert_eq!(proof.regions.len(), 1);
            let selected = &proof.regions[0];
            // The tuple itself has one initializer use. Its immutable Map
            // operand may have genuine external aliases: later implementation
            // must pin/copy this owner, never mutate the old counts in place.
            assert_eq!(
                plan.facts().read_roles(selected.seed).unwrap(),
                [Read::LoopSeed(owned.regions()[0].iteration())]
            );
            assert_eq!(selected.map_ordinal, 2);
        }
    }
    #[test]
    #[ignore = "requires caller-pinned retained benchmark source"]
    fn dictionary_seed_frames_actual_source_has_two_generic_nonempty_seed_packs() {
        let text = std::fs::read_to_string(std::env::var("MECH_MATRIX_KEY_BENCH_SOURCE").unwrap())
            .unwrap();
        let plan = lower(&artifact(&text));
        let proof = Layout::derive(&plan, &owners(&plan)).unwrap();
        assert_eq!(proof.regions.len(), 2, "{proof:#?}");
        let first = proof.regions.iter().find(|p| p.seed.get() == 72).unwrap();
        assert_eq!(
            (
                first.owner,
                first.producer,
                first.initialize,
                first.map_ordinal
            ),
            (0, 44, 45, 2)
        );
        println!("native dictionary seed applicability: {proof:#?}");
    }
}
