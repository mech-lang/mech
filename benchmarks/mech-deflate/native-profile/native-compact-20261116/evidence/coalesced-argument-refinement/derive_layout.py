"""Portable actual-count layout heuristic with unchanged exact-score segment refinement."""
from collections import defaultdict

def order_by_weight(weights, hotness):
    chains = {at: [at] for at in range(len(hotness))}
    origins = list(chains)
    for (src, dest), weight in sorted(weights.items(), key=lambda e: (-e[1], *e[0])):
        if not weight or dest == 0:
            continue
        left, right = origins[src], origins[dest]
        if left == right or chains[left][-1] != src or right != dest:
            continue
        group = chains.pop(right)
        chains[left].extend(group)
        for block in group:
            origins[block] = left
    tails = sorted((head for head in chains if head != 0),
                   key=lambda head: (-max(hotness[node] for node in chains[head]), head))
    return [node for head in [0, *tails] for node in chains[head]]

def refine_fast(initial,weights):
 def edge(a,b):return 0. if a is None or b is None else weights.get((a,b),0.)
 def score(order):return sum(edge(a,b) for a,b in zip(order,order[1:]))
 current=initial
 while True:
  base_score=score(current);best=current;best_score=base_score
  for begin in range(1,len(current)):
   for size in range(1,min(4,len(current)-begin)+1):
    segment=current[begin:begin+size];rem=current[:begin]+current[begin+size:];left=current[begin-1];right=current[begin+size] if begin+size<len(current) else None
    cut=edge(left,right)-edge(left,segment[0])-edge(segment[-1],right)
    for dest in range(1,len(rem)+1):
     if dest==begin:continue
     a=rem[dest-1];b=rem[dest] if dest<len(rem) else None
     estimated=base_score+cut+edge(a,segment[0])+edge(segment[-1],b)-edge(a,b)
     if estimated>best_score+1e-9:
      offered=rem[:dest]+segment+rem[dest:];actual=score(offered)
      assert abs(actual-estimated)<1e-8
      if actual>best_score+1e-9:best,best_score=offered,actual
  if best==current:return current
  assert best_score>base_score+1e-9;current=best

def derive(shape, trace):
    assert trace['object_sha256'] == shape['object_sha256']
    heads = shape['code_heads']
    identifiers = {head: i for i, head in enumerate(heads)}
    graph = [tuple(dict.fromkeys(v for v in row[1:] if v is not None))
             for row in shape['code_blocks']]
    incoming = defaultdict(list)
    for src, targets in enumerate(graph):
        for dst in targets:
            incoming[dst].append((src, dst))
    weighted = defaultdict(float)
    hotness = [0.] * len(graph)
    evidence = {}
    for case, sample in trace['cases'].items():
        visited = {int(b['address'], 16) - shape['atom']['start']: b['visits'] for b in sample['blocks']}
        assert set(visited) <= set(heads)
        counts = [visited.get(h, 0) for h in heads]
        assert counts[0] == sample['calls']
        peak = max(counts)
        flows = {}
        constraints = []
        for src, targets in enumerate(graph):
            if targets:
                constraints.append((counts[src], [(src, dst) for dst in targets]))
        for dst in range(len(graph)):
            constraints.append((counts[dst] - (sample['calls'] if dst == 0 else 0), incoming[dst]))
        changed = True
        while changed:
            changed = False
            for total, edges in constraints:
                remaining = total - sum(flows[e] for e in edges if e in flows)
                assert remaining >= 0
                missing = [e for e in edges if e not in flows]
                if not missing:
                    assert remaining == 0
                elif remaining == 0 or len(missing) == 1:
                    for edge in missing:
                        flows[edge] = 0 if remaining == 0 else remaining
                        changed = True
        edges = [(src, dst) for src, targets in enumerate(graph) for dst in targets]
        unresolved = [edge for edge in edges if edge not in flows]
        for edge, value in flows.items():
            assert 0 <= value <= min(counts[edge[0]], counts[edge[1]])
        scores = dict(flows)
        for src, dst in unresolved:
            out_remaining = counts[src] - sum(flows.get((src, target), 0) for target in graph[src])
            in_remaining = counts[dst] - sum(flows.get(edge, 0) for edge in incoming[dst])
            unknown_out = sum((src, target) not in flows for target in graph[src])
            unknown_in = sum(edge not in flows for edge in incoming[dst])
            scores[src, dst] = min(out_remaining / unknown_out, in_remaining / unknown_in)
        if peak:
            for index, count in enumerate(counts):
                hotness[index] += count / peak
            for edge, score in scores.items():
                weighted[edge] += score / peak
        evidence[case] = {'function_calls': sample['calls'], 'exact_constrained_edges': len(flows),
                          'total_edges': len(edges), 'unresolved_edges': unresolved,
                          'constraint_flows': {str(edge): value for edge, value in flows.items()},
                          'layout_priority_scores': {str(edge): value for edge, value in scores.items()}}
    ranked = {}
    for src, targets in enumerate(graph):
        for dst in targets:
            fall = shape['code_blocks'][src][2]
            potential = weighted[src, fall] if len(targets) == 2 else weighted[src, dst]
            ranked[src, dst] = potential + .35 * weighted[src, dst] + .001
    order = refine_fast(order_by_weight(ranked, hotness), ranked)
    assert sorted(order) == list(range(len(heads)))
    profile = {'order': [heads[i] for i in order] + shape['zero_literal_padding'],
               'literal_offset': shape['pool'], 'thread_jumps': True}
    assert sorted(profile['order']) == shape['heads']
    proof = {'shape': shape, 'cases': evidence,
             'scope': 'Actual native block counts with exact integer flow constraints where uniquely resolved. '
                      'Unresolved edges receive endpoint-based layout priority scores; these are not actual '
                      'edge frequencies or hardware cycles. The resulting order is a layout heuristic; '
                      'shipping elapsed time and strict native checks are measured independently.'}
    return profile, proof
