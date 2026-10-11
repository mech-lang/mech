from std.memory import Pointer
from std.origin import MutUntrackedOrigin
from pool import Pool, initialize, terminate

def main():
    initialize()
    var pool = Pool()
    for round in range(17):
        var addresses = List[UInt64]()
        for i in range(1,2049):
            var address = pool.allocate_bytes(8 * (i % 31 + 1))
            var ptr = Pointer[UInt64, MutUntrackedOrigin](unsafe_from_address=Int(address))
            ptr[0] = UInt64(i + round * 3000)
            ptr[i % 31] = UInt64(i + round * 3000)
            addresses.append(address)
        for i in range(1,2049):
            var ptr = Pointer[UInt64, MutUntrackedOrigin](unsafe_from_address=Int(addresses[i-1]))
            assert ptr[0] == UInt64(i + round * 3000)
            assert ptr[i % 31] == UInt64(i + round * 3000)
        pool.clear()
    pool.destroy()
    terminate()
    print("Passed 34816 distinct allocations across 17 pool clear/reuse rounds")
