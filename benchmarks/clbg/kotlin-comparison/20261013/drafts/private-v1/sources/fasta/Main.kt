package clbg.fasta
import clbg.Support
import java.io.ByteArrayOutputStream

val alu = Support.bytes("GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA")
val iub = listOf('a' to 0.27, 'c' to 0.12, 'g' to 0.12, 't' to 0.27, 'B' to 0.02, 'D' to 0.02, 'H' to 0.02, 'K' to 0.02, 'M' to 0.02, 'N' to 0.02, 'R' to 0.02, 'S' to 0.02, 'V' to 0.02, 'W' to 0.02, 'Y' to 0.02)
val homo = listOf('a' to 0.3029549426680, 'c' to 0.1979883004921, 'g' to 0.1975473066391, 't' to 0.3015094502008)
fun cumulative(table: List<Pair<Char, Double>>): List<Pair<Char, Double>> {
    var sum = 0.0
    return table.map { sum += it.second; it.first to sum }
}
fun repeated(out: ByteArrayOutputStream, count: Int) {
    var i = 0
    while (i < count) { out.write(alu[i % alu.size].toInt()); i++; if (i % 60 == 0) out.write(10) }
    if (count % 60 != 0) out.write(10)
}
fun random(out: ByteArrayOutputStream, count: Int, table: List<Pair<Char, Double>>, initial: Int): Int {
    var seed = initial
    var i = 0
    while (i < count) {
        seed = (seed * 3877 + 29573) % 139968
        val draw = seed / 139968.0
        var j = 0
        while (j < table.size - 1 && draw >= table[j].second) j++
        out.write(table[j].first.code)
        i++
        if (i % 60 == 0) out.write(10)
    }
    if (count % 60 != 0) out.write(10)
    return seed
}
fun main(args: Array<String>) {
    val n = Support.size(args, 1000)
    val out = ByteArrayOutputStream()
    out.write(Support.bytes(">ONE Homo sapiens alu\n")); repeated(out, 2 * n)
    out.write(Support.bytes(">TWO IUB ambiguity codes\n")); val seed = random(out, 3 * n, cumulative(iub), 42)
    out.write(Support.bytes(">THREE Homo sapiens frequency\n")); random(out, 5 * n, cumulative(homo), seed)
    Support.write(out.toByteArray())
}
