//! A fixed key-set owner may retain its borrowed keys and independent current
//! values. Runtime still requires genuine snapshot identity and finite admission.
use super::{borrowed_map_keys, dictionary_seed_frames, generic_dictionary, matrix_map_keys};
use mech_engine::native_artifact::{
    NativeMapOwnerWords, NativeMatchValue as Value, NativeProgramArtifactPlan as Plan,
};

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub(in crate::native_artifact) struct Region {
    pub owner: usize,
    pub initialize: usize,
    pub key_source: Value,
    pub snapshot: Value,
    pub cell: usize,
    pub insert: usize,
    pub map_ordinal: usize,
}
#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize)]
pub(in crate::native_artifact) struct Layout {
    pub regions: Vec<Region>,
}
impl Layout {
    pub(in crate::native_artifact) fn initializer(&self, site: usize) -> Option<&Region> {
        self.regions.iter().find(|r| r.initialize == site)
    }
    pub(in crate::native_artifact) fn derive(
        plan: &Plan,
        owners: Option<&NativeMapOwnerWords>,
        keys: &borrowed_map_keys::Layout,
        cells: &matrix_map_keys::Layout,
        seeds: &dictionary_seed_frames::Layout,
        dictionaries: &generic_dictionary::Layout,
    ) -> Result<Self, &'static str> {
        let Some(owners) = owners else {
            return Ok(Self::default());
        };
        if plan.blocks().len() > 16_384 || plan.values().len() > 65_536 {
            return Ok(Self::default());
        }
        plan.validate_current_visits()?;
        let facts = plan.facts();
        let mut result = Self::default();
        for seed in &seeds.regions {
            let owner = &owners.regions()[seed.owner];
            if !owner.reads().is_empty()
                || !dictionaries.regions.iter().any(|d| d.owner == seed.owner)
            {
                continue;
            }
            let [write] = owner.writes() else {
                continue;
            };
            let Some((_, cell)) = cells.consumer(*write as usize) else {
                continue;
            };
            if cell.owner != seed.owner {
                continue;
            }
            let Some(view) = keys
                .regions
                .iter()
                .find(|k| k.output == cell.source && k.cells == [cell.producer])
            else {
                continue;
            };
            if !facts.strictly_dominates(view.producer as u32, seed.initialize)? {
                continue;
            }
            // This does not infer equality of Map values or reconstruct an
            // original retired arena handle. The runtime checks source/view
            // snapshot identity, and keeps old counts in their own immutable pin.
            result.regions.push(Region {
                owner: seed.owner,
                initialize: seed.initialize as usize,
                key_source: view.output,
                snapshot: view.snapshot,
                cell: cell.producer,
                insert: cell.consumer,
                map_ordinal: seed.map_ordinal,
            });
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_artifact::program::prepare::{
        PreparedNativeProgram,
        matrix_map_keys::tests::{artifact, source as renumber},
    };
    fn source() -> String {
        let s = renumber("at + 1u64", "at", "at + 1u64", "histogram");
        let declarations = s.split("\nbuild(").next().unwrap();
        format!(
            "{declarations}\ntags := tags<[string]>\nhistogram := map/histogram(tags)\nkeys := map/keys(histogram)\nend := sequence/length(keys)\nindex := renumber(keys,0u64,end,histogram)\nids := [index{{tag}} | tag <- tags]\nids\n"
        )
    }
    fn derive(a: &mech_engine::ProgramArtifact) -> Layout {
        let c = mech_stdlib::source_catalog();
        let p = mech_engine::native_artifact::lower_native_program_artifact(a, &c).unwrap();
        let p = PreparedNativeProgram::prepare(a, &c, p).unwrap();
        let m = p.metadata();
        Layout::derive(
            &m.plan,
            m.map_owner_words.as_ref(),
            &m.borrowed_map_keys,
            &m.matrix_map_keys,
            &m.dictionary_seed_frames,
            &m.generic_dictionary,
        )
        .unwrap()
    }
    #[test]
    fn borrowed_dictionary_values_require_one_borrowed_key_writer_and_native_result_use() {
        let original = source();
        for (text, eligible) in [
            (original.clone(), true),
            (
                original.replace("index := renumber(keys,", "index := renumber(tags,"),
                false,
            ),
            (
                format!("{}(ids,index)\n", original.strip_suffix("ids\n").unwrap()),
                false,
            ),
        ] {
            let a = artifact(&text);
            let b = mech_engine::decode_program_artifact_bytecode_v1(
                &mech_engine::encode_program_artifact_bytecode_v1(&a).unwrap(),
            )
            .unwrap();
            let x = derive(&a);
            assert_eq!(!x.regions.is_empty(), eligible, "{text}\n{x:?}");
            assert_eq!(x, derive(&b));
        }
    }
    #[test]
    #[ignore = "requires caller-pinned full source"]
    fn borrowed_dictionary_values_actual_source_selects_two_generic_seed_key_sets() {
        std::thread::Builder::new().stack_size(32*1024*1024).spawn(|| {
            let s=std::fs::read_to_string(std::env::var("MECH_BORROWED_DICTIONARY_SOURCE").unwrap()).unwrap();
            let a=artifact(&s);let b=mech_engine::decode_program_artifact_bytecode_v1(&mech_engine::encode_program_artifact_bytecode_v1(&a).unwrap()).unwrap();
            let x=derive(&a);assert_eq!(x.regions.len(),2,"{x:?}");assert_eq!(x,derive(&b));
            println!("{}",serde_json::json!({"borrowed_dictionary_value_candidates":x,"runtime_connected":false,"timing_eligible":false}));
        }).unwrap().join().unwrap();
    }
}
