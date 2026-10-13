package clbg.pidigits
import clbg.Support
import java.math.BigInteger

fun big(n: Long): BigInteger = BigInteger.valueOf(n)
fun main(args: Array<String>) {
    val wanted = Support.size(args, 27)
    var q = BigInteger.ONE
    var r = BigInteger.ZERO
    var t = BigInteger.ONE
    var k = 1L
    var emitted = 0
    var line = ""
    val out = StringBuilder()
    while (emitted < wanted) {
        val three = (q * big(3) + r) / t
        val four = (q * big(4) + r) / t
        if (three == four) {
            emitted++
            line += three.toInt().toString()
            if (emitted % 10 == 0 || emitted == wanted) out.append(line.padEnd(10, ' ') + "\t:$emitted\n")
            q *= big(10)
            r = (r - t * three) * big(10)
            if (emitted % 10 == 0) line = ""
        } else {
            val factor = big(2 * k + 1)
            r = (r + q * big(2)) * factor
            q *= big(k)
            t *= factor
            k++
        }
    }
    Support.write(out.toString())
}
