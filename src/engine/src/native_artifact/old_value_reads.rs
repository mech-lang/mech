//! Mechanical graph query only. Consumers decide which reads and owners qualify.
use super::NativeProgramFacts;
use crate::native_artifact::NativeMatchValue;

impl NativeProgramFacts<'_> {
    /// An action reads before writing its result; a redefining action kills the
    /// old owner before its terminator. Includes backedges and the frontier's
    /// terminator. Opaque reads and loop/publication roles remain caller policy.
    pub(crate) fn old_value_read_before_redefinition(
        &self,
        value: NativeMatchValue,
        frontier: u32,
        action_reads: impl Fn(u32) -> bool,
        terminator_reads: impl Fn(u32) -> bool,
    ) -> Result<bool, &'static str> {
        let definitions = self.definition_sites(value.get())?;
        reachable_read(
            self.plan().blocks().len(),
            frontier,
            |site| self.successors(site),
            |site| {
                definitions
                    .iter()
                    .any(|definition| definition.site() == Some(site))
            },
            action_reads,
            terminator_reads,
        )
    }
}

fn reachable_read(
    block_count: usize,
    frontier: u32,
    successors: impl Fn(u32) -> Result<[Option<u32>; 2], &'static str>,
    writes: impl Fn(u32) -> bool,
    action_reads: impl Fn(u32) -> bool,
    terminator_reads: impl Fn(u32) -> bool,
) -> Result<bool, &'static str> {
    if frontier as usize >= block_count {
        return Err("old-value frontier outside control graph");
    }
    if terminator_reads(frontier) {
        return Ok(true);
    }
    let mut seen = vec![false; block_count];
    let mut pending = successors(frontier)?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    while let Some(site) = pending.pop() {
        let seen = seen
            .get_mut(site as usize)
            .ok_or("old-value edge outside control graph")?;
        if core::mem::replace(seen, true) {
            continue;
        }
        if action_reads(site) {
            return Ok(true);
        }
        if writes(site) {
            continue;
        }
        if terminator_reads(site) {
            return Ok(true);
        }
        pending.extend(successors(site)?.into_iter().flatten());
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::reachable_read;

    fn query(
        graph: &[[Option<u32>; 2]],
        writes: &[u32],
        action: &[u32],
        term: &[u32],
    ) -> Result<bool, &'static str> {
        reachable_read(
            graph.len(),
            0,
            |site| graph.get(site as usize).copied().ok_or("missing edge"),
            |site| writes.contains(&site),
            |site| action.contains(&site),
            |site| term.contains(&site),
        )
    }

    #[test]
    fn old_value_query_preserves_action_write_terminator_order() {
        let graph = [
            [Some(1), None],
            [Some(2), None],
            [Some(3), None],
            [None, None],
        ];
        assert_eq!(query(&graph, &[2], &[2], &[]), Ok(true));
        assert_eq!(query(&graph, &[2], &[], &[2]), Ok(false));
        assert_eq!(query(&graph, &[2], &[3], &[]), Ok(false));
        assert_eq!(query(&graph, &[2], &[], &[1]), Ok(true));
        assert_eq!(query(&graph, &[2], &[], &[0]), Ok(true));
        // The consuming action itself has already read; only future reads count.
        assert_eq!(query(&graph, &[2], &[0], &[]), Ok(false));
    }

    #[test]
    fn old_value_query_handles_backedges_branches_and_fresh_definitions() {
        let mut graph = [
            [Some(1), None],
            [Some(2), Some(3)],
            [Some(0), None],
            [None, None],
        ];
        assert_eq!(query(&graph, &[], &[0], &[]), Ok(true));
        assert_eq!(query(&graph, &[2], &[0], &[]), Ok(false));
        assert_eq!(query(&graph, &[2], &[3], &[]), Ok(true));
        graph[3] = [Some(3), None];
        assert_eq!(query(&graph, &[2], &[], &[]), Ok(false));
        graph[3] = [Some(4), None];
        assert!(query(&graph, &[], &[], &[]).is_err());
    }
}
