from std.sys import argv
from max.algorithm.backend.cpu.parallelize import parallelize

def work(n: Int, imm facts: List[Int], first: Int, last: Int) -> Tuple[Int, Int]:
    var p = List[Int]()
    var counts = List[Int]()
    for i in range(n):
        p.append(i)
        counts.append(0)
    var rank = first
    for i in range(n - 1, 0, -1):
        var d = rank // facts[i]
        counts[i] = d
        rank %= facts[i]
        var old = List[Int](copy=p)
        for j in range(i + 1):
            p[j] = old[(j + d) % (i + 1)]
    var checksum = 0
    var maximum = 0
    var q = List[Int]()
    q.resize(n, 0)
    for rank in range(first, last):
        for i in range(n):
            q[i] = p[i]
        var flips = 0
        while q[0] != 0:
            var k = q[0]
            for i in range((k + 1) // 2):
                var temp = q[i]
                q[i] = q[k - i]
                q[k - i] = temp
            flips += 1
        checksum += flips if rank % 2 == 0 else -flips
        maximum = max(maximum, flips)
        var r = 1
        while r < n:
            var first_value = p[0]
            for i in range(r):
                p[i] = p[i + 1]
            p[r] = first_value
            if counts[r] < r:
                counts[r] += 1
                break
            counts[r] = 0
            r += 1
    return (checksum, maximum)

def main() raises:
    var n = Int(argv()[1])
    var facts: List[Int] = [1]
    for i in range(1, n + 1):
        facts.append(facts[i - 1] * i)
    var sums: List[Int] = [0, 0, 0, 0, 0, 0, 0, 0]
    var maxima: List[Int] = [0, 0, 0, 0, 0, 0, 0, 0]
    var sum_ptr = sums.unsafe_ptr()
    var max_ptr = maxima.unsafe_ptr()
    def worker(w: Int) {imm facts, var n, var sum_ptr, var max_ptr}:
        var result = work(n, facts, w * facts[n] // 8, (w + 1) * facts[n] // 8)
        sum_ptr[w] = result[0]
        max_ptr[w] = result[1]
    parallelize(worker, 8, 8)
    var checksum = 0
    var maximum = 0
    for w in range(8):
        checksum += sums[w]
        maximum = max(maximum, maxima[w])
    print(checksum)
    print("Pfannkuchen(" + String(n) + ") = " + String(maximum))
