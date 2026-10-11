from common import read_bytes, write_string
from regex import compile, close, count, replace

def substitute(imm text: List[UInt8], imm pattern: String, imm replacement: String) -> List[UInt8]:
    var handle = compile(pattern)
    var result = replace(handle, text, replacement)
    close(handle)
    return result^

def main() raises:
    var input = read_bytes()
    var initial = len(input)
    var sequence = substitute(input, ">.*\n|\n", "")
    var cleaned = len(sequence)
    var patterns: List[String] = ["agggtaaa|tttaccct", "[cgt]gggtaaa|tttaccc[acg]", "a[act]ggtaaa|tttacc[agt]t",
        "ag[act]gtaaa|tttac[agt]ct", "agg[act]taaa|ttta[agt]cct", "aggg[acg]aaa|ttt[cgt]ccct",
        "agggt[cgt]aa|tt[acg]accct", "agggta[cgt]a|t[acg]taccct", "agggtaa[cgt]|[acg]ttaccct"]
    for pattern in patterns:
        var handle = compile(pattern)
        write_string(pattern + " " + String(count(handle, sequence)) + "\n")
        close(handle)
    sequence = substitute(sequence, "tHa[Nt]", "<4>")
    sequence = substitute(sequence, "aND|caN|Ha[DS]|WaS", "<3>")
    sequence = substitute(sequence, "a[NSt]|BY", "<2>")
    sequence = substitute(sequence, "<[^>]*>", "|")
    sequence = substitute(sequence, "\\|[^|][^|]*\\|", "-")
    write_string("\n" + String(initial) + "\n" + String(cleaned) + "\n" + String(len(sequence)) + "\n")
