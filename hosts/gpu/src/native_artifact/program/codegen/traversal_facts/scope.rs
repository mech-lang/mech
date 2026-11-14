//! Acyclic current-visit order; the original latch is a stop.
use super::*;
pub(super) fn successors(term: Term) -> Option<Vec<u32>> {
    Some(match term {
        Term::TailHeader { body, .. } | Term::Scalar(Branch::Jump(body)) => vec![body],
        Term::Scalar(Branch::Branch { yes, no, .. } | Branch::LiteralBranch { yes, no, .. }) => {
            vec![yes, no]
        }
        Term::Scalar(Branch::Reject) => Vec::new(),
        _ => return None,
    })
}
pub(in crate::native_artifact::program::codegen) struct Scope {
    pub header: usize,
    pub latch: usize,
    pub exit: u32,
    pub sites: Vec<usize>,
    /// Edges leave the cloned version and resume original checked code. They
    /// never carry facts into a later source continuation.
    pub outside: BTreeSet<u32>,
}
impl Scope {
    /// Keep the closed, supported part of this fresh visit. An opaque child or
    /// unknown operation is an exit to the original checked traversal, never a
    /// summary that can carry facts into its continuation.
    pub fn supported_prefix(
        &self,
        code: CodePlan<'_>,
        supported: impl Fn(usize) -> bool,
    ) -> Option<Self> {
        let CodePlan::Program(program) = code else {
            return None;
        };
        let owned = self.sites.iter().copied().collect::<BTreeSet<_>>();
        let next = |site: usize| {
            if site == self.latch {
                Some(Vec::new())
            } else {
                successors(code.blocks()[site].terminator())
            }
        };
        let mut kept = BTreeSet::new();
        let mut pending = vec![self.header];
        while let Some(site) = pending.pop() {
            if !owned.contains(&site) || kept.contains(&site) || !supported(site) {
                continue;
            }
            let Some(edges) = next(site) else {
                continue;
            };
            kept.insert(site);
            pending.extend(edges.into_iter().filter_map(|edge| {
                (edge != self.exit && !self.outside.contains(&edge)).then_some(edge as usize)
            }));
        }
        // A continuation reachable from an omitted child/operation must not
        // borrow the facts of a different predecessor. Prune that continuation
        // and its descendants, even when another supported arm reaches it.
        for (site, visit) in program.metadata().visits.iter().enumerate() {
            if !kept.contains(&site) {
                pending.extend(visit.successors().into_iter().flatten().filter_map(|edge| {
                    (edge as usize != self.header && kept.contains(&(edge as usize)))
                        .then_some(edge as usize)
                }));
            }
        }
        while let Some(site) = pending.pop() {
            if site == self.header || !kept.remove(&site) {
                continue;
            }
            pending.extend(next(site)?.into_iter().map(|edge| edge as usize));
        }
        if !kept.contains(&self.header) {
            return None;
        }
        let mut outside = self.outside.clone();
        for &site in &kept {
            outside.extend(
                next(site)?
                    .into_iter()
                    .filter(|edge| *edge != self.exit && !kept.contains(&(*edge as usize))),
            );
        }
        Some(Self {
            header: self.header,
            latch: self.latch,
            exit: self.exit,
            sites: self
                .sites
                .iter()
                .copied()
                .filter(|site| kept.contains(site))
                .collect(),
            outside,
        })
    }

    pub fn order(&self, code: CodePlan<'_>) -> Option<Vec<usize>> {
        let sites = &self.sites;
        if sites.len() > MAX_SITES {
            return None;
        }
        let mut incoming = BTreeMap::<usize, usize>::new();
        for &site in sites {
            incoming.insert(site, 0);
        }
        for &site in sites {
            if site == self.latch {
                continue;
            }
            for next in successors(code.blocks()[site].terminator())? {
                if next != self.exit && !self.outside.contains(&next) {
                    *incoming.get_mut(&(next as usize))? += 1;
                }
            }
        }
        let mut ready = incoming
            .iter()
            .filter_map(|(&site, &n)| (n == 0).then_some(site))
            .collect::<Vec<_>>();
        let mut ordered = Vec::new();
        while let Some(site) = ready.pop() {
            ordered.push(site);
            if site == self.latch {
                continue;
            }
            for next in successors(code.blocks()[site].terminator())? {
                if next == self.exit || self.outside.contains(&next) {
                    continue;
                }
                let count = incoming.get_mut(&(next as usize))?;
                *count = count.checked_sub(1)?;
                if *count == 0 {
                    ready.push(next as usize);
                }
            }
        }
        (ordered.len() == sites.len()).then_some(ordered)
    }
}
