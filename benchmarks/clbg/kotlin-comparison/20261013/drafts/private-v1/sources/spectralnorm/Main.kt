package clbg.spectralnorm
import clbg.Support
import kotlin.math.sqrt

fun a(i: Int, j: Int): Double = 1.0 / (((i + j).toLong() * (i + j + 1) / 2) + i + 1).toDouble()
fun multiply(input: DoubleArray, output: DoubleArray, transpose: Boolean) {
    var i = 0
    while (i < input.size) {
        var sum = 0.0
        var j = 0
        while (j < input.size) { sum += (if (transpose) a(j, i) else a(i, j)) * input[j]; j++ }
        output[i] = sum
        i++
    }
}
fun ata(input: DoubleArray, output: DoubleArray, work: DoubleArray) { multiply(input, work, false); multiply(work, output, true) }
fun main(args: Array<String>) {
    val n = Support.size(args, 100)
    require(n > 0)
    val u = DoubleArray(n) { 1.0 }
    val v = DoubleArray(n)
    val work = DoubleArray(n)
    repeat(10) { ata(u, v, work); ata(v, u, work) }
    var uv = 0.0
    var vv = 0.0
    var i = 0
    while (i < n) { uv += u[i] * v[i]; vv += v[i] * v[i]; i++ }
    Support.write(Support.fixed(sqrt(uv / vv), 9) + "\n")
}
