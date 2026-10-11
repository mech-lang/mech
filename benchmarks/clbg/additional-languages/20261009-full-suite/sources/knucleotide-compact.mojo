from std.collections import Dict
from common import read_bytes, write_string, fixed_string

def encode(imm text: String) -> UInt64:
    var key: UInt64 = 0
    for c in text.as_bytes():
        key = key * 4 + UInt64(0 if c == 65 else 1 if c == 67 else 2 if c == 71 else 3)
    return key

def count(imm dna: List[UInt8], width: Int) -> Dict[UInt64, UInt64]:
    var table = Dict[UInt64, UInt64]()
    var key: UInt64 = 0
    var mask = (UInt64(1) << UInt64(2 * width)) - 1
    for i in range(len(dna)):
        key = ((key << 2) | UInt64(dna[i])) & mask
        if i + 1 >= width:
            table.setdefault(key, 0) += 1
    return table^

def main() raises:
    var text = String(from_utf8=read_bytes())
    var start = text.find("\n", text.find(">THREE")) + 1
    var dna = List[UInt8]()
    for c in text.as_bytes()[start:]:
        if c == 62:
            break
        if c != 10 and c != 13:
            dna.append(UInt8(0 if c == 65 or c == 97 else 1 if c == 67 or c == 99 else 2 if c == 71 or c == 103 else 3))
    for width in range(1, 3):
        var table = count(dna, width)
        for _ in range(1 << (2 * width)):
            var best: UInt64 = 0
            for key in range(1 << (2 * width)):
                if table.get(UInt64(key), 0) > table.get(best, 0):
                    best = UInt64(key)
            if table.get(best, 0) == 0:
                break
            var word = String()
            for j in range(width):
                word += chr(Int("ACGT".as_bytes()[Int((best >> UInt64(2 * (width - j - 1))) & 3)]))
            write_string(word + " " + fixed_string(100.0 * Float64(table[best]) / Float64(len(dna) - width + 1), 3) + "\n")
            table[best] = 0
        write_string("\n")
    var queries: List[String] = ["GGT", "GGTA", "GGTATT", "GGTATTTTAATT", "GGTATTTTAATTTATAGT"]
    for query in queries:
        var table = count(dna, query.byte_length())
        write_string(String(table.get(encode(query), 0)) + "\t" + query + "\n")
