from max.algorithm.backend.cpu.parallelize import parallelize
from common import read_bytes, write_bytes

def main() raises:
    var bytes = read_bytes()
    var starts = List[Int]()
    for i in range(len(bytes)):
        if bytes[i] == 62 and (i == 0 or bytes[i - 1] == 10):
            var j = i
            while j < len(bytes) and bytes[j] != 10:
                j += 1
            starts.append(j + 1)
    var lut = List[UInt8]()
    for i in range(256):
        lut.append(UInt8(i))
    var src = "ACGTUMRWSYKVHDBNacgtumrwsykvhdbn".as_bytes()
    var dst = "TGCAAKYWSRMBDHVNTGCAAKYWSRMBDHVN".as_bytes()
    for i in range(len(src)):
        lut[Int(src[i])] = dst[i]
    var data = bytes.unsafe_ptr()
    var table = lut.unsafe_ptr()
    var offsets = starts.unsafe_ptr()
    var size = len(bytes)
    var count = len(starts)
    def record(k: Int) {var}:
        var lo = offsets[k]
        var hi = size - 1
        if k + 1 < count:
            hi = offsets[k + 1] - 2
            while hi >= lo and data[hi] != 62:
                hi -= 1
            hi -= 1
        while lo <= hi:
            if data[lo] == 10 or data[lo] == 13:
                lo += 1
            elif data[hi] == 10 or data[hi] == 13:
                hi -= 1
            else:
                var a = table[Int(data[lo])]
                data[lo] = table[Int(data[hi])]
                data[hi] = a
                lo += 1
                hi -= 1
    parallelize(record, count, 8)
    write_bytes(bytes)
