from itertools import product
from random import Random

rng = Random(993)
counts = {"inactive": 0, "dominance-live": 0, "dominance-dead": 0, "remaining": 0}
valid_seen = {category: 0 for category in counts}
tested = 0


def flood(successors, start, stop):
    reached = set()
    pending = [start]
    while pending:
        node = pending.pop()
        if node == stop or node in reached:
            continue
        reached.add(node)
        pending.extend(successors[node])
    return reached


def check(successors):
    global tested
    reached = flood(successors, 0, None)
    count = len(successors)
    # Dominance is computed independently as entry reachability when deleting a
    # vertex, with the same entry-reachability restriction as the compiler.
    dominated = [reached - flood(successors, 0, stopper) for stopper in range(count)]
    for owner, target in product(range(count), sorted(reached)):
        old_reachable = flood(successors, target, owner)
        owner_dominates_target = target in dominated[owner]
        for use in range(count):
            old = (None if use not in dominated[target] or not owner_dominates_target else True) if use in old_reachable else False
            if owner == target or use == owner or use not in reached:
                new = False
                category = "inactive"
            elif owner_dominates_target and use in dominated[target]:
                new = True
                category = "dominance-live"
            elif not owner_dominates_target and use in dominated[owner]:
                new = False
                category = "dominance-dead"
            else:
                new = None if use in old_reachable else False
                category = "remaining"
            assert new == old, (successors, owner, target, use, new, old)
            counts[category] += 1
            if use not in reached or use in dominated[owner]:
                valid_seen[category] += 1
            tested += 1


for bits in range(1 << 9):
    check([[dst for dst in range(3) if bits & (1 << (src * 3 + dst))]
           for src in range(3)])
for size in range(4, 9):
    for _ in range(64):
        check([[rng.randrange(size) for _ in range(rng.randrange(2 * size))]
               for src in range(size)])
print(f"{tested:,} queries, all outcomes identical", counts)
print("Dominance-valid use subset", valid_seen)
