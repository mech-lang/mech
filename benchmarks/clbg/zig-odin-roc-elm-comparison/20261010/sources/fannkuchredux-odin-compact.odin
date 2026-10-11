package main
import "core:fmt"
import "core:os"
import "core:strconv"
main :: proc() {
    n := strconv.parse_int(os.args[1]) or_else 7
    assert(n <= 16)
    perm: [16]int
    scratch: [16]int
    count: [16]int
    for i in 0..<n { perm[i] = i }
    r := n
    index := 0
    checksum := 0
    maximum := 0
    for {
        for r > 1 {
            count[r-1] = r
            r -= 1
        }
        copy(scratch[:n], perm[:n])
        flips := 0
        for scratch[0] != 0 {
            k := scratch[0]
            for i in 0..<(k+1)/2 {
                scratch[i], scratch[k-i] = scratch[k-i], scratch[i]
            }
            flips += 1
        }
        maximum = max(maximum, flips)
        if index%2 == 0 { checksum += flips } else { checksum -= flips }
        for {
            if r == n {
                fmt.printf("%d\nPfannkuchen(%d) = %d\n", checksum, n, maximum)
                return
            }
            first := perm[0]
            for i in 0..<r { perm[i] = perm[i+1] }
            perm[r] = first
            count[r] -= 1
            if count[r] > 0 { break }
            r += 1
        }
        index += 1
    }
}
