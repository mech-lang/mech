from common import read_bytes, write_bytes

def main() raises:
    var input = read_bytes()
    var out = List[UInt8]()
    var from_ = "ACGTUMRWSYKVHDBNacgtumrwsykvhdbn".as_bytes()
    var to = "TGCAAKYWSRMBDHVNTGCAAKYWSRMBDHVN".as_bytes()
    var start = 0
    while start < len(input):
        while start < len(input):
            var c = input[start]
            out.append(c)
            start += 1
            if c == 10:
                break
        var end = start
        while end < len(input) and input[end] != 62:
            end += 1
        var column = 0
        for i in range(end - 1, start - 1, -1):
            var c = input[i]
            if c != 10 and c != 13:
                for k in range(len(from_)):
                    if from_[k] == c:
                        c = to[k]
                        break
                out.append(c)
                column += 1
                if column % 60 == 0:
                    out.append(10)
        if column % 60:
            out.append(10)
        start = end
    write_bytes(out)
