"""Find repeated single-entry root regions for the existing checked outliner."""
from collections import defaultdict, deque
from dataclasses import dataclass
from hashlib import sha256
import json
from pathlib import Path
import sys


@dataclass(frozen=True)
class Block:
    cold: bool
    status: int | None
    origin: int | None
    successors: tuple[int, ...]
    parameters: tuple[str, ...]
    instructions: tuple[str, ...]


def split_list(field):
    return tuple(value for value in field.split(',') if value)


def read_graph(path):
    blocks = {}
    for row in path.read_text().splitlines():
        number, cold, status, origin, successors, parameters, instructions = row.split('\t')
        index = int(number)
        assert index not in blocks
        blocks[index] = Block(cold == '1', int(status) if status else None, int(origin) if origin else None,
                              tuple(map(int, split_list(successors))), split_list(parameters), split_list(instructions))
    return blocks


def reachable(blocks, root):
    found = {root}
    pending = [root]
    while pending:
        for successor in blocks[pending.pop()].successors:
            assert successor in blocks
            if successor not in found:
                found.add(successor)
                pending.append(successor)
    return found


def postorder(children, root):
    seen = {root}
    stack = [(root, iter(sorted(children[root])))]
    order = []
    while stack:
        node, cursor = stack[-1]
        successor = next(cursor, None)
        if successor is None:
            order.append(node)
            stack.pop()
        elif successor not in seen:
            seen.add(successor)
            stack.append((successor, iter(sorted(children[successor]))))
    return order


def postdominators(blocks, live):
    # Ignore known nonzero status returns when finding a normal continuation.
    # Region construction below still records each checked error edge.
    virtual_exit = -1
    predecessors = defaultdict(set)
    exits = set()
    for node in live:
        assert blocks[node].instructions
        if blocks[node].instructions[-1] == 'return' and blocks[node].status in (0, None):
            predecessors[virtual_exit].add(node)
            exits.add(node)
        for next_block in blocks[node].successors:
            predecessors[next_block].add(node)
    order = list(reversed(postorder(predecessors, virtual_exit)))
    assert len(order) > 1
    position = {block: index for index, block in enumerate(order)}
    parents = {virtual_exit: virtual_exit}

    def meet(a, b):
        while a != b:
            if position[a] > position[b]:
                a = parents[a]
            else:
                b = parents[b]
        return a

    while True:
        changed = False
        for node in order[1:]:
            successors = blocks[node].successors if node not in exits else (virtual_exit,)
            known = iter(s for s in successors if s in parents)
            parent = next(known, None)
            if parent is None:
                continue
            for earlier in known:
                parent = meet(parent, earlier)
            if parents.get(node) != parent:
                changed = True
                parents[node] = parent
        if not changed:
            break
    assert len(parents) == len(order)
    return parents, predecessors


def region(blocks, predecessors, entry, exit_block):
    numbering = {entry: 0, exit_block: -1}
    members = []
    pending = deque([entry])
    exit_edges = 0
    while pending:
        block = pending.popleft()
        members.append(block)
        if len(members) > 64:
            return None
        body = blocks[block]
        if not body.successors and not (body.instructions[-1] == 'return' and body.status not in (None, 0)):
            return None
        for successor in body.successors:
            if successor == exit_block:
                exit_edges += 1
            elif successor not in numbering:
                numbering[successor] = len(numbering) - 1
                pending.append(successor)
    if not exit_edges:
        return None
    member_set = set(members)
    if any(predecessors[node] - member_set for node in members[1:]):
        return None
    instructions = sum(len(blocks[node].instructions) for node in members)
    if instructions < 12:
        return None
    base = min((blocks[node].origin for node in members if blocks[node].origin is not None), default=0)
    key = tuple((body.cold, body.status, None if body.origin is None else body.origin - base,
                 tuple(numbering[s] for s in body.successors), body.parameters, body.instructions)
                for body in (blocks[node] for node in members))
    return key, {'entry': entry, 'exit': exit_block, 'origin_base': base, 'block_ids': members,
                 'instructions': instructions, 'entry_predecessors': sorted(predecessors[entry]),
                 'cold_blocks': sum(blocks[node].cold for node in members)}


def inspect(path):
    blocks = read_graph(path)
    root = next(iter(blocks))
    live = reachable(blocks, root)
    parents, predecessors = postdominators(blocks, live)
    shapes = defaultdict(list)
    for entry in sorted(live - {root}):
        exit_block = entry
        for _ in range(4):
            exit_block = parents.get(exit_block, -1)
            if exit_block < 0:
                break
            result = region(blocks, predecessors, entry, exit_block)
            if result:
                signature, member = result
                shapes[signature].append(member)
    matches = []
    for groups in shapes.values():
        if len(groups) < 2:
            continue
        records = []
        occupied = set()
        for member in groups:
            member_set = set(member['block_ids'])
            if not member_set & occupied:
                records.append(member)
                occupied.update(member_set)
        if len(records) > 1:
            retained = records[0]['instructions']
            assert all(member['instructions'] == retained for member in records)
            matches.append({'instances': len(records), 'instructions_per_instance': retained,
                            'repeated_instructions_before_calls_and_checks': retained * (len(records) - 1),
                            'regions': records})
    matches.sort(key=lambda item: item['repeated_instructions_before_calls_and_checks'], reverse=True)
    return {'block_roles_sha256': sha256(path.read_bytes()).hexdigest(), 'layout_blocks': len(blocks),
            'reachable_blocks': len(live), 'reachable_instructions': sum(len(blocks[b].instructions) for b in live),
            'normal_postdominator_blocks': len(parents) - 1, 'repeated_region_patterns': len(matches),
            'max_postdominator_depth': 4, 'max_region_blocks': 64,
            'scope': 'Opcode/type/callee/CFG shapes only. Existing outliner must prove exact operands, guards, captures, '
                     'checked exits, ownership and source origins. Rows may overlap.',
            'largest_regions': matches[:30]}


if __name__ == '__main__':
    print(json.dumps(inspect(Path(sys.argv[1])), indent=2))
