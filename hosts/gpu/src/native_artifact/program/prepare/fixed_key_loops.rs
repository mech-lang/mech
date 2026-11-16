//! Complete acyclic bodies over one private fixed key-set map. This cache
//! describes source work; it is not a current loan or an admission certificate.
use super::{borrowed_dictionary_values, matrix_map_keys};
use mech_engine::native_artifact::{
    NativeIterationAction as A, NativeMapOwnerWords, NativeMatchAction as S,
    NativeProgramArtifactPlan as Plan,
};
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub(in crate::native_artifact) struct Region {
    pub owner: usize,
    pub iteration: u32,
    pub initialize: usize,
    pub header: usize,
    pub latch: usize,
    pub finish: usize,
    pub exit: usize,
    pub cell: usize,
    pub insert: usize,
    pub constructors: Vec<usize>,
    pub sites: Vec<usize>,
}
#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize)]
pub(in crate::native_artifact) struct Layout {
    pub regions: Vec<Region>,
}
impl Layout {
    pub(in crate::native_artifact) fn owner(&self, owner: usize) -> Option<&Region> {
        self.regions.iter().find(|r| r.owner == owner)
    }
    pub(in crate::native_artifact) fn derive(
        plan: &Plan,
        owners: Option<&NativeMapOwnerWords>,
        banks: &borrowed_dictionary_values::Layout,
        cells: &matrix_map_keys::Layout,
    ) -> Result<Self, &'static str> {
        let Some(owners) = owners else {
            return Ok(Self::default());
        };
        if plan.blocks().len() > 16_384 || plan.values().len() > 65_536 {
            return Ok(Self::default());
        }
        if banks.regions.is_empty() {
            return Ok(Self::default());
        }
        plan.validate_current_visits()?;
        let facts = plan.facts();
        let mut out = Self::default();
        for bank in &banks.regions {
            let Some(owner) = owners.regions().get(bank.owner) else {
                return Err("fixed-key owner outside inventory");
            };
            let tail = owner.tail();
            // Existing closed map transport returns one Map. Its flat scalar
            // carrier constructors have the exact seed factory envelope; the
            // fixed key set and U64 replacements preserve every logical byte.
            if !matches!(
                plan.values()[tail.returned().get() as usize]
                    .definition()
                    .body(),
                mech_core::SchemaBody::Map { .. }
            ) {
                continue;
            }
            let [definition] = facts.definition_sites(tail.seed().get())? else {
                continue;
            };
            let Some(seed_site) = definition.site() else {
                continue;
            };
            let A::LibraryCall {
                template: seed_factory,
                ..
            } = plan.blocks()[seed_site as usize].action()
            else {
                continue;
            };
            if !matches!(
                seed_factory.primitive(),
                mech_engine::native_artifact::NativeCollectionPrimitive::CompositePack(_)
            ) {
                continue;
            }

            let Some((_, key)) = cells.producer(bank.cell) else {
                return Err("fixed-key producer outside inventory");
            };
            if key.owner != bank.owner
                || key.consumer != bank.insert
                || !owner.reads().is_empty()
                || owner.writes() != [bank.insert as u32]
            {
                continue;
            }
            let mut seen = std::collections::BTreeSet::new();
            let mut pending = vec![tail.header() as usize];
            let mut constructors = Vec::new();
            let mut valid = true;
            while let Some(site) = pending.pop() {
                if site == tail.exit() as usize || !seen.insert(site) {
                    continue;
                }
                let b = &plan.blocks()[site];
                if b.iteration() != Some(owner.iteration())
                    || plan.activation_boundary() == Some(site as u32)
                {
                    valid = false;
                    break;
                }
                let private = owners.action_region(site) == Some(bank.owner);
                let supported=match b.action(){
                    A::Scalar(S::None|S::Primitive{..})=>true,
                    A::Scalar(S::Copy{output,..})=>private||plan.values()[output.get()as usize].scalar_kind().is_some(),
                    A::Structural{..}=>private,
                    A::Latch{iteration}=>*iteration==owner.iteration()&&site==tail.latch()as usize,
                    A::LibraryCall{template,..} if site==bank.cell=>template.primitive()==mech_engine::native_artifact::NativeCollectionPrimitive::MatrixScalarAccess,
                    A::LibraryCall{template,..} if site==bank.insert=>template.primitive()==mech_engine::native_artifact::NativeCollectionPrimitive::MapInsert,
                    A::LibraryCall{template,..} if private=>{
                        if matches!(template.primitive(),mech_engine::native_artifact::NativeCollectionPrimitive::CompositePack(_)|mech_engine::native_artifact::NativeCollectionPrimitive::EnumPack){constructors.push(site);true}else{false}
                    },
                    _=>false,
                };
                if !supported {
                    valid = false;
                    break;
                }
                let nexts = facts.successors(site as u32)?;
                if nexts.contains(&Some(tail.exit())) && site != tail.finish() as usize {
                    valid = false;
                    break;
                }
                for next in nexts.into_iter().flatten() {
                    pending.push(next as usize)
                }
            }
            if !valid
                || ![
                    tail.latch(),
                    tail.finish(),
                    bank.cell as u32,
                    bank.insert as u32,
                ]
                .iter()
                .all(|s| seen.contains(&(*s as usize)))
            {
                continue;
            }
            // The native fuel unit covers one complete source traversal. Only
            // the declared latch may close a cycle; no child loop is hidden.
            let mut degree = std::collections::BTreeMap::<usize, usize>::new();
            for &s in &seen {
                degree.insert(s, 0);
            }
            for &s in &seen {
                for n in facts.successors(s as u32)?.into_iter().flatten() {
                    if n == tail.exit() || (s == tail.latch() as usize && n == tail.header()) {
                        continue;
                    }
                    let Some(d) = degree.get_mut(&(n as usize)) else {
                        valid = false;
                        break;
                    };
                    *d += 1;
                }
            }
            if !valid {
                continue;
            }
            let mut ready: Vec<_> = degree
                .iter()
                .filter_map(|(&s, &d)| (d == 0).then_some(s))
                .collect();
            let mut count = 0;
            while let Some(s) = ready.pop() {
                count += 1;
                for n in facts.successors(s as u32)?.into_iter().flatten() {
                    if n == tail.exit() || (s == tail.latch() as usize && n == tail.header()) {
                        continue;
                    }
                    let d = degree
                        .get_mut(&(n as usize))
                        .ok_or("fixed-key successor outside body")?;
                    *d -= 1;
                    if *d == 0 {
                        ready.push(n as usize)
                    }
                }
            }
            if count != seen.len() {
                continue;
            }
            if constructors.iter().any(|&s| {
                let A::LibraryCall { template, .. } = plan.blocks()[s].action() else {
                    return true;
                };
                template.inputs() != seed_factory.inputs()
                    || template.output() != seed_factory.output()
            }) {
                continue;
            }
            constructors.sort_unstable();
            out.regions.push(Region {
                owner: bank.owner,
                iteration: owner.iteration(),
                initialize: tail.initialize() as usize,
                header: tail.header() as usize,
                latch: tail.latch() as usize,
                finish: tail.finish() as usize,
                exit: tail.exit() as usize,
                cell: bank.cell,
                insert: bank.insert,
                constructors,
                sites: seen.into_iter().collect(),
            });
        }
        Ok(out)
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
        let decl = s.split("\nbuild(").next().unwrap();
        format!(
            "{decl}\ntags := tags<[string]>\nhistogram := map/histogram(tags)\nkeys := map/keys(histogram)\nend := sequence/length(keys)\nindex := renumber(keys,0u64,end,histogram)\nids := [index{{tag}} | tag <- tags]\nids\n"
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
            &m.borrowed_dictionary_values,
            &m.matrix_map_keys,
        )
        .unwrap()
    }
    #[test]
    fn fixed_key_native_loops_require_closed_bodies_and_private_complete_map_uses() {
        let text = source();
        for (s, eligible) in [
            (text.clone(), true),
            (
                text.replace("index := renumber(keys,", "index := renumber(tags,"),
                false,
            ),
            (
                format!("{}(ids,index)\n", text.strip_suffix("ids\n").unwrap()),
                false,
            ),
        ] {
            let a = artifact(&s);
            let b = mech_engine::decode_program_artifact_bytecode_v1(
                &mech_engine::encode_program_artifact_bytecode_v1(&a).unwrap(),
            )
            .unwrap();
            let x = derive(&a);
            assert_eq!(!x.regions.is_empty(), eligible, "{s}\n{x:?}");
            assert_eq!(x, derive(&b));
            if eligible {
                assert_eq!(x.regions[0].constructors.len(), 1);
            }
        }
    }
    #[test]
    #[ignore = "requires caller-pinned full source"]
    fn fixed_key_native_loops_actual_source_has_two_complete_acyclic_bodies() {
        std::thread::Builder::new().stack_size(32*1024*1024).spawn(||{
            let text=std::fs::read_to_string(std::env::var("MECH_BORROWED_DICTIONARY_SOURCE").unwrap()).unwrap();let a=artifact(&text);let b=mech_engine::decode_program_artifact_bytecode_v1(&mech_engine::encode_program_artifact_bytecode_v1(&a).unwrap()).unwrap();let x=derive(&a);assert_eq!(x.regions.len(),2,"{x:?}");assert_eq!(x,derive(&b));println!("{}",serde_json::json!({"native_fixed_key_loop_candidates":x,"emission_connected":true,"timing_eligible":false}));
        }).unwrap().join().unwrap()
    }
}
