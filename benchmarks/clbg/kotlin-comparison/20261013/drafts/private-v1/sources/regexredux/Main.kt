package clbg.regexredux
import clbg.Support
import java.util.regex.Pattern

val variants = listOf("agggtaaa|tttaccct", "[cgt]gggtaaa|tttaccc[acg]", "a[act]ggtaaa|tttacc[agt]t", "ag[act]gtaaa|tttac[agt]ct", "agg[act]taaa|ttta[agt]cct", "aggg[acg]aaa|ttt[cgt]ccct", "agggt[cgt]aa|tt[acg]accct", "agggta[cgt]a|t[acg]taccct", "agggtaa[cgt]|[acg]ttaccct")
val replacements = listOf("tHa[Nt]" to "<4>", "aND|caN|Ha[DS]|WaS" to "<3>", "a[NSt]|BY" to "<2>", "<[^>]*>" to "|", "\\|[^|][^|]*\\|" to "-")
fun matches(pattern: String, text: String): Int {
    val matcher = Pattern.compile(pattern).matcher(text)
    var count = 0
    while (matcher.find()) count++
    return count
}
fun replace(pattern: String, text: String, replacement: String): String {
    val matcher = Pattern.compile(pattern).matcher(text)
    val out = StringBuilder()
    var cursor = 0
    while (matcher.find()) {
        out.append(text, cursor, matcher.start()).append(replacement)
        cursor = matcher.end()
    }
    out.append(text, cursor, text.length)
    return out.toString()
}
fun main(args: Array<String>) {
    val raw = Support.text(Support.input())
    val dna = replace(">[^\n]*\n|\n", raw, "")
    val out = StringBuilder()
    for (pattern in variants) out.append("$pattern ${matches(pattern, dna)}\n")
    var changed = dna
    for ((pattern, replacement) in replacements) changed = replace(pattern, changed, replacement)
    out.append("\n${raw.length}\n${dna.length}\n${changed.length}\n")
    Support.write(out.toString())
}
