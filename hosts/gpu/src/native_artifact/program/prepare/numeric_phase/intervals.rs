//! Synchronous source boundaries in the root traversal. They end native flow;
//! ordinary execution owns the operation, result, and remaining traversal.
use super::*;

pub(super) fn boundaries(
    plan: &Plan,
    root: u32,
) -> Result<(BTreeSet<usize>, BTreeSet<usize>), &'static str> {
    let mut stops = BTreeSet::new();
    let mut prepaid = BTreeSet::new();
    for (site, block) in plan.blocks().iter().enumerate() {
        if block.iteration() != Some(root) {
            continue;
        }
        prepaid.insert(site);
        match block.action() {
            Action::Initialize { iteration } if *iteration != root => {
                let child = &plan.iterations()[*iteration as usize];
                if child.as_ref().parent() != Some(root)
                    || !matches!(
                        child,
                        NativeControlLoop::Tail(_) | NativeControlLoop::Collection(_)
                    )
                {
                    return Err("numeric interval crosses an unsupported progress boundary");
                }
                stops.insert(site);
            }
            Action::CollectionBegin { collection } => {
                if plan.collections()[*collection as usize].parent() != Some(root) {
                    return Err("numeric interval collection has another parent");
                }
                stops.insert(site);
            }
            Action::PrepareU64Range { iteration } => {
                if plan.iterations()[*iteration as usize].as_ref().parent() != Some(root) {
                    return Err("numeric interval range has another parent");
                }
                stops.insert(site);
            }
            _ => {}
        }
    }
    if stops.is_empty() {
        return Err("numeric interval has no synchronous cold boundary");
    }
    Ok((stops, prepaid))
}
