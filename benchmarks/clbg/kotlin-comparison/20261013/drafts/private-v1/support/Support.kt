package clbg

import java.math.BigDecimal
import java.math.RoundingMode
import java.nio.charset.StandardCharsets

object Support {
    fun size(args: Array<String>, fallback: Int): Int {
        require(args.size <= 1)
        val n = if (args.isEmpty()) fallback else args[0].toInt()
        require(n >= 0)
        return n
    }
    fun input(): ByteArray = System.`in`.readBytes()
    fun text(bytes: ByteArray): String = String(bytes, StandardCharsets.ISO_8859_1)
    fun bytes(text: String): ByteArray = text.toByteArray(StandardCharsets.ISO_8859_1)
    fun write(bytes: ByteArray) { System.out.write(bytes); System.out.flush() }
    fun write(text: String) = write(bytes(text))
    fun fixed(value: Double, digits: Int): String {
        require(value.isFinite() && digits in 0..18)
        val rounded = BigDecimal(value).setScale(digits, RoundingMode.HALF_EVEN).toPlainString()
        return if (java.lang.Double.doubleToRawLongBits(value) < 0 && !rounded.startsWith("-")) "-$rounded" else rounded
    }
    fun lines(bytes: ByteArray): List<String> = text(bytes).replace("\r\n", "\n").replace('\r', '\n').split('\n')
}
