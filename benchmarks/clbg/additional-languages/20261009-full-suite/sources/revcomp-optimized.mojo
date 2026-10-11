from max.algorithm.backend.cpu.parallelize import parallelize
from common import read_bytes, write_bytes

def main() raises:
    var input = read_bytes()
    var lut = List[UInt8](unsafe_uninit_length=256)
    for i in range(256):
        lut[i] = UInt8(i)
    var from_ = "ACGTUMRWSYKVHDBNacgtumrwsykvhdbn".as_bytes()
    var to = "TGCAAKYWSRMBDHVNTGCAAKYWSRMBDHVN".as_bytes()
    for i in range(len(from_)):
        lut[Int(from_[i])] = to[i]
    var table = lut.unsafe_ptr()
    var data = input.unsafe_ptr()
    var start = 0
    while start < len(input):
        var header = start
        while header < len(input) and data[header] != 10:
            header += 1
        header += 1
        var end = header
        while end < len(input) and data[end] != 62:
            end += 1
        var dna = List[UInt8](unsafe_uninit_length=end - header)
        var sequence = dna.unsafe_ptr()
        var used = 0
        for i in range(header, end):
            if data[i] != 10 and data[i] != 13:
                sequence[used] = data[i]
                used += 1
        var header_size = header - start
        var size = used + (used + 59) // 60
        var bytes = List[UInt8](unsafe_uninit_length=header_size + size)
        var out = bytes.unsafe_ptr()
        for i in range(header_size):
            out[i] = data[start + i]
        def worker(task: Int) {var}:
            for j in range(task * size // 32, (task + 1) * size // 32):
                var k = j // 61 * 60 + j % 61
                out[header_size + j] = 10 if j % 61 == 60 or k >= used else table[Int(sequence[used - k - 1])]
        parallelize(worker, 32, 8)
        write_bytes(bytes)
        start = end
