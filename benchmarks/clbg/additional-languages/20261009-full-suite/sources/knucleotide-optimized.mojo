from max.algorithm.backend.cpu.parallelize import parallelize
from uintmap import UIntMap
from common import read_bytes, write_string, fixed_string

def encode(imm text: String) -> UInt64:
    var key: UInt64 = 0
    for c in text.as_bytes():
        key = key * 4 + UInt64(0 if c == 65 else 1 if c == 67 else 2 if c == 71 else 3)
    return key

def count(imm dna: List[UInt8], width: Int, wanted: UInt64) -> List[UInt64]:
    var limit = len(dna) - width + 1
    var bins: Int = (1 << (2 * width)) if width <= 2 else 1
    var results = List[UInt64](length=32 * bins, fill=UInt64(0))
    var dst = results.unsafe_ptr()
    var src = dna.unsafe_ptr()
    def worker(piece: Int) {imm dna, var limit, var width, var bins, var dst, var src, var wanted}:
        var first = piece * limit // 32
        var last = (piece + 1) * limit // 32
        var table = UIntMap()
        var key: UInt64 = 0
        var mask = (UInt64(1) << UInt64(2 * width)) - 1
        if first < last:
            for j in range(width):
                key = key * 4 + UInt64(src[first + j])
        for i in range(first, last):
            table.increment(key)
            if i + width < len(dna):
                key = ((key << 2) | UInt64(src[i + width])) & mask
        for bin in range(bins):
            dst[piece * bins + bin] = table.get(UInt64(bin) if width <= 2 else wanted, UInt64(0))
    parallelize(worker, 32, 8)
    var totals = List[UInt64](length=bins, fill=UInt64(0))
    for bin in range(bins):
        for piece in range(32):
            totals[bin] += results[piece * bins + bin]
    return totals^

def frequency(imm dna: List[UInt8], width: Int):
    var values = count(dna, width, 0)
    var symbols = "ACGT".as_bytes()
    for _ in range(len(values)):
        var key = 0
        for j in range(len(values)):
            if values[j] > values[key]:
                key = j
        if values[key] == 0:
            break
        var word = String()
        for j in range(width):
            word += chr(Int(symbols[(key >> (2 * (width - j - 1))) & 3]))
        write_string(word + " " + fixed_string(100.0 * Float64(values[key]) / Float64(len(dna) - width + 1), 3) + "\n")
        values[key] = 0
    write_string("\n")

def main() raises:
    var input = read_bytes()
    var marker = ">THREE".as_bytes()
    var start = 0
    while start + 6 <= len(input):
        var matched = True
        for j in range(6):
            matched = matched and input[start + j] == marker[j]
        if matched:
            break
        start += 1
    while start < len(input) and input[start] != 10:
        start += 1
    var dna = List[UInt8](capacity=len(input) - start)
    for i in range(start + 1, len(input)):
        var c = input[i]
        if c == 62:
            break
        if c != 10 and c != 13:
            dna.append(UInt8(0 if c == 65 or c == 97 else 1 if c == 67 or c == 99 else 2 if c == 71 or c == 103 else 3))
    frequency(dna, 1)
    frequency(dna, 2)
    var queries: List[String] = ["GGT", "GGTA", "GGTATT", "GGTATTTTAATT", "GGTATTTTAATTTATAGT"]
    for query in queries:
        var hits = count(dna, query.byte_length(), encode(query))
        write_string(String(hits[0]) + "\t" + query + "\n")
