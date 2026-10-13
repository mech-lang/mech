package clbg.fannkuchredux
import clbg.Support

fun rotate(p: IntArray, last: Int) {
    val first = p[0]
    var i = 0
    while (i < last) { p[i] = p[i + 1]; i++ }
    p[last] = first
}
fun flips(p: IntArray): Int {
    val work = p.copyOf()
    var total = 0
    while (work[0] != 0) {
        var left = 0
        var right = work[0]
        while (left < right) { val v = work[left]; work[left] = work[right]; work[right] = v; left++; right-- }
        total++
    }
    return total
}
fun main(args: Array<String>) {
    val n = Support.size(args, 7)
    require(n > 0)
    val p = IntArray(n) { it }
    val count = IntArray(n)
    var r = n
    var rank = 0L
    var checksum = 0L
    var maximum = 0
    while (true) {
        while (r != 1) { count[r - 1] = r; r-- }
        val amount = flips(p)
        checksum += if (rank % 2L == 0L) amount.toLong() else -amount.toLong()
        maximum = maxOf(maximum, amount)
        while (r < n) { rotate(p, r); count[r]--; if (count[r] > 0) break; r++ }
        if (r == n) break
        rank++
    }
    Support.write("$checksum\nPfannkuchen($n) = $maximum\n")
}
