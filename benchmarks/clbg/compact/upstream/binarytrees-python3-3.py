# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# contributed by Diogo

from concurrent.futures import ProcessPoolExecutor as PoolExecutor
from multiprocessing import cpu_count
import sys
import gc


def tree_make(n: int) -> list:
    """ Creates a tree recursively based on lists of 2 elements

    Data:   Lists are core data sets for python so they use less
            memory, andalso they provide means for fast iteration

    Advice: List creation is expensive, as much as list appending
            If we create a list from start with the length required
            We can avoid mutating list length, because on low-level
            extending a list length will need to discard existing
            list and creating a new list with a new length.
            But mutating existing list element is not as expensive

    Method: Create a list of nones for optimized creation
            If we are on bottom level return this list
            Else create the child node recursively
            Now e overwrite left and right empty nodes with child
    """

    # Create base node of empty childs
    node = [None, None]

    # If we are on bottom, return empty node
    if n == 0:
        return node

    # Populate and return populated node
    node[0] = tree_make(n-1)
    node[1] = tree_make(n-1)

    return node


def tree_check(n: list) -> int:
    """ Walkthrough the tree and add 1 for each node found """
    # If we are at bottom node, return 1
    if n[0] is None:
        return 1
    # Go through left
    left_result = tree_check(n[0])
    # Go through right
    right_result = tree_check(n[1])
    # Append results
    result = 1 + left_result + right_result
    return result


def tree_make_and_check(n: int, keep_in_memory: bool = False) -> (list, int):
    """ Create a tree and check for its value based on nodes """
    # Create our tree and get number of nodes
    tree: list = tree_make(n)
    check: int = tree_check(tree)

    # Check if we are returning only the value and discarding tree
    # Or if we are keeping tree in memory
    # In the case of the long lived tree we are going to keep it in memory
    if keep_in_memory:
        return (tree, check)
    return (None, check)


def data_iter(d: int, n: int):
    """ Iterate over tree and check it's content """
    niter: int = 1 << (n - d + 4)

    # We will have niter results
    # Then we can store a list of niter length of 0's
    results: list = [0] * niter

    # Then as we create trees we mutate the zeros
    for index in range(0, niter):
        tree: list = tree_make(d)
        check: int = tree_check(tree)
        results[index] = check

    # At the end we sum all results to get final return
    c: int = sum(results)

    return niter, d, c


def binary_trees_run(n: int, workers: int):
    """ Run binary trees benchmark """

    # Disable garbage collection for less overhead
    gc.disable()

    # Create an executor and a dict to track futures
    result: dict = {}
    executor = PoolExecutor(max_workers=workers)

    # Create stretch tree with the pool and verify its check value
    # This first tree we are going to create and discard tree
    # result['stretch'] = executor.submit(tree_make_and_check, n + 1, False)
    stretch_tree = tree_make_and_check(n + 1, False)

    # Create long lived tree with the pool and verify its check value
    # This long lived tree will be kept in memory until the end
    # result['longlived'] = executor.submit(tree_make_and_check, n, True)
    longlived = tree_make_and_check(n, True)

    # Loop through tree to make iterations as soon as they arrive
    # Trees are stored as {id}

    # Getting list of depths and storing tuples of (depth, id)
    depth_list: list = list(range(4, n, 2) if n % 2 != 0 else range(4, n+1, 2))
    id_list: list = list(range(0, len(depth_list)))
    iter_list: list = list(zip(depth_list, id_list))

    # iter_batch will be a tuple of (depth, id)
    # Lets submit workers
    for work in iter_list:
        item = work[1]
        d = work[0]
        result[item] = executor.submit(data_iter, d, n)

    # Time to get results from those futures
    # Print stretch tree result as we get it and discard the tree from memory
    # stretch_check_val: int = result['stretch'].result()[1]
    # print(f"stretch tree of depth {n+1}\t check: {stretch_check_val}")
    # del result['stretch']
    print(f"stretch tree of depth {n+1}\t check: {stretch_tree[1]}")

    # Loop through iterations and get results in order for correct output
    # Once we get the tree for the moment, discard the tree from memory
    for value in id_list:
        niter, d, c = result[value].result()
        print(f"{niter}\t trees of depth {d}\t check: {c}")
        del result[value]

    # Now we can print long lived tree and discard the tree
    # longlived_check_val: int = result['longlived'].result()[1]
    # print(f"long lived tree of depth {n}\t check: {longlived_check_val}")
    # del result['longlived']
    print(f"long lived tree of depth {n}\t check: {longlived[1]}")
    del longlived


if __name__ == "__main__":

    # Get how many cores available
    logicalcpu_cores: int = cpu_count()
    workers: int = 2
    if logicalcpu_cores > 2:
        workers = logicalcpu_cores

    n: int = int(sys.argv[1])
    binary_trees_run(n, workers)
