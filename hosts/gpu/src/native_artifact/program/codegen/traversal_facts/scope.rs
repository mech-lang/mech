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
}
impl Scope {
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
                if next != self.exit {
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
                if next == self.exit {
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
