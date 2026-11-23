//! Capacity frontiers between generated header checks. Every unbounded cycle
//! crosses a declared Tail header. Cutting those frontiers leaves a finite DAG;
//! its longest append path includes child returns and ancestor continuations.
use super::*;

pub(super) fn derive(
    plan: &Plan,
    tree: &super::super::numeric_tree::Proof,
    sites: &BTreeSet<usize>,
    actions: &[(usize, Operation)],
    arrays: &[Array],
) -> Result<Vec<(usize, Vec<(usize, usize)>)>, &'static str> {
    let appends = arrays
        .iter()
        .enumerate()
        .filter_map(|(at, array)| array.appends.then_some(at))
        .collect::<Vec<_>>();
    if appends.is_empty() {
        return Ok(Vec::new());
    }
    // Optional optimization scratch is bounded before allocating its tables.
    // Oversized analyses retain the ordinary/independently proved leaf route.
    const MAX_FACTS: usize = 1 << 20;
    if sites
        .len()
        .checked_mul(appends.len())
        .is_none_or(|facts| facts > MAX_FACTS)
    {
        return Err("numeric append frontier exceeds bounded proof storage");
    }
    let headers = tree
        .nodes
        .iter()
        .map(|node| node.tail.header() as usize)
        .collect::<BTreeSet<_>>();
    let mut next = BTreeMap::new();
    for &site in sites {
        if headers.contains(&site) {
            continue;
        }
        let mut successors = Vec::new();
        for target in NativeProgramFacts::control_successors(plan.blocks()[site].terminator())
            .into_iter()
            .flatten()
            .map(|site| site as usize)
        {
            if target == tree.nodes[0].tail.exit() as usize || headers.contains(&target) {
                continue;
            }
            if !sites.contains(&target) {
                return Err("numeric append frontier leaves its exact tree");
            }
            successors.push(target);
        }
        next.insert(site, successors);
    }
    let mut incoming = next
        .keys()
        .map(|&site| (site, 0usize))
        .collect::<BTreeMap<_, _>>();
    for successors in next.values() {
        for target in successors {
            let count = incoming
                .get_mut(target)
                .ok_or("numeric append frontier reaches an unknown source")?;
            *count = count
                .checked_add(1)
                .ok_or("numeric append frontier edge count overflows")?;
        }
    }
    let mut ready = incoming
        .iter()
        .filter_map(|(&site, &count)| (count == 0).then_some(site))
        .collect::<Vec<_>>();
    let mut order = Vec::new();
    while let Some(site) = ready.pop() {
        order.push(site);
        for target in &next[&site] {
            let count = incoming.get_mut(target).unwrap();
            *count -= 1;
            if *count == 0 {
                ready.push(*target);
            }
        }
    }
    if order.len() != next.len() {
        return Err("numeric append frontier contains an undeclared cycle");
    }
    let mut limits = BTreeMap::<usize, Vec<usize>>::new();
    for &site in order.iter().rev() {
        let mut row = vec![0usize; appends.len()];
        for target in &next[&site] {
            for (maximum, count) in row.iter_mut().zip(&limits[target]) {
                *maximum = (*maximum).max(*count);
            }
        }
        if let Some(Operation::Append { array, .. }) =
            actions.iter().find(|(at, _)| *at == site).map(|(_, op)| op)
        {
            let column = appends
                .iter()
                .position(|at| at == array)
                .ok_or("numeric append frontier lost its array")?;
            row[column] = row[column]
                .checked_add(1)
                .ok_or("numeric append frontier count overflows")?;
        }
        limits.insert(site, row);
    }
    tree.nodes
        .iter()
        .map(|node| {
            let header = node.tail.header() as usize;
            let body = node.tail.body() as usize;
            let row = if headers.contains(&body) {
                vec![0; appends.len()]
            } else {
                limits
                    .get(&body)
                    .cloned()
                    .ok_or("numeric append header lost its exact body")?
            };
            Ok((header, appends.iter().copied().zip(row).collect()))
        })
        .collect()
}
