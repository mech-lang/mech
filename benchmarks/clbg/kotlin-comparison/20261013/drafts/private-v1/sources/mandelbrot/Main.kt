package clbg.mandelbrot
import clbg.Support
import java.io.ByteArrayOutputStream

fun inside(cr: Double, ci: Double): Boolean {
    var zr = 0.0
    var zi = 0.0
    var count = 0
    while (count < 50) {
        val next = zr * zr - zi * zi + cr
        zi = 2.0 * zr * zi + ci
        zr = next
        if (zr * zr + zi * zi > 4.0) return false
        count++
    }
    return true
}
fun main(args: Array<String>) {
    val n = Support.size(args, 200)
    require(n > 0)
    val out = ByteArrayOutputStream()
    out.write(Support.bytes("P4\n$n $n\n"))
    var y = 0
    while (y < n) {
        var byte = 0
        var bits = 0
        var x = 0
        while (x < n) {
            byte = (byte shl 1) or if (inside(2.0 * x / n - 1.5, 2.0 * y / n - 1.0)) 1 else 0
            bits++
            if (bits == 8) { out.write(byte); byte = 0; bits = 0 }
            x++
        }
        if (bits != 0) out.write(byte shl (8 - bits))
        y++
    }
    Support.write(out.toByteArray())
}
