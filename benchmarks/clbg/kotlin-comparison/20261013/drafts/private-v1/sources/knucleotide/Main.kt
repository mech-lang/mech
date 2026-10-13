package clbg.knucleotide
import clbg.Support

val queries = listOf("GGT", "GGTA", "GGTATT", "GGTATTTTAATT", "GGTATTTTAATTTATAGT")
fun table(sequence: String, k: Int): HashMap<String, Long> {
    val counts = HashMap<String, Long>()
    var i = 0
    while (i + k <= sequence.length) {
        val key = sequence.substring(i, i + k)
        counts[key] = (counts[key] ?: 0L) + 1L
        i++
    }
    return counts
}
fun main(args: Array<String>) {
    val dna = StringBuilder()
    var selected = false
    for (line in Support.lines(Support.input())) {
        if (line.startsWith(">")) {
            if (selected) break
            val name = line.substring(1).takeWhile { it != ' ' && it != '\t' }
            selected = name == "THREE"
        } else if (selected && line.isNotEmpty() && !line.startsWith(";")) {
            for (code in line) {
                val upper = if (code in 'a'..'z') code - 32 else code
                require(upper == 'A' || upper == 'C' || upper == 'G' || upper == 'T')
                dna.append(upper)
            }
        }
    }
    val sequence = dna.toString()
    val lengths = listOf(1, 2, 3, 4, 6, 12, 18)
    val maps = lengths.associateWith { table(sequence, it) }
    val out = StringBuilder()
    for (k in listOf(1, 2)) {
        val counts = maps.getValue(k)
        val ordered = counts.entries.sortedWith(compareByDescending<Map.Entry<String, Long>> { it.value }.thenBy { it.key })
        val total = maxOf(0, sequence.length - k + 1)
        for ((key, count) in ordered) out.append("$key ${Support.fixed(100.0 * count / total, 3)}\n")
        out.append('\n')
    }
    for (query in queries) out.append("${maps.getValue(query.length)[query] ?: 0L}\t$query\n")
    Support.write(out.toString())
}
