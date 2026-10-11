from std.sys import argv
from max.algorithm.backend.cpu.parallelize import parallelize

def work(n: Int, imm facts: List[Int], first: Int, last: Int) -> Tuple[Int, Int]:
    var p = List[Int]()
    var counts = List[Int]()
    for i in range(n):
        p.append(i)
        counts.append(0)
    var p_ptr = p.unsafe_ptr()
    var count_ptr = counts.unsafe_ptr()
    var fact_ptr = facts.unsafe_ptr()
    var rank = first
    for i in range(n - 1, 0, -1):
        var d = rank // fact_ptr[i]
        count_ptr[i] = d
        rank %= fact_ptr[i]
        var old = List[Int](copy=p)
        var old_ptr = old.unsafe_ptr()
        for j in range(i + 1):
            p_ptr[j] = old_ptr[(j + d) % (i + 1)]
    var checksum = 0
    var maximum = 0
    var q = List[Int]()
    q.resize(n, 0)
    var q_ptr = q.unsafe_ptr()
    for rank in range(first, last):
        for i in range(n):
            q_ptr[i] = p_ptr[i]
        var flips = 0
        while q_ptr[0] != 0:
            var k = q_ptr[0]
            for i in range((k + 1) // 2):
                var temp = q_ptr[i]
                q_ptr[i] = q_ptr[k - i]
                q_ptr[k - i] = temp
            flips += 1
        checksum += flips if rank % 2 == 0 else -flips
        maximum = max(maximum, flips)
        var r = 1
        while r < n:
            var first_value = p_ptr[0]
            for i in range(r):
                p_ptr[i] = p_ptr[i + 1]
            p_ptr[r] = first_value
            if count_ptr[r] < r:
                count_ptr[r] += 1
                break
            count_ptr[r] = 0
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
