//! Single-use adjacent Tail seeds may be constructed in their child's lanes.
//! Lane allocation stays disjoint; this proof grants neither owners nor fuel.
use super::frame_retirement::FrameRetirements;
use mech_engine::native_artifact::{
    NativeBufferFrameField as Field, NativeBufferFrameLane as Lane,
    NativeBufferFrameOperation as Operation, NativeBufferFrames as Frames,
    NativeControlLoop as Loop, NativeIterationAction as Action, NativeIterationTerminator as Term,
    NativeMatchTerminator as ScalarTerm, NativeMatchValue as Value,
    NativeProgramArtifactPlan as Plan, NativeProgramProducerRole as Producer,
    NativeProgramReadRole as Read,
};

#[derive(Debug)]
pub(in crate::native_artifact) struct Recipe {
    pub pack: u32,
    pub initialize: u32,
    pub seed: Value,
    pub carrier: Value,
    pub copy_work: u64,
}

#[derive(Debug, Default)]
pub(in crate::native_artifact) struct Layout {
    recipes: Box<[Recipe]>,
}
impl Layout {
    pub(in crate::native_artifact) fn derive(
        plan: &Plan,
        frames: &Frames,
        retirements: &FrameRetirements,
    ) -> Result<Self, &'static str> {
        let facts = plan.facts();
        let mut recipes = Vec::new();
        for (pack, block) in plan.blocks().iter().enumerate() {
            let Some(action) = frames.action(pack) else {
                continue;
            };
            let Operation::Pack { output: seed } = action.operation() else {
                continue;
            };
            let Term::Scalar(ScalarTerm::Jump(initialize)) = block.terminator() else {
                continue;
            };
            let Action::Initialize { iteration } = plan.blocks()[initialize as usize].action()
            else {
                continue;
            };
            let Loop::Tail(tail) = &plan.iterations()[*iteration as usize] else {
                continue;
            };
            let carrier = tail.carrier();
            let definitions = facts.definition_sites(seed.get())?;
            if tail.seed() != *seed
                || tail.initialize() != initialize
                || carrier == *seed
                || facts.predecessors(initialize)? != [pack as u32]
                || definitions.len() != 1
                || definitions[0].site() != Some(pack as u32)
                || definitions[0].role() != Producer::Library
                || !facts.read_roles(*seed)?.iter().all(|role| {
                    matches!(role, Read::LoopSeed(id) if id == iteration)
                        || matches!(role, Read::Action(site) if *site == initialize)
                })
                || action.retire().contains(&carrier)
            {
                continue;
            }
            let Some(copy) = frames.action(initialize as usize) else {
                continue;
            };
            if !matches!(copy.operation(), Operation::Copy { source, output }
                if source == seed && *output == carrier)
                || !copy.retire().contains(seed)
            {
                continue;
            }
            let (Some(source), Some(target)) = (frames.fields(*seed), frames.fields(carrier))
            else {
                continue;
            };
            if !same_fields(frames, source, target) {
                continue;
            }
            let Some((retire_work, _)) = retirements.get(seed.get() as usize) else {
                continue;
            };
            // Copy's original 64*n+64 price includes structural nodes, exactly
            // twice the original retirement price. No dynamic debit is omitted.
            let copy_work = retire_work
                .checked_mul(2)
                .ok_or("seed copy work overflows")?;
            recipes.push(Recipe {
                pack: pack as u32,
                initialize,
                seed: *seed,
                carrier,
                copy_work,
            });
        }
        Ok(Self {
            recipes: recipes.into_boxed_slice(),
        })
    }
    pub(in crate::native_artifact) fn at_pack(&self, site: usize) -> Option<(usize, &Recipe)> {
        let index = self
            .recipes
            .binary_search_by_key(&site, |r| r.pack as usize)
            .ok()?;
        Some((index, &self.recipes[index]))
    }
    pub(in crate::native_artifact) fn get(&self, index: usize) -> Option<&Recipe> {
        self.recipes.get(index)
    }
}

fn same_fields(layout: &Frames, source: &[Field], target: &[Field]) -> bool {
    source.len() == target.len()
        && source.iter().zip(target).all(|(source, target)| {
            source.schema() == target.schema()
                && match (source.lane(), target.lane()) {
                    (Lane::Scalar { at: a, kind: ka }, Lane::Scalar { at: b, kind: kb }) => {
                        a != b && ka == kb
                    }
                    (Lane::Managed { at: a }, Lane::Managed { at: b }) => a != b,
                    (Lane::Composite { .. }, Lane::Composite { .. }) => {
                        match (layout.children(*source), layout.children(*target)) {
                            (Some(a), Some(b)) => same_fields(layout, a, b),
                            _ => false,
                        }
                    }
                    _ => false,
                }
        })
}
