package clbg.revcomp
import clbg.Support

fun complement(code: Char): Char = when (code.uppercaseChar()) {
    'A' -> 'T'; 'C' -> 'G'; 'G' -> 'C'; 'T', 'U' -> 'A'
    'M' -> 'K'; 'R' -> 'Y'; 'W' -> 'W'; 'S' -> 'S'; 'Y' -> 'R'; 'K' -> 'M'
    'V' -> 'B'; 'H' -> 'D'; 'D' -> 'H'; 'B' -> 'V'; 'N' -> 'N'
    else -> error("Unsupported nucleotide")
}
fun record(out: StringBuilder, header: String, sequence: StringBuilder) {
    out.append(header).append('\n')
    var column = 0
    var i = sequence.length - 1
    while (i >= 0) { out.append(complement(sequence[i])); column++; if (column == 60) { out.append('\n'); column = 0 }; i-- }
    if (column != 0) out.append('\n')
}
fun main(args: Array<String>) {
    val out = StringBuilder()
    var header: String? = null
    var sequence = StringBuilder()
    for (line in Support.lines(Support.input())) {
        if (line.startsWith(">")) {
            if (header != null) record(out, header, sequence)
            header = line
            sequence = StringBuilder()
        } else if (header != null && line.isNotEmpty() && !line.startsWith(";")) sequence.append(line)
    }
    if (header != null) record(out, header, sequence)
    Support.write(out.toString())
}
