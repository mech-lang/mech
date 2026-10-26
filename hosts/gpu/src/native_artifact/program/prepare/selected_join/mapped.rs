//! A complete-use join result followed only by copies and its String-map entry.
//! These facts never authorize an earlier error or a fabricated String Value.
use mech_engine::native_artifact::{
    NativeControlLoop, NativeIterationAction as Action, NativeIterationTerminator as End,
    NativeIterationValueScope as Scope, NativeMatchAction as Scalar, NativeMatchTerminator as Term,
    NativeProgramArtifactPlan as Plan, NativeSelectedTextJoin, NativeStringMapRegion,
};

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Map {
    pub collection: u32,
    pub iteration: u32,
    pub source: u32,
    pub map: u32,
    // Sorted membership inventory. Source CFG still chooses every successor.
    pub sites: Box<[u32]>,
}

pub(in crate::native_artifact::program::prepare::selected_join) fn derive(
    plan: &Plan,
    join: &NativeSelectedTextJoin,
    maps: &[NativeStringMapRegion],
) -> Option<Map> {
    maps.iter().find_map(|map| derive_one(plan, join, map))
}

fn derive_one(
    plan: &Plan,
    join: &NativeSelectedTextJoin,
    map: &NativeStringMapRegion,
) -> Option<Map> {
    let NativeControlLoop::Collection(generator) =
        plan.iterations().get(map.iteration() as usize)?
    else {
        return None;
    };
    if generator.parent().is_some()
        || plan
            .blocks()
            .get(join.consumer() as usize)?
            .iteration()
            .is_some()
        || !map.owned_source_slots().contains(&join.output())
        || plan.value_scopes().get(map.map().get() as usize)
            != Some(&Scope::Immutable { iteration: None })
    {
        return None;
    }
    let next = |site: u32| match &plan.blocks().get(site as usize)?.terminator() {
        End::Scalar(Term::Jump(next)) => Some(*next),
        _ => None,
    };
    let mut site = next(join.consumer())?;
    let mut current = join.output();
    let mut begun = false;
    let mut initialized = false;
    let mut visited = vec![false; plan.blocks().len()];
    let mut sites = Vec::new();
    loop {
        let block = plan.blocks().get(site as usize)?;
        if plan.activation_boundary() == Some(site) {
            return None;
        }
        if core::mem::replace(visited.get_mut(site as usize)?, true) {
            return None;
        }
        sites.push(site);
        if site == generator.header() {
            if !begun || !initialized || current != map.source() {
                return None;
            }
            sites.sort_unstable();
            return Some(Map {
                collection: map.collection(),
                iteration: map.iteration(),
                source: map.source().get(),
                map: map.map().get(),
                sites: sites.into_boxed_slice(),
            });
        }
        if block.iteration().is_some() {
            return None;
        }
        match &block.action() {
            Action::Scalar(Scalar::None) => {}
            Action::Scalar(Scalar::Copy { source, output })
                if !begun
                    && *source == current
                    && *output != map.map()
                    && map.owned_source_slots().contains(output) =>
            {
                current = *output;
            }
            Action::CollectionBegin { collection } if !begun && *collection == map.collection() => {
                begun = true;
            }
            Action::Initialize { iteration }
                if begun
                    && !initialized
                    && *iteration == map.iteration()
                    && current == map.source() =>
            {
                initialized = true;
            }
            _ => return None,
        }
        site = next(site)?;
    }
}
